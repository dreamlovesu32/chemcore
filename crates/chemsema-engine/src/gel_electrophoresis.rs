use serde::{Deserialize, Serialize};

/// Native, editable gel-electrophoresis plate data. Geometry is local to the
/// owning scene object's `bbox`; `corners` preserves ChemDraw quadrilaterals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GelElectrophoresisData {
    #[serde(default)]
    pub lanes: Vec<GelLane>,
    #[serde(default)]
    pub start_range: f64,
    #[serde(default = "default_end_range")]
    pub end_range: f64,
    #[serde(default)]
    pub unit_id: i32,
    #[serde(default)]
    pub show_scale: bool,
    #[serde(default = "default_true")]
    pub show_borders: bool,
    #[serde(default)]
    pub transparent: bool,
    #[serde(default = "default_line_width")]
    pub line_width: f64,
    #[serde(default = "default_bold_width")]
    pub bold_width: f64,
    #[serde(default = "default_axis_width")]
    pub axis_width: f64,
    #[serde(default = "default_margin_width")]
    pub margin_width: f64,
    #[serde(default = "default_hash_spacing")]
    pub hash_spacing: f64,
    #[serde(default = "default_label_font")]
    pub label_font: i32,
    #[serde(default = "default_label_size")]
    pub label_size: f64,
    #[serde(default)]
    pub label_face: i32,
    #[serde(default)]
    pub labels_angle: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label_text: String,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default = "default_alpha")]
    pub alpha: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corners: Option<[[f64; 2]; 4]>,
}

impl Default for GelElectrophoresisData {
    fn default() -> Self {
        Self {
            lanes: Vec::new(),
            start_range: 0.0,
            end_range: default_end_range(),
            unit_id: 0,
            show_scale: false,
            show_borders: true,
            transparent: false,
            line_width: default_line_width(),
            bold_width: default_bold_width(),
            axis_width: default_axis_width(),
            margin_width: default_margin_width(),
            hash_spacing: default_hash_spacing(),
            label_font: default_label_font(),
            label_size: default_label_size(),
            label_face: 0,
            labels_angle: 0.0,
            label_text: String::new(),
            color: default_color(),
            alpha: default_alpha(),
            corners: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GelLane {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label_text: String,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub bands: Vec<GelBand>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GelBand {
    pub id: String,
    pub value: f64,
    #[serde(default = "default_band_width")]
    pub width: f64,
    #[serde(default = "default_band_height")]
    pub height: f64,
    #[serde(default = "default_curve_type")]
    pub curve_type: i32,
    #[serde(default)]
    pub show_value: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default = "default_alpha")]
    pub alpha: f64,
    #[serde(default)]
    pub z_index: i32,
}

impl GelElectrophoresisData {
    pub fn validate(&self) -> Result<(), String> {
        if ![
            self.start_range,
            self.end_range,
            self.line_width,
            self.bold_width,
            self.axis_width,
            self.margin_width,
            self.hash_spacing,
            self.label_size,
            self.labels_angle,
            self.alpha,
        ]
        .into_iter()
        .all(f64::is_finite)
            || self.line_width < 0.0
            || self.label_size <= 0.0
        {
            return Err("gel plate contains invalid numeric style values".to_string());
        }
        if (self.end_range - self.start_range).abs() <= crate::EPSILON {
            return Err("gel plate range must have non-zero extent".to_string());
        }
        let mut ids = std::collections::BTreeSet::new();
        for lane in &self.lanes {
            if lane.id.is_empty() || !ids.insert(lane.id.as_str()) {
                return Err("gel lane ids must be non-empty and unique".to_string());
            }
            for band in &lane.bands {
                if band.id.is_empty() || !ids.insert(band.id.as_str()) {
                    return Err("gel band ids must be non-empty and unique".to_string());
                }
                if ![band.value, band.width, band.height, band.alpha]
                    .into_iter()
                    .all(f64::is_finite)
                    || band.width <= 0.0
                    || band.height <= 0.0
                {
                    return Err(format!("gel band '{}' has invalid geometry", band.id));
                }
            }
        }
        Ok(())
    }
}

const fn default_true() -> bool {
    true
}
const fn default_end_range() -> f64 {
    1.0
}
const fn default_line_width() -> f64 {
    0.75
}
const fn default_bold_width() -> f64 {
    1.5
}
const fn default_axis_width() -> f64 {
    0.75
}
const fn default_margin_width() -> f64 {
    2.0
}
const fn default_hash_spacing() -> f64 {
    2.7
}
const fn default_label_font() -> i32 {
    3
}
const fn default_label_size() -> f64 {
    10.0
}
fn default_color() -> String {
    "#000000".to_string()
}
const fn default_alpha() -> f64 {
    1.0
}
const fn default_band_width() -> f64 {
    18.0
}
const fn default_band_height() -> f64 {
    3.0
}
const fn default_curve_type() -> i32 {
    128
}
