use super::*;
use serde_json::Value;

#[test]
fn cdxml_round_rectangle_radius_uses_normal_line_width_as_its_basis() {
    let source = r#"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BoundingBox="0 0 240 80" LineWidth="0.6" BoldWidth="2">
  <page id="1" BoundingBox="0 0 240 80">
    <graphic id="10" BoundingBox="10 10 54 40" GraphicType="Rectangle" RectangleType="RoundEdge" CornerRadius="600" LineWidth="0.6"/>
    <graphic id="11" BoundingBox="70 10 114 40" GraphicType="Rectangle" RectangleType="RoundEdge" CornerRadius="600" LineWidth="2"/>
    <graphic id="12" BoundingBox="130 10 174 40" GraphicType="Rectangle" RectangleType="RoundEdge Bold" CornerRadius="600"/>
    <graphic id="13" BoundingBox="190 10 234 40" GraphicType="Rectangle" RectangleType="RoundEdge" CornerRadius="0"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(source, Some("round rectangle radius"))
        .expect("round rectangle probe should import");
    let shapes = document
        .objects
        .iter()
        .filter(|object| object.object_type == "shape")
        .collect::<Vec<_>>();
    assert_eq!(shapes.len(), 4);

    let radii = shapes
        .iter()
        .map(|object| {
            object
                .payload
                .extra
                .get("cornerRadius")
                .and_then(Value::as_f64)
                .expect("round rectangle radius")
        })
        .collect::<Vec<_>>();
    assert_eq!(radii, vec![3.6, 12.0, 3.6, 3.6]);

    let bold_style = document
        .styles
        .get(shapes[2].style_ref.as_deref().expect("bold style ref"))
        .expect("bold style");
    assert_eq!(
        bold_style.get("strokeWidth").and_then(Value::as_f64),
        Some(2.0)
    );
}

#[test]
fn cdxml_round_rectangle_export_preserves_absolute_radius() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 100.0, "height": 80.0, "background": "#ffffff" }
        },
        "styles": {
            "shape": {
                "kind": "shape",
                "fill": null,
                "stroke": "#000000",
                "strokeWidth": 0.6
            }
        },
        "objects": [{
            "id": "rounded",
            "type": "shape",
            "visible": true,
            "zIndex": 1,
            "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "shape",
            "payload": {
                "kind": "roundRect",
                "bbox": [0.0, 0.0, 44.0, 30.0],
                "cornerRadius": 6.0
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("CornerRadius=\"1000\""));
    let reopened = parse_cdxml_document(&exported, Some("round rectangle reopened"))
        .expect("exported round rectangle should reimport");
    let radius = reopened.objects[0]
        .payload
        .extra
        .get("cornerRadius")
        .and_then(Value::as_f64);
    assert_eq!(radius, Some(6.0));
}

#[test]
fn round_rectangle_clamps_corner_axes_independently() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 80.0, "height": 60.0, "background": "#ffffff" }
        },
        "styles": {
            "shape": { "kind": "shape", "stroke": "#000000", "strokeWidth": 1.0 }
        },
        "objects": [{
            "id": "rounded",
            "type": "shape",
            "visible": true,
            "zIndex": 1,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "shape",
            "payload": {
                "kind": "roundRect",
                "bbox": [0.0, 0.0, 44.0, 30.0],
                "cornerRadius": 16.0
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let path = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Path { d, .. } => Some(d),
            _ => None,
        })
        .expect("round rectangle outline");
    assert!(path.starts_with("M 0,15 "), "unexpected path: {path}");
    assert!(path.contains(" 16,0 "), "unexpected path: {path}");
    assert!(path.contains(" 28,0 "), "unexpected path: {path}");
}

#[test]
fn render_document_emits_shape_rect_with_style() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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

    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
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
                role: RenderRole::DocumentText,
                font_size,
                ..
            } => Some(font_size),
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
                    role: RenderRole::DocumentDiagnostic,
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
            node_id: Some(node_id),
            stroke_width,
            ..
        } if node_id == "n1" && (stroke_width - 0.5).abs() < 1.0e-6
    ));
}

#[test]
fn export_svg_omits_invalid_label_diagnostics() {
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
                        "labelRecognition": { "status": "invalid" }
                    }
                }
            }
        ]),
        json!([]),
    );

    let primitives = render_document(&document);
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::DocumentDiagnostic,
            stroke: Some(stroke),
            ..
        } if stroke == "#d32f2f"
    )));

    let svg = document_to_svg(&document);
    assert!(svg.contains(">N</"));
    assert!(!svg.contains("#d32f2f"));
}

#[test]
fn render_document_scales_small_bracket_geometry_without_fixed_minimums() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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
fn render_document_uses_derived_label_clip_geometry_for_knockout_and_endpoint_clipping() {
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
                    "position": [18.0, 23.9],
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
    let document = normalize_test_document(&document);

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
        start_x > 25.0 && start_x < 40.0,
        "endpoint should clip at the derived Arial outline retreat rather than the stale imported glyph rectangles or the node center: {centerlines:?}"
    );
}

#[test]
fn render_document_emits_primitives_for_legacy_molblock_resource() {
    let molblock = concat!(
        "Legacy\n",
        "  ChemSema\n",
        "\n",
        "  2  1  0  0  0  0            999 V2000\n",
        "    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n",
        "    1.2000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n",
        "  1  2  1  0  0  0  0\n",
        "M  END\n"
    );
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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
fn render_document_uses_one_label_cut_plane_for_all_triple_bond_lanes() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 20.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [45.9808, 35.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [42.0, 40.0],
                    "box": [42.0, 30.0, 49.0, 40.0],
                    "glyphPolygons": [[
                        [42.0, 30.0], [49.0, 30.0], [49.0, 40.0], [44.0, 40.0]
                    ]]
                }
            }
        ]),
        json!([{
            "id": "b1",
            "begin": "n1",
            "end": "n2",
            "order": 3,
            "strokeWidth": 1.0
        }]),
    );
    let centerlines = object_bond_centerlines(&render_document(&document));

    assert_eq!(centerlines.len(), 3, "{centerlines:?}");
    let radians = 30_f64.to_radians();
    let axis = (radians.cos(), radians.sin());
    let label_side_projections = centerlines
        .iter()
        .map(|(from, to)| {
            let from_projection = from.x * axis.0 + from.y * axis.1;
            let to_projection = to.x * axis.0 + to.y * axis.1;
            from_projection.max(to_projection)
        })
        .collect::<Vec<_>>();
    let spread = label_side_projections
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        - label_side_projections
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
    assert!(spread < 0.001, "{label_side_projections:?}");
}

#[test]
fn triple_bond_shared_label_cut_plane_does_not_expand_with_line_width() {
    let nodes = json!([
        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 20.0], "charge": 0, "numHydrogens": 0 },
        {
            "id": "n2",
            "element": "N",
            "atomicNumber": 7,
            "position": [45.9808, 35.0],
            "charge": 0,
            "numHydrogens": 0,
            "label": {
                "text": "N",
                "position": [42.0, 40.0],
                "box": [42.0, 30.0, 49.0, 40.0],
                "glyphPolygons": [[
                    [42.0, 30.0], [49.0, 30.0], [49.0, 40.0], [44.0, 40.0]
                ]]
            }
        }
    ]);
    let radians = 30_f64.to_radians();
    let axis = (radians.cos(), radians.sin());
    let label_projection_for_width = |stroke_width| {
        let document = fragment_document(
            nodes.clone(),
            json!([{
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 3,
                "strokeWidth": stroke_width
            }]),
        );
        object_bond_centerlines(&render_document(&document))
            .iter()
            .map(|(from, to)| {
                (from.x * axis.0 + from.y * axis.1).max(to.x * axis.0 + to.y * axis.1)
            })
            .fold(f64::NEG_INFINITY, f64::max)
    };

    let narrow = label_projection_for_width(0.2);
    let wide = label_projection_for_width(4.0);
    assert!((narrow - wide).abs() < 0.0001, "{narrow} != {wide}");
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

    assert_eq!(polygons.len(), 14);
    assert!(polygons.iter().all(|points| points.len() == 4));
    assert!(knockouts.is_empty(), "{knockouts:?}");
    let lengths: Vec<_> = polygons
        .iter()
        .filter_map(|points| bond_axis_length(points))
        .collect();
    assert!(
        lengths
            .iter()
            .all(|length| (*length - 36.0 / 13.0).abs() < 0.01),
        "{lengths:?}"
    );
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
    let polygons = centered_bond_polygons(&primitives, chemsema_engine::Point::new(56.0, 40.0));
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
    let polygons = centered_bond_polygons(&primitives, chemsema_engine::Point::new(56.0, 40.0));
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

    assert_eq!(polygons.len(), 6);
    assert!(polygons.iter().all(|points| points.len() == 4));
    assert!(knockouts.is_empty(), "{knockouts:?}");
    let black_segments: Vec<_> = polygons
        .iter()
        .filter_map(|points| bond_axis_length(points))
        .collect();
    let first_black = black_segments[0];
    assert!(
        black_segments
            .iter()
            .all(|length| (length - first_black).abs() < 0.02),
        "{black_segments:?}"
    );
    assert!(
        (first_black - 30.0 / 11.0).abs() < 0.02,
        "{black_segments:?}"
    );
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
    let polygons = centered_bond_polygons(&primitives, chemsema_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 2);
    assert!(
        polygons.iter().all(|points| points.len() == 5),
        "the dashed terminal stripe must absorb its contact profile: {polygons:?}"
    );
    assert!(object_knockout_polygons(&primitives).is_empty());
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
fn render_document_matches_chemdraw_hashed_wedge_count_thresholds() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [10.0, 20.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [24.49, 20.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [10.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [24.50, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 1.0,
                "hashSpacing": 2.7,
                "stereo": { "kind": "hashed-wedge", "wideEnd": "end" }
            },
            {
                "id": "b2",
                "begin": "n3",
                "end": "n4",
                "order": 1,
                "strokeWidth": 1.0,
                "hashSpacing": 2.7,
                "stereo": { "kind": "hashed-wedge", "wideEnd": "end" }
            }
        ]),
    );

    let polygons = object_bond_polygons_with_ids(&render_document(&document));
    assert_eq!(
        polygons
            .iter()
            .filter(|(bond_id, _)| bond_id == "b1")
            .count(),
        5
    );
    assert_eq!(
        polygons
            .iter()
            .filter(|(bond_id, _)| bond_id == "b2")
            .count(),
        6
    );
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
    let polygons = centered_bond_polygons(&primitives, chemsema_engine::Point::new(56.0, 40.0));
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
            .filter(|point| point.distance(chemsema_engine::Point::new(56.0, 40.0)) <= 4.0)
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
        closest_points_to_target(&hashed_wedge, chemsema_engine::Point::new(56.0, 40.0), 2);

    assert!(
        connected_end.iter().all(|point| point.x < 55.0),
        "{hashed_wedge:?}"
    );
    assert!(
        average_closest_distance_to_point(&branch, chemsema_engine::Point::new(56.0, 40.0), 2)
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
        closest_points_to_target(&hashed_wedge, chemsema_engine::Point::new(20.0, 40.0), 2);
    let wide_end =
        closest_points_to_target(&hashed_wedge, chemsema_engine::Point::new(56.0, 40.0), 2);

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
                    "main": "hash",
                    "left": "solid",
                    "right": "solid"
                },
                "lineWeights": {
                    "main": "normal",
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
    let hash_bond = object_bond_points_for_id(&connected_primitives, "b1");
    let branch = connected_polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b2").then_some(points.clone()))
        .expect("branch polygon");
    let connected_end =
        closest_points_to_target(&hash_bond, chemsema_engine::Point::new(56.0, 40.0), 2);

    assert!(!hash_bond.is_empty(), "hash bond segments");
    assert!(
        connected_end.iter().all(|point| point.x < 55.0),
        "{hash_bond:?}"
    );
    assert!(
        average_closest_distance_to_point(&branch, chemsema_engine::Point::new(56.0, 40.0), 2)
            < 0.6,
        "{branch:?}"
    );
    let knockouts = object_knockout_polygons(&connected_primitives);
    assert!(knockouts.is_empty(), "{knockouts:?}");
}
