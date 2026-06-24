use chemcore_engine::{
    document_to_cdxml, parse_cdxml_document, render_document, Bond, BondLinePattern,
    ChemcoreDocument, Engine, Point, RenderPrimitive, RenderRole, ResourceData,
};
use serde_json::json;

mod support;
use support::read_optional_cdxml_fixture;

#[test]
fn parse_cdxml_imports_rest_fixture_special_bonds_and_table() {
    let Some(cdxml) = read_optional_cdxml_fixture("rest.cdxml") else {
        return;
    };
    let document = parse_cdxml_document(&cdxml, Some("rest")).expect("rest cdxml should parse");

    let bonds: Vec<_> = document
        .resources
        .values()
        .filter_map(|resource| match &resource.data {
            ResourceData::Fragment(fragment) => Some(fragment.bonds.iter()),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(bonds.len(), 2);
    assert!(bonds
        .iter()
        .any(|bond| bond.line_styles.main == BondLinePattern::Wavy));
    assert!(bonds.iter().any(|bond| {
        bond.stereo
            .as_ref()
            .is_some_and(|stereo| stereo.kind == "hollow-wedge" && stereo.wide_end == "end")
    }));

    let table = document
        .objects
        .iter()
        .find(|object| object.id == "obj_shape_table_001")
        .expect("table shape should import");
    assert_eq!(table.object_type, "shape");
    assert_eq!(table.payload.extra.get("kind"), Some(&json!("crossTable")));
    let tlc = document
        .objects
        .iter()
        .find(|object| object.id == "obj_shape_tlc_001")
        .expect("tlc plate should import");
    assert_eq!(tlc.object_type, "shape");
    assert_eq!(tlc.payload.extra.get("kind"), Some(&json!("tlcPlate")));
    assert_eq!(tlc.transform.translate, [365.29, 138.75]);
    assert_eq!(tlc.payload.bbox, Some([0.0, 0.0, 102.37, 172.13]));
    let lanes = tlc
        .payload
        .extra
        .get("lanes")
        .and_then(serde_json::Value::as_array)
        .expect("tlc plate should preserve lanes");
    assert_eq!(lanes.len(), 9, "{lanes:?}");
    assert!(lanes.iter().all(|lane| {
        lane.get("spots")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|spots| !spots.is_empty())
    }));

    let primitives = render_document(&document);
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Path {
            role: RenderRole::DocumentBond,
            bond_id: Some(bond_id),
            d,
            line_join,
            ..
        } if bond_id == "28" && d.contains(" A ") && line_join.as_deref() == Some("bevel")
    )));
    let hollow_wedge_paths: Vec<_> = primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Path {
                    role: RenderRole::DocumentBond,
                    bond_id: Some(bond_id),
                    line_join,
                    ..
                } if bond_id == "32" && line_join.as_deref() == Some("miter")
            )
        })
        .collect();
    assert_eq!(hollow_wedge_paths.len(), 1, "{hollow_wedge_paths:?}");
    let table_graphics: Vec<_> = primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Path {
                    role: RenderRole::DocumentGraphic,
                    object_id: Some(object_id),
                    ..
                } if object_id == "obj_shape_table_001"
            )
        })
        .collect();
    assert_eq!(table_graphics.len(), 3, "{table_graphics:?}");
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object_id),
            ..
        } if object_id == "obj_shape_tlc_001"
    )));
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object_id),
            ..
        } if object_id == "obj_shape_tlc_001"
    )));
}

#[test]
fn parse_cdxml_exports_rest_fixture_special_bonds_and_table_tags() {
    let Some(cdxml) = read_optional_cdxml_fixture("rest.cdxml") else {
        return;
    };
    let document = parse_cdxml_document(&cdxml, Some("rest")).expect("rest cdxml should parse");
    let exported = document_to_cdxml(&document);

    assert!(exported.contains("Display=\"Wavy\""), "{exported}");
    assert!(
        exported.contains("Display=\"HollowWedgeBegin\""),
        "{exported}"
    );
    assert!(exported.contains("<table "), "{exported}");
    assert!(exported.contains("<tlcplate"), "{exported}");
    assert!(exported.contains("<tlclane"), "{exported}");
    assert!(exported.contains("<tlcspot"), "{exported}");
    assert!(exported.contains("BoundsInParent="), "{exported}");
}

#[test]
fn load_cdxml_document_preserves_rest_fixture_tlc_plate_for_editing() {
    let Some(cdxml) = read_optional_cdxml_fixture("rest.cdxml") else {
        return;
    };
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(&cdxml)
        .expect("rest cdxml should load");

    let tlc = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.id == "obj_shape_tlc_001")
        .expect("loaded editing document should keep tlc plate");
    assert_eq!(tlc.payload.extra.get("kind"), Some(&json!("tlcPlate")));
    let lanes = tlc
        .payload
        .extra
        .get("lanes")
        .and_then(serde_json::Value::as_array)
        .expect("loaded tlc plate should preserve lanes");
    assert_eq!(lanes.len(), 9, "{lanes:?}");
}

#[test]
fn parse_cdxml_imports_orbital_fixture_templates_and_styles() {
    let Some(cdxml) = read_optional_cdxml_fixture("orbital.cdxml") else {
        return;
    };
    let document =
        parse_cdxml_document(&cdxml, Some("orbital")).expect("orbital cdxml should parse");

    let orbitals: Vec<_> = document
        .objects
        .iter()
        .filter(|object| object.payload.extra.get("kind") == Some(&json!("orbital")))
        .collect();
    assert_eq!(orbitals.len(), 21, "{orbitals:#?}");

    let p_default = orbitals
        .iter()
        .find(|object| object.id == "obj_shape_orbital_004")
        .expect("p orbital should import");
    assert_eq!(
        p_default.payload.extra.get("orbitalTemplate"),
        Some(&json!("p"))
    );
    assert_eq!(
        p_default.payload.extra.get("orbitalStyle"),
        Some(&json!("shaded"))
    );
    assert_eq!(
        p_default.payload.extra.get("orbitalPhase"),
        Some(&json!("plus"))
    );

    let hybrid_minus = orbitals
        .iter()
        .find(|object| object.id == "obj_shape_orbital_011")
        .expect("hybrid minus orbital should import");
    assert_eq!(
        hybrid_minus.payload.extra.get("orbitalTemplate"),
        Some(&json!("hybrid"))
    );
    assert_eq!(
        hybrid_minus.payload.extra.get("orbitalStyle"),
        Some(&json!("shaded"))
    );
    assert_eq!(
        hybrid_minus.payload.extra.get("orbitalPhase"),
        Some(&json!("minus"))
    );

    let dz2_plus_filled = orbitals
        .iter()
        .find(|object| object.id == "obj_shape_orbital_021")
        .expect("dz2 plus filled orbital should import");
    assert_eq!(
        dz2_plus_filled.payload.extra.get("orbitalTemplate"),
        Some(&json!("dz2"))
    );
    assert_eq!(
        dz2_plus_filled.payload.extra.get("orbitalStyle"),
        Some(&json!("filled"))
    );
    assert_eq!(
        dz2_plus_filled.payload.extra.get("orbitalPhase"),
        Some(&json!("plus"))
    );

    let primitives = render_document(&document);
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::FilledPath {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object_id),
            fill,
            ..
        } if object_id == "obj_shape_orbital_005" && fill == "#000000"
    )));
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::FilledPath {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object_id),
            fill,
            ..
        } if object_id == "obj_shape_orbital_004" && fill != "#ffffff"
    )));
}

#[test]
fn load_cdxml_document_preserves_orbital_axes_for_editing() {
    let Some(cdxml) = read_optional_cdxml_fixture("orbital.cdxml") else {
        return;
    };
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(&cdxml)
        .expect("orbital cdxml should load");

    let p_default = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.id == "obj_shape_orbital_004")
        .expect("p orbital should exist");
    let axis_start = p_default
        .payload
        .extra
        .get("axisStart")
        .and_then(serde_json::Value::as_array)
        .expect("axisStart should be stored");
    let axis_end = p_default
        .payload
        .extra
        .get("axisEnd")
        .and_then(serde_json::Value::as_array)
        .expect("axisEnd should be stored");
    assert_eq!(axis_start[0].as_f64(), Some(235.47));
    assert_eq!(axis_start[1].as_f64(), Some(137.25));
    assert_eq!(axis_end[0].as_f64(), Some(235.47));
    assert_eq!(axis_end[1].as_f64(), Some(155.25));
}

#[test]
fn parse_cdxml_exports_orbital_fixture_orbital_tags() {
    let Some(cdxml) = read_optional_cdxml_fixture("orbital.cdxml") else {
        return;
    };
    let document =
        parse_cdxml_document(&cdxml, Some("orbital")).expect("orbital cdxml should parse");
    let exported = document_to_cdxml(&document);

    assert!(exported.contains("GraphicType=\"Orbital\""), "{exported}");
    assert!(exported.contains("OrbitalType=\"sShaded\""), "{exported}");
    assert!(exported.contains("OrbitalType=\"pFilled\""), "{exported}");
    assert!(
        exported.contains("OrbitalType=\"hybridMinus\""),
        "{exported}"
    );
    assert!(
        exported.contains("OrbitalType=\"dz2PlusFilled\""),
        "{exported}"
    );
    assert!(
        exported.contains("OrbitalType=\"lobeShaded\""),
        "{exported}"
    );
}

#[test]
fn tlc_plate_spot_drag_updates_rf() {
    let Some(cdxml) = read_optional_cdxml_fixture("rest.cdxml") else {
        return;
    };
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(&cdxml)
        .expect("rest cdxml should load");

    let tlc = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.id == "obj_shape_tlc_001")
        .expect("loaded editing document should keep tlc plate");
    let [_, _, width, height] = tlc.payload.bbox.expect("tlc bbox");
    let origin_fraction = tlc
        .payload
        .extra
        .get("originFraction")
        .and_then(serde_json::Value::as_f64)
        .expect("origin fraction");
    let solvent_fraction = tlc
        .payload
        .extra
        .get("solventFrontFraction")
        .and_then(serde_json::Value::as_f64)
        .expect("solvent fraction");
    let first_lane = tlc
        .payload
        .extra
        .get("lanes")
        .and_then(serde_json::Value::as_array)
        .and_then(|lanes| lanes.first())
        .expect("first tlc lane");
    let offset = first_lane
        .get("offset")
        .and_then(serde_json::Value::as_f64)
        .expect("lane offset");
    let initial_rf = first_lane
        .get("spots")
        .and_then(serde_json::Value::as_array)
        .and_then(|spots| spots.first())
        .and_then(|spot| spot.get("rf"))
        .and_then(serde_json::Value::as_f64)
        .expect("first tlc spot rf");
    let tx = tlc.transform.translate[0];
    let ty = tlc.transform.translate[1];
    let lane_x = tx + width * offset;
    let origin_y = ty + height * (1.0 - origin_fraction);
    let solvent_y = ty + height * solvent_fraction;
    let start = Point::new(lane_x, origin_y - (origin_y - solvent_y) * initial_rf);
    let target_rf = 0.25;
    let target = Point::new(lane_x, origin_y - (origin_y - solvent_y) * target_rf);

    let begin = engine
        .begin_tlc_spot_drag(start)
        .expect("spot drag should begin");
    assert_eq!(begin.object_id, "obj_shape_tlc_001");
    assert_eq!(begin.lane_index, 0);
    assert_eq!(begin.spot_index, 0);

    let updated = engine
        .update_tlc_spot_drag(target)
        .expect("spot drag should update");
    assert!(
        (updated.rf - target_rf).abs() < 0.02,
        "expected rf close to {target_rf}, got {updated:?}"
    );

    let finished = engine
        .finish_tlc_spot_drag(target)
        .expect("spot drag should finish");
    assert!(
        (finished.rf - target_rf).abs() < 0.02,
        "expected final rf close to {target_rf}, got {finished:?}"
    );

    let tlc_after = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.id == "obj_shape_tlc_001")
        .expect("tlc plate should remain in document");
    let rf_after = tlc_after
        .payload
        .extra
        .get("lanes")
        .and_then(serde_json::Value::as_array)
        .and_then(|lanes| lanes.first())
        .and_then(|lane| lane.get("spots"))
        .and_then(serde_json::Value::as_array)
        .and_then(|spots| spots.first())
        .and_then(|spot| spot.get("rf"))
        .and_then(serde_json::Value::as_f64)
        .expect("updated rf should persist");
    assert!(
        (rf_after - target_rf).abs() < 0.02,
        "expected persisted rf close to {target_rf}, got {rf_after}"
    );
}

#[test]
fn parse_cdxml_preserves_default_and_acs_hash_spacing_presets_for_dashed_bonds() {
    for (fixture, expected_hash_spacing) in [("dash.cdxml", 2.7), ("dash-acs.cdxml", 2.5)] {
        let Some(cdxml) = read_optional_cdxml_fixture(fixture) else {
            continue;
        };
        let document = parse_cdxml_document(&cdxml, Some(fixture)).expect("cdxml should parse");
        let defaults = &document.document.meta["import"]["cdxml"]["defaults"];
        assert_eq!(
            defaults["hashSpacing"].as_f64(),
            Some(expected_hash_spacing),
            "{fixture} defaults"
        );

        let bond = imported_fragment_bond(&document, "obj_mol_001", "85");
        assert_eq!(
            bond.hash_spacing,
            Some(expected_hash_spacing),
            "{fixture} bond hash spacing"
        );
        assert_eq!(bond.margin_width, None, "{fixture} bond margin width");
        assert_eq!(
            bond.label_clip_margin, None,
            "{fixture} bond label clip margin"
        );
        assert_eq!(
            bond.line_styles.main,
            BondLinePattern::Dashed,
            "{fixture} dashed bond style"
        );
    }
}

#[test]
fn parse_cdxml_uses_document_hash_spacing_for_dashed_lines() {
    for (fixture, expected_dash) in [("dash.cdxml", 2.7_f64), ("dash-acs.cdxml", 2.5_f64)] {
        let Some(cdxml) = read_optional_cdxml_fixture(fixture) else {
            continue;
        };
        let mut engine = Engine::new();
        engine
            .load_cdxml_document(&cdxml)
            .expect("cdxml should load into engine");
        let svg = engine.document_svg();
        let expected_dash = (expected_dash * 100.0_f64).round() / 100.0;
        let expected_dash_attr = format!("stroke-dasharray=\"{expected_dash}\"");

        assert!(
            svg.contains(&expected_dash_attr),
            "{fixture} expected {expected_dash_attr} in {svg}"
        );
        assert!(svg.contains("stroke-linecap=\"butt\""), "{fixture} cap");
        assert!(svg.contains("stroke-linejoin=\"miter\""), "{fixture} join");
    }
}

#[test]
fn tlc_plate_guides_use_document_hash_spacing() {
    let Some(cdxml) = read_optional_cdxml_fixture("rest.cdxml") else {
        return;
    };
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(&cdxml)
        .expect("cdxml should load into engine");
    let svg = engine.document_svg();
    let expected_dash = (2.7_f64 * 100.0).round() / 100.0;
    let expected_dash_attr = format!("stroke-dasharray=\"{expected_dash}\"");
    let dash_occurrences = svg.matches(&expected_dash_attr).count();

    assert!(
        dash_occurrences >= 2,
        "expected at least 2 TLC dash guides with {expected_dash_attr}, svg={svg}"
    );
}

fn imported_fragment_bond<'a>(
    document: &'a ChemcoreDocument,
    object_id: &str,
    bond_id: &str,
) -> &'a Bond {
    let fragment_ref = document
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .and_then(|object| object.payload.resource_ref.as_deref())
        .expect("molecule object should reference fragment resource");
    document
        .resources
        .get(fragment_ref)
        .and_then(|resource| match &resource.data {
            ResourceData::Fragment(fragment) => Some(fragment),
            _ => None,
        })
        .and_then(|fragment| fragment.bonds.iter().find(|bond| bond.id == bond_id))
        .unwrap_or_else(|| panic!("bond {bond_id} missing from fragment {fragment_ref}"))
}
