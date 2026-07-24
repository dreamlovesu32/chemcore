use chemsema_engine::{parse_cdxml_document, render_document, Point, RenderPrimitive};

fn bbox_points(points: &[Point]) -> Option<[f64; 4]> {
    let first = points.first()?;
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x;
    let mut max_y = first.y;
    for point in points {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Some([min_x, min_y, max_x, max_y])
}

fn polygon_width(points: &[Point]) -> Option<f64> {
    if points.len() < 4 {
        return None;
    }
    Some(points[0].distance(points[points.len() - 1]))
}

fn polygon_axis_length(points: &[Point]) -> Option<f64> {
    if points.len() < 4 {
        return None;
    }
    let start = Point::new(
        (points[0].x + points[points.len() - 1].x) * 0.5,
        (points[0].y + points[points.len() - 1].y) * 0.5,
    );
    let middle = points.len() / 2;
    let end = Point::new(
        (points[middle - 1].x + points[middle].x) * 0.5,
        (points[middle - 1].y + points[middle].y) * 0.5,
    );
    Some(start.distance(end))
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tmp/duibi.cdxml".to_string());
    let cdxml = std::fs::read_to_string(&path).expect("cdxml should be readable");
    let document = parse_cdxml_document(&cdxml, Some(&path)).expect("cdxml should parse");
    let primitives = render_document(&document);

    let mut first_bond_length = None;
    let mut first_bond_width = None;
    let mut circle_diameter = None;
    let mut circle_stroke_width = None;
    for primitive in &primitives {
        match primitive {
            RenderPrimitive::Polygon {
                role,
                points,
                bond_id: Some(_),
                ..
            } if *role == chemsema_engine::RenderRole::DocumentBond => {
                first_bond_length.get_or_insert_with(|| polygon_axis_length(points).unwrap_or(0.0));
                first_bond_width.get_or_insert_with(|| polygon_width(points).unwrap_or(0.0));
            }
            RenderPrimitive::Path {
                role,
                points,
                stroke_width,
                object_id: Some(object_id),
                ..
            } if *role == chemsema_engine::RenderRole::DocumentGraphic
                && object_id.contains("symbol")
                && circle_diameter.is_none() =>
            {
                if let Some([min_x, min_y, max_x, max_y]) = bbox_points(points) {
                    circle_diameter = Some((max_x - min_x).max(max_y - min_y));
                    circle_stroke_width = Some(*stroke_width);
                }
            }
            _ => {}
        }
    }

    let bond_length = first_bond_length.unwrap_or(0.0);
    let bond_width = first_bond_width.unwrap_or(0.0);
    let diameter = circle_diameter.unwrap_or(0.0);
    let circle_stroke = circle_stroke_width.unwrap_or(0.0);
    println!("file={path}");
    println!("bond_length={bond_length:.4}");
    println!("bond_width={bond_width:.4}");
    println!("circle_diameter={diameter:.4}");
    println!("circle_stroke_width={circle_stroke:.4}");
    println!(
        "circle_diameter_over_bond_length={:.4}",
        diameter / bond_length
    );
    println!(
        "circle_stroke_over_bond_width={:.4}",
        circle_stroke / bond_width
    );
}
