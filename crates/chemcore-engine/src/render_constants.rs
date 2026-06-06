use crate::{css_px, world_pt, WorldPt, DEFAULT_BOND_STROKE_PT};

pub const VIEWER_BOND_STROKE_PT: WorldPt = world_pt(DEFAULT_BOND_STROKE_PT);

pub const DEFAULT_BOND_SPACING_PERCENT: f64 = 12.0;
pub const DEFAULT_BOND_MARGIN_WIDTH_PT: WorldPt = world_pt(2.0);
pub const ACS_BOND_MARGIN_WIDTH_PT: WorldPt = world_pt(1.6);
pub const DOUBLE_BOND_SIDE_INSET_PT: WorldPt = world_pt(1.05);
pub const HASH_WEDGE_SPACING_PT: WorldPt = world_pt(2.7);
pub const HASH_WEDGE_START_OFFSET_PT: WorldPt = world_pt(1.0);
pub const HASH_WEDGE_END_INSET_PT: WorldPt = world_pt(0.0);
pub const HASH_BLACK_SEGMENT_LENGTH_PT: WorldPt = world_pt(1.0);
pub const HASH_TARGET_GAP_LENGTH_PT: WorldPt = world_pt(1.9);
pub const DEFAULT_HASH_SPACING_PT: WorldPt = HASH_WEDGE_SPACING_PT;
pub const SOLID_WEDGE_END_INSET_PT: WorldPt = world_pt(0.0);
pub const BOLD_BOND_WIDTH_PT: WorldPt = world_pt(4.0);
pub const SOLID_WEDGE_WIDTH_PT: WorldPt = world_pt(6.0);

pub const HASH_WEDGE_INITIAL_HALF_WIDTH_PT: WorldPt = css_px(0.42).to_world_pt();
pub const HASH_WEDGE_PROGRESS_BASE_HALF_WIDTH_PT: WorldPt = css_px(0.16).to_world_pt();
pub const HASH_WEDGE_PROGRESS_HALF_WIDTH_RANGE_PT: WorldPt = css_px(1.72).to_world_pt();
pub const HASH_WEDGE_INITIAL_SEGMENT_WIDTH_PT: WorldPt = css_px(0.82).to_world_pt();
pub const HASH_WEDGE_SEGMENT_WIDTH_PT: WorldPt = css_px(0.72).to_world_pt();

pub const BOUNDARY_JOIN_MIN_BACKTRACK_PT: WorldPt = css_px(0.85).to_world_pt();
pub const LABEL_GEOMETRY_CLIP_MARGIN_PT: WorldPt = world_pt(1.2);
pub const DEFAULT_ARROW_HEAD_LENGTH_RATIO: f64 = 10.0;
pub const TEXT_WRAP_ESTIMATED_CHAR_WIDTH_PT: WorldPt = css_px(6.0).to_world_pt();
pub const DASH_GAP_STROKE_EXTRA_PT: WorldPt = world_pt(0.26);
pub const HASH_WEDGE_GAP_START_OFFSET_PT: WorldPt = world_pt(0.38);
pub const HASH_WEDGE_GAP_END_INSET_PT: WorldPt = world_pt(0.0);
pub const ACS_LABEL_GEOMETRY_CLIP_MARGIN_PT: WorldPt = world_pt(0.8);
