use super::*;

#[derive(Debug, Default)]
pub(super) struct MainBondContactKernel {
    active_endpoints: BTreeSet<MainBondEndpointKey>,
    endpoint_profiles: BTreeMap<MainBondEndpointKey, Vec<Point>>,
    endpoint_retreats: BTreeMap<MainBondEndpointKey, f64>,
    patches: Vec<MainBondContactPatch>,
}

impl MainBondContactKernel {
    pub(super) fn uses_endpoint(&self, bond_id: &str, node_id: &str) -> bool {
        self.active_endpoints
            .contains(&MainBondEndpointKey::new(bond_id, node_id))
    }

    pub(super) fn endpoint_profile(&self, bond_id: &str, node_id: &str) -> Option<Vec<Point>> {
        self.endpoint_profiles
            .get(&MainBondEndpointKey::new(bond_id, node_id))
            .cloned()
    }

    pub(super) fn endpoint_retreat(&self, bond_id: &str, node_id: &str) -> f64 {
        self.endpoint_retreats
            .get(&MainBondEndpointKey::new(bond_id, node_id))
            .copied()
            .unwrap_or(0.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MainBondEndpointKey {
    bond_id: String,
    node_id: String,
}

impl MainBondEndpointKey {
    fn new(bond_id: &str, node_id: &str) -> Self {
        Self {
            bond_id: bond_id.to_string(),
            node_id: node_id.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct MainBondContactPatch {
    points: Vec<Point>,
    stroke: Option<String>,
}

const MIN_CONTACT_POLYGON_AREA: f64 = 1.0e-8;

fn is_valid_contact_polygon(points: &[Point]) -> bool {
    points.len() >= 3 && polygon_area_signed(points).abs() > MIN_CONTACT_POLYGON_AREA
}

fn is_valid_contact_profile(points: &[Point]) -> bool {
    points.len() >= 2
        && points
            .first()
            .zip(points.last())
            .is_some_and(|(first, last)| first.distance(*last) > EPSILON)
}

#[derive(Debug, Clone, Copy)]
struct MainContourJoin {
    first: Point,
    second: Point,
}

impl MainContourJoin {
    fn midpoint(self) -> Point {
        Point::new(
            (self.first.x + self.second.x) * 0.5,
            (self.first.y + self.second.y) * 0.5,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct MainBondContour {
    pub(super) base: Point,
    pub(super) direction: Vector,
    pub(super) extent: f64,
    pub(super) half_width: f64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct MainBondEndpointGeometry<'a> {
    pub(super) bond: &'a Bond,
    pub(super) center: Point,
    pub(super) axis: Vector,
    pub(super) base_plus: Point,
    pub(super) base_minus: Point,
    pub(super) contour_plus: MainBondContour,
    pub(super) contour_minus: MainBondContour,
}

#[derive(Debug, Clone)]
struct TwoBondMainContact {
    first_inner_side: f64,
    second_inner_side: f64,
    first_inner: Point,
    first_outer: Point,
    second_inner: Point,
    second_outer: Point,
    bridge: Option<Vec<Point>>,
}

impl MainBondEndpointGeometry<'_> {
    fn base_for_side(self, side: f64) -> Point {
        if side >= 0.0 {
            self.base_plus
        } else {
            self.base_minus
        }
    }

    fn contour_for_side(self, side: f64) -> MainBondContour {
        if side >= 0.0 {
            self.contour_plus
        } else {
            self.contour_minus
        }
    }
}

pub(super) fn build_main_bond_contact_kernel<'a>(
    document: &ChemcoreDocument,
    object: &SceneObject,
    bonds: &'a [Bond],
    node_map: &BTreeMap<&'a str, &'a Node>,
) -> MainBondContactKernel {
    build_main_bond_contact_kernel_for_node_filter(document, object, bonds, node_map, None)
}

pub(super) fn build_main_bond_contact_kernel_for_nodes<'a>(
    document: &ChemcoreDocument,
    object: &SceneObject,
    bonds: &'a [Bond],
    node_map: &BTreeMap<&'a str, &'a Node>,
    node_ids: &BTreeSet<String>,
) -> MainBondContactKernel {
    build_main_bond_contact_kernel_for_node_filter(document, object, bonds, node_map, Some(node_ids))
}

fn build_main_bond_contact_kernel_for_node_filter<'a>(
    document: &ChemcoreDocument,
    object: &SceneObject,
    bonds: &'a [Bond],
    node_map: &BTreeMap<&'a str, &'a Node>,
    node_ids: Option<&BTreeSet<String>>,
) -> MainBondContactKernel {
    let mut kernel = MainBondContactKernel::default();

    for node in node_map.values() {
        if node_ids.is_some_and(|ids| !ids.contains(&node.id)) {
            continue;
        }
        // Labeled nodes are clipped against glyph geometry instead of mitered
        // against neighboring bonds, so the two systems stay independent.
        if node
            .label
            .as_ref()
            .is_some_and(|label| label.has_visible_text())
        {
            continue;
        }

        let incident_bonds: Vec<&Bond> = bonds
            .iter()
            .filter(|bond| {
                (bond.begin == node.id || bond.end == node.id) && !is_hash_contact_obstacle(bond)
            })
            .collect();
        if incident_bonds.len() < 2 {
            continue;
        }

        let mut geometries = Vec::with_capacity(incident_bonds.len());
        for bond in incident_bonds {
            let stroke_width = bond_stroke_width(document, object, bond);
            if let Some(geometry) =
                main_bond_endpoint_geometry(object, node_map, bond, node.id.as_str(), stroke_width)
            {
                geometries.push(geometry);
            }
        }
        if geometries.len() < 2 {
            continue;
        }

        for geometry in &geometries {
            kernel
                .active_endpoints
                .insert(MainBondEndpointKey::new(&geometry.bond.id, &node.id));
        }

        // Straight chains with two simple bonds use the multi-contact path too;
        // it gives the same endpoint-profile representation as branched nodes.
        if geometries.len() == 2
            && geometries
                .iter()
                .all(|geometry| supports_two_bond_multi_contact_shape(geometry.bond))
        {
            append_multi_bond_main_contact(&mut kernel, &geometries, &node.id);
            continue;
        }

        match geometries.len() {
            2 => {
                if let Some(contact) = two_bond_main_contact(geometries[0], geometries[1]) {
                    for (geometry, inner_side, inner_point, outer_point) in [
                        (
                            geometries[0],
                            contact.first_inner_side,
                            contact.first_inner,
                            contact.first_outer,
                        ),
                        (
                            geometries[1],
                            contact.second_inner_side,
                            contact.second_inner,
                            contact.second_outer,
                        ),
                    ] {
                        let endpoint_key = MainBondEndpointKey::new(&geometry.bond.id, &node.id);
                        if supports_main_bond_polygon_endpoint_cap(geometry.bond) {
                            kernel.endpoint_profiles.insert(
                                endpoint_key,
                                main_bond_endpoint_profile(inner_side, inner_point, outer_point),
                            );
                        } else {
                            push_two_bond_contact_triangles(
                                &mut kernel.patches,
                                geometry,
                                inner_side,
                                inner_point,
                                outer_point,
                            );
                        }
                    }
                    if let Some(points) = contact.bridge {
                        kernel.patches.push(MainBondContactPatch {
                            points,
                            stroke: shared_contact_stroke(&geometries),
                        });
                    }
                }
            }
            _ => append_multi_bond_main_contact(&mut kernel, &geometries, &node.id),
        }
    }

    kernel
}

pub(super) fn render_main_bond_contact_patches(
    out: &mut Vec<RenderPrimitive>,
    kernel: &MainBondContactKernel,
    stroke: &str,
    object_id: Option<String>,
) {
    for patch in &kernel.patches {
        let stroke = patch.stroke.as_deref().unwrap_or(stroke);
        push_polygon(
            out,
            patch.points.clone(),
            stroke,
            stroke,
            0.0,
            RenderRole::DocumentBond,
            object_id.clone(),
        );
    }
}

pub(super) fn main_bond_endpoint_geometry<'a>(
    object: &SceneObject,
    node_map: &BTreeMap<&'a str, &'a Node>,
    bond: &'a Bond,
    node_id: &'a str,
    stroke_width: f64,
) -> Option<MainBondEndpointGeometry<'a>> {
    let begin = world_point(object, node_map.get(bond.begin.as_str()).copied()?);
    let end = world_point(object, node_map.get(bond.end.as_str()).copied()?);
    let (center, other) = if node_id == bond.begin {
        (begin, end)
    } else if node_id == bond.end {
        (end, begin)
    } else {
        return None;
    };
    let forward = Vector::new(other.x - center.x, other.y - center.y);
    let length = forward.length();
    if length <= EPSILON {
        return None;
    }
    let axis = forward.normalized();
    let normal = Vector::new(-axis.y, axis.x);

    if let Some(stereo_kind) = bond_stereo_kind(bond) {
        let narrow_half_width = solid_wedge_tip_half_width(stroke_width);
        let wide_half_width = solid_wedge_half_width_for_bond(bond, stroke_width);
        return match stereo_kind {
            BondStereoKind::SolidWedgeEnd
            | BondStereoKind::HashedWedgeEnd
            | BondStereoKind::HollowWedgeEnd
                if node_id == bond.end =>
            {
                let base_plus = Point::new(
                    center.x + normal.x * wide_half_width,
                    center.y + normal.y * wide_half_width,
                );
                let base_minus = Point::new(
                    center.x - normal.x * wide_half_width,
                    center.y - normal.y * wide_half_width,
                );
                let tip_plus = Point::new(
                    other.x + normal.x * narrow_half_width,
                    other.y + normal.y * narrow_half_width,
                );
                let tip_minus = Point::new(
                    other.x - normal.x * narrow_half_width,
                    other.y - normal.y * narrow_half_width,
                );
                let plus_direction =
                    Vector::new(tip_plus.x - base_plus.x, tip_plus.y - base_plus.y);
                let minus_direction =
                    Vector::new(tip_minus.x - base_minus.x, tip_minus.y - base_minus.y);
                Some(MainBondEndpointGeometry {
                    bond,
                    center,
                    axis,
                    base_plus,
                    base_minus,
                    contour_plus: MainBondContour {
                        base: base_plus,
                        direction: plus_direction,
                        extent: plus_direction.length(),
                        half_width: wide_half_width,
                    },
                    contour_minus: MainBondContour {
                        base: base_minus,
                        direction: minus_direction,
                        extent: minus_direction.length(),
                        half_width: wide_half_width,
                    },
                })
            }
            BondStereoKind::SolidWedgeBegin
            | BondStereoKind::HashedWedgeBegin
            | BondStereoKind::HollowWedgeBegin
                if node_id == bond.begin =>
            {
                let base_plus = Point::new(
                    center.x + normal.x * wide_half_width,
                    center.y + normal.y * wide_half_width,
                );
                let base_minus = Point::new(
                    center.x - normal.x * wide_half_width,
                    center.y - normal.y * wide_half_width,
                );
                let tip_plus = Point::new(
                    other.x + normal.x * narrow_half_width,
                    other.y + normal.y * narrow_half_width,
                );
                let tip_minus = Point::new(
                    other.x - normal.x * narrow_half_width,
                    other.y - normal.y * narrow_half_width,
                );
                let plus_direction =
                    Vector::new(tip_plus.x - base_plus.x, tip_plus.y - base_plus.y);
                let minus_direction =
                    Vector::new(tip_minus.x - base_minus.x, tip_minus.y - base_minus.y);
                Some(MainBondEndpointGeometry {
                    bond,
                    center,
                    axis,
                    base_plus,
                    base_minus,
                    contour_plus: MainBondContour {
                        base: base_plus,
                        direction: plus_direction,
                        extent: plus_direction.length(),
                        half_width: wide_half_width,
                    },
                    contour_minus: MainBondContour {
                        base: base_minus,
                        direction: minus_direction,
                        extent: minus_direction.length(),
                        half_width: wide_half_width,
                    },
                })
            }
            BondStereoKind::SolidWedgeEnd
            | BondStereoKind::SolidWedgeBegin
            | BondStereoKind::HashedWedgeEnd
            | BondStereoKind::HashedWedgeBegin
            | BondStereoKind::HollowWedgeEnd
            | BondStereoKind::HollowWedgeBegin => {
                let base_plus = Point::new(
                    center.x + normal.x * narrow_half_width,
                    center.y + normal.y * narrow_half_width,
                );
                let base_minus = Point::new(
                    center.x - normal.x * narrow_half_width,
                    center.y - normal.y * narrow_half_width,
                );
                let cap_plus = Point::new(
                    other.x + normal.x * wide_half_width,
                    other.y + normal.y * wide_half_width,
                );
                let cap_minus = Point::new(
                    other.x - normal.x * wide_half_width,
                    other.y - normal.y * wide_half_width,
                );
                let plus_direction =
                    Vector::new(cap_plus.x - base_plus.x, cap_plus.y - base_plus.y);
                let minus_direction =
                    Vector::new(cap_minus.x - base_minus.x, cap_minus.y - base_minus.y);
                Some(MainBondEndpointGeometry {
                    bond,
                    center,
                    axis,
                    base_plus,
                    base_minus,
                    contour_plus: MainBondContour {
                        base: base_plus,
                        direction: plus_direction,
                        extent: plus_direction.length(),
                        half_width: narrow_half_width,
                    },
                    contour_minus: MainBondContour {
                        base: base_minus,
                        direction: minus_direction,
                        extent: minus_direction.length(),
                        half_width: narrow_half_width,
                    },
                })
            }
        };
    }

    if !supports_main_bond_contact_shape(bond) {
        return None;
    }

    let half_width =
        line_weight_stroke_width_for_bond(bond, stroke_width, bond.line_weights.main) * 0.5;
    let base_plus = Point::new(
        center.x + normal.x * half_width,
        center.y + normal.y * half_width,
    );
    let base_minus = Point::new(
        center.x - normal.x * half_width,
        center.y - normal.y * half_width,
    );
    Some(MainBondEndpointGeometry {
        bond,
        center,
        axis,
        base_plus,
        base_minus,
        contour_plus: MainBondContour {
            base: base_plus,
            direction: axis,
            extent: length,
            half_width,
        },
        contour_minus: MainBondContour {
            base: base_minus,
            direction: axis,
            extent: length,
            half_width,
        },
    })
}

fn supports_main_bond_contact_shape(bond: &Bond) -> bool {
    if bond.stereo.is_some() {
        return false;
    }
    if bond.order == 1 {
        return bond.line_styles.left == BondLinePattern::Solid
            && bond.line_styles.right == BondLinePattern::Solid
            && bond.line_weights.left == BondLineWeight::Normal
            && bond.line_weights.right == BondLineWeight::Normal
            && matches!(
                (bond.line_styles.main, bond.line_weights.main),
                (BondLinePattern::Solid, BondLineWeight::Normal)
                    | (BondLinePattern::Dashed, BondLineWeight::Normal)
                    | (BondLinePattern::Solid, BondLineWeight::Bold)
                    | (BondLinePattern::Dashed, BondLineWeight::Bold)
            );
    }
    if bond.order == 2 {
        return side_double_placement(bond).is_some()
            && bond.line_styles.main == BondLinePattern::Solid
            && matches!(
                bond.line_weights.main,
                BondLineWeight::Normal | BondLineWeight::Bold
            );
    }
    if bond.order == 3 {
        return bond.double.is_none()
            && bond.line_styles.main == BondLinePattern::Solid
            && bond.line_styles.left == BondLinePattern::Solid
            && bond.line_styles.right == BondLinePattern::Solid
            && bond.line_weights.main == BondLineWeight::Normal
            && bond.line_weights.left == BondLineWeight::Normal
            && bond.line_weights.right == BondLineWeight::Normal;
    }
    false
}

fn supports_main_bond_polygon_endpoint_cap(bond: &Bond) -> bool {
    if let Some(stereo_kind) = bond_stereo_kind(bond) {
        return matches!(
            stereo_kind,
            BondStereoKind::SolidWedgeBegin
                | BondStereoKind::SolidWedgeEnd
                | BondStereoKind::HashedWedgeBegin
                | BondStereoKind::HashedWedgeEnd
                | BondStereoKind::HollowWedgeBegin
                | BondStereoKind::HollowWedgeEnd
        );
    }
    supports_main_bond_contact_shape(bond)
}

fn supports_two_bond_multi_contact_shape(bond: &Bond) -> bool {
    bond.stereo.is_none() && bond.order == 1 && supports_main_bond_contact_shape(bond)
}

fn two_bond_main_contact(
    first: MainBondEndpointGeometry<'_>,
    second: MainBondEndpointGeometry<'_>,
) -> Option<TwoBondMainContact> {
    if main_contact_is_straight_through(first.axis, second.axis) {
        return None;
    }

    let Some(first_inner_side) = main_contact_side(first.axis, second.axis) else {
        return None;
    };
    let Some(second_inner_side) = main_contact_side(second.axis, first.axis) else {
        return None;
    };

    // Intersect the inner pair and outer pair separately. Each bond receives
    // its own endpoint profile, while the bridge patch fills any shared wedge.
    let inner = bounded_main_contour_intersection(
        first.contour_for_side(first_inner_side),
        second.contour_for_side(second_inner_side),
    );
    let outer = bounded_main_contour_intersection(
        first.contour_for_side(-first_inner_side),
        second.contour_for_side(-second_inner_side),
    );

    let bridge = compact_polygon_points(vec![inner.first, outer.first, outer.second, inner.second]);
    let bridge = if is_valid_contact_polygon(&bridge) {
        Some(bridge)
    } else {
        None
    };

    Some(TwoBondMainContact {
        first_inner_side,
        second_inner_side,
        first_inner: inner.first,
        first_outer: outer.first,
        second_inner: inner.second,
        second_outer: outer.second,
        bridge,
    })
}

fn main_bond_endpoint_profile(inner_side: f64, inner: Point, outer: Point) -> Vec<Point> {
    if inner_side >= 0.0 {
        vec![inner, outer]
    } else {
        vec![outer, inner]
    }
}

fn push_two_bond_contact_triangles(
    out: &mut Vec<MainBondContactPatch>,
    geometry: MainBondEndpointGeometry<'_>,
    inner_side: f64,
    inner: Point,
    outer: Point,
) {
    for points in [
        compact_polygon_points(vec![
            geometry.center,
            geometry.base_for_side(inner_side),
            inner,
        ]),
        compact_polygon_points(vec![
            geometry.center,
            outer,
            geometry.base_for_side(-inner_side),
        ]),
    ] {
        if is_valid_contact_polygon(&points) {
            out.push(MainBondContactPatch {
                points,
                stroke: geometry.bond.stroke.clone(),
            });
        }
    }
}

fn append_multi_bond_main_contact(
    kernel: &mut MainBondContactKernel,
    geometries: &[MainBondEndpointGeometry<'_>],
    node_id: &str,
) {
    if geometries.len() < 2 {
        return;
    }
    let mut ordered = geometries.to_vec();
    ordered.sort_by(|a, b| axis_angle(a.axis).total_cmp(&axis_angle(b.axis)));
    if ordered.len() == 2 && vector_cross(ordered[0].axis, ordered[1].axis).abs() <= 1.0e-6 {
        return;
    }

    // Sort by polar angle and intersect only adjacent contours. That builds a
    // stable ring around substituted centers without all-pairs artifacts.
    let mut ring_intersections = Vec::with_capacity(ordered.len());
    for index in 0..ordered.len() {
        let current = ordered[index];
        let next = ordered[(index + 1) % ordered.len()];
        let join = extended_main_contour_intersection(current.contour_plus, next.contour_minus);
        ring_intersections.push(join.midpoint());
    }

    for (index, current) in ordered.iter().enumerate() {
        let previous_intersection = ring_intersections[(index + ordered.len() - 1) % ordered.len()];
        let next_intersection = ring_intersections[index];

        if supports_main_bond_polygon_endpoint_cap(current.bond) {
            let mut profile = vec![next_intersection, current.center, previous_intersection];
            profile = compact_polygon_points(profile);
            let valid_profile = if ordered.len() == 2 {
                is_valid_contact_profile(&profile)
            } else {
                is_valid_contact_polygon(&profile)
            };
            if valid_profile {
                kernel
                    .endpoint_profiles
                    .insert(MainBondEndpointKey::new(&current.bond.id, node_id), profile);
            }
            continue;
        }

        let points = compact_polygon_points(vec![
            current.center,
            previous_intersection,
            next_intersection,
        ]);
        if is_valid_contact_polygon(&points) {
            kernel.patches.push(MainBondContactPatch {
                points,
                stroke: current.bond.stroke.clone(),
            });
        }
    }
}

fn shared_contact_stroke(geometries: &[MainBondEndpointGeometry<'_>]) -> Option<String> {
    let first = geometries.first()?.bond.stroke.as_deref()?;
    geometries
        .iter()
        .all(|geometry| geometry.bond.stroke.as_deref() == Some(first))
        .then(|| first.to_string())
}

pub(super) fn main_contact_side(axis: Vector, other_axis: Vector) -> Option<f64> {
    let normal = Vector::new(-axis.y, axis.x);
    let side_value = normal.x * other_axis.x + normal.y * other_axis.y;
    if side_value.abs() <= EPSILON {
        None
    } else {
        Some(side_value.signum())
    }
}

pub(super) fn main_contact_is_straight_through(axis: Vector, other_axis: Vector) -> bool {
    vector_cross(axis, other_axis).abs() <= 1.0e-6 && vector_dot(axis, other_axis) < 0.0
}

fn bond_ray_angle_degrees(axis: Vector, other_axis: Vector) -> f64 {
    vector_dot(axis.normalized(), other_axis.normalized())
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees()
}

pub(super) fn center_double_skips_extension(axis: Vector, other_axis: Vector) -> bool {
    bond_ray_angle_degrees(axis, other_axis) > CENTER_DOUBLE_NO_EXTENSION_ANGLE_DEGREES
}

pub(super) fn bond_ray_is_acute(axis: Vector, other_axis: Vector) -> bool {
    vector_dot(axis.normalized(), other_axis.normalized()) > EPSILON
}

fn extended_main_contour_intersection(
    first: MainBondContour,
    second: MainBondContour,
) -> MainContourJoin {
    if let Some((intersection, _, _)) = line_intersection_with_parameters(
        first.base,
        first.direction,
        second.base,
        second.direction,
    ) {
        return MainContourJoin {
            first: intersection,
            second: intersection,
        };
    }
    bounded_main_contour_intersection(first, second)
}

fn bounded_main_contour_intersection(
    first: MainBondContour,
    second: MainBondContour,
) -> MainContourJoin {
    let max_distance = (first.half_width.max(second.half_width) * MAIN_CONTACT_MITER_LIMIT)
        .min(first.extent.min(second.extent) * 0.38);
    let fallback_distance = max_distance.min(
        (first.extent.min(second.extent) * 0.18).max(first.half_width.max(second.half_width) * 2.5),
    );
    let fallback_first = point_on_main_contour(first, fallback_distance);
    let fallback_second = point_on_main_contour(second, fallback_distance);

    let Some((intersection, _, _)) = line_intersection_with_parameters(
        first.base,
        first.direction,
        second.base,
        second.direction,
    ) else {
        return MainContourJoin {
            first: fallback_first,
            second: fallback_second,
        };
    };

    let first_unit = first.direction.normalized();
    let second_unit = second.direction.normalized();
    let first_projection = vector_dot(
        Vector::new(intersection.x - first.base.x, intersection.y - first.base.y),
        first_unit,
    );
    let second_projection = vector_dot(
        Vector::new(
            intersection.x - second.base.x,
            intersection.y - second.base.y,
        ),
        second_unit,
    );
    if first_projection.abs() <= max_distance && second_projection.abs() <= max_distance {
        return MainContourJoin {
            first: intersection,
            second: intersection,
        };
    }

    let clamped_first =
        point_on_main_contour(first, first_projection.clamp(-max_distance, max_distance));
    let clamped_second =
        point_on_main_contour(second, second_projection.clamp(-max_distance, max_distance));
    MainContourJoin {
        first: clamped_first,
        second: clamped_second,
    }
}

fn point_on_main_contour(contour: MainBondContour, distance: f64) -> Point {
    let length = contour.direction.length();
    if length <= EPSILON {
        return contour.base;
    }
    let unit = contour.direction.normalized();
    Point::new(
        contour.base.x + unit.x * distance,
        contour.base.y + unit.y * distance,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(left: f64, right: f64) {
        assert!((left - right).abs() <= 1.0e-6, "{left} != {right}");
    }

    fn contour(base: Point, direction: Vector, extent: f64, half_width: f64) -> MainBondContour {
        MainBondContour {
            base,
            direction,
            extent,
            half_width,
        }
    }

    #[test]
    fn contact_side_distinguishes_left_right_and_collinear() {
        assert_eq!(
            main_contact_side(Vector::new(1.0, 0.0), Vector::new(0.0, 1.0)),
            Some(1.0)
        );
        assert_eq!(
            main_contact_side(Vector::new(1.0, 0.0), Vector::new(0.0, -1.0)),
            Some(-1.0)
        );
        assert_eq!(
            main_contact_side(Vector::new(1.0, 0.0), Vector::new(-1.0, 0.0)),
            None
        );
    }

    #[test]
    fn straight_through_and_acute_checks_follow_axis_relation() {
        assert!(main_contact_is_straight_through(
            Vector::new(1.0, 0.0),
            Vector::new(-1.0, 0.0)
        ));
        assert!(!main_contact_is_straight_through(
            Vector::new(1.0, 0.0),
            Vector::new(0.0, 1.0)
        ));

        assert!(bond_ray_is_acute(
            Vector::new(1.0, 0.0),
            Vector::new(1.0, 1.0)
        ));
        assert!(!bond_ray_is_acute(
            Vector::new(1.0, 0.0),
            Vector::new(-1.0, 1.0)
        ));
    }

    #[test]
    fn bounded_main_contour_intersection_clamps_far_miter() {
        let first = contour(Point::new(0.0, 1.0), Vector::new(1.0, 0.0), 20.0, 1.0);
        let second = contour(Point::new(0.0, -1.0), Vector::new(1.0, 0.1), 20.0, 1.0);

        let join = bounded_main_contour_intersection(first, second);
        approx_eq(join.first.distance(first.base), 4.0);
        approx_eq(join.second.distance(second.base), 4.0);
        assert!(join.first.x < 20.0);
        assert!(join.second.x < 20.0);
    }

    #[test]
    fn extended_main_contour_intersection_uses_true_line_intersection() {
        let first = contour(Point::new(0.0, 1.0), Vector::new(1.0, 0.0), 10.0, 1.0);
        let second = contour(Point::new(4.0, -1.0), Vector::new(0.0, 1.0), 10.0, 1.0);

        let join = extended_main_contour_intersection(first, second);
        approx_eq(join.first.x, 4.0);
        approx_eq(join.first.y, 1.0);
        approx_eq(join.second.x, 4.0);
        approx_eq(join.second.y, 1.0);
    }

    #[test]
    fn centered_double_extension_threshold_uses_162_degrees() {
        let axis = Vector::new(1.0, 0.0);
        let skip_axis = Vector::new(
            (-170.0_f64).to_radians().cos(),
            (-170.0_f64).to_radians().sin(),
        );
        let extend_axis = Vector::new(
            (-150.0_f64).to_radians().cos(),
            (-150.0_f64).to_radians().sin(),
        );

        assert!(center_double_skips_extension(axis, skip_axis));
        assert!(!center_double_skips_extension(axis, extend_axis));
    }
}
