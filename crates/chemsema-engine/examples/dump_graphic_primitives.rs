use chemsema_engine::{
    parse_cdxml_document, parse_document_json, render_document, RenderPrimitive, RenderRole,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;

fn load_document(path: &str) -> chemsema_engine::ChemSemaDocument {
    let text = std::fs::read_to_string(path).expect("input should be readable");
    if path.ends_with(".cdxml") {
        parse_cdxml_document(&text, Some(path)).expect("cdxml should parse")
    } else {
        let value: Value = serde_json::from_str(&text).expect("json should parse");
        if let Some(inner) = value.get("chemsemaDocumentJson").and_then(Value::as_str) {
            parse_document_json(inner).expect("chemsemaDocumentJson should parse")
        } else {
            parse_document_json(&text).expect("document json should parse")
        }
    }
}

fn matches_filter(
    filters: &BTreeSet<String>,
    object_id: Option<&String>,
    node_id: Option<&String>,
) -> bool {
    filters.is_empty()
        || object_id.is_some_and(|id| filters.contains(id))
        || node_id.is_some_and(|id| filters.contains(id))
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .unwrap_or_else(|| "tmp/current-thiocyanation.payload.json".to_string());
    let filters: BTreeSet<String> = args.collect();
    let document = load_document(&path);
    let primitives = render_document(&document);

    let mut out = Vec::new();
    for primitive in primitives {
        match primitive {
            RenderPrimitive::Line {
                role,
                object_id,
                bond_id,
                from,
                to,
                stroke,
                stroke_width,
                dash_array,
            } => {
                if role != RenderRole::DocumentGraphic
                    || !matches_filter(&filters, object_id.as_ref(), None)
                {
                    continue;
                }
                out.push(json!({
                    "kind": "line",
                    "objectId": object_id,
                    "bondId": bond_id,
                    "from": from,
                    "to": to,
                    "stroke": stroke,
                    "strokeWidth": stroke_width,
                    "dashArray": dash_array,
                }));
            }
            RenderPrimitive::Circle {
                role,
                object_id,
                node_id,
                center,
                radius,
                fill,
                stroke,
                stroke_width,
            } => {
                if role != RenderRole::DocumentGraphic
                    || !matches_filter(&filters, object_id.as_ref(), node_id.as_ref())
                {
                    continue;
                }
                out.push(json!({
                    "kind": "circle",
                    "objectId": object_id,
                    "nodeId": node_id,
                    "center": center,
                    "radius": radius,
                    "fill": fill,
                    "stroke": stroke,
                    "strokeWidth": stroke_width,
                }));
            }
            RenderPrimitive::Polygon {
                role,
                object_id,
                node_id,
                bond_id,
                points,
                fill,
                stroke,
                stroke_width,
            } => {
                if role != RenderRole::DocumentGraphic
                    || !matches_filter(&filters, object_id.as_ref(), node_id.as_ref())
                {
                    continue;
                }
                out.push(json!({
                    "kind": "polygon",
                    "objectId": object_id,
                    "nodeId": node_id,
                    "bondId": bond_id,
                    "points": points,
                    "fill": fill,
                    "stroke": stroke,
                    "strokeWidth": stroke_width,
                }));
            }
            RenderPrimitive::Rect {
                role,
                object_id,
                node_id,
                x,
                y,
                width,
                height,
                fill,
                stroke,
                stroke_width,
                rx,
                ry,
                dash_array,
                fill_gradient,
            } => {
                if role != RenderRole::DocumentGraphic
                    || !matches_filter(&filters, object_id.as_ref(), node_id.as_ref())
                {
                    continue;
                }
                out.push(json!({
                    "kind": "rect",
                    "objectId": object_id,
                    "nodeId": node_id,
                    "x": x,
                    "y": y,
                    "width": width,
                    "height": height,
                    "fill": fill,
                    "stroke": stroke,
                    "strokeWidth": stroke_width,
                    "rx": rx,
                    "ry": ry,
                    "dashArray": dash_array,
                    "fillGradient": fill_gradient,
                }));
            }
            RenderPrimitive::Ellipse {
                role,
                object_id,
                center,
                rx,
                ry,
                rotate,
                fill,
                stroke,
                stroke_width,
                dash_array,
                fill_gradient,
            } => {
                if role != RenderRole::DocumentGraphic
                    || !matches_filter(&filters, object_id.as_ref(), None)
                {
                    continue;
                }
                out.push(json!({
                    "kind": "ellipse",
                    "objectId": object_id,
                    "center": center,
                    "rx": rx,
                    "ry": ry,
                    "rotate": rotate,
                    "fill": fill,
                    "stroke": stroke,
                    "strokeWidth": stroke_width,
                    "dashArray": dash_array,
                    "fillGradient": fill_gradient,
                }));
            }
            RenderPrimitive::Polyline {
                role,
                object_id,
                bond_id,
                points,
                stroke,
                stroke_width,
                dash_array,
                line_cap,
                line_join,
            } => {
                if role != RenderRole::DocumentGraphic
                    || !matches_filter(&filters, object_id.as_ref(), None)
                {
                    continue;
                }
                out.push(json!({
                    "kind": "polyline",
                    "objectId": object_id,
                    "bondId": bond_id,
                    "points": points,
                    "stroke": stroke,
                    "strokeWidth": stroke_width,
                    "dashArray": dash_array,
                    "lineCap": line_cap,
                    "lineJoin": line_join,
                }));
            }
            RenderPrimitive::Path {
                role,
                object_id,
                bond_id,
                d,
                points,
                stroke,
                stroke_width,
                dash_array,
                line_cap,
                line_join,
                rotate,
                rotate_center,
            } => {
                if role != RenderRole::DocumentGraphic
                    || !matches_filter(&filters, object_id.as_ref(), None)
                {
                    continue;
                }
                out.push(json!({
                    "kind": "path",
                    "objectId": object_id,
                    "bondId": bond_id,
                    "d": d,
                    "points": points,
                    "stroke": stroke,
                    "strokeWidth": stroke_width,
                    "dashArray": dash_array,
                    "lineCap": line_cap,
                    "lineJoin": line_join,
                    "rotate": rotate,
                    "rotateCenter": rotate_center,
                }));
            }
            RenderPrimitive::FilledPath {
                role,
                object_id,
                node_id,
                bond_id,
                d,
                points,
                fill,
                fill_rule,
                clip_path_d,
                clip_rule,
                rotate,
                rotate_center,
            } => {
                if role != RenderRole::DocumentGraphic
                    || !matches_filter(&filters, object_id.as_ref(), node_id.as_ref())
                {
                    continue;
                }
                out.push(json!({
                    "kind": "filled-path",
                    "objectId": object_id,
                    "nodeId": node_id,
                    "bondId": bond_id,
                    "d": d,
                    "points": points,
                    "fill": fill,
                    "fillRule": fill_rule,
                    "clipPathD": clip_path_d,
                    "clipRule": clip_rule,
                    "rotate": rotate,
                    "rotateCenter": rotate_center,
                }));
            }
            RenderPrimitive::Image {
                role,
                object_id,
                x,
                y,
                width,
                height,
                href,
                source_crop,
                source_width,
                source_height,
                opacity,
                preserve_aspect_ratio,
                rotate,
                rotate_center,
            } => {
                if role != RenderRole::DocumentGraphic
                    || !matches_filter(&filters, object_id.as_ref(), None)
                {
                    continue;
                }
                out.push(json!({
                    "kind": "image",
                    "objectId": object_id,
                    "x": x,
                    "y": y,
                    "width": width,
                    "height": height,
                    "href": href,
                    "sourceCrop": source_crop,
                    "sourceWidth": source_width,
                    "sourceHeight": source_height,
                    "opacity": opacity,
                    "preserveAspectRatio": preserve_aspect_ratio,
                    "rotate": rotate,
                    "rotateCenter": rotate_center,
                }));
            }
            RenderPrimitive::Text { .. } => {}
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&out).expect("json should serialize")
    );
}
