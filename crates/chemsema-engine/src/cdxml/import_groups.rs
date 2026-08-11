use super::*;

pub(super) fn normalize_repeated_text_objects(root: &mut XmlNode) -> Result<(), String> {
    normalize_repeated_text_children(root)
}

fn normalize_repeated_text_children(parent: &mut XmlNode) -> Result<(), String> {
    for child in &mut parent.children {
        normalize_repeated_text_children(child)?;
    }

    let mut normalized = Vec::<XmlNode>::with_capacity(parent.children.len());
    let mut text_index_by_id = BTreeMap::<String, usize>::new();
    for child in std::mem::take(&mut parent.children) {
        let repeated_text_index = child
            .is("t")
            .then(|| child.attr("id"))
            .flatten()
            .filter(|id| !matches!(id.trim(), "" | "0"))
            .and_then(|id| text_index_by_id.get(id).copied());
        if let Some(index) = repeated_text_index {
            merge_repeated_text_part(&mut normalized[index], child)?;
            continue;
        }
        if child.is("t") {
            if let Some(id) = child.attr("id").filter(|id| !matches!(id.trim(), "" | "0")) {
                text_index_by_id.insert(id.to_string(), normalized.len());
            }
        }
        normalized.push(child);
    }
    parent.children = normalized;
    Ok(())
}

fn merge_repeated_text_part(target: &mut XmlNode, mut part: XmlNode) -> Result<(), String> {
    let text_id = target.attr("id").unwrap_or("<missing>").to_string();
    if !target.children.is_empty() && target.text.trim().is_empty() {
        target.text.clear();
    }
    if !part.children.is_empty() && part.text.trim().is_empty() {
        part.text.clear();
    }
    for (name, value) in &part.attrs {
        if name == "UTF8Text" {
            continue;
        }
        if let Some(target_value) = target.attrs.get(name) {
            if target_value != value {
                return Err(format!(
                    "CDXML text id '{text_id}' is repeated in one container with conflicting \
                     '{name}' values ('{target_value}' and '{value}')"
                ));
            }
        } else {
            target.attrs.insert(name.clone(), value.clone());
        }
    }

    let target_utf8 = target.attrs.get("UTF8Text").cloned();
    let part_utf8 = part.attrs.remove("UTF8Text");
    let target_has_children = !target.children.is_empty();
    let part_has_children = !part.children.is_empty();
    let target_has_direct_text = !target.text.trim().is_empty();
    let part_has_direct_text = !part.text.trim().is_empty();
    let target_has_content = target_has_children
        || target_has_direct_text
        || target_utf8.as_deref().is_some_and(|s| !s.is_empty());
    let part_has_content = part_has_children
        || part_has_direct_text
        || part_utf8.as_deref().is_some_and(|s| !s.is_empty());

    if !part_has_content {
        return Ok(());
    }
    if !target_has_content {
        target.text = part.text;
        target.children = part.children;
        if let Some(value) = part_utf8 {
            target.attrs.insert("UTF8Text".to_string(), value);
        }
        return Ok(());
    }

    match (target_utf8, part_utf8) {
        (Some(mut first), Some(second))
            if !target_has_children
                && !part_has_children
                && !target_has_direct_text
                && !part_has_direct_text =>
        {
            first.push_str(&second);
            target.attrs.insert("UTF8Text".to_string(), first);
        }
        (None, None) => {
            target.text.push_str(&part.text);
            target.children.append(&mut part.children);
        }
        _ => {
            return Err(format!(
                "CDXML text id '{text_id}' mixes UTF8Text and child-style representations across \
                 repeated parts"
            ));
        }
    }
    Ok(())
}

pub(super) fn interchange_object_from_xml(node: &XmlNode) -> InterchangeObject {
    InterchangeObject {
        name: node.name.clone(),
        format_tag: None,
        id: node.attr("id").map(ToString::to_string),
        properties: node
            .attrs
            .iter()
            .filter(|(name, _)| retain_cdxml_interchange_property(node, name))
            .enumerate()
            .map(|(order, (name, value))| {
                (
                    name.clone(),
                    InterchangeProperty {
                        name: name.clone(),
                        order,
                        value: value.clone(),
                        value_type: Some(cdxml_lexical_type(value).to_string()),
                        cdx_tag: None,
                        cdx_type: None,
                        raw_base64: None,
                    },
                )
            })
            .collect(),
        text: node.text.clone(),
        children: node
            .children
            .iter()
            .map(interchange_object_from_xml)
            .collect(),
    }
}

pub(super) fn retain_cdxml_interchange_property(node: &XmlNode, name: &str) -> bool {
    // CDX/CDXML face is a transport bitmask.  CCJS persists its independent
    // style meanings (weight, italic, underline, outline, shadow, script) and
    // reconstructs face on export; retaining the mask would create a second,
    // conflicting authority.
    if matches!(name, "face" | "LabelFace" | "CaptionFace") {
        return false;
    }
    // CaptionJustification supersedes the obsolete LabelJustification field on
    // text objects.  Once the authoritative caption field is present the old
    // value must not be resurrected during a save.
    if node.name == "t"
        && name == "LabelJustification"
        && node.attr("CaptionJustification").is_some()
    {
        return false;
    }
    true
}

pub(super) fn cdxml_lexical_type(value: &str) -> &'static str {
    let value = value.trim();
    if value.eq_ignore_ascii_case("yes") || value.eq_ignore_ascii_case("no") {
        return "boolean";
    }
    let tokens: Vec<&str> = value.split_whitespace().collect();
    if !tokens.is_empty() && tokens.iter().all(|token| token.parse::<f64>().is_ok()) {
        return if tokens.len() == 1 {
            "number"
        } else {
            "number-list"
        };
    }
    "string"
}

pub(super) fn apply_cdxml_groups(root: &XmlNode, objects: &mut Vec<SceneObject>) {
    let mut groups = Vec::new();
    let mut index = 1;
    collect_cdxml_groups(root, objects, &mut groups, &mut index);
    objects.extend(groups);
}

pub(super) fn collect_cdxml_groups(
    node: &XmlNode,
    objects: &mut Vec<SceneObject>,
    groups: &mut Vec<SceneObject>,
    index: &mut usize,
) {
    for child in &node.children {
        if child.is("group") {
            if let Some(group) = cdxml_group_object(child, objects, index) {
                groups.push(group);
            }
        } else {
            collect_cdxml_groups(child, objects, groups, index);
        }
    }
}

pub(super) fn cdxml_group_object(
    node: &XmlNode,
    objects: &mut Vec<SceneObject>,
    index: &mut usize,
) -> Option<SceneObject> {
    let group_number = *index;
    *index += 1;
    let mut children = Vec::new();
    for child in &node.children {
        if child.is("group") {
            if let Some(group) = cdxml_group_object(child, objects, index) {
                children.push(group);
            }
            continue;
        }
        children.extend(take_cdxml_child_objects(objects, child));
    }
    if children.is_empty() {
        return None;
    }
    let z_index = parse_i32(node.attr("Z")).unwrap_or_else(|| {
        children
            .iter()
            .map(|child| child.z_index)
            .min()
            .unwrap_or(0)
    });
    let payload_bbox = parse_bbox(node.attr("BoundingBox")).map(|bbox| {
        [
            round2(bbox[0]),
            round2(bbox[1]),
            round2(bbox[2] - bbox[0]),
            round2(bbox[3] - bbox[1]),
        ]
    });
    Some(SceneObject {
        id: format!("obj_group_{group_number:03}"),
        object_type: "group".to_string(),
        name: format!("group {group_number}"),
        visible: true,
        locked: false,
        z_index,
        transform: Transform::identity(),
        style_ref: None,
        link_policy: Default::default(),
        meta: json!({
            "source": "cdxml",
            "groupId": node.attr("id"),
            "import": {
                "cdxml": {
                    "groupId": node.attr("id"),
                    "boundingBox": node.attr("BoundingBox"),
                }
            },
        }),
        payload: ObjectPayload {
            resource_ref: None,
            bbox: payload_bbox,
            spectrum: None,
            geometry: None,
            constraint: None,
            table: None,
            stoichiometry_grid: None,
            gel_electrophoresis: None,
            plasmid_map: None,
            bio_shape: None,
            extra: BTreeMap::new(),
        },
        children,
    })
}

pub(super) fn take_cdxml_child_objects(
    objects: &mut Vec<SceneObject>,
    node: &XmlNode,
) -> Vec<SceneObject> {
    let source_id = node.attr("id");
    let mut taken = Vec::new();
    let mut index = 0;
    while index < objects.len() {
        if object_matches_cdxml_node(&objects[index], node, source_id) {
            taken.push(objects.remove(index));
        } else {
            index += 1;
        }
    }
    taken
}

pub(super) fn object_matches_cdxml_node(
    object: &SceneObject,
    node: &XmlNode,
    source_id: Option<&str>,
) -> bool {
    match node.name.as_str() {
        "fragment" => {
            source_id.is_some()
                && object.meta.get("fragmentId").and_then(Value::as_str) == source_id
        }
        "graphic" | "arrow" => {
            source_id.is_some()
                && (object.meta.get("graphicId").and_then(Value::as_str) == source_id
                    || object
                        .meta
                        .get("graphicIds")
                        .and_then(Value::as_array)
                        .is_some_and(|ids| ids.iter().any(|id| id.as_str() == source_id)))
        }
        "curve" => {
            if source_id.is_some() {
                object.meta.get("curveId").and_then(Value::as_str) == source_id
            } else {
                parse_cdxml_curve_points(node.attr("CurvePoints")).is_some_and(|points| {
                    object.payload.extra.get("curvePoints") == Some(&json!(points))
                })
            }
        }
        "t" => {
            source_id.is_some() && object.meta.get("textId").and_then(Value::as_str) == source_id
        }
        "spectrum" => {
            source_id.is_some()
                && object.meta.get("spectrumId").and_then(Value::as_str) == source_id
        }
        _ => false,
    }
}
