use super::*;

#[derive(Clone)]
struct CollapsedWrapperLayoutEntry {
    fragment_id: String,
    node_id: String,
    anchor: Option<[f64; 2]>,
}

pub(super) fn cdxml_collapsed_wrapper_position_overrides(
    root: &XmlNode,
    bond_length: f64,
) -> Result<BTreeMap<(String, String), [f64; 2]>, String> {
    let pages = descendants(root)
        .into_iter()
        .filter(|node| node.is("page"))
        .collect::<Vec<_>>();
    let scopes = if pages.is_empty() { vec![root] } else { pages };
    let mut overrides = BTreeMap::new();
    for scope in scopes {
        let fragments = display_fragments(scope);
        let mut entries = Vec::new();
        for fragment in fragments {
            let Some(fragment_id) = fragment.attr("id") else {
                continue;
            };
            let direct_nodes = fragment.direct_children("n").collect::<Vec<_>>();
            let direct_bonds = fragment.direct_children("b").collect::<Vec<_>>();
            for node in direct_nodes.iter().copied().filter(|node| {
                node.attr("NodeType") == Some("Fragment") && node.attr("p").is_none()
            }) {
                let Some(node_id) = node.attr("id") else {
                    continue;
                };
                if direct_nodes.len() == 1 && direct_bonds.is_empty() {
                    entries.push(CollapsedWrapperLayoutEntry {
                        fragment_id: fragment_id.to_string(),
                        node_id: node_id.to_string(),
                        anchor: None,
                    });
                    continue;
                }
                let parent_anchor = direct_bonds.iter().find_map(|bond| {
                    let begin = bond.attr("B")?;
                    let end = bond.attr("E")?;
                    let (neighbor_id, direction) = if begin == node_id {
                        (end, -1.0)
                    } else if end == node_id {
                        (begin, 1.0)
                    } else {
                        return None;
                    };
                    let neighbor = direct_nodes
                        .iter()
                        .copied()
                        .find(|candidate| candidate.attr("id") == Some(neighbor_id))?;
                    let neighbor_position = parse_xy(neighbor.attr("p"))?;
                    Some([
                        round2(neighbor_position[0] + direction * bond_length),
                        round2(neighbor_position[1]),
                    ])
                });
                if let Some(anchor) = parent_anchor {
                    entries.push(CollapsedWrapperLayoutEntry {
                        fragment_id: fragment_id.to_string(),
                        node_id: node_id.to_string(),
                        anchor: Some(anchor),
                    });
                }
            }
        }

        let missing = entries
            .iter()
            .filter(|entry| entry.anchor.is_none())
            .collect::<Vec<_>>();
        if missing.is_empty() {
            continue;
        }
        let anchors = entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| entry.anchor.map(|point| (index, entry, point)))
            .collect::<Vec<_>>();
        let has_single_anchor = anchors.len() == 1 && entries.len() <= 8;
        let normalized = if anchors.is_empty() {
            chemdraw_collapsed_wrapper_grid(missing.len())?
        } else if has_single_anchor {
            chemdraw_collapsed_wrapper_grid_with_anchor(entries.len())?
        } else {
            // A page with several independently anchored collapsed fragments is
            // not one automatic layout cluster. Only the truly coordinate-free
            // singleton fragments participate in this origin-based cluster.
            chemdraw_collapsed_wrapper_grid(missing.len())?
        };

        if has_single_anchor {
            let (anchor_index, anchor_entry, measured_anchor_position) = anchors[0];
            let anchor_is_last = anchor_index + 1 == entries.len();
            let (origin, anchor_position) = if anchor_is_last {
                ([0.0, 0.0], [0.0, 0.0])
            } else {
                (
                    [
                        round2(measured_anchor_position[0] - bond_length),
                        round2(measured_anchor_position[1]),
                    ],
                    measured_anchor_position,
                )
            };
            overrides.insert(
                (
                    anchor_entry.fragment_id.clone(),
                    anchor_entry.node_id.clone(),
                ),
                anchor_position,
            );
            for (entry, point) in missing.iter().zip(normalized) {
                overrides.insert(
                    (entry.fragment_id.clone(), entry.node_id.clone()),
                    [
                        round2(origin[0] + point[0] * bond_length),
                        round2(origin[1] + point[1] * bond_length),
                    ],
                );
            }
        } else {
            for (entry, point) in missing.iter().zip(normalized) {
                overrides.insert(
                    (entry.fragment_id.clone(), entry.node_id.clone()),
                    [
                        round2(point[0] * bond_length),
                        round2(point[1] * bond_length),
                    ],
                );
            }
        }
    }
    Ok(overrides)
}

fn chemdraw_collapsed_wrapper_grid(count: usize) -> Result<Vec<[f64; 2]>, String> {
    let points: &[[f64; 2]] = match count {
        1 => &[[0.0, 0.0]],
        2 => &[[0.0, 0.0], [1.0, 0.0]],
        3 => &[[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]],
        4 => &[[0.0, 0.0], [0.0, 1.0], [0.0, 2.0], [1.0, 1.5]],
        5 => &[[0.0, 0.0], [0.0, 1.0], [0.0, 2.0], [0.0, 3.0], [1.0, 2.0]],
        6 => &[
            [0.0, 0.0],
            [0.0, 1.0],
            [0.0, 2.0],
            [0.0, 3.0],
            [1.0, 1.5],
            [1.0, 2.5],
        ],
        7 => &[
            [0.0, 0.0],
            [0.0, 1.0],
            [0.0, 2.0],
            [0.0, 3.0],
            [1.0, 1.0],
            [1.0, 2.0],
            [1.0, 3.0],
        ],
        8 => &[
            [0.0, 0.0],
            [0.0, 1.0],
            [0.0, 2.0],
            [0.0, 3.0],
            [1.0, 0.5],
            [1.0, 1.5],
            [1.0, 2.5],
            [1.0, 3.5],
        ],
        _ => {
            return Err(format!(
                "ChemDraw collapsed-fragment automatic layout has been verified for 1-8 coordinate-free wrappers, not {count}"
            ));
        }
    };
    Ok(points.to_vec())
}

fn chemdraw_collapsed_wrapper_grid_with_anchor(
    total_count: usize,
) -> Result<Vec<[f64; 2]>, String> {
    let points: &[[f64; 2]] = match total_count {
        2 => &[[2.0, 0.0]],
        3 => &[[0.0, 1.0], [1.0, 1.0]],
        4 => &[[0.0, 1.0], [1.0, 1.0], [2.0, 0.5]],
        5 => &[[0.0, 1.0], [0.0, 2.0], [1.0, 1.0], [1.0, 2.0]],
        6 => &[[0.0, 1.0], [0.0, 2.0], [0.0, 3.0], [1.0, 1.5], [1.0, 2.5]],
        7 => &[
            [0.0, 1.0],
            [0.0, 2.0],
            [0.0, 3.0],
            [1.0, 1.0],
            [1.0, 2.0],
            [1.0, 3.0],
        ],
        8 => &[
            [0.0, 1.5],
            [0.0, 2.5],
            [0.0, 3.5],
            [1.0, 1.0],
            [1.0, 2.0],
            [1.0, 3.0],
            [1.0, 4.0],
        ],
        _ => {
            return Err(format!(
                "ChemDraw anchored collapsed-fragment automatic layout has been verified for 3-8 wrappers, not {total_count}"
            ));
        }
    };
    Ok(points.to_vec())
}

pub(super) fn cdxml_fragment_bbox(
    fragment: &XmlNode,
    bond_length: f64,
    node_positions: &BTreeMap<String, [f64; 2]>,
) -> Option<[f64; 4]> {
    if let Some(bbox) = parse_bbox(fragment.attr("BoundingBox")) {
        return Some(bbox);
    }

    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut found = false;
    let mut include = |point: [f64; 2]| {
        found = true;
        bounds[0] = bounds[0].min(point[0]);
        bounds[1] = bounds[1].min(point[1]);
        bounds[2] = bounds[2].max(point[0]);
        bounds[3] = bounds[3].max(point[1]);
    };
    for node in fragment.direct_children("n") {
        if let Some(point) = node
            .attr("id")
            .and_then(|id| node_positions.get(id))
            .copied()
        {
            include(point);
        }
        for text in node.direct_children("t") {
            if let Some(bbox) = parse_bbox(text.attr("BoundingBox")) {
                include([bbox[0], bbox[1]]);
                include([bbox[2], bbox[3]]);
            }
        }
    }
    if !found {
        return None;
    }
    let half_padding = bond_length.max(1.0) * 0.5;
    if (bounds[2] - bounds[0]).abs() <= EPSILON {
        bounds[0] -= half_padding;
        bounds[2] += half_padding;
    }
    if (bounds[3] - bounds[1]).abs() <= EPSILON {
        bounds[1] -= half_padding;
        bounds[3] += half_padding;
    }
    Some(bounds.map(round2))
}

pub(super) fn cdxml_fragment_node_positions(
    fragment: &XmlNode,
    bond_length: f64,
) -> Result<BTreeMap<String, [f64; 2]>, String> {
    let nodes: Vec<_> = fragment
        .direct_children("n")
        .filter_map(|node| node.attr("id").map(|id| (id.to_string(), node)))
        .collect();
    let mut explicit: BTreeMap<_, _> = nodes
        .iter()
        .filter_map(|(id, node)| parse_xy(node.attr("p")).map(|point| (id.clone(), point)))
        .collect();
    for (id, node) in &nodes {
        if explicit.contains_key(id) {
            continue;
        }
        if let Some(point) = cdxml_embedded_fragment_connection_position(node, bond_length) {
            explicit.insert(id.clone(), point);
        }
    }
    let bonds: Vec<_> = fragment
        .direct_children("b")
        .filter_map(|bond| Some((bond.attr("B")?.to_string(), bond.attr("E")?.to_string())))
        .collect();
    if !explicit.is_empty() || nodes.is_empty() {
        return Ok(explicit);
    }
    layout_coordinate_free_cdxml_fragment(
        &nodes.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>(),
        &bonds,
        bond_length.max(1.0),
        fragment.attr("id").unwrap_or("<unnamed>"),
    )
}

pub(super) fn cdxml_embedded_fragment_connection_position(
    node: &XmlNode,
    bond_length: f64,
) -> Option<[f64; 2]> {
    let fragment = node.direct_children("fragment").next()?;
    let nested_nodes: BTreeMap<_, _> = fragment
        .direct_children("n")
        .filter_map(|child| child.attr("id").map(|id| (id, child)))
        .collect();
    let bonds: Vec<_> = fragment
        .direct_children("b")
        .filter_map(|bond| Some((bond.attr("B")?, bond.attr("E")?)))
        .collect();

    if let Some((external_id, external)) = nested_nodes
        .iter()
        .find(|(_, child)| child.attr("NodeType") == Some("ExternalConnectionPoint"))
    {
        if let Some(point) = parse_xy(external.attr("p")) {
            return Some(point);
        }
        let anchor_id = bonds.iter().find_map(|(begin, end)| {
            if begin == external_id {
                Some(*end)
            } else if end == external_id {
                Some(*begin)
            } else {
                None
            }
        })?;
        let anchor = nested_nodes.get(anchor_id)?;
        let anchor_point = parse_xy(anchor.attr("p"))?;
        let preceding_point = bonds.iter().find_map(|(begin, end)| {
            let other_id = if begin == &anchor_id && end != external_id {
                Some(*end)
            } else if end == &anchor_id && begin != external_id {
                Some(*begin)
            } else {
                None
            }?;
            nested_nodes
                .get(other_id)
                .and_then(|other| parse_xy(other.attr("p")))
        })?;
        let dx = anchor_point[0] - preceding_point[0];
        let dy = anchor_point[1] - preceding_point[1];
        let length = dx.hypot(dy);
        if length <= EPSILON {
            return None;
        }
        let scale = bond_length.max(1.0) / length;
        return Some([
            round2(anchor_point[0] + dx * scale),
            round2(anchor_point[1] + dy * scale),
        ]);
    }
    None
}

pub(super) fn layout_coordinate_free_cdxml_fragment(
    node_ids: &[String],
    edges: &[(String, String)],
    bond_length: f64,
    fragment_id: &str,
) -> Result<BTreeMap<String, [f64; 2]>, String> {
    let node_order: BTreeMap<_, _> = node_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect();
    let mut adjacency: BTreeMap<&str, Vec<&str>> = node_ids
        .iter()
        .map(|id| (id.as_str(), Vec::new()))
        .collect();
    for (begin, end) in edges {
        if adjacency.contains_key(begin.as_str()) && adjacency.contains_key(end.as_str()) {
            adjacency
                .get_mut(begin.as_str())
                .unwrap()
                .push(end.as_str());
            adjacency
                .get_mut(end.as_str())
                .unwrap()
                .push(begin.as_str());
        }
    }
    for neighbors in adjacency.values_mut() {
        neighbors.sort_by_key(|id| node_order.get(id).copied().unwrap_or(usize::MAX));
        neighbors.dedup();
    }

    let mut components = Vec::new();
    let mut visited = BTreeSet::new();
    for id in node_ids {
        if visited.contains(id.as_str()) {
            continue;
        }
        let mut component = Vec::new();
        let mut queue = VecDeque::from([id.as_str()]);
        while let Some(current) = queue.pop_front() {
            if !visited.insert(current) {
                continue;
            }
            component.push(current);
            if let Some(neighbors) = adjacency.get(current) {
                queue.extend(neighbors.iter().copied());
            }
        }
        component.sort_by_key(|id| node_order.get(id).copied().unwrap_or(usize::MAX));
        components.push(component);
    }

    let mut positions = BTreeMap::new();
    let mut component_x = 0.0;
    for component in components {
        let component_set: BTreeSet<_> = component.iter().copied().collect();
        let edge_count = component
            .iter()
            .map(|id| {
                adjacency
                    .get(id)
                    .into_iter()
                    .flatten()
                    .filter(|neighbor| component_set.contains(**neighbor))
                    .count()
            })
            .sum::<usize>()
            / 2;
        let is_path = component.len() <= 2
            || (edge_count + 1 == component.len()
                && component.iter().all(|id| {
                    adjacency
                        .get(id)
                        .is_none_or(|neighbors| neighbors.len() <= 2)
                }));
        let is_cycle = component.len() >= 3
            && edge_count == component.len()
            && component.iter().all(|id| {
                adjacency
                    .get(id)
                    .is_some_and(|neighbors| neighbors.len() == 2)
            });
        let ordered = if is_path {
            topology_path_order(&component, &adjacency)
        } else if is_cycle {
            if component.len() > 8 {
                return Err(format!(
                    "CDXML coordinate-free fragment '{fragment_id}' contains a {}-member macrocycle; ChemDraw macrocycle layout is not the regular-ring rule used for 3-8 member rings",
                    component.len()
                ));
            }
            topology_cycle_order(&component, &adjacency)
        } else {
            return Err(format!(
                "CDXML coordinate-free fragment '{fragment_id}' has branching or fused topology; no ChemDraw-compatible layout rule is selected"
            ));
        };

        let local = if is_path {
            let dx = round2(bond_length * (std::f64::consts::PI / 6.0).cos());
            let dy = round2(bond_length * 0.5);
            ordered
                .iter()
                .enumerate()
                .map(|(index, id)| {
                    (
                        *id,
                        [index as f64 * dx, if index % 2 == 0 { 0.0 } else { dy }],
                    )
                })
                .collect::<Vec<_>>()
        } else {
            let count = ordered.len().max(3);
            let radius = bond_length / (2.0 * (std::f64::consts::PI / count as f64).sin());
            let start_angle = if count % 2 == 0 {
                std::f64::consts::PI / count as f64
            } else if count % 4 == 1 {
                -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI / count as f64
            } else {
                std::f64::consts::FRAC_PI_2 + std::f64::consts::PI / count as f64
            };
            ordered
                .iter()
                .enumerate()
                .map(|(index, id)| {
                    let angle = start_angle + std::f64::consts::TAU * index as f64 / count as f64;
                    (
                        *id,
                        [round2(radius * angle.cos()), round2(radius * angle.sin())],
                    )
                })
                .collect::<Vec<_>>()
        };
        let min_x = local
            .iter()
            .map(|(_, point)| point[0])
            .fold(f64::INFINITY, f64::min);
        let max_x = local
            .iter()
            .map(|(_, point)| point[0])
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = local
            .iter()
            .map(|(_, point)| point[1])
            .fold(f64::INFINITY, f64::min);
        for (id, point) in local {
            positions.insert(
                id.to_string(),
                [
                    round2(component_x + point[0] - min_x),
                    round2(point[1] - min_y),
                ],
            );
        }
        component_x += (max_x - min_x).max(bond_length) + bond_length;
    }
    Ok(positions)
}

pub(super) fn topology_path_order<'a>(
    component: &[&'a str],
    adjacency: &BTreeMap<&'a str, Vec<&'a str>>,
) -> Vec<&'a str> {
    let start = component
        .iter()
        .copied()
        .find(|id| {
            adjacency
                .get(id)
                .is_none_or(|neighbors| neighbors.len() <= 1)
        })
        .unwrap_or(component[0]);
    topology_walk_order(start, component.len(), adjacency, false)
}

pub(super) fn topology_cycle_order<'a>(
    component: &[&'a str],
    adjacency: &BTreeMap<&'a str, Vec<&'a str>>,
) -> Vec<&'a str> {
    topology_walk_order(component[0], component.len(), adjacency, true)
}

pub(super) fn topology_walk_order<'a>(
    start: &'a str,
    expected: usize,
    adjacency: &BTreeMap<&'a str, Vec<&'a str>>,
    allow_cycle_close: bool,
) -> Vec<&'a str> {
    let mut ordered = Vec::with_capacity(expected);
    let mut previous = None;
    let mut current = start;
    while ordered.len() < expected {
        ordered.push(current);
        let next = adjacency
            .get(current)
            .into_iter()
            .flatten()
            .copied()
            .find(|neighbor| {
                Some(*neighbor) != previous
                    && (!ordered.contains(neighbor) || (allow_cycle_close && *neighbor == start))
            });
        let Some(next) = next else {
            break;
        };
        if next == start {
            break;
        }
        previous = Some(current);
        current = next;
    }
    ordered
}

pub(super) fn cdxml_bonded_node_ids(root: &XmlNode) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for bond in descendants(root).into_iter().filter(|node| node.is("b")) {
        if let Some(begin) = bond.attr("B") {
            ids.insert(begin.to_string());
        }
        if let Some(end) = bond.attr("E") {
            ids.insert(end.to_string());
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_free_cycles_three_through_eight_match_chemdraw_orientation() {
        let expected = [
            vec![[0.0, 14.72], [8.5, 0.0], [17.0, 14.72]],
            vec![[17.0, 17.0], [0.0, 17.0], [0.0, 0.0], [17.0, 0.0]],
            vec![
                [22.25, 0.0],
                [27.5, 16.17],
                [13.75, 26.16],
                [0.0, 16.17],
                [5.25, 0.0],
            ],
            vec![
                [29.44, 25.5],
                [14.72, 34.0],
                [0.0, 25.5],
                [0.0, 8.5],
                [14.72, 0.0],
                [29.44, 8.5],
            ],
            vec![
                [10.6, 37.24],
                [0.0, 23.95],
                [3.78, 7.38],
                [19.1, 0.0],
                [34.42, 7.38],
                [38.2, 23.95],
                [27.6, 37.24],
            ],
            vec![
                [41.04, 29.02],
                [29.02, 41.04],
                [12.02, 41.04],
                [0.0, 29.02],
                [0.0, 12.02],
                [12.02, 0.0],
                [29.02, 0.0],
                [41.04, 12.02],
            ],
        ];

        for (offset, expected_positions) in expected.iter().enumerate() {
            let count = offset + 3;
            let node_ids = (1..=count).map(|id| id.to_string()).collect::<Vec<_>>();
            let edges = (1..=count)
                .map(|id| (id.to_string(), (id % count + 1).to_string()))
                .collect::<Vec<_>>();
            let positions = layout_coordinate_free_cdxml_fragment(&node_ids, &edges, 17.0, "probe")
                .expect("simple cycle should have a verified layout");
            assert_eq!(
                node_ids.iter().map(|id| positions[id]).collect::<Vec<_>>(),
                *expected_positions,
                "{count}-member ring"
            );
        }
    }
}
