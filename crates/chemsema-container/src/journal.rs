use super::{canonical_json_bytes, sha256_hex};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const JOURNAL_SCHEMA: &str = "chemsema.journal.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JournalRecord {
    pub schema: String,
    pub sequence: u64,
    pub base_document_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_record_sha256: Option<String>,
    pub patch: Value,
    pub record_sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Journal {
    base_document_sha256: String,
    records: Vec<JournalRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JournalRead {
    pub journal: Journal,
    pub ignored_truncated_tail: bool,
}

impl Journal {
    pub fn new(base_document_sha256: impl Into<String>) -> Result<Self, String> {
        let hash = base_document_sha256.into();
        validate_hash("base document", &hash)?;
        Ok(Self {
            base_document_sha256: hash,
            records: Vec::new(),
        })
    }

    pub fn base_document_sha256(&self) -> &str {
        &self.base_document_sha256
    }

    pub fn records(&self) -> &[JournalRecord] {
        &self.records
    }

    pub fn append_patch(&mut self, patch: Value) -> Result<&JournalRecord, String> {
        if !patch.is_object() {
            return Err("Journal patch must be a JSON object".to_string());
        }
        let sequence = self.records.len() as u64 + 1;
        let previous_record_sha256 = self
            .records
            .last()
            .map(|record| record.record_sha256.clone());
        let unsigned = unsigned_value(
            sequence,
            &self.base_document_sha256,
            previous_record_sha256.as_deref(),
            &patch,
        );
        let record_sha256 = sha256_hex(&canonical_json_bytes(&unsigned)?);
        self.records.push(JournalRecord {
            schema: JOURNAL_SCHEMA.to_string(),
            sequence,
            base_document_sha256: self.base_document_sha256.clone(),
            previous_record_sha256,
            patch,
            record_sha256,
        });
        Ok(self.records.last().expect("record was appended"))
    }

    pub fn to_jsonl(&self) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        for record in &self.records {
            bytes.extend(canonical_json_bytes(
                &serde_json::to_value(record).map_err(|error| error.to_string())?,
            )?);
            bytes.push(b'\n');
        }
        Ok(bytes)
    }

    /// Start a new journal after the caller has durably written and verified a
    /// checkpoint. This never discards records on disk by itself.
    pub fn compacted(&self, checkpoint_sha256: impl Into<String>) -> Result<Self, String> {
        Self::new(checkpoint_sha256)
    }

    pub fn parse_strict(bytes: &[u8]) -> Result<Self, String> {
        parse(bytes, false).map(|read| read.journal)
    }

    /// Accept an incomplete final record only when the file lacks a final LF.
    /// Corruption in any durable (LF-terminated) record is always rejected.
    pub fn recover_prefix(bytes: &[u8]) -> Result<JournalRead, String> {
        parse(bytes, true)
    }
}

fn parse(bytes: &[u8], allow_truncated_tail: bool) -> Result<JournalRead, String> {
    let has_final_lf = bytes.last().is_none_or(|byte| *byte == b'\n');
    let mut parsed = Vec::<JournalRecord>::new();
    let lines = bytes.split(|byte| *byte == b'\n').collect::<Vec<_>>();
    let mut ignored_truncated_tail = false;
    for (index, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        let record = match serde_json::from_slice::<JournalRecord>(line) {
            Ok(record) => record,
            Err(_error) if allow_truncated_tail && !has_final_lf && index == lines.len() - 1 => {
                ignored_truncated_tail = true;
                break;
            }
            Err(error) => {
                return Err(format!("Invalid journal record {}: {error}", index + 1));
            }
        };
        verify_record(&record, &parsed)?;
        parsed.push(record);
    }
    let base = parsed
        .first()
        .map(|record| record.base_document_sha256.clone())
        .ok_or_else(|| "Journal contains no complete records".to_string())?;
    Ok(JournalRead {
        journal: Journal {
            base_document_sha256: base,
            records: parsed,
        },
        ignored_truncated_tail,
    })
}

fn verify_record(record: &JournalRecord, previous: &[JournalRecord]) -> Result<(), String> {
    if record.schema != JOURNAL_SCHEMA {
        return Err(format!("Unsupported journal schema '{}'.", record.schema));
    }
    validate_hash("base document", &record.base_document_sha256)?;
    validate_hash("record", &record.record_sha256)?;
    let expected_sequence = previous.len() as u64 + 1;
    if record.sequence != expected_sequence {
        return Err(format!(
            "Journal sequence mismatch: found {}, expected {expected_sequence}",
            record.sequence
        ));
    }
    if let Some(first) = previous.first() {
        if record.base_document_sha256 != first.base_document_sha256 {
            return Err("Journal base document hash changed within the chain".to_string());
        }
    }
    let expected_previous = previous.last().map(|entry| entry.record_sha256.as_str());
    if record.previous_record_sha256.as_deref() != expected_previous {
        return Err(format!(
            "Journal previous hash mismatch at sequence {}",
            record.sequence
        ));
    }
    let unsigned = unsigned_value(
        record.sequence,
        &record.base_document_sha256,
        record.previous_record_sha256.as_deref(),
        &record.patch,
    );
    let expected_hash = sha256_hex(&canonical_json_bytes(&unsigned)?);
    if record.record_sha256 != expected_hash {
        return Err(format!(
            "Journal record hash mismatch at sequence {}",
            record.sequence
        ));
    }
    Ok(())
}

fn unsigned_value(
    sequence: u64,
    base_document_sha256: &str,
    previous_record_sha256: Option<&str>,
    patch: &Value,
) -> Value {
    let mut value = serde_json::json!({
        "schema": JOURNAL_SCHEMA,
        "sequence": sequence,
        "baseDocumentSha256": base_document_sha256,
        "patch": patch,
    });
    if let Some(previous) = previous_record_sha256 {
        value
            .as_object_mut()
            .expect("unsigned journal value is an object")
            .insert(
                "previousRecordSha256".to_string(),
                Value::String(previous.to_string()),
            );
    }
    value
}

fn validate_hash(label: &str, value: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "Journal {label} SHA-256 must contain 64 hex digits"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_hash() -> String {
        "ab".repeat(32)
    }

    #[test]
    fn hash_chain_round_trips_and_detects_tampering() {
        let mut journal = Journal::new(base_hash()).unwrap();
        journal
            .append_patch(serde_json::json!({"revision": 1, "upsertEntities": []}))
            .unwrap();
        journal
            .append_patch(serde_json::json!({"revision": 2, "deletedEntityIds": ["a"]}))
            .unwrap();
        let bytes = journal.to_jsonl().unwrap();
        assert_eq!(Journal::parse_strict(&bytes).unwrap(), journal);

        let mut records = String::from_utf8(bytes).unwrap();
        records = records.replacen("deletedEntityIds", "deletedEntityIxs", 1);
        assert!(Journal::parse_strict(records.as_bytes())
            .unwrap_err()
            .contains("hash mismatch"));
    }

    #[test]
    fn recovery_ignores_only_an_unterminated_tail() {
        let mut journal = Journal::new(base_hash()).unwrap();
        journal
            .append_patch(serde_json::json!({"revision": 1}))
            .unwrap();
        let mut bytes = journal.to_jsonl().unwrap();
        bytes.extend_from_slice(b"{\"schema\":\"chemsema.journal.v1\"");
        let recovered = Journal::recover_prefix(&bytes).unwrap();
        assert!(recovered.ignored_truncated_tail);
        assert_eq!(recovered.journal.records().len(), 1);
        assert!(Journal::parse_strict(&bytes).is_err());
    }

    #[test]
    fn recovery_rejects_corrupt_durable_tail() {
        let mut journal = Journal::new(base_hash()).unwrap();
        journal
            .append_patch(serde_json::json!({"revision": 1}))
            .unwrap();
        let mut bytes = journal.to_jsonl().unwrap();
        bytes.extend_from_slice(b"not-json\n");
        assert!(Journal::recover_prefix(&bytes).is_err());
    }
}
