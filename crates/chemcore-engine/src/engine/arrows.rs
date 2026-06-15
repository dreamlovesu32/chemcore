use super::*;
use serde_json::json;

fn round_point(point: Point) -> Point {
    Point::new(crate::round2(point.x), crate::round2(point.y))
}

pub(super) fn arrow_drag_end(start: Point, point: Point, alt_key: bool) -> Point {
    if alt_key {
        return point;
    }
    let dx = point.x - start.x;
    let dy = point.y - start.y;
    let length = (dx * dx + dy * dy).sqrt();
    if length <= crate::EPSILON {
        return point;
    }
    let angle = nearest_angle(crate::angle_between(start, point), GLOBAL_SNAP_ANGLES);
    let direction = crate::direction_from_angle(angle);
    start.translated(direction.scaled(length))
}

fn arrow_head_dimensions(size: ArrowHeadSize, bold: bool) -> (f64, f64, f64) {
    let (length, center_length, width) = match size {
        ArrowHeadSize::Large => (22.5, 19.69, 5.63),
        ArrowHeadSize::Medium => (15.0, 13.13, 3.75),
        ArrowHeadSize::Small => (10.0, 8.75, 2.5),
    };
    if bold {
        (length * 2.0, center_length * 2.0, width * 2.0)
    } else {
        (length, center_length, width)
    }
}

fn open_arrow_head_dimensions(size: ArrowHeadSize, bold: bool) -> (f64, f64, f64) {
    let (length, center_length, width) = match size {
        ArrowHeadSize::Large => (12.0, 12.0, 3.0),
        ArrowHeadSize::Medium | ArrowHeadSize::Small => (6.0, 6.0, 1.5),
    };
    if bold {
        (length * 2.0, center_length * 2.0, width * 2.0)
    } else {
        (length, center_length, width)
    }
}

fn equilibrium_arrow_shaft_spacing(_size: ArrowHeadSize, _bold: bool) -> f64 {
    3.0
}

fn unequal_equilibrium_arrow_ratio() -> f64 {
    3.0
}

pub(super) fn arrow_payload_dimensions(
    variant: ArrowVariant,
    size: ArrowHeadSize,
    bold: bool,
) -> (f64, f64, f64) {
    match variant {
        ArrowVariant::Solid => arrow_head_dimensions(size, bold),
        ArrowVariant::Curved
        | ArrowVariant::CurvedMirror
        | ArrowVariant::Equilibrium
        | ArrowVariant::UnequalEquilibrium => arrow_head_dimensions(size, bold),
        ArrowVariant::Hollow | ArrowVariant::Open => open_arrow_head_dimensions(size, bold),
    }
}

fn arrow_payload_shaft_spacing(
    variant: ArrowVariant,
    size: ArrowHeadSize,
    bold: bool,
) -> Option<f64> {
    if matches!(
        variant,
        ArrowVariant::Equilibrium | ArrowVariant::UnequalEquilibrium
    ) {
        Some(equilibrium_arrow_shaft_spacing(size, bold))
    } else {
        None
    }
}

fn arrow_payload_equilibrium_ratio(variant: ArrowVariant) -> Option<f64> {
    if variant == ArrowVariant::UnequalEquilibrium {
        Some(unequal_equilibrium_arrow_ratio())
    } else {
        None
    }
}

fn arrow_curve_degrees(curve: ArrowCurve) -> f64 {
    match curve {
        ArrowCurve::Arc270 => 270.0,
        ArrowCurve::Arc180 => 180.0,
        ArrowCurve::Arc120 => 120.0,
        ArrowCurve::Arc90 => 90.0,
    }
}

fn line_object_arrow_curve(object: &SceneObject) -> f64 {
    object
        .payload
        .extra
        .get("arrowHead")
        .and_then(|value| value.get("curve"))
        .and_then(JsonValue::as_f64)
        .unwrap_or(0.0)
}

pub(super) fn arrow_curve_sweep_degrees(variant: ArrowVariant, curve: ArrowCurve) -> f64 {
    match variant {
        ArrowVariant::Curved => -arrow_curve_degrees(curve),
        ArrowVariant::CurvedMirror => arrow_curve_degrees(curve),
        ArrowVariant::Solid
        | ArrowVariant::Hollow
        | ArrowVariant::Open
        | ArrowVariant::Equilibrium
        | ArrowVariant::UnequalEquilibrium => 0.0,
    }
}

pub(super) fn arrow_endpoint_enabled(style: ArrowEndpointStyle) -> bool {
    !matches!(style, ArrowEndpointStyle::None)
}

pub(super) fn arrow_endpoint_payload_name(style: ArrowEndpointStyle) -> &'static str {
    match style {
        ArrowEndpointStyle::None => "none",
        ArrowEndpointStyle::Full => "full",
        ArrowEndpointStyle::Left => "half-left",
        ArrowEndpointStyle::Right => "half-right",
    }
}

pub(super) fn arrow_no_go_payload_name(no_go: ArrowNoGo) -> &'static str {
    match no_go {
        ArrowNoGo::None => "none",
        ArrowNoGo::Cross => "cross",
        ArrowNoGo::Hash => "hash",
    }
}

pub(super) fn arrow_variant_name(variant: ArrowVariant) -> &'static str {
    match variant {
        ArrowVariant::Solid => "solid",
        ArrowVariant::Curved => "curved",
        ArrowVariant::CurvedMirror => "curved-mirror",
        ArrowVariant::Hollow => "hollow",
        ArrowVariant::Open => "open",
        ArrowVariant::Equilibrium => "equilibrium",
        ArrowVariant::UnequalEquilibrium => "unequal-equilibrium",
    }
}

pub(super) fn ensure_arrow_style(
    document: &mut ChemcoreDocument,
    style_id: &str,
    stroke_width: f64,
) {
    document
        .styles
        .entry(style_id.to_string())
        .or_insert_with(|| {
            json!({
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": stroke_width,
                "lineCap": "butt",
                "lineJoin": "miter"
            })
        });
}

fn arrow_icon_variant_and_size(kind: &str) -> Option<(ArrowVariant, ArrowHeadSize)> {
    let (variant, size) = match kind {
        "equilibrium-small" => (ArrowVariant::Equilibrium, ArrowHeadSize::Small),
        "equilibrium-medium" => (ArrowVariant::Equilibrium, ArrowHeadSize::Medium),
        "equilibrium-large" => (ArrowVariant::Equilibrium, ArrowHeadSize::Large),
        "unequal-equilibrium-small" => (ArrowVariant::UnequalEquilibrium, ArrowHeadSize::Small),
        "unequal-equilibrium-medium" => (ArrowVariant::UnequalEquilibrium, ArrowHeadSize::Medium),
        "unequal-equilibrium-large" => (ArrowVariant::UnequalEquilibrium, ArrowHeadSize::Large),
        _ => return None,
    };
    Some((variant, size))
}

impl Engine {
    pub fn arrow_tool_icon_svg(kind: &str) -> String {
        let Some((variant, size)) = arrow_icon_variant_and_size(kind) else {
            return String::new();
        };
        let mut engine = Engine::new();
        engine.options.graphic_stroke_width = crate::DEFAULT_BOND_STROKE;
        let mut tool = engine.state.tool.clone();
        tool.active_tool = Tool::Arrow;
        tool.arrow_variant = variant;
        tool.arrow_head_size = size;
        tool.arrow_head_style = ArrowEndpointStyle::Left;
        tool.arrow_tail_style = ArrowEndpointStyle::Left;
        tool.arrow_bold = false;
        tool.arrow_no_go = ArrowNoGo::None;
        engine.set_tool_state(tool);

        let mut document = ChemcoreDocument::blank();
        document.objects.clear();
        let style_id = "style_arrow_default".to_string();
        ensure_arrow_style(
            &mut document,
            &style_id,
            engine.options.graphic_stroke_width,
        );
        document.objects.push(engine.arrow_scene_object(
            Point::new(5.0, 12.0),
            Point::new(31.0, 12.0),
            "__arrow_tool_icon".to_string(),
            style_id,
        ));
        crate::primitives_to_svg_viewbox(
            &crate::render_document(&document),
            [0.0, 0.0, 36.0, 24.0],
            Some("chemcore-icon cc-arrow-icon cc-kernel-arrow-icon"),
        )
        .replace("#000000", "currentColor")
    }

    pub(super) fn pointer_move_arrow(&mut self, event: PointerEvent) {
        let point = event.point();
        self.state.overlay.hover_endpoint = None;
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_text_box = None;
        self.state.overlay.hover_arrow = None;
        self.state.overlay.hover_shape = None;
        if let Some(mut drag) = self.arrow_drag.take() {
            if drag.start.distance(point) >= DRAG_START_THRESHOLD {
                drag.has_dragged = true;
            }
            if drag.has_dragged {
                let end = arrow_drag_end(drag.start, point, event.alt_key);
                drag.end = Some(end);
                self.state.overlay.preview = Some(BondPreview {
                    start: drag.start,
                    end,
                });
            }
            self.arrow_drag = Some(drag);
            return;
        }
        self.state.overlay.preview = None;
        self.state.overlay.hover_arrow =
            hit_test_arrow_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS);
    }

    pub(super) fn pointer_down_arrow(&mut self, event: PointerEvent) {
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        self.arrow_drag = Some(ArrowDragState {
            start: event.point(),
            end: None,
            has_dragged: false,
        });
    }

    pub(super) fn pointer_up_arrow(&mut self, event: PointerEvent) {
        let Some(drag) = self.arrow_drag.take() else {
            return;
        };
        self.state.overlay.preview = None;
        let end = if drag.has_dragged {
            drag.end
                .unwrap_or_else(|| arrow_drag_end(drag.start, event.point(), event.alt_key))
        } else {
            return;
        };
        if drag.start.distance(end) <= crate::EPSILON {
            return;
        }
        if self.add_arrow_between(drag.start, end).is_some() {
            self.state.overlay.hover_arrow = None;
            self.state.overlay.hover_shape = None;
        }
    }

    pub(super) fn add_arrow_between(&mut self, start: Point, end: Point) -> Option<String> {
        let command = EditorCommand::AddArrow {
            begin: CommandAnchor::from(start),
            end: CommandAnchor::from(end),
            variant: self.state.tool.arrow_variant,
            head_size: self.state.tool.arrow_head_size,
            curve: self.state.tool.arrow_curve,
            head_style: self.state.tool.arrow_head_style,
            tail_style: self.state.tool.arrow_tail_style,
            head: arrow_endpoint_enabled(self.state.tool.arrow_head_style),
            tail: arrow_endpoint_enabled(self.state.tool.arrow_tail_style),
            bold: self.state.tool.arrow_bold,
            no_go: self.state.tool.arrow_no_go,
        };
        let mut object_id = None;
        let changed = self.with_command(command, |engine| {
            object_id = engine.add_arrow_between_untracked(start, end);
            object_id.is_some()
        });
        changed.then_some(object_id).flatten()
    }

    pub(super) fn add_arrow_between_untracked(
        &mut self,
        start: Point,
        end: Point,
    ) -> Option<String> {
        self.push_undo_snapshot();
        self.state.selection = SelectionState::default();
        let style_id = self.arrow_style_id();
        ensure_arrow_style(
            &mut self.state.document,
            &style_id,
            self.options.graphic_stroke_width,
        );
        let object_id = self.next_id("obj_line");
        let object = self.arrow_scene_object(start, end, object_id.clone(), style_id);
        self.state.document.objects.push(object);
        self.note_pending_select_target(PendingSelectTarget::GraphicObject(object_id.clone()));
        Some(object_id)
    }

    pub(super) fn arrow_scene_object(
        &self,
        start: Point,
        end: Point,
        object_id: String,
        style_id: String,
    ) -> SceneObject {
        let (length, center_length, width) = arrow_payload_dimensions(
            self.state.tool.arrow_variant,
            self.state.tool.arrow_head_size,
            self.state.tool.arrow_bold,
        );
        let shaft_spacing = arrow_payload_shaft_spacing(
            self.state.tool.arrow_variant,
            self.state.tool.arrow_head_size,
            self.state.tool.arrow_bold,
        );
        let equilibrium_ratio = arrow_payload_equilibrium_ratio(self.state.tool.arrow_variant);
        let head_style = self.state.tool.arrow_head_style;
        let tail_style = self.state.tool.arrow_tail_style;
        let head_enabled = arrow_endpoint_enabled(head_style);
        let tail_enabled = arrow_endpoint_enabled(tail_style);
        let mut extra = std::collections::BTreeMap::new();
        extra.insert("kind".to_string(), json!("line"));
        extra.insert(
            "points".to_string(),
            json!([
                [round_point(start).x, round_point(start).y],
                [round_point(end).x, round_point(end).y]
            ]),
        );
        extra.insert(
            "head".to_string(),
            JsonValue::String(if head_enabled { "end" } else { "none" }.to_string()),
        );
        extra.insert(
            "tail".to_string(),
            JsonValue::String(if tail_enabled { "start" } else { "none" }.to_string()),
        );
        let curve =
            arrow_curve_sweep_degrees(self.state.tool.arrow_variant, self.state.tool.arrow_curve);
        let mut arrow_head_payload = json!({
                "kind": arrow_variant_name(self.state.tool.arrow_variant),
                "curve": curve,
                "head": arrow_endpoint_payload_name(head_style),
                "tail": arrow_endpoint_payload_name(tail_style),
                "length": length,
                "centerLength": center_length,
                "width": width,
                "bold": self.state.tool.arrow_bold,
                "noGo": arrow_no_go_payload_name(self.state.tool.arrow_no_go),
        });
        if let Some(shaft_spacing) = shaft_spacing {
            if let Some(object) = arrow_head_payload.as_object_mut() {
                object.insert("shaftSpacing".to_string(), json!(shaft_spacing));
            }
        }
        if let Some(equilibrium_ratio) = equilibrium_ratio {
            if let Some(object) = arrow_head_payload.as_object_mut() {
                object.insert("equilibriumRatio".to_string(), json!(equilibrium_ratio));
            }
        }
        extra.insert("arrowHead".to_string(), arrow_head_payload);
        if let Some(geometry) = crate::default_arrow_arc_geometry_payload(start, end, curve) {
            extra.insert("arrowGeometry".to_string(), geometry);
        }
        SceneObject {
            id: object_id,
            object_type: "line".to_string(),
            name: "arrow".to_string(),
            visible: true,
            locked: false,
            z_index: 20,
            transform: crate::Transform::identity(),
            style_ref: Some(style_id),
            meta: json!({"source": "chemcore-editor"}),
            payload: crate::ObjectPayload {
                resource_ref: None,
                bbox: None,
                extra,
            },
            children: Vec::new(),
        }
    }

    pub(super) fn arrow_style_id(&self) -> String {
        if (self.options.graphic_stroke_width - crate::DEFAULT_BOND_STROKE).abs() <= crate::EPSILON
        {
            "style_arrow_default".to_string()
        } else {
            format!("style_arrow_{:.2}", self.options.graphic_stroke_width).replace('.', "_")
        }
    }

    pub fn apply_arrow_options_to_selection(
        &mut self,
        variant: ArrowVariant,
        head_size: ArrowHeadSize,
        curve: ArrowCurve,
        head_style: ArrowEndpointStyle,
        tail_style: ArrowEndpointStyle,
        head: bool,
        tail: bool,
        bold: bool,
        no_go: ArrowNoGo,
    ) -> bool {
        let object_ids = self.state.selection.arrow_objects.clone();
        if object_ids.is_empty() {
            return false;
        }
        let command = EditorCommand::ApplyArrowStyle {
            object_ids,
            variant,
            head_size,
            curve,
            head_style,
            tail_style,
            head,
            tail,
            bold,
            no_go,
        };
        self.with_command(command, |engine| {
            engine.apply_arrow_options_to_selection_untracked(
                variant, head_size, curve, head_style, tail_style, bold, no_go,
            )
        })
    }

    pub(super) fn apply_arrow_options_to_selection_untracked(
        &mut self,
        variant: ArrowVariant,
        head_size: ArrowHeadSize,
        curve: ArrowCurve,
        head_style: ArrowEndpointStyle,
        tail_style: ArrowEndpointStyle,
        bold: bool,
        no_go: ArrowNoGo,
    ) -> bool {
        let selected: std::collections::BTreeSet<String> =
            self.state.selection.arrow_objects.iter().cloned().collect();
        if selected.is_empty() {
            return false;
        }
        let (length, center_length, width) = arrow_payload_dimensions(variant, head_size, bold);
        let shaft_spacing = arrow_payload_shaft_spacing(variant, head_size, bold);
        let equilibrium_ratio = arrow_payload_equilibrium_ratio(variant);
        let mut updates = Vec::new();
        for (index, object) in self.state.document.objects.iter().enumerate() {
            if object.object_type != "line" || !selected.contains(&object.id) {
                continue;
            }
            let mut next_extra = object.payload.extra.clone();
            next_extra.insert(
                "head".to_string(),
                JsonValue::String(
                    if arrow_endpoint_enabled(head_style) {
                        "end"
                    } else {
                        "none"
                    }
                    .to_string(),
                ),
            );
            next_extra.insert(
                "tail".to_string(),
                JsonValue::String(
                    if arrow_endpoint_enabled(tail_style) {
                        "start"
                    } else {
                        "none"
                    }
                    .to_string(),
                ),
            );
            let curve_degrees = if matches!(
                variant,
                ArrowVariant::Hollow | ArrowVariant::Equilibrium | ArrowVariant::UnequalEquilibrium
            ) {
                line_object_arrow_curve(object)
            } else {
                arrow_curve_sweep_degrees(variant, curve)
            };
            let mut arrow_head_payload = json!({
                    "kind": arrow_variant_name(variant),
                    "curve": curve_degrees,
                    "head": arrow_endpoint_payload_name(head_style),
                    "tail": arrow_endpoint_payload_name(tail_style),
                    "length": length,
                    "centerLength": center_length,
                    "width": width,
                    "bold": bold,
                    "noGo": arrow_no_go_payload_name(no_go),
            });
            if let Some(shaft_spacing) = shaft_spacing {
                if let Some(object) = arrow_head_payload.as_object_mut() {
                    object.insert("shaftSpacing".to_string(), json!(shaft_spacing));
                }
            }
            if let Some(equilibrium_ratio) = equilibrium_ratio {
                if let Some(object) = arrow_head_payload.as_object_mut() {
                    object.insert("equilibriumRatio".to_string(), json!(equilibrium_ratio));
                }
            }
            next_extra.insert("arrowHead".to_string(), arrow_head_payload);
            if let Some((start, end)) = crate::arrow_payload_line_endpoints(&next_extra) {
                if let Some(geometry) =
                    crate::default_arrow_arc_geometry_payload(start, end, curve_degrees)
                {
                    next_extra.insert("arrowGeometry".to_string(), geometry);
                } else {
                    next_extra.remove("arrowGeometry");
                }
            }
            if object.payload.extra != next_extra
                || object.style_ref.as_deref() != Some("style_arrow_default")
            {
                updates.push((index, next_extra));
            }
        }
        if updates.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        for (index, next_extra) in updates {
            if let Some(object) = self.state.document.objects.get_mut(index) {
                object.payload.extra = next_extra;
                object.style_ref = Some("style_arrow_default".to_string());
            }
        }
        self.state.overlay.hover_arrow = None;
        self.state.overlay.hover_shape = None;
        true
    }
}
