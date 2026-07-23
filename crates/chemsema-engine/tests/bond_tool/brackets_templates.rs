use super::*;

#[test]
fn dragging_one_bracket_side_in_group_does_not_move_other_side() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_bracket_group_side_drag",
            "title": "bracket group side drag",
            "page": { "width": 240.0, "height": 120.0, "background": "#ffffff" }
        },
        "objects": [
            {
                "id": "obj_bracket_group",
                "type": "group",
                "name": "bracket-group",
                "visible": true,
                "locked": false,
                "zIndex": 9,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "meta": { "kind": "bracket-group" },
                "payload": { "bbox": [40.0, 20.0, 118.0, 70.0] },
                "children": [
                    {
                        "id": "obj_left_bracket",
                        "type": "bracket",
                        "visible": true,
                        "zIndex": 10,
                        "transform": { "translate": [40.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                        "payload": {
                            "bbox": [0.0, 0.0, 18.0, 70.0],
                            "kind": "square",
                            "side": "left",
                            "stroke": "#000000",
                            "strokeWidth": 1.0
                        }
                    },
                    {
                        "id": "obj_right_bracket",
                        "type": "bracket",
                        "visible": true,
                        "zIndex": 11,
                        "transform": { "translate": [140.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                        "payload": {
                            "bbox": [0.0, 0.0, 18.0, 70.0],
                            "kind": "square",
                            "side": "right",
                            "stroke": "#000000",
                            "strokeWidth": 1.0
                        }
                    }
                ]
            }
        ],
        "resources": {}
    });
    engine
        .load_document_json(&document.to_string())
        .expect("bracket side document should load");
    engine.set_tool_state(select_tool());

    let start = Point::new(40.5, 55.0);
    let end = Point::new(52.5, 61.0);
    engine.select_at_point(start, false);
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec!["obj_left_bracket".to_string()]
    );
    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert!(engine.update_selection_move(end, false));
    assert!(engine.finish_selection_move(end, false));

    let left = engine
        .state()
        .document
        .find_scene_object("obj_left_bracket")
        .expect("left bracket should remain");
    let right = engine
        .state()
        .document
        .find_scene_object("obj_right_bracket")
        .expect("right bracket should remain");
    assert_eq!(left.transform.translate, [52.0, 26.0]);
    assert_eq!(right.transform.translate, [140.0, 20.0]);
}

#[test]
fn dragging_one_side_of_selected_bracket_pair_moves_both_sides() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Bracket,
        bracket_kind: BracketKind::Square,
        ..ToolState::default()
    });
    drag(
        &mut engine,
        Point::new(120.0, 130.0),
        Point::new(180.0, 220.0),
    );
    engine.set_tool_state(select_tool());

    let group = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "group")
        .expect("bracket tool should create a group");
    let left_id = group
        .children
        .iter()
        .find(|object| {
            object
                .payload
                .extra
                .get("side")
                .and_then(|value| value.as_str())
                == Some("left")
        })
        .map(|object| object.id.clone())
        .expect("left bracket should exist");
    let right_id = group
        .children
        .iter()
        .find(|object| {
            object
                .payload
                .extra
                .get("side")
                .and_then(|value| value.as_str())
                == Some("right")
        })
        .map(|object| object.id.clone())
        .expect("right bracket should exist");
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec![left_id.clone(), right_id.clone()]
    );
    let left_before = engine
        .state()
        .document
        .find_scene_object(&left_id)
        .expect("left bracket should remain")
        .clone();
    let right_before = engine
        .state()
        .document
        .find_scene_object(&right_id)
        .expect("right bracket should remain")
        .clone();
    let left_height = left_before.payload.bbox.expect("left bracket bbox")[3];
    let start = Point::new(
        left_before.transform.translate[0] + 0.5,
        left_before.transform.translate[1] + left_height * 0.5,
    );
    let end = Point::new(start.x + 12.0, start.y + 6.0);

    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec![left_id.clone(), right_id.clone()]
    );
    assert!(engine.update_selection_move(end, false));
    assert!(engine.finish_selection_move(end, false));

    let left_after = engine
        .state()
        .document
        .find_scene_object(&left_id)
        .expect("left bracket should remain");
    let right_after = engine
        .state()
        .document
        .find_scene_object(&right_id)
        .expect("right bracket should remain");
    assert_eq!(
        left_after.transform.translate,
        [
            round_to_2(left_before.transform.translate[0] + 12.0),
            round_to_2(left_before.transform.translate[1] + 6.0)
        ]
    );
    assert_eq!(
        right_after.transform.translate,
        [
            round_to_2(right_before.transform.translate[0] + 12.0),
            round_to_2(right_before.transform.translate[1] + 6.0)
        ]
    );
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec![left_id, right_id]
    );
}

#[test]
fn curved_arrow_path_uses_circular_arc_control_points() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        arrow_variant: ArrowVariant::Curved,
        arrow_curve: ArrowCurve::Arc270,
        ..ToolState::default()
    });

    drag(&mut engine, Point::new(10.0, 20.0), Point::new(110.0, 20.0));

    let path_d = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Path {
                role: RenderRole::DocumentGraphic,
                d,
                ..
            } => Some(d),
            _ => None,
        })
        .expect("curved arrow should render as a path");
    let numbers: Vec<f64> = path_d
        .split(|ch: char| ch.is_ascii_whitespace() || ch == ',' || ch == 'M' || ch == 'C')
        .filter_map(|part| part.parse::<f64>().ok())
        .collect();
    let start = Point::new(numbers[0], numbers[1]);
    let first_control = Point::new(numbers[2], numbers[3]);
    assert!(
        start.distance(first_control) < 50.0,
        "arc control point should stay near the circular tangent, got path {path_d}"
    );
}

#[test]
fn half_arrow_heads_keep_visual_left_and_right_sides_on_curves() {
    fn rendered_half_head(
        variant: ArrowVariant,
        style: ArrowEndpointStyle,
    ) -> (Vec<Point>, Vec<Point>) {
        let mut engine = Engine::new();
        engine.set_tool_state(ToolState {
            active_tool: Tool::Arrow,
            arrow_variant: variant,
            arrow_curve: ArrowCurve::Arc120,
            arrow_head_style: style,
            ..ToolState::default()
        });
        drag(&mut engine, Point::new(10.0, 20.0), Point::new(90.0, 20.0));

        let mut arc = Vec::new();
        let mut head = Vec::new();
        for primitive in engine.render_list() {
            match primitive {
                RenderPrimitive::Path { points, .. } | RenderPrimitive::Polyline { points, .. } => {
                    arc = points
                }
                RenderPrimitive::Line { from, to, .. } => arc = vec![from, to],
                RenderPrimitive::FilledPath { points, .. } if points.len() >= 4 => head = points,
                _ => {}
            }
        }
        (arc, head)
    }

    let (straight_arc, straight_left) =
        rendered_half_head(ArrowVariant::Solid, ArrowEndpointStyle::Left);
    assert_eq!(straight_arc.len(), 2);
    assert_point_close(
        straight_arc[1],
        Point::new(90.0 - (8.75 - 2.5 * 2.0 / 3.0), 20.0),
    );
    assert_point_close(straight_left[0], Point::new(90.0, 20.5));
    assert_point_close(straight_left[1], Point::new(80.0, 17.5));
    assert_point_close(straight_left[3], Point::new(81.25, 20.5));
    assert!(straight_left[1].y < straight_left[2].y);
    assert!(straight_left[2].y < straight_left[3].y);
    let (straight_right_shaft, straight_right) =
        rendered_half_head(ArrowVariant::Solid, ArrowEndpointStyle::Right);
    assert_eq!(straight_right_shaft.len(), 2);
    assert_point_close(
        straight_right_shaft[1],
        Point::new(90.0 - (8.75 - 2.5 * 2.0 / 3.0), 20.0),
    );
    assert_point_close(straight_right[0], Point::new(90.0, 19.5));
    assert_point_close(straight_right[1], Point::new(80.0, 22.5));
    assert!(straight_right[1].y > straight_right[2].y);
    assert!(straight_right[2].y > straight_right[3].y);
    assert_point_close(straight_right[3], Point::new(81.25, 19.5));

    let (curved_arc, curved_left) =
        rendered_half_head(ArrowVariant::Curved, ArrowEndpointStyle::Left);
    assert!(curved_arc[curved_arc.len() / 2].y < curved_arc[0].y);
    assert!((*curved_arc.last().unwrap()).distance(Point::new(90.0, 20.0)) > 1.0);
    assert!(curved_left[1].distance(curved_left[0]) > curved_left[3].distance(curved_left[0]));
    let (_, curved_right) = rendered_half_head(ArrowVariant::Curved, ArrowEndpointStyle::Right);
    assert!(curved_right[1].distance(curved_right[0]) > curved_right[3].distance(curved_right[0]));

    let (mirror_arc, mirror_left) =
        rendered_half_head(ArrowVariant::CurvedMirror, ArrowEndpointStyle::Left);
    assert!(mirror_arc[mirror_arc.len() / 2].y > mirror_arc[0].y);
    assert!((*mirror_arc.last().unwrap()).distance(Point::new(90.0, 20.0)) > 1.0);
    assert!(mirror_left[1].distance(mirror_left[0]) > mirror_left[3].distance(mirror_left[0]));
    let (_, mirror_right) =
        rendered_half_head(ArrowVariant::CurvedMirror, ArrowEndpointStyle::Right);
    assert!(mirror_right[1].distance(mirror_right[0]) > mirror_right[3].distance(mirror_right[0]));
}

#[test]
fn component_selection_from_label_selects_whole_fragment() {
    let mut engine = Engine::new();
    load_label_document(
        &mut engine,
        "Ph",
        vec![json!([
            [px(314.0), px(256.0)],
            [px(324.0), px(256.0)],
            [px(324.0), px(264.0)],
            [px(314.0), px(264.0)]
        ])],
        json!([{ "id": "b1", "begin": "n0", "end": "n1", "order": 1 }]),
    );

    assert!(engine.select_component_at_point(px_point(318.0, 260.0), false));

    let selection = &engine.state().selection;
    assert_eq!(selection.nodes.len(), 2);
    assert!(selection.nodes.contains(&"n0".to_string()));
    assert!(selection.nodes.contains(&"n1".to_string()));
    assert_eq!(selection.bonds, vec!["b1".to_string()]);
    assert_eq!(selection.label_nodes, vec!["n1".to_string()]);
}

#[test]
fn clipboard_document_json_contains_selected_molecule_fragment() {
    let mut engine = Engine::new();
    load_label_document(
        &mut engine,
        "Ph",
        vec![json!([
            [px(314.0), px(256.0)],
            [px(324.0), px(256.0)],
            [px(324.0), px(264.0)],
            [px(314.0), px(264.0)]
        ])],
        json!([{ "id": "b1", "begin": "n0", "end": "n1", "order": 1 }]),
    );
    assert!(engine.select_component_at_point(px_point(318.0, 260.0), false));

    let document_json = engine
        .clipboard_document_json()
        .expect("clipboard document should serialize")
        .expect("selection should produce a clipboard document");
    let document = parse_document_json(&document_json).expect("clipboard document should parse");
    let entry = document
        .editable_fragment()
        .expect("clipboard document should keep the selected molecule");

    assert_eq!(document.objects.len(), 1);
    assert_eq!(entry.fragment.nodes.len(), 2);
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(
        entry
            .fragment
            .nodes
            .iter()
            .filter(|node| node.label.is_some())
            .count(),
        1
    );
}

#[test]
fn select_all_clipboard_document_json_keeps_all_molecule_objects() {
    let mut engine = Engine::new();
    load_two_molecule_document_with_duplicate_local_ids(&mut engine);

    assert!(engine.select_all());
    let document_json = engine
        .clipboard_document_json()
        .expect("clipboard document should serialize")
        .expect("select all should produce a clipboard document");
    let document = parse_document_json(&document_json).expect("clipboard document should parse");

    assert_eq!(
        document.editable_fragments().len(),
        2,
        "select-all Office payload must not collapse multiple molecule objects into the first fragment"
    );
    assert!(
        document.find_scene_object("obj_molecule_a").is_some()
            && document.find_scene_object("obj_molecule_b").is_some()
    );
}

#[test]
fn external_document_paste_keeps_every_molecule_object() {
    let mut source = Engine::new();
    load_two_molecule_document_with_duplicate_local_ids(&mut source);
    let source_json = source
        .document_json()
        .expect("source document should serialize");

    let mut target = Engine::new();
    let baseline_molecules = target.state().document.editable_fragments().len();
    assert!(target
        .paste_document_json(&source_json)
        .expect("external document should paste"));
    assert_eq!(
        target.state().document.editable_fragments().len(),
        baseline_molecules + 2,
        "cross-tab paste must preserve every molecule object instead of flattening into one fragment"
    );
    assert_eq!(target.state().selection.molecule_objects.len(), 2);
}

#[test]
fn portable_fragment_keeps_every_fully_selected_molecule() {
    let mut source = Engine::new();
    load_two_molecule_document_with_duplicate_local_ids(&mut source);
    assert!(source.select_all());
    let fragment_json = source
        .clipboard_selection_json()
        .expect("portable fragment should serialize")
        .expect("select all should produce a portable fragment");

    let mut target = Engine::new();
    let baseline_molecules = target.state().document.editable_fragments().len();
    assert!(target
        .paste_clipboard_json(&fragment_json)
        .expect("portable fragment should paste"));
    assert_eq!(
        target.state().document.editable_fragments().len(),
        baseline_molecules + 2
    );
    assert_eq!(target.state().selection.molecule_objects.len(), 2);
}

#[test]
fn cdxml_clipboard_pastes_as_editable_structure() {
    let cdxml = r#"<CDXML BoundingBox="0 0 100 60"><page><fragment>
        <n id="1" p="20 30"/><n id="2" p="50 30"/>
        <b id="3" B="1" E="2" Order="1"/>
    </fragment></page></CDXML>"#;
    let mut target = Engine::new();
    assert!(target
        .paste_cdxml(cdxml)
        .expect("ChemDraw CDXML clipboard should paste"));
    let fragments = target.state().document.editable_fragments();
    assert_eq!(
        fragments
            .iter()
            .map(|entry| entry.fragment.nodes.len())
            .sum::<usize>(),
        2
    );
    assert_eq!(
        fragments
            .iter()
            .map(|entry| entry.fragment.bonds.len())
            .sum::<usize>(),
        1
    );
}

#[test]
fn single_molecule_clipboard_document_json_does_not_expand_duplicate_local_ids() {
    let mut engine = Engine::new();
    load_two_molecule_document_with_duplicate_local_ids(&mut engine);

    assert!(engine.select_component_at_point(Point::new(95.0, 100.0), false));
    let document_json = engine
        .clipboard_document_json()
        .expect("clipboard document should serialize")
        .expect("single molecule selection should produce a clipboard document");
    let document = parse_document_json(&document_json).expect("clipboard document should parse");

    assert_eq!(
        document.editable_fragments().len(),
        1,
        "object-level select-all marker must be required before copying every molecule"
    );
    assert!(document.find_scene_object("obj_molecule_a").is_some());
    assert!(document.find_scene_object("obj_molecule_b").is_none());
}

#[test]
fn click_on_blank_canvas_creates_up_right_single_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 2);
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(
        entry.fragment.nodes[0].position,
        [FIRST_START_X, FIRST_START_Y]
    );
    assert_eq!(entry.fragment.nodes[1].position, [FIRST_END_X, FIRST_END_Y]);
    assert_eq!(entry.fragment.bonds[0].stroke_width, DEFAULT_BOND_STROKE);
    assert_eq!(entry.fragment.bonds[0].bond_spacing, Some(12.0));
    assert_eq!(entry.fragment.bonds[0].margin_width, Some(2.0));
}

#[test]
fn acs_document_1996_preset_sets_new_bond_metrics() {
    let mut engine = Engine::new();
    engine.set_document_style_preset("acs-document-1996");
    engine.set_tool_state(bond_tool());

    click(&mut engine, px(300.0), px(260.0));

    let entry = engine.state().document.editable_fragment().unwrap();
    let begin = entry.world_point_for_node(&entry.fragment.nodes[0]);
    let end = entry.world_point_for_node(&entry.fragment.nodes[1]);
    let bond = &entry.fragment.bonds[0];
    assert!((begin.distance(end) - 14.4).abs() < 0.001);
    assert!((bond.stroke_width - 0.6).abs() < 0.001);
    assert_eq!(bond.bold_width, Some(2.0));
    assert_eq!(bond.wedge_width, Some(3.0));
    assert_eq!(bond.label_clip_margin, None);
    assert_eq!(bond.hash_spacing, Some(2.5));
    assert_eq!(bond.bond_spacing, Some(18.0));
    assert_eq!(bond.margin_width, Some(1.6));
}

#[test]
fn acs_document_1996_preset_reflows_existing_endpoint_label_geometry() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    hover(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);
    assert!(engine.replace_hovered_endpoint_label("N"));

    let before_clip = {
        let entry = engine.state().document.editable_fragment().unwrap();
        let label = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.element == "N")
            .and_then(|node| node.label.as_ref())
            .expect("N label before preset");
        label_clip_bounds(label)
    };

    engine.set_document_style_preset("acs-document-1996");

    let entry = engine.state().document.editable_fragment().unwrap();
    let labeled = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.element == "N")
        .expect("endpoint label should remain an N node");
    let label = labeled.label.as_ref().expect("N node should have a label");
    let bounds = label_glyph_bounds(label);
    let glyph_width = bounds[2] - bounds[0];
    let box_width = label.bbox().map(|bbox| bbox[2] - bbox[0]).unwrap_or(0.0);
    let after_clip = label_clip_bounds(label);

    assert_eq!(
        label.font_size,
        Some(chemsema_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT)
    );
    assert!(
        glyph_width > 8.0,
        "ACS style switch should reflow glyph geometry for the 10pt label, not keep a scaled 0.48x box: {bounds:?}"
    );
    assert!(
        box_width > 7.0,
        "label bbox should also be reflowed at the current font size: {:?}",
        label.bbox()
    );
    assert!(
        (after_clip[2] - after_clip[0]) < (before_clip[2] - before_clip[0]),
        "changing MarginWidth must synchronously rebuild the retreat region: {before_clip:?} -> {after_clip:?}"
    );
}

#[test]
fn acs_document_1996_preset_keeps_existing_bonds_hoverable() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    let bond_id = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .bonds
        .first()
        .unwrap()
        .id
        .clone();

    engine.set_document_style_preset("acs-document-1996");
    let center = bond_world_center_point(&engine, &bond_id);
    let (endpoint_probe, near_endpoint_probe) = {
        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id)
            .expect("original bond should exist");
        let begin = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)
            .map(|node| entry.world_point_for_node(node))
            .expect("begin node should exist");
        let end = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.end)
            .map(|node| entry.world_point_for_node(node))
            .expect("end node should exist");
        let length = begin.distance(end);
        let normal =
            chemsema_engine::Vector::new(-(end.y - begin.y) / length, (end.x - begin.x) / length);
        let toward_center =
            chemsema_engine::Vector::new((begin.x - end.x) / length, (begin.y - end.y) / length);
        (
            end.translated(normal.scaled(4.5)),
            end.translated(toward_center.scaled(2.5)),
        )
    };
    engine.set_tool_state(bond_tool());
    hover(&mut engine, endpoint_probe.x, endpoint_probe.y);
    assert!(
        engine.state().overlay.hover_endpoint.is_some(),
        "ACS endpoint hit target should remain comfortable near the endpoint"
    );
    hover(&mut engine, near_endpoint_probe.x, near_endpoint_probe.y);
    assert!(
        engine.state().overlay.hover_endpoint.is_some(),
        "ACS endpoint-side bond body should still belong to the endpoint, matching the default feel"
    );
    hover(&mut engine, center.x, center.y);
    assert!(
        engine.state().overlay.hover_bond_center.is_some(),
        "bond tool should still hover an existing bond after switching to ACS"
    );
    click(&mut engine, center.x, center.y);
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = entry
        .fragment
        .bonds
        .iter()
        .find(|bond| bond.id == bond_id)
        .expect("original bond should still exist");
    assert_eq!(
        bond.order, 2,
        "clicking an ACS bond center should cycle the bond, not start an endpoint drag"
    );

    engine.set_tool_state(templates_tool("ring-6"));
    hover(&mut engine, center.x, center.y);
    assert!(
        engine.state().overlay.hover_bond_center.is_some(),
        "template tool should still hover an existing bond after switching to ACS"
    );
}

#[test]
fn acs_template_click_on_bond_uses_bond_as_ring_side() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    let bond_id = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .bonds
        .first()
        .unwrap()
        .id
        .clone();

    engine.set_document_style_preset("acs-document-1996");
    let center = bond_world_center_point(&engine, &bond_id);
    engine.set_tool_state(templates_tool("ring-6"));
    click(&mut engine, center.x, center.y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 6);
    assert_eq!(entry.fragment.bonds.len(), 6);
    assert!(entry.fragment.bonds.iter().any(|bond| bond.id == bond_id));
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn acs_document_1996_preset_sets_bold_render_width() {
    let mut engine = Engine::new();
    engine.set_document_style_preset("acs-document-1996");
    engine.set_tool_state(bold_bond_tool());

    click(&mut engine, px(300.0), px(260.0));

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.line_weights.main, BondLineWeight::Bold);
    assert_eq!(bond.bold_width, Some(2.0));
    let bold_area = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentBond,
                points,
                ..
            } => Some(polygon_area(&points)),
            _ => None,
        })
        .expect("bold bond should render as a filled polygon");
    assert!((bold_area - 28.8).abs() < 0.01, "{bold_area}");
}

#[test]
fn acs_document_1996_preset_sets_new_graphic_strokes() {
    let mut engine = Engine::new();
    engine.set_document_style_preset("acs-document-1996");
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        ..ToolState::default()
    });
    drag(&mut engine, Point::new(10.0, 20.0), Point::new(90.0, 20.0));

    let arrow = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow object should exist");
    let arrow_style = arrow.style_ref.as_ref().expect("arrow should have style");
    assert_eq!(arrow_style, "style_arrow_0_60");
    assert_eq!(
        engine.state().document.styles[arrow_style]
            .get("strokeWidth")
            .and_then(|value| value.as_f64()),
        Some(0.6)
    );

    engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Solid));
    drag(&mut engine, Point::new(20.0, 30.0), Point::new(60.0, 80.0));

    let shape = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "shape")
        .expect("shape object should exist");
    let shape_style = shape.style_ref.as_ref().expect("shape should have style");
    assert_eq!(
        engine.state().document.styles[shape_style]
            .get("strokeWidth")
            .and_then(|value| value.as_f64()),
        Some(0.6)
    );
}

#[test]
fn acs_document_1996_preset_sets_template_ring_bond_lengths() {
    let mut blank = Engine::new();
    blank.set_document_style_preset("acs-document-1996");
    blank.set_tool_state(templates_tool("ring-6"));
    click(&mut blank, px(300.0), px(260.0));
    assert!(ring_bond_lengths(&blank)
        .iter()
        .all(|length| (length - 14.4).abs() < 0.001));

    let mut endpoint = Engine::new();
    endpoint.set_document_style_preset("acs-document-1996");
    endpoint.set_tool_state(bond_tool());
    click(&mut endpoint, px(300.0), px(260.0));
    let anchor = node_world_point(&endpoint, "n_2");
    endpoint.set_tool_state(templates_tool("ring-6"));
    click(&mut endpoint, anchor.x, anchor.y);
    assert!(ring_bond_lengths(&endpoint)
        .iter()
        .all(|length| (length - 14.4).abs() < 0.001));

    let mut fused = Engine::new();
    fused.set_document_style_preset("acs-document-1996");
    fused.set_tool_state(bond_tool());
    click(&mut fused, px(300.0), px(260.0));
    let center = {
        let entry = fused.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        let begin = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)
            .map(|node| entry.world_point_for_node(node))
            .unwrap();
        let end = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.end)
            .map(|node| entry.world_point_for_node(node))
            .unwrap();
        Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5)
    };
    fused.set_tool_state(templates_tool("ring-6"));
    click(&mut fused, center.x, center.y);
    assert!(ring_bond_lengths(&fused)
        .iter()
        .all(|length| (length - 14.4).abs() < 0.001));
}

#[test]
fn acs_document_1996_preset_scales_existing_document_as_one_group() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        ..ToolState::default()
    });
    drag(
        &mut engine,
        Point::new(600.0, 100.0),
        Point::new(660.0, 100.0),
    );

    engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Solid));
    drag(
        &mut engine,
        Point::new(700.0, 200.0),
        Point::new(760.0, 260.0),
    );

    let before_page = engine.state().document.document.page.clone();
    let entry = engine.state().document.editable_fragment().unwrap();
    let before_bond_start = entry.world_point_for_node(&entry.fragment.nodes[0]);
    let before_arrow_start = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .and_then(|object| line_object_points(object).first().copied())
        .expect("arrow start should exist");
    let before_shape_translate = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "shape")
        .expect("shape should exist")
        .transform
        .translate;

    engine.set_document_style_preset("acs-document-1996");

    assert_eq!(
        engine.state().document.document.page.width,
        before_page.width
    );
    assert_eq!(
        engine.state().document.document.page.height,
        before_page.height
    );
    let entry = engine.state().document.editable_fragment().unwrap();
    let after_bond_start = entry.world_point_for_node(&entry.fragment.nodes[0]);
    let after_bond_end = entry.world_point_for_node(&entry.fragment.nodes[1]);
    let after_arrow_start = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .and_then(|object| line_object_points(object).first().copied())
        .expect("arrow start should exist");
    let after_shape_translate = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "shape")
        .expect("shape should exist")
        .transform
        .translate;

    let scale = 14.4 / DEFAULT_BOND_LENGTH;
    assert!((after_bond_start.distance(after_bond_end) - 14.4).abs() < 0.001);
    assert!(
        ((after_arrow_start.x - after_bond_start.x)
            - (before_arrow_start.x - before_bond_start.x) * scale)
            .abs()
            < 0.001
    );
    assert!(
        ((after_arrow_start.y - after_bond_start.y)
            - (before_arrow_start.y - before_bond_start.y) * scale)
            .abs()
            < 0.001
    );
    assert!(
        ((after_shape_translate[0] - after_bond_start.x)
            - (before_shape_translate[0] - before_bond_start.x) * scale)
            .abs()
            < 0.001
    );
    assert!(
        ((after_shape_translate[1] - after_bond_start.y)
            - (before_shape_translate[1] - before_bond_start.y) * scale)
            .abs()
            < 0.001
    );

    let after_once = after_arrow_start;
    engine.set_document_style_preset("acs-document-1996");
    let after_twice = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .and_then(|object| line_object_points(object).first().copied())
        .expect("arrow start should exist");
    assert_point_close(after_once, after_twice);

    let bond = &engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .bonds[0];
    assert!((bond.stroke_width - 0.6).abs() < 0.001);
    assert_eq!(bond.bold_width, Some(2.0));
    assert_eq!(bond.wedge_width, Some(3.0));
    assert_eq!(bond.label_clip_margin, None);
    assert_eq!(bond.hash_spacing, Some(2.5));
    assert_eq!(bond.margin_width, Some(1.6));

    engine.set_document_style_preset("default");
    let entry = engine.state().document.editable_fragment().unwrap();
    let default_bond = &entry.fragment.bonds[0];
    let default_begin = entry.world_point_for_node(
        entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == default_bond.begin)
            .unwrap(),
    );
    let default_end = entry.world_point_for_node(
        entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == default_bond.end)
            .unwrap(),
    );
    assert_eq!(engine.document_style_preset(), "default");
    assert!((default_begin.distance(default_end) - DEFAULT_BOND_LENGTH).abs() < 0.001);
    assert!((default_bond.stroke_width - DEFAULT_BOND_STROKE).abs() < 0.001);
    assert_eq!(default_bond.wedge_width, Some(6.0));
    assert_eq!(default_bond.label_clip_margin, None);
    assert_eq!(default_bond.margin_width, Some(2.0));
}

#[test]
fn object_settings_update_bond_and_graphic_metrics() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_object_settings",
            "title": "object settings",
            "page": { "width": 160.0, "height": 120.0, "background": "#ffffff" }
        },
        "styles": {
            "style_line": { "kind": "stroke", "stroke": "#111111", "strokeWidth": 1.0 },
            "style_shape": { "kind": "shape", "stroke": "#111111", "strokeWidth": 1.0, "fill": null }
        },
        "objects": [
            {
                "id": "obj_mol",
                "type": "molecule",
                "styleRef": "style_molecule_default",
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "resourceRef": "mol" }
            },
            {
                "id": "obj_line",
                "type": "line",
                "styleRef": "style_line",
                "payload": {
                    "points": [[50.0, 20.0], [90.0, 20.0]],
                    "kind": "line"
                }
            },
            {
                "id": "obj_shape",
                "type": "shape",
                "styleRef": "style_shape",
                "transform": { "translate": [50.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "bbox": [0.0, 0.0, 24.0, 12.0], "kind": "rect" }
            },
            {
                "id": "obj_bracket",
                "type": "bracket",
                "transform": { "translate": [90.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "bbox": [0.0, 0.0, 14.0, 28.0], "kind": "round", "stroke": "#111111", "strokeWidth": 1.0 }
            }
        ],
        "resources": {
            "mol": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 40.0, 20.0],
                    "nodes": [
                        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [10.0, 10.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n2", "element": "C", "atomicNumber": 6, "position": [40.0, 10.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 1.0 }
                    ]
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("object settings fixture should load");

    engine.select_at_point(Point::new(25.0, 10.0), false);
    let dialog: serde_json::Value =
        serde_json::from_str(&engine.object_settings_dialog_json()).unwrap();
    let field_keys = dialog["fields"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|field| field["key"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(field_keys, vec!["bondLength", "lineWidth", "marginWidth"]);

    let original_options = engine.options().clone();
    let changed = engine
        .apply_object_settings_dialog_json(
            r#"{
                "unit": "pt",
                "values": {
                    "bondLength": 15.0,
                    "lineWidth": 0.7,
                    "marginWidth": 1.8
                }
            }"#,
        )
        .expect("object settings should parse");
    assert!(changed);
    assert!((engine.options().bond_length - original_options.bond_length).abs() < 0.001);
    assert!(
        (engine.options().bond_stroke_width - original_options.bond_stroke_width).abs() < 0.001
    );
    assert!(
        (engine.options().graphic_stroke_width - original_options.graphic_stroke_width).abs()
            < 0.001
    );

    let fragment = engine.state().document.editable_fragment().unwrap();
    let bond = &fragment.fragment.bonds[0];
    let begin = fragment
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.begin)
        .unwrap();
    let end = fragment
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.end)
        .unwrap();
    assert!((begin.point().distance(end.point()) - 15.0).abs() < 0.001);
    assert!((bond.stroke_width - 0.7).abs() < 0.001);
    assert_eq!(bond.margin_width, Some(1.8));
    assert_eq!(bond.bold_width, None);
    assert_eq!(bond.bond_spacing, None);
    assert_eq!(bond.hash_spacing, None);

    engine.select_at_point(Point::new(70.0, 20.0), false);
    let dialog: serde_json::Value =
        serde_json::from_str(&engine.object_settings_dialog_json()).unwrap();
    let field_keys = dialog["fields"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|field| field["key"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(field_keys, vec!["lineWidth"]);
    assert!(engine
        .apply_object_settings_dialog_json(
            r#"{
                "unit": "pt",
                "values": {
                    "lineWidth": 0.7
                }
            }"#,
        )
        .expect("graphic settings should parse"));

    let line = engine
        .state()
        .document
        .find_scene_object("obj_line")
        .unwrap();
    let line_style = line.style_ref.as_deref().unwrap();
    assert_eq!(
        engine.state().document.styles[line_style]["strokeWidth"],
        json!(0.7)
    );
    assert_eq!(
        engine.state().document.styles["style_line"]["strokeWidth"],
        json!(1.0)
    );
    assert_eq!(
        engine.state().document.styles["style_shape"]["strokeWidth"],
        json!(1.0)
    );
    let bracket = engine
        .state()
        .document
        .find_scene_object("obj_bracket")
        .unwrap();
    assert_eq!(bracket.payload.extra["strokeWidth"], json!(1.0));
    let defaults = &engine.state().document.document.meta["import"]["cdxml"]["defaults"];
    assert!(defaults.is_null());
}

#[test]
fn object_settings_multi_selection_uses_union_and_blanks_mixed_values() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(double_bond_tool());
    click(&mut engine, px(420.0), px(260.0));

    let bond_centers = {
        let entry = engine.state().document.editable_fragment().unwrap();
        entry
            .fragment
            .bonds
            .iter()
            .map(|bond| {
                let begin = entry
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| node.id == bond.begin)
                    .unwrap()
                    .point();
                let end = entry
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| node.id == bond.end)
                    .unwrap()
                    .point();
                (
                    bond.order,
                    Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5),
                )
            })
            .collect::<Vec<_>>()
    };
    assert!(bond_centers.iter().any(|(order, _)| *order >= 2));
    assert!(engine.select_component_at_point(bond_centers[0].1, false));
    assert!(engine
        .apply_object_settings_dialog_json(
            r#"{
                "unit": "pt",
                "values": {
                    "bondLength": 12.0
                }
            }"#,
        )
        .expect("single selected setting should parse"));
    let bond_centers = {
        let entry = engine.state().document.editable_fragment().unwrap();
        entry
            .fragment
            .bonds
            .iter()
            .map(|bond| {
                let begin = entry
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| node.id == bond.begin)
                    .unwrap()
                    .point();
                let end = entry
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| node.id == bond.end)
                    .unwrap()
                    .point();
                (
                    bond.order,
                    Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5),
                )
            })
            .collect::<Vec<_>>()
    };
    assert!(engine.select_component_at_point(bond_centers[0].1, false));
    assert!(engine.select_component_at_point(bond_centers[1].1, true));

    let dialog: serde_json::Value =
        serde_json::from_str(&engine.object_settings_dialog_json()).unwrap();
    let fields = dialog["fields"].as_array().unwrap();
    let field_keys = fields
        .iter()
        .filter_map(|field| field["key"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        field_keys,
        vec!["bondLength", "lineWidth", "bondSpacing", "marginWidth"]
    );
    let bond_length = fields
        .iter()
        .find(|field| field["key"] == "bondLength")
        .unwrap();
    assert_eq!(bond_length["mixed"], json!(true));
    assert!(bond_length["value"].is_null());

    assert!(engine
        .apply_object_settings_dialog_json(
            r#"{
                "unit": "pt",
                "values": {
                    "bondLength": 12.0,
                    "bondSpacing": 14.0
                }
            }"#,
        )
        .expect("mixed settings should parse"));

    let entry = engine.state().document.editable_fragment().unwrap();
    let selected = engine
        .state()
        .selection
        .bonds
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    for bond in &entry.fragment.bonds {
        if !selected.contains(bond.id.as_str()) {
            continue;
        }
        let begin = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)
            .unwrap();
        let end = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.end)
            .unwrap();
        assert!((begin.point().distance(end.point()) - 12.0).abs() < 0.02);
        if bond.order >= 2 {
            assert_eq!(bond.bond_spacing, Some(14.0));
        } else {
            assert_ne!(bond.bond_spacing, Some(14.0));
        }
    }
}

#[test]
fn engine_provides_context_menu_and_numeric_dialog_schemas() {
    let mut engine = Engine::new();
    let canvas_menu: serde_json::Value =
        serde_json::from_str(&engine.context_menu_json(r#"{"kind":"canvas"}"#, false)).unwrap();
    assert!(canvas_menu.as_array().unwrap().iter().any(|item| {
        item.get("command").and_then(serde_json::Value::as_str) == Some("smiles-dialog")
            && item.get("label").and_then(serde_json::Value::as_str) == Some("From SMILES...")
    }));
    assert!(canvas_menu.as_array().unwrap().iter().any(|item| {
        item.get("command").and_then(serde_json::Value::as_str) == Some("insert-image")
            && item.get("label").and_then(serde_json::Value::as_str) == Some("Insert Image...")
    }));

    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    let hit = engine.context_hit_test_json(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y));
    let menu: serde_json::Value =
        serde_json::from_str(&engine.context_menu_json(&hit, false)).unwrap();
    let labels = menu
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item.get("label").and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>();
    assert!(labels.contains(&"Bond Type"));
    assert!(labels.contains(&"Bond Query & Reaction"));
    assert!(labels.contains(&"Object Settings..."));
    assert!(!labels.contains(&"Insert Image..."));

    let atom_hit = engine.context_hit_test_json(Point::new(FIRST_START_X, FIRST_START_Y));
    let atom_menu: serde_json::Value =
        serde_json::from_str(&engine.context_menu_json(&atom_hit, false)).unwrap();
    let atom_properties = atom_menu
        .as_array()
        .unwrap()
        .iter()
        .find(|item| {
            item.get("label").and_then(serde_json::Value::as_str) == Some("Atom Properties")
        })
        .expect("atom menu exposes atom properties");
    assert!(atom_properties
        .get("submenu")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|items| items.iter().any(|item| {
            item.get("label").and_then(serde_json::Value::as_str) == Some("Radical")
        })));
    let atom_query = atom_menu
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item.get("label").and_then(serde_json::Value::as_str) == Some("Atom Query"))
        .expect("atom menu exposes atom query editing");
    assert!(atom_query
        .get("submenu")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|items| items.iter().any(|item| {
            item.get("label").and_then(serde_json::Value::as_str) == Some("Element List...")
        })));
    assert!(atom_query
        .get("submenu")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|items| items.iter().any(|item| {
            item.get("label").and_then(serde_json::Value::as_str) == Some("Reaction Stereo")
        })));

    let scale: serde_json::Value =
        serde_json::from_str(&engine.selection_numeric_dialog_json("scale")).unwrap();
    assert_eq!(scale["kind"], "scale");
    assert_eq!(scale["field"]["unit"], "%");
    let isotope: serde_json::Value =
        serde_json::from_str(&engine.atom_property_dialog_json("isotope")).unwrap();
    assert_eq!(isotope["kind"], "atom-property");
    assert_eq!(isotope["property"], "isotope");
    assert_eq!(isotope["field"]["valueKind"], "integer");
    assert_eq!(isotope["field"]["minimum"], 1);
    assert_eq!(isotope["field"]["maximum"], i16::MAX);
    let element_list: serde_json::Value =
        serde_json::from_str(&engine.atom_property_dialog_json("element-list")).unwrap();
    assert_eq!(element_list["kind"], "atom-property");
    assert_eq!(element_list["property"], "element-list");
    assert_eq!(element_list["field"]["valueKind"], "text");
    assert!(engine.select_all());
    let molecule_menu: serde_json::Value =
        serde_json::from_str(&engine.context_menu_json(&hit, false)).unwrap();
    assert!(molecule_menu.as_array().unwrap().iter().any(|item| {
        item.get("label").and_then(serde_json::Value::as_str) == Some("Chemical Analysis")
            && item
                .get("submenu")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|items| {
                    ["SMILES", "InChI", "InChIKey"].iter().all(|label| {
                        items.iter().any(|item| {
                            item.get("label").and_then(serde_json::Value::as_str) == Some(label)
                        })
                    })
                })
    }));
    let selected_canvas_menu: serde_json::Value =
        serde_json::from_str(&engine.context_menu_json(r#"{"kind":"canvas"}"#, false)).unwrap();
    assert!(selected_canvas_menu.as_array().unwrap().iter().any(|item| {
        item.get("label").and_then(serde_json::Value::as_str) == Some("Chemical Analysis")
    }));
    assert!(engine
        .apply_selection_numeric_dialog_json(r#"{"kind":"scale","value":110}"#)
        .unwrap());
}

#[test]
fn template_click_on_bond_uses_bond_as_ring_side() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.set_tool_state(templates_tool("ring-6"));
    click(&mut engine, FIRST_CENTER_X, FIRST_CENTER_Y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 6);
    assert_eq!(entry.fragment.bonds.len(), 6);
    assert!(entry
        .fragment
        .bonds
        .iter()
        .any(|bond| (bond.begin == "n_1" && bond.end == "n_2")
            || (bond.begin == "n_2" && bond.end == "n_1")));
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_click_on_bond_supports_ring_sizes_three_through_eight() {
    for ring_size in 3..=8 {
        let mut engine = Engine::new();
        engine.set_tool_state(bond_tool());
        click(&mut engine, px(300.0), px(260.0));

        engine.set_tool_state(templates_tool(&format!("ring-{ring_size}")));
        click(&mut engine, FIRST_CENTER_X, FIRST_CENTER_Y);

        let entry = engine.state().document.editable_fragment().unwrap();
        assert_eq!(entry.fragment.nodes.len(), ring_size);
        assert_eq!(entry.fragment.bonds.len(), ring_size);
        assert_no_duplicate_node_positions(&engine);
    }
}

#[test]
fn template_ring_bonds_inherit_existing_anchor_stroke_width() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let mut document: serde_json::Value =
        serde_json::from_str(&engine.state_json().expect("state json")).expect("json");
    document["document"]["resources"]["mol_editor"]["data"]["bonds"][0]["strokeWidth"] =
        json!(0.07);
    engine
        .load_document_json(
            &serde_json::to_string(&document["document"]).expect("document json should encode"),
        )
        .expect("document should reload");

    engine.set_tool_state(templates_tool("ring-3"));
    click(&mut engine, FIRST_END_X, FIRST_END_Y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert!(entry
        .fragment
        .bonds
        .iter()
        .all(|bond| (bond.stroke_width - 0.07).abs() < 0.001));
}

#[test]
fn template_endpoint_ring_connects_adjacent_intersections_through_center() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let endpoint = node_world_point(&engine, "n_2");
    engine.set_tool_state(templates_tool("ring-3"));
    click(&mut engine, endpoint.x, endpoint.y);

    let original_bond_points = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentBond,
                bond_id: Some(bond_id),
                points,
                ..
            } if bond_id == "b_3" => Some(points),
            _ => None,
        })
        .expect("original bond should render as polygon");
    let center_index = original_bond_points
        .iter()
        .position(|point| point.distance(endpoint) < 0.001)
        .expect("polygon should include the shared center point");
    let previous = original_bond_points
        [(center_index + original_bond_points.len() - 1) % original_bond_points.len()];
    let next = original_bond_points[(center_index + 1) % original_bond_points.len()];

    const ENDPOINT_RING_JUNCTION_TOLERANCE_PT: f64 = 2.267_716_535_433_071;

    assert!(
        previous.distance(endpoint) < ENDPOINT_RING_JUNCTION_TOLERANCE_PT,
        "{previous:?}"
    );
    assert!(
        next.distance(endpoint) < ENDPOINT_RING_JUNCTION_TOLERANCE_PT,
        "{next:?}"
    );

    assert!(
        engine.render_list().into_iter().all(|primitive| {
            !matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond,
                    bond_id: None,
                    points,
                    ..
                } if points
                    .iter()
                    .any(|point| point.distance(endpoint) < ENDPOINT_RING_JUNCTION_TOLERANCE_PT)
            )
        }),
        "endpoint ring junction should be covered by bond polygons, not an extra center patch"
    );

    let incident_areas: Vec<f64> = engine
        .render_list()
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentBond,
                bond_id: Some(_),
                points,
                ..
            } if points
                .iter()
                .any(|point| point.distance(endpoint) < DEFAULT_BOND_STROKE) =>
            {
                Some(polygon_area(&points))
            }
            _ => None,
        })
        .collect();
    assert!(
        incident_areas.iter().all(|area| *area > 0.01),
        "{incident_areas:?}"
    );
}

#[test]
fn template_click_on_endpoint_attaches_ring_on_symmetry_axis() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    let existing_begin = node_world_point(&engine, "n_1");
    let endpoint = node_world_point(&engine, "n_2");

    engine.set_tool_state(templates_tool("ring-5"));
    click(&mut engine, endpoint.x, endpoint.y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 6);
    assert_eq!(entry.fragment.bonds.len(), 6);
    assert_eq!(
        entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.begin == "n_2" || bond.end == "n_2")
            .count(),
        3
    );
    let ring_points = entry
        .fragment
        .nodes
        .iter()
        .filter(|node| node.id != "n_1")
        .map(|node| entry.world_point_for_node(node))
        .collect::<Vec<_>>();
    let center = Point::new(
        ring_points.iter().map(|point| point.x).sum::<f64>() / ring_points.len() as f64,
        ring_points.iter().map(|point| point.y).sum::<f64>() / ring_points.len() as f64,
    );
    let expected_axis = chemsema_engine::angle_between(existing_begin, endpoint);
    let actual_axis = chemsema_engine::angle_between(endpoint, center);
    assert!(
        chemsema_engine::angular_distance(expected_axis, actual_axis) < 0.2,
        "{expected_axis} {actual_axis}"
    );
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_drag_on_endpoint_snaps_ring_axis_to_15_degrees() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let endpoint = node_world_point(&engine, "n_2");
    let target = endpoint.translated(direction_from_angle(22.0).scaled(DEFAULT_BOND_LENGTH * 2.0));
    engine.set_tool_state(templates_tool("ring-6"));
    engine.pointer_down(PointerEvent {
        x: endpoint.x,
        y: endpoint.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: target.x,
        y: target.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: target.x,
        y: target.y,
        button: Some(0),
        alt_key: false,
    });

    let ring_points = {
        let entry = engine.state().document.editable_fragment().unwrap();
        entry
            .fragment
            .nodes
            .iter()
            .filter(|node| node.id != "n_1")
            .map(|node| entry.world_point_for_node(node))
            .collect::<Vec<_>>()
    };
    let center = Point::new(
        ring_points.iter().map(|point| point.x).sum::<f64>() / ring_points.len() as f64,
        ring_points.iter().map(|point| point.y).sum::<f64>() / ring_points.len() as f64,
    );
    assert!((chemsema_engine::angle_between(endpoint, center) - 15.0).abs() < 0.2);
    assert_eq!(attached_node_points(&engine, "n_2").len(), 3);
    assert!(
        attached_node_points(&engine, "n_2")
            .iter()
            .filter(|point| point.distance(node_world_point(&engine, "n_1")) > 0.03)
            .count()
            == 2
    );
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_drag_on_endpoint_keeps_live_focus_on_connection_anchor() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let endpoint = node_world_point(&engine, "n_2");
    let target = endpoint.translated(direction_from_angle(22.0).scaled(DEFAULT_BOND_LENGTH * 2.0));
    engine.set_tool_state(templates_tool("ring-6"));
    engine.pointer_down(PointerEvent {
        x: endpoint.x,
        y: endpoint.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: target.x,
        y: target.y,
        button: None,
        alt_key: false,
    });

    let hover = engine
        .state()
        .overlay
        .hover_endpoint
        .as_ref()
        .expect("template drag should keep live focus on the connection endpoint");
    assert_eq!(hover.node_id, "n_2");
    assert!((hover.point.x - endpoint.x).abs() < 0.001, "{hover:?}");
    assert!((hover.point.y - endpoint.y).abs() < 0.001, "{hover:?}");
    assert!(hover.point.distance(target) > DEFAULT_BOND_LENGTH);
    let preview = engine
        .state()
        .overlay
        .preview
        .as_ref()
        .expect("template drag should keep the ring preview active");
    assert!((preview.end.x - endpoint.x).abs() < 0.001, "{preview:?}");
    assert!((preview.end.y - endpoint.y).abs() < 0.001, "{preview:?}");
    assert!(preview.end.distance(target) > DEFAULT_BOND_LENGTH);
}

#[test]
fn template_tool_hover_shows_endpoint_snap_target_before_drag() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let endpoint = node_world_point(&engine, "n_2");
    engine.set_tool_state(templates_tool("ring-6"));
    engine.pointer_move(PointerEvent {
        x: endpoint.x,
        y: endpoint.y,
        button: None,
        alt_key: false,
    });

    let hover = engine
        .state()
        .overlay
        .hover_endpoint
        .as_ref()
        .expect("template tool should expose endpoint snap hover before drag");
    assert_eq!(hover.node_id, "n_2");
    assert!(hover.point.distance(endpoint) < 0.001, "{hover:?}");
}
