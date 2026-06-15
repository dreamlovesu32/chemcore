use chemcore_engine::{
    angular_distance, document_to_cdxml, document_to_svg, hit_test_bond_center,
    parse_cdxml_document, parse_document_json, render_document, render_primitives_bounds,
    ChemcoreDocument, Engine, Point, RenderPrimitive, RenderRole, ResourceData,
};
use serde_json::json;
use serde_json::Map;

mod support;
use support::{cdxml_fixture_exists, read_cdxml_fixture, read_optional_cdxml_fixture};

const fn cdxml_cm_to_pt(value: f64) -> f64 {
    value * chemcore_engine::PT_PER_CM
}

const CDXML_EDIT_SCALE: f64 = 1.0;

fn assert_close(left: f64, right: f64) {
    assert!(
        (left - right).abs() < 1e-6,
        "expected {left:.6} to equal {right:.6}"
    );
}

fn assert_point_close(left: Point, right: Point) {
    assert_close(left.x, right.x);
    assert_close(left.y, right.y);
}

fn fragment_document(nodes: serde_json::Value, bonds: serde_json::Value) -> ChemcoreDocument {
    serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": 0.85,
                "fontFamily": "Arial",
                "fontSize": 11.0
            }
        },
        "objects": [{
            "id": "obj_molecule_001",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_001", "bbox": [0.0, 0.0, 80.0, 40.0] }
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 120.0, 120.0],
                    "nodes": nodes,
                    "bonds": bonds
                }
            }
        }
    }))
    .expect("document should deserialize")
}

fn primitive_polygon_bounds(points: &[Point]) -> [f64; 4] {
    points.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |mut bounds, point| {
            bounds[0] = bounds[0].min(point.x);
            bounds[1] = bounds[1].min(point.y);
            bounds[2] = bounds[2].max(point.x);
            bounds[3] = bounds[3].max(point.y);
            bounds
        },
    )
}

fn comma_point_token(token: &str) -> Option<Point> {
    let token = token.trim_end_matches(|ch: char| ch == ',' || ch == ';');
    let (x, y) = token.split_once(',')?;
    Some(Point::new(x.parse().ok()?, y.parse().ok()?))
}

fn horizontal_path_span_at_y(d: &str, y: f64) -> Option<f64> {
    let xs = d
        .split_whitespace()
        .filter_map(comma_point_token)
        .filter(|point| (point.y - y).abs() < 0.01)
        .map(|point| point.x)
        .collect::<Vec<_>>();
    let min = xs.iter().copied().reduce(f64::min)?;
    let max = xs.iter().copied().reduce(f64::max)?;
    Some(max - min)
}

fn render_primitive_object_id(primitive: &RenderPrimitive) -> Option<&str> {
    match primitive {
        RenderPrimitive::Line { object_id, .. }
        | RenderPrimitive::Circle { object_id, .. }
        | RenderPrimitive::Polygon { object_id, .. }
        | RenderPrimitive::Rect { object_id, .. }
        | RenderPrimitive::Ellipse { object_id, .. }
        | RenderPrimitive::Polyline { object_id, .. }
        | RenderPrimitive::Path { object_id, .. }
        | RenderPrimitive::FilledPath { object_id, .. }
        | RenderPrimitive::Text { object_id, .. } => object_id.as_deref(),
    }
}

fn render_primitive_bond_id(primitive: &RenderPrimitive) -> Option<&str> {
    match primitive {
        RenderPrimitive::Line { bond_id, .. }
        | RenderPrimitive::Polygon { bond_id, .. }
        | RenderPrimitive::Polyline { bond_id, .. }
        | RenderPrimitive::Path { bond_id, .. }
        | RenderPrimitive::FilledPath { bond_id, .. } => bond_id.as_deref(),
        _ => None,
    }
}

fn render_primitive_text_content(primitive: &RenderPrimitive) -> Option<String> {
    match primitive {
        RenderPrimitive::Text { text, runs, .. } => {
            if runs.is_empty() {
                Some(text.clone())
            } else {
                Some(runs.iter().map(|run| run.text.as_str()).collect())
            }
        }
        _ => None,
    }
}

fn rects_with_role(engine: &Engine, role_filter: RenderRole) -> Vec<[f64; 4]> {
    engine
        .render_list()
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Rect {
                role,
                x,
                y,
                width,
                height,
                ..
            } if role == role_filter => Some([x, y, x + width, y + height]),
            _ => None,
        })
        .collect()
}

#[test]
fn render_document_adds_margin_knockout_for_later_crossing_bond() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 60.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [100.0, 60.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [60.0, 20.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [60.0, 100.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b_under", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 },
            { "id": "b_over", "begin": "n3", "end": "n4", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 }
        ]),
    );

    let primitives = render_document(&document);
    let knockout_index = primitives
        .iter()
        .position(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentKnockout,
                    ..
                }
            )
        })
        .expect("crossing over-bond should insert a white margin knockout");
    let under_index = primitives
        .iter()
        .position(|primitive| matches!(primitive, RenderPrimitive::Polygon { role: RenderRole::DocumentBond, bond_id, .. } if bond_id.as_deref() == Some("b_under")))
        .expect("under bond should render");
    let over_index = primitives
        .iter()
        .position(|primitive| matches!(primitive, RenderPrimitive::Polygon { role: RenderRole::DocumentBond, bond_id, .. } if bond_id.as_deref() == Some("b_over")))
        .expect("over bond should render");
    assert!(under_index < knockout_index && knockout_index < over_index);

    let RenderPrimitive::Polygon { points, .. } = &primitives[knockout_index] else {
        unreachable!("knockout is a polygon");
    };
    let bounds = primitive_polygon_bounds(points);
    assert!((bounds[0] - 57.5).abs() < 0.001, "{bounds:?}");
    assert!((bounds[1] - 57.5).abs() < 0.001, "{bounds:?}");
    assert!((bounds[2] - 62.5).abs() < 0.001, "{bounds:?}");
    assert!((bounds[3] - 62.5).abs() < 0.001, "{bounds:?}");
}

#[test]
fn render_document_does_not_add_margin_knockout_for_shared_endpoint_bonds() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 60.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [60.0, 60.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [100.0, 60.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b_left", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 },
            { "id": "b_right", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 }
        ]),
    );

    let primitives = render_document(&document);
    assert!(
        !primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentKnockout,
                ..
            }
        )),
        "endpoint contact should stay in the existing contact kernel, not use crossing margin"
    );
}

#[test]
fn cdxml_group_import_preserves_tree_and_z_order() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="20" LineWidth="1" LabelSize="10" CaptionSize="10">
  <page id="1" BoundingBox="0 0 200 160" Width="200" Height="160">
    <group id="10" BoundingBox="10 10 80 50" Z="77">
      <graphic id="11" GraphicType="Rectangle" RectangleType="Plain" BoundingBox="10 10 40 30" Z="12"/>
      <graphic id="12" GraphicType="Rectangle" RectangleType="Plain" BoundingBox="50 20 80 50" Z="13"/>
    </group>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("group")).expect("cdxml should parse");
    let group = document
        .objects
        .iter()
        .find(|object| object.object_type == "group")
        .expect("group should import");
    assert_eq!(group.z_index, 77);
    assert_eq!(group.children.len(), 2);
    assert!(document
        .objects
        .iter()
        .all(|object| object.object_type != "shape"));

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("<group "));
    assert!(exported.contains("Z=\"77\""));
    let reimported =
        parse_cdxml_document(&exported, Some("group export")).expect("group export should parse");
    assert_eq!(
        reimported
            .objects
            .iter()
            .find(|object| object.object_type == "group")
            .map(|object| object.children.len()),
        Some(2)
    );
}

#[test]
fn grouped_scene_objects_render_and_select_as_one_tight_box() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_group",
            "title": "group",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape": {
                "kind": "shape",
                "stroke": "#000000",
                "strokeWidth": 1.0
            }
        },
        "objects": [{
            "id": "grp_1",
            "type": "group",
            "zIndex": 30,
            "children": [
                {
                    "id": "shape_a",
                    "type": "shape",
                    "zIndex": 10,
                    "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "styleRef": "style_shape",
                    "payload": { "bbox": [0.0, 0.0, 20.0, 10.0], "kind": "rect" }
                },
                {
                    "id": "shape_b",
                    "type": "shape",
                    "zIndex": 20,
                    "transform": { "translate": [50.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "styleRef": "style_shape",
                    "payload": { "bbox": [0.0, 0.0, 30.0, 10.0], "kind": "rect" }
                }
            ]
        }],
        "resources": {}
    }))
    .expect("document should deserialize");
    assert_eq!(render_document(&document).len(), 2);

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");
    engine.select_at_point(Point::new(20.0, 15.0), false);
    let selection_boxes: Vec<_> = engine
        .render_list()
        .into_iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Rect {
                    role: RenderRole::SelectionBox,
                    ..
                }
            )
        })
        .collect();
    assert_eq!(selection_boxes.len(), 1);
    match &selection_boxes[0] {
        RenderPrimitive::Rect {
            x,
            y,
            width,
            height,
            ..
        } => {
            assert!((*x - 9.5).abs() < 0.1);
            assert!((*y - 9.5).abs() < 0.1);
            assert!((*width - 71.0).abs() < 0.2);
            assert!((*height - 41.0).abs() < 0.2);
        }
        _ => unreachable!(),
    }
}

#[test]
fn region_selection_collapses_group_box_only_when_all_children_are_selected() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_group_region",
            "title": "group region",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape": {
                "kind": "shape",
                "stroke": "#000000",
                "strokeWidth": 1.0
            }
        },
        "objects": [{
            "id": "grp_1",
            "type": "group",
            "zIndex": 30,
            "children": [
                {
                    "id": "shape_a",
                    "type": "shape",
                    "zIndex": 10,
                    "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "styleRef": "style_shape",
                    "payload": { "bbox": [0.0, 0.0, 20.0, 10.0], "kind": "rect" }
                },
                {
                    "id": "shape_b",
                    "type": "shape",
                    "zIndex": 20,
                    "transform": { "translate": [50.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "styleRef": "style_shape",
                    "payload": { "bbox": [0.0, 0.0, 30.0, 10.0], "kind": "rect" }
                }
            ]
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");
    engine.select_in_rect(Point::new(5.0, 5.0), Point::new(35.0, 25.0), false);
    let partial_boxes = rects_with_role(&engine, RenderRole::SelectionBox);
    assert_eq!(partial_boxes.len(), 1);
    assert!(
        partial_boxes[0][2] - partial_boxes[0][0] < 30.0,
        "partial group selection should show only the child box, got {partial_boxes:?}"
    );

    engine.select_in_rect(Point::new(0.0, 0.0), Point::new(90.0, 60.0), false);
    let complete_boxes = rects_with_role(&engine, RenderRole::SelectionBox);
    assert_eq!(complete_boxes.len(), 1);
    assert!(
        complete_boxes[0][2] - complete_boxes[0][0] > 60.0,
        "complete group selection should collapse to the group box, got {complete_boxes:?}"
    );
}

#[test]
fn select_all_collapses_grouped_molecule_and_text_to_one_group_box() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_group_molecule",
            "title": "group molecule",
            "page": { "width": 220.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": 0.85,
                "fontFamily": "Arial",
                "fontSize": 11.0
            }
        },
        "objects": [{
            "id": "grp_1",
            "type": "group",
            "zIndex": 30,
            "children": [
                {
                    "id": "obj_molecule_001",
                    "type": "molecule",
                    "visible": true,
                    "zIndex": 10,
                    "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "styleRef": "style_molecule_default",
                    "payload": { "resourceRef": "mol_001", "bbox": [0.0, 0.0, 80.0, 40.0] }
                },
                {
                    "id": "group_text",
                    "type": "text",
                    "visible": true,
                    "zIndex": 20,
                    "transform": { "translate": [100.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "payload": { "text": "Note", "bbox": [0.0, 0.0, 24.0, 12.0], "runs": [] }
                }
            ]
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 80.0, 40.0],
                    "nodes": [
                        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [10.0, 20.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n2", "element": "C", "atomicNumber": 6, "position": [60.0, 20.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
                    ]
                }
            }
        }
    }))
    .expect("document should deserialize");

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");
    assert!(engine.select_all());

    let group_boxes = rects_with_role(&engine, RenderRole::SelectionBox);
    assert_eq!(group_boxes.len(), 1);
    assert!(
        group_boxes[0][0] <= 10.0
            && group_boxes[0][1] <= 10.0
            && group_boxes[0][2] >= 124.0
            && group_boxes[0][3] >= 50.0,
        "group box should cover both molecule and text, got {group_boxes:?}"
    );
    assert!(rects_with_role(&engine, RenderRole::SelectionTextBox).is_empty());
    assert!(rects_with_role(&engine, RenderRole::SelectionBond).is_empty());
    assert!(rects_with_role(&engine, RenderRole::SelectionNode).is_empty());
    let bond_dot_count = engine
        .render_list()
        .into_iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Circle {
                    role: RenderRole::SelectionBondDot,
                    ..
                }
            )
        })
        .count();
    assert_eq!(bond_dot_count, 0);
}

#[test]
fn moving_selected_grouped_molecule_does_not_move_nodes_twice() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_group_molecule_move",
            "title": "group molecule move",
            "page": { "width": 220.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": 0.85,
                "fontFamily": "Arial",
                "fontSize": 11.0
            }
        },
        "objects": [{
            "id": "grp_1",
            "type": "group",
            "zIndex": 30,
            "children": [{
                "id": "obj_molecule_001",
                "type": "molecule",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_molecule_default",
                "payload": { "resourceRef": "mol_001", "bbox": [0.0, 0.0, 80.0, 40.0] }
            }]
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 80.0, 40.0],
                    "nodes": [
                        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [10.0, 20.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n2", "element": "C", "atomicNumber": 6, "position": [60.0, 20.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
                    ]
                }
            }
        }
    }))
    .expect("document should deserialize");

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");
    assert!(engine.select_all());
    assert!(engine.begin_selection_move_at_point(Point::new(20.0, 30.0), false, false));
    assert!(engine.update_selection_move(Point::new(30.0, 30.0), false));
    assert!(engine.finish_selection_move(Point::new(30.0, 30.0), false));

    let molecule = engine
        .state()
        .document
        .find_scene_object("obj_molecule_001")
        .expect("grouped molecule should still exist");
    assert_eq!(molecule.transform.translate, [20.0, 10.0]);
    let fragment = engine
        .state()
        .document
        .resources
        .get("mol_001")
        .and_then(|resource| resource.data.as_fragment())
        .expect("fragment should still exist");
    assert_eq!(fragment.nodes[0].position, [10.0, 20.0]);
    assert_eq!(fragment.nodes[1].position, [60.0, 20.0]);
}

#[test]
fn engine_groups_and_ungroups_selected_scene_objects_without_geometry_drift() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_group_edit",
            "title": "group edit",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape": {
                "kind": "shape",
                "stroke": "#000000",
                "strokeWidth": 1.0
            }
        },
        "objects": [
            {
                "id": "shape_a",
                "type": "shape",
                "zIndex": 10,
                "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_shape",
                "payload": { "bbox": [0.0, 0.0, 20.0, 10.0], "kind": "rect" }
            },
            {
                "id": "shape_b",
                "type": "shape",
                "zIndex": 20,
                "transform": { "translate": [50.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_shape",
                "payload": { "bbox": [0.0, 0.0, 30.0, 10.0], "kind": "rect" }
            }
        ],
        "resources": {}
    }))
    .expect("document should deserialize");
    let before = render_primitives_bounds(render_document(&document).iter()).unwrap();

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");
    engine.select_in_rect(Point::new(0.0, 0.0), Point::new(90.0, 60.0), false);
    assert!(engine.group_selection());
    let group = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "group")
        .expect("group should be created");
    assert_eq!(group.children.len(), 2);
    assert_eq!(group.z_index, 20);
    let grouped_bounds = render_primitives_bounds(render_document(&engine.state().document).iter())
        .expect("grouped document should render");
    assert_eq!(before, grouped_bounds);

    assert!(engine.ungroup_selection());
    assert!(engine
        .state()
        .document
        .objects
        .iter()
        .all(|object| object.object_type != "group"));
    let ungrouped_bounds =
        render_primitives_bounds(render_document(&engine.state().document).iter())
            .expect("ungrouped document should render");
    assert_eq!(before, ungrouped_bounds);
}

#[test]
fn context_hit_test_reports_object_without_mutating_selection() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_context_hit",
            "title": "context hit",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape": {
                "kind": "shape",
                "stroke": "#000000",
                "strokeWidth": 1.0
            }
        },
        "objects": [{
            "id": "shape_a",
            "type": "shape",
            "zIndex": 10,
            "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_shape",
            "payload": { "bbox": [0.0, 0.0, 20.0, 10.0], "kind": "rect" }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");
    let hit: serde_json::Value =
        serde_json::from_str(&engine.context_hit_test_json(Point::new(15.0, 15.0))).unwrap();
    assert_eq!(hit["kind"], "object");
    assert_eq!(hit["objectId"], "shape_a");
    assert_eq!(hit["objectType"], "shape");
    assert_eq!(hit["selected"], false);
    assert!(engine.state().selection.is_empty());

    engine.select_at_point(Point::new(15.0, 15.0), false);
    let selected_hit: serde_json::Value =
        serde_json::from_str(&engine.context_hit_test_json(Point::new(15.0, 15.0))).unwrap();
    assert_eq!(selected_hit["selected"], true);

    let canvas_hit: serde_json::Value =
        serde_json::from_str(&engine.context_hit_test_json(Point::new(150.0, 120.0))).unwrap();
    assert_eq!(canvas_hit["kind"], "canvas");
}

#[test]
fn complete_molecule_selection_suppresses_internal_selection_dots() {
    let mut engine = Engine::new();
    engine
        .load_document_json(
            &serde_json::to_string(&fragment_document(
                json!([
                    { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 20.0], "charge": 0, "numHydrogens": 0 },
                    { "id": "n2", "element": "C", "atomicNumber": 6, "position": [50.0, 20.0], "charge": 0, "numHydrogens": 0 }
                ]),
                json!([
                    { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
                ]),
            ))
            .unwrap(),
        )
        .expect("document should load");
    assert!(engine.select_component_at_point(Point::new(20.0, 20.0), false));
    let dot_count = engine
        .render_list()
        .into_iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Circle {
                    role: RenderRole::SelectionBondDot,
                    ..
                }
            )
        })
        .count();
    assert_eq!(dot_count, 0);
}

#[test]
fn select_all_selects_document_surface_without_expanding_groups() {
    let document = json!({
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_select_all",
            "title": "select all",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape": {
                "kind": "shape",
                "stroke": "#000000",
                "strokeWidth": 1.0
            }
        },
        "objects": [
            {
                "id": "obj_molecule_001",
                "type": "molecule",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "resourceRef": "mol_001" }
            },
            {
                "id": "shape_1",
                "type": "shape",
                "visible": true,
                "zIndex": 20,
                "transform": { "translate": [80.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_shape",
                "payload": { "bbox": [0.0, 0.0, 20.0, 10.0], "kind": "rect" }
            },
            {
                "id": "text_1",
                "type": "text",
                "visible": true,
                "zIndex": 30,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "text": "Note", "bbox": [20.0, 70.0, 60.0, 90.0], "runs": [] }
            },
            {
                "id": "group_1",
                "type": "group",
                "visible": true,
                "zIndex": 40,
                "children": [
                    {
                        "id": "group_child_shape",
                        "type": "shape",
                        "visible": true,
                        "zIndex": 41,
                        "transform": { "translate": [110.0, 70.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                        "styleRef": "style_shape",
                        "payload": { "bbox": [0.0, 0.0, 20.0, 10.0], "kind": "rect" }
                    },
                    {
                        "id": "group_child_text",
                        "type": "text",
                        "visible": true,
                        "zIndex": 42,
                        "transform": { "translate": [138.0, 68.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                        "payload": { "text": "cond", "bbox": [0.0, 0.0, 26.0, 12.0], "runs": [] }
                    }
                ]
            }
        ],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 60.0, 40.0],
                    "nodes": [
                        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [10.0, 10.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n2", "element": "C", "atomicNumber": 6, "position": [40.0, 10.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
                    ]
                }
            }
        }
    });
    let mut engine = Engine::new();
    engine
        .load_document_json(&document.to_string())
        .expect("document should load");

    assert!(engine.select_all());
    let selection = &engine.state().selection;
    assert_eq!(selection.nodes, vec!["n1".to_string(), "n2".to_string()]);
    assert_eq!(selection.bonds, vec!["b1".to_string()]);
    assert_eq!(selection.text_objects, vec!["text_1".to_string()]);
    assert_eq!(
        selection.arrow_objects,
        vec!["shape_1".to_string(), "group_1".to_string()]
    );
    assert!(!selection
        .arrow_objects
        .contains(&"group_child_shape".to_string()));

    let selection_boxes: Vec<_> = engine
        .render_list()
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Rect {
                role: RenderRole::SelectionBox,
                x,
                y,
                width,
                height,
                ..
            } => Some((x, y, width, height)),
            _ => None,
        })
        .collect();
    assert!(selection_boxes.iter().any(|(x, y, width, height)| {
        *x <= 110.0
            && *x >= 100.0
            && *y <= 68.0
            && *y >= 60.0
            && (*x + *width) >= 164.0
            && (*y + *height) >= 80.0
    }));

    let internal_dot_count = engine
        .render_list()
        .into_iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Circle {
                    role: RenderRole::SelectionBondDot,
                    ..
                }
            )
        })
        .count();
    assert_eq!(internal_dot_count, 0);
    assert!(!engine.select_all());
}

#[test]
fn engine_reports_document_colors_from_document_model() {
    let mut engine = Engine::new();
    engine
        .load_document_json(
            &json!({
                "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
                "document": {
                    "id": "doc_colors",
                    "title": "colors",
                    "page": { "width": 200.0, "height": 160.0, "background": "#fff" }
                },
                "styles": {
                    "style_molecule_default": {
                        "kind": "molecule",
                        "stroke": "rgb(1, 2, 3)",
                        "fill": "#abc",
                        "strokeWidth": 0.85,
                        "fontSize": 11.0
                    }
                },
                "objects": [{
                    "id": "obj_molecule",
                    "type": "molecule",
                    "styleRef": "style_molecule_default",
                    "payload": { "resourceRef": "mol" }
                }],
                "resources": {
                    "mol": {
                        "type": "molecule_fragment2d",
                        "encoding": "chemcore.molecule.fragment2d",
                        "data": {
                            "schema": "chemcore.molecule.fragment2d",
                            "bbox": [0.0, 0.0, 200.0, 160.0],
                            "nodes": [{
                                "id": "n1",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [20.0, 20.0],
                                "charge": 0,
                                "numHydrogens": 0,
                                "label": { "text": "Me", "fill": "#00ff00" }
                            }, {
                                "id": "n2",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [50.0, 20.0],
                                "charge": 0,
                                "numHydrogens": 0
                            }],
                            "bonds": [{
                                "id": "b1",
                                "begin": "n1",
                                "end": "n2",
                                "order": 1,
                                "stroke": "#ff00ff",
                                "strokeWidth": 0.85
                            }]
                        }
                    }
                }
            })
            .to_string(),
        )
        .unwrap();

    let colors = engine.document_colors();
    assert!(colors.contains(&"#ffffff".to_string()));
    assert!(colors.contains(&"#010203".to_string()));
    assert!(colors.contains(&"#aabbcc".to_string()));
    assert!(colors.contains(&"#00ff00".to_string()));
    assert!(colors.contains(&"#ff00ff".to_string()));
    assert_eq!(colors.iter().filter(|color| *color == "#ffffff").count(), 1);
}

fn polygon_area(points: &[chemcore_engine::Point]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        area += points[index].x * points[next].y - points[next].x * points[index].y;
    }
    area.abs() * 0.5
}

fn centered_bond_polygons(
    primitives: &[RenderPrimitive],
    center: chemcore_engine::Point,
) -> Vec<Vec<chemcore_engine::Point>> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001")
                && points.iter().any(|point| point.distance(center) <= 4.0) =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .collect()
}

fn object_bond_polygons(primitives: &[RenderPrimitive]) -> Vec<Vec<chemcore_engine::Point>> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .collect()
}

fn object_knockout_polygons(primitives: &[RenderPrimitive]) -> Vec<Vec<chemcore_engine::Point>> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentKnockout
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .collect()
}

fn document_bond_polygon_count_for_object(
    primitives: &[RenderPrimitive],
    object_id: &str,
) -> usize {
    primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role,
                    object_id: primitive_object_id,
                    ..
                } | RenderPrimitive::FilledPath {
                    role,
                    object_id: primitive_object_id,
                    ..
                } if *role == RenderRole::DocumentBond
                    && primitive_object_id.as_deref() == Some(object_id)
            )
        })
        .count()
}

fn document_knockout_count_for_object(primitives: &[RenderPrimitive], object_id: &str) -> usize {
    primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role,
                    object_id: primitive_object_id,
                    ..
                } if *role == RenderRole::DocumentKnockout
                    && primitive_object_id.as_deref() == Some(object_id)
            )
        })
        .count()
}

fn document_knockout_axis_intervals_for_object(
    primitives: &[RenderPrimitive],
    object_id: &str,
) -> Vec<(f64, f64)> {
    let (axis_from, axis_to, bond_start) = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id: primitive_object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && primitive_object_id.as_deref() == Some(object_id) =>
            {
                let (axis_from, axis_to) = bond_axis_from_points(points)?;
                let (bond_start, _) = projection_range_on_axis(points, axis_from, axis_to)?;
                Some((axis_from, axis_to, bond_start))
            }
            _ => None,
        })
        .expect("object should have a bond polygon");
    let mut intervals: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id: primitive_object_id,
                points,
                ..
            } if *role == RenderRole::DocumentKnockout
                && primitive_object_id.as_deref() == Some(object_id) =>
            {
                let (start, end) = projection_range_on_axis(points, axis_from, axis_to)?;
                Some((start - bond_start, end - bond_start))
            }
            _ => None,
        })
        .collect();
    intervals.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
    intervals
}

fn render_roundtrip_signature(document: &ChemcoreDocument) -> Vec<String> {
    let primitives: Vec<_> = render_document(document)
        .into_iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Line {
                    role: RenderRole::DocumentBond | RenderRole::DocumentGraphic,
                    ..
                } | RenderPrimitive::Circle {
                    role: RenderRole::DocumentGraphic | RenderRole::DocumentText,
                    ..
                } | RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond
                        | RenderRole::DocumentGraphic
                        | RenderRole::DocumentKnockout,
                    ..
                } | RenderPrimitive::Rect {
                    role: RenderRole::DocumentGraphic | RenderRole::DocumentText,
                    ..
                } | RenderPrimitive::Ellipse {
                    role: RenderRole::DocumentGraphic,
                    ..
                } | RenderPrimitive::Polyline {
                    role: RenderRole::DocumentGraphic | RenderRole::DocumentBond,
                    ..
                } | RenderPrimitive::Path {
                    role: RenderRole::DocumentGraphic | RenderRole::DocumentBond,
                    ..
                } | RenderPrimitive::FilledPath {
                    role: RenderRole::DocumentGraphic,
                    ..
                } | RenderPrimitive::Text {
                    role: RenderRole::DocumentText,
                    ..
                }
            )
        })
        .collect();
    let (offset_x, offset_y) = render_signature_origin(&primitives);
    let mut signature: Vec<String> = primitives
        .iter()
        .map(|primitive| primitive_signature(primitive, offset_x, offset_y))
        .collect();
    signature.sort();
    signature
}

fn render_signature_origin(primitives: &[RenderPrimitive]) -> (f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    for primitive in primitives {
        for point in primitive_points(primitive) {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
        }
    }
    if min_x.is_finite() && min_y.is_finite() {
        (min_x, min_y)
    } else {
        (0.0, 0.0)
    }
}

fn primitive_points(primitive: &RenderPrimitive) -> Vec<Point> {
    match primitive {
        RenderPrimitive::Line { from, to, .. } => vec![*from, *to],
        RenderPrimitive::Circle { center, radius, .. } => vec![
            Point::new(center.x - radius, center.y - radius),
            Point::new(center.x + radius, center.y + radius),
        ],
        RenderPrimitive::Polygon { points, .. }
        | RenderPrimitive::Polyline { points, .. }
        | RenderPrimitive::Path { points, .. }
        | RenderPrimitive::FilledPath { points, .. } => points.clone(),
        RenderPrimitive::Rect {
            x,
            y,
            width,
            height,
            ..
        } => vec![Point::new(*x, *y), Point::new(*x + *width, *y + *height)],
        RenderPrimitive::Ellipse { center, rx, ry, .. } => vec![
            Point::new(center.x - rx, center.y - ry),
            Point::new(center.x + rx, center.y + ry),
        ],
        RenderPrimitive::Text { x, y, .. } => vec![Point::new(*x, *y)],
    }
}

fn primitive_signature(primitive: &RenderPrimitive, offset_x: f64, offset_y: f64) -> String {
    match primitive {
        RenderPrimitive::Line {
            role,
            from,
            to,
            stroke,
            stroke_width,
            dash_array,
            ..
        } => format!(
            "line:{role:?}:{}:{}:{stroke}:{}:{:?}",
            point_sig(*from, offset_x, offset_y),
            point_sig(*to, offset_x, offset_y),
            num_sig(*stroke_width),
            nums_sig(dash_array),
        ),
        RenderPrimitive::Circle {
            role,
            center,
            radius,
            fill,
            stroke,
            stroke_width,
            ..
        } => format!(
            "circle:{role:?}:{}:{}:{fill}:{stroke}:{}",
            point_sig(*center, offset_x, offset_y),
            num_sig(*radius),
            num_sig(*stroke_width),
        ),
        RenderPrimitive::Polygon {
            role,
            points,
            fill,
            stroke,
            stroke_width,
            ..
        } => format!(
            "polygon:{role:?}:{}:{fill}:{stroke}:{}",
            points_sig(points, offset_x, offset_y),
            num_sig(*stroke_width),
        ),
        RenderPrimitive::Rect {
            role,
            x,
            y,
            width,
            height,
            fill,
            stroke,
            stroke_width,
            rx,
            ry,
            dash_array,
            ..
        } => format!(
            "rect:{role:?}:{}:{}:{}:{}:{fill:?}:{stroke:?}:{}:{rx:?}:{ry:?}:{:?}",
            num_sig(*x - offset_x),
            num_sig(*y - offset_y),
            num_sig(*width),
            num_sig(*height),
            num_sig(*stroke_width),
            nums_sig(dash_array),
        ),
        RenderPrimitive::Ellipse {
            role,
            center,
            rx,
            ry,
            rotate,
            fill,
            stroke,
            stroke_width,
            dash_array,
            ..
        } => format!(
            "ellipse:{role:?}:{}:{}:{}:{}:{fill:?}:{stroke:?}:{}:{:?}",
            point_sig(*center, offset_x, offset_y),
            num_sig(*rx),
            num_sig(*ry),
            num_sig(*rotate),
            num_sig(*stroke_width),
            nums_sig(dash_array),
        ),
        RenderPrimitive::Polyline {
            role,
            points,
            stroke,
            stroke_width,
            dash_array,
            line_cap,
            line_join,
            ..
        } => format!(
            "polyline:{role:?}:{}:{stroke}:{}:{:?}:{line_cap:?}:{line_join:?}",
            points_sig(points, offset_x, offset_y),
            num_sig(*stroke_width),
            nums_sig(dash_array),
        ),
        RenderPrimitive::Path {
            role,
            d,
            points,
            stroke,
            stroke_width,
            dash_array,
            line_cap,
            line_join,
            ..
        } => format!(
            "path:{role:?}:{}:{d}:{stroke}:{}:{:?}:{line_cap:?}:{line_join:?}",
            points_sig(points, offset_x, offset_y),
            num_sig(*stroke_width),
            nums_sig(dash_array),
        ),
        RenderPrimitive::FilledPath {
            role,
            d,
            points,
            fill,
            fill_rule,
            ..
        } => format!(
            "filled-path:{role:?}:{}:{d}:{fill}:{fill_rule:?}",
            points_sig(points, offset_x, offset_y),
        ),
        RenderPrimitive::Text {
            role,
            x,
            y,
            text,
            font_size,
            font_family,
            fill,
            text_anchor,
            line_height,
            box_width,
            runs,
            ..
        } => format!(
            "text:{role:?}:{}:{}:{text}:{}:{font_family:?}:{fill:?}:{text_anchor:?}:{line_height:?}:{box_width:?}:{runs:?}",
            num_sig(*x - offset_x),
            num_sig(*y - offset_y),
            num_sig(*font_size),
        ),
    }
}

fn point_sig(point: Point, offset_x: f64, offset_y: f64) -> String {
    format!(
        "{} {}",
        num_sig(point.x - offset_x),
        num_sig(point.y - offset_y)
    )
}

fn points_sig(points: &[Point], offset_x: f64, offset_y: f64) -> String {
    points
        .iter()
        .map(|point| point_sig(*point, offset_x, offset_y))
        .collect::<Vec<_>>()
        .join(";")
}

fn nums_sig(values: &[f64]) -> Vec<String> {
    values.iter().map(|value| num_sig(*value)).collect()
}

fn num_sig(value: f64) -> String {
    format!("{:.2}", value)
}

fn object_knockout_rect_count(primitives: &[RenderPrimitive]) -> usize {
    primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Rect { role, object_id, .. }
                    if *role == RenderRole::DocumentKnockout
                        && object_id.as_deref() == Some("obj_molecule_001")
            )
        })
        .count()
}

fn object_bond_polygons_with_ids(
    primitives: &[RenderPrimitive],
) -> Vec<(String, Vec<chemcore_engine::Point>)> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                bond_id,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                object_id,
                bond_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some((bond_id.clone().unwrap_or_default(), points.clone()))
            }
            _ => None,
        })
        .collect()
}

fn object_bond_points_for_id(
    primitives: &[RenderPrimitive],
    target_bond_id: &str,
) -> Vec<chemcore_engine::Point> {
    object_bond_polygons_with_ids(primitives)
        .into_iter()
        .filter(|(bond_id, _)| bond_id == target_bond_id)
        .flat_map(|(_, points)| points)
        .collect()
}

fn object_bond_centerlines_with_ids(
    primitives: &[RenderPrimitive],
    target_object_id: &str,
) -> Vec<(String, chemcore_engine::Point, chemcore_engine::Point)> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Line {
                role,
                object_id,
                bond_id,
                from,
                to,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some(target_object_id) =>
            {
                Some((bond_id.clone().unwrap_or_default(), *from, *to))
            }
            RenderPrimitive::Polygon {
                role,
                object_id,
                bond_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some(target_object_id) =>
            {
                let (from, to) = bond_axis_from_points(points)?;
                Some((bond_id.clone().unwrap_or_default(), from, to))
            }
            _ => None,
        })
        .collect()
}

fn bond_axis_from_points(
    points: &[chemcore_engine::Point],
) -> Option<(chemcore_engine::Point, chemcore_engine::Point)> {
    if points.len() != 4 {
        return None;
    }
    Some((
        chemcore_engine::Point::new(
            (points[0].x + points[3].x) * 0.5,
            (points[0].y + points[3].y) * 0.5,
        ),
        chemcore_engine::Point::new(
            (points[1].x + points[2].x) * 0.5,
            (points[1].y + points[2].y) * 0.5,
        ),
    ))
}

fn bond_polygon_normal_widths(points: &[chemcore_engine::Point]) -> Option<(f64, f64)> {
    let (from, to) = bond_axis_from_points(points)?;
    let axis = chemcore_engine::Point::new(to.x - from.x, to.y - from.y);
    let length = (axis.x.powi(2) + axis.y.powi(2)).sqrt();
    if length <= 1.0e-9 {
        return None;
    }
    let normal = chemcore_engine::Point::new(-axis.y / length, axis.x / length);
    let start_width =
        ((points[0].x - points[3].x) * normal.x + (points[0].y - points[3].y) * normal.y).abs();
    let end_width =
        ((points[1].x - points[2].x) * normal.x + (points[1].y - points[2].y) * normal.y).abs();
    Some((start_width, end_width))
}

fn bond_axis_length(points: &[chemcore_engine::Point]) -> Option<f64> {
    let (from, to) = bond_axis_from_points(points)?;
    Some(from.distance(to))
}

fn cdxml_shape_fills_by_z(document: &ChemcoreDocument) -> Vec<String> {
    let mut shapes: Vec<_> = document
        .objects
        .iter()
        .filter(|object| object.object_type == "shape")
        .collect();
    shapes.sort_by_key(|object| object.z_index);
    shapes
        .into_iter()
        .filter_map(|object| {
            let style_ref = object.style_ref.as_ref()?;
            document.styles.get(style_ref)?.get("fill")?.as_str()
        })
        .map(ToString::to_string)
        .collect()
}

#[test]
fn export_cdxml_emits_chemdraw_document_with_native_fragment() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "C",
                "atomicNumber": 6,
                "position": [30.0, 40.0],
                "charge": 0,
                "numHydrogens": 0
            },
            {
                "id": "n2",
                "element": "O",
                "atomicNumber": 8,
                "position": [70.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "O",
                    "sourceText": "O",
                    "position": [70.0, 40.0],
                    "box": [66.0, 28.0, 78.0, 44.0],
                    "fontSize": 10.0,
                    "fill": "#000000",
                    "attachment": "node",
                    "anchor": "start"
                }
            },
            {
                "id": "n3",
                "element": "C",
                "atomicNumber": 6,
                "position": [30.0, 80.0],
                "charge": 0,
                "numHydrogens": 0,
                "isPlaceholder": true,
                "label": {
                    "text": "CF3",
                    "sourceText": "CF3",
                    "position": [30.0, 80.0],
                    "box": [30.0, 70.0, 47.4, 82.5],
                    "fontSize": 10.0,
                    "fill": "#d61f1f",
                    "attachment": "node",
                    "anchor": "start"
                }
            }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "double": { "placement": "center", "frozen": false },
                "strokeWidth": 0.6,
                "bondSpacing": 18.0,
                "marginWidth": 1.6
            },
            {
                "id": "b2",
                "begin": "n1",
                "end": "n3",
                "order": 1,
                "stereo": { "kind": "solid-wedge", "wideEnd": "end" },
                "strokeWidth": 0.6
            }
        ]),
    );

    let cdxml = document_to_cdxml(&document);

    assert!(cdxml.contains("<!DOCTYPE CDXML"));
    assert!(cdxml.contains("<CDXML"));
    assert!(cdxml.contains("CreationProgram=\"ChemCore\""));
    assert!(cdxml.contains("LabelFace=\"96\""));
    assert!(cdxml.contains("CaptionFace=\"0\""));
    assert!(cdxml.contains("color=\"0\" bgcolor=\"1\""));
    assert!(cdxml.contains("<page"));
    assert!(cdxml.contains("HeaderPosition=\"36\""));
    assert!(cdxml.contains("<fragment"));
    assert!(cdxml.contains("Order=\"2\""));
    assert!(cdxml.contains("BS=\"N\""));
    assert!(cdxml.contains("BondSpacing=\"18\""));
    assert!(cdxml.contains("MarginWidth=\"1.6\""));
    assert!(cdxml.contains("NodeType=\"Nickname\""));
    assert!(cdxml.contains("UTF8Text=\"CF3\""));
    assert!(!cdxml.contains("<t font="));
    assert!(!cdxml.contains("<t size="));
    assert!(!cdxml.contains("<t color="));
    assert!(cdxml.contains("<s font=\"3\" size=\"10\" color=\"0\""));

    let roundtripped =
        parse_cdxml_document(&cdxml, Some("roundtrip")).expect("export should parse");
    let fragment = roundtripped
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("roundtrip should create a molecule fragment");
    assert_eq!(fragment.nodes.len(), 3);
    assert_eq!(fragment.bonds.len(), 2);
    assert!(fragment.bonds.iter().any(|bond| bond.order == 2));
    assert!(fragment
        .nodes
        .iter()
        .any(|node| node.label.as_ref().is_some_and(|label| label.text == "CF3")));
    let cf3_label = fragment
        .nodes
        .iter()
        .find_map(|node| node.label.as_ref().filter(|label| label.text == "CF3"))
        .expect("CF3 label should roundtrip");
    assert_eq!(cf3_label.fill.as_deref(), Some("#d61f1f"));
}

#[test]
fn export_cdxml_preserves_text_run_style_across_reimport() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "text style",
            "page": { "width": 200.0, "height": 120.0, "background": "#ffffff" }
        },
        "styles": {
            "style_text_default": {
                "kind": "text",
                "fontFamily": "Times New Roman",
                "fontSize": 16.0,
                "fill": "#d61f1f",
                "stroke": null
            }
        },
        "objects": [{
            "id": "obj_text_001",
            "type": "text",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [24.0, 32.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_text_default",
            "payload": {
                "text": "RedBlue",
                "box": [0.0, 0.0, 80.0, 22.0],
                "align": "left",
                "fontSize": 16.0,
                "runs": [
                    {
                        "text": "Red",
                        "fontFamily": "Times New Roman",
                        "fontSize": 16.0,
                        "fill": "#d61f1f",
                        "fontWeight": 700,
                        "script": "normal"
                    },
                    {
                        "text": "Blue",
                        "fontFamily": "Arial",
                        "fontSize": 12.0,
                        "fill": "#1b32d8",
                        "fontStyle": "italic",
                        "script": "normal"
                    }
                ]
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let cdxml = document_to_cdxml(&document);
    let roundtripped =
        parse_cdxml_document(&cdxml, Some("text style")).expect("export should parse");
    let text_object = roundtripped
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should roundtrip");
    let runs: Vec<chemcore_engine::LabelRun> = serde_json::from_value(
        text_object
            .payload
            .extra
            .get("runs")
            .cloned()
            .expect("text runs should roundtrip"),
    )
    .expect("text runs should deserialize");

    assert_eq!(runs[0].text, "Red");
    assert_eq!(runs[0].fill.as_deref(), Some("#d61f1f"));
    assert_eq!(runs[0].font_family.as_deref(), Some("Times New Roman"));
    assert_eq!(runs[0].font_weight, Some(700));
    assert_eq!(runs[1].text, "Blue");
    assert_eq!(runs[1].fill.as_deref(), Some("#1b32d8"));
    assert_eq!(runs[1].font_family.as_deref(), Some("Arial"));
    assert_eq!(runs[1].font_style.as_deref(), Some("italic"));
}

#[test]
fn cdxml_import_export_import_is_render_stable_for_tmp_fixtures() {
    let fixtures = [
        "molecule.cdxml",
        "shape.cdxml",
        "kuohao.cdxml",
        "duibi.cdxml",
        "color.cdxml",
        "assets-acs.cdxml",
        "arrows-acs.cdxml",
    ];
    if fixtures
        .iter()
        .any(|fixture| !cdxml_fixture_exists(fixture))
    {
        eprintln!("skipping external CDXML roundtrip render suite; fixture set is incomplete");
        return;
    }
    for fixture in fixtures {
        let cdxml = read_cdxml_fixture(fixture);
        let imported = parse_cdxml_document(&cdxml, Some(fixture)).expect("fixture should import");
        let exported = document_to_cdxml(&imported);
        let reimported =
            parse_cdxml_document(&exported, Some(fixture)).expect("export should reimport");

        assert_eq!(
            render_roundtrip_signature(&reimported),
            render_roundtrip_signature(&imported),
            "{fixture} should be stable across import/export/import",
        );
    }
}

#[test]
fn cdxml_import_export_import_is_svg_stable_for_tmp_fixtures() {
    let fixtures = [
        "molecule.cdxml",
        "shape.cdxml",
        "kuohao.cdxml",
        "duibi.cdxml",
        "color.cdxml",
        "assets-acs.cdxml",
        "arrows-acs.cdxml",
    ];
    if fixtures
        .iter()
        .any(|fixture| !cdxml_fixture_exists(fixture))
    {
        eprintln!("skipping external CDXML roundtrip SVG suite; fixture set is incomplete");
        return;
    }
    for fixture in fixtures {
        let cdxml = read_cdxml_fixture(fixture);
        let imported = parse_cdxml_document(&cdxml, Some(fixture)).expect("fixture should import");
        let exported = document_to_cdxml(&imported);
        let reimported =
            parse_cdxml_document(&exported, Some(fixture)).expect("export should reimport");

        assert_eq!(
            document_to_svg(&reimported),
            document_to_svg(&imported),
            "{fixture} should keep the same SVG across import/export/import",
        );
    }
}

#[test]
fn cdxml_exported_arrow_fixtures_are_stable_after_first_save() {
    for fixture in ["assets-acs.cdxml", "arrows-acs.cdxml"] {
        let Some(cdxml) = read_optional_cdxml_fixture(fixture) else {
            continue;
        };
        let imported = parse_cdxml_document(&cdxml, Some(fixture)).expect("fixture should import");
        let first_export = document_to_cdxml(&imported);
        let first_reimport =
            parse_cdxml_document(&first_export, Some(fixture)).expect("first export should import");
        let second_export = document_to_cdxml(&first_reimport);
        let second_reimport = parse_cdxml_document(&second_export, Some(fixture))
            .expect("second export should import");

        assert_eq!(
            render_roundtrip_signature(&second_reimport),
            render_roundtrip_signature(&first_reimport),
            "{fixture} should not drift after the first save",
        );
    }
}

#[test]
fn export_svg_emits_rendered_document_primitives() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "C",
                "atomicNumber": 6,
                "position": [30.0, 40.0],
                "charge": 0,
                "numHydrogens": 0
            },
            {
                "id": "n2",
                "element": "O",
                "atomicNumber": 8,
                "position": [70.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "O",
                    "sourceText": "O",
                    "position": [70.0, 40.0],
                    "box": [66.0, 28.0, 78.0, 44.0],
                    "fontSize": 10.0,
                    "fill": "#000000",
                    "attachment": "node",
                    "anchor": "start"
                }
            }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "double": { "placement": "center", "frozen": false },
                "strokeWidth": 0.6,
                "bondSpacing": 18.0
            }
        ]),
    );

    let svg = document_to_svg(&document);

    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.contains("viewBox=\""));
    assert!(svg.contains("<polygon"));
    assert!(svg.contains("<text"));
    assert!(svg.contains(">O</"));
    assert!(!svg.contains("document-knockout"));
}

#[test]
fn parse_cdxml_merges_display_fragments_for_editing_hit_tests() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="18" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5" BondSpacing="18">
  <page id="1" BoundingBox="0 0 120 80">
    <fragment id="10" BoundingBox="10 10 40 20">
      <n id="11" p="10 15"/>
      <n id="12" p="40 15"/>
      <b id="13" B="11" E="12" Order="1"/>
    </fragment>
    <fragment id="20" BoundingBox="70 10 100 20">
      <n id="21" p="70 15"/>
      <n id="22" p="100 15"/>
      <b id="23" B="21" E="22" Order="1"/>
    </fragment>
  </page>
</CDXML>"#;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into editing engine");
    let document = &engine.state().document;
    let molecule_objects = document
        .objects
        .iter()
        .filter(|object| object.object_type == "molecule")
        .count();
    assert_eq!(molecule_objects, 1);
    let fragment = document
        .editable_fragment()
        .expect("merged fragment should be editable")
        .fragment;
    assert_eq!(fragment.bonds.len(), 2);
    assert!(hit_test_bond_center(
        &document,
        Point::new(85.0 * CDXML_EDIT_SCALE, 15.0 * CDXML_EDIT_SCALE),
        30.0 * CDXML_EDIT_SCALE
    )
    .is_some());
}

#[test]
fn render_cdxml_merged_fragment_node_labels_interleave_with_external_graphics_by_source_z() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="18" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5" BondSpacing="18">
  <page id="1" BoundingBox="0 0 120 80">
    <fragment id="10" BoundingBox="0 0 30 20" Z="5">
      <n id="11" p="5 10" Z="5"/>
      <n id="12" p="25 10" Z="5"/>
      <b id="13" B="11" E="12" Order="1" Z="30"/>
    </fragment>
    <fragment id="20" BoundingBox="40 40 85 65" Z="5">
      <n id="21" p="52 50" Z="30" Element="18">
        <t p="54 54" BoundingBox="44 44 54 54" LabelJustification="Right">
          <s font="3" size="10" color="0">Ar</s>
        </t>
      </n>
      <n id="22" p="74 50" Z="10"/>
      <b id="23" B="21" E="22" Order="1" Z="10"/>
    </fragment>
    <graphic id="30"
      BoundingBox="58.64 50 50 50"
      Z="20"
      GraphicType="Orbital"
      OvalType="Circle Shaded"
      OrbitalType="sShaded"
      Center3D="50 50 0"
      MajorAxisEnd3D="58.64 50 0"
      MinorAxisEnd3D="50 58.64 0"/>
  </page>
</CDXML>"#;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("layered cdxml should load");
    let document = &engine.state().document;
    let primitives = render_document(&document);
    let orbital_last = primitives
        .iter()
        .enumerate()
        .filter(|(_, primitive)| {
            render_primitive_object_id(primitive) == Some("obj_shape_orbital_001")
        })
        .map(|(index, _)| index)
        .max()
        .expect("orbital should render");
    let ar_text = primitives
        .iter()
        .enumerate()
        .find_map(|(index, primitive)| {
            (render_primitive_text_content(primitive).as_deref() == Some("Ar")).then_some(index)
        })
        .expect("Ar node label should render");
    let high_bond = primitives
        .iter()
        .enumerate()
        .find_map(|(index, primitive)| {
            (render_primitive_bond_id(primitive) == Some("f1_13")).then_some(index)
        })
        .unwrap_or_else(|| {
            panic!(
                "high-Z bond should render; bond ids: {:?}",
                primitives
                    .iter()
                    .filter_map(render_primitive_bond_id)
                    .collect::<Vec<_>>()
            )
        });

    assert!(
        orbital_last < ar_text,
        "source node Z should draw the Ar label above the external orbital"
    );
    assert!(
        orbital_last < high_bond,
        "source bond Z should draw the imported bond above the external orbital"
    );
}

#[test]
fn render_cdxml_group_children_keep_source_z_against_external_symbols() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="18" LineWidth="0.6" BondSpacing="18">
  <page id="1" BoundingBox="0 0 80 80">
    <group id="10" Z="30">
      <graphic id="11"
        BoundingBox="40 20 40 34"
        Z="1"
        GraphicType="Orbital"
        OrbitalType="lobe"/>
    </group>
    <graphic id="12"
      BoundingBox="40 26 40 38"
      Z="2"
      GraphicType="Symbol"
      SymbolType="Electron"/>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("group z")).expect("grouped orbital cdxml should parse");
    let primitives = render_document(&document);
    let lobe_last = primitives
        .iter()
        .enumerate()
        .filter(|(_, primitive)| {
            render_primitive_object_id(primitive) == Some("obj_shape_orbital_001")
        })
        .map(|(index, _)| index)
        .max()
        .expect("grouped lobe should render");
    let electron_first = primitives
        .iter()
        .enumerate()
        .find_map(|(index, primitive)| {
            (render_primitive_object_id(primitive) == Some("obj_symbol_001")).then_some(index)
        })
        .expect("external electron should render");

    assert!(
        lobe_last < electron_first,
        "CDXML group Z must not lift the white lobe fill above the higher-Z electron"
    );
}

#[test]
fn cdxml_generic_r_prime_labels_do_not_render_invalid_markers() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="18" LineWidth="0.6" BondSpacing="18">
  <page id="1" BoundingBox="0 0 120 50">
    <fragment id="10" BoundingBox="0 0 100 40">
      <n id="11" p="10 20" NodeType="GenericNickname" Z="1">
        <t p="10 20" BoundingBox="4 12 14 22"><s font="3" size="10">R&apos;</s></t>
      </n>
      <n id="12" p="40 20" Z="2"/>
      <n id="13" p="70 20" NodeType="GenericNickname" Z="3">
        <t p="70 20" BoundingBox="64 12 76 22"><s font="3" size="10">R&apos;&apos;</s></t>
      </n>
      <b id="14" B="11" E="12" Order="1" Z="4"/>
      <b id="15" B="12" E="13" Order="1" Z="5"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("r groups")).expect("generic R labels should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| match &resource.data {
            ResourceData::Fragment(fragment) => Some(fragment),
            _ => None,
        })
        .expect("fragment should be imported");
    for text in ["R'", "R''"] {
        let node = fragment
            .nodes
            .iter()
            .find(|node| {
                node.label
                    .as_ref()
                    .is_some_and(|label| label.source_text.as_deref() == Some(text))
            })
            .unwrap_or_else(|| panic!("{text} node should exist"));
        assert!(
            node.is_placeholder,
            "{text} GenericNickname should import as a placeholder"
        );
        assert_ne!(
            node.meta
                .get("labelRecognition")
                .and_then(|value| value.get("status"))
                .and_then(serde_json::Value::as_str),
            Some("invalid"),
            "{text} should not be an invalid chemical label"
        );
        assert_ne!(
            node.label
                .as_ref()
                .and_then(|label| label.meta.get("labelRecognition"))
                .and_then(|value| value.get("status"))
                .and_then(serde_json::Value::as_str),
            Some("invalid"),
            "{text} label should not be an invalid chemical label"
        );
    }
    assert!(!render_document(&document).iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::DocumentGraphic,
            stroke: Some(stroke),
            ..
        } if stroke == "#d32f2f"
    )));
}

#[test]
fn load_cdxml_document_preserves_imported_acs_drawing_options() {
    let Some(cdxml) = read_optional_cdxml_fixture("db-acs.cdxml") else {
        return;
    };
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(&cdxml)
        .expect("cdxml should load into engine");

    assert!((engine.options().bond_length - 14.4).abs() < 0.05);
    assert!((engine.options().bond_stroke_width - 0.6).abs() < 0.01);
    assert!((engine.options().bold_bond_width - 2.0).abs() < 0.05);
    assert!((engine.options().wedge_width - 3.0).abs() < 0.05);
    assert!((engine.options().hash_spacing - 2.5).abs() < 0.05);
    assert!((engine.options().bond_spacing - 18.0).abs() < 0.05);
    assert!(engine.options().label_clip_margin.abs() < 0.01);
    assert!((engine.options().margin_width - 2.0).abs() < 0.05);
}

#[test]
fn load_cdxml_document_preserves_imported_label_font_size() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" LabelSize="10">
  <page id="p1" BoundingBox="0 0 40 24">
    <fragment id="f1" BoundingBox="0 0 40 24">
      <n id="n1" p="10 12" Element="7">
        <t p="6.40 15.90" BoundingBox="6.40 7.56 13.62 15.90" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <n id="n2" p="24 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into engine");

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment")
        .fragment;
    let label = fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .and_then(|node| node.label.as_ref())
        .expect("imported N label");
    assert_eq!(label.font_family.as_deref(), Some("Arial"));
    assert_eq!(label.font_size, Some(10.0));
    assert_eq!(label.runs.first().and_then(|run| run.font_size), Some(10.0));

    let session = engine
        .begin_text_edit(Point::new(10.0, 12.0))
        .expect("clicking label should open a text edit session");
    assert_eq!(session.font_family.as_deref(), Some("Arial"));
    assert_eq!(session.font_size, Some(10.0));
    assert_eq!(
        session.source_runs.first().and_then(|run| run.font_size),
        Some(10.0)
    );
}

#[test]
fn load_cdxml_document_derives_wedge_width_from_imported_bold_width() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" MarginWidth="1.7" LabelSize="10">
  <page id="p1" BoundingBox="0 0 100 100">
    <fragment id="f1" BoundingBox="10 10 40 20">
      <n id="n1" p="10 15"/>
      <n id="n2" p="24.4 15"/>
      <b id="b1" B="n1" E="n2" Display="WedgeBegin"/>
    </fragment>
  </page>
</CDXML>"#;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into engine");

    assert!((engine.options().bond_length - 14.4).abs() < 0.05);
    assert!((engine.options().bond_stroke_width - 0.99).abs() < 0.01);
    assert!((engine.options().bold_bond_width - 2.01).abs() < 0.01);
    assert!((engine.options().wedge_width - 3.015).abs() < 0.01);
    assert!(engine.options().label_clip_margin.abs() < 0.01);
    assert!((engine.options().margin_width - 2.0).abs() < 0.01);

    let bond = &engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .bonds[0];
    assert!((bond.wedge_width.unwrap_or_default() - 3.015).abs() < 0.01);
    assert_eq!(bond.label_clip_margin, Some(0.0));
    assert_eq!(bond.margin_width, None);
}

#[test]
fn load_cdxml_document_does_not_import_margin_width_as_label_retreat() {
    fn imported_label_clip_margin(line_width: f64, margin_width: f64) -> f64 {
        let cdxml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="{line_width:.2}" BoldWidth="2.00" HashSpacing="2.50" BondSpacing="18" MarginWidth="{margin_width:.2}" LabelSize="10">
  <page id="p1" BoundingBox="0 0 50 30">
    <fragment id="f1" BoundingBox="0 0 50 30">
      <n id="n1" p="10 15"/>
      <n id="n2" p="24.4 15" Element="7">
        <t p="20.8 18.9" BoundingBox="20.8 10.56 28.02 18.9" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#
        );
        let mut engine = Engine::new();
        engine
            .load_cdxml_document(&cdxml)
            .expect("cdxml should load");
        engine.options().label_clip_margin
    }

    let normal = imported_label_clip_margin(0.60, 1.60);
    let wide_line = imported_label_clip_margin(1.80, 1.60);
    let wide_margin = imported_label_clip_margin(0.60, 5.00);

    assert!(normal.abs() < 0.01, "{normal}");
    assert!(
        (wide_line - normal).abs() < 0.01,
        "CDXML MarginWidth should not become label retreat: {normal} {wide_line}"
    );
    assert!((wide_margin - normal).abs() < 0.01, "{wide_margin}");
}

#[test]
fn parse_cdxml_imports_assets_molecules_as_native_fragments() {
    let Some(cdxml) = read_optional_cdxml_fixture("assets-acs.cdxml") else {
        return;
    };
    let document = parse_cdxml_document(&cdxml, Some("assets")).expect("cdxml should parse");

    assert!(document
        .objects
        .iter()
        .any(|object| object.object_type == "molecule"));
    let molecule_count = document
        .objects
        .iter()
        .filter(|object| object.object_type == "molecule")
        .count();
    assert!(molecule_count >= 1);
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("import should create molecule fragment resource");
    assert!(fragment.nodes.len() >= 2);
    assert!(!fragment.bonds.is_empty());
    assert!(fragment
        .bonds
        .iter()
        .all(|bond| (bond.stroke_width - 0.6).abs() < 0.001));
    assert!(render_document(&document).iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Polygon {
            role: RenderRole::DocumentBond,
            ..
        }
    )));
}

#[test]
fn parse_cdxml_imports_arrows_shapes_and_text_objects() {
    let Some(arrows) = read_optional_cdxml_fixture("arrows-acs.cdxml") else {
        return;
    };
    let arrow_document =
        parse_cdxml_document(&arrows, Some("arrows")).expect("arrows should parse");
    assert!(arrow_document
        .objects
        .iter()
        .any(|object| object.object_type == "line"
            && object.payload.extra.get("arrowHead").is_some()));
    assert!(render_document(&arrow_document)
        .iter()
        .any(|primitive| matches!(
            primitive,
            RenderPrimitive::Path {
                role: RenderRole::DocumentGraphic,
                ..
            } | RenderPrimitive::Polygon {
                role: RenderRole::DocumentGraphic,
                ..
            }
        )));

    let Some(shapes) = read_optional_cdxml_fixture("shape.cdxml") else {
        return;
    };
    let shape_document = parse_cdxml_document(&shapes, Some("shape")).expect("shape should parse");
    assert!(shape_document
        .objects
        .iter()
        .any(|object| object.object_type == "shape"));
    assert!(render_document(&shape_document)
        .iter()
        .any(|primitive| matches!(
            primitive,
            RenderPrimitive::Path {
                role: RenderRole::DocumentGraphic,
                ..
            }
        )));
}

#[test]
fn parse_cdxml_preserves_shape_style_parameters() {
    let Some(shapes) = read_optional_cdxml_fixture("shape.cdxml") else {
        return;
    };
    let document = parse_cdxml_document(&shapes, Some("shape")).expect("shape should parse");

    let dashed_circle = document
        .objects
        .iter()
        .find(|object| {
            object.object_type == "shape"
                && object
                    .payload
                    .extra
                    .get("kind")
                    .and_then(|value| value.as_str())
                    == Some("circle")
                && object.style_ref.as_ref().is_some_and(|style_ref| {
                    document.styles[style_ref]
                        .get("dashArray")
                        .and_then(|value| value.as_array())
                        .is_some_and(|dash| !dash.is_empty())
                })
        })
        .expect("dashed circle should import");
    let dashed_style = &document.styles[dashed_circle.style_ref.as_ref().unwrap()];
    assert_eq!(
        dashed_style
            .get("strokeWidth")
            .and_then(|value| value.as_f64()),
        Some(0.6)
    );

    let shadowed_rect = document
        .objects
        .iter()
        .find(|object| {
            object.object_type == "shape"
                && object.style_ref.as_ref().is_some_and(|style_ref| {
                    document.styles[style_ref]
                        .get("shadow")
                        .and_then(|value| value.as_bool())
                        == Some(true)
                })
        })
        .expect("shadowed shape should import");
    let shadow_style = &document.styles[shadowed_rect.style_ref.as_ref().unwrap()];
    assert_eq!(
        shadow_style
            .get("shadowSize")
            .and_then(|value| value.as_f64()),
        Some(4.0)
    );
}

#[test]
fn export_cdxml_writes_shape_style_parameters() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 240.0, "background": "#ffffff" }
        },
        "styles": {
            "style_circle": {
                "kind": "shape",
                "fill": null,
                "stroke": "#000000",
                "strokeWidth": 0.6,
                "dashArray": [2.7]
            },
            "style_shadow": {
                "kind": "shape",
                "fill": null,
                "stroke": "#000000",
                "strokeWidth": 0.6,
                "dashArray": [],
                "shadow": true,
                "shadowSize": 4.0
            }
        },
        "objects": [{
            "id": "obj_shape_001",
            "type": "shape",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_circle",
            "payload": {
                "bbox": [40.0, 40.0, 40.0, 40.0],
                "kind": "circle",
                "center": [60.0, 60.0],
                "majorAxisEnd": [80.0, 60.0],
                "minorAxisEnd": [60.0, 80.0]
            }
        }, {
            "id": "obj_shape_002",
            "type": "shape",
            "visible": true,
            "zIndex": 11,
            "transform": { "translate": [100.0, 50.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_shadow",
            "payload": {
                "bbox": [0.0, 0.0, 50.0, 30.0],
                "kind": "rect"
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let cdxml = document_to_cdxml(&document);
    assert!(cdxml.contains("OvalType=\"Circle Dashed\""));
    assert!(cdxml.contains("RectangleType=\"Shadow\""));
    assert!(cdxml.contains("LineWidth=\"0.6\""));
    assert!(cdxml.contains("ShadowSize=\"400\""));
}

#[test]
fn parse_cdxml_preserves_arrow_geometry_modifiers() {
    let Some(assets) = read_optional_cdxml_fixture("assets-acs.cdxml") else {
        return;
    };
    let document = parse_cdxml_document(&assets, Some("assets")).expect("assets should parse");
    assert!(document.objects.iter().any(|object| {
        object.payload.extra.get("arrowHead").is_some_and(|arrow| {
            arrow.get("noGo").and_then(|value| value.as_str()) == Some("cross")
                && arrow.get("length").and_then(|value| value.as_f64()) == Some(22.5)
                && arrow.get("centerLength").and_then(|value| value.as_f64()) == Some(19.69)
                && arrow.get("width").and_then(|value| value.as_f64()) == Some(5.63)
        })
    }));
    assert!(document.objects.iter().any(|object| {
        object.payload.extra.get("arrowHead").is_some_and(|arrow| {
            arrow.get("curve").and_then(|value| value.as_f64()) == Some(-270.0)
                && arrow.get("length").and_then(|value| value.as_f64()) == Some(8.0)
                && arrow.get("width").and_then(|value| value.as_f64()) == Some(2.0)
        })
    }));
    assert!(document.objects.iter().any(|object| {
        object.payload.extra.get("arrowHead").is_some_and(|arrow| {
            arrow.get("head").and_then(|value| value.as_str()) == Some("half-left")
        })
    }));
    assert!(document.objects.iter().any(|object| {
        object
            .payload
            .extra
            .get("arrowGeometry")
            .is_some_and(|geometry| {
                geometry.get("center").is_some()
                    && geometry.get("majorAxisEnd").is_some()
                    && geometry.get("minorAxisEnd").is_some()
            })
    }));
}

#[test]
fn cdxml_arrow_head_dimensions_are_relative_to_line_width() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML LineWidth="0.6" BondLength="14.4" color="0" bgcolor="1">
  <page id="1" BoundingBox="0 0 160 80">
    <arrow id="2" Head3D="128.21 40 0" Tail3D="0 40 0" Z="1"
      FillType="None" ArrowheadType="Solid" ArrowheadHead="Full"
      HeadSize="2250" ArrowheadCenterSize="1969" ArrowheadWidth="563"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("arrow")).expect("cdxml should parse");
    let arrow = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow should import as line object");
    let arrow_head = arrow
        .payload
        .extra
        .get("arrowHead")
        .expect("arrow should keep cdxml arrow payload");
    assert_eq!(
        arrow_head.get("length").and_then(|value| value.as_f64()),
        Some(22.5)
    );
    assert_eq!(
        arrow_head
            .get("centerLength")
            .and_then(|value| value.as_f64()),
        Some(19.69)
    );
    assert_eq!(
        arrow_head.get("width").and_then(|value| value.as_f64()),
        Some(5.63)
    );

    let primitives = render_document(&document);
    let head_points = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some(arrow.id.as_str()) =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("solid arrow head should render as filled path");
    let head_min_x = head_points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let head_max_x = head_points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let head_min_y = head_points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let head_max_y = head_points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((head_max_x - head_min_x - 13.5).abs() <= 0.001);
    assert!((head_max_y - head_min_y - 6.856).abs() <= 0.001);
}

#[test]
fn cdxml_bold_arrow_head_dimensions_stay_relative_to_cdxml_line_width() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML LineWidth="0.6" BoldWidth="2" BondLength="14.4" color="0" bgcolor="1">
  <page id="1" BoundingBox="0 0 160 80">
    <arrow id="2" Head3D="128.21 40 0" Tail3D="0 40 0" Z="1"
      LineType="Bold" FillType="None" ArrowheadType="Solid" ArrowheadHead="Full"
      HeadSize="4500" ArrowheadCenterSize="3938" ArrowheadWidth="1125"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("bold arrow")).expect("cdxml should parse");
    let arrow = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow should import as line object");
    let primitives = render_document(&document);
    let head_points = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some(arrow.id.as_str()) =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("solid arrow head should render as filled path");
    let head_min_x = head_points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let head_max_x = head_points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let head_min_y = head_points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let head_max_y = head_points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((head_max_x - head_min_x - 27.0).abs() <= 0.001);
    assert!((head_max_y - head_min_y - 13.62).abs() <= 0.001);
}

#[test]
fn cdxml_arrow_head_rendering_does_not_apply_size_floor() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML LineWidth="0.6" BondLength="14.4" color="0" bgcolor="1">
  <page id="1" BoundingBox="0 0 40 20">
    <arrow id="2" Head3D="20 10 0" Tail3D="0 10 0" Z="1"
      FillType="None" ArrowheadType="Solid" ArrowheadHead="Full"
      HeadSize="600" ArrowheadCenterSize="525" ArrowheadWidth="150"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("small arrow")).expect("cdxml should parse");
    let arrow = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow should import as line object");
    let primitives = render_document(&document);
    let head_points = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some(arrow.id.as_str()) =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("solid arrow head should render as filled path");
    let head_min_x = head_points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let head_max_x = head_points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let head_min_y = head_points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let head_max_y = head_points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((head_max_x - head_min_x - 3.6).abs() <= 0.001);
    assert!((head_max_y - head_min_y - 1.9).abs() <= 0.001);
}

#[test]
fn export_cdxml_writes_arrow_geometry_modifiers() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_default": {
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": 1.0,
                "dashArray": [2.7]
            }
        },
        "objects": [{
            "id": "obj_line_001",
            "type": "line",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_arrow_default",
            "payload": {
                "points": [[10.0, 20.0], [110.0, 20.0]],
                "head": "end",
                "tail": "none",
                "arrowHead": {
                    "kind": "solid",
                    "curve": -270.0,
                    "length": 8.0,
                    "centerLength": 7.0,
                    "width": 2.0,
                    "head": "half-left",
                    "tail": "none",
                    "fillType": "none",
                    "bold": true,
                    "noGo": "hash"
                },
                "arrowGeometry": {
                    "boundingBox": [10.0, 10.0, 120.0, 40.0],
                    "center": [65.0, 20.0],
                    "majorAxisEnd": [120.0, 20.0],
                    "minorAxisEnd": [65.0, 75.0]
                }
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let cdxml = document_to_cdxml(&document);
    assert!(cdxml.contains("HeadSize=\"800\""));
    assert!(cdxml.contains("ArrowheadCenterSize=\"700\""));
    assert!(cdxml.contains("ArrowheadWidth=\"200\""));
    assert!(cdxml.contains("ArrowheadHead=\"HalfLeft\""));
    assert!(cdxml.contains("AngularSize=\"-270\""));
    assert!(cdxml.contains("NoGo=\"Hash\""));
    assert!(cdxml.contains("LineType=\"Bold Dashed\""));
    assert!(cdxml.contains("FillType=\"None\""));
    assert!(cdxml.contains("Center3D=\"65 20 0\""));
    assert!(cdxml.contains("MajorAxisEnd3D=\"120 20 0\""));
    assert!(cdxml.contains("MinorAxisEnd3D=\"65 75 0\""));
}

#[test]
fn render_document_uses_arrow_geometry_for_elliptic_curved_arrows() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 80.0, "height": 60.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_default": {
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": 1.0
            }
        },
        "objects": [{
            "id": "obj_line_001",
            "type": "line",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_arrow_default",
            "payload": {
                "points": [[20.0, 0.0], [0.0, 10.0]],
                "head": "none",
                "tail": "none",
                "arrowHead": {
                    "kind": "curved",
                    "curve": -90.0,
                    "head": "none",
                    "tail": "none",
                    "length": 1.0,
                    "centerLength": 0.5,
                    "width": 0.2,
                    "bold": false,
                    "noGo": "none"
                },
                "arrowGeometry": {
                    "center": [0.0, 0.0],
                    "majorAxisEnd": [20.0, 0.0],
                    "minorAxisEnd": [0.0, 10.0]
                }
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let path_points = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Path {
                role: RenderRole::DocumentGraphic,
                object_id,
                points,
                ..
            } if object_id.as_deref() == Some("obj_line_001") => Some(points),
            _ => None,
        })
        .expect("elliptic arrow should render as a path");

    assert!(
        path_points
            .iter()
            .any(|point| point.x > 13.0 && point.x < 15.5 && point.y > 6.5 && point.y < 7.8),
        "{path_points:?}"
    );
}

#[test]
fn parse_document_json_fills_default_arrow_geometry_at_import_boundary() {
    let document = parse_document_json(
        &json!({
            "format": { "name": "chemcore", "version": "0.1" },
            "document": {
                "id": "doc_test",
                "title": "test",
                "page": { "width": 120.0, "height": 80.0, "background": "#ffffff" }
            },
            "styles": {
                "style_arrow_default": {
                    "kind": "stroke",
                    "stroke": "#000000",
                    "strokeWidth": 1.0
                }
            },
            "objects": [{
                "id": "obj_line_001",
                "type": "line",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_arrow_default",
                "payload": {
                    "points": [[10.0, 20.0], [90.0, 20.0]],
                    "head": "end",
                    "tail": "none",
                    "arrowHead": {
                        "kind": "curved",
                        "curve": -120.0,
                        "head": "full",
                        "tail": "none",
                        "length": 10.0,
                        "centerLength": 8.75,
                        "width": 2.5
                    }
                }
            }],
            "resources": {}
        })
        .to_string(),
    )
    .expect("document json should parse");

    let object = document
        .objects
        .iter()
        .find(|object| object.id == "obj_line_001")
        .expect("arrow object should exist");
    let geometry = object
        .payload
        .extra
        .get("arrowGeometry")
        .expect("legacy curved arrow should receive default arc geometry");
    let arrow_head = object
        .payload
        .extra
        .get("arrowHead")
        .expect("arrow head should be normalized at import boundary");
    assert_eq!(
        arrow_head.get("kind").and_then(|value| value.as_str()),
        Some("solid")
    );
    assert_eq!(
        arrow_head.get("bold").and_then(|value| value.as_bool()),
        Some(false)
    );
    assert_eq!(
        arrow_head.get("noGo").and_then(|value| value.as_str()),
        Some("none")
    );
    assert!(geometry.get("center").is_some());
    assert!(geometry.get("majorAxisEnd").is_some());
    assert!(geometry.get("minorAxisEnd").is_some());
    assert!(render_document(&document).iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            object_id,
            ..
        } if object_id.as_deref() == Some("obj_line_001")
    )));
}

#[test]
fn parse_document_json_normalizes_text_and_shape_payloads_at_import_boundary() {
    let document = parse_document_json(
        &json!({
            "format": { "name": "chemcore", "version": "0.1" },
            "document": {
                "id": "doc_test",
                "title": "test",
                "page": { "width": 120.0, "height": 80.0, "background": "#ffffff" }
            },
            "objects": [
                {
                    "id": "obj_text_001",
                    "type": "text",
                    "visible": true,
                    "zIndex": 10,
                    "transform": { "translate": [12.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "payload": { "text": "Note" }
                },
                {
                    "id": "obj_shape_001",
                    "type": "shape",
                    "visible": true,
                    "zIndex": 11,
                    "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "payload": {
                        "kind": "ellipse",
                        "bbox": [10.0, 20.0, 40.0, 0.0]
                    }
                }
            ],
            "resources": {}
        })
        .to_string(),
    )
    .expect("document json should parse");

    let text = document
        .objects
        .iter()
        .find(|object| object.id == "obj_text_001")
        .expect("text object should exist");
    assert_eq!(
        text.payload
            .extra
            .get("fontSize")
            .and_then(|value| value.as_f64()),
        Some(10.0)
    );
    assert!(text.payload.extra.get("lineHeight").is_some());
    assert!(text.payload.extra.get("box").is_some());
    assert_eq!(
        text.payload
            .extra
            .get("align")
            .and_then(|value| value.as_str()),
        Some("left")
    );

    let shape = document
        .objects
        .iter()
        .find(|object| object.id == "obj_shape_001")
        .expect("shape object should exist");
    assert_eq!(
        shape.payload.extra.get("center"),
        Some(&json!([30.0, 20.0]))
    );
    assert_eq!(
        shape.payload.extra.get("majorAxisEnd"),
        Some(&json!([50.0, 20.0]))
    );
    assert_eq!(
        shape.payload.extra.get("minorAxisEnd"),
        Some(&json!([30.0, 40.0]))
    );
}

#[test]
fn parse_cdxml_imports_free_text_object() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 80 36" Justification="Left">
      <s font="3" size="12" face="33" color="0">H2O</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("text")).expect("text cdxml should parse");
    assert!(document
        .objects
        .iter()
        .any(|object| object.object_type == "text"));
    assert!(render_document(&document).iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Text {
            role: RenderRole::DocumentText,
            text,
            runs,
            ..
        } if text == "H2O" || runs.iter().any(|run| run.text == "H2O")
    )));
}

#[test]
fn parse_cdxml_text_auto_line_height_uses_chemdraw_import_compatibility() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="1" p="10 20" BoundingBox="10 10 90 45" Justification="Left" LineHeight="auto">
      <s font="3" size="10" face="0" color="0">Plain A
Plain B</s>
    </t>
    <t id="2" p="110 20" BoundingBox="110 10 210 45" Justification="Left" LineHeight="auto">
      <s font="3" size="10" face="97" color="0">H2O A
H2O B</s>
    </t>
    <t id="3" p="10 90" BoundingBox="10 80 110 125" Justification="Left" LineHeight="auto">
      <s font="3" size="10" face="1" color="0">H</s><s font="3" size="10" face="33" color="0">2</s><s font="3" size="10" face="1" color="0">O A
H</s><s font="3" size="10" face="33" color="0">2</s><s font="3" size="10" face="1" color="0">O B</s>
    </t>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("line height")).expect("text cdxml should parse");
    let mut line_heights: Vec<f64> = document
        .objects
        .iter()
        .filter(|object| object.object_type == "text")
        .filter_map(|object| {
            object
                .payload
                .extra
                .get("lineHeight")
                .and_then(|value| value.as_f64())
        })
        .collect();
    line_heights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    assert_eq!(line_heights, vec![11.5, 11.75, 13.45]);
}

#[test]
fn parse_cdxml_unescapes_text_entities() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 120 52" Justification="Left">
      <s font="3" size="12" face="0" color="0">d.r. &gt; 20:1 &amp; clean</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("entities")).expect("text cdxml should parse");
    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text should import");

    assert_eq!(
        text_object
            .payload
            .extra
            .get("text")
            .and_then(|value| value.as_str()),
        Some("d.r. > 20:1 & clean")
    );
}

#[test]
fn parse_cdxml_skips_bracketusage_objecttag_text() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <graphic id="2" BoundingBox="20 10 20 70" GraphicType="Bracket" BracketType="Square">
      <objecttag id="1" Name="bracketusage">
        <t p="0 0" BoundingBox="0 -6.30 4.17 0"><s font="3" size="7.5" color="0">2</s></t>
      </objecttag>
      <objecttag id="2" Name="parameterizedBracketLabel" Visible="no">
        <t p="24 74" BoundingBox="24 68 42 74" Visible="no"><s font="3" size="7.5" color="0">abc</s></t>
      </objecttag>
    </graphic>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("bracket text")).expect("cdxml should parse");
    let texts: Vec<_> = document
        .objects
        .iter()
        .filter(|object| object.object_type == "text")
        .filter_map(|object| {
            object
                .payload
                .extra
                .get("text")
                .and_then(|value| value.as_str())
        })
        .collect();
    assert_eq!(texts, vec!["abc"]);
}

#[test]
fn parse_cdxml_preserves_small_text_object_bbox() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 18 28" Justification="Left" UTF8Text="x">
      <s font="3" size="6" face="0" color="0">x</s>
    </t>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("small text")).expect("text cdxml should parse");
    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text should import");
    let bbox: [f64; 4] = serde_json::from_value(
        text_object
            .payload
            .extra
            .get("box")
            .cloned()
            .expect("text object should preserve box"),
    )
    .expect("text box should deserialize");

    assert_eq!(bbox, [0.0, 0.0, 8.0, 8.0]);
}

#[test]
fn parse_cdxml_preserves_aligned_text_object_source_bbox() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="50 20" BoundingBox="30 10 70 30" Justification="Center" UTF8Text="center">
      <s font="3" size="10" face="0" color="0">center</s>
    </t>
    <t id="3" p="120 20" BoundingBox="80 10 120 30" Justification="Right" UTF8Text="right">
      <s font="3" size="10" face="0" color="0">right</s>
    </t>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("aligned text")).expect("text cdxml should parse");
    let text_objects: Vec<_> = document
        .scene_objects()
        .into_iter()
        .filter(|object| object.object_type == "text")
        .collect();

    let center = text_objects
        .iter()
        .find(|object| {
            object
                .payload
                .extra
                .get("text")
                .and_then(serde_json::Value::as_str)
                == Some("center")
        })
        .expect("center text should import");
    let center_box: [f64; 4] = serde_json::from_value(
        center
            .payload
            .extra
            .get("box")
            .cloned()
            .expect("center text should preserve box"),
    )
    .expect("center box should deserialize");
    assert_eq!(center.transform.translate, [50.0, 10.0]);
    assert_eq!(center_box, [-20.0, 0.0, 40.0, 20.0]);

    let right = text_objects
        .iter()
        .find(|object| {
            object
                .payload
                .extra
                .get("text")
                .and_then(serde_json::Value::as_str)
                == Some("right")
        })
        .expect("right text should import");
    let right_box: [f64; 4] = serde_json::from_value(
        right
            .payload
            .extra
            .get("box")
            .cloned()
            .expect("right text should preserve box"),
    )
    .expect("right box should deserialize");
    assert_eq!(right.transform.translate, [120.0, 10.0]);
    assert_eq!(right_box, [-40.0, 0.0, 40.0, 20.0]);
}

#[test]
fn load_cdxml_document_hit_tests_aligned_text_object_source_bbox() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="50 20" BoundingBox="30 10 70 30" Justification="Center" UTF8Text="center">
      <s font="3" size="10" face="0" color="0">center</s>
    </t>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into engine");

    engine.select_at_point(
        Point::new(31.0 * CDXML_EDIT_SCALE, 20.0 * CDXML_EDIT_SCALE),
        false,
    );
    assert_eq!(engine.state().selection.text_objects, vec!["obj_text_001"]);

    engine.select_at_point(
        Point::new(71.0 * CDXML_EDIT_SCALE, 20.0 * CDXML_EDIT_SCALE),
        false,
    );
    assert!(engine.state().selection.text_objects.is_empty());
}

#[test]
fn parse_document_json_migrates_legacy_aligned_text_object_box() {
    let document = parse_document_json(
        &json!({
            "format": { "name": "chemcore", "version": "0.1" },
            "document": {
                "id": "doc_test",
                "title": "test",
                "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
            },
            "objects": [{
                "id": "obj_text_001",
                "type": "text",
                "transform": { "translate": [50.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {
                    "text": "center",
                    "align": "center",
                    "box": [0.0, 0.0, 40.0, 20.0]
                }
            }]
        })
        .to_string(),
    )
    .expect("document json should parse");
    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should exist");
    let box_value: [f64; 4] = serde_json::from_value(
        text_object
            .payload
            .extra
            .get("box")
            .cloned()
            .expect("text object should preserve migrated box"),
    )
    .expect("text box should deserialize");

    assert_eq!(box_value, [-20.0, 0.0, 40.0, 20.0]);
}

#[test]
fn parse_cdxml_formula_face_expands_digits_to_subscript() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 80 36" Justification="Left" UTF8Text="CF3">
      <s font="3" size="12" face="97" color="0">CF3</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("formula")).expect("text cdxml should parse");
    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("formula text should import");
    let runs: Vec<chemcore_engine::LabelRun> = serde_json::from_value(
        text_object
            .payload
            .extra
            .get("runs")
            .cloned()
            .expect("imported text should preserve runs"),
    )
    .expect("runs should deserialize");

    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].text, "CF");
    assert_eq!(runs[0].font_weight, Some(700));
    assert_eq!(runs[0].script.as_deref(), Some("normal"));
    assert_eq!(runs[1].text, "3");
    assert_eq!(runs[1].font_weight, Some(700));
    assert_eq!(runs[1].script.as_deref(), Some("subscript"));
}

#[test]
fn parse_cdxml_chemical_face_subscripts_group_multipliers() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 120 36" Justification="Left" UTF8Text="ArB(OH)2">
      <s font="3" size="10" face="96" color="0">ArB(OH)2</s>
    </t>
    <t id="3" p="10 40" BoundingBox="10 40 160 56" Justification="Left" UTF8Text="Cu(CH3CN)4PF6">
      <s font="3" size="10" face="96" color="0">Cu(CH3CN)4PF6</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("formula")).expect("text cdxml should parse");
    let text_runs = |text: &str| -> Vec<chemcore_engine::LabelRun> {
        let object = document
            .objects
            .iter()
            .find(|object| {
                object
                    .payload
                    .extra
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    == Some(text)
            })
            .expect("formula text should import");
        serde_json::from_value(
            object
                .payload
                .extra
                .get("runs")
                .cloned()
                .expect("imported text should preserve runs"),
        )
        .expect("runs should deserialize")
    };

    let ar_boron = text_runs("ArB(OH)2");
    assert_eq!(ar_boron.last().map(|run| run.text.as_str()), Some("2"));
    assert_eq!(
        ar_boron.last().and_then(|run| run.script.as_deref()),
        Some("subscript")
    );

    let copper = text_runs("Cu(CH3CN)4PF6");
    let subscript_text: Vec<_> = copper
        .iter()
        .filter(|run| run.script.as_deref() == Some("subscript"))
        .map(|run| run.text.as_str())
        .collect();
    assert_eq!(subscript_text, vec!["3", "4", "6"]);
}

#[test]
fn parse_cdxml_decodes_face_bit_combinations() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 120 36" Justification="Left" UTF8Text="A2B+NO2">
      <s font="3" size="12" face="39" color="0">A2</s>
      <s font="3" size="12" face="70" color="0">B+</s>
      <s font="3" size="12" face="103" color="0">NO2</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("faces")).expect("text cdxml should parse");
    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text should import");
    let runs: Vec<chemcore_engine::LabelRun> = serde_json::from_value(
        text_object
            .payload
            .extra
            .get("runs")
            .cloned()
            .expect("imported text should preserve runs"),
    )
    .expect("runs should deserialize");

    assert_eq!(runs[0].text, "A2");
    assert_eq!(runs[0].font_weight, Some(700));
    assert_eq!(runs[0].font_style.as_deref(), Some("italic"));
    assert_eq!(runs[0].underline, Some(true));
    assert_eq!(runs[0].script.as_deref(), Some("subscript"));

    assert_eq!(runs[1].text, "B+");
    assert_eq!(runs[1].font_weight, Some(400));
    assert_eq!(runs[1].font_style.as_deref(), Some("italic"));
    assert_eq!(runs[1].underline, Some(true));
    assert_eq!(runs[1].script.as_deref(), Some("superscript"));

    assert_eq!(
        runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(),
        vec!["A2", "B+", "NO", "2"]
    );
    assert_eq!(runs[2].font_weight, Some(700));
    assert_eq!(runs[2].font_style.as_deref(), Some("italic"));
    assert_eq!(runs[2].underline, Some(true));
    assert_eq!(runs[2].script.as_deref(), Some("normal"));
    assert_eq!(runs[3].font_weight, Some(700));
    assert_eq!(runs[3].font_style.as_deref(), Some("italic"));
    assert_eq!(runs[3].underline, Some(true));
    assert_eq!(runs[3].script.as_deref(), Some("subscript"));
}

#[test]
fn parse_cdxml_imports_table_lines_and_text_boxes() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <graphic id="2" GraphicType="Line" LineType="Dashed" Head3D="10 10 0" Tail3D="80 10 0"/>
    <t id="3" p="12 14" BoundingBox="12 14 60 30" Justification="Left">
      <s font="3" size="10" color="0">entry</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("table")).expect("table cdxml should parse");
    let line = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("plain table line should import");
    assert!(line.payload.extra.get("arrowHead").is_none());
    let line_style = document
        .styles
        .get(line.style_ref.as_deref().expect("line style ref"))
        .expect("line style should exist");
    assert_eq!(line_style["dashArray"], json!([2.5]));
    assert!(document
        .objects
        .iter()
        .any(|object| object.object_type == "text"));
    let primitives = render_document(&document);
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Polyline {
            role: RenderRole::DocumentGraphic,
            dash_array,
            ..
        } if !dash_array.is_empty()
    )));
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Text {
            role: RenderRole::DocumentText,
            text,
            runs,
            ..
        } if text == "entry" || runs.iter().any(|run| run.text == "entry")
    )));
}

#[test]
fn parse_cdxml_renders_acs_dashed_bond_patterns_like_chemdraw() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" BondSpacing="18" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <fragment id="2" BoundingBox="80 35 116 45">
      <n id="3" p="80 40"/>
      <n id="4" p="116 40"/>
      <b id="5" B="3" E="4" Display="Dash"/>
    </fragment>
    <fragment id="6" BoundingBox="80 65 116 80">
      <n id="7" p="80 70"/>
      <n id="8" p="116 70"/>
      <b id="9" B="7" E="8" Order="2" Display2="Dash"/>
    </fragment>
    <fragment id="10" BoundingBox="80 95 116 110">
      <n id="11" p="80 100"/>
      <n id="12" p="116 100"/>
      <b id="13" B="11" E="12" Order="2" Display="Dash" Display2="Dash"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("acs dash patterns")).expect("parse cdxml");
    let primitives = render_document(&document);

    assert_eq!(
        document_knockout_count_for_object(&primitives, "obj_mol_001"),
        6
    );
    assert_eq!(
        document_bond_polygon_count_for_object(&primitives, "obj_mol_002"),
        2
    );
    assert_eq!(
        document_knockout_count_for_object(&primitives, "obj_mol_002"),
        6
    );
    assert_eq!(
        document_bond_polygon_count_for_object(&primitives, "obj_mol_003"),
        2
    );
    assert_eq!(
        document_knockout_count_for_object(&primitives, "obj_mol_003"),
        12
    );

    let single_gaps = document_knockout_axis_intervals_for_object(&primitives, "obj_mol_001");
    assert_eq!(single_gaps.len(), 6, "{single_gaps:?}");
    assert!((single_gaps[0].0 - 2.5).abs() < 0.01, "{single_gaps:?}");
    assert!((single_gaps[0].1 - 5.5833).abs() < 0.01, "{single_gaps:?}");
    assert!((single_gaps[5].0 - 30.4167).abs() < 0.01, "{single_gaps:?}");
    assert!((single_gaps[5].1 - 33.5).abs() < 0.01, "{single_gaps:?}");
}

#[test]
fn parse_cdxml_imports_published_formula_face_node_labels_with_subscripts() {
    let Some(cdxml) = read_optional_cdxml_fixture("figure2.cdxml") else {
        return;
    };
    let document =
        parse_cdxml_document(&cdxml, Some("figure2")).expect("published cdxml should parse");
    let cf3_label = document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.nodes.iter())
        .filter_map(|node| node.label.as_ref())
        .find(|label| label.source_text.as_deref() == Some("CF3"))
        .expect("example should import CF3 node label");

    assert_eq!(cf3_label.text, "CF3");
    assert_eq!(
        cf3_label
            .runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        vec!["CF", "3"]
    );
    assert_eq!(cf3_label.runs[1].script.as_deref(), Some("subscript"));
    assert_eq!(
        cf3_label
            .meta
            .pointer("/sourceRuns/0/text")
            .and_then(serde_json::Value::as_str),
        Some("CF3")
    );
}

#[test]
fn load_cdxml_document_preserves_single_character_below_label_position() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" LabelSize="10" color="0" bgcolor="1">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
  </colortable>
  <fonttable>
    <font id="3" charset="iso-8859-1" name="Arial"/>
  </fonttable>
  <page id="p1" BoundingBox="238.76 122.79 310.06 156.07">
    <fragment id="f1" BoundingBox="238.76 122.79 310.06 156.07">
      <n id="n1" p="256.05 139.70"/>
      <n id="n2" p="270.45 139.70" NodeType="Fragment">
        <t id="t1" p="268.70 143.60" BoundingBox="268.70 138.06 272.20 143.60" LabelJustification="Left" LabelAlignment="Below" UTF8Text="•">
          <s font="3" size="10" color="0" face="96">•</s>
        </t>
      </n>
      <n id="n3" p="284.85 139.70"/>
      <b id="b1" B="n1" E="n2" Order="2"/>
      <b id="b2" B="n2" E="n3" Order="2"/>
    </fragment>
  </page>
</CDXML>"#;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into engine");

    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("imported fragment should be editable");
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| {
            node.label
                .as_ref()
                .is_some_and(|label| label.source_text.as_deref() == Some("•"))
        })
        .expect("bullet label node should import");
    let label = node.label.as_ref().expect("node should have a label");
    let position = label.position.expect("bullet label should keep position");
    let world_position = [
        entry.object.transform.translate[0] + position[0],
        entry.object.transform.translate[1] + position[1],
    ];

    assert_eq!(node.element, "C");
    assert_eq!(node.atomic_number, 6);
    assert!(
        !node.is_placeholder,
        "the CDXML bullet is a visible carbon atom, not an invalid text placeholder"
    );
    assert!(
        node.meta.get("labelRecognition").is_none(),
        "the CDXML bullet carbon should not be marked as an invalid functional label"
    );
    assert_eq!(label.text, "•");
    assert!(
        label.meta.get("labelRecognition").is_none(),
        "the CDXML bullet carbon label should not carry invalid-label metadata"
    );
    assert_eq!(
        label
            .meta
            .pointer("/import/cdxml/labelAlignment")
            .and_then(serde_json::Value::as_str),
        Some("Below")
    );
    assert!(
        (world_position[0] - 268.70).abs() < 0.01,
        "single-character CDXML labels should keep source baseline x, got {world_position:?}"
    );
    assert!(
        (world_position[1] - 143.60).abs() < 0.01,
        "single-character CDXML labels should keep source baseline y, got {world_position:?}"
    );
}

#[test]
fn parse_cdxml_keeps_numeric_suffix_node_label_anchored_on_letter() {
    fn labeled_nodes(
        document: &ChemcoreDocument,
    ) -> Vec<(&chemcore_engine::Node, &chemcore_engine::NodeLabel)> {
        document
            .resources
            .values()
            .filter_map(|resource| resource.data.as_fragment())
            .flat_map(|fragment| fragment.nodes.iter())
            .filter_map(|node| node.label.as_ref().map(|label| (node, label)))
            .collect()
    }

    fn anchor_of(label: &chemcore_engine::NodeLabel, index: usize) -> Point {
        let polygon = label
            .glyph_polygons
            .get(index)
            .expect("glyph polygon should exist");
        let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
        let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for [x, y] in polygon {
            min_x = min_x.min(*x);
            min_y = min_y.min(*y);
            max_x = max_x.max(*x);
            max_y = max_y.max(*y);
        }
        Point::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5)
    }

    let invalid_cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" LabelSize="10">
  <page id="p1" BoundingBox="0 0 80 40">
    <fragment id="f1" BoundingBox="0 0 40 20">
      <n id="n1" p="10 10" NodeType="Nickname">
        <t id="t1" p="10 14" BoundingBox="4 4 16 16" UTF8Text="X3">
          <s font="3" size="10" face="96">X3</s>
        </t>
      </n>
      <n id="n2" p="24.4 10"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let invalid_imported =
        parse_cdxml_document(invalid_cdxml, Some("invalid")).expect("invalid label cdxml");
    let (invalid_node, invalid_label) = labeled_nodes(&invalid_imported)
        .into_iter()
        .find(|(_, label)| label.source_text.as_deref() == Some("X3"))
        .expect("invalid X3 label should import");
    assert_eq!(
        invalid_label
            .meta
            .get("labelRecognition")
            .and_then(|meta| meta.get("status"))
            .and_then(serde_json::Value::as_str),
        Some("invalid")
    );
    assert!(
        anchor_of(invalid_label, 0).distance(invalid_node.point()) < 0.01,
        "invalid labels should prefer non-script glyph anchors over subscript/superscript glyphs: node={invalid_node:?}, label={invalid_label:?}"
    );
}

#[test]
fn parse_cdxml_uses_chemdraw_color_table_offset() {
    let Some(cdxml) = read_optional_cdxml_fixture("color.cdxml") else {
        return;
    };
    let document = parse_cdxml_document(&cdxml, Some("color")).expect("color cdxml should parse");

    let shape_fills = cdxml_shape_fills_by_z(&document);
    assert_eq!(
        shape_fills,
        vec![
            "#000000", "#ff0000", "#ffff00", "#00ff00", "#ffffff", "#00ffff", "#0000ff", "#ff00ff",
            "#804040", "#008000", "#0000a0", "#808080",
        ]
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("color=\"4\""), "{exported}");
    assert!(
        exported.contains("<color r=\"1\" g=\"0\" b=\"0\"/>"),
        "{exported}"
    );

    let reimported =
        parse_cdxml_document(&exported, Some("color export")).expect("export should parse");
    assert_eq!(cdxml_shape_fills_by_z(&reimported), shape_fills);
}

#[test]
fn cdxml_electron_symbol_uses_chemdraw_top_anchor_and_color() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" color="0" bgcolor="1">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="1" g="0" b="0"/>
  </colortable>
  <page id="1">
    <graphic id="2" BoundingBox="285.19 130.29 285.19 141.94" Z="1" color="4" GraphicType="Symbol" SymbolType="Electron"/>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("electron symbol cdxml should load");
    let document = &engine.state().document;
    let symbol = document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("electron symbol should import");
    assert_eq!(
        symbol
            .payload
            .extra
            .get("kind")
            .and_then(|value| value.as_str()),
        Some("electron")
    );
    assert_eq!(
        symbol
            .payload
            .extra
            .get("fill")
            .and_then(|value| value.as_str()),
        Some("#ff0000")
    );
    let [_, _, width, height] = symbol.payload.bbox.expect("symbol should have bbox");
    let center = [
        symbol.transform.translate[0] + width * 0.5,
        symbol.transform.translate[1] + height * 0.5,
    ];
    let expected_diameter = 11.65 * 2.0 / 9.0;
    assert!(
        (width - expected_diameter).abs() < 0.01,
        "electron diameter should follow ChemDraw's anchor height ratio, got width={width}"
    );
    assert!(
        (height - expected_diameter).abs() < 0.01,
        "electron diameter should follow ChemDraw's anchor height ratio, got height={height}"
    );
    assert!(
        (center[0] - 285.19).abs() < 0.01,
        "electron center x should use the CDXML anchor x, got {center:?}"
    );
    assert!(
        (center[1] - 130.29).abs() < 0.01,
        "electron center y should use the top of the CDXML anchor bbox, got {center:?}"
    );
    let rendered_diameter = render_document(document)
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                object_id, d, fill, ..
            } if object_id.as_deref() == Some(symbol.id.as_str()) && fill == "#ff0000" => {
                horizontal_path_span_at_y(d, center[1])
            }
            _ => None,
        })
        .expect("electron should render as a filled path");
    assert!(
        (rendered_diameter - expected_diameter).abs() < 0.01,
        "rendered electron diameter should match imported geometry, got {rendered_diameter}"
    );

    let exported = document_to_cdxml(document);
    assert!(exported.contains("SymbolType=\"Electron\""), "{exported}");
    assert!(exported.contains("color=\"4\""), "{exported}");
    let reimported =
        parse_cdxml_document(&exported, Some("electron export")).expect("export should parse");
    let reimported_symbol = reimported
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("exported electron should reimport");
    let [_, _, re_width, re_height] = reimported_symbol
        .payload
        .bbox
        .expect("reimported symbol should have bbox");
    assert!((re_width - expected_diameter).abs() < 0.01, "{re_width}");
    assert!((re_height - expected_diameter).abs() < 0.01, "{re_height}");
    let re_center = [
        reimported_symbol.transform.translate[0] + re_width * 0.5,
        reimported_symbol.transform.translate[1] + re_height * 0.5,
    ];
    assert!((re_center[0] - 285.19).abs() < 0.01, "{re_center:?}");
    assert!((re_center[1] - 130.29).abs() < 0.01, "{re_center:?}");
}

#[test]
fn parse_cdxml_color_table_keeps_duplicate_slots() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML color="0" bgcolor="1">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="0" g="0" b="0"/>
    <color r="1" g="0" b="0"/>
  </colortable>
  <page id="1">
    <graphic id="2" BoundingBox="20 20 40 40" Z="1" color="5" GraphicType="Oval" OvalType="Circle Filled" Center3D="30 30 0" MajorAxisEnd3D="40 30 0" MinorAxisEnd3D="30 40 0"/>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("duplicate colors")).expect("cdxml should parse");

    assert_eq!(cdxml_shape_fills_by_z(&document), vec!["#ff0000"]);
}

#[test]
fn parse_cdxml_infers_benzene_double_bond_sides_and_bond_colors() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" BondSpacing="18" LineWidth="0.60" color="0" bgcolor="1">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="1" g="0" b="0"/>
  </colortable>
  <page id="1">
    <fragment id="2" BoundingBox="7 10 33 39">
      <n id="n1" p="20.00 10.00"/>
      <n id="n2" p="32.47 17.20"/>
      <n id="n3" p="32.47 31.60"/>
      <n id="n4" p="20.00 38.80"/>
      <n id="n5" p="7.53 31.60"/>
      <n id="n6" p="7.53 17.20"/>
      <b id="b1" B="n1" E="n2" Order="2" BondCircularOrdering="b2 0 0 b6"/>
      <b id="b2" B="n2" E="n3"/>
      <b id="b3" B="n3" E="n4" Order="2" color="4" BondCircularOrdering="b4 0 0 b2"/>
      <b id="b4" B="n4" E="n5"/>
      <b id="b5" B="n5" E="n6" Order="2" BondCircularOrdering="b6 0 0 b4"/>
      <b id="b6" B="n6" E="n1"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("benzene")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should import");

    for bond_id in ["b1", "b3", "b5"] {
        let bond = fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id)
            .expect("benzene double bond should import");
        assert_eq!(
            bond.double.as_ref().map(|double| double.placement),
            Some(chemcore_engine::DoubleBondPlacement::Left),
            "{bond_id} should infer an inward side double placement"
        );
    }
    assert_eq!(
        fragment
            .bonds
            .iter()
            .find(|bond| bond.id == "b3")
            .and_then(|bond| bond.stroke.as_deref()),
        Some("#ff0000")
    );

    let primitives = render_document(&document);
    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Line {
                role: RenderRole::DocumentBond,
                bond_id: Some(id),
                stroke,
                ..
            } | RenderPrimitive::Polygon {
                role: RenderRole::DocumentBond,
                bond_id: Some(id),
                stroke,
                ..
            } if id == "b3" && stroke == "#ff0000"
        )),
        "colored CDXML bond should render with its imported stroke"
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("color=\"4\""), "{exported}");
    assert!(exported.contains("DoublePosition=\"Left\""), "{exported}");
    let reimported =
        parse_cdxml_document(&exported, Some("benzene export")).expect("export should parse");
    let reimported_fragment = reimported
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("reimported fragment should exist");
    assert!(reimported_fragment
        .bonds
        .iter()
        .any(|bond| bond.stroke.as_deref() == Some("#ff0000")));
}

#[test]
fn cdxml_export_import_preserves_non_white_page_background() {
    let document = parse_document_json(
        &json!({
            "format": { "name": "chemcore", "version": "0.1" },
            "document": {
                "id": "doc_test",
                "title": "red background",
                "page": { "width": 120.0, "height": 80.0, "background": "#ff0000" }
            },
            "styles": {},
            "objects": [],
            "resources": {}
        })
        .to_string(),
    )
    .expect("document json should parse");

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("bgcolor=\"4\""), "{exported}");

    let reimported =
        parse_cdxml_document(&exported, Some("red background")).expect("export should parse");
    assert_eq!(reimported.document.page.background, "#ff0000");
}

#[test]
fn parse_cdxml_right_side_double_bonds_render_on_begin_to_end_right_side() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="20.00" BondSpacing="18" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" LabelSize="10" BoundingBox="0 0 40 50">
  <page id="p1" BoundingBox="0 0 40 50">
    <fragment id="f1" BoundingBox="0 0 40 50">
      <n id="n1" p="10 10"/>
      <n id="n2" p="10 30"/>
      <b id="b1" B="n1" E="n2" Order="2" DoublePosition="Right"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("right side")).expect("cdxml should parse");
    let object = document
        .objects
        .iter()
        .find(|object| object.id == "obj_mol_001")
        .expect("fragment should import as obj_mol_001");
    let resource_ref = object
        .payload
        .resource_ref
        .as_deref()
        .expect("molecule object should have resourceRef");
    let fragment = document
        .resources
        .get(resource_ref)
        .expect("molecule resource should exist")
        .data
        .as_fragment()
        .expect("molecule resource should have fragment data");

    let bond = fragment
        .bonds
        .iter()
        .find(|bond| bond.id == "b1")
        .expect("double bond should import");
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(chemcore_engine::DoubleBondPlacement::Right)
    );

    let primitives = render_document(&document);
    let centerlines = object_bond_centerlines_with_ids(&primitives, "obj_mol_001");
    let begin = chemcore_engine::Point::new(10.0, 10.0);
    let end = chemcore_engine::Point::new(10.0, 30.0);
    let dx = end.x - begin.x;
    let dy = end.y - begin.y;
    let length = dx.hypot(dy);
    let right_normal = chemcore_engine::Point::new(dy / length, -dx / length);
    let raw_mid = chemcore_engine::Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5);
    let max_rendered_projection = centerlines
        .iter()
        .filter(|(id, _, _)| id == "b1")
        .map(|(_, from, to)| {
            let mid = chemcore_engine::Point::new((from.x + to.x) * 0.5, (from.y + to.y) * 0.5);
            (mid.x - raw_mid.x) * right_normal.x + (mid.y - raw_mid.y) * right_normal.y
        })
        .max_by(|a, b| a.total_cmp(b))
        .expect("double bond should render centerlines");
    assert!(
        max_rendered_projection > 0.0,
        "outer line should render on B->E right side, got {max_rendered_projection}"
    );
}

#[test]
fn parse_cdxml_unspecified_alkene_double_bond_uses_automatic_side_placement() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 120 80">
    <fragment id="f1" BoundingBox="10 10 80 50">
      <n id="n1" p="10 40"/>
      <n id="n2" p="24.4 40"/>
      <n id="n3" p="38.8 40"/>
      <n id="n4" p="24.4 26"/>
      <b id="b1" B="n1" E="n2"/>
      <b id="b2" B="n2" E="n3" Order="2"/>
      <b id="b3" B="n2" E="n4"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("alkene")).expect("cdxml should parse");
    let fragment = document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment;
    let bond = fragment
        .bonds
        .iter()
        .find(|bond| bond.id == "b2")
        .expect("alkene double bond should import");

    let double = bond
        .double
        .as_ref()
        .expect("double bond state should be inferred");
    assert_ne!(
        double.placement,
        chemcore_engine::DoubleBondPlacement::Center
    );
    assert!(!double.frozen);
}

#[test]
fn parse_cdxml_auto_double_bond_matches_chemdraw_center_and_tie_rules() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="20.00" BondSpacing="18" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" LabelSize="10" BoundingBox="0 0 140 70">
  <page id="p1" BoundingBox="0 0 140 70">
    <fragment id="f1" BoundingBox="0 0 60 60">
      <n id="n1" p="30 25"/>
      <n id="n2" p="30 45"/>
      <n id="n3" p="12.68 15"/>
      <n id="n4" p="47.32 15"/>
      <b id="b1" B="n1" E="n2" Order="2"/>
      <b id="b2" B="n1" E="n3"/>
      <b id="b3" B="n1" E="n4"/>
    </fragment>
    <fragment id="f2" BoundingBox="60 0 140 60">
      <n id="m1" p="80 30"/>
      <n id="m2" p="100 30"/>
      <n id="m3" p="62.68 20"/>
      <n id="m4" p="117.32 40"/>
      <b id="m5" B="m1" E="m2" Order="2"/>
      <b id="m6" B="m1" E="m3"/>
      <b id="m7" B="m2" E="m4"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("auto double")).expect("cdxml should parse");
    let mut placements = Vec::new();
    for fragment in document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
    {
        for bond in &fragment.bonds {
            if bond.order == 2 {
                placements.push((
                    bond.id.as_str(),
                    bond.double.as_ref().map(|double| double.placement),
                    bond.double.as_ref().map(|double| double.frozen),
                ));
            }
        }
    }
    assert!(
        placements.contains(&(
            "b1",
            Some(chemcore_engine::DoubleBondPlacement::Center),
            Some(false)
        )),
        "{placements:?}"
    );
    assert!(
        placements.contains(&(
            "m5",
            Some(chemcore_engine::DoubleBondPlacement::Right),
            Some(false)
        )),
        "{placements:?}"
    );
}

#[test]
fn parse_cdxml_auto_double_bond_places_five_member_ring_inside() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="20.00" BondSpacing="18" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" LabelSize="10" BoundingBox="0 0 50 60">
  <page id="p1" BoundingBox="0 0 50 60">
    <fragment id="f1" BoundingBox="0 0 50 60">
      <n id="n1" p="10 20"/>
      <n id="n2" p="24.4 20"/>
      <n id="n3" p="31.25 33.65"/>
      <n id="n4" p="17.2 44"/>
      <n id="n5" p="3.15 33.65"/>
      <b id="b1" B="n1" E="n2" Order="2"/>
      <b id="b2" B="n2" E="n3"/>
      <b id="b3" B="n3" E="n4"/>
      <b id="b4" B="n4" E="n5"/>
      <b id="b5" B="n5" E="n1"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("cyclopentene")).expect("cdxml should parse");
    let fragment = document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment;
    let bond = fragment
        .bonds
        .iter()
        .find(|bond| bond.id == "b1")
        .expect("ring double bond should import");

    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(chemcore_engine::DoubleBondPlacement::Left)
    );
    assert_eq!(
        bond.double.as_ref().map(|double| double.frozen),
        Some(false)
    );
}

#[test]
fn parse_cdxml_auto_double_bond_prefers_alternating_ring_over_short_fused_cycle() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="40 240 90 305">
    <fragment id="f1" BoundingBox="40 240 90 305">
      <n id="a" p="62.90 267.82"/>
      <n id="b" p="76.43 272.76"/>
      <n id="c" p="51.87 277.07"/>
      <n id="d" p="54.35 291.25"/>
      <n id="e" p="67.88 296.19"/>
      <n id="f" p="78.92 286.94"/>
      <n id="g" p="63.42 253.43"/>
      <n id="h" p="77.27 249.48"/>
      <n id="i" p="85.31 261.42"/>
      <b id="target" B="a" E="b" Order="2"/>
      <b id="outer1" B="a" E="c"/>
      <b id="outer2" B="c" E="d" Order="2"/>
      <b id="outer3" B="d" E="e"/>
      <b id="outer4" B="e" E="f" Order="2"/>
      <b id="outer5" B="f" E="b"/>
      <b id="short1" B="a" E="g"/>
      <b id="short2" B="g" E="h"/>
      <b id="short3" B="h" E="i"/>
      <b id="short4" B="i" E="b"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("fused alternating")).expect("cdxml should parse");
    let fragment = document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment;
    let bond = fragment
        .bonds
        .iter()
        .find(|bond| bond.id == "target")
        .expect("target double bond should import");

    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(chemcore_engine::DoubleBondPlacement::Left)
    );
}

#[test]
fn parse_cdxml_attached_atom_label_preserves_source_bbox_size() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 40 24">
    <fragment id="f1" BoundingBox="0 0 40 24">
      <n id="n1" p="10 12" Element="7">
        <t p="6.40 15.90" BoundingBox="6.40 7.56 13.62 15.90" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <n id="n2" p="24 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("atom label bbox")).expect("cdxml should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "n1"))
        .and_then(|node| node.label.as_ref())
        .expect("N label should import");
    let bbox = label.bbox().expect("N label should keep bbox");
    assert!(
        ((bbox[3] - bbox[1]) - 8.34).abs() < 0.01,
        "attached CDXML atom labels should keep ChemDraw bbox height, got {bbox:?}"
    );
    assert!(
        !label.glyph_polygons.is_empty(),
        "refresh should still populate glyph polygons for clipping"
    );
}

#[test]
fn render_cdxml_single_character_atom_label_uses_text_primitive() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 40 24">
    <fragment id="f1" BoundingBox="0 0 40 24">
      <n id="n1" p="10 12" Element="7">
        <t p="6.40 15.90" BoundingBox="6.40 7.56 13.62 15.90" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <n id="n2" p="24 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("single atom label")).expect("cdxml should parse");
    let primitives = render_document(&document);
    let text = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                node_id,
                role,
                x,
                y,
                runs,
                text_anchor,
                ..
            } if node_id.as_deref() == Some("n1") && *role == RenderRole::DocumentText => {
                Some((*x, *y, runs.clone(), text_anchor.clone()))
            }
            _ => None,
        })
        .expect("N label should render as text");

    assert!((text.0 - 6.4).abs() < 0.001, "{text:?}");
    assert!((text.1 - 15.63).abs() < 0.001, "{text:?}");
    assert_eq!(
        text.2
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "N"
    );
    assert_eq!(text.3.as_deref(), Some("start"));
    assert!(!primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::FilledPath {
            node_id,
            role,
            ..
        } if node_id.as_deref() == Some("n1") && *role == RenderRole::DocumentText
    )));
}

#[test]
fn render_cdxml_imported_atom_label_uses_text_primitive() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 48 24">
    <fragment id="f1" BoundingBox="0 0 48 24">
      <n id="n1" p="10 12" Element="7">
        <t p="6.40 15.90" BoundingBox="6.40 7.56 21.60 15.90" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">NH</s>
        </t>
      </n>
      <n id="n2" p="30 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("multi atom label")).expect("cdxml");
    let primitives = render_document(&document);
    let text = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                node_id,
                role,
                runs,
                ..
            } if node_id.as_deref() == Some("n1") && *role == RenderRole::DocumentText => {
                Some(runs.clone())
            }
            _ => None,
        })
        .expect("NH label should render as text");

    assert_eq!(
        text.iter().map(|run| run.text.as_str()).collect::<String>(),
        "NH"
    );
    assert!(!primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::FilledPath {
            node_id,
            role,
            ..
        } if node_id.as_deref() == Some("n1") && *role == RenderRole::DocumentText
    )));
}

#[test]
fn parse_cdxml_right_aligned_chemical_node_label_reverses_display_groups() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 44 24">
    <fragment id="f1" BoundingBox="0 0 44 24">
      <n id="n1" p="22 12" Element="6">
        <t p="22.00 15.90" BoundingBox="10.00 7.56 22.00 15.90" LabelJustification="Right">
          <s font="3" size="10" color="0" face="96">CN</s>
        </t>
      </n>
      <n id="n2" p="34 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("right aligned label")).expect("cdxml should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "n1"))
        .and_then(|node| node.label.as_ref())
        .expect("CN label should import");

    assert_eq!(label.align.as_deref(), Some("right"));
    assert_eq!(label.source_text.as_deref(), Some("CN"));
    assert_eq!(label.text, "NC");
    let display_text: String = label.runs.iter().map(|run| run.text.as_str()).collect();
    assert_eq!(display_text, "NC");
    assert_eq!(
        label
            .meta
            .pointer("/sourceRuns/0/text")
            .and_then(serde_json::Value::as_str),
        Some("CN")
    );
}

#[test]
fn parse_cdxml_attached_sulfur_label_uses_elliptical_clip_geometry() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 40 24">
    <fragment id="f1" BoundingBox="0 0 40 24">
      <n id="n1" p="10 12" Element="16">
        <t p="6.40 15.90" BoundingBox="6.40 7.56 13.10 15.90" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">S</s>
        </t>
      </n>
      <n id="n2" p="24 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("sulfur ellipse clip")).expect("cdxml should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "n1"))
        .and_then(|node| node.label.as_ref())
        .expect("S label should import");

    assert!(
        !label.glyph_polygons.is_empty(),
        "sulfur label should populate glyph polygons"
    );
    assert!(
        label
            .glyph_polygons
            .iter()
            .any(|polygon| polygon.len() >= 16),
        "sulfur clipping should include an ellipse-like polygon for S; text={:?}, polygons={:?}",
        label.text,
        label.glyph_polygons
    );
}

#[test]
fn parse_cdxml_double_bond_spacing_uses_bond_spacing_percent() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" BondSpacing="18" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <fragment id="2" BoundingBox="9 8 26 12">
      <n id="3" p="10 10"/>
      <n id="4" p="24.4 10"/>
      <b id="5" B="3" E="4" Order="2"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("spacing")).expect("cdxml should parse");
    let primitives = render_document(&document);
    let mut center_ys: Vec<f64> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_mol_001") =>
            {
                bond_axis_from_points(points).map(|(from, to)| (from.y + to.y) * 0.5)
            }
            _ => None,
        })
        .collect();
    center_ys.sort_by(f64::total_cmp);

    assert_eq!(center_ys.len(), 2, "{center_ys:?}");
    let center_distance = center_ys[1] - center_ys[0];
    assert!(
        (center_distance - 14.4 * 0.18).abs() < 0.001,
        "{center_distance}"
    );
}

#[test]
fn parse_cdxml_node_labels_use_internal_attached_layout() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 120 120">
      <n id="3" p="50 50" Element="7">
        <t id="30" p="0 0" BoundingBox="0 0 100 100" UTF8Text="NH">
          <s font="3" size="10" color="0">NH</s>
        </t>
      </n>
      <n id="4" p="42 65"/>
      <n id="5" p="58 65"/>
      <b id="6" B="3" E="4"/>
      <b id="7" B="3" E="5"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("labels")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("import should create molecule fragment resource");
    let node = fragment
        .nodes
        .iter()
        .find(|node| node.id == "3")
        .expect("labeled node should import");
    let label = node.label.as_ref().expect("node label should import");

    assert_eq!(node.position, [50.0, 50.0]);
    assert_eq!(label.attachment.as_deref(), Some("node"));
    assert_eq!(label.anchor.as_deref(), Some("start"));
    assert_eq!(label.lines, vec!["H".to_string(), "N".to_string()]);
    assert_eq!(label.layout.as_deref(), Some("attached-group-above"));
    assert!(
        !label.glyph_polygons.is_empty(),
        "internal glyph geometry should be generated"
    );
    let box_value = label.box_value.expect("internal label box should exist");
    assert!(
        box_value[2] - box_value[0] < 30.0 && box_value[3] - box_value[1] < 30.0,
        "{box_value:?}"
    );
    assert_ne!(box_value, [0.0, 0.0, 100.0, 100.0]);

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("Element=\"7\""), "{exported}");
    assert!(exported.contains("NumHydrogens=\"1\""), "{exported}");
    assert!(exported.contains("LabelAlignment=\"Above\""), "{exported}");
    assert!(exported.contains("LineStarts=\"2 4\""), "{exported}");
    assert!(exported.contains("face=\"96\""), "{exported}");

    let reimported =
        parse_cdxml_document(&exported, Some("labels export")).expect("export should parse");
    let reimported_fragment = reimported
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("reimport should create molecule fragment resource");
    let reimported_node = reimported_fragment
        .nodes
        .iter()
        .find(|node| node.atomic_number == 7)
        .expect("nitrogen node should reimport as nitrogen");
    let reimported_label = reimported_node
        .label
        .as_ref()
        .expect("nitrogen label should reimport");
    assert_eq!(reimported_node.num_hydrogens, 1);
    assert_eq!(
        reimported_label.lines,
        vec!["H".to_string(), "N".to_string()]
    );
    assert_eq!(
        reimported_label.layout.as_deref(),
        Some("attached-group-above")
    );
}

#[test]
fn parse_cdxml_matches_default_and_acs_double_bond_spacing_samples() {
    for (fixture, expected_normal, expected_bold, expected_widths) in [
        ("db.cdxml", 3.6, 5.1, [1.0, 4.0]),
        ("db-acs.cdxml", 2.592, 3.292, [0.6, 2.0]),
    ] {
        let Some(cdxml) = read_optional_cdxml_fixture(fixture) else {
            continue;
        };
        let document = parse_cdxml_document(&cdxml, Some(fixture)).expect("cdxml should parse");
        let primitives = render_document(&document);

        let normal = imported_vertical_line_metrics(&primitives, "obj_mol_001");
        assert_line_spacing(&normal, expected_normal, fixture);
        assert_line_widths(&normal, expected_widths[0], expected_widths[0], fixture);

        let dashed_solid = imported_vertical_line_metrics(&primitives, "obj_mol_002");
        assert_line_spacing(&dashed_solid, expected_normal, fixture);
        let dashed_solid_bond = imported_fragment_bond(&document, "obj_mol_002", "9");
        assert_eq!(dashed_solid_bond.order, 2);
        assert_eq!(
            dashed_solid_bond.line_styles.right,
            chemcore_engine::BondLinePattern::Dashed
        );

        let bold = imported_vertical_line_metrics(&primitives, "obj_mol_003");
        assert_line_spacing(&bold, expected_bold, fixture);
        assert_line_widths(&bold, expected_widths[0], expected_widths[1], fixture);

        let dashed = imported_vertical_line_metrics(&primitives, "obj_mol_004");
        assert_line_spacing(&dashed, expected_normal, fixture);
        let dashed_bond = imported_fragment_bond(&document, "obj_mol_004", "17");
        assert_eq!(dashed_bond.order, 2);
        assert_eq!(
            dashed_bond.line_styles.left,
            chemcore_engine::BondLinePattern::Dashed
        );
        assert_eq!(
            dashed_bond.line_styles.right,
            chemcore_engine::BondLinePattern::Dashed
        );
    }
}

#[test]
fn parse_cdxml_double_bond_spacing_scales_with_actual_bond_length() {
    for (fixture, expected_spacings) in [
        (
            "db-chang.cdxml",
            [
                ("obj_mol_001", 9.0002),
                ("obj_mol_002", 12.8413),
                ("obj_mol_003", 14.5250),
                ("obj_mol_004", 9.5205),
            ],
        ),
        (
            "db-acs-chang.cdxml",
            [
                ("obj_mol_001", 4.7411),
                ("obj_mol_002", 5.7277),
                ("obj_mol_003", 5.9441),
                ("obj_mol_004", 5.2895),
            ],
        ),
    ] {
        let Some(cdxml) = read_optional_cdxml_fixture(fixture) else {
            continue;
        };
        let document = parse_cdxml_document(&cdxml, Some(fixture)).expect("cdxml should parse");

        for (object_id, expected) in expected_spacings {
            let rendered = imported_double_bond_center_spacing(&document, object_id);
            let formula = imported_double_bond_formula_spacing(&document, object_id);
            assert!(
                (rendered - expected).abs() < 0.01,
                "{fixture} {object_id}: expected {expected}, rendered {rendered}"
            );
            assert!(
                (formula - expected).abs() < 0.01,
                "{fixture} {object_id}: expected {expected}, formula {formula}"
            );
        }
    }
}

fn imported_fragment_bond<'a>(
    document: &'a ChemcoreDocument,
    object_id: &str,
    bond_id: &str,
) -> &'a chemcore_engine::Bond {
    let object = document
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .expect("imported molecule object should exist");
    let resource_ref = object
        .payload
        .resource_ref
        .as_deref()
        .expect("molecule object should reference fragment resource");
    let fragment = document
        .resources
        .get(resource_ref)
        .and_then(|resource| resource.data.as_fragment())
        .expect("fragment resource should exist");
    fragment
        .bonds
        .iter()
        .find(|bond| bond.id == bond_id)
        .expect("bond should exist")
}

fn imported_double_bond_center_spacing(document: &ChemcoreDocument, object_id: &str) -> f64 {
    let object = document
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .expect("imported molecule object should exist");
    let resource_ref = object
        .payload
        .resource_ref
        .as_deref()
        .expect("molecule object should reference fragment resource");
    let fragment = document
        .resources
        .get(resource_ref)
        .and_then(|resource| resource.data.as_fragment())
        .expect("fragment resource should exist");
    let bond = fragment
        .bonds
        .first()
        .expect("fixture fragment has one bond");
    let begin = fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.begin)
        .expect("begin node should exist");
    let end = fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.end)
        .expect("end node should exist");
    let begin = chemcore_engine::Point::new(
        object.transform.translate[0] + begin.position[0],
        object.transform.translate[1] + begin.position[1],
    );
    let end = chemcore_engine::Point::new(
        object.transform.translate[0] + end.position[0],
        object.transform.translate[1] + end.position[1],
    );
    let dx = end.x - begin.x;
    let dy = end.y - begin.y;
    let length = dx.hypot(dy);
    let normal = chemcore_engine::Point::new(-dy / length, dx / length);
    let mut projections: Vec<f64> = render_document(document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id: primitive_object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond
                && primitive_object_id.as_deref() == Some(object_id) =>
            {
                let projection = points
                    .iter()
                    .map(|point| point.x * normal.x + point.y * normal.y)
                    .sum::<f64>()
                    / points.len() as f64;
                Some(projection)
            }
            _ => None,
        })
        .collect();
    projections.sort_by(f64::total_cmp);
    assert!(
        projections.len() >= 2,
        "{object_id}: expected at least two rendered bond line polygons"
    );
    let split = projections
        .windows(2)
        .enumerate()
        .max_by(|(_, left), (_, right)| {
            (left[1] - left[0])
                .abs()
                .total_cmp(&(right[1] - right[0]).abs())
        })
        .map(|(index, _)| index + 1)
        .expect("projection split should exist");
    let first = projections[..split].iter().sum::<f64>() / split as f64;
    let second = projections[split..].iter().sum::<f64>() / (projections.len() - split) as f64;
    (second - first).abs()
}

fn imported_double_bond_formula_spacing(document: &ChemcoreDocument, object_id: &str) -> f64 {
    let object = document
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .expect("imported molecule object should exist");
    let resource_ref = object
        .payload
        .resource_ref
        .as_deref()
        .expect("molecule object should reference fragment resource");
    let fragment = document
        .resources
        .get(resource_ref)
        .and_then(|resource| resource.data.as_fragment())
        .expect("fragment resource should exist");
    let bond = fragment
        .bonds
        .first()
        .expect("fixture fragment has one bond");
    let begin = fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.begin)
        .expect("begin node should exist");
    let end = fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.end)
        .expect("end node should exist");
    let length = chemcore_engine::Point::new(begin.position[0], begin.position[1]).distance(
        chemcore_engine::Point::new(end.position[0], end.position[1]),
    );
    let ratio = bond
        .bond_spacing
        .expect("cdxml fixture should import bond spacing")
        / 100.0;
    let stroke_width = bond.stroke_width;
    let line_width = |weight| {
        if weight == chemcore_engine::BondLineWeight::Bold {
            bond.bold_width.unwrap_or(stroke_width).max(stroke_width)
        } else {
            stroke_width
        }
    };
    let first_width = line_width(bond.line_weights.left);
    let second_width = line_width(bond.line_weights.right);
    (length * ratio - stroke_width).max(stroke_width * 0.5) + 0.5 * (first_width + second_width)
}

fn imported_vertical_line_metrics(
    primitives: &[RenderPrimitive],
    object_id: &str,
) -> Vec<(f64, f64)> {
    let mut metrics: Vec<(f64, f64)> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id: primitive_object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && primitive_object_id.as_deref() == Some(object_id) =>
            {
                let (from, to) = bond_axis_from_points(points)?;
                let center_x = (from.x + to.x) * 0.5;
                let min_x = points
                    .iter()
                    .map(|point| point.x)
                    .fold(f64::INFINITY, f64::min);
                let max_x = points
                    .iter()
                    .map(|point| point.x)
                    .fold(f64::NEG_INFINITY, f64::max);
                Some((center_x, max_x - min_x))
            }
            _ => None,
        })
        .collect();
    metrics.sort_by(|a, b| a.0.total_cmp(&b.0));
    metrics
}

fn assert_line_spacing(metrics: &[(f64, f64)], expected: f64, context: &str) {
    assert_eq!(metrics.len(), 2, "{context}: {metrics:?}");
    let actual = metrics[1].0 - metrics[0].0;
    assert!(
        (actual - expected).abs() < 0.001,
        "{context}: expected {expected}, got {actual}, metrics={metrics:?}"
    );
}

fn assert_line_widths(
    metrics: &[(f64, f64)],
    expected_left: f64,
    expected_right: f64,
    context: &str,
) {
    assert_eq!(metrics.len(), 2, "{context}: {metrics:?}");
    assert!(
        (metrics[0].1 - expected_left).abs() < 0.001,
        "{context}: expected left width {expected_left}, got {}, metrics={metrics:?}",
        metrics[0].1
    );
    assert!(
        (metrics[1].1 - expected_right).abs() < 0.001,
        "{context}: expected right width {expected_right}, got {}, metrics={metrics:?}",
        metrics[1].1
    );
}

fn projection_range_on_axis(
    points: &[chemcore_engine::Point],
    axis_from: chemcore_engine::Point,
    axis_to: chemcore_engine::Point,
) -> Option<(f64, f64)> {
    let axis = chemcore_engine::Point::new(axis_to.x - axis_from.x, axis_to.y - axis_from.y);
    let length = (axis.x * axis.x + axis.y * axis.y).sqrt();
    if length <= 1.0e-6 {
        return None;
    }
    let unit_x = axis.x / length;
    let unit_y = axis.y / length;
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    for point in points {
        let projection = (point.x - axis_from.x) * unit_x + (point.y - axis_from.y) * unit_y;
        min_value = min_value.min(projection);
        max_value = max_value.max(projection);
    }
    Some((min_value, max_value))
}

fn object_bond_centerlines(
    primitives: &[RenderPrimitive],
) -> Vec<(chemcore_engine::Point, chemcore_engine::Point)> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Line {
                role,
                object_id,
                from,
                to,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some((*from, *to))
            }
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                bond_axis_from_points(points)
            }
            _ => None,
        })
        .collect()
}

fn shared_point_count(
    first: &[chemcore_engine::Point],
    second: &[chemcore_engine::Point],
    tolerance: f64,
) -> usize {
    first
        .iter()
        .filter(|point| {
            second
                .iter()
                .any(|other| point.distance(*other) <= tolerance)
        })
        .count()
}

fn point_lies_on_segment(
    point: chemcore_engine::Point,
    from: chemcore_engine::Point,
    to: chemcore_engine::Point,
    tolerance: f64,
) -> bool {
    let cross = (point.x - from.x) * (to.y - from.y) - (point.y - from.y) * (to.x - from.x);
    if cross.abs() > tolerance {
        return false;
    }
    let dot = (point.x - from.x) * (to.x - from.x) + (point.y - from.y) * (to.y - from.y);
    if dot < -tolerance {
        return false;
    }
    let length_squared = (to.x - from.x).powi(2) + (to.y - from.y).powi(2);
    dot <= length_squared + tolerance
}

fn polygons_have_same_vertices(
    first: &[chemcore_engine::Point],
    second: &[chemcore_engine::Point],
    tolerance: f64,
) -> bool {
    first.len() == second.len()
        && first.iter().all(|point| {
            second
                .iter()
                .any(|other| point.distance(*other) <= tolerance)
        })
}

fn point_lies_on_polygon_boundary(
    point: chemcore_engine::Point,
    polygon: &[chemcore_engine::Point],
    tolerance: f64,
) -> bool {
    if polygon.len() < 2 {
        return false;
    }
    (0..polygon.len()).any(|index| {
        let next = (index + 1) % polygon.len();
        point_lies_on_segment(point, polygon[index], polygon[next], tolerance)
    })
}

fn average_closest_distance_to_point(
    points: &[chemcore_engine::Point],
    target: chemcore_engine::Point,
    count: usize,
) -> f64 {
    let mut distances: Vec<_> = points.iter().map(|point| point.distance(target)).collect();
    distances.sort_by(|a, b| a.total_cmp(b));
    distances.into_iter().take(count).sum::<f64>() / count as f64
}

fn side_double_outer_polygon_for_bond(
    polygons: &[(String, Vec<chemcore_engine::Point>)],
    bond_id: &str,
    shared_node: chemcore_engine::Point,
) -> Vec<chemcore_engine::Point> {
    polygons
        .iter()
        .filter(|(current_bond_id, _)| current_bond_id == bond_id)
        .max_by(|a, b| {
            average_closest_distance_to_point(&a.1, shared_node, 2)
                .total_cmp(&average_closest_distance_to_point(&b.1, shared_node, 2))
        })
        .map(|(_, points)| points.clone())
        .expect("side double outer polygon")
}

fn side_double_main_polygon_for_bond(
    polygons: &[(String, Vec<chemcore_engine::Point>)],
    bond_id: &str,
    shared_node: chemcore_engine::Point,
) -> Vec<chemcore_engine::Point> {
    polygons
        .iter()
        .filter(|(current_bond_id, _)| current_bond_id == bond_id)
        .min_by(|a, b| {
            average_closest_distance_to_point(&a.1, shared_node, 2)
                .total_cmp(&average_closest_distance_to_point(&b.1, shared_node, 2))
        })
        .map(|(_, points)| points.clone())
        .expect("side double main polygon")
}

fn closest_points_to_target(
    points: &[chemcore_engine::Point],
    target: chemcore_engine::Point,
    count: usize,
) -> Vec<chemcore_engine::Point> {
    let mut indexed: Vec<_> = points.iter().copied().collect();
    indexed.sort_by(|a, b| a.distance(target).total_cmp(&b.distance(target)));
    indexed.into_iter().take(count).collect()
}

#[test]
#[ignore = "backend debug output for bond vertices"]
fn debug_print_single_and_bold_bond_vertices() {
    let single_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
        ]),
    );
    let bold_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );

    let single_polygons = object_bond_polygons(&render_document(&single_document));
    let bold_polygons = object_bond_polygons(&render_document(&bold_document));

    let single = single_polygons
        .iter()
        .find(|points| points.len() == 4)
        .expect("single bond should render as one 4-point polygon");
    let bold = bold_polygons
        .iter()
        .find(|points| points.len() == 4)
        .expect("bold bond should render as one 4-point polygon");

    println!("single bond vertices:");
    for (index, point) in single.iter().enumerate() {
        println!("  p{} = ({:.6}, {:.6})", index + 1, point.x, point.y);
    }

    println!("bold bond vertices:");
    for (index, point) in bold.iter().enumerate() {
        println!("  p{} = ({:.6}, {:.6})", index + 1, point.x, point.y);
    }
}

#[test]
#[ignore = "backend debug output for joined bond vertices"]
fn debug_print_joined_single_and_bold_bond_vertices() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            },
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let polygons = object_bond_polygons(&render_document(&document));
    assert_eq!(
        polygons.len(),
        2,
        "joined single+bold should render as two 4-point polygons"
    );

    let first = polygons
        .iter()
        .find(|points| points.iter().any(|point| (point.x - 20.0).abs() < 0.001))
        .expect("first bond polygon");
    let second = polygons
        .iter()
        .find(|points| points.iter().any(|point| (point.x - 74.0).abs() < 1.0))
        .expect("second bond polygon");

    let mut shared = Vec::new();
    for first_point in first {
        for second_point in second {
            if first_point.distance(*second_point) <= 1.0e-6 {
                shared.push(*first_point);
            }
        }
    }

    println!("joined bold bond vertices:");
    for (index, point) in first.iter().enumerate() {
        println!("  p{} = ({:.6}, {:.6})", index + 1, point.x, point.y);
    }

    println!("joined single bond vertices:");
    for (index, point) in second.iter().enumerate() {
        println!("  p{} = ({:.6}, {:.6})", index + 1, point.x, point.y);
    }

    println!("shared vertices:");
    for (index, point) in shared.iter().enumerate() {
        println!("  s{} = ({:.6}, {:.6})", index + 1, point.x, point.y);
    }
}

#[test]
fn render_document_emits_arrow_line_primitives() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_default": {
                "kind": "stroke",
                "stroke": "#222222",
                "strokeWidth": 0.72,
                "lineCap": "butt",
                "lineJoin": "miter"
            }
        },
        "objects": [{
            "id": "obj_line_001",
            "type": "line",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_arrow_default",
            "payload": {
                "points": [[10.0, 20.0], [110.0, 20.0]],
                "head": "end",
                "tail": "none",
                "arrowHead": {
                    "kind": "solid",
                    "length": 22.5,
                    "centerLength": 19.69,
                    "width": 5.63,
                    "curve": 0.0,
                    "head": "full",
                    "tail": "full",
                    "bold": false,
                    "noGo": "none"
                }
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let shaft = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polyline {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_line_001") =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .expect("line shaft primitive");
    assert_eq!(shaft.len(), 2);
    assert!(shaft[1].x < 110.0);

    let arrow_head_paths: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                d,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_line_001")
                && points.len() == 6 =>
            {
                Some((points.clone(), d.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(arrow_head_paths.len(), 2);
    assert!(arrow_head_paths[0].1.contains(" C "));
    let head_width = arrow_head_paths[0]
        .0
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max)
        - arrow_head_paths[0]
            .0
            .iter()
            .map(|point| point.y)
            .fold(f64::INFINITY, f64::min);
    assert!((head_width - 8.2072).abs() <= 0.001);
}

#[test]
fn render_document_rounds_inner_curved_half_arrow_heads() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 100.0, "height": 60.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_default": {
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": 1.0,
                "lineCap": "butt",
                "lineJoin": "miter"
            }
        },
        "objects": [{
            "id": "obj_line_001",
            "type": "line",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_arrow_default",
            "payload": {
                "points": [[40.0, 20.0], [60.0, 20.0]],
                "head": "end",
                "tail": "none",
                "arrowHead": {
                    "kind": "solid",
                    "length": 10.0,
                    "centerLength": 8.75,
                    "width": 2.5,
                    "curve": -120.0,
                    "head": "half-right",
                    "tail": "none",
                    "bold": false,
                    "noGo": "none"
                },
                "arrowGeometry": {
                    "center": [50.0, 25.77],
                    "majorAxisEnd": [61.55, 25.77],
                    "minorAxisEnd": [50.0, 37.32]
                }
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let shaft_end = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Path {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_line_001") =>
            {
                points.last().copied()
            }
            _ => None,
        })
        .expect("inner curved half arrow shaft path");
    let half_head_points = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_line_001")
                && points.len() == 4 =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .expect("inner curved half arrow head path");

    let cut_edge = half_head_points[3];
    assert!(
        shaft_end.distance(cut_edge) <= 0.65,
        "inner curved half-arrow shaft should stop at the head cut edge, shaft={shaft_end:?}, head={half_head_points:?}"
    );
}

#[test]
fn render_document_uses_open_arrow_width_as_extra_head_width() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_default": {
                "kind": "stroke",
                "stroke": "#222222",
                "strokeWidth": 0.72,
                "lineCap": "butt",
                "lineJoin": "miter"
            }
        },
        "objects": [{
            "id": "obj_line_001",
            "type": "line",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_arrow_default",
            "payload": {
                "points": [[10.0, 20.0], [110.0, 20.0]],
                "head": "end",
                "tail": "none",
                "arrowHead": {
                    "kind": "hollow",
                    "length": 12.0,
                    "centerLength": 12.0,
                    "width": 3.0,
                    "curve": 0.0,
                    "head": "full",
                    "tail": "none",
                    "bold": false,
                    "noGo": "none"
                }
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let outline = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_line_001")
                && points.len() > 4 =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .expect("hollow arrow outline polygon");
    let outline_width = outline
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max)
        - outline
            .iter()
            .map(|point| point.y)
            .fold(f64::INFINITY, f64::min);
    assert!((outline_width - 17.28).abs() <= 0.001);
}

#[test]
fn render_document_respects_thin_open_and_hollow_arrow_stroke_width() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_thin": {
                "kind": "stroke",
                "stroke": "#222222",
                "strokeWidth": 0.6,
                "lineCap": "butt",
                "lineJoin": "miter"
            }
        },
        "objects": [
            {
                "id": "obj_hollow",
                "type": "line",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_arrow_thin",
                "payload": {
                    "points": [[10.0, 20.0], [110.0, 20.0]],
                    "head": "end",
                    "tail": "none",
                    "arrowHead": {
                        "kind": "hollow",
                        "length": 12.0,
                        "centerLength": 12.0,
                        "width": 3.0,
                        "curve": 0.0,
                        "head": "full",
                        "tail": "none",
                        "bold": false,
                        "noGo": "none"
                    }
                }
            },
            {
                "id": "obj_open",
                "type": "line",
                "visible": true,
                "zIndex": 11,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_arrow_thin",
                "payload": {
                    "points": [[10.0, 80.0], [110.0, 80.0]],
                    "head": "end",
                    "tail": "none",
                    "arrowHead": {
                        "kind": "open",
                        "length": 12.0,
                        "centerLength": 12.0,
                        "width": 3.0,
                        "curve": 0.0,
                        "head": "full",
                        "tail": "none",
                        "bold": false,
                        "noGo": "none"
                    }
                }
            }
        ],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let hollow_width = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                stroke_width,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_hollow") =>
            {
                Some(*stroke_width)
            }
            _ => None,
        })
        .expect("hollow arrow outline");
    assert!((hollow_width - 0.6).abs() <= 1.0e-6, "{hollow_width}");

    let open_widths: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polyline {
                role,
                object_id,
                stroke_width,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_open") =>
            {
                Some(*stroke_width)
            }
            _ => None,
        })
        .collect();
    assert!(!open_widths.is_empty());
    assert!(
        open_widths
            .iter()
            .all(|width| (*width - 0.6).abs() <= 1.0e-6),
        "{open_widths:?}"
    );
}

#[test]
fn cdxml_acs_hollow_and_open_arrows_keep_chemdraw_head_width() {
    let Some(arrows) = read_optional_cdxml_fixture("arrows-acs.cdxml") else {
        return;
    };
    let document = parse_cdxml_document(&arrows, Some("arrows")).expect("arrows should parse");
    let primitives = render_document(&document);

    for (object_id, expected_height) in [
        ("obj_line_004", 14.4),
        ("obj_line_005", 7.2),
        ("obj_line_006", 14.4),
        ("obj_line_007", 7.2),
    ] {
        let height = primitives
            .iter()
            .filter_map(|primitive| match primitive {
                RenderPrimitive::Polygon {
                    role,
                    object_id: Some(id),
                    points,
                    ..
                }
                | RenderPrimitive::Polyline {
                    role,
                    object_id: Some(id),
                    points,
                    ..
                } if *role == RenderRole::DocumentGraphic && id == object_id => Some(
                    points
                        .iter()
                        .map(|point| point.y)
                        .fold(f64::NEG_INFINITY, f64::max)
                        - points
                            .iter()
                            .map(|point| point.y)
                            .fold(f64::INFINITY, f64::min),
                ),
                _ => None,
            })
            .fold(0.0, f64::max);

        assert!(
            (height - expected_height).abs() <= 0.001,
            "{object_id} height {height}"
        );
    }
}

#[test]
fn cdxml_import_preserves_hollow_and_open_arrow_dimensions() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="0.60">
  <page id="1">
    <arrow id="1" ArrowheadHead="Full" ArrowheadType="Hollow" HeadSize="1200" ArrowheadCenterSize="1200" ArrowheadWidth="300" ArrowShaftSpacing="1200" Head3D="110 20 0" Tail3D="10 20 0"/>
    <arrow id="2" ArrowheadHead="Full" ArrowheadType="Hollow" HeadSize="600" ArrowheadCenterSize="600" ArrowheadWidth="150" ArrowShaftSpacing="600" Head3D="110 50 0" Tail3D="10 50 0"/>
    <arrow id="3" ArrowheadHead="Full" ArrowheadType="Hollow" HeadSize="900" ArrowheadCenterSize="875" ArrowheadWidth="225" ArrowShaftSpacing="875" Head3D="110 80 0" Tail3D="10 80 0"/>
    <arrow id="4" ArrowheadHead="Full" ArrowheadType="Angle" HeadSize="1200" ArrowheadCenterSize="1200" ArrowheadWidth="300" ArrowShaftSpacing="1200" Head3D="110 110 0" Tail3D="10 110 0"/>
    <arrow id="5" ArrowheadHead="Full" ArrowheadType="Angle" HeadSize="600" ArrowheadCenterSize="600" ArrowheadWidth="150" ArrowShaftSpacing="600" Head3D="110 140 0" Tail3D="10 140 0"/>
    <arrow id="6" ArrowheadHead="Full" ArrowheadType="Angle" HeadSize="900" ArrowheadCenterSize="875" ArrowheadWidth="225" ArrowShaftSpacing="875" Head3D="110 170 0" Tail3D="10 170 0"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("hollow-open-sizes"))
        .expect("CDXML hollow/open arrows should parse");
    let arrow_head_for = |object_id: &str| {
        document
            .objects
            .iter()
            .find(|object| object.id == object_id)
            .and_then(|object| object.payload.extra.get("arrowHead"))
            .cloned()
            .expect("arrowHead payload")
    };
    for (object_id, expected_kind, expected_length, expected_center_length, expected_width) in [
        ("obj_line_001", "hollow", 12.0, 12.0, 3.0),
        ("obj_line_002", "hollow", 6.0, 6.0, 1.5),
        ("obj_line_003", "hollow", 9.0, 8.75, 2.25),
        ("obj_line_004", "open", 12.0, 12.0, 3.0),
        ("obj_line_005", "open", 6.0, 6.0, 1.5),
        ("obj_line_006", "open", 9.0, 8.75, 2.25),
    ] {
        let arrow_head = arrow_head_for(object_id);
        assert_eq!(
            arrow_head.get("kind").and_then(serde_json::Value::as_str),
            Some(expected_kind),
            "{object_id}"
        );
        assert_eq!(
            arrow_head.get("length").and_then(serde_json::Value::as_f64),
            Some(expected_length),
            "{object_id}"
        );
        assert_eq!(
            arrow_head
                .get("centerLength")
                .and_then(serde_json::Value::as_f64),
            Some(expected_center_length),
            "{object_id}"
        );
        assert_eq!(
            arrow_head.get("width").and_then(serde_json::Value::as_f64),
            Some(expected_width),
            "{object_id}"
        );
    }
}

#[test]
fn cdxml_imports_exports_and_renders_equilibrium_arrows() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="0 0 140 60">
    <arrow id="1" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="1500" ArrowheadCenterSize="1313" ArrowheadWidth="375" ArrowShaftSpacing="300"
      Head3D="110 30 0" Tail3D="10 30 0"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("equilibrium arrow"))
        .expect("CDXML equilibrium arrow should parse");
    let arrow = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow should import as line");
    let arrow_head = arrow
        .payload
        .extra
        .get("arrowHead")
        .expect("equilibrium arrow should carry arrowHead payload");
    assert_eq!(
        arrow_head.get("kind").and_then(serde_json::Value::as_str),
        Some("equilibrium")
    );
    assert_eq!(
        arrow_head.get("head").and_then(serde_json::Value::as_str),
        Some("half-left")
    );
    assert_eq!(
        arrow_head.get("tail").and_then(serde_json::Value::as_str),
        Some("half-left")
    );
    assert_eq!(
        arrow_head.get("length").and_then(serde_json::Value::as_f64),
        Some(15.0)
    );
    assert_eq!(
        arrow_head
            .get("centerLength")
            .and_then(serde_json::Value::as_f64),
        Some(13.13)
    );
    assert_eq!(
        arrow_head.get("width").and_then(serde_json::Value::as_f64),
        Some(3.75)
    );
    assert_eq!(
        arrow_head
            .get("shaftSpacing")
            .and_then(serde_json::Value::as_f64),
        Some(3.0)
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("ArrowheadType=\"Solid\""));
    assert!(exported.contains("ArrowShaftSpacing=\"300\""));
    assert!(!exported.contains("ArrowheadType=\"Equilibrium\""));

    let primitives: Vec<_> = render_document(&document)
        .into_iter()
        .filter(|primitive| match primitive {
            RenderPrimitive::Polyline { object_id, .. }
            | RenderPrimitive::FilledPath { object_id, .. } => {
                object_id.as_deref() == Some(&arrow.id)
            }
            _ => false,
        })
        .collect();
    assert_eq!(primitives.len(), 4);
    assert_eq!(
        primitives
            .iter()
            .filter(|primitive| matches!(primitive, RenderPrimitive::Polyline { .. }))
            .count(),
        2
    );
    assert_eq!(
        primitives
            .iter()
            .filter(|primitive| matches!(primitive, RenderPrimitive::FilledPath { .. }))
            .count(),
        2
    );
}

fn right_arrow_head_width_from_cdxml(cdxml: &str) -> f64 {
    let document =
        parse_cdxml_document(cdxml, Some("equilibrium arrow")).expect("CDXML arrow should parse");
    render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::FilledPath { points, .. } => {
                let bounds = primitive_polygon_bounds(&points);
                Some((bounds[2], bounds[2] - bounds[0]))
            }
            _ => None,
        })
        .max_by(|left, right| left.0.partial_cmp(&right.0).unwrap())
        .map(|(_, width)| (width * 100.0).round() / 100.0)
        .expect("arrow should render a filled head")
}

fn rounded_pair(points: &[Point]) -> ([f64; 2], [f64; 2]) {
    fn round2(value: f64) -> f64 {
        (value * 100.0).round() / 100.0
    }
    (
        [round2(points[0].x), round2(points[0].y)],
        [round2(points[1].x), round2(points[1].y)],
    )
}

#[test]
fn cdxml_equilibrium_arrow_heads_scale_with_axis_length_like_chemdraw() {
    let regular_short = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="0 0 240 80">
    <arrow id="1" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="2250" ArrowheadCenterSize="1969" ArrowheadWidth="563" ArrowShaftSpacing="300"
      Head3D="194.66 94.13 0" Tail3D="183.79 94.13 0"/>
  </page>
</CDXML>"#;
    let regular_full = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="0 0 300 80">
    <arrow id="1" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="2250" ArrowheadCenterSize="1969" ArrowheadWidth="563" ArrowShaftSpacing="300"
      Head3D="234.50 161.63 0" Tail3D="183.79 161.63 0"/>
  </page>
</CDXML>"#;
    let unequal_short = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="0 0 260 80">
    <arrow id="1" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="2250" ArrowheadCenterSize="1969" ArrowheadWidth="563" ArrowShaftSpacing="300"
      ArrowEquilibriumRatio="300" Head3D="208.54 370.50 0" Tail3D="195.79 370.50 0"/>
  </page>
</CDXML>"#;

    assert_eq!(right_arrow_head_width_from_cdxml(regular_short), 9.25);
    assert_eq!(right_arrow_head_width_from_cdxml(regular_full), 22.5);
    assert_eq!(right_arrow_head_width_from_cdxml(unequal_short), 8.5);
}

#[test]
fn cdxml_unequal_equilibrium_arrow_layout_matches_chemdraw() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="180 450 340 490">
    <arrow id="47" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="2250" ArrowheadCenterSize="1969" ArrowheadWidth="563" ArrowShaftSpacing="300"
      ArrowEquilibriumRatio="300" Head3D="314.80 468.75 0" Tail3D="198.79 468.75 0"/>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("unequal equilibrium arrow")).expect("CDXML arrow parses");
    let primitives = render_document(&document);
    let mut polylines: Vec<([f64; 2], [f64; 2])> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polyline { points, .. } => Some(rounded_pair(points)),
            _ => None,
        })
        .collect();
    polylines.sort_by(|left, right| {
        left.0[1]
            .partial_cmp(&right.0[1])
            .unwrap()
            .then(left.0[0].partial_cmp(&right.0[0]).unwrap())
    });
    assert_eq!(
        polylines,
        vec![
            ([198.79, 467.25], [296.11, 467.25]),
            ([282.36, 470.25], [249.92, 470.25]),
        ]
    );

    let mut head_bounds: Vec<[f64; 4]> = primitives
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::FilledPath { points, .. } => {
                let bounds = primitive_polygon_bounds(&points);
                Some([
                    (bounds[0] * 100.0).round() / 100.0,
                    (bounds[1] * 100.0).round() / 100.0,
                    (bounds[2] * 100.0).round() / 100.0,
                    (bounds[3] * 100.0).round() / 100.0,
                ])
            }
            _ => None,
        })
        .collect();
    head_bounds.sort_by(|left, right| left[0].partial_cmp(&right[0]).unwrap());
    assert_eq!(
        head_bounds,
        vec![
            [231.23, 469.75, 253.73, 475.88],
            [292.3, 461.62, 314.8, 467.75],
        ]
    );
}

#[test]
fn cdxml_imports_exports_and_renders_unequal_equilibrium_arrows() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="0 0 140 60">
    <arrow id="1" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="1500" ArrowheadCenterSize="1313" ArrowheadWidth="375" ArrowShaftSpacing="300"
      ArrowEquilibriumRatio="300" Head3D="110 30 0" Tail3D="10 30 0"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("unequal equilibrium arrow"))
        .expect("CDXML unequal equilibrium arrow should parse");
    let arrow = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow should import as line");
    let arrow_head = arrow
        .payload
        .extra
        .get("arrowHead")
        .expect("unequal equilibrium arrow should carry arrowHead payload");
    assert_eq!(
        arrow_head.get("kind").and_then(serde_json::Value::as_str),
        Some("unequal-equilibrium")
    );
    assert_eq!(
        arrow_head
            .get("equilibriumRatio")
            .and_then(serde_json::Value::as_f64),
        Some(3.0)
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("ArrowheadType=\"Solid\""));
    assert!(exported.contains("ArrowShaftSpacing=\"300\""));
    assert!(exported.contains("ArrowEquilibriumRatio=\"300\""));

    let mut branch_lengths: Vec<f64> = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polyline {
                object_id, points, ..
            } if object_id.as_deref() == Some(&arrow.id) => Some(
                points
                    .windows(2)
                    .map(|pair| pair[0].distance(pair[1]))
                    .sum::<f64>(),
            ),
            _ => None,
        })
        .collect();
    branch_lengths.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(branch_lengths.len(), 2);
    assert!(
        branch_lengths[0] < branch_lengths[1] * 0.45,
        "unequal equilibrium reverse branch should be much shorter: {branch_lengths:?}"
    );
}

#[test]
fn render_document_emits_arrow_no_go_marks_at_current_head_size() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_default": {
                "kind": "stroke",
                "stroke": "#222222",
                "strokeWidth": 0.72,
                "lineCap": "butt",
                "lineJoin": "miter"
            }
        },
        "objects": [
            {
                "id": "obj_line_001",
                "type": "line",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_arrow_default",
                "payload": {
                    "points": [[10.0, 20.0], [110.0, 20.0]],
                    "head": "end",
                    "tail": "none",
                    "arrowHead": {
                        "kind": "solid",
                        "length": 10.0,
                        "centerLength": 8.75,
                        "width": 2.5,
                        "curve": 0.0,
                        "head": "full",
                        "tail": "none",
                        "bold": false,
                        "noGo": "hash"
                    }
                }
            },
            {
                "id": "obj_line_002",
                "type": "line",
                "visible": true,
                "zIndex": 11,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_arrow_default",
                "payload": {
                    "points": [[10.0, 60.0], [110.0, 60.0]],
                    "head": "end",
                    "tail": "none",
                    "arrowHead": {
                        "kind": "solid",
                        "length": 10.0,
                        "centerLength": 8.75,
                        "width": 2.5,
                        "curve": 0.0,
                        "head": "full",
                        "tail": "none",
                        "bold": false,
                        "noGo": "cross"
                    }
                }
            }
        ],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let mark_lines_for = |object_id: &str| -> Vec<(Point, Point, f64)> {
        primitives
            .iter()
            .filter_map(|primitive| match primitive {
                RenderPrimitive::Line {
                    role,
                    object_id: primitive_object_id,
                    from,
                    to,
                    stroke_width,
                    ..
                } if *role == RenderRole::DocumentGraphic
                    && primitive_object_id.as_deref() == Some(object_id) =>
                {
                    Some((*from, *to, *stroke_width))
                }
                _ => None,
            })
            .collect()
    };

    let hash_marks = mark_lines_for("obj_line_001");
    assert_eq!(hash_marks.len(), 2);
    for (from, to, stroke_width) in &hash_marks {
        assert_close(*stroke_width, 0.72);
        assert_close(from.distance(*to), 10.0 * 0.72 * 5.0_f64.sqrt() * 0.5);
    }
    let mut hash_centers: Vec<Point> = hash_marks
        .iter()
        .map(|(from, to, _)| Point::new((from.x + to.x) * 0.5, (from.y + to.y) * 0.5))
        .collect();
    hash_centers.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    assert_point_close(hash_centers[0], Point::new(60.0 - 10.0 * 0.72 * 0.25, 20.0));
    assert_point_close(hash_centers[1], Point::new(60.0 + 10.0 * 0.72 * 0.25, 20.0));
    assert_close(hash_centers[0].distance(hash_centers[1]), 10.0 * 0.72 * 0.5);

    let cross_marks = mark_lines_for("obj_line_002");
    assert_eq!(cross_marks.len(), 2);
    for (from, to, stroke_width) in &cross_marks {
        assert_close(*stroke_width, 0.72);
        assert_close(from.distance(*to), 10.0 * 0.72 * std::f64::consts::SQRT_2);
        assert_point_close(
            Point::new((from.x + to.x) * 0.5, (from.y + to.y) * 0.5),
            Point::new(60.0, 60.0),
        );
    }
}

#[test]
fn render_document_emits_text_lines_from_runs() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_text_001": {
                "kind": "text",
                "fontFamily": "Arial",
                "fontSize": 10.0,
                "fill": "#000000"
            }
        },
        "objects": [{
            "id": "obj_text_001",
            "type": "text",
            "visible": true,
            "zIndex": 20,
            "transform": { "translate": [30.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_text_001",
            "payload": {
                "text": "Na\nCl",
                "align": "center",
                "fontSize": 10.0,
                "lineHeight": 14.0,
                "preserveLines": true,
                "runs": [{
                    "text": "Na\nCl",
                    "fontFamily": "Arial",
                    "fontSize": 10.0,
                    "fill": "#000000",
                    "fontWeight": 400,
                    "fontStyle": "normal",
                    "script": "normal"
                }]
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let text_lines: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text {
                role,
                object_id,
                x,
                y,
                runs,
                text_anchor,
                ..
            } if *role == RenderRole::DocumentText
                && object_id.as_deref() == Some("obj_text_001") =>
            {
                Some((*x, *y, runs.clone(), text_anchor.clone()))
            }
            _ => None,
        })
        .collect();

    assert_eq!(text_lines.len(), 2);
    assert!(text_lines
        .iter()
        .all(|(x, _, _, _)| (*x - 30.0).abs() < 0.001));
    assert_eq!(text_lines[0].2[0].text, "Na");
    assert_eq!(text_lines[1].2[0].text, "Cl");
    assert!(text_lines[1].1 > text_lines[0].1);
    assert_eq!(text_lines[0].3.as_deref(), Some("middle"));
}

#[test]
fn render_document_emits_shape_rect_with_style() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape_001": {
                "kind": "shape",
                "fill": "#55f0f5",
                "stroke": "#000000",
                "strokeWidth": 0.7,
                "fillGradient": {
                    "type": "linear",
                    "x1": "0%",
                    "y1": "0%",
                    "x2": "0%",
                    "y2": "100%",
                    "stops": [
                        { "offset": "0%", "color": "#a2f7fa" },
                        { "offset": "100%", "color": "#4bd3d8" }
                    ]
                },
                "dashArray": [3.2, 2.8]
            }
        },
        "objects": [{
            "id": "obj_shape_001",
            "type": "shape",
            "visible": true,
            "zIndex": 5,
            "transform": { "translate": [18.77, 22.5], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_shape_001",
            "payload": {
                "kind": "roundRect",
                "bbox": [0.0, 0.0, 110.25, 81.0],
                "cornerRadius": 6.0
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let rect = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Rect {
                role,
                object_id,
                x,
                y,
                width,
                height,
                fill,
                stroke,
                stroke_width,
                rx,
                dash_array,
                fill_gradient,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_shape_001") =>
            {
                Some((
                    *x,
                    *y,
                    *width,
                    *height,
                    fill.clone(),
                    stroke.clone(),
                    *stroke_width,
                    *rx,
                    dash_array.clone(),
                    fill_gradient.clone(),
                ))
            }
            _ => None,
        })
        .expect("shape rect primitive");

    assert!((rect.0 - 18.77).abs() < 0.001);
    assert!((rect.1 - 22.5).abs() < 0.001);
    assert!((rect.2 - 110.25).abs() < 0.001);
    assert!((rect.3 - 81.0).abs() < 0.001);
    assert_eq!(rect.4.as_deref(), Some("#55f0f5"));
    assert_eq!(rect.5.as_deref(), Some("#000000"));
    assert!((rect.6 - 0.7).abs() < 0.001);
    assert_eq!(rect.7, Some(6.0));
    assert_eq!(rect.8, vec![3.2, 2.8]);
    assert_eq!(
        rect.9.and_then(|value| value
            .get("stops")
            .and_then(|stops| stops.as_array())
            .map(|stops| stops.len())),
        Some(2)
    );
}

#[test]
fn render_document_emits_cdxml_oval_shape_with_zero_bbox_height() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape_001": {
                "kind": "shape",
                "fill": null,
                "stroke": "#000000",
                "strokeWidth": 1.0,
                "dashArray": [2.7]
            }
        },
        "objects": [{
            "id": "obj_shape_001",
            "type": "shape",
            "visible": true,
            "zIndex": 5,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_shape_001",
            "payload": {
                "kind": "ellipse",
                "bbox": [158.16, 247.50, 36.12, 0.0],
                "center": [158.16, 247.50],
                "majorAxisEnd": [194.28, 247.50],
                "minorAxisEnd": [158.16, 261.95]
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            object_id,
            stroke_width,
            dash_array,
            d,
            ..
        } if object_id.as_deref() == Some("obj_shape_001")
            && (*stroke_width - 1.0).abs() < 0.001
            && dash_array == &vec![2.7]
            && d.starts_with("M ")
            && d.contains(" C ")
    )));
}

#[test]
fn render_document_emits_all_shape_geometry_style_combinations() {
    let mut styles = Map::new();
    styles.insert(
        "solid".to_string(),
        json!({"kind": "shape", "fill": null, "stroke": "#000000", "strokeWidth": 1.0}),
    );
    styles.insert(
        "dashed".to_string(),
        json!({"kind": "shape", "fill": null, "stroke": "#000000", "strokeWidth": 1.0, "dashArray": [2.7]}),
    );
    styles.insert(
        "filled".to_string(),
        json!({"kind": "shape", "fill": "#000000", "stroke": null, "strokeWidth": 1.0}),
    );
    styles.insert(
        "shaded".to_string(),
        json!({"kind": "shape", "fill": null, "stroke": "#000000", "strokeWidth": 1.0, "shaded": true}),
    );
    styles.insert(
        "shadowed".to_string(),
        json!({"kind": "shape", "fill": null, "stroke": "#000000", "strokeWidth": 1.0, "shadow": true}),
    );

    let mut objects = Vec::new();
    let shapes = ["circle", "ellipse", "roundRect", "rect"];
    let style_ids = ["solid", "dashed", "filled", "shaded", "shadowed"];
    for (shape_index, shape) in shapes.iter().enumerate() {
        for (style_index, style_id) in style_ids.iter().enumerate() {
            let id = format!("obj_{shape}_{style_id}");
            let x = 20.0 + style_index as f64 * 40.0;
            let y = 20.0 + shape_index as f64 * 40.0;
            let payload = match *shape {
                "circle" => json!({
                    "kind": "circle",
                    "bbox": [x - 10.0, y, 20.0, 0.0],
                    "center": [x, y],
                    "majorAxisEnd": [x + 10.0, y],
                    "minorAxisEnd": [x, y + 10.0]
                }),
                "ellipse" => json!({
                    "kind": "ellipse",
                    "bbox": [x - 14.0, y, 28.0, 0.0],
                    "center": [x, y],
                    "majorAxisEnd": [x + 14.0, y],
                    "minorAxisEnd": [x, y + 6.0]
                }),
                "roundRect" => json!({
                    "kind": "roundRect",
                    "bbox": [0.0, 0.0, 28.0, 18.0],
                    "cornerRadius": 6.0
                }),
                _ => json!({
                    "kind": "rect",
                    "bbox": [0.0, 0.0, 28.0, 18.0]
                }),
            };
            objects.push(json!({
                "id": id,
                "type": "shape",
                "visible": true,
                "zIndex": shape_index * style_ids.len() + style_index,
                "transform": { "translate": [x, y], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": style_id,
                "payload": payload
            }));
        }
    }

    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 240.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": styles,
        "objects": objects,
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    for shape in shapes {
        for style_id in style_ids {
            let id = format!("obj_{shape}_{style_id}");
            assert!(
                primitives.iter().any(|primitive| match primitive {
                    RenderPrimitive::Rect { object_id, .. }
                    | RenderPrimitive::Path { object_id, .. }
                    | RenderPrimitive::FilledPath { object_id, .. } =>
                        object_id.as_deref() == Some(id.as_str()),
                    _ => false,
                }),
                "missing rendered primitive for {id}"
            );
        }
    }
}

#[test]
fn render_document_emits_fragment_label_text_and_knockout() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 200.0, "height": 120.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": 0.85,
                "fontFamily": "Arial",
                "fontSize": 11.0
            }
        },
        "objects": [{
            "id": "obj_molecule_001",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [12.0, 8.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_001" }
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 80.0, 40.0],
                    "nodes": [{
                        "id": "n1",
                        "element": "N",
                        "atomicNumber": 7,
                        "position": [20.0, 20.0],
                        "charge": 0,
                        "numHydrogens": 1,
                        "label": {
                            "text": "NH",
                            "sourceText": "NH",
                            "position": [16.4, 15.6],
                            "box": [16.4, 7.2, 23.6, 23.8],
                            "runs": [{
                                "text": "NH",
                                "fontFamily": "Arial",
                                "fontSize": 10.0,
                                "fill": "#000000",
                                "fontWeight": 400,
                                "fontStyle": "normal",
                                "script": "normal"
                            }],
                            "lines": ["H", "N"],
                            "lineRuns": [
                                [{
                                    "text": "H",
                                    "fontFamily": "Arial",
                                    "fontSize": 10.0,
                                    "fill": "#000000",
                                    "fontWeight": 400,
                                    "fontStyle": "normal",
                                    "script": "normal"
                                }],
                                [{
                                    "text": "N",
                                    "fontFamily": "Arial",
                                    "fontSize": 10.0,
                                    "fill": "#000000",
                                    "fontWeight": 400,
                                    "fontStyle": "normal",
                                    "script": "normal"
                                }]
                            ],
                            "align": "left",
                            "layout": "hetero-h-above",
                            "fontFamily": "Arial",
                            "fill": "#000000",
                            "fontSize": 10.0
                        }
                    }],
                    "bonds": []
                }
            }
        }
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect { role, object_id, fill, .. }
            if *role == RenderRole::DocumentKnockout
                && object_id.as_deref() == Some("obj_molecule_001")
                && fill.as_deref() == Some("#ffffff")
    )));

    let label_lines: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text {
                role,
                object_id,
                x,
                y,
                runs,
                ..
            } if *role == RenderRole::DocumentText
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some((*x, *y, runs.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(label_lines.len(), 2);
    assert!(label_lines
        .iter()
        .all(|(x, _, _)| (*x - 28.4).abs() < 0.001));
    assert_eq!(label_lines[0].2[0].text, "H");
    assert_eq!(label_lines[1].2[0].text, "N");
    assert!(label_lines[1].1 > label_lines[0].1);
}

#[test]
fn render_document_respects_explicit_small_fragment_label_font_size() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "N",
                "atomicNumber": 7,
                "position": [20.0, 20.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "sourceText": "N",
                    "position": [18.0, 20.0],
                    "box": [18.0, 14.0, 22.0, 20.0],
                    "fontSize": 6.0,
                    "runs": [{
                        "text": "N",
                        "fontFamily": "Arial",
                        "fontSize": 6.0,
                        "fill": "#000000",
                        "fontWeight": 400,
                        "fontStyle": "normal",
                        "script": "normal"
                    }]
                }
            }
        ]),
        json!([]),
    );

    let font_size = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                role, font_size, ..
            } if role == RenderRole::DocumentText => Some(font_size),
            _ => None,
        })
        .expect("fragment label text");

    assert!((font_size - 6.0).abs() <= 1.0e-6, "{font_size}");
}

#[test]
fn render_document_draws_imported_cdxml_invalid_marker_as_non_focusable_diagnostic() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "N",
                "atomicNumber": 7,
                "position": [20.0, 20.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "sourceText": "N",
                    "position": [18.0, 20.0],
                    "box": [18.0, 14.0, 22.0, 20.0],
                    "fontSize": 10.0,
                    "runs": [{
                        "text": "N",
                        "fontFamily": "Arial",
                        "fontSize": 10.0,
                        "fill": "#000000",
                        "fontWeight": 400,
                        "fontStyle": "normal",
                        "script": "normal"
                    }],
                    "meta": {
                        "import": { "cdxml": { "textPosition": [18.0, 20.0] } },
                        "labelRecognition": { "status": "invalid" }
                    }
                }
            }
        ]),
        json!([]),
    );

    let marker = render_document(&document)
        .into_iter()
        .find(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Rect {
                    role: RenderRole::DocumentGraphic,
                    stroke: Some(stroke),
                    ..
                } if stroke == "#d32f2f"
            )
        })
        .expect("imported invalid label should still render a diagnostic marker");

    assert!(matches!(
        marker,
        RenderPrimitive::Rect {
            object_id: None,
            node_id: None,
            ..
        }
    ));
}

#[test]
fn render_document_scales_small_bracket_geometry_without_fixed_minimums() {
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 80.0, "height": 80.0, "background": "#ffffff" }
        },
        "styles": {},
        "objects": [{
            "id": "obj_bracket_001",
            "type": "bracket",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "payload": {
                "kind": "square",
                "bbox": [0.0, 0.0, 2.0, 6.0],
                "stroke": "#000000",
                "strokeWidth": 0.6
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let d = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Path {
                role, object_id, d, ..
            } if role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_bracket_001") =>
            {
                Some(d)
            }
            _ => None,
        })
        .expect("bracket path");
    let first_x = d
        .trim_start_matches("M ")
        .split(',')
        .next()
        .and_then(|value| value.parse::<f64>().ok())
        .expect("first bracket x coordinate");

    assert!(
        first_x > 0.0 && first_x < 0.5,
        "small bracket lip should scale with bbox instead of using a fixed 1pt minimum: {d}"
    );
}

#[test]
fn render_document_uses_label_glyph_polygons_for_knockout_and_endpoint_clipping() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "N",
                "atomicNumber": 7,
                "position": [19.0, 20.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "NH",
                    "position": [18.0, 20.0],
                    "box": [18.0, 16.0, 25.0, 24.0],
                    "glyphPolygons": [
                        [[18.0, 17.0], [20.0, 17.0], [20.0, 23.0], [18.0, 23.0]],
                        [[21.0, 17.0], [23.0, 17.0], [23.0, 23.0], [21.0, 23.0]]
                    ]
                }
            },
            {
                "id": "n2",
                "element": "C",
                "atomicNumber": 6,
                "position": [60.0, 20.0],
                "charge": 0,
                "numHydrogens": 0
            }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let primitives = render_document(&document);
    let knockouts = object_knockout_polygons(&primitives);
    assert_eq!(knockouts.len(), 2, "{knockouts:?}");
    assert_eq!(object_knockout_rect_count(&primitives), 0);
    assert!(primitives.iter().all(|primitive| match primitive {
        RenderPrimitive::Polygon { role, node_id, .. } if *role == RenderRole::DocumentKnockout => {
            node_id.as_deref() == Some("n1")
        }
        _ => true,
    }));
    let svg = document_to_svg(&document);
    assert!(
        !svg.contains("fill=\"#ffffff\""),
        "label clipping geometry should not paint a white knockout in document SVG: {svg}"
    );

    let centerlines = object_bond_centerlines(&primitives);
    assert_eq!(centerlines.len(), 1, "{centerlines:?}");
    let start_x = centerlines[0].0.x.min(centerlines[0].1.x);
    assert!(
        start_x > 24.0 && start_x < 25.0,
        "endpoint should be clipped from the glyph polygon plus margin, not the full label box: {centerlines:?}"
    );
}

#[test]
fn render_document_emits_primitives_for_legacy_molblock_resource() {
    let molblock = concat!(
        "Legacy\n",
        "  Chemcore\n",
        "\n",
        "  2  1  0  0  0  0            999 V2000\n",
        "    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n",
        "    1.2000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n",
        "  1  2  1  0  0  0  0\n",
        "M  END\n"
    );
    let document: ChemcoreDocument = serde_json::from_value(json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 240.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": 0.85,
                "fontFamily": "Arial",
                "fontSize": 11.0
            }
        },
        "objects": [{
            "id": "obj_molecule_legacy",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [24.0, 18.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": {
                "resourceRef": "mol_legacy",
                "bbox": [0.0, 0.0, 96.0, 72.0]
            }
        }],
        "resources": {
            "mol_legacy": {
                "type": "molfile",
                "encoding": "chemical/x-mdl-molfile",
                "data": molblock
            }
        }
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Line { role, object_id, .. }
            if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_legacy")
    )));
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Text { role, object_id, text, .. }
            if *role == RenderRole::DocumentText
                && object_id.as_deref() == Some("obj_molecule_legacy")
                && text == "O"
    )));
}

#[test]
fn render_document_keeps_terminal_triple_outer_lines_full_length() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 3, "strokeWidth": 0.85 }
        ]),
    );

    let centerlines = object_bond_centerlines(&render_document(&document));

    assert_eq!(centerlines.len(), 3);
    for (from, to) in centerlines {
        assert!((from.x - 20.0).abs() < 0.001, "{from:?}");
        assert!((to.x - 56.0).abs() < 0.001, "{to:?}");
    }
}

#[test]
fn render_document_preserves_dashed_double_line_styles() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "double": { "placement": "left" },
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "dashed",
                    "left": "dashed",
                    "right": "solid"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons(&primitives);
    let knockouts = object_knockout_polygons(&primitives);

    assert_eq!(polygons.len(), 2);
    assert!(polygons.iter().all(|points| points.len() == 4));
    assert!(knockouts.len() >= 2, "{knockouts:?}");
    assert!(!primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Line { role, object_id, .. }
            if *role == RenderRole::DocumentBond && object_id.as_deref() == Some("obj_molecule_001")
    )));
}

#[test]
fn render_document_emits_polygon_for_bold_single_bond() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );

    let polygons: Vec<_> = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points)
            }
            _ => None,
        })
        .collect();

    assert_eq!(polygons.len(), 1);
    assert_eq!(polygons[0].len(), 4);
    assert!(polygons[0].iter().any(|point| (point.y - 40.0).abs() > 0.5));
}

#[test]
fn render_document_emits_polygon_for_plain_single_bond() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons(&primitives);

    assert_eq!(polygons.len(), 1);
    assert_eq!(polygons[0].len(), 4);
    assert!(!primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Line { role, object_id, .. }
            if *role == RenderRole::DocumentBond && object_id.as_deref() == Some("obj_molecule_001")
    )));
}

#[test]
fn render_document_emits_main_contact_patches_for_connected_bold_and_single_bonds() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = centered_bond_polygons(&primitives, chemcore_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 2);
    assert!(polygons.iter().all(|points| points.len() == 5));
    assert!(
        polygons.iter().all(|points| polygon_area(points) > 0.01),
        "{polygons:?}"
    );

    assert!(!primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Line {
            role,
            object_id,
            ..
        } if *role == RenderRole::DocumentBond && object_id.as_deref() == Some("obj_molecule_001")
    )));
}

#[test]
fn render_document_emits_two_way_main_contact_patches_for_plain_singles() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 },
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = centered_bond_polygons(&primitives, chemcore_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 2);
    assert!(polygons.iter().all(|points| points.len() == 5));
    assert!(
        polygons.iter().all(|points| polygon_area(points) > 0.01),
        "{polygons:?}"
    );

    let total_bond_polygons: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(total_bond_polygons.len(), 2);
}

#[test]
fn render_document_emits_equal_length_cross_segments_for_bold_dashed_bond() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [50.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 1.0,
                "lineStyles": {
                    "main": "dashed",
                    "left": "solid",
                    "right": "solid"
                },
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons(&primitives);
    let knockouts = object_knockout_polygons(&primitives);

    assert_eq!(polygons.len(), 1);
    assert_eq!(polygons[0].len(), 4);
    assert!(polygon_area(&polygons[0]) > 40.0, "{polygons:?}");
    assert_eq!(knockouts.len(), 10, "{knockouts:?}");
    assert!(knockouts
        .iter()
        .all(|points| points.iter().any(|point| point.y > 40.0)
            && points.iter().any(|point| point.y < 40.0)));
    let (axis_from, axis_to) = bond_axis_from_points(&polygons[0]).expect("hash bond axis");
    let (bond_start, bond_end) =
        projection_range_on_axis(&polygons[0], axis_from, axis_to).expect("hash bond range");
    let mut gaps: Vec<_> = knockouts
        .iter()
        .filter_map(|points| projection_range_on_axis(points, axis_from, axis_to))
        .collect();
    gaps.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut black_segments = Vec::new();
    let mut cursor = bond_start;
    for (gap_start, gap_end) in gaps {
        if gap_start > cursor + 1.0e-6 {
            black_segments.push(gap_start - cursor);
        }
        cursor = gap_end;
    }
    if bond_end > cursor + 1.0e-6 {
        black_segments.push(bond_end - cursor);
    }
    assert_eq!(black_segments.len(), 11, "{black_segments:?}");
    let first_black = black_segments[0];
    assert!(
        black_segments
            .iter()
            .all(|length| (length - first_black).abs() < 0.02),
        "{black_segments:?}"
    );
    assert!((first_black - 1.0).abs() < 0.02, "{black_segments:?}");
    assert!(!primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Line { role, object_id, .. }
            if *role == RenderRole::DocumentBond && object_id.as_deref() == Some("obj_molecule_001")
    )));
}

#[test]
fn render_document_emits_main_contact_patches_for_connected_bold_and_dashed_single_bonds() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "dashed",
                    "left": "solid",
                    "right": "solid"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = centered_bond_polygons(&primitives, chemcore_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 2);
    assert!(polygons.iter().all(|points| points.len() == 5));
    assert!(object_knockout_polygons(&primitives).len() >= 1);
}

#[test]
fn render_document_emits_primitives_for_wedge_and_hashed_wedge() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [92.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [128.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "end"
                }
            },
            {
                "id": "b2",
                "begin": "n3",
                "end": "n4",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "hashed-wedge",
                    "wideEnd": "end"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let bond_polygons: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points.len())
            }
            _ => None,
        })
        .collect();

    assert!(
        bond_polygons.iter().all(|count| *count == 4),
        "{bond_polygons:?}"
    );
    assert!(bond_polygons.len() >= 2);
}

#[test]
fn render_document_emits_main_contact_patches_for_connected_single_and_solid_wedge() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "end"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = centered_bond_polygons(&primitives, chemcore_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 2);
    assert_eq!(
        polygons.iter().filter(|points| points.len() == 4).count(),
        2
    );

    let wedge_polygon = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .max_by(|a, b| polygon_area(a).total_cmp(&polygon_area(b)))
        .expect("solid wedge polygon");
    assert_eq!(wedge_polygon.len(), 4);
    assert_eq!(
        wedge_polygon
            .iter()
            .filter(|point| point.distance(chemcore_engine::Point::new(56.0, 40.0)) <= 4.0)
            .count(),
        2
    );
}

#[test]
fn render_document_retreats_hashed_wedge_against_connected_single_bond() {
    let connected = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "hashed-wedge",
                    "wideEnd": "end"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );
    let connected_primitives = render_document(&connected);
    let connected_polygons = object_bond_polygons_with_ids(&connected_primitives);
    let hashed_wedge = object_bond_points_for_id(&connected_primitives, "b1");
    assert!(!hashed_wedge.is_empty(), "hashed wedge polygons");
    let branch = connected_polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b2").then_some(points.clone()))
        .expect("branch polygon");
    let connected_end =
        closest_points_to_target(&hashed_wedge, chemcore_engine::Point::new(56.0, 40.0), 2);

    assert!(
        connected_end.iter().all(|point| point.x < 55.0),
        "{hashed_wedge:?}"
    );
    assert!(
        average_closest_distance_to_point(&branch, chemcore_engine::Point::new(56.0, 40.0), 2)
            < 0.6,
        "{branch:?}"
    );
}

#[test]
fn render_document_retreats_hashed_wedge_at_both_connected_endpoints() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [2.0, 12.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [74.0, 68.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "hashed-wedge",
                    "wideEnd": "end"
                }
            },
            { "id": "b2", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n2", "end": "n4", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let primitives = render_document(&document);
    let hashed_wedge = object_bond_points_for_id(&primitives, "b1");
    assert!(!hashed_wedge.is_empty(), "hashed wedge polygons");
    let begin_end =
        closest_points_to_target(&hashed_wedge, chemcore_engine::Point::new(20.0, 40.0), 2);
    let wide_end =
        closest_points_to_target(&hashed_wedge, chemcore_engine::Point::new(56.0, 40.0), 2);

    assert!(
        begin_end.iter().all(|point| point.x > 21.0),
        "{hashed_wedge:?}"
    );
    assert!(
        wide_end.iter().all(|point| point.x < 55.0),
        "{hashed_wedge:?}"
    );
}

#[test]
fn render_document_keeps_hashed_wedge_label_clip_without_extra_hash_retreat() {
    let labeled_nodes = json!([
        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
        {
            "id": "n2",
            "element": "N",
            "atomicNumber": 7,
            "position": [56.0, 40.0],
            "charge": 0,
            "numHydrogens": 0,
            "label": {
                "text": "N",
                "position": [52.0, 44.0],
                "box": [50.0, 34.0, 62.0, 46.0],
                "runs": [{ "text": "N", "fontFamily": "Arial", "fontSize": 10.0, "fill": "#000000" }]
            }
        }
    ]);
    let isolated = fragment_document(
        labeled_nodes.clone(),
        json!([{
            "id": "b1",
            "begin": "n1",
            "end": "n2",
            "order": 1,
            "strokeWidth": 0.85,
            "stereo": { "kind": "hashed-wedge", "wideEnd": "end" }
        }]),
    );
    let connected = fragment_document(
        json!([
            labeled_nodes[0].clone(),
            labeled_nodes[1].clone(),
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 58.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": { "kind": "hashed-wedge", "wideEnd": "end" }
            },
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 }
        ]),
    );
    let isolated_wedge = object_bond_polygons_with_ids(&render_document(&isolated))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("isolated hashed wedge polygon");
    let connected_wedge = object_bond_polygons_with_ids(&render_document(&connected))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("connected hashed wedge polygon");

    assert!(polygons_have_same_vertices(
        &isolated_wedge,
        &connected_wedge,
        1.0e-4,
    ));
}

#[test]
fn render_document_retreats_hash_bond_against_connected_single_bond() {
    let connected = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "dashed",
                    "left": "solid",
                    "right": "solid"
                },
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );
    let connected_primitives = render_document(&connected);
    let connected_polygons = object_bond_polygons_with_ids(&connected_primitives);
    let hash_bond = connected_polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("connected hash bond polygon");
    let branch = connected_polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b2").then_some(points.clone()))
        .expect("branch polygon");
    let connected_end =
        closest_points_to_target(&hash_bond, chemcore_engine::Point::new(56.0, 40.0), 2);

    assert_eq!(hash_bond.len(), 4);
    assert!(
        connected_end.iter().all(|point| point.x < 55.0),
        "{hash_bond:?}"
    );
    assert!(
        average_closest_distance_to_point(&branch, chemcore_engine::Point::new(56.0, 40.0), 2)
            < 0.6,
        "{branch:?}"
    );
    let knockouts = object_knockout_polygons(&connected_primitives);
    assert!(knockouts.len() >= 1);
    assert!(
        knockouts.iter().flatten().any(|point| point.x > 55.0),
        "{knockouts:?}"
    );
}

#[test]
fn render_document_keeps_hash_bond_label_clip_without_extra_hash_retreat() {
    let labeled_nodes = json!([
        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
        {
            "id": "n2",
            "element": "N",
            "atomicNumber": 7,
            "position": [56.0, 40.0],
            "charge": 0,
            "numHydrogens": 0,
            "label": {
                "text": "N",
                "position": [52.0, 44.0],
                "box": [50.0, 34.0, 62.0, 46.0],
                "runs": [{ "text": "N", "fontFamily": "Arial", "fontSize": 10.0, "fill": "#000000" }]
            }
        }
    ]);
    let hash_bond = json!({
        "id": "b1",
        "begin": "n1",
        "end": "n2",
        "order": 1,
        "strokeWidth": 0.85,
        "lineStyles": { "main": "dashed", "left": "solid", "right": "solid" },
        "lineWeights": { "main": "bold", "left": "normal", "right": "normal" }
    });
    let isolated = fragment_document(labeled_nodes.clone(), json!([hash_bond.clone()]));
    let connected = fragment_document(
        json!([
            labeled_nodes[0].clone(),
            labeled_nodes[1].clone(),
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 58.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            hash_bond,
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 }
        ]),
    );
    let isolated_hash = object_bond_polygons_with_ids(&render_document(&isolated))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("isolated hash polygon");
    let connected_hash = object_bond_polygons_with_ids(&render_document(&connected))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("connected hash polygon");

    assert!(polygons_have_same_vertices(
        &isolated_hash,
        &connected_hash,
        1.0e-4,
    ));
}

#[test]
fn render_document_retreats_hash_bond_mother_polygon_against_center_double_outer_line() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "dashed",
                    "left": "solid",
                    "right": "solid"
                },
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let hash_bond = object_bond_polygons_with_ids(&primitives)
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b2").then_some(points))
        .expect("hash bond polygon");
    let connected_end =
        closest_points_to_target(&hash_bond, chemcore_engine::Point::new(56.0, 40.0), 2);
    let unit = chemcore_engine::Point::new(18.0, -28.0);
    let unit_length = (unit.x * unit.x + unit.y * unit.y).sqrt();
    let unit_x = unit.x / unit_length;
    let unit_y = unit.y / unit_length;
    let projections: Vec<_> = connected_end
        .iter()
        .map(|point| (point.x - 56.0) * unit_x + (point.y - 40.0) * unit_y)
        .collect();

    assert_eq!(connected_end.len(), 2);
    assert!(
        (projections[0] - projections[1]).abs() <= 1.0e-4,
        "{hash_bond:?} {projections:?}"
    );
    assert!(
        projections.iter().all(|projection| *projection > 0.05),
        "{hash_bond:?} {projections:?}"
    );
    assert!(object_knockout_polygons(&primitives).len() >= 1);
}

#[test]
fn render_document_retreats_hashed_wedge_stripes_against_center_double_outer_line() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "hashed-wedge",
                    "wideEnd": "begin"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let hashed_wedge = object_bond_points_for_id(&primitives, "b2");
    assert!(!hashed_wedge.is_empty(), "hashed wedge polygons");
    let connected_end =
        closest_points_to_target(&hashed_wedge, chemcore_engine::Point::new(56.0, 40.0), 2);
    let unit = chemcore_engine::Point::new(18.0, -28.0);
    let unit_length = (unit.x * unit.x + unit.y * unit.y).sqrt();
    let unit_x = unit.x / unit_length;
    let unit_y = unit.y / unit_length;
    let projections: Vec<_> = connected_end
        .iter()
        .map(|point| (point.x - 56.0) * unit_x + (point.y - 40.0) * unit_y)
        .collect();

    assert_eq!(connected_end.len(), 2);
    assert!(
        (projections[0] - projections[1]).abs() <= 1.0e-4,
        "{hashed_wedge:?} {projections:?}"
    );
    assert!(
        projections.iter().all(|projection| *projection > 0.05),
        "{hashed_wedge:?} {projections:?}"
    );
}

#[test]
fn render_document_retreats_hash_bond_and_ignores_it_for_other_bond_contacts() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [74.0, 68.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "dashed",
                    "left": "solid",
                    "right": "solid"
                },
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            },
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n2", "end": "n4", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let hash_bond = polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points.clone()))
        .expect("hash bond polygon");

    assert!(
        average_closest_distance_to_point(&hash_bond, chemcore_engine::Point::new(56.0, 40.0), 2)
            > 1.0,
        "{hash_bond:?}"
    );
}

#[test]
fn render_document_retreats_hashed_wedge_and_ignores_it_for_other_bond_contacts() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [74.0, 68.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "hashed-wedge",
                    "wideEnd": "end"
                }
            },
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n2", "end": "n4", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let primitives = render_document(&document);
    let hashed_wedge = object_bond_points_for_id(&primitives, "b1");
    assert!(!hashed_wedge.is_empty(), "hashed wedge polygons");

    assert!(
        average_closest_distance_to_point(
            &hashed_wedge,
            chemcore_engine::Point::new(56.0, 40.0),
            2
        ) > 1.0,
        "{hashed_wedge:?}"
    );
}

#[test]
fn render_document_retreats_hash_bond_against_solid_dashed_center_double_outer_line() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" },
                "lineStyles": {
                    "left": "solid",
                    "right": "dashed"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "dashed",
                    "left": "solid",
                    "right": "solid"
                },
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let hash_bond = object_bond_polygons_with_ids(&primitives)
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b2").then_some(points))
        .expect("hash bond polygon");
    let connected_end =
        closest_points_to_target(&hash_bond, chemcore_engine::Point::new(56.0, 40.0), 2);
    let unit = chemcore_engine::Point::new(18.0, -28.0);
    let unit_length = (unit.x * unit.x + unit.y * unit.y).sqrt();
    let unit_x = unit.x / unit_length;
    let unit_y = unit.y / unit_length;
    let projections: Vec<_> = connected_end
        .iter()
        .map(|point| (point.x - 56.0) * unit_x + (point.y - 40.0) * unit_y)
        .collect();

    assert_eq!(connected_end.len(), 2);
    assert!(
        (projections[0] - projections[1]).abs() <= 1.0e-4,
        "{hash_bond:?} {projections:?}"
    );
    assert!(
        projections.iter().all(|projection| *projection > 0.05),
        "{hash_bond:?} {projections:?}"
    );
    assert!(object_knockout_polygons(&primitives).len() >= 1);
}

#[test]
fn render_document_retreats_hashed_wedge_against_double_dashed_center_double_outer_line() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" },
                "lineStyles": {
                    "left": "dashed",
                    "right": "dashed"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "hashed-wedge",
                    "wideEnd": "begin"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let hashed_wedge = object_bond_points_for_id(&primitives, "b2");
    assert!(!hashed_wedge.is_empty(), "hashed wedge polygons");
    let connected_end =
        closest_points_to_target(&hashed_wedge, chemcore_engine::Point::new(56.0, 40.0), 2);
    let unit = chemcore_engine::Point::new(18.0, -28.0);
    let unit_length = (unit.x * unit.x + unit.y * unit.y).sqrt();
    let unit_x = unit.x / unit_length;
    let unit_y = unit.y / unit_length;
    let projections: Vec<_> = connected_end
        .iter()
        .map(|point| (point.x - 56.0) * unit_x + (point.y - 40.0) * unit_y)
        .collect();

    assert_eq!(connected_end.len(), 2);
    assert!(
        (projections[0] - projections[1]).abs() <= 1.0e-4,
        "{hashed_wedge:?} {projections:?}"
    );
    assert!(
        projections.iter().all(|projection| *projection > 0.05),
        "{hashed_wedge:?} {projections:?}"
    );
}

#[test]
fn render_document_scales_side_double_offset_with_bond_length() {
    let short_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": {
                    "placement": "right"
                }
            }
        ]),
    );
    let long_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 80.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [92.0, 80.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": {
                    "placement": "right"
                }
            }
        ]),
    );

    let short_lines = object_bond_centerlines(&render_document(&short_document));
    let long_lines = object_bond_centerlines(&render_document(&long_document));

    let short_offset = short_lines
        .iter()
        .map(|(from, to)| ((from.y + to.y) / 2.0 - 40.0).abs())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();
    let long_offset = long_lines
        .iter()
        .map(|(from, to)| ((from.y + to.y) / 2.0 - 80.0).abs())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();
    assert!(
        (long_offset - short_offset * 2.0).abs() < 0.05,
        "{short_offset} {long_offset}"
    );
}

#[test]
fn render_document_increases_side_double_offset_for_bold_main_line() {
    let normal_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            }
        ]),
    );
    let bold_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" },
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );

    let normal_lines = object_bond_centerlines(&render_document(&normal_document));
    let bold_lines = object_bond_centerlines(&render_document(&bold_document));

    let normal_offset = normal_lines
        .iter()
        .map(|(from, to)| ((from.y + to.y) / 2.0 - 40.0).abs())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();
    let bold_offset = bold_lines
        .iter()
        .map(|(from, to)| ((from.y + to.y) / 2.0 - 40.0).abs())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();

    assert!(
        bold_offset > normal_offset + 0.01,
        "{normal_offset} {bold_offset}"
    );
}

#[test]
fn render_document_keeps_terminal_side_double_outer_line_equal_length() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": {
                    "placement": "right"
                }
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);

    let mut indexed_lengths: Vec<_> = polygons
        .iter()
        .enumerate()
        .map(|(index, points)| (index, bond_axis_length(points).expect("bond axis length")))
        .collect();
    indexed_lengths.sort_by(|(_, a), (_, b)| a.total_cmp(b));

    let (short_index, short_length) = indexed_lengths[0];
    let (long_index, long_length) = indexed_lengths[1];
    let short_axis = bond_axis_from_points(&polygons[short_index]).expect("short axis");
    let long_axis = bond_axis_from_points(&polygons[long_index]).expect("long axis");

    assert!(
        (short_length - long_length).abs() < 0.05,
        "{short_length} {long_length}"
    );
    assert!(
        (short_axis.0.x - 20.0).abs() < 0.05 && (short_axis.1.x - 56.0).abs() < 0.05,
        "{short_axis:?}"
    );
    assert!(
        (long_axis.0.x - 20.0).abs() < 0.05 && (long_axis.1.x - 56.0).abs() < 0.05,
        "{long_axis:?}"
    );
}

#[test]
fn render_document_keeps_side_double_outer_line_full_length_when_only_opposite_side_single_is_attached(
) {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 68.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": {
                    "placement": "right"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);

    let mut indexed_lengths: Vec<_> = polygons
        .iter()
        .enumerate()
        .map(|(index, points)| (index, bond_axis_length(points).expect("bond axis length")))
        .collect();
    indexed_lengths.sort_by(|(_, a), (_, b)| a.total_cmp(b));

    let (short_index, short_length) = indexed_lengths[0];
    let (long_index, long_length) = indexed_lengths[1];
    let short_axis = bond_axis_from_points(&polygons[short_index]).expect("short axis");
    let long_axis = bond_axis_from_points(&polygons[long_index]).expect("long axis");
    assert!(
        (short_length - long_length).abs() < 0.05,
        "short={short_length} long={long_length}"
    );
    assert!(
        (short_axis.0.x - 20.0).abs() < 0.05 && (short_axis.1.x - 56.0).abs() < 0.05,
        "{short_axis:?}"
    );
    assert!(
        (long_axis.0.x - 20.0).abs() < 0.05 && (long_axis.1.x - 56.0).abs() < 0.05,
        "{long_axis:?}"
    );
}

#[test]
fn render_document_keeps_same_side_single_attached_side_double_outer_line_shortened() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": {
                    "placement": "right"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);

    let mut indexed_lengths: Vec<_> = polygons
        .iter()
        .enumerate()
        .map(|(index, points)| (index, bond_axis_length(points).expect("bond axis length")))
        .collect();
    indexed_lengths.sort_by(|(_, a), (_, b)| a.total_cmp(b));

    let short_length = indexed_lengths[0].1;
    let long_length = indexed_lengths[1].1;
    assert!(
        short_length < long_length - 0.05,
        "{short_length} {long_length}"
    );
}

#[test]
fn render_document_recomputes_triple_outer_line_retreat_from_current_bond_length() {
    let short_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [8.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [68.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 3, "strokeWidth": 0.85 },
            { "id": "b2", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n2", "end": "n4", "order": 1, "strokeWidth": 0.85 }
        ]),
    );
    let long_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [92.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [8.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [104.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 3, "strokeWidth": 0.85 },
            { "id": "b2", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n2", "end": "n4", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let retreat_for = |document: &chemcore_engine::ChemcoreDocument| {
        let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(document))
            .into_iter()
            .filter(|(bond_id, _)| bond_id == "b1")
            .map(|(_, points)| points)
            .collect();
        assert_eq!(polygons.len(), 3);

        let mut lengths: Vec<_> = polygons
            .iter()
            .map(|points| bond_axis_length(points).expect("bond axis length"))
            .collect();
        lengths.sort_by(|a, b| a.total_cmp(b));
        let outer_length = lengths[0];
        let main_length = lengths[2];
        main_length - outer_length
    };

    let short_retreat = retreat_for(&short_document);
    let long_retreat = retreat_for(&long_document);

    assert!(
        long_retreat > short_retreat + 0.05,
        "short_retreat={short_retreat} long_retreat={long_retreat}"
    );
    assert!(short_retreat > 0.0, "{short_retreat}");
}

#[test]
fn render_document_joins_center_double_only_on_occupied_side() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);

    let mut indexed_extensions: Vec<_> = polygons
        .iter()
        .enumerate()
        .map(|(index, polygon)| (index, polygon[1].x.max(polygon[2].x)))
        .collect();
    indexed_extensions.sort_by(|(_, a), (_, b)| a.total_cmp(b));
    let unchanged = &polygons[indexed_extensions[0].0];
    let extended = &polygons[indexed_extensions[1].0];

    assert!(
        (unchanged[1].x - 56.0).abs() < 0.001 && (unchanged[2].x - 56.0).abs() < 0.001,
        "{unchanged:?}"
    );
    assert!(
        (extended[1].x - 56.0).abs() > 0.05 || (extended[2].x - 56.0).abs() > 0.05,
        "{extended:?}"
    );
}

#[test]
fn render_document_keeps_center_double_joined_line_normal_width() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.6,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.6
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);

    for polygon in &polygons {
        let (start_width, end_width) =
            bond_polygon_normal_widths(polygon).expect("center double polygon width");
        assert!(
            (start_width - 0.6).abs() <= 1.0e-6 && (end_width - 0.6).abs() <= 1.0e-6,
            "{polygon:?} start={start_width} end={end_width}"
        );
    }
}

#[test]
fn render_document_keeps_center_double_original_for_straight_through_180_degrees() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [92.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);
    for polygon in polygons {
        assert!((polygon[1].x - 56.0).abs() < 0.001, "{polygon:?}");
        assert!((polygon[2].x - 56.0).abs() < 0.001, "{polygon:?}");
    }
}

#[test]
fn render_document_keeps_center_double_original_for_angles_over_162_degrees() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [91.45, 33.75], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);
    for polygon in polygons {
        assert!((polygon[1].x - 56.0).abs() < 0.05, "{polygon:?}");
        assert!((polygon[2].x - 56.0).abs() < 0.05, "{polygon:?}");
    }
}

#[test]
fn render_document_keeps_center_double_equal_at_labeled_endpoint() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "C",
                "atomicNumber": 6,
                "position": [20.0, 60.0],
                "charge": 0,
                "numHydrogens": 0
            },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [56.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [56.0, 45.0],
                    "box": [51.0, 34.0, 61.0, 46.0]
                }
            }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);

    let axis = chemcore_engine::Point::new(20.0 - 56.0, 60.0 - 40.0);
    let axis_length = (axis.x * axis.x + axis.y * axis.y).sqrt();
    let unit_x = axis.x / axis_length;
    let unit_y = axis.y / axis_length;
    let endpoint_retreats: Vec<_> = polygons
        .iter()
        .map(|polygon| {
            let (from, to) = bond_axis_from_points(polygon).expect("bond axis");
            let endpoint = if from.distance(chemcore_engine::Point::new(56.0, 40.0))
                <= to.distance(chemcore_engine::Point::new(56.0, 40.0))
            {
                from
            } else {
                to
            };
            (endpoint.x - 56.0) * unit_x + (endpoint.y - 40.0) * unit_y
        })
        .collect();

    assert!(
        (endpoint_retreats[0] - endpoint_retreats[1]).abs() <= 1.0e-4,
        "{polygons:?} {endpoint_retreats:?}"
    );
    assert!(
        endpoint_retreats.iter().all(|retreat| *retreat > 0.0),
        "{polygons:?} {endpoint_retreats:?}"
    );
}

#[test]
fn render_document_keeps_terminal_side_double_offset_with_label_retreat() {
    let unlabeled = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "N", "atomicNumber": 7, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            }
        ]),
    );
    let labeled = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [56.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [56.0, 45.0],
                    "box": [51.0, 34.0, 61.0, 46.0]
                }
            }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            }
        ]),
    );

    let unlabeled_polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&unlabeled))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    let labeled_polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&labeled))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(unlabeled_polygons.len(), 2);
    assert_eq!(labeled_polygons.len(), 2);

    let unlabeled_axes: Vec<_> = unlabeled_polygons
        .iter()
        .map(|polygon| bond_axis_from_points(polygon).expect("bond axis"))
        .collect();
    let labeled_axes: Vec<_> = labeled_polygons
        .iter()
        .map(|polygon| bond_axis_from_points(polygon).expect("bond axis"))
        .collect();
    let unlabeled_gap = ((unlabeled_axes[0].0.y + unlabeled_axes[0].1.y) * 0.5
        - (unlabeled_axes[1].0.y + unlabeled_axes[1].1.y) * 0.5)
        .abs();
    let labeled_gap = ((labeled_axes[0].0.y + labeled_axes[0].1.y) * 0.5
        - (labeled_axes[1].0.y + labeled_axes[1].1.y) * 0.5)
        .abs();

    assert!(
        (unlabeled_gap - labeled_gap).abs() <= 1.0e-4,
        "{unlabeled_gap} {labeled_gap}"
    );
}

#[test]
fn render_document_applies_label_clip_margin_to_glyph_polygons() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [56.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [56.0, 45.0],
                    "box": [51.0, 34.0, 61.0, 46.0],
                    "glyphPolygons": [[
                        [51.0, 34.0],
                        [61.0, 34.0],
                        [61.0, 46.0],
                        [51.0, 46.0]
                    ]]
                }
            }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let polygon = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("bond polygon should render");
    let (from, to) = bond_axis_from_points(&polygon).expect("bond axis");
    let label_endpoint = if from.x > to.x { from } else { to };

    assert!(
        51.0 - label_endpoint.x > 1.0,
        "glyph polygon clipping should leave extra margin before the label: {polygon:?}"
    );
}

#[test]
fn render_document_uses_smaller_acs_label_clip_margin() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [56.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [56.0, 45.0],
                    "box": [51.0, 34.0, 61.0, 46.0],
                    "glyphPolygons": [[
                        [51.0, 34.0],
                        [61.0, 34.0],
                        [61.0, 46.0],
                        [51.0, 46.0]
                    ]]
                }
            }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.6,
                "boldWidth": 2.0,
                "hashSpacing": 2.5,
                "bondSpacing": 18.0
            }
        ]),
    );

    let polygon = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("bond polygon should render");
    let (from, to) = bond_axis_from_points(&polygon).expect("bond axis");
    let label_endpoint = if from.x > to.x { from } else { to };
    let margin = 51.0 - label_endpoint.x;

    assert!(
        (margin - 0.8).abs() < 0.02,
        "ACS label clipping should use the ACS template margin: {margin} {polygon:?}"
    );
}

#[test]
fn render_document_keeps_center_double_parallel_with_branches_and_labeled_endpoint() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [44.0, 22.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [44.0, 58.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n4",
                "element": "N",
                "atomicNumber": 7,
                "position": [92.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [92.0, 45.0],
                    "box": [87.0, 34.0, 97.0, 46.0]
                }
            },
            { "id": "n5", "element": "C", "atomicNumber": 6, "position": [110.0, 22.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n2", "end": "n4", "order": 2, "strokeWidth": 0.85, "double": { "placement": "center" } },
            { "id": "b2", "begin": "n2", "end": "n1", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b4", "begin": "n4", "end": "n5", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2, "{polygons:?}");

    let axes: Vec<_> = polygons
        .iter()
        .map(|polygon| bond_axis_from_points(polygon).expect("bond axis"))
        .collect();
    let first_direction =
        chemcore_engine::Point::new(axes[0].1.x - axes[0].0.x, axes[0].1.y - axes[0].0.y);
    let second_direction =
        chemcore_engine::Point::new(axes[1].1.x - axes[1].0.x, axes[1].1.y - axes[1].0.y);
    let first_angle = first_direction.y.atan2(first_direction.x).to_degrees();
    let second_angle = second_direction.y.atan2(second_direction.x).to_degrees();

    assert!(
        angular_distance(first_angle, second_angle) <= 1.0e-4,
        "{polygons:?} {first_angle} {second_angle}"
    );
}

#[test]
fn render_document_extends_center_double_lines_to_branch_bonds_and_branches_join_each_other() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [74.0, 68.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            },
            {
                "id": "b3",
                "begin": "n2",
                "end": "n4",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons = object_bond_polygons_with_ids(&render_document(&document));
    let center_double: Vec<_> = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points.clone())
        .collect();
    assert_eq!(center_double.len(), 2);
    let branch_up = polygons
        .iter()
        .find(|(bond_id, _)| bond_id == "b2")
        .map(|(_, points)| points.clone())
        .expect("upper branch polygon");
    let branch_down = polygons
        .iter()
        .find(|(bond_id, _)| bond_id == "b3")
        .map(|(_, points)| points.clone())
        .expect("lower branch polygon");

    assert!(
        center_double.iter().all(|polygon| {
            let end_points = [polygon[1], polygon[2]];
            end_points
                .iter()
                .all(|point| point_lies_on_polygon_boundary(*point, &branch_up, 1.0e-4))
                || end_points
                    .iter()
                    .all(|point| point_lies_on_polygon_boundary(*point, &branch_down, 1.0e-4))
        }),
        "{center_double:?} {branch_up:?} {branch_down:?}"
    );
    assert!(center_double.iter().any(|polygon| {
        [polygon[1], polygon[2]]
            .iter()
            .all(|point| point_lies_on_polygon_boundary(*point, &branch_up, 1.0e-4))
    }));
    assert!(center_double.iter().any(|polygon| {
        [polygon[1], polygon[2]]
            .iter()
            .all(|point| point_lies_on_polygon_boundary(*point, &branch_down, 1.0e-4))
    }));
    assert!(shared_point_count(&branch_up, &branch_down, 1.0e-4) >= 2);
}

#[test]
fn render_document_joins_same_side_double_outer_polygons() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "left" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "left" }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let b1_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemcore_engine::Point::new(56.0, 40.0),
    );
    let b2_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b2",
        chemcore_engine::Point::new(56.0, 40.0),
    );
    let b1_main =
        side_double_main_polygon_for_bond(&polygons, "b1", chemcore_engine::Point::new(56.0, 40.0));
    let b2_main =
        side_double_main_polygon_for_bond(&polygons, "b2", chemcore_engine::Point::new(56.0, 40.0));

    assert_eq!(shared_point_count(&b1_outer, &b2_outer, 1.0e-4), 2);
    assert_eq!(shared_point_count(&b1_outer, &b1_main, 1.0e-4), 0);
    assert_eq!(shared_point_count(&b2_outer, &b2_main, 1.0e-4), 0);
}

#[test]
fn render_document_keeps_opposite_side_double_outer_polygons_inset() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "left" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let b1_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemcore_engine::Point::new(56.0, 40.0),
    );
    let b2_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b2",
        chemcore_engine::Point::new(56.0, 40.0),
    );

    assert_eq!(shared_point_count(&b1_outer, &b2_outer, 1.0e-4), 0);
}

#[test]
fn render_document_joins_side_double_outer_polygon_for_straight_through_180_degrees() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [92.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let b1_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemcore_engine::Point::new(56.0, 40.0),
    );
    let b2_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b2",
        chemcore_engine::Point::new(56.0, 40.0),
    );

    assert_eq!(shared_point_count(&b1_outer, &b2_outer, 1.0e-4), 2);
}

#[test]
fn render_document_joins_inner_side_double_outer_polygon_against_triple_outer_polygon() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 3,
                "strokeWidth": 0.85
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let b1_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemcore_engine::Point::new(56.0, 40.0),
    );
    let triple_shared = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b2")
        .any(|(_, points)| shared_point_count(&b1_outer, points, 1.0e-4) == 2);

    assert!(triple_shared, "{polygons:?}");
}

#[test]
fn render_document_retreats_side_double_outer_polygon_for_acute_single_bond_angles() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [30.0, 60.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let b1_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemcore_engine::Point::new(56.0, 40.0),
    );
    let single_shared = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b2")
        .any(|(_, points)| shared_point_count(&b1_outer, points, 1.0e-4) == 2);

    assert!(!single_shared, "{polygons:?}");
}

#[test]
fn render_document_retreats_side_double_outer_polygon_against_center_double_reference_axis() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [30.0, 60.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let side_double_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemcore_engine::Point::new(56.0, 40.0),
    );
    let centered_double: Vec<_> = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b2")
        .collect();

    assert!(centered_double
        .iter()
        .all(|(_, points)| { shared_point_count(&side_double_outer, points, 1.0e-4) == 0 }));
}

#[test]
fn render_document_scales_triple_offset_with_bond_length() {
    let short_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 3, "strokeWidth": 0.85 }
        ]),
    );
    let long_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 80.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [92.0, 80.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 3, "strokeWidth": 0.85 }
        ]),
    );

    let short_lines = object_bond_centerlines(&render_document(&short_document));
    let long_lines = object_bond_centerlines(&render_document(&long_document));

    let short_offset = short_lines
        .iter()
        .map(|(from, to)| ((from.y + to.y) / 2.0 - 40.0).abs())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();
    let long_offset = long_lines
        .iter()
        .map(|(from, to)| ((from.y + to.y) / 2.0 - 80.0).abs())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();

    assert!(
        (long_offset - short_offset * 2.0).abs() < 0.05,
        "{short_offset} {long_offset}"
    );
}

#[test]
fn render_document_keeps_solid_wedge_cap_width_constant_when_bond_is_longer() {
    let short_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "end"
                }
            }
        ]),
    );
    let long_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 80.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [92.0, 80.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "end"
                }
            }
        ]),
    );

    let short_polygon = render_document(&short_document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("short wedge polygon");
    let long_polygon = render_document(&long_document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("long wedge polygon");

    let short_width = ((short_polygon[1].x - short_polygon[2].x).powi(2)
        + (short_polygon[1].y - short_polygon[2].y).powi(2))
    .sqrt();
    let long_width = ((long_polygon[1].x - long_polygon[2].x).powi(2)
        + (long_polygon[1].y - long_polygon[2].y).powi(2))
    .sqrt();

    assert!(
        (short_width - long_width).abs() < 0.05,
        "{short_width} {long_width}"
    );
}

#[test]
fn render_document_uses_explicit_solid_wedge_wide_and_tip_widths() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [34.4, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.6,
                "boldWidth": 2.0,
                "wedgeWidth": 3.0,
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "end"
                }
            }
        ]),
    );

    let polygon = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("solid wedge polygon");

    let tip_width =
        ((polygon[0].x - polygon[3].x).powi(2) + (polygon[0].y - polygon[3].y).powi(2)).sqrt();
    let wide_width =
        ((polygon[1].x - polygon[2].x).powi(2) + (polygon[1].y - polygon[2].y).powi(2)).sqrt();

    assert!((tip_width - 0.6).abs() < 0.01, "{tip_width}");
    assert!((wide_width - 3.0).abs() < 0.01, "{wide_width}");
}

#[test]
fn render_document_uses_acs_template_wedge_width_for_legacy_json_without_wedge_width() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [34.4, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.6,
                "boldWidth": 2.0,
                "hashSpacing": 2.5,
                "bondSpacing": 18.0,
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "end"
                }
            }
        ]),
    );

    let polygon = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("solid wedge polygon");

    let wide_width =
        ((polygon[1].x - polygon[2].x).powi(2) + (polygon[1].y - polygon[2].y).powi(2)).sqrt();

    assert!((wide_width - 3.0).abs() < 0.01, "{wide_width}");
}

#[test]
fn render_document_emits_three_way_main_contact_patches() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 8.82], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [74.0, 71.18], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 },
            { "id": "b2", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n1", "end": "n4", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = centered_bond_polygons(&primitives, chemcore_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 3);
    assert_eq!(
        polygons.iter().filter(|points| points.len() == 5).count(),
        3
    );
    assert!(polygons.iter().all(|points| polygon_area(points) > 0.01));
}

#[test]
fn render_document_clips_solid_wedge_in_three_way_main_contact() {
    for (begin, end, wide_end) in [("n1", "n3", "begin"), ("n3", "n1", "end")] {
        let document = fragment_document(
            json!([
                { "id": "n1", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
                { "id": "n2", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
                { "id": "n3", "element": "C", "atomicNumber": 6, "position": [96.0, 4.0], "charge": 0, "numHydrogens": 0 },
                { "id": "n4", "element": "C", "atomicNumber": 6, "position": [68.0, 88.0], "charge": 0, "numHydrogens": 0 }
            ]),
            json!([
                {
                    "id": "b1",
                    "begin": begin,
                    "end": end,
                    "order": 1,
                    "strokeWidth": 0.85,
                    "stereo": {
                        "kind": "solid-wedge",
                        "wideEnd": wide_end
                    }
                },
                { "id": "b2", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 },
                { "id": "b3", "begin": "n1", "end": "n4", "order": 1, "strokeWidth": 0.85 }
            ]),
        );

        let primitives = render_document(&document);
        let polygons = object_bond_polygons_with_ids(&primitives);
        let wedge = polygons
            .iter()
            .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
            .expect("solid wedge polygon");
        assert_eq!(wedge.len(), 5, "{wide_end} {wedge:?}");
        assert!(wedge
            .iter()
            .any(|point| point.distance(chemcore_engine::Point::new(56.0, 40.0)) <= 0.001));

        let centered = centered_bond_polygons(&primitives, chemcore_engine::Point::new(56.0, 40.0));
        assert_eq!(centered.len(), 3, "{wide_end} {centered:?}");
        assert!(centered.iter().all(|points| polygon_area(points) > 0.01));
        let center_patches = polygons
            .iter()
            .filter_map(|(bond_id, points)| bond_id.is_empty().then_some(points))
            .collect::<Vec<_>>();
        assert!(center_patches.is_empty(), "{wide_end} {polygons:?}");
    }
}

#[test]
fn render_document_uses_extended_intersections_for_solid_wedge_three_way_contact() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [cdxml_cm_to_pt(7.5), cdxml_cm_to_pt(6.5)], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [cdxml_cm_to_pt(6.45), cdxml_cm_to_pt(6.5)], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [cdxml_cm_to_pt(7.682330586550277), cdxml_cm_to_pt(5.465951859337181)], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [cdxml_cm_to_pt(7.859121150491952), cdxml_cm_to_pt(7.486677251825204)], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b_left", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": cdxml_cm_to_pt(0.035) },
            { "id": "b_up", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": cdxml_cm_to_pt(0.035) },
            {
                "id": "b_wedge",
                "begin": "n1",
                "end": "n4",
                "order": 1,
                "strokeWidth": cdxml_cm_to_pt(0.035),
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "begin"
                }
            }
        ]),
    );

    let expected_up_wedge_intersection = chemcore_engine::Point::new(
        cdxml_cm_to_pt(7.5537589823596605),
        cdxml_cm_to_pt(6.295896144157522),
    );
    let contact_center = chemcore_engine::Point::new(cdxml_cm_to_pt(7.5), cdxml_cm_to_pt(6.5));
    let polygons = object_bond_polygons_with_ids(&render_document(&document));
    let up = polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b_up").then_some(points))
        .expect("upper single polygon");
    let wedge = polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b_wedge").then_some(points))
        .expect("solid wedge polygon");

    assert!(
        up.iter()
            .any(|point| point.distance(expected_up_wedge_intersection) <= cdxml_cm_to_pt(0.001)),
        "{up:?}"
    );
    assert!(
        wedge
            .iter()
            .any(|point| point.distance(expected_up_wedge_intersection) <= cdxml_cm_to_pt(0.001)),
        "{wedge:?}"
    );
    let has_edge = |points: &[chemcore_engine::Point],
                    first: chemcore_engine::Point,
                    second: chemcore_engine::Point| {
        (0..points.len()).any(|index| {
            let next = (index + 1) % points.len();
            (points[index].distance(first) <= cdxml_cm_to_pt(0.001)
                && points[next].distance(second) <= cdxml_cm_to_pt(0.001))
                || (points[index].distance(second) <= cdxml_cm_to_pt(0.001)
                    && points[next].distance(first) <= cdxml_cm_to_pt(0.001))
        })
    };
    assert!(
        has_edge(up, expected_up_wedge_intersection, contact_center),
        "{up:?}"
    );
    assert!(
        has_edge(wedge, expected_up_wedge_intersection, contact_center),
        "{wedge:?}"
    );
}

#[test]
fn render_document_emits_four_way_main_contact_patches() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [92.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [56.0, 4.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n5", "element": "C", "atomicNumber": 6, "position": [56.0, 76.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 },
            { "id": "b2", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n1", "end": "n4", "order": 1, "strokeWidth": 0.85 },
            { "id": "b4", "begin": "n1", "end": "n5", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = centered_bond_polygons(&primitives, chemcore_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 4);
    assert_eq!(
        polygons.iter().filter(|points| points.len() == 5).count(),
        4
    );
    assert!(polygons.iter().all(|points| polygon_area(points) > 0.01));
}
