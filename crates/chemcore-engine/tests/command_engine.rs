use chemcore_engine::Engine;
use serde_json::{json, Value};

fn execute(engine: &mut Engine, command: Value) -> Value {
    let result = engine
        .execute_command_json(&command.to_string())
        .expect("command executes");
    serde_json::from_str(&result).expect("command result json")
}

#[test]
fn execute_command_json_add_bond_tracks_revision_and_targets() {
    let mut engine = Engine::new();

    let result = execute(
        &mut engine,
        json!({
            "type": "add-bond",
            "begin": { "x": 100.0, "y": 100.0 },
            "end": { "x": 148.0, "y": 100.0 },
            "order": 1,
            "variant": "single"
        }),
    );

    assert_eq!(result["changed"], true);
    assert_eq!(result["beforeRevision"], 0);
    assert_eq!(result["revision"], 1);
    assert_eq!(result["command"]["type"], "add-bond");
    assert_eq!(result["created"]["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(result["created"]["bonds"].as_array().unwrap().len(), 1);
    assert_eq!(result["canUndo"], true);
    assert_eq!(engine.revision(), 1);
}

#[test]
fn undo_and_redo_are_revisioned_commands() {
    let mut engine = Engine::new();
    execute(
        &mut engine,
        json!({
            "type": "add-bond",
            "begin": { "x": 100.0, "y": 100.0 },
            "end": { "x": 148.0, "y": 100.0 },
            "order": 1,
            "variant": "single"
        }),
    );

    let undo = execute(&mut engine, json!({ "type": "undo" }));
    assert_eq!(undo["changed"], true);
    assert_eq!(undo["revision"], 2);
    assert_eq!(undo["command"]["type"], "undo");
    assert_eq!(undo["deleted"]["bonds"].as_array().unwrap().len(), 1);
    assert_eq!(undo["canRedo"], true);

    let redo = execute(&mut engine, json!({ "type": "redo" }));
    assert_eq!(redo["changed"], true);
    assert_eq!(redo["revision"], 3);
    assert_eq!(redo["command"]["type"], "redo");
    assert_eq!(redo["created"]["bonds"].as_array().unwrap().len(), 1);
}

#[test]
fn interaction_only_commands_require_active_context() {
    let mut engine = Engine::new();
    let error = engine
        .execute_command_json(r#"{ "type": "move-selection" }"#)
        .expect_err("move-selection needs drag state");
    assert!(error.contains("active interaction context"));
}
