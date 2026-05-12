use super::*;

const TEXT_INK_HORIZONTAL_PAD_EM: f64 = 0.16;
const TEXT_GDI_DESCENT_EM: f64 = 0.59;
const TEXT_GDI_LINE_BOX_EM: f64 = 1.45;

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
    document: &ChemcoreDocument,
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
    document: &ChemcoreDocument,
    object: &SceneObject,
) -> Option<[f64; 4]> {
    let mut out = Vec::new();
    render_shape_object(&mut out, document, object);
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
        RenderPrimitive::Text {
            x,
            y,
            font_size,
            line_height,
            box_width,
            text,
            runs,
            text_anchor,
            ..
        } => {
            let measured_width = estimate_text_width(text, runs, *font_size);
            let width = box_width.unwrap_or(0.0).max(measured_width);
            let max_font_size = estimate_text_max_font_size(*font_size, runs);
            let line_count = estimate_text_line_count(text, runs) as f64;
            let line_height = line_height
                .unwrap_or(max_font_size * TEXT_GDI_LINE_BOX_EM)
                .max(max_font_size);
            let right_pad = max_font_size * TEXT_INK_HORIZONTAL_PAD_EM;
            let left_pad = right_pad;
            let min_x = match text_anchor.as_deref() {
                Some("middle") => x - width * 0.5,
                Some("end") => x - width,
                _ => *x,
            };
            let min_y = y - max_font_size * 0.86;
            let max_y =
                y + (line_count - 1.0).max(0.0) * line_height + max_font_size * TEXT_GDI_DESCENT_EM;
            Some([min_x - left_pad, min_y, min_x + width + right_pad, max_y])
        }
    }
}

fn estimate_text_width(text: &str, runs: &[LabelRun], fallback_font_size: f64) -> f64 {
    if !runs.is_empty() {
        let mut max_width = 0.0;
        let mut line_width = 0.0;
        for run in runs {
            let font_size = run.font_size.unwrap_or(fallback_font_size)
                * crate::shared_script_scale_factor(run.script.as_deref());
            for character in run.text.chars() {
                match character {
                    '\n' => {
                        max_width = f64::max(max_width, line_width);
                        line_width = 0.0;
                    }
                    '\r' => {}
                    _ => line_width += crate::shared_estimated_char_width(character, font_size),
                }
            }
        }
        return f64::max(max_width, line_width);
    }
    text.lines()
        .map(|line| estimate_text_line_width(line, fallback_font_size))
        .fold(0.0, f64::max)
}

fn estimate_text_line_width(text: &str, font_size: f64) -> f64 {
    text.chars()
        .filter(|character| *character != '\r')
        .map(|character| crate::shared_estimated_char_width(character, font_size))
        .sum()
}

fn estimate_text_line_count(text: &str, runs: &[LabelRun]) -> usize {
    if !runs.is_empty() {
        return runs
            .iter()
            .map(|run| {
                run.text
                    .chars()
                    .filter(|character| *character == '\n')
                    .count()
            })
            .sum::<usize>()
            + 1;
    }
    text.lines().count().max(1)
}

fn estimate_text_max_font_size(fallback_font_size: f64, runs: &[LabelRun]) -> f64 {
    runs.iter()
        .map(|run| {
            run.font_size.unwrap_or(fallback_font_size)
                * crate::shared_script_scale_factor(run.script.as_deref())
        })
        .fold(fallback_font_size, f64::max)
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
