use super::*;

fn text_anchor(align: &str) -> String {
    match align {
        "center" => "middle".to_string(),
        "right" => "end".to_string(),
        _ => "start".to_string(),
    }
}

fn fragment_label_font_size(label: &crate::NodeLabel) -> f64 {
    let mut size = label.font_size.unwrap_or(0.0).max(9.5);
    for run in &label.runs {
        size = size.max(run.font_size.unwrap_or(0.0));
    }
    size.max(9.5)
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
        face: None,
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
            }
        }
        ResourceData::Text(molblock) => {
            render_legacy_molecule_object(out, document, object, molblock);
        }
        _ => {}
    }
}

pub(super) fn render_line_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
) {
    let points = payload_points(&object.payload, "points");
    if points.len() < 2 {
        return;
    }

    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    let stroke = style
        .and_then(|value| style_string(value, "stroke"))
        .unwrap_or_else(|| "#222222".to_string());
    let stroke_width = style
        .and_then(|value| {
            style_number(value, "strokeWidth").or_else(|| style_number(value, "stroke_width"))
        })
        .unwrap_or(1.6);
    let line_cap = style
        .and_then(|value| style_string(value, "lineCap"))
        .unwrap_or_else(|| "round".to_string());
    let line_join = style
        .and_then(|value| style_string(value, "lineJoin"))
        .unwrap_or_else(|| "round".to_string());
    let mut shaft_points = points.clone();
    let object_id = Some(object.id.clone());

    if payload_string(&object.payload, "head").as_deref() == Some("end") {
        if let Some(arrow_head) = payload_arrow_head(&object.payload, "arrowHead") {
            if arrow_head.length > 0.0 && shaft_points.len() >= 2 {
                let from = shaft_points[shaft_points.len() - 2];
                let to = shaft_points[shaft_points.len() - 1];
                let shaft_end = arrow_shaft_end(from, to, arrow_head);
                if let Some(last) = shaft_points.last_mut() {
                    *last = shaft_end;
                }
                push_polyline(
                    out,
                    shaft_points,
                    &stroke,
                    stroke_width,
                    Vec::new(),
                    Some(line_cap.clone()),
                    Some(line_join.clone()),
                    RenderRole::DocumentGraphic,
                    object_id.clone(),
                );
                push_polygon(
                    out,
                    arrow_head_points(from, to, arrow_head),
                    &stroke,
                    &stroke,
                    stroke_width,
                    RenderRole::DocumentGraphic,
                    object_id,
                );
                return;
            }
        }
    }

    push_polyline(
        out,
        shaft_points,
        &stroke,
        stroke_width,
        Vec::new(),
        Some(line_cap),
        Some(line_join),
        RenderRole::DocumentGraphic,
        object_id,
    );
}

pub(super) fn render_text_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
) {
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    let font_size = payload_number(&object.payload, "fontSize")
        .or_else(|| {
            style.and_then(|value| {
                style_number(value, "fontSize").or_else(|| style_number(value, "font_size"))
            })
        })
        .unwrap_or(12.0);
    let line_height = payload_number(&object.payload, "lineHeight").unwrap_or(15.0);
    let align = payload_string(&object.payload, "align").unwrap_or_else(|| "left".to_string());
    let text_anchor = text_anchor(&align);
    let font_family = style.and_then(|value| style_string(value, "fontFamily"));
    let fill = style.and_then(|value| style_string(value, "fill"));
    let object_id = Some(object.id.clone());

    if payload_bool(&object.payload, "preserveLines").unwrap_or(false) {
        let runs = payload_runs(&object.payload, "runs");
        if !runs.is_empty() {
            for (index, line_runs) in split_runs_by_line(&runs).into_iter().enumerate() {
                if line_runs.is_empty() {
                    continue;
                }
                push_text(
                    out,
                    tx,
                    ty + font_size * 0.82 + index as f64 * line_height,
                    String::new(),
                    font_size,
                    font_family.clone(),
                    fill.clone(),
                    Some(text_anchor.clone()),
                    line_runs,
                    object_id.clone(),
                );
            }
            return;
        }
        for (index, line) in
            split_preserved_text_lines(&payload_string(&object.payload, "text").unwrap_or_default())
                .into_iter()
                .enumerate()
        {
            push_text(
                out,
                tx,
                ty + font_size * 0.82 + index as f64 * line_height,
                line,
                font_size,
                font_family.clone(),
                fill.clone(),
                Some(text_anchor.clone()),
                Vec::new(),
                object_id.clone(),
            );
        }
        return;
    }

    let box_width = payload_box_width(&object.payload, "box").unwrap_or(160.0);
    for (index, line) in wrap_text_lines(
        &payload_string(&object.payload, "text").unwrap_or_default(),
        box_width,
        font_size,
    )
    .into_iter()
    .enumerate()
    {
        push_text(
            out,
            tx,
            ty + font_size * 0.82 + index as f64 * line_height,
            line,
            font_size,
            font_family.clone(),
            fill.clone(),
            Some(text_anchor.clone()),
            Vec::new(),
            object_id.clone(),
        );
    }
}

pub(super) fn render_shape_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
) {
    let [tx, ty] = object.transform.translate;
    let Some([_, _, width, height]) = object.payload.bbox else {
        return;
    };
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    let fill = style.and_then(|value| style_nullable_string(value, "fill"));
    let stroke = style.and_then(|value| style_nullable_string(value, "stroke"));
    let stroke_width = style
        .and_then(|value| {
            style_number(value, "strokeWidth").or_else(|| style_number(value, "stroke_width"))
        })
        .unwrap_or(1.0);
    let dash_array = style
        .and_then(|value| style_number_array(value, "dashArray"))
        .unwrap_or_default();
    let fill_gradient = style
        .and_then(|value| value.get("fillGradient").cloned())
        .filter(|value| !value.is_null());
    let corner_radius =
        payload_number(&object.payload, "cornerRadius").filter(|value| *value > 0.0);

    out.push(RenderPrimitive::Rect {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object.id.clone()),
        x: tx,
        y: ty,
        width,
        height,
        fill,
        stroke,
        stroke_width,
        rx: corner_radius,
        ry: corner_radius,
        dash_array,
        fill_gradient,
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
            push_knockout_polygon(out, polygon, object_id.clone());
        }
    }

    let lines = fragment_label_lines(label);
    if lines.is_empty() {
        return;
    }
    let world_position = fragment_label_position_world(label, object);
    if lines.len() == 1 {
        push_text(
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
        push_text(
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
        );
    }
}
