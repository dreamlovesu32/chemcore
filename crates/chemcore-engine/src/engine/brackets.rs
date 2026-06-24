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

    pub(super) fn bracket_preview_document(&self) -> Option<ChemcoreDocument> {
        let drag = self.bracket_drag.as_ref()?;
        if !drag.has_dragged {
            return None;
        }
        let mut document = self.state.document.clone();
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
        let mut extra = BTreeMap::new();
        extra.insert(
            "kind".to_string(),
            json!(bracket_kind_name(self.state.tool.bracket_kind)),
        );
        extra.insert("stroke".to_string(), json!("#000000"));
        extra.insert(
            "strokeWidth".to_string(),
            json!(self.options.graphic_stroke_world_pt().value()),
        );
        extra.insert("lipSize".to_string(), json!(60));
        Some(SceneObject {
            id: object_id,
            object_type: "bracket".to_string(),
            name: "bracket".to_string(),
            visible: true,
            locked: false,
            z_index: self.next_shape_z_index(),
            transform: crate::Transform {
                translate: [x1, y1],
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
}
