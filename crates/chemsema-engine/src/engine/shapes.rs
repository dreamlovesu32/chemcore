use super::*;
use crate::round2;

const DEFAULT_SHAPE_CLICK_RADIUS: f64 = 7.7;
const ACS_SHAPE_CLICK_RADIUS: f64 = 7.2;
const DEFAULT_PLASMID_MAP_RADIUS: f64 = 34.0;
const DEFAULT_BIO_SHAPE_MAJOR_RADIUS: f64 = 36.0;
const DEFAULT_BIO_SHAPE_MINOR_RATIO: f64 = 2.0 / 3.0;

impl Engine {
    pub fn shape_tool_icon_svg(kind: ShapeKind, style: ShapeStyle) -> String {
        const ICON_SCALE: f64 = 2.0;
        const ICON_CONTENT_SCALE: f64 = 1.2;
        const ICON_VIEWBOX_SIZE: f64 = 24.0 * ICON_SCALE;
        let icon_point = |x: f64, y: f64| {
            Point::new(
                (12.0 + (x - 12.0) * ICON_CONTENT_SCALE) * ICON_SCALE,
                (12.0 + (y - 12.0) * ICON_CONTENT_SCALE) * ICON_SCALE,
            )
        };

        let mut engine = Engine::new();
        engine.options.graphic_stroke_width *= ICON_SCALE;
        let mut tool = engine.state.tool.clone();
        tool.active_tool = Tool::Shape;
        tool.shape_kind = kind;
        tool.shape_style = style;
        tool.shape_color = "#000000".to_string();
        engine.set_tool_state(tool);

        let style_id = "__shape_icon_style".to_string();
        let object_id = "__shape_icon".to_string();
        let (start, current) = match kind {
            ShapeKind::Circle => (icon_point(12.0, 12.0), icon_point(18.2, 12.0)),
            ShapeKind::Ellipse => (icon_point(12.0, 12.0), icon_point(19.2, 12.0)),
            ShapeKind::RoundRect | ShapeKind::Rect => {
                (icon_point(5.5, 6.2), icon_point(18.5, 17.7))
            }
            ShapeKind::TlcPlate | ShapeKind::GelPlate | ShapeKind::PlasmidMap => {
                (icon_point(5.5, 6.2), icon_point(18.5, 17.7))
            }
        };
        let Some(object) = engine.shape_scene_object(start, current, object_id, style_id.clone())
        else {
            return String::new();
        };
        let mut document = engine.state.document.clone();
        document
            .styles
            .insert(style_id, engine.pending_shape_style());
        document.objects.push(object);
        let primitives = crate::render_document(&document);
        crate::primitives_to_svg_viewbox(
            &primitives,
            [0.0, 0.0, ICON_VIEWBOX_SIZE, ICON_VIEWBOX_SIZE],
            Some("chemsema-icon cc-shape-icon"),
        )
        .replace("#000000", "currentColor")
    }

    pub fn bio_draw_tool_icon_svg(
        kind: crate::BioDrawKind,
        fill_type: crate::BioShapeFillType,
        line_type: crate::BioShapeLineType,
    ) -> String {
        let Some(shape_kind) = kind.bio_shape_kind() else {
            return Self::shape_tool_icon_svg(ShapeKind::PlasmidMap, ShapeStyle::Solid);
        };
        const ICON_SCALE: f64 = 2.0;
        let mut engine = Engine::new();
        engine.options.graphic_stroke_width *= ICON_SCALE;
        let mut tool = engine.state.tool.clone();
        tool.active_tool = Tool::BioDraw;
        tool.bio_draw_kind = kind;
        tool.bio_shape_fill_type = fill_type;
        tool.bio_shape_line_type = line_type;
        tool.shape_color = "#000000".to_string();
        engine.set_tool_state(tool);

        let style_id = "__bio_shape_icon_style".to_string();
        let object_id = "__bio_shape_icon".to_string();
        let center = Point::new(24.0, 24.0);
        let major_axis_end = Point::new(39.0, 24.0);
        let Some(object) = engine.bio_shape_scene_object(
            shape_kind,
            center,
            major_axis_end,
            object_id,
            style_id.clone(),
        ) else {
            return String::new();
        };
        let mut document = engine.state.document.clone();
        document
            .styles
            .insert(style_id, engine.pending_shape_style());
        document.objects.push(object);
        let primitives = crate::render_document(&document);
        let Some([min_x, min_y, max_x, max_y]) = crate::render_primitives_bounds(primitives.iter())
        else {
            return String::new();
        };
        let width = (max_x - min_x).max(1.0);
        let height = (max_y - min_y).max(1.0);
        let side = width.max(height) * 1.18;
        let center_x = (min_x + max_x) * 0.5;
        let center_y = (min_y + max_y) * 0.5;
        crate::primitives_to_svg_viewbox(
            &primitives,
            [center_x - side * 0.5, center_y - side * 0.5, side, side],
            Some("chemsema-icon cc-bio-draw-icon"),
        )
        .replace("#000000", "currentColor")
    }

    pub(super) fn pointer_down_shape(&mut self, event: PointerEvent) {
        let point = event.point();
        if !self.begin_hover_shape_edit(point).is_empty() {
            return;
        }
        let anchor = self.shape_draw_anchor_at_point(point);
        self.clear_interaction();
        self.state.selection = SelectionState::default();
        self.shape_drag = Some(ShapeDragState {
            pointer_start: point,
            start: anchor.point,
            current: anchor.point,
            anchor,
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
            if drag.pointer_start.distance(point) >= DRAG_START_THRESHOLD {
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
        if drag.pointer_start.distance(event.point()) < DRAG_START_THRESHOLD {
            self.state.overlay = OverlayState::default();
        } else {
            drag.has_dragged = true;
        }
        if !drag.has_dragged
            && drag.anchor.kind == ShapeDrawAnchorKind::Free
            && self.state.tool.active_tool != Tool::BioDraw
            && self.state.tool.shape_kind != ShapeKind::PlasmidMap
        {
            return;
        }
        let Some((begin, end)) = self.shape_command_points_from_drag(&drag) else {
            return;
        };
        let command = if self.state.tool.active_tool == Tool::BioDraw {
            if let Some(kind) = self.state.tool.bio_draw_kind.bio_shape_kind() {
                EditorCommand::AddBioShape {
                    kind,
                    fill_type: self.state.tool.bio_shape_fill_type,
                    line_type: self.state.tool.bio_shape_line_type,
                    color: self.state.tool.shape_color.clone(),
                    begin: CommandAnchor::from(begin),
                    end: CommandAnchor::from(end),
                }
            } else {
                EditorCommand::AddShape {
                    kind: ShapeKind::PlasmidMap,
                    style: self.state.tool.shape_style,
                    color: self.state.tool.shape_color.clone(),
                    begin: CommandAnchor::from(begin),
                    end: CommandAnchor::from(end),
                }
            }
        } else {
            EditorCommand::AddShape {
                kind: self.state.tool.shape_kind,
                style: self.state.tool.shape_style,
                color: self.state.tool.shape_color.clone(),
                begin: CommandAnchor::from(begin),
                end: CommandAnchor::from(end),
            }
        };
        self.with_command(command, |engine| engine.insert_shape_from_drag(&drag));
        self.state.overlay = OverlayState::default();
    }

    pub(super) fn shape_preview_document(&self) -> Option<ChemSemaDocument> {
        let drag = self.shape_drag.as_ref()?;
        if !drag.has_dragged {
            return None;
        }
        let mut document = self.state.document.clone();
        let style_id = "__preview_shape_style".to_string();
        document
            .styles
            .insert(style_id.clone(), self.pending_shape_style());
        document.objects.push(self.shape_scene_object_from_drag(
            drag,
            "__preview_shape".to_string(),
            style_id,
        )?);
        Some(document)
    }

    pub(super) fn shape_preview_overlay_document(&self) -> Option<ChemSemaDocument> {
        let drag = self.shape_drag.as_ref()?;
        if !drag.has_dragged {
            return None;
        }
        let mut document = self.preview_document_shell();
        let style_id = "__preview_shape_style".to_string();
        document
            .styles
            .insert(style_id.clone(), self.pending_shape_style());
        document.objects.push(self.shape_scene_object_from_drag(
            drag,
            "__preview_shape".to_string(),
            style_id,
        )?);
        Some(document)
    }

    pub(super) fn insert_shape_from_drag(&mut self, drag: &ShapeDragState) -> bool {
        let object_id = self.next_id("obj_shape");
        let style_id = format!("style_{object_id}");
        let Some(object) =
            self.shape_scene_object_from_drag(drag, object_id.clone(), style_id.clone())
        else {
            return false;
        };
        self.push_undo_snapshot();
        self.state
            .document
            .styles
            .insert(style_id, self.pending_shape_style());
        let plasmid_dialog = object.payload.plasmid_map.as_ref().map(|data| {
            json!({
                "kind": "plasmid-map",
                "mode": "insert",
                "title": "Insert Plasmid Map",
                "objectId": object_id,
                "data": data,
            })
        });
        self.state.document.objects.push(object);
        if let Some(dialog) = plasmid_dialog {
            self.pending_dialog = Some(dialog);
        }
        self.note_pending_select_target(PendingSelectTarget::GraphicObject(object_id));
        true
    }

    pub(super) fn set_shape_geometry_direct(
        &mut self,
        object_id: &str,
        start: Point,
        end: Point,
    ) -> bool {
        let Some(original) = self
            .state
            .document
            .find_scene_object(object_id)
            .filter(|object| object.object_type == "shape")
            .cloned()
        else {
            return false;
        };
        let Some(next_object) = shape_object_with_direct_geometry(&original, start, end) else {
            return false;
        };
        if serde_json::to_value(&original).ok() == serde_json::to_value(&next_object).ok() {
            return false;
        }
        self.push_undo_snapshot();
        let Some(object) = self.state.document.find_scene_object_mut(object_id) else {
            return false;
        };
        *object = next_object;
        self.note_pending_select_target(PendingSelectTarget::GraphicObject(object_id.to_string()));
        true
    }

    fn shape_scene_object_from_drag(
        &self,
        drag: &ShapeDragState,
        object_id: String,
        style_id: String,
    ) -> Option<SceneObject> {
        if drag.has_dragged {
            return self.shape_scene_object(drag.start, drag.current, object_id, style_id);
        }
        self.shape_scene_object_from_click_anchor(drag.anchor, object_id, style_id)
    }

    fn shape_scene_object_from_click_anchor(
        &self,
        anchor: ShapeDrawAnchor,
        object_id: String,
        style_id: String,
    ) -> Option<SceneObject> {
        if self.state.tool.active_tool == Tool::BioDraw {
            if self.state.tool.bio_draw_kind == crate::BioDrawKind::PlasmidMap {
                return self.shape_scene_object_from_centered_radius(
                    anchor.point,
                    DEFAULT_PLASMID_MAP_RADIUS,
                    object_id,
                    style_id,
                );
            }
            let radius = DEFAULT_BIO_SHAPE_MAJOR_RADIUS;
            return self.shape_scene_object(
                anchor.point,
                anchor
                    .point
                    .translated(direction_from_angle(0.0).scaled(radius)),
                object_id,
                style_id,
            );
        }
        match anchor.kind {
            ShapeDrawAnchorKind::Free => (self.state.tool.shape_kind == ShapeKind::PlasmidMap)
                .then(|| {
                    self.shape_scene_object_from_centered_radius(
                        anchor.point,
                        DEFAULT_PLASMID_MAP_RADIUS,
                        object_id,
                        style_id,
                    )
                })
                .flatten(),
            ShapeDrawAnchorKind::Endpoint => self.shape_scene_object_from_centered_radius(
                anchor.point,
                self.shape_click_radius(),
                object_id,
                style_id,
            ),
            ShapeDrawAnchorKind::Label => {
                let bounds = anchor.bounds?;
                match self.state.tool.shape_kind {
                    ShapeKind::Rect
                    | ShapeKind::RoundRect
                    | ShapeKind::TlcPlate
                    | ShapeKind::GelPlate
                    | ShapeKind::PlasmidMap => self.shape_scene_object(
                        Point::new(bounds[0], bounds[1]),
                        Point::new(bounds[2], bounds[3]),
                        object_id,
                        style_id,
                    ),
                    ShapeKind::Circle | ShapeKind::Ellipse => {
                        let width = (bounds[2] - bounds[0]).abs();
                        let height = (bounds[3] - bounds[1]).abs();
                        let radius = (width.max(height) * 0.5).max(crate::EPSILON);
                        self.shape_scene_object_from_centered_radius(
                            anchor.point,
                            radius,
                            object_id,
                            style_id,
                        )
                    }
                }
            }
        }
    }

    fn shape_scene_object_from_centered_radius(
        &self,
        center: Point,
        radius: f64,
        object_id: String,
        style_id: String,
    ) -> Option<SceneObject> {
        if radius <= crate::EPSILON {
            return None;
        }
        match self.state.tool.shape_kind {
            ShapeKind::Circle | ShapeKind::Ellipse => self.shape_scene_object(
                center,
                center.translated(direction_from_angle(0.0).scaled(radius)),
                object_id,
                style_id,
            ),
            ShapeKind::Rect
            | ShapeKind::RoundRect
            | ShapeKind::TlcPlate
            | ShapeKind::GelPlate
            | ShapeKind::PlasmidMap => self.shape_scene_object(
                Point::new(center.x - radius, center.y - radius),
                Point::new(center.x + radius, center.y + radius),
                object_id,
                style_id,
            ),
        }
    }

    fn shape_command_points_from_drag(&self, drag: &ShapeDragState) -> Option<(Point, Point)> {
        if drag.has_dragged {
            return Some((drag.start, drag.current));
        }
        if self.state.tool.active_tool == Tool::BioDraw {
            if self.state.tool.bio_draw_kind == crate::BioDrawKind::PlasmidMap {
                let radius = DEFAULT_PLASMID_MAP_RADIUS;
                return Some((
                    Point::new(drag.anchor.point.x - radius, drag.anchor.point.y - radius),
                    Point::new(drag.anchor.point.x + radius, drag.anchor.point.y + radius),
                ));
            }
            let radius = DEFAULT_BIO_SHAPE_MAJOR_RADIUS;
            return Some((
                drag.anchor.point,
                drag.anchor
                    .point
                    .translated(direction_from_angle(0.0).scaled(radius)),
            ));
        }
        match drag.anchor.kind {
            ShapeDrawAnchorKind::Free => (self.state.tool.shape_kind == ShapeKind::PlasmidMap)
                .then(|| {
                    let radius = DEFAULT_PLASMID_MAP_RADIUS;
                    (
                        Point::new(drag.anchor.point.x - radius, drag.anchor.point.y - radius),
                        Point::new(drag.anchor.point.x + radius, drag.anchor.point.y + radius),
                    )
                }),
            ShapeDrawAnchorKind::Endpoint => {
                let radius = self.shape_click_radius();
                Some(match self.state.tool.shape_kind {
                    ShapeKind::Circle | ShapeKind::Ellipse => (
                        drag.anchor.point,
                        drag.anchor
                            .point
                            .translated(direction_from_angle(0.0).scaled(radius)),
                    ),
                    ShapeKind::Rect
                    | ShapeKind::RoundRect
                    | ShapeKind::TlcPlate
                    | ShapeKind::GelPlate
                    | ShapeKind::PlasmidMap => (
                        Point::new(drag.anchor.point.x - radius, drag.anchor.point.y - radius),
                        Point::new(drag.anchor.point.x + radius, drag.anchor.point.y + radius),
                    ),
                })
            }
            ShapeDrawAnchorKind::Label => {
                let bounds = drag.anchor.bounds?;
                Some(match self.state.tool.shape_kind {
                    ShapeKind::Circle | ShapeKind::Ellipse => {
                        let radius = ((bounds[2] - bounds[0])
                            .abs()
                            .max((bounds[3] - bounds[1]).abs())
                            * 0.5)
                            .max(crate::EPSILON);
                        (
                            drag.anchor.point,
                            drag.anchor
                                .point
                                .translated(direction_from_angle(0.0).scaled(radius)),
                        )
                    }
                    ShapeKind::Rect
                    | ShapeKind::RoundRect
                    | ShapeKind::TlcPlate
                    | ShapeKind::GelPlate
                    | ShapeKind::PlasmidMap => (
                        Point::new(bounds[0], bounds[1]),
                        Point::new(bounds[2], bounds[3]),
                    ),
                })
            }
        }
    }

    fn shape_click_radius(&self) -> f64 {
        if self.options.graphic_stroke_width <= 0.61 {
            ACS_SHAPE_CLICK_RADIUS
        } else {
            DEFAULT_SHAPE_CLICK_RADIUS
        }
    }

    pub(super) fn shape_scene_object(
        &self,
        start: Point,
        current: Point,
        object_id: String,
        style_id: String,
    ) -> Option<SceneObject> {
        if self.state.tool.active_tool == Tool::BioDraw {
            if let Some(kind) = self.state.tool.bio_draw_kind.bio_shape_kind() {
                return self.bio_shape_scene_object(kind, start, current, object_id, style_id);
            }
        }
        let (transform, bbox, extra, gel_electrophoresis) = match self.state.tool.shape_kind {
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
                    None,
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
                    None,
                )
            }
            ShapeKind::RoundRect
            | ShapeKind::Rect
            | ShapeKind::TlcPlate
            | ShapeKind::GelPlate
            | ShapeKind::PlasmidMap => {
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
                        ShapeKind::TlcPlate => "tlcPlate",
                        ShapeKind::GelPlate => "gelPlate",
                        ShapeKind::PlasmidMap => "plasmidMap",
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
                let gel_electrophoresis =
                    (self.state.tool.shape_kind == ShapeKind::GelPlate).then(|| {
                        let lane_count = suggested_tlc_lane_count(width);
                        let lanes = (0..lane_count)
                            .map(|index| crate::GelLane {
                                id: format!("lane_{}", index + 1),
                                label_text: format!("{}", index + 1),
                                visible: true,
                                bands: vec![crate::GelBand {
                                    id: format!("band_{}_1", index + 1),
                                    value: 0.5,
                                    width: (width / (lane_count as f64 + 1.0) * 0.7)
                                        .clamp(6.0, 24.0),
                                    height: 3.0,
                                    curve_type: 128,
                                    show_value: false,
                                    visible: true,
                                    color: self.state.tool.shape_color.clone(),
                                    alpha: 1.0,
                                    z_index: 0,
                                }],
                            })
                            .collect();
                        crate::GelElectrophoresisData {
                            lanes,
                            line_width: self.options.graphic_stroke_width,
                            bold_width: self.options.bold_bond_width,
                            axis_width: self.options.graphic_stroke_width,
                            margin_width: self.options.margin_width,
                            hash_spacing: self.options.hash_spacing,
                            color: self.state.tool.shape_color.clone(),
                            corners: Some([
                                [0.0, 0.0],
                                [width, 0.0],
                                [width, height],
                                [0.0, height],
                            ]),
                            ..Default::default()
                        }
                    });
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
                    gel_electrophoresis,
                )
            }
        };
        let plasmid_map =
            (self.state.tool.shape_kind == ShapeKind::PlasmidMap).then(|| crate::PlasmidMapData {
                radius: bbox[2].min(bbox[3]) * 0.5,
                line_width: self.options.graphic_stroke_width,
                bold_width: self.options.bold_bond_width,
                margin_width: self.options.margin_width,
                color: self.state.tool.shape_color.clone(),
                ..Default::default()
            });
        Some(SceneObject {
            id: object_id,
            object_type: "shape".to_string(),
            name: "shape".to_string(),
            visible: true,
            locked: false,
            z_index: self.next_shape_z_index(),
            transform,
            style_ref: Some(style_id),
            link_policy: Default::default(),
            meta: json!({
                "source": "editor",
            }),
            payload: crate::ObjectPayload {
                resource_ref: None,
                bbox: Some(bbox),
                spectrum: None,
                geometry: None,
                constraint: None,
                table: None,
                stoichiometry_grid: None,
                gel_electrophoresis,
                plasmid_map,
                bio_shape: None,
                extra,
            },
            children: Vec::new(),
        })
    }

    fn bio_shape_scene_object(
        &self,
        kind: crate::BioShapeKind,
        center: Point,
        major_axis_end: Point,
        object_id: String,
        style_id: String,
    ) -> Option<SceneObject> {
        let major_radius = center.distance(major_axis_end);
        if major_radius <= crate::EPSILON {
            return None;
        }
        let rotation = angle_between(center, major_axis_end);
        let minor_radius = major_radius * DEFAULT_BIO_SHAPE_MINOR_RATIO;
        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!("bioShape"));
        let data = crate::BioShapeData {
            kind,
            center: [0.0, 0.0, 0.0],
            major_axis_end: [major_radius, 0.0, 0.0],
            minor_axis_end: [0.0, minor_radius, 0.0],
            fill_type: self.state.tool.bio_shape_fill_type,
            line_type: self.state.tool.bio_shape_line_type,
            color: self.state.tool.shape_color.clone(),
            line_width: self.options.graphic_stroke_width,
            bold_width: self.options.bold_bond_width,
            margin_width: crate::DEFAULT_BOND_MARGIN_WIDTH_PT.value(),
            hash_spacing: self.options.hash_spacing,
            fade_percent: 10.0,
            alpha: None,
            parameters: crate::BioShapeParameters::defaults_for(kind),
        };
        Some(SceneObject {
            id: object_id,
            object_type: "shape".to_string(),
            name: format!("BioShape {}", kind.cdxml_name()),
            visible: true,
            locked: false,
            z_index: self.next_shape_z_index(),
            transform: crate::Transform {
                translate: [center.x, center.y],
                rotate: rotation,
                scale: [1.0, 1.0],
            },
            style_ref: Some(style_id),
            link_policy: Default::default(),
            meta: json!({
                "source": "editor",
            }),
            payload: crate::ObjectPayload {
                resource_ref: None,
                bbox: Some([
                    -major_radius,
                    -minor_radius,
                    major_radius * 2.0,
                    minor_radius * 2.0,
                ]),
                spectrum: None,
                geometry: None,
                constraint: None,
                table: None,
                stoichiometry_grid: None,
                gel_electrophoresis: None,
                plasmid_map: None,
                bio_shape: Some(data),
                extra,
            },
            children: Vec::new(),
        })
    }

    pub(super) fn pending_shape_style(&self) -> JsonValue {
        let color = self.state.tool.shape_color.clone();
        let stroke_width = self.options.graphic_stroke_world_pt().value();
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
        if let Some(action) = self.bracket_side_action_at_point(point) {
            return action;
        }
        self.shape_edit_target_at_point(point)
            .map(|target| target.handle.action_name())
            .unwrap_or("")
    }

    pub fn begin_hover_shape_edit(&mut self, point: Point) -> &'static str {
        if let Some(action) = self.begin_bracket_side_edit(point) {
            return action;
        }
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
        self.bracket_edit_drag = None;
        self.state.overlay.hover_shape = None;
        self.state.overlay.preview = None;
        action
    }

    pub fn update_hover_shape_edit(&mut self, point: Point, alt_key: bool) -> bool {
        let command = self.hover_shape_edit_command();
        self.with_transient_command(command, |engine| {
            if engine.bracket_edit_drag.is_some() {
                engine.update_bracket_side_edit(point, alt_key)
            } else {
                engine.update_hover_shape_edit_untracked(point)
            }
        })
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

    pub fn finish_hover_shape_edit(&mut self, point: Point, alt_key: bool) -> bool {
        let command = self.hover_shape_edit_command();
        self.with_command(command, |engine| {
            if engine.bracket_edit_drag.is_some() {
                engine.finish_bracket_side_edit(point, alt_key)
            } else {
                engine.finish_hover_shape_edit_untracked(point)
            }
        })
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
        self.clear_overlay();
        if changed {
            self.note_pending_select_target(PendingSelectTarget::GraphicObject(object_id));
        }
        changed
    }

    fn hover_shape_edit_command(&self) -> EditorCommand {
        if let Some(drag) = &self.bracket_edit_drag {
            return EditorCommand::EditShapeGeometry {
                object_id: Some(drag.object_id.clone()),
                action: drag.handle.action_name().to_string(),
            };
        }
        let (object_id, action) = self
            .shape_edit_drag
            .as_ref()
            .map(|drag| {
                (
                    Some(drag.object_id.clone()),
                    drag.handle.action_name().to_string(),
                )
            })
            .unwrap_or_else(|| (None, "unknown".to_string()));
        EditorCommand::EditShapeGeometry { object_id, action }
    }

    pub(super) fn refresh_shape_hover(&mut self, point: Point) {
        self.state.overlay.hover_shape = self
            .bracket_hover_at_point(point)
            .or_else(|| self.shape_hover_at_point(point));
        self.state.overlay.hover_endpoint = None;
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_arrow = None;
        self.state.overlay.hover_text_box = None;
        self.state.overlay.preview = None;
        if self.state.overlay.hover_shape.is_some() {
            return;
        }
        if self.state.tool.active_tool == Tool::Orbital {
            if let Some(endpoint) =
                hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
            {
                if let Some(label_anchor) = &endpoint.label_anchor {
                    self.state.overlay.hover_text_box = Some(HoverTextBox {
                        bounds: label_anchor.glyph_box,
                        object_id: None,
                        node_id: Some(endpoint.node_id),
                    });
                }
            }
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
        if let Some(endpoint) = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
        {
            if let Some(label_anchor) = &endpoint.label_anchor {
                self.state.overlay.hover_text_box = Some(HoverTextBox {
                    bounds: label_anchor.glyph_box,
                    object_id: None,
                    node_id: Some(endpoint.node_id),
                });
            }
        }
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
            ShapeObjectKind::PlasmidMap => plasmid_map_hover(object, point).is_some(),
            ShapeObjectKind::BioShape => bio_shape_hit(object, point),
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
        let orbital_tool = self.state.tool.active_tool == Tool::Orbital;
        let mut objects = self.state.document.scene_objects();
        objects.sort_by_key(|object| object.z_index);
        for object in objects.into_iter().rev() {
            if object.object_type != "shape" || !object.visible {
                continue;
            }
            let Some(kind) = shape_object_kind(object) else {
                continue;
            };
            if orbital_tool != (kind == ShapeObjectKind::Orbital) {
                continue;
            }
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
                    let Some(hit) = orbital_hover(object, point) else {
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
                ShapeObjectKind::PlasmidMap => {
                    let Some(hit) = plasmid_map_hover(object, point) else {
                        continue;
                    };
                    return Some(ShapeTarget {
                        object_id: object.id.clone(),
                        object: object.clone(),
                        handle: hit.active_handle.unwrap_or(ShapeEditHandle::CircleRadius),
                        active_handle: hit.active_handle,
                        handles: hit.handles,
                    });
                }
                ShapeObjectKind::BioShape => {
                    let Some(hit) = bio_shape_hover(object, point) else {
                        continue;
                    };
                    let Some(handle) = hit.active_handle else {
                        continue;
                    };
                    return Some(ShapeTarget {
                        object_id: object.id.clone(),
                        object: object.clone(),
                        handle,
                        active_handle: Some(handle),
                        handles: hit.handles,
                    });
                }
            }
        }
        None
    }

    fn shape_draw_anchor_at_point(&self, point: Point) -> ShapeDrawAnchor {
        if let Some((_node_id, bounds)) = self.hit_test_endpoint_label_box(point) {
            return ShapeDrawAnchor {
                kind: ShapeDrawAnchorKind::Label,
                point: Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5),
                bounds: Some(bounds),
            };
        }
        if let Some(endpoint) = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
        {
            if let Some(label_anchor) = endpoint.label_anchor {
                return ShapeDrawAnchor {
                    kind: ShapeDrawAnchorKind::Label,
                    point: Point::new(
                        (label_anchor.glyph_box[0] + label_anchor.glyph_box[2]) * 0.5,
                        (label_anchor.glyph_box[1] + label_anchor.glyph_box[3]) * 0.5,
                    ),
                    bounds: Some(label_anchor.glyph_box),
                };
            }
            return ShapeDrawAnchor {
                kind: ShapeDrawAnchorKind::Endpoint,
                point: endpoint.point,
                bounds: None,
            };
        }
        ShapeDrawAnchor {
            kind: ShapeDrawAnchorKind::Free,
            point,
            bounds: None,
        }
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
    PlasmidMap,
    BioShape,
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
            Self::PlasmidMarkerLabel(_) => "plasmid-marker-label",
            Self::PlasmidRegionStart(_) => "plasmid-region-start",
            Self::PlasmidRegionEnd(_) => "plasmid-region-end",
            Self::PlasmidRegionOffset(_) => "plasmid-region-offset",
            Self::BioReceptorWidth => "bio-receptor-width",
            Self::BioGProteinGammaShape => "bio-gprotein-gamma-shape",
            Self::BioDnaHeight => "bio-dna-height",
            Self::BioDnaSpacing => "bio-dna-spacing",
            Self::BioDnaStrandWidth => "bio-dna-strand-width",
            Self::BioDnaOffset => "bio-dna-offset",
            Self::BioHelixHeight => "bio-helix-height",
            Self::BioHelixStrandWidth => "bio-helix-strand-width",
            Self::BioHelixCylinderWidth => "bio-helix-cylinder-width",
            Self::BioHelixSpacing => "bio-helix-spacing",
            Self::BioMembraneUnitSize => "bio-membrane-unit-size",
            Self::BioMembraneArcStart => "bio-membrane-arc-start",
            Self::BioMembraneArcEnd => "bio-membrane-arc-end",
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
        "plasmidMap" | "plasmid-map" => Some(ShapeObjectKind::PlasmidMap),
        "bioShape" | "bio-shape" => Some(ShapeObjectKind::BioShape),
        _ => None,
    }
}

fn shape_object_with_direct_geometry(
    original: &SceneObject,
    start: Point,
    end: Point,
) -> Option<SceneObject> {
    let mut object = original.clone();
    match shape_object_kind(original)? {
        ShapeObjectKind::Circle => {
            let radius = start.distance(end);
            if radius <= crate::EPSILON {
                return None;
            }
            let angle = angle_between(start, end);
            let minor = start.translated(direction_from_angle(angle + 90.0).scaled(radius));
            object.transform = crate::Transform::identity();
            object.payload.bbox = Some([
                round2(start.x - radius),
                round2(start.y - radius),
                round2(radius * 2.0),
                round2(radius * 2.0),
            ]);
            set_shape_point(&mut object, "center", start);
            set_shape_point(&mut object, "majorAxisEnd", end);
            set_shape_point(&mut object, "minorAxisEnd", minor);
        }
        ShapeObjectKind::Ellipse => {
            let major_radius = start.distance(end);
            if major_radius <= crate::EPSILON {
                return None;
            }
            let ratio = shape_oval_points(original)
                .and_then(|(center, major, minor)| {
                    let major = center.distance(major);
                    let minor = center.distance(minor);
                    (major > crate::EPSILON && minor.is_finite()).then_some(minor / major)
                })
                .filter(|ratio| ratio.is_finite() && *ratio > crate::EPSILON)
                .unwrap_or(ELLIPSE_MINOR_AXIS_RATIO);
            let angle = angle_between(start, end);
            let minor =
                start.translated(direction_from_angle(angle + 90.0).scaled(major_radius * ratio));
            object.transform = crate::Transform::identity();
            object.payload.bbox = Some([
                round2(start.x - major_radius),
                round2(start.y - major_radius * ratio),
                round2(major_radius * 2.0),
                round2(major_radius * ratio * 2.0),
            ]);
            set_shape_point(&mut object, "center", start);
            set_shape_point(&mut object, "majorAxisEnd", end);
            set_shape_point(&mut object, "minorAxisEnd", minor);
        }
        ShapeObjectKind::Rect | ShapeObjectKind::RoundRect => {
            let left = start.x.min(end.x);
            let top = start.y.min(end.y);
            let width = (end.x - start.x).abs();
            let height = (end.y - start.y).abs();
            if width <= crate::EPSILON || height <= crate::EPSILON {
                return None;
            }
            object.transform.translate = [round2(left), round2(top)];
            object.transform.rotate = 0.0;
            object.transform.scale = [1.0, 1.0];
            object.payload.bbox = Some([0.0, 0.0, round2(width), round2(height)]);
            if shape_object_kind(original) == Some(ShapeObjectKind::RoundRect) {
                let radius = ROUND_RECT_CORNER_RADIUS.min(width * 0.5).min(height * 0.5);
                object
                    .payload
                    .extra
                    .insert("cornerRadius".to_string(), json!(round2(radius)));
            }
        }
        ShapeObjectKind::Orbital => return None,
        ShapeObjectKind::PlasmidMap => return None,
        ShapeObjectKind::BioShape => return None,
    }
    Some(object)
}

fn shape_circle_hover(object: &SceneObject, point: Point) -> Option<Point> {
    let center = shape_payload_point(object, "center")?;
    let radius = center.distance(shape_payload_point(object, "majorAxisEnd")?);
    if radius <= crate::EPSILON {
        return None;
    }
    let distance = center.distance(point);
    if (distance - radius).abs() > GRAPHIC_EDGE_HIT_RADIUS {
        return None;
    }
    let direction = if distance <= crate::EPSILON {
        direction_from_angle(0.0)
    } else {
        crate::Vector::new(point.x - center.x, point.y - center.y).normalized()
    };
    Some(center.translated(direction.scaled(radius)))
}

fn plasmid_map_hover(object: &SceneObject, point: Point) -> Option<ShapeHoverHit> {
    let plasmid = object.payload.plasmid_map.as_ref()?;
    let center = plasmid_map_center(object)?;
    let rotate = object.transform.rotate;
    let mut handles = Vec::with_capacity(plasmid.markers.len() + plasmid.regions.len() * 3);
    let mut candidates = Vec::new();

    for (index, marker) in plasmid.markers.iter().enumerate() {
        let angle = marker
            .label_angle
            .unwrap_or_else(|| plasmid.angle_degrees(marker.position));
        let handle = plasmid_map_point(
            center,
            plasmid.radius + marker.offset.max(plasmid.margin_width),
            angle,
            rotate,
        );
        handles.push(handle);
        candidates.push((handle, ShapeEditHandle::PlasmidMarkerLabel(index)));
    }
    for (index, region) in plasmid.regions.iter().enumerate() {
        let start = plasmid.angle_degrees(region.start);
        let end = plasmid.angle_degrees(region.end);
        let sweep = (end - start).rem_euclid(360.0);
        let radius = plasmid.radius + region.offset;
        let start_point = plasmid_map_point(center, radius, start, rotate);
        let end_point = plasmid_map_point(center, radius, end, rotate);
        let offset_point = plasmid_map_point(center, radius, start + sweep * 0.5, rotate);
        handles.extend([start_point, end_point, offset_point]);
        candidates.extend([
            (start_point, ShapeEditHandle::PlasmidRegionStart(index)),
            (end_point, ShapeEditHandle::PlasmidRegionEnd(index)),
            (offset_point, ShapeEditHandle::PlasmidRegionOffset(index)),
        ]);
    }
    let active_handle = candidates
        .into_iter()
        .filter_map(|(handle_point, handle)| {
            let distance = handle_point.distance(point);
            (distance <= ENDPOINT_HIT_RADIUS).then_some((distance, handle))
        })
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, handle)| handle);
    let ring_hit = (center.distance(point) - plasmid.radius).abs() <= GRAPHIC_EDGE_HIT_RADIUS;
    let region_hit = plasmid.regions.iter().any(|region| {
        let radial = center.distance(point);
        let radius = plasmid.radius + region.offset;
        if (radial - radius).abs() > region.width * 0.5 + GRAPHIC_EDGE_HIT_RADIUS {
            return false;
        }
        let angle = plasmid_map_angle(center, point, rotate);
        angle_in_clockwise_sweep(
            angle,
            plasmid.angle_degrees(region.start),
            plasmid.angle_degrees(region.end),
        )
    });
    if active_handle.is_some() || ring_hit || region_hit {
        Some(ShapeHoverHit {
            active_handle,
            handles,
        })
    } else {
        None
    }
}

fn edited_plasmid_map_object_from_handle(
    original: &SceneObject,
    handle: ShapeEditHandle,
    point: Point,
) -> Option<SceneObject> {
    let center = plasmid_map_center(original)?;
    let rotate = original.transform.rotate;
    let mut object = original.clone();
    let plasmid = object.payload.plasmid_map.as_mut()?;
    match handle {
        ShapeEditHandle::PlasmidMarkerLabel(index) => {
            let marker = plasmid.markers.get_mut(index)?;
            marker.offset = (center.distance(point) - plasmid.radius).max(plasmid.margin_width);
            marker.offset = round2(marker.offset);
            marker.label_angle = Some(round2(plasmid_map_angle(center, point, rotate)));
        }
        ShapeEditHandle::PlasmidRegionStart(index) => {
            let position = plasmid_map_position(plasmid, center, point, rotate);
            plasmid.regions.get_mut(index)?.start = position;
        }
        ShapeEditHandle::PlasmidRegionEnd(index) => {
            let position = plasmid_map_position(plasmid, center, point, rotate);
            plasmid.regions.get_mut(index)?.end = position;
        }
        ShapeEditHandle::PlasmidRegionOffset(index) => {
            plasmid.regions.get_mut(index)?.offset =
                round2(center.distance(point) - plasmid.radius);
        }
        _ => return None,
    }
    resize_plasmid_object_bounds(&mut object, center);
    Some(object)
}

fn plasmid_map_center(object: &SceneObject) -> Option<Point> {
    let [x, y, width, height] = object.payload.bbox?;
    Some(Point::new(
        object.transform.translate[0] + x + width * 0.5,
        object.transform.translate[1] + y + height * 0.5,
    ))
}

fn plasmid_map_point(center: Point, radius: f64, angle: f64, rotate: f64) -> Point {
    let radians = (angle + rotate).to_radians();
    Point::new(
        center.x + radius * radians.sin(),
        center.y - radius * radians.cos(),
    )
}

fn plasmid_map_angle(center: Point, point: Point, rotate: f64) -> f64 {
    ((point.x - center.x).atan2(center.y - point.y).to_degrees() - rotate).rem_euclid(360.0)
}

fn plasmid_map_position(
    plasmid: &crate::PlasmidMapData,
    center: Point,
    point: Point,
    rotate: f64,
) -> u64 {
    let angle = plasmid_map_angle(center, point, rotate);
    ((angle / 360.0 * plasmid.number_base_pairs as f64).round() as u64 + 1)
        .clamp(1, plasmid.number_base_pairs)
}

fn angle_in_clockwise_sweep(angle: f64, start: f64, end: f64) -> bool {
    let sweep = (end - start).rem_euclid(360.0);
    let relative = (angle - start).rem_euclid(360.0);
    relative <= sweep + crate::EPSILON
}

fn resize_plasmid_object_bounds(object: &mut SceneObject, center: Point) {
    let Some(plasmid) = object.payload.plasmid_map.as_ref() else {
        return;
    };
    let marker_extent = plasmid
        .markers
        .iter()
        .map(|marker| marker.offset.max(plasmid.margin_width) + plasmid.label_size * 2.0)
        .fold(0.0, f64::max);
    let region_extent = plasmid
        .regions
        .iter()
        .map(|region| region.offset + region.width * 0.5)
        .fold(0.0, f64::max);
    let extent = (plasmid.radius + marker_extent.max(region_extent))
        .max(plasmid.radius + plasmid.label_size);
    object.transform.translate = [center.x - extent, center.y - extent];
    object.payload.bbox = Some([0.0, 0.0, extent * 2.0, extent * 2.0]);
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
    (edge_distance <= GRAPHIC_EDGE_HIT_RADIUS).then_some(())
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
    let on_vertical = (point.x - bounds[0]).abs() <= GRAPHIC_EDGE_HIT_RADIUS
        || (point.x - bounds[2]).abs() <= GRAPHIC_EDGE_HIT_RADIUS;
    let on_horizontal = (point.y - bounds[1]).abs() <= GRAPHIC_EDGE_HIT_RADIUS
        || (point.y - bounds[3]).abs() <= GRAPHIC_EDGE_HIT_RADIUS;
    let within_y = point.y >= bounds[1] - GRAPHIC_EDGE_HIT_RADIUS
        && point.y <= bounds[3] + GRAPHIC_EDGE_HIT_RADIUS;
    let within_x = point.x >= bounds[0] - GRAPHIC_EDGE_HIT_RADIUS
        && point.x <= bounds[2] + GRAPHIC_EDGE_HIT_RADIUS;
    ((on_vertical && within_y) || (on_horizontal && within_x)).then_some(())
}

fn orbital_hover(object: &SceneObject, point: Point) -> Option<ShapeHoverHit> {
    let (handles, handle_defs) = orbital_handle_points(object)?;
    let active_handle = handles
        .iter()
        .zip(handle_defs.iter().copied())
        .filter_map(|(handle_point, handle)| {
            let distance = handle_point.distance(point);
            (distance <= ENDPOINT_HIT_RADIUS).then_some((distance, handle))
        })
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, handle)| handle);
    active_handle.map(|handle| ShapeHoverHit {
        active_handle: Some(handle),
        handles,
    })
}

fn orbital_handle_points(object: &SceneObject) -> Option<(Vec<Point>, Vec<ShapeEditHandle>)> {
    let template = object
        .payload
        .extra
        .get("orbitalTemplate")
        .and_then(JsonValue::as_str)
        .unwrap_or("");
    match template {
        "s" | "oval" => {
            let (center, major, minor) = shape_oval_points(object)?;
            Some((
                vec![
                    major,
                    reflected_point(center, major),
                    minor,
                    reflected_point(center, minor),
                ],
                vec![
                    ShapeEditHandle::EllipseMajorPositive,
                    ShapeEditHandle::EllipseMajorNegative,
                    ShapeEditHandle::EllipseMinorPositive,
                    ShapeEditHandle::EllipseMinorNegative,
                ],
            ))
        }
        "dxy" => {
            let start = shape_payload_point(object, "axisStart")?;
            let end = shape_payload_point(object, "axisEnd")?;
            let vector = crate::Vector::new(end.x - start.x, end.y - start.y);
            let length = vector.length();
            if length <= crate::EPSILON {
                return None;
            }
            let unit = vector.normalized();
            let minor = start.translated(crate::Vector::new(-unit.y, unit.x).scaled(length));
            Some((
                vec![
                    end,
                    reflected_point(start, end),
                    minor,
                    reflected_point(start, minor),
                ],
                vec![
                    ShapeEditHandle::EllipseMajorPositive,
                    ShapeEditHandle::EllipseMajorNegative,
                    ShapeEditHandle::EllipseMinorPositive,
                    ShapeEditHandle::EllipseMinorNegative,
                ],
            ))
        }
        "lobe" => {
            let end = shape_payload_point(object, "axisEnd")?;
            Some((vec![end], vec![ShapeEditHandle::EllipseMajorPositive]))
        }
        "hybrid" => {
            let start = shape_payload_point(object, "axisStart")?;
            let end = shape_payload_point(object, "axisEnd")?;
            let small = Point::new(
                start.x + (start.x - end.x) * 0.4,
                start.y + (start.y - end.y) * 0.4,
            );
            Some((
                vec![end, small],
                vec![
                    ShapeEditHandle::EllipseMajorPositive,
                    ShapeEditHandle::EllipseMajorNegative,
                ],
            ))
        }
        "p" | "dz2" => {
            let start = shape_payload_point(object, "axisStart")?;
            let end = shape_payload_point(object, "axisEnd")?;
            Some((
                vec![end, reflected_point(start, end)],
                vec![
                    ShapeEditHandle::EllipseMajorPositive,
                    ShapeEditHandle::EllipseMajorNegative,
                ],
            ))
        }
        _ => None,
    }
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
        ShapeObjectKind::Orbital => rotated_orbital_object_from_handle(original, handle, point),
        ShapeObjectKind::PlasmidMap => {
            edited_plasmid_map_object_from_handle(original, handle, point)
        }
        ShapeObjectKind::BioShape => edited_bio_shape_object_from_handle(original, handle, point),
    }
}

fn bio_shape_hit(object: &SceneObject, point: Point) -> bool {
    let Some(local) = bio_shape_world_to_local(object, point) else {
        return false;
    };
    let Some(data) = object.payload.bio_shape.as_ref() else {
        return false;
    };
    let center = Point::new(data.center[0], data.center[1]);
    let major = center.distance(Point::new(data.major_axis_end[0], data.major_axis_end[1]));
    let minor = center.distance(Point::new(data.minor_axis_end[0], data.minor_axis_end[1]));
    local.x >= center.x - major
        && local.x <= center.x + major
        && local.y >= center.y - minor
        && local.y <= center.y + minor
}

fn bio_shape_hover(object: &SceneObject, point: Point) -> Option<ShapeHoverHit> {
    if !bio_shape_hit(object, point) {
        return None;
    }
    let definitions = bio_shape_handle_definitions(object)?;
    let handles: Vec<Point> = definitions.iter().map(|(point, _)| *point).collect();
    let active_handle = definitions
        .into_iter()
        .filter_map(|(handle_point, handle)| {
            let distance = handle_point.distance(point);
            (distance <= ENDPOINT_HIT_RADIUS).then_some((distance, handle))
        })
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, handle)| handle);
    Some(ShapeHoverHit {
        active_handle,
        handles,
    })
}

fn bio_shape_handle_definitions(object: &SceneObject) -> Option<Vec<(Point, ShapeEditHandle)>> {
    let data = object.payload.bio_shape.as_ref()?;
    let parameters = data.parameters.resolved_for(data.kind);
    let p = &parameters;
    let local = |u: f64, v: f64| bio_shape_normalized_to_world(object, u, v);
    let center = Point::new(data.center[0], data.center[1]);
    let major_radius = center.distance(Point::new(data.major_axis_end[0], data.major_axis_end[1]));
    let minor_radius = center.distance(Point::new(data.minor_axis_end[0], data.minor_axis_end[1]));
    if major_radius <= crate::EPSILON || minor_radius <= crate::EPSILON {
        return None;
    }
    let definitions = match data.kind {
        crate::BioShapeKind::Receptor => vec![(
            local(p.neck_width? / 100.0, -0.48)?,
            ShapeEditHandle::BioReceptorWidth,
        )],
        crate::BioShapeKind::GProteinGamma => vec![(
            local(0.2, 0.65 + p.gprotein_upper_height? / 500.0)?,
            ShapeEditHandle::BioGProteinGammaShape,
        )],
        crate::BioShapeKind::Dna => {
            let wave_height = p.dna_wave_height?;
            let wave_length = p.dna_wave_length?;
            let wave_width = p.dna_wave_width?;
            let wave_offset = p.dna_wave_offset?;
            vec![
                (
                    local(-0.82, wave_height / (minor_radius * 2.0))?,
                    ShapeEditHandle::BioDnaHeight,
                ),
                (
                    local(-wave_length / (major_radius * 2.0), 0.0)?,
                    ShapeEditHandle::BioDnaSpacing,
                ),
                (
                    local(-0.82, wave_width / minor_radius)?,
                    ShapeEditHandle::BioDnaStrandWidth,
                ),
                (
                    local(-0.82, -wave_offset / minor_radius)?,
                    ShapeEditHandle::BioDnaOffset,
                ),
            ]
        }
        crate::BioShapeKind::HelixProtein => {
            let cylinder_height = p.cylinder_height?;
            let pipe_width = p.pipe_width?;
            let cylinder_width = p.cylinder_width?;
            let cylinder_distance = p.cylinder_distance?;
            vec![
                (
                    local(-0.82, cylinder_height / (minor_radius * 2.0))?,
                    ShapeEditHandle::BioHelixHeight,
                ),
                (
                    local(-0.82, -pipe_width / minor_radius)?,
                    ShapeEditHandle::BioHelixStrandWidth,
                ),
                (
                    local(-cylinder_width / major_radius, 0.30)?,
                    ShapeEditHandle::BioHelixCylinderWidth,
                ),
                (
                    local(-cylinder_distance / major_radius, 0.0)?,
                    ShapeEditHandle::BioHelixSpacing,
                ),
            ]
        }
        crate::BioShapeKind::MembraneLine
        | crate::BioShapeKind::MembraneEllipse
        | crate::BioShapeKind::MembraneMicelle => {
            let element_size = p.membrane_element_size?;
            vec![(
                local(0.0, element_size / (minor_radius * 2.0))?,
                ShapeEditHandle::BioMembraneUnitSize,
            )]
        }
        crate::BioShapeKind::MembraneArc => {
            let start = p.membrane_start_angle?.to_radians();
            let end = p.membrane_end_angle?.to_radians();
            let element_size = p.membrane_element_size?;
            vec![
                (
                    local(0.78 * start.cos(), 0.78 * start.sin())?,
                    ShapeEditHandle::BioMembraneArcStart,
                ),
                (
                    local(0.78 * end.cos(), 0.78 * end.sin())?,
                    ShapeEditHandle::BioMembraneArcEnd,
                ),
                (
                    local(0.0, element_size / (minor_radius * 2.0))?,
                    ShapeEditHandle::BioMembraneUnitSize,
                ),
            ]
        }
        _ => Vec::new(),
    };
    Some(definitions)
}

fn edited_bio_shape_object_from_handle(
    original: &SceneObject,
    handle: ShapeEditHandle,
    point: Point,
) -> Option<SceneObject> {
    let local = bio_shape_world_to_normalized(original, point)?;
    let mut object = original.clone();
    let data = object.payload.bio_shape.as_mut()?;
    let center = Point::new(data.center[0], data.center[1]);
    let major_radius = center.distance(Point::new(data.major_axis_end[0], data.major_axis_end[1]));
    let minor_radius = center.distance(Point::new(data.minor_axis_end[0], data.minor_axis_end[1]));
    let p = &mut data.parameters;
    match handle {
        ShapeEditHandle::BioReceptorWidth => {
            p.neck_width = Some(round2((local.0.abs() * 100.0).clamp(5.0, 70.0)));
        }
        ShapeEditHandle::BioGProteinGammaShape => {
            p.gprotein_upper_height = Some(round2(((local.1 - 0.65) * 500.0).clamp(5.0, 80.0)));
        }
        ShapeEditHandle::BioDnaHeight => {
            p.dna_wave_height = Some(round2((local.1.abs() * minor_radius * 2.0).max(0.1)));
        }
        ShapeEditHandle::BioDnaSpacing => {
            p.dna_wave_length = Some(round2((local.0.abs() * major_radius * 2.0).max(0.1)));
        }
        ShapeEditHandle::BioDnaStrandWidth => {
            p.dna_wave_width = Some(round2((local.1.abs() * minor_radius).max(0.05)));
        }
        ShapeEditHandle::BioDnaOffset => {
            p.dna_wave_offset = Some(round2((local.1.abs() * minor_radius).max(0.0)));
        }
        ShapeEditHandle::BioHelixHeight => {
            p.cylinder_height = Some(round2((local.1.abs() * minor_radius * 2.0).max(0.1)));
        }
        ShapeEditHandle::BioHelixStrandWidth => {
            p.pipe_width = Some(round2((local.1.abs() * minor_radius).max(0.05)));
        }
        ShapeEditHandle::BioHelixCylinderWidth => {
            p.cylinder_width = Some(round2((local.0.abs() * major_radius).max(0.1)));
        }
        ShapeEditHandle::BioHelixSpacing => {
            p.cylinder_distance = Some(round2((local.0.abs() * major_radius).max(0.1)));
        }
        ShapeEditHandle::BioMembraneUnitSize => {
            p.membrane_element_size = Some(round2((local.1.abs() * minor_radius * 2.0).max(0.1)));
        }
        ShapeEditHandle::BioMembraneArcStart => {
            p.membrane_start_angle = Some(round2(local.1.atan2(local.0).to_degrees()));
        }
        ShapeEditHandle::BioMembraneArcEnd => {
            p.membrane_end_angle = Some(round2(local.1.atan2(local.0).to_degrees()));
        }
        _ => return None,
    }
    object.payload.bbox = Some(super::bio_shapes::bio_shape_local_bbox(data));
    Some(object)
}

fn bio_shape_normalized_to_world(object: &SceneObject, u: f64, v: f64) -> Option<Point> {
    let data = object.payload.bio_shape.as_ref()?;
    let local = Point::new(
        data.center[0]
            + (data.major_axis_end[0] - data.center[0]) * u
            + (data.minor_axis_end[0] - data.center[0]) * v,
        data.center[1]
            + (data.major_axis_end[1] - data.center[1]) * u
            + (data.minor_axis_end[1] - data.center[1]) * v,
    );
    Some(bio_shape_local_to_world(object, local))
}

fn bio_shape_local_to_world(object: &SceneObject, local: Point) -> Point {
    let scaled = Point::new(
        local.x * object.transform.scale[0],
        local.y * object.transform.scale[1],
    );
    let angle = object.transform.rotate.to_radians();
    Point::new(
        object.transform.translate[0] + scaled.x * angle.cos() - scaled.y * angle.sin(),
        object.transform.translate[1] + scaled.x * angle.sin() + scaled.y * angle.cos(),
    )
}

fn bio_shape_world_to_local(object: &SceneObject, world: Point) -> Option<Point> {
    if object.transform.scale[0].abs() <= crate::EPSILON
        || object.transform.scale[1].abs() <= crate::EPSILON
    {
        return None;
    }
    let angle = -object.transform.rotate.to_radians();
    let x = world.x - object.transform.translate[0];
    let y = world.y - object.transform.translate[1];
    Some(Point::new(
        (x * angle.cos() - y * angle.sin()) / object.transform.scale[0],
        (x * angle.sin() + y * angle.cos()) / object.transform.scale[1],
    ))
}

fn bio_shape_world_to_normalized(object: &SceneObject, world: Point) -> Option<(f64, f64)> {
    let data = object.payload.bio_shape.as_ref()?;
    let local = bio_shape_world_to_local(object, world)?;
    let major = crate::Vector::new(
        data.major_axis_end[0] - data.center[0],
        data.major_axis_end[1] - data.center[1],
    );
    let minor = crate::Vector::new(
        data.minor_axis_end[0] - data.center[0],
        data.minor_axis_end[1] - data.center[1],
    );
    let determinant = major.x * minor.y - major.y * minor.x;
    if determinant.abs() <= crate::EPSILON {
        return None;
    }
    let dx = local.x - data.center[0];
    let dy = local.y - data.center[1];
    Some((
        (dx * minor.y - dy * minor.x) / determinant,
        (major.x * dy - major.y * dx) / determinant,
    ))
}

fn rotated_orbital_object_from_handle(
    original: &SceneObject,
    handle: ShapeEditHandle,
    point: Point,
) -> Option<SceneObject> {
    let template = original
        .payload
        .extra
        .get("orbitalTemplate")
        .and_then(JsonValue::as_str)
        .unwrap_or("");
    if matches!(template, "s" | "oval") {
        return rotated_orbital_oval_from_handle(original, handle, point);
    }
    rotated_orbital_axis_from_handle(original, handle, point)
}

fn rotated_orbital_oval_from_handle(
    original: &SceneObject,
    handle: ShapeEditHandle,
    point: Point,
) -> Option<SceneObject> {
    let (center, major, minor) = shape_oval_points(original)?;
    let rx = center.distance(major);
    let ry = center.distance(minor);
    if rx <= crate::EPSILON || ry <= crate::EPSILON || center.distance(point) <= crate::EPSILON {
        return None;
    }
    let angle = orbital_angle_from_handle(center, handle, point)?;
    let next_major = center.translated(direction_from_angle(angle).scaled(rx));
    let next_minor = center.translated(direction_from_angle(angle + 90.0).scaled(ry));
    let mut object = original.clone();
    set_shape_point(&mut object, "majorAxisEnd", next_major);
    set_shape_point(&mut object, "minorAxisEnd", next_minor);
    object
        .payload
        .extra
        .insert("angle".to_string(), json!(round2(angle)));
    object.payload.bbox = Some([
        round2(center.x - rx),
        round2(center.y - ry),
        round2(rx * 2.0),
        round2(ry * 2.0),
    ]);
    Some(object)
}

fn rotated_orbital_axis_from_handle(
    original: &SceneObject,
    handle: ShapeEditHandle,
    point: Point,
) -> Option<SceneObject> {
    let start = shape_payload_point(original, "axisStart")?;
    let end = shape_payload_point(original, "axisEnd")?;
    let length = start.distance(end);
    if length <= crate::EPSILON || start.distance(point) <= crate::EPSILON {
        return None;
    }
    let angle = orbital_angle_from_handle(start, handle, point)?;
    let next_end = start.translated(direction_from_angle(angle).scaled(length));
    let mut object = original.clone();
    set_shape_point(&mut object, "axisEnd", next_end);
    object
        .payload
        .extra
        .insert("angle".to_string(), json!(round2(angle)));
    object.payload.bbox = orbital_axis_bbox(start, next_end, length * 0.75);
    Some(object)
}

fn orbital_angle_from_handle(center: Point, handle: ShapeEditHandle, point: Point) -> Option<f64> {
    let base = angle_between(center, point);
    match handle {
        ShapeEditHandle::EllipseMajorPositive => Some(base),
        ShapeEditHandle::EllipseMajorNegative => Some(crate::normalize_angle(base + 180.0)),
        ShapeEditHandle::EllipseMinorPositive => Some(crate::normalize_angle(base - 90.0)),
        ShapeEditHandle::EllipseMinorNegative => Some(crate::normalize_angle(base + 90.0)),
        _ => None,
    }
}

fn orbital_axis_bbox(start: Point, end: Point, padding: f64) -> Option<[f64; 4]> {
    let left = start.x.min(end.x) - padding;
    let top = start.y.min(end.y) - padding;
    let right = start.x.max(end.x) + padding;
    let bottom = start.y.max(end.y) + padding;
    Some([
        round2(left),
        round2(top),
        round2(right - left),
        round2(bottom - top),
    ])
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
    let min_size = crate::px_to_pt(4.0);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbital_tool_rotates_orbital_handle_without_resizing() {
        let mut engine = Engine::new();
        engine
            .execute_command(EditorCommand::AddOrbital {
                template: OrbitalTemplate::P,
                style: OrbitalStyle::Filled,
                phase: OrbitalPhase::Plus,
                color: "#000000".to_string(),
                center: CommandAnchor::from(Point::new(200.0, 300.0)),
                end: CommandAnchor::from(Point::new(200.0, 318.0)),
            })
            .expect("add orbital");
        let mut tool = engine.state.tool.clone();
        tool.active_tool = Tool::Orbital;
        engine.set_tool_state(tool);

        assert_eq!(
            engine.hover_shape_action_at_point(Point::new(200.0, 318.0)),
            "ellipse-major-positive"
        );
        assert_eq!(
            engine.begin_hover_shape_edit(Point::new(200.0, 318.0)),
            "ellipse-major-positive"
        );
        assert!(engine.update_hover_shape_edit(Point::new(218.0, 300.0), false));
        assert!(engine.finish_hover_shape_edit(Point::new(218.0, 300.0), false));

        let orbital = engine
            .state
            .document
            .objects
            .iter()
            .find(|object| {
                object.payload.extra.get("kind").and_then(JsonValue::as_str) == Some("orbital")
            })
            .expect("orbital object");
        let start = shape_payload_point(orbital, "axisStart").expect("axis start");
        let end = shape_payload_point(orbital, "axisEnd").expect("axis end");
        assert_eq!(start, Point::new(200.0, 300.0));
        assert_eq!(end, Point::new(218.0, 300.0));
        assert!((start.distance(end) - 18.0).abs() < 0.01);
    }

    #[test]
    fn plasmid_handles_edit_independent_semantics_and_undo() {
        let mut engine = Engine::new();
        engine
            .load_cdxml_document(include_str!("../../tests/fixtures/cdxml/plasmid-map.cdxml"))
            .expect("plasmid fixture loads");
        let object = engine
            .state
            .document
            .objects
            .iter()
            .find(|object| object.payload.plasmid_map.is_some())
            .cloned()
            .expect("plasmid object");
        let center = plasmid_map_center(&object).expect("center");
        let original = object.payload.plasmid_map.as_ref().expect("data").clone();
        let marker = &original.markers[0];
        let marker_handle = plasmid_map_point(
            center,
            original.radius + marker.offset,
            marker
                .label_angle
                .unwrap_or_else(|| original.angle_degrees(marker.position)),
            object.transform.rotate,
        );
        assert_eq!(
            engine.begin_hover_shape_edit(marker_handle),
            "plasmid-marker-label"
        );
        let marker_target = plasmid_map_point(center, original.radius + 72.0, 90.0, 0.0);
        assert!(engine.finish_hover_shape_edit(marker_target, false));
        let edited = engine
            .state
            .document
            .find_scene_object(&object.id)
            .and_then(|object| object.payload.plasmid_map.as_ref())
            .expect("edited data");
        assert_eq!(edited.markers[0].position, marker.position);
        assert_eq!(edited.markers[0].offset, 72.0);
        assert_eq!(edited.markers[0].label_angle, Some(90.0));

        assert!(engine.undo());
        let restored_object = engine
            .state
            .document
            .find_scene_object(&object.id)
            .cloned()
            .expect("restored object");
        let restored = restored_object
            .payload
            .plasmid_map
            .as_ref()
            .expect("restored data");
        assert_eq!(restored, &original);

        let region = &restored.regions[0];
        let start_handle = plasmid_map_point(
            center,
            restored.radius + region.offset,
            restored.angle_degrees(region.start),
            restored_object.transform.rotate,
        );
        assert_eq!(
            engine.begin_hover_shape_edit(start_handle),
            "plasmid-region-start"
        );
        assert!(engine.finish_hover_shape_edit(
            plasmid_map_point(center, restored.radius + region.offset, 180.0, 0.0),
            false,
        ));
        let edited_region = &engine
            .state
            .document
            .find_scene_object(&object.id)
            .and_then(|object| object.payload.plasmid_map.as_ref())
            .expect("region data")
            .regions[0];
        assert_eq!(edited_region.start, 6001);
        assert_eq!(edited_region.end, region.end);
    }
}
