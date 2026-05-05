use super::*;

#[derive(Debug, Clone)]
enum ValenceTokenKind {
    Atom {
        element: String,
        from_numeric_count: bool,
    },
    Terminal {
        fragment: FragmentDef,
        matched: String,
    },
    Group {
        tokens: Vec<ValenceToken>,
        count: usize,
    },
}

#[derive(Debug, Clone)]
struct ValenceToken {
    label: String,
    kind: ValenceTokenKind,
}

#[derive(Debug, Clone)]
struct ValenceNodeState {
    component_index: usize,
    element: String,
    valence: u8,
    used: u8,
    unconstrained_valence: bool,
}

pub(super) fn parse_valence_terminal_label(label: &str) -> Option<AbbreviationRecognition> {
    let tokens = tokenize_valence_label(label)?;
    let (components, _) = parse_valence_fragment_tokens(&tokens, 1, 0)?;
    let formula = valence_formula(&components);
    let anchor_atom = components
        .first()
        .map(|component| component.left_anchor.clone())
        .unwrap_or_default();
    Some(AbbreviationRecognition {
        label: label.to_string(),
        canonical_label: canonical_valence_label(label),
        kind: "valence-fragment".to_string(),
        formula,
        anchor_atom,
        components,
    })
}

pub(super) fn parse_chemical_text_label(label: &str) -> Option<AbbreviationRecognition> {
    let tokens = tokenize_valence_label(label)?;
    if tokens.is_empty() {
        return None;
    }
    Some(AbbreviationRecognition {
        label: label.to_string(),
        canonical_label: canonical_valence_label(label),
        kind: "chemical-text".to_string(),
        formula: label.to_string(),
        anchor_atom: String::new(),
        components: Vec::new(),
    })
}

fn parse_valence_fragment_tokens(
    tokens: &[ValenceToken],
    root_used: u8,
    allowed_root_remaining: u8,
) -> Option<(Vec<AbbreviationComponent>, Vec<ValenceNodeState>)> {
    if tokens.is_empty() || !matches!(tokens.first()?.kind, ValenceTokenKind::Atom { .. }) {
        return None;
    }
    let mut components = Vec::new();
    let mut nodes = Vec::new();
    for (valence, charge) in valence_options(tokens, 0, root_used) {
        let mut root_components = Vec::new();
        let mut root_nodes = Vec::new();
        let ValenceTokenKind::Atom { element, .. } = &tokens[0].kind else {
            return None;
        };
        let component_index = push_valence_atom_component(
            &mut root_components,
            &tokens[0].label,
            element,
            None,
            None,
            charge,
        );
        root_nodes.push(ValenceNodeState {
            component_index,
            element: element.clone(),
            valence,
            used: root_used,
            unconstrained_valence: is_unconstrained_valence_element(element),
        });
        if parse_valence_tokens_from(
            tokens,
            1,
            root_components,
            root_nodes,
            allowed_root_remaining,
            &mut components,
            &mut nodes,
        ) {
            break;
        }
    }
    if components.is_empty() || !nodes_are_satisfied(&nodes, allowed_root_remaining) {
        return None;
    }
    Some((components, nodes))
}

fn canonical_valence_label(label: &str) -> String {
    match label {
        "COOH" => "CO2H".to_string(),
        "COCH3" => "COMe".to_string(),
        "OCH3" => "OMe".to_string(),
        _ => label.to_string(),
    }
}

fn parse_valence_tokens_from(
    tokens: &[ValenceToken],
    index: usize,
    components: Vec<AbbreviationComponent>,
    nodes: Vec<ValenceNodeState>,
    allowed_root_remaining: u8,
    out_components: &mut Vec<AbbreviationComponent>,
    out_nodes: &mut Vec<ValenceNodeState>,
) -> bool {
    if index >= tokens.len() {
        if nodes_are_satisfied(&nodes, allowed_root_remaining) {
            *out_components = components;
            *out_nodes = nodes;
            return true;
        }
        return false;
    }
    let Some(parent_index) = nodes.iter().rposition(|node| node.used < node.valence) else {
        return false;
    };
    let parent_remaining = nodes[parent_index].valence - nodes[parent_index].used;
    match &tokens[index].kind {
        ValenceTokenKind::Terminal { fragment, matched } => {
            if parent_remaining < 1 {
                return false;
            }
            let mut next_components = components;
            let mut next_nodes = nodes;
            let mut component = fragment.component(matched);
            component.kind = "terminal".to_string();
            component.parent_index = Some(next_nodes[parent_index].component_index);
            component.bond_order_to_parent = Some(1);
            next_components.push(component);
            next_nodes[parent_index].used += 1;
            parse_valence_tokens_from(
                tokens,
                index + 1,
                next_components,
                next_nodes,
                allowed_root_remaining,
                out_components,
                out_nodes,
            )
        }
        ValenceTokenKind::Atom { element, .. } => {
            for (valence, charge) in valence_options(tokens, index, 0) {
                for bond_order in bond_order_candidates(
                    &next_nodes_element(&nodes, parent_index),
                    element,
                    parent_remaining,
                    valence,
                ) {
                    if bond_order > parent_remaining || bond_order > valence {
                        continue;
                    }
                    let mut next_components = components.clone();
                    let mut next_nodes = nodes.clone();
                    let component_index = push_valence_atom_component(
                        &mut next_components,
                        &tokens[index].label,
                        element,
                        Some(next_nodes[parent_index].component_index),
                        Some(bond_order),
                        charge,
                    );
                    next_nodes[parent_index].used += bond_order;
                    next_nodes.push(ValenceNodeState {
                        component_index,
                        element: element.clone(),
                        valence,
                        used: bond_order,
                        unconstrained_valence: is_unconstrained_valence_element(element),
                    });
                    if parse_valence_tokens_from(
                        tokens,
                        index + 1,
                        next_components,
                        next_nodes,
                        allowed_root_remaining,
                        out_components,
                        out_nodes,
                    ) {
                        return true;
                    }
                }
            }
            false
        }
        ValenceTokenKind::Group {
            tokens: group_tokens,
            count,
        } => {
            let Some((group_components, group_nodes)) =
                parse_valence_group_for_attachment(group_tokens)
            else {
                return false;
            };
            let mut next_components = components;
            let mut next_nodes = nodes;
            for _ in 0..*count {
                if !append_attached_valence_group(
                    &group_components,
                    &group_nodes,
                    parent_index,
                    &mut next_components,
                    &mut next_nodes,
                ) {
                    return false;
                }
            }
            parse_valence_tokens_from(
                tokens,
                index + 1,
                next_components,
                next_nodes,
                allowed_root_remaining,
                out_components,
                out_nodes,
            )
        }
    }
}

fn parse_valence_group_for_attachment(
    group_tokens: &[ValenceToken],
) -> Option<(Vec<AbbreviationComponent>, Vec<ValenceNodeState>)> {
    // Parenthesized groups are first parsed as standalone. Only if the first
    // atom has exactly one spare valence do we reparse it as attached.
    parse_valence_fragment_tokens(group_tokens, 0, 1)?;
    parse_valence_fragment_tokens(group_tokens, 1, 0)
}

fn append_attached_valence_group(
    group_components: &[AbbreviationComponent],
    group_nodes: &[ValenceNodeState],
    parent_index: usize,
    components: &mut Vec<AbbreviationComponent>,
    nodes: &mut Vec<ValenceNodeState>,
) -> bool {
    if group_components.is_empty()
        || group_nodes.is_empty()
        || nodes
            .get(parent_index)
            .is_none_or(|node| node.used >= node.valence)
    {
        return false;
    }
    let component_offset = components.len();
    let parent_component_index = nodes[parent_index].component_index;
    for (index, component) in group_components.iter().enumerate() {
        let mut component = component.clone();
        component.parent_index = if index == 0 {
            Some(parent_component_index)
        } else {
            component
                .parent_index
                .map(|parent| parent + component_offset)
        };
        if index == 0 {
            component.bond_order_to_parent = Some(1);
        }
        components.push(component);
    }
    nodes[parent_index].used += 1;
    nodes.extend(group_nodes.iter().cloned().map(|mut node| {
        node.component_index += component_offset;
        node
    }));
    true
}

fn nodes_are_satisfied(nodes: &[ValenceNodeState], allowed_root_remaining: u8) -> bool {
    nodes.iter().enumerate().all(|(index, node)| {
        let allowed_remaining = if index == 0 {
            allowed_root_remaining
        } else {
            0
        };
        node_valence_has_remaining(node, allowed_remaining)
    })
}

fn node_valence_has_remaining(node: &ValenceNodeState, allowed_remaining: u8) -> bool {
    if node.unconstrained_valence {
        return true;
    }
    let remaining = node.valence.saturating_sub(node.used);
    remaining == allowed_remaining
}

fn next_nodes_element(nodes: &[ValenceNodeState], index: usize) -> String {
    nodes
        .get(index)
        .map(|node| node.element.clone())
        .unwrap_or_default()
}

fn push_valence_atom_component(
    components: &mut Vec<AbbreviationComponent>,
    label: &str,
    element: &str,
    parent_index: Option<usize>,
    bond_order_to_parent: Option<u8>,
    formal_charge: Option<i8>,
) -> usize {
    let index = components.len();
    components.push(AbbreviationComponent {
        label: label.to_string(),
        kind: "atom".to_string(),
        name: element.to_string(),
        structure: element.to_string(),
        left_anchor: element.to_string(),
        right_attachment: Some(element.to_string()),
        parent_index,
        bond_order_to_parent,
        formal_charge,
    });
    index
}

fn tokenize_valence_label(label: &str) -> Option<Vec<ValenceToken>> {
    tokenize_valence_label_inner(label)
}

fn tokenize_valence_label_inner(label: &str) -> Option<Vec<ValenceToken>> {
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < label.len() {
        let rest = &label[index..];
        if rest.starts_with('(') {
            let close_offset = matching_close_paren(rest)?;
            let inner = &rest[1..close_offset];
            let after_close = index + close_offset + 1;
            let (count, digit_len) = parse_decimal_prefix(&label[after_close..]);
            let count = count.unwrap_or(1);
            if inner.is_empty() || count == 0 || count > 32 {
                return None;
            }
            let group_tokens = tokenize_valence_label_inner(inner)?;
            tokens.push(ValenceToken {
                label: label[index..after_close + digit_len].to_string(),
                kind: ValenceTokenKind::Group {
                    tokens: group_tokens,
                    count,
                },
            });
            index = after_close + digit_len;
            continue;
        }
        if rest.starts_with(')') {
            return None;
        }
        if let Some((fragment, matched)) = match_valence_terminal_prefix(rest) {
            tokens.push(ValenceToken {
                label: canonical_label_for(matched, fragment.label),
                kind: ValenceTokenKind::Terminal {
                    fragment,
                    matched: matched.to_string(),
                },
            });
            index += matched.len();
            continue;
        }
        let (element, consumed) = parse_element_prefix(rest)?;
        index += consumed;
        let (count, digit_len) = parse_decimal_prefix(&label[index..]);
        index += digit_len;
        let count = count.unwrap_or(1);
        if count == 0 || count > 32 {
            return None;
        }
        for _ in 0..count {
            tokens.push(ValenceToken {
                label: element.to_string(),
                kind: ValenceTokenKind::Atom {
                    element: element.to_string(),
                    from_numeric_count: digit_len > 0,
                },
            });
        }
    }
    Some(tokens)
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

fn parse_element_prefix(text: &str) -> Option<(&'static str, usize)> {
    SUPPORTED_VALENCE_ELEMENTS
        .iter()
        .find(|element| text.starts_with(**element))
        .map(|element| (*element, element.len()))
}

fn parse_decimal_prefix(text: &str) -> (Option<usize>, usize) {
    let len = text
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .map(char::len_utf8)
        .sum();
    if len == 0 {
        return (None, 0);
    }
    (text[..len].parse::<usize>().ok(), len)
}

fn match_valence_terminal_prefix(text: &str) -> Option<(FragmentDef, &str)> {
    TERMINAL_FRAGMENTS
        .iter()
        .copied()
        .filter(is_valence_terminal_fragment)
        .filter_map(|fragment| {
            std::iter::once(fragment.label)
                .chain(fragment.aliases.iter().copied())
                .filter(|candidate| text.starts_with(candidate))
                .max_by_key(|candidate| candidate.len())
                .map(|matched| (fragment, matched))
        })
        .max_by_key(|(_, matched)| matched.len())
}

fn is_valence_terminal_fragment(fragment: &FragmentDef) -> bool {
    fragment
        .label
        .chars()
        .any(|character| character.is_ascii_lowercase())
        || matches!(fragment.label, "R" | "TMS" | "TBDMS" | "TBDPS")
}

const SUPPORTED_VALENCE_ELEMENTS: &[&str] = &[
    "Cl", "Br", "Sc", "Ti", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn", "Zr", "Nb", "Mo", "Tc", "Ru",
    "Rh", "Pd", "Ag", "Cd", "Hf", "Ta", "Re", "Os", "Ir", "Pt", "Au", "Hg", "Si", "As", "Li", "Na",
    "Rb", "Cs", "Fr", "Be", "Mg", "Ca", "Sr", "Ba", "Ra", "H", "B", "C", "N", "O", "S", "P", "F",
    "I", "K", "V", "W", "Y",
];

fn is_unconstrained_valence_element(element: &str) -> bool {
    matches!(
        element,
        "Sc" | "Ti"
            | "V"
            | "Cr"
            | "Mn"
            | "Fe"
            | "Co"
            | "Ni"
            | "Cu"
            | "Zn"
            | "Y"
            | "Zr"
            | "Nb"
            | "Mo"
            | "Tc"
            | "Ru"
            | "Rh"
            | "Pd"
            | "Ag"
            | "Cd"
            | "Hf"
            | "Ta"
            | "W"
            | "Re"
            | "Os"
            | "Ir"
            | "Pt"
            | "Au"
            | "Hg"
    )
}

fn valence_options(
    tokens: &[ValenceToken],
    index: usize,
    already_used: u8,
) -> Vec<(u8, Option<i8>)> {
    let ValenceTokenKind::Atom { element, .. } = &tokens[index].kind else {
        return Vec::new();
    };
    let mut options: Vec<(u8, Option<i8>)> = match element.as_str() {
        _ if is_unconstrained_valence_element(element) => vec![(32, None)],
        "H" => vec![(1, None)],
        "Li" | "Na" | "K" | "Rb" | "Cs" | "Fr" => vec![(1, None)],
        "Be" | "Mg" | "Ca" | "Sr" | "Ba" | "Ra" => vec![(2, None)],
        "B" => {
            if following_hydrogen_count(tokens, index) >= 3 {
                vec![(4, Some(-1)), (3, None)]
            } else {
                vec![(3, None)]
            }
        }
        "C" | "Si" => vec![(4, None)],
        "N" => {
            if following_atoms_are_all_hydrogen(tokens, index)
                && following_hydrogen_count(tokens, index) >= 3
            {
                vec![(4, Some(1)), (3, None)]
            } else {
                vec![(3, None)]
            }
        }
        "O" => {
            if following_atoms_are_all_hydrogen(tokens, index)
                && following_hydrogen_count(tokens, index) >= 3
            {
                vec![(4, Some(2)), (3, Some(1)), (2, None)]
            } else if following_atoms_are_all_hydrogen(tokens, index)
                && following_hydrogen_count(tokens, index) >= 2
            {
                vec![(3, Some(1)), (2, None)]
            } else {
                vec![(2, None)]
            }
        }
        "S" => {
            if next_two_oxygen_tokens(tokens, index).is_some_and(|numeric| numeric) {
                vec![(6, None), (4, None), (2, None)]
            } else if next_two_oxygen_tokens(tokens, index).is_some() {
                vec![(4, None), (6, None), (2, None)]
            } else {
                vec![(2, None), (4, None), (6, None)]
            }
        }
        "P" | "As" => vec![(5, None), (3, None)],
        "F" | "Cl" | "Br" | "I" => vec![(1, None), (3, None), (5, None), (7, None)],
        _ => Vec::new(),
    };
    options.retain(|(valence, _)| *valence >= already_used);
    options
}

fn following_hydrogen_count(tokens: &[ValenceToken], index: usize) -> usize {
    tokens
        .iter()
        .skip(index + 1)
        .take_while(
            |token| matches!(&token.kind, ValenceTokenKind::Atom { element, .. } if element == "H"),
        )
        .count()
}

fn following_atoms_are_all_hydrogen(tokens: &[ValenceToken], index: usize) -> bool {
    tokens.iter().skip(index + 1).all(
        |token| matches!(&token.kind, ValenceTokenKind::Atom { element, .. } if element == "H"),
    )
}

fn next_two_oxygen_tokens(tokens: &[ValenceToken], index: usize) -> Option<bool> {
    let first = tokens.get(index + 1)?;
    let second = tokens.get(index + 2)?;
    let first_is_oxygen =
        matches!(&first.kind, ValenceTokenKind::Atom { element, .. } if element == "O");
    if !first_is_oxygen {
        return None;
    }
    match &second.kind {
        ValenceTokenKind::Atom {
            element,
            from_numeric_count,
        } if element == "O" => Some(*from_numeric_count),
        _ => None,
    }
}

fn bond_order_candidates(
    parent_element: &str,
    child_element: &str,
    parent_remaining: u8,
    child_valence: u8,
) -> Vec<u8> {
    let max_order = parent_remaining.min(child_valence).min(3);
    if max_order == 0 {
        return Vec::new();
    }
    if child_element == "H"
        || matches!(child_element, "F" | "Cl" | "Br" | "I")
        || matches!(child_element, "Li" | "Na" | "K" | "Rb" | "Cs" | "Fr")
    {
        return vec![1];
    }
    if parent_element == "C" && matches!(child_element, "N") && max_order >= 3 {
        return vec![3, 2, 1];
    }
    if parent_element == "C" && matches!(child_element, "O" | "S") && max_order >= 2 {
        return vec![2, 1];
    }
    if matches!(parent_element, "S" | "P" | "As") && child_element == "O" && max_order >= 2 {
        return vec![2, 1];
    }
    vec![1]
}

fn valence_formula(components: &[AbbreviationComponent]) -> String {
    let Some(root_index) = components
        .iter()
        .position(|component| component.parent_index.is_none())
    else {
        return String::new();
    };
    let mut children = vec![Vec::<usize>::new(); components.len()];
    for (index, component) in components.iter().enumerate() {
        if let Some(parent_index) = component.parent_index {
            if let Some(parent_children) = children.get_mut(parent_index) {
                parent_children.push(index);
            }
        }
    }
    format!(
        "-{}",
        render_valence_formula_component(root_index, components, &children)
    )
}

fn render_valence_formula_component(
    index: usize,
    components: &[AbbreviationComponent],
    children: &[Vec<usize>],
) -> String {
    let component = &components[index];
    if component.kind == "terminal" {
        return component.label.clone();
    }
    if component.label == "H" {
        return "H".to_string();
    }
    let mut out = component.label.clone();
    let hydrogen_count = children[index]
        .iter()
        .filter(|child| components[**child].label == "H")
        .count();
    if hydrogen_count == 1 {
        out.push('H');
    } else if hydrogen_count > 1 {
        out.push_str(&format!("H{hydrogen_count}"));
    }
    for child in children[index]
        .iter()
        .copied()
        .filter(|child| components[*child].label != "H")
    {
        let rendered = render_valence_formula_component(child, components, children);
        match components[child].bond_order_to_parent.unwrap_or(1) {
            3 => {
                out.push('#');
                out.push_str(&rendered);
            }
            2 => {
                out.push_str("(=");
                out.push_str(&rendered);
                out.push(')');
            }
            _ => out.push_str(&rendered),
        }
    }
    out
}
