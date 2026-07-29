use crate::direction_from_angle;
use serde::{Deserialize, Serialize};

const SINGLE_CONNECTION_HORIZONTAL_EPSILON: f64 = 1.0e-6;
const MULTI_CONNECTION_GAP_TIE_EPSILON_DEG: f64 = 0.001;
// ChemDraw keeps a nearly trigonal three-connection center in the degenerate
// 120-degree branch until an angular gap departs from 120 degrees by about
// 3 degrees. Silent SVG probes place the transition between 2.95 and 3.05
// degrees for 8/10/14 pt labels and 10/14.35/24 pt bonds. Authored coordinate
// quantization decides samples that land exactly on the boundary.
const NEAR_TRIGONAL_GAP_DEVIATION_DEG: f64 = 3.0;
const MULTI_CONNECTION_BISECTOR_RIGHT_END_DEG: f64 = 67.5;
const MULTI_CONNECTION_BISECTOR_BELOW_END_DEG: f64 = 112.5;
const MULTI_CONNECTION_BISECTOR_LEFT_END_DEG: f64 = 247.5;
const MULTI_CONNECTION_BISECTOR_ABOVE_END_DEG: f64 = 292.5;
const OPPOSITE_CONNECTION_HORIZONTAL_END_DEG: f64 = 22.5;
const OPPOSITE_CONNECTION_FORWARD_END_DEG: f64 = 90.0;
const OPPOSITE_CONNECTION_REVERSE_END_DEG: f64 = 157.5;

fn normalize_degrees(angle: f64) -> f64 {
    angle.rem_euclid(360.0)
}

fn decision_for_flow(flow: LabelFlow) -> LabelLayoutDecision {
    let anchor = match flow {
        LabelFlow::Reverse => LabelAnchorPolicy::OriginalFirstGroup,
        LabelFlow::StackAbove | LabelFlow::StackBelow => LabelAnchorPolicy::FirstGroupLeadGlyph,
        LabelFlow::Forward | LabelFlow::Preserve => LabelAnchorPolicy::FirstGlyph,
    };
    LabelLayoutDecision { flow, anchor }
}

fn classify_multi_connection_bisector(bisector: f64) -> LabelFlow {
    if bisector <= MULTI_CONNECTION_BISECTOR_RIGHT_END_DEG
        || bisector >= MULTI_CONNECTION_BISECTOR_ABOVE_END_DEG
    {
        LabelFlow::Reverse
    } else if bisector < MULTI_CONNECTION_BISECTOR_BELOW_END_DEG {
        LabelFlow::StackAbove
    } else if bisector <= MULTI_CONNECTION_BISECTOR_LEFT_END_DEG {
        LabelFlow::Forward
    } else {
        LabelFlow::StackBelow
    }
}

fn multi_connection_layout(connection_angles: &[f64]) -> LabelLayoutDecision {
    debug_assert!(connection_angles.len() >= 2);
    let mut angles: Vec<f64> = connection_angles
        .iter()
        .map(|angle| normalize_degrees(*angle))
        .collect();
    angles.sort_by(f64::total_cmp);

    let gaps: Vec<(usize, f64)> = angles
        .iter()
        .enumerate()
        .map(|(index, angle)| {
            let next = if index + 1 == angles.len() {
                angles[0] + 360.0
            } else {
                angles[index + 1]
            };
            (index, next - angle)
        })
        .collect();
    let largest_gap = gaps
        .iter()
        .map(|(_, gap)| *gap)
        .max_by(f64::total_cmp)
        .expect("multi-connection layout requires at least one angular gap");
    let tied_gaps: Vec<(usize, f64)> = gaps
        .iter()
        .copied()
        .filter(|(_, gap)| (largest_gap - gap).abs() <= MULTI_CONNECTION_GAP_TIE_EPSILON_DEG)
        .collect();

    if angles.len() == 2
        && tied_gaps.len() == 2
        && (largest_gap - 180.0).abs() <= MULTI_CONNECTION_GAP_TIE_EPSILON_DEG
    {
        let axis = angles[0].rem_euclid(180.0);
        let flow = if axis
            <= OPPOSITE_CONNECTION_HORIZONTAL_END_DEG + MULTI_CONNECTION_GAP_TIE_EPSILON_DEG
            || axis >= OPPOSITE_CONNECTION_REVERSE_END_DEG - MULTI_CONNECTION_GAP_TIE_EPSILON_DEG
        {
            LabelFlow::StackAbove
        } else if axis <= OPPOSITE_CONNECTION_FORWARD_END_DEG + MULTI_CONNECTION_GAP_TIE_EPSILON_DEG
        {
            LabelFlow::Forward
        } else {
            LabelFlow::Reverse
        };
        return decision_for_flow(flow);
    }

    // Three equal or nearly equal sectors have no stable unique opening.
    // ChemDraw fits their common 120-degree phase before applying the phase
    // sectors, and only switches to the largest-gap rule outside this window.
    let near_trigonal = angles.len() == 3
        && gaps.iter().all(|(_, gap)| {
            (*gap - 120.0).abs()
                <= NEAR_TRIGONAL_GAP_DEVIATION_DEG + MULTI_CONNECTION_GAP_TIE_EPSILON_DEG
        });
    if near_trigonal {
        let phase = normalize_degrees(
            angles
                .iter()
                .enumerate()
                .map(|(index, angle)| angle - index as f64 * 120.0)
                .sum::<f64>()
                / 3.0,
        )
        .rem_euclid(120.0);
        let flow = if phase <= 60.0 || phase >= 112.5 {
            LabelFlow::Forward
        } else if phase <= 67.5 {
            LabelFlow::Reverse
        } else {
            LabelFlow::StackAbove
        };
        return decision_for_flow(flow);
    }

    let selected_gap = tied_gaps
        .iter()
        .copied()
        .map(|(index, gap)| {
            let midpoint = normalize_degrees(angles[index] + gap * 0.5);
            let clockwise_from_up = normalize_degrees(midpoint - 270.0);
            let distance_from_up = clockwise_from_up.min(360.0 - clockwise_from_up);
            let right_axis_distance = midpoint.min(360.0 - midpoint);
            (
                index,
                gap,
                right_axis_distance > MULTI_CONNECTION_GAP_TIE_EPSILON_DEG,
                distance_from_up,
                clockwise_from_up,
            )
        })
        .min_by(|left, right| {
            left.2
                .cmp(&right.2)
                .then_with(|| left.3.total_cmp(&right.3))
                .then_with(|| left.4.total_cmp(&right.4))
        })
        .expect("multi-connection layout requires a selected angular gap");
    let occupied_start = if selected_gap.0 + 1 == angles.len() {
        angles[0]
    } else {
        angles[selected_gap.0 + 1]
    };
    let occupied_span = 360.0 - selected_gap.1;
    let bisector = normalize_degrees(occupied_start + occupied_span * 0.5);
    decision_for_flow(classify_multi_connection_bisector(bisector))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LabelFlow {
    Forward,
    Reverse,
    /// Keep the authored text and line breaks exactly as stored.
    Preserve,
    StackAbove,
    StackBelow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LabelAnchorPolicy {
    FirstGlyph,
    LastGlyph,
    /// A fixed, authored left edge. Explicit subscripts may be the attachment
    /// glyph, while superscripts remain decorations outside the bond axis.
    AuthoredFirstGlyph,
    /// A fixed, authored right edge. Explicit subscripts may be the attachment
    /// glyph, while trailing superscripts remain decorations.
    AuthoredLastGlyph,
    OriginalFirstGroup,
    FirstGroupLeadGlyph,
    WholeLabel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelLayoutDecision {
    pub flow: LabelFlow,
    pub anchor: LabelAnchorPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelLayout {
    pub flow: LabelFlow,
    pub anchor: LabelAnchorPolicy,
    pub lines: Vec<String>,
    pub rendered_text: String,
    pub anchor_line: usize,
    pub anchor_char: usize,
}

pub fn compact_label_text(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

pub fn split_label_groups(text: &str) -> Vec<String> {
    // Labels are mirrored as chemistry groups, not as raw characters. Known
    // abbreviations such as TMS must stay atomic when OTMS flips to TMSO.
    let compact = compact_label_text(text);
    split_compact_label_groups(&compact)
}

fn split_compact_label_groups(compact: &str) -> Vec<String> {
    if compact.is_empty() {
        return Vec::new();
    }
    let mut groups = Vec::new();
    let mut current = String::new();
    let mut index = 0usize;
    while index < compact.len() {
        let rest = &compact[index..];
        if rest.starts_with('(') {
            if let Some(group_len) = parenthesized_label_group_len(rest) {
                if !current.is_empty() {
                    groups.push(std::mem::take(&mut current));
                }
                groups.push(rest[..group_len].to_string());
                index += group_len;
                continue;
            }
        }
        if let Some(prefix_len) = crate::label_group_prefix_len(rest) {
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
            groups.push(rest[..prefix_len].to_string());
            index += prefix_len;
            continue;
        }
        let Some(character) = rest.chars().next() else {
            break;
        };
        if character.is_ascii_uppercase() && !current.is_empty() {
            groups.push(std::mem::take(&mut current));
        }
        current.push(character);
        index += character.len_utf8();
    }
    if !current.is_empty() {
        groups.push(current);
    }
    groups
}

pub fn reverse_label_groups(text: &str) -> String {
    let mut groups = split_label_groups(text)
        .into_iter()
        .map(|group| reverse_label_group_for_display(&group))
        .collect::<Vec<_>>();
    groups.reverse();
    groups.concat()
}

pub fn label_text_uses_whole_label_layout(text: &str, connection_count: usize) -> bool {
    let compact = compact_label_text(text);
    crate::recognized_abbreviation_uses_whole_label_layout(&compact, connection_count)
        || hyphenated_label_token_uses_whole_label_layout(&compact)
        || bracketed_query_label_uses_whole_label_layout(&compact)
}

fn bracketed_query_label_uses_whole_label_layout(text: &str) -> bool {
    text.len() >= 2 && text.starts_with('[') && text.ends_with(']')
}

fn hyphenated_label_token_uses_whole_label_layout(text: &str) -> bool {
    if text.is_empty() || text.starts_with('-') || text.ends_with('-') {
        return false;
    }
    let mut hyphen_count = 0usize;
    let mut has_left_letter = false;
    let mut has_right_letter = false;
    let mut seen_hyphen = false;
    let starts_with_digit = text
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit());
    for character in text.chars() {
        match character {
            '-' => {
                hyphen_count += 1;
                seen_hyphen = true;
            }
            _ if character.is_ascii_alphanumeric() => {
                if character.is_ascii_alphabetic() {
                    if seen_hyphen {
                        has_right_letter = true;
                    } else {
                        has_left_letter = true;
                    }
                }
            }
            _ => return false,
        }
    }
    hyphen_count == 1 && has_right_letter && (has_left_letter || starts_with_digit)
}

fn reverse_label_group_for_display(group: &str) -> String {
    let Some((inner, suffix)) = parenthesized_label_group_parts(group) else {
        return group.to_string();
    };
    format!("({}){suffix}", reverse_label_groups(inner))
}

fn parenthesized_label_group_len(text: &str) -> Option<usize> {
    let close = matching_close_paren(text)?;
    let after_close = close + 1;
    let suffix_len = text[after_close..]
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .map(char::len_utf8)
        .sum::<usize>();
    Some(after_close + suffix_len)
}

fn parenthesized_label_group_parts(group: &str) -> Option<(&str, &str)> {
    if !group.starts_with('(') {
        return None;
    }
    let close = matching_close_paren(group)?;
    let suffix = &group[close + 1..];
    if !suffix.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    Some((&group[1..close], suffix))
}

fn matching_close_paren(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, character) in text.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

pub fn terminal_letter_anchor_offset(group: &str) -> usize {
    // Anchor the bond to the terminal letter in a group and ignore digits or
    // generated hydrogens that are visible text but not connection points.
    // Prime marks in generic labels such as R' are part of the connection
    // label identity in ChemDraw and should move the anchor to the visible
    // suffix rather than leaving it at the R glyph center.
    group
        .chars()
        .enumerate()
        .filter_map(|(index, character)| {
            (character.is_ascii_alphabetic() || is_prime_anchor_suffix(character)).then_some(index)
        })
        .last()
        .unwrap_or(0)
}

pub fn is_prime_anchor_suffix(character: char) -> bool {
    matches!(
        character,
        '\'' | '\u{2019}' | '\u{2032}' | '\u{2033}' | '\u{2034}'
    )
}

pub fn decide_label_layout(
    connection_angles: &[f64],
    forward_collides: bool,
    reverse_collides: bool,
) -> LabelLayoutDecision {
    if connection_angles.is_empty() {
        return LabelLayoutDecision {
            flow: LabelFlow::Forward,
            anchor: LabelAnchorPolicy::FirstGlyph,
        };
    }

    if connection_angles.len() == 1 {
        let direction = direction_from_angle(connection_angles[0]);
        // A terminal label follows the complete left/right half-plane and
        // only delegates an effectively vertical bond to the collision
        // resolver. Multi-connection labels use the separate open-sector
        // decision below.
        if direction.x > SINGLE_CONNECTION_HORIZONTAL_EPSILON {
            return LabelLayoutDecision {
                flow: LabelFlow::Reverse,
                anchor: LabelAnchorPolicy::FirstGlyph,
            };
        }
        if direction.x < -SINGLE_CONNECTION_HORIZONTAL_EPSILON {
            return LabelLayoutDecision {
                flow: LabelFlow::Forward,
                anchor: LabelAnchorPolicy::FirstGlyph,
            };
        }
        if forward_collides && !reverse_collides {
            return LabelLayoutDecision {
                flow: LabelFlow::Reverse,
                anchor: LabelAnchorPolicy::FirstGlyph,
            };
        }
        return LabelLayoutDecision {
            flow: LabelFlow::Forward,
            anchor: LabelAnchorPolicy::FirstGlyph,
        };
    }

    multi_connection_layout(connection_angles)
}

pub fn layout_label_text(text: &str, decision: &LabelLayoutDecision) -> LabelLayout {
    let groups = if decision.anchor == LabelAnchorPolicy::WholeLabel {
        let compact = compact_label_text(text);
        if compact.is_empty() {
            Vec::new()
        } else {
            vec![compact]
        }
    } else {
        split_label_groups(text)
    };
    if groups.is_empty() {
        return LabelLayout {
            flow: decision.flow.clone(),
            anchor: decision.anchor.clone(),
            lines: Vec::new(),
            rendered_text: String::new(),
            anchor_line: 0,
            anchor_char: 0,
        };
    }

    match decision.flow {
        LabelFlow::Forward => {
            let rendered_text = groups.concat();
            let anchor_char = if matches!(
                decision.anchor,
                LabelAnchorPolicy::LastGlyph | LabelAnchorPolicy::AuthoredLastGlyph
            ) {
                rendered_text.chars().count().saturating_sub(1)
            } else {
                0
            };
            LabelLayout {
                flow: decision.flow.clone(),
                anchor: decision.anchor.clone(),
                lines: vec![rendered_text.clone()],
                rendered_text,
                anchor_line: 0,
                anchor_char,
            }
        }
        LabelFlow::Reverse => {
            let rendered_groups = groups
                .iter()
                .rev()
                .map(|group| reverse_label_group_for_display(group))
                .collect::<Vec<_>>();
            let rendered_text = rendered_groups.concat();
            let anchor_char = match decision.anchor {
                LabelAnchorPolicy::WholeLabel => rendered_text.chars().count().saturating_sub(1),
                LabelAnchorPolicy::OriginalFirstGroup => {
                    let original_first_group = groups.first().map(String::as_str).unwrap_or("");
                    let original_first_group_start = rendered_groups
                        .iter()
                        .take(rendered_groups.len().saturating_sub(1))
                        .map(|group| group.chars().count())
                        .sum::<usize>();
                    original_first_group_start + terminal_letter_anchor_offset(original_first_group)
                }
                _ => 0,
            };
            LabelLayout {
                flow: decision.flow.clone(),
                anchor: decision.anchor.clone(),
                lines: vec![rendered_text.clone()],
                rendered_text,
                anchor_line: 0,
                anchor_char,
            }
        }
        LabelFlow::Preserve => {
            let lines = text
                .split('\n')
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            let anchor_line = match decision.anchor {
                LabelAnchorPolicy::LastGlyph | LabelAnchorPolicy::AuthoredLastGlyph => {
                    lines.len().saturating_sub(1)
                }
                _ => 0,
            };
            let anchor_char = match decision.anchor {
                LabelAnchorPolicy::LastGlyph | LabelAnchorPolicy::AuthoredLastGlyph => lines
                    .get(anchor_line)
                    .map(|line| line.chars().count().saturating_sub(1))
                    .unwrap_or(0),
                _ => 0,
            };
            LabelLayout {
                flow: decision.flow.clone(),
                anchor: decision.anchor.clone(),
                rendered_text: lines.join("\n"),
                lines,
                anchor_line,
                anchor_char,
            }
        }
        LabelFlow::StackAbove => stacked_layout(
            decision,
            if groups.len() > 1 {
                vec![groups[1..].concat(), groups[0].clone()]
            } else {
                vec![groups[0].clone()]
            },
            if groups.len() > 1 { 1 } else { 0 },
        ),
        LabelFlow::StackBelow => stacked_layout(
            decision,
            if groups.len() > 1 {
                vec![groups[0].clone(), groups[1..].concat()]
            } else {
                vec![groups[0].clone()]
            },
            0,
        ),
    }
}

fn stacked_layout(
    decision: &LabelLayoutDecision,
    lines: Vec<String>,
    anchor_line: usize,
) -> LabelLayout {
    let rendered_text = lines.join("\n");
    let anchor_char = 0;
    LabelLayout {
        flow: decision.flow.clone(),
        anchor: decision.anchor.clone(),
        lines,
        rendered_text,
        anchor_line,
        anchor_char,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flow_marker(flow: LabelFlow) -> u8 {
        match flow {
            LabelFlow::Reverse => b'R',
            LabelFlow::Forward => b'F',
            LabelFlow::StackAbove => b'A',
            LabelFlow::StackBelow => b'B',
            LabelFlow::Preserve => b'P',
        }
    }

    #[test]
    fn splits_formula_text_into_uppercase_led_groups() {
        assert_eq!(split_label_groups("CuF3"), vec!["Cu", "F3"]);
        assert_eq!(split_label_groups("CuF3Ph2"), vec!["Cu", "F3", "Ph2"]);
        assert_eq!(split_label_groups("OTMS"), vec!["O", "TMS"]);
        assert_eq!(split_label_groups("OTBDMS"), vec!["O", "TBDMS"]);
        assert_eq!(split_label_groups("OTFA"), vec!["O", "TFA"]);
        assert_eq!(split_label_groups("OTAA"), vec!["O", "T", "A", "A"]);
        assert_eq!(split_label_groups("OXYZ"), vec!["O", "X", "Y", "Z"]);
        assert_eq!(split_label_groups("OFMOC"), vec!["O", "FMOC"]);
        assert_eq!(split_label_groups("OCH3"), vec!["O", "CH3"]);
        assert_eq!(split_label_groups("TMSOPh"), vec!["TMS", "O", "Ph"]);
        assert_eq!(split_label_groups("CF3"), vec!["C", "F3"]);
        assert_eq!(split_label_groups("N(PhSO2)2"), vec!["N", "(PhSO2)2"]);
        assert_eq!(split_label_groups("C10H21"), vec!["C10H21"]);
        assert_eq!(split_label_groups("C10H21O3"), vec!["C10H21", "O3"]);
    }

    #[test]
    fn bracketed_query_labels_stay_whole_during_connection_layout() {
        assert!(label_text_uses_whole_label_layout("[C,N,P]", 2));
        assert!(!label_text_uses_whole_label_layout("C,N,P", 2));
    }

    #[test]
    fn reverses_formula_by_letter_groups() {
        assert_eq!(reverse_label_groups("CuF3"), "F3Cu");
        assert_eq!(reverse_label_groups("CuF3Ph2"), "Ph2F3Cu");
        assert_eq!(reverse_label_groups("OTMS"), "TMSO");
        assert_eq!(reverse_label_groups("OTBDMS"), "TBDMSO");
        assert_eq!(reverse_label_groups("OTFA"), "TFAO");
        assert_eq!(reverse_label_groups("OTAA"), "AATO");
        assert_eq!(reverse_label_groups("OXYZ"), "ZYXO");
        assert_eq!(reverse_label_groups("OFMOC"), "FMOCO");
        assert_eq!(reverse_label_groups("OCH3"), "CH3O");
        assert_eq!(reverse_label_groups("TMSOPh"), "PhOTMS");
        assert_eq!(reverse_label_groups("CF3"), "F3C");
        assert_eq!(reverse_label_groups("N(PhSO2)2"), "(O2SPh)2N");
        assert_eq!(reverse_label_groups("C10H21"), "C10H21");
        assert_eq!(reverse_label_groups("C10H21O3"), "O3C10H21");
    }

    #[test]
    fn hyphenated_label_tokens_use_whole_label_layout() {
        assert!(label_text_uses_whole_label_layout("2-Np", 1));
        assert!(label_text_uses_whole_label_layout("t-Bu", 1));
        assert!(label_text_uses_whole_label_layout("n-Bu", 1));
        assert!(label_text_uses_whole_label_layout("p-Tol", 1));
        assert!(!label_text_uses_whole_label_layout("CF3", 1));
        assert!(!label_text_uses_whole_label_layout("Cl-", 1));
        assert!(!label_text_uses_whole_label_layout("SO3-", 1));
    }

    #[test]
    fn terminal_letter_anchor_offset_skips_trailing_digits() {
        assert_eq!(terminal_letter_anchor_offset("Ph"), 1);
        assert_eq!(terminal_letter_anchor_offset("Ph2"), 1);
        assert_eq!(terminal_letter_anchor_offset("N3"), 0);
        assert_eq!(terminal_letter_anchor_offset("R'"), 1);
        assert_eq!(terminal_letter_anchor_offset("R\u{2032}"), 1);
    }

    #[test]
    fn whole_label_reverse_keeps_text_and_anchors_rightmost_glyph() {
        let decision = LabelLayoutDecision {
            flow: LabelFlow::Reverse,
            anchor: LabelAnchorPolicy::WholeLabel,
        };
        let layout = layout_label_text("t-Bu", &decision);
        assert_eq!(layout.lines, vec!["t-Bu"]);
        assert_eq!(layout.anchor_char, 3);
    }

    #[test]
    fn keeps_multi_bond_left_labels_forward() {
        let decision = decide_label_layout(&[180.0, 225.0], false, false);
        assert_eq!(decision.flow, LabelFlow::Forward);
        assert_eq!(decision.anchor, LabelAnchorPolicy::FirstGlyph);
    }

    #[test]
    fn reverses_multi_bond_right_labels_but_keeps_original_anchor_group() {
        let decision = decide_label_layout(&[0.0, 315.0], false, false);
        assert_eq!(decision.flow, LabelFlow::Reverse);
        assert_eq!(decision.anchor, LabelAnchorPolicy::OriginalFirstGroup);

        let layout = layout_label_text("CuF3Ph2", &decision);
        assert_eq!(layout.lines, vec!["Ph2F3Cu"]);
        assert_eq!(layout.anchor_line, 0);
        assert_eq!(layout.anchor_char, 6);

        let parenthesized = layout_label_text("N(PhSO2)2", &decision);
        assert_eq!(parenthesized.lines, vec!["(O2SPh)2N"]);
        assert_eq!(parenthesized.anchor_line, 0);
        assert_eq!(parenthesized.anchor_char, 8);
    }

    #[test]
    fn reversed_single_group_anchors_terminal_letter_not_digit() {
        let decision = LabelLayoutDecision {
            flow: LabelFlow::Reverse,
            anchor: LabelAnchorPolicy::OriginalFirstGroup,
        };

        let ph = layout_label_text("Ph", &decision);
        assert_eq!(ph.lines, vec!["Ph"]);
        assert_eq!(ph.anchor_char, 1);

        let n3 = layout_label_text("N3", &decision);
        assert_eq!(n3.lines, vec!["N3"]);
        assert_eq!(n3.anchor_char, 0);

        let r_prime = layout_label_text("R'", &decision);
        assert_eq!(r_prime.lines, vec!["R'"]);
        assert_eq!(r_prime.anchor_char, 1);
    }

    #[test]
    fn stacks_when_all_connections_are_below() {
        let decision = decide_label_layout(&[90.0, 60.0], false, false);
        assert_eq!(decision.flow, LabelFlow::StackAbove);
        assert_eq!(decision.anchor, LabelAnchorPolicy::FirstGroupLeadGlyph);

        let layout = layout_label_text("CuF3Ph2", &decision);
        assert_eq!(layout.lines, vec!["F3Ph2", "Cu"]);
        assert_eq!(layout.anchor_line, 1);
        assert_eq!(layout.anchor_char, 0);
    }

    #[test]
    fn stacks_when_all_connections_are_above() {
        let decision = decide_label_layout(&[270.0, 300.0], false, false);
        assert_eq!(decision.flow, LabelFlow::StackBelow);
        assert_eq!(decision.anchor, LabelAnchorPolicy::FirstGroupLeadGlyph);

        let layout = layout_label_text("CuF3Ph2", &decision);
        assert_eq!(layout.lines, vec!["Cu", "F3Ph2"]);
        assert_eq!(layout.anchor_line, 0);
        assert_eq!(layout.anchor_char, 0);
    }

    #[test]
    fn reverses_multi_bond_right_labels_with_vertical_connection() {
        let decision = decide_label_layout(&[0.0, 270.0], false, false);
        assert_eq!(decision.flow, LabelFlow::Reverse);
        assert_eq!(decision.anchor, LabelAnchorPolicy::OriginalFirstGroup);
    }

    #[test]
    fn chemdraw_two_connection_flow_switches_at_bisector_sector_boundaries() {
        let horizontal = decide_label_layout(&[120.0, 14.9], false, false);
        assert_eq!(horizontal.flow, LabelFlow::Reverse);
        assert_eq!(horizontal.anchor, LabelAnchorPolicy::OriginalFirstGroup);

        let below = decide_label_layout(&[120.0, 15.1], false, false);
        assert_eq!(below.flow, LabelFlow::StackAbove);
        assert_eq!(below.anchor, LabelAnchorPolicy::FirstGroupLeadGlyph);

        let opposite_horizontal = decide_label_layout(&[300.0, 194.9], false, false);
        assert_eq!(opposite_horizontal.flow, LabelFlow::Forward);

        let above = decide_label_layout(&[300.0, 195.1], false, false);
        assert_eq!(above.flow, LabelFlow::StackBelow);
    }

    #[test]
    fn chemdraw_two_connection_flow_matches_the_full_thirty_degree_grid() {
        let angles = [
            0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0, 210.0, 240.0, 270.0, 300.0, 330.0,
        ];
        let expected = [
            "RRRRRAABRRRR",
            "RRRRAAAFRRRR",
            "RRRAAAFFFRRR",
            "RRAAAFFFFFRR",
            "RAAAFFFFFFRR",
            "AAAFFFFFFFFR",
            "AAFFFFFFFFFB",
            "BFFFFFFFFFBB",
            "RRFFFFFFFBBB",
            "RRRFFFFFBBBR",
            "RRRRRFFBBBRR",
            "RRRRRRBBBRRR",
        ];

        for (fixed_index, fixed_angle) in angles.iter().enumerate() {
            for (angle_index, angle) in angles.iter().enumerate() {
                let actual = decide_label_layout(&[*fixed_angle, *angle], false, false).flow;
                let expected_flow = match expected[fixed_index].as_bytes()[angle_index] {
                    b'R' => LabelFlow::Reverse,
                    b'F' => LabelFlow::Forward,
                    b'A' => LabelFlow::StackAbove,
                    b'B' => LabelFlow::StackBelow,
                    other => panic!("unexpected matrix marker {other}"),
                };
                assert_eq!(
                    actual, expected_flow,
                    "fixed angle {fixed_angle}, variable angle {angle}"
                );
            }
        }
    }

    #[test]
    fn chemdraw_three_connection_flow_matches_the_full_thirty_degree_grid() {
        let angles = [
            0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0, 210.0, 240.0, 270.0, 300.0, 330.0,
        ];
        let expected = concat!(
            "RRRAARRRRRRRAAARRRRRAAARRRRAAAFRRRAABBRRBBBBBBBBBRRRRRRRAAAFRRRRAAAFFRRRAAFFRRRAFFFRRFFBBARRRRRRRRRR",
            "AAFFFRRRAFFFFRRFFFFRRFFFFAFFRRRRRRRRFFFFFRRFFFFRRFFFFAFFFAFFRRRRFFFFFRFFFFAFFFFFFFFRRFFFFFFFFFFFFFFF",
            "FFFBFFBFBBFBBBBBBBBR",
        )
        .as_bytes();
        let mut result_index = 0;
        for first in 0..angles.len() {
            for second in first + 1..angles.len() {
                for third in second + 1..angles.len() {
                    let connection_angles = [angles[first], angles[second], angles[third]];
                    let actual = decide_label_layout(&connection_angles, false, false).flow;
                    assert_eq!(
                        flow_marker(actual),
                        expected[result_index],
                        "angles {connection_angles:?}"
                    );
                    result_index += 1;
                }
            }
        }
        assert_eq!(result_index, expected.len());
    }

    #[test]
    fn nearly_trigonal_connections_use_the_fitted_phase_until_three_degrees() {
        // Selecting the microscopically largest gap would reverse this label;
        // ChemDraw instead keeps the fitted trigonal phase.
        let corpus_geometry = decide_label_layout(&[59.3, 179.25, 299.3], false, false);
        assert_eq!(corpus_geometry.flow, LabelFlow::Forward);

        // A different phase verifies that this is a geometric rule rather
        // than a special case for the public-corpus boron center.
        let inside_window = decide_label_layout(&[30.0, 147.05, 270.0], false, false);
        assert_eq!(inside_window.flow, LabelFlow::Forward);

        // Once a gap is more than three degrees away from 120 degrees, the
        // unique largest open sector becomes authoritative.
        let outside_window = decide_label_layout(&[30.0, 146.9, 270.0], false, false);
        assert_eq!(outside_window.flow, LabelFlow::Reverse);
    }

    #[test]
    fn chemdraw_four_connection_flow_matches_the_full_thirty_degree_grid() {
        let angles = [
            0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0, 210.0, 240.0, 270.0, 300.0, 330.0,
        ];
        let expected = concat!(
            "RRAAARRRRRAAARRRRAAAFRRRAAFRRRABBBARRRRRRRRRRRAAARRRRAAAFRRRAAFRRRAFRRARRRRRRRRRRAAAFRRRAAFRRRAFFRAF",
            "RRARRRRRRAAFRRRAFFRAFFFARRRRRRABBBABBBBBBBBBRBBBBBBBBBBBBBBBBRRRRAAAFFRRRAAFFRRRAFFFRRFFFRAFRRRRRRRR",
            "RAAFFRRRAFFFRRFFFRAFFRARRRRRRAFFFRRFFFRAFFAAFRRRRRFFFRAFFBAFBBRRRFBBBBBBBBBRRRRRRRRRRAFFFFRRFFFFRRFF",
            "FFAFFFAFRRRRRFFFFRRFFFFAFFFAFFRRRRFFFFAFFFAFFAFRRFFFAFFRFRRFRRRRRRRRRFFFFRRFFFFAFFFAFFRRRRFFFFAFFFAF",
            "FFFRRFFFAFFFFFRFFFFFFFRRRFFFFAFFFFFFFFFRFFFFFFFFFFFFFFFFFFFRFFFFFFFFFFFFFFFFFFFFFFBFBBFBBBBBBBB",
        )
        .as_bytes();
        let mut result_index = 0;
        for first in 0..angles.len() {
            for second in first + 1..angles.len() {
                for third in second + 1..angles.len() {
                    for fourth in third + 1..angles.len() {
                        let connection_angles =
                            [angles[first], angles[second], angles[third], angles[fourth]];
                        let actual = decide_label_layout(&connection_angles, false, false).flow;
                        assert_eq!(
                            flow_marker(actual),
                            expected[result_index],
                            "angles {connection_angles:?}"
                        );
                        result_index += 1;
                    }
                }
            }
        }
        assert_eq!(result_index, expected.len());
    }

    #[test]
    fn chemdraw_opposite_connection_axis_uses_its_own_sector_boundaries() {
        for (angles, expected) in [
            ([22.5, 202.5], LabelFlow::StackAbove),
            ([22.6, 202.6], LabelFlow::Forward),
            ([90.0, 270.0], LabelFlow::Forward),
            ([90.1, 270.1], LabelFlow::Reverse),
            ([157.4, 337.4], LabelFlow::Reverse),
            ([157.5, 337.5], LabelFlow::StackAbove),
        ] {
            assert_eq!(
                decide_label_layout(&angles, false, false).flow,
                expected,
                "angles {angles:?}"
            );
        }
    }

    #[test]
    fn single_connection_uses_the_complete_left_and_right_half_planes() {
        assert_eq!(
            decide_label_layout(&[30.0], false, false).flow,
            LabelFlow::Reverse,
        );
        assert_eq!(
            decide_label_layout(&[150.0], false, false).flow,
            LabelFlow::Forward,
        );
    }

    #[test]
    fn single_right_side_connection_prefers_reverse() {
        let decision = decide_label_layout(&[0.0], false, false);
        assert_eq!(decision.flow, LabelFlow::Reverse);
    }

    #[test]
    fn single_vertical_connection_reverses_only_when_forward_collides() {
        let forward = decide_label_layout(&[90.0], false, false);
        assert_eq!(forward.flow, LabelFlow::Forward);

        let reverse = decide_label_layout(&[90.0], true, false);
        assert_eq!(reverse.flow, LabelFlow::Reverse);

        let default_layout = decide_label_layout(&[90.0], true, true);
        assert_eq!(default_layout.flow, LabelFlow::Forward);
    }
}
