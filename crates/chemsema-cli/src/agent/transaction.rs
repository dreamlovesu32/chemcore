use super::diff::document_diff;
use super::*;
use crate::protocol::COMMAND_TRANSACTION_SCHEMA_VERSION;

#[derive(Debug)]
pub(crate) struct TransactionExecution {
    pub(crate) ok: bool,
    pub(crate) report: Value,
    pub(crate) error_message: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct TransactionOptions {
    atomic: bool,
    dry_run: bool,
    continue_on_error: bool,
}

pub(crate) fn is_transaction_script(value: &Value) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(COMMAND_TRANSACTION_SCHEMA_VERSION)
        || (value.get("commands").is_some()
            && (value.get("preconditions").is_some()
                || value.get("scope").is_some()
                || value.get("postconditions").is_some()
                || value.get("options").is_some()))
}

pub(crate) fn execute_transaction_script(
    engine: &mut Engine,
    script: &Value,
) -> TransactionExecution {
    let before_hash = crate::document_hash(engine);
    let before_revision = engine.revision();
    let before_document = match engine_document(engine) {
        Ok(document) => document,
        Err(error) => {
            return transaction_error("read-document", error, before_hash, before_revision, engine)
        }
    };
    let commands = match transaction_commands(script) {
        Ok(commands) => commands,
        Err(error) => {
            return transaction_error("read-script", error, before_hash, before_revision, engine)
        }
    };
    let options = match transaction_options(script) {
        Ok(options) => options,
        Err(error) => {
            return transaction_error("read-options", error, before_hash, before_revision, engine)
        }
    };
    let preconditions =
        match validate_preconditions(script, &before_document, &before_hash, before_revision) {
            Ok(value) => value,
            Err((value, error)) => {
                let mut report =
                    base_transaction_report(script, options, before_hash, before_revision, engine);
                set_object_field(&mut report, "preconditions", value);
                set_object_field(
                    &mut report,
                    "execution",
                    empty_execution_report(commands.len(), options.continue_on_error),
                );
                set_object_field(
                    &mut report,
                    "error",
                    json!({ "stage": "preconditions", "message": error }),
                );
                return TransactionExecution {
                    ok: false,
                    report,
                    error_message: Some(error),
                };
            }
        };

    let mut working = engine.clone();
    let execution =
        execute_transaction_commands(&mut working, &commands, options.continue_on_error);
    if execution.failed_count > 0 {
        let error = execution
            .first_error
            .clone()
            .unwrap_or_else(|| "Transaction command failed.".to_string());
        let mut report =
            base_transaction_report(script, options, before_hash, before_revision, engine);
        set_object_field(&mut report, "preconditions", preconditions);
        set_object_field(&mut report, "execution", execution.value);
        set_object_field(
            &mut report,
            "error",
            json!({ "stage": "execute-command", "message": error }),
        );
        return TransactionExecution {
            ok: false,
            report,
            error_message: Some(error),
        };
    }

    let after_document = match engine_document(&working) {
        Ok(document) => document,
        Err(error) => {
            return transaction_error(
                "read-after-document",
                error,
                before_hash,
                before_revision,
                engine,
            )
        }
    };
    let mut diff = match document_diff(&before_document, &after_document) {
        Ok(diff) => diff.value,
        Err(error) => {
            return transaction_error("diff", error, before_hash, before_revision, engine)
        }
    };
    let scope = match validate_scope(script, &before_document, &diff) {
        Ok(scope) => scope,
        Err((scope, error)) => {
            set_object_field(
                &mut diff,
                "unexpectedChanges",
                scope["unexpectedChanges"].clone(),
            );
            let mut report =
                base_transaction_report(script, options, before_hash, before_revision, engine);
            set_object_field(&mut report, "preconditions", preconditions);
            set_object_field(&mut report, "execution", execution.value);
            set_object_field(&mut report, "diff", diff);
            set_object_field(&mut report, "scope", scope);
            set_object_field(
                &mut report,
                "error",
                json!({ "stage": "scope", "message": error }),
            );
            return TransactionExecution {
                ok: false,
                report,
                error_message: Some(error),
            };
        }
    };
    set_object_field(
        &mut diff,
        "unexpectedChanges",
        scope["unexpectedChanges"].clone(),
    );

    let postconditions = match validate_postconditions(script, &after_document, &working, &scope) {
        Ok(value) => value,
        Err((value, error)) => {
            let mut report =
                base_transaction_report(script, options, before_hash, before_revision, engine);
            set_object_field(&mut report, "preconditions", preconditions);
            set_object_field(&mut report, "execution", execution.value);
            set_object_field(&mut report, "diff", diff);
            set_object_field(&mut report, "scope", scope);
            set_object_field(&mut report, "postconditions", value);
            set_object_field(
                &mut report,
                "error",
                json!({ "stage": "postconditions", "message": error }),
            );
            return TransactionExecution {
                ok: false,
                report,
                error_message: Some(error),
            };
        }
    };

    let applied = !options.dry_run;
    if applied {
        *engine = working;
    }
    let mut report = base_transaction_report(
        script,
        options,
        before_hash.clone(),
        before_revision,
        engine,
    );
    set_object_field(&mut report, "preconditions", preconditions);
    set_object_field(&mut report, "execution", execution.value);
    set_object_field(&mut report, "diff", diff);
    set_object_field(&mut report, "scope", scope);
    set_object_field(&mut report, "postconditions", postconditions);
    set_object_field(
        &mut report,
        "transaction",
        json!({
            "schema": COMMAND_TRANSACTION_SCHEMA_VERSION,
            "atomic": options.atomic,
            "dryRun": options.dry_run,
            "applied": applied,
        }),
    );
    set_object_field(
        &mut report,
        "document",
        transaction_document_transition(before_hash, before_revision, engine),
    );
    set_object_field(&mut report, "ok", json!(true));
    TransactionExecution {
        ok: true,
        report,
        error_message: None,
    }
}

fn transaction_error(
    stage: &str,
    error: String,
    before_hash: Option<String>,
    before_revision: u64,
    engine: &Engine,
) -> TransactionExecution {
    let report = json!({
        "ok": false,
        "schema": COMMAND_TRANSACTION_SCHEMA_VERSION,
        "transaction": {
            "schema": COMMAND_TRANSACTION_SCHEMA_VERSION,
            "atomic": true,
            "dryRun": false,
            "applied": false,
        },
        "document": transaction_document_transition(before_hash, before_revision, engine),
        "error": {
            "stage": stage,
            "message": error,
        },
    });
    TransactionExecution {
        ok: false,
        report,
        error_message: Some(error),
    }
}

fn base_transaction_report(
    script: &Value,
    options: TransactionOptions,
    before_hash: Option<String>,
    before_revision: u64,
    engine: &Engine,
) -> Value {
    json!({
        "ok": false,
        "schema": COMMAND_TRANSACTION_SCHEMA_VERSION,
        "transaction": {
            "schema": script.get("schema").and_then(Value::as_str).unwrap_or(COMMAND_TRANSACTION_SCHEMA_VERSION),
            "atomic": options.atomic,
            "dryRun": options.dry_run,
            "applied": false,
        },
        "document": transaction_document_transition(before_hash, before_revision, engine),
    })
}

fn transaction_document_transition(
    before_hash: Option<String>,
    before_revision: u64,
    engine: &Engine,
) -> Value {
    let after_hash = crate::document_hash(engine);
    json!({
        "hashAlgorithm": "sha256",
        "hashInput": "canonical-document-json",
        "beforeHash": before_hash,
        "afterHash": after_hash,
        "hashChanged": match (&before_hash, &after_hash) {
            (Some(before), Some(after)) => Value::Bool(before != after),
            _ => Value::Null,
        },
        "beforeRevision": before_revision,
        "afterRevision": engine.revision(),
    })
}

fn transaction_commands(script: &Value) -> Result<Vec<Value>, String> {
    let Some(commands) = script.get("commands").and_then(Value::as_array) else {
        return Err("Transaction requires a non-empty commands array.".to_string());
    };
    if commands.is_empty() {
        return Err("Transaction commands must not be empty.".to_string());
    }
    Ok(commands.clone())
}

fn transaction_options(script: &Value) -> Result<TransactionOptions, String> {
    let options = script.get("options").unwrap_or(&Value::Null);
    Ok(TransactionOptions {
        atomic: option_bool(options, "atomic")?.unwrap_or(true),
        dry_run: option_bool(options, "dryRun")?
            .or(option_bool(options, "dry-run")?)
            .unwrap_or(false),
        continue_on_error: option_bool(options, "continueOnError")?
            .or(option_bool(options, "continue-on-error")?)
            .unwrap_or(false),
    })
}

fn option_bool(value: &Value, key: &str) -> Result<Option<bool>, String> {
    let Some(value) = value.get(key) else {
        return Ok(None);
    };
    value
        .as_bool()
        .map(Some)
        .ok_or_else(|| format!("options.{key} must be a boolean."))
}

fn validate_preconditions(
    script: &Value,
    document: &ChemSemaDocument,
    current_hash: &Option<String>,
    current_revision: u64,
) -> Result<Value, (Value, String)> {
    let preconditions = script.get("preconditions").unwrap_or(&Value::Null);
    let mut checks = Vec::new();
    let mut failures = Vec::new();
    if let Some(expected) = preconditions
        .get("expectedDocumentHash")
        .or_else(|| preconditions.get("expectedHash"))
        .and_then(Value::as_str)
    {
        let ok = current_hash.as_deref() == Some(expected);
        if !ok {
            failures.push("expectedDocumentHash".to_string());
        }
        checks.push(json!({ "type": "expected-document-hash", "ok": ok, "expected": expected, "actual": current_hash }));
    }
    if let Some(expected) = preconditions
        .get("expectedRevision")
        .and_then(Value::as_u64)
    {
        let ok = current_revision == expected;
        if !ok {
            failures.push("expectedRevision".to_string());
        }
        checks.push(json!({ "type": "expected-revision", "ok": ok, "expected": expected, "actual": current_revision }));
    }
    for selector in string_array(preconditions, "requiredSelectors") {
        let ok = selector_exists(document, &selector);
        if !ok {
            failures.push(selector.clone());
        }
        checks.push(json!({ "type": "selector-exists", "ok": ok, "selector": selector }));
    }
    let value = json!({ "ok": failures.is_empty(), "checks": checks, "failures": failures });
    if failures.is_empty() {
        Ok(value)
    } else {
        Err((value, "Transaction preconditions failed.".to_string()))
    }
}

struct CommandExecution {
    value: Value,
    failed_count: usize,
    first_error: Option<String>,
}

fn execute_transaction_commands(
    engine: &mut Engine,
    commands: &[Value],
    continue_on_error: bool,
) -> CommandExecution {
    let mut results = Vec::new();
    let mut failed_indices = Vec::new();
    let mut executed_count = 0usize;
    let mut first_error = None;
    for (index, command) in commands.iter().enumerate() {
        let before_revision = engine.revision();
        let command_type = command
            .get("type")
            .and_then(Value::as_str)
            .map(Value::from)
            .unwrap_or(Value::Null);
        match engine.execute_command_json(&command.to_string()) {
            Ok(text) => {
                executed_count += 1;
                let result = serde_json::from_str::<Value>(&text).unwrap_or_else(
                    |error| json!({ "ok": false, "error": { "message": error.to_string() } }),
                );
                results.push(json!({
                    "index": index,
                    "ok": true,
                    "executed": true,
                    "changed": result.get("changed").and_then(Value::as_bool).unwrap_or(false),
                    "commandType": command_type,
                    "beforeRevision": before_revision,
                    "afterRevision": engine.revision(),
                    "result": result,
                }));
            }
            Err(error) => {
                failed_indices.push(index);
                if first_error.is_none() {
                    first_error = Some(error.clone());
                }
                results.push(json!({
                    "index": index,
                    "ok": false,
                    "executed": false,
                    "changed": false,
                    "commandType": command_type,
                    "beforeRevision": before_revision,
                    "afterRevision": engine.revision(),
                    "error": { "stage": "execute-command", "message": error },
                }));
                if !continue_on_error {
                    break;
                }
            }
        }
    }
    let failed_count = failed_indices.len();
    CommandExecution {
        value: json!({
            "ok": failed_count == 0,
            "commandCount": commands.len(),
            "executedCount": executed_count,
            "failedCount": failed_count,
            "failedIndex": failed_indices.first().copied(),
            "failedIndices": failed_indices,
            "continueOnError": continue_on_error,
            "commands": results,
        }),
        failed_count,
        first_error,
    }
}

fn empty_execution_report(command_count: usize, continue_on_error: bool) -> Value {
    json!({
        "ok": false,
        "commandCount": command_count,
        "executedCount": 0,
        "failedCount": 0,
        "failedIndex": null,
        "failedIndices": [],
        "continueOnError": continue_on_error,
        "commands": [],
    })
}

fn validate_scope(
    script: &Value,
    before_document: &ChemSemaDocument,
    diff: &Value,
) -> Result<Value, (Value, String)> {
    let scope = script.get("scope").unwrap_or(&Value::Null);
    let editable_targets = string_array(scope, "editableTargets");
    if editable_targets.is_empty() {
        return Ok(json!({
            "ok": true,
            "editableTargets": [],
            "allowedSelectors": [],
            "unexpectedChanges": [],
            "forbidChangesOutsideScope": false,
        }));
    }
    let include_descendants = scope_bool(scope, "includeDescendants", false)?;
    let include_resources = scope_bool(scope, "includeReferencedResources", false)?;
    let allow_create = scope_bool(scope, "allowCreate", false)?;
    let allow_delete = scope_bool(scope, "allowDelete", false)?;
    let forbid_outside = scope_bool(scope, "forbidChangesOutsideScope", true)?;
    let mut allowed = BTreeSet::new();
    for target in &editable_targets {
        collect_allowed_scope(
            before_document,
            target,
            include_descendants,
            include_resources,
            &mut allowed,
        )
        .map_err(|error| {
            (
                json!({ "ok": false, "editableTargets": editable_targets, "error": error }),
                error,
            )
        })?;
    }
    let changes = diff_changed_selectors(diff);
    let mut unexpected = Vec::new();
    for change in changes {
        let action = change["action"].as_str().unwrap_or("");
        let selector = change["selector"].as_str().unwrap_or("");
        let allowed_change = (!forbid_outside || allowed.contains(selector))
            && (allow_create || action != "created")
            && (allow_delete || action != "deleted");
        if !allowed_change {
            unexpected.push(change);
        }
    }
    let value = json!({
        "ok": unexpected.is_empty(),
        "editableTargets": editable_targets,
        "includeDescendants": include_descendants,
        "includeReferencedResources": include_resources,
        "allowCreate": allow_create,
        "allowDelete": allow_delete,
        "forbidChangesOutsideScope": forbid_outside,
        "allowedSelectors": allowed.into_iter().collect::<Vec<_>>(),
        "unexpectedChanges": unexpected,
    });
    if value["ok"].as_bool() == Some(true) {
        Ok(value)
    } else {
        Err((
            value,
            "Transaction changed selectors outside its editable scope.".to_string(),
        ))
    }
}

fn scope_bool(scope: &Value, key: &str, default: bool) -> Result<bool, (Value, String)> {
    match scope.get(key) {
        Some(value) => value.as_bool().ok_or_else(|| {
            (
                json!({ "ok": false }),
                format!("scope.{key} must be a boolean."),
            )
        }),
        None => Ok(default),
    }
}

fn collect_allowed_scope(
    document: &ChemSemaDocument,
    selector: &str,
    include_descendants: bool,
    include_resources: bool,
    allowed: &mut BTreeSet<String>,
) -> Result<(), String> {
    match parse_target_selector(selector)? {
        TargetSelector::Object(id) => {
            let object = find_object(&document.objects, &id)
                .ok_or_else(|| format!("Editable target not found: {selector}."))?;
            collect_object_scope(
                document,
                object,
                include_descendants,
                include_resources,
                allowed,
            );
        }
        TargetSelector::Molecule(index) => {
            let entry = document
                .editable_fragments()
                .into_iter()
                .nth(index)
                .ok_or_else(|| format!("Editable target not found: {selector}."))?;
            allowed.insert(format!("object:{}", entry.object.id));
            if let Some(resource_ref) = entry.object.payload.resource_ref.as_ref() {
                collect_resource_scope(document, resource_ref, include_resources, allowed);
            }
        }
        TargetSelector::Node(id) => {
            let (object, resource_ref) = object_for_node(document, &id)
                .ok_or_else(|| format!("Editable target not found: {selector}."))?;
            allowed.insert(format!("node:{id}"));
            allowed.insert(format!("object:{}", object.id));
            collect_resource_scope(document, &resource_ref, include_resources, allowed);
        }
        TargetSelector::Bond(id) => {
            let (object, resource_ref) = object_for_bond(document, &id)
                .ok_or_else(|| format!("Editable target not found: {selector}."))?;
            allowed.insert(format!("bond:{id}"));
            allowed.insert(format!("object:{}", object.id));
            collect_resource_scope(document, &resource_ref, include_resources, allowed);
        }
        TargetSelector::Selection(targets) => {
            for target in targets {
                collect_allowed_scope(
                    document,
                    &target.selector(),
                    include_descendants,
                    include_resources,
                    allowed,
                )?;
            }
        }
        TargetSelector::All => {
            for object in document.scene_objects() {
                collect_object_scope(document, object, true, true, allowed);
            }
            for id in document.resources.keys() {
                allowed.insert(format!("resource:{id}"));
            }
            for id in document.styles.keys() {
                allowed.insert(format!("style:{id}"));
            }
        }
        TargetSelector::Bounds(_) => {
            return Err("Bounds selectors are visual scope only and cannot be editable transaction targets.".to_string());
        }
    }
    Ok(())
}

fn collect_object_scope(
    document: &ChemSemaDocument,
    object: &SceneObject,
    include_descendants: bool,
    include_resources: bool,
    allowed: &mut BTreeSet<String>,
) {
    allowed.insert(format!("object:{}", object.id));
    if let Some(style_ref) = object.style_ref.as_ref() {
        allowed.insert(format!("style:{style_ref}"));
    }
    if let Some(resource_ref) = object.payload.resource_ref.as_ref() {
        collect_resource_scope(document, resource_ref, include_resources, allowed);
    }
    if include_descendants {
        for child in &object.children {
            collect_object_scope(
                document,
                child,
                include_descendants,
                include_resources,
                allowed,
            );
        }
    }
}

fn collect_resource_scope(
    document: &ChemSemaDocument,
    resource_ref: &str,
    include_entities: bool,
    allowed: &mut BTreeSet<String>,
) {
    allowed.insert(format!("resource:{resource_ref}"));
    if !include_entities {
        return;
    }
    let Some(fragment) = document
        .resources
        .get(resource_ref)
        .and_then(|resource| resource.data.as_fragment())
    else {
        return;
    };
    for node in &fragment.nodes {
        allowed.insert(format!("node:{}", node.id));
    }
    for bond in &fragment.bonds {
        allowed.insert(format!("bond:{}", bond.id));
    }
}

fn find_object<'a>(objects: &'a [SceneObject], id: &str) -> Option<&'a SceneObject> {
    for object in objects {
        if object.id == id {
            return Some(object);
        }
        if let Some(child) = find_object(&object.children, id) {
            return Some(child);
        }
    }
    None
}

fn object_for_node<'a>(
    document: &'a ChemSemaDocument,
    node_id: &str,
) -> Option<(&'a SceneObject, String)> {
    document.editable_fragments().into_iter().find_map(|entry| {
        let resource_ref = entry.object.payload.resource_ref.clone()?;
        entry
            .fragment
            .nodes
            .iter()
            .any(|node| node.id == node_id)
            .then_some((entry.object, resource_ref))
    })
}

fn object_for_bond<'a>(
    document: &'a ChemSemaDocument,
    bond_id: &str,
) -> Option<(&'a SceneObject, String)> {
    document.editable_fragments().into_iter().find_map(|entry| {
        let resource_ref = entry.object.payload.resource_ref.clone()?;
        entry
            .fragment
            .bonds
            .iter()
            .any(|bond| bond.id == bond_id)
            .then_some((entry.object, resource_ref))
    })
}

fn diff_changed_selectors(diff: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    for section in ["objects", "resources", "styles", "nodes", "bonds"] {
        for action in ["created", "updated", "deleted"] {
            let Some(items) = diff
                .get(section)
                .and_then(|section| section.get(action))
                .and_then(Value::as_array)
            else {
                continue;
            };
            for selector in items.iter().filter_map(Value::as_str) {
                out.push(json!({ "selector": selector, "section": section, "action": action }));
            }
        }
    }
    if diff.pointer("/document/updated").and_then(Value::as_bool) == Some(true) {
        out.push(json!({ "selector": "document", "section": "document", "action": "updated" }));
    }
    if diff.pointer("/page/updated").and_then(Value::as_bool) == Some(true) {
        out.push(json!({ "selector": "document:page", "section": "page", "action": "updated" }));
    }
    out.sort_by(|left, right| {
        let left_key = (
            left.get("selector").and_then(Value::as_str).unwrap_or(""),
            left.get("action").and_then(Value::as_str).unwrap_or(""),
        );
        let right_key = (
            right.get("selector").and_then(Value::as_str).unwrap_or(""),
            right.get("action").and_then(Value::as_str).unwrap_or(""),
        );
        left_key.cmp(&right_key)
    });
    out
}

fn validate_postconditions(
    script: &Value,
    after_document: &ChemSemaDocument,
    after_engine: &Engine,
    scope: &Value,
) -> Result<Value, (Value, String)> {
    let mut checks = Vec::new();
    let mut failures = Vec::new();
    let postconditions = script
        .get("postconditions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for postcondition in postconditions {
        let kind = postcondition
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("");
        let ok = match kind {
            "document-valid" => document_valid(after_engine),
            "no-unexpected-changes" => scope
                .get("unexpectedChanges")
                .and_then(Value::as_array)
                .is_none_or(Vec::is_empty),
            "selector-exists" => postcondition
                .get("selector")
                .and_then(Value::as_str)
                .is_some_and(|selector| selector_exists(after_document, selector)),
            _ => false,
        };
        if !ok {
            failures.push(postcondition.clone());
        }
        checks.push(json!({ "type": kind, "ok": ok, "postcondition": postcondition }));
    }
    let value = json!({ "ok": failures.is_empty(), "checks": checks, "failures": failures });
    if failures.is_empty() {
        Ok(value)
    } else {
        Err((value, "Transaction postconditions failed.".to_string()))
    }
}

fn document_valid(engine: &Engine) -> bool {
    crate::document_json(engine)
        .and_then(|text| {
            let mut check = Engine::new();
            check.load_document_json(&text)
        })
        .is_ok()
}

fn selector_exists(document: &ChemSemaDocument, selector: &str) -> bool {
    match parse_target_selector(selector) {
        Ok(TargetSelector::All) => true,
        Ok(TargetSelector::Object(id)) => find_object(&document.objects, &id).is_some(),
        Ok(TargetSelector::Molecule(index)) => document
            .editable_fragments()
            .into_iter()
            .nth(index)
            .is_some(),
        Ok(TargetSelector::Node(id)) => object_for_node(document, &id).is_some(),
        Ok(TargetSelector::Bond(id)) => object_for_bond(document, &id).is_some(),
        Ok(TargetSelector::Selection(targets)) => targets
            .iter()
            .all(|target| selector_exists(document, &target.selector())),
        Ok(TargetSelector::Bounds(_)) => false,
        Err(_) => {
            if let Some(id) = selector.strip_prefix("resource:") {
                document.resources.contains_key(id)
            } else if let Some(id) = selector.strip_prefix("style:") {
                document.styles.contains_key(id)
            } else {
                selector == "document" || selector == "document:page"
            }
        }
    }
}

fn string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn set_object_field(value: &mut Value, key: &str, field: Value) {
    if !value.is_object() {
        *value = json!({});
    }
    if let Some(object) = value.as_object_mut() {
        object.insert(key.to_string(), field);
    }
}
