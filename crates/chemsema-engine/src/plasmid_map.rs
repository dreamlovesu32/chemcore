use serde::{Deserialize, Serialize};

/// Native, editable plasmid-map data. Base-pair coordinates use ChemDraw's
/// inclusive 1..=number_base_pairs domain; geometry is local to the owning
/// scene object and angles are measured clockwise from twelve o'clock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlasmidMapData {
    pub number_base_pairs: u64,
    pub radius: f64,
    #[serde(default = "default_true")]
    pub show_base_pairs: bool,
    #[serde(default = "default_line_width")]
    pub line_width: f64,
    #[serde(default = "default_bold_width")]
    pub bold_width: f64,
    #[serde(default = "default_margin_width")]
    pub margin_width: f64,
    #[serde(default = "default_label_font")]
    pub label_font: i32,
    #[serde(default = "default_label_size")]
    pub label_size: f64,
    #[serde(default)]
    pub label_face: i32,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub regions: Vec<PlasmidRegion>,
    #[serde(default)]
    pub markers: Vec<PlasmidMarker>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlasmidRegion {
    pub id: String,
    pub start: u64,
    pub end: u64,
    #[serde(default)]
    pub offset: f64,
    #[serde(default)]
    pub arrow_at_start: bool,
    #[serde(default)]
    pub arrow_at_end: bool,
    #[serde(default)]
    pub filled: bool,
    #[serde(default)]
    pub shaded: bool,
    #[serde(default)]
    pub faded: bool,
    #[serde(default = "default_region_width")]
    pub width: f64,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default = "default_alpha")]
    pub alpha: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlasmidMarker {
    pub id: String,
    pub position: u64,
    pub label: String,
    #[serde(default = "default_marker_offset")]
    pub offset: f64,
    /// Explicit label angle in degrees. This is independent of `position` so
    /// dragging a label does not change its semantic base-pair coordinate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_angle: Option<f64>,
    #[serde(default = "default_color")]
    pub color: String,
}

impl Default for PlasmidMapData {
    fn default() -> Self {
        Self {
            number_base_pairs: 10_000,
            radius: 34.0,
            show_base_pairs: true,
            line_width: default_line_width(),
            bold_width: default_bold_width(),
            margin_width: default_margin_width(),
            label_font: default_label_font(),
            label_size: default_label_size(),
            label_face: 0,
            color: default_color(),
            regions: Vec::new(),
            markers: Vec::new(),
        }
    }
}

impl PlasmidMapData {
    pub fn validate(&self) -> Result<(), String> {
        if self.number_base_pairs == 0 {
            return Err("plasmid map base-pair count must be greater than zero".to_string());
        }
        if ![
            self.radius,
            self.line_width,
            self.bold_width,
            self.margin_width,
            self.label_size,
        ]
        .into_iter()
        .all(f64::is_finite)
            || self.radius <= 0.0
            || self.line_width < 0.0
            || self.bold_width < 0.0
            || self.margin_width < 0.0
            || self.label_size <= 0.0
        {
            return Err("plasmid map contains invalid numeric style values".to_string());
        }
        let mut ids = std::collections::BTreeSet::new();
        for region in &self.regions {
            if region.id.is_empty() || !ids.insert(region.id.as_str()) {
                return Err("plasmid region ids must be non-empty and unique".to_string());
            }
            if !self.contains_position(region.start) || !self.contains_position(region.end) {
                return Err(format!(
                    "plasmid region '{}' lies outside the base-pair domain",
                    region.id
                ));
            }
            if ![region.offset, region.width, region.alpha]
                .into_iter()
                .all(f64::is_finite)
                || region.width <= 0.0
                || !(0.0..=1.0).contains(&region.alpha)
            {
                return Err(format!(
                    "plasmid region '{}' has invalid geometry",
                    region.id
                ));
            }
        }
        for marker in &self.markers {
            if marker.id.is_empty() || !ids.insert(marker.id.as_str()) {
                return Err("plasmid marker ids must be non-empty and unique".to_string());
            }
            if !self.contains_position(marker.position) {
                return Err(format!(
                    "plasmid marker '{}' lies outside the base-pair domain",
                    marker.id
                ));
            }
            if !marker.offset.is_finite()
                || marker.label_angle.is_some_and(|angle| !angle.is_finite())
            {
                return Err(format!(
                    "plasmid marker '{}' has invalid geometry",
                    marker.id
                ));
            }
        }
        Ok(())
    }

    pub fn angle_degrees(&self, position: u64) -> f64 {
        (position.saturating_sub(1) as f64 / self.number_base_pairs as f64) * 360.0
    }

    fn contains_position(&self, position: u64) -> bool {
        (1..=self.number_base_pairs).contains(&position)
    }
}

const fn default_true() -> bool {
    true
}
const fn default_line_width() -> f64 {
    0.75
}
const fn default_bold_width() -> f64 {
    2.6
}
const fn default_margin_width() -> f64 {
    2.0
}
const fn default_label_font() -> i32 {
    3
}
const fn default_label_size() -> f64 {
    12.0
}
const fn default_region_width() -> f64 {
    6.0
}
const fn default_marker_offset() -> f64 {
    48.0
}
const fn default_alpha() -> f64 {
    1.0
}
fn default_color() -> String {
    "#000000".to_string()
}
