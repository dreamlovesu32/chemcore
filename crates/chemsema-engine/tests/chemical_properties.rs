use chemsema_engine::{
    document_to_cdx, document_to_cdxml, parse_cdx_document, parse_cdxml_document,
    ChemicalPropertyCalculationState, ChemicalPropertyValueOrigin, CommandTargetSet, EditorCommand,
    Engine, LinkPolicy,
};
use serde_json::{json, Value};

fn property_cdxml(active: bool) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="14.4" LabelFont="3" LabelSize="10" CaptionFont="3" CaptionSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 220 160">
    <fragment id="10" BoundingBox="30 30 80 55">
      <n id="11" p="40 45"/>
      <n id="12" p="65 45"/>
      <b id="13" B="11" E="12" Order="1"/>
    </fragment>
    <t id="20" p="42 85" BoundingBox="42 76 96 88" CaptionJustification="Left" UTF8Text="ethane">
      <s font="3" size="10" color="0">ethane</s>
    </t>
    <chemicalproperty id="30" ChemicalPropertyType="ChemicalName"
      ChemicalPropertyDisplayID="20" ChemicalPropertyIsActive="{}"
      BasisObjects="10 11 13"/>
  </page>
</CDXML>"#,
        if active { "yes" } else { "no" }
    )
}

#[test]
fn cdxml_import_promotes_official_fields_without_changing_display_text() {
    let document = parse_cdxml_document(&property_cdxml(true), None).unwrap();
    let property = &document.chemical_properties[0];
    assert_eq!(property.id, "chemical_property_30");
    assert_eq!(property.source_id.as_deref(), Some("30"));
    assert!(property.property_type.is_chemical_name());
    assert!(property.is_active);
    assert_eq!(
        property.calculation_state,
        ChemicalPropertyCalculationState::Stale
    );
    assert_eq!(property.basis_entity_ids, vec!["obj_mol_001", "11", "13"]);
    let display_id = property.display_object_id.as_deref().unwrap();
    let display = document.find_scene_object(display_id).unwrap();
    assert_eq!(
        display.payload.extra.get("text").and_then(Value::as_str),
        Some("ethane")
    );
    assert_eq!(display.link_policy, LinkPolicy::Linked);
    assert!(document.links.iter().any(|relation| {
        relation.kind == "chemical-property-display"
            && relation.data["chemicalPropertyId"] == property.id
    }));
}

#[test]
fn cdxml_and_cdx_export_rewrite_live_object_references() {
    let document = parse_cdxml_document(&property_cdxml(false), None).unwrap();
    let exported = document_to_cdxml(&document);
    assert!(exported.contains(r#"ChemicalPropertyDisplayID="20""#));
    assert_eq!(exported.matches(r#"id="20""#).count(), 1);
    let reparsed = parse_cdxml_document(&exported, None).unwrap();
    assert_eq!(reparsed.chemical_properties.len(), 1);
    let property = &reparsed.chemical_properties[0];
    assert_eq!(property.basis_entity_ids.len(), 3);
    assert!(property.display_object_id.is_some());
    assert!(!property.is_active);

    let cdx = document_to_cdx(&document).unwrap();
    let from_cdx = parse_cdx_document(&cdx, None).unwrap();
    assert_eq!(from_cdx.chemical_properties.len(), 1);
    let property = &from_cdx.chemical_properties[0];
    assert!(property.property_type.is_chemical_name());
    assert_eq!(property.basis_entity_ids.len(), 3);
    assert!(property.display_object_id.is_some());
}

#[test]
fn active_property_moves_without_recalculation_and_structure_changes_become_stale() {
    let mut engine = Engine::new();
    engine.load_cdxml_document(&property_cdxml(true)).unwrap();
    engine
        .apply_chemical_property_result_json(
            &json!({"propertyId": "chemical_property_30", "value": "ethane"}).to_string(),
        )
        .unwrap();
    assert_eq!(
        engine.state().document.chemical_properties[0].calculation_state,
        ChemicalPropertyCalculationState::Current
    );

    engine
        .execute_command(EditorCommand::MoveTargets {
            targets: CommandTargetSet {
                objects: vec!["obj_mol_001".to_string()],
                ..CommandTargetSet::default()
            },
            delta: chemsema_engine::CommandDelta { dx: 8.0, dy: 3.0 },
        })
        .unwrap();
    assert_eq!(
        engine.state().document.chemical_properties[0].calculation_state,
        ChemicalPropertyCalculationState::Current
    );

    engine
        .execute_command(EditorCommand::ReplaceNodeLabel {
            node_id: "12".to_string(),
            label: "N".to_string(),
        })
        .unwrap();
    assert_eq!(
        engine.state().document.chemical_properties[0].calculation_state,
        ChemicalPropertyCalculationState::Stale
    );
    let requests: Value =
        serde_json::from_str(&engine.chemical_property_requests_json().unwrap()).unwrap();
    assert_eq!(requests.as_array().unwrap().len(), 1);
    assert_eq!(requests[0]["propertyId"], "chemical_property_30");
}

#[test]
fn dialog_creation_manual_edit_and_delete_have_explicit_history() {
    let mut engine = Engine::new();
    engine.load_cdxml_document(&property_cdxml(false)).unwrap();
    assert!(engine.select_component_at_point(chemsema_engine::Point::new(52.0, 45.0), false,));
    let molecule_menu: Value =
        serde_json::from_str(&engine.context_menu_json(r#"{"kind":"canvas"}"#, false)).unwrap();
    assert!(molecule_menu
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["command"] == "chemical-property-dialog"));
    assert_ne!(
        engine.chemical_property_dialog_json(),
        "null",
        "selection: {:?}",
        engine.state().selection
    );
    assert!(engine
        .apply_chemical_property_dialog_json(
            &json!({
                "typeCode": 1,
                "typeName": "ChemicalName",
                "value": "ethane",
                "isActive": true
            })
            .to_string()
        )
        .unwrap());
    assert_eq!(engine.state().document.chemical_properties.len(), 2);
    let property = engine.state().document.chemical_properties.last().unwrap();
    assert_eq!(property.value_origin, ChemicalPropertyValueOrigin::Authored);
    assert!(engine.can_undo());
    let property_menu: Value =
        serde_json::from_str(&engine.context_menu_json(r#"{"kind":"text"}"#, false)).unwrap();
    assert!(property_menu
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["command"] == "chemical-property-dialog"));

    let display_id = property.display_object_id.clone().unwrap();
    let display = engine
        .state()
        .document
        .find_scene_object(&display_id)
        .unwrap();
    let point = chemsema_engine::Point::new(
        display.transform.translate[0] + 2.0,
        display.transform.translate[1] + 2.0,
    );
    let mut session = engine.begin_text_edit(point).unwrap();
    session.text = "edited name".to_string();
    assert!(engine.apply_text_edit(session));
    let property = engine.state().document.chemical_properties.last().unwrap();
    assert!(!property.is_active);
    assert_eq!(
        property.calculation_state,
        ChemicalPropertyCalculationState::Static
    );
    assert_eq!(property.value_origin, ChemicalPropertyValueOrigin::Authored);

    engine.select_at_point(point, false);
    assert!(engine.delete_selected_chemical_property());
    assert_eq!(engine.state().document.chemical_properties.len(), 1);
    assert!(engine.undo());
    assert_eq!(engine.state().document.chemical_properties.len(), 2);
}

#[test]
fn copy_paste_remaps_property_display_basis_and_relation_ids() {
    let mut engine = Engine::new();
    engine.load_cdxml_document(&property_cdxml(false)).unwrap();
    assert!(engine.select_all());
    assert!(engine.copy_selection());
    assert!(engine.paste_clipboard());
    assert_eq!(engine.state().document.chemical_properties.len(), 2);
    let first = &engine.state().document.chemical_properties[0];
    let second = &engine.state().document.chemical_properties[1];
    assert_ne!(first.id, second.id);
    assert_ne!(first.display_object_id, second.display_object_id);
    assert_ne!(first.basis_entity_ids, second.basis_entity_ids);
    assert!(engine.state().document.links.iter().any(|relation| {
        relation.kind == "chemical-property-display"
            && relation.data["chemicalPropertyId"] == second.id
    }));
}

#[test]
fn empty_property_and_source_or_display_deletion_follow_explicit_rules() {
    let empty = parse_cdxml_document(
        r#"<CDXML><page id="1"><chemicalproperty id="7" ChemicalPropertyIsActive="yes"/></page></CDXML>"#,
        None,
    )
    .unwrap();
    let property = &empty.chemical_properties[0];
    assert_eq!(property.property_type.code, None);
    assert_eq!(property.property_type.name, None);
    assert_eq!(
        property.calculation_state,
        ChemicalPropertyCalculationState::Unsupported
    );
    assert!(property.basis_entity_ids.is_empty());
    assert!(property.display_object_id.is_none());

    let mut display_deleted = Engine::new();
    display_deleted
        .load_cdxml_document(&property_cdxml(false))
        .unwrap();
    let display_id = display_deleted.state().document.chemical_properties[0]
        .display_object_id
        .clone()
        .unwrap();
    let display = display_deleted
        .state()
        .document
        .find_scene_object(&display_id)
        .unwrap();
    let display_point = chemsema_engine::Point::new(
        display.transform.translate[0] + 2.0,
        display.transform.translate[1] + 2.0,
    );
    display_deleted.select_at_point(display_point, false);
    assert!(display_deleted.delete_selection());
    assert_eq!(
        display_deleted.state().document.chemical_properties.len(),
        1
    );
    assert!(display_deleted.state().document.chemical_properties[0]
        .display_object_id
        .is_none());
    assert!(display_deleted
        .state()
        .document
        .find_scene_object(&display_id)
        .is_none());

    let mut source_deleted = Engine::new();
    source_deleted
        .load_cdxml_document(&property_cdxml(false))
        .unwrap();
    assert!(
        source_deleted.select_component_at_point(chemsema_engine::Point::new(52.0, 45.0), false,)
    );
    assert!(source_deleted.delete_selection());
    assert!(source_deleted
        .state()
        .document
        .chemical_properties
        .is_empty());
    assert!(source_deleted
        .state()
        .document
        .scene_objects()
        .iter()
        .any(|object| object.object_type == "text"));
}

#[test]
fn cdx_custom_property_types_require_and_preserve_official_numeric_codes() {
    let numeric = parse_cdxml_document(
        r#"<CDXML><page id="1"><chemicalproperty id="7" ChemicalPropertyType="32769"/></page></CDXML>"#,
        None,
    )
    .unwrap();
    let cdx = document_to_cdx(&numeric).expect("numeric custom type should be CDX-safe");
    let reopened = parse_cdx_document(&cdx, None).unwrap();
    assert_eq!(
        reopened.chemical_properties[0].property_type.code,
        Some(0x8001)
    );

    let named = parse_cdxml_document(
        r#"<CDXML><page id="1"><chemicalproperty id="7" ChemicalPropertyType="VendorProperty"/></page></CDXML>"#,
        None,
    )
    .unwrap();
    let error = document_to_cdx(&named).unwrap_err();
    assert!(error.contains("CDXML-only named custom type"));
    assert!(error.contains("greater than 0x8000"));
}

#[test]
fn empty_declared_display_text_keeps_its_identity_and_position() {
    let document = parse_cdxml_document(
        r#"<CDXML><page id="1">
          <t id="20" p="42 85" BoundingBox="42 76 42.05 76.05"><s></s></t>
          <chemicalproperty id="30" ChemicalPropertyDisplayID="20"/>
        </page></CDXML>"#,
        None,
    )
    .unwrap();
    let property = &document.chemical_properties[0];
    let display_id = property
        .display_object_id
        .as_deref()
        .expect("empty standard display object must remain addressable");
    let display = document.find_scene_object(display_id).unwrap();
    assert_eq!(display.payload.extra["text"], "");
    assert_eq!(display.meta["textId"], "20");

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("ChemicalPropertyDisplayID="));
    let reopened = parse_cdxml_document(&exported, None).unwrap();
    assert!(reopened.chemical_properties[0].display_object_id.is_some());
}

#[test]
fn basis_aliases_resolve_nested_labels_containers_and_superseded_graphics() {
    let document = parse_cdxml_document(
        r#"<CDXML><page id="1">
          <fragment id="10"><n id="11" p="10 10"><t id="12" p="10 10"><s>N</s></t></n></fragment>
          <graphic id="20" SupersededBy="21" GraphicType="Line" BoundingBox="0 0 10 0"/>
          <arrow id="21" BoundingBox="0 0 10 0"/>
          <chemicalproperty id="30" BasisObjects="10 11 12 20 21"/>
        </page></CDXML>"#,
        None,
    )
    .unwrap();
    let property = &document.chemical_properties[0];
    assert!(
        property.unresolved_basis_ids.is_empty(),
        "{:?}",
        property.unresolved_basis_ids
    );
    assert!(!property.basis_entity_ids.is_empty());
    let exported = document_to_cdxml(&document);
    let reopened = parse_cdxml_document(&exported, None).unwrap();
    assert!(reopened.chemical_properties[0]
        .unresolved_basis_ids
        .is_empty());
}

#[test]
fn command_json_can_create_edit_and_delete_the_native_property() {
    let mut engine = Engine::new();
    engine.load_cdxml_document(&property_cdxml(false)).unwrap();
    assert!(engine.select_component_at_point(chemsema_engine::Point::new(52.0, 45.0), false,));
    engine
        .execute_command_json(
            r#"{
              "type":"apply-chemical-property",
              "property_type":{"code":1,"name":"ChemicalName"},
              "value":"ethane",
              "isActive":true
            }"#,
        )
        .unwrap();
    let property_id = engine.state().document.chemical_properties[1].id.clone();
    engine
        .execute_command_json(
            &json!({
                "type": "delete-chemical-property",
                "propertyId": property_id
            })
            .to_string(),
        )
        .unwrap();
    assert_eq!(engine.state().document.chemical_properties.len(), 1);
}
