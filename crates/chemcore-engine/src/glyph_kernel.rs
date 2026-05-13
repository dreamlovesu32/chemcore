use crate::LabelRun;
use serde::Deserialize;
use std::collections::HashMap;
use std::f64::consts::TAU;
use std::fmt::Write;
use std::sync::OnceLock;

const RECT_CHAMFER_RATIO: f64 = 0.18;
const SPECIAL_CORNER_CUT_RATIO: f64 = 0.42;
const ELLIPSE_STEPS: usize = 20;
const CHEMDRAW_BOLD_SUBSCRIPT_SHIFT_DOWN_EM: f64 = 0.215;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShapeKind {
    Rect,
    Ellipse,
    RectCutTopRight,
    RectCutBottomRight,
    RectCutTopLeft,
    RectCutBottomLeft,
}

#[derive(Debug, Clone, Copy)]
struct GlyphProfile {
    shape_kind: ShapeKind,
    advance_em: f64,
    ink_left_em: f64,
    ink_top_em: f64,
    ink_right_em: f64,
    ink_bottom_em: f64,
    pad_x_em: f64,
    pad_y_em: f64,
    visible: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GlyphProfileJson {
    shape: String,
    advance_em: f64,
    ink_left_em: f64,
    ink_top_em: f64,
    ink_right_em: f64,
    ink_bottom_em: f64,
    pad_x_em: f64,
    pad_y_em: f64,
    visible: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SharedGlyphLayoutJson {
    tracking_em: f64,
    subscript_scale: f64,
    superscript_scale: f64,
    subscript_shift_down_em: f64,
    superscript_shift_up_em: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct SharedGlyphDefaultsJson {
    upper: GlyphProfileJson,
    lower: GlyphProfileJson,
    digit: GlyphProfileJson,
    punctuation: GlyphProfileJson,
}

#[derive(Debug, Clone, Deserialize)]
struct SharedGlyphProfilesJson {
    layout: SharedGlyphLayoutJson,
    defaults: SharedGlyphDefaultsJson,
    specials: HashMap<String, GlyphProfileJson>,
}

#[derive(Debug, Clone)]
struct SharedGlyphLayout {
    tracking_em: f64,
    subscript_scale: f64,
    superscript_scale: f64,
    subscript_shift_down_em: f64,
    superscript_shift_up_em: f64,
}

#[derive(Debug, Clone)]
struct SharedGlyphDefaults {
    upper: GlyphProfile,
    lower: GlyphProfile,
    digit: GlyphProfile,
    punctuation: GlyphProfile,
}

#[derive(Debug, Clone)]
struct SharedGlyphProfiles {
    layout: SharedGlyphLayout,
    defaults: SharedGlyphDefaults,
    specials: HashMap<char, GlyphProfile>,
}

#[derive(Debug, Clone, Copy)]
struct LayoutConfig {
    font_size_px: f64,
    tracking_em: f64,
    subscript_scale: f64,
    superscript_scale: f64,
    subscript_shift_down_em: f64,
    superscript_shift_up_em: f64,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        let layout = &shared_glyph_profiles().layout;
        Self {
            font_size_px: 11.0,
            tracking_em: layout.tracking_em,
            subscript_scale: layout.subscript_scale,
            superscript_scale: layout.superscript_scale,
            subscript_shift_down_em: layout.subscript_shift_down_em,
            superscript_shift_up_em: layout.superscript_shift_up_em,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptKind {
    Normal,
    Subscript,
    Superscript,
}

impl ScriptKind {
    fn as_int(self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::Subscript => 1,
            Self::Superscript => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewAlign {
    Right,
    Left,
    Above,
    Below,
}

#[derive(Debug, Clone, Copy)]
struct GlyphInput {
    codepoint: char,
    script: ScriptKind,
}

#[derive(Debug, Clone)]
struct GlyphPlacement {
    codepoint: char,
    script: ScriptKind,
    visible: bool,
    font_size_px: f64,
    origin_x_px: f64,
    baseline_y_px: f64,
    advance_px: f64,
    background_box_px: [f64; 4],
    shape_kind: ShapeKind,
}

#[derive(Debug, Clone, Copy)]
struct LabelAnchor {
    valid: bool,
    glyph_index: usize,
    x_px: f64,
    y_px: f64,
}

#[derive(Debug, Clone)]
struct PatternSpec {
    text: String,
    anchor_index: Option<usize>,
    align: PreviewAlign,
}

#[derive(Debug, Clone)]
struct RowRender {
    label: String,
    placements: Vec<GlyphPlacement>,
    anchor: LabelAnchor,
    align: PreviewAlign,
    min_x: f64,
    max_x: f64,
    max_y: f64,
    baseline_y: f64,
}

static SHARED_GLYPH_PROFILES: OnceLock<SharedGlyphProfiles> = OnceLock::new();
const LABEL_GLYPH_CLIP_PAD_SCALE: f64 = 0.25;

#[derive(Debug, Clone, Copy)]
pub(crate) struct SharedGlyphMetrics {
    pub advance: f64,
    pub top: f64,
    pub bottom: f64,
}

pub fn build_label_glyph_polygons(
    runs: &[LabelRun],
    line_runs: &[Vec<LabelRun>],
    position: [f64; 2],
    box_value: Option<[f64; 4]>,
    fallback_font_size: f64,
) -> Vec<Vec<[f64; 2]>> {
    let lines: Vec<Vec<LabelRun>> = if !line_runs.is_empty() {
        line_runs.to_vec()
    } else if !runs.is_empty() {
        vec![runs.to_vec()]
    } else {
        Vec::new()
    };
    if lines.is_empty() {
        return Vec::new();
    }

    let line_height = box_value
        .filter(|_| lines.len() > 1)
        .map(|value| (value[3] - value[1]) / lines.len() as f64)
        .unwrap_or_else(|| (fallback_font_size * 1.05).max(fallback_font_size));
    let box_top = box_value
        .filter(|_| lines.len() > 1)
        .map(|value| value[1])
        .unwrap_or(position[1] - line_height * 0.82);

    let mut polygons = Vec::new();
    for (line_index, line) in lines.iter().enumerate() {
        let baseline_y = if lines.len() == 1 {
            position[1]
        } else {
            box_top + line_height * line_index as f64 + line_height * 0.82
        };
        polygons.extend(
            glyph_placements_for_runs(line, position[0], baseline_y, fallback_font_size)
                .into_iter()
                .filter_map(|placement| shape_polygon(&placement)),
        );
    }
    polygons
}

pub fn render_glyph_preview_svg(pattern_specs: &[&str]) -> String {
    let patterns: Vec<PatternSpec> = pattern_specs
        .iter()
        .map(|spec| parse_pattern_spec(spec))
        .collect();

    let mut config = LayoutConfig::default();
    config.font_size_px = 28.0;

    let left_margin = 40.0;
    let top_margin = 40.0;
    let row_gap = 44.0;
    let title_gap = 26.0;

    let mut rows = Vec::new();
    let mut baseline_y = top_margin + 52.0;
    let mut min_x = 0.0;
    let mut max_x = 0.0;

    for pattern in &patterns {
        let row = make_preview_row(pattern, config, left_margin + 120.0, baseline_y);
        min_x = if rows.is_empty() {
            row.min_x
        } else {
            min_x.min(row.min_x)
        };
        max_x = if rows.is_empty() {
            row.max_x
        } else {
            max_x.max(row.max_x)
        };
        baseline_y = row.max_y + row_gap;
        rows.push(row);
    }

    let width = 760.0f64.max(max_x + left_margin).max(min_x + 280.0);
    let height = 320.0f64.max(baseline_y + top_margin);
    let mut svg = String::new();

    let _ = writeln!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.3}\" height=\"{:.3}\" viewBox=\"0 0 {:.3} {:.3}\">",
        width, height, width, height
    );
    let _ = writeln!(
        svg,
        "  <rect width=\"100%\" height=\"100%\" fill=\"#050505\"/>"
    );
    let _ = writeln!(
        svg,
        "  <text x=\"{:.3}\" y=\"{:.3}\" fill=\"#f3f3f3\" font-size=\"24\" font-family=\"IBM Plex Sans, Arial, sans-serif\" dominant-baseline=\"hanging\">chemcore glyph kernel preview</text>",
        left_margin, top_margin
    );
    let _ = writeln!(
        svg,
        "  <text x=\"{:.3}\" y=\"{:.3}\" fill=\"#a9a9a9\" font-size=\"13\" font-family=\"IBM Plex Sans, Arial, sans-serif\" dominant-baseline=\"hanging\">rust kernel geometry is deterministic; browser SVG text is only a quick preview</text>",
        left_margin,
        top_margin + title_gap
    );

    for row in &rows {
        let _ = writeln!(
            svg,
            "  <text x=\"{:.3}\" y=\"{:.3}\" fill=\"#9f9f9f\" font-size=\"18\" font-family=\"IBM Plex Sans, Arial, sans-serif\" data-role=\"row-label\" dominant-baseline=\"alphabetic\">{}</text>",
            left_margin,
            row.baseline_y,
            escape_xml(&row.label)
        );
    }

    for row in &rows {
        for placement in &row.placements {
            if !placement.visible {
                continue;
            }
            if placement.shape_kind == ShapeKind::Ellipse {
                let [x1, y1, x2, y2] = placement.background_box_px;
                let _ = writeln!(
                    svg,
                    "  <ellipse cx=\"{:.3}\" cy=\"{:.3}\" rx=\"{:.3}\" ry=\"{:.3}\" fill=\"#ffffff\" data-role=\"glyph-shape\" data-shape=\"ellipse\"/>",
                    (x1 + x2) * 0.5,
                    (y1 + y2) * 0.5,
                    (x2 - x1) * 0.5,
                    (y2 - y1) * 0.5
                );
            } else if let Some(polygon) = shape_polygon(placement) {
                let _ = writeln!(
                    svg,
                    "  <path d=\"{}\" fill=\"#ffffff\" data-role=\"glyph-shape\" data-shape=\"{}\"/>",
                    svg_path_for_polygon(&polygon),
                    shape_name(placement.shape_kind)
                );
            }
        }
    }

    for row in &rows {
        for placement in &row.placements {
            if !placement.visible {
                continue;
            }
            let _ = writeln!(
                svg,
                "  <text x=\"{:.3}\" y=\"{:.3}\" fill=\"#050505\" font-size=\"{:.3}\" font-family=\"TeXGyreHeros, Arial, Helvetica, sans-serif\" data-role=\"glyph-text\" data-script=\"{}\" dominant-baseline=\"alphabetic\">{}</text>",
                placement.origin_x_px,
                placement.baseline_y_px,
                placement.font_size_px,
                placement.script.as_int(),
                escape_xml_char(placement.codepoint)
            );
        }
    }

    for row in &rows {
        if !row.anchor.valid {
            continue;
        }
        let _ = writeln!(
            svg,
            "  <circle cx=\"{:.3}\" cy=\"{:.3}\" r=\"3.200\" fill=\"#ffd400\" stroke=\"#050505\" stroke-width=\"0.900\" data-role=\"label-anchor\" data-anchor-index=\"{}\" data-align=\"{}\"/>",
            row.anchor.x_px,
            row.anchor.y_px,
            row.anchor.glyph_index,
            align_name(row.align)
        );
    }

    svg.push_str("</svg>\n");
    svg
}

fn glyph_placements_for_runs(
    runs: &[LabelRun],
    start_x: f64,
    baseline_y: f64,
    fallback_font_size: f64,
) -> Vec<GlyphPlacement> {
    let mut placements = Vec::new();
    let mut cursor_x = start_x;

    for run in runs {
        let font_size = run
            .font_size
            .unwrap_or(fallback_font_size)
            .max(crate::css_px(1.0).to_world_cm().value());
        let config = LayoutConfig {
            font_size_px: font_size,
            ..LayoutConfig::default()
        };
        let script = script_kind(run.script.as_deref());
        for character in run.text.chars() {
            let placement = layout_glyph(character, script, config, cursor_x, baseline_y);
            cursor_x += placement.advance_px;
            placements.push(placement);
        }
    }

    placements
}

fn script_kind(value: Option<&str>) -> ScriptKind {
    match value {
        Some("subscript") => ScriptKind::Subscript,
        Some("superscript") => ScriptKind::Superscript,
        _ => ScriptKind::Normal,
    }
}

fn layout_glyph(
    character: char,
    script: ScriptKind,
    config: LayoutConfig,
    origin_x_px: f64,
    baseline_y_px: f64,
) -> GlyphPlacement {
    let profile = lookup_glyph_profile(character);
    let scale = config.font_size_px * script_scale(config, script);
    let mut baseline_y = baseline_y_px + script_baseline_shift(config, script);
    if matches!(character, '+' | '-') {
        baseline_y += charge_sign_baseline_adjustment(profile, config, script);
    }
    let advance_px = (profile.advance_em + config.tracking_em) * scale;
    let ink_box = [
        origin_x_px + profile.ink_left_em * scale,
        baseline_y + profile.ink_top_em * scale,
        origin_x_px + profile.ink_right_em * scale,
        baseline_y + profile.ink_bottom_em * scale,
    ];
    let background_box = if profile.visible {
        [
            ink_box[0] - profile.pad_x_em * LABEL_GLYPH_CLIP_PAD_SCALE * scale,
            ink_box[1] - profile.pad_y_em * LABEL_GLYPH_CLIP_PAD_SCALE * scale,
            ink_box[2] + profile.pad_x_em * LABEL_GLYPH_CLIP_PAD_SCALE * scale,
            ink_box[3] + profile.pad_y_em * LABEL_GLYPH_CLIP_PAD_SCALE * scale,
        ]
    } else {
        [0.0, 0.0, 0.0, 0.0]
    };
    GlyphPlacement {
        codepoint: character,
        script,
        visible: profile.visible,
        font_size_px: scale,
        origin_x_px,
        baseline_y_px: baseline_y,
        advance_px,
        background_box_px: background_box,
        shape_kind: profile.shape_kind,
    }
}

fn layout_glyph_run(
    glyphs: &[GlyphInput],
    config: LayoutConfig,
    start_x_px: f64,
    baseline_y_px: f64,
) -> Vec<GlyphPlacement> {
    let mut placements = Vec::with_capacity(glyphs.len());
    let mut cursor_x = start_x_px;
    for glyph in glyphs {
        let placement = layout_glyph(
            glyph.codepoint,
            glyph.script,
            config,
            cursor_x,
            baseline_y_px,
        );
        cursor_x += placement.advance_px;
        placements.push(placement);
    }
    placements
}

fn layout_glyph_run_aligned(
    glyphs: &[GlyphInput],
    config: LayoutConfig,
    anchor_origin_x_px: f64,
    anchor_baseline_y_px: f64,
    anchor_glyph_index: Option<usize>,
    align: PreviewAlign,
) -> Vec<GlyphPlacement> {
    if glyphs.is_empty() {
        return Vec::new();
    }

    let probe = layout_glyph_run(glyphs, config, 0.0, anchor_baseline_y_px);
    let Some(anchor_index) = resolve_anchor_glyph_index(&probe, anchor_glyph_index) else {
        return probe;
    };

    if matches!(align, PreviewAlign::Right | PreviewAlign::Left) {
        let dx = anchor_origin_x_px - probe[anchor_index].origin_x_px;
        return probe
            .into_iter()
            .map(|mut placement| {
                translate_placement(&mut placement, dx, 0.0);
                placement
            })
            .collect();
    }

    let anchor = layout_glyph(
        glyphs[anchor_index].codepoint,
        glyphs[anchor_index].script,
        config,
        anchor_origin_x_px,
        anchor_baseline_y_px,
    );
    let mut placements = vec![anchor.clone(); glyphs.len()];
    placements[anchor_index] = anchor.clone();

    let mut other_glyphs = Vec::new();
    let mut other_indices = Vec::new();
    for (index, glyph) in glyphs.iter().enumerate() {
        if index == anchor_index {
            continue;
        }
        other_glyphs.push(*glyph);
        other_indices.push(index);
    }
    if other_glyphs.is_empty() {
        return placements;
    }

    let mut others = layout_glyph_run(
        &other_glyphs,
        config,
        anchor_origin_x_px,
        anchor_baseline_y_px,
    );
    let stack_gap_px = config.font_size_px * 0.02;
    let anchor_bounds =
        visible_bounds(std::slice::from_ref(&anchor)).unwrap_or(anchor.background_box_px);
    let other_bounds = visible_bounds(&others).unwrap_or(anchor.background_box_px);
    let dy = match align {
        PreviewAlign::Above => anchor_bounds[1] - stack_gap_px - other_bounds[3],
        PreviewAlign::Below => anchor_bounds[3] + stack_gap_px - other_bounds[1],
        PreviewAlign::Left | PreviewAlign::Right => 0.0,
    };

    for (placement, index) in others.iter_mut().zip(other_indices.into_iter()) {
        translate_placement(placement, 0.0, dy);
        placements[index] = placement.clone();
    }

    placements
}

fn resolve_anchor_glyph_index(
    placements: &[GlyphPlacement],
    requested_index: Option<usize>,
) -> Option<usize> {
    if let Some(index) = requested_index {
        if index < placements.len() && placements[index].visible {
            return Some(index);
        }
    }
    placements.iter().position(|placement| placement.visible)
}

fn locate_glyph_run(
    placements: &[GlyphPlacement],
    _config: LayoutConfig,
    anchor_glyph_index: Option<usize>,
) -> LabelAnchor {
    let Some(index) = resolve_anchor_glyph_index(placements, anchor_glyph_index) else {
        return LabelAnchor {
            valid: false,
            glyph_index: 0,
            x_px: 0.0,
            y_px: 0.0,
        };
    };
    let placement = &placements[index];
    LabelAnchor {
        valid: true,
        glyph_index: index,
        x_px: (placement.background_box_px[0] + placement.background_box_px[2]) * 0.5,
        y_px: (placement.background_box_px[1] + placement.background_box_px[3]) * 0.5,
    }
}

fn visible_bounds(placements: &[GlyphPlacement]) -> Option<[f64; 4]> {
    let mut out: Option<[f64; 4]> = None;
    for placement in placements {
        if !placement.visible {
            continue;
        }
        let bbox = placement.background_box_px;
        out = Some(match out {
            Some(current) => [
                current[0].min(bbox[0]),
                current[1].min(bbox[1]),
                current[2].max(bbox[2]),
                current[3].max(bbox[3]),
            ],
            None => bbox,
        });
    }
    out
}

fn translate_placement(placement: &mut GlyphPlacement, dx: f64, dy: f64) {
    placement.origin_x_px += dx;
    placement.baseline_y_px += dy;
    placement.background_box_px[0] += dx;
    placement.background_box_px[1] += dy;
    placement.background_box_px[2] += dx;
    placement.background_box_px[3] += dy;
}

fn script_scale(config: LayoutConfig, script: ScriptKind) -> f64 {
    match script {
        ScriptKind::Subscript => config.subscript_scale,
        ScriptKind::Superscript => config.superscript_scale,
        ScriptKind::Normal => 1.0,
    }
}

fn script_baseline_shift(config: LayoutConfig, script: ScriptKind) -> f64 {
    match script {
        ScriptKind::Subscript => config.subscript_shift_down_em * config.font_size_px,
        ScriptKind::Superscript => -config.superscript_shift_up_em * config.font_size_px,
        ScriptKind::Normal => 0.0,
    }
}

fn charge_sign_baseline_adjustment(
    profile: GlyphProfile,
    config: LayoutConfig,
    script: ScriptKind,
) -> f64 {
    if matches!(script, ScriptKind::Normal) {
        return 0.0;
    }
    let digit_profile = default_digit_profile();
    let digit_center_em = (digit_profile.ink_top_em + digit_profile.ink_bottom_em) * 0.5;
    let sign_center_em = (profile.ink_top_em + profile.ink_bottom_em) * 0.5;
    (digit_center_em - sign_center_em) * config.font_size_px * script_scale(config, script)
}

fn shape_polygon(placement: &GlyphPlacement) -> Option<Vec<[f64; 2]>> {
    if !placement.visible {
        return None;
    }
    Some(match placement.shape_kind {
        ShapeKind::Ellipse => ellipse_polygon(placement.background_box_px),
        _ => chamfer_polygon(placement.background_box_px, placement.shape_kind),
    })
}

fn ellipse_polygon(background_box: [f64; 4]) -> Vec<[f64; 2]> {
    let [x1, y1, x2, y2] = background_box;
    let cx = (x1 + x2) * 0.5;
    let cy = (y1 + y2) * 0.5;
    let rx = ((x2 - x1) * 0.5).max(0.1);
    let ry = ((y2 - y1) * 0.5).max(0.1);
    (0..ELLIPSE_STEPS)
        .map(|index| {
            let theta = TAU * index as f64 / ELLIPSE_STEPS as f64;
            [cx + rx * theta.cos(), cy + ry * theta.sin()]
        })
        .collect()
}

fn chamfer_polygon(background_box: [f64; 4], shape_kind: ShapeKind) -> Vec<[f64; 2]> {
    let [x1, y1, x2, y2] = background_box;
    let width = (x2 - x1).max(0.0);
    let height = (y2 - y1).max(0.0);
    let mut tl = clamp_corner_cut(width.min(height) * RECT_CHAMFER_RATIO, width, height);
    let mut tr = tl;
    let mut br = tl;
    let mut bl = tl;
    let special = clamp_corner_cut(width.min(height) * SPECIAL_CORNER_CUT_RATIO, width, height);
    match shape_kind {
        ShapeKind::RectCutTopRight => tr = special,
        ShapeKind::RectCutBottomRight => br = special,
        ShapeKind::RectCutTopLeft => tl = special,
        ShapeKind::RectCutBottomLeft => bl = special,
        ShapeKind::Rect | ShapeKind::Ellipse => {}
    }
    vec![
        [x1 + tl, y1],
        [x2 - tr, y1],
        [x2, y1 + tr],
        [x2, y2 - br],
        [x2 - br, y2],
        [x1 + bl, y2],
        [x1, y2 - bl],
        [x1, y1 + tl],
    ]
}

fn clamp_corner_cut(value: f64, width: f64, height: f64) -> f64 {
    value.max(0.0).min(width * 0.48).min(height * 0.48)
}

fn make_preview_row(
    spec: &PatternSpec,
    config: LayoutConfig,
    start_x_px: f64,
    baseline_y_px: f64,
) -> RowRender {
    let inputs = parse_pattern(&spec.text);
    let placements = layout_glyph_run_aligned(
        &inputs,
        config,
        start_x_px,
        baseline_y_px,
        spec.anchor_index,
        spec.align,
    );
    let anchor = locate_glyph_run(&placements, config, spec.anchor_index);
    let bounds = visible_bounds(&placements).unwrap_or([
        start_x_px,
        baseline_y_px - config.font_size_px,
        start_x_px,
        baseline_y_px + config.font_size_px,
    ]);
    RowRender {
        label: pattern_label(spec),
        placements,
        anchor,
        align: spec.align,
        min_x: bounds[0],
        max_x: bounds[2],
        max_y: bounds[3],
        baseline_y: baseline_y_px,
    }
}

fn parse_pattern_spec(arg: &str) -> PatternSpec {
    let mut text = arg.to_string();
    let mut align = PreviewAlign::Right;
    let mut anchor_index = None;

    if let Some(marker) = text.rfind('#') {
        if marker > 0 {
            match &text[marker + 1..] {
                "left" => {
                    align = PreviewAlign::Left;
                    text.truncate(marker);
                }
                "above" => {
                    align = PreviewAlign::Above;
                    text.truncate(marker);
                }
                "below" => {
                    align = PreviewAlign::Below;
                    text.truncate(marker);
                }
                "right" => {
                    align = PreviewAlign::Right;
                    text.truncate(marker);
                }
                _ => {}
            }
        }
    }

    if let Some(marker) = text.rfind('@') {
        if marker > 0 {
            let suffix = &text[marker + 1..];
            if is_unsigned_integer(suffix) {
                anchor_index = suffix.parse::<usize>().ok();
                text.truncate(marker);
            }
        }
    }

    PatternSpec {
        text,
        anchor_index,
        align,
    }
}

fn is_unsigned_integer(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|ch| ch.is_ascii_digit())
}

fn parse_pattern(pattern: &str) -> Vec<GlyphInput> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = Vec::new();
    let mut pending = ScriptKind::Normal;
    for (index, ch) in chars.iter().copied().enumerate() {
        if ch == '^' {
            pending = ScriptKind::Superscript;
            continue;
        }
        if ch == '_' {
            pending = ScriptKind::Subscript;
            continue;
        }
        out.push(GlyphInput {
            codepoint: ch,
            script: pending,
        });
        let next = chars.get(index + 1).copied().unwrap_or('\0');
        if pending != ScriptKind::Normal
            && ch.is_ascii_digit()
            && (next.is_ascii_digit() || matches!(next, '+' | '-'))
        {
            continue;
        }
        pending = ScriptKind::Normal;
    }
    out
}

fn pattern_label(spec: &PatternSpec) -> String {
    let mut label = spec.text.clone();
    if let Some(anchor_index) = spec.anchor_index {
        label.push_str(&format!(" @{}", anchor_index));
    }
    if spec.align != PreviewAlign::Right {
        label.push_str(&format!(" #{}", align_name(spec.align)));
    }
    label
}

fn align_name(align: PreviewAlign) -> &'static str {
    match align {
        PreviewAlign::Right => "right",
        PreviewAlign::Left => "left",
        PreviewAlign::Above => "above",
        PreviewAlign::Below => "below",
    }
}

fn shape_name(shape: ShapeKind) -> &'static str {
    match shape {
        ShapeKind::Rect => "rect-chamfered",
        ShapeKind::Ellipse => "ellipse",
        ShapeKind::RectCutTopRight => "rect-cut-top-right",
        ShapeKind::RectCutBottomRight => "rect-cut-bottom-right",
        ShapeKind::RectCutTopLeft => "rect-cut-top-left",
        ShapeKind::RectCutBottomLeft => "rect-cut-bottom-left",
    }
}

fn svg_path_for_polygon(points: &[[f64; 2]]) -> String {
    let mut path = String::new();
    for (index, point) in points.iter().enumerate() {
        let _ = write!(
            path,
            "{} {:.3} {:.3}",
            if index == 0 { "M" } else { " L" },
            point[0],
            point[1]
        );
    }
    path.push_str(" Z");
    path
}

fn escape_xml(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

fn escape_xml_char(ch: char) -> String {
    escape_xml(&ch.to_string())
}

fn shared_glyph_profiles() -> &'static SharedGlyphProfiles {
    SHARED_GLYPH_PROFILES.get_or_init(|| {
        let manifest: SharedGlyphProfilesJson =
            serde_json::from_str(include_str!("../../../shared/glyph_profiles.json"))
                .expect("shared glyph profile manifest must be valid JSON");
        SharedGlyphProfiles::from_json(manifest)
    })
}

fn default_lower_profile() -> GlyphProfile {
    shared_glyph_profiles().defaults.lower
}

fn default_digit_profile() -> GlyphProfile {
    shared_glyph_profiles().defaults.digit
}

fn default_punctuation_profile() -> GlyphProfile {
    shared_glyph_profiles().defaults.punctuation
}

pub(crate) fn shared_script_scale_factor(script: Option<&str>) -> f64 {
    match script {
        Some("subscript") => shared_glyph_profiles().layout.subscript_scale,
        Some("superscript") => shared_glyph_profiles().layout.superscript_scale,
        _ => 1.0,
    }
}

pub(crate) fn shared_script_baseline_shift_em(
    script: Option<&str>,
    font_weight: Option<u32>,
) -> f64 {
    match script {
        Some("subscript") if font_weight.unwrap_or(400) >= 600 => {
            CHEMDRAW_BOLD_SUBSCRIPT_SHIFT_DOWN_EM
        }
        Some("subscript") => shared_glyph_profiles().layout.subscript_shift_down_em,
        Some("superscript") => -shared_glyph_profiles().layout.superscript_shift_up_em,
        _ => 0.0,
    }
}

pub(crate) fn shared_svg_script_baseline_shift_em(
    script: Option<&str>,
    font_weight: Option<u32>,
) -> f64 {
    -shared_script_baseline_shift_em(script, font_weight)
}

pub(crate) fn shared_estimated_char_width(character: char, font_size: f64) -> f64 {
    lookup_glyph_profile(character).advance_em * font_size
}

pub(crate) fn shared_glyph_metrics(
    character: char,
    font_size: f64,
    script: Option<&str>,
) -> SharedGlyphMetrics {
    let config = LayoutConfig {
        font_size_px: font_size,
        ..LayoutConfig::default()
    };
    let placement = layout_glyph(character, script_kind(script), config, 0.0, 0.0);
    SharedGlyphMetrics {
        advance: placement.advance_px,
        top: placement.background_box_px[1],
        bottom: placement.background_box_px[3],
    }
}

impl SharedGlyphProfiles {
    fn from_json(manifest: SharedGlyphProfilesJson) -> Self {
        let mut specials = HashMap::new();
        for (key, value) in manifest.specials {
            let mut chars = key.chars();
            let character = chars
                .next()
                .filter(|_| chars.next().is_none())
                .unwrap_or_else(|| {
                    panic!("glyph profile key must be exactly one character: {key:?}")
                });
            specials.insert(character, glyph_profile_from_json(&value));
        }
        Self {
            layout: SharedGlyphLayout {
                tracking_em: manifest.layout.tracking_em,
                subscript_scale: manifest.layout.subscript_scale,
                superscript_scale: manifest.layout.superscript_scale,
                subscript_shift_down_em: manifest.layout.subscript_shift_down_em,
                superscript_shift_up_em: manifest.layout.superscript_shift_up_em,
            },
            defaults: SharedGlyphDefaults {
                upper: glyph_profile_from_json(&manifest.defaults.upper),
                lower: glyph_profile_from_json(&manifest.defaults.lower),
                digit: glyph_profile_from_json(&manifest.defaults.digit),
                punctuation: glyph_profile_from_json(&manifest.defaults.punctuation),
            },
            specials,
        }
    }
}

fn glyph_profile_from_json(profile: &GlyphProfileJson) -> GlyphProfile {
    GlyphProfile {
        shape_kind: shape_kind_from_name(&profile.shape),
        advance_em: profile.advance_em,
        ink_left_em: profile.ink_left_em,
        ink_top_em: profile.ink_top_em,
        ink_right_em: profile.ink_right_em,
        ink_bottom_em: profile.ink_bottom_em,
        pad_x_em: profile.pad_x_em,
        pad_y_em: profile.pad_y_em,
        visible: profile.visible,
    }
}

fn shape_kind_from_name(shape: &str) -> ShapeKind {
    match shape {
        "rect" => ShapeKind::Rect,
        "ellipse" => ShapeKind::Ellipse,
        "rect-cut-top-right" => ShapeKind::RectCutTopRight,
        "rect-cut-bottom-right" => ShapeKind::RectCutBottomRight,
        "rect-cut-top-left" => ShapeKind::RectCutTopLeft,
        "rect-cut-bottom-left" => ShapeKind::RectCutBottomLeft,
        _ => panic!("unknown glyph profile shape: {shape}"),
    }
}

fn lookup_glyph_profile(character: char) -> GlyphProfile {
    let shared = shared_glyph_profiles();
    if let Some(profile) = shared.specials.get(&character) {
        return *profile;
    }
    if character.is_whitespace() {
        return GlyphProfile {
            shape_kind: ShapeKind::Rect,
            advance_em: 0.28,
            ink_left_em: 0.0,
            ink_top_em: 0.0,
            ink_right_em: 0.0,
            ink_bottom_em: 0.0,
            pad_x_em: 0.0,
            pad_y_em: 0.0,
            visible: false,
        };
    }
    if is_cjk_or_fullwidth(character) {
        return fallback_rect_profile(1.0, -0.86, 1.0, 0.14);
    }
    if is_math_or_arrow_symbol(character) {
        return fallback_rect_profile(0.84, -0.74, 0.84, 0.06);
    }
    if matches!(character, '\u{2030}' | '\u{2031}') {
        return fallback_rect_profile(1.34, -0.74, 1.34, 0.06);
    }
    if character.is_ascii_uppercase() {
        return shared.defaults.upper;
    }
    if character.is_ascii_lowercase() {
        return default_lower_profile();
    }
    if character.is_ascii_digit() {
        return shared.defaults.digit;
    }
    if character.is_alphabetic() {
        if character.is_uppercase() {
            return fallback_rect_profile(0.72, -0.74, 0.72, 0.04);
        }
        return fallback_rect_profile(0.62, -0.62, 0.62, 0.08);
    }
    if character.is_ascii_punctuation() {
        return default_punctuation_profile();
    }
    fallback_rect_profile(0.62, -0.74, 0.62, 0.08)
}

fn fallback_rect_profile(
    advance_em: f64,
    ink_top_em: f64,
    ink_right_em: f64,
    ink_bottom_em: f64,
) -> GlyphProfile {
    GlyphProfile {
        shape_kind: ShapeKind::Rect,
        advance_em,
        ink_left_em: 0.0,
        ink_top_em,
        ink_right_em,
        ink_bottom_em,
        pad_x_em: 0.09,
        pad_y_em: 0.09,
        visible: true,
    }
}

fn is_cjk_or_fullwidth(character: char) -> bool {
    let code = character as u32;
    matches!(
        code,
        0x1100..=0x11FF
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7AF
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE6F
            | 0xFF00..=0xFFEF
            | 0x20000..=0x2FA1F
    )
}

fn is_math_or_arrow_symbol(character: char) -> bool {
    let code = character as u32;
    matches!(code, 0x2190..=0x21FF | 0x2200..=0x22FF | 0x27F0..=0x27FF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_text_symbols_have_non_punctuation_metrics() {
        let expected_min_widths = [
            ('%', 0.90),
            ('‰', 1.10),
            ('α', 0.50),
            ('≤', 0.70),
            ('→', 0.70),
            ('℃', 0.90),
            ('中', 0.90),
        ];
        for (character, min_width) in expected_min_widths {
            let profile = lookup_glyph_profile(character);
            assert!(
                profile.advance_em >= min_width,
                "{character} should not use narrow punctuation fallback: {profile:?}"
            );
            assert!(profile.visible, "{character} should be visible");
            assert!(
                profile.ink_bottom_em > profile.ink_top_em,
                "{character} should have a usable vertical ink box: {profile:?}"
            );
        }
    }

    #[test]
    fn unknown_cjk_text_gets_conservative_square_profile() {
        let profile = lookup_glyph_profile('龘');
        assert!(profile.advance_em >= 0.95, "{profile:?}");
        assert!(profile.ink_right_em >= 0.95, "{profile:?}");
        assert!(profile.visible);
    }

    #[test]
    fn text_symbol_polygons_are_available_for_label_clipping() {
        let runs = vec![LabelRun {
            text: "‰α≤→℃中".to_string(),
            font_family: Some("Arial".to_string()),
            font_size: Some(10.0),
            fill: Some("#000000".to_string()),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: None,
            script: Some("normal".to_string()),
        }];
        let polygons = build_label_glyph_polygons(&runs, &[], [0.0, 0.0], None, 10.0);
        assert_eq!(polygons.len(), 6, "{polygons:?}");
        assert!(polygons.iter().all(|polygon| polygon.len() >= 4));
    }

    #[test]
    fn sulfur_uses_ellipse_clip_profile() {
        let profile = lookup_glyph_profile('S');
        assert_eq!(
            profile.shape_kind,
            ShapeKind::Ellipse,
            "uppercase S should use ellipse clipping"
        );

        let runs = vec![LabelRun {
            text: "S".to_string(),
            font_family: Some("Arial".to_string()),
            font_size: Some(10.0),
            fill: Some("#000000".to_string()),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: None,
            script: Some("normal".to_string()),
        }];
        let polygons = build_label_glyph_polygons(&runs, &[], [0.0, 0.0], None, 10.0);
        assert_eq!(polygons.len(), 1, "{polygons:?}");
        assert!(
            polygons[0].len() >= 16,
            "uppercase S should generate ellipse-like clip geometry: {:?}",
            polygons[0]
        );
    }
}
