use super::*;

fn bracket_kind_name(kind: crate::BracketKind) -> &'static str {
    match kind {
        crate::BracketKind::Round => "round",
        crate::BracketKind::Square => "square",
        crate::BracketKind::Curly => "curly",
        crate::BracketKind::DoubleDagger => "double-dagger",
        crate::BracketKind::Dagger => "dagger",
        crate::BracketKind::CirclePlus => "circle-plus",
        crate::BracketKind::Plus => "plus",
        crate::BracketKind::RadicalCation => "radical-cation",
        crate::BracketKind::LonePair => "lone-pair",
        crate::BracketKind::CircleMinus => "circle-minus",
        crate::BracketKind::Minus => "minus",
        crate::BracketKind::RadicalAnion => "radical-anion",
        crate::BracketKind::Electron => "electron",
    }
}

fn bracket_symbol_metrics(kind: crate::BracketKind, line_width: f64) -> crate::CdxmlSymbolMetrics {
    crate::cdxml_symbol_metrics_for_line_width(bracket_kind_name(kind), line_width)
}

fn symbol_orbit_point(anchor: SymbolOrbitAnchor, pointer: Point) -> Point {
    let angle = angle_between(anchor.point, pointer).to_radians();
    let (rx, ry) = match anchor.mode {
        SymbolOrbitMode::Endpoint => (13.0, 13.0),
        SymbolOrbitMode::Label => (13.0, 8.0),
    };
    Point::new(
        anchor.point.x + angle.cos() * rx,
        anchor.point.y + angle.sin() * ry,
    )
}

impl Engine {
    pub fn symbol_tool_icon_svg(kind: crate::BracketKind) -> String {
        let mut engine = Engine::new();
        let mut tool = engine.state.tool.clone();
        tool.active_tool = Tool::Symbol;
        tool.symbol_kind = kind;
        engine.set_tool_state(tool);

        let object =
            engine.bracket_symbol_scene_object(Point::new(12.0, 12.0), "__symbol_icon".to_string());
        let mut document = engine.state.document.clone();
        document.objects.push(object);
        let primitives = crate::render_document(&document);
        crate::primitives_to_svg_viewbox(
            &primitives,
            [4.5, 4.5, 15.0, 15.0],
            Some("chemcore-icon cc-symbol-icon"),
        )
        .replace("#000000", "currentColor")
    }

    pub(super) fn pointer_down_bracket(&mut self, event: PointerEvent) {
        let point = event.point();
        if self.begin_hover_shape_edit(point) != "" {
            return;
        }
        self.clear_interaction();
        self.state.selection = SelectionState::default();
        self.bracket_drag = Some(BracketDragState {
            start: point,
            current: point,
            symbol_anchor: None,
            has_dragged: false,
        });
    }

    pub(super) fn pointer_move_bracket(&mut self, event: PointerEvent) {
        let point = event.point();
        self.state.overlay = OverlayState::default();
        if let Some(mut drag) = self.bracket_drag.take() {
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
            self.bracket_drag = Some(drag);
            return;
        }
        self.state.overlay.hover_endpoint = None;
        self.state.overlay.hover_text_box = None;
        self.state.overlay.hover_arrow = None;
        self.state.overlay.hover_shape = None;
        self.state.overlay.preview = None;
        self.state.overlay.hover_shape = self.bracket_hover_at_point(point);
        if self.state.overlay.hover_shape.is_some() {
            return;
        }
        self.state.overlay.hover_bond_center =
            hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS);
    }

    pub(super) fn pointer_up_bracket(&mut self, event: PointerEvent) {
        let Some(mut drag) = self.bracket_drag.take() else {
            return;
        };
        drag.current = event.point();
        if drag.start.distance(drag.current) < DRAG_START_THRESHOLD {
            self.state.overlay = OverlayState::default();
            return;
        }
        drag.has_dragged = true;
        let command = EditorCommand::AddBracket {
            kind: self.state.tool.bracket_kind,
            begin: CommandAnchor::from(drag.start),
            end: CommandAnchor::from(drag.current),
        };
        self.with_command(command, |engine| engine.insert_bracket_from_drag(&drag));
        self.state.overlay = OverlayState::default();
    }

    pub(super) fn pointer_down_symbol(&mut self, event: PointerEvent) {
        let point = event.point();
        let symbol_anchor = self.symbol_orbit_anchor_at(point);
        self.clear_interaction();
        self.state.selection = SelectionState::default();
        self.bracket_drag = Some(BracketDragState {
            start: point,
            current: point,
            symbol_anchor,
            has_dragged: false,
        });
    }

    pub(super) fn pointer_move_symbol(&mut self, event: PointerEvent) {
        let point = event.point();
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_arrow = None;
        self.state.overlay.hover_shape = None;
        self.state.overlay.hover_endpoint = None;
        self.state.overlay.hover_text_box = None;
        self.state.overlay.preview = None;
        if let Some(mut drag) = self.bracket_drag.take() {
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
            self.bracket_drag = Some(drag);
            return;
        }
        self.hover_symbol_target(point);
    }

    pub(super) fn pointer_up_symbol(&mut self, event: PointerEvent) {
        let Some(mut drag) = self.bracket_drag.take() else {
            return;
        };
        drag.current = event.point();
        let point = self.bracket_symbol_insert_point(&drag);
        let command = EditorCommand::AddSymbol {
            kind: self.state.tool.symbol_kind,
            center: CommandAnchor::from(point),
        };
        self.with_command(command, |engine| engine.insert_bracket_symbol(point));
        self.state.overlay = OverlayState::default();
    }

    pub(super) fn bracket_preview_overlay_document(&self) -> Option<ChemcoreDocument> {
        let drag = self.bracket_drag.as_ref()?;
        if !drag.has_dragged {
            return None;
        }
        let mut document = self.preview_document_shell();
        if self.state.tool.active_tool == Tool::Symbol {
            document.objects.push(self.bracket_symbol_scene_object(
                self.bracket_symbol_insert_point(drag),
                "__preview_symbol".to_string(),
            ));
        } else {
            document.objects.push(self.bracket_scene_object(
                drag.start,
                drag.current,
                "__preview_bracket".to_string(),
            )?);
        }
        Some(document)
    }

    pub(super) fn insert_bracket_from_drag(&mut self, drag: &BracketDragState) -> bool {
        let object_id = self.next_id("obj_bracket");
        let Some(object) = self.bracket_scene_object(drag.start, drag.current, object_id.clone())
        else {
            return false;
        };
        self.push_undo_snapshot();
        self.state.document.objects.push(object);
        refresh_repeating_units(&mut self.state.document);
        self.note_pending_select_target(PendingSelectTarget::GraphicObject(object_id));
        true
    }

    pub(super) fn insert_bracket_symbol(&mut self, point: Point) -> bool {
        let object_id = self.next_id("obj_symbol");
        let object = self.bracket_symbol_scene_object(point, object_id.clone());
        self.push_undo_snapshot();
        self.state.document.objects.push(object);
        self.refresh_symbol_chemistry();
        self.note_pending_select_target(PendingSelectTarget::GraphicObject(object_id));
        true
    }

    pub(super) fn refresh_symbol_chemistry(&mut self) -> bool {
        let mut changed = crate::refresh_attached_electron_symbols(&mut self.state.document);
        if changed {
            if let Some(mut entry) = self.state.document.editable_fragment_mut() {
                refresh_attached_node_label_geometry_for_all_nodes(
                    entry.fragment,
                    entry.object.transform.translate,
                    self.options.bond_stroke_world_pt().value(),
                );
                entry.update_bounds();
            }
            changed |= crate::refresh_attached_electron_symbols(&mut self.state.document);
            changed |= refresh_repeating_units(&mut self.state.document);
        }
        changed
    }

    pub(super) fn bracket_scene_object(
        &self,
        start: Point,
        current: Point,
        object_id: String,
    ) -> Option<SceneObject> {
        let x1 = start.x.min(current.x);
        let y1 = start.y.min(current.y);
        let width = (current.x - start.x).abs();
        let height = (current.y - start.y).abs();
        if width <= crate::EPSILON || height <= crate::EPSILON {
            return None;
        }
        let kind = bracket_kind_name(self.state.tool.bracket_kind);
        let stroke_width = self.options.graphic_stroke_world_pt().value();
        let z_index = self.next_shape_z_index();
        let left_id = format!("{object_id}_left");
        let right_id = format!("{object_id}_right");
        let left = bracket_side_scene_object(
            left_id,
            BracketSide::Left,
            kind,
            stroke_width,
            x1,
            y1,
            width,
            height,
            z_index,
        );
        let right = bracket_side_scene_object(
            right_id,
            BracketSide::Right,
            kind,
            stroke_width,
            x1,
            y1,
            width,
            height,
            z_index + 1,
        );
        Some(SceneObject {
            id: object_id,
            object_type: "group".to_string(),
            name: "bracket-group".to_string(),
            visible: true,
            locked: false,
            z_index,
            transform: crate::Transform {
                translate: [0.0, 0.0],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: None,
            meta: json!({
                "source": "editor",
                "kind": "bracket-group",
            }),
            payload: crate::ObjectPayload {
                resource_ref: None,
                bbox: Some([round2(x1), round2(y1), round2(width), round2(height)]),
                extra: BTreeMap::new(),
            },
            children: vec![left, right],
        })
    }

    pub(super) fn bracket_symbol_scene_object(
        &self,
        point: Point,
        object_id: String,
    ) -> SceneObject {
        let metrics = bracket_symbol_metrics(
            self.state.tool.symbol_kind,
            self.options.graphic_stroke_world_pt().value(),
        );
        let (width, height) = (metrics.width, metrics.height);
        let mut extra = BTreeMap::new();
        let style = crate::cdxml_symbol_style_from_line_width(metrics.line_width);
        extra.insert(
            "kind".to_string(),
            json!(bracket_kind_name(self.state.tool.symbol_kind)),
        );
        extra.insert("fill".to_string(), json!("#000000"));
        extra.insert(
            "symbolStyle".to_string(),
            json!(crate::cdxml_symbol_style_name(style)),
        );
        extra.insert(
            "symbolAnchorWidth".to_string(),
            json!(metrics.cdxml_anchor_width),
        );
        extra.insert(
            "symbolAnchorHeight".to_string(),
            json!(metrics.cdxml_anchor_height),
        );
        extra.insert("symbolLineWidth".to_string(), json!(metrics.line_width));
        if let Some(stroke_width) = metrics.stroke_width {
            extra.insert("strokeWidth".to_string(), json!(stroke_width));
        }
        SceneObject {
            id: object_id,
            object_type: "symbol".to_string(),
            name: "symbol".to_string(),
            visible: true,
            locked: false,
            z_index: self.next_shape_z_index(),
            transform: crate::Transform {
                translate: [point.x - width * 0.5, point.y - height * 0.5],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: None,
            meta: json!({
                "source": "editor",
            }),
            payload: crate::ObjectPayload {
                resource_ref: None,
                bbox: Some([0.0, 0.0, width, height]),
                extra,
            },
            children: Vec::new(),
        }
    }

    pub(super) fn bracket_symbol_insert_point(&self, drag: &BracketDragState) -> Point {
        if drag.has_dragged {
            if let Some(anchor) = drag.symbol_anchor {
                return symbol_orbit_point(anchor, drag.current);
            }
            return drag.current;
        }
        self.bracket_symbol_click_insert_point(drag.current)
            .unwrap_or(drag.current)
    }

    pub(super) fn bracket_symbol_click_insert_point(&self, point: Point) -> Option<Point> {
        if let Some(endpoint) = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
        {
            if let Some(anchor) = endpoint.label_anchor {
                let [_, top, right, bottom] = anchor.glyph_box;
                return Some(Point::new(right + 4.0, (top + bottom) * 0.5));
            }
            return Some(self.bracket_symbol_endpoint_click_insert_point(&endpoint));
        }
        if let Some((_object_id, bounds)) = self.hit_test_text_object(point) {
            return Some(Point::new(bounds[2] + 4.0, (bounds[1] + bounds[3]) * 0.5));
        }
        None
    }

    pub(super) fn bracket_symbol_endpoint_click_insert_point(
        &self,
        endpoint: &EndpointHit,
    ) -> Point {
        let fallback = Point::new(endpoint.point.x + 6.0, endpoint.point.y - 6.0);
        let Some(entry) = self.state.document.editable_fragment() else {
            return fallback;
        };
        let directions = adjacent_directions(&entry, &endpoint.node_id);
        let angle = match directions.len() {
            1 => normalize_angle(directions[0] + 180.0),
            2 => largest_angular_gap(&directions).center,
            _ => return fallback,
        };
        endpoint.point.translated(
            direction_from_angle(angle).scaled(self.bracket_symbol_click_center_distance()),
        )
    }

    pub(super) fn bracket_symbol_click_center_distance(&self) -> f64 {
        let metrics = bracket_symbol_metrics(
            self.state.tool.symbol_kind,
            self.options.graphic_stroke_world_pt().value(),
        );
        metrics.width.max(metrics.height) * 0.5 + SYMBOL_CLICK_CLEARANCE
    }

    pub(super) fn symbol_orbit_anchor_at(&self, point: Point) -> Option<SymbolOrbitAnchor> {
        if let Some(endpoint) = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
        {
            return Some(if let Some(anchor) = endpoint.label_anchor {
                SymbolOrbitAnchor {
                    point: anchor.glyph_point,
                    mode: SymbolOrbitMode::Label,
                }
            } else {
                SymbolOrbitAnchor {
                    point: endpoint.point,
                    mode: SymbolOrbitMode::Endpoint,
                }
            });
        }
        if let Some((_node_id, bounds)) = self.hit_test_endpoint_label_box(point) {
            return Some(SymbolOrbitAnchor {
                point: Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5),
                mode: SymbolOrbitMode::Label,
            });
        }
        if let Some((_object_id, bounds)) = self.hit_test_text_object(point) {
            return Some(SymbolOrbitAnchor {
                point: Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5),
                mode: SymbolOrbitMode::Label,
            });
        }
        None
    }

    pub(super) fn hover_symbol_target(&mut self, point: Point) {
        if let Some(endpoint) = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
        {
            self.state.overlay.hover_endpoint = Some(endpoint);
            return;
        }
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
        }
    }

    pub(super) fn bracket_side_action_at_point(&self, point: Point) -> Option<&'static str> {
        self.bracket_edit_target_at_point(point)
            .map(|target| target.handle.action_name())
    }

    pub(super) fn bracket_hover_at_point(&self, point: Point) -> Option<HoverShape> {
        let target = self.bracket_hover_target_at_point(point)?;
        Some(HoverShape {
            object_id: target.object_id,
            handles: target.handles,
        })
    }

    pub(super) fn begin_bracket_side_edit(&mut self, point: Point) -> Option<&'static str> {
        let target = self.bracket_edit_target_at_point(point)?;
        let action = target.handle.action_name();
        self.bracket_edit_drag = Some(BracketEditDragState {
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
        self.shape_edit_drag = None;
        self.bracket_drag = None;
        self.state.overlay.hover_shape = None;
        self.state.overlay.preview = None;
        Some(action)
    }

    pub(super) fn update_bracket_side_edit(&mut self, point: Point, alt_key: bool) -> bool {
        let Some(mut drag) = self.bracket_edit_drag.take() else {
            return false;
        };
        if drag.start_pointer.distance(point) > crate::EPSILON {
            drag.has_dragged = true;
        }
        if drag.has_dragged {
            let Some(next_object) =
                resized_bracket_side_object(&drag.original_object, drag.handle, point, alt_key)
            else {
                self.bracket_edit_drag = Some(drag);
                return false;
            };
            if !drag.undo_pushed {
                self.push_undo_snapshot();
                drag.undo_pushed = true;
            }
            if let Some(object) = self.state.document.find_scene_object_mut(&drag.object_id) {
                *object = next_object;
                drag.changed = true;
            }
        }
        self.bracket_edit_drag = Some(drag);
        true
    }

    pub(super) fn finish_bracket_side_edit(&mut self, point: Point, alt_key: bool) -> bool {
        if self.bracket_edit_drag.is_none() {
            return false;
        }
        self.update_bracket_side_edit(point, alt_key);
        let (changed, object_id) = self
            .bracket_edit_drag
            .as_ref()
            .map(|drag| (drag.changed, drag.object_id.clone()))
            .unwrap_or((false, String::new()));
        self.bracket_edit_drag = None;
        self.clear_overlay();
        if changed {
            self.note_pending_select_target(PendingSelectTarget::GraphicObject(object_id));
        }
        changed
    }

    fn bracket_edit_target_at_point(&self, point: Point) -> Option<BracketTarget> {
        let target = self.bracket_hover_target_at_point(point)?;
        Some(BracketTarget {
            handle: target.active_handle?,
            ..target
        })
    }

    fn bracket_hover_target_at_point(&self, point: Point) -> Option<BracketTarget> {
        let mut objects = self.state.document.scene_objects();
        objects.sort_by_key(|object| object.z_index);
        for object in objects.into_iter().rev() {
            if object.object_type != "bracket" || !object.visible {
                continue;
            }
            if bracket_side(object).is_none() {
                continue;
            }
            let Some(hit) = bracket_side_hover(object, point) else {
                continue;
            };
            return Some(BracketTarget {
                object_id: object.id.clone(),
                object: object.clone(),
                handle: hit.active_handle.unwrap_or(BracketEditHandle::Bottom),
                active_handle: hit.active_handle,
                handles: hit.handles,
            });
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BracketSide {
    Left,
    Right,
}

impl BracketSide {
    fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

struct BracketTarget {
    object_id: String,
    object: SceneObject,
    handle: BracketEditHandle,
    active_handle: Option<BracketEditHandle>,
    handles: Vec<Point>,
}

struct BracketHoverHit {
    active_handle: Option<BracketEditHandle>,
    handles: Vec<Point>,
}

impl BracketEditHandle {
    pub(super) fn action_name(self) -> &'static str {
        match self {
            Self::Top => "n",
            Self::Bottom => "s",
        }
    }
}

fn bracket_side_scene_object(
    object_id: String,
    side: BracketSide,
    kind: &str,
    stroke_width: f64,
    pair_x: f64,
    pair_y: f64,
    pair_width: f64,
    pair_height: f64,
    z_index: i32,
) -> SceneObject {
    let side_width = bracket_side_width(kind, pair_width, pair_height).max(stroke_width);
    let translate_x = match side {
        BracketSide::Left => {
            if kind == "round" {
                pair_x - side_width
            } else {
                pair_x
            }
        }
        BracketSide::Right => {
            if kind == "round" {
                pair_x + pair_width
            } else {
                pair_x + pair_width - side_width
            }
        }
    };
    let mut extra = BTreeMap::new();
    extra.insert("kind".to_string(), json!(kind));
    extra.insert("side".to_string(), json!(side.name()));
    extra.insert("stroke".to_string(), json!("#000000"));
    extra.insert("strokeWidth".to_string(), json!(stroke_width));
    extra.insert("lipSize".to_string(), json!(60));
    SceneObject {
        id: object_id,
        object_type: "bracket".to_string(),
        name: format!("bracket-{}", side.name()),
        visible: true,
        locked: false,
        z_index,
        transform: crate::Transform {
            translate: [round2(translate_x), round2(pair_y)],
            rotate: 0.0,
            scale: [1.0, 1.0],
        },
        style_ref: None,
        meta: json!({
            "source": "editor",
            "bracketSide": side.name(),
        }),
        payload: crate::ObjectPayload {
            resource_ref: None,
            bbox: Some([0.0, 0.0, round2(side_width), round2(pair_height)]),
            extra,
        },
        children: Vec::new(),
    }
}

fn bracket_side_width(kind: &str, pair_width: f64, height: f64) -> f64 {
    match kind {
        "square" => (height * 0.07248).min(pair_width * 0.22).max(0.0),
        "curly" => (height * 0.14423).min(pair_width * 0.24).max(0.0),
        _ => (height * (1.0 - 3.0_f64.sqrt() * 0.5))
            .min(pair_width * 0.22)
            .max(0.0),
    }
}

fn bracket_side(object: &SceneObject) -> Option<BracketSide> {
    match object
        .payload
        .extra
        .get("side")
        .and_then(JsonValue::as_str)?
    {
        "right" => Some(BracketSide::Right),
        "left" => Some(BracketSide::Left),
        _ => None,
    }
}

fn bracket_kind(object: &SceneObject) -> &str {
    object
        .payload
        .extra
        .get("kind")
        .and_then(JsonValue::as_str)
        .unwrap_or("round")
}

fn bracket_side_hover(object: &SceneObject, point: Point) -> Option<BracketHoverHit> {
    let (handles, handle_defs) = bracket_side_handle_points(object)?;
    let nearest = handles
        .iter()
        .zip(handle_defs.iter().copied())
        .map(|(handle_point, handle)| (handle_point.distance(point), handle))
        .min_by(|left, right| left.0.total_cmp(&right.0));
    let active_handle = nearest
        .filter(|(distance, _)| *distance <= ENDPOINT_HIT_RADIUS)
        .map(|(_, handle)| handle);
    if active_handle.is_some() || bracket_side_contains_point(object, point) {
        return Some(BracketHoverHit {
            active_handle: active_handle.or_else(|| nearest.map(|(_, handle)| handle)),
            handles,
        });
    }
    None
}

fn bracket_side_handle_points(
    object: &SceneObject,
) -> Option<(Vec<Point>, Vec<BracketEditHandle>)> {
    let [x, y, width, height] = object.payload.bbox?;
    if width <= crate::EPSILON || height <= crate::EPSILON {
        return None;
    }
    let side = bracket_side(object)?;
    let kind = bracket_kind(object);
    let tx = object.transform.translate[0] + x;
    let ty = object.transform.translate[1] + y;
    let center = Point::new(tx + width * 0.5, ty + height * 0.5);
    let handle_x = bracket_side_handle_x(kind, side, width);
    let top = rotate_point_around(
        Point::new(tx + handle_x, ty),
        center,
        object.transform.rotate,
    );
    let bottom = rotate_point_around(
        Point::new(tx + handle_x, ty + height),
        center,
        object.transform.rotate,
    );
    Some((
        vec![top, bottom],
        vec![BracketEditHandle::Top, BracketEditHandle::Bottom],
    ))
}

fn bracket_side_contains_point(object: &SceneObject, point: Point) -> bool {
    let Some([x, y, width, height]) = object.payload.bbox else {
        return false;
    };
    let tx = object.transform.translate[0] + x;
    let ty = object.transform.translate[1] + y;
    let center = Point::new(tx + width * 0.5, ty + height * 0.5);
    let local = rotate_point_around(point, center, -object.transform.rotate);
    let pad = ENDPOINT_HIT_RADIUS;
    local.x >= tx - pad
        && local.x <= tx + width + pad
        && local.y >= ty - pad
        && local.y <= ty + height + pad
}

fn bracket_side_handle_x(kind: &str, side: BracketSide, width: f64) -> f64 {
    match kind {
        "square" | "curly" => match side {
            BracketSide::Left => 0.0,
            BracketSide::Right => width,
        },
        _ => match side {
            BracketSide::Left => width,
            BracketSide::Right => 0.0,
        },
    }
}

fn resized_bracket_side_object(
    original: &SceneObject,
    handle: BracketEditHandle,
    point: Point,
    alt_key: bool,
) -> Option<SceneObject> {
    let (handles, _) = bracket_side_handle_points(original)?;
    let original_top = handles[0];
    let original_bottom = handles[1];
    let (top, bottom) = match handle {
        BracketEditHandle::Top => {
            let next_top = snapped_bracket_drag_point(original_bottom, point, alt_key)?;
            (next_top, original_bottom)
        }
        BracketEditHandle::Bottom => {
            let next_bottom = snapped_bracket_drag_point(original_top, point, alt_key)?;
            (original_top, next_bottom)
        }
    };
    bracket_side_object_from_handles(original, top, bottom)
}

fn snapped_bracket_drag_point(pivot: Point, point: Point, alt_key: bool) -> Option<Point> {
    let length = pivot.distance(point);
    if length <= crate::px_to_pt(4.0) {
        return None;
    }
    let mut angle = angle_between(pivot, point);
    if !alt_key {
        angle = (angle / 15.0).round() * 15.0;
    }
    Some(pivot.translated(direction_from_angle(angle).scaled(length)))
}

fn bracket_side_object_from_handles(
    original: &SceneObject,
    top: Point,
    bottom: Point,
) -> Option<SceneObject> {
    let side = bracket_side(original)?;
    let kind = bracket_kind(original);
    let [_, _, original_width, original_height] = original.payload.bbox?;
    if original_height <= crate::EPSILON {
        return None;
    }
    let height = top.distance(bottom);
    if height <= crate::px_to_pt(4.0) {
        return None;
    }
    let ratio = (original_width / original_height).max(0.02);
    let width = (height * ratio).max(
        original
            .payload
            .extra
            .get("strokeWidth")
            .and_then(JsonValue::as_f64)
            .unwrap_or(1.0),
    );
    let rotate = normalize_angle(angle_between(top, bottom) - 90.0);
    let handle_x = bracket_side_handle_x(kind, side, width);
    let center = Point::new(width * 0.5, height * 0.5);
    let local_top = Point::new(handle_x, 0.0);
    let rotated_top_delta = rotate_point_around(local_top, center, rotate);
    let translate = Point::new(top.x - rotated_top_delta.x, top.y - rotated_top_delta.y);
    let mut object = original.clone();
    object.transform.translate = [round2(translate.x), round2(translate.y)];
    object.transform.rotate = round2(rotate);
    object.payload.bbox = Some([0.0, 0.0, round2(width), round2(height)]);
    Some(object)
}

fn rotate_point_around(point: Point, center: Point, degrees: f64) -> Point {
    let radians = degrees.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    Point::new(
        center.x + dx * cos - dy * sin,
        center.y + dx * sin + dy * cos,
    )
}
