use crate::direction_from_angle;
use serde::{Deserialize, Serialize};

const DIRECTION_EPSILON: f64 = 1.0e-6;

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
        if direction.x > DIRECTION_EPSILON {
            return LabelLayoutDecision {
                flow: LabelFlow::Reverse,
                anchor: LabelAnchorPolicy::FirstGlyph,
            };
        }
        if direction.x < -DIRECTION_EPSILON {
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

    let all_left = connection_angles
        .iter()
        .all(|angle| direction_from_angle(*angle).x < -DIRECTION_EPSILON);
    if all_left {
        return LabelLayoutDecision {
            flow: LabelFlow::Forward,
            anchor: LabelAnchorPolicy::FirstGlyph,
        };
    }

    let all_right = connection_angles
        .iter()
        .all(|angle| direction_from_angle(*angle).x > DIRECTION_EPSILON);
    if all_right {
        return LabelLayoutDecision {
            flow: LabelFlow::Reverse,
            anchor: LabelAnchorPolicy::OriginalFirstGroup,
        };
    }

    let all_below = connection_angles
        .iter()
        .all(|angle| direction_from_angle(*angle).y > DIRECTION_EPSILON);
    if all_below {
        return LabelLayoutDecision {
            flow: LabelFlow::StackAbove,
            anchor: LabelAnchorPolicy::FirstGroupLeadGlyph,
        };
    }

    let all_above = connection_angles
        .iter()
        .all(|angle| direction_from_angle(*angle).y < -DIRECTION_EPSILON);
    if all_above {
        return LabelLayoutDecision {
            flow: LabelFlow::StackBelow,
            anchor: LabelAnchorPolicy::FirstGroupLeadGlyph,
        };
    }

    let has_right = connection_angles
        .iter()
        .any(|angle| direction_from_angle(*angle).x > DIRECTION_EPSILON);
    let all_right_or_vertical = connection_angles
        .iter()
        .all(|angle| direction_from_angle(*angle).x >= -DIRECTION_EPSILON);
    if has_right && all_right_or_vertical {
        return LabelLayoutDecision {
            flow: LabelFlow::Reverse,
            anchor: LabelAnchorPolicy::OriginalFirstGroup,
        };
    }

    LabelLayoutDecision {
        flow: LabelFlow::Forward,
        anchor: LabelAnchorPolicy::FirstGlyph,
    }
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
            let anchor_char = if decision.anchor == LabelAnchorPolicy::LastGlyph {
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
                LabelAnchorPolicy::LastGlyph => lines.len().saturating_sub(1),
                _ => 0,
            };
            let anchor_char = match decision.anchor {
                LabelAnchorPolicy::LastGlyph => lines
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
