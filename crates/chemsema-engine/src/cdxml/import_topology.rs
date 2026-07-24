use super::*;

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
    topology_only_cdxmlwriter: bool,
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
    if !topology_only_cdxmlwriter {
        return Err(format!(
            "CDXML fragment '{}' has nodes but no authoritative p coordinates",
            fragment.attr("id").unwrap_or("<unnamed>")
        ));
    }
    Ok(layout_topology_only_cdxmlwriter_fragment(
        &nodes.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>(),
        &bonds,
        bond_length.max(1.0),
    ))
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

pub(super) fn layout_topology_only_cdxmlwriter_fragment(
    node_ids: &[String],
    edges: &[(String, String)],
    bond_length: f64,
) -> BTreeMap<String, [f64; 2]> {
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
            topology_cycle_order(&component, &adjacency)
        } else {
            component.clone()
        };

        let local = if is_path {
            let dx = bond_length * (std::f64::consts::PI / 6.0).cos();
            let dy = bond_length * 0.5;
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
            let start_angle = if count == 4 || count % 2 == 1 {
                -std::f64::consts::FRAC_PI_2 - std::f64::consts::PI / count as f64
            } else {
                -std::f64::consts::FRAC_PI_2
            };
            ordered
                .iter()
                .enumerate()
                .map(|(index, id)| {
                    let angle = start_angle + std::f64::consts::TAU * index as f64 / count as f64;
                    (*id, [radius * angle.cos(), radius * angle.sin()])
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
    positions
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

pub(super) fn cdxml_node_owns_embedded_fragment(node: &XmlNode) -> bool {
    // A fragment nested directly inside a node is that node's connection table
    // or expansion definition.  It is not an independently displayed fragment.
    // The rule is structural: CDXML permits these children on more node kinds
    // than the common Fragment/Nickname cases (notably alternative groups).
    node.is("n") && node.direct_children("fragment").next().is_some()
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
