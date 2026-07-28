fn reorder_by_id<T>(
    values: &mut Vec<T>,
    id: &str,
    index: usize,
    item_id: impl Fn(&T) -> &str,
) -> bool {
    let Some(current) = values.iter().position(|item| item_id(item) == id) else {
        return false;
    };
    let target = index.min(values.len().saturating_sub(1));
    if current == target {
        return false;
    }
    let item = values.remove(current);
    values.insert(target, item);
    true
}

fn detach_stoichiometry_grids_for_removed_steps(
    objects: &mut [crate::SceneObject],
    removed_step_ids: &BTreeSet<String>,
) {
    for object in objects {
        if let Some(grid) = object.payload.stoichiometry_grid.as_mut() {
            if grid
                .source_reaction_step_id
                .as_ref()
                .is_some_and(|id| removed_step_ids.contains(id))
            {
                grid.source_reaction_step_id = None;
                grid.binding_state = crate::StoichiometryBindingState::Orphaned;
                object.link_policy = LinkPolicy::Unlinked;
                for datum in &mut grid.data {
                    if datum.origin == crate::StoichiometryValueOrigin::Calculated {
                        datum.origin = crate::StoichiometryValueOrigin::Imported;
                    }
                }
            }
        }
        detach_stoichiometry_grids_for_removed_steps(&mut object.children, removed_step_ids);
    }
}

fn logical_family<T: serde::Serialize>(
    kind: &str,
    label: &str,
    items: &[T],
    fields: Vec<Value>,
    default_value: Value,
) -> Value {
    serde_json::json!({
        "kind": kind,
        "label": label,
        "items": items,
        "fields": fields,
        "defaultValue": default_value,
    })
}

fn logical_value_family(
    kind: &str,
    label: &str,
    items: Vec<Value>,
    fields: Vec<Value>,
    default_value: Value,
) -> Value {
    serde_json::json!({
        "kind": kind,
        "label": label,
        "items": items,
        "fields": fields,
        "defaultValue": default_value,
    })
}

fn logical_field(key: &str, label: &str, value_kind: &str) -> Value {
    serde_json::json!({ "key": key, "label": label, "valueKind": value_kind })
}

fn choice_field(key: &str, label: &str, values: &[(&str, &str)]) -> Value {
    serde_json::json!({
        "key": key,
        "label": label,
        "valueKind": "choice",
        "options": values.iter().map(|(value, label)| serde_json::json!({
            "value": value,
            "label": label,
        })).collect::<Vec<_>>(),
    })
}

fn common_id_field() -> Value {
    serde_json::json!({
        "key": "id",
        "label": "Internal ID",
        "valueKind": "text",
        "readOnlyWhenPresent": true,
        "placeholder": "Assigned automatically"
    })
}

fn binding_origin_field() -> Value {
    choice_field(
        "bindingOrigin",
        "Origin",
        &[
            ("authored", "Authored"),
            ("imported", "Imported"),
            ("inferred", "Inferred"),
            ("none", "None"),
        ],
    )
}

fn reaction_scheme_fields() -> Vec<Value> {
    vec![common_id_field()]
}

fn reaction_step_fields() -> Vec<Value> {
    vec![
        common_id_field(),
        logical_field("schemeId", "Reaction scheme", "text"),
        choice_field(
            "linkPolicy",
            "Link policy",
            &[
                ("auto", "Auto"),
                ("linked", "Linked"),
                ("unlinked", "Unlinked"),
            ],
        ),
        logical_field("reactantEntityIds", "Reactants", "entity-list"),
        logical_field("productEntityIds", "Products", "entity-list"),
        logical_field("arrowObjectIds", "Reaction arrows", "entity-list"),
        logical_field("plusObjectIds", "Plus signs", "entity-list"),
        logical_field("objectsAboveArrow", "Objects above arrow", "entity-list"),
        logical_field("objectsBelowArrow", "Objects below arrow", "entity-list"),
        logical_field("atomMappings", "Atom mappings", "json"),
        choice_field(
            "interpretationState",
            "Interpretation",
            &[
                ("current", "Current"),
                ("stale", "Stale"),
                ("invalid", "Invalid"),
            ],
        ),
        binding_origin_field(),
    ]
}

fn alternative_group_fields() -> Vec<Value> {
    vec![
        common_id_field(),
        logical_field("memberEntityIds", "Members", "entity-list"),
        logical_field("attachmentNodeIds", "Attachment atoms", "entity-list"),
        logical_field("valence", "Valence", "optional-integer"),
        logical_field("position", "Position", "optional-number-list-2"),
        logical_field("boundingBox", "Bounding box", "optional-number-list-4"),
        logical_field("textFrame", "Text frame", "optional-number-list-4"),
        logical_field("groupFrame", "Group frame", "optional-number-list-4"),
        logical_field("opacity", "Opacity", "optional-number"),
        logical_field("color", "Color", "optional-text"),
        logical_field("zIndex", "Z order", "optional-integer"),
        logical_field("visible", "Visible", "boolean"),
        logical_field("ignoreWarnings", "Ignore warnings", "boolean"),
        logical_field("warning", "Warning", "optional-text"),
        logical_field("supersededById", "Superseded by", "optional-text"),
        binding_origin_field(),
    ]
}

fn bracketed_group_fields() -> Vec<Value> {
    vec![
        common_id_field(),
        logical_field("bracketObjectIds", "Bracket graphics", "entity-list"),
        logical_field("bracketedEntityIds", "Bracketed objects", "entity-list"),
        logical_field("nestedGroupIds", "Nested bracketed groups", "text-list"),
        choice_field(
            "usage",
            "Usage",
            &[
                ("unspecified", "Unspecified"),
                ("any-polymer", "Any polymer"),
                ("component", "Component"),
                ("copolymer", "Copolymer"),
                ("copolymer-alternating", "Alternating copolymer"),
                ("copolymer-block", "Block copolymer"),
                ("copolymer-random", "Random copolymer"),
                ("crosslink", "Crosslink"),
                ("generic", "Generic"),
                ("graft", "Graft"),
                ("mer", "Mer"),
                ("mixture-ordered", "Ordered mixture"),
                ("mixture-unordered", "Unordered mixture"),
                ("modification", "Modification"),
                ("monomer", "Monomer"),
                ("multiple-group", "Multiple group"),
                ("sru", "SRU"),
            ],
        ),
        logical_field("componentOrder", "Component order", "optional-integer"),
        choice_field(
            "polymerRepeatPattern",
            "Repeat pattern",
            &[
                ("head-to-tail", "Head to tail"),
                ("head-to-head", "Head to head"),
                ("either-unknown", "Either / unknown"),
            ],
        ),
        choice_field(
            "polymerFlipType",
            "Flip",
            &[
                ("unspecified", "Unspecified"),
                ("no-flip", "No flip"),
                ("flip", "Flip"),
            ],
        ),
        logical_field("repeatCount", "Repeat count", "optional-number"),
        logical_field("sruLabel", "SRU label", "optional-text"),
        logical_field(
            "attachments",
            "Bracket attachments and crossing bonds",
            "json",
        ),
        binding_origin_field(),
    ]
}

fn sequence_fields() -> Vec<Value> {
    vec![
        common_id_field(),
        logical_field("identifier", "Sequence identifier", "text"),
        logical_field("textObjectIds", "Displayed text objects", "entity-list"),
        binding_origin_field(),
    ]
}

fn cross_reference_fields() -> Vec<Value> {
    vec![
        common_id_field(),
        logical_field("identifier", "Cross-reference identifier", "text"),
        logical_field("sequenceIdentifier", "Sequence identifier", "text"),
        logical_field("container", "Container", "optional-text"),
        logical_field("document", "Document", "optional-text"),
        logical_field("textObjectIds", "Displayed text objects", "entity-list"),
        binding_origin_field(),
    ]
}

fn object_tag_fields() -> Vec<Value> {
    vec![
        common_id_field(),
        logical_field("ownerEntityId", "Owner", "optional-entity"),
        logical_field("name", "Name", "text"),
        logical_field("displayName", "Display name", "optional-text"),
        choice_field(
            "tagType",
            "Value type",
            &[
                ("unknown", "Unknown"),
                ("string", "String"),
                ("long", "Integer"),
                ("double", "Number"),
            ],
        ),
        logical_field("value", "Value", "optional-text"),
        choice_field(
            "positioningType",
            "Positioning",
            &[
                ("auto", "Auto"),
                ("angle", "Angle"),
                ("offset", "Offset"),
                ("absolute", "Absolute"),
            ],
        ),
        logical_field("positioningAngle", "Positioning angle", "optional-number"),
        logical_field(
            "positioningOffset",
            "Positioning offset",
            "optional-number-list-2",
        ),
        logical_field("persistent", "Persistent", "boolean"),
        logical_field("tracking", "Track owner", "boolean"),
        logical_field("visible", "Visible", "boolean"),
        logical_field("displayObjectIds", "Displayed text objects", "entity-list"),
        binding_origin_field(),
    ]
}

fn annotation_fields() -> Vec<Value> {
    vec![
        common_id_field(),
        logical_field("ownerEntityId", "Owner", "optional-entity"),
        logical_field("keyword", "Keyword", "optional-text"),
        logical_field("content", "Content", "optional-text"),
        binding_origin_field(),
    ]
}

fn registry_number_fields() -> Vec<Value> {
    vec![
        common_id_field(),
        logical_field("ownerEntityId", "Owner", "optional-entity"),
        logical_field("authority", "Authority", "text"),
        logical_field("number", "Registry number", "text"),
        binding_origin_field(),
    ]
}

fn representation_fields() -> Vec<Value> {
    vec![
        common_id_field(),
        logical_field("ownerEntityId", "Owner", "optional-entity"),
        logical_field("targetEntityId", "Target", "optional-entity"),
        logical_field("attribute", "Represented attribute", "text"),
        binding_origin_field(),
    ]
}

fn upsert_logical_value(
    engine: &mut Engine,
    logical: &mut crate::LogicalObjectData,
    kind: &str,
    value: Value,
) -> Result<Value, String> {
    macro_rules! upsert {
        ($field:ident, $ty:ty) => {{
            let mut item: $ty = serde_json::from_value(value).map_err(|error| error.to_string())?;
            if item.id.trim().is_empty() {
                item.id = engine.next_id(&kind.replace('-', "_"));
            }
            if let Some(existing) = logical
                .$field
                .iter_mut()
                .find(|existing| existing.id == item.id)
            {
                *existing = item.clone();
            } else {
                logical.$field.push(item.clone());
            }
            serde_json::to_value(item).map_err(|error| error.to_string())
        }};
    }
    match kind {
        "alternative-group" => upsert!(alternative_groups, crate::AlternativeGroupData),
        "bracketed-group" => upsert!(bracketed_groups, crate::BracketedGroupData),
        "sequence" => upsert!(sequences, crate::SequenceData),
        "cross-reference" => upsert!(cross_references, crate::CrossReferenceData),
        "object-tag" => upsert!(object_tags, crate::ObjectTagData),
        "annotation" => upsert!(annotations, crate::AnnotationData),
        "registry-number" => upsert!(registry_numbers, crate::RegistryNumberData),
        "representation" => upsert!(representations, crate::RepresentationData),
        _ => Err(format!("unsupported logical object kind '{kind}'")),
    }
}

fn delete_logical_value(
    logical: &mut crate::LogicalObjectData,
    kind: &str,
    id: &str,
) -> Result<bool, String> {
    macro_rules! remove {
        ($field:ident) => {{
            let before = logical.$field.len();
            logical.$field.retain(|item| item.id != id);
            before != logical.$field.len()
        }};
    }
    let removed = match kind {
        "alternative-group" => remove!(alternative_groups),
        "bracketed-group" => {
            let removed = remove!(bracketed_groups);
            if removed {
                for group in &mut logical.bracketed_groups {
                    group.nested_group_ids.retain(|child_id| child_id != id);
                }
            }
            removed
        }
        "sequence" => {
            let identifier = logical
                .sequences
                .iter()
                .find(|sequence| sequence.id == id)
                .map(|sequence| sequence.identifier.clone());
            let removed = remove!(sequences);
            if let Some(identifier) = identifier {
                logical
                    .cross_references
                    .retain(|reference| reference.sequence_identifier != identifier);
            }
            removed
        }
        "cross-reference" => remove!(cross_references),
        "object-tag" => remove!(object_tags),
        "annotation" => remove!(annotations),
        "registry-number" => remove!(registry_numbers),
        "representation" => remove!(representations),
        _ => return Err(format!("unsupported logical object kind '{kind}'")),
    };
    if removed {
        for group in &mut logical.alternative_groups {
            if group.superseded_by_id.as_deref() == Some(id) {
                group.superseded_by_id = None;
            }
        }
    }
    Ok(removed)
}

fn reorder_logical_value(
    logical: &mut crate::LogicalObjectData,
    kind: &str,
    id: &str,
    index: usize,
) -> Result<bool, String> {
    let changed = match kind {
        "alternative-group" => {
            reorder_by_id(&mut logical.alternative_groups, id, index, |item| &item.id)
        }
        "bracketed-group" => {
            reorder_by_id(&mut logical.bracketed_groups, id, index, |item| &item.id)
        }
        "sequence" => reorder_by_id(&mut logical.sequences, id, index, |item| &item.id),
        "cross-reference" => {
            reorder_by_id(&mut logical.cross_references, id, index, |item| &item.id)
        }
        "object-tag" => reorder_by_id(&mut logical.object_tags, id, index, |item| &item.id),
        "annotation" => reorder_by_id(&mut logical.annotations, id, index, |item| &item.id),
        "registry-number" => {
            reorder_by_id(&mut logical.registry_numbers, id, index, |item| &item.id)
        }
        "representation" => reorder_by_id(&mut logical.representations, id, index, |item| &item.id),
        _ => return Err(format!("unsupported logical object kind '{kind}'")),
    };
    Ok(changed)
}

fn reaction_step_entity_ids(step: &crate::ReactionStepData) -> impl Iterator<Item = &String> {
    step.reactant_entity_ids
        .iter()
        .chain(step.product_entity_ids.iter())
        .chain(step.arrow_object_ids.iter())
        .chain(step.plus_object_ids.iter())
        .chain(step.objects_above_arrow.iter())
        .chain(step.objects_below_arrow.iter())
}

fn reaction_candidate_entity_ids(candidate: &InferredReactionStep) -> BTreeSet<String> {
    std::iter::once(candidate.arrow_id.clone())
        .chain(candidate.reactants.iter().map(|(id, _)| id.clone()))
        .chain(candidate.products.iter().map(|(id, _)| id.clone()))
        .chain(candidate.pluses.iter().map(|(id, _)| id.clone()))
        .chain(candidate.above.iter().map(|(id, _)| id.clone()))
        .chain(candidate.below.iter().map(|(id, _)| id.clone()))
        .collect()
}

fn retain_existing(ids: &mut Vec<String>, exists: &impl Fn(&String) -> bool) -> bool {
    let before = ids.len();
    ids.retain(exists);
    before != ids.len()
}

fn is_reaction_arrow(object: &crate::SceneObject) -> bool {
    if object.object_type != "line" {
        return false;
    }
    let Some(arrow) = object.payload.extra.get("arrowHead") else {
        return false;
    };
    let has_endpoint = ["head", "tail"].into_iter().any(|endpoint| {
        arrow
            .get(endpoint)
            .and_then(Value::as_str)
            .is_some_and(|value| value != "none" && !value.is_empty())
    });
    has_endpoint
        && !matches!(
            arrow.get("kind").and_then(Value::as_str),
            Some("curved" | "curved-mirror")
        )
}

fn reaction_arrow_axis(object: &crate::SceneObject) -> Option<ReactionArrowAxis> {
    let (mut start, mut end) = crate::arrow_payload_line_endpoints(&object.payload.extra)?;
    start.x += object.transform.translate[0];
    start.y += object.transform.translate[1];
    end.x += object.transform.translate[0];
    end.y += object.transform.translate[1];
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length = dx.hypot(dy);
    if length <= crate::EPSILON {
        return None;
    }
    Some(ReactionArrowAxis {
        object_id: object.id.clone(),
        start,
        end,
        center: Point::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5),
        unit: [dx / length, dy / length],
        normal: [-dy / length, dx / length],
        length,
    })
}

fn unique_reaction_side_candidate(
    point: Point,
    axes: &[ReactionArrowAxis],
) -> Option<ReactionAutoCandidate> {
    let mut candidates = axes
        .iter()
        .filter_map(|axis| {
            let (projection, perpendicular) = axis_coordinates(point, axis);
            let side = if projection < -axis.length * 0.15 {
                ReactionSide::Reactant
            } else if projection > axis.length * 0.15 {
                ReactionSide::Product
            } else {
                return None;
            };
            let endpoint = match side {
                ReactionSide::Reactant => axis.start,
                ReactionSide::Product => axis.end,
            };
            let max_axial = (axis.length * 4.0).max(crate::DEFAULT_BOND_LENGTH * 8.0);
            let max_perpendicular = (axis.length * 1.5).max(crate::DEFAULT_BOND_LENGTH * 3.0);
            let endpoint_distance = point.distance(endpoint);
            if endpoint_distance > max_axial || perpendicular.abs() > max_perpendicular {
                return None;
            }
            Some(ReactionAutoCandidate {
                arrow_id: axis.object_id.clone(),
                side,
                score: endpoint_distance,
                projection,
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.score
            .total_cmp(&right.score)
            .then_with(|| left.arrow_id.cmp(&right.arrow_id))
    });
    unique_best(candidates, |candidate| candidate.score)
}

fn unique_reaction_condition_candidate(
    point: Point,
    axes: &[ReactionArrowAxis],
) -> Option<(String, f64, f64)> {
    let mut candidates = axes
        .iter()
        .filter_map(|axis| {
            let (projection, perpendicular) = axis_coordinates(point, axis);
            let minimum_offset = crate::DEFAULT_BOND_LENGTH * 0.2;
            let maximum_offset = (axis.length * 1.5).max(crate::DEFAULT_BOND_LENGTH * 3.0);
            if projection.abs() > axis.length * 0.85
                || perpendicular.abs() < minimum_offset
                || perpendicular.abs() > maximum_offset
            {
                return None;
            }
            Some((
                axis.object_id.clone(),
                projection,
                perpendicular,
                perpendicular.abs(),
            ))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.3
            .total_cmp(&right.3)
            .then_with(|| left.0.cmp(&right.0))
    });
    let best = unique_best(candidates, |candidate| candidate.3)?;
    Some((best.0, best.1, best.2))
}

fn unique_best<T>(mut candidates: Vec<T>, score: impl Fn(&T) -> f64) -> Option<T> {
    if candidates.is_empty() {
        return None;
    }
    if candidates.len() > 1
        && (score(&candidates[0]) - score(&candidates[1])).abs() <= crate::DEFAULT_BOND_LENGTH * 0.1
    {
        return None;
    }
    Some(candidates.remove(0))
}

fn axis_coordinates(point: Point, axis: &ReactionArrowAxis) -> (f64, f64) {
    let dx = point.x - axis.center.x;
    let dy = point.y - axis.center.y;
    (
        dx * axis.unit[0] + dy * axis.unit[1],
        dx * axis.normal[0] + dy * axis.normal[1],
    )
}

fn scene_object_center(
    document: &crate::ChemSemaDocument,
    object: &crate::SceneObject,
) -> Option<Point> {
    let [left, top, right, bottom] =
        super::select::object_selection_bounds_for_render(document, object)?;
    Some(Point::new((left + right) * 0.5, (top + bottom) * 0.5))
}

fn is_plus_symbol(object: &crate::SceneObject) -> bool {
    object.object_type == "symbol"
        && (object.payload.extra.get("kind").and_then(Value::as_str) == Some("plus")
            || object.name.eq_ignore_ascii_case("plus"))
}

fn sorted_ids(mut values: Vec<(String, f64)>) -> Vec<String> {
    values.sort_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    values.into_iter().map(|(id, _)| id).collect()
}
