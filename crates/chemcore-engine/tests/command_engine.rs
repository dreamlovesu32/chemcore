use chemcore_engine::{Engine, RenderPrimitive};
use serde_json::{json, Value};
use std::collections::BTreeSet;

fn execute(engine: &mut Engine, command: Value) -> Value {
    let result = engine
        .execute_command_json(&command.to_string())
        .expect("command executes");
    serde_json::from_str(&result).expect("command result json")
}

fn document_value(engine: &Engine) -> Value {
    serde_json::from_str(&engine.document_json().expect("document json")).expect("document value")
}

fn created_object_id(result: &Value) -> String {
    result["created"]["objects"][0]
        .as_str()
        .expect("created object id")
        .to_string()
}

fn created_node_id(result: &Value, index: usize) -> String {
    result["created"]["nodes"][index]
        .as_str()
        .expect("created node id")
        .to_string()
}

fn created_bond_id(result: &Value) -> String {
    result["created"]["bonds"][0]
        .as_str()
        .expect("created bond id")
        .to_string()
}

fn find_object(value: &Value, object_id: &str) -> Value {
    fn search(objects: &[Value], object_id: &str) -> Option<Value> {
        for object in objects {
            if object["id"].as_str() == Some(object_id) {
                return Some(object.clone());
            }
            if let Some(found) = object["children"]
                .as_array()
                .and_then(|children| search(children, object_id))
            {
                return Some(found);
            }
        }
        None
    }
    search(
        value["objects"].as_array().expect("objects array"),
        object_id,
    )
    .expect("object by id")
}

fn find_node(value: &Value, node_id: &str) -> Value {
    for resource in value["resources"].as_object().expect("resources").values() {
        if let Some(nodes) = resource["data"]["nodes"].as_array() {
            if let Some(node) = nodes
                .iter()
                .find(|node| node["id"].as_str() == Some(node_id))
            {
                return node.clone();
            }
        }
    }
    panic!("node {node_id} not found");
}

fn node_position(value: &Value, node_id: &str) -> (f64, f64) {
    let node = find_node(value, node_id);
    let position = node["position"].as_array().expect("node position");
    (
        position[0].as_f64().expect("x"),
        position[1].as_f64().expect("y"),
    )
}

fn document_bond_count(value: &Value) -> usize {
    value["resources"]
        .as_object()
        .expect("resources")
        .values()
        .filter_map(|resource| resource["data"]["bonds"].as_array())
        .map(Vec::len)
        .sum()
}

fn rendered_bond_ids(primitives: &[RenderPrimitive]) -> BTreeSet<String> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Line { bond_id, .. }
            | RenderPrimitive::Polygon { bond_id, .. }
            | RenderPrimitive::Polyline { bond_id, .. }
            | RenderPrimitive::Path { bond_id, .. }
            | RenderPrimitive::FilledPath { bond_id, .. } => bond_id.clone(),
            _ => None,
        })
        .collect()
}

fn rendered_node_ids(primitives: &[RenderPrimitive]) -> BTreeSet<String> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Circle { node_id, .. }
            | RenderPrimitive::Polygon { node_id, .. }
            | RenderPrimitive::Rect { node_id, .. }
            | RenderPrimitive::FilledPath { node_id, .. }
            | RenderPrimitive::Text { node_id, .. } => node_id.clone(),
            _ => None,
        })
        .collect()
}

fn crossing_document_value() -> Value {
    json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_crossing",
            "title": "crossing",
            "page": { "width": 140.0, "height": 140.0, "background": "#ffffff" }
        },
        "objects": [{
            "id": "obj_molecule_001",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "payload": { "resourceRef": "mol_001" }
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [20.0, 20.0, 100.0, 100.0],
                    "nodes": [
                        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 60.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n2", "element": "C", "atomicNumber": 6, "position": [100.0, 60.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n3", "element": "C", "atomicNumber": 6, "position": [60.0, 20.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n4", "element": "C", "atomicNumber": 6, "position": [60.0, 100.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b_under", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 },
                        { "id": "b_over", "begin": "n3", "end": "n4", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 }
                    ]
                }
            }
        }
    })
}

fn shared_junction_document_value() -> Value {
    json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_shared_junction",
            "title": "shared junction",
            "page": { "width": 140.0, "height": 140.0, "background": "#ffffff" }
        },
        "objects": [{
            "id": "obj_molecule_001",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "payload": { "resourceRef": "mol_001" }
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [20.0, 20.0, 100.0, 100.0],
                    "nodes": [
                        {
                            "id": "n0",
                            "element": "O",
                            "atomicNumber": 8,
                            "position": [60.0, 60.0],
                            "charge": 0,
                            "numHydrogens": 0,
                            "label": {
                                "text": "O",
                                "position": [60.0, 60.0],
                                "box": [56.0, 54.0, 64.0, 66.0],
                                "align": "center",
                                "anchor": "middle",
                                "fontSize": 12.0,
                                "glyphPolygons": [[[56.0, 54.0], [64.0, 54.0], [64.0, 66.0], [56.0, 66.0]]]
                            }
                        },
                        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 60.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n2", "element": "C", "atomicNumber": 6, "position": [90.0, 95.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n3", "element": "C", "atomicNumber": 6, "position": [110.0, 25.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n4", "element": "C", "atomicNumber": 6, "position": [125.0, 45.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b_dragged", "begin": "n0", "end": "n1", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 },
                        { "id": "b_neighbor", "begin": "n0", "end": "n2", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 },
                        { "id": "b_unrelated", "begin": "n3", "end": "n4", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 }
                    ]
                }
            }
        }
    })
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
fn add_bond_from_existing_atom_marks_existing_endpoint_for_incremental_render() {
    let mut engine = Engine::new();
    let first = execute(
        &mut engine,
        json!({
            "type": "add-bond",
            "begin": { "x": 100.0, "y": 100.0 },
            "end": { "x": 148.0, "y": 100.0 },
            "order": 1,
            "variant": "single"
        }),
    );
    let existing_node_id = created_node_id(&first, 0);
    let existing_bond_id = created_bond_id(&first);

    let second = execute(
        &mut engine,
        json!({
            "type": "add-bond",
            "begin": { "nodeId": existing_node_id, "x": 100.0, "y": 100.0 },
            "end": { "x": 124.0, "y": 141.57 },
            "order": 1,
            "variant": "single"
        }),
    );
    let new_bond_id = created_bond_id(&second);

    assert_eq!(second["changed"], true);
    assert!(
        second["targets"]["nodes"]
            .as_array()
            .expect("target nodes")
            .iter()
            .any(|node| node.as_str() == Some(existing_node_id.as_str())),
        "existing endpoint should be included so desktop incremental render refreshes bond contacts: {second}"
    );

    let target_nodes: BTreeSet<String> = second["targets"]["nodes"]
        .as_array()
        .expect("target nodes")
        .iter()
        .filter_map(|node| node.as_str().map(ToString::to_string))
        .collect();
    let target_bonds: BTreeSet<String> = second["targets"]["bonds"]
        .as_array()
        .expect("target bonds")
        .iter()
        .filter_map(|bond| bond.as_str().map(ToString::to_string))
        .collect();
    let rendered = engine.render_targets(&target_nodes, &target_bonds, &BTreeSet::new());
    let rendered_bonds = rendered_bond_ids(&rendered);
    assert!(
        rendered_bonds.contains(&existing_bond_id) && rendered_bonds.contains(&new_bond_id),
        "desktop partial render should redraw both bonds at the changed junction: {rendered_bonds:?}"
    );
}

#[test]
fn render_targets_for_moved_terminal_node_include_shared_junction_bonds() {
    let mut engine = Engine::new();
    engine
        .load_document_json(&shared_junction_document_value().to_string())
        .expect("document should load");

    let target_nodes = BTreeSet::from(["n1".to_string()]);
    let rendered = engine.render_targets(&target_nodes, &BTreeSet::new(), &BTreeSet::new());
    let rendered_bonds = rendered_bond_ids(&rendered);
    let rendered_nodes = rendered_node_ids(&rendered);

    assert!(
        rendered_bonds.contains("b_dragged") && rendered_bonds.contains("b_neighbor"),
        "moving a terminal atom must refresh all bonds sharing the affected junction: {rendered_bonds:?}"
    );
    assert!(
        !rendered_bonds.contains("b_unrelated"),
        "partial render should stay local to the changed junction: {rendered_bonds:?}"
    );
    assert!(
        rendered_nodes.contains("n0"),
        "partial render should refresh labels at affected bond contact nodes: {rendered_nodes:?}"
    );
}

#[test]
fn direct_text_commands_create_and_update_text() {
    let mut engine = Engine::new();

    let add = execute(
        &mut engine,
        json!({
            "type": "add-text",
            "position": { "x": 120.0, "y": 80.0 },
            "text": "Yield 85%",
            "fontFamily": "Arial",
            "fontSize": 10.0,
            "fill": "#000000"
        }),
    );
    assert_eq!(add["changed"], true);
    assert_eq!(add["command"]["type"], "add-text");
    let object_id = created_object_id(&add);

    let update = execute(
        &mut engine,
        json!({
            "type": "set-text-runs",
            "objectId": object_id,
            "runs": [
                { "text": "H2O", "script": "chemical" }
            ]
        }),
    );
    assert_eq!(update["changed"], true);
    assert_eq!(update["command"]["type"], "set-text-runs");
    assert_eq!(update["updated"]["objects"][0], object_id);

    let document = document_value(&engine);
    let object = find_object(&document, object_id.as_str());
    assert_eq!(object["payload"]["text"], "H2O");
    assert_eq!(object["payload"]["sourceRuns"][0]["script"], "chemical");
}

#[test]
fn direct_node_label_runs_update_endpoint_label() {
    let mut engine = Engine::new();
    let add = execute(
        &mut engine,
        json!({
            "type": "add-bond",
            "begin": { "x": 100.0, "y": 100.0 },
            "end": { "x": 148.0, "y": 100.0 },
            "order": 1,
            "variant": "single"
        }),
    );
    let node_id = created_node_id(&add, 0);

    let update = execute(
        &mut engine,
        json!({
            "type": "set-node-label-runs",
            "nodeId": node_id,
            "runs": [
                { "text": "OMe", "script": "chemical" }
            ]
        }),
    );

    assert_eq!(update["changed"], true);
    assert_eq!(update["command"]["type"], "set-node-label-runs");
    let document = document_value(&engine);
    let node = find_node(&document, &node_id);
    assert_eq!(node["label"]["sourceText"], "OMe");
    assert_eq!(node["label"]["meta"]["sourceRuns"][0]["text"], "OMe");
}

#[test]
fn move_targets_moves_bond_endpoints_by_delta() {
    let mut engine = Engine::new();
    let add = execute(
        &mut engine,
        json!({
            "type": "add-bond",
            "begin": { "x": 100.0, "y": 100.0 },
            "end": { "x": 148.0, "y": 100.0 },
            "order": 1,
            "variant": "single"
        }),
    );
    let first_node_id = created_node_id(&add, 0);
    let second_node_id = created_node_id(&add, 1);
    let bond_id = created_bond_id(&add);

    let moved = execute(
        &mut engine,
        json!({
            "type": "move-targets",
            "targets": { "bonds": [bond_id] },
            "delta": { "dx": 10.0, "dy": -5.0 }
        }),
    );

    assert_eq!(moved["changed"], true);
    assert_eq!(moved["command"]["type"], "move-targets");
    let document = document_value(&engine);
    assert_eq!(node_position(&document, &first_node_id), (110.0, 95.0));
    assert_eq!(node_position(&document, &second_node_id), (158.0, 95.0));
}

#[test]
fn moving_under_crossing_bond_marks_over_bond_for_incremental_render() {
    let mut engine = Engine::new();
    engine
        .load_document_json(&crossing_document_value().to_string())
        .expect("document should load");

    let moved = execute(
        &mut engine,
        json!({
            "type": "move-targets",
            "targets": { "bonds": ["b_under"] },
            "delta": { "dx": 0.0, "dy": 60.0 }
        }),
    );

    let target_bonds: BTreeSet<String> = moved["targets"]["bonds"]
        .as_array()
        .expect("target bonds")
        .iter()
        .filter_map(|bond| bond.as_str().map(ToString::to_string))
        .collect();
    assert!(
        target_bonds.contains("b_under") && target_bonds.contains("b_over"),
        "moving the lower crossing bond must also refresh the upper knockout owner: {moved}"
    );

    let target_nodes: BTreeSet<String> = moved["targets"]["nodes"]
        .as_array()
        .expect("target nodes")
        .iter()
        .filter_map(|node| node.as_str().map(ToString::to_string))
        .collect();
    let rendered = engine.render_targets(&target_nodes, &target_bonds, &BTreeSet::new());
    let rendered_bonds = rendered_bond_ids(&rendered);
    assert!(
        rendered_bonds.contains("b_over"),
        "desktop partial render should return the refreshed upper bond: {rendered_bonds:?}"
    );
}

#[test]
fn rotate_targets_rotates_text_object_by_degrees() {
    let mut engine = Engine::new();
    let add = execute(
        &mut engine,
        json!({
            "type": "add-text",
            "position": { "x": 120.0, "y": 80.0 },
            "text": "rotatable"
        }),
    );
    let object_id = created_object_id(&add);

    let rotated = execute(
        &mut engine,
        json!({
            "type": "rotate-targets",
            "targets": { "objects": [object_id] },
            "center": { "x": 120.0, "y": 80.0 },
            "degrees": 90.0
        }),
    );

    assert_eq!(rotated["changed"], true);
    assert_eq!(rotated["command"]["type"], "rotate-targets");
    let document = document_value(&engine);
    let object = find_object(&document, &object_id);
    assert_eq!(object["transform"]["translate"], json!([120.0, 80.0]));
    assert_eq!(object["transform"]["rotate"], 90.0);
}

#[test]
fn set_arrow_geometry_updates_points_curve_and_endpoint_styles() {
    let mut engine = Engine::new();
    let add = execute(
        &mut engine,
        json!({
            "type": "add-arrow",
            "begin": { "x": 10.0, "y": 20.0 },
            "end": { "x": 50.0, "y": 20.0 },
            "variant": "solid",
            "headSize": "small",
            "curve": "arc270",
            "headStyle": "full",
            "tailStyle": "none",
            "head": true,
            "tail": false,
            "bold": false,
            "noGo": "none"
        }),
    );
    let object_id = created_object_id(&add);

    let update = execute(
        &mut engine,
        json!({
            "type": "set-arrow-geometry",
            "objectId": object_id,
            "begin": { "x": 20.0, "y": 30.0 },
            "end": { "x": 80.0, "y": 30.0 },
            "curve": 90.0,
            "headStyle": "none",
            "tailStyle": "full"
        }),
    );

    assert_eq!(update["changed"], true);
    assert_eq!(update["command"]["type"], "set-arrow-geometry");
    let document = document_value(&engine);
    let object = find_object(&document, &object_id);
    assert_eq!(object["payload"]["points"][0], json!([20.0, 30.0]));
    assert_eq!(object["payload"]["points"][1], json!([80.0, 30.0]));
    assert_eq!(object["payload"]["head"], "none");
    assert_eq!(object["payload"]["tail"], "start");
    assert_eq!(object["payload"]["arrowHead"]["curve"], 90.0);
    assert_eq!(object["payload"]["arrowHead"]["head"], "none");
    assert_eq!(object["payload"]["arrowHead"]["tail"], "full");
}

#[test]
fn set_shape_geometry_updates_rect_bounds() {
    let mut engine = Engine::new();
    let add = execute(
        &mut engine,
        json!({
            "type": "add-shape",
            "kind": "rect",
            "style": "solid",
            "color": "#000000",
            "begin": { "x": 10.0, "y": 10.0 },
            "end": { "x": 20.0, "y": 20.0 }
        }),
    );
    let object_id = created_object_id(&add);

    let update = execute(
        &mut engine,
        json!({
            "type": "set-shape-geometry",
            "objectId": object_id,
            "begin": { "x": 30.0, "y": 35.0 },
            "end": { "x": 70.0, "y": 65.0 }
        }),
    );

    assert_eq!(update["changed"], true);
    assert_eq!(update["command"]["type"], "set-shape-geometry");
    let document = document_value(&engine);
    let object = find_object(&document, &object_id);
    assert_eq!(object["transform"]["translate"], json!([30.0, 35.0]));
    assert_eq!(object["payload"]["bbox"], json!([0.0, 0.0, 40.0, 30.0]));
}

#[test]
fn delete_targets_removes_object_by_id() {
    let mut engine = Engine::new();
    let add = execute(
        &mut engine,
        json!({
            "type": "add-text",
            "position": { "x": 120.0, "y": 80.0 },
            "text": "temporary"
        }),
    );
    let object_id = created_object_id(&add);

    let delete = execute(
        &mut engine,
        json!({
            "type": "delete-targets",
            "targets": { "objects": [object_id] }
        }),
    );

    assert_eq!(delete["changed"], true);
    assert_eq!(delete["command"]["type"], "delete-targets");
    assert_eq!(delete["deleted"]["objects"][0], object_id);
}

#[test]
fn export_document_command_returns_requested_format_payloads() {
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

    let json_export = execute(
        &mut engine,
        json!({
            "type": "export-document",
            "format": "json"
        }),
    );

    assert_eq!(json_export["changed"], false);
    assert_eq!(json_export["command"]["type"], "export-document");
    assert_eq!(json_export["output"]["format"], "json");
    let exported_document: Value =
        serde_json::from_str(json_export["output"]["content"].as_str().unwrap()).unwrap();
    assert_eq!(document_bond_count(&exported_document), 1);

    let svg_export = execute(
        &mut engine,
        json!({
            "type": "export-document",
            "format": "svg"
        }),
    );
    assert_eq!(svg_export["output"]["format"], "svg");
    assert!(svg_export["output"]["content"]
        .as_str()
        .unwrap()
        .contains("<svg"));
}

#[test]
fn load_document_command_imports_cdxml_and_resets_session_history() {
    let mut source = Engine::new();
    execute(
        &mut source,
        json!({
            "type": "add-bond",
            "begin": { "x": 100.0, "y": 100.0 },
            "end": { "x": 148.0, "y": 100.0 },
            "order": 1,
            "variant": "single"
        }),
    );
    let cdxml = source.document_cdxml();

    let mut engine = Engine::new();
    let loaded = execute(
        &mut engine,
        json!({
            "type": "load-document",
            "format": "cdxml",
            "content": cdxml
        }),
    );

    assert_eq!(loaded["changed"], true);
    assert_eq!(loaded["revision"], 0);
    assert_eq!(loaded["canUndo"], false);
    assert_eq!(loaded["command"]["type"], "load-document");
    assert!(loaded["command"].get("content").is_none());
    assert_eq!(loaded["output"]["format"], "cdxml");
    let document = document_value(&engine);
    assert_eq!(document_bond_count(&document), 1);
}

#[test]
fn convert_document_command_returns_output_without_replacing_current_document() {
    let mut source = Engine::new();
    execute(
        &mut source,
        json!({
            "type": "add-bond",
            "begin": { "x": 100.0, "y": 100.0 },
            "end": { "x": 148.0, "y": 100.0 },
            "order": 1,
            "variant": "single"
        }),
    );
    let cdxml = source.document_cdxml();

    let mut engine = Engine::new();
    let converted = execute(
        &mut engine,
        json!({
            "type": "convert-document",
            "from": "cdxml",
            "to": "json",
            "content": cdxml
        }),
    );

    assert_eq!(converted["changed"], false);
    assert_eq!(converted["command"]["type"], "convert-document");
    assert!(converted["command"].get("content").is_none());
    assert_eq!(converted["output"]["format"], "json");
    let converted_document: Value =
        serde_json::from_str(converted["output"]["content"].as_str().unwrap()).unwrap();
    assert_eq!(document_bond_count(&converted_document), 1);
    let current_document = document_value(&engine);
    assert_eq!(document_bond_count(&current_document), 0);
}

#[test]
fn inspect_document_command_returns_agent_friendly_summary() {
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
    execute(
        &mut engine,
        json!({
            "type": "add-text",
            "position": { "x": 120.0, "y": 80.0 },
            "text": "agent note"
        }),
    );

    let inspected = execute(
        &mut engine,
        json!({
            "type": "inspect-document",
            "include": ["summary", "objects", "molecules", "styles"]
        }),
    );

    assert_eq!(inspected["changed"], false);
    assert_eq!(inspected["command"]["type"], "inspect-document");
    assert_eq!(inspected["output"]["summary"]["counts"]["bonds"], 1);
    assert_eq!(
        inspected["output"]["summary"]["counts"]["objectTypes"]["text"],
        1
    );
    assert_eq!(inspected["output"]["molecules"][0]["nodeCount"], 2);
    assert!(inspected["output"]["objects"]
        .as_array()
        .unwrap()
        .iter()
        .any(|object| object["type"] == "text"));
    assert!(inspected["output"]["styles"].as_array().unwrap().len() >= 1);
}

#[test]
fn apply_document_style_command_is_available_to_agents() {
    let mut engine = Engine::new();

    let result = execute(
        &mut engine,
        json!({
            "type": "apply-document-style",
            "preset": "acs-document-1996"
        }),
    );

    assert_eq!(result["changed"], true);
    assert_eq!(result["command"]["type"], "apply-document-style");
    let inspected = execute(
        &mut engine,
        json!({
            "type": "inspect-document",
            "include": ["summary"]
        }),
    );
    assert_eq!(
        inspected["output"]["summary"]["documentStylePreset"],
        "acs-document-1996"
    );
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
