use super::*;

const TEXT_GDI_DESCENT_EM: f64 = 0.59;
const TEXT_GDI_LINE_BOX_EM: f64 = 1.45;
const TEXT_GDI_ASCENT_EM: f64 = 0.86;

pub fn render_primitives_bounds<'a>(
    primitives: impl IntoIterator<Item = &'a RenderPrimitive>,
) -> Option<[f64; 4]> {
    let mut bounds: Option<[f64; 4]> = None;
    for primitive in primitives {
        let Some([min_x, min_y, max_x, max_y]) = render_primitive_bounds(primitive) else {
            continue;
        };
        bounds = Some(match bounds {
            Some([current_min_x, current_min_y, current_max_x, current_max_y]) => [
                f64::min(current_min_x, min_x),
                f64::min(current_min_y, min_y),
                f64::max(current_max_x, max_x),
                f64::max(current_max_y, max_y),
            ],
            None => [min_x, min_y, max_x, max_y],
        });
    }
    bounds
}

pub(crate) fn fragment_bond_visual_bounds(
    document: &ChemSemaDocument,
    object: &SceneObject,
    fragment: &MoleculeFragment,
    bond: &Bond,
) -> Option<[f64; 4]> {
    let node_map: BTreeMap<&str, &Node> = fragment
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let contact_kernel =
        build_main_bond_contact_kernel(document, object, &fragment.bonds, &node_map);
    let mut out = Vec::new();
    render_fragment_bond(
        &mut out,
        document,
        object,
        &contact_kernel,
        &fragment.bonds,
        &node_map,
        bond,
        &molecule_stroke(document, object),
        None,
    );

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut found = false;

    for primitive in out {
        if !primitive_matches_bond(&primitive, &bond.id) {
            continue;
        }
        let Some([x1, y1, x2, y2]) = render_primitive_bounds(&primitive) else {
            continue;
        };
        min_x = min_x.min(x1);
        min_y = min_y.min(y1);
        max_x = max_x.max(x2);
        max_y = max_y.max(y2);
        found = true;
    }

    found.then_some([min_x, min_y, max_x, max_y])
}

pub(crate) fn shape_object_visual_bounds(
    document: &ChemSemaDocument,
    object: &SceneObject,
) -> Option<[f64; 4]> {
    let mut out = Vec::new();
    render_shape_object(&mut out, document, object);
    render_primitives_bounds(out.iter())
}

pub(crate) fn line_object_visual_bounds(
    document: &ChemSemaDocument,
    object: &SceneObject,
) -> Option<[f64; 4]> {
    let mut out = Vec::new();
    render_line_object(&mut out, document, object);
    render_primitives_bounds(out.iter())
}

pub(crate) fn curve_object_visual_bounds(
    document: &ChemSemaDocument,
    object: &SceneObject,
) -> Option<[f64; 4]> {
    let mut out = Vec::new();
    render_curve_object(&mut out, document, object);
    render_primitives_bounds(out.iter())
}

pub(crate) fn text_object_visual_bounds(
    document: &ChemSemaDocument,
    object: &SceneObject,
) -> Option<[f64; 4]> {
    let mut out = Vec::new();
    render_text_object(&mut out, document, object);
    render_primitives_bounds(out.iter())
}

pub(crate) fn bracket_object_visual_bounds(
    document: &ChemSemaDocument,
    object: &SceneObject,
) -> Option<[f64; 4]> {
    let mut out = Vec::new();
    render_bracket_object(&mut out, document, object);
    render_primitives_bounds(out.iter())
}

pub(crate) fn primitive_matches_bond(primitive: &RenderPrimitive, bond_id: &str) -> bool {
    match primitive {
        RenderPrimitive::Line {
            bond_id: Some(current),
            ..
        }
        | RenderPrimitive::Polygon {
            bond_id: Some(current),
            ..
        }
        | RenderPrimitive::Polyline {
            bond_id: Some(current),
            ..
        }
        | RenderPrimitive::Path {
            bond_id: Some(current),
            ..
        } => current == bond_id,
        _ => false,
    }
}

pub fn render_primitive_bounds(primitive: &RenderPrimitive) -> Option<[f64; 4]> {
    match primitive {
        RenderPrimitive::Line {
            from,
            to,
            stroke_width,
            ..
        } => {
            let half_width = stroke_width * 0.5;
            Some([
                from.x.min(to.x) - half_width,
                from.y.min(to.y) - half_width,
                from.x.max(to.x) + half_width,
                from.y.max(to.y) + half_width,
            ])
        }
        RenderPrimitive::Polygon {
            points,
            stroke_width,
            ..
        }
        | RenderPrimitive::Polyline {
            points,
            stroke_width,
            ..
        }
        | RenderPrimitive::Path {
            points,
            stroke_width,
            ..
        } => point_list_bounds(points, *stroke_width * 0.5),
        RenderPrimitive::FilledPath { points, .. } => point_list_bounds(points, 0.0),
        RenderPrimitive::Image {
            x,
            y,
            width,
            height,
            rotate,
            rotate_center,
            ..
        } => {
            let corners = [
                Point::new(*x, *y),
                Point::new(*x + *width, *y),
                Point::new(*x + *width, *y + *height),
                Point::new(*x, *y + *height),
            ];
            if rotate.abs() <= crate::EPSILON {
                return point_list_bounds(&corners, 0.0);
            }
            let center = rotate_center.unwrap_or(Point::new(*x + *width * 0.5, *y + *height * 0.5));
            let radians = rotate.to_radians();
            let cos = radians.cos();
            let sin = radians.sin();
            let rotated: Vec<Point> = corners
                .into_iter()
                .map(|point| {
                    let dx = point.x - center.x;
                    let dy = point.y - center.y;
                    Point::new(
                        center.x + dx * cos - dy * sin,
                        center.y + dx * sin + dy * cos,
                    )
                })
                .collect();
            point_list_bounds(&rotated, 0.0)
        }
        RenderPrimitive::Rect {
            x,
            y,
            width,
            height,
            stroke_width,
            ..
        } => {
            let half_width = stroke_width * 0.5;
            Some([
                *x - half_width,
                *y - half_width,
                *x + *width + half_width,
                *y + *height + half_width,
            ])
        }
        RenderPrimitive::Ellipse {
            center,
            rx,
            ry,
            stroke_width,
            ..
        } => {
            let half_width = stroke_width * 0.5;
            Some([
                center.x - rx - half_width,
                center.y - ry - half_width,
                center.x + rx + half_width,
                center.y + ry + half_width,
            ])
        }
        RenderPrimitive::Circle { center, radius, .. } => Some([
            center.x - radius,
            center.y - radius,
            center.x + radius,
            center.y + radius,
        ]),
        RenderPrimitive::Text { .. } => text_primitive_visual_bounds(primitive),
    }
}

pub(crate) fn text_primitive_visual_bounds(primitive: &RenderPrimitive) -> Option<[f64; 4]> {
    let RenderPrimitive::Text {
        x,
        y,
        font_size,
        line_height,
        box_width,
        text,
        runs,
        text_anchor,
        dominant_baseline,
        font_family,
        rotate,
        rotate_center,
        ..
    } = primitive
    else {
        return None;
    };
    let ink_bounds = crate::shared_text_horizontal_ink_bounds(
        text,
        runs,
        *font_size,
        font_family.as_deref(),
        text_anchor.as_deref(),
    );
    let max_font_size = crate::shared_estimated_text_max_font_size(*font_size, runs);
    let line_count = crate::shared_estimated_text_line_count(text, runs) as f64;
    let line_height = line_height
        .unwrap_or(max_font_size * TEXT_GDI_LINE_BOX_EM)
        .max(max_font_size);
    let box_bounds = box_width.map(|width| match text_anchor.as_deref() {
        Some("middle") => [-width * 0.5, width * 0.5],
        Some("end") => [-width, 0.0],
        _ => [0.0, width],
    });
    let min_x = x + box_bounds.map_or(ink_bounds[0], |bounds| bounds[0].min(ink_bounds[0]));
    let max_x = x + box_bounds.map_or(ink_bounds[1], |bounds| bounds[1].max(ink_bounds[1]));
    let (min_y, max_y) = if matches!(dominant_baseline.as_deref(), Some("central" | "middle")) {
        let block_height = line_height * line_count.max(1.0);
        (y - block_height * 0.5, y + block_height * 0.5)
    } else {
        (
            y - max_font_size * TEXT_GDI_ASCENT_EM,
            y + (line_count - 1.0).max(0.0) * line_height + max_font_size * TEXT_GDI_DESCENT_EM,
        )
    };
    let corners = [
        Point::new(min_x, min_y),
        Point::new(max_x, min_y),
        Point::new(max_x, max_y),
        Point::new(min_x, max_y),
    ];
    if rotate.abs() <= crate::EPSILON {
        return point_list_bounds(&corners, 0.0);
    }
    let center = rotate_center.unwrap_or(Point::new(*x, *y));
    let radians = rotate.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let rotated = corners.map(|point| {
        let dx = point.x - center.x;
        let dy = point.y - center.y;
        Point::new(
            center.x + dx * cos - dy * sin,
            center.y + dx * sin + dy * cos,
        )
    });
    point_list_bounds(&rotated, 0.0)
}

fn point_list_bounds(points: &[Point], margin: f64) -> Option<[f64; 4]> {
    let mut iter = points.iter().copied();
    let first = iter.next()?;
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x;
    let mut max_y = first.y;
    for point in iter {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Some([
        min_x - margin,
        min_y - margin,
        max_x + margin,
        max_y + margin,
    ])
}
