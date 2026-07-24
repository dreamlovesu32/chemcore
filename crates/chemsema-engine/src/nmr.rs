use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NmrNucleus {
    #[serde(rename = "unknown")]
    #[default]
    Unknown,
    #[serde(rename = "1H")]
    Hydrogen1,
    #[serde(rename = "13C")]
    Carbon13,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NmrAssignmentQuality {
    #[default]
    Unknown,
    Good,
    Medium,
    Rough,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NmrAssignment {
    #[serde(default)]
    pub nucleus: NmrNucleus,
    pub shift_ppm: f64,
    pub range_low_ppm: f64,
    pub range_high_ppm: f64,
    #[serde(default)]
    pub quality: NmrAssignmentQuality,
    pub label: crate::NodeLabel,
}

impl NmrAssignment {
    pub fn validate(&self) -> Result<(), String> {
        if !self.shift_ppm.is_finite()
            || !self.range_low_ppm.is_finite()
            || !self.range_high_ppm.is_finite()
        {
            return Err("NMR assignment values must be finite".to_string());
        }
        if self.range_low_ppm > self.range_high_ppm {
            return Err("NMR assignment rangeLowPpm must not exceed rangeHighPpm".to_string());
        }
        if !self.label.has_visible_text() {
            return Err("NMR assignment label must not be empty".to_string());
        }
        Ok(())
    }
}
