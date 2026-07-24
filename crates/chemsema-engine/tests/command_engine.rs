use chemsema_engine::{Engine, RenderPrimitive};
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

fn find_bond(value: &Value, bond_id: &str) -> Value {
    for resource in value["resources"].as_object().expect("resources").values() {
        if let Some(bonds) = resource["data"]["bonds"].as_array() {
            if let Some(bond) = bonds
                .iter()
                .find(|bond| bond["id"].as_str() == Some(bond_id))
            {
                return bond.clone();
            }
        }
    }
    panic!("bond {bond_id} not found");
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
        "format": { "name": "chemsema", "version": "0.1" },
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
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
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
        "format": { "name": "chemsema", "version": "0.1" },
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
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
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
fn execute_command_json_add_bond_accepts_explicit_double_placement() {
    let mut engine = Engine::new();

    let result = execute(
        &mut engine,
        json!({
            "type": "add-bond",
            "begin": { "x": 100.0, "y": 100.0 },
            "end": { "x": 148.0, "y": 100.0 },
            "order": 2,
            "variant": "double",
            "doublePlacement": "left"
        }),
    );
    let bond_id = created_bond_id(&result);
    let document = document_value(&engine);
    let bond = find_bond(&document, &bond_id);

    assert_eq!(bond["order"], 2);
    assert_eq!(bond["double"]["placement"], "left");
    assert_eq!(bond["double"]["frozen"], true);

    let nested = execute(
        &mut engine,
        json!({
            "type": "add-bond",
            "begin": { "x": 100.0, "y": 140.0 },
            "end": { "x": 148.0, "y": 140.0 },
            "order": 2,
            "variant": "double",
            "double": { "placement": "right" }
        }),
    );
    let nested_bond_id = created_bond_id(&nested);
    let document = document_value(&engine);
    let nested_bond = find_bond(&document, &nested_bond_id);

    assert_eq!(nested_bond["double"]["placement"], "right");
    assert_eq!(nested_bond["double"]["frozen"], true);
}

#[test]
fn execute_command_json_add_bond_accepts_line_weight_override() {
    let mut engine = Engine::new();

    let result = execute(
        &mut engine,
        json!({
            "type": "add-bond",
            "begin": { "x": 100.0, "y": 100.0 },
            "end": { "x": 148.0, "y": 100.0 },
            "order": 2,
            "variant": "double",
            "doublePlacement": "right",
            "lineWeights": { "main": "bold" }
        }),
    );
    let bond_id = created_bond_id(&result);
    let document = document_value(&engine);
    let bond = find_bond(&document, &bond_id);

    assert_eq!(bond["order"], 2);
    assert_eq!(bond["double"]["placement"], "right");
    assert_eq!(bond["double"]["frozen"], true);
    assert_eq!(bond["lineWeights"]["main"], "bold");
    assert_eq!(bond["lineWeights"]["left"], "normal");
    assert_eq!(bond["lineWeights"]["right"], "normal");
}

#[test]
fn execute_command_json_add_bond_accepts_stroke_override() {
    let mut engine = Engine::new();

    let result = execute(
        &mut engine,
        json!({
            "type": "add-bond",
            "begin": { "x": 100.0, "y": 100.0 },
            "end": { "x": 148.0, "y": 100.0 },
            "order": 1,
            "variant": "single",
            "stroke": "#cc3355"
        }),
    );
    let bond_id = created_bond_id(&result);
    let document = document_value(&engine);
    let bond = find_bond(&document, &bond_id);
    assert_eq!(bond["stroke"], "#cc3355");

    execute(&mut engine, json!({ "type": "undo" }));
    execute(&mut engine, json!({ "type": "redo" }));
    let redone = find_bond(&document_value(&engine), &bond_id);
    assert_eq!(redone["stroke"], "#cc3355");
}

#[test]
fn plan_bond_returns_engine_landing_geometry_without_changing_document() {
    let mut engine = Engine::new();

    let result = execute(
        &mut engine,
        json!({
            "type": "plan-bond",
            "begin": { "x": 100.0, "y": 120.0 },
            "angle": 0.0,
            "bondLength": 20.0,
            "order": 1,
            "variant": "single"
        }),
    );

    assert_eq!(result["changed"], false);
    assert_eq!(result["revision"], 0);
    assert_eq!(result["output"]["schema"], "chemsema.plan.bond.v1");
    assert_eq!(result["output"]["angleSource"], "explicit-angle");
    assert_eq!(result["output"]["command"]["type"], "add-bond");
    assert_eq!(result["output"]["command"]["end"]["x"], 120.0);
    assert_eq!(result["output"]["command"]["end"]["y"], 120.0);
    assert!(
        result["output"]["keypadSlots"]
            .as_array()
            .expect("keypad slots")
            .iter()
            .any(|slot| slot["key"] == "5"),
        "plan-bond should expose a default numeric keypad slot"
    );
}

#[test]
fn plan_template_reports_benzene_vertices_and_edges_without_inserting() {
    let mut engine = Engine::new();

    let result = execute(
        &mut engine,
        json!({
            "type": "plan-template",
            "template": "benzene",
            "x": 100.0,
            "y": 100.0,
            "angle": 270.0,
            "bondLength": 20.0
        }),
    );

    let output = &result["output"];
    assert_eq!(result["changed"], false);
    assert_eq!(output["schema"], "chemsema.plan.template.v1");
    assert_eq!(output["anchorKind"], "center");
    assert_eq!(output["vertices"].as_array().expect("vertices").len(), 6);
    assert_eq!(output["edges"].as_array().expect("edges").len(), 6);
    assert_eq!(
        output["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .filter(|edge| edge["order"] == 2)
            .count(),
        3
    );
    assert_eq!(output["insertCommand"]["type"], "insert-template");
}

#[test]
fn insert_template_can_use_endpoint_anchor_and_explicit_angle() {
    let mut engine = Engine::new();
    let first_bond = execute(
        &mut engine,
        json!({
            "type": "add-bond",
            "begin": { "x": 100.0, "y": 100.0 },
            "end": { "x": 120.0, "y": 100.0 },
            "order": 1,
            "variant": "single"
        }),
    );
    let anchor_id = created_node_id(&first_bond, 0);

    let result = execute(
        &mut engine,
        json!({
            "type": "insert-template",
            "template": "benzene",
            "x": 100.0,
            "y": 100.0,
            "anchor": { "nodeId": anchor_id, "x": 100.0, "y": 100.0 },
            "angle": 0.0,
            "bondLength": 20.0
        }),
    );

    assert_eq!(result["changed"], true);
    assert_eq!(document_bond_count(&document_value(&engine)), 7);
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
    let node_id = created_node_id(&add, 1);

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
fn direct_node_label_runs_can_override_source_text_for_visible_order() {
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
    let node_id = created_node_id(&add, 1);

    let update = execute(
        &mut engine,
        json!({
            "type": "set-node-label-runs",
            "nodeId": node_id,
            "text": "HN",
            "runs": [
                { "text": "HN", "script": "normal" }
            ],
            "sourceText": "NH",
            "box": [120.0, 92.0, 144.0, 104.0],
            "anchorOffset": [28.0, 8.0],
            "preserveMeasuredBox": true,
            "defaultChemical": false
        }),
    );

    assert_eq!(update["changed"], true);
    assert_eq!(update["command"]["type"], "set-node-label-runs");
    let document = document_value(&engine);
    let node = find_node(&document, &node_id);
    assert_eq!(node["label"]["text"], "HN");
    assert_eq!(node["label"]["sourceText"], "NH");
    assert_eq!(node["label"]["position"], json!([120.0, 100.2]));
    assert_eq!(node["label"]["box"], json!([120.0, 92.0, 144.0, 104.0]));
    assert_eq!(node["label"]["meta"]["sourceRuns"][0]["text"], "NH");
    assert_eq!(node["label"]["meta"]["sourceRuns"][0]["script"], "normal");
    assert_eq!(
        node["label"]["meta"]["measuredGeometry"]["box"],
        json!([120.0, 92.0, 144.0, 104.0])
    );
    assert_eq!(
        node["label"]["meta"]["measuredGeometry"]["textPosition"],
        json!([120.0, 100.2])
    );
}

#[test]
fn direct_node_label_runs_can_stack_visible_text_above_source_runs() {
    let mut engine = Engine::new();
    let add = execute(
        &mut engine,
        json!({
            "type": "add-bond",
            "begin": { "x": 100.0, "y": 100.0 },
            "end": { "x": 100.0, "y": 148.0 },
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
            "text": "NH",
            "runs": [
                { "text": "NH", "script": "normal" }
            ],
            "sourceText": "NH",
            "displayMode": "stack-above",
            "defaultChemical": false
        }),
    );

    assert_eq!(update["changed"], true);
    let document = document_value(&engine);
    let node = find_node(&document, &node_id);
    let label = &node["label"];
    assert_eq!(label["text"], json!("H\nN"));
    assert_eq!(label["sourceText"], json!("NH"));
    assert!(label.get("runs").is_none());
    assert_eq!(label["lines"], json!(["H", "N"]));
    assert_eq!(label["lineRuns"][0][0]["text"], json!("H"));
    assert_eq!(label["lineRuns"][1][0]["text"], json!("N"));
    assert_eq!(label["meta"]["sourceRuns"][0]["text"], json!("NH"));
    assert_eq!(label["layout"], json!("attached-group-above"));
}

#[test]
fn direct_node_charge_updates_attached_label_valence_semantics() {
    let mut engine = Engine::new();
    let add = execute(
        &mut engine,
        json!({
            "type": "add-bond",
            "begin": { "x": 100.0, "y": 100.0 },
            "end": { "x": 148.0, "y": 100.0 },
            "order": 2,
            "variant": "double",
            "doublePlacement": "center"
        }),
    );
    let node_id = created_node_id(&add, 0);
    execute(
        &mut engine,
        json!({
            "type": "set-node-label-runs",
            "nodeId": node_id,
            "runs": [
                { "text": "NH2", "script": "chemical" }
            ]
        }),
    );

    let update = execute(
        &mut engine,
        json!({
            "type": "set-node-charge",
            "nodeId": node_id,
            "charge": 1
        }),
    );

    assert_eq!(update["changed"], true);
    assert_eq!(update["command"]["type"], "set-node-charge");
    let document = document_value(&engine);
    let node = find_node(&document, &node_id);
    assert_eq!(node["charge"], json!(1));
    assert_eq!(node["label"]["sourceText"], "NH2");
    assert_eq!(node["label"]["text"], "H2N");
}

#[test]
fn direct_node_label_runs_can_preserve_user_edited_hydrogen_source_text() {
    let mut engine = Engine::new();
    let add = execute(
        &mut engine,
        json!({
            "type": "add-bond",
            "begin": { "x": 100.0, "y": 100.0 },
            "end": { "x": 148.0, "y": 100.0 },
            "order": 2,
            "variant": "double",
            "doublePlacement": "center"
        }),
    );
    let node_id = created_node_id(&add, 1);

    let update = execute(
        &mut engine,
        json!({
            "type": "set-node-label-runs",
            "nodeId": node_id,
            "runs": [
                { "text": "NH2", "script": "chemical" }
            ],
            "preserveImplicitHydrogenLabel": true
        }),
    );

    assert_eq!(update["changed"], true);
    assert_eq!(update["command"]["type"], "set-node-label-runs");
    let document = document_value(&engine);
    let node = find_node(&document, &node_id);
    assert_eq!(node["element"], "N");
    assert_eq!(node["charge"], json!(0));
    assert_eq!(node["numHydrogens"], json!(1));
    assert_eq!(node["label"]["sourceText"], "NH2");
    assert_eq!(node["label"]["text"], "NH2");
    assert_eq!(
        node["label"]["meta"]["implicitHydrogenLabel"]["userEdited"],
        json!(true)
    );
}

#[test]
fn interpret_chemically_selection_toggle_updates_label_semantics() {
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
    let node_id = created_node_id(&add, 1);
    execute(
        &mut engine,
        json!({
            "type": "set-node-label-runs",
            "nodeId": node_id,
            "runs": [
                { "text": "NMe4", "script": "chemical" }
            ]
        }),
    );
    execute(
        &mut engine,
        json!({
            "type": "select-targets",
            "targets": { "nodes": [node_id] }
        }),
    );

    let update = execute(
        &mut engine,
        json!({
            "type": "set-interpret-chemically-for-selection",
            "enabled": false
        }),
    );

    assert_eq!(update["changed"], true);
    let document = document_value(&engine);
    let node = find_node(&document, &node_id);
    let label = &node["label"];
    assert_eq!(label["meta"]["defaultChemical"], json!(false));
    assert_eq!(label["meta"]["sourceRuns"][0]["script"], "chemical");
    assert_eq!(label["meta"].get("labelRecognition"), None);
    assert_eq!(node["meta"].get("labelRecognition"), None);
    assert!(
        engine
            .document_cdxml()
            .contains("InterpretChemically=\"no\""),
        "nonchemical labels should export as InterpretChemically=no"
    );
}

#[test]
fn implicit_hydrogen_count_selection_override_can_hide_and_restore_auto_label() {
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
    let node_id = created_node_id(&add, 1);
    execute(
        &mut engine,
        json!({
            "type": "set-node-label-runs",
            "nodeId": node_id,
            "runs": [
                { "text": "N", "script": "chemical" }
            ]
        }),
    );
    execute(
        &mut engine,
        json!({
            "type": "select-targets",
            "targets": { "nodes": [node_id] }
        }),
    );

    let hidden = execute(
        &mut engine,
        json!({
            "type": "set-implicit-hydrogen-count-for-selection",
            "count": 0
        }),
    );

    assert_eq!(hidden["changed"], true);
    let document = document_value(&engine);
    let node = find_node(&document, &node_id);
    assert_eq!(node["numHydrogens"], json!(0));
    assert_eq!(node["meta"]["numHydrogensOverride"], json!(0));
    assert_eq!(node["label"]["text"], "N");
    assert!(
        engine.document_cdxml().contains("NumHydrogens=\"0\""),
        "explicit hidden hydrogens must survive CDXML export"
    );

    let auto = execute(
        &mut engine,
        json!({
            "type": "set-implicit-hydrogen-count-for-selection",
            "count": null
        }),
    );

    assert_eq!(auto["changed"], true);
    let document = document_value(&engine);
    let node = find_node(&document, &node_id);
    assert_eq!(node["numHydrogens"], json!(2));
    assert_eq!(node["meta"].get("numHydrogensOverride"), None);
    assert_eq!(node["label"]["text"], "NH2");
}

#[test]
fn direct_node_label_runs_can_preserve_measured_endpoint_box() {
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
                { "text": "Ph", "script": "normal" }
            ],
            "box": [72.0, 92.0, 96.0, 104.0],
            "anchorOffset": [28.0, 8.0],
            "textPosition": [71.2, 104.0],
            "glyphPolygons": [
                [[72.0, 92.0], [84.0, 92.0], [84.0, 104.0], [72.0, 104.0]],
                [[86.0, 92.0], [96.0, 92.0], [96.0, 104.0], [86.0, 104.0]]
            ],
            "preserveMeasuredBox": true,
            "defaultChemical": true
        }),
    );

    assert_eq!(update["changed"], true);
    let document = document_value(&engine);
    let node = find_node(&document, &node_id);
    assert_eq!(node["label"]["text"], "Ph");
    assert_eq!(node["label"]["position"], json!([71.2, 104.0]));
    assert_eq!(node["label"]["box"], json!([72.0, 92.0, 96.0, 104.0]));
    assert_ne!(
        node["label"]["glyphPolygons"],
        json!([
            [[72.0, 92.0], [84.0, 92.0], [84.0, 104.0], [72.0, 104.0]],
            [[86.0, 92.0], [96.0, 92.0], [96.0, 104.0], [86.0, 104.0]]
        ]),
        "external rectangles must not override kernel glyph geometry"
    );
    assert_eq!(
        node["label"]["meta"]["measuredGeometry"]["box"],
        json!([72.0, 92.0, 96.0, 104.0])
    );
    assert_eq!(
        node["label"]["meta"]["measuredGeometry"]["textPosition"],
        json!([71.2, 104.0])
    );
    assert!(
        node["label"]["meta"].get("import").is_none(),
        "source-neutral command geometry must not be encoded as CDXML import metadata"
    );
    assert!(node["label"]["meta"]
        .get("glyphPolygonsAuthoritative")
        .is_none());
    assert!(node["label"]["meta"]
        .get("ocrGlyphPolygonsAuthoritative")
        .is_none());
    assert_eq!(
        node["label"]["meta"]["measuredTextPositionAuthoritative"],
        json!(true)
    );

    let mut reloaded = Engine::new();
    execute(
        &mut reloaded,
        json!({
            "type": "load-document",
            "format": "json",
            "content": document.to_string()
        }),
    );
    let reloaded_document = document_value(&reloaded);
    let reloaded_node = find_node(&reloaded_document, &node_id);
    assert_eq!(
        reloaded_node["label"]["glyphPolygons"],
        node["label"]["glyphPolygons"]
    );
    assert_eq!(reloaded_node["label"]["position"], json!([71.2, 104.0]));
}

#[test]
fn direct_node_label_runs_preserve_measured_text_position_when_rebuilding_glyphs() {
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
                { "text": "Ph", "script": "normal" }
            ],
            "box": [72.0, 92.0, 96.0, 104.0],
            "anchorOffset": [28.0, 8.0],
            "textPosition": [71.2, 104.0],
            "preserveMeasuredBox": true,
            "defaultChemical": true
        }),
    );

    assert_eq!(update["changed"], true);
    let document = document_value(&engine);
    let node = find_node(&document, &node_id);
    assert_eq!(node["label"]["text"], "Ph");
    assert_eq!(node["label"]["position"], json!([71.2, 104.0]));
    assert_eq!(node["label"]["box"], json!([72.0, 92.0, 96.0, 104.0]));
    assert!(node["label"]["glyphPolygons"]
        .as_array()
        .is_some_and(|polygons| !polygons.is_empty()));
    assert_eq!(
        node["label"]["meta"]["measuredTextPositionAuthoritative"],
        json!(true)
    );
    assert_eq!(
        node["label"]["meta"]["measuredGeometry"]["box"],
        json!([72.0, 92.0, 96.0, 104.0])
    );
    assert_eq!(
        node["label"]["meta"]["measuredGeometry"]["textPosition"],
        json!([71.2, 104.0])
    );
    assert!(
        node["label"]["meta"].get("import").is_none(),
        "source-neutral command geometry must not be encoded as CDXML import metadata"
    );
    assert!(node["label"]["meta"]
        .get("glyphPolygonsAuthoritative")
        .is_none());
    assert!(node["label"]["meta"]
        .get("ocrGlyphPolygonsAuthoritative")
        .is_none());

    let mut reloaded = Engine::new();
    execute(
        &mut reloaded,
        json!({
            "type": "load-document",
            "format": "json",
            "content": document.to_string()
        }),
    );
    let reloaded_document = document_value(&reloaded);
    let reloaded_node = find_node(&reloaded_document, &node_id);
    assert_eq!(reloaded_node["label"]["position"], json!([71.2, 104.0]));
    assert_eq!(
        reloaded_node["label"]["box"],
        json!([72.0, 92.0, 96.0, 104.0])
    );
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
fn scale_targets_stretches_bond_endpoints_by_factor() {
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

    let scaled = execute(
        &mut engine,
        json!({
            "type": "scale-targets",
            "targets": { "bonds": [bond_id] },
            "scaleX": 2.0,
            "scaleY": 1.0,
            "pivot": { "x": 100.0, "y": 100.0 }
        }),
    );

    assert_eq!(scaled["changed"], true);
    assert_eq!(scaled["command"]["type"], "scale-targets");
    let document = document_value(&engine);
    assert_eq!(node_position(&document, &first_node_id), (100.0, 100.0));
    assert_eq!(node_position(&document, &second_node_id), (196.0, 100.0));
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
fn select_targets_enables_selection_arrange_command() {
    let mut engine = Engine::new();
    let first = execute(
        &mut engine,
        json!({
            "type": "add-text",
            "position": { "x": 10.0, "y": 10.0 },
            "text": "A",
            "box": [0.0, 0.0, 10.0, 10.0]
        }),
    );
    let second = execute(
        &mut engine,
        json!({
            "type": "add-text",
            "position": { "x": 40.0, "y": 30.0 },
            "text": "B",
            "box": [0.0, 0.0, 10.0, 10.0]
        }),
    );
    let third = execute(
        &mut engine,
        json!({
            "type": "add-text",
            "position": { "x": 100.0, "y": 50.0 },
            "text": "C",
            "box": [0.0, 0.0, 30.0, 10.0]
        }),
    );
    let object_ids = vec![
        created_object_id(&first),
        created_object_id(&second),
        created_object_id(&third),
    ];

    let selected = execute(
        &mut engine,
        json!({
            "type": "select-targets",
            "targets": { "objects": object_ids }
        }),
    );

    assert_eq!(selected["changed"], false);
    assert_eq!(selected["output"]["selectionChanged"], true);
    assert_eq!(selected["output"]["counts"]["textObjects"], 3);

    let arranged = execute(
        &mut engine,
        json!({
            "type": "apply-selection-arrange",
            "command": "align-left"
        }),
    );

    assert_eq!(arranged["changed"], true);
    assert_eq!(arranged["command"]["type"], "apply-selection-arrange");
    let document = document_value(&engine);
    for object_id in selected["output"]["selection"]["textObjects"]
        .as_array()
        .expect("selected text objects")
        .iter()
        .filter_map(Value::as_str)
    {
        let object = find_object(&document, object_id);
        assert_eq!(object["transform"]["translate"][0], 10.0);
    }
}

#[test]
fn select_targets_can_drive_group_selection_without_ids() {
    let mut engine = Engine::new();
    let first = execute(
        &mut engine,
        json!({
            "type": "add-text",
            "position": { "x": 10.0, "y": 10.0 },
            "text": "A"
        }),
    );
    let second = execute(
        &mut engine,
        json!({
            "type": "add-text",
            "position": { "x": 40.0, "y": 30.0 },
            "text": "B"
        }),
    );
    let object_ids = vec![created_object_id(&first), created_object_id(&second)];
    execute(
        &mut engine,
        json!({
            "type": "select-targets",
            "targets": { "objects": object_ids }
        }),
    );

    let grouped = execute(&mut engine, json!({ "type": "group-selection" }));

    assert_eq!(grouped["changed"], true);
    assert_eq!(grouped["command"]["type"], "group-selection");
    let document = document_value(&engine);
    let groups = document["objects"]
        .as_array()
        .expect("objects")
        .iter()
        .filter(|object| object["type"].as_str() == Some("group"))
        .collect::<Vec<_>>();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["children"].as_array().expect("children").len(), 2);
}

#[test]
fn select_all_and_clear_selection_report_current_selection() {
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
    let text = execute(
        &mut engine,
        json!({
            "type": "add-text",
            "position": { "x": 120.0, "y": 80.0 },
            "text": "colored"
        }),
    );
    let text_id = created_object_id(&text);

    let selected = execute(&mut engine, json!({ "type": "select-all" }));

    assert_eq!(selected["changed"], false);
    assert_eq!(selected["output"]["selectionChanged"], true);
    assert_eq!(selected["output"]["counts"]["textObjects"], 1);
    assert_eq!(selected["output"]["counts"]["moleculeObjects"], 1);
    assert_eq!(selected["output"]["counts"]["nodes"], 2);
    assert_eq!(selected["output"]["counts"]["bonds"], 1);

    let colored = execute(
        &mut engine,
        json!({
            "type": "apply-selection-color",
            "color": "#336699"
        }),
    );

    assert_eq!(colored["changed"], true);
    let document = document_value(&engine);
    let text_object = find_object(&document, &text_id);
    let style_id = text_object["styleRef"].as_str().expect("text style ref");
    assert_eq!(document["styles"][style_id]["fill"], "#336699");

    let cleared = execute(&mut engine, json!({ "type": "clear-selection" }));
    assert_eq!(cleared["changed"], false);
    assert_eq!(cleared["output"]["selectionChanged"], true);
    assert_eq!(cleared["output"]["empty"], true);
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
fn atom_properties_command_round_trips_through_ccjs_cdxml_and_cdx() {
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
    execute(
        &mut engine,
        json!({
            "type": "select-targets",
            "targets": { "nodes": [node_id] }
        }),
    );
    for (property, value) in [
        ("isotope", "13"),
        ("isotopic-abundance", "enriched"),
        ("radical", "doublet"),
        ("atom-number", "42"),
        ("stereo", "R"),
        ("element-list", "NOT 7 8"),
        ("generic-list", "R X"),
        ("free-sites", "2"),
        ("show-atom-query", "true"),
        ("ring-bond-count", "simple-ring"),
        ("unsaturated-bonds", "must-be-present"),
        ("substituents-exactly", "3"),
        ("translation", "narrow"),
        ("abnormal-valence", "true"),
        ("reaction-change", "true"),
        ("reaction-stereo", "inversion"),
        ("show-terminal-carbon-label", "true"),
        ("show-non-terminal-carbon-label", "false"),
    ] {
        let result = execute(
            &mut engine,
            json!({
                "type": "set-atom-property-for-selection",
                "property": property,
                "value": value
            }),
        );
        assert_eq!(result["changed"], true, "{property}");
    }

    let document = document_value(&engine);
    let node = find_node(&document, &node_id);
    assert_eq!(node["atomProperties"]["isotopeMass"], 13);
    assert_eq!(node["atomProperties"]["isotopicAbundance"], "enriched");
    assert_eq!(node["atomProperties"]["radical"], "doublet");
    assert_eq!(node["atomProperties"]["atomNumber"], "42");
    assert_eq!(node["atomProperties"]["showAtomNumber"], true);
    assert_eq!(node["atomProperties"]["cipStereo"], "R");
    assert_eq!(node["atomProperties"]["showAtomStereo"], true);
    assert_eq!(node["atomProperties"]["elementList"], json!([7, 8]));
    assert_eq!(node["atomProperties"]["elementListExcluded"], true);
    assert_eq!(node["atomProperties"]["genericList"], json!(["R", "X"]));
    assert_eq!(node["atomProperties"]["freeSites"], 2);
    assert_eq!(node["atomProperties"]["showAtomQuery"], true);
    assert_eq!(node["atomProperties"]["ringBondCount"], "simple-ring");
    assert_eq!(
        node["atomProperties"]["unsaturatedBonds"],
        "must-be-present"
    );
    assert_eq!(node["atomProperties"]["substituentsExactly"], 3);
    assert_eq!(node["atomProperties"]["translation"], "narrow");
    assert_eq!(node["atomProperties"]["abnormalValence"], true);
    assert_eq!(node["atomProperties"]["reactionChange"], true);
    assert_eq!(node["atomProperties"]["reactionStereo"], "inversion");
    let rendered_text = engine
        .render_list()
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text { text, runs, .. } => {
                if text.is_empty() {
                    Some(runs.iter().map(|run| run.text.as_str()).collect::<String>())
                } else {
                    Some(text.clone())
                }
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    for expected in ["13", "•", "42", "(R)", "X3SRCTLI"] {
        assert!(
            rendered_text.iter().any(|text| text == expected),
            "missing rendered atom decoration {expected}: {rendered_text:?}"
        );
    }

    let cdxml = engine.document_cdxml();
    for attribute in [
        "Isotope=\"13\"",
        "IsotopicAbundance=\"Enriched\"",
        "Radical=\"Doublet\"",
        "AtomNumber=\"42\"",
        "ShowAtomNumber=\"yes\"",
        "ShowAtomStereo=\"yes\"",
        "AS=\"R\"",
        "ElementList=\"NOT 7 8\"",
        "NodeType=\"ElementList\"",
        "GenericList=\"R X\"",
        "FreeSites=\"2\"",
        "ShowAtomQuery=\"yes\"",
        "RingBondCount=\"SimpleRing\"",
        "UnsaturatedBonds=\"MustBePresent\"",
        "SubstituentsExactly=\"3\"",
        "Translation=\"Narrow\"",
        "AbnormalValence=\"yes\"",
        "RxnChange=\"yes\"",
        "RxnStereo=\"Inversion\"",
        "ShowTerminalCarbonLabels=\"yes\"",
        "ShowNonTerminalCarbonLabels=\"no\"",
    ] {
        assert!(cdxml.contains(attribute), "missing {attribute}\n{cdxml}");
    }

    let cdx = engine.document_cdx().expect("CDX export");
    let reopened = chemsema_engine::parse_cdx_document(&cdx, None).expect("CDX import");
    let reopened_json = serde_json::to_value(reopened).expect("document JSON");
    let reopened_node = reopened_json["resources"]
        .as_object()
        .expect("resources")
        .values()
        .find_map(|resource| resource["data"]["nodes"].as_array())
        .and_then(|nodes| nodes.first())
        .cloned()
        .expect("reopened node");
    assert_eq!(reopened_node["atomProperties"]["isotopeMass"], 13);
    assert_eq!(
        reopened_node["atomProperties"]["isotopicAbundance"],
        "enriched"
    );
    assert_eq!(reopened_node["atomProperties"]["radical"], "doublet");
    assert_eq!(reopened_node["atomProperties"]["atomNumber"], "42");
    assert_eq!(reopened_node["atomProperties"]["cipStereo"], "R");
    assert_eq!(
        reopened_node["atomProperties"]["elementList"],
        json!([7, 8])
    );
    assert_eq!(
        reopened_node["atomProperties"]["genericList"],
        json!(["R", "X"])
    );
    assert_eq!(
        reopened_node["atomProperties"]["ringBondCount"],
        "simple-ring"
    );
    assert_eq!(
        reopened_node["atomProperties"]["unsaturatedBonds"],
        "must-be-present"
    );
    assert_eq!(reopened_node["atomProperties"]["translation"], "narrow");
    assert_eq!(reopened_node["atomProperties"]["abnormalValence"], true);
    assert_eq!(reopened_node["atomProperties"]["reactionChange"], true);
    assert_eq!(
        reopened_node["atomProperties"]["reactionStereo"],
        "inversion"
    );
}

#[test]
fn bond_query_reaction_command_round_trips_through_ccjs_cdxml_and_cdx() {
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
    let bond_id = created_bond_id(&add);
    execute(
        &mut engine,
        json!({
            "type": "select-targets",
            "targets": { "bonds": [bond_id] }
        }),
    );
    for (property, value) in [
        ("query-orders", "single-double"),
        ("topology", "ring"),
        ("reaction-participation", "make-and-change"),
        ("absolute-stereo", "e"),
        ("show-query", "true"),
        ("show-reaction", "true"),
        ("show-stereo", "true"),
    ] {
        let result = execute(
            &mut engine,
            json!({
                "type": "set-bond-property-for-selection",
                "property": property,
                "value": value
            }),
        );
        assert_eq!(result["changed"], true, "{property}");
    }

    let bond = find_bond(&document_value(&engine), &bond_id);
    assert_eq!(
        bond["properties"]["queryOrders"],
        json!(["single", "double"])
    );
    assert_eq!(bond["properties"]["topology"], "ring");
    assert_eq!(
        bond["properties"]["reactionParticipation"],
        "make-and-change"
    );
    assert_eq!(bond["properties"]["absoluteStereo"], "e");
    assert_eq!(bond["properties"]["showQuery"], true);
    assert_eq!(bond["properties"]["showReaction"], true);
    assert_eq!(bond["properties"]["showStereo"], true);

    let rendered_text = engine
        .render_list()
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text { text, runs, .. } => Some(if text.is_empty() {
                runs.iter().map(|run| run.text.as_str()).collect()
            } else {
                text.clone()
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        rendered_text.iter().any(|text| text == "RngRxnS/D"),
        "{rendered_text:?}"
    );
    assert!(
        rendered_text.iter().any(|text| text == "(E)"),
        "{rendered_text:?}"
    );

    let cdxml = engine.document_cdxml();
    for attribute in [
        "Order=\"1 2\"",
        "Topology=\"Ring\"",
        "RxnParticipation=\"MakeAndChange\"",
        "BS=\"E\"",
        "ShowBondQuery=\"yes\"",
        "ShowBondRxn=\"yes\"",
        "ShowBondStereo=\"yes\"",
    ] {
        assert!(cdxml.contains(attribute), "missing {attribute}\n{cdxml}");
    }

    let cdx = engine.document_cdx().expect("CDX export");
    let reopened = chemsema_engine::parse_cdx_document(&cdx, None).expect("CDX import");
    let reopened = serde_json::to_value(reopened).expect("CCJS");
    let reopened_bond = reopened["resources"]
        .as_object()
        .expect("resources")
        .values()
        .find_map(|resource| resource["data"]["bonds"].as_array())
        .and_then(|bonds| bonds.first())
        .expect("reopened bond");
    assert_eq!(
        reopened_bond["properties"]["queryOrders"],
        json!(["single", "double"])
    );
    assert_eq!(reopened_bond["properties"]["topology"], "ring");
    assert_eq!(
        reopened_bond["properties"]["reactionParticipation"],
        "make-and-change"
    );
    assert_eq!(reopened_bond["properties"]["absoluteStereo"], "e");
    assert_eq!(reopened_bond["properties"]["showQuery"], true);
    assert_eq!(reopened_bond["properties"]["showReaction"], true);
    assert_eq!(reopened_bond["properties"]["showStereo"], true);
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
    assert!(!inspected["output"]["styles"].as_array().unwrap().is_empty());
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
fn document_json_persists_document_style_defaults_for_later_edits() {
    let mut engine = Engine::new();
    engine.set_document_style_preset("acs-document-1996");

    let saved = engine.document_json().expect("document json");
    let saved_value: serde_json::Value =
        serde_json::from_str(&saved).expect("document should be valid json");
    assert_eq!(saved_value["style"]["preset"], "acs-document-1996");
    assert_eq!(saved_value["style"]["defaults"]["lineWidth"], 0.6);
    assert_eq!(saved_value["style"]["defaults"]["wedgeWidth"], 3.0);

    let mut loaded = Engine::new();
    loaded
        .load_document_json(&saved)
        .expect("styled document should load");
    assert_eq!(loaded.document_style_preset(), "acs-document-1996");

    execute(
        &mut loaded,
        json!({
            "type": "add-bond",
            "begin": { "x": 40.0, "y": 40.0 },
            "end": { "x": 54.4, "y": 40.0 },
            "order": 1,
            "variant": "single"
        }),
    );

    let entry = loaded
        .state()
        .document
        .editable_fragment()
        .expect("fragment should exist");
    let bond = entry.fragment.bonds.last().expect("bond should be added");
    assert_eq!(bond.stroke_width, 0.6);
    assert_eq!(bond.bold_width, Some(2.0));
    assert_eq!(bond.wedge_width, Some(3.0));
    assert_eq!(bond.hash_spacing, Some(2.5));
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
