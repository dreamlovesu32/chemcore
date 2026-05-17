use chemcore_engine::{
    parse_cdxml_document, parse_document_json, render_document, LabelRun, RenderPrimitive,
    RenderRole,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;

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

fn run_summary(runs: &[LabelRun]) -> Vec<Value> {
    runs.iter()
        .map(|run| {
            json!({
                "text": run.text,
                "script": run.script,
                "fontFamily": run.font_family,
                "fontSize": run.font_size,
                "fontWeight": run.font_weight,
                "fontStyle": run.font_style,
                "fill": run.fill,
                "underline": run.underline,
            })
        })
        .collect()
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
        let RenderPrimitive::Text {
            role,
            object_id,
            node_id,
            x,
            y,
            baseline_offset,
            text,
            font_size,
            font_family,
            fill,
            text_anchor,
            line_height,
            preserve_lines,
            box_width,
            runs,
            rotate,
            rotate_center,
        } = primitive
        else {
            continue;
        };
        if role != RenderRole::DocumentText {
            continue;
        }
        let matches = filters.is_empty()
            || object_id.as_ref().is_some_and(|id| filters.contains(id))
            || node_id.as_ref().is_some_and(|id| filters.contains(id));
        if !matches {
            continue;
        }
        out.push(json!({
            "role": role,
            "objectId": object_id,
            "nodeId": node_id,
            "x": x,
            "y": y,
            "baselineOffset": baseline_offset,
            "text": text,
            "fontSize": font_size,
            "fontFamily": font_family,
            "fill": fill,
            "textAnchor": text_anchor,
            "lineHeight": line_height,
            "preserveLines": preserve_lines,
            "boxWidth": box_width,
            "rotate": rotate,
            "rotateCenter": rotate_center,
            "runs": run_summary(&runs),
        }));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&out).expect("json should serialize")
    );
}
