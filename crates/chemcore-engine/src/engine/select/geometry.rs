use super::*;

pub(super) fn add_hit_to_selection(selection: &mut SelectionState, hit: SelectHit) {
    match hit {
        SelectHit::TextObject { object_id } => push_unique(&mut selection.text_objects, object_id),
        SelectHit::ArrowObject { object_id } => {
            push_unique(&mut selection.arrow_objects, object_id)
        }
        SelectHit::Label { node_id } => push_unique(&mut selection.label_nodes, node_id),
        SelectHit::Node { node_id } => push_unique(&mut selection.nodes, node_id),
        SelectHit::Bond { bond_id } => push_unique(&mut selection.bonds, bond_id),
    }
}

pub(super) fn selection_contains_hit(selection: &SelectionState, hit: &SelectHit) -> bool {
    match hit {
        SelectHit::TextObject { object_id } => selection.text_objects.contains(object_id),
        SelectHit::ArrowObject { object_id } => selection.arrow_objects.contains(object_id),
        SelectHit::Label { node_id } => selection.label_nodes.contains(node_id),
        SelectHit::Node { node_id } => selection.nodes.contains(node_id),
        SelectHit::Bond { bond_id } => selection.bonds.contains(bond_id),
    }
}

pub(super) fn merge_selection(
    current: SelectionState,
    next: SelectionState,
    additive: bool,
) -> SelectionState {
    if !additive {
        return next;
    }
    let mut merged = current;
    merged.region = merged.region || next.region;
    for object_id in next.text_objects {
        push_unique(&mut merged.text_objects, object_id);
    }
    for object_id in next.arrow_objects {
        push_unique(&mut merged.arrow_objects, object_id);
    }
    for node_id in next.label_nodes {
        push_unique(&mut merged.label_nodes, node_id);
    }
    for node_id in next.nodes {
        push_unique(&mut merged.nodes, node_id);
    }
    for bond_id in next.bonds {
        push_unique(&mut merged.bonds, bond_id);
    }
    merged
}

pub(super) fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

pub(super) fn push_selection_box(
    out: &mut Vec<RenderPrimitive>,
    bounds: AxisBounds,
    role: RenderRole,
) {
    out.push(RenderPrimitive::Rect {
        role,
        object_id: None,
        node_id: None,
        x: bounds.min_x,
        y: bounds.min_y,
        width: (bounds.max_x - bounds.min_x).max(0.0),
        height: (bounds.max_y - bounds.min_y).max(0.0),
        fill: None,
        stroke: Some("rgba(47,111,237,0.86)".to_string()),
        stroke_width: SELECTION_BOX_STROKE_WIDTH,
        rx: None,
        ry: None,
        dash_array: Vec::new(),
        fill_gradient: None,
    });
}

pub(super) fn push_selection_item_box(out: &mut Vec<RenderPrimitive>, item: FragmentSelectionItem) {
    let role = match item.kind {
        FragmentItemKind::Node => RenderRole::SelectionNode,
        FragmentItemKind::Label => RenderRole::SelectionTextBox,
        FragmentItemKind::Bond => RenderRole::SelectionBond,
    };
    push_selection_box(out, item.bounds, role);
}

pub(super) fn push_selection_bond_dot(out: &mut Vec<RenderPrimitive>, center: Point) {
    out.push(RenderPrimitive::Circle {
        role: RenderRole::SelectionBondDot,
        object_id: None,
        node_id: None,
        center,
        radius: SELECTION_BOND_DOT_RADIUS,
        fill: "rgba(47,111,237,0.9)".to_string(),
        stroke: "#ffffff".to_string(),
        stroke_width: crate::px_to_pt(1.0),
    });
}

pub(super) fn render_selection_resize_handles(
    out: &mut Vec<RenderPrimitive>,
    use_global_bounds_only: bool,
) {
    for bounds in selection_control_bounds(out, use_global_bounds_only) {
        push_selection_resize_handles_for_bounds(out, bounds);
    }
}

pub(super) fn selection_control_bounds(
    primitives: &[RenderPrimitive],
    use_global_bounds_only: bool,
) -> Vec<AxisBounds> {
    let item_bounds: Vec<_> = primitives
        .iter()
        .filter_map(selection_rect_primitive_bounds)
        .collect();
    if !use_global_bounds_only {
        return item_bounds;
    }

    let mut global_bounds = None;
    for bounds in item_bounds {
        include_optional_bounds(&mut global_bounds, bounds);
    }
    global_bounds.into_iter().collect()
}

pub(super) fn selection_rect_primitive_bounds(primitive: &RenderPrimitive) -> Option<AxisBounds> {
    match primitive {
        RenderPrimitive::Rect {
            role:
                RenderRole::SelectionBox
                | RenderRole::SelectionBond
                | RenderRole::SelectionNode
                | RenderRole::SelectionTextBox,
            x,
            y,
            width,
            height,
            ..
        } => Some(AxisBounds::new(*x, *y, *x + *width, *y + *height)),
        _ => None,
    }
}

fn push_selection_resize_handles_for_bounds(out: &mut Vec<RenderPrimitive>, bounds: AxisBounds) {
    for handle in [
        SelectionResizeHandle::NorthWest,
        SelectionResizeHandle::North,
        SelectionResizeHandle::NorthEast,
        SelectionResizeHandle::East,
        SelectionResizeHandle::SouthEast,
        SelectionResizeHandle::South,
        SelectionResizeHandle::SouthWest,
        SelectionResizeHandle::West,
    ] {
        let center = selection_resize_handle_center(handle, bounds);
        let size = SELECTION_RESIZE_HANDLE_SIZE;
        out.push(RenderPrimitive::Rect {
            role: RenderRole::SelectionResizeHandle,
            object_id: Some(handle.name().to_string()),
            node_id: None,
            x: center.x - size * 0.5,
            y: center.y - size * 0.5,
            width: size,
            height: size,
            fill: Some("rgba(47,111,237,0.92)".to_string()),
            stroke: None,
            stroke_width: 0.0,
            rx: None,
            ry: None,
            dash_array: Vec::new(),
            fill_gradient: None,
        });
    }
}

fn selection_resize_handle_center(handle: SelectionResizeHandle, bounds: AxisBounds) -> Point {
    match handle {
        SelectionResizeHandle::North => Point::new(bounds.center_x(), bounds.min_y),
        SelectionResizeHandle::South => Point::new(bounds.center_x(), bounds.max_y),
        SelectionResizeHandle::East => Point::new(bounds.max_x, bounds.center_y()),
        SelectionResizeHandle::West => Point::new(bounds.min_x, bounds.center_y()),
        SelectionResizeHandle::NorthEast => Point::new(bounds.max_x, bounds.min_y),
        SelectionResizeHandle::NorthWest => Point::new(bounds.min_x, bounds.min_y),
        SelectionResizeHandle::SouthEast => Point::new(bounds.max_x, bounds.max_y),
        SelectionResizeHandle::SouthWest => Point::new(bounds.min_x, bounds.max_y),
    }
}

pub(super) fn midpoint(a: Point, b: Point) -> Point {
    Point::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

pub(super) fn include_optional_bounds(target: &mut Option<AxisBounds>, bounds: AxisBounds) {
    if let Some(existing) = target {
        existing.include_bounds(bounds);
    } else {
        *target = Some(bounds);
    }
}

pub(super) fn point_in_bounds(point: Point, bounds: AxisBounds) -> bool {
    point.x >= bounds.min_x
        && point.x <= bounds.max_x
        && point.y >= bounds.min_y
        && point.y <= bounds.max_y
}

pub(super) fn bounds_intersect(a: AxisBounds, b: AxisBounds) -> bool {
    a.min_x <= b.max_x && a.max_x >= b.min_x && a.min_y <= b.max_y && a.max_y >= b.min_y
}

pub(super) fn polygon_bounds(points: &[Point]) -> AxisBounds {
    let mut bounds = AxisBounds::around_point(points[0], 0.0);
    for point in &points[1..] {
        bounds.include_point(*point);
    }
    bounds
}

pub(super) fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
    let mut inside = false;
    let mut previous = *polygon.last().unwrap_or(&point);
    for current in polygon {
        let intersects = ((current.y > point.y) != (previous.y > point.y))
            && (point.x
                < (previous.x - current.x) * (point.y - current.y)
                    / (previous.y - current.y + 1.0e-12)
                    + current.x);
        if intersects {
            inside = !inside;
        }
        previous = *current;
    }
    inside
}

pub(super) fn segment_intersects_bounds(start: Point, end: Point, bounds: AxisBounds) -> bool {
    if point_in_bounds(start, bounds) || point_in_bounds(end, bounds) {
        return true;
    }
    let corners = [
        Point::new(bounds.min_x, bounds.min_y),
        Point::new(bounds.max_x, bounds.min_y),
        Point::new(bounds.max_x, bounds.max_y),
        Point::new(bounds.min_x, bounds.max_y),
    ];
    (0..4).any(|index| segments_intersect(start, end, corners[index], corners[(index + 1) % 4]))
}

pub(super) fn rect_intersects_polygon(
    bounds: AxisBounds,
    polygon: &[Point],
    polygon_bounds: AxisBounds,
) -> bool {
    if !bounds_intersect(bounds, polygon_bounds) {
        return false;
    }
    let rect_points = [
        Point::new(bounds.min_x, bounds.min_y),
        Point::new(bounds.max_x, bounds.min_y),
        Point::new(bounds.max_x, bounds.max_y),
        Point::new(bounds.min_x, bounds.max_y),
    ];
    if rect_points
        .iter()
        .any(|point| point_in_polygon(*point, polygon))
    {
        return true;
    }
    if polygon.iter().any(|point| point_in_bounds(*point, bounds)) {
        return true;
    }
    (0..4).any(|edge_index| {
        let rect_start = rect_points[edge_index];
        let rect_end = rect_points[(edge_index + 1) % 4];
        polygon.iter().enumerate().any(|(index, start)| {
            let end = polygon[(index + 1) % polygon.len()];
            segments_intersect(rect_start, rect_end, *start, end)
        })
    })
}

pub(super) fn segment_intersects_polygon(
    start: Point,
    end: Point,
    polygon: &[Point],
    polygon_bounds: AxisBounds,
) -> bool {
    if !bounds_intersect(
        AxisBounds::new(start.x, start.y, end.x, end.y),
        polygon_bounds,
    ) {
        return false;
    }
    if point_in_polygon(start, polygon) || point_in_polygon(end, polygon) {
        return true;
    }
    polygon.iter().enumerate().any(|(index, edge_start)| {
        let edge_end = polygon[(index + 1) % polygon.len()];
        segments_intersect(start, end, *edge_start, edge_end)
    })
}

pub(super) fn orientation(a: Point, b: Point, c: Point) -> f64 {
    (b.y - a.y) * (c.x - b.x) - (b.x - a.x) * (c.y - b.y)
}

pub(super) fn on_segment(a: Point, b: Point, c: Point) -> bool {
    b.x >= a.x.min(c.x) - 1.0e-9
        && b.x <= a.x.max(c.x) + 1.0e-9
        && b.y >= a.y.min(c.y) - 1.0e-9
        && b.y <= a.y.max(c.y) + 1.0e-9
}

pub(super) fn segments_intersect(a1: Point, a2: Point, b1: Point, b2: Point) -> bool {
    let o1 = orientation(a1, a2, b1);
    let o2 = orientation(a1, a2, b2);
    let o3 = orientation(b1, b2, a1);
    let o4 = orientation(b1, b2, a2);
    if (o1 > 0.0) != (o2 > 0.0) && (o3 > 0.0) != (o4 > 0.0) {
        return true;
    }
    (o1.abs() <= 1.0e-9 && on_segment(a1, b1, a2))
        || (o2.abs() <= 1.0e-9 && on_segment(a1, b2, a2))
        || (o3.abs() <= 1.0e-9 && on_segment(b1, a1, b2))
        || (o4.abs() <= 1.0e-9 && on_segment(b1, a2, b2))
}

pub(super) fn connected_component_node_ids(
    fragment: &crate::MoleculeFragment,
    start_node_id: &str,
) -> Vec<String> {
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut queue = VecDeque::new();
    visited.insert(start_node_id.to_string());
    queue.push_back(start_node_id.to_string());
    while let Some(current) = queue.pop_front() {
        for bond in &fragment.bonds {
            let neighbor = if bond.begin == current {
                Some(bond.end.as_str())
            } else if bond.end == current {
                Some(bond.begin.as_str())
            } else {
                None
            };
            let Some(neighbor) = neighbor else {
                continue;
            };
            if visited.insert(neighbor.to_string()) {
                queue.push_back(neighbor.to_string());
            }
        }
    }
    visited.into_iter().collect()
}
