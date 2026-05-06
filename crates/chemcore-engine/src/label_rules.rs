use crate::direction_from_angle;
use serde::{Deserialize, Serialize};

const DIRECTION_EPSILON: f64 = 1.0e-6;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LabelFlow {
    Forward,
    Reverse,
    StackAbove,
    StackBelow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LabelAnchorPolicy {
    FirstGlyph,
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
    let compact = compact_label_text(text);
    if compact.is_empty() {
        return Vec::new();
    }
    let mut groups = Vec::new();
    let mut current = String::new();
    for character in compact.chars() {
        if character.is_ascii_uppercase() && !current.is_empty() {
            groups.push(std::mem::take(&mut current));
        }
        current.push(character);
    }
    if !current.is_empty() {
        groups.push(current);
    }
    groups
}

pub fn reverse_label_groups(text: &str) -> String {
    let mut groups = split_label_groups(text);
    groups.reverse();
    groups.concat()
}

pub fn terminal_letter_anchor_offset(group: &str) -> usize {
    group.chars().count().saturating_sub(1)
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
            LabelLayout {
                flow: decision.flow.clone(),
                anchor: decision.anchor.clone(),
                lines: vec![rendered_text.clone()],
                rendered_text,
                anchor_line: 0,
                anchor_char: 0,
            }
        }
        LabelFlow::Reverse => {
            let rendered_groups = groups.iter().rev().cloned().collect::<Vec<_>>();
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
        LabelFlow::StackAbove => stacked_layout(
            &groups,
            decision,
            if groups.len() > 1 {
                vec![groups[1..].concat(), groups[0].clone()]
            } else {
                vec![groups[0].clone()]
            },
            if groups.len() > 1 { 1 } else { 0 },
        ),
        LabelFlow::StackBelow => stacked_layout(
            &groups,
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
    groups: &[String],
    decision: &LabelLayoutDecision,
    lines: Vec<String>,
    anchor_line: usize,
) -> LabelLayout {
    let rendered_text = lines.join("\n");
    let anchor_char = if groups.is_empty() { 0 } else { 0 };
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
    }

    #[test]
    fn reverses_formula_by_letter_groups() {
        assert_eq!(reverse_label_groups("CuF3"), "F3Cu");
        assert_eq!(reverse_label_groups("CuF3Ph2"), "Ph2F3Cu");
    }

    #[test]
    fn terminal_letter_anchor_offset_uses_terminal_glyph() {
        assert_eq!(terminal_letter_anchor_offset("Ph"), 1);
        assert_eq!(terminal_letter_anchor_offset("Ph2"), 2);
        assert_eq!(terminal_letter_anchor_offset("N3"), 1);
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
    }

    #[test]
    fn reversed_single_group_anchors_terminal_glyph() {
        let decision = LabelLayoutDecision {
            flow: LabelFlow::Reverse,
            anchor: LabelAnchorPolicy::OriginalFirstGroup,
        };

        let ph = layout_label_text("Ph", &decision);
        assert_eq!(ph.lines, vec!["Ph"]);
        assert_eq!(ph.anchor_char, 1);

        let n3 = layout_label_text("N3", &decision);
        assert_eq!(n3.lines, vec!["N3"]);
        assert_eq!(n3.anchor_char, 1);
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

        let fallback = decide_label_layout(&[90.0], true, true);
        assert_eq!(fallback.flow, LabelFlow::Forward);
    }
}
