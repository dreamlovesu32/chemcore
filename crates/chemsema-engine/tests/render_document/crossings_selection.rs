use super::*;

#[test]
fn hit_testing_checks_grouped_molecule_fragments() {
    let document = grouped_two_fragment_document();
    assert_eq!(document.editable_fragments().len(), 2);
    let first_hit = hit_test_bond_center(&document, Point::new(20.0, 0.0), 5.0)
        .expect("first molecule bond should be hoverable");
    assert_eq!(first_hit.bond_id, "b_first");
    let hit = hit_test_bond_center(&document, Point::new(120.0, 20.0), 5.0)
        .expect("grouped molecule bond should be hoverable");
    assert_eq!(hit.bond_id, "b_grouped");
}

#[test]
fn render_document_splits_ordinary_under_bond_at_later_crossing_bond() {
    let document = fragment_document_preserving_disconnected_components(
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
    assert!(
        !primitives
            .iter()
            .any(|primitive| primitive.role() == RenderRole::DocumentKnockout),
        "ordinary solid crossings should be represented by split bond geometry"
    );
    let mut under_bounds = primitives
        .iter()
        .filter_map(|primitive| {
            let RenderPrimitive::Polygon {
                role: RenderRole::DocumentBond,
                bond_id,
                points,
                ..
            } = primitive
            else {
                return None;
            };
            (bond_id.as_deref() == Some("b_under")).then(|| primitive_polygon_bounds(points))
        })
        .collect::<Vec<_>>();
    under_bounds.sort_by(|left, right| left[0].total_cmp(&right[0]));
    assert_eq!(under_bounds.len(), 2, "{primitives:?}");
    assert_eq!(under_bounds[0], [20.0, 59.5, 57.5, 60.5]);
    assert_eq!(under_bounds[1], [62.5, 59.5, 100.0, 60.5]);
    assert_eq!(
        primitives
            .iter()
            .filter(|primitive| matches!(primitive, RenderPrimitive::Polygon { role: RenderRole::DocumentBond, bond_id, .. } if bond_id.as_deref() == Some("b_over")))
            .count(),
        1,
        "upper bond must remain continuous"
    );
}

#[test]
fn render_document_adds_wavy_margin_knockout_across_molecule_objects() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 140.0, "height": 120.0, "background": "#ffffff" }
        },
        "style": {
            "defaults": {
                "lineWidth": 0.85,
                "marginWidth": 2.0
            }
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
            "id": "obj_under",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_under", "bbox": [0.0, 0.0, 120.0, 80.0] }
        }, {
            "id": "obj_wavy",
            "type": "molecule",
            "visible": true,
            "zIndex": 20,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_wavy", "bbox": [0.0, 0.0, 120.0, 80.0] }
        }],
        "resources": {
            "mol_under": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 120.0, 80.0],
                    "nodes": [
                        { "id": "u1", "element": "C", "atomicNumber": 6, "position": [20.0, 60.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "u2", "element": "C", "atomicNumber": 6, "position": [100.0, 60.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b_under", "begin": "u1", "end": "u2", "order": 1, "strokeWidth": 0.85 }
                    ]
                }
            },
            "mol_wavy": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 120.0, 80.0],
                    "nodes": [
                        { "id": "w1", "element": "C", "atomicNumber": 6, "position": [60.0, 35.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "w2", "element": "C", "atomicNumber": 6, "position": [60.0, 85.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        {
                            "id": "b_wavy",
                            "begin": "w1",
                            "end": "w2",
                            "order": 1,
                            "strokeWidth": 0.85,
                            "lineStyles": { "main": "wavy" }
                        }
                    ]
                }
            }
        }
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let under_index = primitives
        .iter()
        .position(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond,
                    bond_id,
                    ..
                } if bond_id.as_deref() == Some("b_under")
            )
        })
        .expect("under bond should render");
    let knockout_index = primitives
        .iter()
        .position(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentKnockout,
                    bond_id,
                    ..
                } if bond_id.as_deref() == Some("b_wavy")
            )
        })
        .expect("wavy over-bond should insert a local crossing knockout");
    let wavy_index = primitives
        .iter()
        .position(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Path {
                    role: RenderRole::DocumentBond,
                    bond_id,
                    ..
                } if bond_id.as_deref() == Some("b_wavy")
            )
        })
        .expect("wavy bond should render");

    assert!(under_index < knockout_index && knockout_index < wavy_index);
    let RenderPrimitive::Polygon { points, .. } = &primitives[knockout_index] else {
        unreachable!("local wavy crossing knockout is a polygon");
    };
    let bounds = primitive_polygon_bounds(points);
    assert!(bounds[2] - bounds[0] < 12.0, "{bounds:?}");
    assert!(bounds[3] - bounds[1] < 2.0, "{bounds:?}");
}

#[test]
fn cdxml_crossing_knockouts_match_chemdraw_style_envelopes() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 320 110" LineWidth="0.60" BoldWidth="2.0"
 BondLength="35" BondSpacing="18" MarginWidth="1.60">
 <page id="1" BoundingBox="0 0 320 110">
  <fragment id="11"><n id="100" p="10 65"/><n id="101" p="70 65"/><n id="102" p="40 35"/><n id="103" p="40 95"/>
   <b id="110" Z="1" B="100" E="101" CrossingBonds="111"/><b id="111" Z="2" B="102" E="103" Order="2" DoublePosition="Center" CrossingBonds="110"/></fragment>
  <fragment id="21"><n id="120" p="90 65"/><n id="121" p="150 65"/><n id="122" p="120 35"/><n id="123" p="120 95"/>
   <b id="130" Z="1" B="120" E="121" CrossingBonds="131"/><b id="131" Z="2" B="122" E="123" Order="2" DoublePosition="Left" CrossingBonds="130"/></fragment>
  <fragment id="31"><n id="140" p="170 65"/><n id="141" p="230 65"/><n id="142" p="200 35"/><n id="143" p="200 95"/>
   <b id="150" Z="1" B="140" E="141" CrossingBonds="151"/><b id="151" Z="2" B="142" E="143" Display="WedgeBegin" CrossingBonds="150"/></fragment>
  <fragment id="41"><n id="160" p="250 65"/><n id="161" p="310 65"/><n id="162" p="280 35"/><n id="163" p="280 95"/>
   <b id="170" Z="1" B="160" E="161" CrossingBonds="171"/><b id="171" Z="2" B="162" E="163" Display="Wavy" CrossingBonds="170"/></fragment>
 </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("crossing style envelope"))
        .expect("crossing style matrix should parse");
    let mut bounds = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentKnockout,
                points,
                ..
            } => Some(primitive_polygon_bounds(&points)),
            _ => None,
        })
        .collect::<Vec<_>>();
    bounds.sort_by(|left, right| left[0].total_cmp(&right[0]));

    assert_eq!(bounds.len(), 4, "expected one local patch per crossing");
    let expected_x = [(33.0, 47.0), (107.6, 121.6), (197.5, 202.5), (277.4, 282.6)];
    for (bounds, (expected_min, expected_max)) in bounds.iter().zip(expected_x) {
        assert!((bounds[0] - expected_min).abs() < 0.001, "{bounds:?}");
        assert!((bounds[2] - expected_max).abs() < 0.001, "{bounds:?}");
        assert!((bounds[1] - 64.65).abs() < 0.001, "{bounds:?}");
        assert!((bounds[3] - 65.35).abs() < 0.001, "{bounds:?}");
    }
}

#[test]
fn attached_label_crossings_use_the_rendered_character_axis() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 180 130" LineWidth="0.60" BondLength="35"
 MarginWidth="1.60" LabelFont="3" LabelSize="12">
 <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
 <page id="1" BoundingBox="0 0 180 130">
  <fragment id="11">
   <n id="100" p="15 70"/><n id="101" p="165 70"/>
   <n id="102" p="35 25" NodeType="Nickname" LabelDisplay="Right">
    <t p="82 34" BoundingBox="55 18 90 38"><s font="3" size="12">ABC</s></t>
   </n>
   <n id="103" p="82 115"/>
   <b id="110" Z="1" B="100" E="101" CrossingBonds="111"/>
   <b id="111" Z="2" B="102" E="103" BeginAttach="1" CrossingBonds="110"/>
  </fragment>
 </page>
</CDXML>"#;
    let mut document = parse_cdxml_document(cdxml, Some("attached crossing axis"))
        .expect("attached crossing probe should parse");
    let resource_id = document
        .resources
        .iter()
        .find_map(|(resource_id, resource)| {
            resource
                .data
                .as_fragment()
                .is_some_and(|fragment| fragment.nodes.iter().any(|node| node.id == "102"))
                .then_some(resource_id.clone())
        })
        .expect("resource containing attached label");
    let object = document
        .objects
        .iter()
        .find(|object| object.payload.resource_ref.as_deref() == Some(resource_id.as_str()))
        .expect("molecule object");
    let translate_x = object.transform.translate[0];
    let fragment = document
        .resources
        .get_mut(&resource_id)
        .and_then(|resource| resource.data.as_fragment_mut())
        .expect("molecule fragment");
    let label_node = fragment
        .nodes
        .iter()
        .find(|node| node.id == "102")
        .expect("attached label");
    let raw_node_x = label_node.position[0] + translate_x;
    let attached_character_x = label_node
        .label
        .as_ref()
        .and_then(|label| label.glyph_polygons().get(1).cloned())
        .map(|polygon| primitive_polygon_bounds(&polygon))
        .map(|bounds| (bounds[0] + bounds[2]) * 0.5 + translate_x)
        .expect("second authored character should have glyph geometry");
    assert!(
        (attached_character_x - raw_node_x).abs() > 5.0,
        "probe must distinguish the raw node from the authored character"
    );
    fragment
        .nodes
        .iter_mut()
        .find(|node| node.id == "103")
        .expect("upper bond end")
        .position[0] = attached_character_x - translate_x;

    let assert_crossing_axis = |primitives: &[RenderPrimitive]| {
        let mut under_bounds = primitives
            .iter()
            .filter_map(|primitive| match primitive {
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond,
                    bond_id,
                    points,
                    ..
                } if bond_id.as_deref() == Some("110") => Some(primitive_polygon_bounds(points)),
                _ => None,
            })
            .collect::<Vec<_>>();
        under_bounds.sort_by(|left, right| left[0].total_cmp(&right[0]));
        assert_eq!(
            under_bounds.len(),
            2,
            "lower bond must split around the attached upper axis: {primitives:?}"
        );
        let gap_center = (under_bounds[0][2] + under_bounds[1][0]) * 0.5;
        assert!(
            (gap_center - attached_character_x).abs() < 1.0e-6,
            "gap center {gap_center} must follow attached character axis {attached_character_x}"
        );
        assert!(
            (gap_center - raw_node_x).abs() > 5.0,
            "crossing must not use raw label-node coordinate {raw_node_x}"
        );
    };

    assert_crossing_axis(&render_document(&document));

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load for target rendering");
    let target_primitives = engine.render_targets(
        &BTreeSet::new(),
        &BTreeSet::from(["110".to_string()]),
        &BTreeSet::new(),
    );
    assert_crossing_axis(&target_primitives);
}

#[test]
fn cdxml_near_endpoint_crossings_use_finite_margin_caps() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 260 110" LineWidth="0.60" BoldWidth="2.0"
 BondLength="14.40" BondSpacing="18" MarginWidth="1.60">
 <page id="1" BoundingBox="0 0 260 110">
  <!-- Infinite lines meet 1.17 pt beyond the lower bond's end. -->
  <fragment id="11">
   <n id="100" p="43.15 43.94"/><n id="101" p="20.33 50.27"/>
   <n id="102" p="10.74 43.32"/><n id="103" p="30.30 60.10"/>
   <b id="110" Z="1" B="100" E="101"/><b id="111" Z="2" B="102" E="103"/>
  </fragment>
  <!-- Infinite lines meet 2.40 pt before the upper bond's begin cap. -->
  <fragment id="21">
   <n id="120" p="90.04 41.50"/><n id="121" p="93.21 72.94"/>
   <n id="122" p="94.35 63.04"/><n id="123" p="113.55 47.01"/>
   <b id="130" Z="1" B="120" E="121"/><b id="131" Z="2" B="122" E="123"/>
  </fragment>
  <!-- Same topology, but farther than the finite upper margin cap. -->
  <fragment id="31">
   <n id="140" p="170 25"/><n id="141" p="170 85"/>
   <n id="142" p="174 60"/><n id="143" p="200 30"/>
   <b id="150" Z="1" B="140" E="141"/><b id="151" Z="2" B="142" E="143"/>
  </fragment>
 </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("near endpoint crossing caps"))
        .expect("near-endpoint crossing matrix should parse");
    let knockouts = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentKnockout,
                bond_id,
                points,
                ..
            } => Some((bond_id, points)),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(knockouts.len(), 2, "{knockouts:?}");
    assert_eq!(knockouts[0].0.as_deref(), Some("111"));
    assert_eq!(knockouts[1].0.as_deref(), Some("131"));
    for (points, expected) in [
        (
            &knockouts[0].1,
            [20.236447, 49.695888, 21.730173, 50.607265],
        ),
        (
            &knockouts[1].1,
            [91.904096, 62.132698, 92.604508, 63.445850],
        ),
    ] {
        let bounds = primitive_polygon_bounds(points);
        for (actual, expected) in bounds.into_iter().zip(expected) {
            assert!((actual - expected).abs() < 1.0e-5, "{bounds:?}");
        }
    }
    assert!(
        knockouts
            .iter()
            .all(|(_, points)| polygon_area(points).abs() > 1.0e-4),
        "near-endpoint contacts must produce real finite overlap polygons: {knockouts:?}"
    );

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load for target rendering");
    let target_primitives = engine.render_targets(
        &BTreeSet::new(),
        &BTreeSet::from(["110".to_string()]),
        &BTreeSet::new(),
    );
    assert!(
        target_primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentBond,
                bond_id,
                ..
            } if bond_id.as_deref() == Some("111")
        )),
        "near-endpoint upper bond must be an incremental-render dependency: {target_primitives:?}"
    );
    assert!(
        target_primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentKnockout,
                bond_id,
                ..
            } if bond_id.as_deref() == Some("111")
        )),
        "incremental rendering must retain the near-endpoint knockout: {target_primitives:?}"
    );
}

#[test]
fn explicit_crossing_bonds_are_authoritative_over_geometric_fallback() {
    let document = fragment_document_preserving_disconnected_components(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 60.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [100.0, 60.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [60.0, 20.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [60.0, 100.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b_under", "begin": "n1", "end": "n2", "order": 1,
                "strokeWidth": 1.0, "marginWidth": 2.0,
                "meta": { "import": { "cdxml": { "crossingBonds": [] } } }
            },
            { "id": "b_over", "begin": "n3", "end": "n4", "order": 1, "strokeWidth": 1.0, "marginWidth": 2.0 }
        ]),
    );

    assert!(
        !render_document(&document).iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentKnockout,
                ..
            }
        )),
        "an explicit empty crossing list must suppress geometric inference"
    );
}

#[test]
fn explicit_crossing_bond_ids_are_global_across_fragments() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 120 120" LineWidth="1" MarginWidth="2">
  <page id="1" BoundingBox="0 0 120 120">
    <fragment id="2">
      <n id="10" p="20 60"/><n id="11" p="100 60"/>
      <b id="20" Z="1" B="10" E="11" CrossingBonds="31"/>
    </fragment>
    <fragment id="3">
      <n id="30" p="60 20"/><n id="32" p="60 100"/>
      <b id="31" Z="2" B="30" E="32" CrossingBonds="20"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("cross-fragment crossings"))
        .expect("cross-fragment CDXML should parse");
    let primitives = render_document(&document);
    assert_eq!(
        primitives
            .iter()
            .filter(|primitive| matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond,
                    bond_id,
                    ..
                } if bond_id.as_deref() == Some("20")
            ))
            .count(),
        2,
        "explicit crossing IDs must split the lower bond across fragment scope: {primitives:?}"
    );
    assert_eq!(
        primitives
            .iter()
            .filter(|primitive| matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond,
                    bond_id,
                    ..
                } if bond_id.as_deref() == Some("31")
            ))
            .count(),
        1,
        "the upper bond must remain continuous: {primitives:?}"
    );
}

#[test]
fn cdxml_crossing_bonds_round_trip_with_remapped_object_ids() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 120 120" LineWidth="0.6" MarginWidth="1.6">
  <page id="1" BoundingBox="0 0 120 120">
    <fragment id="2">
      <n id="10" p="20 60"/><n id="11" p="100 60"/>
      <n id="12" p="60 20"/><n id="13" p="60 100"/>
      <b id="20" Z="7" B="10" E="11" CrossingBonds="21"/>
      <b id="21" Z="8" B="12" E="13" CrossingBonds="20"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("crossings")).expect("CDXML should parse");
    let exported = document_to_cdxml(&document);
    assert!(exported.contains("CrossingBonds=\""), "{exported}");
    assert!(exported.contains("Z=\"7\""), "{exported}");
    assert!(exported.contains("Z=\"8\""), "{exported}");

    let reopened = parse_cdxml_document(&exported, Some("crossings reopened"))
        .expect("exported CDXML should parse");
    let bonds: Vec<_> = reopened
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.bonds.iter())
        .collect();
    assert_eq!(bonds.len(), 2);
    for (index, bond) in bonds.iter().enumerate() {
        let other = bonds[1 - index];
        let crossings = bond
            .meta
            .pointer("/import/cdxml/crossingBonds")
            .and_then(serde_json::Value::as_array)
            .expect("crossing list should survive");
        assert_eq!(crossings, &vec![json!(other.id)]);
    }
}

#[test]
fn coordinate_free_cdxml_chain_gets_deterministic_topology_layout() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="CDXMLWriter"><page id="10"><fragment id="11">
  <n id="1" Element="6"/><n id="2" Element="6"/><n id="3" Element="6"/><n id="4" Element="6"/>
  <b B="1" E="2" id="5"/><b B="2" E="3" id="6"/><b B="3" E="4" id="7"/>
</fragment></page></CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("coordinate-free chain"))
        .expect("topology-only CDXML should import");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("coordinate-free fragment should survive");

    assert_eq!(fragment.nodes.len(), 4);
    assert_eq!(fragment.bonds.len(), 3);
    assert_eq!(fragment.nodes[0].position, [0.0, 0.0]);
    assert_eq!(fragment.nodes[1].position, [25.98, 15.0]);
    assert_eq!(fragment.nodes[2].position, [51.96, 0.0]);
    assert_eq!(fragment.nodes[3].position, [77.94, 15.0]);
    assert_eq!(
        render_document(&document)
            .iter()
            .filter(|primitive| render_primitive_bond_id(primitive).is_some())
            .count(),
        3
    );
}

#[test]
fn coordinate_free_cdxml_aromatic_ring_and_missing_bond_ids_remain_visible() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="CDXMLWriter"><page><fragment>
  <n id="1"/><n id="2"/><n id="3"/><n id="4"/><n id="5"/><n id="6"/>
  <b B="1" E="2" Order="1.5" Display="Dash"/><b B="2" E="3" Order="1.5" Display="Dash"/>
  <b B="3" E="4" Order="1.5" Display="Dash"/><b B="4" E="5" Order="1.5" Display="Dash"/>
  <b B="5" E="6" Order="1.5" Display="Dash"/><b B="6" E="1" Order="1.5" Display="Dash"/>
</fragment></page></CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("coordinate-free aromatic ring"))
        .expect("topology-only aromatic ring should import");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("coordinate-free ring should survive");
    let ids = fragment
        .bonds
        .iter()
        .map(|bond| bond.id.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(fragment.nodes.len(), 6);
    assert_eq!(fragment.bonds.len(), 6);
    assert_eq!(ids.len(), 6);
    assert!(ids.iter().all(|id| id.starts_with("cdxml_bond_")));
    assert!(fragment.bonds.iter().all(|bond| {
        bond.order == 1
            && bond.line_styles.main == chemsema_engine::BondLinePattern::Solid
            && bond
                .meta
                .pointer("/import/cdxml/topologyOnlyAromaticDash")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
            && bond
                .meta
                .pointer("/import/cdxml/aromatic")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
    }));
    assert_eq!(
        render_document(&document)
            .iter()
            .filter(|primitive| render_primitive_bond_id(primitive).is_some())
            .count(),
        6
    );

    let exported = document_to_cdxml(&document);
    assert_eq!(exported.matches("Order=\"1\"").count(), 6, "{exported}");
    assert!(!exported.contains("Order=\"1.5\""), "{exported}");
    assert!(!exported.contains("Display=\"Dash\""), "{exported}");
    assert_eq!(
        exported.matches("Display=\"Solid\"").count(),
        6,
        "{exported}"
    );

    let reparsed = parse_cdxml_document(&exported, Some("laid-out aromatic ring"))
        .expect("canonicalized ring should reimport");
    let reparsed_fragment = reparsed
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("canonicalized fragment should survive");
    assert!(reparsed_fragment.bonds.iter().all(|bond| {
        bond.order == 1 && bond.line_styles.main == chemsema_engine::BondLinePattern::Solid
    }));
}

#[test]
fn coordinate_free_cdxml_cycle_uses_chemdraw_ring_orientation_without_producer_hint() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="17"><page><fragment>
  <n id="1" AbnormalValence="yes"/><n id="2" AbnormalValence="yes"/>
  <n id="3" AbnormalValence="yes"/><n id="4" AbnormalValence="yes"/>
  <n id="5" AbnormalValence="yes"/>
  <b B="1" E="2"/><b B="2" E="3"/><b B="3" E="4"/>
  <b B="4" E="5"/><b B="5" E="1"/>
</fragment></page></CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("coordinate-free cyclopentane"))
        .expect("a producer name is not required for coordinate-free ring layout");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("cycle should survive");
    let positions = fragment
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.position))
        .collect::<Vec<_>>();
    assert_eq!(
        positions,
        vec![
            ("1", [22.25, 0.0]),
            ("2", [27.5, 16.17]),
            ("3", [13.75, 26.16]),
            ("4", [0.0, 16.17]),
            ("5", [5.25, 0.0]),
        ]
    );
    assert!(fragment
        .nodes
        .iter()
        .all(|node| node.atom_properties.abnormal_valence));
    assert_eq!(fragment.bonds.len(), 5);

    let exported = document_to_cdxml(&document);
    let reparsed = parse_cdxml_document(&exported, Some("coordinate-free cycle export"))
        .expect("laid-out cycle should reimport");
    let reparsed_fragment = reparsed
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("reparsed cycle should survive");
    assert_eq!(
        reparsed_fragment
            .nodes
            .iter()
            .map(|node| node.position)
            .collect::<Vec<_>>(),
        fragment
            .nodes
            .iter()
            .map(|node| node.position)
            .collect::<Vec<_>>()
    );
}

#[test]
fn coordinate_free_cdxml_branching_is_not_silently_drawn_as_a_polygon() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="17"><page><fragment id="branch">
  <n id="1"/><n id="2"/><n id="3"/><n id="4"/>
  <b B="1" E="2"/><b B="1" E="3"/><b B="1" E="4"/>
</fragment></page></CDXML>"#;
    let error = parse_cdxml_document(cdxml, Some("coordinate-free branch"))
        .expect_err("an unverified branching layout must not use a polygon fallback");
    assert!(error.contains("branching or fused topology"), "{error}");
    assert!(error.contains("'branch'"), "{error}");
}

#[test]
fn positioned_cdxml_aromatic_dash_remains_visibly_dashed() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML HashSpacing="2.7"><page><fragment>
  <n id="1" p="10 10"/><n id="2" p="40 10"/>
  <b id="3" B="1" E="2" Order="1.5" Display="Dash"/>
</fragment></page></CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("positioned aromatic dash"))
        .expect("positioned aromatic dash should import");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should survive");
    let bond = fragment.bonds.first().expect("bond should survive");

    assert_eq!(bond.order, 1);
    assert_eq!(
        bond.line_styles.main,
        chemsema_engine::BondLinePattern::Dashed
    );
    assert_eq!(
        bond.meta
            .pointer("/import/cdxml/aromatic")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert!(document_bond_polygon_count_for_object(&render_document(&document), "obj_mol_001") > 1);

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("Order=\"1.5\""), "{exported}");
    assert!(exported.contains("Display=\"Dash\""), "{exported}");
}

#[test]
fn positioned_solid_order_one_point_five_is_not_drawn_as_a_double_bond() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML><page><fragment>
  <n id="1" p="10 10"/><n id="2" p="40 10"/>
  <b id="3" B="1" E="2" Order="1.5"/>
</fragment></page></CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("solid delocalized bond"))
        .expect("solid order-1.5 bond should import");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should survive");
    let bond = fragment.bonds.first().expect("bond should survive");
    assert_eq!(bond.order, 1);
    assert_eq!(
        bond.meta
            .pointer("/import/cdxml/aromatic")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        document_bond_polygon_count_for_object(&render_document(&document), "obj_mol_001"),
        1
    );
    let exported = document_to_cdxml(&document);
    assert!(exported.contains("Order=\"1.5\""));
    assert!(!exported.contains("Display=\"Dash\""), "{exported}");
    let reopened = parse_cdxml_document(&exported, Some("solid delocalized bond reopened"))
        .expect("solid order-1.5 export should reopen");
    let reopened_bond = reopened
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.bonds.first())
        .expect("reopened bond should survive");
    assert_eq!(
        reopened_bond.line_styles.main,
        chemsema_engine::BondLinePattern::Solid
    );
}

#[test]
fn positioned_order_one_point_five_uses_explicit_display2_as_second_lane() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML><page><fragment>
  <n id="1" p="10 10"/><n id="2" p="40 10"/>
  <b id="3" B="1" E="2" Order="1.5" Display="Dash" Display2="Dash"/>
</fragment></page></CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("two-lane delocalized bond"))
        .expect("explicit second display lane should import");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should survive");
    let bond = fragment.bonds.first().expect("bond should survive");
    assert_eq!(bond.order, 2);
    assert_eq!(
        bond.line_styles.main,
        chemsema_engine::BondLinePattern::Dashed
    );
    assert_eq!(
        bond.line_styles.right,
        chemsema_engine::BondLinePattern::Dashed
    );
    assert!(document_bond_polygon_count_for_object(&render_document(&document), "obj_mol_001") > 2);

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("Order=\"1.5\""), "{exported}");
    assert!(exported.contains("Display2=\"Dash\""), "{exported}");
}

#[test]
fn positioned_unlabeled_hetero_atom_gets_its_element_label() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LabelFont="3" LabelSize="10"><fonttable>
  <font id="3" charset="iso-8859-1" name="Arial"/>
</fonttable><page><fragment>
  <n id="1" p="10 20"/><n id="2" p="40 20" Element="7" NumHydrogens="0"/>
  <b id="3" B="1" E="2"/>
</fragment></page></CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("positioned unlabeled nitrogen"))
        .expect("positioned nitrogen should import");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should survive");
    let nitrogen = fragment
        .nodes
        .iter()
        .find(|node| node.id == "2")
        .expect("nitrogen should survive");

    assert_eq!(
        nitrogen.label.as_ref().map(|label| label.text.as_str()),
        Some("N")
    );
    assert!(render_document(&document).iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Text { text, runs, .. }
            if text == "N" || runs.iter().any(|run| run.text == "N")
    )));
}

#[test]
fn coordinate_free_cdxml_dative_chain_keeps_donor_hydrogen_and_arrowhead() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="CDXMLWriter"><page><fragment>
  <n id="1" Element="6"/><n id="2" Element="8"/><n id="3" Element="6"/><n id="4" Element="6"/>
  <b id="5" B="1" E="2"/><b id="6" B="2" E="3" Order="dative"/><b id="7" B="3" E="4"/>
</fragment></page></CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("coordinate-free dative chain"))
        .expect("topology-only dative chain should import");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("coordinate-free dative chain should survive");
    let oxygen = fragment
        .nodes
        .iter()
        .find(|node| node.id == "2")
        .expect("oxygen donor should survive");

    assert_eq!(oxygen.num_hydrogens, 1);
    assert!(oxygen
        .label
        .as_ref()
        .is_some_and(|label| label.text.contains('H')));
    assert_eq!(
        render_document(&document)
            .iter()
            .filter(|primitive| render_primitive_bond_id(primitive) == Some("6"))
            .count(),
        2,
        "dative bond should render a shaft and one solid arrowhead"
    );
    assert!(document_to_cdxml(&document).contains("Order=\"dative\""));
}

#[test]
fn cdxml_restrict_implicit_hydrogens_renders_an_independent_atom_query_marker() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemDraw 6.0.1" LabelFont="3" LabelSize="10" LabelFace="96"
       BondLength="30" LineWidth="1" BoldWidth="4" MarginWidth="2">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page><fragment>
    <n id="22" p="131.35 270"/><n id="23" p="131.35 300"/>
    <n id="24" p="159.89 309.27"/><n id="25" p="177.52 285"/>
    <n id="26" p="159.89 260.73" NumHydrogens="1" Charge="-1"
       ImplicitHydrogens="yes">
      <t id="33" p="157.07 254.19" BoundingBox="158 247 163 265"
         LabelAlignment="Above" LineStarts="2 3">
        <s font="3" size="10" face="96">CH</s>
      </t>
    </n>
    <b id="27" B="22" E="23" Order="2"/><b id="28" B="23" E="24"/>
    <b id="29" B="24" E="25" Order="2"/><b id="30" B="25" E="26"/>
    <b id="31" B="26" E="22"/>
  </fragment></page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("legacy restricted hydrogen"))
        .expect("legacy CDXML should import");
    let carbon = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "26"))
        .expect("labeled carbon should survive");
    let label = carbon.label.as_ref().expect("carbon label should survive");

    assert_eq!(carbon.num_hydrogens, 1);
    assert_eq!(label.source_text.as_deref(), Some("CH"));
    assert_eq!(label.lines, vec!["H", "C"]);
    assert_eq!(label.text.matches('H').count(), 1);

    let node_h_positions = |document: &ChemSemaDocument| {
        render_document(document)
            .iter()
            .filter_map(|primitive| match primitive {
                RenderPrimitive::Text {
                    node_id: Some(node_id),
                    x,
                    y,
                    font_size,
                    runs,
                    ..
                } if node_id == "26" && runs.iter().any(|run| run.text.trim() == "H") => {
                    Some((*x, *y, *font_size))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let h_positions = node_h_positions(&document);
    assert_eq!(h_positions.len(), 2);
    assert!(
        h_positions.iter().any(|(x, y, size)| {
            (*x - 162.42).abs() <= 0.15
                && (*y - 246.38).abs() <= 0.25
                && (*size - 7.5).abs() <= 0.01
        }),
        "the query H should use ChemDraw's standard atom-query placement above the open bond sector: {h_positions:?}"
    );

    let combined_query = parse_cdxml_document(
        &cdxml.replace(
            "ImplicitHydrogens=\"yes\"",
            "ImplicitHydrogens=\"yes\" FreeSites=\"2\" RingBondCount=\"SimpleRing\" UnsaturatedBonds=\"MustBePresent\"",
        ),
        Some("combined implicit-hydrogen query"),
    )
    .expect("combined atom query should import");
    let combined_query_text = render_document(&combined_query)
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text {
                node_id: Some(node_id),
                runs,
                ..
            } if node_id == "26" => {
                Some(runs.iter().map(|run| run.text.as_str()).collect::<String>())
            }
            _ => None,
        })
        .find(|text| text.contains('*'))
        .expect("combined query annotation should render");
    assert_eq!(
        combined_query_text, "*2HSR",
        "ChemDraw orders the implicit-hydrogen restriction after free sites and before S/R"
    );

    let without_num_hydrogens = parse_cdxml_document(
        &cdxml.replace(" NumHydrogens=\"1\"", ""),
        Some("query marker without NumHydrogens"),
    )
    .expect("query marker should not depend on NumHydrogens");
    assert_eq!(node_h_positions(&without_num_hydrogens).len(), 2);

    let hidden_query_marker = parse_cdxml_document(
        &cdxml.replace(
            "<CDXML CreationProgram=",
            "<CDXML ShowAtomQuery=\"no\" CreationProgram=",
        ),
        Some("hidden atom query marker"),
    )
    .expect("ShowAtomQuery=no should import");
    assert_eq!(
        node_h_positions(&hidden_query_marker).len(),
        1,
        "only the H authored inside CH should remain"
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("NumHydrogens=\"1\""), "{exported}");
    assert!(exported.contains("ImplicitHydrogens=\"yes\""), "{exported}");
    assert!(exported.contains(">CH</s>"), "{exported}");
}

#[test]
fn atom_query_annotations_follow_chemdraws_continuous_open_sector_layout() {
    let cases = [
        ("130 100", (85.14, 102.79)),
        ("125.980762 115", (87.13, 98.11)),
        ("100 130", (100.0, 93.44)),
        ("70 100", (114.86, 102.79)),
        ("100 70", (100.0, 112.13)),
    ];
    for (neighbor, expected) in cases {
        let cdxml = format!(
            r#"<CDXML ShowAtomQuery="yes" LabelFont="3" LabelSize="10"
 MarginWidth="1.6" LineWidth="0.6" BoldWidth="2" BondLength="30">
 <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
 <page><fragment>
  <n id="101" p="100 100" Element="7" FreeSites="2"
     RingBondCount="SimpleRing" UnsaturatedBonds="MustBePresent"/>
  <n id="102" p="{neighbor}"/>
  <b id="103" B="101" E="102"/>
 </fragment></page>
</CDXML>"#
        );
        let document = parse_cdxml_document(&cdxml, Some("continuous atom-query placement probe"))
            .expect("atom-query probe should import");
        let label_bounds = document
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "101"))
            .and_then(|node| node.label.as_ref())
            .map(|label| label.bbox())
            .expect("atom label should have measured bounds");
        let rendered = render_document(&document);
        let label_position = rendered
            .iter()
            .find_map(|primitive| match primitive {
                RenderPrimitive::Text {
                    node_id: Some(node_id),
                    x,
                    y,
                    runs,
                    ..
                } if node_id == "101"
                    && runs.iter().map(|run| run.text.as_str()).collect::<String>() == "N" =>
                {
                    Some((*x, *y))
                }
                _ => None,
            })
            .expect("atom label should render");
        assert!(
            (label_position.0 - 96.39).abs() <= 0.5
                && (label_position.1 - 103.90).abs() <= 0.5,
            "neighbor={neighbor}, ChemDraw keeps the atom label centered; actual={label_position:?}"
        );
        let position = rendered
            .iter()
            .find_map(|primitive| match primitive {
                RenderPrimitive::Text {
                    node_id: Some(node_id),
                    x,
                    y,
                    runs,
                    ..
                } if node_id == "101" && runs.iter().any(|run| run.text == "*") => Some((*x, *y)),
                _ => None,
            })
            .expect("atom-query annotation should render");
        assert!(
            (position.0 - expected.0).abs() <= 0.5 && (position.1 - expected.1).abs() <= 0.5,
            "neighbor={neighbor}, expected={expected:?}, actual={position:?}, label_bounds={label_bounds:?}"
        );
    }
}

#[test]
fn atom_query_free_sites_consume_implicit_hydrogen_valence() {
    for (free_sites, expected_hydrogens) in [(0, 2), (1, 1), (2, 0)] {
        let cdxml = format!(
            r#"<CDXML LabelFont="3" LabelSize="10">
 <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
 <page><fragment>
  <n id="101" p="100 100" Element="7" FreeSites="{free_sites}"/>
  <n id="102" p="130 100"/>
  <b id="103" B="101" E="102"/>
 </fragment></page>
</CDXML>"#
        );
        let document = parse_cdxml_document(&cdxml, Some("free-site valence probe"))
            .expect("free-site probe should import");
        let node = document
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "101"))
            .expect("query node should survive");
        assert_eq!(
            node.num_hydrogens, expected_hydrogens,
            "FreeSites={free_sites} must reproduce ChemDraw's explicit hydrogen count"
        );
    }
}

#[test]
fn render_targets_for_under_crossing_bond_include_over_bond_dependency() {
    let document = fragment_document_preserving_disconnected_components(
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
    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");

    let primitives = engine.render_targets(
        &BTreeSet::new(),
        &BTreeSet::from(["b_under".to_string()]),
        &BTreeSet::new(),
    );

    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond,
                    bond_id,
                    ..
                } if bond_id.as_deref() == Some("b_over")
            )
        }),
        "targeting the lower crossing bond should also return the upper bond for desktop patching: {primitives:?}"
    );
    assert_eq!(
        primitives
            .iter()
            .filter(|primitive| matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond,
                    bond_id,
                    ..
                } if bond_id.as_deref() == Some("b_under")
            ))
            .count(),
        2,
        "target rendering must preserve both split lower-bond segments: {primitives:?}"
    );
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
fn grouped_scene_object_child_click_selects_child_not_group() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
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
    assert_eq!(engine.state().selection.arrow_objects, vec!["shape_a"]);
    let hit: serde_json::Value =
        serde_json::from_str(&engine.context_hit_test_json(Point::new(20.0, 15.0))).unwrap();
    assert_eq!(hit["objectId"], "shape_a");
    assert_eq!(hit["objectType"], "shape");
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
            assert!((*width - 21.0).abs() < 0.2);
            assert!((*height - 11.0).abs() < 0.2);
        }
        _ => unreachable!(),
    }

    assert!(engine.select_component_at_point(Point::new(20.0, 15.0), false));
    assert_eq!(engine.state().selection.arrow_objects, vec!["grp_1"]);
    let grouped_hit: serde_json::Value =
        serde_json::from_str(&engine.context_hit_test_json(Point::new(20.0, 15.0))).unwrap();
    assert_eq!(grouped_hit["objectId"], "grp_1");
    assert_eq!(grouped_hit["objectType"], "group");
    assert_eq!(grouped_hit["selected"], true);

    assert!(engine.set_selection_locked(true));
    engine.clear_selection();
    let locked_hit: serde_json::Value =
        serde_json::from_str(&engine.context_hit_test_json(Point::new(20.0, 15.0))).unwrap();
    assert_eq!(locked_hit["objectId"], "grp_1");
    assert_eq!(locked_hit["selected"], false);
    engine.select_at_point(Point::new(20.0, 15.0), false);
    assert_eq!(engine.state().selection.arrow_objects, vec!["grp_1"]);
}

#[test]
fn region_selection_collapses_group_box_only_when_all_children_are_selected() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
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
fn region_selecting_grouped_molecule_moves_nodes_not_parent_group() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_group_region_molecule",
            "title": "group region molecule",
            "page": { "width": 220.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": 0.85,
                "fontFamily": "Arial",
                "fontSize": 11.0
            },
            "style_bracket": {
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": 1.0
            }
        },
        "objects": [{
            "id": "grp_1",
            "type": "group",
            "zIndex": 30,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
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
                    "id": "bracket_1",
                    "type": "bracket",
                    "visible": true,
                    "zIndex": 20,
                    "transform": { "translate": [120.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                    "styleRef": "style_bracket",
                    "payload": { "bbox": [0.0, 0.0, 30.0, 60.0], "kind": "square" }
                }
            ]
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
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

    engine.select_in_rect(Point::new(0.0, 0.0), Point::new(90.0, 60.0), false);
    assert!(
        !engine
            .state()
            .selection
            .arrow_objects
            .contains(&"grp_1".to_string()),
        "region selection must not directly select the parent group: {:?}",
        engine.state().selection
    );
    assert_eq!(engine.state().selection.bonds, vec!["b1"]);
    assert!(engine.state().selection.nodes.contains(&"n1".to_string()));
    assert!(engine.state().selection.nodes.contains(&"n2".to_string()));

    assert!(engine.begin_selection_move_at_point(Point::new(30.0, 30.0), false, false));
    assert!(engine.update_selection_move(Point::new(40.0, 30.0), false));
    assert!(engine.finish_selection_move(Point::new(40.0, 30.0), false));

    let group = engine
        .state()
        .document
        .find_scene_object("grp_1")
        .expect("group should remain");
    assert_eq!(group.transform.translate, [0.0, 0.0]);
    let bracket = engine
        .state()
        .document
        .find_scene_object("bracket_1")
        .expect("bracket should remain");
    assert_eq!(bracket.transform.translate, [120.0, 20.0]);
    let fragment = engine
        .state()
        .document
        .resources
        .get("mol_001")
        .and_then(|resource| resource.data.as_fragment())
        .expect("fragment should still exist");
    assert_eq!(fragment.nodes[0].position, [20.0, 20.0]);
    assert_eq!(fragment.nodes[1].position, [70.0, 20.0]);
}

#[test]
fn select_all_collapses_grouped_molecule_and_text_to_one_group_box() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
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
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
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
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
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
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
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
fn render_targets_for_selected_group_include_child_molecule_labels() {
    let document = grouped_labeled_molecule_document();
    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");

    let primitives = engine.render_targets(
        &BTreeSet::new(),
        &BTreeSet::new(),
        &BTreeSet::from(["grp_1".to_string()]),
    );

    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Text {
                    role: RenderRole::DocumentText,
                    object_id,
                    node_id,
                    ..
                } if object_id.as_deref() == Some("obj_molecule_001")
                    && node_id.as_deref() == Some("n2")
            )
        }),
        "rendering a selected group target must include child molecule label primitives"
    );
    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Line {
                    role: RenderRole::DocumentBond,
                    object_id,
                    bond_id,
                    ..
                }
                | RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond,
                    object_id,
                    bond_id,
                    ..
                } if object_id.as_deref() == Some("obj_molecule_001")
                    && bond_id.as_deref() == Some("b1")
            )
        }),
        "rendering a selected group target must include child molecule bond primitives"
    );
}

#[test]
fn moving_selected_grouped_molecule_moves_child_label_world_position() {
    let document = grouped_labeled_molecule_document();
    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");
    assert!(engine.select_component_at_point(Point::new(70.0, 30.0), false));
    assert_eq!(engine.state().selection.arrow_objects, vec!["grp_1"]);

    let before_x = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                role: RenderRole::DocumentText,
                node_id,
                x,
                ..
            } if node_id.as_deref() == Some("n2") => Some(x),
            _ => None,
        })
        .expect("label should render before move");

    assert!(engine.begin_selection_move_at_point(Point::new(70.0, 30.0), false, false));
    assert!(engine.update_selection_move(Point::new(82.0, 35.0), false));
    assert!(engine.finish_selection_move(Point::new(82.0, 35.0), false));

    let after_x = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                role: RenderRole::DocumentText,
                node_id,
                x,
                ..
            } if node_id.as_deref() == Some("n2") => Some(x),
            _ => None,
        })
        .expect("label should render after move");

    assert_close(after_x, before_x + 12.0);
}

#[test]
fn double_click_grouped_molecule_bond_selects_group() {
    let document = grouped_two_fragment_document();
    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("document should load");

    engine.select_at_point(Point::new(120.0, 20.0), false);
    assert_eq!(engine.state().selection.bonds, vec!["b_grouped"]);

    assert!(engine.select_component_at_point(Point::new(120.0, 20.0), false));
    assert_eq!(engine.state().selection.arrow_objects, vec!["obj_group"]);
    assert!(engine.state().selection.bonds.is_empty());
}

#[test]
fn engine_groups_and_ungroups_selected_scene_objects_without_geometry_drift() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
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
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
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
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
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
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
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
