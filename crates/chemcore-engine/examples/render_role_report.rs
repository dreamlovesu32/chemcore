use chemcore_engine::{
    parse_cdxml_document, parse_document_json, render_document, render_primitives_bounds, Point,
    RenderPrimitive, RenderRole,
};
use serde_json::Value;
use std::collections::BTreeMap;

fn load_document(path: &str) -> chemcore_engine::ChemcoreDocument {
    let text = std::fs::read_to_string(path).expect("input should be readable");
    if path.ends_with(".cdxml") {
        parse_cdxml_document(&text, Some(path)).expect("cdxml should parse")
    } else {
        let value: Value = serde_json::from_str(&text).expect("json should parse");
        if let Some(inner) = value.get("chemcoreDocumentJson").and_then(Value::as_str) {
            parse_document_json(inner).expect("chemcoreDocumentJson should parse")
        } else {
            parse_document_json(&text).expect("document json should parse")
        }
    }
}

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

fn primitive_bbox(primitive: &RenderPrimitive) -> Option<[f64; 4]> {
    match primitive {
        RenderPrimitive::Polygon { points, .. }
        | RenderPrimitive::Polyline { points, .. }
        | RenderPrimitive::Path { points, .. }
        | RenderPrimitive::FilledPath { points, .. } => bbox_points(points),
        RenderPrimitive::Line { from, to, .. } => Some([
            from.x.min(to.x),
            from.y.min(to.y),
            from.x.max(to.x),
            from.y.max(to.y),
        ]),
        RenderPrimitive::Rect {
            x,
            y,
            width,
            height,
            ..
        } => Some([*x, *y, *x + *width, *y + *height]),
        RenderPrimitive::Circle { center, radius, .. } => Some([
            center.x - *radius,
            center.y - *radius,
            center.x + *radius,
            center.y + *radius,
        ]),
        RenderPrimitive::Ellipse { center, rx, ry, .. } => Some([
            center.x - *rx,
            center.y - *ry,
            center.x + *rx,
            center.y + *ry,
        ]),
        RenderPrimitive::Text {
            x,
            y,
            font_size,
            box_width,
            text,
            ..
        } => {
            let width = box_width.unwrap_or((*font_size * text.len() as f64 * 0.5).max(*font_size));
            Some([*x, *y - *font_size, *x + width, *y + *font_size * 0.3])
        }
    }
}

fn union(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    [
        a[0].min(b[0]),
        a[1].min(b[1]),
        a[2].max(b[2]),
        a[3].max(b[3]),
    ]
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../../figure2.cdxml".to_string());
    let document = load_document(&path);
    let primitives = render_document(&document);

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut node_knockout_count = 0usize;
    let mut plain_knockout_count = 0usize;
    let mut knockout_bounds = None;
    let mut visible_no_knockout: Vec<&RenderPrimitive> = Vec::new();
    let mut visible_with_knockout: Vec<&RenderPrimitive> = Vec::new();
    let mut sample_knockouts = Vec::new();
    let mut text_samples = Vec::new();

    for primitive in &primitives {
        let role = match primitive {
            RenderPrimitive::Line { role, .. }
            | RenderPrimitive::Circle { role, .. }
            | RenderPrimitive::Polygon { role, .. }
            | RenderPrimitive::Rect { role, .. }
            | RenderPrimitive::Ellipse { role, .. }
            | RenderPrimitive::Polyline { role, .. }
            | RenderPrimitive::Path { role, .. }
            | RenderPrimitive::FilledPath { role, .. }
            | RenderPrimitive::Text { role, .. } => *role,
        };
        *counts.entry(format!("{role:?}")).or_default() += 1;
        if matches!(
            role,
            RenderRole::DocumentBond
                | RenderRole::DocumentGraphic
                | RenderRole::DocumentText
                | RenderRole::DocumentKnockout
        ) {
            visible_with_knockout.push(primitive);
        }
        if matches!(
            role,
            RenderRole::DocumentBond | RenderRole::DocumentGraphic | RenderRole::DocumentText
        ) {
            visible_no_knockout.push(primitive);
        }
        if role == RenderRole::DocumentKnockout {
            let node_id = match primitive {
                RenderPrimitive::Polygon { node_id, .. }
                | RenderPrimitive::Rect { node_id, .. } => node_id.as_deref(),
                _ => None,
            };
            if node_id.is_some() {
                node_knockout_count += 1;
            } else {
                plain_knockout_count += 1;
            }
            if let Some(b) = primitive_bbox(primitive) {
                knockout_bounds = Some(match knockout_bounds {
                    Some(acc) => union(acc, b),
                    None => b,
                });
                if sample_knockouts.len() < 12 {
                    let object_id = match primitive {
                        RenderPrimitive::Polygon { object_id, .. }
                        | RenderPrimitive::Rect { object_id, .. } => object_id.as_deref(),
                        _ => None,
                    };
                    sample_knockouts.push(serde_json::json!({
                        "nodeId": node_id,
                        "objectId": object_id,
                        "bbox": b,
                    }));
                }
            }
        }
        if role == RenderRole::DocumentText {
            if let RenderPrimitive::Text {
                x,
                y,
                text,
                font_size,
                object_id,
                ..
            } = primitive
            {
                text_samples.push(serde_json::json!({
                    "text": text,
                    "x": x,
                    "y": y,
                    "fontSize": font_size,
                    "objectId": object_id,
                    "bbox": primitive_bbox(primitive),
                }));
            }
        }
    }

    let visible_with_knockout_bounds =
        render_primitives_bounds(visible_with_knockout.iter().copied());
    let visible_no_knockout_bounds = render_primitives_bounds(visible_no_knockout.iter().copied());

    let report = serde_json::json!({
        "file": path,
        "primitiveCount": primitives.len(),
        "roleCounts": counts,
        "visibleBoundsWithKnockout": visible_with_knockout_bounds,
        "visibleBoundsNoKnockout": visible_no_knockout_bounds,
        "knockout": {
            "nodeSpecificCount": node_knockout_count,
            "plainCount": plain_knockout_count,
            "unionBounds": knockout_bounds,
            "samples": sample_knockouts,
        },
        "textSamples": text_samples,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("report json")
    );
}
