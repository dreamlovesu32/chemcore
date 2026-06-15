use std::collections::BTreeMap;

use chemcore_engine::{
    parse_cdxml_document, render_document, ChemcoreDocument, MoleculeFragment, Point,
    RenderPrimitive, RenderRole, SceneObject,
};

fn project(start: Point, unit: Point, point: Point) -> f64 {
    (point.x - start.x) * unit.x + (point.y - start.y) * unit.y
}

fn primitive_projection(
    start: Point,
    unit: Point,
    primitive: &RenderPrimitive,
) -> Option<(f64, f64)> {
    let mut min_t = f64::INFINITY;
    let mut max_t = f64::NEG_INFINITY;
    let mut seen = false;
    match primitive {
        RenderPrimitive::Line { from, to, .. } => {
            for point in [*from, *to] {
                let t = project(start, unit, point);
                min_t = min_t.min(t);
                max_t = max_t.max(t);
                seen = true;
            }
        }
        RenderPrimitive::Polygon { points, .. }
        | RenderPrimitive::Path { points, .. }
        | RenderPrimitive::FilledPath { points, .. } => {
            for point in points {
                let t = project(start, unit, *point);
                min_t = min_t.min(t);
                max_t = max_t.max(t);
                seen = true;
            }
        }
        _ => {}
    }
    seen.then_some((min_t, max_t))
}

fn label_box_exit_distance(node_position: Point, unit: Point, bbox: [f64; 4]) -> Option<f64> {
    let dx = unit.x;
    let dy = unit.y;
    let mut candidates = Vec::new();
    if dx.abs() > chemcore_engine::EPSILON {
        for x in [bbox[0], bbox[2]] {
            let t = (x - node_position.x) / dx;
            let y = node_position.y + dy * t;
            if t >= -chemcore_engine::EPSILON && y >= bbox[1] && y <= bbox[3] {
                candidates.push(t.max(0.0));
            }
        }
    }
    if dy.abs() > chemcore_engine::EPSILON {
        for y in [bbox[1], bbox[3]] {
            let t = (y - node_position.y) / dy;
            let x = node_position.x + dx * t;
            if t >= -chemcore_engine::EPSILON && x >= bbox[0] && x <= bbox[2] {
                candidates.push(t.max(0.0));
            }
        }
    }
    candidates
        .into_iter()
        .min_by(|left, right| left.total_cmp(right))
}

fn polygon_exit_distance(node_position: Point, unit: Point, polygon: &[Point]) -> Option<f64> {
    if polygon.len() < 3 {
        return None;
    }
    let mut best: Option<f64> = None;
    let ray_end = Point::new(
        node_position.x + unit.x * 10_000.0,
        node_position.y + unit.y * 10_000.0,
    );
    for index in 0..polygon.len() {
        let next = (index + 1) % polygon.len();
        let Some(t) =
            segment_intersection_fraction(node_position, ray_end, polygon[index], polygon[next])
        else {
            continue;
        };
        if t < -chemcore_engine::EPSILON {
            continue;
        }
        let distance = t * 10_000.0;
        best = Some(best.map_or(distance, |current| current.max(distance)));
    }
    best
}

fn segment_intersection_fraction(
    start: Point,
    end: Point,
    first: Point,
    second: Point,
) -> Option<f64> {
    let direction = Point::new(end.x - start.x, end.y - start.y);
    let edge = Point::new(second.x - first.x, second.y - first.y);
    let denom = direction.x * edge.y - direction.y * edge.x;
    if denom.abs() <= chemcore_engine::EPSILON {
        return None;
    }
    let offset = Point::new(first.x - start.x, first.y - start.y);
    let t = (offset.x * edge.y - offset.y * edge.x) / denom;
    let u = (offset.x * direction.y - offset.y * direction.x) / denom;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some(t)
}

fn label_glyph_exit_distance(
    node_position: Point,
    unit: Point,
    glyph_polygons: &[Vec<[f64; 2]>],
) -> Option<f64> {
    glyph_polygons
        .iter()
        .filter_map(|polygon| {
            let points: Vec<_> = polygon
                .iter()
                .map(|point| Point::new(point[0], point[1]))
                .collect();
            polygon_exit_distance(node_position, unit, &points)
        })
        .max_by(|left, right| left.total_cmp(right))
}

fn fragment_for_object<'a>(
    document: &'a ChemcoreDocument,
    object: &SceneObject,
) -> Option<&'a MoleculeFragment> {
    let resource_id = object.payload.resource_ref.as_deref()?;
    document
        .resources
        .get(resource_id)
        .and_then(|resource| resource.data.as_fragment())
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../../figure2.cdxml".to_string());
    let cdxml = std::fs::read_to_string(&path).expect("cdxml should be readable");
    let document = parse_cdxml_document(&cdxml, Some(&path)).expect("cdxml should parse");
    let primitives = render_document(&document);
    let mut rows = Vec::new();
    for object in &document.objects {
        let Some(fragment) = fragment_for_object(&document, object) else {
            continue;
        };
        let nodes: BTreeMap<_, _> = fragment.nodes.iter().map(|node| (&node.id, node)).collect();
        for bond in &fragment.bonds {
            if bond.order != 1 || bond.stereo.is_some() {
                continue;
            }
            let Some(begin) = nodes.get(&bond.begin) else {
                continue;
            };
            let Some(end) = nodes.get(&bond.end) else {
                continue;
            };
            for (node, other, endpoint_is_begin) in [(begin, end, true), (end, begin, false)] {
                let Some(label) = node.label.as_ref().filter(|label| label.has_visible_text())
                else {
                    continue;
                };
                let Some(bbox) = label.bbox() else {
                    continue;
                };
                let node_position = Point::new(
                    object.transform.translate[0] + node.position[0],
                    object.transform.translate[1] + node.position[1],
                );
                let other_position = Point::new(
                    object.transform.translate[0] + other.position[0],
                    object.transform.translate[1] + other.position[1],
                );
                let vector = Point::new(
                    other_position.x - node_position.x,
                    other_position.y - node_position.y,
                );
                let length = (vector.x * vector.x + vector.y * vector.y).sqrt();
                if length <= chemcore_engine::EPSILON {
                    continue;
                }
                let unit = Point::new(vector.x / length, vector.y / length);
                let visible = primitives
                    .iter()
                    .filter_map(|primitive| {
                        let primitive_bond_id = match primitive {
                            RenderPrimitive::Line {
                                role,
                                bond_id: Some(id),
                                ..
                            }
                            | RenderPrimitive::Polygon {
                                role,
                                bond_id: Some(id),
                                ..
                            }
                            | RenderPrimitive::Path {
                                role,
                                bond_id: Some(id),
                                ..
                            }
                            | RenderPrimitive::FilledPath {
                                role,
                                bond_id: Some(id),
                                ..
                            } if *role == RenderRole::DocumentBond => id,
                            _ => return None,
                        };
                        (primitive_bond_id == &bond.id)
                            .then(|| primitive_projection(node_position, unit, primitive))
                            .flatten()
                    })
                    .map(|(min_t, max_t)| {
                        if endpoint_is_begin {
                            min_t
                        } else {
                            length - max_t
                        }
                    })
                    .filter(|value| *value >= -chemcore_engine::EPSILON)
                    .min_by(|left, right| left.total_cmp(right));
                let Some(visible_distance) = visible else {
                    continue;
                };
                let world_bbox = [
                    bbox[0] + object.transform.translate[0],
                    bbox[1] + object.transform.translate[1],
                    bbox[2] + object.transform.translate[0],
                    bbox[3] + object.transform.translate[1],
                ];
                let box_exit =
                    label_box_exit_distance(node_position, unit, world_bbox).unwrap_or(0.0);
                let world_glyphs: Vec<_> = label
                    .glyph_polygons
                    .iter()
                    .map(|polygon| {
                        polygon
                            .iter()
                            .map(|point| {
                                [
                                    point[0] + object.transform.translate[0],
                                    point[1] + object.transform.translate[1],
                                ]
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect();
                let glyph_exit = label_glyph_exit_distance(node_position, unit, &world_glyphs)
                    .unwrap_or(box_exit);
                rows.push((
                    visible_distance - glyph_exit,
                    visible_distance - box_exit,
                    visible_distance,
                    glyph_exit,
                    box_exit,
                    object.id.clone(),
                    bond.id.clone(),
                    node.id.clone(),
                    label.text.clone(),
                    bond.label_clip_margin.unwrap_or_default(),
                ));
            }
        }
    }

    rows.sort_by(|left, right| right.0.total_cmp(&left.0));
    println!(
        "gap_to_glyph gap_to_box visible glyph_exit box_exit object bond node label clip_margin"
    );
    for row in rows.into_iter().take(40) {
        println!(
            "{:.3} {:.3} {:.3} {:.3} {:.3} {} {} {} {:?} {:.3}",
            row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8, row.9
        );
    }
}
