use super::text_edit::{endpoint_label_world_bounds, text_object_world_bounds};
use super::Engine;
use crate::{
    fragment_bond_visual_bounds, hit_test_bond_center, hit_test_endpoint, HoverTextBox, Point,
    RenderPrimitive, RenderRole, SelectionState, BOND_CENTER_HIT_RADIUS, ENDPOINT_FOCUS_RADIUS,
    ENDPOINT_HIT_RADIUS,
};
use std::collections::{BTreeSet, VecDeque};

const SELECTION_NODE_BOX_SIZE: f64 = ENDPOINT_FOCUS_RADIUS * 2.0;
const SELECTION_BOX_STROKE_WIDTH: f64 = crate::px_to_cm(1.2);
const SELECTION_BOND_DOT_RADIUS: f64 = crate::px_to_cm(3.0);

#[derive(Clone)]
enum SelectHit {
    TextObject { object_id: String },
    Label { node_id: String },
    Node { node_id: String },
    Bond { bond_id: String },
}

#[derive(Clone, Copy)]
struct AxisBounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl AxisBounds {
    fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x: min_x.min(max_x),
            min_y: min_y.min(max_y),
            max_x: min_x.max(max_x),
            max_y: min_y.max(max_y),
        }
    }

    fn around_point(point: Point, half_size: f64) -> Self {
        Self::new(
            point.x - half_size,
            point.y - half_size,
            point.x + half_size,
            point.y + half_size,
        )
    }

    fn from_array(bounds: [f64; 4]) -> Self {
        Self::new(bounds[0], bounds[1], bounds[2], bounds[3])
    }

    fn include_point(&mut self, point: Point) {
        self.min_x = self.min_x.min(point.x);
        self.min_y = self.min_y.min(point.y);
        self.max_x = self.max_x.max(point.x);
        self.max_y = self.max_y.max(point.y);
    }

    fn include_bounds(&mut self, bounds: AxisBounds) {
        self.min_x = self.min_x.min(bounds.min_x);
        self.min_y = self.min_y.min(bounds.min_y);
        self.max_x = self.max_x.max(bounds.max_x);
        self.max_y = self.max_y.max(bounds.max_y);
    }
}

struct ComponentSelection {
    node_ids: Vec<String>,
    label_node_ids: Vec<String>,
    bond_ids: Vec<String>,
}

#[derive(Clone, Copy)]
enum FragmentItemKind {
    Node,
    Label,
    Bond,
}

#[derive(Clone, Copy)]
struct FragmentSelectionItem {
    kind: FragmentItemKind,
    bounds: AxisBounds,
    center: Point,
}

impl Engine {
    pub fn select_at_point(&mut self, point: Point, additive: bool) {
        let hit = self.select_hit_at_point(point);
        self.state.selection = if let Some(hit) = hit {
            let mut selection = if additive {
                self.state.selection.clone()
            } else {
                SelectionState::default()
            };
            if !additive {
                selection.region = false;
            }
            add_hit_to_selection(&mut selection, hit);
            selection
        } else if additive {
            self.state.selection.clone()
        } else {
            SelectionState::default()
        };
        self.state.overlay.preview = None;
        self.hover_select_target(point);
    }

    pub fn select_in_rect(&mut self, start: Point, end: Point, additive: bool) {
        let bounds = AxisBounds::new(start.x, start.y, end.x, end.y);
        let selection = self.collect_region_selection(
            |point| point_in_bounds(point, bounds),
            |segment_start, segment_end| {
                segment_intersects_bounds(segment_start, segment_end, bounds)
            },
            |candidate_bounds| bounds_intersect(bounds, candidate_bounds),
        );
        self.state.selection = merge_selection(self.state.selection.clone(), selection, additive);
        self.clear_interaction();
    }

    pub fn select_in_polygon(&mut self, points: Vec<Point>, additive: bool) {
        if points.len() < 3 {
            return;
        }
        let polygon_bounds = polygon_bounds(&points);
        let selection = self.collect_region_selection(
            |point| point_in_polygon(point, &points),
            |segment_start, segment_end| {
                segment_intersects_polygon(segment_start, segment_end, &points, polygon_bounds)
            },
            |candidate_bounds| {
                bounds_intersect(polygon_bounds, candidate_bounds)
                    && rect_intersects_polygon(candidate_bounds, &points, polygon_bounds)
            },
        );
        self.state.selection = merge_selection(self.state.selection.clone(), selection, additive);
        self.clear_interaction();
    }

    pub(super) fn hover_select_target(&mut self, point: Point) {
        self.drag = None;
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_text_box = None;
        self.state.overlay.hover_endpoint = None;
        self.state.overlay.preview = None;
        if let Some((node_id, bounds)) = self.hit_test_endpoint_label_box(point) {
            self.state.overlay.hover_text_box = Some(HoverTextBox {
                bounds,
                object_id: None,
                node_id: Some(node_id),
            });
            return;
        }
        if let Some((object_id, bounds)) = self.hit_test_text_object(point) {
            self.state.overlay.hover_text_box = Some(HoverTextBox {
                bounds,
                object_id: Some(object_id),
                node_id: None,
            });
            return;
        }
        if let Some(endpoint) = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
        {
            self.state.overlay.hover_endpoint = Some(endpoint);
            return;
        }
        if let Some(center) =
            hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
        {
            self.state.overlay.hover_bond_center = Some(center);
        }
    }

    fn select_hit_at_point(&self, point: Point) -> Option<SelectHit> {
        if let Some((node_id, _)) = self.hit_test_endpoint_label_box(point) {
            return Some(SelectHit::Label { node_id });
        }
        if let Some((object_id, _)) = self.hit_test_text_object(point) {
            return Some(SelectHit::TextObject { object_id });
        }
        if let Some(endpoint) = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
        {
            return Some(SelectHit::Node {
                node_id: endpoint.node_id,
            });
        }
        hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS).map(|center| {
            SelectHit::Bond {
                bond_id: center.bond_id,
            }
        })
    }

    fn collect_region_selection<FP, FS, FB>(
        &self,
        mut point_inside: FP,
        mut segment_selected: FS,
        mut bounds_selected: FB,
    ) -> SelectionState
    where
        FP: FnMut(Point) -> bool,
        FS: FnMut(Point, Point) -> bool,
        FB: FnMut(AxisBounds) -> bool,
    {
        let mut selection = SelectionState::default();
        selection.region = true;
        for object in &self.state.document.objects {
            if object.object_type != "text" || !object.visible {
                continue;
            }
            let Some(bounds) = text_object_world_bounds(object) else {
                continue;
            };
            if bounds_selected(AxisBounds::from_array(bounds)) {
                selection.text_objects.push(object.id.clone());
            }
        }

        let Some(entry) = self.state.document.editable_fragment() else {
            return selection;
        };

        for node in &entry.fragment.nodes {
            if let Some(bounds) =
                endpoint_label_world_bounds(node, entry.object.transform.translate)
            {
                if bounds_selected(AxisBounds::from_array(bounds)) {
                    selection.label_nodes.push(node.id.clone());
                }
            }
            let node_point = entry.world_point_for_node(node);
            if point_inside(node_point) {
                selection.nodes.push(node.id.clone());
            }
        }

        for bond in &entry.fragment.bonds {
            let Some(begin) = entry
                .fragment
                .nodes
                .iter()
                .find(|node| node.id == bond.begin)
            else {
                continue;
            };
            let Some(end) = entry.fragment.nodes.iter().find(|node| node.id == bond.end) else {
                continue;
            };
            let begin_point = entry.world_point_for_node(begin);
            let end_point = entry.world_point_for_node(end);
            if segment_selected(begin_point, end_point) {
                selection.bonds.push(bond.id.clone());
            }
        }
        selection
    }

    pub(super) fn selection_render_list(&self) -> Vec<RenderPrimitive> {
        let mut out = Vec::new();
        render_selected_text_boxes(self, &mut out);
        render_selected_fragment_content(self, &mut out);
        out
    }
}

fn render_selected_text_boxes(engine: &Engine, out: &mut Vec<RenderPrimitive>) {
    let selected_text_objects: BTreeSet<&str> = engine
        .state
        .selection
        .text_objects
        .iter()
        .map(String::as_str)
        .collect();
    for object in &engine.state.document.objects {
        if !selected_text_objects.contains(object.id.as_str()) {
            continue;
        }
        let Some(bounds) = text_object_world_bounds(object) else {
            continue;
        };
        push_selection_box(
            out,
            AxisBounds::from_array(bounds),
            RenderRole::SelectionTextBox,
        );
    }
}

fn render_selected_fragment_content(engine: &Engine, out: &mut Vec<RenderPrimitive>) {
    let Some(entry) = engine.state.document.editable_fragment() else {
        return;
    };

    for component in selected_component_summaries(engine) {
        let items = component_selection_items(&engine.state.document, &entry, &component);
        if items.is_empty() {
            continue;
        }
        if items.len() == 1 {
            let item = items[0];
            push_selection_item_box(out, item);
            push_selection_bond_dot(out, item.center);
            continue;
        }
        let group_bounds = items.iter().skip(1).fold(items[0].bounds, |mut acc, item| {
            acc.include_bounds(item.bounds);
            acc
        });
        push_selection_box(out, group_bounds, RenderRole::SelectionBox);
        for item in items {
            push_selection_bond_dot(out, item.center);
        }
    }
}

fn selected_component_summaries(engine: &Engine) -> Vec<ComponentSelection> {
    let Some(entry) = engine.state.document.editable_fragment() else {
        return Vec::new();
    };
    let selected_nodes: BTreeSet<&str> = engine
        .state
        .selection
        .nodes
        .iter()
        .map(String::as_str)
        .collect();
    let selected_bonds: BTreeSet<&str> = engine
        .state
        .selection
        .bonds
        .iter()
        .map(String::as_str)
        .collect();
    let selected_label_nodes: BTreeSet<&str> = engine
        .state
        .selection
        .label_nodes
        .iter()
        .map(String::as_str)
        .collect();
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut components = Vec::new();

    for node in &entry.fragment.nodes {
        if visited.contains(&node.id) {
            continue;
        }
        let component_node_ids = connected_component_node_ids(entry.fragment, &node.id);
        for node_id in &component_node_ids {
            visited.insert(node_id.clone());
        }
        let component_bond_ids: Vec<String> = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| {
                component_node_ids.contains(&bond.begin) && component_node_ids.contains(&bond.end)
            })
            .map(|bond| bond.id.clone())
            .collect();

        let component_selected_nodes: Vec<String> = component_node_ids
            .iter()
            .filter(|node_id| selected_nodes.contains(node_id.as_str()))
            .cloned()
            .collect();
        let component_selected_label_nodes: Vec<String> = component_node_ids
            .iter()
            .filter(|node_id| selected_label_nodes.contains(node_id.as_str()))
            .cloned()
            .collect();
        let component_selected_bonds: Vec<String> = component_bond_ids
            .iter()
            .filter(|bond_id| selected_bonds.contains(bond_id.as_str()))
            .cloned()
            .collect();
        if component_selected_nodes.is_empty()
            && component_selected_label_nodes.is_empty()
            && component_selected_bonds.is_empty()
        {
            continue;
        }
        components.push(ComponentSelection {
            node_ids: component_selected_nodes,
            label_node_ids: component_selected_label_nodes,
            bond_ids: component_selected_bonds,
        });
    }
    components
}

fn component_selection_items(
    document: &crate::ChemcoreDocument,
    entry: &crate::EditableFragment<'_>,
    component: &ComponentSelection,
) -> Vec<FragmentSelectionItem> {
    let mut items = Vec::new();
    for node_id in &component.label_node_ids {
        let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == *node_id) else {
            continue;
        };
        let Some(bounds) = endpoint_label_world_bounds(node, entry.object.transform.translate)
        else {
            continue;
        };
        items.push(FragmentSelectionItem {
            kind: FragmentItemKind::Label,
            bounds: AxisBounds::from_array(bounds),
            center: Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5),
        });
    }
    for node_id in &component.node_ids {
        let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == *node_id) else {
            continue;
        };
        let center = entry.world_point_for_node(node);
        items.push(FragmentSelectionItem {
            kind: FragmentItemKind::Node,
            bounds: AxisBounds::around_point(center, SELECTION_NODE_BOX_SIZE / 2.0),
            center,
        });
    }
    for bond_id in &component.bond_ids {
        let Some(bond) = entry.fragment.bonds.iter().find(|bond| bond.id == *bond_id) else {
            continue;
        };
        let Some(begin) = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)
        else {
            continue;
        };
        let Some(end) = entry.fragment.nodes.iter().find(|node| node.id == bond.end) else {
            continue;
        };
        let begin_point = entry.world_point_for_node(begin);
        let end_point = entry.world_point_for_node(end);
        let bounds = fragment_bond_visual_bounds(document, entry.object, entry.fragment, bond)
            .map(AxisBounds::from_array)
            .unwrap_or_else(|| {
                AxisBounds::new(begin_point.x, begin_point.y, end_point.x, end_point.y)
            });
        items.push(FragmentSelectionItem {
            kind: FragmentItemKind::Bond,
            bounds,
            center: midpoint(begin_point, end_point),
        });
    }
    items
}

fn add_hit_to_selection(selection: &mut SelectionState, hit: SelectHit) {
    match hit {
        SelectHit::TextObject { object_id } => push_unique(&mut selection.text_objects, object_id),
        SelectHit::Label { node_id } => push_unique(&mut selection.label_nodes, node_id),
        SelectHit::Node { node_id } => push_unique(&mut selection.nodes, node_id),
        SelectHit::Bond { bond_id } => push_unique(&mut selection.bonds, bond_id),
    }
}

fn merge_selection(
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

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn push_selection_box(out: &mut Vec<RenderPrimitive>, bounds: AxisBounds, role: RenderRole) {
    out.push(RenderPrimitive::Rect {
        role,
        object_id: None,
        x: bounds.min_x,
        y: bounds.min_y,
        width: (bounds.max_x - bounds.min_x).max(0.0),
        height: (bounds.max_y - bounds.min_y).max(0.0),
        fill: Some("rgba(47,111,237,0.08)".to_string()),
        stroke: Some("rgba(47,111,237,0.86)".to_string()),
        stroke_width: SELECTION_BOX_STROKE_WIDTH,
        rx: None,
        ry: None,
        dash_array: Vec::new(),
        fill_gradient: None,
    });
}

fn push_selection_item_box(out: &mut Vec<RenderPrimitive>, item: FragmentSelectionItem) {
    let role = match item.kind {
        FragmentItemKind::Node => RenderRole::SelectionNode,
        FragmentItemKind::Label => RenderRole::SelectionTextBox,
        FragmentItemKind::Bond => RenderRole::SelectionBond,
    };
    push_selection_box(out, item.bounds, role);
}

fn push_selection_bond_dot(out: &mut Vec<RenderPrimitive>, center: Point) {
    out.push(RenderPrimitive::Circle {
        role: RenderRole::SelectionBondDot,
        object_id: None,
        center,
        radius: SELECTION_BOND_DOT_RADIUS,
        fill: "rgba(47,111,237,0.9)".to_string(),
        stroke: "#ffffff".to_string(),
        stroke_width: crate::px_to_cm(1.0),
    });
}

fn midpoint(a: Point, b: Point) -> Point {
    Point::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

fn point_in_bounds(point: Point, bounds: AxisBounds) -> bool {
    point.x >= bounds.min_x
        && point.x <= bounds.max_x
        && point.y >= bounds.min_y
        && point.y <= bounds.max_y
}

fn bounds_intersect(a: AxisBounds, b: AxisBounds) -> bool {
    a.min_x <= b.max_x && a.max_x >= b.min_x && a.min_y <= b.max_y && a.max_y >= b.min_y
}

fn polygon_bounds(points: &[Point]) -> AxisBounds {
    let mut bounds = AxisBounds::around_point(points[0], 0.0);
    for point in &points[1..] {
        bounds.include_point(*point);
    }
    bounds
}

fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
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

fn segment_intersects_bounds(start: Point, end: Point, bounds: AxisBounds) -> bool {
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

fn rect_intersects_polygon(
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

fn segment_intersects_polygon(
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

fn orientation(a: Point, b: Point, c: Point) -> f64 {
    (b.y - a.y) * (c.x - b.x) - (b.x - a.x) * (c.y - b.y)
}

fn on_segment(a: Point, b: Point, c: Point) -> bool {
    b.x >= a.x.min(c.x) - 1.0e-9
        && b.x <= a.x.max(c.x) + 1.0e-9
        && b.y >= a.y.min(c.y) - 1.0e-9
        && b.y <= a.y.max(c.y) + 1.0e-9
}

fn segments_intersect(a1: Point, a2: Point, b1: Point, b2: Point) -> bool {
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

fn connected_component_node_ids(
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
