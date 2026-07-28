use super::*;
use crate::{ObjectPayload, Resource, ResourceData, SceneObject, Transform};
use serde_json::json;
use std::collections::BTreeMap;

fn molecule_at(id: &str, resource_id: &str, x: f64, y: f64) -> (SceneObject, (String, Resource)) {
    let fragment = crate::MoleculeFragment::blank();
    let object = SceneObject {
        id: id.to_string(),
        object_type: "molecule".to_string(),
        name: id.to_string(),
        visible: true,
        locked: false,
        z_index: 10,
        transform: Transform {
            translate: [x, y],
            ..Transform::identity()
        },
        style_ref: None,
        link_policy: LinkPolicy::Auto,
        meta: Value::Null,
        payload: ObjectPayload {
            resource_ref: Some(resource_id.to_string()),
            bbox: Some([0.0, 0.0, 30.0, 30.0]),
            ..ObjectPayload::default()
        },
        children: Vec::new(),
    };
    (
        object,
        (
            resource_id.to_string(),
            Resource {
                resource_type: "molecule".to_string(),
                encoding: "chemical/x-chemsema-fragment+json".to_string(),
                data: ResourceData::Fragment(fragment),
                meta: Value::Null,
            },
        ),
    )
}

fn molecule(id: &str, resource_id: &str, x: f64) -> (SceneObject, (String, Resource)) {
    molecule_at(id, resource_id, x, 100.0)
}

fn reaction_arrow() -> SceneObject {
    SceneObject {
        id: "arrow".to_string(),
        object_type: "line".to_string(),
        name: "arrow".to_string(),
        visible: true,
        locked: false,
        z_index: 20,
        transform: Transform::identity(),
        style_ref: None,
        link_policy: LinkPolicy::Auto,
        meta: Value::Null,
        payload: ObjectPayload {
            bbox: Some([80.0, 100.0, 80.0, 1.0]),
            extra: BTreeMap::from([
                ("points".to_string(), json!([[80.0, 115.0], [160.0, 115.0]])),
                (
                    "arrowHead".to_string(),
                    json!({"kind": "solid", "head": "full", "tail": "none"}),
                ),
            ]),
            ..ObjectPayload::default()
        },
        children: Vec::new(),
    }
}

fn condition_text(id: &str, x: f64, y: f64) -> SceneObject {
    SceneObject {
        id: id.to_string(),
        object_type: "text".to_string(),
        name: id.to_string(),
        visible: true,
        locked: false,
        z_index: 15,
        transform: Transform {
            translate: [x, y],
            ..Transform::identity()
        },
        style_ref: None,
        link_policy: LinkPolicy::Auto,
        meta: Value::Null,
        payload: ObjectPayload {
            bbox: Some([0.0, 0.0, 20.0, 10.0]),
            ..ObjectPayload::default()
        },
        children: Vec::new(),
    }
}

#[test]
fn auto_link_policy_materializes_typed_reaction_step() {
    let mut engine = Engine::new();
    engine.state.document.objects.clear();
    engine.state.document.resources.clear();
    let (left, left_resource) = molecule("left", "left_resource", 20.0);
    let (right, right_resource) = molecule("right", "right_resource", 190.0);
    engine.state.document.objects = vec![left, reaction_arrow(), right];
    engine
        .state
        .document
        .resources
        .extend([left_resource, right_resource]);

    assert!(engine.reconcile_logical_relations_after_document_change());
    let step = &engine.state.document.reaction_schemes[0].steps[0];
    assert_eq!(step.binding_origin, LogicalBindingOrigin::Inferred);
    assert_eq!(step.link_policy, LinkPolicy::Auto);
    assert_eq!(step.reactant_entity_ids, ["left"]);
    assert_eq!(step.product_entity_ids, ["right"]);
    assert_eq!(step.arrow_object_ids, ["arrow"]);

    engine
        .state
        .document
        .find_scene_object_mut("right")
        .unwrap()
        .link_policy = LinkPolicy::Unlinked;
    assert!(engine.reconcile_logical_relations_after_document_change());
    assert!(engine.state.document.reaction_schemes.is_empty());
}

#[test]
fn mechanism_curved_arrow_is_not_a_reaction_anchor() {
    let mut engine = Engine::new();
    engine.state.document.objects.clear();
    let mut arrow = reaction_arrow();
    arrow
        .payload
        .extra
        .get_mut("arrowHead")
        .and_then(Value::as_object_mut)
        .unwrap()
        .insert("kind".to_string(), json!("curved"));
    engine.state.document.objects.push(arrow);
    assert!(!engine.reconcile_logical_relations_after_document_change());
    assert!(engine.state.document.reaction_schemes.is_empty());
}

#[test]
fn headless_line_is_not_a_reaction_anchor() {
    let mut engine = Engine::new();
    engine.state.document.objects.clear();
    let mut line = reaction_arrow();
    let arrow = line
        .payload
        .extra
        .get_mut("arrowHead")
        .and_then(Value::as_object_mut)
        .unwrap();
    arrow.insert("head".to_string(), json!("none"));
    arrow.insert("tail".to_string(), json!("none"));
    engine.state.document.objects.push(line);
    assert!(!engine.reconcile_logical_relations_after_document_change());
    assert!(engine.state.document.reaction_schemes.is_empty());
}

#[test]
fn diagonal_arrow_uses_its_local_axis_for_reactant_and_product() {
    let mut engine = Engine::new();
    engine.state.document.objects.clear();
    engine.state.document.resources.clear();
    let (left, left_resource) = molecule_at("left", "left_resource", 20.0, 20.0);
    let (right, right_resource) = molecule_at("right", "right_resource", 190.0, 190.0);
    let mut arrow = reaction_arrow();
    arrow
        .payload
        .extra
        .insert("points".to_string(), json!([[80.0, 80.0], [160.0, 160.0]]));
    engine.state.document.objects = vec![left, arrow, right];
    engine
        .state
        .document
        .resources
        .extend([left_resource, right_resource]);

    assert!(engine.reconcile_logical_relations_after_document_change());
    let step = &engine.state.document.reaction_schemes[0].steps[0];
    assert_eq!(step.reactant_entity_ids, ["left"]);
    assert_eq!(step.product_entity_ids, ["right"]);
}

#[test]
fn auto_classifies_condition_text_above_and_below_the_arrow_axis() {
    let mut engine = Engine::new();
    engine.state.document.objects.clear();
    engine.state.document.resources.clear();
    let (left, left_resource) = molecule("left", "left_resource", 20.0);
    let (right, right_resource) = molecule("right", "right_resource", 190.0);
    engine.state.document.objects = vec![
        left,
        reaction_arrow(),
        condition_text("above", 110.0, 75.0),
        condition_text("below", 110.0, 145.0),
        right,
    ];
    engine
        .state
        .document
        .resources
        .extend([left_resource, right_resource]);

    assert!(engine.reconcile_logical_relations_after_document_change());
    let step = &engine.state.document.reaction_schemes[0].steps[0];
    assert_eq!(step.objects_above_arrow, ["above"]);
    assert_eq!(step.objects_below_arrow, ["below"]);
}

#[test]
fn equally_good_reaction_arrows_leave_auto_binding_unresolved() {
    let mut engine = Engine::new();
    engine.state.document.objects.clear();
    engine.state.document.resources.clear();
    let (left, left_resource) = molecule("left", "left_resource", 20.0);
    let (right, right_resource) = molecule("right", "right_resource", 190.0);
    let mut second_arrow = reaction_arrow();
    second_arrow.id = "arrow_2".to_string();
    engine.state.document.objects = vec![left, reaction_arrow(), second_arrow, right];
    engine
        .state
        .document
        .resources
        .extend([left_resource, right_resource]);

    assert!(!engine.reconcile_logical_relations_after_document_change());
    assert!(engine.state.document.reaction_schemes.is_empty());
}

#[test]
fn explicit_link_and_unlink_override_reaction_auto() {
    let mut engine = Engine::new();
    engine.state.document.objects.clear();
    engine.state.document.resources.clear();
    let (left, left_resource) = molecule("left", "left_resource", 20.0);
    let (right, right_resource) = molecule("right", "right_resource", 190.0);
    engine.state.document.objects = vec![left, reaction_arrow(), right];
    engine
        .state
        .document
        .resources
        .extend([left_resource, right_resource]);
    engine.state.selection.molecule_objects = vec!["left".to_string(), "right".to_string()];
    engine.state.selection.arrow_objects = vec!["arrow".to_string()];

    assert!(engine.selection_can_link());
    assert!(engine.link_selection());
    let step = &engine.state.document.reaction_schemes[0].steps[0];
    assert_eq!(step.link_policy, LinkPolicy::Linked);
    assert_eq!(step.binding_origin, LogicalBindingOrigin::Authored);

    assert!(engine.unlink_selection());
    assert!(engine.state.document.reaction_schemes.is_empty());
    for id in ["left", "arrow", "right"] {
        assert_eq!(
            engine
                .state
                .document
                .find_scene_object(id)
                .unwrap()
                .link_policy,
            LinkPolicy::Unlinked
        );
    }
}

#[test]
fn clipboard_remaps_native_logical_object_owners_and_targets() {
    let mut engine = Engine::new();
    engine.state.document.objects.clear();
    engine.state.document.resources.clear();
    let (left, left_resource) = molecule("left", "left_resource", 20.0);
    let (right, right_resource) = molecule("right", "right_resource", 190.0);
    engine.state.document.objects = vec![left, reaction_arrow(), right];
    engine
        .state
        .document
        .resources
        .extend([left_resource, right_resource]);
    engine
        .state
        .document
        .logical_objects
        .object_tags
        .push(crate::ObjectTagData {
            id: "tag".to_string(),
            owner_entity_id: Some("left".to_string()),
            unresolved_owner_source_id: None,
            name: "catalog".to_string(),
            display_name: None,
            tag_type: crate::ObjectTagType::String,
            value: Some("A-1".to_string()),
            positioning_type: Default::default(),
            positioning_angle: None,
            positioning_offset: None,
            persistent: true,
            tracking: true,
            visible: false,
            display_object_ids: Vec::new(),
            binding_origin: LogicalBindingOrigin::Authored,
        });
    engine
        .state
        .document
        .logical_objects
        .registry_numbers
        .push(crate::RegistryNumberData {
            id: "registration".to_string(),
            owner_entity_id: Some("left".to_string()),
            unresolved_owner_source_id: None,
            authority: "CAS".to_string(),
            number: "50-00-0".to_string(),
            binding_origin: LogicalBindingOrigin::Authored,
        });
    engine
        .state
        .document
        .logical_objects
        .representations
        .push(crate::RepresentationData {
            id: "representation".to_string(),
            owner_entity_id: Some("arrow".to_string()),
            unresolved_owner_source_id: None,
            target_entity_id: Some("left".to_string()),
            unresolved_target_source_id: None,
            attribute: "Element".to_string(),
            binding_origin: LogicalBindingOrigin::Authored,
        });

    assert!(engine.select_all());
    assert!(engine.copy_selection());
    assert!(engine.paste_clipboard());

    let logical = &engine.state.document.logical_objects;
    assert_eq!(logical.object_tags.len(), 2);
    assert_eq!(logical.registry_numbers.len(), 2);
    assert_eq!(logical.representations.len(), 2);
    let pasted_tag = logical
        .object_tags
        .iter()
        .find(|tag| tag.id != "tag")
        .unwrap();
    let pasted_owner = pasted_tag.owner_entity_id.as_deref().unwrap();
    assert_ne!(pasted_owner, "left");
    assert!(engine
        .state
        .document
        .find_scene_object(pasted_owner)
        .is_some());
    let pasted_representation = logical
        .representations
        .iter()
        .find(|relation| relation.id != "representation")
        .unwrap();
    assert_ne!(
        pasted_representation.target_entity_id.as_deref(),
        Some("left")
    );
    assert!(engine
        .state
        .document
        .find_scene_object(pasted_representation.target_entity_id.as_deref().unwrap())
        .is_some());
}

#[test]
fn logical_object_commands_validate_reorder_and_round_trip_history() {
    let mut engine = Engine::new();
    engine.state.document.objects.clear();
    engine.state.document.resources.clear();
    let (left, left_resource) = molecule("left", "left_resource", 20.0);
    engine.state.document.objects.push(left);
    engine
        .state
        .document
        .resources
        .insert(left_resource.0, left_resource.1);

    assert!(engine
        .set_logical_object_value(
            "annotation",
            json!({
                "id": "note_a",
                "ownerEntityId": "left",
                "keyword": "source",
                "content": "catalogue",
                "bindingOrigin": "authored"
            }),
        )
        .unwrap());
    assert!(engine
        .set_logical_object_value(
            "annotation",
            json!({
                "id": "note_b",
                "ownerEntityId": "left",
                "content": "reviewed",
                "bindingOrigin": "authored"
            }),
        )
        .unwrap());
    assert_eq!(
        engine.state.document.logical_objects.annotations[0].id,
        "note_a"
    );
    assert!(engine
        .reorder_logical_object("annotation", "note_b", 0)
        .unwrap());
    assert_eq!(
        engine.state.document.logical_objects.annotations[0].id,
        "note_b"
    );

    let error = engine
        .set_logical_object_value(
            "annotation",
            json!({
                "id": "invalid",
                "ownerEntityId": "missing",
                "content": "must fail"
            }),
        )
        .unwrap_err();
    assert!(error.contains("missing"));
    assert_eq!(engine.state.document.logical_objects.annotations.len(), 2);

    assert!(engine
        .delete_logical_object("annotation", "note_b")
        .unwrap());
    assert_eq!(
        engine.state.document.logical_objects.annotations[0].id,
        "note_a"
    );
    assert!(engine.undo());
    assert_eq!(
        engine.state.document.logical_objects.annotations[0].id,
        "note_b"
    );
    assert!(engine.redo());
    assert_eq!(
        engine.state.document.logical_objects.annotations[0].id,
        "note_a"
    );
}

#[test]
fn reaction_scheme_and_step_use_the_same_logical_command_surface() {
    let mut engine = Engine::new();
    engine.state.document.objects.clear();
    engine.state.document.resources.clear();
    let (left, left_resource) = molecule("left", "left_resource", 20.0);
    let (right, right_resource) = molecule("right", "right_resource", 190.0);
    engine.state.document.objects = vec![left, reaction_arrow(), right];
    engine
        .state
        .document
        .resources
        .extend([left_resource, right_resource]);

    let result = engine
        .execute_command_json(
            &json!({
                "type": "set-logical-object",
                "kind": "reaction-scheme",
                "value": {"id": "scheme_authored", "steps": []}
            })
            .to_string(),
        )
        .unwrap();
    assert!(serde_json::from_str::<Value>(&result).unwrap()["changed"] == true);
    assert!(engine
        .set_logical_object_value(
            "reaction-step",
            json!({
                "id": "step_authored",
                "schemeId": "scheme_authored",
                "linkPolicy": "linked",
                "bindingOrigin": "authored",
                "reactantEntityIds": ["left"],
                "productEntityIds": ["right"],
                "arrowObjectIds": ["arrow"],
                "interpretationState": "current"
            }),
        )
        .unwrap());
    assert_eq!(
        engine.state.document.reaction_schemes[0].steps[0].id,
        "step_authored"
    );
    assert!(engine
        .delete_logical_object("reaction-step", "step_authored")
        .unwrap());
    assert!(engine.state.document.reaction_schemes[0].steps.is_empty());
    assert!(engine.undo());
    assert_eq!(
        engine.state.document.reaction_schemes[0].steps[0].id,
        "step_authored"
    );
}
