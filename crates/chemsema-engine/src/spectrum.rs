use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpectrumClass {
    #[default]
    Unknown,
    Chromatogram,
    Infrared,
    UvVis,
    XRayDiffraction,
    MassSpectrum,
    Nmr,
    Raman,
    Fluorescence,
    Atomic,
}

impl SpectrumClass {
    pub(crate) fn from_cdxml(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("Unknown").trim() {
            "" | "0" | "Unknown" => Ok(Self::Unknown),
            "1" | "Chromatogram" => Ok(Self::Chromatogram),
            "2" | "Infrared" => Ok(Self::Infrared),
            "3" | "UVVis" => Ok(Self::UvVis),
            "4" | "XRayDiffraction" => Ok(Self::XRayDiffraction),
            "5" | "MassSpectrum" => Ok(Self::MassSpectrum),
            "6" | "NMR" => Ok(Self::Nmr),
            "7" | "Raman" => Ok(Self::Raman),
            "8" | "Fluorescence" => Ok(Self::Fluorescence),
            "9" | "Atomic" => Ok(Self::Atomic),
            value => Err(format!("unsupported CDXML spectrum Class '{value}'")),
        }
    }

    pub(crate) const fn as_cdxml(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Chromatogram => "Chromatogram",
            Self::Infrared => "Infrared",
            Self::UvVis => "UVVis",
            Self::XRayDiffraction => "XRayDiffraction",
            Self::MassSpectrum => "MassSpectrum",
            Self::Nmr => "NMR",
            Self::Raman => "Raman",
            Self::Fluorescence => "Fluorescence",
            Self::Atomic => "Atomic",
        }
    }

    pub(crate) const fn cdx_value(self) -> i16 {
        match self {
            Self::Unknown => 0,
            Self::Chromatogram => 1,
            Self::Infrared => 2,
            Self::UvVis => 3,
            Self::XRayDiffraction => 4,
            Self::MassSpectrum => 5,
            Self::Nmr => 6,
            Self::Raman => 7,
            Self::Fluorescence => 8,
            Self::Atomic => 9,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpectrumXAxisType {
    #[default]
    Unknown,
    Wavenumbers,
    Microns,
    Hertz,
    MassUnits,
    PartsPerMillion,
    Other,
}

impl SpectrumXAxisType {
    pub(crate) fn from_cdxml(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("Unknown").trim() {
            "" | "0" | "Unknown" => Ok(Self::Unknown),
            "1" | "Wavenumbers" => Ok(Self::Wavenumbers),
            "2" | "Microns" => Ok(Self::Microns),
            "3" | "Hertz" => Ok(Self::Hertz),
            "4" | "MassUnits" => Ok(Self::MassUnits),
            "5" | "PartsPerMillion" => Ok(Self::PartsPerMillion),
            "6" | "Other" => Ok(Self::Other),
            value => Err(format!("unsupported CDXML spectrum XType '{value}'")),
        }
    }

    pub(crate) const fn as_cdxml(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Wavenumbers => "Wavenumbers",
            Self::Microns => "Microns",
            Self::Hertz => "Hertz",
            Self::MassUnits => "MassUnits",
            Self::PartsPerMillion => "PartsPerMillion",
            Self::Other => "Other",
        }
    }

    pub(crate) const fn cdx_value(self) -> i16 {
        match self {
            Self::Unknown => 0,
            Self::Wavenumbers => 1,
            Self::Microns => 2,
            Self::Hertz => 3,
            Self::MassUnits => 4,
            Self::PartsPerMillion => 5,
            Self::Other => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpectrumYAxisType {
    #[default]
    Unknown,
    Absorbance,
    Transmittance,
    PercentTransmittance,
    Other,
    ArbitraryUnits,
}

impl SpectrumYAxisType {
    pub(crate) fn from_cdxml(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("Unknown").trim() {
            "" | "0" | "Unknown" => Ok(Self::Unknown),
            "1" | "Absorbance" => Ok(Self::Absorbance),
            "2" | "Transmittance" => Ok(Self::Transmittance),
            "3" | "PercentTransmittance" => Ok(Self::PercentTransmittance),
            "4" | "Other" => Ok(Self::Other),
            "5" | "ArbitraryUnits" => Ok(Self::ArbitraryUnits),
            value => Err(format!("unsupported CDXML spectrum YType '{value}'")),
        }
    }

    pub(crate) const fn as_cdxml(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Absorbance => "Absorbance",
            Self::Transmittance => "Transmittance",
            Self::PercentTransmittance => "PercentTransmittance",
            Self::Other => "Other",
            Self::ArbitraryUnits => "ArbitraryUnits",
        }
    }

    pub(crate) const fn cdx_value(self) -> i16 {
        match self {
            Self::Unknown => 0,
            Self::Absorbance => 1,
            Self::Transmittance => 2,
            Self::PercentTransmittance => 3,
            Self::Other => 4,
            Self::ArbitraryUnits => 5,
        }
    }
}

fn default_y_scale() -> f64 {
    1.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpectrumData {
    #[serde(default)]
    pub class: SpectrumClass,
    pub x_low: f64,
    pub x_spacing: f64,
    #[serde(default)]
    pub x_type: SpectrumXAxisType,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub x_axis_label: String,
    #[serde(default)]
    pub y_low: f64,
    #[serde(default = "default_y_scale")]
    pub y_scale: f64,
    #[serde(default)]
    pub y_type: SpectrumYAxisType,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub y_axis_label: String,
    pub data_points: Vec<f64>,
}

impl SpectrumData {
    pub const MAX_DATA_POINTS: usize = 10_000_000;

    pub fn validate(&self) -> Result<(), String> {
        if !self.x_low.is_finite() {
            return Err("spectrum xLow must be finite".to_string());
        }
        if !self.x_spacing.is_finite() {
            return Err("spectrum xSpacing must be finite".to_string());
        }
        if !self.y_low.is_finite() {
            return Err("spectrum yLow must be finite".to_string());
        }
        if !self.y_scale.is_finite() {
            return Err("spectrum yScale must be finite".to_string());
        }
        if self.data_points.is_empty() {
            return Err("spectrum dataPoints must not be empty".to_string());
        }
        if self.data_points.len() > Self::MAX_DATA_POINTS {
            return Err(format!(
                "spectrum dataPoints exceeds the {} point limit",
                Self::MAX_DATA_POINTS
            ));
        }
        if self.data_points.iter().any(|value| !value.is_finite()) {
            return Err("spectrum dataPoints must contain only finite numbers".to_string());
        }
        Ok(())
    }

    pub fn decoded_points(&self) -> impl ExactSizeIterator<Item = f64> + '_ {
        self.data_points
            .iter()
            .map(|value| self.y_low + value * self.y_scale)
    }

    pub fn x_high(&self) -> f64 {
        self.x_low + self.x_spacing * self.data_points.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectrum_class_covers_every_official_name_and_binary_value() {
        let cases = [
            ("Unknown", SpectrumClass::Unknown, 0),
            ("Chromatogram", SpectrumClass::Chromatogram, 1),
            ("Infrared", SpectrumClass::Infrared, 2),
            ("UVVis", SpectrumClass::UvVis, 3),
            ("XRayDiffraction", SpectrumClass::XRayDiffraction, 4),
            ("MassSpectrum", SpectrumClass::MassSpectrum, 5),
            ("NMR", SpectrumClass::Nmr, 6),
            ("Raman", SpectrumClass::Raman, 7),
            ("Fluorescence", SpectrumClass::Fluorescence, 8),
            ("Atomic", SpectrumClass::Atomic, 9),
        ];
        for (name, expected, value) in cases {
            assert_eq!(SpectrumClass::from_cdxml(Some(name)), Ok(expected));
            assert_eq!(expected.as_cdxml(), name);
            assert_eq!(expected.cdx_value(), value);
            assert_eq!(
                SpectrumClass::from_cdxml(Some(&value.to_string())),
                Ok(expected)
            );
        }
    }

    #[test]
    fn spectrum_axis_types_cover_every_official_name_and_binary_value() {
        let x_cases = [
            ("Unknown", SpectrumXAxisType::Unknown, 0),
            ("Wavenumbers", SpectrumXAxisType::Wavenumbers, 1),
            ("Microns", SpectrumXAxisType::Microns, 2),
            ("Hertz", SpectrumXAxisType::Hertz, 3),
            ("MassUnits", SpectrumXAxisType::MassUnits, 4),
            ("PartsPerMillion", SpectrumXAxisType::PartsPerMillion, 5),
            ("Other", SpectrumXAxisType::Other, 6),
        ];
        for (name, expected, value) in x_cases {
            assert_eq!(SpectrumXAxisType::from_cdxml(Some(name)), Ok(expected));
            assert_eq!(expected.as_cdxml(), name);
            assert_eq!(expected.cdx_value(), value);
            assert_eq!(
                SpectrumXAxisType::from_cdxml(Some(&value.to_string())),
                Ok(expected)
            );
        }

        let y_cases = [
            ("Unknown", SpectrumYAxisType::Unknown, 0),
            ("Absorbance", SpectrumYAxisType::Absorbance, 1),
            ("Transmittance", SpectrumYAxisType::Transmittance, 2),
            (
                "PercentTransmittance",
                SpectrumYAxisType::PercentTransmittance,
                3,
            ),
            ("Other", SpectrumYAxisType::Other, 4),
            ("ArbitraryUnits", SpectrumYAxisType::ArbitraryUnits, 5),
        ];
        for (name, expected, value) in y_cases {
            assert_eq!(SpectrumYAxisType::from_cdxml(Some(name)), Ok(expected));
            assert_eq!(expected.as_cdxml(), name);
            assert_eq!(expected.cdx_value(), value);
            assert_eq!(
                SpectrumYAxisType::from_cdxml(Some(&value.to_string())),
                Ok(expected)
            );
        }
    }

    #[test]
    fn spectrum_enums_reject_unofficial_values() {
        assert!(SpectrumClass::from_cdxml(Some("NMR2")).is_err());
        assert!(SpectrumXAxisType::from_cdxml(Some("ppm")).is_err());
        assert!(SpectrumYAxisType::from_cdxml(Some("Percent")).is_err());
    }
}
