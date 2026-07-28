use super::*;

pub(super) fn remove_native_logical_objects(source: &mut crate::InterchangeObject) {
    // Constraint object tags are their native, editable value displays. The
    // constraint exporter regenerates them and merge_interchange_tree uses the
    // retained source child only for fields the native display does not own.
    if source.name == "constraint" {
        return;
    }
    source.children.retain(|child| {
        !matches!(
            child.name.as_str(),
            "scheme"
                | "altgroup"
                | "bracketedgroup"
                | "sequence"
                | "crossreference"
                | "objecttag"
                | "annotation"
                | "regnum"
                | "represent"
                | "splitter"
        )
    });
    for child in &mut source.children {
        remove_native_logical_objects(child);
    }
}

pub(super) fn apply_native_logical_objects(
    root: &mut crate::cdxml::xml::XmlNode,
    document: &ChemSemaDocument,
    entity_ids: &BTreeMap<String, String>,
) {
    let mut ids = LogicalXmlIds::new(root);
    let mut logical_xml_ids = BTreeMap::new();
    for group in &document.logical_objects.alternative_groups {
        logical_xml_ids.insert(group.id.clone(), ids.claim(&group.id));
    }
    for group in &document.logical_objects.bracketed_groups {
        logical_xml_ids.insert(group.id.clone(), ids.claim(&group.id));
        for attachment in &group.attachments {
            logical_xml_ids.insert(attachment.id.clone(), ids.claim(&attachment.id));
            for crossing in &attachment.crossing_bonds {
                logical_xml_ids.insert(crossing.id.clone(), ids.claim(&crossing.id));
            }
        }
    }
    for sequence in &document.logical_objects.sequences {
        logical_xml_ids.insert(sequence.id.clone(), ids.claim(&sequence.id));
    }
    for cross_reference in &document.logical_objects.cross_references {
        logical_xml_ids.insert(cross_reference.id.clone(), ids.claim(&cross_reference.id));
    }
    for tag in &document.logical_objects.object_tags {
        logical_xml_ids.insert(tag.id.clone(), ids.claim(&tag.id));
    }
    for annotation in &document.logical_objects.annotations {
        logical_xml_ids.insert(annotation.id.clone(), ids.claim(&annotation.id));
    }
    for registration in &document.logical_objects.registry_numbers {
        logical_xml_ids.insert(registration.id.clone(), ids.claim(&registration.id));
    }

    write_alternative_groups(root, document, entity_ids, &logical_xml_ids);
    write_bracketed_groups(root, document, entity_ids, &logical_xml_ids);
    write_sequences(root, document, entity_ids, &logical_xml_ids);
    write_attached_metadata(root, document, entity_ids, &logical_xml_ids);
}

fn write_alternative_groups(
    root: &mut crate::cdxml::xml::XmlNode,
    document: &ChemSemaDocument,
    entity_ids: &BTreeMap<String, String>,
    logical_ids: &BTreeMap<String, String>,
) {
    let colors = CdxmlColorTable::from_cdxml(root);
    for group in &document.logical_objects.alternative_groups {
        let Some(xml_id) = logical_ids.get(&group.id) else {
            continue;
        };
        let mut children = Vec::new();
        let mut seen = BTreeSet::new();
        for entity_id in &group.member_entity_ids {
            let Some(source_id) = entity_ids.get(entity_id) else {
                continue;
            };
            if seen.insert(source_id.clone()) {
                if let Some(child) = take_xml_node_by_id(root, source_id) {
                    children.push(child);
                }
            }
        }
        if children.is_empty() {
            continue;
        }
        let mut attrs = BTreeMap::from([("id".to_string(), xml_id.clone())]);
        if let Some(valence) = group.valence {
            attrs.insert("Valence".to_string(), valence.to_string());
        }
        if let Some([x, y]) = group.position {
            attrs.insert("p".to_string(), format!("{} {}", fmt_num(x), fmt_num(y)));
        }
        insert_optional_box(&mut attrs, "BoundingBox", group.bounding_box);
        insert_optional_box(&mut attrs, "TextFrame", group.text_frame);
        insert_optional_box(&mut attrs, "GroupFrame", group.group_frame);
        insert_optional_number(&mut attrs, "alpha", group.opacity);
        if let Some(color) = &group.color {
            attrs.insert("color".to_string(), colors.id_for(color));
        }
        if let Some(z_index) = group.z_index {
            attrs.insert("Z".to_string(), z_index.to_string());
        }
        if !group.visible {
            attrs.insert("Visible".to_string(), "no".to_string());
        }
        if group.ignore_warnings {
            attrs.insert("IgnoreWarnings".to_string(), "yes".to_string());
        }
        if let Some(warning) = &group.warning {
            attrs.insert("Warning".to_string(), warning.clone());
        }
        if let Some(target) = group
            .superseded_by_id
            .as_ref()
            .and_then(|id| logical_ids.get(id).or_else(|| entity_ids.get(id)))
            .cloned()
            .or_else(|| group.unresolved_superseded_by_source_id.clone())
        {
            attrs.insert("SupersededBy".to_string(), target);
        }
        page_mut(root).children.push(crate::cdxml::xml::XmlNode {
            name: "altgroup".to_string(),
            attrs,
            text: String::new(),
            children,
        });
        for node_id in &group.attachment_node_ids {
            let Some(node_xml_id) = entity_ids.get(node_id) else {
                continue;
            };
            if let Some(node) = find_xml_node_mut_by_name_and_id(root, "n", node_xml_id) {
                node.attrs.insert("AltGroupID".to_string(), xml_id.clone());
            }
        }
    }
}

fn write_bracketed_groups(
    root: &mut crate::cdxml::xml::XmlNode,
    document: &ChemSemaDocument,
    entity_ids: &BTreeMap<String, String>,
    logical_ids: &BTreeMap<String, String>,
) {
    let groups = document
        .logical_objects
        .bracketed_groups
        .iter()
        .map(|group| (group.id.as_str(), group))
        .collect::<BTreeMap<_, _>>();
    let nested_ids = groups
        .values()
        .flat_map(|group| group.nested_group_ids.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    let mut nodes = Vec::new();
    for group in groups
        .values()
        .filter(|group| !nested_ids.contains(group.id.as_str()))
    {
        if let Some(node) = build_bracketed_group_node(
            group,
            &groups,
            entity_ids,
            logical_ids,
            &mut BTreeSet::new(),
        ) {
            nodes.push(node);
        }
    }
    page_mut(root).children.extend(nodes);
}

fn build_bracketed_group_node(
    group: &crate::BracketedGroupData,
    groups: &BTreeMap<&str, &crate::BracketedGroupData>,
    entity_ids: &BTreeMap<String, String>,
    logical_ids: &BTreeMap<String, String>,
    active: &mut BTreeSet<String>,
) -> Option<crate::cdxml::xml::XmlNode> {
    if !active.insert(group.id.clone()) {
        return None;
    }
    let bracketed_ids = group
        .bracketed_entity_ids
        .iter()
        .filter_map(|id| entity_ids.get(id).cloned())
        .chain(group.unresolved_bracketed_source_ids.iter().cloned())
        .collect::<Vec<_>>();
    if bracketed_ids.is_empty() || group.attachments.is_empty() {
        active.remove(&group.id);
        return None;
    }
    let mut attrs = BTreeMap::from([
        (
            "id".to_string(),
            logical_ids
                .get(&group.id)
                .cloned()
                .unwrap_or_else(|| group.id.clone()),
        ),
        ("BracketedObjectIDs".to_string(), bracketed_ids.join(" ")),
        (
            "BracketUsage".to_string(),
            group.usage.as_cdxml().to_string(),
        ),
    ]);
    if let Some(component_order) = group.component_order {
        attrs.insert("ComponentOrder".to_string(), component_order.to_string());
    }
    attrs.insert(
        "PolymerRepeatPattern".to_string(),
        group.polymer_repeat_pattern.as_cdxml().to_string(),
    );
    attrs.insert(
        "PolymerFlipType".to_string(),
        group.polymer_flip_type.as_cdxml().to_string(),
    );
    insert_optional_number(&mut attrs, "RepeatCount", group.repeat_count);
    if let Some(label) = &group.sru_label {
        attrs.insert("SRULabel".to_string(), label.clone());
    }
    let mut children = Vec::new();
    for attachment in &group.attachments {
        let graphic_id = attachment
            .bracket_object_id
            .as_ref()
            .and_then(|id| entity_ids.get(id))
            .cloned()
            .or_else(|| attachment.unresolved_bracket_source_id.clone());
        let Some(graphic_id) = graphic_id else {
            continue;
        };
        let mut crossing_children = Vec::new();
        for crossing in &attachment.crossing_bonds {
            let bond_id = crossing
                .bond_id
                .as_ref()
                .and_then(|id| entity_ids.get(id))
                .cloned()
                .or_else(|| crossing.unresolved_bond_source_id.clone());
            let atom_id = crossing
                .inner_atom_id
                .as_ref()
                .and_then(|id| entity_ids.get(id))
                .cloned()
                .or_else(|| crossing.unresolved_inner_atom_source_id.clone());
            let (Some(bond_id), Some(atom_id)) = (bond_id, atom_id) else {
                continue;
            };
            crossing_children.push(crate::cdxml::xml::XmlNode {
                name: "crossingbond".to_string(),
                attrs: BTreeMap::from([
                    (
                        "id".to_string(),
                        logical_ids
                            .get(&crossing.id)
                            .cloned()
                            .unwrap_or_else(|| crossing.id.clone()),
                    ),
                    ("BondID".to_string(), bond_id),
                    ("InnerAtomID".to_string(), atom_id),
                ]),
                text: String::new(),
                children: Vec::new(),
            });
        }
        children.push(crate::cdxml::xml::XmlNode {
            name: "bracketattachment".to_string(),
            attrs: BTreeMap::from([
                (
                    "id".to_string(),
                    logical_ids
                        .get(&attachment.id)
                        .cloned()
                        .unwrap_or_else(|| attachment.id.clone()),
                ),
                ("GraphicID".to_string(), graphic_id),
            ]),
            text: String::new(),
            children: crossing_children,
        });
    }
    for child_id in &group.nested_group_ids {
        if let Some(child) = groups.get(child_id.as_str()).and_then(|child| {
            build_bracketed_group_node(child, groups, entity_ids, logical_ids, active)
        }) {
            children.push(child);
        }
    }
    active.remove(&group.id);
    (!children.is_empty()).then_some(crate::cdxml::xml::XmlNode {
        name: "bracketedgroup".to_string(),
        attrs,
        text: String::new(),
        children,
    })
}

fn write_sequences(
    root: &mut crate::cdxml::xml::XmlNode,
    document: &ChemSemaDocument,
    entity_ids: &BTreeMap<String, String>,
    logical_ids: &BTreeMap<String, String>,
) {
    for sequence in &document.logical_objects.sequences {
        let children = take_entity_nodes(root, &sequence.text_object_ids, entity_ids);
        page_mut(root).children.push(crate::cdxml::xml::XmlNode {
            name: "sequence".to_string(),
            attrs: BTreeMap::from([
                (
                    "id".to_string(),
                    logical_ids
                        .get(&sequence.id)
                        .cloned()
                        .unwrap_or_else(|| sequence.id.clone()),
                ),
                (
                    "SequenceIdentifier".to_string(),
                    sequence.identifier.clone(),
                ),
            ]),
            text: String::new(),
            children,
        });
    }
    for cross_reference in &document.logical_objects.cross_references {
        let mut attrs = BTreeMap::from([
            (
                "id".to_string(),
                logical_ids
                    .get(&cross_reference.id)
                    .cloned()
                    .unwrap_or_else(|| cross_reference.id.clone()),
            ),
            (
                "CrossReferenceIdentifier".to_string(),
                cross_reference.identifier.clone(),
            ),
            (
                "CrossReferenceSequence".to_string(),
                cross_reference.sequence_identifier.clone(),
            ),
        ]);
        if let Some(value) = &cross_reference.container {
            attrs.insert("CrossReferenceContainer".to_string(), value.clone());
        }
        if let Some(value) = &cross_reference.document {
            attrs.insert("CrossReferenceDocument".to_string(), value.clone());
        }
        let children = take_entity_nodes(root, &cross_reference.text_object_ids, entity_ids);
        page_mut(root).children.push(crate::cdxml::xml::XmlNode {
            name: "crossreference".to_string(),
            attrs,
            text: String::new(),
            children,
        });
    }
}

fn write_attached_metadata(
    root: &mut crate::cdxml::xml::XmlNode,
    document: &ChemSemaDocument,
    entity_ids: &BTreeMap<String, String>,
    logical_ids: &BTreeMap<String, String>,
) {
    for tag in &document.logical_objects.object_tags {
        let children = take_entity_nodes(root, &tag.display_object_ids, entity_ids);
        let mut attrs = BTreeMap::from([
            (
                "id".to_string(),
                logical_ids
                    .get(&tag.id)
                    .cloned()
                    .unwrap_or_else(|| tag.id.clone()),
            ),
            ("Name".to_string(), tag.name.clone()),
            ("TagType".to_string(), tag.tag_type.as_cdxml().to_string()),
        ]);
        if let Some(value) = &tag.display_name {
            attrs.insert("DisplayName".to_string(), value.clone());
        }
        if let Some(value) = &tag.value {
            attrs.insert("Value".to_string(), value.clone());
        }
        if tag.positioning_type != crate::AnnotationPositioningType::Auto {
            attrs.insert(
                "PositioningType".to_string(),
                tag.positioning_type.as_cdxml().to_string(),
            );
        }
        insert_optional_number(&mut attrs, "PositioningAngle", tag.positioning_angle);
        if let Some([x, y]) = tag.positioning_offset {
            attrs.insert(
                "PositioningOffset".to_string(),
                format!("{} {}", fmt_num(x), fmt_num(y)),
            );
        }
        insert_false(&mut attrs, "Persistent", tag.persistent);
        insert_false(&mut attrs, "Tracking", tag.tracking);
        insert_false(&mut attrs, "Visible", tag.visible);
        attach_logical_node(
            root,
            document,
            tag.owner_entity_id.as_ref(),
            tag.unresolved_owner_source_id.as_ref(),
            entity_ids,
            crate::cdxml::xml::XmlNode {
                name: "objecttag".to_string(),
                attrs,
                text: String::new(),
                children,
            },
        );
    }
    for annotation in &document.logical_objects.annotations {
        let mut attrs = BTreeMap::from([(
            "id".to_string(),
            logical_ids
                .get(&annotation.id)
                .cloned()
                .unwrap_or_else(|| annotation.id.clone()),
        )]);
        if let Some(value) = &annotation.keyword {
            attrs.insert("Keyword".to_string(), value.clone());
        }
        if let Some(value) = &annotation.content {
            attrs.insert("Content".to_string(), value.clone());
        }
        attach_logical_node(
            root,
            document,
            annotation.owner_entity_id.as_ref(),
            annotation.unresolved_owner_source_id.as_ref(),
            entity_ids,
            crate::cdxml::xml::XmlNode {
                name: "annotation".to_string(),
                attrs,
                text: String::new(),
                children: Vec::new(),
            },
        );
    }
    for registration in &document.logical_objects.registry_numbers {
        attach_logical_node(
            root,
            document,
            registration.owner_entity_id.as_ref(),
            registration.unresolved_owner_source_id.as_ref(),
            entity_ids,
            crate::cdxml::xml::XmlNode {
                name: "regnum".to_string(),
                attrs: BTreeMap::from([
                    (
                        "id".to_string(),
                        logical_ids
                            .get(&registration.id)
                            .cloned()
                            .unwrap_or_else(|| registration.id.clone()),
                    ),
                    (
                        "RegistryAuthority".to_string(),
                        registration.authority.clone(),
                    ),
                    ("RegistryNumber".to_string(), registration.number.clone()),
                ]),
                text: String::new(),
                children: Vec::new(),
            },
        );
    }
    for representation in &document.logical_objects.representations {
        let target_id = representation
            .target_entity_id
            .as_ref()
            .and_then(|id| entity_ids.get(id))
            .cloned()
            .or_else(|| representation.unresolved_target_source_id.clone());
        let Some(target_id) = target_id else {
            continue;
        };
        attach_logical_node(
            root,
            document,
            representation.owner_entity_id.as_ref(),
            representation.unresolved_owner_source_id.as_ref(),
            entity_ids,
            crate::cdxml::xml::XmlNode {
                name: "represent".to_string(),
                attrs: BTreeMap::from([
                    ("object".to_string(), target_id),
                    ("attribute".to_string(), representation.attribute.clone()),
                ]),
                text: String::new(),
                children: Vec::new(),
            },
        );
    }
}

fn attach_logical_node(
    root: &mut crate::cdxml::xml::XmlNode,
    document: &ChemSemaDocument,
    owner_entity_id: Option<&String>,
    unresolved_owner_source_id: Option<&String>,
    entity_ids: &BTreeMap<String, String>,
    node: crate::cdxml::xml::XmlNode,
) {
    let owner_xml_id = owner_entity_id
        .and_then(|id| entity_ids.get(id))
        .or(unresolved_owner_source_id)
        .cloned();
    let molecular_owner_tag = owner_entity_id.and_then(|id| molecular_owner_xml_tag(document, id));
    let owner_exists = owner_xml_id.as_deref().is_some_and(|id| {
        molecular_owner_tag.map_or_else(
            || xml_node_contains_id(root, id),
            |tag| xml_node_contains_name_and_id(root, tag, id),
        )
    });
    let parent = if owner_exists {
        let id = owner_xml_id
            .as_deref()
            .expect("checked logical owner id must exist");
        match molecular_owner_tag {
            Some(tag) => find_xml_node_mut_by_name_and_id(root, tag, id),
            None => find_xml_node_mut_by_id(root, id),
        }
        .expect("checked logical owner node must exist")
    } else {
        page_mut(root)
    };
    parent.children.push(node);
}

fn molecular_owner_xml_tag<'a>(
    document: &'a ChemSemaDocument,
    owner_entity_id: &str,
) -> Option<&'static str> {
    for fragment in document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
    {
        if fragment.nodes.iter().any(|node| node.id == owner_entity_id) {
            return Some("n");
        }
        if fragment.bonds.iter().any(|bond| bond.id == owner_entity_id) {
            return Some("b");
        }
    }
    None
}

fn take_entity_nodes(
    root: &mut crate::cdxml::xml::XmlNode,
    entity_ids_to_take: &[String],
    entity_ids: &BTreeMap<String, String>,
) -> Vec<crate::cdxml::xml::XmlNode> {
    entity_ids_to_take
        .iter()
        .filter_map(|id| entity_ids.get(id))
        .filter_map(|xml_id| take_xml_node_by_id(root, xml_id))
        .collect()
}

fn find_xml_node_mut_by_id<'a>(
    node: &'a mut crate::cdxml::xml::XmlNode,
    id: &str,
) -> Option<&'a mut crate::cdxml::xml::XmlNode> {
    if node.attr("id") == Some(id) {
        return Some(node);
    }
    node.children
        .iter_mut()
        .find_map(|child| find_xml_node_mut_by_id(child, id))
}

fn find_xml_node_mut_by_name_and_id<'a>(
    node: &'a mut crate::cdxml::xml::XmlNode,
    name: &str,
    id: &str,
) -> Option<&'a mut crate::cdxml::xml::XmlNode> {
    if node.name == name && node.attr("id") == Some(id) {
        return Some(node);
    }
    node.children
        .iter_mut()
        .find_map(|child| find_xml_node_mut_by_name_and_id(child, name, id))
}

fn xml_node_contains_id(node: &crate::cdxml::xml::XmlNode, id: &str) -> bool {
    node.attr("id") == Some(id)
        || node
            .children
            .iter()
            .any(|child| xml_node_contains_id(child, id))
}

fn xml_node_contains_name_and_id(node: &crate::cdxml::xml::XmlNode, name: &str, id: &str) -> bool {
    (node.name == name && node.attr("id") == Some(id))
        || node
            .children
            .iter()
            .any(|child| xml_node_contains_name_and_id(child, name, id))
}

fn take_xml_node_by_id(
    node: &mut crate::cdxml::xml::XmlNode,
    id: &str,
) -> Option<crate::cdxml::xml::XmlNode> {
    if let Some(index) = node
        .children
        .iter()
        .position(|child| child.attr("id") == Some(id))
    {
        return Some(node.children.remove(index));
    }
    node.children
        .iter_mut()
        .find_map(|child| take_xml_node_by_id(child, id))
}

fn page_mut(root: &mut crate::cdxml::xml::XmlNode) -> &mut crate::cdxml::xml::XmlNode {
    let index = root
        .children
        .iter()
        .position(|child| child.name == "page")
        .expect("generated CDXML must contain a page");
    &mut root.children[index]
}

fn insert_optional_number(attrs: &mut BTreeMap<String, String>, name: &str, value: Option<f64>) {
    if let Some(value) = value {
        attrs.insert(name.to_string(), fmt_num(value));
    }
}

fn insert_optional_box(attrs: &mut BTreeMap<String, String>, name: &str, value: Option<[f64; 4]>) {
    if let Some(value) = value {
        attrs.insert(name.to_string(), fmt_bbox(value));
    }
}

fn insert_false(attrs: &mut BTreeMap<String, String>, name: &str, value: bool) {
    if !value {
        attrs.insert(name.to_string(), "no".to_string());
    }
}

struct LogicalXmlIds {
    used: BTreeSet<u64>,
    next: u64,
}

impl LogicalXmlIds {
    fn new(root: &crate::cdxml::xml::XmlNode) -> Self {
        let mut used = BTreeSet::new();
        collect_xml_ids(root, &mut used);
        let next = used.iter().next_back().copied().unwrap_or(0) + 1;
        Self { used, next }
    }

    fn claim(&mut self, preferred: &str) -> String {
        if let Ok(id) = preferred.parse::<u64>() {
            if id > 0 && self.used.insert(id) {
                return preferred.to_string();
            }
        }
        while !self.used.insert(self.next) {
            self.next += 1;
        }
        let id = self.next;
        self.next += 1;
        id.to_string()
    }
}

fn collect_xml_ids(node: &crate::cdxml::xml::XmlNode, out: &mut BTreeSet<u64>) {
    if let Some(id) = node.attr("id").and_then(|value| value.parse().ok()) {
        out.insert(id);
    }
    for child in &node.children {
        collect_xml_ids(child, out);
    }
}
