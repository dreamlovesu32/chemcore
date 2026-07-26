use serde::{Deserialize, Serialize};

use super::default_true;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeometryFeature {
    PointFromPointPointDistance,
    PointFromPointPointPercentage,
    PointFromPointNormalDistance,
    LineFromPoints,
    PlaneFromPoints,
    PlaneFromPointLine,
    CentroidFromPoints,
    NormalFromPointPlane,
}

impl GeometryFeature {
    pub const fn as_cdxml(self) -> &'static str {
        match self {
            Self::PointFromPointPointDistance => "PointFromPointPointDistance",
            Self::PointFromPointPointPercentage => "PointFromPointPointPercentage",
            Self::PointFromPointNormalDistance => "PointFromPointNormalDistance",
            Self::LineFromPoints => "LineFromPoints",
            Self::PlaneFromPoints => "PlaneFromPoints",
            Self::PlaneFromPointLine => "PlaneFromPointLine",
            Self::CentroidFromPoints => "CentroidFromPoints",
            Self::NormalFromPointPlane => "NormalFromPointPlane",
        }
    }

    pub fn from_cdxml(value: &str) -> Option<Self> {
        Some(match value {
            "PointFromPointPointDistance" | "1" => Self::PointFromPointPointDistance,
            "PointFromPointPointPercentage" | "2" => Self::PointFromPointPointPercentage,
            "PointFromPointNormalDistance" | "3" => Self::PointFromPointNormalDistance,
            "LineFromPoints" | "4" => Self::LineFromPoints,
            "PlaneFromPoints" | "5" => Self::PlaneFromPoints,
            "PlaneFromPointLine" | "6" => Self::PlaneFromPointLine,
            "CentroidFromPoints" | "7" => Self::CentroidFromPoints,
            "NormalFromPointPlane" | "8" => Self::NormalFromPointPlane,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeometryData {
    pub feature: GeometryFeature,
    #[serde(default)]
    pub basis_entity_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved_basis_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation_value: Option<f64>,
    #[serde(default)]
    pub point_is_directed: bool,
}

impl GeometryData {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.basis_entity_ids.is_empty() && self.unresolved_basis_ids.is_empty() {
            return Err("geometry requires at least one basis entity".to_string());
        }
        if self.relation_value.is_some_and(|value| !value.is_finite()) {
            return Err("geometry relation value must be finite".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConstraintType {
    Distance,
    Angle,
    ExclusionSphere,
}

impl ConstraintType {
    pub const fn as_cdxml(self) -> &'static str {
        match self {
            Self::Distance => "Distance",
            Self::Angle => "Angle",
            Self::ExclusionSphere => "ExclusionSphere",
        }
    }

    pub fn from_cdxml(value: &str) -> Option<Self> {
        Some(match value {
            "Distance" | "1" => Self::Distance,
            "Angle" | "2" => Self::Angle,
            "ExclusionSphere" | "3" => Self::ExclusionSphere,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintData {
    pub constraint_type: ConstraintType,
    #[serde(default)]
    pub basis_entity_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved_basis_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(default)]
    pub ignore_unconnected_atoms: bool,
    #[serde(default)]
    pub dihedral_is_chiral: bool,
    #[serde(default)]
    pub point_is_directed: bool,
    #[serde(default, skip_serializing_if = "AnnotationDisplay::is_default")]
    pub display: AnnotationDisplay,
}

impl ConstraintData {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.basis_entity_ids.is_empty() && self.unresolved_basis_ids.is_empty() {
            return Err("constraint requires at least one basis entity".to_string());
        }
        for value in [self.minimum, self.maximum].into_iter().flatten() {
            if !value.is_finite() {
                return Err("constraint bounds must be finite".to_string());
            }
        }
        if self
            .minimum
            .zip(self.maximum)
            .is_some_and(|(minimum, maximum)| minimum > maximum)
        {
            return Err("constraint minimum cannot exceed maximum".to_string());
        }
        self.display.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationDisplay {
    #[serde(default = "default_true")]
    pub auto_value: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_override: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[f64; 2]>,
    #[serde(default)]
    pub positioning_type: AnnotationPositioningType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub positioning_angle: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub positioning_offset: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(default)]
    pub font_weight: u32,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    #[serde(default = "default_true")]
    pub indicator_visible: bool,
}

impl Default for AnnotationDisplay {
    fn default() -> Self {
        Self {
            auto_value: true,
            text_override: None,
            position: None,
            positioning_type: AnnotationPositioningType::Auto,
            positioning_angle: None,
            positioning_offset: None,
            font_family: None,
            font_size: None,
            fill: None,
            font_weight: 400,
            italic: false,
            underline: false,
            indicator_visible: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationPositioningType {
    #[default]
    Auto,
    Angle,
    Offset,
    Absolute,
}

impl AnnotationPositioningType {
    pub const fn as_cdxml(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Angle => "angle",
            Self::Offset => "offset",
            Self::Absolute => "absolute",
        }
    }

    pub fn from_cdxml(value: &str) -> Option<Self> {
        Some(match value.to_ascii_lowercase().as_str() {
            "auto" => Self::Auto,
            "angle" => Self::Angle,
            "offset" => Self::Offset,
            "absolute" => Self::Absolute,
            _ => return None,
        })
    }
}

impl AnnotationDisplay {
    pub(super) fn is_default(&self) -> bool {
        self == &Self::default()
    }

    fn validate(&self) -> Result<(), String> {
        if self
            .font_size
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            return Err("annotation font size must be finite and positive".to_string());
        }
        if self
            .position
            .into_iter()
            .chain(self.positioning_offset)
            .flatten()
            .any(|value| !value.is_finite())
            || self
                .positioning_angle
                .is_some_and(|value| !value.is_finite())
        {
            return Err("annotation positioning values must be finite".to_string());
        }
        if self.positioning_type == AnnotationPositioningType::Absolute && self.position.is_none() {
            return Err("absolute annotation positioning requires a position".to_string());
        }
        if self.positioning_type == AnnotationPositioningType::Offset
            && self.positioning_offset.is_none()
        {
            return Err("offset annotation positioning requires an offset".to_string());
        }
        if self.positioning_type == AnnotationPositioningType::Angle
            && self.positioning_angle.is_none()
        {
            return Err("angle annotation positioning requires an angle".to_string());
        }
        Ok(())
    }
}
