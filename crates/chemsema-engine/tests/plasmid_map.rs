use chemsema_engine::{Engine, PointerEvent, ShapeKind, Tool};
use serde_json::{json, Value};

fn command(engine: &mut Engine, value: Value) -> Value {
    let result = engine
        .execute_command_json(&value.to_string())
        .expect("command should execute");
    serde_json::from_str(&result).expect("command result JSON")
}

#[test]
fn biodraw_click_creates_native_plasmid_and_requests_kernel_dialog() {
    let mut engine = Engine::new();
    let mut tool = engine.state().tool.clone();
    tool.active_tool = Tool::BioDraw;
    tool.shape_kind = ShapeKind::PlasmidMap;
    engine.set_tool_state(tool);

    engine.pointer_down(PointerEvent {
        x: 140.0,
        y: 160.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 140.0,
        y: 160.0,
        button: Some(0),
        alt_key: false,
    });

    let dialog: Value =
        serde_json::from_str(&engine.take_pending_dialog_json()).expect("dialog JSON");
    assert_eq!(dialog["kind"], "plasmid-map");
    assert_eq!(dialog["mode"], "insert");
    assert_eq!(dialog["data"]["numberBasePairs"], 10_000);
    assert_eq!(dialog["data"]["radius"], 34.0);
    let object_id = dialog["objectId"].as_str().expect("object id").to_string();
    let object = engine
        .state()
        .document
        .find_scene_object(&object_id)
        .expect("native plasmid object");
    let mut data = object.payload.plasmid_map.clone().expect("plasmid data");
    data.number_base_pairs = 12_000;
    command(
        &mut engine,
        json!({
            "type":"set-plasmid-map",
            "objectId":object_id,
            "data":data,
            "finalizeInsert":true
        }),
    );
    assert_eq!(
        engine
            .state()
            .document
            .find_scene_object(&object_id)
            .and_then(|object| object.payload.plasmid_map.as_ref())
            .map(|data| data.number_base_pairs),
        Some(12_000)
    );
    assert!(
        engine.undo(),
        "one undo should remove the finalized insertion"
    );
    assert!(engine
        .state()
        .document
        .find_scene_object(&object_id)
        .is_none());
}

#[test]
fn plasmid_dialog_command_is_validated_and_undoable() {
    let cdxml = include_str!("fixtures/cdxml/plasmid-map.cdxml");
    let mut engine = Engine::new();
    engine.load_cdxml_document(cdxml).expect("fixture loads");
    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.payload.plasmid_map.is_some())
        .expect("plasmid object");
    let object_id = object.id.clone();
    let mut data = object.payload.plasmid_map.clone().expect("plasmid data");
    data.number_base_pairs = 15_000;
    data.show_base_pairs = false;
    data.regions.push(chemsema_engine::PlasmidRegion {
        id: "region_2".to_string(),
        start: 250,
        end: 2_500,
        offset: 10.0,
        arrow_at_start: true,
        arrow_at_end: false,
        filled: true,
        shaded: false,
        faded: false,
        width: 7.0,
        color: "#ff0000".to_string(),
        alpha: 0.7,
    });

    let result = command(
        &mut engine,
        json!({"type":"set-plasmid-map","objectId":object_id,"data":data}),
    );
    assert_eq!(result["changed"], true);
    let updated = engine
        .state()
        .document
        .find_scene_object(&object_id)
        .and_then(|object| object.payload.plasmid_map.as_ref())
        .expect("updated data");
    assert_eq!(updated.number_base_pairs, 15_000);
    assert!(!updated.show_base_pairs);
    assert_eq!(updated.regions.len(), 2);

    assert!(engine.undo());
    let restored = engine
        .state()
        .document
        .find_scene_object(&object_id)
        .and_then(|object| object.payload.plasmid_map.as_ref())
        .expect("restored data");
    assert_eq!(restored.number_base_pairs, 12_000);
    assert!(restored.show_base_pairs);

    let mut invalid = restored.clone();
    invalid.number_base_pairs = 0;
    let error = engine
        .execute_command_json(
            &json!({"type":"set-plasmid-map","objectId":object_id,"data":invalid}).to_string(),
        )
        .expect_err("invalid base-pair domain must fail");
    assert!(error.contains("greater than zero"), "{error}");
}
