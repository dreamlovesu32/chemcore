use crate::{
    legacy_mol::{parse_molblock, LegacyAtom, LegacyBond as LegacyMolBond, LegacyMol},
    point_at_distance_from_start, px_to_pt, rotate_point_around, Bond, BondLinePattern,
    BondLineWeight, ChemSemaDocument, DoubleBondPlacement, LabelRun, MoleculeFragment, Node,
    ObjectPayload, Point, ResourceData, SceneObject, Vector, DEFAULT_BOND_STROKE,
    DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT, EPSILON,
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
#[path = "render/geometry_constraints.rs"]
mod geometry_constraint_render;
#[path = "render/labels.rs"]
mod labels;
#[path = "render_legacy.rs"]
mod legacy_render;
#[path = "render_objects.rs"]
mod object_render;
#[path = "render_primitives.rs"]
mod primitives;
#[path = "render_spectrum.rs"]
mod spectrum_render;
#[path = "render/style_payload.rs"]
mod style_payload;

pub(crate) use bond_metrics::double_bond_center_distance_for_bond_weights;
use bond_render::{
    bond_endpoint_world, bond_label_clipped_body_geometry, compute_solid_wedge_points,
    imported_cdxml_dative_bond, render_fragment_bond,
};
pub use bounds::{render_primitive_bounds, render_primitives_bounds};
pub(crate) use bounds::{text_object_visual_bounds, text_primitive_visual_bounds};
use contact::{
    bond_ray_is_acute, build_main_bond_contact_kernel, build_main_bond_contact_kernel_for_nodes,
    center_double_skips_extension, main_bond_endpoint_geometry, main_contact_is_straight_through,
    main_contact_side, render_main_bond_contact_patches, MainBondContactKernel,
};
use geometry_constraint_render::render_geometry_constraint_object;
use legacy_render::render_legacy_molecule_object;
use object_render::{
    render_bracket_object, render_curve_object, render_line_object, render_molecule_object,
    render_molecule_object_targets, render_shape_object, render_stoichiometry_grid_object,
    render_table_object, render_text_object,
};
use primitives::{
    push_bond_polygon, push_label_knockout_polygon, push_line, push_node_polygon, push_path,
    push_polygon, push_polyline, push_text, push_text_for_node, push_text_rotated,
};
pub use primitives::{RenderPrimitive, RenderRole};
use spectrum_render::render_spectrum_object;

use bond_geometry::*;
use bond_metrics::*;
pub(crate) use bounds::{
    bracket_object_visual_bounds, curve_object_visual_bounds, fragment_bond_visual_bounds,
    line_object_visual_bounds, shape_object_visual_bounds,
};
use labels::{
    algebraic_body_segment_after_label_retreats, apply_label_endpoint_retreats,
    attached_label_glyph_anchor_world, body_segment_label_retreats,
    clip_body_segment_out_of_label_geometry, clip_wavy_body_segment_out_of_label_geometry,
    label_box_world, label_clip_polygons_world, label_clip_polygons_world_for_cardinal_strip,
    label_clip_polygons_world_for_segment, label_polygons_world, render_fragment_line,
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

fn bond_main_line_pattern(bond: &Bond) -> BondLinePattern {
    if bond
        .meta
        .pointer("/import/cdxml/collapsedFragmentDashDisplaysSolid")
        .and_then(JsonValue::as_bool)
        == Some(true)
    {
        BondLinePattern::Solid
    } else {
        bond.line_styles.main
    }
}
const HASH_TARGET_GAP_LENGTH: f64 = crate::HASH_TARGET_GAP_LENGTH_PT.value();
const SOLID_WEDGE_END_INSET: f64 = crate::SOLID_WEDGE_END_INSET_PT.value();
const CENTER_DOUBLE_NO_EXTENSION_ANGLE_DEGREES: f64 = 162.0;
const CHEMSEMA_INK: &str = "#000000";
const KNOCKOUT_FILL: &str = "#ffffff";
const BOLD_BOND_WIDTH: f64 = crate::BOLD_BOND_WIDTH_PT.value();
// Bracketed by real ChemDraw output at roughly 0.235-0.237 of the shorter
// usable contour. Use the conservative edge of that observed transition.
const CHEMDRAW_CONTACT_MITER_EXTENT_RATIO: f64 = 0.235;

#[derive(Debug, Clone, Copy)]
pub(crate) struct RectBox {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

impl RectBox {}

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

pub fn render_document(document: &ChemSemaDocument) -> Vec<RenderPrimitive> {
    let mut out = Vec::new();
    let mut sequence = 0;
    render_scene_objects_layered(&mut out, &mut sequence, document, &document.objects, None);
    out.sort_by(|a, b| {
        a.z_index
            .cmp(&b.z_index)
            .then_with(|| a.sequence.cmp(&b.sequence))
    });
    let primitives: Vec<_> = out
        .into_iter()
        .map(|primitive| primitive.primitive)
        .collect();
    insert_bond_margin_silhouettes(document, primitives)
}

pub fn render_document_targets(
    document: &ChemSemaDocument,
    node_ids: &BTreeSet<String>,
    bond_ids: &BTreeSet<String>,
    object_ids: &BTreeSet<String>,
) -> Vec<RenderPrimitive> {
    let mut out = Vec::new();
    let expanded_bond_ids = expand_target_bond_ids_with_document_crossings(document, bond_ids);
    for object in &document.objects {
        render_scene_object_targets(
            &mut out,
            document,
            object,
            node_ids,
            &expanded_bond_ids,
            object_ids,
            false,
        );
    }
    insert_bond_margin_silhouettes(document, out)
}

fn insert_bond_margin_silhouettes(
    document: &ChemSemaDocument,
    primitives: Vec<RenderPrimitive>,
) -> Vec<RenderPrimitive> {
    let bonds = collect_document_bond_render_info(document);
    let mut rendered_bonds: Vec<&DocumentBondRenderInfo> = Vec::new();
    let mut prepared_bond_keys = BTreeSet::new();
    let mut with_silhouettes = Vec::with_capacity(primitives.len() * 2);
    for primitive in primitives {
        if primitive.role() == RenderRole::DocumentBond {
            if let (Some(object_id), Some(bond_id)) =
                (primitive.object_id(), primitive_bond_id(&primitive))
            {
                let key = (Some(object_id.to_string()), bond_id.to_string());
                if prepared_bond_keys.insert(key.clone()) {
                    if let Some(info) = bonds.get(&key) {
                        let crossing_primitives = prepare_bond_crossings(
                            document,
                            &mut with_silhouettes,
                            &rendered_bonds,
                            info,
                        );
                        with_silhouettes.extend(crossing_primitives);
                        rendered_bonds.push(info);
                    }
                }
            }
        }
        with_silhouettes.push(primitive);
    }
    with_silhouettes
}

#[derive(Debug)]
struct DocumentBondRenderInfo {
    object_id: Option<String>,
    id: String,
    begin: String,
    end: String,
    start: Point,
    end_point: Point,
    margin_width: f64,
    crossing_envelope: BondCrossingEnvelope,
    explicit_crossings: Option<BTreeSet<String>>,
    can_split_crossings: bool,
}

#[derive(Debug, Clone, Copy)]
struct CrossingOffsets {
    min: f64,
    max: f64,
}

#[derive(Debug, Clone, Copy)]
struct BondCrossingEnvelope {
    silhouette_start: CrossingOffsets,
    silhouette_end: CrossingOffsets,
    clearance_start: CrossingOffsets,
    clearance_end: CrossingOffsets,
}

fn collect_document_bond_render_info(
    document: &ChemSemaDocument,
) -> BTreeMap<(Option<String>, String), DocumentBondRenderInfo> {
    let mut bonds = BTreeMap::new();
    collect_object_bond_render_info(document, &document.objects, &mut bonds);
    bonds
}

fn collect_object_bond_render_info(
    document: &ChemSemaDocument,
    objects: &[SceneObject],
    bonds: &mut BTreeMap<(Option<String>, String), DocumentBondRenderInfo>,
) {
    for object in objects {
        if let Some(fragment) = molecule_fragment_for_object(document, object) {
            let node_map: BTreeMap<&str, &Node> = fragment
                .nodes
                .iter()
                .map(|node| (node.id.as_str(), node))
                .collect();
            for bond in &fragment.bonds {
                let (Some(begin), Some(end)) = (
                    node_map.get(bond.begin.as_str()),
                    node_map.get(bond.end.as_str()),
                ) else {
                    continue;
                };
                let start = bond_endpoint_world(object, begin, bond, "begin");
                let end_point = bond_endpoint_world(object, end, bond, "end");
                if start.distance(end_point) <= EPSILON {
                    continue;
                }
                let stroke_width = bond_stroke_width(document, object, bond);
                let object_id = Some(object.id.clone());
                let crossing_envelope =
                    document_bond_crossing_envelope(bond, start, end_point, stroke_width);
                bonds.insert(
                    (object_id.clone(), bond.id.clone()),
                    DocumentBondRenderInfo {
                        object_id,
                        id: bond.id.clone(),
                        begin: bond.begin.clone(),
                        end: bond.end.clone(),
                        start,
                        end_point,
                        margin_width: document_margin_width_for_bond(document, bond, stroke_width),
                        crossing_envelope,
                        explicit_crossings: imported_cdxml_crossing_bonds(bond),
                        can_split_crossings: bond.order == 1
                            && bond_stereo_kind(bond).is_none()
                            && bond_main_line_pattern(bond) == BondLinePattern::Solid
                            && bond.line_weights.main == BondLineWeight::Normal
                            && !imported_cdxml_dative_bond(bond),
                    },
                );
            }
        }
        collect_object_bond_render_info(document, &object.children, bonds);
    }
}

fn prepare_bond_crossings(
    document: &ChemSemaDocument,
    primitives: &mut Vec<RenderPrimitive>,
    rendered_bonds: &[&DocumentBondRenderInfo],
    over_bond: &DocumentBondRenderInfo,
) -> Vec<RenderPrimitive> {
    if over_bond.margin_width <= EPSILON {
        return Vec::new();
    }
    rendered_bonds
        .iter()
        .filter_map(|under_bond| {
            if !bonds_are_crossing_candidates(under_bond, over_bond) {
                return None;
            }
            if let Some(crossing) = bond_crossing_point_for_margin(under_bond, over_bond) {
                if under_bond.can_split_crossings
                    && over_bond.can_split_crossings
                    && split_rendered_bond_at_crossing(primitives, under_bond, over_bond, crossing)
                {
                    None
                } else {
                    local_bond_crossing_knockout(
                        under_bond,
                        over_bond,
                        crossing,
                        &document.document.page.background,
                    )
                }
            } else {
                near_endpoint_bond_crossing_knockout(
                    under_bond,
                    over_bond,
                    &document.document.page.background,
                )
            }
        })
        .collect()
}

fn split_rendered_bond_at_crossing(
    primitives: &mut Vec<RenderPrimitive>,
    under_bond: &DocumentBondRenderInfo,
    over_bond: &DocumentBondRenderInfo,
    crossing: Point,
) -> bool {
    let matches_under_bond = |primitive: &RenderPrimitive| {
        primitive.role() == RenderRole::DocumentBond
            && primitive.object_id() == under_bond.object_id.as_deref()
            && primitive_bond_id(primitive) == Some(under_bond.id.as_str())
    };
    let matching = primitives
        .iter()
        .filter(|primitive| matches_under_bond(primitive))
        .collect::<Vec<_>>();
    if matching.is_empty()
        || matching
            .iter()
            .any(|primitive| !matches!(primitive, RenderPrimitive::Polygon { .. }))
    {
        return false;
    }

    let over_axis = Vector::new(
        over_bond.end_point.x - over_bond.start.x,
        over_bond.end_point.y - over_bond.start.y,
    )
    .normalized();
    let over_normal = Vector::new(-over_axis.y, over_axis.x);
    let offsets = over_bond.clearance_offsets_at(crossing);
    let lower_boundary = offsets.min - over_bond.margin_width;
    let upper_boundary = offsets.max + over_bond.margin_width;
    let previous = std::mem::take(primitives);
    for primitive in previous {
        if !matches_under_bond(&primitive) {
            primitives.push(primitive);
            continue;
        }
        let RenderPrimitive::Polygon {
            role,
            object_id,
            node_id,
            bond_id,
            points,
            fill,
            stroke,
            stroke_width,
        } = primitive
        else {
            unreachable!("ordinary split crossing was prevalidated as polygon geometry");
        };
        let before = clip_polygon_to_crossing_half_plane(
            &points,
            crossing,
            over_normal,
            lower_boundary,
            false,
        );
        let after = clip_polygon_to_crossing_half_plane(
            &points,
            crossing,
            over_normal,
            upper_boundary,
            true,
        );
        for clipped in [before, after] {
            if clipped.len() < 3 || polygon_area_signed(&clipped).abs() <= 1.0e-4 {
                continue;
            }
            primitives.push(RenderPrimitive::Polygon {
                role,
                object_id: object_id.clone(),
                node_id: node_id.clone(),
                bond_id: bond_id.clone(),
                points: clipped,
                fill: fill.clone(),
                stroke: stroke.clone(),
                stroke_width,
            });
        }
    }
    true
}

fn clip_polygon_to_crossing_half_plane(
    polygon: &[Point],
    origin: Point,
    normal: Vector,
    boundary: f64,
    keep_greater: bool,
) -> Vec<Point> {
    if polygon.len() < 3 {
        return Vec::new();
    }
    let signed_offset =
        |point: Point| (point.x - origin.x) * normal.x + (point.y - origin.y) * normal.y;
    let is_inside = |offset: f64| {
        if keep_greater {
            offset >= boundary - EPSILON
        } else {
            offset <= boundary + EPSILON
        }
    };
    let mut output = Vec::new();
    let mut previous = *polygon.last().expect("non-empty polygon");
    let mut previous_offset = signed_offset(previous);
    let mut previous_inside = is_inside(previous_offset);
    for &current in polygon {
        let current_offset = signed_offset(current);
        let current_inside = is_inside(current_offset);
        if current_inside != previous_inside {
            let denominator = current_offset - previous_offset;
            if denominator.abs() > EPSILON {
                let t = ((boundary - previous_offset) / denominator).clamp(0.0, 1.0);
                output.push(Point::new(
                    previous.x + (current.x - previous.x) * t,
                    previous.y + (current.y - previous.y) * t,
                ));
            }
        }
        if current_inside {
            output.push(current);
        }
        previous = current;
        previous_offset = current_offset;
        previous_inside = current_inside;
    }
    compact_polygon_points(output)
}

fn bonds_are_crossing_candidates(
    under_bond: &DocumentBondRenderInfo,
    over_bond: &DocumentBondRenderInfo,
) -> bool {
    if under_bond.object_id == over_bond.object_id
        && (under_bond.begin == over_bond.begin
            || under_bond.begin == over_bond.end
            || under_bond.end == over_bond.begin
            || under_bond.end == over_bond.end)
    {
        return false;
    }
    if (under_bond.explicit_crossings.is_some() || over_bond.explicit_crossings.is_some())
        && !under_bond
            .explicit_crossings
            .as_ref()
            .is_some_and(|ids| ids.contains(&over_bond.id))
        && !over_bond
            .explicit_crossings
            .as_ref()
            .is_some_and(|ids| ids.contains(&under_bond.id))
    {
        return false;
    }
    let under_vector = Vector::new(
        under_bond.end_point.x - under_bond.start.x,
        under_bond.end_point.y - under_bond.start.y,
    );
    let over_vector = Vector::new(
        over_bond.end_point.x - over_bond.start.x,
        over_bond.end_point.y - over_bond.start.y,
    );
    if under_vector.length() <= EPSILON || over_vector.length() <= EPSILON {
        return false;
    }
    let crossing_sin =
        target_render_vector_cross(under_vector.normalized(), over_vector.normalized()).abs();
    crossing_sin > 0.1
}

fn bond_crossing_point_for_margin(
    under_bond: &DocumentBondRenderInfo,
    over_bond: &DocumentBondRenderInfo,
) -> Option<Point> {
    target_render_segment_intersection(
        under_bond.start,
        under_bond.end_point,
        over_bond.start,
        over_bond.end_point,
    )
}

fn local_bond_crossing_knockout(
    under_bond: &DocumentBondRenderInfo,
    over_bond: &DocumentBondRenderInfo,
    crossing: Point,
    background: &str,
) -> Option<RenderPrimitive> {
    let under_axis = Vector::new(
        under_bond.end_point.x - under_bond.start.x,
        under_bond.end_point.y - under_bond.start.y,
    )
    .normalized();
    let over_axis = Vector::new(
        over_bond.end_point.x - over_bond.start.x,
        over_bond.end_point.y - over_bond.start.y,
    )
    .normalized();
    let under_normal = Vector::new(-under_axis.y, under_axis.x);
    let over_normal = Vector::new(-over_axis.y, over_axis.x);
    let determinant = target_render_vector_cross(under_normal, over_normal);
    if determinant.abs() <= 0.1 {
        return None;
    }

    // The two over-normal edges are the visible ChemDraw cut boundaries. The
    // under-normal edges merely keep the background patch local to the lower
    // bond and receive a tiny AA allowance without widening the cut itself.
    let under_offsets = under_bond.silhouette_offsets_at(crossing);
    let over_offsets = over_bond.clearance_offsets_at(crossing);
    let under_min = under_offsets.min - 0.05;
    let under_max = under_offsets.max + 0.05;
    let over_min = over_offsets.min - over_bond.margin_width;
    let over_max = over_offsets.max + over_bond.margin_width;
    let local_corner = |under_offset: f64, over_offset: f64| {
        let x = (under_offset * over_normal.y - under_normal.y * over_offset) / determinant;
        let y = (under_normal.x * over_offset - under_offset * over_normal.x) / determinant;
        Point::new(crossing.x + x, crossing.y + y)
    };
    let points = vec![
        local_corner(under_max, over_max),
        local_corner(under_min, over_max),
        local_corner(under_min, over_min),
        local_corner(under_max, over_min),
    ];
    Some(RenderPrimitive::Polygon {
        role: RenderRole::DocumentKnockout,
        object_id: over_bond.object_id.clone(),
        node_id: None,
        bond_id: Some(over_bond.id.clone()),
        points,
        fill: background.to_string(),
        stroke: background.to_string(),
        stroke_width: 0.0,
    })
}

fn near_endpoint_bond_crossing_knockout(
    under_bond: &DocumentBondRenderInfo,
    over_bond: &DocumentBondRenderInfo,
    background: &str,
) -> Option<RenderPrimitive> {
    // ChemDraw applies a finite margin silhouette to the upper bond, including
    // a margin-width extension beyond both butt caps. This can notch or shorten
    // a lower bond even when the two centerline segments narrowly miss. It is
    // observable in warning-heavy CDXML where CrossingBonds records a contact
    // whose infinite-line intersection falls just outside one segment.
    let under_polygon = crossing_strip_polygon(
        under_bond,
        under_bond.crossing_envelope.silhouette_start,
        under_bond.crossing_envelope.silhouette_end,
        0.05,
        0.0,
    )?;
    let over_polygon = crossing_strip_polygon(
        over_bond,
        over_bond.crossing_envelope.clearance_start,
        over_bond.crossing_envelope.clearance_end,
        over_bond.margin_width,
        over_bond.margin_width,
    )?;
    let points = intersect_convex_polygons(&under_polygon, &over_polygon);
    if points.len() < 3 || polygon_area_signed(&points).abs() <= 1.0e-4 {
        return None;
    }
    Some(RenderPrimitive::Polygon {
        role: RenderRole::DocumentKnockout,
        object_id: over_bond.object_id.clone(),
        node_id: None,
        bond_id: Some(over_bond.id.clone()),
        points,
        fill: background.to_string(),
        stroke: background.to_string(),
        stroke_width: 0.0,
    })
}

fn crossing_strip_polygon(
    bond: &DocumentBondRenderInfo,
    start_offsets: CrossingOffsets,
    end_offsets: CrossingOffsets,
    lateral_margin: f64,
    longitudinal_margin: f64,
) -> Option<Vec<Point>> {
    crossing_strip_polygon_for_segment(
        bond.start,
        bond.end_point,
        start_offsets,
        end_offsets,
        lateral_margin,
        longitudinal_margin,
    )
}

fn crossing_strip_polygon_for_segment(
    start_point: Point,
    end_point: Point,
    start_offsets: CrossingOffsets,
    end_offsets: CrossingOffsets,
    lateral_margin: f64,
    longitudinal_margin: f64,
) -> Option<Vec<Point>> {
    let axis = Vector::new(end_point.x - start_point.x, end_point.y - start_point.y);
    if axis.length() <= EPSILON {
        return None;
    }
    let unit = axis.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let start = start_point.translated(unit.scaled(-longitudinal_margin));
    let end = end_point.translated(unit.scaled(longitudinal_margin));
    Some(vec![
        start.translated(normal.scaled(start_offsets.max + lateral_margin)),
        end.translated(normal.scaled(end_offsets.max + lateral_margin)),
        end.translated(normal.scaled(end_offsets.min - lateral_margin)),
        start.translated(normal.scaled(start_offsets.min - lateral_margin)),
    ])
}

fn intersect_convex_polygons(subject: &[Point], clip: &[Point]) -> Vec<Point> {
    let mut output = subject.to_vec();
    if output.len() < 3 || clip.len() < 3 {
        return Vec::new();
    }
    let orientation = if polygon_area_signed(clip) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    for index in 0..clip.len() {
        let edge_start = clip[index];
        let edge_end = clip[(index + 1) % clip.len()];
        let edge = Vector::new(edge_end.x - edge_start.x, edge_end.y - edge_start.y);
        let input = std::mem::take(&mut output);
        if input.is_empty() {
            break;
        }
        let inside = |point: Point| {
            orientation
                * target_render_vector_cross(
                    edge,
                    Vector::new(point.x - edge_start.x, point.y - edge_start.y),
                )
                >= -1.0e-8
        };
        let intersection = |first: Point, second: Point| {
            line_intersection(
                first,
                Vector::new(second.x - first.x, second.y - first.y),
                edge_start,
                edge,
            )
        };
        let mut previous = *input.last().expect("non-empty polygon");
        let mut previous_inside = inside(previous);
        for current in input {
            let current_inside = inside(current);
            if current_inside != previous_inside {
                if let Some(point) = intersection(previous, current) {
                    output.push(point);
                }
            }
            if current_inside {
                output.push(current);
            }
            previous = current;
            previous_inside = current_inside;
        }
    }
    compact_polygon_points(output)
}

fn imported_cdxml_crossing_bonds(bond: &Bond) -> Option<BTreeSet<String>> {
    bond.meta
        .pointer("/import/cdxml/crossingBonds")
        .and_then(JsonValue::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(JsonValue::as_str)
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(str::to_string)
                .collect()
        })
}

impl DocumentBondRenderInfo {
    fn offsets_at(
        &self,
        point: Point,
        start: CrossingOffsets,
        end: CrossingOffsets,
    ) -> CrossingOffsets {
        let axis = Vector::new(
            self.end_point.x - self.start.x,
            self.end_point.y - self.start.y,
        );
        let length = axis.length();
        if length <= EPSILON {
            return CrossingOffsets {
                min: start.min.min(end.min),
                max: start.max.max(end.max),
            };
        }
        let unit = axis.normalized();
        let offset = Vector::new(point.x - self.start.x, point.y - self.start.y);
        let fraction = (vector_dot(offset, unit) / length).clamp(0.0, 1.0);
        CrossingOffsets {
            min: start.min + (end.min - start.min) * fraction,
            max: start.max + (end.max - start.max) * fraction,
        }
    }

    fn silhouette_offsets_at(&self, point: Point) -> CrossingOffsets {
        self.offsets_at(
            point,
            self.crossing_envelope.silhouette_start,
            self.crossing_envelope.silhouette_end,
        )
    }

    fn clearance_offsets_at(&self, point: Point) -> CrossingOffsets {
        self.offsets_at(
            point,
            self.crossing_envelope.clearance_start,
            self.crossing_envelope.clearance_end,
        )
    }
}

fn document_bond_crossing_envelope(
    bond: &Bond,
    start: Point,
    end: Point,
    stroke_width: f64,
) -> BondCrossingEnvelope {
    let main_half_width =
        line_weight_stroke_width_for_bond(bond, stroke_width, bond.line_weights.main) * 0.5;
    let symmetric = |half_width: f64| CrossingOffsets {
        min: -half_width,
        max: half_width,
    };
    let constant = |silhouette: CrossingOffsets, clearance: CrossingOffsets| BondCrossingEnvelope {
        silhouette_start: silhouette,
        silhouette_end: silhouette,
        clearance_start: clearance,
        clearance_end: clearance,
    };

    if let Some(stereo) = bond_stereo_kind(bond) {
        let tip = solid_wedge_tip_half_width(stroke_width);
        let wide = solid_wedge_half_width_for_bond(bond, stroke_width);
        let (start_half_width, end_half_width) = match stereo {
            BondStereoKind::SolidWedgeBegin
            | BondStereoKind::HashedWedgeBegin
            | BondStereoKind::HollowWedgeBegin => (wide, tip),
            BondStereoKind::SolidWedgeEnd
            | BondStereoKind::HashedWedgeEnd
            | BondStereoKind::HollowWedgeEnd => (tip, wide),
        };
        let start_offsets = symmetric(start_half_width);
        let end_offsets = symmetric(end_half_width);
        return BondCrossingEnvelope {
            silhouette_start: start_offsets,
            silhouette_end: end_offsets,
            clearance_start: start_offsets,
            clearance_end: end_offsets,
        };
    }
    if bond.order == 1 && bond_main_line_pattern(bond) == BondLinePattern::Wavy {
        let amplitude = wavy_bond_amplitude_for_bond(bond, stroke_width);
        return constant(symmetric(amplitude + main_half_width), symmetric(amplitude));
    }
    if bond.order == 2 {
        let (centerlines, silhouette) = match bond.double.as_ref().map(|double| double.placement) {
            Some(DoubleBondPlacement::Left) => {
                let outer_half_width =
                    line_weight_stroke_width_for_bond(bond, stroke_width, bond.line_weights.left)
                        * 0.5;
                let center_distance = double_bond_center_distance_for_bond_weights(
                    bond,
                    start,
                    end,
                    stroke_width,
                    bond.line_weights.main,
                    bond.line_weights.left,
                );
                (
                    CrossingOffsets {
                        min: 0.0,
                        max: center_distance,
                    },
                    CrossingOffsets {
                        min: -main_half_width,
                        max: center_distance + outer_half_width,
                    },
                )
            }
            Some(DoubleBondPlacement::Right) => {
                let outer_half_width =
                    line_weight_stroke_width_for_bond(bond, stroke_width, bond.line_weights.right)
                        * 0.5;
                let center_distance = double_bond_center_distance_for_bond_weights(
                    bond,
                    start,
                    end,
                    stroke_width,
                    bond.line_weights.main,
                    bond.line_weights.right,
                );
                (
                    CrossingOffsets {
                        min: -center_distance,
                        max: 0.0,
                    },
                    CrossingOffsets {
                        min: -center_distance - outer_half_width,
                        max: main_half_width,
                    },
                )
            }
            _ => {
                let left_half_width =
                    line_weight_stroke_width_for_bond(bond, stroke_width, bond.line_weights.left)
                        * 0.5;
                let right_half_width =
                    line_weight_stroke_width_for_bond(bond, stroke_width, bond.line_weights.right)
                        * 0.5;
                let center_distance = double_bond_center_distance_for_bond_weights(
                    bond,
                    start,
                    end,
                    stroke_width,
                    bond.line_weights.left,
                    bond.line_weights.right,
                );
                (
                    CrossingOffsets {
                        min: -center_distance * 0.5,
                        max: center_distance * 0.5,
                    },
                    CrossingOffsets {
                        min: -center_distance * 0.5 - left_half_width,
                        max: center_distance * 0.5 + right_half_width,
                    },
                )
            }
        };
        return constant(silhouette, centerlines);
    }
    if bond.order >= 3 {
        let offset = triple_bond_offset_distance_for_bond(bond, start, end, stroke_width);
        let left_half_width =
            line_weight_stroke_width_for_bond(bond, stroke_width, bond.line_weights.left) * 0.5;
        let right_half_width =
            line_weight_stroke_width_for_bond(bond, stroke_width, bond.line_weights.right) * 0.5;
        return constant(
            CrossingOffsets {
                min: (-offset - right_half_width).min(-main_half_width),
                max: (offset + left_half_width).max(main_half_width),
            },
            symmetric(offset),
        );
    }
    constant(symmetric(main_half_width), symmetric(main_half_width))
}

#[derive(Debug, Clone)]
struct TargetRenderBondSegment {
    id: String,
    begin: String,
    end: String,
    start: Point,
    end_point: Point,
    margin_width: f64,
    crossing_envelope: BondCrossingEnvelope,
    explicit_crossings: Option<BTreeSet<String>>,
}

fn expand_target_bond_ids_with_document_crossings(
    document: &ChemSemaDocument,
    bond_ids: &BTreeSet<String>,
) -> BTreeSet<String> {
    if bond_ids.is_empty() {
        return BTreeSet::new();
    }
    let segments = collect_document_target_bond_segments(document);
    if segments.is_empty() {
        return bond_ids.clone();
    }
    let mut expanded = bond_ids.clone();
    let target_indices: Vec<usize> = segments
        .iter()
        .enumerate()
        .filter_map(|(index, segment)| bond_ids.contains(&segment.id).then_some(index))
        .collect();
    for target_index in target_indices {
        for other_index in 0..segments.len() {
            if target_index == other_index {
                continue;
            }
            let (under, over) = if target_index < other_index {
                (&segments[target_index], &segments[other_index])
            } else {
                (&segments[other_index], &segments[target_index])
            };
            if target_render_bond_segments_cross(under, over) {
                expanded.insert(over.id.clone());
            }
        }
    }
    expanded
}

fn collect_document_target_bond_segments(
    document: &ChemSemaDocument,
) -> Vec<TargetRenderBondSegment> {
    let mut segments = Vec::new();
    for entry in document.editable_fragments() {
        let node_map: BTreeMap<&str, &Node> = entry
            .fragment
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();
        for bond in &entry.fragment.bonds {
            let (Some(begin), Some(end)) = (
                node_map.get(bond.begin.as_str()),
                node_map.get(bond.end.as_str()),
            ) else {
                continue;
            };
            let start = bond_endpoint_world(entry.object, begin, bond, "begin");
            let end_point = bond_endpoint_world(entry.object, end, bond, "end");
            if start.distance(end_point) <= EPSILON {
                continue;
            }
            let stroke_width = bond_stroke_width(document, entry.object, bond);
            segments.push(TargetRenderBondSegment {
                id: bond.id.clone(),
                begin: bond.begin.clone(),
                end: bond.end.clone(),
                start,
                end_point,
                margin_width: document_margin_width_for_bond(document, bond, stroke_width),
                crossing_envelope: document_bond_crossing_envelope(
                    bond,
                    start,
                    end_point,
                    stroke_width,
                ),
                explicit_crossings: imported_cdxml_crossing_bonds(bond),
            });
        }
    }
    segments
}

fn target_render_bond_segments_cross(
    first: &TargetRenderBondSegment,
    second: &TargetRenderBondSegment,
) -> bool {
    if first.begin == second.begin
        || first.begin == second.end
        || first.end == second.begin
        || first.end == second.end
    {
        return false;
    }
    if (first.explicit_crossings.is_some() || second.explicit_crossings.is_some())
        && !first
            .explicit_crossings
            .as_ref()
            .is_some_and(|ids| ids.contains(&second.id))
        && !second
            .explicit_crossings
            .as_ref()
            .is_some_and(|ids| ids.contains(&first.id))
    {
        return false;
    }
    let first_vector = Vector::new(
        first.end_point.x - first.start.x,
        first.end_point.y - first.start.y,
    );
    let second_vector = Vector::new(
        second.end_point.x - second.start.x,
        second.end_point.y - second.start.y,
    );
    if first_vector.length() <= EPSILON || second_vector.length() <= EPSILON {
        return false;
    }
    let crossing_sin =
        target_render_vector_cross(first_vector.normalized(), second_vector.normalized()).abs();
    if crossing_sin <= 0.1 {
        return false;
    }
    if target_render_segment_intersection(
        first.start,
        first.end_point,
        second.start,
        second.end_point,
    )
    .is_some()
    {
        return true;
    }
    let Some(first_polygon) = crossing_strip_polygon_for_segment(
        first.start,
        first.end_point,
        first.crossing_envelope.silhouette_start,
        first.crossing_envelope.silhouette_end,
        0.05,
        0.0,
    ) else {
        return false;
    };
    let Some(second_polygon) = crossing_strip_polygon_for_segment(
        second.start,
        second.end_point,
        second.crossing_envelope.clearance_start,
        second.crossing_envelope.clearance_end,
        second.margin_width,
        second.margin_width,
    ) else {
        return false;
    };
    let overlap = intersect_convex_polygons(&first_polygon, &second_polygon);
    overlap.len() >= 3 && polygon_area_signed(&overlap).abs() > 1.0e-4
}

fn target_render_segment_intersection(a1: Point, a2: Point, b1: Point, b2: Point) -> Option<Point> {
    let a = Vector::new(a2.x - a1.x, a2.y - a1.y);
    let b = Vector::new(b2.x - b1.x, b2.y - b1.y);
    let denom = target_render_vector_cross(a, b);
    if denom.abs() <= EPSILON {
        return None;
    }
    let offset = Vector::new(b1.x - a1.x, b1.y - a1.y);
    let t = target_render_vector_cross(offset, b) / denom;
    let u = target_render_vector_cross(offset, a) / denom;
    if t <= 1.0e-6 || t >= 1.0 - 1.0e-6 || u <= 1.0e-6 || u >= 1.0 - 1.0e-6 {
        return None;
    }
    Some(Point::new(a1.x + a.x * t, a1.y + a.y * t))
}

fn target_render_vector_cross(first: Vector, second: Vector) -> f64 {
    first.x * second.y - first.y * second.x
}

fn render_scene_object_targets(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    node_ids: &BTreeSet<String>,
    bond_ids: &BTreeSet<String>,
    object_ids: &BTreeSet<String>,
    ancestor_targeted: bool,
) {
    if !object.visible {
        return;
    }
    let directly_targeted = object_ids.contains(&object.id);
    let object_targeted = directly_targeted || ancestor_targeted;
    if object.object_type == "molecule" && object_targeted {
        render_scene_object(out, document, object);
    } else if object.object_type == "molecule" && (!node_ids.is_empty() || !bond_ids.is_empty()) {
        render_molecule_object_targets(out, document, object, node_ids, bond_ids);
    } else if object_targeted && object.object_type != "group" {
        render_scene_object(out, document, object);
    }
    for child in &object.children {
        render_scene_object_targets(
            out,
            document,
            child,
            node_ids,
            bond_ids,
            object_ids,
            object_targeted,
        );
    }
}

struct LayeredPrimitive {
    z_index: i32,
    sequence: usize,
    primitive: RenderPrimitive,
}

fn render_scene_objects_layered(
    out: &mut Vec<LayeredPrimitive>,
    sequence: &mut usize,
    document: &ChemSemaDocument,
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
    document: &ChemSemaDocument,
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
    document: &ChemSemaDocument,
    object: &SceneObject,
) {
    match object.kind() {
        crate::SceneObjectKind::Molecule => render_molecule_object(out, document, object),
        crate::SceneObjectKind::Line => render_line_object(out, document, object),
        crate::SceneObjectKind::Curve => render_curve_object(out, document, object),
        crate::SceneObjectKind::Text => render_text_object(out, document, object),
        crate::SceneObjectKind::Shape => render_shape_object(out, document, object),
        crate::SceneObjectKind::Table => render_table_object(out, object),
        crate::SceneObjectKind::StoichiometryGrid => render_stoichiometry_grid_object(out, object),
        crate::SceneObjectKind::Image => render_image_object(out, document, object),
        crate::SceneObjectKind::Spectrum => render_spectrum_object(out, document, object),
        crate::SceneObjectKind::Bracket | crate::SceneObjectKind::Symbol => {
            render_bracket_object(out, document, object)
        }
        crate::SceneObjectKind::Geometry | crate::SceneObjectKind::Constraint => {
            render_geometry_constraint_object(out, document, object)
        }
        crate::SceneObjectKind::Group => {}
    }
}

fn render_image_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
) {
    let Some(resource_ref) = object.payload.resource_ref.as_ref() else {
        return;
    };
    let Some(resource) = document.resources.get(resource_ref) else {
        return;
    };
    let Some([local_x, local_y, local_width, local_height]) = object.payload.bbox else {
        return;
    };
    let scale_x = object.transform.scale[0];
    let scale_y = object.transform.scale[1];
    let x = object.transform.translate[0] + local_x * scale_x;
    let y = object.transform.translate[1] + local_y * scale_y;
    let width = local_width * scale_x;
    let height = local_height * scale_y;
    if !x.is_finite()
        || !y.is_finite()
        || !width.is_finite()
        || !height.is_finite()
        || width.abs() <= crate::EPSILON
        || height.abs() <= crate::EPSILON
    {
        return;
    }
    let opacity = object
        .payload
        .extra
        .get("opacity")
        .and_then(JsonValue::as_f64)
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    let preserve_aspect_ratio =
        object.payload.extra.get("fit").and_then(JsonValue::as_str) == Some("contain");
    let center = Point::new(x + width * 0.5, y + height * 0.5);
    let image = resource.display_image();
    if image.is_none() {
        let radians = object.transform.rotate.to_radians();
        let cos = radians.cos();
        let sin = radians.sin();
        let rotate_point = |point: Point| {
            let dx = point.x - center.x;
            let dy = point.y - center.y;
            Point::new(
                center.x + dx * cos - dy * sin,
                center.y + dx * sin + dy * cos,
            )
        };
        let points = [
            Point::new(x, y),
            Point::new(x + width, y),
            Point::new(x + width, y + height),
            Point::new(x, y + height),
        ]
        .into_iter()
        .map(rotate_point)
        .collect();
        out.push(RenderPrimitive::Polygon {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object.id.clone()),
            node_id: None,
            bond_id: None,
            points,
            fill: "#f3f4f6".to_string(),
            stroke: "#6b7280".to_string(),
            stroke_width: 0.75,
        });
        let embedded = resource.data.as_embedded_object();
        let format = embedded
            .as_ref()
            .map(|value| value.format.as_str())
            .unwrap_or("object");
        let status = embedded
            .as_ref()
            .map(|value| value.preview_status.as_str().to_string())
            .unwrap_or_else(|| "invalid-resource".to_string());
        out.push(RenderPrimitive::Text {
            role: RenderRole::DocumentText,
            object_id: Some(object.id.clone()),
            node_id: None,
            x: center.x,
            y: center.y,
            baseline_offset: None,
            dominant_baseline: Some("central".to_string()),
            text: format!("Embedded {format} ({status})"),
            font_size: 8.0_f64.min(height.abs() * 0.3).max(3.0),
            font_family: Some("Arial".to_string()),
            fill: Some("#4b5563".to_string()),
            text_anchor: Some("middle".to_string()),
            line_height: None,
            preserve_lines: false,
            box_width: Some(width.abs()),
            runs: Vec::new(),
            rotate: object.transform.rotate,
            rotate_center: Some(center),
        });
        return;
    }
    let image = image.expect("image preview was checked above");
    let Ok(source_crop) = object.payload.image_crop() else {
        return;
    };
    if source_crop.is_some_and(|crop| {
        crop.validate(image.pixel_width, image.pixel_height)
            .is_err()
    }) {
        return;
    }
    out.push(RenderPrimitive::Image {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object.id.clone()),
        x: crate::round2(x),
        y: crate::round2(y),
        width: crate::round2(width),
        height: crate::round2(height),
        href: format!("data:{};base64,{}", image.mime_type, image.data_base64),
        source_crop,
        source_width: image.pixel_width,
        source_height: image.pixel_height,
        opacity,
        preserve_aspect_ratio,
        rotate: crate::round2(object.transform.rotate),
        rotate_center: Some(center),
    });
}

fn cdxml_primitive_z_index(
    document: &ChemSemaDocument,
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
    document: &'a ChemSemaDocument,
    object: &SceneObject,
) -> Option<&'a MoleculeFragment> {
    let resource_ref = object.payload.resource_ref.as_ref()?;
    let resource = document.resources.get(resource_ref)?;
    match &resource.data {
        ResourceData::Fragment(fragment)
            if resource.resource_type == "molecule_fragment2d"
                || resource.encoding == "chemsema.molecule.fragment2d" =>
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
            highlight_color: None,
            properties: Default::default(),
            double: None,
            stereo: None,
            stroke_width: VIEWER_BOND_STROKE,
            stroke: None,
            bold_width: None,
            wedge_width: None,
            label_clip_margin: None,
            hash_spacing: None,
            bond_spacing: None,
            bond_spacing_absolute: None,
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
            &[(2.88, 5.76), (8.64, 11.52)],
        );
        assert_gap_intervals(
            &chemdraw_dashed_bond_gap_intervals(36.0, 2.5, 2.5),
            &[
                (2.4, 4.8),
                (7.2, 9.6),
                (12.0, 14.4),
                (16.8, 19.2),
                (21.6, 24.0),
                (26.4, 28.8),
                (31.2, 33.6),
            ],
        );
    }

    #[test]
    fn chemdraw_dashed_bond_gap_intervals_switch_at_twice_hash_spacing() {
        assert_gap_intervals(&chemdraw_dashed_bond_gap_intervals(4.99, 2.5, 2.5), &[]);
        assert_gap_intervals(
            &chemdraw_dashed_bond_gap_intervals(5.0, 2.5, 2.5),
            &[(5.0 / 3.0, 10.0 / 3.0)],
        );
        assert_gap_intervals(
            &chemdraw_dashed_bond_gap_intervals(7.0, 2.5, 2.5),
            &[(7.0 / 3.0, 14.0 / 3.0)],
        );
        assert_gap_intervals(
            &chemdraw_dashed_bond_gap_intervals(8.0, 2.5, 2.5),
            &[(8.0 / 3.0, 16.0 / 3.0)],
        );
        assert_gap_intervals(
            &chemdraw_dashed_bond_gap_intervals(9.99, 2.5, 2.5),
            &[(9.99 / 3.0, 19.98 / 3.0)],
        );
        assert_gap_intervals(
            &chemdraw_dashed_bond_gap_intervals(10.0, 2.5, 2.5),
            &[(2.0, 4.0), (6.0, 8.0)],
        );
        assert_gap_intervals(&chemdraw_dashed_bond_gap_intervals(5.39, 2.7, 2.7), &[]);
        assert_gap_intervals(
            &chemdraw_dashed_bond_gap_intervals(5.4, 2.7, 2.7),
            &[(1.8, 3.6)],
        );
    }

    #[test]
    fn hashed_wedge_gap_intervals_keep_every_stripe_one_line_width_long() {
        let gaps = hashed_wedge_gap_intervals(18.0, VIEWER_BOND_STROKE * 2.0, &test_bond());
        assert!(!gaps.is_empty());
        assert!(gaps[0].0 > 0.0);
        assert!(gaps.last().unwrap().1 < 18.0);

        let black_lengths = black_segment_lengths(18.0, 0.0, 0.0, &gaps);
        for length in &black_lengths {
            approx_eq(*length, VIEWER_BOND_STROKE * 2.0);
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
