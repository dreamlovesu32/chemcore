use super::*;

#[test]
fn parse_cdxml_imports_bezier_curve_flags_and_arrowheads() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" HashSpacing="2.50">
  <colortable><color r="1" g="1" b="1"/><color r="1" g="0" b="0"/></colortable>
  <page id="1">
    <curve id="2" Z="7" color="3" CurveType="26" ArrowheadType="Solid"
      CurvePoints="5 30 10 30 20 10 40 10 50 30 60 50 80 50 90 30 95 30"/>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("bezier curve")).expect("parse cdxml");
    let curve = document
        .objects
        .iter()
        .find(|object| object.object_type == "curve")
        .expect("curve should import");
    assert_eq!(curve.payload.extra["curveType"], json!(26));
    assert_eq!(curve.payload.extra["head"], json!("full"));
    assert_eq!(curve.payload.extra["tail"], json!("full"));
    assert_eq!(curve.payload.extra["closed"], json!(false));
    let style = document
        .styles
        .get(curve.style_ref.as_deref().expect("curve style"))
        .expect("curve style should exist");
    assert_eq!(style["dashArray"], json!([2.5]));
    let primitives = render_document(&document);
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            d,
            dash_array,
            ..
        } if d.contains(" C ") && !dash_array.is_empty()
    )));
    assert_eq!(
        primitives
            .iter()
            .filter(|primitive| matches!(primitive, RenderPrimitive::FilledPath { .. }))
            .count(),
        2
    );
    let exported = document_to_cdxml(&document);
    assert!(exported.contains("<curve "), "{exported}");
    assert!(exported.contains("CurveType=\"26\""), "{exported}");
    assert!(exported.contains("ArrowheadHead=\"Full\""), "{exported}");
    assert!(exported.contains("ArrowheadTail=\"Full\""), "{exported}");
    let reparsed = parse_cdxml_document(&exported, Some("curve roundtrip"))
        .expect("exported curve should parse");
    assert!(reparsed
        .objects
        .iter()
        .any(|object| object.object_type == "curve"));
}

#[test]
fn render_cdxml_closed_curve_uses_outer_guides_for_the_closing_cubic() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LineWidth="1">
  <page id="1">
    <curve id="2" CurveType="1"
      CurvePoints="0 0 10 10 20 10 30 20 40 10 50 0"/>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("closed bezier curve"))
        .expect("closed curve should parse");
    let (path, points) = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Path {
                role: RenderRole::DocumentGraphic,
                d,
                points,
                ..
            } => Some((d, points)),
            _ => None,
        })
        .expect("closed curve path");

    assert_eq!(
        path,
        "M 10.0000 10.0000 C 20.0000 10.0000 30.0000 20.0000 40.0000 10.0000 C 50.0000 0.0000 0.0000 0.0000 10.0000 10.0000"
    );
    assert!(!path.contains(" Z"));
    assert!(points.contains(&Point::new(50.0, 0.0)));
    assert!(points.contains(&Point::new(0.0, 0.0)));
}

#[test]
fn render_cdxml_curve_keeps_half_arrow_side_at_both_endpoints() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LineWidth="1">
  <page id="1">
    <curve id="2" ArrowheadHead="HalfLeft" ArrowheadTail="HalfRight"
      HeadSize="1000" ArrowheadCenterSize="875" ArrowheadWidth="250"
      CurvePoints="-10 0 0 0 10 0 50 0 50 0 60 0"/>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("directional curve half arrows")).expect("parse curve");
    let curve = document
        .objects
        .iter()
        .find(|object| object.object_type == "curve")
        .expect("curve object");
    assert_eq!(curve.payload.extra["head"], json!("half-left"));
    assert_eq!(curve.payload.extra["tail"], json!("half-right"));
    let exported = document_to_cdxml(&document);
    assert!(
        exported.contains("ArrowheadHead=\"HalfLeft\""),
        "{exported}"
    );
    assert!(
        exported.contains("ArrowheadTail=\"HalfRight\""),
        "{exported}"
    );

    let arrowheads: Vec<_> = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::FilledPath { points, .. } => Some(points),
            _ => None,
        })
        .collect();
    assert_eq!(arrowheads.len(), 2);
    let head_outer = arrowheads[0][1];
    let tail_outer = arrowheads[1][1];
    assert!(head_outer.y < 0.0, "HalfLeft head outer={head_outer:?}");
    assert!(tail_outer.y < 0.0, "HalfRight tail outer={tail_outer:?}");
}

#[test]
fn render_cdxml_curve_default_arrow_dimensions_scale_with_effective_line_width() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LineWidth="1">
  <page id="1">
    <curve id="2" LineWidth="1" ArrowheadHead="Full"
      CurvePoints="-10 0 0 0 10 0 40 0 50 0 60 0"/>
    <curve id="3" LineWidth="1.77" ArrowheadHead="Full"
      CurvePoints="-10 30 0 30 10 30 40 30 50 30 60 30"/>
    <curve id="4" LineWidth="1.77" ArrowheadHead="Full"
      HeadSize="1000" ArrowheadCenterSize="875" ArrowheadWidth="250"
      CurvePoints="-10 60 0 60 10 60 40 60 50 60 60 60"/>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("curve default arrow dimensions")).expect("parse curves");
    let curves: Vec<_> = document
        .objects
        .iter()
        .filter(|object| object.object_type == "curve")
        .collect();
    assert_eq!(curves.len(), 3);
    assert_eq!(curves[0].payload.extra["headLength"], json!(8.0));
    assert_eq!(curves[0].payload.extra["headCenterLength"], json!(6.75));
    assert_eq!(curves[0].payload.extra["headWidth"], json!(2.5));
    assert_eq!(curves[1].payload.extra["headLength"], json!(14.16));
    assert_eq!(curves[1].payload.extra["headCenterLength"], json!(11.9475));
    assert_eq!(curves[1].payload.extra["headWidth"], json!(4.425));
    assert_eq!(curves[2].payload.extra["headLength"], json!(10.0));

    let arrowheads: Vec<_> = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::FilledPath { points, .. } => Some(points),
            _ => None,
        })
        .collect();
    assert_eq!(arrowheads.len(), 3);
    let lengths: Vec<_> = arrowheads
        .iter()
        .map(|points| {
            let min_x = points
                .iter()
                .map(|point| point.x)
                .fold(f64::INFINITY, f64::min);
            let max_x = points
                .iter()
                .map(|point| point.x)
                .fold(f64::NEG_INFINITY, f64::max);
            max_x - min_x
        })
        .collect();
    assert_close(lengths[0], 8.0);
    assert_close(lengths[1], 14.16);
    assert_close(lengths[2], 10.0);
    assert_point_close(arrowheads[0][1], Point::new(50.0, 0.0));
    assert_point_close(arrowheads[0][3], Point::new(56.75, 0.0));
}

#[test]
fn parse_cdxml_curve_type_half_bits_normalize_to_half_left() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML>
  <page id="1">
    <curve id="2" CurveType="32"
      CurvePoints="-10 0 0 0 10 0 40 0 50 0 60 0"/>
    <curve id="3" CurveType="64"
      CurvePoints="-10 30 0 30 10 30 40 30 50 30 60 30"/>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("curve type half bits")).expect("parse curves");
    let curves: Vec<_> = document
        .objects
        .iter()
        .filter(|object| object.object_type == "curve")
        .collect();
    assert_eq!(curves[0].payload.extra["head"], json!("half-left"));
    assert_eq!(curves[0].payload.extra["tail"], json!("none"));
    assert_eq!(curves[1].payload.extra["head"], json!("none"));
    assert_eq!(curves[1].payload.extra["tail"], json!("half-left"));
}
