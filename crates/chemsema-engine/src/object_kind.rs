use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SceneObjectKind {
    Molecule,
    Text,
    Line,
    Curve,
    Bracket,
    Symbol,
    Shape,
    Image,
    Spectrum,
    Geometry,
    Constraint,
    Group,
}

impl SceneObjectKind {
    pub const ALL: [Self; 12] = [
        Self::Molecule,
        Self::Text,
        Self::Line,
        Self::Curve,
        Self::Bracket,
        Self::Symbol,
        Self::Shape,
        Self::Image,
        Self::Spectrum,
        Self::Geometry,
        Self::Constraint,
        Self::Group,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Molecule => "molecule",
            Self::Text => "text",
            Self::Line => "line",
            Self::Curve => "curve",
            Self::Bracket => "bracket",
            Self::Symbol => "symbol",
            Self::Shape => "shape",
            Self::Image => "image",
            Self::Spectrum => "spectrum",
            Self::Geometry => "geometry",
            Self::Constraint => "constraint",
            Self::Group => "group",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "molecule" => Ok(Self::Molecule),
            "text" => Ok(Self::Text),
            "line" => Ok(Self::Line),
            "curve" => Ok(Self::Curve),
            "bracket" => Ok(Self::Bracket),
            "symbol" => Ok(Self::Symbol),
            "shape" => Ok(Self::Shape),
            "image" => Ok(Self::Image),
            "spectrum" => Ok(Self::Spectrum),
            "geometry" => Ok(Self::Geometry),
            "constraint" => Ok(Self::Constraint),
            "group" => Ok(Self::Group),
            _ => Err(format!("Unsupported scene object type '{value}'")),
        }
    }

    pub const fn is_graphic_selection(self) -> bool {
        matches!(
            self,
            Self::Line
                | Self::Curve
                | Self::Bracket
                | Self::Symbol
                | Self::Shape
                | Self::Image
                | Self::Spectrum
                | Self::Geometry
                | Self::Constraint
                | Self::Group
        )
    }
}
