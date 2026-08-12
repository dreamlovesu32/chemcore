use super::*;

#[test]
fn switching_to_select_without_tool_changes_does_not_restore_previous_latest_object() {
    let mut engine = Engine::new();
    engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: 20.0,
        y: 20.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 60.0,
        y: 44.0,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 60.0,
        y: 44.0,
        button: Some(0),
        alt_key: false,
    });
    engine.set_tool_state(select_tool());
    assert!(!engine.state().selection.arrow_objects.is_empty());

    engine.select_at_point(Point::new(500.0, 500.0), false);
    assert!(engine.state().selection.is_empty());

    engine.set_tool_state(shape_tool(ShapeKind::Circle, ShapeStyle::Solid));
    engine.set_tool_state(select_tool());
    assert!(engine.state().selection.is_empty());
}

#[test]
fn shape_tool_circle_uses_click_as_center_and_cursor_as_radius() {
    let mut engine = Engine::new();
    let center = px_point(300.0, 260.0);
    let cursor = px_point(360.0, 290.0);

    engine.set_tool_state(shape_tool(ShapeKind::Circle, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: center.x,
        y: center.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: cursor.x,
        y: cursor.y,
        button: None,
        alt_key: false,
    });

    let preview = engine.render_list();
    assert!(preview.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            object_id: Some(id),
            stroke_width,
            ..
        } if id == "__preview_shape" && (*stroke_width - 1.0).abs() < 0.001
    )));

    engine.pointer_up(PointerEvent {
        x: cursor.x,
        y: cursor.y,
        button: Some(0),
        alt_key: false,
    });

    assert_point_close(shape_payload_point(&engine, "center"), center);
    assert_point_close(shape_payload_point(&engine, "majorAxisEnd"), cursor);
    let minor = shape_payload_point(&engine, "minorAxisEnd");
    let radius = center.distance(cursor);
    assert!((center.distance(minor) - radius).abs() < 0.001);
}

#[test]
fn shape_tool_ellipse_uses_center_and_snaps_major_axis_to_15_degrees() {
    let mut engine = Engine::new();
    let center = px_point(300.0, 260.0);
    let cursor = center.translated(direction_from_angle(29.0).scaled(DEFAULT_BOND_LENGTH * 2.0));

    engine.set_tool_state(shape_tool(ShapeKind::Ellipse, ShapeStyle::Dashed));
    engine.pointer_down(PointerEvent {
        x: center.x,
        y: center.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: cursor.x,
        y: cursor.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: cursor.x,
        y: cursor.y,
        button: Some(0),
        alt_key: false,
    });

    assert_point_close(shape_payload_point(&engine, "center"), center);
    let major = shape_payload_point(&engine, "majorAxisEnd");
    let minor = shape_payload_point(&engine, "minorAxisEnd");
    assert!((angle_between(center, major) - 30.0).abs() < 0.001);
    assert!((center.distance(minor) / center.distance(major) - 0.4).abs() < 0.001);
}

#[test]
fn shape_tool_rectangles_use_drag_corners() {
    let mut engine = Engine::new();
    let top_left = px_point(300.0, 260.0);
    let bottom_right = px_point(380.0, 330.0);

    engine.set_tool_state(shape_tool(ShapeKind::RoundRect, ShapeStyle::Shadowed));
    engine.pointer_down(PointerEvent {
        x: top_left.x,
        y: top_left.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: bottom_right.x,
        y: bottom_right.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: bottom_right.x,
        y: bottom_right.y,
        button: Some(0),
        alt_key: false,
    });

    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "shape")
        .expect("shape object should exist");
    assert_eq!(object.transform.translate, [top_left.x, top_left.y]);
    assert_eq!(
        object.payload.bbox,
        Some([
            0.0,
            0.0,
            bottom_right.x - top_left.x,
            bottom_right.y - top_left.y
        ])
    );
    assert_eq!(
        object
            .payload
            .extra
            .get("kind")
            .and_then(serde_json::Value::as_str),
        Some("roundRect")
    );
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            stroke_width,
            dash_array,
            d,
            ..
        } if (*stroke_width - 1.0).abs() < 0.001 && dash_array.is_empty() && d.starts_with("M ")
    )));
}

#[test]
fn shape_tool_click_on_existing_atom_adds_fixed_rect_centered_on_atom() {
    let mut engine = Engine::new();
    let endpoint = px_point(300.0, 260.0);
    let target = px_point(330.0, 260.0);

    engine.set_tool_state(bond_tool());
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

    engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: endpoint.x,
        y: endpoint.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: endpoint.x,
        y: endpoint.y,
        button: Some(0),
        alt_key: false,
    });

    let object = first_shape_object(&engine);
    assert_point_close(
        Point::new(object.transform.translate[0], object.transform.translate[1]),
        Point::new(endpoint.x - 7.7, endpoint.y - 7.7),
    );
    let bbox = object.payload.bbox.expect("shape should have bbox");
    assert!((bbox[2] - 15.4).abs() < 1e-9, "{bbox:?}");
    assert!((bbox[3] - 15.4).abs() < 1e-9, "{bbox:?}");
}

#[test]
fn shape_tool_drag_from_atom_uses_atom_as_rect_corner() {
    let mut engine = Engine::new();
    let endpoint = px_point(300.0, 260.0);
    let bond_target = px_point(330.0, 260.0);
    let rect_target = px_point(340.0, 292.0);

    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: endpoint.x,
        y: endpoint.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: bond_target.x,
        y: bond_target.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: bond_target.x,
        y: bond_target.y,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: endpoint.x,
        y: endpoint.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: rect_target.x,
        y: rect_target.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: rect_target.x,
        y: rect_target.y,
        button: Some(0),
        alt_key: false,
    });

    let object = first_shape_object(&engine);
    assert_point_close(
        Point::new(object.transform.translate[0], object.transform.translate[1]),
        endpoint,
    );
    assert_eq!(
        object.payload.bbox,
        Some([
            0.0,
            0.0,
            rect_target.x - endpoint.x,
            rect_target.y - endpoint.y
        ])
    );
}

#[test]
fn shape_tool_click_on_label_rect_uses_label_box() {
    let mut engine = Engine::new();
    load_label_document(
        &mut engine,
        "Ph",
        vec![json!([
            [px(294.0), px(256.0)],
            [px(324.0), px(256.0)],
            [px(324.0), px(264.0)],
            [px(294.0), px(264.0)]
        ])],
        json!([]),
    );
    let label_center = px_point(309.0, 260.0);

    engine.set_tool_state(shape_tool(ShapeKind::RoundRect, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: label_center.x,
        y: label_center.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: label_center.x,
        y: label_center.y,
        button: Some(0),
        alt_key: false,
    });

    let object = first_shape_object(&engine);
    assert_point_close(
        Point::new(object.transform.translate[0], object.transform.translate[1]),
        px_point(294.0, 256.0),
    );
    assert_eq!(object.payload.bbox, Some([0.0, 0.0, px(30.0), px(8.0)]));
}

#[test]
fn shape_tool_drag_from_label_circle_uses_label_center() {
    let mut engine = Engine::new();
    load_label_document(
        &mut engine,
        "Ph",
        vec![json!([
            [px(294.0), px(256.0)],
            [px(324.0), px(256.0)],
            [px(324.0), px(264.0)],
            [px(294.0), px(264.0)]
        ])],
        json!([]),
    );
    let click_point = px_point(296.0, 258.0);
    let label_center = px_point(309.0, 260.0);
    let target = px_point(340.0, 260.0);

    engine.set_tool_state(shape_tool(ShapeKind::Circle, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: click_point.x,
        y: click_point.y,
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

    assert_point_close(shape_payload_point(&engine, "center"), label_center);
    assert_point_close(shape_payload_point(&engine, "majorAxisEnd"), target);
}

#[test]
fn shape_tool_ignores_plain_text_focus() {
    let mut engine = Engine::new();
    load_text_object_document(&mut engine);

    engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Solid));
    engine.pointer_move(PointerEvent {
        x: px(300.0),
        y: px(250.0),
        button: None,
        alt_key: false,
    });
    assert!(engine.state().overlay.hover_text_box.is_none());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(250.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(250.0),
        button: Some(0),
        alt_key: false,
    });
    assert!(
        engine
            .state()
            .document
            .objects
            .iter()
            .all(|object| object.object_type != "shape"),
        "plain text click should not create a shape"
    );
}

#[test]
fn select_tool_click_selects_loaded_shape_object() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_shape_select",
            "title": "shape select",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape": {
                "kind": "shape",
                "stroke": "#000000",
                "strokeWidth": 0.6,
                "fill": null
            }
        },
        "objects": [{
            "id": "obj_shape_loaded",
            "type": "shape",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [20.0, 30.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_shape",
            "payload": {
                "kind": "rect",
                "bbox": [0.0, 0.0, 40.0, 24.0]
            }
        }],
        "resources": {}
    });
    engine
        .load_document_json(&document.to_string())
        .expect("shape document should load");
    engine.set_tool_state(select_tool());

    engine.select_at_point(Point::new(40.0, 42.0), false);

    assert_eq!(
        engine.state().selection.arrow_objects,
        vec!["obj_shape_loaded".to_string()]
    );
}

#[test]
fn select_tool_shape_hover_and_hit_testing_follow_shape_geometry() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_shape_select",
            "title": "shape select",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape": {
                "kind": "shape",
                "stroke": "#000000",
                "strokeWidth": 0.6,
                "fill": null
            }
        },
        "objects": [{
            "id": "obj_circle_loaded",
            "type": "shape",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_shape",
            "payload": {
                "kind": "circle",
                "bbox": [20.0, 20.0, 40.0, 40.0],
                "center": [40.0, 40.0],
                "majorAxisEnd": [60.0, 40.0],
                "minorAxisEnd": [40.0, 60.0]
            }
        }],
        "resources": {}
    });
    engine
        .load_document_json(&document.to_string())
        .expect("shape document should load");
    engine.set_tool_state(select_tool());

    engine.pointer_move(PointerEvent {
        x: 40.0,
        y: 20.0,
        button: None,
        alt_key: false,
    });
    assert_eq!(
        engine.hover_shape_action_at_point(Point::new(40.0, 20.0)),
        "circle-radius"
    );
    assert_eq!(hover_shape_handle_count(&engine), 1);

    engine.select_at_point(Point::new(40.0, 40.0), false);
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec!["obj_circle_loaded".to_string()]
    );
    engine.select_at_point(
        Point::new(60.0 + GRAPHIC_EDGE_HIT_RADIUS - px(0.25), 40.0),
        false,
    );
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec!["obj_circle_loaded".to_string()]
    );
    engine.select_at_point(
        Point::new(60.0 + GRAPHIC_EDGE_HIT_RADIUS + px(0.25), 40.0),
        false,
    );
    assert!(engine.state().selection.arrow_objects.is_empty());

    engine.select_at_point(Point::new(63.5, 63.5), false);
    assert!(engine.state().selection.arrow_objects.is_empty());
}

#[test]
fn selected_shape_boxes_are_tight_axis_aligned_bounds() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_shape_selection_boxes",
            "title": "shape selection boxes",
            "page": { "width": 260.0, "height": 180.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape": {
                "kind": "shape",
                "stroke": "#000000",
                "strokeWidth": 0.6,
                "fill": null
            }
        },
        "objects": [
            {
                "id": "shape_circle",
                "type": "shape",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_shape",
                "payload": {
                    "kind": "circle",
                    "bbox": [20.0, 20.0, 40.0, 40.0],
                    "center": [40.0, 40.0],
                    "majorAxisEnd": [60.0, 40.0],
                    "minorAxisEnd": [40.0, 60.0]
                }
            },
            {
                "id": "shape_ellipse",
                "type": "shape",
                "visible": true,
                "zIndex": 11,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_shape",
                "payload": {
                    "kind": "ellipse",
                    "bbox": [80.0, 60.0, 80.0, 80.0],
                    "center": [120.0, 100.0],
                    "majorAxisEnd": [160.0, 100.0],
                    "minorAxisEnd": [120.0, 112.0]
                }
            },
            {
                "id": "shape_rect",
                "type": "shape",
                "visible": true,
                "zIndex": 12,
                "transform": { "translate": [170.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_shape",
                "payload": {
                    "kind": "rect",
                    "bbox": [0.0, 0.0, 48.0, 24.0]
                }
            },
            {
                "id": "shape_rotated_ellipse",
                "type": "shape",
                "visible": true,
                "zIndex": 13,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_shape",
                "payload": {
                    "kind": "ellipse",
                    "bbox": [10.0, 80.0, 100.0, 100.0],
                    "center": [50.0, 130.0],
                    "majorAxisEnd": [80.0, 170.0],
                    "minorAxisEnd": [42.0, 136.0]
                }
            }
        ],
        "resources": {}
    });
    engine
        .load_document_json(&document.to_string())
        .expect("shape document should load");
    engine.set_tool_state(select_tool());

    engine.select_at_point(Point::new(40.0, 40.0), false);
    assert_rect_close(selection_box_rect(&engine), (19.7, 19.7, 40.6, 40.6));

    engine.select_at_point(Point::new(120.0, 100.0), false);
    assert_rect_close(selection_box_rect(&engine), (79.7, 87.7, 80.6, 24.6));

    engine.select_at_point(Point::new(190.0, 50.0), false);
    assert_rect_close(selection_box_rect(&engine), (169.7, 39.7, 48.6, 24.6));

    engine.select_at_point(Point::new(50.0, 130.0), false);
    let rotated_extent_x = (30.0_f64 * 30.0 + (-8.0_f64) * (-8.0)).sqrt();
    let rotated_extent_y = (40.0_f64 * 40.0 + 6.0 * 6.0).sqrt();
    assert_rect_close(
        selection_box_rect(&engine),
        (
            50.0 - rotated_extent_x - 0.3,
            130.0 - rotated_extent_y - 0.3,
            rotated_extent_x * 2.0 + 0.6,
            rotated_extent_y * 2.0 + 0.6,
        ),
    );
}

#[test]
fn shape_tool_circle_edge_handle_resizes_existing_circle_without_drawing_new_shape() {
    let mut engine = Engine::new();
    let center = Point::new(40.0, 40.0);
    let edge = Point::new(60.0, 40.0);
    engine.set_tool_state(shape_tool(ShapeKind::Circle, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: center.x,
        y: center.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: edge.x,
        y: edge.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: edge.x,
        y: edge.y,
        button: Some(0),
        alt_key: false,
    });

    assert_eq!(shape_object_count(&engine), 1);
    engine.pointer_move(PointerEvent {
        x: edge.x,
        y: edge.y,
        button: None,
        alt_key: false,
    });
    assert_eq!(engine.hover_shape_action_at_point(edge), "circle-radius");
    assert_eq!(hover_shape_handle_count(&engine), 1);

    engine.pointer_down(PointerEvent {
        x: edge.x,
        y: edge.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 80.0,
        y: 40.0,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 80.0,
        y: 40.0,
        button: Some(0),
        alt_key: false,
    });
    assert!(engine.state().overlay.hover_shape.is_none());
    assert_eq!(hover_shape_handle_count(&engine), 0);

    assert_eq!(shape_object_count(&engine), 1);
    assert_point_close(
        shape_payload_point(&engine, "majorAxisEnd"),
        Point::new(80.0, 40.0),
    );
}

#[test]
fn shape_tool_ellipse_handles_resize_axes_and_non_handle_drag_draws_new_shape() {
    let mut engine = Engine::new();
    let center = Point::new(40.0, 40.0);
    let major = Point::new(80.0, 40.0);
    engine.set_tool_state(shape_tool(ShapeKind::Ellipse, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: center.x,
        y: center.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: major.x,
        y: major.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: major.x,
        y: major.y,
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_move(PointerEvent {
        x: major.x,
        y: major.y,
        button: None,
        alt_key: false,
    });
    assert_eq!(
        engine.hover_shape_action_at_point(major),
        "ellipse-major-positive"
    );
    assert_eq!(hover_shape_handle_count(&engine), 4);

    engine.pointer_down(PointerEvent {
        x: major.x,
        y: major.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 100.0,
        y: 40.0,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 100.0,
        y: 40.0,
        button: Some(0),
        alt_key: false,
    });
    assert_eq!(shape_object_count(&engine), 1);
    assert_point_close(
        shape_payload_point(&engine, "majorAxisEnd"),
        Point::new(100.0, 40.0),
    );

    engine.pointer_down(PointerEvent {
        x: center.x,
        y: center.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 65.0,
        y: 65.0,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 65.0,
        y: 65.0,
        button: Some(0),
        alt_key: false,
    });
    assert_eq!(shape_object_count(&engine), 2);
}

#[test]
fn shape_tool_rect_handles_resize_but_edge_non_handles_continue_drawing() {
    let mut engine = Engine::new();
    engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: 20.0,
        y: 20.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 60.0,
        y: 44.0,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 60.0,
        y: 44.0,
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_move(PointerEvent {
        x: 60.0,
        y: 32.0,
        button: None,
        alt_key: false,
    });
    assert_eq!(
        engine.hover_shape_action_at_point(Point::new(60.0, 32.0)),
        "e"
    );
    assert_eq!(hover_shape_handle_count(&engine), 8);
    engine.pointer_down(PointerEvent {
        x: 60.0,
        y: 32.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 80.0,
        y: 32.0,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 80.0,
        y: 32.0,
        button: Some(0),
        alt_key: false,
    });

    assert_eq!(shape_object_count(&engine), 1);
    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "shape")
        .expect("shape object should exist");
    assert_eq!(object.payload.bbox, Some([0.0, 0.0, 60.0, 24.0]));

    engine.pointer_down(PointerEvent {
        x: 35.0,
        y: 20.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 90.0,
        y: 55.0,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 90.0,
        y: 55.0,
        button: Some(0),
        alt_key: false,
    });
    assert_eq!(shape_object_count(&engine), 2);
}

#[test]
fn shape_tool_dashed_round_rect_uses_chemdraw_path_dash_spacing() {
    let mut engine = Engine::new();
    let top_left = px_point(300.0, 260.0);
    let bottom_right = px_point(380.0, 330.0);

    engine.set_tool_state(shape_tool(ShapeKind::RoundRect, ShapeStyle::Dashed));
    engine.pointer_down(PointerEvent {
        x: top_left.x,
        y: top_left.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: bottom_right.x,
        y: bottom_right.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: bottom_right.x,
        y: bottom_right.y,
        button: Some(0),
        alt_key: false,
    });

    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            stroke_width,
            dash_array,
            d,
            ..
        } if (*stroke_width - 1.0).abs() < 0.001
            && dash_array == &vec![2.7]
            && d.starts_with(&format!("M {},{}", top_left.x, bottom_right.y - 6.0))
    )));
}

#[test]
fn shape_tool_shaded_style_renders_chemdraw_gray_layers() {
    let mut engine = Engine::new();
    let top_left = px_point(300.0, 260.0);
    let bottom_right = px_point(380.0, 330.0);

    engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Shaded));
    engine.pointer_down(PointerEvent {
        x: top_left.x,
        y: top_left.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: bottom_right.x,
        y: bottom_right.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: bottom_right.x,
        y: bottom_right.y,
        button: Some(0),
        alt_key: false,
    });

    let render_list = engine.render_list();
    let shaded_fills = render_list
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::FilledPath {
                    role: RenderRole::DocumentGraphic,
                    fill_rule: None,
                    ..
                }
            )
        })
        .count();
    assert!(
        shaded_fills >= 32,
        "expected ChemDraw-style shaded fill stack"
    );
    assert!(render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            stroke_width,
            dash_array,
            ..
        } if (*stroke_width - 1.0).abs() < 0.001 && dash_array.is_empty()
    )));
}

#[test]
fn shape_tool_shadowed_style_masks_shadow_inside_original_shape() {
    let mut engine = Engine::new();
    let top_left = px_point(300.0, 260.0);
    let bottom_right = px_point(380.0, 330.0);

    engine.set_tool_state(shape_tool(ShapeKind::RoundRect, ShapeStyle::Shadowed));
    engine.pointer_down(PointerEvent {
        x: top_left.x,
        y: top_left.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: bottom_right.x,
        y: bottom_right.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: bottom_right.x,
        y: bottom_right.y,
        button: Some(0),
        alt_key: false,
    });

    let render_list = engine.render_list();
    assert!(render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::FilledPath {
            role: RenderRole::DocumentGraphic,
            clip_rule: Some(rule),
            clip_path_d: Some(_),
            fill,
            ..
        } if rule == "evenodd" && fill == "rgba(0,0,0,0.247059)"
    )));
    assert!(!render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::DocumentGraphic,
            stroke: None,
            stroke_width,
            ..
        } if *stroke_width == 0.0
    )));
}

#[test]
fn shape_tool_shadowed_ellipse_matches_reference_clipped_shadow() {
    let mut engine = Engine::new();
    let center = px_point(300.0, 260.0);
    let cursor = center.translated(direction_from_angle(30.0).scaled(DEFAULT_BOND_LENGTH * 2.0));

    engine.set_tool_state(shape_tool(ShapeKind::Ellipse, ShapeStyle::Shadowed));
    engine.pointer_down(PointerEvent {
        x: center.x,
        y: center.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: cursor.x,
        y: cursor.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: cursor.x,
        y: cursor.y,
        button: Some(0),
        alt_key: false,
    });

    let render_list = engine.render_list();
    assert!(render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::FilledPath {
            role: RenderRole::DocumentGraphic,
            clip_rule: Some(rule),
            clip_path_d: Some(_),
            fill,
            ..
        } if rule == "evenodd" && fill == "rgba(0,0,0,0.247059)"
    )));
    assert!(!render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::FilledPath {
            role: RenderRole::DocumentGraphic,
            fill_rule: Some(rule),
            ..
        } if rule == "evenodd"
    )));
}

#[test]
fn select_tool_click_on_side_double_bond_wraps_both_lines() {
    let mut single = Engine::new();
    single.set_tool_state(bond_tool());
    click(&mut single, px(300.0), px(260.0));
    single.set_tool_state(select_tool());
    single.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    let (_, _, single_width, single_height) = selection_bond_rect(&single);

    let mut double = Engine::new();
    double.set_tool_state(double_bond_tool());
    click(&mut double, px(300.0), px(260.0));
    double.set_tool_state(select_tool());
    double.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    let (_, _, double_width, double_height) = selection_bond_rect(&double);

    assert!(
        double_width > single_width + 0.04 || double_height > single_height + 0.04,
        "expected double bond rect to exceed single bond rect, single=({single_width}, {single_height}) double=({double_width}, {double_height})"
    );
}

#[test]
fn select_tool_click_on_triple_bond_wraps_all_three_lines() {
    let mut single = Engine::new();
    single.set_tool_state(bond_tool());
    click(&mut single, px(300.0), px(260.0));
    single.set_tool_state(select_tool());
    single.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    let (_, _, single_width, single_height) = selection_bond_rect(&single);

    let mut triple = Engine::new();
    triple.set_tool_state(triple_bond_tool());
    click(&mut triple, px(300.0), px(260.0));
    triple.set_tool_state(select_tool());
    triple.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    let (_, _, triple_width, triple_height) = selection_bond_rect(&triple);

    assert!(
        triple_width > single_width + 0.08 || triple_height > single_height + 0.08,
        "expected triple bond rect to exceed single bond rect, single=({single_width}, {single_height}) triple=({triple_width}, {triple_height})"
    );
}

#[test]
fn select_tool_box_selecting_single_bond_preserves_bond_selection_role() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());

    engine.select_in_rect(px_point(290.0, 234.0), px_point(346.0, 286.0), false);

    assert_eq!(engine.state().selection.nodes.len(), 2);
    assert_eq!(engine.state().selection.bonds.len(), 1);
    assert!(engine.render_list().iter().any(|primitive| {
        matches!(
            primitive,
            RenderPrimitive::Rect {
                role: RenderRole::SelectionBond,
                ..
            }
        )
    }));
    assert!(!engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionBox,
            ..
        }
    )));
    assert_eq!(
        selection_bond_dots(&engine).len(),
        0,
        "box-selecting a complete single-bond fragment should suppress internal bond center affordances"
    );
}

#[test]
fn select_tool_whole_fragment_box_ignores_hidden_atom_handles() {
    let mut complete = Engine::new();
    complete.set_tool_state(bond_tool());
    click(&mut complete, px(300.0), px(260.0));
    complete.set_tool_state(select_tool());
    complete.select_in_rect(px_point(290.0, 234.0), px_point(346.0, 286.0), false);
    let complete_rect = selection_bond_rect(&complete);

    let mut bond = Engine::new();
    bond.set_tool_state(bond_tool());
    click(&mut bond, px(300.0), px(260.0));
    bond.set_tool_state(select_tool());
    bond.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    let bond_rect = selection_bond_rect(&bond);

    assert_rect_close(complete_rect, bond_rect);
    assert!(!complete.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionNode,
            ..
        }
    )));
}

#[test]
fn select_tool_shift_click_adds_to_selection() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());

    engine.select_at_point(Point::new(FIRST_END_X, FIRST_END_Y), false);
    engine.select_at_point(Point::new(FIRST_START_X, FIRST_START_Y), true);

    assert_eq!(engine.state().selection.nodes.len(), 2);
}

#[test]
fn switching_from_select_tool_to_drawing_tool_clears_selection() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());

    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    assert!(!engine.state().selection.is_empty());
    assert!(engine.render_list().iter().any(|primitive| {
        matches!(
            primitive,
            RenderPrimitive::Rect {
                role: RenderRole::SelectionBond,
                ..
            } | RenderPrimitive::Circle {
                role: RenderRole::SelectionBondDot,
                ..
            }
        )
    }));

    engine.set_tool_state(bond_tool());

    assert!(engine.state().selection.is_empty());
    assert!(engine.render_list().iter().all(|primitive| {
        !matches!(
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
        )
    }));
}

#[test]
fn selection_color_applies_to_all_selected_object_kinds() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_selection_color",
            "title": "selection color",
            "page": { "width": 160.0, "height": 100.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#111111",
                "fill": "#111111",
                "strokeWidth": 0.85,
                "fontSize": 10.0
            },
            "style_text": { "kind": "text", "fill": "#111111", "fontSize": 10.0 },
            "style_line": {
                "kind": "line",
                "stroke": "#111111",
                "strokeWidth": 1.0,
                "lineCap": "round",
                "lineJoin": "round"
            },
            "style_shape": {
                "kind": "shape",
                "fill": null,
                "stroke": "#111111",
                "strokeWidth": 1.0
            }
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
                "id": "obj_text",
                "type": "text",
                "styleRef": "style_text",
                "transform": { "translate": [40.0, 4.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {
                    "bbox": [0.0, 0.0, 26.0, 12.0],
                    "text": "note",
                    "fontSize": 10.0,
                    "lineHeight": 12.0,
                    "align": "left",
                    "preserveLines": true
                }
            },
            {
                "id": "obj_line",
                "type": "line",
                "styleRef": "style_line",
                "payload": { "points": [[8.0, 40.0], [30.0, 40.0]], "kind": "line" }
            },
            {
                "id": "obj_shape",
                "type": "shape",
                "styleRef": "style_shape",
                "transform": { "translate": [40.0, 30.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "bbox": [0.0, 0.0, 22.0, 14.0], "kind": "rect" }
            },
            {
                "id": "obj_bracket",
                "type": "bracket",
                "transform": { "translate": [70.0, 26.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {
                    "bbox": [0.0, 0.0, 16.0, 24.0],
                    "kind": "round",
                    "stroke": "#111111",
                    "strokeWidth": 1.0
                }
            },
            {
                "id": "obj_symbol",
                "type": "symbol",
                "transform": { "translate": [96.0, 30.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {
                    "bbox": [0.0, 0.0, 10.0, 10.0],
                    "kind": "plus",
                    "fill": "#111111",
                    "strokeWidth": 1.0
                }
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
                        { "id": "n2", "element": "C", "atomicNumber": 6, "position": [30.0, 10.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
                    ]
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("selection color fixture should load");

    engine.select_in_rect(Point::new(0.0, 0.0), Point::new(120.0, 70.0), false);
    assert_eq!(engine.state().selection.text_objects, vec!["obj_text"]);
    assert!(engine
        .state()
        .selection
        .arrow_objects
        .iter()
        .any(|object_id| object_id == "obj_shape"));
    assert!(engine.apply_color_to_selection("#2288cc"));

    let document = &engine.state().document;
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should remain editable");
    assert_eq!(
        fragment.bonds[0].stroke.as_deref(),
        Some("#2288cc"),
        "selected bond should receive its own stroke"
    );

    let style_value = |style_id: &str, key: &str| {
        document
            .styles
            .get(style_id)
            .and_then(|style| style.get(key))
            .and_then(|value| value.as_str())
            .map(str::to_string)
    };
    assert_eq!(
        style_value("style_obj_text_color", "fill").as_deref(),
        Some("#2288cc")
    );
    assert_eq!(
        style_value("style_obj_line_color", "stroke").as_deref(),
        Some("#2288cc")
    );
    assert_eq!(
        style_value("style_obj_shape_color", "stroke").as_deref(),
        Some("#2288cc")
    );
    assert_eq!(
        style_value("style_obj_mol_color", "stroke").as_deref(),
        Some("#2288cc")
    );
    let payload_value = |object_id: &str, key: &str| {
        document
            .objects
            .iter()
            .find(|object| object.id == object_id)
            .and_then(|object| object.payload.extra.get(key))
            .and_then(|value| value.as_str())
            .map(str::to_string)
    };
    assert_eq!(
        payload_value("obj_bracket", "stroke").as_deref(),
        Some("#2288cc")
    );
    assert_eq!(
        payload_value("obj_symbol", "fill").as_deref(),
        Some("#2288cc")
    );
}

#[test]
fn context_style_commands_apply_to_graphic_text_and_bond_selections() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_context_styles",
            "title": "context styles",
            "page": { "width": 160.0, "height": 100.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": { "kind": "molecule" },
            "style_line": { "kind": "line", "stroke": "#111111", "strokeWidth": 1.0 },
            "style_shape": { "kind": "shape", "stroke": "#111111", "strokeWidth": 1.0, "fill": null },
            "style_text": { "kind": "text", "fill": "#111111", "fontSize": 10.0 }
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
                    "points": [[12.0, 42.0], [34.0, 42.0]],
                    "kind": "line",
                    "arrowHead": { "kind": "solid", "head": "full", "tail": "none", "length": 10.0, "centerLength": 8.0, "width": 2.0, "bold": false }
                }
            },
            {
                "id": "obj_shape",
                "type": "shape",
                "styleRef": "style_shape",
                "transform": { "translate": [48.0, 30.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "bbox": [0.0, 0.0, 22.0, 14.0], "kind": "rect" }
            },
            {
                "id": "obj_bracket",
                "type": "bracket",
                "transform": { "translate": [78.0, 26.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "bbox": [0.0, 0.0, 16.0, 24.0], "kind": "round", "stroke": "#111111", "strokeWidth": 1.0 }
            },
            {
                "id": "obj_text",
                "type": "text",
                "styleRef": "style_text",
                "transform": { "translate": [105.0, 28.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "bbox": [0.0, 0.0, 32.0, 14.0], "text": "note", "fontSize": 10.0, "lineHeight": 12.0, "align": "left", "preserveLines": true }
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
                        { "id": "n2", "element": "C", "atomicNumber": 6, "position": [30.0, 10.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
                    ]
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("context style fixture should load");

    engine.select_at_point(Point::new(55.0, 36.0), false);
    assert!(engine.apply_shape_style_to_selection("filled"));
    let shape = engine
        .state()
        .document
        .find_scene_object("obj_shape")
        .expect("shape should exist");
    let shape_style = engine
        .state()
        .document
        .styles
        .get(shape.style_ref.as_deref().unwrap())
        .expect("shape style should exist");
    assert!(shape_style
        .get("stroke")
        .is_some_and(|value| value.is_null()));
    assert_eq!(shape_style["fill"], "#111111");

    engine.select_at_point(Point::new(80.0, 30.0), false);
    assert!(engine.apply_bracket_kind_to_selection("curly"));
    let bracket = engine
        .state()
        .document
        .find_scene_object("obj_bracket")
        .expect("bracket should exist");
    assert_eq!(bracket.payload.extra["kind"], "curly");

    engine.select_at_point(Point::new(23.0, 42.0), false);
    assert!(engine.apply_line_style_to_selection("bold"));
    let line = engine
        .state()
        .document
        .find_scene_object("obj_line")
        .expect("line should exist");
    assert_eq!(line.payload.extra["arrowHead"]["bold"], true);
    let line_style = engine
        .state()
        .document
        .styles
        .get(line.style_ref.as_deref().unwrap())
        .expect("line style should exist");
    assert_eq!(line_style["strokeWidth"], 2.0);

    engine.select_at_point(Point::new(112.0, 34.0), false);
    assert!(engine.apply_text_style_to_selection("bold", "on"));
    assert!(engine.apply_text_style_to_selection("outline", "on"));
    assert!(engine.apply_text_style_to_selection("shadow", "on"));
    assert!(engine.apply_text_style_to_selection("align", "center"));
    let text = engine
        .state()
        .document
        .find_scene_object("obj_text")
        .expect("text should exist");
    assert_eq!(text.payload.extra["align"], "center");
    assert_eq!(text.payload.extra["runs"][0]["fontWeight"], 700.0);
    assert_eq!(text.payload.extra["runs"][0]["outline"], true);
    assert_eq!(text.payload.extra["runs"][0]["shadow"], true);
    assert!(text.payload.extra["runs"][0].get("face").is_none());

    engine.select_at_point(Point::new(20.0, 10.0), false);
    assert!(engine.apply_bond_style_to_selection("double-double-dashed"));
    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    let bond = fragment.bonds.iter().find(|bond| bond.id == "b1").unwrap();
    assert_eq!(bond.order, 2);
    assert_eq!(bond.line_styles.left, BondLinePattern::Dashed);
    assert_eq!(bond.line_styles.right, BondLinePattern::Dashed);

    assert!(engine.undo());
    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    let bond = fragment.bonds.iter().find(|bond| bond.id == "b1").unwrap();
    assert_eq!(bond.order, 1);
}

#[test]
fn hovered_bond_style_shortcut_updates_bond_without_changing_selection() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    click(&mut engine, FIRST_END_X, FIRST_END_Y);

    let first_bond_id = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .bonds[0]
        .id
        .clone();
    assert!(engine.select_all());
    let before_selection = engine.state().selection.clone();

    engine.set_tool_state(select_tool());
    let center = bond_center_point(&engine, &first_bond_id);
    engine.pointer_move(PointerEvent {
        x: center.x,
        y: center.y,
        button: None,
        alt_key: false,
    });

    assert!(engine.apply_hovered_bond_style("double-center"));
    assert_eq!(engine.state().selection, before_selection);

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    let bond = fragment
        .bonds
        .iter()
        .find(|bond| bond.id == first_bond_id)
        .unwrap();
    assert_eq!(bond.order, 2);
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center)
    );
}
