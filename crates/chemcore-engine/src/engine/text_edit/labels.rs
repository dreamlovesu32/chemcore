use super::*;

#[derive(Clone, Copy)]
pub(super) enum NodeLabelReplacement<'a> {
    Carbon,
    Element { element: &'a str, atomic_number: u8 },
    Abbreviation,
}

pub(super) fn classify_node_label_replacement_for_connection_count(
    label: &str,
    connection_count: usize,
) -> Option<NodeLabelReplacement<'_>> {
    if label_prefers_abbreviation_over_element(label, connection_count) {
        return Some(NodeLabelReplacement::Abbreviation);
    }
    parse_element_hydrogen_label(label)
        .and_then(|parsed| element_label_replacement(parsed.element))
        .or_else(|| element_label_replacement(label))
        .or_else(|| {
            crate::recognize_abbreviation_label_for_connection_count(label, connection_count)
                .map(|_| NodeLabelReplacement::Abbreviation)
        })
}

#[derive(Clone, Copy)]
pub(super) struct ParsedElementHydrogenLabel<'a> {
    element: &'a str,
}

pub(super) fn parse_element_hydrogen_label(label: &str) -> Option<ParsedElementHydrogenLabel<'_>> {
    let element = ELEMENT_REPLACEMENTS
        .iter()
        .map(|(element, _)| *element)
        .filter(|element| *element != "C" && *element != "H" && label.starts_with(element))
        .max_by_key(|element| element.len())?;
    let rest = &label[element.len()..];
    if rest.is_empty() {
        return Some(ParsedElementHydrogenLabel { element });
    }
    let hydrogen_suffix = rest.strip_prefix('H')?;
    if hydrogen_suffix.is_empty()
        || hydrogen_suffix
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return Some(ParsedElementHydrogenLabel { element });
    }
    None
}

pub(crate) fn element_symbol_info(label: &str) -> Option<(&'static str, u8)> {
    ELEMENT_REPLACEMENTS
        .iter()
        .find(|(element, _)| *element == label)
        .copied()
}

pub(super) fn element_label_replacement(label: &str) -> Option<NodeLabelReplacement<'_>> {
    element_symbol_info(label).map(|(element, atomic_number)| {
        if element == "C" {
            NodeLabelReplacement::Carbon
        } else {
            NodeLabelReplacement::Element {
                element,
                atomic_number,
            }
        }
    })
}

const ELEMENT_REPLACEMENTS: &[(&str, u8)] = &[
    ("H", 1),
    ("He", 2),
    ("Li", 3),
    ("Be", 4),
    ("B", 5),
    ("C", 6),
    ("N", 7),
    ("O", 8),
    ("F", 9),
    ("Ne", 10),
    ("Na", 11),
    ("Mg", 12),
    ("Al", 13),
    ("Si", 14),
    ("P", 15),
    ("S", 16),
    ("Cl", 17),
    ("Ar", 18),
    ("K", 19),
    ("Ca", 20),
    ("Sc", 21),
    ("Ti", 22),
    ("V", 23),
    ("Cr", 24),
    ("Mn", 25),
    ("Fe", 26),
    ("Co", 27),
    ("Ni", 28),
    ("Cu", 29),
    ("Zn", 30),
    ("Ga", 31),
    ("Ge", 32),
    ("As", 33),
    ("Se", 34),
    ("Br", 35),
    ("Kr", 36),
    ("Rb", 37),
    ("Sr", 38),
    ("Y", 39),
    ("Zr", 40),
    ("Nb", 41),
    ("Mo", 42),
    ("Tc", 43),
    ("Ru", 44),
    ("Rh", 45),
    ("Pd", 46),
    ("Ag", 47),
    ("Cd", 48),
    ("In", 49),
    ("Sn", 50),
    ("Sb", 51),
    ("Te", 52),
    ("I", 53),
    ("Xe", 54),
    ("Cs", 55),
    ("Ba", 56),
    ("La", 57),
    ("Ce", 58),
    ("Pr", 59),
    ("Nd", 60),
    ("Pm", 61),
    ("Sm", 62),
    ("Eu", 63),
    ("Gd", 64),
    ("Tb", 65),
    ("Dy", 66),
    ("Ho", 67),
    ("Er", 68),
    ("Tm", 69),
    ("Yb", 70),
    ("Lu", 71),
    ("Hf", 72),
    ("Ta", 73),
    ("W", 74),
    ("Re", 75),
    ("Os", 76),
    ("Ir", 77),
    ("Pt", 78),
    ("Au", 79),
    ("Hg", 80),
    ("Tl", 81),
    ("Pb", 82),
    ("Bi", 83),
    ("Po", 84),
    ("At", 85),
    ("Rn", 86),
    ("Fr", 87),
    ("Ra", 88),
    ("Ac", 89),
    ("Th", 90),
    ("Pa", 91),
    ("U", 92),
    ("Np", 93),
    ("Pu", 94),
    ("Am", 95),
    ("Cm", 96),
    ("Bk", 97),
    ("Cf", 98),
    ("Es", 99),
    ("Fm", 100),
    ("Md", 101),
    ("No", 102),
    ("Lr", 103),
    ("Rf", 104),
    ("Db", 105),
    ("Sg", 106),
    ("Bh", 107),
    ("Hs", 108),
    ("Mt", 109),
    ("Ds", 110),
    ("Rg", 111),
    ("Cn", 112),
    ("Nh", 113),
    ("Fl", 114),
    ("Mc", 115),
    ("Lv", 116),
    ("Ts", 117),
    ("Og", 118),
    ("D", 1),
];

pub(crate) fn standalone_element_hydrogen_count(atomic_number: u8) -> u8 {
    match atomic_number {
        1 => 1,
        5 => 3,
        6 => 4,
        7 => 3,
        8 => 2,
        9 => 1,
        _ => third_period_main_group_valence_series(atomic_number)
            .map(|(base_valence, _)| base_valence as u8)
            .unwrap_or(0),
    }
}

fn third_period_main_group_valence_series(atomic_number: u8) -> Option<(i32, i32)> {
    match atomic_number {
        13 | 31 | 49 | 81 | 113 => Some((3, 3)),
        14 | 32 | 50 | 82 | 114 => Some((4, 4)),
        15 | 33 | 51 | 83 | 115 => Some((3, 5)),
        16 | 34 | 52 | 84 | 116 => Some((2, 6)),
        17 | 35 | 53 | 85 | 117 => Some((1, 7)),
        _ => None,
    }
}

fn third_period_main_group_target_valence(atomic_number: u8, used_valence: i32) -> Option<i32> {
    let (base_valence, max_valence) = third_period_main_group_valence_series(atomic_number)?;
    if used_valence >= max_valence {
        return Some(max_valence);
    }
    let mut target = base_valence;
    while target < used_valence {
        target += 2;
    }
    Some(target.min(max_valence))
}

fn implicit_hydrogen_charge_penalty(atomic_number: u8, charge: i32) -> i32 {
    if third_period_main_group_valence_series(atomic_number).is_some() {
        charge.abs()
    } else if charge > 0 {
        0
    } else {
        charge.abs()
    }
}

pub(super) fn make_centered_node_label(text: &str, position: [f64; 2]) -> crate::NodeLabel {
    let font_size = DEFAULT_CENTERED_LABEL_FONT_SIZE;
    let (label_position, label_box) = estimated_centered_label_geometry(text, position, font_size);
    crate::NodeLabel {
        text: text.to_string(),
        source_text: Some(text.to_string()),
        position: Some(label_position),
        box_field: Some(label_box),
        runs: vec![crate::LabelRun {
            text: text.to_string(),
            font_family: Some("Arial".to_string()),
            font_size: Some(font_size),
            fill: Some("#000000".to_string()),
            font_weight: Some(700),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            script: Some("normal".to_string()),
        }],
        line_runs: Vec::new(),
        lines: Vec::new(),
        align: Some("center".to_string()),
        layout: None,
        attachment: None,
        anchor: Some("middle".to_string()),
        font_family: Some("Arial".to_string()),
        fill: Some("#000000".to_string()),
        font_size: Some(font_size),
        glyph_polygons: Vec::new(),
        box_value: Some(label_box),
        meta: serde_json::Value::Null,
    }
}

pub(super) fn endpoint_session_box_size(session: &TextEditSession) -> Option<(f64, f64)> {
    let [x1, y1, x2, y2] = session.box_value?;
    let width = (x2 - x1).abs();
    let height = (y2 - y1).abs();
    if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
        Some((width, height))
    } else {
        None
    }
}

pub(super) fn make_centered_node_label_from_runs(
    text: &str,
    position: [f64; 2],
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    font_family: &str,
    font_size: f64,
    fill: &str,
    connection_angles: &[f64],
    session: &TextEditSession,
    preserve_measured_box: bool,
    treat_as_literal_text_mode: bool,
    forced_flow: Option<LabelFlow>,
) -> crate::NodeLabel {
    let mut decision = label_layout_decision_for_text_mode(
        text,
        connection_angles,
        if treat_as_literal_text_mode {
            false
        } else {
            source_runs_are_chemical(&source_runs)
        },
    );
    if let Some(flow) = forced_flow {
        decision.flow = flow;
    }
    let layout = layout_label_text(text, &decision);
    let (lines, line_runs) = layout_display_runs(&display_runs, &decision);
    let anchor_char = label_anchor_char_for_layout(&line_runs, &layout);
    let line_height = (font_size * 1.05).max(font_size);
    let estimated_width = lines
        .iter()
        .zip(line_runs.iter())
        .map(|(_, runs)| estimate_line_runs_width(runs, font_size))
        .fold(font_size * 0.6, f64::max);
    let estimated_height = round2((line_height * lines.len().max(1) as f64).max(line_height));
    let anchor_prefix_width = line_runs
        .get(layout.anchor_line)
        .map(|runs| estimate_prefix_width(runs, anchor_char, font_size))
        .unwrap_or(0.0);
    let anchor_char_width = line_runs
        .get(layout.anchor_line)
        .and_then(|runs| estimate_anchor_char_width(runs, anchor_char, font_size))
        .unwrap_or(font_size * 0.62);
    let anchor_center_x = anchor_prefix_width + anchor_char_width * 0.5;
    let can_preserve_imported_single_line_box =
        preserve_measured_box && lines.len() == 1 && layout.anchor_line == 0;
    let can_use_measured_geometry = (matches!(decision.flow, LabelFlow::Forward)
        || can_preserve_imported_single_line_box)
        && lines.len() == 1
        && layout.anchor_line == 0;
    let measured_anchor = session
        .anchor_offset_world_pt()
        .map(|value| (value[0].value(), value[1].value()));
    let measured_box_size = endpoint_session_box_size(session);
    let fallback_geometry = || {
        let x1 = round2(position[0] - anchor_center_x);
        let y1 = round2(position[1] - font_size * 0.42 - layout.anchor_line as f64 * line_height);
        let baseline_y = round2(y1 + layout.anchor_line as f64 * line_height + font_size * 0.82);
        (estimated_width, estimated_height, x1, y1, baseline_y)
    };
    let (width, height, mut x1, mut y1, mut baseline_y) = if can_use_measured_geometry {
        if let (Some((anchor_offset_x, anchor_offset_y)), Some((measured_width, measured_height))) =
            (measured_anchor, measured_box_size)
        {
            const MAX_MEASURED_SIZE_RATIO: f64 = 8.0;
            let max_width = estimated_width.max(font_size) * MAX_MEASURED_SIZE_RATIO;
            let max_height = estimated_height.max(font_size) * MAX_MEASURED_SIZE_RATIO;
            let valid_anchor_x = anchor_offset_x.is_finite()
                && anchor_offset_x >= -estimated_width * 0.25
                && anchor_offset_x <= max_width;
            let valid_anchor_y = anchor_offset_y.is_finite()
                && anchor_offset_y >= -estimated_height * 0.25
                && anchor_offset_y <= max_height;
            let valid_size = measured_width.is_finite()
                && measured_height.is_finite()
                && measured_width > 0.0
                && measured_height > 0.0
                && measured_width <= max_width
                && measured_height <= max_height;
            if valid_anchor_x && valid_anchor_y && valid_size {
                let x1 = round2(position[0] - anchor_offset_x);
                let y1 = round2(position[1] - anchor_offset_y);
                let width = round2(if preserve_measured_box {
                    measured_width
                } else {
                    measured_width.max(estimated_width)
                });
                let height = round2(if preserve_measured_box {
                    measured_height
                } else {
                    measured_height.max(estimated_height)
                });
                let baseline_y = round2(y1 + font_size * 0.82);
                (width, height, x1, y1, baseline_y)
            } else {
                fallback_geometry()
            }
        } else {
            fallback_geometry()
        }
    } else {
        fallback_geometry()
    };
    let mut x2 = round2(x1 + width);
    let mut y2 = round2(y1 + height);
    let mut meta = serde_json::Map::new();
    meta.insert(
        "sourceRuns".to_string(),
        serde_json::to_value(source_runs).unwrap_or(Value::Array(Vec::new())),
    );
    let mut glyph_polygons = build_label_glyph_polygons(
        if line_runs.len() == 1 {
            line_runs.first().map(Vec::as_slice).unwrap_or(&[])
        } else {
            &[]
        },
        if line_runs.len() > 1 { &line_runs } else { &[] },
        [x1, baseline_y],
        Some([x1, y1, x2, y2]),
        font_size,
    );
    if let Some(anchor_polygon_index) =
        label_anchor_polygon_index(&line_runs, layout.anchor_line, anchor_char)
    {
        if let Some(current_anchor) = glyph_polygons
            .get(anchor_polygon_index)
            .and_then(|polygon| {
                let points: Vec<_> = polygon
                    .iter()
                    .map(|point| Point::new(point[0], point[1]))
                    .collect();
                polygon_anchor_point(&points)
            })
        {
            let dx = round2(position[0] - current_anchor.x);
            let dy = round2(position[1] - current_anchor.y);
            if dx.abs() > crate::EPSILON || dy.abs() > crate::EPSILON {
                x1 = round2(x1 + dx);
                y1 = round2(y1 + dy);
                x2 = round2(x2 + dx);
                y2 = round2(y2 + dy);
                baseline_y = round2(baseline_y + dy);
                for polygon in &mut glyph_polygons {
                    for point in polygon {
                        point[0] = round2(point[0] + dx);
                        point[1] = round2(point[1] + dy);
                    }
                }
            }
        }
    }
    crate::NodeLabel {
        text: layout.rendered_text,
        source_text: Some(text.to_string()),
        position: Some([x1, baseline_y]),
        box_field: Some([x1, y1, x2, y2]),
        runs: if line_runs.len() == 1 {
            line_runs.first().cloned().unwrap_or_default()
        } else {
            Vec::new()
        },
        line_runs: if line_runs.len() > 1 {
            line_runs
        } else {
            Vec::new()
        },
        lines: if lines.len() > 1 {
            lines.clone()
        } else {
            Vec::new()
        },
        align: Some("left".to_string()),
        layout: Some(match decision.flow {
            LabelFlow::StackAbove => "attached-group-above".to_string(),
            _ => "attached-group".to_string(),
        }),
        attachment: Some("node".to_string()),
        anchor: Some("start".to_string()),
        font_family: Some(font_family.to_string()),
        fill: Some(fill.to_string()),
        font_size: Some(font_size),
        glyph_polygons,
        box_value: Some([x1, y1, x2, y2]),
        meta: Value::Object(meta),
    }
}

fn label_anchor_polygon_index(
    line_runs: &[Vec<LabelRun>],
    anchor_line: usize,
    anchor_char: usize,
) -> Option<usize> {
    let mut index = 0usize;
    for (line_index, runs) in line_runs.iter().enumerate() {
        let line_len: usize = runs.iter().map(|run| run.text.chars().count()).sum();
        if line_index == anchor_line {
            return (anchor_char < line_len).then_some(index + anchor_char);
        }
        index += line_len;
    }
    None
}

pub(super) fn label_layout_decision_for_text_mode(
    text: &str,
    connection_angles: &[f64],
    is_chemical_label: bool,
) -> crate::LabelLayoutDecision {
    let mut decision = decide_label_layout(connection_angles, false, false);
    if !is_chemical_label {
        if !matches!(decision.flow, LabelFlow::Reverse) {
            decision.flow = LabelFlow::Forward;
        }
        decision.anchor = crate::LabelAnchorPolicy::WholeLabel;
        return decision;
    }
    if label_should_render_as_whole_group(text, connection_angles.len()) {
        decision.anchor = crate::LabelAnchorPolicy::WholeLabel;
    } else if matches!(decision.flow, LabelFlow::Reverse) {
        if parse_element_hydrogen_label(text).is_some()
            || crate::recognize_abbreviation_label_for_connection_count(
                text.trim(),
                connection_angles.len(),
            )
            .is_some()
        {
            decision.anchor = crate::LabelAnchorPolicy::OriginalFirstGroup;
        }
    }
    decision
}

pub(super) fn label_should_render_as_whole_group(text: &str, connection_count: usize) -> bool {
    if crate::recognized_abbreviation_uses_whole_label_layout(text.trim(), connection_count) {
        return true;
    }
    label_recognition_meta_for_text(text, connection_count)
        .is_some_and(|meta| meta.get("status").and_then(Value::as_str) == Some("invalid"))
}

#[derive(Clone)]
pub(super) struct StyledGlyph {
    ch: char,
    run: LabelRun,
}

pub(super) fn layout_display_runs(
    display_runs: &[LabelRun],
    decision: &crate::LabelLayoutDecision,
) -> (Vec<String>, Vec<Vec<LabelRun>>) {
    let groups = split_styled_groups(
        display_runs,
        decision.anchor == crate::LabelAnchorPolicy::WholeLabel,
    );
    if groups.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let lines = match decision.flow {
        LabelFlow::Forward => vec![groups.concat()],
        LabelFlow::Reverse => vec![groups.into_iter().rev().flatten().collect()],
        LabelFlow::StackAbove => {
            if groups.len() > 1 {
                vec![groups[1..].concat(), groups[0].clone()]
            } else {
                vec![groups[0].clone()]
            }
        }
        LabelFlow::StackBelow => {
            if groups.len() > 1 {
                vec![groups[0].clone(), groups[1..].concat()]
            } else {
                vec![groups[0].clone()]
            }
        }
    };
    let line_texts = lines
        .iter()
        .map(|line| line.iter().map(|glyph| glyph.ch).collect::<String>())
        .collect::<Vec<_>>();
    let line_runs = lines
        .iter()
        .map(|line| merge_styled_glyph_runs(line))
        .collect();
    (line_texts, line_runs)
}

pub(super) fn split_styled_groups(
    display_runs: &[LabelRun],
    whole_label: bool,
) -> Vec<Vec<StyledGlyph>> {
    let mut groups = Vec::new();
    let mut current = Vec::new();
    let mut glyphs = Vec::new();
    for run in display_runs {
        for ch in run.text.chars() {
            if ch.is_whitespace() {
                continue;
            }
            glyphs.push(StyledGlyph {
                ch,
                run: LabelRun {
                    text: ch.to_string(),
                    font_family: run.font_family.clone(),
                    font_size: run.font_size,
                    fill: run.fill.clone(),
                    font_weight: run.font_weight,
                    font_style: run.font_style.clone(),
                    underline: run.underline,
                    script: run.script.clone(),
                },
            });
        }
    }
    if whole_label {
        if glyphs.is_empty() {
            return Vec::new();
        }
        return vec![glyphs];
    }
    let compact_text = glyphs.iter().map(|glyph| glyph.ch).collect::<String>();
    let mut glyph_index = 0usize;
    let mut byte_index = 0usize;
    while glyph_index < glyphs.len() && byte_index < compact_text.len() {
        let rest = &compact_text[byte_index..];
        if let Some(prefix_len) = crate::label_group_abbreviation_prefix_len(rest) {
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
            let char_count = rest[..prefix_len].chars().count();
            groups.push(glyphs[glyph_index..glyph_index + char_count].to_vec());
            glyph_index += char_count;
            byte_index += prefix_len;
            continue;
        }
        let glyph = glyphs[glyph_index].clone();
        if glyph.ch.is_ascii_uppercase() && !current.is_empty() {
            groups.push(std::mem::take(&mut current));
        }
        byte_index += glyph.ch.len_utf8();
        glyph_index += 1;
        current.push(glyph);
    }
    if !current.is_empty() {
        groups.push(current);
    }
    groups
}

pub(super) fn merge_styled_glyph_runs(line: &[StyledGlyph]) -> Vec<LabelRun> {
    let mut runs: Vec<LabelRun> = Vec::new();
    for glyph in line {
        if let Some(previous) = runs.last_mut() {
            if previous.font_family == glyph.run.font_family
                && previous.font_size == glyph.run.font_size
                && previous.fill == glyph.run.fill
                && previous.font_weight == glyph.run.font_weight
                && previous.font_style == glyph.run.font_style
                && previous.underline == glyph.run.underline
                && previous.script == glyph.run.script
            {
                previous.text.push(glyph.ch);
                continue;
            }
        }
        let mut next = glyph.run.clone();
        next.text = glyph.ch.to_string();
        runs.push(next);
    }
    runs
}

pub(super) fn label_anchor_index_for_layout(
    line_runs: &[Vec<LabelRun>],
    layout: &crate::LabelLayout,
) -> usize {
    let local_anchor = label_anchor_char_for_layout(line_runs, layout);
    line_runs
        .iter()
        .take(layout.anchor_line)
        .map(|line| {
            line.iter()
                .map(|run| run.text.chars().count())
                .sum::<usize>()
        })
        .sum::<usize>()
        + local_anchor
}

fn label_anchor_char_for_layout(line_runs: &[Vec<LabelRun>], layout: &crate::LabelLayout) -> usize {
    line_runs
        .get(layout.anchor_line)
        .map(|runs| label_anchor_char_for_runs(runs, layout.anchor_char))
        .unwrap_or(layout.anchor_char)
}

fn label_anchor_char_for_runs(runs: &[LabelRun], fallback_index: usize) -> usize {
    let mut shifted = Vec::new();
    for run in runs {
        let is_shifted = matches!(run.script.as_deref(), Some("subscript" | "superscript"));
        shifted.extend(run.text.chars().map(|_| is_shifted));
    }
    if shifted.is_empty() {
        return fallback_index;
    }

    let fallback_index = fallback_index.min(shifted.len() - 1);
    if !shifted[fallback_index] {
        return fallback_index;
    }
    (0..=fallback_index)
        .rev()
        .find(|index| !shifted[*index])
        .or_else(|| (fallback_index + 1..shifted.len()).find(|index| !shifted[*index]))
        .unwrap_or(fallback_index)
}

pub(super) fn estimate_line_runs_width(runs: &[LabelRun], fallback_font_size: f64) -> f64 {
    runs.iter().fold(0.0, |width, run| {
        let run_font_size = run.font_size.unwrap_or(fallback_font_size)
            * crate::glyph_kernel::shared_script_scale_factor(run.script.as_deref());
        width
            + run
                .text
                .chars()
                .map(|ch| estimated_char_width(ch, run_font_size))
                .sum::<f64>()
    })
}

pub(super) fn estimate_prefix_width(
    runs: &[LabelRun],
    char_count: usize,
    fallback_font_size: f64,
) -> f64 {
    let mut remaining = char_count;
    let mut width = 0.0;
    for run in runs {
        if remaining == 0 {
            break;
        }
        let run_font_size = run.font_size.unwrap_or(fallback_font_size)
            * crate::glyph_kernel::shared_script_scale_factor(run.script.as_deref());
        for ch in run.text.chars() {
            if remaining == 0 {
                break;
            }
            width += estimated_char_width(ch, run_font_size);
            remaining -= 1;
        }
    }
    width
}

pub(super) fn estimate_anchor_char_width(
    runs: &[LabelRun],
    char_index: usize,
    fallback_font_size: f64,
) -> Option<f64> {
    let mut current_index = 0usize;
    for run in runs {
        let run_font_size = run.font_size.unwrap_or(fallback_font_size)
            * crate::glyph_kernel::shared_script_scale_factor(run.script.as_deref());
        for ch in run.text.chars() {
            if current_index == char_index {
                return Some(estimated_char_width(ch, run_font_size));
            }
            current_index += 1;
        }
    }
    None
}

pub(super) fn adjacent_angles_for_fragment_node(
    fragment: &crate::MoleculeFragment,
    node_id: &str,
) -> Vec<f64> {
    let Some(node) = fragment.nodes.iter().find(|node| node.id == node_id) else {
        return Vec::new();
    };
    let point = Point::new(node.position[0], node.position[1]);
    let mut out = Vec::new();
    for bond in &fragment.bonds {
        if bond.begin != node_id && bond.end != node_id {
            continue;
        }
        let other_id = if bond.begin == node_id {
            &bond.end
        } else {
            &bond.begin
        };
        let Some(other) = fragment.nodes.iter().find(|node| &node.id == other_id) else {
            continue;
        };
        out.push(crate::angle_between(
            point,
            Point::new(other.position[0], other.position[1]),
        ));
    }
    out
}

pub(super) fn same_node_label(
    current: Option<&crate::NodeLabel>,
    next: Option<&crate::NodeLabel>,
) -> bool {
    match (current, next) {
        (None, None) => true,
        (Some(current), Some(next)) => {
            current.text == next.text
                && current.align == next.align
                && current.runs == next.runs
                && current.font_family == next.font_family
                && current.font_size == next.font_size
                && current.fill == next.fill
                && current.meta == next.meta
        }
        _ => false,
    }
}

pub(super) fn label_recognition_meta_from_node(node: &crate::Node) -> Option<Value> {
    node.meta.get("labelRecognition").cloned()
}

pub(super) fn label_source_text(label: &crate::NodeLabel) -> String {
    let source_runs = source_runs_from_node_label(label);
    if !source_runs.is_empty() {
        runs_text(&source_runs)
    } else {
        label
            .source_text
            .clone()
            .unwrap_or_else(|| label.text.clone())
    }
}

pub(super) fn label_recognition_meta_for_text(
    text: &str,
    connection_count: usize,
) -> Option<Value> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed == "C" {
        return None;
    }
    if label_prefers_abbreviation_over_element(trimmed, connection_count) {
        return crate::recognized_abbreviation_meta_for_connection_count(trimmed, connection_count);
    }
    if parse_element_hydrogen_label(trimmed)
        .and_then(|parsed| element_label_replacement(parsed.element))
        .or_else(|| element_label_replacement(trimmed))
        .is_some()
    {
        return None;
    }
    crate::recognized_abbreviation_meta_for_connection_count(trimmed, connection_count)
        .or_else(|| Some(crate::invalid_abbreviation_meta(trimmed)))
}

pub(super) fn label_recognition_meta_for_node_text(
    fragment: &crate::MoleculeFragment,
    node_id: &str,
    text: &str,
) -> Option<Value> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed == "C" {
        return None;
    }
    let connection_count = fragment
        .bonds
        .iter()
        .filter(|bond| bond.begin == node_id || bond.end == node_id)
        .count();
    let Some(node) = fragment.nodes.iter().find(|node| node.id == node_id) else {
        return label_recognition_meta_for_text(trimmed, connection_count);
    };
    if is_bullet_carbon_atom_label(trimmed, node) {
        return None;
    }
    if label_prefers_abbreviation_over_element(trimmed, connection_count) {
        return crate::recognized_abbreviation_meta_for_connection_count(trimmed, connection_count);
    }
    if parse_element_hydrogen_label(trimmed)
        .and_then(|parsed| element_label_replacement(parsed.element).map(|_| parsed))
        .is_some()
        || element_label_replacement(trimmed).is_some()
    {
        return if element_hydrogen_label_is_valid_for_node(trimmed, fragment, node) {
            None
        } else {
            Some(crate::invalid_abbreviation_meta(trimmed))
        };
    }
    if connection_count == 0 {
        return Some(crate::invalid_abbreviation_meta(trimmed));
    }
    crate::recognized_abbreviation_meta_for_connection_count(trimmed, connection_count)
        .or_else(|| Some(crate::invalid_abbreviation_meta(trimmed)))
}

fn label_prefers_abbreviation_over_element(label: &str, connection_count: usize) -> bool {
    let trimmed = label.trim();
    trimmed == "Ar"
        && connection_count > 0
        && crate::recognize_abbreviation_label_for_connection_count(trimmed, connection_count)
            .is_some_and(|recognition| recognition.canonical_label == "Ar")
}

fn is_bullet_carbon_atom_label(text: &str, node: &crate::Node) -> bool {
    text == "•" && node.element == "C" && node.atomic_number == 6
}

pub(super) fn element_hydrogen_label_is_valid_for_node(
    text: &str,
    fragment: &crate::MoleculeFragment,
    node: &crate::Node,
) -> bool {
    if node.is_placeholder {
        return false;
    }
    let trimmed = text.trim();
    if trimmed == "C" {
        return node.element == "C" && node.atomic_number == 6;
    }
    if !parse_element_hydrogen_label(trimmed).is_some_and(|parsed| parsed.element == node.element)
        && trimmed != node.element
    {
        return false;
    }
    if !element_valence_is_valid_for_node(fragment, node) {
        return false;
    }
    let expected_hydrogens = implicit_hydrogen_count(fragment, node.id.as_str());
    trimmed == implicit_hydrogen_label_text_for_count(&node.element, expected_hydrogens)
}

pub(super) fn element_valence_is_valid_for_node(
    fragment: &crate::MoleculeFragment,
    node: &crate::Node,
) -> bool {
    if node.is_placeholder || node.atomic_number == 1 || node.atomic_number == 6 {
        return true;
    }
    let connection_order: i32 = fragment
        .bonds
        .iter()
        .filter(|bond| bond.begin == node.id || bond.end == node.id)
        .map(|bond| i32::from(bond.order.max(1)))
        .sum();
    let radical_count = 0;
    let charge = node.charge;
    let abs_charge = charge.abs();
    let Some(valence) = typical_valence_for_implicit_hydrogen(
        node.atomic_number,
        charge,
        connection_order,
        radical_count,
        abs_charge,
    ) else {
        return true;
    };
    if third_period_main_group_valence_series(node.atomic_number).is_some() {
        connection_order + radical_count + abs_charge <= valence
    } else {
        connection_order <= valence
    }
}

pub(super) fn set_node_label_recognition_meta(node: &mut crate::Node, meta: Option<Value>) {
    set_meta_object_field(&mut node.meta, "labelRecognition", meta);
}

pub(super) fn set_label_recognition_meta(label: &mut crate::NodeLabel, meta: Option<Value>) {
    set_meta_object_field(&mut label.meta, "labelRecognition", meta);
}

pub(super) fn implicit_hydrogen_label_meta(label: &crate::NodeLabel) -> Option<&Value> {
    label.meta.get(IMPLICIT_HYDROGEN_LABEL_META_KEY)
}

pub(super) fn implicit_hydrogen_label_meta_value(source: &str, user_edited: bool) -> Value {
    json!({
        "source": source,
        "userEdited": user_edited,
    })
}

pub(super) fn implicit_hydrogen_label_source(meta: &Value) -> Option<&str> {
    meta.get("source").and_then(Value::as_str)
}

pub(super) fn implicit_hydrogen_label_user_edited(meta: &Value) -> bool {
    meta.get("userEdited")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(super) fn implicit_hydrogen_label_is_user_edited(label: &crate::NodeLabel) -> bool {
    implicit_hydrogen_label_meta(label).is_some_and(implicit_hydrogen_label_user_edited)
}

pub(super) fn set_node_implicit_hydrogen_label_meta(node: &mut crate::Node, meta: Option<Value>) {
    set_meta_object_field(&mut node.meta, IMPLICIT_HYDROGEN_LABEL_META_KEY, meta);
}

pub(super) fn set_label_implicit_hydrogen_label_meta(
    label: &mut crate::NodeLabel,
    meta: Option<Value>,
) {
    set_meta_object_field(&mut label.meta, IMPLICIT_HYDROGEN_LABEL_META_KEY, meta);
}

pub(crate) fn mark_shortcut_implicit_hydrogen_label(node: &mut crate::Node, label: &str) {
    if element_label_replacement(label)
        .is_some_and(|replacement| matches!(replacement, NodeLabelReplacement::Element { .. }))
    {
        let meta = implicit_hydrogen_label_meta_value("shortcut", false);
        set_node_implicit_hydrogen_label_meta(node, Some(meta.clone()));
        if let Some(label) = node.label.as_mut() {
            set_label_implicit_hydrogen_label_meta(label, Some(meta));
        }
    } else {
        set_node_implicit_hydrogen_label_meta(node, None);
        if let Some(label) = node.label.as_mut() {
            set_label_implicit_hydrogen_label_meta(label, None);
        }
    }
}

pub(super) fn set_meta_object_field(meta_value: &mut Value, key: &str, value: Option<Value>) {
    if !meta_value.is_object() {
        *meta_value = Value::Object(serde_json::Map::new());
    }
    let Some(object) = meta_value.as_object_mut() else {
        return;
    };
    match value {
        Some(value) => {
            object.insert(key.to_string(), value);
        }
        None => {
            object.remove(key);
        }
    }
    if object.is_empty() {
        *meta_value = Value::Null;
    }
}

pub(super) fn estimated_centered_label_geometry(
    text: &str,
    center: [f64; 2],
    font_size: f64,
) -> ([f64; 2], [f64; 4]) {
    let width = text
        .chars()
        .map(|ch| estimated_char_width(ch, font_size))
        .sum::<f64>()
        .max(crate::glyph_kernel::shared_estimated_char_width(
            'C', font_size,
        ));
    let height = (font_size * 0.84).max(crate::px_to_pt(8.0));
    let half_width = width * 0.5;
    let half_height = height * 0.5;
    let x1 = center[0] - half_width;
    let y1 = center[1] - half_height;
    let x2 = center[0] + half_width;
    let y2 = center[1] + half_height;
    ([center[0], y2], [x1, y1, x2, y2])
}

pub(crate) fn refresh_attached_node_label_geometry_for_all_nodes(
    fragment: &mut crate::MoleculeFragment,
    object_translate: [f64; 2],
    stroke_width: f64,
) {
    refresh_implicit_hydrogens(fragment);
    let node_ids: Vec<_> = fragment.nodes.iter().map(|node| node.id.clone()).collect();
    for node_id in node_ids {
        refresh_attached_node_label_geometry_for_node_inner(
            fragment,
            object_translate,
            &node_id,
            stroke_width,
        );
    }
}

pub(crate) fn refresh_attached_node_label_geometry_for_node(
    fragment: &mut crate::MoleculeFragment,
    object_translate: [f64; 2],
    node_id: &str,
    stroke_width: f64,
) {
    refresh_implicit_hydrogens(fragment);
    refresh_attached_node_label_geometry_for_node_inner(
        fragment,
        object_translate,
        node_id,
        stroke_width,
    );
}

pub(super) fn refresh_attached_node_label_geometry_for_node_inner(
    fragment: &mut crate::MoleculeFragment,
    object_translate: [f64; 2],
    node_id: &str,
    stroke_width: f64,
) {
    let Some(node_index) = fragment.nodes.iter().position(|node| node.id == node_id) else {
        return;
    };
    refresh_label_recognition_for_node(fragment, node_id);
    let Some(next_label) =
        refreshed_attached_node_label(fragment, node_id, object_translate, stroke_width)
    else {
        return;
    };
    fragment.nodes[node_index].label = Some(next_label);
    refresh_label_recognition_for_node(fragment, node_id);
}

pub(super) fn refresh_label_recognition_for_node(
    fragment: &mut crate::MoleculeFragment,
    node_id: &str,
) {
    let Some(node_index) = fragment.nodes.iter().position(|node| node.id == node_id) else {
        return;
    };
    let Some(label) = fragment.nodes[node_index].label.as_ref() else {
        set_node_label_recognition_meta(&mut fragment.nodes[node_index], None);
        set_node_implicit_hydrogen_label_meta(&mut fragment.nodes[node_index], None);
        return;
    };
    if !source_runs_are_chemical(&source_runs_from_node_label(label)) {
        let node = &mut fragment.nodes[node_index];
        set_node_label_recognition_meta(node, None);
        if let Some(label) = node.label.as_mut() {
            set_label_recognition_meta(label, None);
        }
        return;
    }
    let text = label_source_text(label);
    let recognition_meta = label_recognition_meta_for_node_text(fragment, node_id, &text);
    let node = &mut fragment.nodes[node_index];
    set_node_label_recognition_meta(node, recognition_meta.clone());
    if let Some(label) = node.label.as_mut() {
        set_label_recognition_meta(label, recognition_meta);
    }
}

pub(crate) fn refresh_element_valence_recognition_for_all_nodes(
    fragment: &mut crate::MoleculeFragment,
) {
    let node_ids: Vec<_> = fragment.nodes.iter().map(|node| node.id.clone()).collect();
    for node_id in node_ids {
        refresh_element_valence_recognition_for_node(fragment, &node_id);
    }
}

fn refresh_element_valence_recognition_for_node(
    fragment: &mut crate::MoleculeFragment,
    node_id: &str,
) {
    let Some(node_index) = fragment.nodes.iter().position(|node| node.id == node_id) else {
        return;
    };
    let Some(label) = fragment.nodes[node_index].label.as_ref() else {
        return;
    };
    if !source_runs_are_chemical(&source_runs_from_node_label(label)) {
        return;
    }
    let text = label_source_text(label);
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed == "C" {
        return;
    }
    let is_element_label = parse_element_hydrogen_label(trimmed)
        .and_then(|parsed| element_label_replacement(parsed.element).map(|_| parsed))
        .is_some()
        || element_label_replacement(trimmed).is_some();
    if !is_element_label {
        return;
    }
    let recognition_meta = {
        let node = &fragment.nodes[node_index];
        if element_hydrogen_label_is_valid_for_node(trimmed, fragment, node) {
            None
        } else {
            Some(crate::invalid_abbreviation_meta(trimmed))
        }
    };
    let node = &mut fragment.nodes[node_index];
    set_node_label_recognition_meta(node, recognition_meta.clone());
    if let Some(label) = node.label.as_mut() {
        set_label_recognition_meta(label, recognition_meta);
    }
}

pub(super) fn is_generated_centered_label(label: &crate::NodeLabel) -> bool {
    label.align.as_deref() == Some("center")
        && label.anchor.as_deref() == Some("middle")
        && label.glyph_polygons.is_empty()
        && label.runs.len() == 1
}

pub(super) fn is_attached_node_label(label: &crate::NodeLabel) -> bool {
    label.attachment.as_deref() == Some("node")
        && label.align.as_deref() == Some("left")
        && label.anchor.as_deref() == Some("start")
}

fn is_cdxml_imported_right_aligned_attached_label(label: &crate::NodeLabel) -> bool {
    label.attachment.as_deref() == Some("node")
        && label.align.as_deref() == Some("right")
        && label.meta.pointer("/import/cdxml/boundingBox").is_some()
}

fn is_cdxml_imported_single_character_centered_label(label: &crate::NodeLabel) -> bool {
    is_cdxml_imported_centered_attached_label(label)
        && label
            .source_text
            .as_deref()
            .unwrap_or(label.text.as_str())
            .chars()
            .count()
            == 1
}

fn is_cdxml_imported_centered_attached_label(label: &crate::NodeLabel) -> bool {
    label.attachment.as_deref() == Some("node")
        && label.align.as_deref() == Some("center")
        && label.meta.pointer("/import/cdxml/boundingBox").is_some()
}

fn cdxml_imported_label_alignment_is_horizontal_only(label: &crate::NodeLabel) -> bool {
    let alignment = label
        .meta
        .pointer("/import/cdxml/labelAlignment")
        .and_then(serde_json::Value::as_str);
    match alignment {
        Some(value) => matches!(value, "Left" | "Center" | "Right"),
        None => true,
    }
}

fn imported_cdxml_label_geometry_is_authoritative(label: &crate::NodeLabel) -> bool {
    label.attachment.as_deref() == Some("node")
        && cdxml_imported_label_alignment_is_horizontal_only(label)
        && label.meta.pointer("/import/cdxml/boundingBox").is_some()
        && label.meta.pointer("/import/cdxml/textPosition").is_some()
}

fn imported_cdxml_single_character_label_geometry_is_authoritative(
    label: &crate::NodeLabel,
    text: &str,
) -> bool {
    label.attachment.as_deref() == Some("node")
        && label.meta.pointer("/import/cdxml/boundingBox").is_some()
        && label.meta.pointer("/import/cdxml/textPosition").is_some()
        && !text.contains('\n')
        && text.chars().count() == 1
}

fn cdxml_imported_label_flow_override(label: &crate::NodeLabel) -> Option<LabelFlow> {
    match label
        .meta
        .pointer("/import/cdxml/labelAlignment")
        .and_then(serde_json::Value::as_str)
    {
        Some("Above") => Some(LabelFlow::StackAbove),
        Some("Below") => Some(LabelFlow::StackBelow),
        _ => None,
    }
}

fn refreshed_authoritative_imported_label_display(
    label: &crate::NodeLabel,
    source_text: &str,
    source_runs: &[LabelRun],
    decision: &crate::LabelLayoutDecision,
) -> crate::NodeLabel {
    let mut next_label = label.clone();
    if source_text.trim().is_empty() {
        return next_label;
    }

    let layout = layout_label_text(source_text, &decision);
    let font_family = label
        .font_family
        .clone()
        .unwrap_or_else(|| DEFAULT_TEXT_FONT_FAMILY.to_string());
    let font_size = WorldPt(label.font_size.unwrap_or(DEFAULT_TEXT_FONT_SIZE)).value();
    let fill = label
        .fill
        .clone()
        .unwrap_or_else(|| DEFAULT_TEXT_FILL.to_string());
    let display_runs = display_runs_from_source_runs(source_runs, &font_family, font_size, &fill);
    let (lines, line_runs) = layout_display_runs(&display_runs, &decision);

    next_label.text = layout.rendered_text;
    next_label.runs = if line_runs.len() == 1 {
        line_runs.first().cloned().unwrap_or_default()
    } else {
        Vec::new()
    };
    next_label.line_runs = if line_runs.len() > 1 {
        line_runs.clone()
    } else {
        Vec::new()
    };
    next_label.lines = if lines.len() > 1 { lines } else { Vec::new() };

    if let Some(bbox) = label.bbox() {
        let baseline_y = label
            .position
            .map(|position| position[1])
            .unwrap_or_else(|| round2(bbox[1] + font_size * 0.82));
        next_label.glyph_polygons = build_label_glyph_polygons(
            if line_runs.len() == 1 {
                line_runs.first().map(Vec::as_slice).unwrap_or(&[])
            } else {
                &[]
            },
            if line_runs.len() > 1 { &line_runs } else { &[] },
            [round2(bbox[0]), baseline_y],
            Some(bbox),
            font_size,
        );
    }

    next_label
}

pub(super) fn refreshed_attached_node_label(
    fragment: &crate::MoleculeFragment,
    node_id: &str,
    object_translate: [f64; 2],
    stroke_width: f64,
) -> Option<crate::NodeLabel> {
    let node = fragment.nodes.iter().find(|node| node.id == node_id)?;
    let label = node.label.as_ref()?;
    let source_runs = source_runs_from_node_label(label);
    let source_text = label_source_text(label);
    let connection_angles = adjacent_angles_for_fragment_node(fragment, node_id);
    let world_anchor =
        attached_node_label_anchor_world(fragment, node_id, object_translate, stroke_width);
    let local_anchor = [
        round2(world_anchor.x - object_translate[0]),
        round2(world_anchor.y - object_translate[1]),
    ];
    if is_generated_centered_label(label) {
        return Some(make_centered_node_label(&label.text, local_anchor));
    }
    let text = if implicit_hydrogen_label_is_user_edited(label) {
        source_text.clone()
    } else {
        implicit_hydrogen_label_text(node, &source_text)
    };
    let should_use_internal_whole_label_layout =
        label_should_render_as_whole_group(&text, connection_angles.len());
    if !is_attached_node_label(label)
        && !is_cdxml_imported_right_aligned_attached_label(label)
        && !is_cdxml_imported_single_character_centered_label(label)
        && !(is_cdxml_imported_centered_attached_label(label)
            && should_use_internal_whole_label_layout)
    {
        return None;
    }
    let font_family = label
        .font_family
        .clone()
        .unwrap_or_else(|| DEFAULT_TEXT_FONT_FAMILY.to_string());
    let font_size = WorldPt(label.font_size.unwrap_or(DEFAULT_TEXT_FONT_SIZE)).value();
    let fill = label
        .fill
        .clone()
        .unwrap_or_else(|| DEFAULT_TEXT_FILL.to_string());
    let source_runs = source_runs_for_attached_label(node, source_runs, &text, label);
    let mut decision = label_layout_decision_for_text_mode(
        &text,
        &connection_angles,
        source_runs_are_chemical(&source_runs),
    );
    if let Some(flow) = cdxml_imported_label_flow_override(label) {
        decision.flow = flow;
    }
    if imported_cdxml_single_character_label_geometry_is_authoritative(label, &text) {
        return Some(label.clone());
    }
    if imported_cdxml_label_geometry_is_authoritative(label) {
        if matches!(decision.flow, LabelFlow::Reverse)
            && is_cdxml_imported_right_aligned_attached_label(label)
            && !should_use_internal_whole_label_layout
        {
            return Some(refreshed_authoritative_imported_label_display(
                label,
                &text,
                &source_runs,
                &decision,
            ));
        }
        if !matches!(decision.flow, LabelFlow::StackAbove | LabelFlow::StackBelow)
            && !should_use_internal_whole_label_layout
        {
            return Some(label.clone());
        }
    }
    let display_runs = display_runs_from_source_runs(&source_runs, &font_family, font_size, &fill);
    let (anchor_offset, box_value) =
        current_node_label_editor_geometry(node, object_translate, &connection_angles);
    let session = TextEditSession {
        target: TextEditTarget::EndpointLabel {
            node_id: node_id.to_string(),
            x: world_anchor.x,
            y: world_anchor.y,
        },
        text: text.clone(),
        source_runs: source_runs.clone(),
        font_family: Some(font_family.clone()),
        font_size: Some(font_size),
        fill: Some(fill.clone()),
        align: Some("left".to_string()),
        line_height: Some((font_size * 1.05).max(font_size)),
        box_value,
        anchor_offset,
        preserve_lines: true,
        default_chemical: source_runs
            .iter()
            .any(|run| run.script.as_deref() == Some("chemical")),
    };
    let mut next_label = make_centered_node_label_from_runs(
        &text,
        local_anchor,
        source_runs,
        display_runs,
        &font_family,
        font_size,
        &fill,
        &connection_angles,
        &session,
        false,
        false,
        cdxml_imported_label_flow_override(label),
    );
    if let Some(import_meta) = label.meta.get("import").cloned() {
        set_meta_object_field(&mut next_label.meta, "import", Some(import_meta));
    }
    let recognition_meta = label
        .meta
        .get("labelRecognition")
        .cloned()
        .or_else(|| label_recognition_meta_from_node(node));
    set_label_recognition_meta(&mut next_label, recognition_meta);
    set_label_implicit_hydrogen_label_meta(
        &mut next_label,
        implicit_hydrogen_label_meta(label).cloned(),
    );
    Some(next_label)
}

pub(super) fn refresh_implicit_hydrogens(fragment: &mut crate::MoleculeFragment) {
    let next_counts: Vec<(String, u8)> = fragment
        .nodes
        .iter()
        .map(|node| {
            (
                node.id.clone(),
                implicit_hydrogen_count(fragment, node.id.as_str()),
            )
        })
        .collect();
    for (node_id, num_hydrogens) in next_counts {
        if let Some(node) = fragment.nodes.iter_mut().find(|node| node.id == node_id) {
            node.num_hydrogens = num_hydrogens;
        }
    }
}

pub(super) fn implicit_hydrogen_count(fragment: &crate::MoleculeFragment, node_id: &str) -> u8 {
    let Some(node) = fragment.nodes.iter().find(|node| node.id == node_id) else {
        return 0;
    };
    if node.is_placeholder || node.atomic_number == 1 || node.atomic_number == 6 {
        return 0;
    }
    let connection_count: i32 = fragment
        .bonds
        .iter()
        .filter(|bond| bond.begin == node_id || bond.end == node_id)
        .map(|bond| i32::from(bond.order.max(1)))
        .sum();
    let radical_count = 0;
    let charge = node.charge;
    let abs_charge = charge.abs();
    let Some(valence) = typical_valence_for_implicit_hydrogen(
        node.atomic_number,
        charge,
        connection_count,
        radical_count,
        abs_charge,
    ) else {
        return 0;
    };
    let charge_hydrogen_penalty = implicit_hydrogen_charge_penalty(node.atomic_number, charge);
    (valence - radical_count - connection_count - charge_hydrogen_penalty).clamp(0, 9) as u8
}

pub(crate) fn formula_hydrogen_count_for_node(
    fragment: &crate::MoleculeFragment,
    node_id: &str,
) -> u8 {
    let Some(node) = fragment.nodes.iter().find(|node| node.id == node_id) else {
        return 0;
    };
    if node.is_placeholder || node.atomic_number == 1 {
        return 0;
    }
    if let Some(value) = crate::node_effective_num_hydrogens_override(node) {
        return value;
    }
    if node.atomic_number != 6 {
        return implicit_hydrogen_count(fragment, node_id);
    }
    let connection_order: i32 = fragment
        .bonds
        .iter()
        .filter(|bond| bond.begin == node_id || bond.end == node_id)
        .map(|bond| i32::from(bond.order.max(1)))
        .sum();
    (4 - connection_order - node.charge.abs()).clamp(0, 4) as u8
}

pub(super) fn typical_valence_for_implicit_hydrogen(
    atomic_number: u8,
    charge: i32,
    connection_count: i32,
    radical_count: i32,
    abs_charge: i32,
) -> Option<i32> {
    if let Some(target_valence) = third_period_main_group_target_valence(
        atomic_number,
        connection_count + radical_count + abs_charge,
    ) {
        return Some(target_valence);
    }

    match atomic_number {
        5 => Some(if charge == -1 { 4 } else { 3 }),
        7 => {
            if charge == 1 {
                Some(4)
            } else if charge < 0 {
                Some(3)
            } else if charge == 2 || radical_count + connection_count + abs_charge <= 3 {
                Some(3)
            } else {
                Some(5)
            }
        }
        8 => Some(if charge >= 1 { 3 } else { 2 }),
        9 => Some(1),
        _ => None,
    }
}

pub(super) fn implicit_hydrogen_label_text(node: &crate::Node, current_text: &str) -> String {
    if node.is_placeholder || node.atomic_number == 1 {
        return current_text.to_string();
    }
    if !label_text_matches_node_element(current_text, node) {
        return current_text.to_string();
    }
    implicit_hydrogen_label_text_for_count(&node.element, node.num_hydrogens)
}

pub(crate) fn implicit_hydrogen_label_text_for_count(element: &str, num_hydrogens: u8) -> String {
    if num_hydrogens == 0 {
        return element.to_string();
    }
    if num_hydrogens == 1 {
        format!("{element}H")
    } else {
        format!("{element}H{num_hydrogens}")
    }
}

pub(super) fn label_text_matches_node_element(text: &str, node: &crate::Node) -> bool {
    let trimmed = text.trim();
    if trimmed == node.element {
        return true;
    }
    parse_element_hydrogen_label(trimmed).is_some_and(|parsed| parsed.element == node.element)
}

pub(super) fn source_runs_for_attached_label(
    node: &crate::Node,
    source_runs: Vec<LabelRun>,
    text: &str,
    label: &crate::NodeLabel,
) -> Vec<LabelRun> {
    if node.is_placeholder || !label_text_matches_node_element(text, node) {
        return source_runs;
    }
    let template = source_runs
        .first()
        .cloned()
        .or_else(|| label.runs.first().cloned())
        .unwrap_or(LabelRun {
            text: String::new(),
            font_family: label.font_family.clone(),
            font_size: label.font_size,
            fill: label.fill.clone(),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            script: Some("chemical".to_string()),
        });
    vec![LabelRun {
        text: text.to_string(),
        font_family: template.font_family.or_else(|| label.font_family.clone()),
        font_size: template.font_size.or(label.font_size),
        fill: template.fill.or_else(|| label.fill.clone()),
        font_weight: template.font_weight.or(Some(400)),
        font_style: template.font_style.or_else(|| Some("normal".to_string())),
        underline: template.underline.or(Some(false)),
        script: Some("chemical".to_string()),
    }]
}

pub(super) fn source_runs_from_node_label(label: &crate::NodeLabel) -> Vec<LabelRun> {
    label
        .meta
        .get("sourceRuns")
        .cloned()
        .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
        .or_else(|| (!label.runs.is_empty()).then(|| label.runs.clone()))
        .unwrap_or_else(|| {
            let text = label
                .source_text
                .clone()
                .unwrap_or_else(|| label.text.clone());
            if text.is_empty() {
                Vec::new()
            } else {
                vec![LabelRun {
                    text,
                    font_family: label.font_family.clone(),
                    font_size: label.font_size,
                    fill: label.fill.clone(),
                    font_weight: Some(400),
                    font_style: Some("normal".to_string()),
                    underline: Some(false),
                    script: Some("normal".to_string()),
                }]
            }
        })
}
