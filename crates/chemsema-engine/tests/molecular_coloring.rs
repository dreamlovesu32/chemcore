use chemsema_engine::{
    document_to_cdx, document_to_cdxml, parse_cdx_document, parse_cdxml_document, render_document,
    render_document_targets, Engine, Point, RenderPrimitive, RenderRole,
};
use serde_json::Value;
use std::collections::BTreeSet;

const COLORED_BENZENE: &str = r##"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="30" LineWidth="1" BoldWidth="2.6" MarginWidth="2">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="1" g="0" b="0"/>
    <color r="1" g="1" b="0"/>
    <color r="0" g="1" b="0"/>
  </colortable>
  <page id="1" BoundingBox="0 0 200 180">
    <fragment id="10" BoundingBox="40 30 130 110">
      <n id="11" p="85 30" highlightColor="6"/>
      <n id="12" p="124 52.5" highlightColor="6"/>
      <n id="13" p="124 97.5" highlightColor="6"/>
      <n id="14" p="85 120" highlightColor="6"/>
      <n id="15" p="46 97.5" highlightColor="6"/>
      <n id="16" p="46 52.5" highlightColor="6"/>
      <b id="21" B="11" E="12" Order="1" highlightColor="6"/>
      <b id="22" B="12" E="13" Order="2" highlightColor="6"/>
      <b id="23" B="13" E="14" Order="1" highlightColor="6"/>
      <b id="24" B="14" E="15" Order="2" highlightColor="6"/>
      <b id="25" B="15" E="16" Order="1" highlightColor="6"/>
      <b id="26" B="16" E="11" Order="2" highlightColor="6"/>
      <ColoredMolecularArea id="30" bgcolor="5" BasisObjects="21 22 23 24 25 26"/>
    </fragment>
  </page>
</CDXML>"##;

const FUSED_RINGS: &str = r##"<CDXML BondLength="30">
  <page id="1">
    <fragment id="10">
      <n id="a" p="30 60"/><n id="b" p="45 34"/><n id="c" p="75 34"/>
      <n id="d" p="90 60"/><n id="e" p="75 86"/><n id="f" p="45 86"/>
      <n id="g" p="105 8"/><n id="h" p="135 8"/><n id="i" p="150 34"/>
      <n id="j" p="120 60"/>
      <b id="ab" B="a" E="b"/><b id="bc" B="b" E="c"/>
      <b id="cd" B="c" E="d"/><b id="de" B="d" E="e"/>
      <b id="ef" B="e" E="f"/><b id="fa" B="f" E="a"/>
      <b id="cg" B="c" E="g"/><b id="gh" B="g" E="h"/>
      <b id="hi" B="h" E="i"/><b id="ij" B="i" E="j"/>
      <b id="jd" B="j" E="d"/>
    </fragment>
  </page>
</CDXML>"##;

fn fragment(document: &chemsema_engine::ChemSemaDocument) -> &chemsema_engine::MoleculeFragment {
    document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("molecule fragment")
}

#[test]
fn cdxml_and_cdx_roundtrip_native_molecular_colors() {
    let document = parse_cdxml_document(COLORED_BENZENE, None).unwrap();
    let imported_fragment = fragment(&document);
    assert!(imported_fragment
        .nodes
        .iter()
        .all(|node| node.highlight_color.as_deref() == Some("#00ff00")));
    assert!(imported_fragment
        .bonds
        .iter()
        .all(|bond| bond.highlight_color.as_deref() == Some("#00ff00")));
    assert_eq!(imported_fragment.colored_areas.len(), 1);
    assert_eq!(imported_fragment.colored_areas[0].color, "#ffff00");

    let cdxml = document_to_cdxml(&document);
    assert_eq!(cdxml.matches("highlightColor=").count(), 12);
    assert_eq!(cdxml.matches("<ColoredMolecularArea ").count(), 1);
    let reopened = parse_cdxml_document(&cdxml, None).unwrap();
    assert_eq!(fragment(&reopened).colored_areas.len(), 1);

    let cdx = document_to_cdx(&document).unwrap();
    assert!(cdx
        .windows(6)
        .any(|bytes| bytes == [0x08, 0x03, 0x02, 0x00, 0x06, 0x00]));
    assert!(cdx.windows(2).any(|bytes| bytes == [0x32, 0x80]));
    let from_cdx = parse_cdx_document(&cdx, None).unwrap();
    assert_eq!(fragment(&from_cdx).colored_areas.len(), 1);
    assert_eq!(
        fragment(&from_cdx).nodes[0].highlight_color.as_deref(),
        Some("#00ff00")
    );
}

#[test]
fn renderer_uses_chemdraw_radius_and_live_ring_geometry() {
    let document = parse_cdxml_document(COLORED_BENZENE, None).unwrap();
    let primitives = render_document(&document);
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle {
            role: RenderRole::DocumentMolecularColor,
            radius,
            ..
        } if (*radius - 4.6).abs() < 1e-9
    )));
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Polyline {
            role: RenderRole::DocumentMolecularColor,
            stroke_width,
            line_cap: Some(cap),
            ..
        } if (*stroke_width - 9.2).abs() < 1e-9 && cap == "round"
    )));
    let ring = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentMolecularColor,
                points,
                fill,
                ..
            } if fill == "#ffff00" => Some(points),
            _ => None,
        })
        .expect("ring fill polygon");
    assert_eq!(ring.len(), 6);

    let targeted = render_document_targets(
        &document,
        &BTreeSet::from(["13".to_string()]),
        &BTreeSet::new(),
        &BTreeSet::new(),
    );
    assert!(targeted.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Polygon {
            role: RenderRole::DocumentMolecularColor,
            bond_id: Some(bond_id),
            fill,
            ..
        } if bond_id == "21" && fill == "#ffff00"
    )));
}

#[test]
fn right_click_commands_apply_remove_and_offer_ring_fill_only_for_a_complete_ring() {
    let mut engine = Engine::new();
    engine.load_cdxml_document(COLORED_BENZENE).unwrap();
    assert!(engine.select_component_at_point(Point::new(85.0, 30.0), false));

    let menu: Value =
        serde_json::from_str(&engine.context_menu_json(r#"{"kind":"atom","nodeId":"11"}"#, false))
            .unwrap();
    let labels = menu
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item.get("label").and_then(Value::as_str))
        .collect::<Vec<_>>();
    assert!(labels.contains(&"Highlight"));
    assert!(labels.contains(&"Ring Fill"));

    engine
        .execute_command_json(r##"{"type":"apply-molecular-highlight","color":"#ff0000"}"##)
        .unwrap();
    let ring_result: Value = serde_json::from_str(
        &engine
            .execute_command_json(r##"{"type":"apply-ring-fill","color":"#00ffff"}"##)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(ring_result["changed"], true);
    assert_eq!(ring_result["updated"]["bonds"].as_array().unwrap().len(), 6);
    let colored_fragment = fragment(&engine.state().document);
    assert!(colored_fragment
        .nodes
        .iter()
        .all(|node| node.highlight_color.as_deref() == Some("#ff0000")));
    assert_eq!(colored_fragment.colored_areas[0].color, "#00ffff");
    assert!(engine.undo());
    assert_eq!(
        fragment(&engine.state().document).colored_areas[0].color,
        "#ffff00"
    );
    assert!(engine.redo());
    assert_eq!(
        fragment(&engine.state().document).colored_areas[0].color,
        "#00ffff"
    );
    assert!(engine.select_component_at_point(Point::new(85.0, 30.0), false));

    engine
        .execute_command_json(r#"{"type":"apply-molecular-highlight","color":null}"#)
        .unwrap();
    engine
        .execute_command_json(r#"{"type":"apply-ring-fill","color":null}"#)
        .unwrap();
    let cleared_fragment = fragment(&engine.state().document);
    assert!(cleared_fragment
        .nodes
        .iter()
        .all(|node| node.highlight_color.is_none()));
    assert!(cleared_fragment.colored_areas.is_empty());
}

#[test]
fn partial_ring_selection_hides_ring_fill_and_bond_deletion_prunes_the_area() {
    let mut engine = Engine::new();
    engine.load_cdxml_document(COLORED_BENZENE).unwrap();
    engine.select_at_point(Point::new(104.5, 41.25), false);
    let menu: Value =
        serde_json::from_str(&engine.context_menu_json(r#"{"kind":"bond","bondId":"21"}"#, false))
            .unwrap();
    let labels = menu
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item.get("label").and_then(Value::as_str))
        .collect::<Vec<_>>();
    assert!(labels.contains(&"Highlight"));
    assert!(!labels.contains(&"Ring Fill"));

    engine
        .execute_command_json(r#"{"type":"delete-selection"}"#)
        .unwrap();
    let fragment = fragment(&engine.state().document);
    assert!(fragment.colored_areas.is_empty());
}

#[test]
fn fused_selection_fills_each_chordless_ring_not_the_outer_perimeter() {
    let mut engine = Engine::new();
    engine.load_cdxml_document(FUSED_RINGS).unwrap();
    assert!(engine.select_component_at_point(Point::new(60.0, 34.0), false));
    engine
        .execute_command_json(r##"{"type":"apply-ring-fill","color":"#00ff00"}"##)
        .unwrap();
    let areas = &fragment(&engine.state().document).colored_areas;
    assert_eq!(areas.len(), 2);
    assert!(areas.iter().all(|area| area.basis_bonds.len() == 6));
}
