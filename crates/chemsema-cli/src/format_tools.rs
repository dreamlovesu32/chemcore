use super::*;

const VALIDATION_SCHEMA: &str = "chemsema.validation-report.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValidationLevel {
    Structural,
    Semantic,
    RoundTrip,
}

impl ValidationLevel {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "structural" | "structure" => Ok(Self::Structural),
            "chemical" | "chemistry" | "semantic" | "semantics" => Ok(Self::Semantic),
            "roundtrip" | "round-trip" | "full" => Ok(Self::RoundTrip),
            _ => Err(format!(
                "Unknown validation level '{value}'; expected structural, chemical, or roundtrip."
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Structural => "structural",
            Self::Semantic => "chemical",
            Self::RoundTrip => "roundtrip",
        }
    }
}

pub(crate) fn validate_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut output = None;
    let mut level = ValidationLevel::Semantic;
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--level" => {
                index += 1;
                level = ValidationLevel::parse(
                    args.get(index)
                        .ok_or_else(|| "--level requires a value.".to_string())?,
                )?;
            }
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--pretty" => pretty = true,
            value if input.is_none() => input = Some(value.to_string()),
            value => return Err(format!("Unexpected validate argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "validate requires an input document.".to_string())?;
    let mut service = DesktopDocumentService::default();
    let opened = service.read_document_file(&input)?;
    let mut checks = vec![json!({
        "name": "read",
        "ok": true,
        "format": opened.format,
        "bytes": fs::metadata(&input).map(|value| value.len()).unwrap_or_default(),
    })];

    if matches!(opened.format.as_str(), "ccjs" | "ccjz") {
        let value: Value = serde_json::from_str(&opened.text)
            .map_err(|error| format!("CCJS structural validation failed: {error}"))?;
        validate_ccjs_structure(&value)?;
        checks.push(json!({"name": "ccjs-structure", "ok": true}));
    }

    let mut canonical = None;
    if matches!(opened.format.as_str(), "ccjs" | "ccjz") || level != ValidationLevel::Structural {
        let engine = load_engine_from_file(&input)?;
        let json = document_json(&engine)?;
        checks.push(json!({
            "name": if level == ValidationLevel::Structural {
                "engine-document-invariants"
            } else {
                "engine-semantics"
            },
            "ok": true
        }));
        canonical = Some(json);
    }
    if level == ValidationLevel::RoundTrip {
        let canonical = canonical
            .as_ref()
            .expect("semantic validation produced JSON");
        let mut second = Engine::new();
        second.load_document_json(canonical)?;
        let reparsed = document_json(&second)?;
        let before: Value = serde_json::from_str(canonical).map_err(|error| error.to_string())?;
        let after: Value = serde_json::from_str(&reparsed).map_err(|error| error.to_string())?;
        if before != after {
            return Err("CCJS semantic round trip changed the canonical document.".to_string());
        }
        checks.push(json!({"name": "semantic-roundtrip", "ok": true}));
    }

    let fingerprint = canonical.as_deref().unwrap_or(&opened.text);
    let mut digest = Sha256::new();
    digest.update(fingerprint.as_bytes());
    let sha256 = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    write_json_value(
        json!({
            "ok": true,
            "schema": VALIDATION_SCHEMA,
            "input": input,
            "format": opened.format,
            "level": level.as_str(),
            "canonicalSha256": sha256,
            "checks": checks,
        }),
        output.as_deref(),
        pretty,
    )
}

pub(crate) fn canonicalize_command(args: &[String]) -> Result<(), String> {
    rewrite_command("canonicalize", args)
}

pub(crate) fn migrate_command(args: &[String]) -> Result<(), String> {
    rewrite_command("migrate", args)
}

pub(crate) fn conformance_command(args: &[String]) -> Result<(), String> {
    let mut output = None;
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--pretty" => pretty = true,
            value => return Err(format!("Unexpected conformance argument '{value}'.")),
        }
        index += 1;
    }

    let engine = Engine::new();
    let document = document_json(&engine)?;
    let first = chemsema_container::encode_ccjz(&document)?;
    let second = chemsema_container::encode_ccjz(&document)?;
    if first != second {
        return Err("CCJZ writer is not byte deterministic.".to_string());
    }
    let decoded = chemsema_container::decode_ccjz(&first)?;
    let before: Value = serde_json::from_str(&document).map_err(|error| error.to_string())?;
    let after: Value = serde_json::from_str(&decoded).map_err(|error| error.to_string())?;
    if before != after {
        return Err("CCJZ round trip changed the semantic document.".to_string());
    }
    let base_hash = format!("{:x}", Sha256::digest(document.as_bytes()));
    let mut journal = chemsema_container::Journal::new(base_hash)?;
    journal.append_patch(json!({"revision": 1, "beforeRevision": 0}))?;
    let jsonl = journal.to_jsonl()?;
    let parsed = chemsema_container::Journal::parse_strict(&jsonl)?;
    if parsed.records() != journal.records() {
        return Err("Recovery journal round trip changed records.".to_string());
    }

    write_json_value(
        json!({
            "ok": true,
            "schema": "chemsema.conformance.v1",
            "profiles": ["ccjs-0.2", "ccjz-container-v1", "journal-v1"],
            "checks": [
                {"name": "ccjz-byte-determinism", "ok": true, "bytes": first.len()},
                {"name": "ccjz-semantic-roundtrip", "ok": true},
                {"name": "journal-hash-chain-roundtrip", "ok": true}
            ]
        }),
        output.as_deref(),
        pretty,
    )
}

fn rewrite_command(command: &str, args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut output = None;
    let mut format = None;
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--format" | "-f" => {
                index += 1;
                format = Some(
                    args.get(index)
                        .ok_or_else(|| "--format requires a value.".to_string())?
                        .clone(),
                );
            }
            "--pretty" => pretty = true,
            value if input.is_none() => input = Some(value.to_string()),
            value => return Err(format!("Unexpected {command} argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| format!("{command} requires an input document."))?;
    let output = output.ok_or_else(|| {
        format!("{command} requires --out <path>; source files are never overwritten implicitly.")
    })?;
    if Path::new(&input) == Path::new(&output) {
        return Err(format!(
            "{command} refuses to overwrite its input; choose a distinct --out path."
        ));
    }
    let engine = load_engine_from_file(&input)?;
    write_engine_output(&engine, &output, format.as_deref())?;
    let canonical = document_json(&engine)?;
    let value: Value = serde_json::from_str(&canonical).map_err(|error| error.to_string())?;
    write_json_value(
        json!({
            "ok": true,
            "command": command,
            "input": input,
            "output": output,
            "format": value.pointer("/format/name").and_then(Value::as_str),
            "version": value.pointer("/format/version").and_then(Value::as_str),
            "unit": value.pointer("/format/unit").and_then(Value::as_str),
            "profile": value.pointer("/format/profile").and_then(Value::as_str),
        }),
        None,
        pretty,
    )
}

fn validate_ccjs_structure(value: &Value) -> Result<(), String> {
    let root = value
        .as_object()
        .ok_or_else(|| "CCJS root must be an object.".to_string())?;
    let format = root
        .get("format")
        .and_then(Value::as_object)
        .ok_or_else(|| "CCJS requires format metadata.".to_string())?;
    if format.get("name").and_then(Value::as_str) != Some("chemsema") {
        return Err("CCJS format.name must be 'chemsema'.".to_string());
    }
    let version = format
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| "CCJS format.version is required.".to_string())?;
    if !matches!(version, "0.1" | "0.2") {
        return Err(format!("Unsupported CCJS version '{version}'."));
    }
    if version == "0.2" {
        if format.get("unit").and_then(Value::as_str) != Some("pt")
            || format.get("profile").and_then(Value::as_str) != Some("snapshot")
        {
            return Err("CCJS 0.2 requires unit 'pt' and profile 'snapshot'.".to_string());
        }
        if !root
            .get("entities")
            .and_then(|value| value.get("scene"))
            .is_some_and(Value::is_array)
        {
            return Err("CCJS 0.2 requires entities.scene array.".to_string());
        }
        if !root
            .get("hierarchy")
            .and_then(|value| value.get("roots"))
            .is_some_and(Value::is_array)
        {
            return Err("CCJS 0.2 requires hierarchy.roots array.".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structural_validator_accepts_engine_v02() {
        let engine = Engine::new();
        let value: Value = serde_json::from_str(&document_json(&engine).unwrap()).unwrap();
        validate_ccjs_structure(&value).unwrap();
    }

    #[test]
    fn structural_validator_rejects_unknown_version() {
        let value = json!({"format": {"name": "chemsema", "version": "9.9"}});
        assert!(validate_ccjs_structure(&value)
            .unwrap_err()
            .contains("Unsupported"));
    }
}
