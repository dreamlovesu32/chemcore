use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BioDrawKind {
    OneSubstrateEnzyme,
    TwoSubstrateEnzyme,
    Receptor,
    GProteinAlpha,
    GProteinBeta,
    GProteinGamma,
    Immunoglobulin,
    IonChannel,
    EndoplasmicReticulum,
    Golgi,
    MembraneLine,
    MembraneArc,
    MembraneEllipse,
    MembraneMicelle,
    Dna,
    HelixProtein,
    Mitochondrion,
    Cloud,
    TRna,
    RibosomeA,
    RibosomeB,
    PlasmidMap,
}

impl Default for BioDrawKind {
    fn default() -> Self {
        Self::PlasmidMap
    }
}

impl BioDrawKind {
    pub const BIO_SHAPES: [Self; 21] = [
        Self::OneSubstrateEnzyme,
        Self::TwoSubstrateEnzyme,
        Self::Receptor,
        Self::GProteinAlpha,
        Self::GProteinBeta,
        Self::GProteinGamma,
        Self::Immunoglobulin,
        Self::IonChannel,
        Self::EndoplasmicReticulum,
        Self::Golgi,
        Self::MembraneLine,
        Self::MembraneArc,
        Self::MembraneEllipse,
        Self::MembraneMicelle,
        Self::Dna,
        Self::HelixProtein,
        Self::Mitochondrion,
        Self::Cloud,
        Self::TRna,
        Self::RibosomeA,
        Self::RibosomeB,
    ];

    pub const fn bio_shape_kind(self) -> Option<BioShapeKind> {
        Some(match self {
            Self::OneSubstrateEnzyme => BioShapeKind::OneSubstrateEnzyme,
            Self::TwoSubstrateEnzyme => BioShapeKind::TwoSubstrateEnzyme,
            Self::Receptor => BioShapeKind::Receptor,
            Self::GProteinAlpha => BioShapeKind::GProteinAlpha,
            Self::GProteinBeta => BioShapeKind::GProteinBeta,
            Self::GProteinGamma => BioShapeKind::GProteinGamma,
            Self::Immunoglobulin => BioShapeKind::Immunoglobulin,
            Self::IonChannel => BioShapeKind::IonChannel,
            Self::EndoplasmicReticulum => BioShapeKind::EndoplasmicReticulum,
            Self::Golgi => BioShapeKind::Golgi,
            Self::MembraneLine => BioShapeKind::MembraneLine,
            Self::MembraneArc => BioShapeKind::MembraneArc,
            Self::MembraneEllipse => BioShapeKind::MembraneEllipse,
            Self::MembraneMicelle => BioShapeKind::MembraneMicelle,
            Self::Dna => BioShapeKind::Dna,
            Self::HelixProtein => BioShapeKind::HelixProtein,
            Self::Mitochondrion => BioShapeKind::Mitochondrion,
            Self::Cloud => BioShapeKind::Cloud,
            Self::TRna => BioShapeKind::TRna,
            Self::RibosomeA => BioShapeKind::RibosomeA,
            Self::RibosomeB => BioShapeKind::RibosomeB,
            Self::PlasmidMap => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BioShapeKind {
    OneSubstrateEnzyme,
    TwoSubstrateEnzyme,
    Receptor,
    GProteinAlpha,
    GProteinBeta,
    GProteinGamma,
    Immunoglobulin,
    IonChannel,
    EndoplasmicReticulum,
    Golgi,
    MembraneLine,
    MembraneArc,
    MembraneEllipse,
    MembraneMicelle,
    Dna,
    HelixProtein,
    Mitochondrion,
    Cloud,
    TRna,
    RibosomeA,
    RibosomeB,
}

impl BioShapeKind {
    pub const fn cdxml_name(self) -> &'static str {
        match self {
            Self::OneSubstrateEnzyme => "1SubstrateEnzyme",
            Self::TwoSubstrateEnzyme => "2SubstrateEnzyme",
            Self::Receptor => "Receptor",
            Self::GProteinAlpha => "GProteinAlpha",
            Self::GProteinBeta => "GProteinBeta",
            Self::GProteinGamma => "GProteinGamma",
            Self::Immunoglobulin => "Immunoglobin",
            Self::IonChannel => "IonChannel",
            Self::EndoplasmicReticulum => "EndoplasmicReticulum",
            Self::Golgi => "Golgi",
            Self::MembraneLine => "MembraneLine",
            Self::MembraneArc => "MembraneArc",
            Self::MembraneEllipse => "MembraneEllipse",
            Self::MembraneMicelle => "MembraneMicelle",
            Self::Dna => "DNA",
            Self::HelixProtein => "HelixProtein",
            Self::Mitochondrion => "Mitochondrion",
            Self::Cloud => "Cloud",
            Self::TRna => "tRNA",
            Self::RibosomeA => "RibosomeA",
            Self::RibosomeB => "RibosomeB",
        }
    }

    pub fn from_cdxml_name(value: &str) -> Option<Self> {
        Some(match value {
            "1SubstrateEnzyme" => Self::OneSubstrateEnzyme,
            "2SubstrateEnzyme" => Self::TwoSubstrateEnzyme,
            "Receptor" => Self::Receptor,
            "GProteinAlpha" => Self::GProteinAlpha,
            "GProteinBeta" => Self::GProteinBeta,
            "GProteinGamma" => Self::GProteinGamma,
            "Immunoglobin" => Self::Immunoglobulin,
            "IonChannel" => Self::IonChannel,
            "EndoplasmicReticulum" => Self::EndoplasmicReticulum,
            "Golgi" => Self::Golgi,
            "MembraneLine" => Self::MembraneLine,
            "MembraneArc" => Self::MembraneArc,
            "MembraneEllipse" => Self::MembraneEllipse,
            "MembraneMicelle" => Self::MembraneMicelle,
            "DNA" => Self::Dna,
            "HelixProtein" => Self::HelixProtein,
            "Mitochondrion" => Self::Mitochondrion,
            "Cloud" => Self::Cloud,
            "tRNA" => Self::TRna,
            "RibosomeA" => Self::RibosomeA,
            "RibosomeB" => Self::RibosomeB,
            _ => return None,
        })
    }

    pub const fn parameter_fields(self) -> &'static [&'static str] {
        match self {
            Self::OneSubstrateEnzyme => &["enzymeReceptorSize"],
            Self::Receptor => &["neckWidth"],
            Self::GProteinGamma => &["gproteinUpperHeight"],
            Self::MembraneLine | Self::MembraneEllipse | Self::MembraneMicelle => {
                &["membraneElementSize"]
            }
            Self::MembraneArc => &[
                "membraneElementSize",
                "membraneStartAngle",
                "membraneEndAngle",
            ],
            Self::Dna => &[
                "dnaWaveHeight",
                "dnaWaveLength",
                "dnaWaveOffset",
                "dnaWaveWidth",
            ],
            Self::HelixProtein => &[
                "cylinderDistance",
                "cylinderHeight",
                "cylinderWidth",
                "pipeWidth",
                "helixProteinExtra",
            ],
            _ => &[],
        }
    }

    pub fn parameter_field_specs(self) -> Vec<BioShapeParameterField> {
        self.parameter_fields()
            .iter()
            .map(|key| BioShapeParameterField::for_key(key))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BioShapeParameterField {
    pub key: &'static str,
    pub label: &'static str,
    pub minimum: f64,
    pub maximum: f64,
    pub step: f64,
    pub unit: &'static str,
}

impl BioShapeParameterField {
    fn for_key(key: &'static str) -> Self {
        let label = match key {
            "cylinderDistance" => "Cylinder spacing",
            "cylinderHeight" => "Cylinder height",
            "cylinderWidth" => "Cylinder width",
            "dnaWaveHeight" => "Wave height",
            "dnaWaveLength" => "Wave spacing",
            "dnaWaveOffset" => "Second-strand offset",
            "dnaWaveWidth" => "Strand width",
            "enzymeHeight" => "Enzyme height",
            "enzymeReceptorSize" => "Receptor size",
            "enzymeWidth" => "Enzyme width",
            "golgiHeight" => "Golgi height",
            "golgiLength" => "Golgi length",
            "golgiWidth" => "Golgi width",
            "gproteinLowerHeight" => "Lower height",
            "gproteinUpperHeight" => "Upper height",
            "helixProteinExtra" => "Helix extra",
            "immunoglobulinHeight" => "Immunoglobulin height",
            "immunoglobulinWidth" => "Immunoglobulin width",
            "membraneElementSize" => "Membrane unit size",
            "membraneEndAngle" => "Arc end angle",
            "membraneMajorAxisSize" => "Arc major-axis size",
            "membraneMinorAxisSize" => "Arc minor-axis size",
            "membraneStartAngle" => "Arc start angle",
            "neckHeight" => "Neck height",
            "neckWidth" => "Neck width",
            "pipeWidth" => "Strand width",
            _ => unreachable!("parameter_fields only contains declared BioShape keys"),
        };
        let (minimum, maximum, step, unit) = match key {
            "membraneStartAngle" | "membraneEndAngle" => (-360.0, 360.0, 1.0, "°"),
            "neckWidth"
            | "neckHeight"
            | "enzymeHeight"
            | "enzymeWidth"
            | "enzymeReceptorSize"
            | "golgiHeight"
            | "golgiLength"
            | "golgiWidth"
            | "gproteinLowerHeight"
            | "gproteinUpperHeight"
            | "immunoglobulinHeight"
            | "immunoglobulinWidth"
            | "membraneMajorAxisSize"
            | "membraneMinorAxisSize" => (0.0, 400.0, 1.0, "%"),
            _ => (0.0, 1000.0, 0.1, "pt"),
        };
        Self {
            key,
            label,
            minimum,
            maximum,
            step,
            unit,
        }
    }
}

impl From<BioShapeKind> for BioDrawKind {
    fn from(value: BioShapeKind) -> Self {
        match value {
            BioShapeKind::OneSubstrateEnzyme => Self::OneSubstrateEnzyme,
            BioShapeKind::TwoSubstrateEnzyme => Self::TwoSubstrateEnzyme,
            BioShapeKind::Receptor => Self::Receptor,
            BioShapeKind::GProteinAlpha => Self::GProteinAlpha,
            BioShapeKind::GProteinBeta => Self::GProteinBeta,
            BioShapeKind::GProteinGamma => Self::GProteinGamma,
            BioShapeKind::Immunoglobulin => Self::Immunoglobulin,
            BioShapeKind::IonChannel => Self::IonChannel,
            BioShapeKind::EndoplasmicReticulum => Self::EndoplasmicReticulum,
            BioShapeKind::Golgi => Self::Golgi,
            BioShapeKind::MembraneLine => Self::MembraneLine,
            BioShapeKind::MembraneArc => Self::MembraneArc,
            BioShapeKind::MembraneEllipse => Self::MembraneEllipse,
            BioShapeKind::MembraneMicelle => Self::MembraneMicelle,
            BioShapeKind::Dna => Self::Dna,
            BioShapeKind::HelixProtein => Self::HelixProtein,
            BioShapeKind::Mitochondrion => Self::Mitochondrion,
            BioShapeKind::Cloud => Self::Cloud,
            BioShapeKind::TRna => Self::TRna,
            BioShapeKind::RibosomeA => Self::RibosomeA,
            BioShapeKind::RibosomeB => Self::RibosomeB,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BioShapeFillType {
    Unspecified,
    None,
    Solid,
    #[default]
    Shaded,
}

impl BioShapeFillType {
    pub const fn cdxml_name(self) -> &'static str {
        match self {
            Self::Unspecified => "Unspecified",
            Self::None => "None",
            Self::Solid => "Solid",
            Self::Shaded => "Shaded",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BioShapeLineType {
    #[default]
    Solid,
    Dashed,
    Bold,
    Wavy,
}

impl BioShapeLineType {
    pub const fn cdxml_name(self) -> &'static str {
        match self {
            Self::Solid => "Solid",
            Self::Dashed => "Dashed",
            Self::Bold => "Bold",
            Self::Wavy => "Wavy",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BioShapeParameters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cylinder_distance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cylinder_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cylinder_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dna_wave_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dna_wave_length: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dna_wave_offset: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dna_wave_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enzyme_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enzyme_receptor_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enzyme_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub golgi_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub golgi_length: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub golgi_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gprotein_lower_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gprotein_upper_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub helix_protein_extra: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub immunoglobulin_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub immunoglobulin_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub membrane_element_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub membrane_end_angle: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub membrane_major_axis_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub membrane_minor_axis_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub membrane_start_angle: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub neck_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub neck_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipe_width: Option<f64>,
}

impl BioShapeParameters {
    pub fn defaults_for(kind: BioShapeKind) -> Self {
        let mut parameters = Self::default();
        match kind {
            BioShapeKind::OneSubstrateEnzyme => parameters.enzyme_receptor_size = Some(25.0),
            BioShapeKind::Receptor => parameters.neck_width = Some(25.0),
            BioShapeKind::GProteinGamma => parameters.gprotein_upper_height = Some(25.0),
            BioShapeKind::MembraneLine
            | BioShapeKind::MembraneArc
            | BioShapeKind::MembraneEllipse
            | BioShapeKind::MembraneMicelle => {
                parameters.membrane_element_size = Some(4.8);
                if kind == BioShapeKind::MembraneArc {
                    parameters.membrane_start_angle = Some(-90.0);
                    parameters.membrane_end_angle = Some(0.0);
                }
            }
            BioShapeKind::Dna => {
                parameters.dna_wave_height = Some(14.4);
                parameters.dna_wave_length = Some(19.01);
                parameters.dna_wave_width = Some(3.6);
                parameters.dna_wave_offset = Some(3.6);
            }
            BioShapeKind::HelixProtein => {
                parameters.cylinder_width = Some(4.32);
                parameters.cylinder_height = Some(14.4);
                parameters.cylinder_distance = Some(2.59);
                parameters.pipe_width = Some(0.86);
                parameters.helix_protein_extra = Some(3.6);
            }
            _ => {}
        }
        parameters
    }

    pub fn resolved_for(&self, kind: BioShapeKind) -> Self {
        let defaults = Self::defaults_for(kind);
        let mut resolved = self.clone();
        macro_rules! fill_default {
            ($($field:ident),+ $(,)?) => {
                $(
                    if resolved.$field.is_none() {
                        resolved.$field = defaults.$field;
                    }
                )+
            };
        }
        fill_default!(
            cylinder_distance,
            cylinder_height,
            cylinder_width,
            dna_wave_height,
            dna_wave_length,
            dna_wave_offset,
            dna_wave_width,
            enzyme_height,
            enzyme_receptor_size,
            enzyme_width,
            golgi_height,
            golgi_length,
            golgi_width,
            gprotein_lower_height,
            gprotein_upper_height,
            helix_protein_extra,
            immunoglobulin_height,
            immunoglobulin_width,
            membrane_element_size,
            membrane_end_angle,
            membrane_major_axis_size,
            membrane_minor_axis_size,
            membrane_start_angle,
            neck_height,
            neck_width,
            pipe_width,
        );
        resolved
    }

    fn values(&self) -> [(&'static str, Option<f64>); 26] {
        [
            ("cylinderDistance", self.cylinder_distance),
            ("cylinderHeight", self.cylinder_height),
            ("cylinderWidth", self.cylinder_width),
            ("dnaWaveHeight", self.dna_wave_height),
            ("dnaWaveLength", self.dna_wave_length),
            ("dnaWaveOffset", self.dna_wave_offset),
            ("dnaWaveWidth", self.dna_wave_width),
            ("enzymeHeight", self.enzyme_height),
            ("enzymeReceptorSize", self.enzyme_receptor_size),
            ("enzymeWidth", self.enzyme_width),
            ("golgiHeight", self.golgi_height),
            ("golgiLength", self.golgi_length),
            ("golgiWidth", self.golgi_width),
            ("gproteinLowerHeight", self.gprotein_lower_height),
            ("gproteinUpperHeight", self.gprotein_upper_height),
            ("helixProteinExtra", self.helix_protein_extra),
            ("immunoglobulinHeight", self.immunoglobulin_height),
            ("immunoglobulinWidth", self.immunoglobulin_width),
            ("membraneElementSize", self.membrane_element_size),
            ("membraneEndAngle", self.membrane_end_angle),
            ("membraneMajorAxisSize", self.membrane_major_axis_size),
            ("membraneMinorAxisSize", self.membrane_minor_axis_size),
            ("membraneStartAngle", self.membrane_start_angle),
            ("neckHeight", self.neck_height),
            ("neckWidth", self.neck_width),
            ("pipeWidth", self.pipe_width),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BioShapeData {
    pub kind: BioShapeKind,
    pub center: [f64; 3],
    pub major_axis_end: [f64; 3],
    pub minor_axis_end: [f64; 3],
    #[serde(default)]
    pub fill_type: BioShapeFillType,
    #[serde(default)]
    pub line_type: BioShapeLineType,
    pub color: String,
    pub line_width: f64,
    pub bold_width: f64,
    #[serde(default = "default_bio_shape_margin_width")]
    pub margin_width: f64,
    pub hash_spacing: f64,
    pub fade_percent: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha: Option<f64>,
    #[serde(default)]
    pub parameters: BioShapeParameters,
}

impl BioShapeData {
    pub fn validate(&self) -> Result<(), String> {
        for (name, point) in [
            ("center", self.center),
            ("majorAxisEnd", self.major_axis_end),
            ("minorAxisEnd", self.minor_axis_end),
        ] {
            if !point.into_iter().all(f64::is_finite) {
                return Err(format!("BioShape {name} contains a non-finite coordinate"));
            }
        }
        let major = (self.major_axis_end[0] - self.center[0])
            .hypot(self.major_axis_end[1] - self.center[1]);
        let minor = (self.minor_axis_end[0] - self.center[0])
            .hypot(self.minor_axis_end[1] - self.center[1]);
        if major <= crate::EPSILON || minor <= crate::EPSILON {
            return Err("BioShape axes must have positive length".to_string());
        }
        for (name, value) in [
            ("lineWidth", self.line_width),
            ("boldWidth", self.bold_width),
            ("marginWidth", self.margin_width),
            ("hashSpacing", self.hash_spacing),
            ("fadePercent", self.fade_percent),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!("BioShape {name} must be finite and non-negative"));
            }
        }
        if let Some(alpha) = self.alpha {
            if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
                return Err("BioShape alpha must be finite and between 0 and 1".to_string());
            }
        }
        for (name, value) in self.parameters.values() {
            let Some(value) = value else {
                continue;
            };
            if !value.is_finite() {
                return Err(format!("BioShape parameter {name} must be finite"));
            }
            if !matches!(name, "membraneStartAngle" | "membraneEndAngle") && value < 0.0 {
                return Err(format!("BioShape parameter {name} must be non-negative"));
            }
        }
        if !self.color.starts_with('#') || self.color.len() != 7 {
            return Err("BioShape color must be #RRGGBB".to_string());
        }
        Ok(())
    }
}

fn default_bio_shape_margin_width() -> f64 {
    crate::DEFAULT_BOND_MARGIN_WIDTH_PT.value()
}
