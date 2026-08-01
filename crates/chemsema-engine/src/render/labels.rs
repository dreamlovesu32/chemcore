use super::*;
use crate::NodeLabel;

pub(super) fn world_point(object: &SceneObject, node: &Node) -> Point {
    Point::new(
        object.transform.translate[0] + node.position[0],
        object.transform.translate[1] + node.position[1],
    )
}

pub(super) fn attached_label_glyph_anchor_world(
    object: &SceneObject,
    node: &Node,
    authored_character_index: usize,
) -> Option<Point> {
    let label = node.label.as_ref()?;
    let source = label.source_text.as_deref().unwrap_or(&label.text);
    let glyph_index = authored_character_glyph_index(source, authored_character_index)?;
    let polygons = label.glyph_polygons();
    let bounds = polygon_bounds(polygons.get(glyph_index)?)?;
    let anchor_y = if source.contains('\r') || source.contains('\n') {
        (bounds.y1 + bounds.y2) * 0.5
    } else {
        let baseline_y = label.position?[1];
        baseline_y - crate::node_label_anchor_baseline_offset(label)
    };
    Some(Point::new(
        (bounds.x1 + bounds.x2) * 0.5 + object.transform.translate[0],
        anchor_y + object.transform.translate[1],
    ))
}

fn authored_character_glyph_index(source: &str, authored_character_index: usize) -> Option<usize> {
    let mut visible_index = 0usize;
    for (index, character) in source.chars().enumerate() {
        if index == authored_character_index {
            return if matches!(character, '\r' | '\n') {
                visible_index.checked_sub(1)
            } else {
                Some(visible_index)
            };
        }
        if !matches!(character, '\r' | '\n') {
            visible_index += 1;
        }
    }
    None
}

#[cfg(test)]
mod attachment_tests {
    use super::{
        algebraic_body_segment_after_label_retreats, authored_character_glyph_index,
        clip_body_segment_out_of_label_geometry, strip_endpoint_label_retreat,
    };
    use crate::{Point, Vector};

    #[test]
    fn authored_multiline_attachment_indices_map_to_visible_glyphs() {
        assert_eq!(authored_character_glyph_index("H+\nN", 2), Some(1));
        assert_eq!(authored_character_glyph_index("H+\nN", 3), Some(2));
    }

    #[test]
    fn authored_single_line_attachment_indices_are_unchanged() {
        assert_eq!(authored_character_glyph_index("(PhO)2POH", 6), Some(6));
    }

    #[test]
    fn continuous_strip_retreat_uses_contacts_between_center_and_edge_rays() {
        let polygon = vec![vec![
            Point::new(4.0, 0.35),
            Point::new(6.0, 0.35),
            Point::new(6.0, 0.65),
            Point::new(4.0, 0.65),
        ]];
        let retreat = strip_endpoint_label_retreat(
            Point::new(0.0, 0.0),
            Vector::new(1.0, 0.0),
            Vector::new(0.0, 1.0),
            &polygon,
            1.0,
        );
        assert!((retreat - 6.0).abs() <= crate::EPSILON);
    }

    #[test]
    fn label_retreat_can_cross_a_short_bond_endpoint() {
        let label = vec![vec![
            Point::new(-5.0, -2.0),
            Point::new(5.0, -2.0),
            Point::new(5.0, 2.0),
            Point::new(-5.0, 2.0),
        ]];
        let (start, end) = clip_body_segment_out_of_label_geometry(
            Point::new(0.0, 0.0),
            Point::new(2.0, 0.0),
            None,
            &label,
            0.0,
            None,
            &[],
            0.0,
        )
        .expect("ChemDraw keeps the reversed short-bond body");
        assert!((start.x - 5.0).abs() < crate::EPSILON);
        assert!((end.x - 2.0).abs() < crate::EPSILON);
    }

    #[test]
    fn overlapping_endpoint_labels_collapse_the_visible_bond_body() {
        let start_label = vec![vec![
            Point::new(-2.0, -2.0),
            Point::new(2.0, -2.0),
            Point::new(2.0, 2.0),
            Point::new(-2.0, 2.0),
        ]];
        let end_label = vec![vec![
            Point::new(1.0, -2.0),
            Point::new(5.0, -2.0),
            Point::new(5.0, 2.0),
            Point::new(1.0, 2.0),
        ]];
        assert!(clip_body_segment_out_of_label_geometry(
            Point::new(0.0, 0.0),
            Point::new(3.0, 0.0),
            None,
            &start_label,
            0.0,
            None,
            &end_label,
            0.0,
        )
        .is_none());

        let (start, end, start_retreat, end_retreat) = algebraic_body_segment_after_label_retreats(
            Point::new(0.0, 0.0),
            Point::new(3.0, 0.0),
            None,
            &start_label,
            0.0,
            None,
            &end_label,
            0.0,
        )
        .expect("parallel bond rails retain their algebraic clipping axis");
        assert!(start_retreat + end_retreat > 3.0);
        assert!(start.x > end.x);
    }
}

pub(super) fn label_box_world(node: &Node, object: &SceneObject) -> Option<RectBox> {
    let label = node.label.as_ref()?;
    let bbox = label.bbox()?;
    Some(RectBox {
        x1: bbox[0] + object.transform.translate[0],
        y1: bbox[1] + object.transform.translate[1],
        x2: bbox[2] + object.transform.translate[0],
        y2: bbox[3] + object.transform.translate[1],
    })
}

pub(super) fn label_polygons_world(node: &Node, object: &SceneObject) -> Vec<Vec<Point>> {
    node.label
        .as_ref()
        .map(|label| {
            label
                .glyph_polygons()
                .into_iter()
                .map(|polygon| polygon_to_world(polygon, object))
                .filter(|polygon| polygon.len() >= 3)
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn label_clip_polygons_world(node: &Node, object: &SceneObject) -> Vec<Vec<Point>> {
    node.label
        .as_ref()
        .map(|label| {
            label_clip_polygons(label)
                .into_iter()
                .map(|polygon| polygon_to_world(polygon, object))
                .filter(|polygon| polygon.len() >= 3)
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn label_clip_polygons_world_for_segment(
    node: &Node,
    object: &SceneObject,
    endpoint: Point,
    other: Point,
    half_width: f64,
) -> Vec<Vec<Point>> {
    let Some(label) = node.label.as_ref() else {
        return Vec::new();
    };
    let direction = Vector::new(other.x - endpoint.x, other.y - endpoint.y);
    if direction.length() <= EPSILON {
        return Vec::new();
    }
    let direction = direction.normalized();
    let cardinal_sector = crate::glyph_kernel::GLYPH_AXIS_HALF_SECTOR_DEG
        .to_radians()
        .sin();
    // ChemDraw's horizontal cardinal sector is a dedicated run-envelope
    // contact. It replaces the general outline/feature kernel inside the
    // measured ten-degree sector; taking their union makes trailing lowercase
    // glyphs such as the r in Tyr retreat too far. BeginAttach/EndAttach only
    // choose the authored character used as the ray origin and do not change
    // this boundary.
    if direction.y.abs() <= cardinal_sector {
        return label_horizontal_axis_contact_polygons(label)
            .into_iter()
            .map(|polygon| polygon_to_world(polygon, object))
            .filter(|polygon| polygon.len() >= 3)
            .collect();
    }
    // Diagonal contacts use the complete label outline. The remaining
    // top/bottom cardinal sectors are owned by the glyph column under the bond.
    if direction.x.abs() > cardinal_sector {
        return label_clip_polygons_world(node, object);
    }
    let local_endpoint = Point::new(
        endpoint.x - object.transform.translate[0],
        endpoint.y - object.transform.translate[1],
    );
    label_clip_polygons_for_segment(label, local_endpoint, direction, half_width)
        .into_iter()
        .map(|polygon| polygon_to_world(polygon, object))
        .filter(|polygon| polygon.len() >= 3)
        .collect()
}

pub(super) fn label_clip_polygons_world_for_cardinal_strip(
    node: &Node,
    object: &SceneObject,
    endpoint: Point,
    other: Point,
    half_width: f64,
) -> Vec<Vec<Point>> {
    let Some(label) = node.label.as_ref() else {
        return Vec::new();
    };
    let direction = Vector::new(other.x - endpoint.x, other.y - endpoint.y);
    if direction.length() <= EPSILON {
        return Vec::new();
    }
    let direction = direction.normalized();
    debug_assert!(
        direction.x.abs()
            <= crate::glyph_kernel::GLYPH_AXIS_HALF_SECTOR_DEG
                .to_radians()
                .sin()
    );
    let local_endpoint = Point::new(
        endpoint.x - object.transform.translate[0],
        endpoint.y - object.transform.translate[1],
    );
    label_clip_polygons_for_strip(label, local_endpoint, direction, half_width)
        .into_iter()
        .map(|polygon| polygon_to_world(polygon, object))
        .filter(|polygon| polygon.len() >= 3)
        .collect()
}

fn label_horizontal_axis_contact_polygons(label: &NodeLabel) -> Vec<Vec<Point>> {
    let clip_polygons = label.glyph_clip_polygons();
    if clip_polygons.is_empty() {
        // A hand-built label without a derived clip profile has only its
        // authoritative glyph outlines.
        return label.glyph_polygons();
    }
    if label.glyph_clip_polygon_owners.len() != clip_polygons.len() {
        // Explicit unowned in-memory clip geometry is already the complete
        // authored contact representation. Imported and edited labels always
        // carry the parallel ownership vector produced by the glyph kernel.
        return clip_polygons;
    }
    let horizontal_contacts = clip_polygons
        .iter()
        .zip(label.glyph_clip_polygon_owners.iter())
        .filter_map(|(polygon, owner)| owner.is_none().then_some(polygon.clone()))
        .collect::<Vec<_>>();
    if horizontal_contacts.is_empty() {
        clip_polygons
    } else {
        horizontal_contacts
    }
}

fn polygon_to_world(polygon: Vec<Point>, object: &SceneObject) -> Vec<Point> {
    compact_polygon_points(
        polygon
            .into_iter()
            .map(|point| {
                Point::new(
                    point.x + object.transform.translate[0],
                    point.y + object.transform.translate[1],
                )
            })
            .collect(),
    )
}

#[derive(Debug, Clone, Copy)]
struct GlyphClipInfo {
    index: usize,
    bounds: RectBox,
    center_x: f64,
    center_y: f64,
    height: f64,
}

fn label_clip_polygons(label: &NodeLabel) -> Vec<Vec<Point>> {
    let glyph_indices = (0..label.glyph_polygons.len()).collect::<Vec<_>>();
    label_clip_polygons_for_glyph_indices(label, &glyph_indices, true)
}

fn label_clip_polygons_for_segment(
    label: &NodeLabel,
    endpoint: Point,
    direction: Vector,
    half_width: f64,
) -> Vec<Vec<Point>> {
    let glyph_polygons = label.glyph_polygons();
    let normal = Vector::new(-direction.y, direction.x);
    let rays = [0.0, half_width, -half_width].map(|offset| {
        Point::new(
            endpoint.x + normal.x * offset,
            endpoint.y + normal.y * offset,
        )
    });
    let mut glyph_indices = glyph_polygons
        .iter()
        .enumerate()
        .filter_map(|(index, polygon)| {
            let bounds = polygon_bounds(polygon)?;
            rays.iter()
                .any(|ray| ray_intersects_rect_forward(*ray, direction, bounds))
                .then_some(index)
        })
        .collect::<Vec<_>>();
    let mut row_glyphs = glyph_polygons
        .iter()
        .enumerate()
        .filter_map(|(index, polygon)| {
            let bounds = polygon_bounds(polygon)?;
            label_glyph_can_join_horizontal_clip(label, index).then_some(GlyphClipInfo {
                index,
                bounds,
                center_x: (bounds.x1 + bounds.x2) * 0.5,
                center_y: (bounds.y1 + bounds.y2) * 0.5,
                height: (bounds.y2 - bounds.y1).max(0.0),
            })
        })
        .collect::<Vec<_>>();
    row_glyphs.sort_by(|left, right| {
        left.center_y
            .total_cmp(&right.center_y)
            .then_with(|| left.center_x.total_cmp(&right.center_x))
    });
    let mut rows: Vec<Vec<GlyphClipInfo>> = Vec::new();
    for glyph in row_glyphs {
        if let Some(row) = rows.iter_mut().find(|row| {
            row.last()
                .is_some_and(|previous| horizontal_clip_glyphs_share_row(*previous, glyph))
        }) {
            row.push(glyph);
        } else {
            rows.push(vec![glyph]);
        }
    }
    for row in &mut rows {
        row.sort_by(|left, right| left.center_x.total_cmp(&right.center_x));
        for pair in row.windows(2) {
            let left = pair[0];
            let right = pair[1];
            let bridge = RectBox {
                x1: left.bounds.x2,
                y1: left.bounds.y1.max(right.bounds.y1),
                x2: right.bounds.x1,
                y2: left.bounds.y2.min(right.bounds.y2),
            };
            if bridge.x2 > bridge.x1 + EPSILON
                && bridge.y2 > bridge.y1 + EPSILON
                && rays
                    .iter()
                    .any(|ray| ray_intersects_rect_forward(*ray, direction, bridge))
            {
                glyph_indices.extend([left.index, right.index]);
            }
        }
    }
    if glyph_indices.is_empty() {
        // A bond axis can lie in a kerning gap or beside a shifted formula
        // script. ChemDraw assigns that gap to the nearest glyph column, then
        // applies that glyph's MarginWidth. A script participates here only
        // when the raw bond rays did not already select another glyph.
        let candidates = glyph_polygons
            .iter()
            .enumerate()
            .filter_map(|(index, polygon)| {
                let bounds = polygon_bounds(polygon)?;
                forward_ray_rect_distance(endpoint, direction, normal, bounds)
                    .map(|distance| (index, distance))
            })
            .collect::<Vec<_>>();
        if let Some((index, _)) = candidates.into_iter().min_by(|left, right| {
            left.1
                .total_cmp(&right.1)
                .then_with(|| left.0.cmp(&right.0))
        }) {
            glyph_indices.push(index);
        }
    }
    glyph_indices.sort_unstable();
    glyph_indices.dedup();
    label_clip_polygons_for_glyph_indices(label, &glyph_indices, false)
}

fn label_clip_polygons_for_strip(
    label: &NodeLabel,
    endpoint: Point,
    direction: Vector,
    half_width: f64,
) -> Vec<Vec<Point>> {
    let normal = Vector::new(-direction.y, direction.x);
    let glyph_indices = label
        .glyph_polygons()
        .iter()
        .enumerate()
        .filter_map(|(index, polygon)| {
            let bounds = polygon_bounds(polygon)?;
            forward_ray_rect_distance(endpoint, direction, normal, bounds)
                .is_some_and(|distance| distance <= half_width + EPSILON)
                .then_some(index)
        })
        .collect::<Vec<_>>();
    label_clip_polygons_for_glyph_indices(label, &glyph_indices, false)
}

fn label_clip_polygons_for_glyph_indices(
    label: &NodeLabel,
    glyph_indices: &[usize],
    include_shared_axis_contacts: bool,
) -> Vec<Vec<Point>> {
    let glyph_polygons = label.glyph_polygons();
    let authored_clip_polygons = label.glyph_clip_polygons();
    let selected = glyph_indices.iter().copied().collect::<BTreeSet<_>>();
    // The glyph outline is itself authoritative retreat geometry.  Some import
    // paths store a separately expanded clip outline, while native labels only
    // carry the glyph outline.  An empty expanded set therefore selects the
    // glyph outline; it must never fall through to an inferred text box.
    let mut polygons = if authored_clip_polygons.is_empty() {
        glyph_indices
            .iter()
            .filter_map(|index| glyph_polygons.get(*index).cloned())
            .collect::<Vec<_>>()
    } else if label.glyph_clip_polygon_owners.len() == authored_clip_polygons.len() {
        authored_clip_polygons
            .into_iter()
            .zip(label.glyph_clip_polygon_owners.iter())
            .filter_map(|(polygon, owner)| match owner {
                Some(index) if selected.contains(index) => Some(polygon),
                None if include_shared_axis_contacts => Some(polygon),
                _ => None,
            })
            .collect::<Vec<_>>()
    } else {
        // Explicitly unowned clip geometry is accepted only for hand-built
        // in-memory labels. Imported and edited labels are rebuilt by the
        // glyph kernel and always carry parallel ownership.
        authored_clip_polygons
    };
    let mut glyphs: Vec<GlyphClipInfo> = glyph_polygons
        .iter()
        .enumerate()
        .filter(|(index, _)| selected.contains(index))
        .filter_map(|(index, polygon)| {
            let bounds = polygon_bounds(polygon)?;
            Some(GlyphClipInfo {
                index,
                bounds,
                center_x: (bounds.x1 + bounds.x2) * 0.5,
                center_y: (bounds.y1 + bounds.y2) * 0.5,
                height: (bounds.y2 - bounds.y1).max(0.0),
            })
        })
        .filter(|glyph| label_glyph_can_join_horizontal_clip(label, glyph.index))
        .collect();
    if glyphs.len() < 2 {
        return polygons;
    }

    glyphs.sort_by(|left, right| {
        left.center_y
            .total_cmp(&right.center_y)
            .then_with(|| left.center_x.total_cmp(&right.center_x))
    });

    let mut rows: Vec<Vec<GlyphClipInfo>> = Vec::new();
    for glyph in glyphs {
        if let Some(row) = rows.iter_mut().find(|row| {
            row.last()
                .is_some_and(|previous| horizontal_clip_glyphs_share_row(*previous, glyph))
        }) {
            row.push(glyph);
        } else {
            rows.push(vec![glyph]);
        }
    }

    for mut row in rows {
        if row.len() < 2 {
            continue;
        }
        row.sort_by(|left, right| left.center_x.total_cmp(&right.center_x));
        polygons.extend(horizontal_label_internal_clip_polygons(&row));
    }

    polygons
}

fn forward_ray_rect_distance(
    start: Point,
    direction: Vector,
    normal: Vector,
    bounds: RectBox,
) -> Option<f64> {
    let center = Point::new((bounds.x1 + bounds.x2) * 0.5, (bounds.y1 + bounds.y2) * 0.5);
    let half_width = (bounds.x2 - bounds.x1).max(0.0) * 0.5;
    let half_height = (bounds.y2 - bounds.y1).max(0.0) * 0.5;
    let offset = Vector::new(center.x - start.x, center.y - start.y);
    let forward_center = offset.x * direction.x + offset.y * direction.y;
    let forward_radius = direction.x.abs() * half_width + direction.y.abs() * half_height;
    if forward_center + forward_radius < -EPSILON {
        return None;
    }
    let normal_center = offset.x * normal.x + offset.y * normal.y;
    let normal_radius = normal.x.abs() * half_width + normal.y.abs() * half_height;
    Some((normal_center.abs() - normal_radius).max(0.0))
}

fn ray_intersects_rect_forward(start: Point, direction: Vector, bounds: RectBox) -> bool {
    let mut near = f64::NEG_INFINITY;
    let mut far = f64::INFINITY;
    for (origin, delta, minimum, maximum) in [
        (start.x, direction.x, bounds.x1, bounds.x2),
        (start.y, direction.y, bounds.y1, bounds.y2),
    ] {
        if delta.abs() <= EPSILON {
            if origin < minimum - EPSILON || origin > maximum + EPSILON {
                return false;
            }
            continue;
        }
        let first = (minimum - origin) / delta;
        let second = (maximum - origin) / delta;
        near = near.max(first.min(second));
        far = far.min(first.max(second));
        if near > far + EPSILON {
            return false;
        }
    }
    far >= -EPSILON
}

fn label_glyph_can_join_horizontal_clip(label: &NodeLabel, glyph_index: usize) -> bool {
    !matches!(
        label_glyph_script(label, glyph_index),
        Some("subscript" | "superscript")
    )
}

fn label_glyph_script(label: &NodeLabel, glyph_index: usize) -> Option<&str> {
    let mut remaining = glyph_index;
    let runs = if !label.line_runs.is_empty() {
        label.line_runs.iter().flatten().collect::<Vec<_>>()
    } else {
        label.runs.iter().collect::<Vec<_>>()
    };
    for run in runs {
        let count = run.text.chars().count();
        if remaining < count {
            return run.script.as_deref();
        }
        remaining = remaining.saturating_sub(count);
    }
    None
}

fn horizontal_clip_glyphs_share_row(left: GlyphClipInfo, right: GlyphClipInfo) -> bool {
    let vertical_overlap =
        (left.bounds.y2.min(right.bounds.y2) - left.bounds.y1.max(right.bounds.y1)).max(0.0);
    let min_height = left.height.min(right.height);
    if min_height <= EPSILON || vertical_overlap < min_height * 0.45 {
        return false;
    }
    let max_height = left.height.max(right.height);
    if (left.center_y - right.center_y).abs() > max_height * 0.45 {
        return false;
    }
    let gap = right.bounds.x1 - left.bounds.x2;
    gap <= max_height * 0.65
}

fn horizontal_label_internal_clip_polygons(row: &[GlyphClipInfo]) -> Vec<Vec<Point>> {
    let mut rectangles = Vec::new();
    let last_index = row.len().saturating_sub(1);

    // Keep the outer half of the first and last glyph as real outline. Their
    // inward halves, and every middle glyph, are rectangularized using that
    // glyph's own bounds. A low parenthesis therefore cannot drag the P-side
    // clipping edge down to the parenthesis baseline.
    for (index, glyph) in row.iter().enumerate() {
        let x1 = if index == 0 {
            glyph.center_x
        } else {
            glyph.bounds.x1
        };
        let x2 = if index == last_index {
            glyph.center_x
        } else {
            glyph.bounds.x2
        };
        if let Some(rectangle) = clip_rectangle(x1, glyph.bounds.y1, x2, glyph.bounds.y2) {
            rectangles.push(rectangle);
        }
    }

    // Bridge only the vertical overlap of adjacent glyphs. This fills an
    // internal character gap without flattening the whole row to a shared
    // top or bottom.
    for pair in row.windows(2) {
        let left = pair[0];
        let right = pair[1];
        let y1 = left.bounds.y1.max(right.bounds.y1);
        let y2 = left.bounds.y2.min(right.bounds.y2);
        if let Some(rectangle) = clip_rectangle(left.bounds.x2, y1, right.bounds.x1, y2) {
            rectangles.push(rectangle);
        }
    }

    rectangles
}

fn clip_rectangle(x1: f64, y1: f64, x2: f64, y2: f64) -> Option<Vec<Point>> {
    if x2 <= x1 + EPSILON || y2 <= y1 + EPSILON {
        return None;
    }
    Some(vec![
        Point::new(x1, y1),
        Point::new(x2, y1),
        Point::new(x2, y2),
        Point::new(x1, y2),
    ])
}

pub(super) fn polygon_bounds(polygon: &[Point]) -> Option<RectBox> {
    let mut bounds = RectBox {
        x1: f64::INFINITY,
        y1: f64::INFINITY,
        x2: f64::NEG_INFINITY,
        y2: f64::NEG_INFINITY,
    };
    for point in polygon {
        bounds.x1 = bounds.x1.min(point.x);
        bounds.y1 = bounds.y1.min(point.y);
        bounds.x2 = bounds.x2.max(point.x);
        bounds.y2 = bounds.y2.max(point.y);
    }
    (bounds.x1.is_finite()
        && bounds.y1.is_finite()
        && bounds.x2.is_finite()
        && bounds.y2.is_finite()
        && bounds.x2 + EPSILON >= bounds.x1
        && bounds.y2 + EPSILON >= bounds.y1)
        .then_some(bounds)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn clip_body_segment_out_of_label_geometry(
    start: Point,
    end: Point,
    start_rect: Option<RectBox>,
    start_polygons: &[Vec<Point>],
    start_half_width: f64,
    end_rect: Option<RectBox>,
    end_polygons: &[Vec<Point>],
    end_half_width: f64,
) -> Option<(Point, Point)> {
    let (clipped_start, clipped_end, start_retreat, end_retreat) =
        algebraic_body_segment_after_label_retreats(
            start,
            end,
            start_rect,
            start_polygons,
            start_half_width,
            end_rect,
            end_polygons,
            end_half_width,
        )?;
    let authored_length = start.distance(end);
    if start_retreat > EPSILON
        && end_retreat > EPSILON
        && start_retreat + end_retreat + EPSILON >= authored_length
    {
        // ChemDraw keeps an algebraic centerline for two overlapping endpoint
        // labels but collapses its body width to zero. It therefore contributes
        // no visible ink. A one-label overrun is different and remains a
        // reversed, visible bond body.
        return None;
    }
    (clipped_start.distance(clipped_end) > EPSILON).then_some((clipped_start, clipped_end))
}

pub(super) fn clip_wavy_body_segment_out_of_label_geometry(
    start: Point,
    end: Point,
    start_polygons: &[Vec<Point>],
    start_half_width: f64,
    start_uses_strip: bool,
    end_polygons: &[Vec<Point>],
    end_half_width: f64,
    end_uses_strip: bool,
) -> Option<(Point, Point)> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let authored_length = direction.length();
    if authored_length <= EPSILON {
        return None;
    }
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let start_retreat = if start_uses_strip {
        strip_endpoint_label_retreat(start, unit, normal, start_polygons, start_half_width)
    } else {
        wedge_endpoint_label_retreat(start, unit, normal, None, start_polygons, start_half_width)
    };
    let end_axis = Vector::new(-unit.x, -unit.y);
    let end_retreat = if end_uses_strip {
        strip_endpoint_label_retreat(end, end_axis, normal, end_polygons, end_half_width)
    } else {
        wedge_endpoint_label_retreat(end, end_axis, normal, None, end_polygons, end_half_width)
    };
    if start_retreat > EPSILON
        && end_retreat > EPSILON
        && start_retreat + end_retreat + EPSILON >= authored_length
    {
        return None;
    }
    let (clipped_start, clipped_end) =
        apply_label_endpoint_retreats(start, end, start_retreat, end_retreat);
    (clipped_start.distance(clipped_end) > EPSILON).then_some((clipped_start, clipped_end))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn algebraic_body_segment_after_label_retreats(
    start: Point,
    end: Point,
    start_rect: Option<RectBox>,
    start_polygons: &[Vec<Point>],
    start_half_width: f64,
    end_rect: Option<RectBox>,
    end_polygons: &[Vec<Point>],
    end_half_width: f64,
) -> Option<(Point, Point, f64, f64)> {
    let (start_retreat, end_retreat) = body_segment_label_retreats(
        start,
        end,
        start_rect,
        start_polygons,
        start_half_width,
        end_rect,
        end_polygons,
        end_half_width,
    )?;
    let (clipped_start, clipped_end) =
        apply_label_endpoint_retreats(start, end, start_retreat, end_retreat);
    Some((clipped_start, clipped_end, start_retreat, end_retreat))
}

pub(super) fn apply_label_endpoint_retreats(
    start: Point,
    end: Point,
    start_retreat: f64,
    end_retreat: f64,
) -> (Point, Point) {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    if direction.length() <= EPSILON {
        return (start, end);
    }
    let unit = direction.normalized();
    (
        Point::new(
            start.x + unit.x * start_retreat.max(0.0),
            start.y + unit.y * start_retreat.max(0.0),
        ),
        Point::new(
            end.x - unit.x * end_retreat.max(0.0),
            end.y - unit.y * end_retreat.max(0.0),
        ),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn body_segment_label_retreats(
    start: Point,
    end: Point,
    start_rect: Option<RectBox>,
    start_polygons: &[Vec<Point>],
    start_half_width: f64,
    end_rect: Option<RectBox>,
    end_polygons: &[Vec<Point>],
    end_half_width: f64,
) -> Option<(f64, f64)> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return None;
    }
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let start_retreat = wedge_endpoint_label_retreat(
        start,
        unit,
        normal,
        start_rect,
        start_polygons,
        start_half_width,
    );
    let end_retreat = wedge_endpoint_label_retreat(
        end,
        Vector::new(-unit.x, -unit.y),
        normal,
        end_rect,
        end_polygons,
        end_half_width,
    );
    Some((start_retreat, end_retreat))
}

#[allow(clippy::too_many_arguments)]
fn wedge_endpoint_label_retreat(
    endpoint: Point,
    axis_from_endpoint: Vector,
    normal: Vector,
    _rect: Option<RectBox>,
    polygons: &[Vec<Point>],
    endpoint_half_width: f64,
) -> f64 {
    let mut retreat: f64 = 0.0;
    for side in [0.0, 1.0, -1.0] {
        let endpoint_offset = endpoint_half_width * side;
        let ray_start = Point::new(
            endpoint.x + normal.x * endpoint_offset,
            endpoint.y + normal.y * endpoint_offset,
        );
        retreat = retreat.max(ray_exit_distance_from_polygons(
            ray_start,
            axis_from_endpoint,
            polygons,
        ));
    }
    retreat
}

fn strip_endpoint_label_retreat(
    endpoint: Point,
    axis_from_endpoint: Vector,
    normal: Vector,
    polygons: &[Vec<Point>],
    half_width: f64,
) -> f64 {
    let mut retreat: f64 = 0.0;
    for polygon in polygons {
        for index in 0..polygon.len() {
            let first = polygon[index];
            let second = polygon[(index + 1) % polygon.len()];
            let first_offset = Vector::new(first.x - endpoint.x, first.y - endpoint.y);
            let second_offset = Vector::new(second.x - endpoint.x, second.y - endpoint.y);
            let first_axis =
                first_offset.x * axis_from_endpoint.x + first_offset.y * axis_from_endpoint.y;
            let second_axis =
                second_offset.x * axis_from_endpoint.x + second_offset.y * axis_from_endpoint.y;
            let first_normal = first_offset.x * normal.x + first_offset.y * normal.y;
            let second_normal = second_offset.x * normal.x + second_offset.y * normal.y;
            if first_normal.abs() <= half_width + EPSILON && first_axis >= -EPSILON {
                retreat = retreat.max(first_axis.max(0.0));
            }
            if second_normal.abs() <= half_width + EPSILON && second_axis >= -EPSILON {
                retreat = retreat.max(second_axis.max(0.0));
            }
            for boundary in [-half_width, half_width] {
                let denominator = second_normal - first_normal;
                if denominator.abs() <= EPSILON {
                    continue;
                }
                let fraction = (boundary - first_normal) / denominator;
                if !(-EPSILON..=1.0 + EPSILON).contains(&fraction) {
                    continue;
                }
                let axis = first_axis + (second_axis - first_axis) * fraction;
                if axis >= -EPSILON {
                    retreat = retreat.max(axis.max(0.0));
                }
            }
        }
    }
    retreat
}

fn ray_exit_distance_from_polygons(
    start: Point,
    direction: Vector,
    polygons: &[Vec<Point>],
) -> f64 {
    let mut farthest: f64 = 0.0;
    for polygon in polygons {
        for index in 0..polygon.len() {
            let first = polygon[index];
            let second = polygon[(index + 1) % polygon.len()];
            let edge = Vector::new(second.x - first.x, second.y - first.y);
            let offset = Vector::new(first.x - start.x, first.y - start.y);
            let denominator = vector_cross(direction, edge);
            if denominator.abs() <= EPSILON {
                if vector_cross(offset, direction).abs() <= EPSILON {
                    for point in [first, second] {
                        let distance =
                            (point.x - start.x) * direction.x + (point.y - start.y) * direction.y;
                        if distance >= -EPSILON {
                            farthest = farthest.max(distance.max(0.0));
                        }
                    }
                }
                continue;
            }
            let distance = vector_cross(offset, edge) / denominator;
            let edge_fraction = vector_cross(offset, direction) / denominator;
            if distance >= -EPSILON && (-EPSILON..=1.0 + EPSILON).contains(&edge_fraction) {
                farthest = farthest.max(distance.max(0.0));
            }
        }
    }
    farthest
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_fragment_line(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    start_box: Option<RectBox>,
    end_box: Option<RectBox>,
    allow_bold_contacts: bool,
    stroke: &str,
    stroke_width: f64,
    dash_array: Vec<f64>,
    line_weight: BondLineWeight,
    object_id: Option<String>,
) {
    render_fragment_line_with_profiles(
        out,
        document,
        object,
        contact_kernel,
        bonds,
        node_map,
        bond,
        start,
        end,
        start_box,
        end_box,
        allow_bold_contacts,
        stroke,
        stroke_width,
        dash_array,
        line_weight,
        object_id,
        false,
        true,
        true,
        true,
        None,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_fragment_line_with_profiles(
    out: &mut Vec<RenderPrimitive>,
    _document: &ChemSemaDocument,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    start_box: Option<RectBox>,
    end_box: Option<RectBox>,
    allow_bold_contacts: bool,
    stroke: &str,
    stroke_width: f64,
    dash_array: Vec<f64>,
    line_weight: BondLineWeight,
    object_id: Option<String>,
    clip_against_label_geometry: bool,
    allow_start_join: bool,
    allow_end_join: bool,
    inherit_kernel_profiles: bool,
    start_endpoint_profile_override: Option<Vec<Point>>,
    end_endpoint_profile_override: Option<Vec<Point>>,
) {
    let start_polygons = node_map
        .get(bond.begin.as_str())
        .map(|node| label_clip_polygons_world(node, object))
        .unwrap_or_default();
    let end_polygons = node_map
        .get(bond.end.as_str())
        .map(|node| label_clip_polygons_world(node, object))
        .unwrap_or_default();
    let start_has_label = node_map
        .get(bond.begin.as_str())
        .and_then(|node| node.label.as_ref())
        .is_some_and(|label| label.has_visible_text());
    let end_has_label = node_map
        .get(bond.end.as_str())
        .and_then(|node| node.label.as_ref())
        .is_some_and(|label| label.has_visible_text());
    let allow_start_join = allow_start_join && !start_has_label;
    let allow_end_join = allow_end_join && !end_has_label;
    let start_endpoint_profile_override = if start_has_label {
        None
    } else {
        start_endpoint_profile_override
    };
    let end_endpoint_profile_override = if end_has_label {
        None
    } else {
        end_endpoint_profile_override
    };
    let Some((clipped_start, clipped_end)) = (if clip_against_label_geometry {
        let half_width = line_weight_stroke_width_for_bond(bond, stroke_width, line_weight) * 0.5;
        clip_body_segment_out_of_label_geometry(
            start,
            end,
            start_box,
            &start_polygons,
            half_width,
            end_box,
            &end_polygons,
            half_width,
        )
    } else {
        Some((start, end))
    }) else {
        return;
    };
    let mut start_retreat = if start_has_label {
        0.0
    } else {
        contact_kernel.endpoint_retreat(&bond.id, &bond.begin)
    };
    let mut end_retreat = if end_has_label {
        0.0
    } else {
        contact_kernel.endpoint_retreat(&bond.id, &bond.end)
    };
    if is_hash_bond(bond) && line_weight == BondLineWeight::Bold && !dash_array.is_empty() {
        let retreat = hash_contact_retreat_distance_for_bond(bond, stroke_width);
        if !start_has_label && endpoint_has_other_bond(bonds, bond, &bond.begin) {
            start_retreat = start_retreat.max(retreat);
        }
        if !end_has_label && endpoint_has_other_bond(bonds, bond, &bond.end) {
            end_retreat = end_retreat.max(retreat);
        }
    }
    let (clipped_start, clipped_end) =
        apply_segment_endpoint_retreats(clipped_start, clipped_end, start_retreat, end_retreat);
    let mut start_endpoint_profile = start_endpoint_profile_override.or_else(|| {
        if inherit_kernel_profiles {
            contact_kernel.endpoint_profile(&bond.id, &bond.begin)
        } else {
            None
        }
    });
    let mut end_endpoint_profile = end_endpoint_profile_override.or_else(|| {
        if inherit_kernel_profiles {
            contact_kernel.endpoint_profile(&bond.id, &bond.end)
        } else {
            None
        }
    });
    if start_retreat > EPSILON {
        start_endpoint_profile = None;
    }
    if end_retreat > EPSILON {
        end_endpoint_profile = None;
    }
    let use_start_contact_kernel = !start_has_label
        && (contact_kernel.uses_endpoint(&bond.id, &bond.begin)
            || start_endpoint_profile.is_some());
    let use_end_contact_kernel = !end_has_label
        && (contact_kernel.uses_endpoint(&bond.id, &bond.end) || end_endpoint_profile.is_some());
    if line_weight == BondLineWeight::Normal && dash_array.is_empty() {
        let allow_main_line_join =
            is_joinable_main_line_render(bond, allow_bold_contacts, line_weight);
        if let Some(points) = main_line_polygon_points(
            object,
            bonds,
            node_map,
            bond,
            clipped_start,
            clipped_end,
            stroke_width,
            allow_main_line_join && allow_start_join && !use_start_contact_kernel,
            allow_main_line_join && allow_end_join && !use_end_contact_kernel,
            start_endpoint_profile.clone(),
            end_endpoint_profile.clone(),
        ) {
            push_bond_polygon(out, &bond.id, points, stroke, stroke, 0.0, object_id);
            return;
        }
    }
    if !dash_array.is_empty() {
        let visual_width = line_weight_stroke_width_for_bond(bond, stroke_width, line_weight);
        let segment_polygons = if line_weight == BondLineWeight::Bold && is_hash_bond(bond) {
            hash_bond_segment_polygons(clipped_start, clipped_end, visual_width, stroke_width)
        } else {
            dashed_bond_segment_polygons_with_profiles(
                clipped_start,
                clipped_end,
                visual_width,
                &dash_array,
                start_endpoint_profile.as_deref(),
                end_endpoint_profile.as_deref(),
            )
        };
        if !segment_polygons.is_empty() {
            for points in segment_polygons {
                push_bond_polygon(
                    out,
                    &bond.id,
                    points,
                    stroke,
                    stroke,
                    0.0,
                    object_id.clone(),
                );
            }
            return;
        }
        if let Some(points) =
            simple_main_line_polygon_points(clipped_start, clipped_end, visual_width)
        {
            push_bond_polygon(
                out,
                &bond.id,
                points,
                stroke,
                stroke,
                0.0,
                object_id.clone(),
            )
        }
        return;
    }
    if line_weight == BondLineWeight::Bold && dash_array.is_empty() {
        let direction = Vector::new(
            clipped_end.x - clipped_start.x,
            clipped_end.y - clipped_start.y,
        );
        if direction.length() > EPSILON {
            if allow_start_join && !use_start_contact_kernel {
                if let Some(points) = bold_main_line_join_polygon(
                    object,
                    bonds,
                    node_map,
                    bond,
                    &bond.begin,
                    clipped_start,
                    direction,
                    stroke_width,
                ) {
                    push_bond_polygon(
                        out,
                        &bond.id,
                        points,
                        stroke,
                        stroke,
                        0.0,
                        object_id.clone(),
                    );
                }
            }
            if allow_end_join && !use_end_contact_kernel {
                if let Some(points) = bold_main_line_join_polygon(
                    object,
                    bonds,
                    node_map,
                    bond,
                    &bond.end,
                    clipped_end,
                    Vector::new(-direction.x, -direction.y),
                    stroke_width,
                ) {
                    push_bond_polygon(
                        out,
                        &bond.id,
                        points,
                        stroke,
                        stroke,
                        0.0,
                        object_id.clone(),
                    );
                }
            }
        }
        push_bond_polygon(
            out,
            &bond.id,
            compute_bold_bond_points(
                object,
                bonds,
                node_map,
                bond,
                clipped_start,
                clipped_end,
                stroke_width,
                allow_bold_contacts && allow_start_join && !use_start_contact_kernel,
                allow_bold_contacts && allow_end_join && !use_end_contact_kernel,
                start_endpoint_profile,
                end_endpoint_profile,
            ),
            stroke,
            stroke,
            0.0,
            object_id,
        );
        return;
    }
    if let Some(points) = simple_main_line_polygon_points(
        clipped_start,
        clipped_end,
        line_weight_stroke_width_for_bond(bond, stroke_width, line_weight),
    ) {
        push_bond_polygon(out, &bond.id, points, stroke, stroke, 0.0, object_id);
    }
}
