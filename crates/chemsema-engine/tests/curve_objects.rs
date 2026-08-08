use chemsema_engine::{parse_document_json, Engine, Point, RenderBoundsScope};

const CURVE_CDXML: &str = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="30">
  <page id="1" BoundingBox="0 0 120 80">
    <curve id="2"
      CurvePoints="5 30 10 30 20 10 40 10 50 30 60 50 80 50 90 30 95 30"
      CurveType="26"
      ArrowheadType="Solid"
      Head3D="Full"
      Tail3D="None"
      LineWidth="1"
      Z="1"/>
  </page>
</CDXML>"#;

fn curve_points(engine: &Engine) -> Vec<[f64; 2]> {
    serde_json::from_value(
        engine
            .state()
            .document
            .objects
            .iter()
            .find(|object| object.object_type == "curve")
            .expect("curve object")
            .payload
            .extra
            .get("curvePoints")
            .expect("curve points")
            .clone(),
    )
    .expect("curve points decode")
}

#[test]
fn curve_object_supports_selection_move_rotate_resize_copy_cut_paste_and_delete() {
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(CURVE_CDXML)
        .expect("curve CDXML loads");

    assert!(engine.select_all());
    assert_eq!(engine.state().selection.arrow_objects.len(), 1);
    let original_bounds = engine
        .render_bounds(RenderBoundsScope::Selection)
        .expect("curve selection bounds");

    let center = Point::new(
        (original_bounds[0] + original_bounds[2]) * 0.5,
        (original_bounds[1] + original_bounds[3]) * 0.5,
    );
    assert!(engine.begin_selection_move_at_point(center, false, false));
    assert!(engine.finish_selection_move(Point::new(center.x + 12.0, center.y + 8.0), true));
    let moved_translate = engine.state().document.objects[0].transform.translate;
    assert_eq!(moved_translate, [12.0, 8.0]);

    let moved_bounds = engine
        .render_bounds(RenderBoundsScope::Selection)
        .expect("moved curve bounds");
    let moved_center = Point::new(
        (moved_bounds[0] + moved_bounds[2]) * 0.5,
        (moved_bounds[1] + moved_bounds[3]) * 0.5,
    );
    let before_rotate = curve_points(&engine);
    assert!(engine.begin_selection_rotate(Point::new(moved_center.x, moved_bounds[1] - 12.0)));
    assert!(
        engine.finish_selection_rotate(Point::new(moved_bounds[2] + 12.0, moved_center.y), true)
    );
    assert_ne!(curve_points(&engine), before_rotate);

    let rotated_bounds = engine
        .render_bounds(RenderBoundsScope::Selection)
        .expect("rotated curve bounds");
    let before_resize = curve_points(&engine);
    assert!(engine.begin_selection_resize("se", Point::new(rotated_bounds[2], rotated_bounds[3])));
    assert!(engine.finish_selection_resize(Point::new(
        rotated_bounds[2] + 30.0,
        rotated_bounds[3] + 20.0
    )));
    assert_ne!(curve_points(&engine), before_resize);

    assert!(engine.copy_selection());
    assert!(engine.paste_clipboard());
    assert_eq!(
        engine
            .state()
            .document
            .objects
            .iter()
            .filter(|object| object.object_type == "curve")
            .count(),
        2
    );
    assert!(engine.cut_selection());
    assert_eq!(
        engine
            .state()
            .document
            .objects
            .iter()
            .filter(|object| object.object_type == "curve")
            .count(),
        1
    );
    assert!(engine.paste_clipboard());
    assert!(engine.delete_selection());
}

#[test]
fn document_json_rejects_unknown_scene_object_types() {
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(CURVE_CDXML)
        .expect("curve CDXML loads");
    let mut document: serde_json::Value =
        serde_json::from_str(&engine.document_json().expect("document JSON")).expect("JSON");
    document["entities"]["scene"][0]["type"] = serde_json::json!("mystery-object");
    let error = parse_document_json(&document.to_string()).expect_err("unknown type must fail");
    assert!(error.contains("Unsupported scene object type 'mystery-object'"));
}
