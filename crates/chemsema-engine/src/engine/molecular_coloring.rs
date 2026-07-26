use super::*;

impl Engine {
    pub fn apply_molecular_highlight(&mut self, color: Option<&str>) -> bool {
        let color = match color {
            Some(value) => match normalize_molecular_color(value) {
                Some(color) => Some(color),
                None => return false,
            },
            None => None,
        };
        self.with_command(
            EditorCommand::ApplyMolecularHighlight {
                color: color.clone(),
            },
            |engine| engine.apply_molecular_highlight_untracked(color.as_deref()),
        )
    }

    fn apply_molecular_highlight_untracked(&mut self, color: Option<&str>) -> bool {
        let selected_nodes = self
            .state
            .selection
            .nodes
            .iter()
            .chain(self.state.selection.label_nodes.iter())
            .cloned()
            .collect::<BTreeSet<_>>();
        let selected_bonds = self
            .state
            .selection
            .bonds
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let selected_objects = self
            .state
            .selection
            .molecule_objects
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if selected_nodes.is_empty() && selected_bonds.is_empty() && selected_objects.is_empty() {
            return false;
        }

        self.push_undo_snapshot();
        let mut changed = false;
        let object_ids = self
            .state
            .document
            .editable_fragments()
            .into_iter()
            .map(|entry| entry.object.id.clone())
            .collect::<Vec<_>>();
        for object_id in object_ids {
            let Some(entry) = self
                .state
                .document
                .editable_fragment_mut_for_object(&object_id)
            else {
                continue;
            };
            let whole_object = selected_objects.contains(&entry.object.id);
            for node in &mut entry.fragment.nodes {
                if whole_object || selected_nodes.contains(&node.id) {
                    changed |= set_optional_color(&mut node.highlight_color, color);
                }
            }
            for bond in &mut entry.fragment.bonds {
                if whole_object || selected_bonds.contains(&bond.id) {
                    changed |= set_optional_color(&mut bond.highlight_color, color);
                }
            }
        }
        if !changed {
            self.undo_stack.pop();
        }
        changed
    }

    pub fn apply_ring_fill(&mut self, color: Option<&str>) -> bool {
        let color = match color {
            Some(value) => match normalize_molecular_color(value) {
                Some(color) => Some(color),
                None => return false,
            },
            None => None,
        };
        self.with_command(
            EditorCommand::ApplyRingFill {
                color: color.clone(),
            },
            |engine| engine.apply_ring_fill_untracked(color.as_deref()),
        )
    }

    fn apply_ring_fill_untracked(&mut self, color: Option<&str>) -> bool {
        let selected_bonds = self
            .state
            .selection
            .bonds
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let selected_objects = self
            .state
            .selection
            .molecule_objects
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if selected_bonds.is_empty() && selected_objects.is_empty() {
            return false;
        }

        let plans = self
            .state
            .document
            .editable_fragments()
            .into_iter()
            .filter_map(|entry| {
                let basis = if selected_objects.contains(&entry.object.id) {
                    entry
                        .fragment
                        .bonds
                        .iter()
                        .map(|bond| bond.id.clone())
                        .collect()
                } else {
                    selected_bonds.clone()
                };
                let cycles = chordless_selected_ring_cycles(entry.fragment, &basis);
                (!cycles.is_empty()).then(|| (entry.object.id.clone(), cycles))
            })
            .collect::<Vec<_>>();
        if plans.is_empty() {
            return false;
        }

        self.push_undo_snapshot();
        let mut changed = false;
        for (object_id, cycles) in plans {
            let Some(entry) = self
                .state
                .document
                .editable_fragment_mut_for_object(&object_id)
            else {
                continue;
            };
            for cycle in cycles {
                if let Some(color) = color {
                    if let Some(area) = entry
                        .fragment
                        .colored_areas
                        .iter_mut()
                        .find(|area| same_bond_set(&area.basis_bonds, &cycle))
                    {
                        if area.color != color {
                            area.color = color.to_string();
                            changed = true;
                        }
                    } else {
                        let id = next_colored_area_id(entry.fragment);
                        entry
                            .fragment
                            .colored_areas
                            .push(crate::ColoredMolecularArea {
                                id,
                                color: color.to_string(),
                                basis_bonds: cycle,
                            });
                        changed = true;
                    }
                } else {
                    let before = entry.fragment.colored_areas.len();
                    entry
                        .fragment
                        .colored_areas
                        .retain(|area| !same_bond_set(&area.basis_bonds, &cycle));
                    changed |= entry.fragment.colored_areas.len() != before;
                }
            }
        }
        if !changed {
            self.undo_stack.pop();
        }
        changed
    }

    pub(crate) fn selected_ring_cycles(&self) -> Vec<Vec<String>> {
        let selected_bonds = self
            .state
            .selection
            .bonds
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let selected_objects = self
            .state
            .selection
            .molecule_objects
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        self.state
            .document
            .editable_fragments()
            .into_iter()
            .flat_map(|entry| {
                let basis = if selected_objects.contains(&entry.object.id) {
                    entry
                        .fragment
                        .bonds
                        .iter()
                        .map(|bond| bond.id.clone())
                        .collect()
                } else {
                    selected_bonds.clone()
                };
                chordless_selected_ring_cycles(entry.fragment, &basis)
            })
            .collect()
    }
}

fn normalize_molecular_color(color: &str) -> Option<String> {
    let value = color.trim();
    if value.len() == 7
        && value.starts_with('#')
        && value[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Some(value.to_ascii_lowercase())
    } else {
        None
    }
}

fn set_optional_color(target: &mut Option<String>, color: Option<&str>) -> bool {
    if target.as_deref() == color {
        return false;
    }
    *target = color.map(str::to_string);
    true
}

fn same_bond_set(left: &[String], right: &[String]) -> bool {
    left.len() == right.len()
        && left.iter().collect::<BTreeSet<_>>() == right.iter().collect::<BTreeSet<_>>()
}

fn next_colored_area_id(fragment: &crate::MoleculeFragment) -> String {
    let mut index = 1usize;
    loop {
        let candidate = format!("colored_area_{index}");
        if fragment
            .colored_areas
            .iter()
            .all(|area| area.id != candidate)
        {
            return candidate;
        }
        index += 1;
    }
}

pub(crate) fn chordless_selected_ring_cycles(
    fragment: &crate::MoleculeFragment,
    selected_bonds: &BTreeSet<String>,
) -> Vec<Vec<String>> {
    let bonds = fragment
        .bonds
        .iter()
        .filter(|bond| selected_bonds.contains(&bond.id))
        .collect::<Vec<_>>();
    let mut adjacency = BTreeMap::<&str, Vec<(&str, &str)>>::new();
    for bond in &bonds {
        adjacency
            .entry(bond.begin.as_str())
            .or_default()
            .push((bond.end.as_str(), bond.id.as_str()));
        adjacency
            .entry(bond.end.as_str())
            .or_default()
            .push((bond.begin.as_str(), bond.id.as_str()));
    }
    for neighbors in adjacency.values_mut() {
        neighbors.sort_unstable();
    }

    let mut cycles = BTreeSet::<Vec<String>>::new();
    for &start in adjacency.keys() {
        let mut path_nodes = vec![start];
        let mut path_bonds = Vec::<&str>::new();
        enumerate_cycles(
            start,
            start,
            &adjacency,
            &mut path_nodes,
            &mut path_bonds,
            &mut cycles,
        );
    }
    cycles
        .into_iter()
        .filter(|cycle| {
            crate::ordered_colored_area_node_ids(fragment, cycle).is_some()
                && is_chordless_cycle(fragment, cycle)
        })
        .collect()
}

fn enumerate_cycles<'a>(
    start: &'a str,
    current: &'a str,
    adjacency: &BTreeMap<&'a str, Vec<(&'a str, &'a str)>>,
    path_nodes: &mut Vec<&'a str>,
    path_bonds: &mut Vec<&'a str>,
    cycles: &mut BTreeSet<Vec<String>>,
) {
    let Some(neighbors) = adjacency.get(current) else {
        return;
    };
    for &(next, bond_id) in neighbors {
        if next < start {
            continue;
        }
        if next == start {
            if path_nodes.len() >= 3 {
                let mut cycle = path_bonds
                    .iter()
                    .copied()
                    .chain(std::iter::once(bond_id))
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                cycle.sort();
                cycles.insert(cycle);
            }
            continue;
        }
        if path_nodes.contains(&next) {
            continue;
        }
        path_nodes.push(next);
        path_bonds.push(bond_id);
        enumerate_cycles(start, next, adjacency, path_nodes, path_bonds, cycles);
        path_bonds.pop();
        path_nodes.pop();
    }
}

fn is_chordless_cycle(fragment: &crate::MoleculeFragment, cycle: &[String]) -> bool {
    let Some(nodes) = crate::ordered_colored_area_node_ids(fragment, cycle) else {
        return false;
    };
    let node_set = nodes.iter().map(String::as_str).collect::<BTreeSet<_>>();
    fragment
        .bonds
        .iter()
        .filter(|bond| {
            node_set.contains(bond.begin.as_str()) && node_set.contains(bond.end.as_str())
        })
        .count()
        == cycle.len()
}
