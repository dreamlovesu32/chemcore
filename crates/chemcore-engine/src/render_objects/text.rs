use super::*;

fn split_runs_by_line(runs: &[LabelRun]) -> Vec<Vec<LabelRun>> {
    let mut out = vec![Vec::new()];
    for run in runs {
        let segments: Vec<&str> = run.text.split('\n').collect();
        for (index, segment) in segments.iter().enumerate() {
            if !segment.is_empty() {
                let mut next_run = run.clone();
                next_run.text = (*segment).to_string();
                out.last_mut()
                    .expect("line vector always exists")
                    .push(next_run);
            }
            if index + 1 < segments.len() {
                out.push(Vec::new());
            }
        }
    }
    out
}

fn split_preserved_text_lines(text: &str) -> Vec<String> {
    text.split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn wrap_text_lines(text: &str, max_width: f64, font_size: f64) -> Vec<String> {
    let raw_lines: Vec<&str> = text.split('\n').collect();
    let max_chars = (max_width
        / crate::TEXT_WRAP_ESTIMATED_CHAR_WIDTH_PT
            .value()
            .max(font_size * 0.6))
    .floor()
    .max(8.0) as usize;
    let mut out = Vec::new();

    for raw_line in raw_lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.len() <= max_chars || !line.contains(' ') {
            out.push(line.to_string());
            continue;
        }
        let mut current = String::new();
        for word in line.split_whitespace() {
            let next = if current.is_empty() {
                word.to_string()
            } else {
                format!("{current} {word}")
            };
            if next.len() > max_chars && !current.is_empty() {
                out.push(current);
                current = word.to_string();
            } else {
                current = next;
            }
        }
        if !current.is_empty() {
            out.push(current);
        }
    }

    out
}

pub(crate) fn render_text_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
) {
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    let rotate = object.transform.rotate;
    let rotate_center = (rotate.abs() > crate::EPSILON).then_some(Point::new(tx, ty));
    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    let Some(font_size) = payload_number(&object.payload, "fontSize") else {
        return;
    };
    let Some(line_height) = payload_number(&object.payload, "lineHeight") else {
        return;
    };
    let Some(align) = payload_string(&object.payload, "align") else {
        return;
    };
    let text_anchor = text_anchor(&align);
    let font_family = style
        .and_then(|value| style_string(value, "fontFamily"))
        .or_else(|| Some("Arial".to_string()));
    let fill = style.and_then(|value| style_string(value, "fill"));
    let object_id = Some(object.id.clone());

    let Some(preserve_lines) = payload_bool(&object.payload, "preserveLines") else {
        return;
    };
    if preserve_lines {
        let baseline_offset =
            payload_number(&object.payload, "baselineOffset").unwrap_or(font_size * 0.82);
        let runs = payload_runs(&object.payload, "runs");
        if !runs.is_empty() {
            for (index, line_runs) in split_runs_by_line(&runs).into_iter().enumerate() {
                if line_runs.is_empty() {
                    continue;
                }
                push_text_rotated(
                    out,
                    tx,
                    ty + baseline_offset + index as f64 * line_height,
                    Some(baseline_offset),
                    String::new(),
                    font_size,
                    font_family.clone(),
                    fill.clone(),
                    Some(text_anchor.clone()),
                    line_runs,
                    object_id.clone(),
                    rotate,
                    rotate_center,
                );
            }
            return;
        }
        for (index, line) in
            split_preserved_text_lines(&payload_string(&object.payload, "text").unwrap_or_default())
                .into_iter()
                .enumerate()
        {
            push_text_rotated(
                out,
                tx,
                ty + baseline_offset + index as f64 * line_height,
                Some(baseline_offset),
                line,
                font_size,
                font_family.clone(),
                fill.clone(),
                Some(text_anchor.clone()),
                Vec::new(),
                object_id.clone(),
                rotate,
                rotate_center,
            );
        }
        return;
    }

    let Some(box_width) = payload_box_width(&object.payload, "box") else {
        return;
    };
    for (index, line) in wrap_text_lines(
        &payload_string(&object.payload, "text").unwrap_or_default(),
        box_width,
        font_size,
    )
    .into_iter()
    .enumerate()
    {
        push_text_rotated(
            out,
            tx,
            ty + font_size * 0.82 + index as f64 * line_height,
            Some(font_size * 0.82),
            line,
            font_size,
            font_family.clone(),
            fill.clone(),
            Some(text_anchor.clone()),
            Vec::new(),
            object_id.clone(),
            rotate,
            rotate_center,
        );
    }
}
