use chemcore_engine::{parse_cdxml_document, render_document, Point, RenderPrimitive, RenderRole};
use std::path::PathBuf;

fn polygon_bounds(polygon: &[[f64; 2]]) -> Option<[f64; 4]> {
    let mut iter = polygon.iter();
    let first = iter.next()?;
    let mut min_x = first[0];
    let mut min_y = first[1];
    let mut max_x = first[0];
    let mut max_y = first[1];
    for point in iter {
        min_x = min_x.min(point[0]);
        min_y = min_y.min(point[1]);
        max_x = max_x.max(point[0]);
        max_y = max_y.max(point[1]);
    }
    Some([min_x, min_y, max_x, max_y])
}

fn main() {
    let cdxml_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join("cdxml")
                .join("manual")
                .join("desktop")
                .join("glyph.cdxml")
        });
    let cdxml = std::fs::read_to_string(&cdxml_path).expect("read cdxml");
    let document = parse_cdxml_document(&cdxml, Some("glyph")).expect("parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment");
    let node = fragment
        .nodes
        .iter()
        .find(|node| node.element == "N")
        .expect("N node");
    let label = node.label.as_ref().expect("label");
    let bounds = polygon_bounds(&label.glyph_polygons[0]).expect("bounds");
    println!("node.position={:?}", node.position);
    println!("label.position={:?}", label.position);
    println!("label.box={:?}", label.bbox());
    println!("glyph.bounds={:?}", bounds);
    println!(
        "glyph.center=({}, {})",
        (bounds[0] + bounds[2]) * 0.5,
        (bounds[1] + bounds[3]) * 0.5
    );

    let primitives = render_document(&document);
    let center = Point::new(261.59, 202.38);
    for primitive in &primitives {
        let (bond_id, role, points) = match primitive {
            RenderPrimitive::Polygon {
                bond_id,
                role,
                points,
                ..
            }
            | RenderPrimitive::Path {
                bond_id,
                role,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                bond_id,
                role,
                points,
                ..
            }
            | RenderPrimitive::Polyline {
                bond_id,
                role,
                points,
                ..
            } => (bond_id.as_deref(), *role, points.as_slice()),
            RenderPrimitive::Line {
                bond_id,
                role,
                from,
                to,
                ..
            } => {
                let near = if from.distance(center) <= to.distance(center) {
                    *from
                } else {
                    *to
                };
                if *role == RenderRole::DocumentBond {
                    let dx = near.x - center.x;
                    let dy = near.y - center.y;
                    println!(
                        "bond {:?} near=({:.3},{:.3}) r={:.3} angle={:.3}",
                        bond_id,
                        near.x,
                        near.y,
                        (dx * dx + dy * dy).sqrt(),
                        dy.atan2(dx).to_degrees()
                    );
                }
                continue;
            }
            _ => continue,
        };
        if role != RenderRole::DocumentBond || points.is_empty() {
            continue;
        }
        let mut near = points[0];
        let mut best = near.distance(center);
        for point in points.iter().copied() {
            let d = point.distance(center);
            if d < best {
                near = point;
                best = d;
            }
        }
        let dx = near.x - center.x;
        let dy = near.y - center.y;
        println!(
            "bond {:?} near=({:.3},{:.3}) r={:.3} angle={:.3}",
            bond_id,
            near.x,
            near.y,
            (dx * dx + dy * dy).sqrt(),
            dy.atan2(dx).to_degrees()
        );
    }
}
