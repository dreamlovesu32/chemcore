#![recursion_limit = "256"]

use chemcore_engine::{
    BondAnchor, BondVariant, Engine, Point, PointerEvent, RenderPrimitive, RenderRole,
    TextEditLayoutRequest, TextEditSelection, TextEditTarget, Tool, ToolState, DEFAULT_BOND_LENGTH,
};
use serde_json::json;

fn px(value: f64) -> f64 {
    chemcore_engine::px_to_pt(value)
}

fn px_point(x: f64, y: f64) -> Point {
    Point::new(px(x), px(y))
}

fn click(engine: &mut Engine, x: f64, y: f64) {
    engine.pointer_down(PointerEvent {
        x: px(x),
        y: px(y),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(x),
        y: px(y),
        button: Some(0),
        alt_key: false,
    });
}

fn tool_state(bond_variant: BondVariant) -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant,
        ..ToolState::default()
    }
}

fn delete_tool_state() -> ToolState {
    ToolState {
        active_tool: Tool::Delete,
        bond_variant: BondVariant::Single,
        ..ToolState::default()
    }
}

fn select_tool_state() -> ToolState {
    ToolState {
        active_tool: Tool::Select,
        bond_variant: BondVariant::Single,
        ..ToolState::default()
    }
}

fn free_anchor(point: Point) -> BondAnchor {
    BondAnchor {
        node_id: None,
        point,
        label_anchor: None,
    }
}

fn node_anchor(node_id: &str, point: Point) -> BondAnchor {
    BondAnchor {
        node_id: Some(node_id.to_string()),
        point,
        label_anchor: None,
    }
}

fn first_glyph_anchor(label: &chemcore_engine::NodeLabel) -> Point {
    glyph_anchor(label, 0)
}

fn last_glyph_anchor(label: &chemcore_engine::NodeLabel) -> Point {
    glyph_anchor(label, label.glyph_polygons.len().saturating_sub(1))
}

fn normal_source_run(text: &str) -> chemcore_engine::LabelRun {
    chemcore_engine::LabelRun {
        text: text.to_string(),
        font_family: Some("Arial".to_string()),
        font_size: Some(chemcore_engine::DEFAULT_TEXT_FONT_SIZE_PT),
        fill: Some("#000000".to_string()),
        font_weight: Some(400),
        font_style: Some("normal".to_string()),
        underline: Some(false),
        script: Some("normal".to_string()),
    }
}

fn glyph_anchor(label: &chemcore_engine::NodeLabel, index: usize) -> Point {
    let polygon = label
        .glyph_polygons
        .get(index)
        .expect("label should have glyph polygons");
    let bounds = polygon_bounds(polygon);
    Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5)
}

fn polygon_bounds(points: &[[f64; 2]]) -> [f64; 4] {
    assert!(!points.is_empty(), "polygon should not be empty");
    points.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |bounds, point| {
            [
                bounds[0].min(point[0]),
                bounds[1].min(point[1]),
                bounds[2].max(point[0]),
                bounds[3].max(point[1]),
            ]
        },
    )
}

fn glyph_polygons_bounds(polygons: &[Vec<[f64; 2]>]) -> [f64; 4] {
    assert!(!polygons.is_empty(), "label should have glyph polygons");
    polygons.iter().flat_map(|polygon| polygon.iter()).fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |bounds, point| {
            [
                bounds[0].min(point[0]),
                bounds[1].min(point[1]),
                bounds[2].max(point[0]),
                bounds[3].max(point[1]),
            ]
        },
    )
}

fn load_single_carbon_node(engine: &mut Engine) {
    let document = json!({
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_single_atom",
            "title": "single atom",
            "page": { "width": px(400.0), "height": px(320.0), "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": chemcore_engine::DEFAULT_BOND_STROKE,
                "fontFamily": "Arial",
                "fontSize": chemcore_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT
            }
        },
        "objects": [{
            "id": "obj_molecule_001",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_001" }
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, px(400.0), px(320.0)],
                    "nodes": [{
                        "id": "n1",
                        "element": "C",
                        "atomicNumber": 6,
                        "position": [px(300.0), px(260.0)],
                        "charge": 0,
                        "numHydrogens": 0
                    }],
                    "bonds": []
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("single atom document should load");
}

fn load_one_bond_terminal_node(engine: &mut Engine) {
    let document = json!({
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_one_bond_atom",
            "title": "one bond atom",
            "page": { "width": px(400.0), "height": px(320.0), "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": chemcore_engine::DEFAULT_BOND_STROKE,
                "fontFamily": "Arial",
                "fontSize": chemcore_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT
            }
        },
        "objects": [{
            "id": "obj_molecule_001",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_001" }
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, px(400.0), px(320.0)],
                    "nodes": [
                        {
                            "id": "n1",
                            "element": "C",
                            "atomicNumber": 6,
                            "position": [px(300.0), px(260.0)],
                            "charge": 0,
                            "numHydrogens": 0
                        },
                        {
                            "id": "n2",
                            "element": "C",
                            "atomicNumber": 6,
                            "position": [px(240.0), px(260.0)],
                            "charge": 0,
                            "numHydrogens": 0
                        }
                    ],
                    "bonds": [{
                        "id": "b1",
                        "begin": "n1",
                        "end": "n2",
                        "order": 1,
                        "strokeWidth": chemcore_engine::DEFAULT_BOND_STROKE,
                        "lineStyles": {},
                        "lineWeights": {}
                    }]
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("one bond document should load");
}

fn load_two_bond_central_node(engine: &mut Engine) {
    let document = json!({
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_two_bond_atom",
            "title": "two bond atom",
            "page": { "width": px(400.0), "height": px(320.0), "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": chemcore_engine::DEFAULT_BOND_STROKE,
                "fontFamily": "Arial",
                "fontSize": chemcore_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT
            }
        },
        "objects": [{
            "id": "obj_molecule_001",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_001" }
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, px(400.0), px(320.0)],
                    "nodes": [
                        {
                            "id": "n1",
                            "element": "C",
                            "atomicNumber": 6,
                            "position": [px(300.0), px(260.0)],
                            "charge": 0,
                            "numHydrogens": 0
                        },
                        {
                            "id": "n2",
                            "element": "C",
                            "atomicNumber": 6,
                            "position": [px(240.0), px(260.0)],
                            "charge": 0,
                            "numHydrogens": 0
                        },
                        {
                            "id": "n3",
                            "element": "C",
                            "atomicNumber": 6,
                            "position": [px(360.0), px(260.0)],
                            "charge": 0,
                            "numHydrogens": 0
                        }
                    ],
                    "bonds": [
                        {
                            "id": "b1",
                            "begin": "n1",
                            "end": "n2",
                            "order": 1,
                            "strokeWidth": chemcore_engine::DEFAULT_BOND_STROKE,
                            "lineStyles": {},
                            "lineWeights": {}
                        },
                        {
                            "id": "b2",
                            "begin": "n1",
                            "end": "n3",
                            "order": 1,
                            "strokeWidth": chemcore_engine::DEFAULT_BOND_STROKE,
                            "lineStyles": {},
                            "lineWeights": {}
                        }
                    ]
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("two bond document should load");
}

fn load_three_bond_central_oxygen(engine: &mut Engine, charge: i32) {
    let document = json!({
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_three_bond_oxygen",
            "title": "three bond oxygen",
            "page": { "width": px(400.0), "height": px(320.0), "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": chemcore_engine::DEFAULT_BOND_STROKE,
                "fontFamily": "Arial",
                "fontSize": chemcore_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT
            }
        },
        "objects": [{
            "id": "obj_molecule_001",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_001" }
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, px(400.0), px(320.0)],
                    "nodes": [
                        {
                            "id": "n1",
                            "element": "O",
                            "atomicNumber": 8,
                            "position": [px(300.0), px(260.0)],
                            "charge": charge,
                            "numHydrogens": 0,
                            "label": {
                                "text": "O",
                                "sourceText": "O",
                                "position": [px(296.5), px(263.5)],
                                "box": [px(296.5), px(254.0), px(303.5), px(264.0)],
                                "runs": [{
                                    "text": "O",
                                    "fontFamily": "Arial",
                                    "fontSize": chemcore_engine::DEFAULT_TEXT_FONT_SIZE_PT,
                                    "fill": "#000000",
                                    "fontWeight": 400,
                                    "fontStyle": "normal",
                                    "underline": false,
                                    "script": "chemical"
                                }],
                                "align": "left",
                                "layout": "attached-group",
                                "attachment": "node",
                                "anchor": "start",
                                "fontFamily": "Arial",
                                "fill": "#000000",
                                "fontSize": chemcore_engine::DEFAULT_TEXT_FONT_SIZE_PT,
                                "boxValue": [px(296.5), px(254.0), px(303.5), px(264.0)],
                                "meta": {
                                    "sourceRuns": [{
                                        "text": "O",
                                        "fontFamily": "Arial",
                                        "fontSize": chemcore_engine::DEFAULT_TEXT_FONT_SIZE_PT,
                                        "fill": "#000000",
                                        "fontWeight": 400,
                                        "fontStyle": "normal",
                                        "underline": false,
                                        "script": "chemical"
                                    }]
                                }
                            }
                        },
                        { "id": "n2", "element": "C", "atomicNumber": 6, "position": [px(240.0), px(260.0)], "charge": 0, "numHydrogens": 0 },
                        { "id": "n3", "element": "C", "atomicNumber": 6, "position": [px(360.0), px(260.0)], "charge": 0, "numHydrogens": 0 },
                        { "id": "n4", "element": "C", "atomicNumber": 6, "position": [px(300.0), px(200.0)], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": chemcore_engine::DEFAULT_BOND_STROKE, "lineStyles": {}, "lineWeights": {} },
                        { "id": "b2", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": chemcore_engine::DEFAULT_BOND_STROKE, "lineStyles": {}, "lineWeights": {} },
                        { "id": "b3", "begin": "n1", "end": "n4", "order": 1, "strokeWidth": chemcore_engine::DEFAULT_BOND_STROKE, "lineStyles": {}, "lineWeights": {} }
                    ]
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("three bond oxygen document should load");
}

fn assert_endpoint_target_near(session: &chemcore_engine::TextEditSession, expected: Point) {
    match session.target {
        TextEditTarget::EndpointLabel { x, y, .. } => {
            assert!((x - expected.x).abs() < 0.01, "{x} vs {}", expected.x);
            assert!((y - expected.y).abs() < 0.01, "{y} vs {}", expected.y);
        }
        ref other => panic!("unexpected target: {other:?}"),
    }
}

#[test]
fn begin_and_apply_text_object_edit_creates_text_scene_object() {
    let mut engine = Engine::new();
    let session = engine
        .begin_text_edit(px_point(120.0, 88.0))
        .expect("text session should be created");

    match &session.target {
        TextEditTarget::TextObject { object_id, x, y } => {
            assert!(object_id.is_none());
            assert_eq!((*x, *y), (px(120.0), px(88.0)));
        }
        other => panic!("unexpected target: {other:?}"),
    }

    let changed = engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "reaction note".to_string(),
        ..session
    });
    assert!(changed);

    let text_object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should exist");
    assert_eq!(
        text_object
            .payload
            .extra
            .get("text")
            .and_then(serde_json::Value::as_str),
        Some("reaction note")
    );
}

#[test]
fn switching_to_select_selects_latest_text_object_or_edited_molecule_label() {
    let mut text_object_engine = Engine::new();
    text_object_engine.set_tool_state(ToolState {
        active_tool: Tool::Text,
        bond_variant: BondVariant::Single,
        ..ToolState::default()
    });
    let session = text_object_engine
        .begin_text_edit(px_point(120.0, 88.0))
        .expect("text session should be created");
    assert!(
        text_object_engine.apply_text_edit(chemcore_engine::TextEditSession {
            text: "reaction note".to_string(),
            ..session
        })
    );
    let text_object_id = text_object_engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should exist")
        .id
        .clone();
    text_object_engine.set_tool_state(select_tool_state());
    assert_eq!(
        text_object_engine.state().selection.text_objects,
        vec![text_object_id]
    );

    let mut label_engine = Engine::new();
    label_engine.set_tool_state(tool_state(BondVariant::Single));
    click(&mut label_engine, 300.0, 260.0);
    label_engine.set_tool_state(ToolState {
        active_tool: Tool::Text,
        bond_variant: BondVariant::Single,
        ..ToolState::default()
    });
    let session = label_engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint text session should be created");
    assert!(
        label_engine.apply_text_edit(chemcore_engine::TextEditSession {
            text: "NH3".to_string(),
            ..session
        })
    );
    label_engine.set_tool_state(select_tool_state());
    assert_eq!(label_engine.state().selection.nodes.len(), 2);
    assert_eq!(label_engine.state().selection.bonds.len(), 1);

    let mut abbreviation_engine = Engine::new();
    abbreviation_engine.set_tool_state(tool_state(BondVariant::Single));
    click(&mut abbreviation_engine, 300.0, 260.0);
    abbreviation_engine.set_tool_state(ToolState {
        active_tool: Tool::Text,
        bond_variant: BondVariant::Single,
        ..ToolState::default()
    });
    let end_node_position = {
        let entry = abbreviation_engine
            .state()
            .document
            .editable_fragment()
            .expect("editable fragment should exist");
        entry
            .fragment
            .nodes
            .get(1)
            .expect("terminal node should exist")
            .position
    };
    let session = abbreviation_engine
        .begin_text_edit(Point::new(end_node_position[0], end_node_position[1]))
        .expect("terminal endpoint text session should be created");
    assert!(
        abbreviation_engine.apply_text_edit(chemcore_engine::TextEditSession {
            text: "Ph".to_string(),
            ..session
        })
    );
    abbreviation_engine.set_tool_state(select_tool_state());
    assert_eq!(abbreviation_engine.state().selection.nodes.len(), 2);
    assert_eq!(abbreviation_engine.state().selection.bonds.len(), 1);
    let label_bounds = abbreviation_engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .get(1)
        .and_then(|node| node.label.as_ref())
        .and_then(|label| label.bbox())
        .expect("terminal Ph label should have bounds");
    let selection_box = abbreviation_engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Rect {
                role: RenderRole::SelectionBox,
                x,
                y,
                width,
                height,
                ..
            } => Some([x, y, x + width, y + height]),
            _ => None,
        })
        .expect("complete molecule selection should render a group box");
    assert!(
        selection_box[0] <= label_bounds[0]
            && selection_box[1] <= label_bounds[1]
            && selection_box[2] >= label_bounds[2]
            && selection_box[3] >= label_bounds[3],
        "selection box {selection_box:?} should include Ph label bounds {label_bounds:?}"
    );
}

#[test]
fn preview_text_edit_layout_returns_kernel_caret_and_selection_geometry() {
    let mut engine = Engine::new();
    let session = engine
        .begin_text_edit(px_point(120.0, 88.0))
        .expect("text session should be created");

    let layout = engine.preview_text_edit_layout(&TextEditLayoutRequest {
        session: chemcore_engine::TextEditSession {
            text: "Hello".to_string(),
            ..session
        },
        selection: Some(TextEditSelection {
            anchor: 2,
            focus: 5,
        }),
    });

    assert_eq!(layout.text, "Hello");
    assert_eq!(layout.anchor_offset, [0.0, 0.0]);
    assert_eq!(layout.lines.len(), 1);
    assert_eq!(layout.lines[0].start_offset, 0);
    assert_eq!(layout.lines[0].end_offset, 5);
    assert_eq!(layout.caret_positions.len(), 6);
    assert_eq!(
        layout.caret_positions.last().map(|caret| caret.offset),
        Some(5)
    );
    assert_eq!(
        layout.selection.as_ref().map(|selection| selection.start),
        Some(2)
    );
    assert_eq!(
        layout.selection.as_ref().map(|selection| selection.end),
        Some(5)
    );
    assert_eq!(layout.selection_rects.len(), 1);
    assert_eq!(layout.selection_rects[0].y, layout.lines[0].y);
    assert_eq!(layout.selection_rects[0].height, layout.lines[0].height);
    assert!(layout.width >= px(8.0));
    assert!(layout.height >= layout.line_height);
}

#[test]
fn centered_text_object_edit_layout_anchors_editor_to_box_center() {
    let engine = Engine::new();
    let session = chemcore_engine::TextEditSession {
        target: TextEditTarget::TextObject {
            object_id: None,
            x: 50.0,
            y: 10.0,
        },
        text: "center".to_string(),
        source_runs: Vec::new(),
        font_family: Some("Arial".to_string()),
        font_size: Some(10.0),
        fill: Some("#000000".to_string()),
        align: Some("center".to_string()),
        line_height: Some(12.0),
        box_value: Some([-20.0, 0.0, 40.0, 20.0]),
        anchor_offset: None,
        preserve_lines: true,
        default_chemical: false,
    };

    let layout = engine.preview_text_edit_layout(&TextEditLayoutRequest {
        session,
        selection: None,
    });

    assert_eq!(layout.width, 40.0);
    assert_eq!(layout.height, 20.0);
    assert_eq!(layout.anchor_offset, [20.0, 0.0]);
    assert_eq!(layout.lines[0].x, 20.0);
    assert_eq!(layout.lines[0].text_anchor, "middle");
}

#[test]
fn centered_text_object_selection_rects_use_visual_line_bounds() {
    let engine = Engine::new();
    let session = chemcore_engine::TextEditSession {
        target: TextEditTarget::TextObject {
            object_id: None,
            x: 50.0,
            y: 10.0,
        },
        text: "center".to_string(),
        source_runs: Vec::new(),
        font_family: Some("Arial".to_string()),
        font_size: Some(10.0),
        fill: Some("#000000".to_string()),
        align: Some("center".to_string()),
        line_height: Some(12.0),
        box_value: Some([-20.0, 0.0, 40.0, 20.0]),
        anchor_offset: None,
        preserve_lines: true,
        default_chemical: false,
    };

    let layout = engine.preview_text_edit_layout(&TextEditLayoutRequest {
        session,
        selection: Some(TextEditSelection {
            anchor: 0,
            focus: 6,
        }),
    });
    let line_anchor_x = layout.lines[0].x;
    let selection_rect = layout
        .selection_rects
        .first()
        .expect("selection should produce a visual rect");
    let first_caret = layout
        .caret_positions
        .iter()
        .find(|caret| caret.offset == 0)
        .expect("first caret should exist");

    assert!(selection_rect.x < line_anchor_x, "{selection_rect:?}");
    assert!(
        selection_rect.x + selection_rect.width > line_anchor_x,
        "{selection_rect:?}"
    );
    assert_eq!(first_caret.x, selection_rect.x);
    assert_eq!(selection_rect.y, layout.lines[0].y);
    assert_eq!(selection_rect.height, layout.lines[0].height);
}

#[test]
fn text_object_default_box_height_wraps_descenders() {
    let mut engine = Engine::new();
    let session = engine
        .begin_text_edit(px_point(120.0, 88.0))
        .expect("text session should be created");

    assert_eq!(
        session.line_height,
        Some(chemcore_engine::DEFAULT_TEXT_LINE_HEIGHT_PT)
    );
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "apple".to_string(),
        ..session
    }));

    let text_object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should exist");
    let font_size = text_object
        .payload
        .extra
        .get("fontSize")
        .and_then(serde_json::Value::as_f64)
        .expect("font size should be persisted");
    let line_height = text_object
        .payload
        .extra
        .get("lineHeight")
        .and_then(serde_json::Value::as_f64)
        .expect("line height should be persisted");
    let bbox = text_object.payload.bbox.expect("text bbox");

    assert!(line_height > font_size, "{font_size} {line_height}");
    assert!(bbox[3] >= line_height, "{bbox:?} {line_height}");
}

#[test]
fn text_object_caret_treats_percent_as_wide_single_glyph() {
    let engine = Engine::new();
    let session = chemcore_engine::TextEditSession {
        target: TextEditTarget::TextObject {
            object_id: None,
            x: 0.0,
            y: 0.0,
        },
        text: "mol%)".to_string(),
        source_runs: Vec::new(),
        font_family: Some("Arial".to_string()),
        font_size: Some(10.0),
        fill: Some("#000000".to_string()),
        align: Some("left".to_string()),
        line_height: Some(12.0),
        box_value: None,
        anchor_offset: None,
        preserve_lines: true,
        default_chemical: false,
    };

    let layout = engine.preview_text_edit_layout(&TextEditLayoutRequest {
        session,
        selection: None,
    });
    let caret_x = |offset| {
        layout
            .caret_positions
            .iter()
            .find(|caret| caret.offset == offset)
            .map(|caret| caret.x)
            .expect("caret offset should exist")
    };

    assert!((caret_x(4) - caret_x(3) - 10.0).abs() < 1.0e-6);
}

#[test]
fn applying_centered_text_object_edit_stores_box_relative_to_anchor() {
    let mut engine = Engine::new();
    let session = chemcore_engine::TextEditSession {
        target: TextEditTarget::TextObject {
            object_id: None,
            x: 50.0,
            y: 10.0,
        },
        text: "center".to_string(),
        source_runs: Vec::new(),
        font_family: Some("Arial".to_string()),
        font_size: Some(10.0),
        fill: Some("#000000".to_string()),
        align: Some("center".to_string()),
        line_height: Some(12.0),
        box_value: Some([-20.0, 0.0, 40.0, 20.0]),
        anchor_offset: None,
        preserve_lines: true,
        default_chemical: false,
    };

    assert!(engine.apply_text_edit(session));
    let text_object = engine
        .state()
        .document
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
            .expect("text object should store box"),
    )
    .expect("text object box should deserialize");

    assert_eq!(text_object.transform.translate, [50.0, 10.0]);
    assert_eq!(box_value[0], -box_value[2] * 0.5);
}

#[test]
fn reopening_text_object_preserves_default_font_size_precision() {
    let mut engine = Engine::new();
    let session = engine
        .begin_text_edit(px_point(120.0, 88.0))
        .expect("text session should be created");

    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "reaction note".to_string(),
        ..session
    }));

    let text_object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should exist");

    let stored_font_size = text_object
        .payload
        .extra
        .get("fontSize")
        .and_then(serde_json::Value::as_f64)
        .expect("text object should persist font size");
    assert!((stored_font_size - chemcore_engine::DEFAULT_TEXT_FONT_SIZE_PT).abs() < 1.0e-6);

    let reopened = engine
        .begin_text_edit(px_point(120.0, 88.0))
        .expect("text object session should reopen");
    let reopened_font_size = reopened.font_size.expect("reopened session font size");
    assert!((reopened_font_size - chemcore_engine::DEFAULT_TEXT_FONT_SIZE_PT).abs() < 1.0e-6);
}

#[test]
fn delete_tool_click_on_text_box_removes_text_object() {
    let mut engine = Engine::new();
    let session = engine
        .begin_text_edit(px_point(120.0, 88.0))
        .expect("text session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "reaction note".to_string(),
        ..session
    }));

    let text_object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should exist");
    let translate = text_object.transform.translate;
    let bounds = text_object.payload.bbox.expect("text bbox");

    engine.set_tool_state(delete_tool_state());
    click(
        &mut engine,
        chemcore_engine::pt_to_px(translate[0] + bounds[2] * 0.5),
        chemcore_engine::pt_to_px(translate[1] + bounds[3] * 0.5),
    );
    assert!(engine
        .state()
        .document
        .objects
        .iter()
        .all(|object| object.object_type != "text"));
}

#[test]
fn endpoint_text_edit_defaults_to_chemical_and_formats_charge() {
    let mut engine = Engine::new();
    click(&mut engine, 300.0, 260.0);
    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .cloned()
        .expect("node should exist");

    let click_x = node.position[0] + px(6.0);
    let click_y = node.position[1] + px(4.0);
    let session = engine
        .begin_text_edit(Point::new(click_x, click_y))
        .expect("endpoint session should be created");
    assert!(session.default_chemical);
    match &session.target {
        TextEditTarget::EndpointLabel { x, y, .. } => {
            assert_eq!((*x, *y), (node.position[0], node.position[1]));
        }
        other => panic!("unexpected target: {other:?}"),
    }

    let changed = engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "Fe2+".to_string(),
        source_runs: Vec::new(),
        ..session
    });
    assert!(changed);

    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .expect("node should exist");
    let label = node.label.as_ref().expect("label should be generated");
    assert_eq!(label.text, "Fe2+");
    assert_eq!(label.runs.len(), 2);
    assert_eq!(label.align.as_deref(), Some("left"));
    assert_eq!(label.runs[0].text, "Fe");
    assert_eq!(label.runs[0].script.as_deref(), Some("normal"));
    assert_eq!(label.runs[1].text, "2+");
    assert_eq!(label.runs[1].script.as_deref(), Some("superscript"));
}

#[test]
fn endpoint_element_text_generates_implicit_hydrogen_label() {
    let mut engine = Engine::new();
    load_single_carbon_node(&mut engine);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint session should be created");

    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "N".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .expect("node should exist");
    assert_eq!(node.element, "N");
    assert_eq!(node.atomic_number, 7);
    assert_eq!(node.num_hydrogens, 3);
    assert_eq!(
        node.label.as_ref().map(|label| label.text.as_str()),
        Some("NH3")
    );
}

#[test]
fn generated_hydrogen_label_glyphs_do_not_become_bond_anchors() {
    let mut engine = Engine::new();
    load_single_carbon_node(&mut engine);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "N".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    let (node_id, node_position, hydrogen_anchor) = {
        let entry = engine
            .state()
            .document
            .editable_fragment()
            .expect("editable fragment should exist");
        let node = entry.fragment.nodes.first().expect("node should exist");
        assert_eq!(node.num_hydrogens, 3);
        let label = node.label.as_ref().expect("label should exist");
        assert_eq!(label.text, "NH3");
        assert!(label.glyph_polygons.len() >= 2);
        (node.id.clone(), node.point(), glyph_anchor(label, 1))
    };

    engine.set_tool_state(tool_state(BondVariant::Single));
    engine.pointer_down(PointerEvent {
        x: hydrogen_anchor.x,
        y: hydrogen_anchor.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: hydrogen_anchor.x,
        y: hydrogen_anchor.y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist");
    let nitrogen = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .expect("nitrogen should exist");
    assert_eq!(nitrogen.num_hydrogens, 2);
    let nitrogen_label = nitrogen
        .label
        .as_ref()
        .expect("nitrogen label should exist");
    assert_eq!(nitrogen_label.source_text.as_deref(), Some("NH2"));
    assert_eq!(nitrogen_label.text, "H2N");
    assert!(
        glyph_anchor(nitrogen_label, 2).distance(node_position) < 0.01,
        "reversed implicit hydrogen label should keep nitrogen as the attachment anchor: label={nitrogen_label:?}, node={node_position:?}"
    );
    let bond = entry
        .fragment
        .bonds
        .first()
        .expect("bond should be created");
    assert!(bond.begin == node_id || bond.end == node_id);
    let other_id = if bond.begin == node_id {
        &bond.end
    } else {
        &bond.begin
    };
    let other = entry
        .fragment
        .nodes
        .iter()
        .find(|node| &node.id == other_id)
        .expect("new atom should exist");
    assert!(
        (node_position.distance(other.point()) - DEFAULT_BOND_LENGTH).abs() < 0.01,
        "new bond should originate at nitrogen, not at the generated hydrogen glyph: distance={}, expected={}, nitrogen={:?}, other={:?}, hydrogen={:?}",
        node_position.distance(other.point()),
        DEFAULT_BOND_LENGTH,
        node_position,
        other.point(),
        hydrogen_anchor
    );
}

#[test]
fn generated_hydrogen_label_hover_can_move_between_adjacent_glyphs() {
    let mut engine = Engine::new();
    load_single_carbon_node(&mut engine);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "N".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    let (node_position, right_glyph, left_glyph) = {
        let entry = engine
            .state()
            .document
            .editable_fragment()
            .expect("editable fragment should exist");
        let node = entry.fragment.nodes.first().expect("node should exist");
        assert_eq!(node.num_hydrogens, 3);
        let label = node.label.as_ref().expect("label should exist");
        assert_eq!(label.text, "NH3");
        assert!(label.glyph_polygons.len() >= 3);
        (node.point(), glyph_anchor(label, 2), glyph_anchor(label, 1))
    };

    engine.set_tool_state(tool_state(BondVariant::Single));
    engine.pointer_move(PointerEvent {
        x: right_glyph.x,
        y: right_glyph.y,
        button: None,
        alt_key: false,
    });
    let hover = engine
        .state()
        .overlay
        .hover_endpoint
        .as_ref()
        .expect("right glyph should be focused");
    assert_eq!(
        hover
            .label_anchor
            .as_ref()
            .expect("label anchor should exist")
            .glyph_index,
        2
    );

    engine.pointer_move(PointerEvent {
        x: left_glyph.x,
        y: left_glyph.y,
        button: None,
        alt_key: false,
    });
    let hover = engine
        .state()
        .overlay
        .hover_endpoint
        .as_ref()
        .expect("left glyph should be focused");
    assert_eq!(
        hover
            .label_anchor
            .as_ref()
            .expect("label anchor should exist")
            .glyph_index,
        1
    );
    assert!(
        hover.point.distance(node_position) < 0.01,
        "implicit hydrogen hover should still anchor at nitrogen: hover={hover:?}, node={node_position:?}"
    );
}

fn implicit_hydrogen_case(element: &str, bond_order: u8) -> (u8, String) {
    let mut engine = Engine::new();
    load_single_carbon_node(&mut engine);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: element.to_string(),
        source_runs: Vec::new(),
        ..session
    }));
    if bond_order > 0 {
        let (node_id, node_position) = {
            let entry = engine
                .state()
                .document
                .editable_fragment()
                .expect("editable fragment should exist");
            let node = entry.fragment.nodes.first().expect("node should exist");
            (node.id.clone(), node.point())
        };
        assert!(engine.add_bond_between(
            node_anchor(&node_id, node_position),
            free_anchor(Point::new(
                node_position.x + DEFAULT_BOND_LENGTH,
                node_position.y
            )),
            bond_order,
        ));
    }
    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist");
    let node = entry.fragment.nodes.first().expect("node should exist");
    (
        node.num_hydrogens,
        node.label
            .as_ref()
            .and_then(|label| label.source_text.clone())
            .unwrap_or_default(),
    )
}

#[test]
fn third_period_and_later_halogens_follow_alternating_implicit_hydrogen_rule() {
    for element in ["Cl", "Br", "I", "At"] {
        for (bond_order, expected_hydrogens) in [
            (1, 0),
            (2, 1),
            (3, 0),
            (4, 1),
            (5, 0),
            (6, 1),
            (7, 0),
            (8, 0),
        ] {
            let (hydrogens, source_text) = implicit_hydrogen_case(element, bond_order);
            assert_eq!(
                hydrogens, expected_hydrogens,
                "{element} bond order {bond_order}"
            );
            let expected_text = if expected_hydrogens == 0 {
                element.to_string()
            } else {
                format!("{element}H")
            };
            assert_eq!(
                source_text, expected_text,
                "{element} bond order {bond_order}"
            );
        }
    }
}

#[test]
fn third_period_and_later_group_15_and_16_use_even_odd_expanded_valence_rule() {
    for (element, cases) in [
        (
            "P",
            &[(1_u8, 2_u8), (2, 1), (3, 0), (4, 1), (5, 0), (6, 0)][..],
        ),
        (
            "As",
            &[(1_u8, 2_u8), (2, 1), (3, 0), (4, 1), (5, 0), (6, 0)][..],
        ),
        (
            "S",
            &[(1_u8, 1_u8), (2, 0), (3, 1), (4, 0), (5, 1), (6, 0), (7, 0)][..],
        ),
        (
            "Se",
            &[(1_u8, 1_u8), (2, 0), (3, 1), (4, 0), (5, 1), (6, 0), (7, 0)][..],
        ),
    ] {
        for &(bond_order, expected_hydrogens) in cases {
            let (hydrogens, source_text) = implicit_hydrogen_case(element, bond_order);
            assert_eq!(
                hydrogens, expected_hydrogens,
                "{element} bond order {bond_order}"
            );
            let expected_text = match expected_hydrogens {
                0 => element.to_string(),
                1 => format!("{element}H"),
                count => format!("{element}H{count}"),
            };
            assert_eq!(
                source_text, expected_text,
                "{element} bond order {bond_order}"
            );
        }
    }
}

fn charged_implicit_hydrogen_case(
    element: &str,
    atomic_number: u8,
    bond_order: u8,
    charge: i32,
) -> (u8, String) {
    let mut engine = Engine::new();
    let mut nodes = vec![json!({
        "id": "n1",
        "element": element,
        "atomicNumber": atomic_number,
        "position": [px(300.0), px(260.0)],
        "charge": charge,
        "numHydrogens": 0
    })];
    let mut bonds: Vec<serde_json::Value> = Vec::new();
    if bond_order > 0 {
        nodes.push(json!({
            "id": "c1",
            "element": "C",
            "atomicNumber": 6,
            "position": [px(340.0), px(260.0)],
            "charge": 0,
            "numHydrogens": 0
        }));
        bonds.push(json!({
            "id": "b1",
            "begin": "n1",
            "end": "c1",
            "order": bond_order,
            "strokeWidth": chemcore_engine::DEFAULT_BOND_STROKE,
            "lineStyles": {},
            "lineWeights": {}
        }));
    }
    let document = json!({
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_charged_atom",
            "title": "charged atom",
            "page": { "width": px(400.0), "height": px(320.0), "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": chemcore_engine::DEFAULT_BOND_STROKE,
                "fontFamily": "Arial",
                "fontSize": chemcore_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT
            }
        },
        "objects": [{
            "id": "obj_molecule_001",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_001" }
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, px(400.0), px(320.0)],
                    "nodes": nodes,
                    "bonds": bonds
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("charged atom document should load");
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("charged atom endpoint session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: element.to_string(),
        source_runs: Vec::new(),
        ..session
    }));
    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist");
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .expect("charged node should exist");
    (
        node.num_hydrogens,
        node.label
            .as_ref()
            .and_then(|label| label.source_text.clone())
            .unwrap_or_default(),
    )
}

#[test]
fn third_period_and_later_main_group_charges_reduce_available_implicit_hydrogens() {
    for (element, atomic_number, bond_order, charge, expected_hydrogens) in [
        ("P", 15, 1, 1, 1),
        ("P", 15, 1, -1, 1),
        ("P", 15, 3, 1, 1),
        ("S", 16, 1, 1, 0),
        ("S", 16, 1, -1, 0),
        ("Cl", 17, 0, 1, 0),
        ("Cl", 17, 2, 1, 0),
        ("Se", 34, 3, -1, 0),
    ] {
        let (hydrogens, source_text) =
            charged_implicit_hydrogen_case(element, atomic_number, bond_order, charge);
        assert_eq!(
            hydrogens, expected_hydrogens,
            "{element} bond order {bond_order} charge {charge}"
        );
        let expected_text = match expected_hydrogens {
            0 => element.to_string(),
            1 => format!("{element}H"),
            count => format!("{element}H{count}"),
        };
        assert_eq!(
            source_text, expected_text,
            "{element} bond order {bond_order} charge {charge}"
        );
    }
}

#[test]
fn preview_text_edit_layout_matches_committed_endpoint_label_geometry() {
    let mut engine = Engine::new();
    click(&mut engine, 300.0, 260.0);
    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .cloned()
        .expect("node should exist");
    let session = engine
        .begin_text_edit(Point::new(node.position[0], node.position[1]))
        .expect("endpoint session should be created");
    let edited_session = chemcore_engine::TextEditSession {
        text: "H2SO4".to_string(),
        source_runs: Vec::new(),
        ..session.clone()
    };

    let layout = engine.preview_text_edit_layout(&TextEditLayoutRequest {
        session: edited_session.clone(),
        selection: Some(TextEditSelection {
            anchor: 5,
            focus: 5,
        }),
    });

    assert!(engine.apply_text_edit(edited_session));
    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .expect("node should exist");
    let label = node.label.as_ref().expect("label should be generated");
    assert!(layout.width > 0.0);
    assert!(layout.height > 0.0);
    assert!(layout.anchor_offset[0] >= 0.0);
    assert!(layout.anchor_offset[1] >= 0.0);
    let preview_text = layout.lines[0]
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();
    assert_eq!(preview_text, "H2SO4");
    assert_eq!(label.source_text.as_deref(), Some("H2SO4"));
}

#[test]
fn endpoint_text_edit_populates_kernel_glyph_polygons_for_abbreviation_labels() {
    let mut engine = Engine::new();
    click(&mut engine, 300.0, 260.0);
    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .cloned()
        .expect("node should exist");

    let session = engine
        .begin_text_edit(Point::new(node.position[0], node.position[1]))
        .expect("endpoint session should be created");
    let changed = engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "Ph".to_string(),
        source_runs: Vec::new(),
        ..session
    });
    assert!(changed);

    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .expect("node should exist");
    let label = node.label.as_ref().expect("label should be generated");
    assert_eq!(label.text, "Ph");
    assert_eq!(label.glyph_polygons.len(), 2, "{:?}", label.glyph_polygons);
    for polygon in &label.glyph_polygons {
        assert!(polygon.len() >= 4, "{polygon:?}");
        let bounds = polygon_bounds(polygon);
        assert!(bounds[2] > bounds[0], "{bounds:?}");
        assert!(bounds[3] > bounds[1], "{bounds:?}");
    }
    let first_bounds = polygon_bounds(&label.glyph_polygons[0]);
    let second_bounds = polygon_bounds(&label.glyph_polygons[1]);
    assert!(first_bounds[0] < second_bounds[0], "{label:?}");
}

#[test]
fn right_side_cf3_label_renders_reversed_but_anchors_on_carbon() {
    let mut engine = Engine::new();
    assert!(engine.add_bond_between(
        free_anchor(px_point(300.0, 260.0)),
        free_anchor(px_point(340.0, 260.0)),
        1,
    ));
    let left_node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .iter()
        .min_by(|left, right| left.position[0].total_cmp(&right.position[0]))
        .expect("left node should exist")
        .clone();
    let session = engine
        .begin_text_edit(Point::new(left_node.position[0], left_node.position[1]))
        .expect("endpoint session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "CF3".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == left_node.id)
        .expect("left node should still exist");
    let label = node.label.as_ref().expect("label should exist");
    assert_eq!(label.source_text.as_deref(), Some("CF3"));
    assert_eq!(label.text, "F3C");
    assert_eq!(label.glyph_polygons.len(), 3);
    let expected_anchor = glyph_anchor(label, 2);
    let reopened = engine
        .begin_text_edit(Point::new(node.position[0], node.position[1]))
        .expect("endpoint session should reopen");
    assert_endpoint_target_near(&reopened, expected_anchor);
}

#[test]
fn right_side_otms_label_keeps_tms_as_single_reversed_group() {
    let mut engine = Engine::new();
    assert!(engine.add_bond_between(
        free_anchor(px_point(300.0, 260.0)),
        free_anchor(px_point(340.0, 260.0)),
        1,
    ));
    let left_node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .iter()
        .min_by(|left, right| left.position[0].total_cmp(&right.position[0]))
        .expect("left node should exist")
        .clone();
    let session = engine
        .begin_text_edit(Point::new(left_node.position[0], left_node.position[1]))
        .expect("endpoint session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "OTMS".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == left_node.id)
        .expect("left node should still exist");
    let label = node.label.as_ref().expect("label should exist");
    assert_eq!(label.source_text.as_deref(), Some("OTMS"));
    assert_eq!(label.text, "TMSO");
    assert_eq!(
        node.meta
            .get("labelRecognition")
            .and_then(|value| value.get("status"))
            .and_then(serde_json::Value::as_str),
        Some("recognized")
    );
    let expected_anchor = glyph_anchor(label, 3);
    let reopened = engine
        .begin_text_edit(Point::new(node.position[0], node.position[1]))
        .expect("endpoint session should reopen");
    assert_endpoint_target_near(&reopened, expected_anchor);
}

#[test]
fn right_side_whole_labels_do_not_reverse_and_anchor_on_last_glyph() {
    for text in ["t-Bu", "XYZ"] {
        let mut engine = Engine::new();
        assert!(engine.add_bond_between(
            free_anchor(px_point(300.0, 260.0)),
            free_anchor(px_point(340.0, 260.0)),
            1,
        ));
        let left_node = engine
            .state()
            .document
            .editable_fragment()
            .expect("editable fragment should exist")
            .fragment
            .nodes
            .iter()
            .min_by(|left, right| left.position[0].total_cmp(&right.position[0]))
            .expect("left node should exist")
            .clone();
        let session = engine
            .begin_text_edit(Point::new(left_node.position[0], left_node.position[1]))
            .expect("endpoint session should be created");
        assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
            text: text.to_string(),
            source_runs: Vec::new(),
            ..session
        }));

        let node = engine
            .state()
            .document
            .editable_fragment()
            .expect("editable fragment should exist")
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == left_node.id)
            .expect("left node should still exist");
        let label = node.label.as_ref().expect("label should exist");
        assert_eq!(label.source_text.as_deref(), Some(text));
        assert_eq!(label.text, text);
        let expected_anchor = glyph_anchor(label, label.glyph_polygons.len() - 1);
        let reopened = engine
            .begin_text_edit(Point::new(node.position[0], node.position[1]))
            .expect("endpoint session should reopen");
        assert_endpoint_target_near(&reopened, expected_anchor);
    }
}

#[test]
fn nonchemical_endpoint_label_skips_recognition_and_red_box() {
    let mut engine = Engine::new();
    load_single_carbon_node(&mut engine);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint text session should start");

    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "NotAGroup".to_string(),
        source_runs: vec![normal_source_run("NotAGroup")],
        default_chemical: false,
        ..session
    }));

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .expect("node should exist");
    assert!(node.is_placeholder);
    assert!(node.meta.get("labelRecognition").is_none());
    assert!(node
        .label
        .as_ref()
        .expect("plain label should exist")
        .meta
        .get("labelRecognition")
        .is_none());
    assert!(!engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::DocumentDiagnostic,
            stroke: Some(stroke),
            ..
        } if stroke == "#d32f2f"
    )));

    let reopened = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint text session should reopen");
    assert!(!reopened.default_chemical);
}

#[test]
fn nonchemical_right_side_endpoint_label_keeps_text_and_anchors_last_glyph() {
    let mut engine = Engine::new();
    assert!(engine.add_bond_between(
        free_anchor(px_point(300.0, 260.0)),
        free_anchor(px_point(340.0, 260.0)),
        1,
    ));
    let left_node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .iter()
        .min_by(|left, right| left.position[0].total_cmp(&right.position[0]))
        .expect("left node should exist")
        .clone();
    let session = engine
        .begin_text_edit(Point::new(left_node.position[0], left_node.position[1]))
        .expect("endpoint session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "CF3".to_string(),
        source_runs: vec![normal_source_run("CF3")],
        default_chemical: false,
        ..session
    }));

    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == left_node.id)
        .expect("left node should still exist");
    assert!(node.meta.get("labelRecognition").is_none());
    let label = node.label.as_ref().expect("label should exist");
    assert_eq!(label.source_text.as_deref(), Some("CF3"));
    assert_eq!(label.text, "CF3");
    let expected_anchor = glyph_anchor(label, label.glyph_polygons.len() - 1);
    let reopened = engine
        .begin_text_edit(Point::new(node.position[0], node.position[1]))
        .expect("endpoint session should reopen");
    assert!(!reopened.default_chemical);
    assert_endpoint_target_near(&reopened, expected_anchor);
}

#[test]
fn reopening_endpoint_label_session_preserves_bbox_and_anchor_precision() {
    let mut engine = Engine::new();
    click(&mut engine, 300.0, 260.0);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "Ph".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .cloned()
        .expect("node should exist");
    let label_box = node
        .label
        .as_ref()
        .and_then(|label| label.bbox())
        .expect("label box");

    let reopened = engine
        .begin_text_edit(Point::new(node.position[0], node.position[1]))
        .expect("endpoint session should reopen");
    let box_value = reopened.box_value.expect("session box");
    let anchor_offset = reopened.anchor_offset.expect("session anchor offset");
    let expected_anchor = last_glyph_anchor(node.label.as_ref().expect("label should exist"));

    assert!((box_value[0] - label_box[0]).abs() < 1.0e-6);
    assert!((box_value[1] - label_box[1]).abs() < 1.0e-6);
    assert!((box_value[2] - label_box[2]).abs() < 1.0e-6);
    assert!((box_value[3] - label_box[3]).abs() < 1.0e-6);
    assert!((anchor_offset[0] - (expected_anchor.x - label_box[0])).abs() < 0.01);
    assert!((anchor_offset[1] - (expected_anchor.y - label_box[1])).abs() < 0.01);
    assert_endpoint_target_near(&reopened, expected_anchor);
}

#[test]
fn endpoint_text_edit_ignores_implausible_dom_label_measurements() {
    let mut engine = Engine::new();
    click(&mut engine, 300.0, 260.0);
    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .cloned()
        .expect("node should exist");

    let session = engine
        .begin_text_edit(Point::new(node.position[0], node.position[1]))
        .expect("endpoint session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "N".to_string(),
        source_runs: Vec::new(),
        box_value: Some([0.0, 0.0, px(3000.0), px(3000.0)]),
        anchor_offset: Some([px(2000.0), px(2000.0)]),
        ..session
    }));

    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .expect("node should exist");
    let label = node.label.as_ref().expect("label should be generated");
    let box_value = label.bbox().expect("label should have bounds");
    let glyph_box = glyph_polygons_bounds(&label.glyph_polygons);
    assert!((box_value[2] - box_value[0]) < px(28.0), "{box_value:?}");
    assert!((box_value[3] - box_value[1]) < px(28.0), "{box_value:?}");
    assert!((glyph_box[2] - glyph_box[0]) < px(32.0), "{glyph_box:?}");
    assert!((glyph_box[3] - glyph_box[1]) < px(32.0), "{glyph_box:?}");
    assert!(
        ((box_value[0] + box_value[2]) * 0.5 - node.position[0]).abs() < px(12.0),
        "{box_value:?} vs {:?}",
        node.position
    );
    assert!(
        ((box_value[1] + box_value[3]) * 0.5 - node.position[1]).abs() < px(12.0),
        "{box_value:?} vs {:?}",
        node.position
    );
}

#[test]
fn preview_text_runs_expands_chemical_source_runs_in_kernel() {
    let engine = Engine::new();
    let session = chemcore_engine::TextEditSession {
        target: TextEditTarget::TextObject {
            object_id: None,
            x: 0.0,
            y: 0.0,
        },
        text: "Fe2+".to_string(),
        source_runs: vec![chemcore_engine::LabelRun {
            text: "Fe2+".to_string(),
            font_family: Some("Arial".to_string()),
            font_size: Some(12.0),
            fill: Some("#000000".to_string()),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            script: Some("chemical".to_string()),
        }],
        font_family: Some("Arial".to_string()),
        font_size: Some(12.0),
        fill: Some("#000000".to_string()),
        align: Some("left".to_string()),
        line_height: Some(12.6),
        box_value: None,
        anchor_offset: None,
        preserve_lines: true,
        default_chemical: true,
    };

    let (source_runs, display_runs) = engine.preview_text_runs(&session);
    assert_eq!(source_runs.len(), 1);
    assert_eq!(source_runs[0].script.as_deref(), Some("chemical"));
    assert_eq!(display_runs.len(), 2);
    assert_eq!(display_runs[0].text, "Fe");
    assert_eq!(display_runs[0].script.as_deref(), Some("normal"));
    assert_eq!(display_runs[1].text, "2+");
    assert_eq!(display_runs[1].script.as_deref(), Some("superscript"));
}

#[test]
fn reopening_existing_endpoint_label_uses_stable_label_anchor() {
    let mut engine = Engine::new();
    click(&mut engine, 300.0, 260.0);
    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .cloned()
        .expect("node should exist");

    let session = engine
        .begin_text_edit(Point::new(node.position[0], node.position[1]))
        .expect("endpoint session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "Ph".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .expect("node should exist");
    let label = node.label.as_ref().expect("label should exist");
    let box_value = label.bbox().expect("label should have bbox");
    let expected_anchor = last_glyph_anchor(label);

    let reopened = engine
        .begin_text_edit(expected_anchor)
        .expect("existing label session should be created");
    assert_endpoint_target_near(&reopened, expected_anchor);
    let anchor_offset = reopened.anchor_offset.expect("session anchor offset");
    assert!((anchor_offset[0] - (expected_anchor.x - box_value[0])).abs() < 0.01);
    assert!((anchor_offset[1] - (expected_anchor.y - box_value[1])).abs() < 0.01);
}

#[test]
fn endpoint_label_anchor_tracks_terminal_double_status() {
    let mut engine = Engine::new();
    engine.set_tool_state(tool_state(BondVariant::Double));
    assert!(engine.add_bond_between(
        free_anchor(px_point(100.0, 100.0)),
        free_anchor(px_point(140.0, 100.0)),
        2,
    ));

    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist");
    let bond = entry.fragment.bonds.first().expect("bond should exist");
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(chemcore_engine::DoubleBondPlacement::Center)
    );
    let terminal_node = entry
        .fragment
        .nodes
        .iter()
        .max_by(|left, right| left.position[0].total_cmp(&right.position[0]))
        .expect("terminal node should exist")
        .clone();
    let terminal_session = engine
        .begin_text_edit(Point::new(
            terminal_node.position[0],
            terminal_node.position[1],
        ))
        .expect("endpoint session should be created");
    let terminal_anchor = match terminal_session.target.clone() {
        TextEditTarget::EndpointLabel { x, y, .. } => Point::new(x, y),
        other => panic!("unexpected target: {other:?}"),
    };
    assert!(
        (terminal_anchor.x - terminal_node.position[0]).abs() < 0.001,
        "{terminal_anchor:?} vs {:?}",
        terminal_node.position
    );
    assert!(
        (terminal_anchor.y - terminal_node.position[1]).abs() < 0.001,
        "{terminal_anchor:?} vs {:?}",
        terminal_node.position
    );
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "Ph".to_string(),
        source_runs: Vec::new(),
        ..terminal_session
    }));

    let reopened_terminal = engine
        .begin_text_edit(Point::new(
            terminal_node.position[0],
            terminal_node.position[1],
        ))
        .expect("existing endpoint label session should be created");
    let reopened_node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .iter()
        .find(|candidate| candidate.id == terminal_node.id)
        .expect("terminal node should still exist");
    let reopened_label_box = reopened_node
        .label
        .as_ref()
        .and_then(|label| label.bbox())
        .expect("reopened label box");
    let reopened_anchor = first_glyph_anchor(reopened_node.label.as_ref().expect("reopened label"));
    assert_endpoint_target_near(&reopened_terminal, reopened_anchor);
    let reopened_offset = reopened_terminal
        .anchor_offset
        .expect("reopened anchor offset");
    assert!((reopened_offset[0] - (reopened_anchor.x - reopened_label_box[0])).abs() < 0.01);
    assert!((reopened_offset[1] - (reopened_anchor.y - reopened_label_box[1])).abs() < 0.01);

    engine.set_tool_state(tool_state(BondVariant::Single));
    assert!(engine.add_single_bond_between(
        node_anchor(
            &terminal_node.id,
            Point::new(terminal_node.position[0], terminal_node.position[1]),
        ),
        free_anchor(px_point(172.0, 128.0)),
    ));

    let node_after_attachment = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .iter()
        .find(|candidate| candidate.id == terminal_node.id)
        .expect("terminal node should still exist")
        .position;
    let attached_session = engine
        .begin_text_edit(Point::new(
            node_after_attachment[0],
            node_after_attachment[1],
        ))
        .expect("attached endpoint label session should be created");
    let attached_node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .iter()
        .find(|candidate| candidate.id == terminal_node.id)
        .expect("terminal node should still exist");
    let attached_label_box = attached_node
        .label
        .as_ref()
        .and_then(|label| label.bbox())
        .expect("attached label box");
    let attached_anchor = first_glyph_anchor(attached_node.label.as_ref().expect("attached label"));
    assert_endpoint_target_near(&attached_session, attached_anchor);
    let attached_offset = attached_session
        .anchor_offset
        .expect("attached anchor offset");
    assert!((attached_offset[0] - (attached_anchor.x - attached_label_box[0])).abs() < 0.01);
    assert!((attached_offset[1] - (attached_anchor.y - attached_label_box[1])).abs() < 0.01);
}

#[test]
fn endpoint_label_reanchors_when_double_bond_style_changes() {
    let mut engine = Engine::new();
    engine.set_tool_state(tool_state(BondVariant::Double));
    assert!(engine.add_bond_between(
        free_anchor(px_point(100.0, 100.0)),
        free_anchor(px_point(140.0, 100.0)),
        2,
    ));

    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist");
    let terminal_node = entry
        .fragment
        .nodes
        .iter()
        .max_by(|left, right| left.position[0].total_cmp(&right.position[0]))
        .expect("terminal node should exist")
        .clone();
    let bond_id = entry
        .fragment
        .bonds
        .first()
        .expect("bond should exist")
        .id
        .clone();
    let session = engine
        .begin_text_edit(Point::new(
            terminal_node.position[0],
            terminal_node.position[1],
        ))
        .expect("endpoint session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "Ph".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    for _ in 0..3 {
        assert!(engine.cycle_bond_center_style(&bond_id));
        let entry = engine
            .state()
            .document
            .editable_fragment()
            .expect("editable fragment should exist");
        let bond = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id)
            .expect("bond should exist");
        if bond.double.as_ref().map(|double| double.placement)
            == Some(chemcore_engine::DoubleBondPlacement::Center)
        {
            break;
        }
    }

    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist");
    let bond = entry
        .fragment
        .bonds
        .iter()
        .find(|bond| bond.id == bond_id)
        .expect("bond should exist");
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(chemcore_engine::DoubleBondPlacement::Center)
    );
    let terminal_node_position = entry
        .fragment
        .nodes
        .iter()
        .find(|candidate| candidate.id == terminal_node.id)
        .expect("terminal node should exist")
        .position;
    let centered_session = engine
        .begin_text_edit(Point::new(
            terminal_node_position[0],
            terminal_node_position[1],
        ))
        .expect("centered endpoint label session should be created");
    let centered_node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .iter()
        .find(|candidate| candidate.id == terminal_node.id)
        .expect("terminal node should exist");
    let centered_label_box = centered_node
        .label
        .as_ref()
        .and_then(|label| label.bbox())
        .expect("centered label box");
    let centered_anchor = first_glyph_anchor(centered_node.label.as_ref().expect("centered label"));
    assert_endpoint_target_near(&centered_session, centered_anchor);
    let centered_offset = centered_session
        .anchor_offset
        .expect("centered anchor offset");
    assert!((centered_offset[0] - (centered_anchor.x - centered_label_box[0])).abs() < 0.01);
    assert!((centered_offset[1] - (centered_anchor.y - centered_label_box[1])).abs() < 0.01);
}

#[test]
fn text_mode_hover_prefers_label_box_over_endpoint_focus() {
    let mut engine = Engine::new();
    click(&mut engine, 300.0, 260.0);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "Ph".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .expect("node should exist");
    let label_box = node
        .label
        .as_ref()
        .and_then(|label| label.bbox())
        .expect("label box");

    engine.set_tool_state(ToolState {
        active_tool: Tool::Text,
        bond_variant: BondVariant::Single,
        ..ToolState::default()
    });
    engine.pointer_move(PointerEvent {
        x: (label_box[0] + label_box[2]) * 0.5,
        y: (label_box[1] + label_box[3]) * 0.5,
        button: None,
        alt_key: false,
    });

    assert!(engine.state().overlay.hover_text_box.is_some());
    assert!(engine.state().overlay.hover_endpoint.is_none());
}

#[test]
fn text_mode_hover_on_label_glyph_still_focuses_whole_label_box() {
    let mut engine = Engine::new();
    click(&mut engine, 300.0, 260.0);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "NH3".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    let glyph_point = {
        let entry = engine
            .state()
            .document
            .editable_fragment()
            .expect("editable fragment should exist");
        let node = entry.fragment.nodes.first().expect("node should exist");
        glyph_anchor(node.label.as_ref().expect("label should exist"), 1)
    };

    engine.set_tool_state(ToolState {
        active_tool: Tool::Text,
        bond_variant: BondVariant::Single,
        ..ToolState::default()
    });
    engine.pointer_move(PointerEvent {
        x: glyph_point.x,
        y: glyph_point.y,
        button: None,
        alt_key: false,
    });

    assert!(engine.state().overlay.hover_text_box.is_some());
    assert!(engine.state().overlay.hover_endpoint.is_none());
    assert!(!engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::HoverLabelGlyph,
            ..
        }
    )));
}

#[test]
fn text_mode_hover_focuses_plain_text_box_bounds() {
    let mut engine = Engine::new();
    let session = engine
        .begin_text_edit(px_point(120.0, 88.0))
        .expect("text session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "note".to_string(),
        ..session
    }));

    let text_object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should exist");
    let object_id = text_object.id.clone();
    let translate = text_object.transform.translate;
    let bounds = text_object.payload.bbox.expect("text bbox");

    engine.set_tool_state(ToolState {
        active_tool: Tool::Text,
        bond_variant: BondVariant::Single,
        ..ToolState::default()
    });
    engine.pointer_move(PointerEvent {
        x: translate[0] + bounds[2] * 0.5,
        y: translate[1] + bounds[3] * 0.5,
        button: None,
        alt_key: false,
    });

    let hover = engine
        .state()
        .overlay
        .hover_text_box
        .as_ref()
        .expect("text hover box should exist");
    assert_eq!(hover.object_id.as_deref(), Some(object_id.as_str()));
    assert!(engine.state().overlay.hover_endpoint.is_none());
}

#[test]
fn endpoint_label_stores_composite_abbreviation_recognition_metadata() {
    let mut engine = Engine::new();
    load_one_bond_terminal_node(&mut engine);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint text session should start");

    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "CO2Boc".to_string(),
        ..session
    }));

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .expect("node should exist");
    assert!(node.is_placeholder);
    let recognition = node
        .meta
        .get("labelRecognition")
        .expect("recognition metadata should be stored");
    assert_eq!(recognition["status"], "recognized");
    assert_eq!(recognition["canonicalLabel"], "CO2Boc");
    assert_eq!(recognition["source"], "valence-parser");
    let components = recognition["components"].as_array().unwrap();
    assert_eq!(components[0]["label"], "C");
    assert_eq!(components[1]["label"], "O");
    assert_eq!(components[1]["bondOrderToParent"], 2);
    assert_eq!(components[2]["label"], "O");
    assert_eq!(components[3]["label"], "Boc");
    assert_eq!(components[3]["bondOrderToParent"], 1);
    let expansion = recognition
        .get("expansion")
        .expect("recognized label should include structural expansion");
    assert_eq!(expansion["schema"], "chemcore.functionalGroupExpansion.v1");
    assert_eq!(expansion["connectionKind"], "terminal");
    assert_eq!(expansion["complete"], true);
    assert_eq!(expansion["attachments"][0]["role"], "external");
    assert_eq!(expansion["attachments"][0]["atomId"], "c1");
    assert_eq!(expansion["atoms"].as_array().unwrap().len(), 10);
    assert!(expansion["bonds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|bond| bond["begin"] == "c1" && bond["end"] == "o1" && bond["order"] == 2));

    let label_recognition = node
        .label
        .as_ref()
        .and_then(|label| label.meta.get("labelRecognition"))
        .expect("label should carry recognition metadata for rendering");
    assert_eq!(label_recognition["status"], "recognized");
    assert_eq!(label_recognition["expansion"], recognition["expansion"]);
}

#[test]
fn context_expand_label_replaces_terminal_abbreviation_with_fragment() {
    let mut engine = Engine::new();
    load_one_bond_terminal_node(&mut engine);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint text session should start");

    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "Et".to_string(),
        ..session
    }));
    engine.set_tool_state(select_tool_state());
    engine.select_at_point(px_point(300.0, 260.0), false);

    assert!(engine.expand_labels_in_selection());
    let entry = engine.state().document.editable_fragment().unwrap();
    assert!(entry.fragment.nodes.iter().all(|node| node.id != "n1"));
    assert!(entry.fragment.nodes.iter().any(|node| node.id == "n1_c1"));
    assert!(entry.fragment.nodes.iter().any(|node| node.id == "n1_c2"));
    assert!(entry.fragment.bonds.iter().any(|bond| {
        (bond.begin == "n1_c1" && bond.end == "n1_c2")
            || (bond.begin == "n1_c2" && bond.end == "n1_c1")
    }));
    assert!(entry
        .fragment
        .bonds
        .iter()
        .any(|bond| bond.id == "b1" && (bond.begin == "n1_c1" || bond.end == "n1_c1")));
    assert!(engine
        .state()
        .selection
        .nodes
        .contains(&"n1_c1".to_string()));

    assert!(engine.undo());
    let entry = engine.state().document.editable_fragment().unwrap();
    assert!(entry.fragment.nodes.iter().any(|node| node.id == "n1"));
}

#[test]
fn zero_connection_endpoint_rejects_terminal_abbreviations() {
    let mut engine = Engine::new();
    load_single_carbon_node(&mut engine);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint text session should start");

    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "CO2Boc".to_string(),
        ..session
    }));

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .expect("node should exist");
    assert_eq!(
        node.meta
            .get("labelRecognition")
            .and_then(|value| value.get("status"))
            .and_then(serde_json::Value::as_str),
        Some("invalid")
    );
    assert!(node
        .meta
        .get("labelRecognition")
        .and_then(|value| value.get("expansion"))
        .is_none());
}

#[test]
fn endpoint_label_renders_red_box_for_unrecognized_abbreviation() {
    let mut engine = Engine::new();
    load_single_carbon_node(&mut engine);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint text session should start");

    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "NotAGroup".to_string(),
        ..session
    }));

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .expect("node should exist");
    assert_eq!(
        node.meta
            .get("labelRecognition")
            .and_then(|value| value.get("status"))
            .and_then(serde_json::Value::as_str),
        Some("invalid")
    );

    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::DocumentDiagnostic,
            stroke: Some(stroke),
            ..
        } if stroke == "#d32f2f"
    )));
}

#[test]
fn two_connection_endpoint_accepts_bridge_abbreviations() {
    let mut engine = Engine::new();
    load_two_bond_central_node(&mut engine);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint text session should start");

    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "NTs".to_string(),
        ..session
    }));

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .expect("node should exist");
    let recognition = node
        .meta
        .get("labelRecognition")
        .expect("bridge recognition metadata should be stored");
    assert_eq!(recognition["status"], "recognized");
    assert_eq!(recognition["groupKind"], "bridge-fragment");
    assert_eq!(recognition["canonicalLabel"], "NTs");
    let components = recognition["components"].as_array().unwrap();
    assert_eq!(components[0]["label"], "N");
    assert_eq!(components[1]["label"], "Ts");
    let expansion = recognition["expansion"].as_object().unwrap();
    assert_eq!(expansion["connectionKind"], "bridge");
    assert_eq!(expansion["complete"], true);
    assert_eq!(expansion["attachments"][0]["atomId"], "n1");
    assert_eq!(expansion["attachments"][1]["atomId"], "n1");

    let mut engine = Engine::new();
    load_two_bond_central_node(&mut engine);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint text session should restart");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "SO".to_string(),
        ..session
    }));
    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .expect("node should exist");
    let recognition = node
        .meta
        .get("labelRecognition")
        .expect("SO bridge recognition metadata should be stored");
    assert_eq!(recognition["status"], "recognized");
    assert_eq!(recognition["groupKind"], "bridge-fragment");
    assert_eq!(recognition["canonicalLabel"], "SO");
    assert_eq!(recognition["formula"], "-S(=O)-");
    let expansion = recognition["expansion"].as_object().unwrap();
    assert_eq!(expansion["connectionKind"], "bridge");
    assert_eq!(expansion["attachments"][0]["atomId"], "s1");
    assert_eq!(expansion["attachments"][1]["atomId"], "s1");
    assert!(expansion["bonds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|bond| bond["begin"] == "s1" && bond["end"] == "o1" && bond["order"] == 2));
}

#[test]
fn two_connection_endpoint_rejects_terminal_abbreviations() {
    let mut engine = Engine::new();
    load_two_bond_central_node(&mut engine);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint text session should start");

    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "Boc".to_string(),
        ..session
    }));

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .expect("node should exist");
    assert_eq!(
        node.meta
            .get("labelRecognition")
            .and_then(|value| value.get("status"))
            .and_then(serde_json::Value::as_str),
        Some("invalid")
    );
}

#[test]
fn two_connection_shortcut_nitrogen_edited_to_plain_n_is_invalid() {
    let mut engine = Engine::new();
    load_two_bond_central_node(&mut engine);
    engine.set_tool_state(tool_state(BondVariant::Single));
    engine.pointer_move(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: None,
        alt_key: false,
    });
    assert!(engine.replace_hovered_endpoint_label("N"));

    let label_box = {
        let entry = engine.state().document.editable_fragment().unwrap();
        let node = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == "n1")
            .expect("node should exist");
        assert_eq!(
            node.label
                .as_ref()
                .and_then(|label| label.source_text.as_deref()),
            Some("NH")
        );
        assert!(
            node.meta.get("labelRecognition").is_none(),
            "generated two-connection NH should be valid"
        );
        node.label.as_ref().and_then(|label| label.bbox()).unwrap()
    };
    let session = engine
        .begin_text_edit(Point::new(
            (label_box[0] + label_box[2]) * 0.5,
            (label_box[1] + label_box[3]) * 0.5,
        ))
        .expect("label edit session should start");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "N".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .expect("node should exist");
    assert_eq!(
        node.label
            .as_ref()
            .and_then(|label| label.source_text.as_deref()),
        Some("N")
    );
    assert_eq!(
        node.meta
            .get("labelRecognition")
            .and_then(|value| value.get("status"))
            .and_then(serde_json::Value::as_str),
        Some("invalid")
    );
}

#[test]
fn neutral_three_connection_oxygen_label_is_invalid() {
    let mut engine = Engine::new();
    load_three_bond_central_oxygen(&mut engine, 0);

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .expect("oxygen node should exist");
    assert_eq!(
        node.meta
            .get("labelRecognition")
            .and_then(|value| value.get("status"))
            .and_then(serde_json::Value::as_str),
        Some("invalid")
    );
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::DocumentDiagnostic,
            stroke: Some(stroke),
            ..
        } if stroke == "#d32f2f"
    )));
}

#[test]
fn positive_three_connection_oxygen_label_is_valid() {
    let mut engine = Engine::new();
    load_three_bond_central_oxygen(&mut engine, 1);

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .expect("oxygen node should exist");
    assert!(
        node.meta.get("labelRecognition").is_none(),
        "three-connection oxygen should be valid when it carries +1 charge"
    );
}

#[test]
fn endpoint_text_editor_preview_expands_right_even_for_reversed_labels() {
    let mut engine = Engine::new();
    load_single_carbon_node(&mut engine);
    engine.set_tool_state(tool_state(BondVariant::Single));
    click(&mut engine, 300.0, 260.0);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
        .expect("endpoint text session should start");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "CO2Et".to_string(),
        ..session
    }));

    let label_anchor = {
        let entry = engine.state().document.editable_fragment().unwrap();
        let node = entry.fragment.nodes.first().expect("node should exist");
        assert_ne!(
            node.label.as_ref().expect("label should exist").text,
            "CO2Et"
        );
        first_glyph_anchor(node.label.as_ref().expect("label should exist"))
    };
    let edit_session = engine
        .begin_text_edit(label_anchor)
        .expect("label edit session should start");
    let layout = engine.preview_text_edit_layout(&TextEditLayoutRequest {
        session: edit_session,
        selection: None,
    });
    let line_text = layout.lines[0]
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();
    assert_eq!(line_text, "CO2Et");
}
