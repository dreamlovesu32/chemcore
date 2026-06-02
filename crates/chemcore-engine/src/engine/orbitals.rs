use super::*;
use crate::Transform;

const DEFAULT_ORBITAL_SIZE_RATIO: f64 = 1.6;
const OVAL_MINOR_RATIO: f64 = 0.4;

impl Engine {
    pub(super) fn pointer_down_orbital(&mut self, event: PointerEvent) {
        let point = event.point();
        self.clear_interaction();
        self.state.selection = SelectionState::default();
        self.orbital_drag = Some(OrbitalDragState {
            anchor: point,
            current: point,
            has_dragged: false,
        });
    }

    pub(super) fn pointer_move_orbital(&mut self, event: PointerEvent) {
        let point = event.point();
        self.state.overlay = OverlayState::default();
        if let Some(mut drag) = self.orbital_drag.take() {
            drag.current = point;
            if drag.anchor.distance(point) >= DRAG_START_THRESHOLD {
                drag.has_dragged = true;
            }
            if drag.has_dragged {
                self.state.overlay.preview = Some(BondPreview {
                    start: drag.anchor,
                    end: point,
                });
            }
            self.orbital_drag = Some(drag);
        }
    }

    pub(super) fn pointer_up_orbital(&mut self, event: PointerEvent) {
        let Some(mut drag) = self.orbital_drag.take() else {
            return;
        };
        drag.current = event.point();
        if drag.anchor.distance(drag.current) >= DRAG_START_THRESHOLD {
            drag.has_dragged = true;
        }
        let command = EditorCommand::LegacyMutation {
            label: "add-orbital".to_string(),
        };
        if drag.has_dragged {
            self.with_command(command, |engine| engine.insert_orbital_from_drag(&drag));
        }
        self.state.overlay = OverlayState::default();
    }

    pub(super) fn orbital_preview_document(&self) -> Option<ChemcoreDocument> {
        let drag = self.orbital_drag.as_ref()?;
        if !drag.has_dragged {
            return None;
        }
        let mut document = self.state.document.clone();
        let style_id = "__preview_orbital_style".to_string();
        document
            .styles
            .insert(style_id.clone(), self.pending_orbital_style());
        document.objects.push(self.orbital_scene_object(
            drag.anchor,
            drag.current,
            "__preview_orbital".to_string(),
            style_id,
        )?);
        Some(document)
    }

    pub(super) fn insert_orbital_from_drag(&mut self, drag: &OrbitalDragState) -> bool {
        let object_id = self.next_id("obj_shape_orbital");
        let style_id = format!("style_{object_id}");
        let Some(object) = self.orbital_scene_object(
            drag.anchor,
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
            .insert(style_id, self.pending_orbital_style());
        self.state.document.objects.push(object);
        self.note_pending_select_target(PendingSelectTarget::GraphicObject(object_id));
        true
    }

    fn orbital_scene_object(
        &self,
        anchor: Point,
        current: Point,
        object_id: String,
        style_id: String,
    ) -> Option<SceneObject> {
        let template = self.state.tool.orbital_template;
        let style = self.state.tool.orbital_style;
        let phase = self.state.tool.orbital_phase;
        let angle = snapped_orbital_angle(anchor, current);
        let size = self.default_orbital_size();
        let direction = crate::direction_from_angle(angle);
        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!("orbital"));
        extra.insert("orbitalTemplate".to_string(), json!(orbital_template_name(template)));
        extra.insert("orbitalStyle".to_string(), json!(orbital_style_name(style)));
        extra.insert("orbitalPhase".to_string(), json!(orbital_phase_name(phase)));
        extra.insert("orbitalColor".to_string(), json!(self.state.tool.orbital_color.clone()));
        extra.insert("angle".to_string(), json!(round2(angle)));
        extra.insert("size".to_string(), json!(round2(size)));

        let (transform, bbox) = match template {
            OrbitalTemplate::S => {
                let radius = size;
                let major = anchor.translated(direction.scaled(radius));
                let minor = anchor.translated(direction_from_angle(angle + 90.0).scaled(radius));
                extra.insert("center".to_string(), json!([round2(anchor.x), round2(anchor.y)]));
                extra.insert(
                    "majorAxisEnd".to_string(),
                    json!([round2(major.x), round2(major.y)]),
                );
                extra.insert(
                    "minorAxisEnd".to_string(),
                    json!([round2(minor.x), round2(minor.y)]),
                );
                (
                    Transform::identity(),
                    Some([
                        round2(anchor.x - radius),
                        round2(anchor.y - radius),
                        round2(radius * 2.0),
                        round2(radius * 2.0),
                    ]),
                )
            }
            OrbitalTemplate::Oval => {
                let rx = size;
                let ry = size * OVAL_MINOR_RATIO;
                let major = anchor.translated(direction.scaled(rx));
                let minor = anchor.translated(direction_from_angle(angle + 90.0).scaled(ry));
                extra.insert("center".to_string(), json!([round2(anchor.x), round2(anchor.y)]));
                extra.insert(
                    "majorAxisEnd".to_string(),
                    json!([round2(major.x), round2(major.y)]),
                );
                extra.insert(
                    "minorAxisEnd".to_string(),
                    json!([round2(minor.x), round2(minor.y)]),
                );
                (
                    Transform::identity(),
                    Some([
                        round2(anchor.x - rx),
                        round2(anchor.y - ry),
                        round2(rx * 2.0),
                        round2(ry * 2.0),
                    ]),
                )
            }
            OrbitalTemplate::Lobe => {
                let end = anchor.translated(direction.scaled(size));
                extra.insert("axisStart".to_string(), json!([round2(anchor.x), round2(anchor.y)]));
                extra.insert("axisEnd".to_string(), json!([round2(end.x), round2(end.y)]));
                let [x1, y1, x2, y2] = orbital_axis_bounds(anchor, end, size * 0.75);
                (
                    Transform::identity(),
                    Some([round2(x1), round2(y1), round2(x2 - x1), round2(y2 - y1)]),
                )
            }
            OrbitalTemplate::Hybrid => {
                let start = anchor;
                let end = anchor.translated(direction.scaled(size));
                extra.insert("axisStart".to_string(), json!([round2(start.x), round2(start.y)]));
                extra.insert("axisEnd".to_string(), json!([round2(end.x), round2(end.y)]));
                let [x1, y1, x2, y2] = orbital_axis_bounds(start, end, size * 0.75);
                (
                    Transform::identity(),
                    Some([round2(x1), round2(y1), round2(x2 - x1), round2(y2 - y1)]),
                )
            }
            _ => {
                let start = anchor;
                let end = anchor.translated(direction.scaled(size));
                extra.insert("axisStart".to_string(), json!([round2(start.x), round2(start.y)]));
                extra.insert("axisEnd".to_string(), json!([round2(end.x), round2(end.y)]));
                let [x1, y1, x2, y2] = orbital_axis_bounds(start, end, size * 0.75);
                (
                    Transform::identity(),
                    Some([round2(x1), round2(y1), round2(x2 - x1), round2(y2 - y1)]),
                )
            }
        };

        Some(SceneObject {
            id: object_id,
            object_type: "shape".to_string(),
            name: "orbital".to_string(),
            visible: true,
            locked: false,
            z_index: self.next_shape_z_index(),
            transform,
            style_ref: Some(style_id),
            meta: json!({
                "source": "editor",
                "orbital": true,
            }),
            payload: crate::ObjectPayload {
                resource_ref: None,
                bbox,
                extra,
            },
            children: Vec::new(),
        })
    }

    fn pending_orbital_style(&self) -> JsonValue {
        let color = self.state.tool.orbital_color.clone();
        let stroke_width = self.options.graphic_stroke_world_cm().value();
        match self.state.tool.orbital_style {
            OrbitalStyle::Hollow => json!({
                "kind": "shape",
                "fill": null,
                "stroke": color,
                "strokeWidth": stroke_width,
                "dashArray": [],
            }),
            OrbitalStyle::Filled => json!({
                "kind": "shape",
                "fill": color,
                "stroke": null,
                "strokeWidth": stroke_width,
                "dashArray": [],
            }),
            OrbitalStyle::Shaded => json!({
                "kind": "shape",
                "fill": color,
                "stroke": color,
                "strokeWidth": stroke_width,
                "dashArray": [],
                "shaded": true,
            }),
        }
    }

    fn default_orbital_size(&self) -> f64 {
        (self.options.bond_length_world_cm().value() * DEFAULT_ORBITAL_SIZE_RATIO).max(12.0)
    }
}

fn snapped_orbital_angle(anchor: Point, current: Point) -> f64 {
    if anchor.distance(current) <= crate::EPSILON {
        return 90.0;
    }
    nearest_angle(angle_between(anchor, current), GLOBAL_SNAP_ANGLES)
}

fn orbital_axis_bounds(start: Point, end: Point, padding: f64) -> [f64; 4] {
    [
        start.x.min(end.x) - padding,
        start.y.min(end.y) - padding,
        start.x.max(end.x) + padding,
        start.y.max(end.y) + padding,
    ]
}

fn orbital_template_name(value: OrbitalTemplate) -> &'static str {
    match value {
        OrbitalTemplate::S => "s",
        OrbitalTemplate::P => "p",
        OrbitalTemplate::Dxy => "dxy",
        OrbitalTemplate::Oval => "oval",
        OrbitalTemplate::Hybrid => "hybrid",
        OrbitalTemplate::Dz2 => "dz2",
        OrbitalTemplate::Lobe => "lobe",
    }
}

fn orbital_style_name(value: OrbitalStyle) -> &'static str {
    match value {
        OrbitalStyle::Hollow => "hollow",
        OrbitalStyle::Shaded => "shaded",
        OrbitalStyle::Filled => "filled",
    }
}

fn orbital_phase_name(value: OrbitalPhase) -> &'static str {
    match value {
        OrbitalPhase::Plus => "plus",
        OrbitalPhase::Minus => "minus",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbital_tool_defaults_match_chemdraw_fixture_sizes() {
        let mut engine = Engine::new();
        let anchor = Point::new(200.0, 300.0);
        let current = Point::new(200.0, 360.0);

        engine.state.tool.orbital_template = OrbitalTemplate::S;
        let s = engine
            .orbital_scene_object(anchor, current, "s".to_string(), "style_s".to_string())
            .expect("s orbital");
        assert_eq!(s.payload.bbox, Some([152.0, 252.0, 96.0, 96.0]));

        engine.state.tool.orbital_template = OrbitalTemplate::Oval;
        let oval = engine
            .orbital_scene_object(anchor, current, "oval".to_string(), "style_oval".to_string())
            .expect("oval orbital");
        assert_eq!(oval.payload.bbox, Some([152.0, 280.8, 96.0, 38.4]));

        for template in [
            OrbitalTemplate::P,
            OrbitalTemplate::Dxy,
            OrbitalTemplate::Hybrid,
            OrbitalTemplate::Dz2,
            OrbitalTemplate::Lobe,
        ] {
            engine.state.tool.orbital_template = template;
            let object = engine
                .orbital_scene_object(anchor, current, "orb".to_string(), "style_orb".to_string())
                .expect("orbital object");
            assert_eq!(object.payload.bbox, Some([164.0, 264.0, 72.0, 120.0]));
            assert_eq!(object.payload.extra.get("axisStart"), Some(&json!([200.0, 300.0])));
            assert_eq!(object.payload.extra.get("axisEnd"), Some(&json!([200.0, 348.0])));
        }
    }

    #[test]
    fn orbital_tool_defaults_match_acs_fixture_sizes() {
        let mut engine = Engine::new();
        engine.set_document_style_preset(super::super::ACS_DOCUMENT_1996_PRESET);
        let anchor = Point::new(200.0, 300.0);
        let current = Point::new(200.0, 340.0);

        engine.state.tool.orbital_template = OrbitalTemplate::S;
        let s = engine
            .orbital_scene_object(anchor, current, "s".to_string(), "style_s".to_string())
            .expect("s orbital");
        assert_eq!(s.payload.bbox, Some([176.96, 276.96, 46.08, 46.08]));

        engine.state.tool.orbital_template = OrbitalTemplate::Oval;
        let oval = engine
            .orbital_scene_object(anchor, current, "oval".to_string(), "style_oval".to_string())
            .expect("oval orbital");
        assert_eq!(oval.payload.bbox, Some([176.96, 290.78, 46.08, 18.43]));

        for template in [
            OrbitalTemplate::P,
            OrbitalTemplate::Dxy,
            OrbitalTemplate::Hybrid,
            OrbitalTemplate::Dz2,
            OrbitalTemplate::Lobe,
        ] {
            engine.state.tool.orbital_template = template;
            let object = engine
                .orbital_scene_object(anchor, current, "orb".to_string(), "style_orb".to_string())
                .expect("orbital object");
            assert_eq!(object.payload.bbox, Some([182.72, 282.72, 34.56, 57.6]));
            assert_eq!(object.payload.extra.get("axisStart"), Some(&json!([200.0, 300.0])));
            assert_eq!(object.payload.extra.get("axisEnd"), Some(&json!([200.0, 323.04])));
        }
    }
}
