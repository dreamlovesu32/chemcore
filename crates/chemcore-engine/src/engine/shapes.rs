use super::*;
use crate::round2;

impl Engine {
    pub(super) fn pointer_down_shape(&mut self, event: PointerEvent) {
        let point = event.point();
        if self.begin_hover_shape_edit(point) != "" {
            return;
        }
        self.clear_interaction();
        self.state.selection = SelectionState::default();
        self.shape_drag = Some(ShapeDragState {
            start: point,
            current: point,
            has_dragged: false,
        });
    }

    pub(super) fn pointer_move_shape(&mut self, event: PointerEvent) {
        let point = event.point();
        if self.shape_edit_drag.is_some() {
            self.update_hover_shape_edit(point, event.alt_key);
            return;
        }
        self.state.overlay = OverlayState::default();
        if let Some(mut drag) = self.shape_drag.take() {
            drag.current = point;
            if drag.start.distance(point) >= DRAG_START_THRESHOLD {
                drag.has_dragged = true;
            }
            if drag.has_dragged {
                self.state.overlay.preview = Some(BondPreview {
                    start: point,
                    end: point,
                });
            }
            self.shape_drag = Some(drag);
        } else {
            self.refresh_shape_hover(point);
        }
    }

    pub(super) fn pointer_up_shape(&mut self, event: PointerEvent) {
        if self.shape_edit_drag.is_some() {
            self.finish_hover_shape_edit(event.point(), event.alt_key);
            return;
        }
        let Some(mut drag) = self.shape_drag.take() else {
            return;
        };
        drag.current = event.point();
        if drag.start.distance(drag.current) < DRAG_START_THRESHOLD {
            self.state.overlay = OverlayState::default();
            return;
        }
        drag.has_dragged = true;
        let command = EditorCommand::AddShape {
            kind: self.state.tool.shape_kind,
            style: self.state.tool.shape_style,
            color: self.state.tool.shape_color.clone(),
            begin: CommandAnchor::from(drag.start),
            end: CommandAnchor::from(drag.current),
        };
        self.with_command(command, |engine| engine.insert_shape_from_drag(&drag));
        self.state.overlay = OverlayState::default();
    }

    pub(super) fn shape_preview_document(&self) -> Option<ChemcoreDocument> {
        let drag = self.shape_drag.as_ref()?;
        if !drag.has_dragged {
            return None;
        }
        let mut document = self.state.document.clone();
        let style_id = "__preview_shape_style".to_string();
        document
            .styles
            .insert(style_id.clone(), self.pending_shape_style());
        document.objects.push(self.shape_scene_object(
            drag.start,
            drag.current,
            "__preview_shape".to_string(),
            style_id,
        )?);
        Some(document)
    }

    pub(super) fn insert_shape_from_drag(&mut self, drag: &ShapeDragState) -> bool {
        let object_id = self.next_id("obj_shape");
        let style_id = format!("style_{object_id}");
        let Some(object) = self.shape_scene_object(
            drag.start,
            drag.current,
            object_id.clone(),
            style_id.clone(),
        ) else {
            return false;
        };
        self.push_undo_snapshot();
        self.state
            .document
            .styles
            .insert(style_id, self.pending_shape_style());
        self.state.document.objects.push(object);
        self.note_pending_select_target(PendingSelectTarget::GraphicObject(object_id));
        true
    }

    pub(super) fn shape_scene_object(
        &self,
        start: Point,
        current: Point,
        object_id: String,
        style_id: String,
    ) -> Option<SceneObject> {
        let (transform, bbox, extra) = match self.state.tool.shape_kind {
            ShapeKind::Circle => {
                let radius = start.distance(current);
                if radius <= crate::EPSILON {
                    return None;
                }
                let angle = angle_between(start, current);
                let major = current;
                let minor = start.translated(direction_from_angle(angle + 90.0).scaled(radius));
                let mut extra = BTreeMap::new();
                extra.insert("kind".to_string(), json!("circle"));
                extra.insert("center".to_string(), json!([start.x, start.y]));
                extra.insert("majorAxisEnd".to_string(), json!([major.x, major.y]));
                extra.insert("minorAxisEnd".to_string(), json!([minor.x, minor.y]));
                (
                    crate::Transform::identity(),
                    [
                        start.x - radius,
                        start.y - radius,
                        radius * 2.0,
                        radius * 2.0,
                    ],
                    extra,
                )
            }
            ShapeKind::Ellipse => {
                let major_radius = start.distance(current);
                if major_radius <= crate::EPSILON {
                    return None;
                }
                let angle = nearest_angle(angle_between(start, current), GLOBAL_SNAP_ANGLES);
                let major = start.translated(direction_from_angle(angle).scaled(major_radius));
                let minor_radius = major_radius * ELLIPSE_MINOR_AXIS_RATIO;
                let minor =
                    start.translated(direction_from_angle(angle + 90.0).scaled(minor_radius));
                let mut extra = BTreeMap::new();
                extra.insert("kind".to_string(), json!("ellipse"));
                extra.insert("center".to_string(), json!([start.x, start.y]));
                extra.insert("majorAxisEnd".to_string(), json!([major.x, major.y]));
                extra.insert("minorAxisEnd".to_string(), json!([minor.x, minor.y]));
                (
                    crate::Transform::identity(),
                    [
                        start.x - major_radius,
                        start.y - major_radius,
                        major_radius * 2.0,
                        major_radius * 2.0,
                    ],
                    extra,
                )
            }
            ShapeKind::RoundRect
            | ShapeKind::Rect
            | ShapeKind::CrossTable
            | ShapeKind::TlcPlate => {
                let x1 = start.x.min(current.x);
                let y1 = start.y.min(current.y);
                let width = (current.x - start.x).abs();
                let height = (current.y - start.y).abs();
                if width <= crate::EPSILON || height <= crate::EPSILON {
                    return None;
                }
                let mut extra = BTreeMap::new();
                extra.insert(
                    "kind".to_string(),
                    json!(match self.state.tool.shape_kind {
                        ShapeKind::RoundRect => "roundRect",
                        ShapeKind::CrossTable => "crossTable",
                        ShapeKind::TlcPlate => "tlcPlate",
                        _ => "rect",
                    }),
                );
                if self.state.tool.shape_kind == ShapeKind::TlcPlate {
                    extra.insert("originFraction".to_string(), json!(0.1));
                    extra.insert("solventFrontFraction".to_string(), json!(0.1));
                    extra.insert("showOrigin".to_string(), json!(true));
                    extra.insert("showSolventFront".to_string(), json!(true));
                    extra.insert("showBorders".to_string(), json!(true));
                    extra.insert("showSideTicks".to_string(), json!(true));
                    extra.insert(
                        "dashSpacing".to_string(),
                        json!(round2(self.options.hash_spacing)),
                    );
                    let lane_count = suggested_tlc_lane_count(width);
                    let lanes: Vec<_> = (0..lane_count)
                        .map(|index| {
                            let offset = (index as f64 + 1.0) / (lane_count as f64 + 1.0);
                            json!({
                                "offset": round2(offset),
                                "spots": [
                                    {
                                        "rf": 0.15,
                                    }
                                ]
                            })
                        })
                        .collect();
                    extra.insert("lanes".to_string(), json!(lanes));
                }
                if self.state.tool.shape_kind == ShapeKind::RoundRect {
                    extra.insert(
                        "cornerRadius".to_string(),
                        json!(ROUND_RECT_CORNER_RADIUS.min(width * 0.5).min(height * 0.5)),
                    );
                }
                (
                    crate::Transform {
                        translate: [x1, y1],
                        rotate: 0.0,
                        scale: [1.0, 1.0],
                    },
                    [0.0, 0.0, width, height],
                    extra,
                )
            }
        };
        Some(SceneObject {
            id: object_id,
            object_type: "shape".to_string(),
            name: "shape".to_string(),
            visible: true,
            locked: false,
            z_index: self.next_shape_z_index(),
            transform,
            style_ref: Some(style_id),
            meta: json!({
                "source": "editor",
            }),
            payload: crate::ObjectPayload {
                resource_ref: None,
                bbox: Some(bbox),
                extra,
            },
            children: Vec::new(),
        })
    }

    pub(super) fn pending_shape_style(&self) -> JsonValue {
        let color = self.state.tool.shape_color.clone();
        let stroke_width = self.options.graphic_stroke_world_cm().value();
        match self.state.tool.shape_style {
            ShapeStyle::Solid => json!({
                "kind": "shape",
                "fill": null,
                "stroke": color,
                "strokeWidth": stroke_width,
                "dashArray": [],
            }),
            ShapeStyle::Dashed => json!({
                "kind": "shape",
                "fill": null,
                "stroke": color,
                "strokeWidth": stroke_width,
                "dashArray": [self.options.hash_spacing],
            }),
            ShapeStyle::Shaded => json!({
                "kind": "shape",
                "fill": color,
                "stroke": color,
                "strokeWidth": stroke_width,
                "dashArray": [],
                "shaded": true,
            }),
            ShapeStyle::Filled => json!({
                "kind": "shape",
                "fill": color,
                "stroke": null,
                "strokeWidth": 0.0,
                "dashArray": [],
            }),
            ShapeStyle::Shadowed => json!({
                "kind": "shape",
                "fill": null,
                "stroke": color,
                "strokeWidth": stroke_width,
                "dashArray": [],
                "shadow": true,
                "shadowSize": 4.0,
            }),
        }
    }

    pub(super) fn next_shape_z_index(&self) -> i32 {
        self.state
            .document
            .objects
            .iter()
            .map(|object| object.z_index)
            .max()
            .unwrap_or(10)
            + 1
    }

    pub fn hover_shape_action_at_point(&self, point: Point) -> &'static str {
        self.shape_edit_target_at_point(point)
            .map(|target| target.handle.action_name())
            .unwrap_or("")
    }

    pub fn begin_hover_shape_edit(&mut self, point: Point) -> &'static str {
        let Some(target) = self.shape_edit_target_at_point(point) else {
            return "";
        };
        let action = target.handle.action_name();
        self.shape_edit_drag = Some(ShapeEditDragState {
            object_id: target.object_id,
            handle: target.handle,
            original_object: target.object,
            start_pointer: point,
            has_dragged: false,
            undo_pushed: false,
            changed: false,
        });
        self.drag = None;
        self.arrow_drag = None;
        self.arrow_edit_drag = None;
        self.selection_drag = None;
        self.selection_rotate_drag = None;
        self.selection_resize_drag = None;
        self.shape_drag = None;
        self.bracket_drag = None;
        self.state.overlay.hover_shape = None;
        self.state.overlay.preview = None;
        action
    }

    pub fn update_hover_shape_edit(&mut self, point: Point, _alt_key: bool) -> bool {
        self.with_command(
            EditorCommand::LegacyMutation {
                label: "edit-shape".to_string(),
            },
            |engine| engine.update_hover_shape_edit_untracked(point),
        )
    }

    fn update_hover_shape_edit_untracked(&mut self, point: Point) -> bool {
        let Some(mut drag) = self.shape_edit_drag.take() else {
            return false;
        };
        if drag.start_pointer.distance(point) > crate::EPSILON {
            drag.has_dragged = true;
        }
        if drag.has_dragged {
            let Some(next_object) =
                resized_shape_object_from_handle(&drag.original_object, drag.handle, point)
            else {
                self.shape_edit_drag = Some(drag);
                return false;
            };
            if !drag.undo_pushed {
                self.push_undo_snapshot();
                drag.undo_pushed = true;
            }
            if let Some(object) = self
                .state
                .document
                .objects
                .iter_mut()
                .find(|object| object.id == drag.object_id)
            {
                *object = next_object;
                drag.changed = true;
            }
        }
        self.shape_edit_drag = Some(drag);
        true
    }

    pub fn finish_hover_shape_edit(&mut self, point: Point, _alt_key: bool) -> bool {
        self.with_command(
            EditorCommand::LegacyMutation {
                label: "edit-shape".to_string(),
            },
            |engine| engine.finish_hover_shape_edit_untracked(point),
        )
    }

    fn finish_hover_shape_edit_untracked(&mut self, point: Point) -> bool {
        if self.shape_edit_drag.is_none() {
            return false;
        }
        self.update_hover_shape_edit_untracked(point);
        let (changed, object_id) = self
            .shape_edit_drag
            .as_ref()
            .map(|drag| (drag.changed, drag.object_id.clone()))
            .unwrap_or((false, String::new()));
        self.shape_edit_drag = None;
        self.refresh_shape_hover(point);
        if changed {
            self.note_pending_select_target(PendingSelectTarget::GraphicObject(object_id));
        }
        changed
    }

    pub(super) fn refresh_shape_hover(&mut self, point: Point) {
        self.state.overlay.hover_shape = self.shape_hover_at_point(point);
        self.state.overlay.hover_endpoint = None;
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_arrow = None;
        self.state.overlay.hover_text_box = None;
        self.state.overlay.preview = None;
    }

    pub(super) fn shape_select_hit_at_point(&self, point: Point, object: &SceneObject) -> bool {
        if object.object_type != "shape" || !object.visible {
            return false;
        }
        let Some(kind) = shape_object_kind(object) else {
            return false;
        };
        match kind {
            ShapeObjectKind::Circle | ShapeObjectKind::Ellipse => {
                shape_oval_hit(object, point, true).is_some()
            }
            ShapeObjectKind::Rect | ShapeObjectKind::RoundRect => {
                shape_rect_hit(object, point, true).is_some()
            }
            ShapeObjectKind::Orbital => shape_rect_hit(object, point, true).is_some(),
        }
    }

    pub(super) fn shape_hover_at_point(&self, point: Point) -> Option<HoverShape> {
        let target = self.shape_hover_target_at_point(point)?;
        Some(HoverShape {
            object_id: target.object_id,
            handles: target.handles,
        })
    }

    fn shape_edit_target_at_point(&self, point: Point) -> Option<ShapeTarget> {
        let target = self.shape_hover_target_at_point(point)?;
        target
            .active_handle
            .map(|handle| ShapeTarget { handle, ..target })
    }

    fn shape_hover_target_at_point(&self, point: Point) -> Option<ShapeTarget> {
        for object in self.state.document.objects.iter().rev() {
            if object.object_type != "shape" || !object.visible {
                continue;
            }
            let Some(kind) = shape_object_kind(object) else {
                continue;
            };
            match kind {
                ShapeObjectKind::Circle => {
                    let Some(hit) = shape_circle_hover(object, point) else {
                        continue;
                    };
                    return Some(ShapeTarget {
                        object_id: object.id.clone(),
                        object: object.clone(),
                        handle: ShapeEditHandle::CircleRadius,
                        active_handle: Some(ShapeEditHandle::CircleRadius),
                        handles: vec![hit],
                    });
                }
                ShapeObjectKind::Ellipse => {
                    let Some(hit) = shape_ellipse_hover(object, point) else {
                        continue;
                    };
                    return Some(ShapeTarget {
                        object_id: object.id.clone(),
                        object: object.clone(),
                        handle: hit
                            .active_handle
                            .unwrap_or(ShapeEditHandle::EllipseMajorPositive),
                        active_handle: hit.active_handle,
                        handles: hit.handles,
                    });
                }
                ShapeObjectKind::Rect | ShapeObjectKind::RoundRect => {
                    let Some(hit) = shape_rect_hover(object, point) else {
                        continue;
                    };
                    return Some(ShapeTarget {
                        object_id: object.id.clone(),
                        object: object.clone(),
                        handle: hit.active_handle.unwrap_or(ShapeEditHandle::NorthWest),
                        active_handle: hit.active_handle,
                        handles: hit.handles,
                    });
                }
                ShapeObjectKind::Orbital => {
                    if shape_rect_hit(object, point, true).is_none() {
                        continue;
                    }
                    return None;
                }
            }
        }
        None
    }
}

fn suggested_tlc_lane_count(width: f64) -> usize {
    ((width / 11.4).round() as isize).clamp(3, 12) as usize
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShapeObjectKind {
    Circle,
    Ellipse,
    Rect,
    RoundRect,
    Orbital,
}

struct ShapeTarget {
    object_id: String,
    object: SceneObject,
    handle: ShapeEditHandle,
    active_handle: Option<ShapeEditHandle>,
    handles: Vec<Point>,
}

struct ShapeHoverHit {
    active_handle: Option<ShapeEditHandle>,
    handles: Vec<Point>,
}

impl ShapeEditHandle {
    fn action_name(self) -> &'static str {
        match self {
            Self::CircleRadius => "circle-radius",
            Self::EllipseMajorPositive => "ellipse-major-positive",
            Self::EllipseMajorNegative => "ellipse-major-negative",
            Self::EllipseMinorPositive => "ellipse-minor-positive",
            Self::EllipseMinorNegative => "ellipse-minor-negative",
            Self::North => "n",
            Self::South => "s",
            Self::East => "e",
            Self::West => "w",
            Self::NorthEast => "ne",
            Self::NorthWest => "nw",
            Self::SouthEast => "se",
            Self::SouthWest => "sw",
        }
    }
}

fn shape_object_kind(object: &SceneObject) -> Option<ShapeObjectKind> {
    match object
        .payload
        .extra
        .get("kind")
        .and_then(JsonValue::as_str)
        .unwrap_or("rect")
    {
        "circle" => Some(ShapeObjectKind::Circle),
        "ellipse" => Some(ShapeObjectKind::Ellipse),
        "roundRect" | "round-rect" => Some(ShapeObjectKind::RoundRect),
        "rect" => Some(ShapeObjectKind::Rect),
        "orbital" => Some(ShapeObjectKind::Orbital),
        _ => None,
    }
}

fn shape_circle_hover(object: &SceneObject, point: Point) -> Option<Point> {
    let center = shape_payload_point(object, "center")?;
    let radius = center.distance(shape_payload_point(object, "majorAxisEnd")?);
    if radius <= crate::EPSILON {
        return None;
    }
    let distance = center.distance(point);
    if (distance - radius).abs() > ENDPOINT_HIT_RADIUS {
        return None;
    }
    let direction = if distance <= crate::EPSILON {
        direction_from_angle(0.0)
    } else {
        crate::Vector::new(point.x - center.x, point.y - center.y).normalized()
    };
    Some(center.translated(direction.scaled(radius)))
}

fn shape_ellipse_hover(object: &SceneObject, point: Point) -> Option<ShapeHoverHit> {
    let (center, major, minor) = shape_oval_points(object)?;
    let handles = vec![
        major,
        reflected_point(center, major),
        minor,
        reflected_point(center, minor),
    ];
    let handle_defs = [
        ShapeEditHandle::EllipseMajorPositive,
        ShapeEditHandle::EllipseMajorNegative,
        ShapeEditHandle::EllipseMinorPositive,
        ShapeEditHandle::EllipseMinorNegative,
    ];
    let active_handle = handles
        .iter()
        .zip(handle_defs)
        .filter_map(|(handle_point, handle)| {
            let distance = handle_point.distance(point);
            (distance <= ENDPOINT_HIT_RADIUS).then_some((distance, handle))
        })
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, handle)| handle);
    if active_handle.is_some() || shape_oval_hit(object, point, false).is_some() {
        return Some(ShapeHoverHit {
            active_handle,
            handles,
        });
    }
    None
}

fn shape_rect_hover(object: &SceneObject, point: Point) -> Option<ShapeHoverHit> {
    let bounds = shape_rect_bounds(object)?;
    let handles = rect_handle_points(bounds);
    let handle_defs = rect_handle_defs();
    let active_handle = handles
        .iter()
        .zip(handle_defs)
        .filter_map(|(handle_point, handle)| {
            let distance = handle_point.distance(point);
            (distance <= ENDPOINT_HIT_RADIUS).then_some((distance, handle))
        })
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, handle)| handle);
    if active_handle.is_some() || shape_rect_hit(object, point, false).is_some() {
        return Some(ShapeHoverHit {
            active_handle,
            handles,
        });
    }
    None
}

fn shape_oval_hit(object: &SceneObject, point: Point, include_fill: bool) -> Option<()> {
    let (center, major, minor) = shape_oval_points(object)?;
    let major_vector = crate::Vector::new(major.x - center.x, major.y - center.y);
    let minor_vector = crate::Vector::new(minor.x - center.x, minor.y - center.y);
    let rx = major_vector.length();
    let ry = minor_vector.length();
    if rx <= crate::EPSILON || ry <= crate::EPSILON {
        return None;
    }
    let ux = major_vector.normalized();
    let uy = minor_vector.normalized();
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    let local_x = dx * ux.x + dy * ux.y;
    let local_y = dx * uy.x + dy * uy.y;
    let normalized = (local_x / rx).powi(2) + (local_y / ry).powi(2);
    if include_fill && normalized <= 1.0 {
        return Some(());
    }
    let radial = normalized.sqrt();
    let edge_distance = ((radial - 1.0).abs()) * rx.min(ry);
    (edge_distance <= ENDPOINT_HIT_RADIUS).then_some(())
}

fn shape_rect_hit(object: &SceneObject, point: Point, include_fill: bool) -> Option<()> {
    let bounds = shape_rect_bounds(object)?;
    if include_fill
        && point.x >= bounds[0]
        && point.x <= bounds[2]
        && point.y >= bounds[1]
        && point.y <= bounds[3]
    {
        return Some(());
    }
    let on_vertical = (point.x - bounds[0]).abs() <= ENDPOINT_HIT_RADIUS
        || (point.x - bounds[2]).abs() <= ENDPOINT_HIT_RADIUS;
    let on_horizontal = (point.y - bounds[1]).abs() <= ENDPOINT_HIT_RADIUS
        || (point.y - bounds[3]).abs() <= ENDPOINT_HIT_RADIUS;
    let within_y =
        point.y >= bounds[1] - ENDPOINT_HIT_RADIUS && point.y <= bounds[3] + ENDPOINT_HIT_RADIUS;
    let within_x =
        point.x >= bounds[0] - ENDPOINT_HIT_RADIUS && point.x <= bounds[2] + ENDPOINT_HIT_RADIUS;
    ((on_vertical && within_y) || (on_horizontal && within_x)).then_some(())
}

fn resized_shape_object_from_handle(
    original: &SceneObject,
    handle: ShapeEditHandle,
    point: Point,
) -> Option<SceneObject> {
    let kind = shape_object_kind(original)?;
    match kind {
        ShapeObjectKind::Circle => resized_circle_object(original, point),
        ShapeObjectKind::Ellipse => resized_ellipse_object(original, handle, point),
        ShapeObjectKind::Rect | ShapeObjectKind::RoundRect => {
            resized_rect_object(original, handle, point)
        }
        ShapeObjectKind::Orbital => None,
    }
}

fn resized_circle_object(original: &SceneObject, point: Point) -> Option<SceneObject> {
    let center = shape_payload_point(original, "center")?;
    let radius = center.distance(point);
    if radius <= crate::EPSILON {
        return None;
    }
    let angle = angle_between(center, point);
    let major = point;
    let minor = center.translated(direction_from_angle(angle + 90.0).scaled(radius));
    let mut object = original.clone();
    object.payload.bbox = Some([
        round2(center.x - radius),
        round2(center.y - radius),
        round2(radius * 2.0),
        round2(radius * 2.0),
    ]);
    set_shape_point(&mut object, "majorAxisEnd", major);
    set_shape_point(&mut object, "minorAxisEnd", minor);
    Some(object)
}

fn resized_ellipse_object(
    original: &SceneObject,
    handle: ShapeEditHandle,
    point: Point,
) -> Option<SceneObject> {
    let (center, major, minor) = shape_oval_points(original)?;
    let mut next_major = major;
    let mut next_minor = minor;
    match handle {
        ShapeEditHandle::EllipseMajorPositive => next_major = point,
        ShapeEditHandle::EllipseMajorNegative => next_major = reflected_point(center, point),
        ShapeEditHandle::EllipseMinorPositive => next_minor = point,
        ShapeEditHandle::EllipseMinorNegative => next_minor = reflected_point(center, point),
        _ => return None,
    }
    if center.distance(next_major) <= crate::EPSILON
        || center.distance(next_minor) <= crate::EPSILON
    {
        return None;
    }
    let mut object = original.clone();
    set_shape_point(&mut object, "majorAxisEnd", next_major);
    set_shape_point(&mut object, "minorAxisEnd", next_minor);
    let rx = center.distance(next_major);
    let ry = center.distance(next_minor);
    object.payload.bbox = Some([
        round2(center.x - rx),
        round2(center.y - ry),
        round2(rx * 2.0),
        round2(ry * 2.0),
    ]);
    Some(object)
}

fn resized_rect_object(
    original: &SceneObject,
    handle: ShapeEditHandle,
    point: Point,
) -> Option<SceneObject> {
    let bounds = shape_rect_bounds(original)?;
    let min_size = crate::px_to_cm(4.0);
    let mut left = bounds[0];
    let mut top = bounds[1];
    let mut right = bounds[2];
    let mut bottom = bounds[3];
    match handle {
        ShapeEditHandle::West | ShapeEditHandle::NorthWest | ShapeEditHandle::SouthWest => {
            left = point.x.min(right - min_size);
        }
        ShapeEditHandle::East | ShapeEditHandle::NorthEast | ShapeEditHandle::SouthEast => {
            right = point.x.max(left + min_size);
        }
        _ => {}
    }
    match handle {
        ShapeEditHandle::North | ShapeEditHandle::NorthEast | ShapeEditHandle::NorthWest => {
            top = point.y.min(bottom - min_size);
        }
        ShapeEditHandle::South | ShapeEditHandle::SouthEast | ShapeEditHandle::SouthWest => {
            bottom = point.y.max(top + min_size);
        }
        _ => {}
    }
    let mut object = original.clone();
    object.transform.translate = [round2(left), round2(top)];
    object.payload.bbox = Some([0.0, 0.0, round2(right - left), round2(bottom - top)]);
    if shape_object_kind(original) == Some(ShapeObjectKind::RoundRect) {
        let radius = ROUND_RECT_CORNER_RADIUS
            .min((right - left) * 0.5)
            .min((bottom - top) * 0.5);
        object
            .payload
            .extra
            .insert("cornerRadius".to_string(), json!(round2(radius)));
    }
    Some(object)
}

fn shape_payload_point(object: &SceneObject, key: &str) -> Option<Point> {
    object
        .payload
        .extra
        .get(key)
        .and_then(JsonValue::as_array)
        .and_then(|coords| {
            Some(Point::new(
                coords.first()?.as_f64()?,
                coords.get(1)?.as_f64()?,
            ))
        })
}

fn shape_oval_points(object: &SceneObject) -> Option<(Point, Point, Point)> {
    Some((
        shape_payload_point(object, "center")?,
        shape_payload_point(object, "majorAxisEnd")?,
        shape_payload_point(object, "minorAxisEnd")?,
    ))
}

fn shape_rect_bounds(object: &SceneObject) -> Option<[f64; 4]> {
    let [x, y, width, height] = object.payload.bbox?;
    if width <= crate::EPSILON || height <= crate::EPSILON {
        return None;
    }
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    Some([tx + x, ty + y, tx + x + width, ty + y + height])
}

fn reflected_point(center: Point, point: Point) -> Point {
    Point::new(center.x * 2.0 - point.x, center.y * 2.0 - point.y)
}

fn rect_handle_points(bounds: [f64; 4]) -> Vec<Point> {
    let [left, top, right, bottom] = bounds;
    let mid_x = (left + right) * 0.5;
    let mid_y = (top + bottom) * 0.5;
    vec![
        Point::new(left, top),
        Point::new(mid_x, top),
        Point::new(right, top),
        Point::new(right, mid_y),
        Point::new(right, bottom),
        Point::new(mid_x, bottom),
        Point::new(left, bottom),
        Point::new(left, mid_y),
    ]
}

fn rect_handle_defs() -> [ShapeEditHandle; 8] {
    [
        ShapeEditHandle::NorthWest,
        ShapeEditHandle::North,
        ShapeEditHandle::NorthEast,
        ShapeEditHandle::East,
        ShapeEditHandle::SouthEast,
        ShapeEditHandle::South,
        ShapeEditHandle::SouthWest,
        ShapeEditHandle::West,
    ]
}

fn set_shape_point(object: &mut SceneObject, key: &str, point: Point) {
    object
        .payload
        .extra
        .insert(key.to_string(), json!([round2(point.x), round2(point.y)]));
}
