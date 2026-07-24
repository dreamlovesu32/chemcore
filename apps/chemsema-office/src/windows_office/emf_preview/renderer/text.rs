use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_preview_text(
    dc: HDC,
    x: f64,
    y: f64,
    baseline_offset: Option<f64>,
    text: &str,
    font_size: f64,
    font_family: Option<&str>,
    fill: Option<&str>,
    text_anchor: Option<&str>,
    line_height: Option<f64>,
    runs: &[chemsema_engine::LabelRun],
    transform: &PreviewTransform,
    cache: &mut PreviewGdiCache,
    node_id: Option<&str>,
    label_context: Option<&PreviewLabelContext>,
) {
    let x_nudge_px = preview_attached_label_replay_nudge_px(
        node_id,
        runs,
        fill,
        text_anchor,
        label_context,
        x,
        y,
        baseline_offset,
        font_size,
        transform,
    );
    let effective_font_scale =
        preview_attached_label_replay_font_scale(node_id, runs, fill, text_anchor, label_context);
    let effective_font_size = font_size * effective_font_scale;
    let x = x + x_nudge_px / (transform.scale * transform.record_scale.max(1.0));
    let y_nudge_px = preview_attached_label_replay_y_nudge_px(
        node_id,
        runs,
        fill,
        text_anchor,
        label_context,
        x,
        y,
        baseline_offset,
        effective_font_size,
        transform,
    );
    let old_align = SetTextAlign(dc, TA_LEFT | TA_BASELINE);
    SetBkMode(dc, TRANSPARENT as i32);
    SetTextColor(dc, fill.and_then(colorref_from_css).unwrap_or(0x000000));

    let line_step_world = line_height.unwrap_or(effective_font_size * 1.2).max(0.01);
    let mut lines = preview_text_lines(text, runs);
    preview_scale_text_run_font_sizes(&mut lines, effective_font_scale);
    for (index, line_runs) in lines.iter().enumerate() {
        if line_runs.is_empty() {
            continue;
        }
        let origin = transform.xy(
            x,
            y + y_nudge_px / (transform.scale * transform.record_scale.max(1.0))
                + index as f64 * line_step_world,
        );
        let width = preview_line_width_measured(
            dc,
            line_runs,
            effective_font_size,
            font_family,
            transform,
            cache,
        );
        let mut cursor_x = match text_anchor {
            Some("middle") => origin.x - width / 2,
            Some("end") => origin.x - width,
            _ => origin.x,
        };
        for run in line_runs {
            let dx = preview_script_dx(run, effective_font_size, transform);
            let advance = draw_preview_text_run(
                dc,
                cursor_x + dx,
                origin.y,
                run,
                effective_font_size,
                font_family,
                transform,
                cache,
            );
            cursor_x += dx + advance;
        }
    }

    SetTextAlign(dc, old_align);
}

pub(super) fn preview_packaged_attached_start_layout_mode(
    node_id: Option<&str>,
    fill: &str,
    text_anchor: Option<&str>,
    label_context: Option<&PreviewLabelContext>,
    transform: &PreviewTransform,
) -> PreviewAttachedStartLayoutMode {
    if !transform.emf_recording || !matches!(text_anchor, Some("start")) {
        return PreviewAttachedStartLayoutMode::Default;
    }
    let Some(node_id) = node_id else {
        return PreviewAttachedStartLayoutMode::Default;
    };
    let Some(info) = label_context.and_then(|context| context.infos.get(node_id)) else {
        return PreviewAttachedStartLayoutMode::Default;
    };
    if !matches!(
        info.layout.as_deref(),
        Some("attached-group" | "attached-group-above")
    ) {
        return PreviewAttachedStartLayoutMode::Default;
    }
    if let Some(raw) = std::env::var_os(ENV_PACKAGED_ATTACHED_START_ZERO_LAYOUT) {
        let token = raw.to_string_lossy();
        let token = token.trim();
        if token.is_empty()
            || token.eq_ignore_ascii_case("1")
            || token.eq_ignore_ascii_case("true")
            || token.eq_ignore_ascii_case("all")
        {
            return PreviewAttachedStartLayoutMode::Zero;
        }
        if token.eq_ignore_ascii_case("above-single-black")
            && info.layout.as_deref() == Some("attached-group-above")
            && info.line_count == 1
            && fill.eq_ignore_ascii_case("#000000")
        {
            return PreviewAttachedStartLayoutMode::Zero;
        }
    }
    if preview_env_enabled(ENV_PACKAGED_ATTACHED_START_TIGHT_RECT) {
        return PreviewAttachedStartLayoutMode::Tight;
    }
    PreviewAttachedStartLayoutMode::Default
}

#[allow(clippy::too_many_arguments)]
pub(super) fn preview_attached_label_replay_nudge_px(
    _node_id: Option<&str>,
    _runs: &[chemsema_engine::LabelRun],
    _default_fill: Option<&str>,
    _text_anchor: Option<&str>,
    _label_context: Option<&PreviewLabelContext>,
    _x: f64,
    _y: f64,
    _baseline_offset: Option<f64>,
    _default_font_size: f64,
    _transform: &PreviewTransform,
) -> f64 {
    0.0
}

pub(super) fn preview_attached_label_replay_phase_policy_name() -> Option<String> {
    let raw = std::env::var_os(ENV_ATTACHED_LABEL_REPLAY_PHASE_POLICY_EXPERIMENT);
    let value = raw.as_ref().map(|token| token.to_string_lossy());
    let value = value.as_deref().map(str::trim);
    match value {
        Some(value) => {
            let lowered = value.to_ascii_lowercase();
            if value.is_empty()
                || matches!(
                    lowered.as_str(),
                    "0" | "false" | "off" | "none" | "disabled"
                )
            {
                None
            } else {
                Some(value.to_string())
            }
        }
        None => Some("phase3band".to_string()),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn preview_attached_label_replay_y_nudge_px(
    node_id: Option<&str>,
    runs: &[chemsema_engine::LabelRun],
    default_fill: Option<&str>,
    text_anchor: Option<&str>,
    label_context: Option<&PreviewLabelContext>,
    x: f64,
    y: f64,
    baseline_offset: Option<f64>,
    default_font_size: f64,
    transform: &PreviewTransform,
) -> f64 {
    let mut total = 0.0;
    if let Some(nudge_px) = preview_attached_label_replay_default_family_y_nudge_px(
        node_id,
        runs,
        default_fill,
        text_anchor,
        label_context,
    ) {
        total += nudge_px;
    }
    if let Some(nudge_px) = preview_attached_label_replay_phase_policy_y_nudge_px(
        node_id,
        runs,
        default_fill,
        text_anchor,
        label_context,
        x,
        y,
        baseline_offset,
        default_font_size,
        transform,
    ) {
        total += nudge_px;
    }
    total
}

pub(super) fn preview_attached_label_replay_default_family_y_nudge_px(
    node_id: Option<&str>,
    runs: &[chemsema_engine::LabelRun],
    default_fill: Option<&str>,
    text_anchor: Option<&str>,
    label_context: Option<&PreviewLabelContext>,
) -> Option<f64> {
    if !matches!(text_anchor, Some("start")) {
        return None;
    }
    let node_id = node_id?;
    let info = label_context.and_then(|context| context.infos.get(node_id))?;
    let fill = runs
        .iter()
        .find_map(|run| run.fill.as_deref())
        .or(default_fill)
        .unwrap_or("#000000");
    if !fill.eq_ignore_ascii_case("#000000") {
        return None;
    }
    match info.layout.as_deref() {
        Some("attached-group-above" | "attached-group") if info.line_count > 1 => {
            Some(preview_env_f64_or(
                ENV_DEFAULT_MULTILINE_BLACK_LABEL_Y_NUDGE_PX,
                CHEMDRAW_DEFAULT_MULTILINE_BLACK_LABEL_Y_NUDGE_PX,
            ))
        }
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn preview_attached_label_replay_phase_policy_y_nudge_px(
    node_id: Option<&str>,
    runs: &[chemsema_engine::LabelRun],
    default_fill: Option<&str>,
    text_anchor: Option<&str>,
    label_context: Option<&PreviewLabelContext>,
    x: f64,
    y: f64,
    baseline_offset: Option<f64>,
    default_font_size: f64,
    transform: &PreviewTransform,
) -> Option<f64> {
    let policy = preview_attached_label_replay_phase_policy_name()?;
    if !transform.emf_recording || !matches!(text_anchor, Some("start")) {
        return None;
    }
    let node_id = node_id?;
    let info = label_context.and_then(|context| context.infos.get(node_id))?;
    if info.layout.as_deref() != Some("attached-group") {
        return None;
    }
    let fill = runs
        .iter()
        .find_map(|run| run.fill.as_deref())
        .or(default_fill)
        .unwrap_or("#000000");
    let font_px = (default_font_size * gdiplus_text_scale(transform)).max(1.0) as f32;
    let baseline_top = baseline_offset
        .map(|value| (value * gdiplus_text_scale(transform)) as f32)
        .unwrap_or(font_px * 0.905_273_44);
    let origin = transform.gdip_point(CorePoint { x, y });
    let top_page_phase = (origin.Y - baseline_top).rem_euclid(1.0);
    match policy.as_str() {
        "fillonly" => {
            if fill.eq_ignore_ascii_case("#000000") {
                Some(-1.0)
            } else {
                Some(-2.0)
            }
        }
        "phaseonly" => {
            if top_page_phase < 0.239_101 {
                Some(-2.0)
            } else {
                Some(-1.0)
            }
        }
        "phase3band" => {
            if top_page_phase < 0.239_101 {
                Some(-2.0)
            } else if top_page_phase < 0.564_073 {
                Some(-1.0)
            } else if top_page_phase < 0.631_379 {
                Some(-2.0)
            } else {
                Some(-1.0)
            }
        }
        "threeband" => {
            if fill.eq_ignore_ascii_case("#000000") && top_page_phase < 0.564_073 {
                Some(-1.0)
            } else if top_page_phase < 0.631_379 {
                Some(-2.0)
            } else {
                Some(-1.0)
            }
        }
        _ => {
            if fill.eq_ignore_ascii_case("#000000") && top_page_phase < 0.516_096 {
                Some(-1.0)
            } else {
                Some(-2.0)
            }
        }
    }
}

pub(super) fn preview_attached_label_replay_font_scale(
    _node_id: Option<&str>,
    _runs: &[chemsema_engine::LabelRun],
    _default_fill: Option<&str>,
    _text_anchor: Option<&str>,
    _label_context: Option<&PreviewLabelContext>,
) -> f64 {
    1.0
}

pub(super) fn preview_attached_label_replay_text_hint(
    _node_id: Option<&str>,
    _runs: &[chemsema_engine::LabelRun],
    _default_fill: Option<&str>,
    _text_anchor: Option<&str>,
    _label_context: Option<&PreviewLabelContext>,
) -> Option<i32> {
    None
}

#[allow(clippy::too_many_arguments)]
pub(super) fn preview_attached_label_replay_top_nudge_px(
    _node_id: Option<&str>,
    _runs: &[chemsema_engine::LabelRun],
    _default_fill: Option<&str>,
    _text_anchor: Option<&str>,
    _label_context: Option<&PreviewLabelContext>,
    _x: f64,
    _y: f64,
    _baseline_offset: Option<f64>,
    _default_font_size: f64,
    _transform: &PreviewTransform,
) -> f64 {
    0.0
}

pub(super) fn preview_scale_text_run_font_sizes(lines: &mut [Vec<PreviewTextRun>], scale: f64) {
    if (scale - 1.0).abs() <= f64::EPSILON {
        return;
    }
    for line in lines {
        for run in line {
            if let Some(font_size) = run.font_size.as_mut() {
                *font_size *= scale;
            }
        }
    }
}

pub(super) fn preview_default_gdiplus_text_rendering_hint(transform: &PreviewTransform) -> i32 {
    if transform.emf_recording {
        if preview_env_enabled(ENV_PACKAGED_TEXT_GRIDFIT) {
            TextRenderingHintAntiAliasGridFit
        } else {
            TextRenderingHintAntiAlias
        }
    } else {
        TextRenderingHintAntiAliasGridFit
    }
}

pub(super) fn preview_text_lines(
    text: &str,
    runs: &[chemsema_engine::LabelRun],
) -> Vec<Vec<PreviewTextRun>> {
    if runs.is_empty() {
        return text
            .lines()
            .map(|line| {
                let tighten_advance = line.chars().any(|ch| ch.is_whitespace());
                preview_text_chunks(line)
                    .into_iter()
                    .map(|chunk| PreviewTextRun {
                        text: chunk,
                        font_family: None,
                        font_size: None,
                        fill: None,
                        font_weight: None,
                        font_style: None,
                        underline: None,
                        script: None,
                        tighten_advance,
                    })
                    .collect()
            })
            .collect();
    }

    let tighten_advance = text.chars().any(|ch| ch.is_whitespace());
    let mut lines = vec![Vec::new()];
    for run in runs {
        let segments: Vec<&str> = run.text.split('\n').collect();
        for (index, segment) in segments.iter().enumerate() {
            if !segment.is_empty() {
                for chunk in preview_text_chunks(segment) {
                    lines.last_mut().expect("line exists").push(PreviewTextRun {
                        text: chunk,
                        font_family: run.font_family.clone(),
                        font_size: run.font_size,
                        fill: run.fill.clone(),
                        font_weight: run.font_weight,
                        font_style: run.font_style.clone(),
                        underline: run.underline,
                        script: run.script.clone(),
                        tighten_advance,
                    });
                }
            }
            if index + 1 < segments.len() {
                lines.push(Vec::new());
            }
        }
    }
    lines
}

pub(super) fn preview_text_chunks(segment: &str) -> Vec<String> {
    if segment.is_empty() {
        return Vec::new();
    }
    let mut chunks = Vec::new();
    let mut cursor = 0usize;
    while cursor < segment.len() {
        let leading_start = cursor;
        while let Some(ch) = segment[cursor..].chars().next() {
            if !ch.is_whitespace() {
                break;
            }
            cursor += ch.len_utf8();
            if cursor >= segment.len() {
                break;
            }
        }
        if cursor > leading_start {
            chunks.push(segment[leading_start..cursor].to_string());
            if cursor >= segment.len() {
                break;
            }
        }

        let token_start = cursor;
        while let Some(ch) = segment[cursor..].chars().next() {
            if ch.is_whitespace() {
                break;
            }
            cursor += ch.len_utf8();
            if cursor >= segment.len() {
                break;
            }
        }
        if cursor <= token_start {
            break;
        }

        let whitespace_start = cursor;
        while let Some(ch) = segment[cursor..].chars().next() {
            if !ch.is_whitespace() {
                break;
            }
            cursor += ch.len_utf8();
            if cursor >= segment.len() {
                break;
            }
        }

        if whitespace_start == cursor {
            chunks.push(segment[token_start..cursor].to_string());
            continue;
        }

        let first_whitespace_end = whitespace_start
            + segment[whitespace_start..cursor]
                .chars()
                .next()
                .map(|ch| ch.len_utf8())
                .unwrap_or(0);
        chunks.push(segment[token_start..first_whitespace_end].to_string());
        if first_whitespace_end < cursor {
            chunks.push(segment[first_whitespace_end..cursor].to_string());
        }
    }
    chunks
}

pub(super) unsafe fn preview_line_width_measured(
    dc: HDC,
    runs: &[PreviewTextRun],
    default_font_size: f64,
    default_family: Option<&str>,
    transform: &PreviewTransform,
    cache: &mut PreviewGdiCache,
) -> i32 {
    runs.iter()
        .map(|run| {
            preview_text_run_extent(dc, run, default_font_size, default_family, transform, cache)
        })
        .sum()
}

pub(super) fn preview_line_width_f32(
    runs: &[PreviewTextRun],
    default_font_size: f64,
    transform: &PreviewTransform,
) -> f32 {
    runs.iter()
        .map(|run| preview_text_run_advance_estimate_f32(run, default_font_size, transform))
        .sum()
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_preview_text_run(
    dc: HDC,
    x: i32,
    baseline_y: i32,
    run: &PreviewTextRun,
    default_font_size: f64,
    default_family: Option<&str>,
    transform: &PreviewTransform,
    cache: &mut PreviewGdiCache,
) -> i32 {
    let label: Vec<u16> = run.text.encode_utf16().collect();
    if label.is_empty() {
        return 0;
    }
    let font = cache.font_for_run(run, default_font_size, default_family, transform);
    let old_font = select_preview_font(dc, font);
    let text_color = run
        .fill
        .as_deref()
        .and_then(colorref_from_css)
        .unwrap_or(0x000000);
    SetTextColor(dc, text_color);
    let script_shift = preview_script_baseline_shift(run, default_font_size, transform);
    let advance = if run.tighten_advance {
        preview_text_extent(dc, &label, true)
    } else {
        preview_structure_label_extent(dc, run, default_font_size, default_family, transform)
    }
    .unwrap_or_else(|| preview_text_run_advance_estimate(run, default_font_size, transform));
    if run.tighten_advance {
        if let Some(dx) = preview_text_dx_array(dc, &label, true) {
            ExtTextOutW(
                dc,
                x,
                baseline_y + script_shift,
                0,
                null(),
                label.as_ptr(),
                label.len() as u32,
                dx.as_ptr(),
            );
        } else {
            TextOutW(
                dc,
                x,
                baseline_y + script_shift,
                label.as_ptr(),
                label.len() as i32,
            );
        }
    } else {
        TextOutW(
            dc,
            x,
            baseline_y + script_shift,
            label.as_ptr(),
            label.len() as i32,
        );
    }
    restore_preview_font(dc, old_font);
    advance
}

pub(super) unsafe fn preview_text_run_extent(
    dc: HDC,
    run: &PreviewTextRun,
    default_font_size: f64,
    default_family: Option<&str>,
    transform: &PreviewTransform,
    cache: &mut PreviewGdiCache,
) -> i32 {
    let label: Vec<u16> = run.text.encode_utf16().collect();
    if label.is_empty() {
        return 0;
    }
    let font = cache.font_for_run(run, default_font_size, default_family, transform);
    let old_font = select_preview_font(dc, font);
    let advance = if run.tighten_advance {
        preview_text_extent(dc, &label, true)
    } else {
        preview_structure_label_extent(dc, run, default_font_size, default_family, transform)
    }
    .unwrap_or_else(|| preview_text_run_advance_estimate(run, default_font_size, transform));
    restore_preview_font(dc, old_font);
    advance
}

pub(super) unsafe fn select_preview_font(dc: HDC, font: HGDIOBJ) -> HGDIOBJ {
    if font.is_null() {
        null_mut()
    } else {
        SelectObject(dc, font as HGDIOBJ)
    }
}

pub(super) unsafe fn restore_preview_font(dc: HDC, old_font: HGDIOBJ) {
    if !old_font.is_null() {
        SelectObject(dc, old_font);
    }
}

pub(super) unsafe fn preview_text_extent(
    dc: HDC,
    label: &[u16],
    tighten_advance: bool,
) -> Option<i32> {
    if tighten_advance {
        if let Some(dx) = preview_text_dx_array(dc, label, true) {
            return Some(dx.iter().sum::<i32>().max(0));
        }
    }
    let mut size = SIZE { cx: 0, cy: 0 };
    if GetTextExtentPoint32W(dc, label.as_ptr(), label.len() as i32, &mut size) == 0 {
        None
    } else {
        Some(size.cx.max(0))
    }
}

pub(super) unsafe fn preview_text_dx_array(
    dc: HDC,
    label: &[u16],
    tighten_advance: bool,
) -> Option<Vec<i32>> {
    if label.is_empty() {
        return Some(Vec::new());
    }
    let mut size = SIZE { cx: 0, cy: 0 };
    let mut fit = 0i32;
    let mut partial = vec![0i32; label.len()];
    if GetTextExtentExPointW(
        dc,
        label.as_ptr(),
        label.len() as i32,
        i32::MAX,
        &mut fit,
        partial.as_mut_ptr(),
        &mut size,
    ) == 0
        || fit != label.len() as i32
    {
        return None;
    }
    let tighten = if tighten_advance {
        CHEMDRAW_GDI_TEXT_ADVANCE_TIGHTEN
    } else {
        1.0
    };
    let mut dx = Vec::with_capacity(label.len());
    let mut previous_scaled = 0i32;
    for cumulative in partial {
        let scaled_cumulative = ((cumulative as f64) * tighten).round() as i32;
        let scaled_step = (scaled_cumulative - previous_scaled).max(0).max(1);
        previous_scaled = scaled_cumulative;
        dx.push(scaled_step);
    }
    Some(dx)
}

pub(super) unsafe fn preview_structure_label_extent(
    dc: HDC,
    run: &PreviewTextRun,
    default_font_size: f64,
    default_family: Option<&str>,
    transform: &PreviewTransform,
) -> Option<i32> {
    let dx =
        preview_structure_label_dx_array(dc, run, default_font_size, default_family, transform)?;
    Some(dx.iter().sum::<i32>().max(0))
}

pub(super) unsafe fn preview_structure_label_dx_array(
    _dc: HDC,
    run: &PreviewTextRun,
    default_font_size: f64,
    default_family: Option<&str>,
    transform: &PreviewTransform,
) -> Option<Vec<i32>> {
    if run.text.is_empty() {
        return Some(Vec::new());
    }
    let wide: Vec<u16> = run.text.encode_utf16().collect();
    let font = create_gdiplus_font(run, default_font_size, default_family, transform)?;
    let Some(format) = create_gdiplus_string_format() else {
        GdipDeleteFont(font);
        return None;
    };
    let measure_dc = CreateCompatibleDC(null_mut());
    if measure_dc.is_null() {
        GdipDeleteStringFormat(format);
        GdipDeleteFont(font);
        return None;
    }
    let mut graphics: *mut GpGraphics = null_mut();
    if GdipCreateFromHDC(measure_dc, &mut graphics) != GDI_PLUS_OK || graphics.is_null() {
        DeleteDC(measure_dc);
        GdipDeleteStringFormat(format);
        GdipDeleteFont(font);
        return None;
    }
    GdipSetTextRenderingHint(graphics, TextRenderingHintAntiAliasGridFit);
    let mut dx = Vec::with_capacity(wide.len());
    let mut previous = 0i32;
    for end in 1..=wide.len() {
        let Some(width) = gdiplus_measure_text_width(
            graphics,
            font,
            format,
            &wide[..end],
            run,
            default_font_size,
            transform,
        ) else {
            GdipDeleteGraphics(graphics);
            DeleteDC(measure_dc);
            GdipDeleteStringFormat(format);
            GdipDeleteFont(font);
            return None;
        };
        let cumulative = width.round().max(previous as f32) as i32;
        let step = (cumulative - previous).max(1);
        previous = cumulative;
        dx.push(step);
    }
    GdipDeleteGraphics(graphics);
    DeleteDC(measure_dc);
    GdipDeleteStringFormat(format);
    GdipDeleteFont(font);
    Some(dx)
}

pub(super) fn preview_text_run_advance_estimate(
    run: &PreviewTextRun,
    default_font_size: f64,
    transform: &PreviewTransform,
) -> i32 {
    let script_scale = preview_script_scale(run.script.as_deref());
    let font_size = run.font_size.unwrap_or(default_font_size) * script_scale;
    let world_width: f64 = run
        .text
        .chars()
        .map(|character| preview_char_advance_em(character) * font_size)
        .sum();
    (world_width * transform.scale).round().max(0.0) as i32
}

pub(super) fn preview_text_run_advance_estimate_f32(
    run: &PreviewTextRun,
    default_font_size: f64,
    transform: &PreviewTransform,
) -> f32 {
    let script_scale = preview_script_scale(run.script.as_deref());
    let font_size = run.font_size.unwrap_or(default_font_size) * script_scale;
    let world_width: f64 = run
        .text
        .chars()
        .map(|character| preview_char_advance_em(character) * font_size)
        .sum();
    (world_width * transform.scale).max(0.0) as f32
}

pub(super) fn preview_font_key(
    run: &PreviewTextRun,
    default_font_size: f64,
    default_family: Option<&str>,
    transform: &PreviewTransform,
) -> PreviewFontKey {
    let script_scale = preview_script_scale(run.script.as_deref());
    let font_size = run.font_size.unwrap_or(default_font_size) * script_scale;
    PreviewFontKey {
        height: transform.length(font_size).max(1),
        family: run
            .font_family
            .as_deref()
            .or(default_family)
            .unwrap_or("Arial")
            .to_string(),
        weight: run.font_weight.unwrap_or(400).clamp(100, 900) as i32,
        italic: run.font_style.as_deref() == Some("italic"),
        underline: run.underline.unwrap_or(false),
    }
}

pub(super) unsafe fn create_preview_font(key: &PreviewFontKey) -> HGDIOBJ {
    let family = wide_null(&key.family);
    CreateFontW(
        -key.height,
        0,
        0,
        0,
        key.weight,
        key.italic as u32,
        key.underline as u32,
        0,
        0,
        OUT_TT_ONLY_PRECIS_VALUE,
        0,
        ANTIALIASED_QUALITY as u32,
        0,
        family.as_ptr(),
    ) as HGDIOBJ
}

pub(super) fn preview_script_baseline_shift(
    run: &PreviewTextRun,
    default_font_size: f64,
    transform: &PreviewTransform,
) -> i32 {
    let base_height = transform.length(run.font_size.unwrap_or(default_font_size));
    (base_height as f64 * preview_script_baseline_shift_em(run)).round() as i32
}

pub(super) fn preview_script_baseline_shift_f32(
    run: &PreviewTextRun,
    default_font_size: f64,
    transform: &PreviewTransform,
) -> f32 {
    let base_height = run.font_size.unwrap_or(default_font_size) * gdiplus_text_scale(transform);
    (base_height * preview_script_baseline_shift_em(run)) as f32
}

pub(super) fn preview_script_baseline_shift_em(run: &PreviewTextRun) -> f64 {
    match run.script.as_deref() {
        Some("subscript") if run.font_weight.unwrap_or(400) >= 600 => {
            CHEMDRAW_BOLD_SUBSCRIPT_SHIFT_DOWN_EM
        }
        Some("subscript") => CHEMDRAW_SUBSCRIPT_SHIFT_DOWN_EM,
        Some("superscript") => -CHEMDRAW_SUPERSCRIPT_SHIFT_UP_EM,
        _ => 0.0,
    }
}

pub(super) fn preview_script_dx(
    run: &PreviewTextRun,
    default_font_size: f64,
    transform: &PreviewTransform,
) -> i32 {
    preview_script_dx_f64(run, default_font_size, transform)
        .round()
        .clamp(i32::MIN as f64, i32::MAX as f64) as i32
}

pub(super) fn preview_script_dx_f32(
    run: &PreviewTextRun,
    default_font_size: f64,
    transform: &PreviewTransform,
) -> f32 {
    preview_script_dx_f64(run, default_font_size, transform) as f32
}

pub(super) fn preview_script_dx_f64(
    run: &PreviewTextRun,
    default_font_size: f64,
    transform: &PreviewTransform,
) -> f64 {
    if run.script.as_deref() != Some("superscript") {
        return 0.0;
    }
    let font_size =
        run.font_size.unwrap_or(default_font_size) * preview_script_scale(run.script.as_deref());
    -0.02 * font_size * transform.scale * transform.record_scale
}

pub(super) fn preview_script_scale(script: Option<&str>) -> f64 {
    match script {
        Some("subscript" | "superscript") => CHEMDRAW_SCRIPT_SCALE,
        _ => 1.0,
    }
}

pub(super) fn preview_char_advance_em(character: char) -> f64 {
    match character {
        ' ' | '\t' => 0.32,
        'i' | 'l' | 'I' | '!' | '|' => 0.28,
        'f' | 'j' | 'r' | 't' | ',' | '.' | ':' | ';' => 0.34,
        '(' | ')' | '[' | ']' | '{' | '}' => 0.36,
        'M' | 'W' => 0.86,
        'm' | 'w' => 0.78,
        '0'..='9' => 0.56,
        'A'..='Z' => 0.68,
        '+' | '-' | '=' | '/' | '\\' => 0.55,
        _ if character.is_ascii() => 0.52,
        _ => 0.9,
    }
}

pub(super) fn ansi_metafile_text_bytes(text: &str) -> Vec<u8> {
    const CP_ACP: u32 = 0;
    let wide: Vec<u16> = text.encode_utf16().collect();
    if wide.is_empty() {
        return Vec::new();
    }
    unsafe {
        let needed = WideCharToMultiByte(
            CP_ACP,
            0,
            wide.as_ptr(),
            wide.len() as i32,
            null_mut(),
            0,
            null(),
            null_mut(),
        );
        if needed <= 0 {
            return text
                .chars()
                .map(|ch| if ch.is_ascii() { ch as u8 } else { b'?' })
                .collect();
        }
        let mut out = vec![0u8; needed as usize];
        let written = WideCharToMultiByte(
            CP_ACP,
            0,
            wide.as_ptr(),
            wide.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
            null(),
            null_mut(),
        );
        if written <= 0 {
            Vec::new()
        } else {
            out.truncate(written as usize);
            out
        }
    }
}

pub(super) fn preview_pen_style(
    line_cap: Option<&str>,
    line_join: Option<&str>,
    style: i32,
) -> u32 {
    let cap = match line_cap {
        Some("round") => PS_ENDCAP_ROUND,
        Some("square") => PS_ENDCAP_SQUARE,
        _ => PS_ENDCAP_FLAT,
    };
    let join = match line_join {
        Some("round") => PS_JOIN_ROUND,
        Some("bevel") => PS_JOIN_BEVEL,
        _ => PS_JOIN_MITER,
    };
    (PS_GEOMETRIC | style | cap | join) as u32
}

pub(super) fn preview_dash_style(dash_array: &[f64], transform: &PreviewTransform) -> Vec<u32> {
    dash_array
        .iter()
        .copied()
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| transform.length(value).max(1) as u32)
        .collect()
}

pub(super) unsafe fn create_preview_pen(
    color: COLORREF,
    width: i32,
    line_cap: Option<&str>,
    line_join: Option<&str>,
    dash_array: &[f64],
    transform: &PreviewTransform,
) -> HGDIOBJ {
    if width <= 0 {
        return GetStockObject(NULL_PEN);
    }
    let mut dash_style = preview_dash_style(dash_array, transform);
    let pen_style = if dash_style.is_empty() {
        PS_SOLID
    } else {
        if dash_style.len() % 2 == 1 {
            dash_style.extend_from_within(..);
        }
        dash_style.truncate(16);
        PS_USERSTYLE
    };
    let brush = LOGBRUSH {
        lbStyle: BS_SOLID,
        lbColor: color,
        lbHatch: 0,
    };
    let pen = ExtCreatePen(
        preview_pen_style(line_cap, line_join, pen_style),
        width.max(1) as u32,
        &brush,
        dash_style.len() as u32,
        if dash_style.is_empty() {
            null()
        } else {
            dash_style.as_ptr()
        },
    );
    if pen.is_null() {
        CreatePen(PS_SOLID, width.max(1), color) as HGDIOBJ
    } else {
        pen as HGDIOBJ
    }
}

pub(super) unsafe fn set_preview_miter_limit(dc: HDC) {
    SetMiterLimit(dc, PREVIEW_MITER_LIMIT, null_mut());
}

pub(super) unsafe fn delete_preview_pen(pen: HGDIOBJ) {
    if pen != GetStockObject(NULL_PEN) {
        DeleteObject(pen);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_preview_line(
    dc: HDC,
    from: POINT,
    to: POINT,
    color: &str,
    stroke_width: f64,
    line_cap: Option<&str>,
    line_join: Option<&str>,
    transform: &PreviewTransform,
    dash_array: &[f64],
) {
    let points = [from, to];
    draw_preview_polyline_points(
        dc,
        &points,
        color,
        stroke_width,
        line_cap,
        line_join,
        transform,
        dash_array,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_preview_polyline(
    dc: HDC,
    points: &[CorePoint],
    color: &str,
    stroke_width: f64,
    line_cap: Option<&str>,
    line_join: Option<&str>,
    transform: &PreviewTransform,
    dash_array: &[f64],
) {
    if points.len() < 2 {
        return;
    }
    let mapped: Vec<POINT> = points.iter().map(|point| transform.point(*point)).collect();
    draw_preview_polyline_points(
        dc,
        &mapped,
        color,
        stroke_width,
        line_cap,
        line_join,
        transform,
        dash_array,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_preview_polyline_points(
    dc: HDC,
    points: &[POINT],
    color: &str,
    stroke_width: f64,
    line_cap: Option<&str>,
    line_join: Option<&str>,
    transform: &PreviewTransform,
    dash_array: &[f64],
) {
    if points.len() < 2 {
        return;
    }
    let pen = create_preview_pen(
        colorref_from_css(color).unwrap_or(0x000000),
        transform.pen_width(stroke_width),
        line_cap,
        line_join,
        dash_array,
        transform,
    );
    let old_pen = SelectObject(dc, pen as HGDIOBJ);
    set_preview_miter_limit(dc);
    Polyline(dc, points.as_ptr(), points.len() as i32);
    SelectObject(dc, old_pen);
    delete_preview_pen(pen);
}
