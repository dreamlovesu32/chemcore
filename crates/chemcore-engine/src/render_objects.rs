use super::*;
use crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM;

#[path = "render_objects/arrows.rs"]
mod arrows;
#[path = "render_objects/graphics.rs"]
mod graphics;
#[path = "render_objects/text.rs"]
mod text;

pub(super) use arrows::render_line_object;
pub(super) use graphics::{render_bracket_object, render_shape_object};
pub(super) use text::render_text_object;

fn text_anchor(align: &str) -> String {
    match align {
        "center" => "middle".to_string(),
        "right" => "end".to_string(),
        _ => "start".to_string(),
    }
}

fn fragment_label_font_size(label: &crate::NodeLabel) -> f64 {
    let mut size = label.font_size;
    for run in &label.runs {
        if let Some(run_size) = run.font_size {
            size = Some(size.map_or(run_size, |current| current.max(run_size)));
        }
    }
    size.unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM)
}

fn fragment_label_lines(label: &crate::NodeLabel) -> Vec<String> {
    if !label.lines.is_empty() {
        return label
            .lines
            .iter()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
    }
    if label.text.contains('\n') {
        return label
            .text
            .split('\n')
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect();
    }
    if label.text.trim().is_empty() {
        Vec::new()
    } else {
        vec![label.text.clone()]
    }
}

fn fragment_label_runs_for_line(
    label: &crate::NodeLabel,
    index: usize,
    line: &str,
) -> Vec<LabelRun> {
    if let Some(line_runs) = label.line_runs.get(index) {
        return line_runs.clone();
    }
    if index == 0 && !label.runs.is_empty() && !label.text.contains('\n') && label.lines.is_empty()
    {
        return label.runs.clone();
    }
    vec![LabelRun {
        text: line.to_string(),
        font_family: label.font_family.clone(),
        font_size: label.font_size,
        fill: label.fill.clone(),
        font_weight: None,
        font_style: None,
        underline: None,
        script: None,
    }]
}

fn fragment_label_position_world(label: &crate::NodeLabel, object: &SceneObject) -> Point {
    let position = label.position.unwrap_or([0.0, 0.0]);
    Point::new(
        object.transform.translate[0] + position[0],
        object.transform.translate[1] + position[1],
    )
}

pub(super) fn render_molecule_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
) {
    let Some(resource_ref) = object.payload.resource_ref.as_ref() else {
        return;
    };
    let Some(resource) = document.resources.get(resource_ref) else {
        return;
    };
    match &resource.data {
        ResourceData::Fragment(fragment)
            if resource.resource_type == "molecule_fragment2d"
                || resource.encoding == "chemcore.molecule.fragment2d" =>
        {
            let node_map: BTreeMap<&str, &Node> = fragment
                .nodes
                .iter()
                .map(|node| (node.id.as_str(), node))
                .collect();
            let stroke = molecule_stroke(document, object);
            let object_id = Some(object.id.clone());
            let contact_kernel =
                build_main_bond_contact_kernel(document, object, &fragment.bonds, &node_map);

            for bond in &fragment.bonds {
                render_fragment_bond(
                    out,
                    document,
                    object,
                    &contact_kernel,
                    &fragment.bonds,
                    &node_map,
                    bond,
                    &stroke,
                    object_id.clone(),
                );
            }
            render_main_bond_contact_patches(out, &contact_kernel, &stroke, object_id.clone());

            for node in &fragment.nodes {
                render_fragment_label(out, document, object, node, object_id.clone());
                render_fragment_node_invalid_marker(out, object, node, object_id.clone());
            }
        }
        ResourceData::Text(molblock) => {
            render_legacy_molecule_object(out, document, object, molblock);
        }
        _ => {}
    }
}

fn render_fragment_node_invalid_marker(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    node: &Node,
    object_id: Option<String>,
) {
    if !crate::node_has_charge_symbol_invalid(node) {
        return;
    }
    let center = Point::new(
        object.transform.translate[0] + node.position[0],
        object.transform.translate[1] + node.position[1],
    );
    out.push(RenderPrimitive::Circle {
        role: RenderRole::DocumentGraphic,
        object_id,
        node_id: Some(node.id.clone()),
        center,
        radius: crate::ENDPOINT_FOCUS_RADIUS,
        fill: "none".to_string(),
        stroke: "#d32f2f".to_string(),
        stroke_width: 1.0,
    });
}

pub(super) fn render_fragment_label(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
    node: &Node,
    object_id: Option<String>,
) {
    let Some(label) = node.label.as_ref() else {
        return;
    };
    if !label.has_visible_text() {
        return;
    }

    let font_size = fragment_label_font_size(label);
    let text_anchor = text_anchor(label.align.as_deref().unwrap_or("left"));
    let font_family = label.font_family.clone().or_else(|| {
        object
            .style_ref
            .as_ref()
            .and_then(|style_ref| document.styles.get(style_ref))
            .and_then(|style| style_string(style, "fontFamily"))
    });
    let fill = label.fill.clone().or_else(|| {
        object
            .style_ref
            .as_ref()
            .and_then(|style_ref| document.styles.get(style_ref))
            .and_then(|style| style_string(style, "fill"))
    });
    let knockout_polygons = label_polygons_world(node, object);
    if knockout_polygons.is_empty() {
        if let Some(box_value) = label_box_world(node, object) {
            out.push(RenderPrimitive::Rect {
                role: RenderRole::DocumentKnockout,
                object_id: object_id.clone(),
                node_id: Some(node.id.clone()),
                x: box_value.x1,
                y: box_value.y1,
                width: (box_value.x2 - box_value.x1).max(0.0),
                height: (box_value.y2 - box_value.y1).max(0.0),
                fill: Some(document.document.page.background.clone()),
                stroke: None,
                stroke_width: 0.0,
                rx: None,
                ry: None,
                dash_array: Vec::new(),
                fill_gradient: None,
            });
        }
    } else {
        for polygon in knockout_polygons {
            push_label_knockout_polygon(out, polygon, object_id.clone(), node.id.clone());
        }
    }
    if fragment_label_is_invalid(label) {
        if let Some(box_value) = label_box_world(node, object) {
            out.push(RenderPrimitive::Rect {
                role: RenderRole::DocumentGraphic,
                object_id: object_id.clone(),
                node_id: Some(node.id.clone()),
                x: box_value.x1,
                y: box_value.y1,
                width: (box_value.x2 - box_value.x1).max(0.0),
                height: (box_value.y2 - box_value.y1).max(0.0),
                fill: Some("none".to_string()),
                stroke: Some("#d32f2f".to_string()),
                stroke_width: 1.0,
                rx: None,
                ry: None,
                dash_array: Vec::new(),
                fill_gradient: None,
            });
        }
    }

    let lines = fragment_label_lines(label);
    if lines.is_empty() {
        return;
    }
    let world_position = fragment_label_position_world(label, object);
    if lines.len() == 1 {
        push_text_for_node(
            out,
            world_position.x,
            world_position.y,
            String::new(),
            font_size,
            font_family,
            fill,
            Some(text_anchor),
            fragment_label_runs_for_line(label, 0, &lines[0]),
            object_id,
            Some(node.id.clone()),
        );
        return;
    }

    let label_box = label_box_world(node, object);
    let line_height = label_box
        .map(|box_value| (box_value.y2 - box_value.y1) / lines.len() as f64)
        .unwrap_or(font_size * 1.05);
    let box_top = label_box
        .map(|box_value| box_value.y1)
        .unwrap_or(world_position.y - line_height * 0.82);
    for (index, line) in lines.iter().enumerate() {
        let baseline_y = box_top + line_height * index as f64 + line_height * 0.82;
        push_text_for_node(
            out,
            world_position.x,
            baseline_y,
            String::new(),
            font_size,
            font_family.clone(),
            fill.clone(),
            Some(text_anchor.clone()),
            fragment_label_runs_for_line(label, index, line),
            object_id.clone(),
            Some(node.id.clone()),
        );
    }
}

fn fragment_label_is_invalid(label: &crate::NodeLabel) -> bool {
    label
        .meta
        .get("labelRecognition")
        .and_then(|value| value.get("status"))
        .and_then(serde_json::Value::as_str)
        == Some("invalid")
}
