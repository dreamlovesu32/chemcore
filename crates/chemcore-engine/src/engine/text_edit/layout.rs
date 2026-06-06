use super::*;

#[derive(Clone)]
pub(super) struct ResolvedTextEditLine {
    x: f64,
    y: f64,
    baseline_y: f64,
    height: f64,
    text_anchor: String,
    runs: Vec<LabelRun>,
}

#[derive(Clone)]
pub(super) struct ResolvedTextEditCharBox {
    offset: usize,
    x: f64,
    width: f64,
    height: f64,
    line_y: f64,
    line_height: f64,
    line_index: usize,
}

pub(super) fn normalize_text_edit_selection(
    text: &str,
    selection: Option<&TextEditSelection>,
) -> Option<TextEditSelectionState> {
    let Some(selection) = selection else {
        return None;
    };
    let text_length = text.chars().count();
    let anchor = selection.anchor.min(text_length);
    let focus = selection.focus.min(text_length);
    Some(TextEditSelectionState {
        anchor,
        focus,
        start: anchor.min(focus),
        end: anchor.max(focus),
        collapsed: anchor == focus,
    })
}

pub(super) fn split_runs_by_line_preserving_empty(runs: &[LabelRun]) -> Vec<Vec<LabelRun>> {
    let mut lines = vec![Vec::new()];
    for run in runs {
        let segments: Vec<&str> = run.text.split('\n').collect();
        for (index, segment) in segments.iter().enumerate() {
            if !segment.is_empty() {
                let mut next_run = run.clone();
                next_run.text = (*segment).to_string();
                lines
                    .last_mut()
                    .expect("line vector always exists")
                    .push(next_run);
            }
            if index + 1 < segments.len() {
                lines.push(Vec::new());
            }
        }
    }
    if lines.is_empty() {
        vec![Vec::new()]
    } else {
        lines
    }
}

pub(super) fn text_anchor_for_align(align: &str) -> String {
    match align {
        "right" => "end".to_string(),
        "center" => "middle".to_string(),
        _ => "start".to_string(),
    }
}

pub(super) fn anchor_x_for_align(align: &str, width: f64) -> f64 {
    match align {
        "right" => width,
        "center" => width * 0.5,
        _ => 0.0,
    }
}

pub(super) fn text_object_box_for_align(align: &str, width: f64, height: f64) -> [f64; 4] {
    [-anchor_x_for_align(align, width), 0.0, width, height]
}

pub(super) fn measure_text_edit_line_width(runs: &[LabelRun], fallback_font_size: f64) -> f64 {
    runs.iter().fold(0.0, |width, run| {
        let run_font_size = run.font_size.unwrap_or(fallback_font_size);
        width
            + run
                .text
                .chars()
                .map(|character| {
                    crate::shared_glyph_metrics(character, run_font_size, run.script.as_deref())
                        .advance
                })
                .sum::<f64>()
    })
}

pub(super) fn build_text_edit_layout_geometry(
    text: String,
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    lines: Vec<ResolvedTextEditLine>,
    width: f64,
    height: f64,
    line_height: f64,
    anchor_offset: [f64; 2],
    selection: Option<TextEditSelectionState>,
    fallback_font_size: f64,
) -> TextEditLayout {
    let mut layout_lines = Vec::new();
    let mut caret_positions = Vec::new();
    let mut char_boxes = Vec::new();
    let mut offset = 0usize;

    for (line_index, line) in lines.iter().enumerate() {
        let mut caret_offsets = Vec::new();
        let line_start = offset;
        let line_width = measure_text_edit_line_width(&line.runs, fallback_font_size);
        let visual_line_x = match line.text_anchor.as_str() {
            "end" => line.x - line_width,
            "middle" => line.x - line_width * 0.5,
            _ => line.x,
        };
        let mut cursor_x = visual_line_x;
        let caret_y = line.y;
        let caret_height = line.height.max(0.0);
        let start_caret = TextEditLayoutCaret {
            offset,
            x: round6(cursor_x),
            y: round6(caret_y),
            height: round6(caret_height),
            line_index,
        };
        caret_offsets.push(TextEditLayoutCaretOffset {
            offset,
            x: round6(cursor_x),
        });
        caret_positions.push(start_caret);

        for run in &line.runs {
            let run_font_size = run.font_size.unwrap_or(fallback_font_size);
            for character in run.text.chars() {
                let metrics =
                    crate::shared_glyph_metrics(character, run_font_size, run.script.as_deref());
                let char_top = line.baseline_y + metrics.top;
                let char_bottom = line.baseline_y + metrics.bottom;
                char_boxes.push(ResolvedTextEditCharBox {
                    offset,
                    x: cursor_x,
                    width: metrics.advance,
                    height: (char_bottom - char_top).max(0.0),
                    line_y: line.y,
                    line_height: line.height,
                    line_index,
                });
                cursor_x += metrics.advance;
                offset += 1;
                caret_offsets.push(TextEditLayoutCaretOffset {
                    offset,
                    x: round6(cursor_x),
                });
                caret_positions.push(TextEditLayoutCaret {
                    offset,
                    x: round6(cursor_x),
                    y: round6(caret_y),
                    height: round6(caret_height),
                    line_index,
                });
            }
        }

        let line_end = offset;
        layout_lines.push(TextEditLayoutLine {
            index: line_index,
            x: round6(line.x),
            y: round6(line.y),
            baseline_y: round6(line.baseline_y),
            height: round6(line.height),
            start_offset: line_start,
            end_offset: line_end,
            text_anchor: line.text_anchor.clone(),
            runs: line.runs.clone(),
            caret_offsets,
        });
        if line_index + 1 < lines.len() {
            offset += 1;
        }
    }

    let selection_rects = build_text_edit_selection_rects(&char_boxes, selection.as_ref());
    TextEditLayout {
        text,
        source_runs,
        display_runs,
        lines: layout_lines,
        width: round6(width),
        height: round6(height),
        line_height: round6(line_height),
        anchor_offset: [round6(anchor_offset[0]), round6(anchor_offset[1])],
        caret_positions,
        selection_rects,
        selection,
    }
}

pub(super) fn build_text_edit_selection_rects(
    char_boxes: &[ResolvedTextEditCharBox],
    selection: Option<&TextEditSelectionState>,
) -> Vec<TextEditLayoutRect> {
    let Some(selection) = selection else {
        return Vec::new();
    };
    if selection.collapsed {
        return Vec::new();
    }
    let mut grouped: Vec<(usize, TextEditLayoutRect)> = Vec::new();
    for entry in char_boxes {
        if entry.offset < selection.start || entry.offset >= selection.end {
            continue;
        }
        if let Some((_, current)) = grouped
            .iter_mut()
            .find(|(line_index, _)| *line_index == entry.line_index)
        {
            current.x = current.x.min(entry.x);
            current.y = current.y.min(entry.line_y);
            current.width = current.width.max(entry.x + entry.width - current.x);
            current.height = current.height.max(entry.line_height);
            continue;
        }
        grouped.push((
            entry.line_index,
            TextEditLayoutRect {
                x: entry.x,
                y: entry.line_y,
                width: entry.width.max(0.0),
                height: entry.line_height.max(entry.height).max(0.0),
            },
        ));
    }
    grouped
        .into_iter()
        .map(|(_, rect)| TextEditLayoutRect {
            x: round6(rect.x),
            y: round6(rect.y),
            width: round6(rect.width.max(0.0)),
            height: round6(rect.height.max(0.0)),
        })
        .collect()
}

pub(super) fn build_text_object_edit_layout(
    session: &TextEditSession,
    text: String,
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    line_height: f64,
    selection: Option<TextEditSelectionState>,
) -> TextEditLayout {
    let fallback_font_size = session
        .font_size_world_pt()
        .unwrap_or(WorldPt(DEFAULT_TEXT_FONT_SIZE))
        .value();
    let align = session.align.as_deref().unwrap_or("left");
    let line_runs = split_runs_by_line_preserving_empty(&display_runs);
    let line_widths: Vec<f64> = line_runs
        .iter()
        .map(|runs| measure_text_edit_line_width(runs, fallback_font_size))
        .collect();
    let session_box = session.box_value;
    let box_width = session_box.map(|bbox| bbox[2].max(0.0)).unwrap_or(0.0);
    let box_height = session_box.map(|bbox| bbox[3].max(0.0)).unwrap_or(0.0);
    let width = round2(
        line_widths
            .iter()
            .copied()
            .fold(TEXT_EDIT_BOX_WIDTH.max(box_width), f64::max),
    );
    let height = round2(
        (line_height * line_runs.len().max(1) as f64)
            .max(line_height)
            .max(box_height),
    );
    let text_anchor = text_anchor_for_align(align);
    let local_box = text_object_box_for_align(align, width, height);
    let lines = line_runs
        .into_iter()
        .enumerate()
        .map(|(index, runs)| {
            let y = index as f64 * line_height;
            ResolvedTextEditLine {
                x: anchor_x_for_align(align, width),
                y,
                baseline_y: y + fallback_font_size * 0.82,
                height: line_height,
                text_anchor: text_anchor.clone(),
                runs,
            }
        })
        .collect();
    build_text_edit_layout_geometry(
        text,
        source_runs,
        display_runs,
        lines,
        width,
        height,
        line_height,
        [-local_box[0], -local_box[1]],
        selection,
        fallback_font_size,
    )
}

pub(super) fn build_endpoint_label_edit_layout_from_label(
    text: String,
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    label: &crate::NodeLabel,
    local_anchor: [f64; 2],
    line_height: f64,
    selection: Option<TextEditSelectionState>,
) -> TextEditLayout {
    let fallback_font_size = label.font_size.unwrap_or(DEFAULT_TEXT_FONT_SIZE);
    let box_value = label.bbox().unwrap_or([
        local_anchor[0],
        local_anchor[1] - fallback_font_size * 0.42,
        local_anchor[0] + TEXT_EDIT_BOX_WIDTH,
        local_anchor[1] - fallback_font_size * 0.42 + line_height,
    ]);
    let baseline_x = label.position.map(|value| value[0]).unwrap_or(box_value[0]);
    let first_baseline_y = label
        .position
        .map(|value| value[1])
        .unwrap_or(box_value[1] + fallback_font_size * 0.82);
    let editor_origin_x = baseline_x;
    let editor_origin_y = box_value[1];
    let width = round2((box_value[2] - editor_origin_x).max(TEXT_EDIT_BOX_WIDTH));
    let height = round2((box_value[3] - editor_origin_y).max(line_height));
    let line_runs = if !label.line_runs.is_empty() {
        label.line_runs.clone()
    } else {
        vec![label.runs.clone()]
    };
    let lines = line_runs
        .into_iter()
        .enumerate()
        .map(|(index, runs)| {
            let y = if index == 0 {
                0.0
            } else {
                index as f64 * line_height
            };
            let baseline_y = if index == 0 {
                first_baseline_y - editor_origin_y
            } else {
                y + line_height * 0.82
            };
            ResolvedTextEditLine {
                x: 0.0,
                y,
                baseline_y,
                height: line_height,
                text_anchor: "start".to_string(),
                runs,
            }
        })
        .collect();
    build_text_edit_layout_geometry(
        text,
        source_runs,
        display_runs,
        lines,
        width,
        height,
        line_height,
        [
            local_anchor[0] - editor_origin_x,
            local_anchor[1] - editor_origin_y,
        ],
        selection,
        fallback_font_size,
    )
}

pub(super) fn estimate_text_block_size(
    runs: &[LabelRun],
    font_size: f64,
    line_height: f64,
) -> (f64, f64) {
    let mut max_width = font_size * 0.6;
    let mut line_width = 0.0;
    let mut line_count = 1usize;

    for run in runs {
        let script_scale = crate::glyph_kernel::shared_script_scale_factor(run.script.as_deref());
        let run_font_size = run.font_size.unwrap_or(font_size) * script_scale;
        for character in run.text.chars() {
            if character == '\n' {
                max_width = max_width.max(line_width);
                line_width = 0.0;
                line_count += 1;
                continue;
            }
            line_width += estimated_char_width(character, run_font_size);
        }
    }
    max_width = max_width.max(line_width);
    let width = round2((max_width + font_size * 0.24).max(crate::px_to_pt(8.0)));
    let height = round2((line_height * line_count as f64).max(line_height));
    (width, height)
}

pub(super) fn estimated_char_width(character: char, font_size: f64) -> f64 {
    crate::glyph_kernel::shared_estimated_char_width(character, font_size)
}
