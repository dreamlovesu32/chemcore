use chemsema_engine::{
    document_to_cdx, document_to_cdxml, parse_cdxml_document, render_document, Engine, LinkPolicy,
    StoichiometryBindingState, StoichiometryCalculationState, StoichiometryValueOrigin,
};
use serde_json::json;

const STOICHIOMETRY_CDXML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BoundingBox="0 0 500 400" LineWidth="0.75" BoldWidth="1.5"
 LabelFont="3" LabelSize="10" CaptionFont="3" CaptionSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 500 400">
    <fragment id="10" BoundingBox="30 40 90 80">
      <n id="11" p="40 60" Element="6"/>
      <n id="12" p="70 60" Element="8"/>
      <b id="13" B="11" E="12" Order="1"/>
    </fragment>
    <fragment id="20" BoundingBox="250 40 310 80">
      <n id="21" p="260 60" Element="6"/>
      <n id="22" p="290 60" Element="7"/>
      <b id="23" B="21" E="22" Order="1"/>
    </fragment>
    <graphic id="30" GraphicType="Line" ArrowType="FullHead"
      BoundingBox="120 60 220 60" Head3D="220 60 0" Tail3D="120 60 0"/>
    <scheme id="40">
      <step id="41" ReactionStepReactants="10" ReactionStepProducts="20"
        ReactionStepArrows="30"/>
    </scheme>
    <stoichiometrygrid id="50" BoundingBox="60 130 360 310"
      LineWidth="0.75" BoldWidth="1.5" MarginWidth="2" LabelFont="3"
      LabelSize="10" LabelFace="0" color="2">
      <sgcomponent id="51" ComponentIsHeader="yes" Visible="yes" Width="86">
        <sgdatum id="511" SGPropertyType="Mass" SGDataType="Number"
          SGDataValue="" IsReadOnly="no"/>
      </sgcomponent>
      <sgcomponent id="52" ComponentIsReactant="yes" ComponentReferenceID="10"
        Visible="yes" Width="72">
        <sgdatum id="521" SGPropertyType="MolecularWeight" SGDataType="Number"
          SGDataValue="44.01" IsReadOnly="yes"/>
        <sgdatum id="522" SGPropertyType="Mass" SGDataType="Number"
          SGDataValue="100" IsEdited="yes"/>
      </sgcomponent>
      <sgcomponent id="53" ComponentIsReactant="no" ComponentReferenceID="20"
        Visible="yes" Width="72">
        <sgdatum id="531" SGPropertyType="MolecularWeight" SGDataType="Number"
          SGDataValue="41.05" IsReadOnly="yes"/>
      </sgcomponent>
    </stoichiometrygrid>
  </page>
</CDXML>"#;

#[test]
fn cdxml_import_builds_native_reaction_and_stoichiometry_models() {
    let document =
        parse_cdxml_document(STOICHIOMETRY_CDXML, Some("stoichiometry")).expect("valid CDXML");
    let step = &document.reaction_schemes[0].steps[0];
    assert_eq!(step.reactant_entity_ids.len(), 1);
    assert_eq!(step.product_entity_ids.len(), 1);
    assert_eq!(step.arrow_object_ids.len(), 1);
    let object = document
        .objects
        .iter()
        .find(|object| object.object_type == "stoichiometry-grid")
        .expect("native grid object");
    let grid = object
        .payload
        .stoichiometry_grid
        .as_ref()
        .expect("typed grid payload");
    grid.validate().expect("grid invariants");
    assert_eq!(grid.binding_state, StoichiometryBindingState::Current);
    assert_eq!(
        grid.source_reaction_step_id.as_deref(),
        Some(step.id.as_str())
    );
    assert_eq!(grid.components.len(), 3);
    assert!(grid
        .data
        .iter()
        .any(|datum| { datum.origin == StoichiometryValueOrigin::Authored && datum.is_edited }));
    assert!(render_document(&document)
        .iter()
        .any(|primitive| primitive.object_id() == Some(object.id.as_str())));
}

#[test]
fn stoichiometry_cdxml_and_ccjs_round_trip_native_fields() {
    let document =
        parse_cdxml_document(STOICHIOMETRY_CDXML, Some("stoichiometry")).expect("valid CDXML");
    let ccjs = serde_json::to_string(&document).expect("CCJS serialization");
    let reopened_ccjs: chemsema_engine::ChemSemaDocument =
        serde_json::from_str(&ccjs).expect("CCJS reopens");
    assert_eq!(reopened_ccjs.reaction_schemes, document.reaction_schemes);
    let cdxml = document_to_cdxml(&document);
    for token in [
        "<scheme ",
        "<step ",
        "ReactionStepReactants=",
        "<stoichiometrygrid ",
        "<sgcomponent ",
        "<sgdatum ",
        "ComponentReferenceID=",
        "SGPropertyType=",
        "SGDataType=",
        "SGDataValue=",
        "IsEdited=\"yes\"",
    ] {
        assert!(cdxml.contains(token), "missing {token}\n{cdxml}");
    }
    let reopened = parse_cdxml_document(&cdxml, Some("roundtrip")).expect("exported CDXML reopens");
    let grid = reopened
        .objects
        .iter()
        .find_map(|object| object.payload.stoichiometry_grid.as_ref())
        .expect("reopened grid");
    grid.validate().expect("reopened grid invariants");
}

#[test]
fn authored_mass_drives_amount_without_overwriting_authored_cells() {
    let document =
        parse_cdxml_document(STOICHIOMETRY_CDXML, Some("stoichiometry")).expect("valid CDXML");
    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("load native document");
    let grid = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "stoichiometry-grid")
        .unwrap();
    let object_id = grid.id.clone();
    let component_id = grid
        .payload
        .stoichiometry_grid
        .as_ref()
        .unwrap()
        .components
        .iter()
        .find(|component| component.reference_entity_id.is_some())
        .unwrap()
        .id
        .clone();
    engine
        .execute_command_json(
            &json!({
                "type": "edit-stoichiometry-grid",
                "objectId": object_id,
                "action": "add-row",
                "entityId": "Amount"
            })
            .to_string(),
        )
        .expect("add amount row");
    let amount_row = engine
        .state()
        .document
        .find_scene_object(&object_id)
        .and_then(|object| object.payload.stoichiometry_grid.as_ref())
        .unwrap()
        .rows
        .iter()
        .find(|row| row.property_type == "Amount")
        .unwrap()
        .id
        .clone();
    engine
        .execute_command_json(
            &json!({
                "type": "set-stoichiometry-datum",
                "objectId": object_id,
                "componentId": component_id,
                "rowId": amount_row,
                "value": "2.5",
                "unit": "mmol"
            })
            .to_string(),
        )
        .expect("author amount");
    let datum = engine
        .state()
        .document
        .find_scene_object(&object_id)
        .and_then(|object| object.payload.stoichiometry_grid.as_ref())
        .unwrap()
        .data
        .iter()
        .find(|datum| datum.component_id == component_id && datum.row_id == amount_row)
        .unwrap();
    assert_eq!(datum.origin, StoichiometryValueOrigin::Authored);
    assert_eq!(datum.value.display, "2.5");
    assert_eq!(
        datum.calculation_state,
        StoichiometryCalculationState::Inconsistent
    );
}

#[test]
fn unlink_freezes_grid_without_creating_private_link_relation() {
    let document =
        parse_cdxml_document(STOICHIOMETRY_CDXML, Some("stoichiometry")).expect("valid CDXML");
    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("load native document");
    let object_id = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "stoichiometry-grid")
        .unwrap()
        .id
        .clone();
    engine
        .execute_command_json(
            &json!({
                "type": "bind-stoichiometry-grid",
                "objectId": object_id,
                "reactionStepId": null,
                "policy": "unlinked"
            })
            .to_string(),
        )
        .expect("detach command");
    let object = engine
        .state()
        .document
        .find_scene_object(&object_id)
        .unwrap();
    assert_eq!(object.link_policy, LinkPolicy::Unlinked);
    let grid = object.payload.stoichiometry_grid.as_ref().unwrap();
    assert_eq!(grid.binding_state, StoichiometryBindingState::Detached);
    assert!(grid.source_reaction_step_id.is_none());
    assert!(grid
        .components
        .iter()
        .any(|component| component.reference_entity_id.is_some()));
    let detached_cdxml = engine.document_cdxml();
    assert!(
        !detached_cdxml.contains("ComponentReferenceID="),
        "{detached_cdxml}"
    );
    assert!(engine
        .state()
        .document
        .links
        .iter()
        .all(|relation| relation.kind != "reaction-stoichiometry"));
    engine
        .execute_command_json(
            &json!({
                "type": "bind-stoichiometry-grid",
                "objectId": object_id,
                "reactionStepId": null,
                "policy": "auto"
            })
            .to_string(),
        )
        .expect("auto rebind command");
    let rebound = engine
        .state()
        .document
        .find_scene_object(&object_id)
        .unwrap();
    assert_eq!(rebound.link_policy, LinkPolicy::Auto);
    assert!(rebound
        .payload
        .stoichiometry_grid
        .as_ref()
        .unwrap()
        .source_reaction_step_id
        .is_some());
}

#[test]
fn cdx_export_rejects_unrepresentable_stoichiometry_grid_explicitly() {
    let document =
        parse_cdxml_document(STOICHIOMETRY_CDXML, Some("stoichiometry")).expect("valid CDXML");
    let error = document_to_cdx(&document).expect_err("CDX cannot represent the native grid");
    assert!(error.contains("no verified StoichiometryGrid"));
}

#[test]
fn portable_clipboard_remaps_reaction_step_and_grid_binding_together() {
    let document =
        parse_cdxml_document(STOICHIOMETRY_CDXML, Some("stoichiometry")).expect("valid CDXML");
    let mut source = Engine::new();
    source
        .load_document_json(&serde_json::to_string(&document).unwrap())
        .expect("load native document");
    assert!(source.select_all());
    let clipboard = source
        .clipboard_selection_json()
        .expect("serialize clipboard")
        .expect("clipboard content");
    let mut target = Engine::new();
    assert!(target
        .paste_clipboard_json(&clipboard)
        .expect("paste native clipboard"));
    let grid = target
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "stoichiometry-grid")
        .expect("pasted grid");
    let step_id = grid
        .payload
        .stoichiometry_grid
        .as_ref()
        .and_then(|grid| grid.source_reaction_step_id.as_deref())
        .expect("pasted grid remains bound");
    assert!(target
        .state()
        .document
        .reaction_schemes
        .iter()
        .flat_map(|scheme| scheme.steps.iter())
        .any(|step| step.id == step_id));
}
