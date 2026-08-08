use super::*;

#[test]
fn select_tool_move_undo_redo_returns_to_final_drag_position() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    let start = Point::new(FIRST_CENTER_X, FIRST_CENTER_Y);
    let mid = Point::new(FIRST_CENTER_X + px(10.0), FIRST_CENTER_Y);
    let end = Point::new(FIRST_CENTER_X + px(24.0), FIRST_CENTER_Y + px(18.0));

    engine.select_at_point(start, false);
    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert!(engine.update_selection_move(mid, false));
    assert!(engine.update_selection_move(end, false));
    assert!(engine.finish_selection_move(end, false));

    let final_n1 = node_world_point(&engine, "n_1");
    assert!(engine.undo());
    assert_eq!(
        node_world_point(&engine, "n_1"),
        Point::new(FIRST_START_X, FIRST_START_Y)
    );
    assert!(engine.redo());
    assert_eq!(node_world_point(&engine, "n_1"), final_n1);
}

#[test]
fn select_tool_resizing_selected_bond_from_east_scales_selected_nodes() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    let center = Point::new(FIRST_CENTER_X, FIRST_CENTER_Y);

    engine.select_at_point(center, false);
    let (x, y, width, height) = selection_bond_rect(&engine);
    let pivot_x = x;
    let target = Point::new(x + width * 2.0, y + height * 0.5);

    assert!(engine.begin_selection_resize("east", Point::new(x + width, y + height * 0.5)));
    assert!(engine.update_selection_resize(target));
    assert!(engine.finish_selection_resize(target));

    let n1 = node_world_point(&engine, "n_1");
    let n2 = node_world_point(&engine, "n_2");
    assert!((n1.x - round_to_2(pivot_x + (FIRST_START_X - pivot_x) * 2.0)).abs() < 0.001);
    assert!((n1.y - FIRST_START_Y).abs() < 0.001);
    assert!((n2.x - round_to_2(pivot_x + (FIRST_END_X - pivot_x) * 2.0)).abs() < 0.001);
    assert!((n2.y - FIRST_END_Y).abs() < 0.001);

    let final_n2 = n2;
    assert!(engine.undo());
    assert_eq!(
        node_world_point(&engine, "n_2"),
        Point::new(FIRST_END_X, FIRST_END_Y)
    );
    assert!(engine.redo());
    assert_eq!(node_world_point(&engine, "n_2"), final_n2);
}

#[test]
fn select_tool_resizing_one_text_selection_box_scales_all_selected_text_objects() {
    let mut engine = Engine::new();
    load_arrange_text_document(&mut engine);
    select_all_arrange_text_objects(&mut engine);

    assert!(engine.begin_selection_resize("east", Point::new(40.0, 25.0)));
    assert!(engine.update_selection_resize(Point::new(260.0, 25.0)));
    assert!(engine.finish_selection_resize(Point::new(260.0, 25.0)));

    assert_eq!(text_translate(&engine, "obj_text_a"), [0.0, 0.0]);
    assert_eq!(text_translate(&engine, "obj_text_b"), [60.0, 20.0]);
    assert_eq!(text_translate(&engine, "obj_text_c"), [200.0, 40.0]);
    assert_eq!(text_bbox(&engine, "obj_text_a")[2], 20.0);
    assert_eq!(text_bbox(&engine, "obj_text_b")[2], 20.0);
    assert_eq!(text_bbox(&engine, "obj_text_c")[2], 60.0);
}

#[test]
fn select_tool_dragging_inside_combined_selection_box_moves_selection() {
    let mut engine = Engine::new();
    load_arrange_text_document(&mut engine);
    select_all_arrange_text_objects(&mut engine);

    let gap_inside_selection_box = Point::new(65.0, 5.0);
    assert!(engine.selection_contains_point(gap_inside_selection_box));
    assert!(engine.begin_selection_move_at_point(gap_inside_selection_box, false, false));
    assert!(engine.update_selection_move(Point::new(75.0, 15.0), false));
    assert!(engine.finish_selection_move(Point::new(75.0, 15.0), false));

    assert_eq!(text_translate(&engine, "obj_text_a"), [10.0, 10.0]);
    assert_eq!(text_translate(&engine, "obj_text_b"), [40.0, 30.0]);
    assert_eq!(text_translate(&engine, "obj_text_c"), [110.0, 50.0]);
}

#[test]
fn select_tool_corner_resize_is_proportional() {
    let mut engine = Engine::new();
    load_arrange_text_document(&mut engine);
    select_all_arrange_text_objects(&mut engine);

    assert!(engine.begin_selection_resize("ne", Point::new(130.0, 0.0)));
    assert!(engine.update_selection_resize(Point::new(260.0, -50.0)));
    assert!(engine.finish_selection_resize(Point::new(260.0, -50.0)));

    assert_eq!(text_translate(&engine, "obj_text_b"), [60.0, -10.0]);
    assert_eq!(text_translate(&engine, "obj_text_c"), [200.0, 30.0]);
    assert_eq!(text_bbox(&engine, "obj_text_a")[2], 20.0);
    assert_eq!(text_bbox(&engine, "obj_text_a")[3], 20.0);
    assert_eq!(text_bbox(&engine, "obj_text_c")[2], 60.0);
    assert_eq!(text_bbox(&engine, "obj_text_c")[3], 20.0);
}

#[test]
fn select_tool_dragging_unselected_bond_focus_starts_move() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(10000.0, 10000.0), false);
    let start = Point::new(FIRST_CENTER_X, FIRST_CENTER_Y);
    let end = Point::new(FIRST_CENTER_X + px(16.0), FIRST_CENTER_Y);

    assert!(engine.state().selection.is_empty());
    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert_eq!(engine.state().selection.bonds, vec!["b_3"]);
    assert!(engine.render_list().iter().all(|primitive| !matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionBox
                | RenderRole::SelectionBond
                | RenderRole::SelectionNode
                | RenderRole::SelectionTextBox,
            ..
        } | RenderPrimitive::Circle {
            role: RenderRole::SelectionBondDot,
            ..
        }
    )));
    assert!(engine.update_selection_move(end, false));
    assert!(engine.finish_selection_move(end, false));
    assert_eq!(engine.state().selection.bonds, vec!["b_3"]);

    let entry = engine.state().document.editable_fragment().unwrap();
    let n1 = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n_1")
        .unwrap();
    assert!((n1.position[0] - round_to_2(FIRST_START_X + px(16.0))).abs() < 0.001);
}

#[test]
fn select_tool_rotating_selected_bond_snaps_to_15_degrees() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    let center = Point::new(FIRST_CENTER_X, FIRST_CENTER_Y);
    let start = center.translated(direction_from_angle(0.0).scaled(1.0));
    let target = center.translated(direction_from_angle(22.0).scaled(1.0));

    engine.select_at_point(center, false);
    assert!(engine.begin_selection_rotate(start));
    assert!(engine.update_selection_rotate(target, false));
    assert!(engine.render_list().iter().all(|primitive| !matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionBox
                | RenderRole::SelectionBond
                | RenderRole::SelectionNode
                | RenderRole::SelectionTextBox,
            ..
        } | RenderPrimitive::Circle {
            role: RenderRole::SelectionBondDot,
            ..
        }
    )));
    assert!(engine.finish_selection_rotate(target, false));

    let expected_n1 = rotate_point_around(Point::new(FIRST_START_X, FIRST_START_Y), center, 15.0);
    let expected_n2 = rotate_point_around(Point::new(FIRST_END_X, FIRST_END_Y), center, 15.0);
    let n1 = node_world_point(&engine, "n_1");
    let n2 = node_world_point(&engine, "n_2");
    assert!((n1.x - round_to_2(expected_n1.x)).abs() < 0.001, "{n1:?}");
    assert!((n1.y - round_to_2(expected_n1.y)).abs() < 0.001, "{n1:?}");
    assert!((n2.x - round_to_2(expected_n2.x)).abs() < 0.001, "{n2:?}");
    assert!((n2.y - round_to_2(expected_n2.y)).abs() < 0.001, "{n2:?}");
}

#[test]
fn select_tool_alt_rotating_selected_bond_uses_free_angle() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    let center = Point::new(FIRST_CENTER_X, FIRST_CENTER_Y);
    let start = center.translated(direction_from_angle(0.0).scaled(1.0));
    let target = center.translated(direction_from_angle(22.0).scaled(1.0));

    engine.select_at_point(center, false);
    assert!(engine.begin_selection_rotate(start));
    assert!(engine.update_selection_rotate(target, true));
    assert!(engine.finish_selection_rotate(target, true));

    let expected_n2 = rotate_point_around(Point::new(FIRST_END_X, FIRST_END_Y), center, 22.0);
    let n2 = node_world_point(&engine, "n_2");
    assert!((n2.x - round_to_2(expected_n2.x)).abs() < 0.001, "{n2:?}");
    assert!((n2.y - round_to_2(expected_n2.y)).abs() < 0.001, "{n2:?}");
}

#[test]
fn select_tool_dragging_single_terminal_endpoint_snaps_to_15_degrees() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(10000.0, 10000.0), false);
    let start = Point::new(FIRST_END_X, FIRST_END_Y);
    let target = Point::new(FIRST_START_X, FIRST_START_Y)
        .translated(direction_from_angle(22.0).scaled(DEFAULT_BOND_LENGTH * 1.4));

    engine.select_at_point(start, false);
    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert!(engine.state().overlay.hover_endpoint.is_none());
    assert!(engine.update_selection_move(target, false));
    assert!(engine.state().overlay.hover_endpoint.is_none());
    assert!(engine.finish_selection_move(target, false));

    let expected = Point::new(FIRST_START_X, FIRST_START_Y)
        .translated(direction_from_angle(15.0).scaled(DEFAULT_BOND_LENGTH));
    let entry = engine.state().document.editable_fragment().unwrap();
    let n2 = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n_2")
        .unwrap();
    assert!((n2.position[0] - round_to_2(expected.x)).abs() < 0.001);
    assert!((n2.position[1] - round_to_2(expected.y)).abs() < 0.001);
    assert_eq!(engine.state().selection.nodes, vec!["n_2"]);
}

#[test]
fn select_tool_dragging_unselected_single_terminal_endpoint_selects_dragged_endpoint() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(FIRST_START_X, FIRST_START_Y), false);
    let start = Point::new(FIRST_END_X, FIRST_END_Y);
    let target = Point::new(FIRST_START_X, FIRST_START_Y)
        .translated(direction_from_angle(22.0).scaled(DEFAULT_BOND_LENGTH * 1.4));

    assert_eq!(engine.state().selection.nodes, vec!["n_1"]);
    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert_eq!(engine.state().selection.nodes, vec!["n_2"]);
    assert!(engine.update_selection_move(target, false));
    assert!(engine.finish_selection_move(target, false));

    let expected = Point::new(FIRST_START_X, FIRST_START_Y)
        .translated(direction_from_angle(15.0).scaled(DEFAULT_BOND_LENGTH));
    let entry = engine.state().document.editable_fragment().unwrap();
    let n2 = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n_2")
        .unwrap();
    assert!((n2.position[0] - round_to_2(expected.x)).abs() < 0.001);
    assert!((n2.position[1] - round_to_2(expected.y)).abs() < 0.001);
    assert_eq!(engine.state().selection.nodes, vec!["n_2"]);
}

#[test]
fn select_tool_alt_dragging_single_terminal_endpoint_uses_pointer_position() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    let start = Point::new(FIRST_END_X, FIRST_END_Y);
    let target = Point::new(FIRST_END_X + px(17.0), FIRST_END_Y + px(23.0));

    engine.select_at_point(start, false);
    assert!(engine.begin_selection_move_at_point(start, false, true));
    assert!(engine.update_selection_move(target, true));
    assert!(engine.finish_selection_move(target, true));

    let entry = engine.state().document.editable_fragment().unwrap();
    let n2 = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n_2")
        .unwrap();
    assert!((n2.position[0] - round_to_2(target.x)).abs() < 0.001);
    assert!((n2.position[1] - round_to_2(target.y)).abs() < 0.001);
    assert_eq!(engine.state().selection.nodes, vec!["n_2"]);
}

#[test]
fn select_toolbar_align_left_uses_outer_left_edge() {
    let mut engine = Engine::new();
    load_arrange_text_document(&mut engine);
    select_all_arrange_text_objects(&mut engine);

    assert!(engine.apply_selection_arrange_command("align-left"));

    assert_eq!(text_translate(&engine, "obj_text_a")[0], 0.0);
    assert_eq!(text_translate(&engine, "obj_text_b")[0], 0.0);
    assert_eq!(text_translate(&engine, "obj_text_c")[0], 0.0);
}

#[test]
fn select_toolbar_vertical_center_aligns_box_centers_on_y_axis() {
    let mut engine = Engine::new();
    load_arrange_text_document(&mut engine);
    select_all_arrange_text_objects(&mut engine);

    assert!(engine.apply_selection_arrange_command("align-v-center"));

    assert_eq!(text_translate(&engine, "obj_text_a")[1], 20.0);
    assert_eq!(text_translate(&engine, "obj_text_b")[1], 20.0);
    assert_eq!(text_translate(&engine, "obj_text_c")[1], 20.0);
}

#[test]
fn select_toolbar_horizontal_distribution_equalizes_edge_gaps_not_centers() {
    let mut engine = Engine::new();
    load_arrange_text_document(&mut engine);
    select_all_arrange_text_objects(&mut engine);

    assert!(engine.apply_selection_arrange_command("distribute-h"));

    assert_eq!(text_translate(&engine, "obj_text_a")[0], 0.0);
    assert_eq!(text_translate(&engine, "obj_text_b")[0], 50.0);
    assert_eq!(text_translate(&engine, "obj_text_c")[0], 100.0);
}

#[test]
fn select_toolbar_flip_horizontal_keeps_selection_center_fixed() {
    let mut engine = Engine::new();
    load_arrange_text_document(&mut engine);
    select_all_arrange_text_objects(&mut engine);

    assert!(engine.apply_selection_arrange_command("flip-h"));

    assert_eq!(text_translate(&engine, "obj_text_a")[0], 120.0);
    assert_eq!(text_translate(&engine, "obj_text_b")[0], 90.0);
    assert_eq!(text_translate(&engine, "obj_text_c")[0], 0.0);
}

#[test]
fn select_toolbar_flip_horizontal_mirrors_selected_molecule_geometry_in_place() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);

    assert!(engine.apply_selection_arrange_command("flip-h"));

    let entry = engine.state().document.editable_fragment().unwrap();
    let n1 = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n_1")
        .unwrap();
    let n2 = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n_2")
        .unwrap();
    assert!((n1.position[0] - FIRST_END_X).abs() < 0.01);
    assert!((n1.position[1] - FIRST_START_Y).abs() < 0.01);
    assert!((n2.position[0] - FIRST_START_X).abs() < 0.01);
    assert!((n2.position[1] - FIRST_END_Y).abs() < 0.01);
}

#[test]
fn delete_tool_click_degrades_double_before_removing_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(double_bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let bond_id = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .bonds
        .first()
        .expect("bond should exist")
        .id
        .clone();
    let center = bond_center_point(&engine, &bond_id);

    engine.set_tool_state(delete_tool());
    click(&mut engine, center.x, center.y);
    assert_eq!(bond_order(&engine, &bond_id), Some(1));

    click(&mut engine, center.x, center.y);
    assert_eq!(fragment_counts(&engine), (0, 0));
}

#[test]
fn delete_tool_click_uses_focused_bond_center_before_wide_endpoint_hit() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_short_double",
            "title": "short double bond",
            "page": { "width": 220.0, "height": 180.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": DEFAULT_BOND_STROKE,
                "fontFamily": "Arial",
                "fontSize": chemsema_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT
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
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [95.0, 95.0, 20.0, 10.0],
                    "nodes": [{
                        "id": "n1",
                        "element": "C",
                        "atomicNumber": 6,
                        "position": [100.0, 100.0],
                        "charge": 0,
                        "numHydrogens": 0
                    }, {
                        "id": "n2",
                        "element": "C",
                        "atomicNumber": 6,
                        "position": [110.0, 100.0],
                        "charge": 0,
                        "numHydrogens": 0
                    }],
                    "bonds": [{ "id": "b1", "begin": "n1", "end": "n2", "order": 2 }]
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("document should load");

    let center = Point::new(105.0, 100.0);
    engine.set_tool_state(delete_tool());
    hover(&mut engine, center.x, center.y);
    assert!(engine.state().overlay.hover_bond_center.is_some());
    assert!(engine.state().overlay.hover_endpoint.is_none());

    click(&mut engine, center.x, center.y);

    assert_eq!(bond_order(&engine, "b1"), Some(1));
    assert_eq!(fragment_counts(&engine), (2, 1));
}

#[test]
fn delete_tool_click_degrades_triple_to_side_double() {
    let mut engine = Engine::new();
    engine.set_tool_state(triple_bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = entry.fragment.bonds.first().expect("bond should exist");
    let bond_id = bond.id.clone();
    let center = bond_center_point(&engine, &bond_id);
    let _ = entry;

    engine.set_tool_state(delete_tool());
    click(&mut engine, center.x, center.y);

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = entry
        .fragment
        .bonds
        .iter()
        .find(|bond| bond.id == bond_id)
        .expect("bond should remain");
    assert_eq!(bond.order, 2);
    assert!(bond.double.is_some());
    assert_ne!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center)
    );
}

#[test]
fn delete_tool_click_on_endpoint_removes_all_connected_bonds() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    click(&mut engine, FIRST_END_X, FIRST_END_Y);
    assert_eq!(fragment_counts(&engine), (3, 2));

    let entry = engine.state().document.editable_fragment().unwrap();
    let branch_node_id = node_degrees(&engine)
        .into_iter()
        .find_map(|(node_id, degree)| (degree == 2).then_some(node_id))
        .expect("branch node should exist");
    let branch_node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == branch_node_id)
        .expect("branch node should exist");
    let branch_point = (branch_node.position[0], branch_node.position[1]);
    let _ = entry;

    engine.set_tool_state(delete_tool());
    click(&mut engine, branch_point.0, branch_point.1);
    assert_eq!(fragment_counts(&engine), (0, 0));
}

#[test]
fn delete_tool_click_on_label_removes_only_label() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    hover(&mut engine, px(300.0), px(260.0));
    assert!(engine.replace_hovered_endpoint_label("Ph"));

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry.fragment.nodes.first().expect("node should exist");
    let label_box = node
        .label
        .as_ref()
        .and_then(|label| label.bbox())
        .expect("label box");
    let _ = entry;

    engine.set_tool_state(delete_tool());
    click(
        &mut engine,
        (label_box[0] + label_box[2]) * 0.5,
        (label_box[1] + label_box[3]) * 0.5,
    );

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry.fragment.nodes.first().expect("node should remain");
    assert!(node.label.is_none());
    assert_eq!(entry.fragment.bonds.len(), 1);
}

#[test]
fn delete_command_on_hovered_bond_center_removes_entire_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(double_bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    let bond_id = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .bonds
        .first()
        .expect("bond should exist")
        .id
        .clone();
    let center = bond_center_point(&engine, &bond_id);

    engine.set_tool_state(bond_tool());
    hover(&mut engine, center.x, center.y);
    assert!(engine.delete_selection());
    assert_eq!(fragment_counts(&engine), (0, 0));
}

#[test]
fn bond_tool_focuses_bond_center_and_cycles_double_styles() {
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
    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: None,
        alt_key: false,
    });

    let center = engine.state().overlay.hover_bond_center.as_ref().unwrap();
    assert!((center.point.x - FIRST_CENTER_X).abs() < 0.001);
    assert!((center.point.y - FIRST_CENTER_Y).abs() < 0.001);
    assert_eq!(center.order, 1);
    let center_rect = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon { role, points, .. }
                if role == RenderRole::HoverBondCenter && points.len() == 4 =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("single-bond center focus should render as a 4-point rectangle");
    assert_eq!(center_rect.len(), 4);
    let focus_length = (0..4)
        .map(|index| center_rect[index].distance(center_rect[(index + 1) % 4]))
        .fold(0.0, f64::max);
    assert!(
        (focus_length - DEFAULT_BOND_LENGTH * 0.5).abs() < 0.001,
        "bond center focus length should be half the bond length: {focus_length}"
    );

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 2);
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left),
    );
    assert!(engine.can_undo());

    engine.pointer_move(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: None,
        alt_key: false,
    });
    let double_center = engine.state().overlay.hover_bond_center.as_ref().unwrap();
    assert_eq!(double_center.order, 2);
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Polygon { points, .. } if points.len() == 4
    )));

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center),
    );

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Right),
    );
}

#[test]
fn double_tool_defaults_to_center_on_three_connected_node() {
    let mut engine = Engine::new();
    engine.add_single_bond(
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemsema_engine::Point::new(FIRST_END_X, FIRST_END_Y),
    );
    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            object_id: None,
            point: chemsema_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: chemsema_engine::Point::new(
                FIRST_END_SINGLE_EXTEND_X,
                FIRST_END_SINGLE_EXTEND_Y,
            ),
            label_anchor: None,
        },
    );
    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            object_id: None,
            point: chemsema_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(331.18, 278.0),
            label_anchor: None,
        },
    );

    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = entry
        .fragment
        .bonds
        .iter()
        .find(|bond| bond.begin == "n_1" && bond.end == "n_2")
        .unwrap();
    assert_eq!(bond.order, 2);
    assert!(matches!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
    ));
    assert_ne!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center)
    );
}

#[test]
fn double_tool_does_not_default_to_center_when_each_endpoint_has_one_same_side_substituent() {
    let mut engine = Engine::new();
    engine.add_single_bond(
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemsema_engine::Point::new(FIRST_END_X, FIRST_END_Y),
    );
    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_1".to_string()),
            object_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(268.82, 242.0),
            label_anchor: None,
        },
    );
    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            object_id: None,
            point: chemsema_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(300.0, 242.0),
            label_anchor: None,
        },
    );

    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = entry
        .fragment
        .bonds
        .iter()
        .find(|bond| bond.begin == "n_1" && bond.end == "n_2")
        .unwrap();
    assert_eq!(bond.order, 2);
    assert!(matches!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
    ));
    assert_ne!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center)
    );
}

#[test]
fn double_tool_defaults_to_side_when_substituents_span_both_sides() {
    let mut engine = Engine::new();
    engine.add_single_bond(
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemsema_engine::Point::new(FIRST_END_X, FIRST_END_Y),
    );
    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_1".to_string()),
            object_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(268.82, 242.0),
            label_anchor: None,
        },
    );
    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            object_id: None,
            point: chemsema_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: chemsema_engine::Point::new(
                FIRST_END_SINGLE_EXTEND_X,
                FIRST_END_SINGLE_EXTEND_Y,
            ),
            label_anchor: None,
        },
    );

    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = entry
        .fragment
        .bonds
        .iter()
        .find(|bond| bond.begin == "n_1" && bond.end == "n_2")
        .unwrap();
    assert_eq!(bond.order, 2);
    assert!(matches!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
    ));
    assert_ne!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center)
    );
}

#[test]
fn collinear_attachment_does_not_trigger_centered_double_default() {
    let mut engine = Engine::new();
    engine.add_single_bond(
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemsema_engine::Point::new(FIRST_END_X, FIRST_END_Y),
    );
    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_1".to_string()),
            object_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(268.82, 242.0),
            label_anchor: None,
        },
    );
    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            object_id: None,
            point: chemsema_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: chemsema_engine::Point::new(
                FIRST_END_TRIPLE_EXTEND_X,
                FIRST_END_TRIPLE_EXTEND_Y,
            ),
            label_anchor: None,
        },
    );

    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    {
        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        assert!(matches!(
            bond.double.as_ref().map(|double| double.placement),
            Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
        ));
        assert_ne!(
            bond.double.as_ref().map(|double| double.placement),
            Some(DoubleBondPlacement::Center)
        );
        assert_eq!(
            bond.double.as_ref().map(|double| double.frozen),
            Some(false)
        );
    }
}

#[test]
fn adding_fourth_bond_to_unfrozen_center_double_moves_to_last_drawn_side_on_tie() {
    let mut engine = Engine::new();
    engine.add_single_bond(
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemsema_engine::Point::new(FIRST_END_X, FIRST_END_Y),
    );
    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_1".to_string()),
            object_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(268.82, 242.0),
            label_anchor: None,
        },
    );
    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            object_id: None,
            point: chemsema_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: chemsema_engine::Point::new(
                FIRST_END_SINGLE_EXTEND_X,
                FIRST_END_SINGLE_EXTEND_Y,
            ),
            label_anchor: None,
        },
    );

    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_1".to_string()),
            object_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: chemsema_engine::Point::new(ROOT_SINGLE_BRANCH_X, ROOT_SINGLE_BRANCH_Y),
            label_anchor: None,
        },
    );
    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            object_id: None,
            point: chemsema_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(331.18, 278.0),
            label_anchor: None,
        },
    );

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left)
    );
    assert_eq!(
        bond.double.as_ref().map(|double| double.frozen),
        Some(false)
    );
}

#[test]
fn exact_four_substituent_tie_uses_last_downward_attachment_side() {
    let mut engine = Engine::new();
    let anchor = |node_id: Option<&str>, x: f64, y: f64| chemsema_engine::BondAnchor {
        node_id: node_id.map(str::to_string),
        object_id: None,
        point: chemsema_engine::Point::new(x, y),
        label_anchor: None,
    };

    assert!(engine.add_single_bond_between(anchor(None, 100.0, 100.0), anchor(None, 148.0, 100.0),));
    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 124.0,
        y: 100.0,
        button: Some(0),
        alt_key: false,
    });
    assert!(engine
        .add_single_bond_between(anchor(Some("n_1"), 100.0, 100.0), anchor(None, 100.0, 52.0),));
    assert!(engine.add_single_bond_between(
        anchor(Some("n_1"), 100.0, 100.0),
        anchor(None, 100.0, 148.0),
    ));
    assert!(engine
        .add_single_bond_between(anchor(Some("n_2"), 148.0, 100.0), anchor(None, 148.0, 52.0),));

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(
        entry.fragment.bonds[0]
            .double
            .as_ref()
            .map(|double| double.placement),
        Some(DoubleBondPlacement::Right)
    );

    assert!(engine.add_single_bond_between(
        anchor(Some("n_2"), 148.0, 100.0),
        anchor(None, 148.0, 148.0),
    ));

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(
        entry.fragment.bonds[0]
            .double
            .as_ref()
            .map(|double| double.placement),
        Some(DoubleBondPlacement::Left)
    );
    assert_eq!(
        entry.fragment.bonds[0]
            .double
            .as_ref()
            .map(|double| double.frozen),
        Some(false)
    );
}

#[test]
fn exact_four_substituent_tie_uses_last_upward_attachment_side() {
    let mut engine = Engine::new();
    let anchor = |node_id: Option<&str>, x: f64, y: f64| chemsema_engine::BondAnchor {
        node_id: node_id.map(str::to_string),
        object_id: None,
        point: chemsema_engine::Point::new(x, y),
        label_anchor: None,
    };

    assert!(engine.add_single_bond_between(anchor(None, 100.0, 100.0), anchor(None, 148.0, 100.0),));
    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 124.0,
        y: 100.0,
        button: Some(0),
        alt_key: false,
    });
    assert!(engine
        .add_single_bond_between(anchor(Some("n_1"), 100.0, 100.0), anchor(None, 100.0, 52.0),));
    assert!(engine.add_single_bond_between(
        anchor(Some("n_1"), 100.0, 100.0),
        anchor(None, 100.0, 148.0),
    ));
    assert!(engine.add_single_bond_between(
        anchor(Some("n_2"), 148.0, 100.0),
        anchor(None, 148.0, 148.0),
    ));

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(
        entry.fragment.bonds[0]
            .double
            .as_ref()
            .map(|double| double.placement),
        Some(DoubleBondPlacement::Left)
    );

    assert!(engine
        .add_single_bond_between(anchor(Some("n_2"), 148.0, 100.0), anchor(None, 148.0, 52.0),));

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(
        entry.fragment.bonds[0]
            .double
            .as_ref()
            .map(|double| double.placement),
        Some(DoubleBondPlacement::Right)
    );
    assert_eq!(
        entry.fragment.bonds[0]
            .double
            .as_ref()
            .map(|double| double.frozen),
        Some(false)
    );
}

#[test]
fn adding_cis_substituent_to_unfrozen_monosubstituted_double_moves_to_inner_side() {
    let mut engine = Engine::new();
    engine.add_single_bond(
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemsema_engine::Point::new(FIRST_END_X, FIRST_END_Y),
    );
    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_1".to_string()),
            object_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(268.82, 242.0),
            label_anchor: None,
        },
    );

    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    {
        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        assert_eq!(
            bond.double.as_ref().map(|double| double.placement),
            Some(DoubleBondPlacement::Right)
        );
        assert_eq!(
            bond.double.as_ref().map(|double| double.frozen),
            Some(false)
        );
    }

    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            object_id: None,
            point: chemsema_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: chemsema_engine::Point::new(
                FIRST_END_SINGLE_EXTEND_X,
                FIRST_END_SINGLE_EXTEND_Y,
            ),
            label_anchor: None,
        },
    );

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left)
    );
    assert_eq!(
        bond.double.as_ref().map(|double| double.frozen),
        Some(false)
    );
}

#[test]
fn frozen_double_bond_keeps_manual_style_after_new_attachment() {
    let mut engine = Engine::new();
    engine.add_single_bond(
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemsema_engine::Point::new(FIRST_END_X, FIRST_END_Y),
    );
    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_1".to_string()),
            object_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: px_point(268.82, 242.0),
            label_anchor: None,
        },
    );
    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            object_id: None,
            point: chemsema_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: chemsema_engine::Point::new(
                FIRST_END_SINGLE_EXTEND_X,
                FIRST_END_SINGLE_EXTEND_Y,
            ),
            label_anchor: None,
        },
    );

    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let manual_placement = {
        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        let placement = bond.double.as_ref().map(|double| double.placement);
        assert!(matches!(
            placement,
            Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
        ));
        assert_eq!(bond.double.as_ref().map(|double| double.frozen), Some(true));
        placement
    };

    engine.add_single_bond_between(
        chemsema_engine::BondAnchor {
            node_id: Some("n_1".to_string()),
            object_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemsema_engine::BondAnchor {
            node_id: None,
            object_id: None,
            point: chemsema_engine::Point::new(ROOT_SINGLE_BRANCH_X, ROOT_SINGLE_BRANCH_Y),
            label_anchor: None,
        },
    );

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        manual_placement
    );
    assert_eq!(bond.double.as_ref().map(|double| double.frozen), Some(true));
}

#[test]
fn bracket_tool_drag_creates_bracket_object() {
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

    let bracket_group = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "group")
        .expect("dragging bracket tool should create bracket group");
    assert_eq!(
        bracket_group
            .meta
            .get("kind")
            .and_then(|value| value.as_str()),
        Some("bracket-group")
    );
    assert_eq!(bracket_group.payload.bbox, Some([120.0, 130.0, 60.0, 90.0]));
    let sides: Vec<_> = bracket_group
        .children
        .iter()
        .filter(|object| object.object_type == "bracket")
        .collect();
    assert_eq!(sides.len(), 2);
    assert!(sides.iter().all(|side| side
        .payload
        .extra
        .get("kind")
        .and_then(|value| value.as_str())
        == Some("square")));
    let side_ids: Vec<String> = sides.iter().map(|side| side.id.clone()).collect();

    engine.set_tool_state(select_tool());
    assert_eq!(
        engine.state().selection.arrow_objects,
        side_ids,
        "new bracket pairs should select the child brackets drawn together"
    );
}

#[test]
fn selected_bracket_stroke_hits_count_as_selection_points() {
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
    let bbox = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "group")
        .and_then(|object| object.payload.bbox)
        .expect("dragging bracket tool should create bracket group bounds");

    engine.set_tool_state(select_tool());

    let mut checked_hits = 0usize;
    let min_x = bbox[0] - 35.0;
    let max_x = bbox[0] + bbox[2] + 35.0;
    let min_y = bbox[1] - 10.0;
    let max_y = bbox[1] + bbox[3] + 10.0;
    let mut y = min_y;
    while y <= max_y {
        let mut x = min_x;
        while x <= max_x {
            let point = Point::new(x, y);
            let hit: serde_json::Value =
                serde_json::from_str(&engine.context_hit_test_json(point)).unwrap();
            if hit.get("objectType").and_then(|value| value.as_str()) == Some("bracket")
                && hit.get("selected").and_then(|value| value.as_bool()) == Some(true)
            {
                checked_hits += 1;
                assert!(
                    engine.selection_contains_point(point),
                    "selected bracket hit at {point:?} should count as a selection point"
                );
            }
            x += 2.5;
        }
        y += 2.5;
    }
    assert!(
        checked_hits > 0,
        "round bracket scan should find selected hits"
    );
}

#[test]
fn bracket_tool_focuses_bonds_but_not_endpoints() {
    let mut engine = Engine::new();
    click(&mut engine, FIRST_START_X, FIRST_START_Y);
    engine.set_tool_state(ToolState {
        active_tool: Tool::Bracket,
        bracket_kind: BracketKind::Round,
        ..ToolState::default()
    });

    hover(&mut engine, FIRST_CENTER_X, FIRST_CENTER_Y);
    assert!(engine.state().overlay.hover_bond_center.is_some());
    assert!(engine.state().overlay.hover_endpoint.is_none());

    hover(&mut engine, FIRST_END_X, FIRST_END_Y);
    assert!(engine.state().overlay.hover_endpoint.is_none());
}

#[test]
fn bracket_symbol_click_creates_selectable_symbol_object() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::DoubleDagger,
        ..ToolState::default()
    });

    click(&mut engine, 150.0, 160.0);
    let symbol = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("clicking double dagger should create symbol object");
    assert_eq!(
        symbol
            .payload
            .extra
            .get("kind")
            .and_then(|value| value.as_str()),
        Some("double-dagger")
    );
    let symbol_id = symbol.id.clone();

    engine.set_tool_state(ToolState {
        active_tool: Tool::Select,
        ..ToolState::default()
    });
    engine.select_at_point(Point::new(150.0, 160.0), false);
    assert_eq!(engine.state().selection.arrow_objects, vec![symbol_id]);
}

#[test]
fn electron_symbol_click_on_atom_sets_radical_semantics_and_exports_it() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, FIRST_START_X, FIRST_START_Y);
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::Electron,
        ..ToolState::default()
    });

    click(&mut engine, FIRST_START_X, FIRST_START_Y);

    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable molecule");
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| {
            Point::new(node.position[0], node.position[1])
                .distance(Point::new(FIRST_START_X, FIRST_START_Y))
                < 0.1
        })
        .expect("clicked atom");
    assert_eq!(chemsema_engine::node_radical_count(node), 1);
    assert_eq!(
        node.atom_properties.radical,
        chemsema_engine::AtomRadical::Doublet
    );
    assert!(chemsema_engine::node_attached_electron_symbols(node)
        .iter()
        .any(|symbol| symbol["radicalDelta"] == 1));
    let cdxml = engine.document_cdxml();
    assert_eq!(cdxml.matches("SymbolType=\"Electron\"").count(), 1);
    assert!(
        cdxml.contains("<represent attribute=\"Radical\""),
        "the electron graphic must carry its radical semantics without a duplicate node attribute"
    );
    assert!(
        !cdxml.contains("Radical=\"Doublet\""),
        "an attached electron graphic and Radical attribute would display the same dot twice"
    );
    let mut reopened = Engine::new();
    reopened
        .load_cdxml_document(&cdxml)
        .expect("exported radical CDXML reopens");
    assert!(reopened
        .state()
        .document
        .editable_fragment()
        .expect("reopened editable molecule")
        .fragment
        .nodes
        .iter()
        .any(|node| chemsema_engine::node_radical_count(node) == 1));
}

#[test]
fn symbol_tools_focus_endpoints_and_label_glyphs_but_not_bonds() {
    let symbol_kinds = [
        BracketKind::CirclePlus,
        BracketKind::Plus,
        BracketKind::RadicalCation,
        BracketKind::LonePair,
        BracketKind::CircleMinus,
        BracketKind::Minus,
        BracketKind::RadicalAnion,
        BracketKind::Electron,
    ];

    let mut engine = Engine::new();
    click(&mut engine, FIRST_START_X, FIRST_START_Y);
    for symbol_kind in symbol_kinds {
        engine.set_tool_state(ToolState {
            active_tool: Tool::Symbol,
            symbol_kind,
            ..ToolState::default()
        });

        hover(&mut engine, FIRST_CENTER_X, FIRST_CENTER_Y);
        assert!(engine.state().overlay.hover_bond_center.is_none());
        assert!(engine.state().overlay.hover_endpoint.is_none());

        hover(&mut engine, FIRST_END_X, FIRST_END_Y);
        let endpoint = engine
            .state()
            .overlay
            .hover_endpoint
            .as_ref()
            .unwrap_or_else(|| panic!("{symbol_kind:?} should focus a bare endpoint"));
        assert!(endpoint.label_anchor.is_none());
        assert!(engine.state().overlay.hover_bond_center.is_none());
        assert!(engine.render_list().iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Circle {
                role: RenderRole::HoverEndpoint,
                ..
            }
        )));
    }

    let mut labeled_engine = Engine::new();
    load_label_document(
        &mut labeled_engine,
        "N",
        vec![rect_polygon(294.0, 256.0, 300.0, 264.0)],
        json!([]),
    );
    let labeled_hit_point = {
        let label = labeled_engine
            .state()
            .document
            .editable_fragment()
            .unwrap()
            .fragment
            .nodes[0]
            .label
            .as_ref()
            .unwrap();
        label_glyph_hit_point(label, 0)
    };
    for symbol_kind in symbol_kinds {
        labeled_engine.set_tool_state(ToolState {
            active_tool: Tool::Symbol,
            symbol_kind,
            ..ToolState::default()
        });
        hover(
            &mut labeled_engine,
            labeled_hit_point.x,
            labeled_hit_point.y,
        );
        let endpoint = labeled_engine
            .state()
            .overlay
            .hover_endpoint
            .as_ref()
            .unwrap_or_else(|| panic!("{symbol_kind:?} should focus a labeled endpoint"));
        assert!(endpoint.label_anchor.is_some());
        assert!(labeled_engine
            .render_list()
            .iter()
            .any(|primitive| matches!(
                primitive,
                RenderPrimitive::Rect {
                    role: RenderRole::HoverLabelGlyph,
                    ..
                }
            )));
    }
}

#[test]
fn symbol_tool_drag_from_endpoint_orbits_around_endpoint() {
    let mut engine = Engine::new();
    click(&mut engine, FIRST_START_X, FIRST_START_Y);
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::Plus,
        ..ToolState::default()
    });

    drag(
        &mut engine,
        Point::new(FIRST_END_X, FIRST_END_Y),
        Point::new(FIRST_END_X, FIRST_END_Y - 13.0),
    );

    let symbol = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("dragging from an endpoint should create a symbol");
    assert_eq!(
        symbol
            .payload
            .extra
            .get("kind")
            .and_then(|value| value.as_str()),
        Some("plus")
    );
    assert_eq!(
        round_to_2(symbol.transform.translate[0]),
        round_to_2(FIRST_END_X - 2.16675)
    );
    assert_eq!(
        round_to_2(symbol.transform.translate[1]),
        round_to_2(FIRST_END_Y - 13.0 - 2.16675)
    );
}

#[test]
fn symbol_tool_click_on_single_bond_endpoint_uses_extension_angle() {
    let mut engine = Engine::new();
    load_symbol_direction_document(
        &mut engine,
        json!([
            {"id": "n1", "element": "C", "atomicNumber": 6, "position": [100.0, 100.0], "charge": 0, "numHydrogens": 0},
            {"id": "n2", "element": "C", "atomicNumber": 6, "position": [130.0, 100.0], "charge": 0, "numHydrogens": 0}
        ]),
        json!([
            {"id": "b1", "begin": "n1", "end": "n2", "order": 1}
        ]),
    );
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::CirclePlus,
        ..ToolState::default()
    });

    click(&mut engine, 130.0, 100.0);

    let symbol = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("clicking endpoint should create a symbol");
    assert_eq!(round_to_2(symbol.transform.translate[0]), 132.50);
    assert_eq!(round_to_2(symbol.transform.translate[1]), 96.40);
}

#[test]
fn symbol_tool_click_on_two_bond_junction_uses_convex_side_center() {
    let mut engine = Engine::new();
    load_symbol_direction_document(
        &mut engine,
        json!([
            {"id": "n1", "element": "C", "atomicNumber": 6, "position": [100.0, 100.0], "charge": 0, "numHydrogens": 0},
            {"id": "n2", "element": "C", "atomicNumber": 6, "position": [70.0, 130.0], "charge": 0, "numHydrogens": 0},
            {"id": "n3", "element": "C", "atomicNumber": 6, "position": [130.0, 130.0], "charge": 0, "numHydrogens": 0}
        ]),
        json!([
            {"id": "b1", "begin": "n1", "end": "n2", "order": 1},
            {"id": "b2", "begin": "n1", "end": "n3", "order": 1}
        ]),
    );
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::CirclePlus,
        ..ToolState::default()
    });

    click(&mut engine, 100.0, 100.0);

    let symbol = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("clicking two-bond junction should create a symbol");
    assert_eq!(round_to_2(symbol.transform.translate[0]), 96.40);
    assert_eq!(round_to_2(symbol.transform.translate[1]), 90.30);
}

#[test]
fn charge_symbol_attaches_to_terminal_carbon_and_reduces_hidden_hydrogen() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, FIRST_START_X, FIRST_START_Y);
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::Plus,
        ..ToolState::default()
    });

    click(&mut engine, FIRST_END_X, FIRST_END_Y);

    let entry = engine.state().document.editable_fragment().unwrap();
    let terminal = entry
        .fragment
        .nodes
        .iter()
        .find(|node| (node.position[0] - FIRST_END_X).abs() < 0.01)
        .expect("terminal carbon should exist");
    assert_eq!(terminal.charge, 1);
    assert_eq!(
        terminal
            .meta
            .get("effectiveNumHydrogens")
            .and_then(|value| value.as_u64()),
        Some(2)
    );
    assert_eq!(
        terminal
            .meta
            .get("chargeSymbolInvalid")
            .and_then(|value| value.as_bool()),
        None
    );
    let symbol = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("symbol should exist");
    assert_eq!(
        symbol
            .payload
            .extra
            .get("attachedAtomId")
            .and_then(|value| value.as_str()),
        Some(terminal.id.as_str())
    );
}

#[test]
fn ordinary_charge_or_radical_on_four_connected_carbon_is_invalid_but_radical_ion_is_allowed() {
    fn four_connected_engine(symbol_kind: BracketKind) -> Engine {
        let mut engine = Engine::new();
        load_symbol_direction_document(
            &mut engine,
            json!([
                {"id": "n0", "element": "C", "atomicNumber": 6, "position": [100.0, 100.0], "charge": 0, "numHydrogens": 0},
                {"id": "n1", "element": "C", "atomicNumber": 6, "position": [70.0, 100.0], "charge": 0, "numHydrogens": 0},
                {"id": "n2", "element": "C", "atomicNumber": 6, "position": [130.0, 100.0], "charge": 0, "numHydrogens": 0},
                {"id": "n3", "element": "C", "atomicNumber": 6, "position": [100.0, 70.0], "charge": 0, "numHydrogens": 0},
                {"id": "n4", "element": "C", "atomicNumber": 6, "position": [100.0, 130.0], "charge": 0, "numHydrogens": 0}
            ]),
            json!([
                {"id": "b1", "begin": "n0", "end": "n1", "order": 1},
                {"id": "b2", "begin": "n0", "end": "n2", "order": 1},
                {"id": "b3", "begin": "n0", "end": "n3", "order": 1},
                {"id": "b4", "begin": "n0", "end": "n4", "order": 1}
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

    for symbol_kind in [BracketKind::Plus, BracketKind::Minus, BracketKind::Electron] {
        let engine = four_connected_engine(symbol_kind);
        let entry = engine.state().document.editable_fragment().unwrap();
        let center = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == "n0")
            .unwrap();
        assert_eq!(
            center
                .meta
                .get("chargeSymbolInvalid")
                .and_then(|value| value.as_bool()),
            Some(true),
            "{symbol_kind:?} should be invalid on four-connected carbon"
        );
        assert!(
            engine.render_list().into_iter().any(|primitive| matches!(
                primitive,
                RenderPrimitive::Circle {
                    role: RenderRole::DocumentDiagnostic,
                    node_id: Some(ref node_id),
                    stroke,
                    ..
                } if node_id == "n0" && stroke == "#d32f2f"
            )),
            "{symbol_kind:?} should render an invalid red circle"
        );
    }

    for symbol_kind in [BracketKind::RadicalCation, BracketKind::RadicalAnion] {
        let engine = four_connected_engine(symbol_kind);
        let entry = engine.state().document.editable_fragment().unwrap();
        let center = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == "n0")
            .unwrap();
        assert_eq!(
            center
                .meta
                .get("chargeSymbolInvalid")
                .and_then(|value| value.as_bool()),
            None,
            "{symbol_kind:?} should be allowed on four-connected carbon"
        );
    }
}
