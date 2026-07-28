use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LogicalObjectData {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternative_groups: Vec<AlternativeGroupData>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bracketed_groups: Vec<BracketedGroupData>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sequences: Vec<SequenceData>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cross_references: Vec<CrossReferenceData>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub object_tags: Vec<ObjectTagData>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<AnnotationData>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub registry_numbers: Vec<RegistryNumberData>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub representations: Vec<RepresentationData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum LogicalBindingOrigin {
    Authored,
    Imported,
    Inferred,
    #[default]
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AlternativeGroupData {
    pub id: String,
    #[serde(default)]
    pub member_entity_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved_member_source_ids: Vec<String>,
    #[serde(default)]
    pub attachment_node_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valence: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounding_box: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_frame: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_frame: Option<[f64; 4]>,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub ignore_warnings: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(default)]
    pub binding_origin: LogicalBindingOrigin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BracketedGroupData {
    pub id: String,
    #[serde(default)]
    pub bracket_object_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved_bracket_source_ids: Vec<String>,
    #[serde(default)]
    pub bracketed_entity_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved_bracketed_source_ids: Vec<String>,
    #[serde(default)]
    pub attachments: Vec<BracketAttachmentData>,
    #[serde(default)]
    pub usage: BracketUsage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component_order: Option<i16>,
    #[serde(default)]
    pub polymer_repeat_pattern: PolymerRepeatPattern,
    #[serde(default)]
    pub polymer_flip_type: PolymerFlipType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_count: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sru_label: Option<String>,
    #[serde(default)]
    pub binding_origin: LogicalBindingOrigin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BracketAttachmentData {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bracket_object_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved_bracket_source_id: Option<String>,
    #[serde(default)]
    pub crossing_bonds: Vec<CrossingBondData>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CrossingBondData {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bond_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved_bond_source_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inner_atom_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved_inner_atom_source_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum BracketUsage {
    #[default]
    Unspecified,
    AnyPolymer,
    Component,
    Copolymer,
    CopolymerAlternating,
    CopolymerBlock,
    CopolymerRandom,
    Crosslink,
    Generic,
    Graft,
    Mer,
    MixtureOrdered,
    MixtureUnordered,
    Modification,
    Monomer,
    MultipleGroup,
    Sru,
}

impl BracketUsage {
    pub fn from_cdxml(value: Option<&str>) -> Self {
        match value.unwrap_or_default() {
            "Anypolymer" => Self::AnyPolymer,
            "Component" => Self::Component,
            "Copolymer" => Self::Copolymer,
            "CopolymerAlternating" => Self::CopolymerAlternating,
            "CopolymerBlock" => Self::CopolymerBlock,
            "CopolymerRandom" => Self::CopolymerRandom,
            "Crosslink" => Self::Crosslink,
            "Generic" => Self::Generic,
            "Graft" => Self::Graft,
            "Mer" => Self::Mer,
            "MixtureOrdered" => Self::MixtureOrdered,
            "MixtureUnordered" => Self::MixtureUnordered,
            "Modification" => Self::Modification,
            "Monomer" => Self::Monomer,
            "MultipleGroup" => Self::MultipleGroup,
            "SRU" => Self::Sru,
            _ => Self::Unspecified,
        }
    }

    pub const fn as_cdxml(self) -> &'static str {
        match self {
            Self::Unspecified => "Unspecified",
            Self::AnyPolymer => "Anypolymer",
            Self::Component => "Component",
            Self::Copolymer => "Copolymer",
            Self::CopolymerAlternating => "CopolymerAlternating",
            Self::CopolymerBlock => "CopolymerBlock",
            Self::CopolymerRandom => "CopolymerRandom",
            Self::Crosslink => "Crosslink",
            Self::Generic => "Generic",
            Self::Graft => "Graft",
            Self::Mer => "Mer",
            Self::MixtureOrdered => "MixtureOrdered",
            Self::MixtureUnordered => "MixtureUnordered",
            Self::Modification => "Modification",
            Self::Monomer => "Monomer",
            Self::MultipleGroup => "MultipleGroup",
            Self::Sru => "SRU",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PolymerRepeatPattern {
    HeadToTail,
    HeadToHead,
    #[default]
    EitherUnknown,
}

impl PolymerRepeatPattern {
    pub fn from_cdxml(value: Option<&str>) -> Self {
        match value.unwrap_or_default() {
            "HeadToTail" => Self::HeadToTail,
            "HeadToHead" => Self::HeadToHead,
            _ => Self::EitherUnknown,
        }
    }

    pub const fn as_cdxml(self) -> &'static str {
        match self {
            Self::HeadToTail => "HeadToTail",
            Self::HeadToHead => "HeadToHead",
            Self::EitherUnknown => "EitherUnknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PolymerFlipType {
    #[default]
    Unspecified,
    NoFlip,
    Flip,
}

impl PolymerFlipType {
    pub fn from_cdxml(value: Option<&str>) -> Self {
        match value.unwrap_or_default() {
            "NoFlip" => Self::NoFlip,
            "Flip" => Self::Flip,
            _ => Self::Unspecified,
        }
    }

    pub const fn as_cdxml(self) -> &'static str {
        match self {
            Self::Unspecified => "Unspecified",
            Self::NoFlip => "NoFlip",
            Self::Flip => "Flip",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SequenceData {
    pub id: String,
    pub identifier: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub text_object_ids: Vec<String>,
    #[serde(default)]
    pub binding_origin: LogicalBindingOrigin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CrossReferenceData {
    pub id: String,
    pub identifier: String,
    pub sequence_identifier: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub text_object_ids: Vec<String>,
    #[serde(default)]
    pub binding_origin: LogicalBindingOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ObjectTagType {
    #[default]
    Unknown,
    String,
    Long,
    Double,
}

impl ObjectTagType {
    pub fn from_cdxml(value: Option<&str>) -> Self {
        match value.unwrap_or_default() {
            "String" => Self::String,
            "Long" => Self::Long,
            "Double" => Self::Double,
            _ => Self::Unknown,
        }
    }

    pub const fn as_cdxml(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::String => "String",
            Self::Long => "Long",
            Self::Double => "Double",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectTagData {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_entity_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved_owner_source_id: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default)]
    pub tag_type: ObjectTagType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default)]
    pub positioning_type: crate::AnnotationPositioningType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub positioning_angle: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub positioning_offset: Option<[f64; 2]>,
    #[serde(default = "default_true")]
    pub persistent: bool,
    #[serde(default = "default_true")]
    pub tracking: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub display_object_ids: Vec<String>,
    #[serde(default)]
    pub binding_origin: LogicalBindingOrigin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationData {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_entity_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved_owner_source_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default)]
    pub binding_origin: LogicalBindingOrigin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RegistryNumberData {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_entity_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved_owner_source_id: Option<String>,
    pub authority: String,
    pub number: String,
    #[serde(default)]
    pub binding_origin: LogicalBindingOrigin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RepresentationData {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_entity_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved_owner_source_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_entity_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved_target_source_id: Option<String>,
    pub attribute: String,
    #[serde(default)]
    pub binding_origin: LogicalBindingOrigin,
}

fn default_true() -> bool {
    true
}

impl LogicalObjectData {
    pub fn is_empty(&self) -> bool {
        self.alternative_groups.is_empty()
            && self.bracketed_groups.is_empty()
            && self.sequences.is_empty()
            && self.cross_references.is_empty()
            && self.object_tags.is_empty()
            && self.annotations.is_empty()
            && self.registry_numbers.is_empty()
            && self.representations.is_empty()
    }

    pub fn all_ids(&self) -> Vec<&str> {
        let mut ids = Vec::new();
        ids.extend(self.alternative_groups.iter().map(|item| item.id.as_str()));
        for group in &self.bracketed_groups {
            ids.push(group.id.as_str());
            for attachment in &group.attachments {
                ids.push(attachment.id.as_str());
                ids.extend(
                    attachment
                        .crossing_bonds
                        .iter()
                        .map(|item| item.id.as_str()),
                );
            }
        }
        ids.extend(self.sequences.iter().map(|item| item.id.as_str()));
        ids.extend(self.cross_references.iter().map(|item| item.id.as_str()));
        ids.extend(self.object_tags.iter().map(|item| item.id.as_str()));
        ids.extend(self.annotations.iter().map(|item| item.id.as_str()));
        ids.extend(self.registry_numbers.iter().map(|item| item.id.as_str()));
        ids.extend(self.representations.iter().map(|item| item.id.as_str()));
        ids
    }

    pub fn subset_for_entities(&self, selected_entity_ids: &BTreeSet<String>) -> Self {
        let all_selected = |ids: &[String]| ids.iter().all(|id| selected_entity_ids.contains(id));
        let owner_selected = |owner: &Option<String>| {
            owner
                .as_ref()
                .is_some_and(|id| selected_entity_ids.contains(id))
        };
        let mut logical = Self {
            alternative_groups: self
                .alternative_groups
                .iter()
                .filter(|group| {
                    all_selected(&group.member_entity_ids)
                        && all_selected(&group.attachment_node_ids)
                        && (!group.member_entity_ids.is_empty()
                            || !group.attachment_node_ids.is_empty())
                })
                .cloned()
                .collect(),
            bracketed_groups: self
                .bracketed_groups
                .iter()
                .filter(|group| {
                    all_selected(&group.bracket_object_ids)
                        && all_selected(&group.bracketed_entity_ids)
                        && group.attachments.iter().all(|attachment| {
                            attachment
                                .bracket_object_id
                                .as_ref()
                                .is_none_or(|id| selected_entity_ids.contains(id))
                                && attachment.crossing_bonds.iter().all(|crossing| {
                                    crossing
                                        .bond_id
                                        .as_ref()
                                        .is_none_or(|id| selected_entity_ids.contains(id))
                                        && crossing
                                            .inner_atom_id
                                            .as_ref()
                                            .is_none_or(|id| selected_entity_ids.contains(id))
                                })
                        })
                })
                .cloned()
                .collect(),
            sequences: self
                .sequences
                .iter()
                .filter(|sequence| {
                    !sequence.text_object_ids.is_empty() && all_selected(&sequence.text_object_ids)
                })
                .cloned()
                .collect(),
            cross_references: self
                .cross_references
                .iter()
                .filter(|cross_reference| {
                    !cross_reference.text_object_ids.is_empty()
                        && all_selected(&cross_reference.text_object_ids)
                })
                .cloned()
                .collect(),
            object_tags: self
                .object_tags
                .iter()
                .filter(|tag| {
                    owner_selected(&tag.owner_entity_id) && all_selected(&tag.display_object_ids)
                })
                .cloned()
                .collect(),
            annotations: self
                .annotations
                .iter()
                .filter(|annotation| owner_selected(&annotation.owner_entity_id))
                .cloned()
                .collect(),
            registry_numbers: self
                .registry_numbers
                .iter()
                .filter(|registration| owner_selected(&registration.owner_entity_id))
                .cloned()
                .collect(),
            representations: self
                .representations
                .iter()
                .filter(|representation| {
                    representation
                        .owner_entity_id
                        .as_ref()
                        .is_some_and(|id| selected_entity_ids.contains(id))
                        && representation
                            .target_entity_id
                            .as_ref()
                            .is_some_and(|id| selected_entity_ids.contains(id))
                })
                .cloned()
                .collect(),
        };
        let selected_sequences = logical
            .sequences
            .iter()
            .map(|sequence| sequence.identifier.as_str())
            .collect::<BTreeSet<_>>();
        logical.cross_references.retain(|cross_reference| {
            cross_reference.container.is_some()
                || cross_reference.document.is_some()
                || selected_sequences.contains(cross_reference.sequence_identifier.as_str())
        });
        logical
    }

    pub fn validate(
        &self,
        scene_ids: &BTreeSet<String>,
        node_ids: &BTreeSet<String>,
        bond_ids: &BTreeSet<String>,
    ) -> Result<(), String> {
        let mut context = LogicalValidationContext::new(scene_ids, node_ids, bond_ids);
        self.validate_alternative_groups(&mut context)?;
        self.validate_bracketed_groups(&mut context)?;
        self.validate_sequences(&mut context)?;
        self.validate_metadata(&mut context)
    }

    fn validate_alternative_groups(
        &self,
        context: &mut LogicalValidationContext<'_>,
    ) -> Result<(), String> {
        let mut alternative_members = BTreeSet::new();
        for group in &self.alternative_groups {
            context.register("alternative group", &group.id)?;
            if group.member_entity_ids.is_empty()
                && group.unresolved_member_source_ids.is_empty()
                && group.attachment_node_ids.is_empty()
            {
                return Err(format!("alternative group '{}' is empty", group.id));
            }
            validate_unique_existing(
                "alternative group member",
                &group.member_entity_ids,
                &|id| context.entity_exists(id),
            )?;
            for member_id in &group.member_entity_ids {
                if !alternative_members.insert(member_id) {
                    return Err(format!(
                        "alternative group member '{member_id}' belongs to more than one group"
                    ));
                }
            }
            validate_unique_nonempty(
                "unresolved alternative group member",
                &group.unresolved_member_source_ids,
            )?;
            validate_unique_existing(
                "alternative group attachment",
                &group.attachment_node_ids,
                &|id| context.node_ids.contains(id),
            )?;
            validate_optional_box("alternative group boundingBox", group.bounding_box)?;
            validate_optional_box("alternative group textFrame", group.text_frame)?;
            validate_optional_box("alternative group groupFrame", group.group_frame)?;
        }
        Ok(())
    }

    fn validate_bracketed_groups(
        &self,
        context: &mut LogicalValidationContext<'_>,
    ) -> Result<(), String> {
        for group in &self.bracketed_groups {
            context.register("bracketed group", &group.id)?;
            if group.attachments.is_empty()
                || (group.bracketed_entity_ids.is_empty()
                    && group.unresolved_bracketed_source_ids.is_empty())
            {
                return Err(format!(
                    "bracketed group '{}' has no attachment or bracketed object",
                    group.id
                ));
            }
            validate_unique_existing("bracket graphic", &group.bracket_object_ids, &|id| {
                context.scene_ids.contains(id)
            })?;
            validate_unique_existing("bracketed entity", &group.bracketed_entity_ids, &|id| {
                context.entity_exists(id)
            })?;
            validate_unique_nonempty(
                "unresolved bracket graphic",
                &group.unresolved_bracket_source_ids,
            )?;
            validate_unique_nonempty(
                "unresolved bracketed entity",
                &group.unresolved_bracketed_source_ids,
            )?;
            if group
                .repeat_count
                .is_some_and(|value| !value.is_finite() || value <= 0.0)
            {
                return Err(format!(
                    "bracketed group '{}' has invalid repeatCount",
                    group.id
                ));
            }
            for attachment in &group.attachments {
                context.register("bracket attachment", &attachment.id)?;
                if attachment.bracket_object_id.is_some()
                    && attachment.unresolved_bracket_source_id.is_some()
                {
                    return Err(format!(
                        "bracket attachment '{}' has both resolved and unresolved graphics",
                        attachment.id
                    ));
                }
                if attachment
                    .bracket_object_id
                    .as_ref()
                    .is_some_and(|id| !context.scene_ids.contains(id))
                    || (attachment.bracket_object_id.is_none()
                        && !has_unresolved(&attachment.unresolved_bracket_source_id))
                {
                    return Err(format!(
                        "bracket attachment '{}' has a missing or absent graphic",
                        attachment.id
                    ));
                }
                for crossing in &attachment.crossing_bonds {
                    context.register("crossing bond", &crossing.id)?;
                    if (crossing.bond_id.is_some() && crossing.unresolved_bond_source_id.is_some())
                        || (crossing.inner_atom_id.is_some()
                            && crossing.unresolved_inner_atom_source_id.is_some())
                    {
                        return Err(format!(
                            "crossing bond '{}' mixes resolved and unresolved references",
                            crossing.id
                        ));
                    }
                    if crossing
                        .bond_id
                        .as_ref()
                        .is_some_and(|id| !context.bond_ids.contains(id))
                        || (crossing.bond_id.is_none()
                            && !has_unresolved(&crossing.unresolved_bond_source_id))
                        || crossing
                            .inner_atom_id
                            .as_ref()
                            .is_some_and(|id| !context.node_ids.contains(id))
                        || (crossing.inner_atom_id.is_none()
                            && !has_unresolved(&crossing.unresolved_inner_atom_source_id))
                    {
                        return Err(format!(
                            "crossing bond '{}' references a missing bond or inner atom",
                            crossing.id
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_sequences(&self, context: &mut LogicalValidationContext<'_>) -> Result<(), String> {
        for sequence in &self.sequences {
            context.register("sequence", &sequence.id)?;
            if sequence.identifier.trim().is_empty()
                || !context
                    .sequence_identifiers
                    .insert(sequence.identifier.clone())
            {
                return Err(format!(
                    "sequence identifier '{}' is empty or duplicated",
                    sequence.identifier
                ));
            }
            validate_unique_existing("sequence text", &sequence.text_object_ids, &|id| {
                context.scene_ids.contains(id)
            })?;
            validate_display_ownership(
                "sequence",
                &sequence.id,
                &sequence.text_object_ids,
                &mut context.display_object_owners,
            )?;
        }
        let mut cross_reference_identifiers = BTreeSet::new();
        for cross_reference in &self.cross_references {
            context.register("cross reference", &cross_reference.id)?;
            if cross_reference.identifier.trim().is_empty()
                || cross_reference.sequence_identifier.trim().is_empty()
            {
                return Err(format!(
                    "cross reference '{}' is missing a required identifier",
                    cross_reference.id
                ));
            }
            if !cross_reference_identifiers.insert(cross_reference.identifier.as_str()) {
                return Err(format!(
                    "cross reference identifier '{}' is duplicated",
                    cross_reference.identifier
                ));
            }
            let is_external =
                cross_reference.container.is_some() || cross_reference.document.is_some();
            if !is_external
                && !context
                    .sequence_identifiers
                    .contains(cross_reference.sequence_identifier.as_str())
            {
                return Err(format!(
                    "cross reference '{}' points to missing local sequence '{}'",
                    cross_reference.id, cross_reference.sequence_identifier
                ));
            }
            validate_unique_existing(
                "cross-reference text",
                &cross_reference.text_object_ids,
                &|id| context.scene_ids.contains(id),
            )?;
            validate_display_ownership(
                "cross reference",
                &cross_reference.id,
                &cross_reference.text_object_ids,
                &mut context.display_object_owners,
            )?;
        }
        Ok(())
    }

    fn validate_metadata(&self, context: &mut LogicalValidationContext<'_>) -> Result<(), String> {
        for tag in &self.object_tags {
            context.register("object tag", &tag.id)?;
            validate_owner(
                &tag.id,
                tag.owner_entity_id.as_deref(),
                tag.unresolved_owner_source_id.as_deref(),
                &|id| context.entity_exists(id),
            )?;
            validate_unique_existing("object-tag display", &tag.display_object_ids, &|id| {
                context.scene_ids.contains(id)
            })?;
            validate_display_ownership(
                "object tag",
                &tag.id,
                &tag.display_object_ids,
                &mut context.display_object_owners,
            )?;
            if tag.name.trim().is_empty() {
                return Err(format!("object tag '{}' has an empty name", tag.id));
            }
            if tag
                .positioning_angle
                .is_some_and(|value| !value.is_finite())
                || tag
                    .positioning_offset
                    .is_some_and(|value| !value.into_iter().all(f64::is_finite))
            {
                return Err(format!(
                    "object tag '{}' has invalid positioning values",
                    tag.id
                ));
            }
            if tag.tag_type == ObjectTagType::Long
                && tag
                    .value
                    .as_deref()
                    .is_some_and(|value| value.parse::<i32>().is_err())
            {
                return Err(format!(
                    "object tag '{}' has a non-integer Long value",
                    tag.id
                ));
            }
            if tag.tag_type == ObjectTagType::Double
                && tag.value.as_deref().is_some_and(|value| {
                    value
                        .parse::<f64>()
                        .map_or(true, |number| !number.is_finite())
                })
            {
                return Err(format!(
                    "object tag '{}' has a non-numeric Double value",
                    tag.id
                ));
            }
        }
        for annotation in &self.annotations {
            context.register("annotation", &annotation.id)?;
            validate_owner(
                &annotation.id,
                annotation.owner_entity_id.as_deref(),
                annotation.unresolved_owner_source_id.as_deref(),
                &|id| context.entity_exists(id),
            )?;
        }
        for registration in &self.registry_numbers {
            context.register("registry number", &registration.id)?;
            validate_owner(
                &registration.id,
                registration.owner_entity_id.as_deref(),
                registration.unresolved_owner_source_id.as_deref(),
                &|id| context.entity_exists(id),
            )?;
            if registration.owner_entity_id.is_none()
                && registration.unresolved_owner_source_id.is_none()
            {
                return Err(format!(
                    "registry number '{}' is missing its owner",
                    registration.id
                ));
            }
            if registration.authority.trim().is_empty() || registration.number.trim().is_empty() {
                return Err(format!(
                    "registry number '{}' is missing authority or number",
                    registration.id
                ));
            }
        }
        for representation in &self.representations {
            context.register("representation", &representation.id)?;
            if (representation.owner_entity_id.is_some()
                && representation.unresolved_owner_source_id.is_some())
                || (representation.target_entity_id.is_some()
                    && representation.unresolved_target_source_id.is_some())
                || representation
                    .owner_entity_id
                    .as_ref()
                    .is_some_and(|id| !context.entity_exists(id))
                || (representation.owner_entity_id.is_none()
                    && !has_unresolved(&representation.unresolved_owner_source_id))
                || representation
                    .target_entity_id
                    .as_ref()
                    .is_some_and(|id| !context.entity_exists(id))
                || (representation.target_entity_id.is_none()
                    && !has_unresolved(&representation.unresolved_target_source_id))
                || representation.attribute.trim().is_empty()
            {
                return Err(format!(
                    "representation '{}' has an invalid owner, target, or attribute",
                    representation.id
                ));
            }
        }
        Ok(())
    }
}

struct LogicalValidationContext<'a> {
    scene_ids: &'a BTreeSet<String>,
    node_ids: &'a BTreeSet<String>,
    bond_ids: &'a BTreeSet<String>,
    registered_ids: BTreeSet<String>,
    sequence_identifiers: BTreeSet<String>,
    display_object_owners: BTreeSet<String>,
}

impl<'a> LogicalValidationContext<'a> {
    fn new(
        scene_ids: &'a BTreeSet<String>,
        node_ids: &'a BTreeSet<String>,
        bond_ids: &'a BTreeSet<String>,
    ) -> Self {
        Self {
            scene_ids,
            node_ids,
            bond_ids,
            registered_ids: scene_ids
                .iter()
                .chain(node_ids.iter())
                .chain(bond_ids.iter())
                .cloned()
                .collect(),
            sequence_identifiers: BTreeSet::new(),
            display_object_owners: BTreeSet::new(),
        }
    }

    fn entity_exists(&self, id: &str) -> bool {
        self.scene_ids.contains(id) || self.node_ids.contains(id) || self.bond_ids.contains(id)
    }

    fn register(&mut self, kind: &str, id: &str) -> Result<(), String> {
        if id.trim().is_empty() || !self.registered_ids.insert(id.to_string()) {
            Err(format!("{kind} id '{id}' is empty or duplicated"))
        } else {
            Ok(())
        }
    }
}

fn validate_owner(
    id: &str,
    owner: Option<&str>,
    unresolved_owner: Option<&str>,
    exists: &impl Fn(&str) -> bool,
) -> Result<(), String> {
    if owner.is_some_and(|owner| !exists(owner))
        || (owner.is_some() && unresolved_owner.is_some())
        || unresolved_owner.is_some_and(|value| value.trim().is_empty())
    {
        Err(format!("logical object '{id}' references a missing owner"))
    } else {
        Ok(())
    }
}

fn has_unresolved(value: &Option<String>) -> bool {
    value
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
}

fn validate_unique_existing(
    kind: &str,
    ids: &[String],
    exists: &impl Fn(&str) -> bool,
) -> Result<(), String> {
    let mut unique = BTreeSet::new();
    for id in ids {
        if !unique.insert(id.as_str()) {
            return Err(format!("{kind} repeats entity '{id}'"));
        }
        if !exists(id) {
            return Err(format!("{kind} references missing entity '{id}'"));
        }
    }
    Ok(())
}

fn validate_unique_nonempty(kind: &str, ids: &[String]) -> Result<(), String> {
    let mut unique = BTreeSet::new();
    for id in ids {
        if id.trim().is_empty() || !unique.insert(id.as_str()) {
            return Err(format!("{kind} id '{id}' is empty or duplicated"));
        }
    }
    Ok(())
}

fn validate_display_ownership(
    kind: &str,
    owner_id: &str,
    display_ids: &[String],
    owners: &mut BTreeSet<String>,
) -> Result<(), String> {
    for display_id in display_ids {
        if !owners.insert(display_id.clone()) {
            return Err(format!(
                "display text '{display_id}' belongs to multiple logical objects, including {kind} '{owner_id}'"
            ));
        }
    }
    Ok(())
}

fn validate_optional_box(name: &str, value: Option<[f64; 4]>) -> Result<(), String> {
    if value.is_some_and(|value| !value.into_iter().all(f64::is_finite)) {
        Err(format!("{name} contains a non-finite value"))
    } else {
        Ok(())
    }
}
