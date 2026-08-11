use super::*;

#[test]
fn cdxml_imported_f_label_margin_expands_internal_bar_clip() {
    fn imported_f_endpoint_distance(margin_width: f64) -> (f64, f64, usize) {
        let cdxml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" BondSpacing="18" MarginWidth="{margin_width:.2}" LabelSize="10">
  <page id="p1" BoundingBox="0 0 70 40">
    <fragment id="f1" BoundingBox="0 0 70 40">
      <n id="n13" p="37.41 12.17"/>
      <n id="n14" p="24.94 4.97" Element="9" InterpretChemically="yes">
        <t p="21.91 8.87" BoundingBox="21.91 0.67 28.01 9.57" LabelJustification="Left" LabelAlignment="Auto">
          <s font="3" size="10" color="0" face="96">F</s>
        </t>
      </n>
      <b id="b28" B="n13" E="n14"/>
    </fragment>
  </page>
</CDXML>"#
        );
        let document =
            parse_cdxml_document(&cdxml, Some("imported F clip")).expect("cdxml should parse");
        let entry = document.editable_fragment().expect("editable fragment");
        let f_node = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == "n14")
            .expect("F node should import");
        let label = f_node.label.as_ref().expect("F label should import");
        let import_meta = label
            .meta
            .pointer("/import/cdxml")
            .expect("CDXML label import metadata should exist");
        assert_eq!(
            import_meta
                .get("naturalOutsetPt")
                .and_then(|value| value.as_f64()),
            Some(margin_width)
        );
        assert!(
            !label.glyph_polygons.is_empty(),
            "imported F label should carry glyph clip polygons"
        );

        let f_world = Point::new(
            entry.object.transform.translate[0] + f_node.position[0],
            entry.object.transform.translate[1] + f_node.position[1],
        );
        let polygon = render_document(&document)
            .into_iter()
            .find_map(|primitive| match primitive {
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond,
                    object_id,
                    bond_id,
                    points,
                    ..
                } if object_id.as_deref() == Some("obj_mol_001")
                    && bond_id.as_deref() == Some("b28") =>
                {
                    Some(points)
                }
                _ => None,
            })
            .expect("F bond polygon should render");
        let (from, to) = bond_axis_from_points(&polygon).expect("bond axis");
        let label_endpoint = if from.distance(f_world) < to.distance(f_world) {
            from
        } else {
            to
        };
        (
            label_endpoint.distance(f_world),
            import_meta
                .get("marginWidth")
                .and_then(|value| value.as_f64())
                .unwrap_or_default(),
            label.glyph_clip_polygons.len(),
        )
    }

    let (one_pt_distance, one_pt_margin, one_pt_points) = imported_f_endpoint_distance(1.0);
    let (two_pt_distance, two_pt_margin, two_pt_points) = imported_f_endpoint_distance(2.0);

    assert_eq!(one_pt_margin, 1.0);
    assert_eq!(two_pt_margin, 2.0);
    assert!(one_pt_points > 8);
    assert!(two_pt_points > 8);
    assert!(
        two_pt_distance > one_pt_distance + 0.45,
        "imported F internal-bar clipping must expand with MarginWidth: {one_pt_distance} -> {two_pt_distance}"
    );
}

#[test]
fn render_document_does_not_join_bold_bond_at_labeled_endpoint() {
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
            },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [62.0, 28.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineWeights": { "main": "bold" }
            },
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let polygon = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("bold bond polygon should render");
    let max_x = polygon
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);

    assert!(
        max_x <= 51.0 + 1.0e-6,
        "labeled endpoints should be clipped by glyphs and must not rejoin at the atom point: {polygon:?}"
    );
}

#[test]
fn render_document_retreats_bond_when_label_anchor_lies_on_glyph_boundary() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [50.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [50.0, 45.0],
                    "box": [50.0, 34.0, 60.0, 46.0],
                    "glyphPolygons": [[
                        [50.0, 34.0],
                        [60.0, 34.0],
                        [60.0, 46.0],
                        [50.0, 46.0]
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
    let max_x = polygon
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);

    assert!(
        max_x <= 50.0 + 1.0e-6,
        "a bond whose atom anchor lies on a glyph edge should still retreat outside the glyph: {polygon:?}"
    );
}

#[test]
fn render_document_allows_bond_between_close_labels_to_disappear() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "N",
                "atomicNumber": 7,
                "position": [50.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [50.0, 45.0],
                    "box": [45.0, 34.0, 55.0, 46.0],
                    "glyphPolygons": [[
                        [45.0, 34.0],
                        [55.0, 34.0],
                        [55.0, 46.0],
                        [45.0, 46.0]
                    ]]
                }
            },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [60.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [60.0, 45.0],
                    "box": [55.0, 34.0, 65.0, 46.0],
                    "glyphPolygons": [[
                        [55.0, 34.0],
                        [65.0, 34.0],
                        [65.0, 46.0],
                        [55.0, 46.0]
                    ]]
                }
            }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    assert!(
        !object_bond_polygons_with_ids(&render_document(&document))
            .iter()
            .any(|(bond_id, _)| bond_id == "b1"),
        "when label glyph retreats consume the whole segment, the bond should disappear instead of preserving a minimum visible length"
    );
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
    assert!(arrow_document.objects.iter().any(
        |object| object.object_type == "line" && object.payload.extra.contains_key("arrowHead")
    ));
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
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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
fn parse_cdxml_maps_legacy_and_modern_arrow_types_without_losing_endpoints() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="0.6" BondLength="14.4">
  <page id="1" BoundingBox="0 0 180 220">
    <graphic id="10" GraphicType="Line" ArrowType="FullHead" HeadSize="1000" BoundingBox="150 20 10 20"/>
    <graphic id="11" GraphicType="Line" ArrowType="HalfHead" HeadSize="1000" BoundingBox="150 40 10 40"/>
    <graphic id="12" GraphicType="Line" ArrowType="HalfHead" HeadSize="-1000" BoundingBox="150 60 10 60"/>
    <graphic id="13" GraphicType="Line" ArrowType="Resonance" BoundingBox="150 80 10 80"/>
    <graphic id="14" GraphicType="Line" ArrowType="Equilibrium" BoundingBox="150 100 10 100"/>
    <graphic id="15" GraphicType="Line" ArrowType="Hollow" BoundingBox="150 120 10 120"/>
    <graphic id="16" GraphicType="Line" ArrowType="RetroSynthetic" BoundingBox="150 140 10 140"/>
    <graphic id="17" GraphicType="Line" ArrowType="FullHead NoGo Dipole" BoundingBox="150 160 10 160"/>
    <arrow id="18" ArrowheadHead="Full" ArrowheadTail="HalfRight" ArrowheadType="Solid"
      HeadSize="1250" ArrowheadCenterSize="1100" ArrowheadWidth="325"
      AngularSize="90" CurveSpacing="450" NoGo="Hash" Dipole="yes" Closed="yes"
      ArrowSource="12" ArrowTarget="34" Head3D="150 190 0" Tail3D="10 190 0"/>
    <graphic id="19" GraphicType="Arc" ArrowType="FullHead" HeadSize="1000"
      AngularSize="90" BoundingBox="129.5 290.5 80 340"/>
    <arrow id="20" ArrowType="Equilibrium" ArrowheadHead="Full" ArrowheadType="Hollow"
      Head3D="150 215 0" Tail3D="10 215 0"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("arrow matrix")).expect("arrows should parse");
    let arrows = document
        .objects
        .iter()
        .filter_map(|object| object.payload.extra.get("arrowHead"))
        .collect::<Vec<_>>();
    assert_eq!(arrows.len(), 11);

    let endpoint =
        |index: usize, key: &str| arrows[index].get(key).and_then(|value| value.as_str());
    let kind = |index: usize| endpoint(index, "kind");
    assert_eq!(endpoint(0, "head"), Some("full"));
    assert_eq!(endpoint(1, "head"), Some("half-left"));
    assert_eq!(endpoint(2, "head"), Some("half-right"));
    assert_eq!(
        (endpoint(3, "head"), endpoint(3, "tail")),
        (Some("full"), Some("full"))
    );
    assert_eq!(
        (endpoint(4, "head"), endpoint(4, "tail")),
        (Some("half-left"), Some("half-left"))
    );
    assert_eq!(kind(4), Some("equilibrium"));
    assert_eq!(kind(5), Some("hollow"));
    assert_eq!(kind(6), Some("open"));
    assert_eq!(
        arrows[7].get("noGo").and_then(|value| value.as_str()),
        Some("cross")
    );
    assert_eq!(
        arrows[7].get("dipole").and_then(|value| value.as_bool()),
        Some(true)
    );

    assert_eq!(
        (endpoint(8, "head"), endpoint(8, "tail")),
        (Some("full"), Some("half-right"))
    );
    assert_eq!(
        arrows[8].get("curve").and_then(|value| value.as_f64()),
        Some(90.0)
    );
    assert_eq!(
        arrows[8]
            .get("curveSpacing")
            .and_then(|value| value.as_f64()),
        Some(4.5)
    );
    assert_eq!(
        arrows[8].get("noGo").and_then(|value| value.as_str()),
        Some("hash")
    );
    assert_eq!(
        arrows[8].get("dipole").and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        arrows[8].get("closed").and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        arrows[8].get("source").and_then(|value| value.as_str()),
        Some("12")
    );
    assert_eq!(
        arrows[8].get("target").and_then(|value| value.as_str()),
        Some("34")
    );

    let arc = document
        .objects
        .iter()
        .find(|object| {
            object
                .meta
                .pointer("/graphicId")
                .and_then(|value| value.as_str())
                == Some("19")
        })
        .expect("legacy arc should import");
    assert_eq!(
        arc.payload.extra.get("points"),
        Some(&json!([[129.5, 389.5], [129.5, 290.5]]))
    );
    assert_eq!(
        arc.payload
            .extra
            .get("arrowGeometry")
            .and_then(|value| value.get("center")),
        Some(&json!([80.0, 340.0]))
    );
    assert_eq!(endpoint(10, "head"), Some("full"));
    assert_eq!(endpoint(10, "tail"), Some("none"));
    assert_eq!(kind(10), Some("hollow"));

    let exported = document_to_cdxml(&document);
    let reopened = parse_cdxml_document(&exported, Some("arrow matrix reopened"))
        .expect("arrows should reopen");
    assert_eq!(
        reopened
            .objects
            .iter()
            .filter(|object| object.object_type == "line")
            .count(),
        11,
        "legacy Graphic arrows must be replaced rather than retained beside native Arrow objects"
    );
}

#[test]
fn modern_arrow_wins_over_same_identity_legacy_graphic_representation() {
    let cdxml = r#"<CDXML LineWidth="0.6" BondLength="14.4">
  <page id="1">
    <graphic id="10" GraphicType="Line" ArrowType="FullHead"
      HeadSize="1000" BoundingBox="150 20 10 20"/>
    <arrow id="10" ArrowheadHead="Full" ArrowheadType="Solid"
      HeadSize="1000" Head3D="150 20 0" Tail3D="10 20 0"/>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("dual arrow representation")).expect("arrow should parse");
    assert_eq!(
        document
            .objects
            .iter()
            .filter(|object| object.object_type == "line")
            .count(),
        1
    );
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
    assert_eq!(
        arrow_head.get("head").and_then(|value| value.as_str()),
        Some("full")
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
    let notch = head_points[3];
    let left_control = head_points[2];
    assert!((left_control.x - notch.x).abs() <= 0.001);
    assert!((left_control.y - notch.y - 1.47675).abs() <= 0.001);
}

#[test]
fn cdxml_dipole_arrow_renders_chemdraw_tail_bar_geometry() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BondLength="14.4">
  <page id="1" BoundingBox="0 0 180 60">
    <arrow id="2" Tail3D="10 30 0" Head3D="150 30 0" ArrowheadHead="Full"
      ArrowheadType="Solid" HeadSize="1000" ArrowheadCenterSize="875"
      ArrowheadWidth="250" Dipole="yes"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("dipole arrow")).expect("arrow should parse");
    let primitives = render_document(&document);
    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Polyline { points, stroke_width, .. }
                if points.len() == 2
                    && (*stroke_width - 1.0).abs() <= 0.001
                    && (points[0].x - 12.5).abs() <= 0.001
                    && (points[1].x - 12.5).abs() <= 0.001
                    && (points[0].y - 25.0).abs() <= 0.001
                    && (points[1].y - 35.0).abs() <= 0.001
        )),
        "dipole bar should be offset by head width and span one head length: {primitives:?}"
    );
}

#[test]
fn cdxml_grouped_arrow_keeps_renderable_head_payload() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML LineWidth="0.6" BondLength="14.4" color="0" bgcolor="1">
  <page id="1" BoundingBox="0 0 180 80">
    <group id="group1">
      <graphic id="g1" SupersededBy="a1" GraphicType="Line" ArrowType="FullHead"
        HeadSize="2250" Head3D="150 40 0" Tail3D="20 40 0"/>
      <arrow id="a1" BoundingBox="20 36 150 44" Z="1" FillType="None"
        ArrowheadType="Solid" ArrowheadHead="Full" HeadSize="2250"
        ArrowheadCenterSize="1969" ArrowheadWidth="563"
        Head3D="150 40 0" Tail3D="20 40 0"/>
    </group>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("grouped arrow")).expect("cdxml should parse");
    let primitives = render_document(&document);
    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::FilledPath {
                role: RenderRole::DocumentGraphic,
                object_id,
                ..
            } if object_id.as_deref() == Some("obj_line_001")
        )),
        "grouped arrow should render a filled arrow head: {primitives:?}"
    );
}

#[test]
fn cdxml_arrow_line_width_scales_arrow_head_ratios() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML LineWidth="0.6" BondLength="14.4" color="0" bgcolor="1">
  <page id="1" BoundingBox="0 0 120 40">
    <arrow id="2" Head3D="100 20 0" Tail3D="0 20 0" Z="1"
      LineWidth="1.19" FillType="None" ArrowheadType="Solid" ArrowheadHead="Full"
      HeadSize="800" ArrowheadCenterSize="700" ArrowheadWidth="200"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("wide arrow")).expect("cdxml should parse");
    let arrow = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow should import as line object");
    let style = arrow
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .expect("arrow should use a line-width-specific style");
    assert_eq!(
        style.get("strokeWidth").and_then(|value| value.as_f64()),
        Some(1.19)
    );
    let arrow_head = arrow
        .payload
        .extra
        .get("arrowHead")
        .expect("arrow should keep cdxml arrow payload");
    assert_eq!(
        arrow_head.get("length").and_then(|value| value.as_f64()),
        Some(8.0)
    );
    assert_eq!(
        arrow_head.get("width").and_then(|value| value.as_f64()),
        Some(2.0)
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
    assert!((head_max_x - head_min_x - 9.52).abs() <= 0.001);
    assert!((head_max_y - head_min_y - 4.86).abs() <= 0.001);
}

#[test]
fn cdxml_arrow_type_without_endpoint_does_not_enable_head() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML LineWidth="0.6" BondLength="14.4" color="0" bgcolor="1">
  <page id="1" BoundingBox="0 0 160 80">
    <arrow id="2" Head3D="128.21 40 0" Tail3D="0 40 0" Z="1"
      FillType="None" ArrowheadType="Solid"
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
        arrow_head.get("head").and_then(|value| value.as_str()),
        Some("none")
    );

    let primitives = render_document(&document);
    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Polyline {
                role: RenderRole::DocumentGraphic,
                object_id,
                ..
            } if object_id.as_deref() == Some(arrow.id.as_str())
        )),
        "arrow element should still render its shaft"
    );
    assert!(
        !primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::FilledPath {
                role: RenderRole::DocumentGraphic,
                object_id,
                ..
            } if object_id.as_deref() == Some(arrow.id.as_str())
        )),
        "ArrowheadType alone describes the head kind, not an enabled endpoint"
    );
}

#[test]
fn cdxml_bold_line_uses_imported_bold_width_without_render_floor() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML LineWidth="0.57" BoldWidth="1.91" BondLength="13.78" color="0" bgcolor="1">
  <page id="1" BoundingBox="0 0 360 80">
    <arrow id="2" Head3D="340 40 0" Tail3D="20 40 0" Z="1"
      LineType="Bold" FillType="None" ArrowheadType="Solid"
      HeadSize="2000" ArrowheadCenterSize="1750" ArrowheadWidth="500"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("bold line")).expect("cdxml should parse");
    let line = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("bold line should import as line object");
    let style = line
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .expect("line should use imported bold-width style");
    assert_eq!(
        style.get("strokeWidth").and_then(|value| value.as_f64()),
        Some(1.91)
    );

    let primitives = render_document(&document);
    let shaft_width = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polyline {
                role,
                object_id,
                stroke_width,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some(line.id.as_str()) =>
            {
                Some(*stroke_width)
            }
            _ => None,
        })
        .expect("bold line shaft should render");
    assert!((shaft_width - 1.91).abs() <= 0.001);
    assert!(
        !primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::FilledPath {
                role: RenderRole::DocumentGraphic,
                object_id,
                ..
            } if object_id.as_deref() == Some(line.id.as_str())
        )),
        "line should not gain an arrowhead without ArrowheadHead or ArrowheadTail"
    );
}

#[test]
fn cdxml_bold_arrow_head_dimensions_scale_with_imported_line_width() {
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
    assert!((head_max_x - head_min_x - 90.0).abs() <= 0.001);
    assert!((head_max_y - head_min_y - 45.1).abs() <= 0.001);
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
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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
                    "noGo": "hash",
                    "curveSpacing": 4.5,
                    "dipole": true,
                    "closed": true,
                    "source": "12",
                    "target": 34
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
    assert!(cdxml.contains("CurveSpacing=\"450\""));
    assert!(cdxml.contains("Dipole=\"yes\""));
    assert!(cdxml.contains("Closed=\"yes\""));
    assert!(cdxml.contains("ArrowSource=\"12\""));
    assert!(cdxml.contains("ArrowTarget=\"34\""));
    assert!(cdxml.contains("LineType=\"Bold Dashed\""));
    assert!(cdxml.contains("FillType=\"None\""));
    assert!(cdxml.contains("Center3D=\"65 20 0\""));
    assert!(cdxml.contains("MajorAxisEnd3D=\"120 20 0\""));
    assert!(cdxml.contains("MinorAxisEnd3D=\"65 75 0\""));
}

#[test]
fn render_document_uses_arrow_geometry_for_elliptic_curved_arrows() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
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
            "format": { "name": "chemsema", "version": "0.1" },
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
            "format": { "name": "chemsema", "version": "0.1" },
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
    assert!(text.payload.extra.contains_key("lineHeight"));
    assert!(text.payload.extra.contains_key("box"));
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

    assert_eq!(line_heights, vec![11.5, 13.45, 13.45]);
}

#[test]
fn parse_cdxml_auto_line_height_uses_the_tallest_styled_run() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML>
  <page id="1">
    <t id="mixed" p="10 20" BoundingBox="10 10 100 70" CaptionLineHeight="auto">
      <s font="3" size="8" face="0" color="0">small
</s><s font="3" size="18" face="0" color="0">large
</s><s font="3" size="12" face="0" color="0">medium</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("mixed auto line height"))
        .expect("text cdxml should parse");
    let text = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should import");
    assert_eq!(
        text.payload
            .extra
            .get("lineHeight")
            .and_then(|value| value.as_f64()),
        Some(20.7),
        "ChemDraw Auto uses the tallest styled run anywhere in the object"
    );
    assert_eq!(
        text.payload
            .extra
            .get("lineHeightMode")
            .and_then(|value| value.as_str()),
        Some("auto")
    );

    let baselines = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text {
                object_id: Some(object_id),
                y,
                ..
            } if object_id == text.id => Some(y),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(baselines.len(), 3);
    assert!((baselines[1] - baselines[0] - 20.7).abs() < 0.01);
    assert!((baselines[2] - baselines[1] - 20.7).abs() < 0.01);
    assert!(document_to_cdxml(&document).contains("CaptionLineHeight=\"auto\""));
}

#[test]
fn parse_cdxml_variable_line_height_keeps_per_transition_advances() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML>
  <page id="1">
    <t id="variable" p="10 20" BoundingBox="10 10 100 70" CaptionLineHeight="variable">
      <s font="3" size="10" face="0" color="0">g
A
Q</s>
    </t>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("variable line height")).expect("text cdxml should parse");
    let text = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should import");
    let advances = text
        .payload
        .extra
        .get("lineAdvances")
        .and_then(|value| value.as_array())
        .expect("variable line height should retain each transition")
        .iter()
        .filter_map(|value| value.as_f64())
        .collect::<Vec<_>>();
    assert_eq!(
        text.payload
            .extra
            .get("lineHeightMode")
            .and_then(|value| value.as_str()),
        Some("variable")
    );
    assert_eq!(advances.len(), 2);
    assert_ne!(advances[0], advances[1]);

    let baselines = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text {
                object_id: Some(object_id),
                y,
                ..
            } if object_id == text.id => Some(y),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(baselines.len(), 3);
    assert!((baselines[1] - baselines[0] - advances[0]).abs() < 0.01);
    assert!((baselines[2] - baselines[1] - advances[1]).abs() < 0.01);
    assert!(document_to_cdxml(&document).contains("CaptionLineHeight=\"variable\""));
}

#[test]
fn parse_cdxml_root_text_styles_keep_resolved_line_height_semantics() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="3" LabelSize="7" LabelLineHeight="6"
       CaptionFont="3" CaptionSize="7" CaptionLineHeight="7.1">
  <fonttable><font id="3" charset="iso-8859-1" name="Times New Roman"/></fonttable>
  <page id="1"/>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("fixed style line height"))
        .expect("style sheet cdxml should parse");
    assert_eq!(document.style.label_style.line_height, 6.0);
    assert_eq!(document.style.label_style.line_height_mode, "fixed");
    assert_eq!(document.style.caption_style.line_height, 7.1);
    assert_eq!(document.style.caption_style.line_height_mode, "fixed");
}

#[test]
fn parse_cdxml_node_label_fixed_line_height_controls_rendered_baselines() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelSize="10">
  <page id="1">
    <fragment id="fragment">
      <n id="label-node" p="30 30" NodeType="GenericNickname" GenericNickname="A B C">
        <t p="30 30" BoundingBox="20 10 50 55" LabelLineHeight="14" InterpretChemically="no">
          <s font="3" size="10" face="0" color="0">A
B
C</s>
        </t>
      </n>
      <n id="carbon" p="60 30"/>
      <b id="bond" B="label-node" E="carbon"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("fixed node label line height"))
        .expect("node label cdxml should parse");
    let label = document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.nodes.iter())
        .find(|node| node.id == "label-node")
        .and_then(|node| node.label.as_ref())
        .expect("node label should import");
    assert_eq!(label.line_height, Some(14.0));
    assert_eq!(label.line_height_mode, "fixed");
    assert!(label.line_advances.is_empty());

    let baselines = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text {
                node_id: Some(node_id),
                y,
                ..
            } if node_id == "label-node" => Some(y),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(baselines.len(), 3);
    assert!((baselines[1] - baselines[0] - 14.0).abs() < 0.01);
    assert!((baselines[2] - baselines[1] - 14.0).abs() < 0.01);
    assert!(document_to_cdxml(&document).contains("LabelLineHeight=\"14\""));
}

#[test]
fn imported_node_label_preserves_inherited_line_height_until_explicitly_edited() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="3" LabelSize="10" LabelLineHeight="6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1">
    <fragment id="fragment">
      <n id="nitrogen" p="30 30" Element="7">
        <t p="25 34" BoundingBox="25 22 33 35">
          <s font="3" size="10">N</s>
        </t>
      </n>
    </fragment>
  </page>
</CDXML>"##;
    let mut document = parse_cdxml_document(cdxml, Some("inherited label line height"))
        .expect("inherited node label should parse");
    let first_export = document_to_cdxml(&document);
    assert_eq!(first_export.matches("LabelLineHeight=\"6\"").count(), 1);
    assert!(!first_export
        .split("<t ")
        .nth(1)
        .expect("exported label")
        .split('>')
        .next()
        .expect("label start tag")
        .contains("LabelLineHeight="));

    let label = document
        .resources
        .values_mut()
        .find_map(|resource| match &mut resource.data {
            ResourceData::Fragment(fragment) => fragment
                .nodes
                .iter_mut()
                .find(|node| node.id == "nitrogen")
                .and_then(|node| node.label.as_mut()),
            _ => None,
        })
        .expect("imported nitrogen label");
    label.line_height_mode = "fixed".to_string();
    label.line_height = Some(9.5);
    let edited_export = document_to_cdxml(&document);
    assert!(edited_export.contains("LabelLineHeight=\"9.5\""));
}

#[test]
fn generated_atom_label_recomputes_variable_spacing_after_hydrogen_stacking() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="3" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1">
    <fragment id="fragment">
      <n id="left" p="20 45"/>
      <n id="nitrogen" p="40 20" Element="7" NumHydrogens="1"/>
      <n id="right" p="60 45"/>
      <b id="left-bond" B="left" E="nitrogen"/>
      <b id="right-bond" B="nitrogen" E="right"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("generated stacked atom label"))
        .expect("stacked atom label should parse");
    let object_translate_y = document
        .editable_fragment()
        .expect("stacked atom fragment")
        .object
        .transform
        .translate[1];
    let label = document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.nodes.iter())
        .find(|node| node.id == "nitrogen")
        .and_then(|node| node.label.as_ref())
        .expect("nitrogen label should be generated");

    assert_eq!(label.text, "H\nN");
    assert_eq!(label.line_height_mode, "variable");
    assert_eq!(label.line_advances.len(), 1);
    assert!((label.line_height.unwrap_or_default() - label.line_advances[0]).abs() < 0.01);
    assert!(label.line_advances[0] < 9.0);

    let position = label.position.expect("stacked label anchor baseline");
    let bbox = label.bbox().expect("stacked label box");
    let first_line_baseline = bbox[1] + label.font_size.unwrap_or(10.0) * 0.82;
    assert!(
        (first_line_baseline + label.line_advances[0] - position[1]).abs() < 0.01,
        "the box top must be reframed after variable spacing is resolved: {label:?}"
    );
    assert!(
        ((bbox[3] - bbox[1]) - label.line_advances[0] * 2.0).abs() < 0.01,
        "the multiline box height must use the resolved variable advance: {label:?}"
    );

    let rendered_baselines = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text {
                node_id: Some(node_id),
                y,
                ..
            } if node_id == "nitrogen" => Some(y),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(rendered_baselines.len(), 2);
    assert!((rendered_baselines[0] - first_line_baseline - object_translate_y).abs() < 0.01);
    assert!((rendered_baselines[1] - position[1] - object_translate_y).abs() < 0.01);
}

#[test]
fn generated_atom_label_reframes_variable_spacing_for_below_stack() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="3" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1">
    <fragment id="fragment">
      <n id="left" p="20 15"/>
      <n id="nitrogen" p="40 40" Element="7" NumHydrogens="1"/>
      <n id="right" p="60 15"/>
      <b id="left-bond" B="left" E="nitrogen"/>
      <b id="right-bond" B="nitrogen" E="right"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("generated below stacked atom label"))
        .expect("below stacked atom label should parse");
    let object_translate_y = document
        .editable_fragment()
        .expect("below stacked atom fragment")
        .object
        .transform
        .translate[1];
    let label = document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.nodes.iter())
        .find(|node| node.id == "nitrogen")
        .and_then(|node| node.label.as_ref())
        .expect("nitrogen label should be generated");

    assert_eq!(label.text, "N\nH");
    assert_eq!(label.layout.as_deref(), Some("attached-group-below"));
    assert_eq!(label.line_height_mode, "variable");
    assert_eq!(label.line_advances.len(), 1);

    let position = label.position.expect("element line anchor baseline");
    let bbox = label.bbox().expect("below stacked label box");
    let first_line_baseline = bbox[1] + label.font_size.unwrap_or(10.0) * 0.82;
    assert!((first_line_baseline - position[1]).abs() < 0.01);
    assert!(((bbox[3] - bbox[1]) - label.line_advances[0] * 2.0).abs() < 0.01);

    let rendered_baselines = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text {
                node_id: Some(node_id),
                y,
                ..
            } if node_id == "nitrogen" => Some(y),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(rendered_baselines.len(), 2);
    assert!((rendered_baselines[0] - position[1] - object_translate_y).abs() < 0.01);
    assert!(
        (rendered_baselines[1] - position[1] - object_translate_y - label.line_advances[0]).abs()
            < 0.01
    );
}

#[test]
fn generated_node_labels_remain_derived_until_their_text_is_explicitly_edited() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="3" LabelSize="10" ShowTerminalCarbonLabels="yes">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1">
    <fragment id="fragment">
      <n id="oxygen" p="20 20" Element="8" NumHydrogens="1"/>
      <n id="query" p="50 20" NodeType="ElementList" ElementList="7 8"/>
      <n id="carbon" p="80 20" Element="6" ShowTerminalCarbonLabels="yes"/>
      <b id="oxygen-query" B="oxygen" E="query"/>
      <b id="query-carbon" B="query" E="carbon"/>
    </fragment>
  </page>
</CDXML>"##;
    let mut document = parse_cdxml_document(cdxml, Some("automatic derived node labels"))
        .expect("automatic node labels should parse");
    let imported_carbon = document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.nodes.iter())
        .find(|node| node.id == "carbon")
        .expect("carbon node");
    assert!(
        imported_carbon.label.is_none(),
        "imported carbon display fields are preserved but do not synthesize authored labels"
    );
    let first_labels = document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.nodes.iter())
        .filter_map(|node| {
            node.label.as_ref().map(|label| {
                (
                    node.atomic_number,
                    label.source_text.clone(),
                    label.text.clone(),
                    label.position,
                    label.bbox(),
                )
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(first_labels.len(), 2);

    let first_export = document_to_cdxml(&document);
    assert!(!first_export.contains("<t "), "{first_export}");
    assert!(first_export.contains("ElementList=\"7 8\""));
    assert!(first_export.contains("ShowTerminalCarbonLabels=\"yes\""));

    let reparsed = parse_cdxml_document(&first_export, Some("reparsed automatic node labels"))
        .expect("exported automatic labels should parse");
    let reparsed_labels = reparsed
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.nodes.iter())
        .filter_map(|node| {
            node.label.as_ref().map(|label| {
                (
                    node.atomic_number,
                    label.source_text.clone(),
                    label.text.clone(),
                    label.position,
                    label.bbox(),
                )
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(reparsed_labels, first_labels);

    let oxygen_label = document
        .resources
        .values_mut()
        .find_map(|resource| match &mut resource.data {
            ResourceData::Fragment(fragment) => fragment
                .nodes
                .iter_mut()
                .find(|node| node.id == "oxygen")
                .and_then(|node| node.label.as_mut()),
            _ => None,
        })
        .expect("generated oxygen label");
    oxygen_label.meta["implicitHydrogenLabel"]["userEdited"] = json!(true);
    let edited_export = document_to_cdxml(&document);
    assert_eq!(edited_export.matches("<t ").count(), 1);
    assert!(edited_export.contains("UTF8Text=\"OH\""));
}

#[test]
fn generated_atom_labels_include_cdxml_charge_in_chemical_text() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="3" LabelSize="10">
  <page id="1">
    <fragment id="fragment">
      <n id="oxide" p="20 20" Element="8" NumHydrogens="0" Charge="-1"/>
      <n id="cesium" p="60 20" Element="55" NumHydrogens="0" Charge="1"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("generated charged atom labels"))
        .expect("charged atom labels should parse");
    let label_text = |id: &str| {
        document
            .resources
            .values()
            .filter_map(|resource| resource.data.as_fragment())
            .flat_map(|fragment| fragment.nodes.iter())
            .find(|node| node.id == id)
            .and_then(|node| node.label.as_ref())
            .map(|label| label.text.as_str())
    };
    assert_eq!(label_text("oxide"), Some("O-"));
    assert_eq!(label_text("cesium"), Some("Cs+"));
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
fn parse_cdxml_uses_explicit_bracket_attachments_before_geometry_pairing() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <graphic id="2" BoundingBox="20 75 20 15" GraphicType="Bracket" BracketType="Square"/>
    <graphic id="3" BoundingBox="80 10 80 70" GraphicType="Bracket" BracketType="Square" LineWidth="0.75">
      <objecttag id="1" Name="bracketusage">
        <t p="0 0" BoundingBox="0 -6.30 4.17 0"><s font="3" size="7.5" color="0">2</s></t>
      </objecttag>
      <objecttag id="2" Name="parameterizedBracketLabel" Visible="no">
        <t p="84 74" BoundingBox="84 68 102 74" Visible="no"><s font="3" size="7.5" color="0">abc</s></t>
      </objecttag>
    </graphic>
    <bracketedgroup id="4" BracketUsage="MultipleGroup" RepeatCount="2">
      <bracketattachment id="5" GraphicID="2"/>
      <bracketattachment id="6" GraphicID="3"/>
    </bracketedgroup>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("bracket text")).expect("cdxml should parse");
    let bracket_group = document
        .scene_objects()
        .into_iter()
        .find(|object| object_is_bracket_group(object))
        .expect("paired bracket should import");
    assert_eq!(
        bracket_group
            .meta
            .get("repeatCount")
            .and_then(|value| value.as_u64()),
        Some(2)
    );
    let sides: Vec<_> = bracket_group
        .children
        .iter()
        .filter(|object| object.object_type == "bracket")
        .collect();
    assert_eq!(sides.len(), 2, "paired bracket should import as two sides");
    assert!(sides.iter().any(|object| {
        object
            .payload
            .extra
            .get("side")
            .and_then(|value| value.as_str())
            == Some("left")
    }));
    assert!(sides.iter().any(|object| {
        object
            .payload
            .extra
            .get("side")
            .and_then(|value| value.as_str())
            == Some("right")
    }));
    let left_side = sides
        .iter()
        .find(|object| {
            object
                .payload
                .extra
                .get("side")
                .and_then(|value| value.as_str())
                == Some("left")
        })
        .expect("left bracket side should import");
    let right_side = sides
        .iter()
        .find(|object| {
            object
                .payload
                .extra
                .get("side")
                .and_then(|value| value.as_str())
                == Some("right")
        })
        .expect("right bracket side should import");
    assert_eq!(left_side.transform.translate[1], 15.0);
    assert_eq!(right_side.transform.translate[1], 10.0);
    assert_eq!(left_side.payload.bbox.expect("left bracket bbox")[3], 60.0);
    assert_eq!(
        right_side.payload.bbox.expect("right bracket bbox")[3],
        60.0
    );
    assert_eq!(
        left_side.payload.extra.get("strokeWidth"),
        Some(&json!(0.6))
    );
    assert_eq!(
        right_side.payload.extra.get("strokeWidth"),
        Some(&json!(0.75))
    );

    let text_objects: Vec<_> = document
        .objects
        .iter()
        .filter(|object| object.object_type == "text")
        .collect();
    let texts: Vec<_> = text_objects
        .iter()
        .filter(|object| object.visible)
        .filter_map(|object| {
            object
                .payload
                .extra
                .get("text")
                .and_then(|value| value.as_str())
        })
        .collect();
    assert_eq!(texts, vec!["abc"]);
    let roles: Vec<_> = text_objects
        .iter()
        .filter(|object| object.visible)
        .filter_map(|object| object.meta.get("role").and_then(|value| value.as_str()))
        .collect();
    assert_eq!(roles, vec!["parameterized_bracket_label"]);
    assert!(text_objects.iter().any(|object| {
        !object.visible
            && object
                .payload
                .extra
                .get("text")
                .and_then(|value| value.as_str())
                == Some("2")
    }));
    let label = text_objects
        .iter()
        .find(|object| object.visible)
        .expect("parameterized bracket label should provide the visible text");
    assert_eq!(label.transform.translate, [81.41, 68.0]);
    assert_eq!(
        label
            .payload
            .extra
            .get("baselineOffset")
            .and_then(|value| value.as_f64()),
        Some(6.0)
    );
}

#[test]
fn parse_cdxml_bracketusage_without_parameterized_label_uses_automatic_bracket_anchor() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <graphic id="2" BoundingBox="20 70 20 10" GraphicType="Bracket" BracketType="Square"/>
    <graphic id="3" BoundingBox="80 10 80 70" GraphicType="Bracket" BracketType="Square">
      <objecttag id="1" Name="bracketusage" Value="2">
        <t p="83.51 72.64" BoundingBox="83.75 67.37 87.37 72.64">
          <s font="3" size="7.5" color="0">2</s>
        </t>
      </objecttag>
    </graphic>
    <bracketedgroup id="4" BracketUsage="MultipleGroup" RepeatCount="2">
      <bracketattachment id="5" GraphicID="2"/>
      <bracketattachment id="6" GraphicID="3"/>
    </bracketedgroup>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("legacy bracket text"))
        .expect("legacy bracket label should parse");
    let label = document
        .objects
        .iter()
        .find(|object| object.object_type == "text" && object.visible)
        .expect("bracketusage should remain visible when no parameterized label exists");

    assert_eq!(
        label.meta.get("role").and_then(|value| value.as_str()),
        Some("bracket_usage")
    );
    assert_eq!(label.transform.translate, [81.41, 67.37]);
    assert_eq!(
        label
            .payload
            .extra
            .get("box")
            .and_then(|value| value.as_array())
            .and_then(|value| value.first())
            .and_then(|value| value.as_f64()),
        Some(2.34)
    );
    assert_eq!(
        label
            .payload
            .extra
            .get("baselineOffset")
            .and_then(|value| value.as_f64()),
        Some(5.27)
    );
}

#[test]
fn parse_cdxml_bracket_label_with_explicit_offset_uses_recorded_position() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <graphic id="2" BoundingBox="20 70 20 10" GraphicType="Bracket" BracketType="Square"/>
    <graphic id="3" BoundingBox="80 10 80 70" GraphicType="Bracket" BracketType="Square">
      <objecttag id="1" Name="bracketusage" Value="2" PositioningType="offset" PositioningOffset="12 4">
        <t p="92 74" BoundingBox="92 68.73 95.62 74">
          <s font="3" size="7.5" color="0">2</s>
        </t>
      </objecttag>
    </graphic>
    <bracketedgroup id="4" BracketUsage="MultipleGroup" RepeatCount="2">
      <bracketattachment id="5" GraphicID="2"/>
      <bracketattachment id="6" GraphicID="3"/>
    </bracketedgroup>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("offset bracket text"))
        .expect("offset bracket label should parse");
    let label = document
        .objects
        .iter()
        .find(|object| object.object_type == "text" && object.visible)
        .expect("explicitly positioned bracket label should remain visible");

    assert_eq!(label.transform.translate, [92.0, 68.73]);
    assert_eq!(
        label
            .payload
            .extra
            .get("anchorOffsetX")
            .and_then(|value| value.as_f64()),
        Some(0.0)
    );
}

#[test]
fn parse_cdxml_imports_visible_stereo_object_tags_inside_fragments() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="30" LineWidth="1" BoldWidth="4" HashSpacing="2.7">
  <page id="1">
    <fragment id="2">
      <n id="3" p="20 20" EnhancedStereoType="Or" EnhancedStereoGroupNum="1">
        <objecttag Name="enhancedstereo">
          <t p="23 18" BoundingBox="23 12 34 18"><s font="3" size="7.5">or1</s></t>
        </objecttag>
      </n>
      <n id="4" p="50 20" AS="R">
        <objecttag Name="stereo">
          <t p="42 18" BoundingBox="42 12 52 18"><s font="3" size="7.5">(R)</s></t>
        </objecttag>
      </n>
      <n id="5" p="80 20">
        <objecttag Name="stereo" Visible="no">
          <t p="72 18" BoundingBox="72 12 82 18"><s font="3" size="7.5">(S)</s></t>
        </objecttag>
      </n>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("stereo tags")).expect("stereo object tags should parse");
    let tagged_text: Vec<_> = document
        .objects
        .iter()
        .filter(|object| object.object_type == "text" && object.visible)
        .map(|object| {
            (
                object
                    .payload
                    .extra
                    .get("text")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default(),
                object
                    .meta
                    .get("role")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default(),
            )
        })
        .collect();

    assert_eq!(
        tagged_text,
        vec![("or1", "enhanced_stereo"), ("(R)", "stereo")]
    );
    let enhanced = document
        .objects
        .iter()
        .find(|object| {
            object.meta.get("role").and_then(|value| value.as_str()) == Some("enhanced_stereo")
        })
        .expect("enhanced-stereo label should import");
    assert_eq!(enhanced.transform.translate, [22.0, 16.7]);
    assert_eq!(
        enhanced
            .payload
            .extra
            .get("baselineOffset")
            .and_then(|value| value.as_f64()),
        Some(6.3)
    );
    assert!(document.objects.iter().any(|object| {
        object.object_type == "text"
            && !object.visible
            && object
                .payload
                .extra
                .get("text")
                .and_then(|value| value.as_str())
                == Some("(S)")
    }));
}
