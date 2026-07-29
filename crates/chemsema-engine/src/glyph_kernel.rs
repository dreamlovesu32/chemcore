use crate::LabelRun;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Write;
use std::io::Read;
use std::sync::OnceLock;

const CHEMDRAW_BOLD_SUBSCRIPT_SHIFT_DOWN_EM: f64 = 0.215;

#[derive(Debug, Clone, Copy)]
struct GlyphProfile {
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GlyphOutlineJson {
    advance_em: f64,
    bounds_em: [f64; 4],
    commands: Vec<GlyphOutlineCommandJson>,
}

#[derive(Debug, Clone, Deserialize)]
struct GlyphOutlineCommandJson {
    op: String,
    points: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, Deserialize)]
struct GlyphOutlineFaceJson {
    glyphs: HashMap<String, GlyphOutlineJson>,
}

#[derive(Debug, Clone, Deserialize)]
struct GlyphOutlineFamilyJson {
    faces: HashMap<String, GlyphOutlineFaceJson>,
}

#[derive(Debug, Clone, Deserialize)]
struct SharedGlyphOutlinesJson {
    version: u32,
    aliases: HashMap<String, String>,
    families: HashMap<String, GlyphOutlineFamilyJson>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphClipProfile {
    pub natural_outset_pt: f64,
}

impl GlyphClipProfile {
    pub fn from_margin_width(margin_width: f64) -> Self {
        let natural_outset_pt = margin_width.max(0.0);
        Self { natural_outset_pt }
    }
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
    font_family: String,
    font_weight: u32,
    italic: bool,
    visible: bool,
    font_size_px: f64,
    origin_x_px: f64,
    baseline_y_px: f64,
    advance_px: f64,
    ink_box_px: [f64; 4],
    background_box_px: [f64; 4],
}

#[derive(Debug, Clone, Copy)]
struct AxisContactGlyph {
    bounds: [f64; 4],
    baseline_y: f64,
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
static SHARED_GLYPH_OUTLINES: OnceLock<SharedGlyphOutlinesJson> = OnceLock::new();
const LABEL_GLYPH_CLIP_PAD_SCALE: f64 = 0.25;
const GLYPH_CURVE_STEPS: usize = 12;
const GLYPH_CIRCLE_STEPS: usize = 20;
pub(crate) const GLYPH_AXIS_HALF_SECTOR_DEG: f64 = 10.0;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LabelGlyphGeometry {
    pub glyph_polygons: Vec<Vec<[f64; 2]>>,
    pub clip_polygons: Vec<Vec<[f64; 2]>>,
    pub clip_polygon_owners: Vec<Option<usize>>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SharedGlyphMetrics {
    pub advance: f64,
    pub top: f64,
    pub bottom: f64,
}

pub(crate) fn molecule_label_line_advance(default_font_size: f64) -> f64 {
    (default_font_size * crate::MOLECULE_LABEL_LINE_ADVANCE_RATIO).max(0.1)
}

pub(crate) fn variable_text_line_advances(
    lines: &[Vec<LabelRun>],
    default_font_size: f64,
) -> Vec<f64> {
    if lines.len() < 2 {
        return Vec::new();
    }
    let bounds = lines
        .iter()
        .map(|line| {
            let mut top = f64::INFINITY;
            let mut bottom = f64::NEG_INFINITY;
            let mut max_size = default_font_size;
            for run in line {
                max_size = max_size.max(run.font_size.unwrap_or(default_font_size));
            }
            for placement in glyph_placements_for_runs(line, 0.0, 0.0, default_font_size) {
                top = top.min(placement.ink_box_px[1]);
                bottom = bottom.max(placement.ink_box_px[3]);
            }
            if !top.is_finite() || !bottom.is_finite() {
                top = -default_font_size * 0.73;
                bottom = default_font_size * 0.16;
            }
            (top, bottom, max_size)
        })
        .collect::<Vec<_>>();
    bounds
        .windows(2)
        .map(|pair| {
            let (_, previous_bottom, previous_size) = pair[0];
            let (next_top, _, next_size) = pair[1];
            // ChemDraw's Variable mode packs consecutive glyph ink boxes and
            // leaves about one tenth of an em between them. The glyph bounds
            // already include face, size and script baseline shifts.
            (previous_bottom - next_top + previous_size.max(next_size) * 0.1).max(0.1)
        })
        .collect()
}

fn line_baseline_offset(line_index: usize, line_height: f64, line_advances: &[f64]) -> f64 {
    (0..line_index)
        .map(|index| line_advances.get(index).copied().unwrap_or(line_height))
        .sum()
}

#[allow(clippy::too_many_arguments)]
pub fn build_label_glyph_geometry_with_profile(
    runs: &[LabelRun],
    line_runs: &[Vec<LabelRun>],
    position: [f64; 2],
    box_value: Option<[f64; 4]>,
    default_font_size: f64,
    line_height: f64,
    line_advances: &[f64],
    retreat_origin: [f64; 2],
    profile: GlyphClipProfile,
) -> LabelGlyphGeometry {
    // Build world-space clip polygons from the same styled runs used for
    // label rendering, so bonds can retreat from actual glyph shapes instead
    // of the much coarser text bounding box.
    let lines: Vec<Vec<LabelRun>> = if !line_runs.is_empty() {
        line_runs.to_vec()
    } else if !runs.is_empty() {
        vec![runs.to_vec()]
    } else {
        Vec::new()
    };
    if lines.is_empty() {
        return LabelGlyphGeometry::default();
    }

    let box_top = box_value
        .filter(|_| lines.len() > 1)
        .map(|value| value[1])
        .unwrap_or(position[1] - default_font_size * 0.82);

    let mut geometry = LabelGlyphGeometry::default();
    let mut contact_glyphs = Vec::new();
    for (line_index, line) in lines.iter().enumerate() {
        let baseline_y = if lines.len() == 1 {
            position[1]
        } else {
            box_top
                + line_baseline_offset(line_index, line_height, line_advances)
                + default_font_size * 0.82
        };
        for placement in glyph_placements_for_runs(line, position[0], baseline_y, default_font_size)
        {
            let Some(glyph_geometry) = glyph_geometry_with_profile(&placement, profile) else {
                continue;
            };
            let glyph_index = geometry.glyph_polygons.len();
            contact_glyphs.push(AxisContactGlyph {
                bounds: placement.ink_box_px,
                baseline_y: placement.baseline_y_px,
            });
            geometry.glyph_polygons.push(glyph_geometry.glyph_polygon);
            geometry.clip_polygon_owners.extend(std::iter::repeat_n(
                Some(glyph_index),
                glyph_geometry.clip_polygons.len(),
            ));
            geometry.clip_polygons.extend(glyph_geometry.clip_polygons);
        }
    }
    if !contact_glyphs.is_empty() {
        let contacts = localized_axis_contact_polygons(
            &contact_glyphs,
            retreat_origin,
            profile.natural_outset_pt,
        );
        for (polygon, owner) in contacts {
            geometry.clip_polygons.push(polygon);
            geometry.clip_polygon_owners.push(owner);
        }
    }
    geometry
}

pub fn render_glyph_preview_svg(pattern_specs: &[&str]) -> String {
    let patterns: Vec<PatternSpec> = pattern_specs
        .iter()
        .map(|spec| parse_pattern_spec(spec))
        .collect();

    let config = LayoutConfig {
        font_size_px: 28.0,
        ..LayoutConfig::default()
    };

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
        "  <text x=\"{:.3}\" y=\"{:.3}\" fill=\"#f3f3f3\" font-size=\"24\" font-family=\"IBM Plex Sans, Arial, sans-serif\" dominant-baseline=\"hanging\">chemsema glyph kernel preview</text>",
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
            if let Some(polygon) = shape_polygon(placement) {
                let _ = writeln!(
                    svg,
                    "  <path d=\"{}\" fill=\"#ffffff\" data-role=\"glyph-shape\" data-shape=\"clip-polygon\"/>",
                    svg_path_for_polygon(&polygon),
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
    default_font_size: f64,
) -> Vec<GlyphPlacement> {
    let mut placements = Vec::new();
    let mut cursor_x = start_x;

    for run in runs {
        let font_size = run
            .font_size
            .unwrap_or(default_font_size)
            .max(crate::css_px(1.0).to_world_pt().value());
        let mut config = LayoutConfig {
            font_size_px: font_size,
            ..LayoutConfig::default()
        };
        let script = script_kind(run.script.as_deref());
        let font_family = run.font_family.as_deref().unwrap_or("Arial");
        let font_weight = run.font_weight.unwrap_or(400);
        config.subscript_shift_down_em = shared_script_baseline_shift_em_for_face(
            Some("subscript"),
            Some(font_weight),
            Some(font_family),
            font_size,
        );
        config.superscript_shift_up_em = -shared_script_baseline_shift_em_for_face(
            Some("superscript"),
            Some(font_weight),
            Some(font_family),
            font_size,
        );
        let italic = run.font_style.as_deref() == Some("italic");
        for character in run.text.chars() {
            let placement = layout_glyph(
                character,
                script,
                config,
                cursor_x,
                baseline_y,
                GlyphFaceRef {
                    family: font_family,
                    weight: font_weight,
                    italic,
                },
            );
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

#[derive(Clone, Copy)]
struct GlyphFaceRef<'a> {
    family: &'a str,
    weight: u32,
    italic: bool,
}

fn layout_glyph(
    character: char,
    script: ScriptKind,
    config: LayoutConfig,
    origin_x_px: f64,
    baseline_y_px: f64,
    face: GlyphFaceRef<'_>,
) -> GlyphPlacement {
    let profile = lookup_glyph_profile(character);
    let outline = lookup_glyph_outline(face.family, face.weight, face.italic, character);
    let scale = config.font_size_px * script_scale(config, script);
    let mut baseline_y = baseline_y_px + script_baseline_shift(config, script);
    if matches!(character, '+' | '-') {
        baseline_y += charge_sign_baseline_adjustment(profile, config, script);
    }
    let advance_em = outline.map_or(profile.advance_em, |glyph| glyph.advance_em);
    let bounds_em = outline.map_or(
        [
            profile.ink_left_em,
            profile.ink_top_em,
            profile.ink_right_em,
            profile.ink_bottom_em,
        ],
        |glyph| glyph.bounds_em,
    );
    let advance_px = (advance_em + config.tracking_em) * scale;
    let ink_box = [
        origin_x_px + bounds_em[0] * scale,
        baseline_y + bounds_em[1] * scale,
        origin_x_px + bounds_em[2] * scale,
        baseline_y + bounds_em[3] * scale,
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
        font_family: face.family.to_string(),
        font_weight: face.weight,
        italic: face.italic,
        visible: profile.visible,
        font_size_px: scale,
        origin_x_px,
        baseline_y_px: baseline_y,
        advance_px,
        ink_box_px: ink_box,
        background_box_px: background_box,
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
            GlyphFaceRef {
                family: "Arial",
                weight: 400,
                italic: false,
            },
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
        GlyphFaceRef {
            family: "Arial",
            weight: 400,
            italic: false,
        },
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

    for (placement, index) in others.iter_mut().zip(other_indices) {
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
    placement.ink_box_px[0] += dx;
    placement.ink_box_px[1] += dy;
    placement.ink_box_px[2] += dx;
    placement.ink_box_px[3] += dy;
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
    shape_polygon_with_profile(
        placement,
        GlyphClipProfile::from_margin_width(crate::DEFAULT_BOND_MARGIN_WIDTH_PT.value()),
    )
}

fn shape_polygon_with_profile(
    placement: &GlyphPlacement,
    profile: GlyphClipProfile,
) -> Option<Vec<[f64; 2]>> {
    glyph_geometry_with_profile(placement, profile).map(|geometry| geometry.glyph_polygon)
}

#[derive(Debug, Clone)]
struct BuiltGlyphGeometry {
    glyph_polygon: Vec<[f64; 2]>,
    clip_polygons: Vec<Vec<[f64; 2]>>,
}

fn glyph_geometry_with_profile(
    placement: &GlyphPlacement,
    profile: GlyphClipProfile,
) -> Option<BuiltGlyphGeometry> {
    if !placement.visible {
        return None;
    }
    let outline = lookup_glyph_outline(
        &placement.font_family,
        placement.font_weight,
        placement.italic,
        placement.codepoint,
    )?;
    let contours = flatten_glyph_contours(
        outline,
        placement.origin_x_px,
        placement.baseline_y_px,
        placement.font_size_px,
    );
    let all_points: Vec<[f64; 2]> = contours.iter().flatten().copied().collect();
    let glyph_polygon = convex_hull(&all_points);
    if glyph_polygon.len() < 3 {
        return None;
    }

    let margin = profile.natural_outset_pt.max(0.0);
    let mut clip_polygons = Vec::new();
    for contour in &contours {
        if contour.len() < 3 {
            continue;
        }
        clip_polygons.push(contour.clone());
        if margin > crate::EPSILON {
            for index in 0..contour.len() {
                let start = contour[index];
                let end = contour[(index + 1) % contour.len()];
                if let Some(capsule) = capsule_polygon(start, end, margin) {
                    clip_polygons.push(capsule);
                }
            }
        }
    }

    let feature_margin = margin.min(placement.font_size_px * 0.25);
    if feature_margin > crate::EPSILON {
        let center = [
            (placement.ink_box_px[0] + placement.ink_box_px[2]) * 0.5,
            (placement.ink_box_px[1] + placement.ink_box_px[3]) * 0.5,
        ];
        for vertex in &glyph_polygon {
            let dx = center[0] - vertex[0];
            let dy = center[1] - vertex[1];
            let length = dx.hypot(dy);
            let inset = feature_margin * 0.5;
            let anchor = if length <= crate::EPSILON {
                *vertex
            } else {
                [
                    vertex[0] + dx / length * inset,
                    vertex[1] + dy / length * inset,
                ]
            };
            clip_polygons.push(circle_polygon(anchor, feature_margin * 1.5));
        }
    }

    Some(BuiltGlyphGeometry {
        glyph_polygon,
        clip_polygons,
    })
}

fn shared_glyph_outlines() -> &'static SharedGlyphOutlinesJson {
    SHARED_GLYPH_OUTLINES.get_or_init(|| {
        let compressed = include_bytes!(concat!(env!("OUT_DIR"), "/glyph_outlines.json.gz"));
        let mut decoder = flate2::read::GzDecoder::new(&compressed[..]);
        let mut json = String::new();
        decoder
            .read_to_string(&mut json)
            .expect("shared glyph outline manifest must decompress");
        let manifest: SharedGlyphOutlinesJson =
            serde_json::from_str(&json).expect("shared glyph outline manifest must be valid JSON");
        assert_eq!(manifest.version, 2, "unsupported glyph outline manifest");
        manifest
    })
}

fn glyph_face_key(font_weight: u32, italic: bool) -> &'static str {
    match (font_weight >= 600, italic) {
        (false, false) => "regular",
        (true, false) => "bold",
        (false, true) => "italic",
        (true, true) => "boldItalic",
    }
}

fn lookup_glyph_outline(
    font_family: &str,
    font_weight: u32,
    italic: bool,
    character: char,
) -> Option<&'static GlyphOutlineJson> {
    let manifest = shared_glyph_outlines();
    let resolved_family = manifest
        .aliases
        .get(font_family)
        .map(String::as_str)
        .unwrap_or(font_family);
    let lookup_character = |key: &str| {
        manifest
            .families
            .get(resolved_family)
            .and_then(|family| family.faces.get(glyph_face_key(font_weight, italic)))
            .and_then(|face| face.glyphs.get(key))
            .or_else(|| {
                // This is the same explicit glyph-substitution chain used by the
                // label renderer for characters absent from the selected family.
                // This is character resolution, not retreat geometry:
                // whichever outline wins also supplies advance, bounds and clip.
                ["Segoe UI Symbol", "SimSun", "Arial"]
                    .into_iter()
                    .filter(|family| *family != resolved_family)
                    .find_map(|family| {
                        manifest
                            .families
                            .get(family)
                            .and_then(|family| family.faces.get("regular"))
                            .and_then(|face| face.glyphs.get(key))
                    })
            })
    };
    let key = character.to_string();
    lookup_character(&key).or_else(|| (character != '□').then(|| lookup_character("□")).flatten())
}

fn flatten_glyph_contours(
    outline: &GlyphOutlineJson,
    origin_x: f64,
    baseline_y: f64,
    scale: f64,
) -> Vec<Vec<[f64; 2]>> {
    let map = |point: [f64; 2]| [origin_x + point[0] * scale, baseline_y + point[1] * scale];
    let mut contours = Vec::new();
    let mut current = Vec::new();
    for command in &outline.commands {
        match command.op.as_str() {
            "M" => {
                if current.len() >= 3 {
                    contours.push(std::mem::take(&mut current));
                } else {
                    current.clear();
                }
                if let Some(point) = command.points.first() {
                    current.push(map(*point));
                }
            }
            "L" => {
                if let Some(point) = command.points.first() {
                    current.push(map(*point));
                }
            }
            "Q" => append_quadratic_segments(&mut current, &command.points, map),
            "C" => append_cubic_segment(&mut current, &command.points, map),
            "Z" => {
                if current.len() >= 3 {
                    contours.push(std::mem::take(&mut current));
                } else {
                    current.clear();
                }
            }
            _ => {}
        }
    }
    if current.len() >= 3 {
        contours.push(current);
    }
    for contour in &mut contours {
        compact_points(contour);
    }
    contours.retain(|contour| contour.len() >= 3);
    contours
}

fn append_quadratic_segments<F>(current: &mut Vec<[f64; 2]>, points: &[[f64; 2]], map: F)
where
    F: Fn([f64; 2]) -> [f64; 2],
{
    if current.is_empty() || points.len() < 2 {
        return;
    }
    let controls = &points[..points.len() - 1];
    let final_point = points[points.len() - 1];
    for (index, control) in controls.iter().enumerate() {
        let end = controls.get(index + 1).map_or(final_point, |next| {
            [(control[0] + next[0]) * 0.5, (control[1] + next[1]) * 0.5]
        });
        let start = *current.last().expect("quadratic contour has a start");
        let control = map(*control);
        let end = map(end);
        for step in 1..=GLYPH_CURVE_STEPS {
            let t = step as f64 / GLYPH_CURVE_STEPS as f64;
            let mt = 1.0 - t;
            current.push([
                mt * mt * start[0] + 2.0 * mt * t * control[0] + t * t * end[0],
                mt * mt * start[1] + 2.0 * mt * t * control[1] + t * t * end[1],
            ]);
        }
    }
}

fn append_cubic_segment<F>(current: &mut Vec<[f64; 2]>, points: &[[f64; 2]], map: F)
where
    F: Fn([f64; 2]) -> [f64; 2],
{
    if current.is_empty() || points.len() != 3 {
        return;
    }
    let start = *current.last().expect("cubic contour has a start");
    let control1 = map(points[0]);
    let control2 = map(points[1]);
    let end = map(points[2]);
    for step in 1..=GLYPH_CURVE_STEPS {
        let t = step as f64 / GLYPH_CURVE_STEPS as f64;
        let mt = 1.0 - t;
        current.push([
            mt.powi(3) * start[0]
                + 3.0 * mt * mt * t * control1[0]
                + 3.0 * mt * t * t * control2[0]
                + t.powi(3) * end[0],
            mt.powi(3) * start[1]
                + 3.0 * mt * mt * t * control1[1]
                + 3.0 * mt * t * t * control2[1]
                + t.powi(3) * end[1],
        ]);
    }
}

fn compact_points(points: &mut Vec<[f64; 2]>) {
    points.dedup_by(|left, right| (left[0] - right[0]).hypot(left[1] - right[1]) <= 1e-7);
    if points.len() >= 2
        && points
            .first()
            .zip(points.last())
            .is_some_and(|(first, last)| (first[0] - last[0]).hypot(first[1] - last[1]) <= 1e-7)
    {
        points.pop();
    }
}

fn convex_hull(points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut sorted = points.to_vec();
    sorted.sort_by(|left, right| {
        left[0]
            .total_cmp(&right[0])
            .then(left[1].total_cmp(&right[1]))
    });
    sorted.dedup_by(|left, right| {
        (left[0] - right[0]).abs() <= 1e-8 && (left[1] - right[1]).abs() <= 1e-8
    });
    if sorted.len() <= 2 {
        return sorted;
    }
    let cross = |origin: [f64; 2], left: [f64; 2], right: [f64; 2]| {
        (left[0] - origin[0]) * (right[1] - origin[1])
            - (left[1] - origin[1]) * (right[0] - origin[0])
    };
    let mut lower = Vec::new();
    for point in sorted.iter().copied() {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], point) <= 0.0
        {
            lower.pop();
        }
        lower.push(point);
    }
    let mut upper = Vec::new();
    for point in sorted.iter().rev().copied() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], point) <= 0.0
        {
            upper.pop();
        }
        upper.push(point);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn circle_polygon(center: [f64; 2], radius: f64) -> Vec<[f64; 2]> {
    (0..GLYPH_CIRCLE_STEPS)
        .map(|index| {
            let angle = std::f64::consts::TAU * index as f64 / GLYPH_CIRCLE_STEPS as f64;
            [
                center[0] + radius * angle.cos(),
                center[1] + radius * angle.sin(),
            ]
        })
        .collect()
}

fn capsule_polygon(start: [f64; 2], end: [f64; 2], radius: f64) -> Option<Vec<[f64; 2]>> {
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    if dx.hypot(dy) <= crate::EPSILON {
        return None;
    }
    let angle = dy.atan2(dx);
    let half_steps = GLYPH_CIRCLE_STEPS / 2;
    let mut points = Vec::with_capacity(GLYPH_CIRCLE_STEPS + 2);
    for index in 0..=half_steps {
        let theta = angle
            + std::f64::consts::FRAC_PI_2
            + std::f64::consts::PI * index as f64 / half_steps as f64;
        points.push([
            start[0] + radius * theta.cos(),
            start[1] + radius * theta.sin(),
        ]);
    }
    for index in 0..=half_steps {
        let theta = angle - std::f64::consts::FRAC_PI_2
            + std::f64::consts::PI * index as f64 / half_steps as f64;
        points.push([end[0] + radius * theta.cos(), end[1] + radius * theta.sin()]);
    }
    Some(points)
}

fn localized_axis_contact_polygons(
    glyphs: &[AxisContactGlyph],
    origin: [f64; 2],
    margin: f64,
) -> Vec<(Vec<[f64; 2]>, Option<usize>)> {
    let margin = margin.max(0.0);
    // ChemDraw treats a horizontal text run as one left/right envelope:
    // scripts on the attachment-facing end still contribute their X extent.
    // Vertical contacts are column-local, so a script beside the attachment
    // column cannot deepen a top/bottom retreat.
    let right = glyphs
        .iter()
        .map(|glyph| glyph.bounds[2] + margin - origin[0])
        .filter(|extent| *extent > crate::EPSILON)
        .max_by(f64::total_cmp);
    let left = glyphs
        .iter()
        .map(|glyph| origin[0] - glyph.bounds[0] + margin)
        .filter(|extent| *extent > crate::EPSILON)
        .max_by(f64::total_cmp);

    let vertical_candidates = glyphs
        .iter()
        .enumerate()
        .filter(|(_, glyph)| {
            origin[0] + crate::EPSILON >= glyph.bounds[0] - margin
                && origin[0] - crate::EPSILON <= glyph.bounds[2] + margin
        })
        .collect::<Vec<_>>();
    let mut contacts = Vec::new();
    if let Some(extent) = right {
        contacts.push((0.0, extent, None));
    }
    for (index, glyph) in &vertical_candidates {
        // ChemDraw's baseline-facing contact uses the descent envelope of the
        // whole same-baseline run. A normal "Tyr" therefore inherits the y
        // descender even when the bond crosses T, while a shifted formula
        // subscript remains a separate baseline group.
        let down = glyphs
            .iter()
            .filter(|peer| (peer.baseline_y - glyph.baseline_y).abs() <= crate::EPSILON)
            .map(|peer| peer.bounds[3] + margin - origin[1])
            .filter(|extent| *extent > crate::EPSILON)
            .max_by(f64::total_cmp);
        if let Some(extent) = down {
            contacts.push((90.0, extent, Some(*index)));
        }
    }
    if let Some(extent) = left {
        contacts.push((180.0, extent, None));
    }
    for (index, glyph) in vertical_candidates {
        // The cap-facing contact remains column-local: distant capitals in a
        // run do not deepen a bond that meets a shorter glyph such as y.
        let extent = origin[1] - glyph.bounds[1] + margin;
        if extent > crate::EPSILON {
            contacts.push((270.0, extent, Some(index)));
        }
    }
    contacts
        .into_iter()
        .map(|(axis_deg, extent, owner)| {
            let mut polygon = Vec::with_capacity(7);
            polygon.push(origin);
            for index in 0..=5 {
                let offset_deg = -GLYPH_AXIS_HALF_SECTOR_DEG
                    + 2.0 * GLYPH_AXIS_HALF_SECTOR_DEG * index as f64 / 5.0;
                let offset = offset_deg.to_radians();
                let angle = (axis_deg + offset_deg).to_radians();
                let radius = extent * offset.cos();
                polygon.push([
                    origin[0] + radius * angle.cos(),
                    origin[1] + radius * angle.sin(),
                ]);
            }
            (polygon, owner)
        })
        .collect()
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

pub(crate) fn shared_script_baseline_shift_em_for_face(
    script: Option<&str>,
    font_weight: Option<u32>,
    font_family: Option<&str>,
    font_size: f64,
) -> f64 {
    let family = font_family.unwrap_or("Arial").to_ascii_lowercase();
    let bold = font_weight.unwrap_or(400) >= 600;
    if family.contains("times new roman") && font_size <= 8.0 {
        return match script {
            Some("subscript") if bold => 0.214_286_25,
            Some("subscript") => 0.243_214_894,
            Some("superscript") if bold => -0.392_679_553,
            Some("superscript") => -0.378_750_947,
            _ => 0.0,
        };
    }
    if family.contains("calibri") {
        return match script {
            Some("subscript") => 0.27,
            Some("superscript") => -0.365,
            _ => 0.0,
        };
    }
    shared_script_baseline_shift_em(script, font_weight)
}

pub(crate) fn shared_svg_script_baseline_shift_em_for_face(
    script: Option<&str>,
    font_weight: Option<u32>,
    font_family: Option<&str>,
    font_size: f64,
) -> f64 {
    -shared_script_baseline_shift_em_for_face(script, font_weight, font_family, font_size)
}

pub(crate) fn shared_estimated_char_width(character: char, font_size: f64) -> f64 {
    lookup_glyph_profile(character).advance_em * font_size
}

pub(crate) fn shared_estimated_char_width_for_face(
    character: char,
    font_size: f64,
    font_family: &str,
    font_weight: u32,
    font_style: Option<&str>,
) -> f64 {
    let italic = font_style.is_some_and(|style| {
        matches!(
            style.trim().to_ascii_lowercase().as_str(),
            "italic" | "oblique"
        )
    });
    let advance_em = lookup_glyph_outline(font_family, font_weight, italic, character)
        .map(|outline| outline.advance_em)
        .unwrap_or_else(|| lookup_glyph_profile(character).advance_em);
    advance_em * font_size
}

pub(crate) fn shared_estimated_text_width(
    text: &str,
    runs: &[crate::LabelRun],
    default_font_size: f64,
) -> f64 {
    if !runs.is_empty() {
        let mut max_width = 0.0;
        let mut line_width = 0.0;
        for run in runs {
            let font_size = run.font_size.unwrap_or(default_font_size)
                * shared_script_scale_factor(run.script.as_deref());
            for character in run.text.chars() {
                match character {
                    '\n' => {
                        max_width = f64::max(max_width, line_width);
                        line_width = 0.0;
                    }
                    '\r' => {}
                    _ => line_width += shared_estimated_char_width(character, font_size),
                }
            }
        }
        return f64::max(max_width, line_width);
    }
    text.lines()
        .map(|line| {
            line.chars()
                .filter(|character| *character != '\r')
                .map(|character| shared_estimated_char_width(character, default_font_size))
                .sum()
        })
        .fold(0.0, f64::max)
}

pub(crate) fn shared_estimated_text_line_count(text: &str, runs: &[crate::LabelRun]) -> usize {
    if !runs.is_empty() {
        return runs
            .iter()
            .map(|run| {
                run.text
                    .chars()
                    .filter(|character| *character == '\n')
                    .count()
            })
            .sum::<usize>()
            + 1;
    }
    text.lines().count().max(1)
}

pub(crate) fn shared_estimated_text_max_font_size(
    default_font_size: f64,
    runs: &[crate::LabelRun],
) -> f64 {
    runs.iter()
        .map(|run| {
            run.font_size.unwrap_or(default_font_size)
                * shared_script_scale_factor(run.script.as_deref())
        })
        .fold(default_font_size, f64::max)
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
    let placement = layout_glyph(
        character,
        script_kind(script),
        config,
        0.0,
        0.0,
        GlyphFaceRef {
            family: "Arial",
            weight: 400,
            italic: false,
        },
    );
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

fn lookup_glyph_profile(character: char) -> GlyphProfile {
    let shared = shared_glyph_profiles();
    if let Some(profile) = shared.specials.get(&character) {
        return *profile;
    }
    if character.is_whitespace() {
        return GlyphProfile {
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
        return replacement_rect_profile(1.0, -0.86, 1.0, 0.14);
    }
    if is_math_or_arrow_symbol(character) {
        return replacement_rect_profile(0.84, -0.74, 0.84, 0.06);
    }
    if matches!(character, '\u{2030}' | '\u{2031}') {
        return replacement_rect_profile(1.34, -0.74, 1.34, 0.06);
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
            return replacement_rect_profile(0.72, -0.74, 0.72, 0.04);
        }
        return replacement_rect_profile(0.62, -0.62, 0.62, 0.08);
    }
    if character.is_ascii_punctuation() {
        return default_punctuation_profile();
    }
    replacement_rect_profile(0.62, -0.74, 0.62, 0.08)
}

fn replacement_rect_profile(
    advance_em: f64,
    ink_top_em: f64,
    ink_right_em: f64,
    ink_bottom_em: f64,
) -> GlyphProfile {
    GlyphProfile {
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
    fn script_baseline_shifts_follow_measured_chemdraw_face_rules() {
        assert!((shared_script_scale_factor(Some("subscript")) - 0.75).abs() < 1e-12);
        assert!((shared_script_scale_factor(Some("superscript")) - 0.75).abs() < 1e-12);

        assert!(
            (shared_script_baseline_shift_em_for_face(
                Some("subscript"),
                Some(400),
                Some("Times New Roman"),
                7.0,
            ) - 0.243_214_894)
                .abs()
                < 1e-12
        );
        assert!(
            (shared_script_baseline_shift_em_for_face(
                Some("superscript"),
                Some(700),
                Some("Times New Roman"),
                7.0,
            ) + 0.392_679_553)
                .abs()
                < 1e-12
        );
        assert!(
            (shared_script_baseline_shift_em_for_face(
                Some("subscript"),
                Some(400),
                Some("Calibri"),
                14.45,
            ) - 0.27)
                .abs()
                < 1e-12
        );
        assert!(
            (shared_script_baseline_shift_em_for_face(
                Some("superscript"),
                Some(400),
                Some("Calibri"),
                18.0,
            ) + 0.365)
                .abs()
                < 1e-12
        );

        let generic = shared_script_baseline_shift_em(Some("subscript"), Some(400));
        assert_eq!(
            shared_script_baseline_shift_em_for_face(
                Some("subscript"),
                Some(400),
                Some("Arial"),
                10.0,
            ),
            generic
        );
    }

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
                "{character} should not use the narrow punctuation replacement profile: {profile:?}"
            );
            assert!(profile.visible, "{character} should be visible");
            assert!(
                profile.ink_bottom_em > profile.ink_top_em,
                "{character} should have a usable vertical ink box: {profile:?}"
            );
        }
    }

    #[test]
    fn unknown_cjk_text_uses_real_replacement_outline() {
        let character = '龘';
        let profile = lookup_glyph_profile(character);
        assert!(profile.advance_em >= 0.95, "{profile:?}");
        assert!(profile.ink_right_em >= 0.95, "{profile:?}");
        assert!(profile.visible);
        let placement = test_placement(character, "Arial", 10.0, 400, false);
        let geometry =
            glyph_geometry_with_profile(&placement, GlyphClipProfile::from_margin_width(1.0))
                .expect("unknown characters should resolve to a real replacement outline");
        assert!(geometry.glyph_polygon.len() >= 4);
        assert!(!geometry.clip_polygons.is_empty());
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
            outline: None,
            shadow: None,
            script: Some("normal".to_string()),
        }];
        let geometry = build_label_glyph_geometry_with_profile(
            &runs,
            &[],
            [0.0, 0.0],
            None,
            10.0,
            crate::molecule_label_line_advance(10.0),
            &[],
            [0.0, 0.0],
            GlyphClipProfile::from_margin_width(crate::DEFAULT_BOND_MARGIN_WIDTH_PT.value()),
        );
        assert_eq!(geometry.glyph_polygons.len(), 6, "{geometry:?}");
        assert!(geometry
            .glyph_polygons
            .iter()
            .all(|polygon| polygon.len() >= 4));
    }

    fn polygon_bounds(points: &[[f64; 2]]) -> [f64; 4] {
        let mut bounds = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for point in points {
            bounds[0] = bounds[0].min(point[0]);
            bounds[1] = bounds[1].min(point[1]);
            bounds[2] = bounds[2].max(point[0]);
            bounds[3] = bounds[3].max(point[1]);
        }
        bounds
    }

    fn test_placement(
        character: char,
        family: &str,
        size: f64,
        weight: u32,
        italic: bool,
    ) -> GlyphPlacement {
        let config = LayoutConfig {
            font_size_px: size,
            ..LayoutConfig::default()
        };
        layout_glyph(
            character,
            ScriptKind::Normal,
            config,
            0.0,
            0.0,
            GlyphFaceRef {
                family,
                weight,
                italic,
            },
        )
    }

    fn polygons_bounds(polygons: &[Vec<[f64; 2]>]) -> [f64; 4] {
        let mut bounds = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for polygon in polygons {
            let next = polygon_bounds(polygon);
            bounds[0] = bounds[0].min(next[0]);
            bounds[1] = bounds[1].min(next[1]);
            bounds[2] = bounds[2].max(next[2]);
            bounds[3] = bounds[3].max(next[3]);
        }
        bounds
    }

    #[test]
    fn outline_manifest_contains_measured_families_and_faces() {
        let manifest = shared_glyph_outlines();
        assert_eq!(manifest.version, 2);
        for family in ["Arial", "Times New Roman", "Calibri", "Cambria"] {
            let family = manifest.families.get(family).expect("measured family");
            for face in ["regular", "bold", "italic", "boldItalic"] {
                assert!(family.faces.contains_key(face), "missing {face}");
            }
        }
    }

    #[test]
    fn real_outline_and_retreat_geometry_are_built_together() {
        let placement = test_placement('N', "Arial", 10.0, 400, false);
        let geometry =
            glyph_geometry_with_profile(&placement, GlyphClipProfile::from_margin_width(1.0))
                .expect("N geometry");
        assert!(geometry.glyph_polygon.len() >= 4);
        assert!(geometry.clip_polygons.len() > geometry.glyph_polygon.len());
        let clip_bounds = polygons_bounds(&geometry.clip_polygons);
        assert!(clip_bounds[0] <= placement.ink_box_px[0] - 0.95);
        assert!(clip_bounds[1] <= placement.ink_box_px[1] - 0.95);
        assert!(clip_bounds[2] >= placement.ink_box_px[2] + 0.95);
        assert!(clip_bounds[3] >= placement.ink_box_px[3] + 0.95);
    }

    #[test]
    fn margin_width_expands_real_edges_and_internal_stroke_ends() {
        for character in ['F', 'r', 'C', '+'] {
            let placement = test_placement(character, "Arial", 10.0, 400, false);
            let one =
                glyph_geometry_with_profile(&placement, GlyphClipProfile::from_margin_width(1.0))
                    .unwrap();
            let two =
                glyph_geometry_with_profile(&placement, GlyphClipProfile::from_margin_width(2.0))
                    .unwrap();
            let one_bounds = polygons_bounds(&one.clip_polygons);
            let two_bounds = polygons_bounds(&two.clip_polygons);
            assert!(two_bounds[0] <= one_bounds[0] - 0.9, "{character}");
            assert!(two_bounds[1] <= one_bounds[1] - 0.9, "{character}");
            assert!(two_bounds[2] >= one_bounds[2] + 0.9, "{character}");
            assert!(two_bounds[3] >= one_bounds[3] + 0.9, "{character}");
        }
    }

    #[test]
    fn font_family_and_face_select_distinct_real_outlines() {
        let arial = test_placement('N', "Arial", 14.0, 400, false);
        let times = test_placement('N', "Times New Roman", 14.0, 400, false);
        let bold = test_placement('N', "Arial", 14.0, 700, false);
        let italic = test_placement('N', "Arial", 14.0, 400, true);
        assert_ne!(arial.ink_box_px, times.ink_box_px);
        assert_ne!(arial.ink_box_px, bold.ink_box_px);
        assert_ne!(arial.ink_box_px, italic.ink_box_px);
    }

    #[test]
    fn face_aware_advance_matches_times_new_roman_label_width() {
        let width = "HCO"
            .chars()
            .map(|character| {
                shared_estimated_char_width_for_face(
                    character,
                    7.0,
                    "Times New Roman",
                    400,
                    Some("normal"),
                )
            })
            .sum::<f64>();
        assert!(
            (width - 14.78).abs() < 0.02,
            "ChemDraw's 7 pt Times New Roman HCO advance is 14.78 pt, got {width}"
        );
        let generic_width = "HCO"
            .chars()
            .map(|character| shared_estimated_char_width(character, 7.0))
            .sum::<f64>();
        assert!(
            (generic_width - width).abs() > 0.5,
            "the generic profile must not silently replace face metrics"
        );
    }

    #[test]
    fn feature_margin_is_capped_at_quarter_em() {
        let placement = test_placement('A', "Arial", 8.0, 400, false);
        let two = glyph_geometry_with_profile(&placement, GlyphClipProfile::from_margin_width(2.0))
            .unwrap();
        let three =
            glyph_geometry_with_profile(&placement, GlyphClipProfile::from_margin_width(3.0))
                .unwrap();
        let hull_vertex_count = two.glyph_polygon.len();
        let two_feature = &two.clip_polygons[two.clip_polygons.len() - hull_vertex_count..];
        let three_feature = &three.clip_polygons[three.clip_polygons.len() - hull_vertex_count..];
        assert_eq!(two_feature, three_feature);
    }

    #[test]
    fn axial_contact_sectors_are_limited_to_ten_degrees() {
        let polygons = localized_axis_contact_polygons(
            &[AxisContactGlyph {
                bounds: [-2.0, -4.0, 5.0, 3.0],
                baseline_y: 0.0,
            }],
            [0.0, 0.0],
            1.0,
        );
        assert_eq!(polygons.len(), 4);
        let (right, owner) = &polygons[0];
        assert_eq!(*owner, None, "horizontal run envelopes stay shared");
        let angles: Vec<f64> = right[1..]
            .iter()
            .map(|point| point[1].atan2(point[0]).to_degrees())
            .collect();
        assert!((angles[0] + 10.0).abs() < 1e-9);
        assert!((angles[angles.len() - 1] - 10.0).abs() < 1e-9);
    }

    #[test]
    fn axial_contact_uses_only_glyphs_crossing_the_attachment_axis() {
        let glyphs = [
            AxisContactGlyph {
                bounds: [9.46, 1.18, 15.79, 8.58],
                baseline_y: 7.4,
            },
            AxisContactGlyph {
                bounds: [16.98, 1.30, 22.60, 8.46],
                baseline_y: 7.4,
            },
            AxisContactGlyph {
                bounds: [23.73, 5.27, 27.23, 10.76],
                baseline_y: 10.4,
            },
        ];
        let origin = [12.62, 4.56];
        let polygons = localized_axis_contact_polygons(&glyphs, origin, 1.6);
        let (down, owner) = polygons
            .iter()
            .find(|(polygon, _)| polygon.iter().skip(1).all(|point| point[1] > origin[1]))
            .expect("downward axial sector");
        assert_eq!(*owner, Some(0), "the crossed C-like glyph owns the contact");
        let maximum_y = down
            .iter()
            .map(|point| point[1])
            .max_by(f64::total_cmp)
            .expect("downward extent");

        assert!((maximum_y - 10.17315498123).abs() < 1.0e-9, "{maximum_y}");
        assert!(
            maximum_y < glyphs[2].bounds[3] + 1.6,
            "the distant subscript must not deepen the attachment column"
        );
    }

    #[test]
    fn horizontal_axial_contact_keeps_the_whole_text_run_envelope() {
        let glyphs = [
            AxisContactGlyph {
                bounds: [0.0, -4.0, 4.0, 3.0],
                baseline_y: 0.0,
            },
            AxisContactGlyph {
                bounds: [8.0, 4.0, 12.0, 8.0],
                baseline_y: 4.0,
            },
        ];
        let origin = [2.0, 0.0];
        let polygons = localized_axis_contact_polygons(&glyphs, origin, 1.0);
        let (right, owner) = polygons
            .iter()
            .find(|(polygon, _)| polygon.iter().skip(1).all(|point| point[0] > origin[0]))
            .expect("rightward axial sector");
        assert_eq!(*owner, None, "the horizontal run envelope is shared");
        let maximum_x = right
            .iter()
            .map(|point| point[0])
            .max_by(f64::total_cmp)
            .expect("rightward extent");

        assert!(
            maximum_x > 12.9,
            "an off-baseline glyph on the attachment-facing end remains in the horizontal envelope: {maximum_x}"
        );
    }

    #[test]
    fn multi_character_layout_keeps_character_index_and_compound_clip_separate() {
        let runs = vec![LabelRun {
            text: "NH2".to_string(),
            font_family: Some("Arial".to_string()),
            font_size: Some(10.0),
            font_weight: Some(700),
            font_style: Some("italic".to_string()),
            script: Some("normal".to_string()),
            ..LabelRun::default()
        }];
        let geometry = build_label_glyph_geometry_with_profile(
            &runs,
            &[],
            [0.0, 0.0],
            None,
            10.0,
            crate::molecule_label_line_advance(10.0),
            &[],
            [0.0, 0.0],
            GlyphClipProfile::from_margin_width(1.0),
        );
        assert_eq!(geometry.glyph_polygons.len(), 3);
        assert!(geometry.clip_polygons.len() > 3);
    }
}
