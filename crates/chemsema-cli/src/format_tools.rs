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

pub(crate) fn validate_command(args: &[String]) -> CliResult<()> {
    let mut input = None;
    let mut output = None;
    let mut level = ValidationLevel::Semantic;
    let mut target_formats = Vec::new();
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--level" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    CliError::for_command("validate", "--level requires a value.".to_string())
                })?;
                level = ValidationLevel::parse(value)
                    .map_err(|error| CliError::for_command("validate", error))?;
            }
            "--target-format" | "--format" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    CliError::for_command(
                        "validate",
                        "--target-format requires a value.".to_string(),
                    )
                })?;
                for format in value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    target_formats.push(
                        normalize_format(format)
                            .map_err(|error| CliError::for_command("validate", error))?,
                    );
                }
            }
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| {
                            CliError::for_command("validate", "--out requires a path.".to_string())
                        })?
                        .clone(),
                );
            }
            "--pretty" => pretty = true,
            value if input.is_none() => input = Some(value.to_string()),
            value => {
                return Err(CliError::for_command(
                    "validate",
                    format!("Unexpected validate argument '{value}'."),
                ))
            }
        }
        index += 1;
    }
    let input = input.ok_or_else(|| {
        CliError::for_command(
            "validate",
            "validate requires an input document.".to_string(),
        )
    })?;
    let mut service = DesktopDocumentService::default();
    let opened = match service.read_document_file(&input) {
        Ok(opened) => opened,
        Err(message) => {
            return validation_failure(
                validation_report(
                    &input,
                    None,
                    level,
                    Vec::new(),
                    vec![validation_issue(
                        "DOCUMENT_READ_FAILED",
                        "container",
                        "",
                        "CCJZ-v1 section 2; CCJS-v0.2 section 2",
                        "error",
                        "possible",
                        message,
                    )],
                    None,
                ),
                output.as_deref(),
                pretty,
            )
        }
    };
    let mut checks = vec![json!({
        "name": "read",
        "ok": true,
        "format": opened.format,
        "bytes": fs::metadata(&input).map(|value| value.len()).unwrap_or_default(),
    })];

    let mut issues = Vec::new();
    if matches!(opened.format.as_str(), "ccjs" | "ccjz") {
        match serde_json::from_str::<Value>(&opened.text) {
            Ok(value) => match validate_ccjs_structure(&value) {
                Ok(()) => checks.push(json!({"name": "ccjs-structure", "ok": true})),
                Err(message) => issues.push(structural_issue(&message)),
            },
            Err(error) => issues.push(validation_issue(
                "CCJS_JSON_INVALID",
                "structure",
                "",
                "CCJS-v0.2 section 2",
                "error",
                "possible",
                error.to_string(),
            )),
        }
    }

    let mut canonical = None;
    let mut loaded_engine = None;
    if issues.is_empty()
        && (matches!(opened.format.as_str(), "ccjs" | "ccjz")
            || level != ValidationLevel::Structural)
    {
        let engine = match load_engine_from_file(&input) {
            Ok(engine) => engine,
            Err(message) => {
                issues.push(validation_issue(
                    "CCJS_DOCUMENT_INVARIANT",
                    "structure",
                    "",
                    "CCJS-v0.2 sections 3-9",
                    "error",
                    "possible",
                    message,
                ));
                Engine::new()
            }
        };
        let json =
            document_json(&engine).map_err(|error| CliError::for_command("validate", error))?;
        checks.push(json!({
            "name": if level == ValidationLevel::Structural {
                "engine-document-invariants"
            } else {
                "engine-semantics"
            },
            "ok": true
        }));
        canonical = Some(json);
        loaded_engine = Some(engine);
    }

    if level != ValidationLevel::Structural {
        if let Some(engine) = loaded_engine.as_ref() {
            let chemical: Value = serde_json::from_str(
                &engine
                    .chemical_validation_json()
                    .map_err(|error| CliError::for_command("validate", error))?,
            )
            .map_err(|error| CliError::for_command("validate", error.to_string()))?;
            for issue in chemical
                .get("issues")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let kind = issue
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let object_id = issue
                    .get("objectId")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                issues.push(validation_issue(
                    &format!("CHEMICAL_{}", stable_code_part(kind)),
                    "chemical",
                    &format!("entity:{object_id}"),
                    "CCJS-v0.2 section 10",
                    "error",
                    "confirmed",
                    issue
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("Chemical validation failed."),
                ));
            }
            checks.push(json!({
                "name": "chemical-sanitizer",
                "ok": chemical.get("ok").and_then(Value::as_bool).unwrap_or(false),
                "report": chemical,
            }));
        }
    }

    if level == ValidationLevel::RoundTrip && issues.is_empty() {
        let engine = loaded_engine
            .as_ref()
            .expect("roundtrip validation loaded an engine");
        if target_formats.is_empty() {
            target_formats.push(match opened.format.as_str() {
                "ccjz" | "cdxml" | "cdx" | "sdf" => opened.format.clone(),
                _ => "ccjs".to_string(),
            });
        }
        target_formats.sort();
        target_formats.dedup();
        for format in &target_formats {
            match validate_target_roundtrip(engine, format) {
                Ok(check) => checks.push(check),
                Err(issue) => issues.push(issue),
            }
        }
    }

    let fingerprint = canonical.as_deref().unwrap_or(&opened.text);
    let mut digest = Sha256::new();
    digest.update(fingerprint.as_bytes());
    let sha256 = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let report = validation_report(
        &input,
        Some(&opened.format),
        level,
        checks,
        issues,
        Some(sha256),
    );
    if report.get("ok").and_then(Value::as_bool) != Some(true) {
        return validation_failure(report, output.as_deref(), pretty);
    }
    write_json_value(report, output.as_deref(), pretty)
        .map_err(|error| CliError::for_command("validate", error))
}

fn validation_failure(report: Value, output: Option<&str>, pretty: bool) -> CliResult<()> {
    if let Some(path) = output {
        write_json_value(report.clone(), Some(path), pretty)
            .map_err(|error| CliError::for_command("validate", error))?;
    }
    Err(CliError::validation(report))
}

fn validation_report(
    input: &str,
    format: Option<&str>,
    level: ValidationLevel,
    checks: Vec<Value>,
    issues: Vec<Value>,
    canonical_sha256: Option<String>,
) -> Value {
    json!({
        "ok": issues.is_empty(),
        "schema": VALIDATION_SCHEMA,
        "input": input,
        "format": format,
        "level": level.as_str(),
        "canonicalSha256": canonical_sha256,
        "checks": checks,
        "issues": issues,
    })
}

fn validation_issue(
    code: &str,
    stage: &str,
    pointer_or_entry: &str,
    clause: &str,
    severity: &str,
    information_loss: &str,
    message: impl Into<String>,
) -> Value {
    json!({
        "code": code,
        "stage": stage,
        "pointerOrEntry": pointer_or_entry,
        "clause": clause,
        "severity": severity,
        "informationLoss": information_loss,
        "message": message.into(),
    })
}

fn stable_code_part(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn structural_issue(message: &str) -> Value {
    let (code, pointer, clause) = if message.contains("format.name") {
        ("CCJS_FORMAT_NAME", "/format/name", "CCJS-v0.2 section 3")
    } else if message.contains("version") {
        (
            "CCJS_FORMAT_VERSION",
            "/format/version",
            "CCJS-v0.2 section 3",
        )
    } else if message.contains("unit") {
        ("CCJS_FORMAT_UNIT", "/format/unit", "CCJS-v0.2 section 3")
    } else if message.contains("profile") {
        (
            "CCJS_FORMAT_PROFILE",
            "/format/profile",
            "CCJS-v0.2 section 3",
        )
    } else if message.contains("entities.scene") {
        (
            "CCJS_SCENE_REQUIRED",
            "/entities/scene",
            "CCJS-v0.2 section 4",
        )
    } else if message.contains("hierarchy") {
        (
            "CCJS_HIERARCHY_INVALID",
            "/hierarchy",
            "CCJS-v0.2 section 5",
        )
    } else {
        ("CCJS_STRUCTURE_INVALID", "", "CCJS-v0.2 sections 2-9")
    };
    validation_issue(
        code,
        "structure",
        pointer,
        clause,
        "error",
        "possible",
        message,
    )
}

fn validate_target_roundtrip(engine: &Engine, format: &str) -> Result<Value, Value> {
    let include_visual = format != "sdf";
    if format == "sdf" {
        let document: Value = serde_json::from_str(&document_json(engine).map_err(|message| {
            roundtrip_issue("ROUNDTRIP_BASELINE_FAILED", format, "possible", message)
        })?)
        .map_err(|error| {
            roundtrip_issue(
                "ROUNDTRIP_BASELINE_FAILED",
                format,
                "possible",
                error.to_string(),
            )
        })?;
        let has_non_molecule = document
            .pointer("/entities/scene")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .any(|entity| entity.get("type").and_then(Value::as_str) != Some("molecule"));
        let has_relations = document
            .get("relations")
            .and_then(Value::as_array)
            .is_some_and(|relations| !relations.is_empty());
        if has_non_molecule || has_relations {
            return Err(roundtrip_issue(
                "ROUNDTRIP_INFORMATION_LOSS",
                format,
                "confirmed",
                "SDF cannot preserve non-molecule scene entities or document relations.",
            ));
        }
    }
    let before = roundtrip_fingerprint(engine, include_visual).map_err(|message| {
        roundtrip_issue("ROUNDTRIP_BASELINE_FAILED", format, "possible", message)
    })?;
    let mut reopened = Engine::new();
    let result = match format {
        "ccjs" | "json" => {
            document_json(engine).and_then(|json| reopened.load_document_json(&json))
        }
        "ccjz" => document_json(engine)
            .and_then(|json| chemsema_container::encode_ccjz(&json))
            .and_then(|bytes| chemsema_container::decode_ccjz(&bytes))
            .and_then(|json| reopened.load_document_json(&json)),
        "cdxml" => reopened.load_cdxml_document(&engine.document_cdxml()),
        "cdx" => engine
            .document_cdx()
            .and_then(|bytes| reopened.load_cdx_document(&bytes)),
        "sdf" => engine
            .document_sdf()
            .and_then(|text| reopened.load_sdf_document(&text)),
        other => {
            return Err(roundtrip_issue(
                "ROUNDTRIP_FORMAT_UNSUPPORTED",
                other,
                "possible",
                format!("Unsupported roundtrip target format '{other}'."),
            ))
        }
    };
    result.map_err(|message| {
        roundtrip_issue("ROUNDTRIP_IMPORT_FAILED", format, "possible", message)
    })?;
    let after = roundtrip_fingerprint(&reopened, include_visual).map_err(|message| {
        roundtrip_issue("ROUNDTRIP_REVALIDATION_FAILED", format, "possible", message)
    })?;
    if !roundtrip_fingerprints_equivalent(&before, &after) {
        let changed_sections = ["sceneTypes", "relationCount", "molecules", "visual"]
            .into_iter()
            .filter(|key| {
                if *key == "visual" {
                    !visual_multisets_equivalent(before.get(key), after.get(key), 2.0)
                } else {
                    before.get(key) != after.get(key)
                }
            })
            .collect::<Vec<_>>();
        let mut issue = roundtrip_issue(
            "ROUNDTRIP_FINGERPRINT_CHANGED",
            format,
            "confirmed",
            format!(
                "{format} export/import changed fingerprint sections: {}.",
                changed_sections.join(", ")
            ),
        );
        issue["changedSections"] = json!(changed_sections);
        if changed_sections.contains(&"visual") {
            issue["visualDifference"] =
                visual_difference_summary(before.get("visual"), after.get("visual"));
        }
        return Err(issue);
    }
    Ok(json!({
        "name": "target-format-roundtrip",
        "format": format,
        "ok": true,
        "visualCompared": include_visual,
        "fingerprint": before,
    }))
}

fn visual_difference_summary(before: Option<&Value>, after: Option<&Value>) -> Value {
    let (Some(Value::Array(before)), Some(Value::Array(after))) = (before, after) else {
        return json!({"beforeType": before.map(Value::to_string), "afterType": after.map(Value::to_string)});
    };
    let first_unmatched = before.iter().find(|expected| {
        !after
            .iter()
            .any(|actual| json_approximately_equal(expected, actual, 2.0))
    });
    json!({
        "beforePrimitiveCount": before.len(),
        "afterPrimitiveCount": after.len(),
        "firstUnmatchedBefore": first_unmatched,
    })
}

fn roundtrip_fingerprints_equivalent(before: &Value, after: &Value) -> bool {
    ["sceneTypes", "relationCount", "molecules"]
        .into_iter()
        .all(|key| before.get(key) == after.get(key))
        && visual_multisets_equivalent(before.get("visual"), after.get("visual"), 2.0)
}

fn visual_multisets_equivalent(
    before: Option<&Value>,
    after: Option<&Value>,
    epsilon: f64,
) -> bool {
    let (Some(Value::Array(before)), Some(Value::Array(after))) = (before, after) else {
        return before == after;
    };
    if before.len() != after.len() {
        return false;
    }
    let adjacency = before
        .iter()
        .map(|expected| {
            after
                .iter()
                .enumerate()
                .filter_map(|(index, actual)| {
                    json_approximately_equal(expected, actual, epsilon).then_some(index)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut matched_before = vec![None; after.len()];
    (0..before.len()).all(|before_index| {
        let mut seen = vec![false; after.len()];
        augment_visual_match(before_index, &adjacency, &mut matched_before, &mut seen)
    })
}

fn augment_visual_match(
    before_index: usize,
    adjacency: &[Vec<usize>],
    matched_before: &mut [Option<usize>],
    seen: &mut [bool],
) -> bool {
    for &after_index in &adjacency[before_index] {
        if seen[after_index] {
            continue;
        }
        seen[after_index] = true;
        let can_claim = match matched_before[after_index] {
            None => true,
            Some(previous) => augment_visual_match(previous, adjacency, matched_before, seen),
        };
        if can_claim {
            matched_before[after_index] = Some(before_index);
            return true;
        }
    }
    false
}

fn json_approximately_equal(left: &Value, right: &Value, epsilon: f64) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => match (left.as_f64(), right.as_f64()) {
            (Some(left), Some(right)) => (left - right).abs() <= epsilon,
            _ => left == right,
        },
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| json_approximately_equal(left, right, epsilon))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, left)| {
                    right
                        .get(key)
                        .is_some_and(|right| json_approximately_equal(left, right, epsilon))
                })
        }
        _ => left == right,
    }
}

fn roundtrip_issue(
    code: &str,
    format: &str,
    information_loss: &str,
    message: impl Into<String>,
) -> Value {
    validation_issue(
        code,
        "roundtrip",
        &format!("format:{format}"),
        "CCJS-v0.2 section 12",
        "error",
        information_loss,
        message,
    )
}

fn roundtrip_fingerprint(engine: &Engine, include_visual: bool) -> Result<Value, String> {
    let document: Value =
        serde_json::from_str(&document_json(engine)?).map_err(|error| error.to_string())?;
    let chemistry: Value = serde_json::from_str(&engine.chemical_validation_json()?)
        .map_err(|error| error.to_string())?;
    if chemistry.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err("chemical validation failed while computing roundtrip fingerprint".to_string());
    }
    let mut molecules = chemistry
        .get("molecules")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for molecule in &mut molecules {
        if let Some(object) = molecule.as_object_mut() {
            object.remove("objectId");
        }
    }
    molecules.sort_by_key(Value::to_string);
    let mut scene_types = BTreeMap::<String, usize>::new();
    for entity in document
        .pointer("/entities/scene")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        *scene_types
            .entry(
                entity
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
            )
            .or_default() += 1;
    }
    let visual = if include_visual {
        normalize_visual_primitives(
            serde_json::to_value(engine.render_list()).map_err(|error| error.to_string())?,
        )
    } else {
        Value::Null
    };
    Ok(json!({
        "sceneTypes": scene_types,
        "relationCount": document
            .get("relations")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        "molecules": molecules,
        "visual": visual,
    }))
}

fn normalize_visual_primitives(value: Value) -> Value {
    let Value::Array(values) = value else {
        return scrub_identity(value);
    };
    let mut primitives = values.into_iter().map(scrub_identity).collect::<Vec<_>>();
    primitives.sort_by_key(Value::to_string);
    Value::Array(primitives)
}

fn scrub_identity(value: Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.into_iter().map(scrub_identity).collect()),
        Value::Object(values) => {
            let has_points = values.contains_key("points");
            Value::Object(
                values
                    .into_iter()
                    .filter(|(key, _)| {
                        if has_points && key == "d" {
                            return false;
                        }
                        !matches!(
                            key.as_str(),
                            "id" | "objectId" | "nodeId" | "bondId" | "resourceId" | "sourceId"
                        )
                    })
                    .map(|(key, value)| (key, scrub_identity(value)))
                    .collect(),
            )
        }
        Value::Number(number) => number
            .as_f64()
            .and_then(|value| serde_json::Number::from_f64((value * 100.0).round() / 100.0))
            .map(Value::Number)
            .unwrap_or(Value::Number(number)),
        value => value,
    }
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

    #[test]
    fn diagnostics_have_stable_machine_readable_fields() {
        let issue = validation_issue(
            "CCJS_FORMAT_VERSION",
            "structure",
            "/format/version",
            "CCJS-v0.2 section 3",
            "error",
            "possible",
            "unsupported version",
        );
        let report = validation_report(
            "invalid.ccjs",
            Some("ccjs"),
            ValidationLevel::Structural,
            Vec::new(),
            vec![issue],
            None,
        );
        assert_eq!(report["schema"], VALIDATION_SCHEMA);
        for field in [
            "code",
            "stage",
            "pointerOrEntry",
            "clause",
            "severity",
            "informationLoss",
            "message",
        ] {
            assert!(report["issues"][0].get(field).is_some(), "missing {field}");
        }
    }

    #[test]
    fn chemical_validation_rejects_pentavalent_neutral_carbon() {
        let sdf = r#"invalid
  ChemSema

  6  5  0  0  0  0            999 V2000
    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    1.0000    0.0000    0.0000 H   0  0  0  0  0  0  0  0  0  0  0  0
   -1.0000    0.0000    0.0000 H   0  0  0  0  0  0  0  0  0  0  0  0
    0.0000    1.0000    0.0000 H   0  0  0  0  0  0  0  0  0  0  0  0
    0.0000   -1.0000    0.0000 H   0  0  0  0  0  0  0  0  0  0  0  0
    0.0000    0.0000    1.0000 H   0  0  0  0  0  0  0  0  0  0  0  0
  1  2  1  0  0  0  0
  1  3  1  0  0  0  0
  1  4  1  0  0  0  0
  1  5  1  0  0  0  0
  1  6  1  0  0  0  0
M  END
$$$$
"#;
        let mut engine = Engine::new();
        engine
            .load_sdf_document(sdf)
            .expect("invalid SDF still parses");
        let report: Value = serde_json::from_str(&engine.chemical_validation_json().unwrap())
            .expect("chemical report parses");
        assert_eq!(report["ok"], false);
        assert!(report["issues"]
            .as_array()
            .is_some_and(|issues| !issues.is_empty()));
    }

    #[test]
    fn target_roundtrip_gate_covers_all_declared_formats() {
        let mut engine = Engine::new();
        engine
            .execute_command_json(
                r#"{"type":"add-bond","begin":{"x":80.0,"y":80.0},"end":{"x":128.0,"y":80.0},"order":1,"variant":"single"}"#,
            )
            .expect("valid molecule is created");
        for format in ["ccjs", "ccjz", "cdxml", "cdx", "sdf"] {
            let check = validate_target_roundtrip(&engine, format)
                .unwrap_or_else(|issue| panic!("{format} roundtrip failed: {issue}"));
            assert_eq!(check["ok"], true, "{format}");
            assert_eq!(check["format"], format);
        }
    }
}
