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
    bond_ray_is_acute, build_main_bond_contact_kernel, build_main_bond_contact_kernel_for_nodes,
    center_double_skips_extension, main_bond_endpoint_geometry, main_contact_is_straight_through,
    main_contact_side, render_main_bond_contact_patches, MainBondContactKernel,
};
use legacy_render::render_legacy_molecule_object;
use object_render::{
    render_bracket_object, render_line_object, render_molecule_object,
    render_molecule_object_targets, render_shape_object, render_text_object,
};
use primitives::{
    push_bond_knockout_polygon, push_bond_polygon, push_label_knockout_polygon, push_line,
    push_path, push_polygon, push_polyline, push_text, push_text_for_node, push_text_rotated,
};
pub use primitives::{RenderPrimitive, RenderRole};

use bond_geometry::*;
use bond_metrics::*;
pub(crate) use bounds::{
    bracket_object_visual_bounds, fragment_bond_visual_bounds, line_object_visual_bounds,
    shape_object_visual_bounds,
};
use labels::{
    clip_segment_out_of_label_geometry, label_box_world, label_polygons_world,
    render_fragment_line, render_fragment_line_with_profiles, world_point,
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
    shaft_spacing: f64,
    equilibrium_ratio: f64,
    kind: ArrowHeadKind,
    curve: f64,
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
    Equilibrium,
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
    let mut sequence = 0;
    render_scene_objects_layered(&mut out, &mut sequence, document, &document.objects, None);
    out.sort_by(|a, b| {
        a.z_index
            .cmp(&b.z_index)
            .then_with(|| a.sequence.cmp(&b.sequence))
    });
    out.into_iter()
        .map(|primitive| primitive.primitive)
        .collect()
}

pub fn render_document_targets(
    document: &ChemcoreDocument,
    node_ids: &BTreeSet<String>,
    bond_ids: &BTreeSet<String>,
    object_ids: &BTreeSet<String>,
) -> Vec<RenderPrimitive> {
    let mut out = Vec::new();
    for object in document.scene_objects() {
        if !object.visible {
            continue;
        }
        if object.object_type == "molecule" && (!node_ids.is_empty() || !bond_ids.is_empty()) {
            render_molecule_object_targets(&mut out, document, object, node_ids, bond_ids);
        } else if object_ids.contains(&object.id) {
            render_scene_object(&mut out, document, object);
        }
    }
    out
}

struct LayeredPrimitive {
    z_index: i32,
    sequence: usize,
    primitive: RenderPrimitive,
}

fn render_scene_objects_layered(
    out: &mut Vec<LayeredPrimitive>,
    sequence: &mut usize,
    document: &ChemcoreDocument,
    objects: &[SceneObject],
    parent_z_index: Option<i32>,
) {
    let mut visible_objects: Vec<&SceneObject> =
        objects.iter().filter(|object| object.visible).collect();
    visible_objects.sort_by(|a, b| a.z_index.cmp(&b.z_index).then_with(|| a.id.cmp(&b.id)));

    for object in visible_objects {
        render_scene_object_layered(out, sequence, document, object, parent_z_index);
    }
}

fn render_scene_object_layered(
    out: &mut Vec<LayeredPrimitive>,
    sequence: &mut usize,
    document: &ChemcoreDocument,
    object: &SceneObject,
    parent_z_index: Option<i32>,
) {
    let object_z_index = parent_z_index.unwrap_or(object.z_index);
    if object.object_type == "group" {
        let child_parent_z_index = if cdxml_group_preserves_child_z(object) {
            parent_z_index
        } else {
            Some(object_z_index)
        };
        render_scene_objects_layered(
            out,
            sequence,
            document,
            &object.children,
            child_parent_z_index,
        );
        return;
    }

    let mut rendered = Vec::new();
    render_scene_object(&mut rendered, document, object);
    for primitive in rendered {
        let z_index = if parent_z_index.is_none() {
            cdxml_primitive_z_index(document, object, &primitive).unwrap_or(object_z_index)
        } else {
            object_z_index
        };
        out.push(LayeredPrimitive {
            z_index,
            sequence: *sequence,
            primitive,
        });
        *sequence += 1;
    }
}

fn cdxml_group_preserves_child_z(object: &SceneObject) -> bool {
    object.object_type == "group"
        && object
            .meta
            .pointer("/import/cdxml/groupId")
            .and_then(JsonValue::as_str)
            .is_some()
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
        _ => {}
    }
}

fn cdxml_primitive_z_index(
    document: &ChemcoreDocument,
    object: &SceneObject,
    primitive: &RenderPrimitive,
) -> Option<i32> {
    if object.object_type != "molecule" {
        return None;
    }
    let fragment = molecule_fragment_for_object(document, object)?;
    if let Some(node_id) = primitive_node_id(primitive) {
        if let Some(z_index) = fragment
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .and_then(|node| cdxml_meta_z_index(&node.meta))
        {
            return Some(z_index);
        }
    }
    if let Some(bond_id) = primitive_bond_id(primitive) {
        if let Some(z_index) = fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id)
            .and_then(|bond| cdxml_meta_z_index(&bond.meta))
        {
            return Some(z_index);
        }
    }
    None
}

fn molecule_fragment_for_object<'a>(
    document: &'a ChemcoreDocument,
    object: &SceneObject,
) -> Option<&'a MoleculeFragment> {
    let resource_ref = object.payload.resource_ref.as_ref()?;
    let resource = document.resources.get(resource_ref)?;
    match &resource.data {
        ResourceData::Fragment(fragment)
            if resource.resource_type == "molecule_fragment2d"
                || resource.encoding == "chemcore.molecule.fragment2d" =>
        {
            Some(fragment)
        }
        _ => None,
    }
}

fn cdxml_meta_z_index(meta: &JsonValue) -> Option<i32> {
    meta.pointer("/import/cdxml/z")
        .and_then(JsonValue::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

fn primitive_node_id(primitive: &RenderPrimitive) -> Option<&str> {
    match primitive {
        RenderPrimitive::Circle { node_id, .. }
        | RenderPrimitive::Polygon { node_id, .. }
        | RenderPrimitive::Rect { node_id, .. }
        | RenderPrimitive::FilledPath { node_id, .. }
        | RenderPrimitive::Text { node_id, .. } => node_id.as_deref(),
        _ => None,
    }
}

fn primitive_bond_id(primitive: &RenderPrimitive) -> Option<&str> {
    match primitive {
        RenderPrimitive::Line { bond_id, .. }
        | RenderPrimitive::Polygon { bond_id, .. }
        | RenderPrimitive::Polyline { bond_id, .. }
        | RenderPrimitive::Path { bond_id, .. }
        | RenderPrimitive::FilledPath { bond_id, .. } => bond_id.as_deref(),
        _ => None,
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

    fn assert_gap_intervals(actual: &[(f64, f64)], expected: &[(f64, f64)]) {
        assert_eq!(actual.len(), expected.len(), "{actual:?}");
        for ((actual_start, actual_end), (expected_start, expected_end)) in
            actual.iter().zip(expected.iter())
        {
            assert!(
                (actual_start - expected_start).abs() < 0.01
                    && (actual_end - expected_end).abs() < 0.01,
                "actual={actual:?}, expected={expected:?}"
            );
        }
    }

    #[test]
    fn chemdraw_dashed_bond_gap_intervals_anchor_black_segments() {
        assert_gap_intervals(
            &chemdraw_dashed_bond_gap_intervals(14.4, 2.5, 2.5),
            &[(2.5, 5.95), (8.45, 11.9)],
        );
        assert_gap_intervals(
            &chemdraw_dashed_bond_gap_intervals(36.0, 2.5, 2.5),
            &[
                (2.5, 5.5833333333),
                (8.0833333333, 11.1666666667),
                (13.6666666667, 16.75),
                (19.25, 22.3333333333),
                (24.8333333333, 27.9166666667),
                (30.4166666667, 33.5),
            ],
        );
    }

    #[test]
    fn chemdraw_dashed_bond_gap_intervals_keep_short_bonds_single_segment() {
        assert_gap_intervals(
            &chemdraw_dashed_bond_gap_intervals(7.0, 2.5, 2.5),
            &[(2.5, 7.0)],
        );
        assert_gap_intervals(
            &chemdraw_dashed_bond_gap_intervals(8.0, 2.5, 2.5),
            &[(2.5, 5.5)],
        );
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
