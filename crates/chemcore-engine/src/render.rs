use crate::{
    legacy_mol::{parse_molblock, LegacyAtom, LegacyBond as LegacyMolBond, LegacyMol},
    px_to_pt, Bond, BondLinePattern, BondLineWeight, ChemcoreDocument, DoubleBondPlacement,
    LabelRun, MoleculeFragment, Node, ObjectPayload, Point, ResourceData, SceneObject, Vector,
    DEFAULT_BOND_STROKE, EPSILON,
};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, BTreeSet};

#[path = "render/bond_geometry.rs"]
mod bond_geometry;
#[path = "render/bond_metrics.rs"]
mod bond_metrics;
#[path = "render_bonds.rs"]
mod bond_render;
#[path = "render/bounds.rs"]
mod bounds;
#[path = "render_contact.rs"]
mod contact;
#[path = "render/labels.rs"]
mod labels;
#[path = "render_legacy.rs"]
mod legacy_render;
#[path = "render_objects.rs"]
mod object_render;
#[path = "render_primitives.rs"]
mod primitives;
#[path = "render/style_payload.rs"]
mod style_payload;

use bond_render::{compute_solid_wedge_points, render_fragment_bond};
pub use bounds::{render_primitive_bounds, render_primitives_bounds};
use contact::{
    bond_ray_is_acute, build_main_bond_contact_kernel, center_double_skips_extension,
    main_bond_endpoint_geometry, main_contact_is_straight_through, main_contact_side,
    render_main_bond_contact_patches, MainBondContactKernel,
};
use legacy_render::render_legacy_molecule_object;
use object_render::{
    render_bracket_object, render_line_object, render_molecule_object, render_shape_object,
    render_text_object,
};
use primitives::{
    push_bond_polygon, push_knockout_polygon, push_label_knockout_polygon, push_line, push_path,
    push_polygon, push_polyline, push_text, push_text_for_node, push_text_rotated,
};
pub use primitives::{RenderPrimitive, RenderRole};

use bond_geometry::*;
use bond_metrics::*;
pub(crate) use bounds::{fragment_bond_visual_bounds, shape_object_visual_bounds};
use labels::{
    clip_point_out_of_label_geometry, label_box_world, label_polygons_world, render_fragment_line,
    render_fragment_line_with_profiles, world_point,
};
use style_payload::*;

const VIEWER_BOND_STROKE: f64 = crate::VIEWER_BOND_STROKE_PT.value();
const DEFAULT_MULTI_BOND_CENTER_SPACING_RATIO: f64 = crate::DEFAULT_BOND_SPACING_PERCENT / 100.0;
const DOUBLE_BOND_SIDE_INSET: f64 = crate::DOUBLE_BOND_SIDE_INSET_PT.value();
const DOUBLE_BOND_SIDE_INSET_RATIO: f64 = 0.14;
const HASH_WEDGE_SPACING: f64 = crate::HASH_WEDGE_SPACING_PT.value();
const HASH_WEDGE_START_OFFSET: f64 = crate::HASH_WEDGE_START_OFFSET_PT.value();
const HASH_WEDGE_END_INSET: f64 = crate::HASH_WEDGE_END_INSET_PT.value();
const HASH_BLACK_SEGMENT_LENGTH: f64 = crate::HASH_BLACK_SEGMENT_LENGTH_PT.value();
const HASH_TARGET_GAP_LENGTH: f64 = crate::HASH_TARGET_GAP_LENGTH_PT.value();
const SOLID_WEDGE_END_INSET: f64 = crate::SOLID_WEDGE_END_INSET_PT.value();
const CENTER_DOUBLE_NO_EXTENSION_ANGLE_DEGREES: f64 = 162.0;
const CHEMCORE_INK: &str = "#000000";
const KNOCKOUT_FILL: &str = "#ffffff";
const BOLD_BOND_WIDTH: f64 = crate::BOLD_BOND_WIDTH_PT.value();
const MAIN_CONTACT_MITER_LIMIT: f64 = 4.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct RectBox {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

impl RectBox {
    fn expanded(self, margin: f64) -> Self {
        Self {
            x1: self.x1 - margin,
            y1: self.y1 - margin,
            x2: self.x2 + margin,
            y2: self.y2 + margin,
        }
    }

    fn contains(self, point: Point) -> bool {
        point.x >= self.x1 && point.x <= self.x2 && point.y >= self.y1 && point.y <= self.y2
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LineGeometry {
    point: Point,
    direction: Vector,
    shared: Point,
    length: f64,
    offset_distance: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ArrowHeadGeometry {
    length: f64,
    center_length: f64,
    width: f64,
    kind: ArrowHeadKind,
    curve: f64,
    head_full: bool,
    bold: bool,
    no_go: ArrowNoGoGeometry,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ArrowArcGeometry {
    center: Point,
    major_axis_end: Point,
    minor_axis_end: Point,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ArrowHeadKind {
    #[default]
    Solid,
    Hollow,
    Open,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ArrowNoGoGeometry {
    #[default]
    None,
    Cross,
    Hash,
}

pub fn render_document(document: &ChemcoreDocument) -> Vec<RenderPrimitive> {
    let mut out = Vec::new();
    render_scene_objects(&mut out, document, &document.objects);
    out
}

fn render_scene_objects(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    objects: &[SceneObject],
) {
    let mut visible_objects: Vec<&SceneObject> =
        objects.iter().filter(|object| object.visible).collect();
    visible_objects.sort_by(|a, b| a.z_index.cmp(&b.z_index).then_with(|| a.id.cmp(&b.id)));

    for object in visible_objects {
        render_scene_object(out, document, object);
    }
}

fn render_scene_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
) {
    match object.object_type.as_str() {
        "molecule" => render_molecule_object(out, document, object),
        "line" => render_line_object(out, document, object),
        "text" => render_text_object(out, document, object),
        "shape" => render_shape_object(out, document, object),
        "bracket" | "symbol" => render_bracket_object(out, document, object),
        "group" => render_scene_objects(out, document, &object.children),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(left: f64, right: f64) {
        assert!((left - right).abs() <= 1.0e-6, "{left} != {right}");
    }

    fn line_geometry(point: Point, direction: Vector, shared: Point) -> LineGeometry {
        LineGeometry {
            point,
            direction,
            shared,
            length: 20.0,
            offset_distance: 1.0,
        }
    }

    fn test_bond() -> Bond {
        Bond {
            id: "b1".to_string(),
            begin: "n1".to_string(),
            end: "n2".to_string(),
            order: 1,
            double: None,
            stereo: None,
            stroke_width: VIEWER_BOND_STROKE,
            stroke: None,
            bold_width: None,
            wedge_width: None,
            label_clip_margin: None,
            hash_spacing: None,
            bond_spacing: None,
            margin_width: None,
            line_styles: crate::BondLineStyles::default(),
            line_weights: crate::BondLineWeights::default(),
            meta: serde_json::Value::Null,
        }
    }

    fn black_segment_lengths(
        length: f64,
        start_offset: f64,
        end_inset: f64,
        gaps: &[(f64, f64)],
    ) -> Vec<f64> {
        let usable_start = start_offset.max(0.0);
        let usable_end = (length - end_inset).max(usable_start);
        let mut cursor = usable_start;
        let mut segments = Vec::new();
        for (gap_start, gap_end) in gaps {
            segments.push(gap_start - cursor);
            cursor = *gap_end;
        }
        if usable_end > cursor {
            segments.push(usable_end - cursor);
        }
        segments
    }

    #[test]
    fn equal_black_segment_gap_intervals_keep_black_segments_equal() {
        let gaps = equal_black_segment_gap_intervals(10.0, 0.4, 0.6, 1.0, 1.5);
        assert!(!gaps.is_empty());

        let black_lengths = black_segment_lengths(10.0, 0.4, 0.6, &gaps);
        assert!(black_lengths.len() >= 2);
        for length in &black_lengths {
            approx_eq(*length, 1.0);
        }
    }

    #[test]
    fn equal_black_segment_gap_intervals_return_empty_when_too_short() {
        let gaps = equal_black_segment_gap_intervals(0.8, 0.0, 0.0, 1.0, 1.5);
        assert!(gaps.is_empty());
    }

    #[test]
    fn hashed_wedge_gap_intervals_respect_start_offset_and_end_inset() {
        let gaps = hashed_wedge_gap_intervals(18.0, VIEWER_BOND_STROKE * 2.0, &test_bond());
        assert!(!gaps.is_empty());
        assert!(gaps[0].0 > 0.0);
        assert!(gaps.last().unwrap().1 < 18.0);

        let start_offset = crate::HASH_WEDGE_GAP_START_OFFSET_PT.value() * 2.0;
        let end_inset = crate::HASH_WEDGE_GAP_END_INSET_PT.value() * 2.0;
        let black_lengths = black_segment_lengths(18.0, start_offset, end_inset, &gaps);
        for length in &black_lengths {
            approx_eq(*length, black_lengths[0]);
        }
    }

    #[test]
    fn hashed_wedge_segments_follow_bond_normal_for_diagonal_bonds() {
        let start = Point::new(10.0, 20.0);
        let end = Point::new(50.0, 60.0);
        let axis = Vector::new(end.x - start.x, end.y - start.y).normalized();
        let segments = compute_hashed_wedge_segments(start, end, VIEWER_BOND_STROKE);

        assert!(segments.len() >= 2);
        for (segment_start, segment_end, _) in segments {
            let segment_axis = Vector::new(
                segment_end.x - segment_start.x,
                segment_end.y - segment_start.y,
            )
            .normalized();
            let dot = axis.x * segment_axis.x + axis.y * segment_axis.y;
            assert!(dot.abs() <= 1.0e-6, "{segment_start:?} {segment_end:?}");
        }
    }

    #[test]
    fn simple_main_line_polygon_points_returns_constant_width_shaft() {
        let points = simple_main_line_polygon_points(
            Point::new(10.0, 20.0),
            Point::new(30.0, 20.0),
            VIEWER_BOND_STROKE,
        )
        .expect("simple shaft polygon");
        assert_eq!(points.len(), 4);
        approx_eq(points[0].distance(points[3]), VIEWER_BOND_STROKE);
        approx_eq(points[1].distance(points[2]), VIEWER_BOND_STROKE);
    }

    #[test]
    fn boundary_line_join_candidate_rejects_far_backward_intersection() {
        let current = line_geometry(
            Point::new(0.0, 0.0),
            Vector::new(1.0, 0.0),
            Point::new(0.0, 0.0),
        );
        let other = line_geometry(
            Point::new(-5.0, -1.0),
            Vector::new(0.0, 1.0),
            Point::new(-5.0, 0.0),
        );

        assert!(boundary_line_join_candidate(&current, &other).is_none());
    }

    #[test]
    fn paired_boundary_line_join_points_avoids_trivial_assignment() {
        let current = [
            line_geometry(
                Point::new(0.0, 1.0),
                Vector::new(1.0, 0.0),
                Point::new(0.0, 0.0),
            ),
            line_geometry(
                Point::new(0.0, -1.0),
                Vector::new(1.0, 0.0),
                Point::new(0.0, 0.0),
            ),
        ];
        let other = [
            line_geometry(
                Point::new(0.0, -1.0),
                Vector::new(1.0, 1.0),
                Point::new(0.0, 0.0),
            ),
            line_geometry(
                Point::new(0.0, 1.0),
                Vector::new(1.0, -1.0),
                Point::new(0.0, 0.0),
            ),
        ];

        let (points, _) = paired_boundary_line_join_points(current, other).expect("join points");
        approx_eq(points[0].x, 2.0);
        approx_eq(points[0].y, 1.0);
        approx_eq(points[1].x, 2.0);
        approx_eq(points[1].y, -1.0);
    }
}
