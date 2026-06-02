use crate::{
    Bond, BondLinePattern, BondLineStyles, BondLineWeight, BondLineWeights, BondStereo,
    BondVariant, DoubleBond, DoubleBondPlacement, Point,
};

pub(super) fn update_terminal_double_bond_placement_after_new_attachment(
    fragment: &mut crate::MoleculeFragment,
    attached_node_id: &str,
    new_bond_id: &str,
) {
    let connected_bond_ids: Vec<_> = fragment
        .bonds
        .iter()
        .filter(|bond| bond.begin == attached_node_id || bond.end == attached_node_id)
        .map(|bond| bond.id.clone())
        .collect();
    for bond_id in connected_bond_ids {
        if bond_id != new_bond_id {
            update_unfrozen_double_bond_auto_placement(fragment, &bond_id, new_bond_id);
        }
    }
}

#[derive(Default)]
struct SegmentEndpointSideCounts {
    begin_left: usize,
    begin_right: usize,
    end_left: usize,
    end_right: usize,
}

fn connected_attachment_side_counts_for_segment(
    fragment: &crate::MoleculeFragment,
    begin_id: &str,
    end_id: &str,
    ignored_bond_id: Option<&str>,
) -> Option<SegmentEndpointSideCounts> {
    let begin = fragment.nodes.iter().find(|node| node.id == begin_id)?;
    let end = fragment.nodes.iter().find(|node| node.id == end_id)?;
    let begin_point = begin.point();
    let end_point = end.point();
    let axis_x = end_point.x - begin_point.x;
    let axis_y = end_point.y - begin_point.y;
    let axis_length = axis_x.hypot(axis_y);
    if axis_length <= crate::EPSILON {
        return None;
    }
    let normal_x = -axis_y / axis_length;
    let normal_y = axis_x / axis_length;

    let mut counts = SegmentEndpointSideCounts::default();
    for other in &fragment.bonds {
        if ignored_bond_id.is_some_and(|ignored| other.id == ignored) {
            continue;
        }
        let (shared_id, shared_is_begin) = if other.begin == begin_id || other.end == begin_id {
            (Some(begin_id), true)
        } else if other.begin == end_id || other.end == end_id {
            (Some(end_id), false)
        } else {
            (None, false)
        };
        let Some(shared_id) = shared_id else {
            continue;
        };
        let other_id = if other.begin == shared_id {
            other.end.as_str()
        } else {
            other.begin.as_str()
        };
        let Some(shared_node) = fragment.nodes.iter().find(|node| node.id == shared_id) else {
            continue;
        };
        let Some(other_node) = fragment.nodes.iter().find(|node| node.id == other_id) else {
            continue;
        };
        let side_score = (other_node.position[0] - shared_node.position[0]) * normal_x
            + (other_node.position[1] - shared_node.position[1]) * normal_y;
        if side_score > crate::EPSILON {
            if shared_is_begin {
                counts.begin_left += 1;
            } else {
                counts.end_left += 1;
            }
        } else if side_score < -crate::EPSILON {
            if shared_is_begin {
                counts.begin_right += 1;
            } else {
                counts.end_right += 1;
            }
        }
    }

    Some(counts)
}

pub(super) fn should_default_center_double_bond_for_segment(
    fragment: &crate::MoleculeFragment,
    begin_id: &str,
    end_id: &str,
    ignored_bond_id: Option<&str>,
) -> bool {
    automatic_double_bond_placement_for_segment(fragment, begin_id, end_id, ignored_bond_id)
        == DoubleBondPlacement::Center
}

fn terminal_geminal_endpoint_should_center(
    left_count: usize,
    right_count: usize,
    other_total: usize,
) -> bool {
    other_total == 0 && left_count > 0 && right_count > 0
}

pub(super) fn preferred_double_bond_side_for_segment(
    fragment: &crate::MoleculeFragment,
    begin_id: &str,
    end_id: &str,
    ignored_bond_id: Option<&str>,
) -> Option<DoubleBondPlacement> {
    Some(automatic_double_bond_side_for_segment(
        fragment,
        begin_id,
        end_id,
        ignored_bond_id,
    ))
}

fn automatic_double_bond_side_for_segment(
    fragment: &crate::MoleculeFragment,
    begin_id: &str,
    end_id: &str,
    ignored_bond_id: Option<&str>,
) -> DoubleBondPlacement {
    if let Some(placement) =
        ring_double_bond_placement_for_segment(fragment, begin_id, end_id, ignored_bond_id)
    {
        return placement;
    }
    preferred_substituent_side_for_segment(fragment, begin_id, end_id, ignored_bond_id)
        .unwrap_or(DoubleBondPlacement::Right)
}

fn preferred_substituent_side_for_segment(
    fragment: &crate::MoleculeFragment,
    begin_id: &str,
    end_id: &str,
    ignored_bond_id: Option<&str>,
) -> Option<DoubleBondPlacement> {
    let begin = fragment.nodes.iter().find(|node| node.id == begin_id)?;
    let end = fragment.nodes.iter().find(|node| node.id == end_id)?;
    let begin_point = begin.point();
    let end_point = end.point();
    let dx = end_point.x - begin_point.x;
    let dy = end_point.y - begin_point.y;
    let length = dx.hypot(dy);
    if length <= crate::EPSILON {
        return Some(DoubleBondPlacement::Left);
    }
    let normal_x = -dy / length;
    let normal_y = dx / length;
    let mut score = 0.0;
    let mut attachment_count = 0usize;
    for other in &fragment.bonds {
        if ignored_bond_id.is_some_and(|ignored| other.id == ignored) {
            continue;
        }
        if other.begin == begin_id || other.end == begin_id {
            let other_id = if other.begin == begin_id {
                &other.end
            } else {
                &other.begin
            };
            if let Some(neighbor) = fragment.nodes.iter().find(|node| &node.id == other_id) {
                let point = neighbor.point();
                attachment_count += 1;
                score +=
                    (point.x - begin_point.x) * normal_x + (point.y - begin_point.y) * normal_y;
            }
        } else if other.begin == end_id || other.end == end_id {
            let other_id = if other.begin == end_id {
                &other.end
            } else {
                &other.begin
            };
            if let Some(neighbor) = fragment.nodes.iter().find(|node| &node.id == other_id) {
                let point = neighbor.point();
                attachment_count += 1;
                score += (point.x - end_point.x) * normal_x + (point.y - end_point.y) * normal_y;
            }
        }
    }
    if attachment_count == 0 {
        return None;
    }
    Some(placement_from_signed_side_score(score))
}

fn placement_from_signed_side_score(score: f64) -> DoubleBondPlacement {
    if score > crate::EPSILON {
        DoubleBondPlacement::Left
    } else if score < -crate::EPSILON {
        DoubleBondPlacement::Right
    } else {
        DoubleBondPlacement::Right
    }
}

pub fn automatic_double_bond_placement_for_segment(
    fragment: &crate::MoleculeFragment,
    begin_id: &str,
    end_id: &str,
    ignored_bond_id: Option<&str>,
) -> DoubleBondPlacement {
    if segment_has_neighbor_double_bond(fragment, begin_id, end_id, ignored_bond_id) {
        return DoubleBondPlacement::Center;
    }
    if let Some(placement) =
        ring_double_bond_placement_for_segment(fragment, begin_id, end_id, ignored_bond_id)
    {
        return placement;
    }
    let Some(counts) =
        connected_attachment_side_counts_for_segment(fragment, begin_id, end_id, ignored_bond_id)
    else {
        return DoubleBondPlacement::Right;
    };
    let begin_total = counts.begin_left + counts.begin_right;
    let end_total = counts.end_left + counts.end_right;
    if begin_total + end_total == 0 {
        return DoubleBondPlacement::Center;
    }
    if terminal_geminal_endpoint_should_center(counts.begin_left, counts.begin_right, end_total)
        || terminal_geminal_endpoint_should_center(counts.end_left, counts.end_right, begin_total)
    {
        return DoubleBondPlacement::Center;
    }
    preferred_substituent_side_for_segment(fragment, begin_id, end_id, ignored_bond_id)
        .unwrap_or(DoubleBondPlacement::Right)
}

fn segment_has_neighbor_double_bond(
    fragment: &crate::MoleculeFragment,
    begin_id: &str,
    end_id: &str,
    ignored_bond_id: Option<&str>,
) -> bool {
    fragment.bonds.iter().any(|bond| {
        !ignored_bond_id.is_some_and(|ignored| bond.id == ignored)
            && bond.order == 2
            && (bond.begin == begin_id
                || bond.end == begin_id
                || bond.begin == end_id
                || bond.end == end_id)
    })
}

fn ring_double_bond_placement_for_segment(
    fragment: &crate::MoleculeFragment,
    begin_id: &str,
    end_id: &str,
    ignored_bond_id: Option<&str>,
) -> Option<DoubleBondPlacement> {
    let cycle = preferred_cycle_path_for_segment(fragment, begin_id, end_id, ignored_bond_id)?;
    let begin = fragment
        .nodes
        .iter()
        .find(|node| node.id == begin_id)?
        .point();
    let end = fragment
        .nodes
        .iter()
        .find(|node| node.id == end_id)?
        .point();
    let dx = end.x - begin.x;
    let dy = end.y - begin.y;
    let length = dx.hypot(dy);
    if length <= crate::EPSILON {
        return None;
    }
    let normal_x = -dy / length;
    let normal_y = dx / length;
    let midpoint = Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5);
    let mut score = 0.0;
    for node_id in cycle {
        if node_id == begin_id || node_id == end_id {
            continue;
        }
        let point = fragment
            .nodes
            .iter()
            .find(|node| node.id == node_id)?
            .point();
        score += (point.x - midpoint.x) * normal_x + (point.y - midpoint.y) * normal_y;
    }
    (score.abs() > crate::EPSILON).then(|| placement_from_signed_side_score(score))
}

fn preferred_cycle_path_for_segment(
    fragment: &crate::MoleculeFragment,
    begin_id: &str,
    end_id: &str,
    ignored_bond_id: Option<&str>,
) -> Option<Vec<String>> {
    let mut best: Option<(Vec<String>, CyclePreference)> = None;
    let mut queue = std::collections::VecDeque::from([vec![begin_id.to_string()]]);
    while let Some(path) = queue.pop_front() {
        if path.len() > 12 {
            continue;
        }
        let current = path.last()?;
        for bond in &fragment.bonds {
            if ignored_bond_id.is_some_and(|ignored| bond.id == ignored) {
                continue;
            }
            let neighbor = if bond.begin == *current {
                bond.end.as_str()
            } else if bond.end == *current {
                bond.begin.as_str()
            } else {
                continue;
            };
            if neighbor == end_id {
                let mut cycle = path.clone();
                cycle.push(end_id.to_string());
                if cycle.len() >= 3 {
                    let preference = cycle_preference_for_path(fragment, &cycle, ignored_bond_id);
                    if best
                        .as_ref()
                        .is_none_or(|(_, best_preference)| preference > *best_preference)
                    {
                        best = Some((cycle, preference));
                    }
                }
                continue;
            }
            if path.iter().any(|node_id| node_id == neighbor) {
                continue;
            }
            let mut next = path.clone();
            next.push(neighbor.to_string());
            queue.push_back(next);
        }
    }
    best.map(|(cycle, _)| cycle)
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CyclePreference {
    is_alternating_six: bool,
    is_fully_alternating: bool,
    alternating_matches: usize,
    near_six_score: i32,
    short_score: i32,
}

fn cycle_preference_for_path(
    fragment: &crate::MoleculeFragment,
    path: &[String],
    ignored_bond_id: Option<&str>,
) -> CyclePreference {
    let orders = cycle_path_bond_orders(fragment, path, ignored_bond_id).unwrap_or_default();
    let mut alternating_matches = 0usize;
    let mut is_fully_alternating = !orders.is_empty();
    for (index, order) in orders.iter().enumerate() {
        let expected = if index % 2 == 0 { 1 } else { 2 };
        if *order == expected {
            alternating_matches += 1;
        } else {
            is_fully_alternating = false;
        }
    }
    let length = path.len() as i32;
    CyclePreference {
        is_alternating_six: is_fully_alternating && path.len() == 6,
        is_fully_alternating,
        alternating_matches,
        near_six_score: -(length - 6).abs(),
        short_score: -length,
    }
}

fn cycle_path_bond_orders(
    fragment: &crate::MoleculeFragment,
    path: &[String],
    ignored_bond_id: Option<&str>,
) -> Option<Vec<u8>> {
    if path.len() < 2 {
        return None;
    }
    let mut orders = Vec::with_capacity(path.len() - 1);
    for pair in path.windows(2) {
        let bond = fragment.bonds.iter().find(|bond| {
            !ignored_bond_id.is_some_and(|ignored| bond.id == ignored)
                && ((bond.begin == pair[0] && bond.end == pair[1])
                    || (bond.begin == pair[1] && bond.end == pair[0]))
        })?;
        orders.push(bond.order);
    }
    Some(orders)
}

fn update_unfrozen_double_bond_auto_placement(
    fragment: &mut crate::MoleculeFragment,
    double_bond_id: &str,
    _new_bond_id: &str,
) {
    let Some(double_index) = fragment
        .bonds
        .iter()
        .position(|bond| bond.id == double_bond_id && bond.order == 2)
    else {
        return;
    };
    let Some(double) = fragment.bonds[double_index].double.as_ref() else {
        return;
    };
    if double.frozen {
        return;
    }

    let bond = fragment.bonds[double_index].clone();
    let placement = automatic_double_bond_placement_for_segment(
        fragment,
        &bond.begin,
        &bond.end,
        Some(&bond.id),
    );
    fragment.bonds[double_index].double = Some(crate::DoubleBond {
        placement,
        center_exit_side: None,
        frozen: false,
    });
}

fn opposite_double_bond_placement(placement: DoubleBondPlacement) -> DoubleBondPlacement {
    match placement {
        DoubleBondPlacement::Left => DoubleBondPlacement::Right,
        DoubleBondPlacement::Right => DoubleBondPlacement::Left,
        DoubleBondPlacement::Center => DoubleBondPlacement::Right,
    }
}

pub(super) fn apply_single_tool_center_style(
    bond: &mut Bond,
    default_placement: DoubleBondPlacement,
) -> bool {
    if is_plain_single_bond(bond) {
        return advance_plain_double_cycle(bond, default_placement);
    }
    if is_plain_double_bond(bond) {
        return advance_plain_double_cycle(bond, default_placement);
    }
    replace_with_plain_single_bond_style(bond)
}

pub(super) fn apply_double_tool_center_style(
    bond: &mut Bond,
    default_placement: DoubleBondPlacement,
) -> bool {
    if is_plain_single_bond(bond) || is_plain_triple_bond(bond) {
        return replace_with_plain_double_bond_style(bond, default_placement);
    }
    if is_plain_double_bond(bond) {
        return advance_plain_double_cycle(bond, default_placement);
    }
    if is_bold_family_bond(bond) {
        return if bond.order == 2 {
            cycle_bold_double_bond_style(bond, Some(default_placement))
        } else {
            cycle_bold_single_bond_style(bond, Some(default_placement))
        };
    }
    replace_with_plain_double_bond_style(bond, default_placement)
}

pub(super) fn cycle_dashed_bond_center_style(
    bond: &mut Bond,
    default_placement: DoubleBondPlacement,
) -> bool {
    if bond.order == 2 && !has_stereo_style(bond) {
        return cycle_dashed_double_bond_style(bond, Some(default_placement));
    }
    replace_with_plain_dashed_bond_style(bond)
}

pub(super) fn cycle_dashed_double_bond_tool_center_style(
    bond: &mut Bond,
    default_placement: DoubleBondPlacement,
) -> bool {
    if bond.order == 2 && !has_stereo_style(bond) {
        return advance_plain_dashed_double_cycle(bond, default_placement);
    }
    replace_with_plain_dashed_double_bond_style(bond, default_placement)
}

pub(super) fn cycle_bold_bond_center_style(
    bond: &mut Bond,
    default_placement: DoubleBondPlacement,
) -> bool {
    if bond.order == 2 && !has_stereo_style(bond) {
        if is_bold_family_bond(bond) {
            return cycle_bold_double_bond_style(bond, Some(default_placement));
        }
        let placement = bond
            .double
            .as_ref()
            .map(|double| double.placement)
            .unwrap_or(default_placement);
        return init_bold_double_bond_style(bond, placement, default_placement);
    }
    if bond.order == 1 && !has_stereo_style(bond) && all_line_patterns_solid(bond) {
        return cycle_bold_single_bond_style(bond, Some(default_placement));
    }
    if is_bold_family_bond(bond) && bond.order == 2 {
        return cycle_bold_double_bond_style(bond, Some(default_placement));
    }
    replace_with_plain_bold_bond_style(bond)
}

fn cycle_dashed_double_bond_style(
    bond: &mut Bond,
    default_placement: Option<DoubleBondPlacement>,
) -> bool {
    let default_side = default_placement.unwrap_or(DoubleBondPlacement::Right);
    let placement = bond
        .double
        .as_ref()
        .map(|double| double.placement)
        .unwrap_or(default_side);
    match placement {
        DoubleBondPlacement::Left | DoubleBondPlacement::Right => {
            let side_pattern = outer_line_pattern_mut(&mut bond.line_styles, placement);
            if *side_pattern != BondLinePattern::Dashed {
                *side_pattern = BondLinePattern::Dashed;
            } else if bond.line_styles.main != BondLinePattern::Dashed {
                bond.line_styles.main = BondLinePattern::Dashed;
            } else {
                let exit_side = opposite_double_bond_placement(placement);
                bond.double = Some(DoubleBond {
                    placement: DoubleBondPlacement::Center,
                    center_exit_side: Some(exit_side),
                    frozen: false,
                });
                bond.line_styles.main = BondLinePattern::Solid;
                bond.line_styles.left = BondLinePattern::Dashed;
                bond.line_styles.right = BondLinePattern::Dashed;
            }
            true
        }
        DoubleBondPlacement::Center => {
            let dashed_sides = centered_dashed_sides(&bond.line_styles);
            if dashed_sides.is_empty() {
                *outer_line_pattern_mut(&mut bond.line_styles, default_side) =
                    BondLinePattern::Dashed;
                bond.double = Some(DoubleBond {
                    placement: DoubleBondPlacement::Center,
                    center_exit_side: None,
                    frozen: false,
                });
                return true;
            }
            if dashed_sides.len() == 1 {
                let first_dashed = dashed_sides[0];
                let second_side = opposite_double_bond_placement(first_dashed);
                *outer_line_pattern_mut(&mut bond.line_styles, second_side) =
                    BondLinePattern::Dashed;
                bond.double = Some(DoubleBond {
                    placement: DoubleBondPlacement::Center,
                    center_exit_side: Some(opposite_double_bond_placement(first_dashed)),
                    frozen: false,
                });
                return true;
            }

            let exit_side = bond
                .double
                .as_ref()
                .and_then(|double| double.center_exit_side)
                .unwrap_or(default_side);
            bond.double = Some(DoubleBond {
                placement: exit_side,
                center_exit_side: None,
                frozen: false,
            });
            bond.line_styles.main = BondLinePattern::Solid;
            bond.line_styles.left = BondLinePattern::Solid;
            bond.line_styles.right = BondLinePattern::Solid;
            *outer_line_pattern_mut(&mut bond.line_styles, exit_side) = BondLinePattern::Dashed;
            true
        }
    }
}

fn advance_plain_dashed_double_cycle(
    bond: &mut Bond,
    default_placement: DoubleBondPlacement,
) -> bool {
    let opposite_placement = opposite_double_bond_placement(default_placement);
    let dashed_side = current_dashed_double_side(bond);
    let next_placement = match bond.double.as_ref().map(|double| double.placement) {
        Some(current) if current == default_placement => DoubleBondPlacement::Center,
        Some(DoubleBondPlacement::Center) if dashed_side == Some(default_placement) => {
            opposite_placement
        }
        Some(current) if current == opposite_placement => DoubleBondPlacement::Center,
        Some(DoubleBondPlacement::Center) if dashed_side == Some(opposite_placement) => {
            default_placement
        }
        _ => default_placement,
    };

    bond.order = 2;
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    bond.double = Some(DoubleBond {
        placement: next_placement,
        center_exit_side: None,
        frozen: false,
    });

    let next_dashed_side = match next_placement {
        DoubleBondPlacement::Left | DoubleBondPlacement::Right => next_placement,
        DoubleBondPlacement::Center => dashed_side.unwrap_or(default_placement),
    };
    *outer_line_pattern_mut(&mut bond.line_styles, next_dashed_side) = BondLinePattern::Dashed;
    true
}

fn advance_plain_double_cycle(bond: &mut Bond, default_placement: DoubleBondPlacement) -> bool {
    let opposite_placement = opposite_double_bond_placement(default_placement);
    let next_placement = if bond.order == 1 {
        default_placement
    } else {
        match bond.double.as_ref().map(|double| double.placement) {
            Some(current) if current == default_placement => DoubleBondPlacement::Center,
            Some(DoubleBondPlacement::Center) => opposite_placement,
            Some(current) if current == opposite_placement => default_placement,
            _ => default_placement,
        }
    };
    bond.order = 2;
    bond.double = Some(DoubleBond {
        placement: next_placement,
        center_exit_side: None,
        frozen: false,
    });
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    true
}

fn replace_with_plain_single_bond_style(bond: &mut Bond) -> bool {
    bond.order = 1;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    true
}

fn replace_with_plain_double_bond_style(bond: &mut Bond, placement: DoubleBondPlacement) -> bool {
    bond.order = 2;
    bond.double = Some(DoubleBond {
        placement,
        center_exit_side: None,
        frozen: false,
    });
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    true
}

pub(super) fn replace_with_plain_triple_bond_style(bond: &mut Bond) -> bool {
    bond.order = 3;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    true
}

fn replace_with_plain_dashed_bond_style(bond: &mut Bond) -> bool {
    bond.order = 1;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles {
        main: BondLinePattern::Dashed,
        ..BondLineStyles::default()
    };
    bond.line_weights = BondLineWeights::default();
    true
}

pub(super) fn replace_with_plain_wavy_bond_style(bond: &mut Bond) -> bool {
    bond.order = 1;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles {
        main: BondLinePattern::Wavy,
        ..BondLineStyles::default()
    };
    bond.line_weights = BondLineWeights::default();
    true
}

fn replace_with_plain_dashed_double_bond_style(
    bond: &mut Bond,
    placement: DoubleBondPlacement,
) -> bool {
    bond.order = 2;
    bond.double = Some(DoubleBond {
        placement,
        center_exit_side: None,
        frozen: false,
    });
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    *outer_line_pattern_mut(&mut bond.line_styles, placement) = BondLinePattern::Dashed;
    true
}

fn current_dashed_double_side(bond: &Bond) -> Option<DoubleBondPlacement> {
    let left_dashed = bond.line_styles.left == BondLinePattern::Dashed;
    let right_dashed = bond.line_styles.right == BondLinePattern::Dashed;
    match (left_dashed, right_dashed) {
        (true, false) => Some(DoubleBondPlacement::Left),
        (false, true) => Some(DoubleBondPlacement::Right),
        _ => None,
    }
}

fn replace_with_plain_bold_bond_style(bond: &mut Bond) -> bool {
    bond.order = 1;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights {
        main: BondLineWeight::Bold,
        ..BondLineWeights::default()
    };
    true
}

pub(super) fn replace_with_bold_dashed_bond_style(bond: &mut Bond) -> bool {
    bond.order = 1;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles {
        main: BondLinePattern::Dashed,
        ..BondLineStyles::default()
    };
    bond.line_weights = BondLineWeights {
        main: BondLineWeight::Bold,
        ..BondLineWeights::default()
    };
    true
}

pub(super) fn replace_with_stereo_bond_style(bond: &mut Bond, variant: BondVariant) -> bool {
    let kind = match variant {
        BondVariant::Wedge => "solid-wedge",
        BondVariant::HashedWedge => "hashed-wedge",
        BondVariant::HollowWedge => "hollow-wedge",
        _ => return false,
    };
    let current_wide_end = bond
        .stereo
        .as_ref()
        .map(|stereo| stereo.wide_end.as_str())
        .unwrap_or("end");
    let next_wide_end = match bond.stereo.as_ref() {
        Some(stereo) if stereo.kind == kind && stereo.wide_end == "end" => "begin",
        Some(stereo) if stereo.kind == kind && stereo.wide_end == "begin" => "end",
        Some(_) => current_wide_end,
        None => "end",
    };
    bond.order = 1;
    bond.double = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    bond.stereo = Some(BondStereo {
        kind: kind.to_string(),
        wide_end: next_wide_end.to_string(),
    });
    true
}

fn init_bold_double_bond_style(
    bond: &mut Bond,
    placement: DoubleBondPlacement,
    default_placement: DoubleBondPlacement,
) -> bool {
    bond.order = 2;
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    match placement {
        DoubleBondPlacement::Left | DoubleBondPlacement::Right => {
            bond.double = Some(DoubleBond {
                placement,
                center_exit_side: None,
                frozen: false,
            });
            bond.line_weights.main = BondLineWeight::Bold;
        }
        DoubleBondPlacement::Center => {
            bond.double = Some(DoubleBond {
                placement: DoubleBondPlacement::Center,
                center_exit_side: Some(opposite_double_bond_placement(default_placement)),
                frozen: false,
            });
            *outer_line_weight_mut(&mut bond.line_weights, default_placement) =
                BondLineWeight::Bold;
        }
    }
    true
}

fn cycle_bold_single_bond_style(
    bond: &mut Bond,
    default_placement: Option<DoubleBondPlacement>,
) -> bool {
    if bond.line_weights.main != BondLineWeight::Bold {
        bond.line_weights.main = BondLineWeight::Bold;
        return true;
    }

    let side = default_placement
        .filter(|placement| *placement != DoubleBondPlacement::Center)
        .unwrap_or(DoubleBondPlacement::Right);
    bond.order = 2;
    bond.double = Some(DoubleBond {
        placement: side,
        center_exit_side: None,
        frozen: false,
    });
    bond.line_weights.main = BondLineWeight::Bold;
    bond.line_weights.left = BondLineWeight::Normal;
    bond.line_weights.right = BondLineWeight::Normal;
    true
}

fn cycle_bold_double_bond_style(
    bond: &mut Bond,
    default_placement: Option<DoubleBondPlacement>,
) -> bool {
    let default_side = default_placement.unwrap_or(DoubleBondPlacement::Right);
    let placement = bond
        .double
        .as_ref()
        .map(|double| double.placement)
        .unwrap_or(default_side);
    match placement {
        DoubleBondPlacement::Left | DoubleBondPlacement::Right => {
            if bond.line_weights.main != BondLineWeight::Bold {
                bond.line_weights.main = BondLineWeight::Bold;
                return true;
            }

            let exit_side = opposite_double_bond_placement(placement);
            bond.double = Some(DoubleBond {
                placement: DoubleBondPlacement::Center,
                center_exit_side: Some(exit_side),
                frozen: false,
            });
            bond.line_weights.main = BondLineWeight::Normal;
            bond.line_weights.left = BondLineWeight::Normal;
            bond.line_weights.right = BondLineWeight::Normal;
            *outer_line_weight_mut(&mut bond.line_weights, placement) = BondLineWeight::Bold;
            true
        }
        DoubleBondPlacement::Center => {
            let bold_sides = centered_bold_sides(&bond.line_weights);
            if bold_sides.is_empty() {
                *outer_line_weight_mut(&mut bond.line_weights, default_side) = BondLineWeight::Bold;
                bond.double = Some(DoubleBond {
                    placement: DoubleBondPlacement::Center,
                    center_exit_side: Some(opposite_double_bond_placement(default_side)),
                    frozen: false,
                });
                return true;
            }

            let exit_side = bond
                .double
                .as_ref()
                .and_then(|double| double.center_exit_side)
                .unwrap_or_else(|| opposite_double_bond_placement(bold_sides[0]));
            bond.double = Some(DoubleBond {
                placement: exit_side,
                center_exit_side: None,
                frozen: false,
            });
            bond.line_weights.main = BondLineWeight::Bold;
            bond.line_weights.left = BondLineWeight::Normal;
            bond.line_weights.right = BondLineWeight::Normal;
            true
        }
    }
}

fn is_plain_single_bond(bond: &Bond) -> bool {
    bond.order == 1
        && bond.double.is_none()
        && bond.stereo.is_none()
        && all_line_patterns_solid(bond)
        && all_line_weights_normal(bond)
}

fn is_plain_double_bond(bond: &Bond) -> bool {
    bond.order == 2
        && bond.stereo.is_none()
        && all_line_patterns_solid(bond)
        && all_line_weights_normal(bond)
}

fn is_plain_triple_bond(bond: &Bond) -> bool {
    bond.order == 3
        && bond.double.is_none()
        && bond.stereo.is_none()
        && all_line_patterns_solid(bond)
        && all_line_weights_normal(bond)
}

fn is_bold_family_bond(bond: &Bond) -> bool {
    bond.stereo.is_none()
        && all_line_patterns_solid(bond)
        && (bond.line_weights.main == BondLineWeight::Bold
            || bond.line_weights.left == BondLineWeight::Bold
            || bond.line_weights.right == BondLineWeight::Bold)
}

fn has_stereo_style(bond: &Bond) -> bool {
    bond.stereo.is_some()
}

fn all_line_patterns_solid(bond: &Bond) -> bool {
    bond.line_styles.main == BondLinePattern::Solid
        && bond.line_styles.left == BondLinePattern::Solid
        && bond.line_styles.right == BondLinePattern::Solid
}

fn all_line_weights_normal(bond: &Bond) -> bool {
    bond.line_weights.main == BondLineWeight::Normal
        && bond.line_weights.left == BondLineWeight::Normal
        && bond.line_weights.right == BondLineWeight::Normal
}

fn centered_dashed_sides(line_styles: &BondLineStyles) -> Vec<DoubleBondPlacement> {
    let mut out = Vec::new();
    if line_styles.left == BondLinePattern::Dashed {
        out.push(DoubleBondPlacement::Left);
    }
    if line_styles.right == BondLinePattern::Dashed {
        out.push(DoubleBondPlacement::Right);
    }
    out
}

fn centered_bold_sides(line_weights: &BondLineWeights) -> Vec<DoubleBondPlacement> {
    let mut out = Vec::new();
    if line_weights.left == BondLineWeight::Bold {
        out.push(DoubleBondPlacement::Left);
    }
    if line_weights.right == BondLineWeight::Bold {
        out.push(DoubleBondPlacement::Right);
    }
    out
}

fn outer_line_pattern_mut(
    line_styles: &mut BondLineStyles,
    placement: DoubleBondPlacement,
) -> &mut BondLinePattern {
    match placement {
        DoubleBondPlacement::Left => &mut line_styles.left,
        DoubleBondPlacement::Right => &mut line_styles.right,
        DoubleBondPlacement::Center => &mut line_styles.right,
    }
}

fn outer_line_weight_mut(
    line_weights: &mut BondLineWeights,
    placement: DoubleBondPlacement,
) -> &mut BondLineWeight {
    match placement {
        DoubleBondPlacement::Left => &mut line_weights.left,
        DoubleBondPlacement::Right => &mut line_weights.right,
        DoubleBondPlacement::Center => &mut line_weights.right,
    }
}

pub(super) fn centered_oriented_rect_points(
    start: Point,
    end: Point,
    length_along_bond: f64,
    width_across_bond: f64,
) -> Vec<Point> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let bond_length = dx.hypot(dy);
    let center = Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
    if bond_length <= crate::EPSILON {
        let half = width_across_bond / 2.0;
        return vec![
            Point::new(center.x - half, center.y - half),
            Point::new(center.x + half, center.y - half),
            Point::new(center.x + half, center.y + half),
            Point::new(center.x - half, center.y + half),
        ];
    }
    let ux = dx / bond_length;
    let uy = dy / bond_length;
    let tx = ux * length_along_bond / 2.0;
    let ty = uy * length_along_bond / 2.0;
    let nx = -uy * width_across_bond / 2.0;
    let ny = ux * width_across_bond / 2.0;
    vec![
        Point::new(center.x - tx + nx, center.y - ty + ny),
        Point::new(center.x + tx + nx, center.y + ty + ny),
        Point::new(center.x + tx - nx, center.y + ty - ny),
        Point::new(center.x - tx - nx, center.y - ty - ny),
    ]
}
