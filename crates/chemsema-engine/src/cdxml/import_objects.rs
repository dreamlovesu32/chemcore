use super::*;
use crate::Point;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

#[path = "import_brackets.rs"]
mod import_brackets;
#[path = "import_graphics.rs"]
mod import_graphics;
#[path = "import_images.rs"]
mod import_images;
#[path = "import_lines.rs"]
mod import_lines;
#[path = "import_spectra.rs"]
mod import_spectra;
#[path = "import_stoichiometry.rs"]
mod import_stoichiometry;
#[path = "import_text.rs"]
mod import_text;

use import_brackets::parse_ordered_bbox;
use import_lines::non_bond_dash_array;

pub(super) use import_brackets::append_bracket_objects;
pub(super) use import_graphics::{
    append_orbital_shape_objects, append_shape_objects, append_table_shape_objects,
    append_tlc_plate_shape_objects, associate_table_cell_contents,
};
pub(super) use import_images::append_embedded_image_objects;
pub(super) use import_lines::{append_curve_objects, append_line_objects};
pub(super) use import_spectra::append_spectrum_objects;
pub(super) use import_stoichiometry::import_reactions_and_stoichiometry_grids;
pub(super) use import_text::{
    append_synthesized_enhanced_stereo_text_objects, append_text_objects,
};

const CHEMDRAW_AUTO_BRACKET_LABEL_GAP_EM: f64 = 0.1875;
const MAX_EMBEDDED_IMAGE_BYTES: usize = 64 * 1024 * 1024;
const MAX_EMBEDDED_IMAGE_DIMENSION_PX: u32 = 32_768;
const MAX_EMBEDDED_IMAGE_PIXELS: u64 = 100_000_000;

#[derive(Clone, Copy)]
struct LegacyArcGeometry {
    head: [f64; 2],
    tail: [f64; 2],
    center: [f64; 2],
    radius: f64,
}

#[derive(Clone)]
struct PendingCdxmlBracket {
    kind: String,
    bbox: [f64; 4],
    z_index: i32,
    graphic_id: Option<String>,
    repeat_count: Option<u32>,
    stroke: String,
    stroke_width: f64,
    lip_size: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CdxmlTextObjectRole {
    FreeText,
    BracketUsage,
    ParameterizedBracketLabel,
    AtomNumber,
    Query,
    Stereo,
    EnhancedStereo,
    NmrAssignment,
}

impl CdxmlTextObjectRole {
    fn from_object_tag_name(name: Option<&str>) -> Option<Self> {
        Some(match name? {
            "bracketusage" => Self::BracketUsage,
            "parameterizedBracketLabel" => Self::ParameterizedBracketLabel,
            "number" => Self::AtomNumber,
            "query" => Self::Query,
            "stereo" => Self::Stereo,
            "enhancedstereo" => Self::EnhancedStereo,
            "/CS/CD/assign" => Self::NmrAssignment,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::FreeText => "free_text",
            Self::BracketUsage => "bracket_usage",
            Self::ParameterizedBracketLabel => "parameterized_bracket_label",
            Self::AtomNumber => "atom_number",
            Self::Query => "query",
            Self::Stereo => "stereo",
            Self::EnhancedStereo => "enhanced_stereo",
            Self::NmrAssignment => "nmr_assignment",
        }
    }

    fn is_bracket_label(self) -> bool {
        matches!(self, Self::BracketUsage | Self::ParameterizedBracketLabel)
    }
}
