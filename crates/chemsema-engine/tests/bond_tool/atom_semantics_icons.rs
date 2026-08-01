use super::*;

#[test]
fn hetero_atom_charge_symbols_update_hydrogens_and_invalid_state() {
    fn terminal_nitrogen_engine(symbol_kind: BracketKind) -> Engine {
        let mut engine = Engine::new();
        load_symbol_direction_document(
            &mut engine,
            json!([
                {"id": "n1", "element": "N", "atomicNumber": 7, "position": [100.0, 100.0], "charge": 0, "numHydrogens": 0},
                {"id": "c1", "element": "C", "atomicNumber": 6, "position": [130.0, 100.0], "charge": 0, "numHydrogens": 0}
            ]),
            json!([
                {"id": "b1", "begin": "n1", "end": "c1", "order": 1}
            ]),
        );
        engine.set_tool_state(ToolState {
            active_tool: Tool::Symbol,
            symbol_kind,
            ..ToolState::default()
        });
        click(&mut engine, 100.0, 100.0);
        engine
    }

    let engine = terminal_nitrogen_engine(BracketKind::Plus);
    let nitrogen = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .unwrap();
    assert_eq!(nitrogen.charge, 1);
    assert_eq!(nitrogen.num_hydrogens, 3);
    assert_eq!(
        nitrogen
            .meta
            .get("chargeSymbolInvalid")
            .and_then(|value| value.as_bool()),
        None
    );

    let engine = terminal_nitrogen_engine(BracketKind::Minus);
    let nitrogen = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .unwrap();
    assert_eq!(nitrogen.charge, -1);
    assert_eq!(nitrogen.num_hydrogens, 1);
    assert_eq!(
        nitrogen
            .meta
            .get("chargeSymbolInvalid")
            .and_then(|value| value.as_bool()),
        None
    );

    let mut engine = Engine::new();
    load_symbol_direction_document(
        &mut engine,
        json!([
            {"id": "n1", "element": "N", "atomicNumber": 7, "position": [100.0, 100.0], "charge": 0, "numHydrogens": 0},
            {"id": "c1", "element": "C", "atomicNumber": 6, "position": [70.0, 100.0], "charge": 0, "numHydrogens": 0},
            {"id": "c2", "element": "C", "atomicNumber": 6, "position": [130.0, 100.0], "charge": 0, "numHydrogens": 0},
            {"id": "c3", "element": "C", "atomicNumber": 6, "position": [100.0, 130.0], "charge": 0, "numHydrogens": 0}
        ]),
        json!([
            {"id": "b1", "begin": "n1", "end": "c1", "order": 1},
            {"id": "b2", "begin": "n1", "end": "c2", "order": 1},
            {"id": "b3", "begin": "n1", "end": "c3", "order": 1}
        ]),
    );
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::Minus,
        ..ToolState::default()
    });
    click(&mut engine, 100.0, 100.0);
    let nitrogen = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .unwrap();
    assert_eq!(nitrogen.charge, -1);
    assert_eq!(
        nitrogen
            .meta
            .get("chargeSymbolInvalid")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
}

#[test]
fn third_period_hetero_charge_symbols_use_main_group_hydrogen_rule() {
    fn terminal_hetero_engine(
        element: &str,
        atomic_number: u8,
        symbol_kind: BracketKind,
    ) -> Engine {
        let mut engine = Engine::new();
        load_symbol_direction_document(
            &mut engine,
            json!([
                {"id": "n1", "element": element, "atomicNumber": atomic_number, "position": [100.0, 100.0], "charge": 0, "numHydrogens": 0},
                {"id": "c1", "element": "C", "atomicNumber": 6, "position": [130.0, 100.0], "charge": 0, "numHydrogens": 0}
            ]),
            json!([
                {"id": "b1", "begin": "n1", "end": "c1", "order": 1}
            ]),
        );
        engine.set_tool_state(ToolState {
            active_tool: Tool::Symbol,
            symbol_kind,
            ..ToolState::default()
        });
        click(&mut engine, 100.0, 100.0);
        engine
    }

    for (element, atomic_number, symbol_kind, expected_charge, expected_hydrogens) in [
        ("P", 15, BracketKind::Plus, 1, 1),
        ("P", 15, BracketKind::Minus, -1, 1),
        ("S", 16, BracketKind::Plus, 1, 0),
        ("S", 16, BracketKind::Minus, -1, 0),
    ] {
        let engine = terminal_hetero_engine(element, atomic_number, symbol_kind);
        let node = engine
            .state()
            .document
            .editable_fragment()
            .unwrap()
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == "n1")
            .unwrap();
        assert_eq!(
            node.charge, expected_charge,
            "{element} {symbol_kind:?} charge"
        );
        assert_eq!(
            node.num_hydrogens, expected_hydrogens,
            "{element} {symbol_kind:?} hydrogens"
        );
        assert_eq!(
            node.meta
                .get("chargeSymbolInvalid")
                .and_then(|value| value.as_bool()),
            None,
            "{element} {symbol_kind:?} should remain valid"
        );
    }
}

#[test]
fn symbol_tool_uses_current_document_symbol_line_width() {
    let mut engine = Engine::new();
    engine.set_document_style_preset("acs-document-1996");
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::CirclePlus,
        ..ToolState::default()
    });

    click(&mut engine, 120.0, 140.0);

    let symbol = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("symbol should be created");
    assert_eq!(symbol.payload.bbox, Some([0.0, 0.0, 7.18, 7.18]));
    assert_eq!(
        symbol
            .payload
            .extra
            .get("symbolStyle")
            .and_then(|value| value.as_str()),
        Some("acs")
    );
    assert_eq!(
        symbol
            .payload
            .extra
            .get("symbolLineWidth")
            .and_then(|value| value.as_f64()),
        Some(0.6)
    );
    assert_eq!(
        symbol
            .payload
            .extra
            .get("symbolAnchorHeight")
            .and_then(|value| value.as_f64()),
        Some(7.5)
    );
}

#[test]
fn acs_circle_charge_symbol_uses_full_size_internal_sign() {
    let mut engine = Engine::new();
    engine.set_document_style_preset("acs-document-1996");
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::CirclePlus,
        ..ToolState::default()
    });

    click(&mut engine, 120.0, 140.0);

    let symbol_id = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("symbol should be created")
        .id
        .clone();
    let sign_bounds = engine
        .render_list()
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                object_id: Some(object_id),
                d,
                ..
            } if object_id == symbol_id => Some(rect_path_coordinate_bounds(&d)),
            _ => None,
        })
        .reduce(|left, right| {
            [
                left[0].min(right[0]),
                left[1].min(right[1]),
                left[2].max(right[2]),
                left[3].max(right[3]),
            ]
        })
        .expect("circle-plus sign should render as a filled path");

    assert_eq!(round_to_2(sign_bounds[2] - sign_bounds[0]), 3.93);
    assert_eq!(round_to_2(sign_bounds[3] - sign_bounds[1]), 3.93);
}

#[test]
fn bracket_tool_imports_chemdraw_charge_symbol_kinds() {
    let Some(cdxml) = read_optional_cdxml_fixture("kuohao-acs.cdxml") else {
        return;
    };
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(&cdxml)
        .expect("fixture should load");
    let kinds: Vec<_> = engine
        .state()
        .document
        .objects
        .iter()
        .filter(|object| object.object_type == "symbol")
        .filter_map(|object| {
            object
                .payload
                .extra
                .get("kind")
                .and_then(|value| value.as_str())
        })
        .collect();
    for kind in [
        "circle-plus",
        "plus",
        "radical-cation",
        "lone-pair",
        "circle-minus",
        "minus",
        "radical-anion",
        "electron",
    ] {
        assert!(kinds.contains(&kind), "missing imported symbol kind {kind}");
    }
    let plus = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| {
            object.object_type == "symbol"
                && object
                    .payload
                    .extra
                    .get("kind")
                    .and_then(|value| value.as_str())
                    == Some("plus")
        })
        .expect("ACS plus symbol should import");
    assert_eq!(plus.payload.bbox, Some([0.0, 0.0, 10.49, 10.49]));
    assert_eq!(
        plus.payload
            .extra
            .get("symbolStyle")
            .and_then(|value| value.as_str()),
        Some("acs")
    );
    let radical = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| {
            object.object_type == "symbol"
                && object
                    .payload
                    .extra
                    .get("kind")
                    .and_then(|value| value.as_str())
                    == Some("radical-cation")
        })
        .expect("ACS radical cation should import");
    assert_eq!(radical.payload.bbox, Some([0.0, 0.0, 8.8, 5.87]));
}

#[test]
fn bracket_symbol_drag_from_label_glyph_orbits_around_clicked_glyph() {
    let mut engine = Engine::new();
    let document = json!({
        "format": {"name": "chemsema", "version": "0.1", "unit": "pt"},
        "document": {
            "id": "doc_symbol_orbit",
            "title": "symbol orbit",
            "page": {"width": 200.0, "height": 120.0, "background": "#ffffff"}
        },
        "styles": {},
        "objects": [
            {
                "id": "obj_mol_1",
                "type": "molecule",
                "visible": true,
                "locked": false,
                "zIndex": 10,
                "transform": {"translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "payload": {"resourceRef": "mol_1", "bbox": [0.0, 0.0, 80.0, 40.0]}
            }
        ],
        "resources": {
            "mol_1": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 80.0, 40.0],
                    "nodes": [
                        {
                            "id": "n1",
                            "element": "N",
                            "atomicNumber": 7,
                            "position": [30.0, 30.0],
                            "charge": 0,
                            "numHydrogens": 0,
                            "label": {
                                "text": "N",
                                "sourceText": "N",
                                "position": [30.0, 30.0],
                                "box": [26.0, 22.0, 34.0, 32.0],
                                "glyphPolygons": [[[26.0, 22.0], [34.0, 22.0], [34.0, 32.0], [26.0, 32.0]]]
                            }
                        }
                    ],
                    "bonds": []
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("symbol orbit fixture should load");
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::Plus,
        ..ToolState::default()
    });

    let (hit_point, glyph_anchor) = {
        let label = engine
            .state()
            .document
            .editable_fragment()
            .unwrap()
            .fragment
            .nodes[0]
            .label
            .as_ref()
            .unwrap();
        (
            label_glyph_hit_point(label, 0),
            label_glyph_anchor(label, 0),
        )
    };

    drag(
        &mut engine,
        hit_point,
        Point::new(glyph_anchor.x, glyph_anchor.y - 13.0),
    );

    let symbol = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("dragging from label glyph should create symbol");
    assert_eq!(
        symbol
            .payload
            .extra
            .get("kind")
            .and_then(|value| value.as_str()),
        Some("plus")
    );
    let [_, _, width, height] = symbol.payload.bbox.expect("symbol bbox");
    let symbol_center = Point::new(
        symbol.transform.translate[0] + width * 0.5,
        symbol.transform.translate[1] + height * 0.5,
    );
    assert!(
        (symbol_center.x - glyph_anchor.x).abs() < 0.01,
        "{symbol_center:?}"
    );
    assert!(
        (symbol_center.y - (glyph_anchor.y - 8.0)).abs() < 0.01,
        "{symbol_center:?}"
    );
}

#[test]
fn double_click_component_selection_includes_enclosing_bracket() {
    let mut engine = Engine::new();
    click(&mut engine, 300.0, 300.0);
    let (min_x, min_y, max_x, max_y, hit_point) = {
        let entry = engine.state().document.editable_fragment().unwrap();
        let points: Vec<Point> = entry
            .fragment
            .nodes
            .iter()
            .map(|node| entry.world_point_for_node(node))
            .collect();
        let min_x = points
            .iter()
            .map(|point| point.x)
            .fold(f64::INFINITY, f64::min);
        let min_y = points
            .iter()
            .map(|point| point.y)
            .fold(f64::INFINITY, f64::min);
        let max_x = points
            .iter()
            .map(|point| point.x)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_y = points
            .iter()
            .map(|point| point.y)
            .fold(f64::NEG_INFINITY, f64::max);
        (min_x, min_y, max_x, max_y, points[0])
    };
    engine.set_tool_state(ToolState {
        active_tool: Tool::Bracket,
        bracket_kind: BracketKind::Round,
        ..ToolState::default()
    });
    drag(
        &mut engine,
        Point::new(min_x - 10.0, min_y - 10.0),
        Point::new(max_x + 10.0, max_y + 10.0),
    );
    let bracket_id = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "group")
        .map(|object| object.id.clone())
        .unwrap();

    assert!(engine.select_component_at_point(hit_point, false));
    assert!(engine.state().selection.arrow_objects.contains(&bracket_id));
    assert_eq!(engine.state().selection.nodes.len(), 2);
    assert_eq!(engine.state().selection.bonds.len(), 1);
}

#[test]
fn bracketed_multiple_group_stores_repeating_unit_expansion_when_legal() {
    let mut engine = Engine::new();
    let document = json!({
        "format": {"name": "chemsema", "version": "0.1", "unit": "pt"},
        "document": {
            "id": "doc_repeat",
            "title": "repeat",
            "page": {"width": 200.0, "height": 120.0, "background": "#ffffff"}
        },
        "styles": {},
        "objects": [
            {
                "id": "obj_mol_1",
                "type": "molecule",
                "visible": true,
                "locked": false,
                "zIndex": 10,
                "transform": {"translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "payload": {"resourceRef": "mol_1", "bbox": [0.0, 0.0, 80.0, 20.0]}
            },
            {
                "id": "obj_bracket_1",
                "type": "bracket",
                "visible": true,
                "locked": false,
                "zIndex": 20,
                "transform": {"translate": [15.0, -10.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "meta": {"linkedTextObjectId": "obj_text_1"},
                "payload": {"bbox": [0.0, 0.0, 30.0, 20.0], "kind": "square"}
            },
            {
                "id": "obj_text_1",
                "type": "text",
                "visible": true,
                "locked": false,
                "zIndex": 30,
                "transform": {"translate": [45.0, 5.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "meta": {"linkKind": "bracket-label", "linkedBracketObjectId": "obj_bracket_1"},
                "payload": {"bbox": [0.0, 0.0, 8.0, 10.0], "text": "3"}
            },
            {
                "id": "obj_symbol_1",
                "type": "symbol",
                "visible": true,
                "locked": false,
                "zIndex": 40,
                "transform": {"translate": [17.83325, -2.16675], "rotate": 0.0, "scale": [1.0, 1.0]},
                "payload": {"bbox": [0.0, 0.0, 4.3335, 4.3335], "kind": "plus", "fill": "#000000"}
            }
        ],
        "resources": {
            "mol_1": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, -10.0, 70.0, 20.0],
                    "nodes": [
                        {"id": "n1", "element": "C", "atomicNumber": 6, "position": [0.0, 0.0], "charge": 0, "numHydrogens": 0},
                        {"id": "n2", "element": "C", "atomicNumber": 6, "position": [20.0, 0.0], "charge": 0, "numHydrogens": 0},
                        {"id": "n3", "element": "C", "atomicNumber": 6, "position": [40.0, 0.0], "charge": 0, "numHydrogens": 0},
                        {"id": "n4", "element": "C", "atomicNumber": 6, "position": [60.0, 0.0], "charge": 0, "numHydrogens": 0}
                    ],
                    "bonds": [
                        {"id": "b1", "begin": "n1", "end": "n2", "order": 1},
                        {"id": "b2", "begin": "n2", "end": "n3", "order": 1},
                        {"id": "b3", "begin": "n3", "end": "n4", "order": 1}
                    ]
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("repeat fixture should load");

    assert!(engine.select_all());
    let summary: serde_json::Value =
        serde_json::from_str(&engine.selection_chemistry_summary_json()).unwrap();
    assert_eq!(summary["formula"], "C8H15");
    assert_eq!(summary["atomCount"], 23);
    assert!((summary["formulaWeight"].as_f64().unwrap() - 111.208).abs() < 1.0e-9);
    assert!((summary["exactMass"].as_f64().unwrap() - 111.117_375_483_45).abs() < 1.0e-9);

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    let units = fragment
        .meta
        .get("repeatingUnits")
        .and_then(|value| value.as_array())
        .expect("legal bracketed group should store repeatingUnits");
    assert_eq!(units.len(), 1);
    let unit = &units[0];
    assert_eq!(unit["bracketObjectId"], "obj_bracket_1");
    assert_eq!(unit["countTextObjectId"], "obj_text_1");
    assert_eq!(unit["repeatCount"]["value"], 3);
    assert_eq!(unit["atomIds"], json!(["n2", "n3"]));
    assert_eq!(unit["internalBondIds"], json!(["b2"]));

    let expansion = unit
        .get("expansion")
        .expect("legal bracketed group should include expansion");
    assert_eq!(expansion["schema"], "chemsema.repeatingUnitExpansion.v1");
    assert_eq!(expansion["complete"], true);
    assert_eq!(expansion["count"], 3);
    assert_eq!(expansion["atoms"].as_array().unwrap().len(), 6);
    assert_eq!(expansion["bonds"].as_array().unwrap().len(), 5);
    assert_eq!(expansion["attachments"][0]["atomId"], "n2_r1");
    assert_eq!(expansion["attachments"][1]["atomId"], "n3_r3");
    let first_atom = expansion["atoms"]
        .as_array()
        .unwrap()
        .iter()
        .find(|atom| atom["id"] == "n2_r1")
        .expect("expanded charged atom should exist");
    assert_eq!(first_atom["charge"], 1);
    assert_eq!(first_atom["numHydrogens"], 1);
    assert_eq!(first_atom["electronSymbols"][0]["kind"], "plus");
    assert_eq!(
        first_atom["electronSymbols"][0]["sourceSymbolObjectId"],
        "obj_symbol_1"
    );
}

#[test]
fn bracket_label_count_links_with_bracket_and_selects_with_component() {
    let mut engine = Engine::new();
    engine
        .load_document_json(&repeat_unit_chain_document().to_string())
        .expect("repeat fixture should load");

    assert!(engine.apply_bracket_label_text("obj_bracket_1", bracket_label_session("3")));
    assert!(
        engine.state().selection.is_empty(),
        "committing a bracket label should not select it until the Select tool consumes the pending target"
    );
    assert!(engine
        .state()
        .document
        .objects
        .iter()
        .all(|object| object.object_type != "group"));
    let count_text = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("bracket label commit should create text");
    let text_id = count_text.id.clone();
    let bracket = engine
        .state()
        .document
        .find_scene_object("obj_bracket_1")
        .expect("bracket should remain a top-level object");
    let relation = engine
        .state()
        .document
        .links
        .iter()
        .find(|relation| relation.kind == "bracket-repeat-label")
        .expect("bracket label should create a typed relation");
    assert!(relation
        .endpoints
        .iter()
        .any(|endpoint| endpoint.role == "bracket" && endpoint.entity_id == bracket.id));
    assert!(relation
        .endpoints
        .iter()
        .any(|endpoint| endpoint.role == "label" && endpoint.entity_id == text_id));

    assert!(engine.select_component_at_point(Point::new(20.0, 0.0), false));
    assert!(engine
        .state()
        .selection
        .arrow_objects
        .contains(&"obj_bracket_1".to_string()));
    assert!(engine.state().selection.text_objects.contains(&text_id));
    assert_eq!(engine.state().selection.nodes.len(), 4);
    assert_eq!(engine.state().selection.bonds.len(), 3);

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    let units = fragment
        .meta
        .get("repeatingUnits")
        .and_then(|value| value.as_array())
        .expect("numeric bracket label should create repeatingUnits");
    assert_eq!(units.len(), 1);
    assert_eq!(units[0]["bracketObjectId"], "obj_bracket_1");
    assert!(units[0]["countTextObjectId"]
        .as_str()
        .is_some_and(|object_id| object_id.starts_with("obj_text")));
}

#[test]
fn bracket_label_count_links_with_new_bracket_group() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Bracket,
        bracket_kind: BracketKind::Round,
        ..ToolState::default()
    });

    drag(
        &mut engine,
        Point::new(120.0, 130.0),
        Point::new(180.0, 220.0),
    );

    let bracket_id = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| {
            object.object_type == "group"
                && object.meta.get("kind").and_then(|value| value.as_str()) == Some("bracket-group")
        })
        .map(|object| object.id.clone())
        .expect("dragging bracket tool should create a bracket group");

    assert!(engine.apply_bracket_label_text(&bracket_id, bracket_label_session("3")));

    let text = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("bracket group label commit should create text");
    let text_id = text.id.clone();
    let bracket = engine
        .state()
        .document
        .find_scene_object(&bracket_id)
        .expect("bracket group should remain in the document");
    assert!(engine.state().document.links.iter().any(|relation| {
        relation.kind == "bracket-repeat-label"
            && relation
                .endpoints
                .iter()
                .any(|endpoint| endpoint.entity_id == bracket.id)
            && relation
                .endpoints
                .iter()
                .any(|endpoint| endpoint.entity_id == text_id)
    }));
}

#[test]
fn unlinking_repeat_unit_link_detaches_count_label() {
    let mut engine = Engine::new();
    engine
        .load_document_json(&repeat_unit_chain_document().to_string())
        .expect("repeat fixture should load");
    assert!(engine.apply_bracket_label_text("obj_bracket_1", bracket_label_session("3")));
    assert!(engine.select_component_at_point(Point::new(20.0, 0.0), false));
    assert!(engine.selection_can_unlink_bracket_text());

    assert!(engine.unlink_selection());
    assert!(!engine.selection_can_unlink_bracket_text());
    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    assert!(fragment.meta.get("repeatingUnits").is_none());
    let count_text = engine
        .state()
        .document
        .scene_objects()
        .into_iter()
        .find(|object| object.object_type == "text")
        .expect("unlink should keep the count text object");
    assert_eq!(
        count_text.link_policy,
        chemsema_engine::LinkPolicy::Unlinked
    );
    let bracket = engine
        .state()
        .document
        .scene_objects()
        .into_iter()
        .find(|object| object.id == "obj_bracket_1")
        .expect("unlink should keep the bracket object");
    assert!(bracket.meta.get("repeatUnitId").is_none());
    assert!(engine.state().document.links.is_empty());
}

#[test]
fn editing_linked_repeat_count_refreshes_repeat_unit_semantics() {
    let mut engine = Engine::new();
    engine
        .load_document_json(&repeat_unit_chain_document().to_string())
        .expect("repeat fixture should load");
    assert!(engine.apply_bracket_label_text("obj_bracket_1", bracket_label_session("3")));
    let text_id = engine
        .state()
        .document
        .scene_objects()
        .into_iter()
        .find(|object| object.object_type == "text")
        .expect("linked count text should exist")
        .id
        .clone();

    let session = engine
        .begin_text_edit(Point::new(46.0, 6.0))
        .expect("linked count text should stay independently editable");
    match &session.target {
        TextEditTarget::TextObject { object_id, .. } => {
            assert_eq!(object_id.as_deref(), Some(text_id.as_str()));
        }
        _ => panic!("linked repeat count should edit as a text object"),
    }
    assert!(engine.apply_text_edit(TextEditSession {
        text: "4".to_string(),
        ..session
    }));

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .expect("fragment should still be editable")
        .fragment;
    let units = fragment
        .meta
        .get("repeatingUnits")
        .and_then(|value| value.as_array())
        .expect("linked count edit should refresh repeatingUnits");
    assert_eq!(units.len(), 1);
    assert_eq!(units[0]["bracketObjectId"], "obj_bracket_1");
    assert_eq!(units[0]["countTextObjectId"], text_id);
    assert_eq!(units[0]["repeatCount"]["value"], 4);
}

#[test]
fn bracketed_group_without_numeric_count_does_not_store_expansion() {
    let mut engine = Engine::new();
    let document = json!({
        "format": {"name": "chemsema", "version": "0.1", "unit": "pt"},
        "document": {
            "id": "doc_repeat_invalid",
            "title": "repeat invalid",
            "page": {"width": 200.0, "height": 120.0, "background": "#ffffff"}
        },
        "styles": {},
        "objects": [
            {
                "id": "obj_mol_1",
                "type": "molecule",
                "visible": true,
                "locked": false,
                "zIndex": 10,
                "transform": {"translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "payload": {"resourceRef": "mol_1", "bbox": [0.0, 0.0, 80.0, 20.0]}
            },
            {
                "id": "obj_bracket_1",
                "type": "bracket",
                "visible": true,
                "locked": false,
                "zIndex": 20,
                "transform": {"translate": [15.0, -10.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "payload": {"bbox": [0.0, 0.0, 30.0, 20.0], "kind": "square"}
            },
            {
                "id": "obj_text_1",
                "type": "text",
                "visible": true,
                "locked": false,
                "zIndex": 30,
                "transform": {"translate": [45.0, 5.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "payload": {"bbox": [0.0, 0.0, 8.0, 10.0], "text": "n"}
            }
        ],
        "resources": {
            "mol_1": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, -10.0, 70.0, 20.0],
                    "nodes": [
                        {"id": "n1", "element": "C", "atomicNumber": 6, "position": [0.0, 0.0], "charge": 0, "numHydrogens": 0},
                        {"id": "n2", "element": "C", "atomicNumber": 6, "position": [20.0, 0.0], "charge": 0, "numHydrogens": 0},
                        {"id": "n3", "element": "C", "atomicNumber": 6, "position": [40.0, 0.0], "charge": 0, "numHydrogens": 0},
                        {"id": "n4", "element": "C", "atomicNumber": 6, "position": [60.0, 0.0], "charge": 0, "numHydrogens": 0}
                    ],
                    "bonds": [
                        {"id": "b1", "begin": "n1", "end": "n2", "order": 1},
                        {"id": "b2", "begin": "n2", "end": "n3", "order": 1},
                        {"id": "b3", "begin": "n3", "end": "n4", "order": 1}
                    ]
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("invalid repeat fixture should load");

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    assert!(fragment.meta.get("repeatingUnits").is_none());
}

#[test]
fn bond_tool_icons_are_rendered_with_kernel_bond_styles() {
    let dashed = Engine::bond_tool_icon_svg(BondVariant::Dashed, 1.32, 3.68);
    assert!(
        dashed.contains(r#"class="chemsema-icon cc-bond-icon""#),
        "{dashed}"
    );
    assert!(dashed.contains(r#"viewBox="0 0 24 24""#), "{dashed}");
    assert!(dashed.contains(r#"fill="currentColor""#), "{dashed}");
    assert_eq!(dashed.matches(r##"fill="#ffffff""##).count(), 0, "{dashed}");
    assert_eq!(dashed.matches("<polygon ").count(), 3, "{dashed}");
    assert_eq!(dashed.matches("<line ").count(), 0, "{dashed}");

    let single = Engine::bond_tool_icon_svg(BondVariant::Single, 1.32, 3.68);
    assert!(single.contains(r#"fill="currentColor""#), "{single}");

    let bold = Engine::bond_tool_icon_svg(BondVariant::Bold, 1.32, 3.68);
    assert!(bold.contains(r#"fill="currentColor""#), "{bold}");

    let wavy = Engine::bond_tool_icon_svg(BondVariant::Wavy, 1.32, 3.68);
    assert_eq!(wavy.matches(" C ").count(), 14, "{wavy}");
    assert!(wavy.contains(r#"C 17.7322,7.5198"#), "{wavy}");
    assert!(wavy.contains(r#"stroke-width="1.32""#), "{wavy}");
}

#[test]
fn text_format_icons_are_rendered_with_kernel_text_runs() {
    let tool = Engine::text_format_icon_svg("tool");
    assert!(
        tool.contains(r#"class="chemsema-icon cc-tool-icon cc-text-tool-icon""#),
        "{tool}"
    );
    assert!(tool.contains(r#"font-family="Times New Roman""#), "{tool}");
    assert!(tool.contains(">A</tspan>"), "{tool}");

    let bold = Engine::text_format_icon_svg("bold");
    assert!(
        bold.contains(r#"class="chemsema-icon cc-text-format-icon""#),
        "{bold}"
    );
    assert!(bold.contains("<text "), "{bold}");
    assert!(bold.contains("<tspan"), "{bold}");
    assert!(bold.contains(r#"font-family="Times New Roman""#), "{bold}");
    assert!(bold.contains(r#"font-size="320""#), "{bold}");
    assert!(
        bold.contains(r#"transform="matrix(0.05 0 0 0.05 12 17)""#),
        "{bold}"
    );
    assert!(bold.contains(r#"font-weight="700""#), "{bold}");
    assert!(bold.contains(">B</tspan>"), "{bold}");

    let italic = Engine::text_format_icon_svg("italic");
    assert!(italic.contains(r#"font-style="italic""#), "{italic}");
    assert!(italic.contains(">I</tspan>"), "{italic}");

    let underline = Engine::text_format_icon_svg("underline");
    assert!(
        underline.contains(r#"text-decoration="underline""#),
        "{underline}"
    );
    assert!(underline.contains(">U</tspan>"), "{underline}");

    let outline = Engine::text_format_icon_svg("outline");
    assert!(outline.contains(r#"fill="none""#), "{outline}");
    assert!(outline.contains(r#"paint-order="stroke""#), "{outline}");

    let shadow = Engine::text_format_icon_svg("shadow");
    assert!(shadow.contains("drop-shadow"), "{shadow}");

    let chemical = Engine::text_format_icon_svg("chemical");
    assert!(
        chemical.contains(r#"class="chemsema-icon cc-text-format-icon cc-script-icon""#),
        "{chemical}"
    );
    assert!(chemical.contains(r#"font-size="280""#), "{chemical}");
    assert!(chemical.contains(">CH</tspan>"), "{chemical}");
    assert!(chemical.contains(">2</tspan>"), "{chemical}");
    assert!(chemical.contains("baseline-shift"), "{chemical}");

    let subscript = Engine::text_format_icon_svg("subscript");
    assert!(subscript.contains(">X</tspan>"), "{subscript}");
    assert!(subscript.contains(">2</tspan>"), "{subscript}");
    assert!(subscript.contains(r#"font-size="240""#), "{subscript}");
    assert!(subscript.contains("baseline-shift"), "{subscript}");

    let superscript = Engine::text_format_icon_svg("superscript");
    assert!(superscript.contains(">X</tspan>"), "{superscript}");
    assert!(superscript.contains(">2</tspan>"), "{superscript}");
    assert!(superscript.contains(r#"font-size="240""#), "{superscript}");
    assert!(superscript.contains("baseline-shift"), "{superscript}");
}

#[test]
fn shape_tool_icons_are_rendered_in_double_size_viewbox() {
    let styled_kinds = [
        ShapeKind::Circle,
        ShapeKind::Ellipse,
        ShapeKind::RoundRect,
        ShapeKind::Rect,
    ];
    let styles = [
        ShapeStyle::Solid,
        ShapeStyle::Dashed,
        ShapeStyle::Shaded,
        ShapeStyle::Filled,
        ShapeStyle::Shadowed,
    ];
    for kind in styled_kinds {
        for style in styles {
            let svg = Engine::shape_tool_icon_svg(kind, style);
            assert!(
                svg.contains(r#"viewBox="0 0 48 48""#),
                "{kind:?} {style:?}: {svg}"
            );
        }
    }

    let round_rect = Engine::shape_tool_icon_svg(ShapeKind::RoundRect, ShapeStyle::Solid);
    assert!(round_rect.contains("M 8.4,31.68"), "{round_rect}");
    assert!(round_rect.contains("39.6,16.08"), "{round_rect}");
    assert!(round_rect.contains(r#"stroke-width="2""#), "{round_rect}");
}
