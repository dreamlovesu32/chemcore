use crate::{css_px, world_cm, WorldCm, DEFAULT_BOND_STROKE_CM};

pub const VIEWER_BOND_STROKE_CM: WorldCm = world_cm(DEFAULT_BOND_STROKE_CM);

pub const DEFAULT_BOND_SPACING_PERCENT: f64 = 12.0;
pub const DEFAULT_BOND_MARGIN_WIDTH_CM: WorldCm = world_cm(2.0);
pub const ACS_BOND_MARGIN_WIDTH_CM: WorldCm = world_cm(1.6);
pub const DOUBLE_BOND_SIDE_INSET_CM: WorldCm = world_cm(1.05);
pub const HASH_WEDGE_SPACING_CM: WorldCm = world_cm(2.7);
pub const HASH_WEDGE_START_OFFSET_CM: WorldCm = world_cm(1.0);
pub const HASH_WEDGE_END_INSET_CM: WorldCm = world_cm(0.0);
pub const HASH_BLACK_SEGMENT_LENGTH_CM: WorldCm = world_cm(1.0);
pub const HASH_TARGET_GAP_LENGTH_CM: WorldCm = world_cm(1.9);
pub const DEFAULT_HASH_SPACING_CM: WorldCm = HASH_WEDGE_SPACING_CM;
pub const SOLID_WEDGE_END_INSET_CM: WorldCm = world_cm(0.0);
pub const BOLD_BOND_WIDTH_CM: WorldCm = world_cm(4.0);
pub const SOLID_WEDGE_WIDTH_CM: WorldCm = world_cm(6.0);

pub const HASH_WEDGE_INITIAL_HALF_WIDTH_CM: WorldCm = css_px(0.42).to_world_cm();
pub const HASH_WEDGE_PROGRESS_BASE_HALF_WIDTH_CM: WorldCm = css_px(0.16).to_world_cm();
pub const HASH_WEDGE_PROGRESS_HALF_WIDTH_RANGE_CM: WorldCm = css_px(1.72).to_world_cm();
pub const HASH_WEDGE_INITIAL_SEGMENT_WIDTH_CM: WorldCm = css_px(0.82).to_world_cm();
pub const HASH_WEDGE_SEGMENT_WIDTH_CM: WorldCm = css_px(0.72).to_world_cm();

pub const BOUNDARY_JOIN_MIN_BACKTRACK_CM: WorldCm = css_px(0.85).to_world_cm();
pub const LABEL_GEOMETRY_CLIP_MARGIN_CM: WorldCm = world_cm(1.2);
pub const DEFAULT_ARROW_HEAD_LENGTH_RATIO: f64 = 10.0;
pub const TEXT_WRAP_ESTIMATED_CHAR_WIDTH_CM: WorldCm = css_px(6.0).to_world_cm();
pub const DASH_GAP_STROKE_EXTRA_CM: WorldCm = world_cm(0.26);
pub const HASH_WEDGE_GAP_START_OFFSET_CM: WorldCm = world_cm(0.38);
pub const HASH_WEDGE_GAP_END_INSET_CM: WorldCm = world_cm(0.0);
pub const ACS_LABEL_GEOMETRY_CLIP_MARGIN_CM: WorldCm = world_cm(0.8);
