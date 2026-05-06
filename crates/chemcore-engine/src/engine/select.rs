use super::text_edit::{
    endpoint_label_world_bounds, refresh_attached_node_label_geometry_for_all_nodes,
    text_object_world_bounds,
};
use super::{ArrowEditDragState, ArrowEditMode, EditorCommand, Engine};
use crate::{
    angle_between, arrow_object_handle_points, direction_from_angle, fragment_bond_visual_bounds,
    hit_test_arrow_center, hit_test_bond_center, hit_test_endpoint, line_object_points,
    nearest_angle, round2, HoverTextBox, Point, RenderPrimitive, RenderRole, SelectionState,
    BOND_CENTER_HIT_RADIUS, DEFAULT_BOND_LENGTH, DRAG_START_THRESHOLD, ENDPOINT_FOCUS_RADIUS,
    ENDPOINT_HIT_RADIUS, GLOBAL_SNAP_ANGLES,
};
use serde_json::{json, Value as JsonValue};
use std::collections::{BTreeSet, VecDeque};

const SELECTION_NODE_BOX_SIZE: f64 = ENDPOINT_FOCUS_RADIUS * 2.0;
const SELECTION_BOX_STROKE_WIDTH: f64 = crate::px_to_cm(1.2);
const SELECTION_BOND_DOT_RADIUS: f64 = crate::px_to_cm(3.0);

#[path = "select/arrange.rs"]
mod arrange;
#[path = "select/arrows.rs"]
mod arrows;
#[path = "select/drag.rs"]
mod drag;
#[path = "select/geometry.rs"]
mod geometry;
#[path = "select/render.rs"]
mod render;

use self::arrange::*;
use self::arrows::*;
use self::drag::*;
use self::geometry::*;
use self::render::*;

#[derive(Clone)]
enum SelectHit {
    TextObject { object_id: String },
    ArrowObject { object_id: String },
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

    fn expanded(self, amount: f64) -> Self {
        Self {
            min_x: self.min_x - amount,
            min_y: self.min_y - amount,
            max_x: self.max_x + amount,
            max_y: self.max_y + amount,
        }
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

#[derive(Clone)]
struct SelectionArrangeItem {
    original_bounds: AxisBounds,
    bounds: AxisBounds,
    node_ids: Vec<String>,
    text_object_ids: Vec<String>,
    mirror_x: Option<f64>,
    mirror_y: Option<f64>,
}

#[derive(Clone)]
struct NodeMoveOriginal {
    node_id: String,
    position: [f64; 2],
}

#[derive(Clone)]
struct TextMoveOriginal {
    object_id: String,
    translate: [f64; 2],
}

#[derive(Clone)]
enum SelectionMoveMode {
    Translate,
    TerminalNode {
        node_id: String,
        pivot: Point,
        length: f64,
    },
}

pub(super) struct SelectionMoveDrag {
    start: Point,
    node_originals: Vec<NodeMoveOriginal>,
    text_originals: Vec<TextMoveOriginal>,
    mode: SelectionMoveMode,
    preserve_selection_after_drag: bool,
    undo_pushed: bool,
    changed: bool,
}

pub(super) struct SelectionRotateDrag {
    center: Point,
    start_angle: f64,
    node_originals: Vec<NodeMoveOriginal>,
    text_originals: Vec<TextMoveOriginal>,
    undo_pushed: bool,
    changed: bool,
}

impl Engine {
    pub fn selection_contains_point(&self, point: Point) -> bool {
        self.selection_hit_bounds()
            .into_iter()
            .any(|bounds| point_in_bounds(point, bounds))
    }

    pub fn hover_arrow_action_at_point(&self, point: Point) -> &'static str {
        match self.hover_arrow_edit_mode_at_point(point) {
            Some(ArrowEditMode::Head) => "head",
            Some(ArrowEditMode::Tail) => "tail",
            Some(ArrowEditMode::Curve) => "curve",
            None => "",
        }
    }

    pub fn begin_hover_arrow_edit(&mut self, point: Point) -> &'static str {
        let Some((object_id, mode, points, curve)) = self.hover_arrow_edit_target_at_point(point)
        else {
            return "";
        };
        self.arrow_edit_drag = Some(ArrowEditDragState {
            object_id,
            mode,
            original_points: points,
            start_pointer: point,
            has_dragged: false,
            current_degrees: curve.abs().round(),
            undo_pushed: false,
        });
        clear_select_hover_overlay(self);
        self.state.overlay.preview = None;
        self.drag = None;
        self.arrow_drag = None;
        self.selection_drag = None;
        self.selection_rotate_drag = None;
        match mode {
            ArrowEditMode::Head => "head",
            ArrowEditMode::Tail => "tail",
            ArrowEditMode::Curve => "curve",
        }
    }

    pub fn update_hover_arrow_edit(&mut self, point: Point, alt_key: bool) -> bool {
        self.with_command(
            EditorCommand::LegacyMutation {
                label: "edit-arrow".to_string(),
            },
            |engine| engine.update_hover_arrow_edit_untracked(point, alt_key),
        )
    }

    fn update_hover_arrow_edit_untracked(&mut self, point: Point, alt_key: bool) -> bool {
        let Some(mut drag) = self.arrow_edit_drag.take() else {
            return false;
        };
        if drag.start_pointer.distance(point) >= DRAG_START_THRESHOLD {
            drag.has_dragged = true;
        }
        if drag.has_dragged {
            self.apply_arrow_edit_drag(&mut drag, point, alt_key);
        }
        self.arrow_edit_drag = Some(drag);
        true
    }

    pub fn finish_hover_arrow_edit(&mut self, point: Point, alt_key: bool) -> bool {
        self.with_command(
            EditorCommand::LegacyMutation {
                label: "edit-arrow".to_string(),
            },
            |engine| engine.finish_hover_arrow_edit_untracked(point, alt_key),
        )
    }

    fn finish_hover_arrow_edit_untracked(&mut self, point: Point, alt_key: bool) -> bool {
        let Some(mut drag) = self.arrow_edit_drag.take() else {
            return false;
        };
        if drag.start_pointer.distance(point) >= DRAG_START_THRESHOLD {
            drag.has_dragged = true;
        }
        if !drag.has_dragged {
            self.hover_select_target(point);
            return false;
        }
        self.apply_arrow_edit_drag(&mut drag, point, alt_key);
        self.hover_select_target(point);
        true
    }

    pub fn active_arrow_edit_degrees(&self) -> f64 {
        self.arrow_edit_drag
            .as_ref()
            .map(|drag| drag.current_degrees)
            .unwrap_or(0.0)
    }

    fn hover_arrow_edit_mode_at_point(&self, point: Point) -> Option<ArrowEditMode> {
        self.hover_arrow_edit_target_at_point(point)
            .map(|(_, mode, _, _)| mode)
    }

    fn hover_arrow_edit_target_at_point(
        &self,
        point: Point,
    ) -> Option<(String, ArrowEditMode, Vec<Point>, f64)> {
        let hover = hit_test_arrow_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)?;
        if self
            .state
            .selection
            .arrow_objects
            .contains(&hover.object_id)
        {
            return None;
        }
        let mut candidates = Vec::new();
        if let Some(tail) = hover.handles.first() {
            candidates.push((tail.distance(point), ArrowEditMode::Tail));
        }
        if let Some(head) = hover.handles.get(2) {
            candidates.push((head.distance(point), ArrowEditMode::Head));
        }
        if let Some(center) = hover.handles.get(1) {
            candidates.push((center.distance(point), ArrowEditMode::Curve));
        }
        let (_, mode) = candidates
            .into_iter()
            .filter(|(distance, _)| *distance <= ENDPOINT_HIT_RADIUS)
            .min_by(|left, right| left.0.total_cmp(&right.0))?;
        let object = self
            .state
            .document
            .objects
            .iter()
            .find(|object| object.id == hover.object_id)?;
        Some((
            hover.object_id,
            mode,
            line_object_points(object),
            object_arrow_curve(object),
        ))
    }

    fn apply_arrow_edit_drag(
        &mut self,
        drag: &mut ArrowEditDragState,
        point: Point,
        alt_key: bool,
    ) -> bool {
        if drag.original_points.len() < 2 {
            return false;
        }
        if !drag.undo_pushed {
            self.push_undo_snapshot();
            drag.undo_pushed = true;
        }
        let start = drag.original_points[0];
        let end = *drag.original_points.last().unwrap_or(&start);
        match drag.mode {
            ArrowEditMode::Head => {
                let next_end = snapped_arrow_endpoint(start, point, alt_key);
                update_arrow_object_points(self, &drag.object_id, start, next_end)
            }
            ArrowEditMode::Tail => {
                let next_start = snapped_arrow_endpoint(end, point, alt_key);
                update_arrow_object_points(self, &drag.object_id, next_start, end)
            }
            ArrowEditMode::Curve => {
                let curve = snapped_arrow_curve_from_point(start, end, point, alt_key);
                drag.current_degrees = curve.abs().round();
                update_arrow_object_curve(self, &drag.object_id, curve)
            }
        }
    }

    pub fn begin_selection_move_at_point(
        &mut self,
        point: Point,
        additive: bool,
        _alt_key: bool,
    ) -> bool {
        let mut preserve_selection_after_drag = true;
        if self.state.selection.is_empty() || !self.selection_contains_point(point) {
            let Some(hit) = self.select_hit_at_point(point) else {
                return false;
            };
            preserve_selection_after_drag = selection_contains_hit(&self.state.selection, &hit);
            let mut selection = if additive {
                self.state.selection.clone()
            } else {
                SelectionState::default()
            };
            if !additive {
                selection.region = false;
            }
            add_hit_to_selection(&mut selection, hit);
            self.state.selection = selection;
        }
        let Some(drag) = self.build_selection_move_drag(point, preserve_selection_after_drag)
        else {
            return false;
        };
        self.drag = None;
        clear_select_hover_overlay(self);
        self.selection_drag = Some(drag);
        true
    }

    pub fn begin_selection_rotate(&mut self, point: Point) -> bool {
        let Some(drag) = self.build_selection_rotate_drag(point) else {
            return false;
        };
        self.drag = None;
        self.selection_drag = None;
        clear_select_hover_overlay(self);
        self.selection_rotate_drag = Some(drag);
        true
    }

    pub fn update_selection_rotate(&mut self, point: Point, alt_key: bool) -> bool {
        self.with_command(EditorCommand::RotateSelection, |engine| {
            engine.update_selection_rotate_untracked(point, alt_key)
        })
    }

    fn update_selection_rotate_untracked(&mut self, point: Point, alt_key: bool) -> bool {
        if self.selection_rotate_drag.is_none() {
            return false;
        }
        clear_select_hover_overlay(self);
        self.apply_selection_rotate_drag(point, alt_key);
        true
    }

    pub fn finish_selection_rotate(&mut self, point: Point, alt_key: bool) -> bool {
        self.with_command(EditorCommand::RotateSelection, |engine| {
            engine.finish_selection_rotate_untracked(point, alt_key)
        })
    }

    fn finish_selection_rotate_untracked(&mut self, point: Point, alt_key: bool) -> bool {
        if self.selection_rotate_drag.is_none() {
            return false;
        }
        self.apply_selection_rotate_drag(point, alt_key);
        let changed = self
            .selection_rotate_drag
            .as_ref()
            .is_some_and(|drag| drag.changed);
        self.selection_rotate_drag = None;
        self.hover_select_target(point);
        changed
    }

    pub fn update_selection_move(&mut self, point: Point, alt_key: bool) -> bool {
        self.with_command(EditorCommand::MoveSelection, |engine| {
            engine.update_selection_move_untracked(point, alt_key)
        })
    }

    fn update_selection_move_untracked(&mut self, point: Point, alt_key: bool) -> bool {
        if self.selection_drag.is_none() {
            return false;
        }
        clear_select_hover_overlay(self);
        self.apply_selection_move_drag(point, alt_key);
        true
    }

    pub fn finish_selection_move(&mut self, point: Point, alt_key: bool) -> bool {
        self.with_command(EditorCommand::MoveSelection, |engine| {
            engine.finish_selection_move_untracked(point, alt_key)
        })
    }

    fn finish_selection_move_untracked(&mut self, point: Point, alt_key: bool) -> bool {
        if self.selection_drag.is_none() {
            return false;
        }
        self.apply_selection_move_drag(point, alt_key);
        let changed = self
            .selection_drag
            .as_ref()
            .is_some_and(|drag| drag.changed);
        let should_clear_selection = changed
            && self
                .selection_drag
                .as_ref()
                .is_some_and(|drag| !drag.preserve_selection_after_drag);
        self.selection_drag = None;
        if should_clear_selection {
            self.state.selection = SelectionState::default();
        }
        self.hover_select_target(point);
        changed
    }

    pub fn apply_selection_arrange_command(&mut self, command: &str) -> bool {
        self.with_command(
            EditorCommand::ApplySelectionArrange {
                command: command.to_string(),
            },
            |engine| engine.apply_selection_arrange_command_untracked(command),
        )
    }

    fn apply_selection_arrange_command_untracked(&mut self, command: &str) -> bool {
        let mut items = self.selection_arrange_items();
        if items.is_empty() {
            return false;
        }
        let changed = match command {
            "align-left" => align_items(&mut items, AlignAxis::XMin),
            "align-right" => align_items(&mut items, AlignAxis::XMax),
            "align-top" => align_items(&mut items, AlignAxis::YMin),
            "align-bottom" => align_items(&mut items, AlignAxis::YMax),
            "align-h-center" => align_items(&mut items, AlignAxis::XCenter),
            "align-v-center" => align_items(&mut items, AlignAxis::YCenter),
            "distribute-h" => distribute_items(&mut items, DistributeAxis::Horizontal),
            "distribute-v" => distribute_items(&mut items, DistributeAxis::Vertical),
            "flip-h" => flip_items(&mut items, FlipAxis::Horizontal),
            "flip-v" => flip_items(&mut items, FlipAxis::Vertical),
            _ => false,
        };
        if !changed {
            return false;
        }
        self.push_undo_snapshot();
        apply_arrange_items_to_document(self, &items);
        self.clear_interaction();
        true
    }

    pub fn apply_color_to_selection(&mut self, color: &str) -> bool {
        let color = normalize_selection_color(color);
        self.with_command(
            EditorCommand::ApplySelectionColor {
                color: color.clone(),
            },
            |engine| engine.apply_color_to_selection_untracked(&color),
        )
    }

    fn apply_color_to_selection_untracked(&mut self, color: &str) -> bool {
        if self.state.selection.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        let selected_text: BTreeSet<String> =
            self.state.selection.text_objects.iter().cloned().collect();
        let selected_graphics: BTreeSet<String> =
            self.state.selection.arrow_objects.iter().cloned().collect();
        let mut changed = false;
        for index in 0..self.state.document.objects.len() {
            let object_type = self.state.document.objects[index].object_type.clone();
            let object_id = self.state.document.objects[index].id.clone();
            if selected_text.contains(&object_id) && object_type == "text" {
                changed |= self.apply_color_to_object(index, color, ColorTarget::Text);
            } else if selected_graphics.contains(&object_id) {
                changed |= self.apply_color_to_object(index, color, ColorTarget::Graphic);
            }
        }
        changed |= self.apply_color_to_selected_fragment(color);
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        self.state.overlay.hover_arrow = None;
        self.state.overlay.hover_text_box = None;
        self.state.overlay.hover_endpoint = None;
        true
    }

    fn apply_color_to_selected_fragment(&mut self, color: &str) -> bool {
        let selected_labels: BTreeSet<String> =
            self.state.selection.label_nodes.iter().cloned().collect();
        let selected_nodes: BTreeSet<String> = self.state.selection.nodes.iter().cloned().collect();
        let selected_bonds: BTreeSet<String> = self.state.selection.bonds.iter().cloned().collect();
        let mut changed = false;
        if let Some(entry) = self.state.document.editable_fragment_mut() {
            for node in &mut entry.fragment.nodes {
                if !selected_labels.contains(&node.id) && !selected_nodes.contains(&node.id) {
                    continue;
                }
                if let Some(label) = &mut node.label {
                    if label.fill.as_deref() != Some(color) {
                        label.fill = Some(color.to_string());
                        changed = true;
                    }
                    for run in &mut label.runs {
                        if run.fill.as_deref() != Some(color) {
                            run.fill = Some(color.to_string());
                            changed = true;
                        }
                    }
                    for line in &mut label.line_runs {
                        for run in line {
                            if run.fill.as_deref() != Some(color) {
                                run.fill = Some(color.to_string());
                                changed = true;
                            }
                        }
                    }
                }
            }
            if !selected_nodes.is_empty() || !selected_bonds.is_empty() {
                let object_id = entry.object.id.clone();
                drop(entry);
                if let Some(index) = self
                    .state
                    .document
                    .objects
                    .iter()
                    .position(|object| object.id == object_id)
                {
                    changed |= self.apply_color_to_object(index, color, ColorTarget::Molecule);
                }
            }
        }
        changed
    }

    fn apply_color_to_object(&mut self, index: usize, color: &str, target: ColorTarget) -> bool {
        let Some(object) = self.state.document.objects.get(index) else {
            return false;
        };
        let object_id = object.id.clone();
        let object_type = object.object_type.clone();
        let source_style_ref = object.style_ref.clone();
        let style_id = format!("style_{object_id}_color");
        let base_style = source_style_ref
            .as_ref()
            .and_then(|style_ref| self.state.document.styles.get(style_ref))
            .cloned()
            .unwrap_or_else(|| json!({ "kind": object_type }));
        let mut style = base_style.as_object().cloned().unwrap_or_default();
        let mut changed = source_style_ref.as_deref() != Some(style_id.as_str());
        match target {
            ColorTarget::Text => {
                changed |= set_style_string(&mut style, "fill", color);
                changed |= self.apply_color_to_text_object_runs(index, color);
            }
            ColorTarget::Molecule => {
                changed |= set_style_string(&mut style, "stroke", color);
                changed |= set_style_string(&mut style, "fill", color);
            }
            ColorTarget::Graphic => {
                let has_stroke = style
                    .get("stroke")
                    .is_some_and(|value| !value.is_null());
                let has_fill = style.get("fill").is_some_and(|value| !value.is_null());
                if has_stroke || !has_fill {
                    changed |= set_style_string(&mut style, "stroke", color);
                }
                if has_fill {
                    changed |= set_style_string(&mut style, "fill", color);
                }
            }
        }
        if !changed {
            return false;
        }
        self.state
            .document
            .styles
            .insert(style_id.clone(), JsonValue::Object(style));
        if let Some(object) = self.state.document.objects.get_mut(index) {
            object.style_ref = Some(style_id);
        }
        true
    }

    fn apply_color_to_text_object_runs(&mut self, index: usize, color: &str) -> bool {
        let Some(object) = self.state.document.objects.get_mut(index) else {
            return false;
        };
        let Some(runs) = object.payload.extra.get_mut("runs").and_then(JsonValue::as_array_mut)
        else {
            return false;
        };
        let mut changed = false;
        for run in runs {
            if let Some(run_object) = run.as_object_mut() {
                changed |= set_style_string(run_object, "fill", color);
            }
        }
        changed
    }

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

    pub fn select_component_at_point(&mut self, point: Point, additive: bool) -> bool {
        let Some(entry) = self.state.document.editable_fragment() else {
            return false;
        };
        let hit_node = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
            .map(|hit| hit.node_id);
        let hit_bond = if hit_node.is_none() {
            hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
                .map(|hit| hit.bond_id)
        } else {
            None
        };
        let seed_node_id = if let Some(node_id) = hit_node {
            node_id
        } else if let Some(bond_id) = hit_bond {
            let Some(bond) = entry.fragment.bonds.iter().find(|bond| bond.id == bond_id) else {
                return false;
            };
            bond.begin.clone()
        } else {
            return false;
        };
        let component_node_ids = connected_component_node_ids(entry.fragment, &seed_node_id);
        let component_bond_ids: Vec<String> = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| {
                component_node_ids.contains(&bond.begin) && component_node_ids.contains(&bond.end)
            })
            .map(|bond| bond.id.clone())
            .collect();
        let label_node_ids: Vec<String> = entry
            .fragment
            .nodes
            .iter()
            .filter(|node| component_node_ids.contains(&node.id) && node.label.is_some())
            .map(|node| node.id.clone())
            .collect();
        let bracket_ids = bracket_object_ids_containing_component(
            &self.state.document,
            &entry,
            &component_node_ids,
        );
        let mut selection = if additive {
            self.state.selection.clone()
        } else {
            SelectionState::default()
        };
        selection.region = false;
        for node_id in component_node_ids {
            push_unique(&mut selection.nodes, node_id);
        }
        for bond_id in component_bond_ids {
            push_unique(&mut selection.bonds, bond_id);
        }
        for node_id in label_node_ids {
            push_unique(&mut selection.label_nodes, node_id);
        }
        for object_id in bracket_ids {
            push_unique(&mut selection.arrow_objects, object_id);
        }
        self.state.selection = selection;
        self.clear_interaction();
        true
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
        if self.selection_drag.is_some() {
            return;
        }
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_arrow = None;
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
            return;
        }
        if let Some(arrow) =
            hit_test_arrow_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
        {
            if !self
                .state
                .selection
                .arrow_objects
                .contains(&arrow.object_id)
            {
                self.state.overlay.hover_arrow = Some(arrow);
            }
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
        if let Some(center) =
            hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
        {
            return Some(SelectHit::Bond {
                bond_id: center.bond_id,
            });
        }
        for object in self.state.document.objects.iter().rev() {
            if !matches!(object.object_type.as_str(), "bracket" | "symbol" | "shape")
                || !object.visible
            {
                continue;
            }
            let Some(bounds) = scene_object_selection_bounds(object) else {
                continue;
            };
            if point_in_bounds(point, bounds.expanded(crate::px_to_cm(3.0))) {
                return Some(SelectHit::ArrowObject {
                    object_id: object.id.clone(),
                });
            }
        }
        hit_test_arrow_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS).map(|arrow| {
            SelectHit::ArrowObject {
                object_id: arrow.object_id,
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
        for object in &self.state.document.objects {
            if object.object_type != "line" || !object.visible {
                continue;
            }
            let points = line_object_points(object);
            if points.len() < 2 {
                continue;
            }
            let start = points[0];
            let end = *points.last().unwrap_or(&start);
            if segment_selected(start, end) {
                selection.arrow_objects.push(object.id.clone());
            }
        }
        for object in &self.state.document.objects {
            if !matches!(object.object_type.as_str(), "bracket" | "symbol" | "shape")
                || !object.visible
            {
                continue;
            }
            let Some(bounds) = scene_object_selection_bounds(object) else {
                continue;
            };
            if bounds_selected(bounds) {
                selection.arrow_objects.push(object.id.clone());
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

    fn selection_hit_bounds(&self) -> Vec<AxisBounds> {
        let mut bounds = Vec::new();
        for object in &self.state.document.objects {
            if !self
                .state
                .selection
                .text_objects
                .iter()
                .any(|object_id| object_id == &object.id)
            {
                continue;
            }
            if let Some(text_bounds) = text_object_world_bounds(object) {
                bounds.push(AxisBounds::from_array(text_bounds));
            }
        }
        for object in &self.state.document.objects {
            if !self.state.selection.arrow_objects.contains(&object.id) {
                continue;
            }
            if let Some(arrow_bounds) = scene_object_selection_bounds(object) {
                bounds.push(arrow_bounds);
            }
        }

        let Some(entry) = self.state.document.editable_fragment() else {
            return bounds;
        };
        for component in selected_component_summaries(self) {
            let items = component_selection_items(&self.state.document, &entry, &component);
            if items.is_empty() {
                continue;
            }
            if items.len() == 1 {
                bounds.push(items[0].bounds);
            } else {
                bounds.push(items.iter().skip(1).fold(items[0].bounds, |mut acc, item| {
                    acc.include_bounds(item.bounds);
                    acc
                }));
            }
        }
        bounds
    }

    fn selection_arrange_items(&self) -> Vec<SelectionArrangeItem> {
        let mut items = Vec::new();
        for object in &self.state.document.objects {
            if !self
                .state
                .selection
                .text_objects
                .iter()
                .any(|object_id| object_id == &object.id)
            {
                continue;
            }
            let Some(bounds) = text_object_world_bounds(object) else {
                continue;
            };
            items.push(SelectionArrangeItem {
                original_bounds: AxisBounds::from_array(bounds),
                bounds: AxisBounds::from_array(bounds),
                node_ids: Vec::new(),
                text_object_ids: vec![object.id.clone()],
                mirror_x: None,
                mirror_y: None,
            });
        }

        let Some(entry) = self.state.document.editable_fragment() else {
            return items;
        };
        for component in selected_component_summaries(self) {
            let fragment_items =
                component_selection_items(&self.state.document, &entry, &component);
            if fragment_items.is_empty() {
                continue;
            }
            let bounds =
                fragment_items
                    .iter()
                    .skip(1)
                    .fold(fragment_items[0].bounds, |mut acc, item| {
                        acc.include_bounds(item.bounds);
                        acc
                    });
            let node_ids = component_movable_node_ids(entry.fragment, &component);
            if node_ids.is_empty() {
                continue;
            }
            items.push(SelectionArrangeItem {
                original_bounds: bounds,
                bounds,
                node_ids,
                text_object_ids: Vec::new(),
                mirror_x: None,
                mirror_y: None,
            });
        }
        items
    }

    fn build_selection_move_drag(
        &self,
        start: Point,
        preserve_selection_after_drag: bool,
    ) -> Option<SelectionMoveDrag> {
        if self.state.selection.is_empty() {
            return None;
        }
        let mut node_ids = selected_movable_node_ids(self);
        let text_ids = selected_text_object_ids(self);
        if node_ids.is_empty() && text_ids.is_empty() {
            return None;
        }

        let mut node_originals = Vec::new();
        let mut mode = SelectionMoveMode::Translate;
        if let Some(entry) = self.state.document.editable_fragment() {
            node_ids.sort();
            for node_id in &node_ids {
                if let Some(node) = entry.fragment.nodes.iter().find(|node| &node.id == node_id) {
                    node_originals.push(NodeMoveOriginal {
                        node_id: node.id.clone(),
                        position: node.position,
                    });
                }
            }
            if node_ids.len() == 1 && text_ids.is_empty() && self.state.selection.bonds.is_empty() {
                if let Some((pivot, length)) = terminal_node_drag_axis(entry, &node_ids[0]) {
                    mode = SelectionMoveMode::TerminalNode {
                        node_id: node_ids[0].clone(),
                        pivot,
                        length,
                    };
                }
            }
        }

        let mut text_originals = Vec::new();
        for object in &self.state.document.objects {
            if text_ids.contains(&object.id) {
                text_originals.push(TextMoveOriginal {
                    object_id: object.id.clone(),
                    translate: object.transform.translate,
                });
            }
        }

        if node_originals.is_empty() && text_originals.is_empty() {
            return None;
        }

        Some(SelectionMoveDrag {
            start,
            node_originals,
            text_originals,
            mode,
            preserve_selection_after_drag,
            undo_pushed: false,
            changed: false,
        })
    }

    fn apply_selection_move_drag(&mut self, point: Point, alt_key: bool) {
        let Some(mut drag) = self.selection_drag.take() else {
            return;
        };
        let changed = selection_drag_changes_document(&drag, point, alt_key);
        if changed && !drag.undo_pushed {
            self.push_undo_snapshot();
            drag.undo_pushed = true;
        }
        if changed {
            apply_selection_drag_to_document(self, &drag, point, alt_key);
            drag.changed = true;
        }
        self.selection_drag = Some(drag);
    }

    fn build_selection_rotate_drag(&self, start: Point) -> Option<SelectionRotateDrag> {
        if self.state.selection.is_empty() {
            return None;
        }
        let mut node_ids = selected_movable_node_ids(self);
        let text_ids = selected_text_object_ids(self);
        if node_ids.is_empty() && text_ids.is_empty() {
            return None;
        }

        let center = self.selection_rotation_bounds().map(|bounds| {
            Point::new(
                (bounds.min_x + bounds.max_x) * 0.5,
                (bounds.min_y + bounds.max_y) * 0.5,
            )
        })?;
        let mut node_originals = Vec::new();
        if let Some(entry) = self.state.document.editable_fragment() {
            node_ids.sort();
            for node_id in &node_ids {
                if let Some(node) = entry.fragment.nodes.iter().find(|node| &node.id == node_id) {
                    node_originals.push(NodeMoveOriginal {
                        node_id: node.id.clone(),
                        position: node.position,
                    });
                }
            }
        }

        let mut text_originals = Vec::new();
        for object in &self.state.document.objects {
            if text_ids.contains(&object.id) {
                text_originals.push(TextMoveOriginal {
                    object_id: object.id.clone(),
                    translate: object.transform.translate,
                });
            }
        }

        if node_originals.is_empty() && text_originals.is_empty() {
            return None;
        }

        Some(SelectionRotateDrag {
            center,
            start_angle: angle_between(center, start),
            node_originals,
            text_originals,
            undo_pushed: false,
            changed: false,
        })
    }

    fn apply_selection_rotate_drag(&mut self, point: Point, alt_key: bool) {
        let Some(mut drag) = self.selection_rotate_drag.take() else {
            return;
        };
        let angle = selection_rotate_delta_degrees(&drag, point, alt_key);
        let changed = angle.abs() > crate::EPSILON;
        if changed && !drag.undo_pushed {
            self.push_undo_snapshot();
            drag.undo_pushed = true;
        }
        if changed {
            apply_selection_rotation_to_document(self, &drag, angle);
            drag.changed = true;
        }
        self.selection_rotate_drag = Some(drag);
    }

    pub(super) fn selection_render_list(&self) -> Vec<RenderPrimitive> {
        if self.selection_rotate_drag.is_some() {
            return Vec::new();
        }
        let mut out = Vec::new();
        render_selected_text_boxes(self, &mut out);
        render_selected_arrow_handles(self, &mut out);
        render_selected_fragment_content(self, &mut out);
        out
    }

    fn selection_rotation_bounds(&self) -> Option<AxisBounds> {
        let mut out = None;
        for object in &self.state.document.objects {
            if !self.state.selection.text_objects.contains(&object.id) {
                continue;
            }
            let Some(bounds) = text_object_world_bounds(object) else {
                continue;
            };
            include_optional_bounds(&mut out, AxisBounds::from_array(bounds));
        }
        for object in &self.state.document.objects {
            if !self.state.selection.arrow_objects.contains(&object.id) {
                continue;
            }
            if let Some(bounds) = scene_object_selection_bounds(object) {
                include_optional_bounds(&mut out, bounds);
            }
        }
        let Some(entry) = self.state.document.editable_fragment() else {
            return out;
        };
        for component in selected_component_summaries(self) {
            for item in component_selection_items(&self.state.document, &entry, &component) {
                include_optional_bounds(&mut out, item.bounds);
            }
        }
        out
    }
}

#[derive(Clone, Copy)]
enum ColorTarget {
    Text,
    Graphic,
    Molecule,
}

fn normalize_selection_color(color: &str) -> String {
    let trimmed = color.trim();
    if trimmed.starts_with('#') && trimmed.len() == 7 {
        trimmed.to_ascii_lowercase()
    } else {
        "#000000".to_string()
    }
}

fn set_style_string(
    style: &mut serde_json::Map<String, JsonValue>,
    key: &str,
    value: &str,
) -> bool {
    if style.get(key).and_then(JsonValue::as_str) == Some(value) {
        return false;
    }
    style.insert(key.to_string(), JsonValue::String(value.to_string()));
    true
}
