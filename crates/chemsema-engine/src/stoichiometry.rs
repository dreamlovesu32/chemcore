use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReactionSchemeData {
    pub id: String,
    #[serde(default)]
    pub steps: Vec<ReactionStepData>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReactionStepData {
    pub id: String,
    #[serde(default)]
    pub link_policy: crate::LinkPolicy,
    #[serde(default)]
    pub binding_origin: crate::LogicalBindingOrigin,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactant_entity_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub product_entity_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arrow_object_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plus_object_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects_above_arrow: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects_below_arrow: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub atom_mappings: Vec<ReactionAtomMapping>,
    #[serde(default)]
    pub interpretation_state: ReactionInterpretationState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactionAtomMapping {
    pub reactant_atom_id: String,
    pub product_atom_id: String,
    #[serde(default)]
    pub origin: ReactionAtomMappingOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ReactionAtomMappingOrigin {
    Manual,
    Automatic,
    #[default]
    Imported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ReactionInterpretationState {
    #[default]
    Current,
    Stale,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoichiometryGridData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_reaction_step_id: Option<String>,
    #[serde(default)]
    pub binding_origin: StoichiometryBindingOrigin,
    #[serde(default)]
    pub binding_state: StoichiometryBindingState,
    #[serde(default)]
    pub anchor_mode: StoichiometryAnchorMode,
    #[serde(default)]
    pub components: Vec<StoichiometryComponent>,
    #[serde(default)]
    pub rows: Vec<StoichiometryRow>,
    #[serde(default)]
    pub data: Vec<StoichiometryDatum>,
    #[serde(default)]
    pub style: StoichiometryGridStyle,
}

impl Default for StoichiometryGridData {
    fn default() -> Self {
        Self {
            source_reaction_step_id: None,
            binding_origin: Default::default(),
            binding_state: StoichiometryBindingState::Detached,
            anchor_mode: Default::default(),
            components: Vec::new(),
            rows: Vec::new(),
            data: Vec::new(),
            style: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum StoichiometryBindingOrigin {
    Authored,
    Imported,
    Inferred,
    #[default]
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum StoichiometryBindingState {
    Current,
    Stale,
    Unresolved,
    Orphaned,
    #[default]
    Detached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum StoichiometryAnchorMode {
    #[default]
    Follow,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum StoichiometryComponentRole {
    Header,
    #[default]
    Reactant,
    Product,
    Reagent,
    Condition,
    Unspecified,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoichiometryComponent {
    pub id: String,
    #[serde(default)]
    pub role: StoichiometryComponentRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_entity_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved_reference_id: Option<String>,
    #[serde(default)]
    pub is_header: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_component_width")]
    pub width: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoichiometryRow {
    pub id: String,
    pub property_type: String,
    pub data_type: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_unit: Option<String>,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_row_height")]
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoichiometryDatum {
    pub id: String,
    pub component_id: String,
    pub row_id: String,
    #[serde(default)]
    pub value: StoichiometryValue,
    #[serde(default)]
    pub origin: StoichiometryValueOrigin,
    #[serde(default)]
    pub is_edited: bool,
    #[serde(default)]
    pub is_hidden: bool,
    #[serde(default)]
    pub is_read_only: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub calculation_state: StoichiometryCalculationState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StoichiometryValue {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub canonical: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub display: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum StoichiometryValueOrigin {
    Authored,
    Calculated,
    Imported,
    #[default]
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum StoichiometryCalculationState {
    Current,
    Stale,
    Incomplete,
    Inconsistent,
    Unsupported,
    #[default]
    Empty,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoichiometryGridStyle {
    pub line_width: f64,
    pub bold_width: f64,
    pub margin_width: f64,
    pub color: String,
    pub label_font: String,
    pub label_size: f64,
    pub label_face: i32,
}

impl Default for StoichiometryGridStyle {
    fn default() -> Self {
        Self {
            line_width: 0.75,
            bold_width: 1.5,
            margin_width: 2.0,
            color: "#000000".to_string(),
            label_font: "Arial".to_string(),
            label_size: 10.0,
            label_face: 0,
        }
    }
}

const fn default_true() -> bool {
    true
}

const fn default_component_width() -> f64 {
    72.0
}

const fn default_row_height() -> f64 {
    18.0
}

impl StoichiometryGridData {
    pub fn validate(&self) -> Result<(), String> {
        let mut ids = std::collections::BTreeSet::new();
        for component in &self.components {
            if component.id.is_empty() || !ids.insert(component.id.as_str()) {
                return Err("stoichiometry component ids must be non-empty and unique".to_string());
            }
            if !component.width.is_finite() || component.width <= 0.0 {
                return Err(format!("component '{}' has an invalid width", component.id));
            }
        }
        let component_ids = ids;
        let mut row_ids = std::collections::BTreeSet::new();
        let mut numeric_row_ids = std::collections::BTreeSet::new();
        for row in &self.rows {
            if row.id.is_empty() || !row_ids.insert(row.id.as_str()) {
                return Err("stoichiometry row ids must be non-empty and unique".to_string());
            }
            if !row.height.is_finite() || row.height <= 0.0 {
                return Err(format!("row '{}' has an invalid height", row.id));
            }
            if row.data_type.eq_ignore_ascii_case("number") {
                numeric_row_ids.insert(row.id.as_str());
            }
        }
        let mut datum_ids = std::collections::BTreeSet::new();
        let mut cells = std::collections::BTreeSet::new();
        for datum in &self.data {
            if datum.id.is_empty() || !datum_ids.insert(datum.id.as_str()) {
                return Err("stoichiometry datum ids must be non-empty and unique".to_string());
            }
            if !component_ids.contains(datum.component_id.as_str())
                || !row_ids.contains(datum.row_id.as_str())
            {
                return Err(format!(
                    "datum '{}' references a missing row or component",
                    datum.id
                ));
            }
            if !cells.insert((datum.component_id.as_str(), datum.row_id.as_str())) {
                return Err(format!(
                    "component '{}' has duplicate datum for row '{}'",
                    datum.component_id, datum.row_id
                ));
            }
            if numeric_row_ids.contains(datum.row_id.as_str())
                && !datum.value.canonical.is_empty()
                && datum.value.canonical.parse::<f64>().is_err()
            {
                return Err(format!(
                    "datum '{}' has a non-numeric canonical value",
                    datum.id
                ));
            }
        }
        Ok(())
    }
}
