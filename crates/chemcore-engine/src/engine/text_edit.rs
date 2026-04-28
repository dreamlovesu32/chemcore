use super::{Engine, TextEditLayoutRequest};
use crate::{
    build_label_glyph_polygons, decide_label_layout, layout_label_text, round2, round6, Bond,
    BondLineWeight, DoubleBondPlacement, EndpointHit, LabelFlow, LabelRun, Point, WorldCm,
    WorldPoint,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const DEFAULT_TEXT_FONT_FAMILY: &str = "Arial";
const DEFAULT_TEXT_FONT_SIZE: f64 = crate::DEFAULT_TEXT_FONT_SIZE_CM;
const DEFAULT_TEXT_FILL: &str = "#000000";
const DEFAULT_TEXT_LINE_HEIGHT: f64 = crate::DEFAULT_TEXT_LINE_HEIGHT_CM;
const DEFAULT_TEXT_BLOCK_LINE_HEIGHT: f64 = crate::DEFAULT_TEXT_BLOCK_LINE_HEIGHT_CM;
const DEFAULT_CENTERED_LABEL_FONT_SIZE: f64 = crate::DEFAULT_CENTERED_LABEL_FONT_SIZE_CM;
const TEXT_EDIT_BOX_WIDTH: f64 = crate::px_to_cm(8.0);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TextEditTarget {
    TextObject {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        x: f64,
        y: f64,
    },
    EndpointLabel {
        node_id: String,
        x: f64,
        y: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEditSession {
    pub target: TextEditTarget,
    #[serde(default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_runs: Vec<LabelRun>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "box")]
    pub box_value: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_offset: Option<[f64; 2]>,
    #[serde(default)]
    pub preserve_lines: bool,
    #[serde(default)]
    pub default_chemical: bool,
}

impl TextEditTarget {
    pub const fn world_point(&self) -> WorldPoint {
        match self {
            Self::TextObject { x, y, .. } | Self::EndpointLabel { x, y, .. } => {
                WorldPoint::new(WorldCm(*x), WorldCm(*y))
            }
        }
    }
}

impl TextEditSession {
    pub const fn font_size_world_cm(&self) -> Option<WorldCm> {
        match self.font_size {
            Some(value) => Some(WorldCm(value)),
            None => None,
        }
    }

    pub const fn line_height_world_cm(&self) -> Option<WorldCm> {
        match self.line_height {
            Some(value) => Some(WorldCm(value)),
            None => None,
        }
    }

    pub const fn target_world_point(&self) -> WorldPoint {
        self.target.world_point()
    }

    pub const fn anchor_offset_world_cm(&self) -> Option<[WorldCm; 2]> {
        match self.anchor_offset {
            Some([x, y]) => Some([WorldCm(x), WorldCm(y)]),
            None => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEditSelection {
    pub anchor: usize,
    pub focus: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEditSelectionState {
    pub anchor: usize,
    pub focus: usize,
    pub start: usize,
    pub end: usize,
    pub collapsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEditLayoutRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEditLayoutCaretOffset {
    pub offset: usize,
    pub x: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEditLayoutCaret {
    pub offset: usize,
    pub x: f64,
    pub y: f64,
    pub height: f64,
    pub line_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEditLayoutLine {
    pub index: usize,
    pub x: f64,
    pub y: f64,
    pub baseline_y: f64,
    pub height: f64,
    pub start_offset: usize,
    pub end_offset: usize,
    pub text_anchor: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<LabelRun>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub caret_offsets: Vec<TextEditLayoutCaretOffset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEditLayout {
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_runs: Vec<LabelRun>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub display_runs: Vec<LabelRun>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<TextEditLayoutLine>,
    pub width: f64,
    pub height: f64,
    pub line_height: f64,
    pub anchor_offset: [f64; 2],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub caret_positions: Vec<TextEditLayoutCaret>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selection_rects: Vec<TextEditLayoutRect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<TextEditSelectionState>,
}

impl Engine {
    pub fn begin_text_edit(&mut self, point: Point) -> Option<TextEditSession> {
        self.clear_interaction();
        if let Some((node_id, bounds)) = self.hit_test_endpoint_label_box(point) {
            self.state.overlay.hover_text_box = Some(crate::HoverTextBox {
                bounds,
                object_id: None,
                node_id: Some(node_id.clone()),
            });
            return self.endpoint_text_session(&node_id, point);
        }
        if let Some((object_id, bounds)) = self.hit_test_text_object(point) {
            self.state.overlay.hover_text_box = Some(crate::HoverTextBox {
                bounds,
                object_id: Some(object_id.clone()),
                node_id: None,
            });
            return self.text_object_session(&object_id);
        }
        if let Some(endpoint) =
            crate::hit_test_endpoint(&self.state.document, point, crate::ENDPOINT_HIT_RADIUS)
        {
            self.state.overlay.hover_endpoint = Some(endpoint.clone());
            return self.endpoint_text_session(&endpoint.node_id, endpoint.point);
        }
        Some(TextEditSession {
            target: TextEditTarget::TextObject {
                object_id: None,
                x: point.x,
                y: point.y,
            },
            text: String::new(),
            source_runs: Vec::new(),
            font_family: Some(DEFAULT_TEXT_FONT_FAMILY.to_string()),
            font_size: Some(DEFAULT_TEXT_FONT_SIZE),
            fill: Some(DEFAULT_TEXT_FILL.to_string()),
            align: Some("left".to_string()),
            line_height: Some(DEFAULT_TEXT_LINE_HEIGHT),
            box_value: Some([0.0, 0.0, TEXT_EDIT_BOX_WIDTH, DEFAULT_TEXT_LINE_HEIGHT]),
            anchor_offset: None,
            preserve_lines: true,
            default_chemical: false,
        })
    }

    pub fn apply_text_edit(&mut self, session: TextEditSession) -> bool {
        match &session.target {
            TextEditTarget::TextObject { object_id, .. } => {
                self.apply_text_object_edit(object_id.as_deref(), &session)
            }
            TextEditTarget::EndpointLabel { node_id, .. } => {
                self.apply_endpoint_text_edit(node_id, &session)
            }
        }
    }

    pub fn preview_text_runs(&self, session: &TextEditSession) -> (Vec<LabelRun>, Vec<LabelRun>) {
        let text = if !session.source_runs.is_empty() {
            runs_text(&session.source_runs)
        } else {
            session.text.clone()
        };
        let fallback_font_family = session
            .font_family
            .as_deref()
            .unwrap_or(DEFAULT_TEXT_FONT_FAMILY);
        let fallback_font_size = session
            .font_size_world_cm()
            .unwrap_or(WorldCm(DEFAULT_TEXT_FONT_SIZE))
            .value();
        let fallback_fill = session.fill.as_deref().unwrap_or(DEFAULT_TEXT_FILL);
        let source_runs = merge_adjacent_runs(normalize_source_runs(session, &text));
        let display_runs = display_runs_from_source_runs(
            &source_runs,
            fallback_font_family,
            fallback_font_size,
            fallback_fill,
        );
        (source_runs, display_runs)
    }

    pub fn preview_text_edit_layout(&self, request: &TextEditLayoutRequest) -> TextEditLayout {
        let session = &request.session;
        let raw_text = if !session.source_runs.is_empty() {
            runs_text(&session.source_runs)
        } else {
            session.text.clone()
        };
        let text = match session.target {
            TextEditTarget::EndpointLabel { .. } => raw_text
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .replace('\n', " "),
            TextEditTarget::TextObject { .. } => raw_text,
        };
        let fallback_font_family = session
            .font_family
            .as_deref()
            .unwrap_or(DEFAULT_TEXT_FONT_FAMILY);
        let fallback_font_size = session
            .font_size_world_cm()
            .unwrap_or(WorldCm(DEFAULT_TEXT_FONT_SIZE))
            .value();
        let fallback_fill = session.fill.as_deref().unwrap_or(DEFAULT_TEXT_FILL);
        let line_height = session
            .line_height_world_cm()
            .unwrap_or(WorldCm(DEFAULT_TEXT_LINE_HEIGHT))
            .value();
        let source_runs = merge_adjacent_runs(normalize_source_runs(session, &text));
        let display_runs = display_runs_from_source_runs(
            &source_runs,
            fallback_font_family,
            fallback_font_size,
            fallback_fill,
        );
        let selection = normalize_text_edit_selection(&text, request.selection.as_ref());
        match &session.target {
            TextEditTarget::EndpointLabel { node_id, .. } => self.build_endpoint_text_edit_layout(
                node_id,
                session,
                text,
                source_runs,
                display_runs,
                fallback_font_family,
                fallback_font_size,
                fallback_fill,
                line_height,
                selection,
            ),
            TextEditTarget::TextObject { .. } => build_text_object_edit_layout(
                session,
                text,
                source_runs,
                display_runs,
                line_height,
                selection,
            ),
        }
    }

    pub fn replace_hovered_endpoint_label(&mut self, label: &str) -> bool {
        let Some(hovered_node_id) = self
            .state
            .overlay
            .hover_endpoint
            .as_ref()
            .map(|hit| hit.node_id.clone())
        else {
            return false;
        };

        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let object_translate = entry.object.transform.translate;
        let Some(node_index) = entry
            .fragment
            .nodes
            .iter()
            .position(|node| node.id == hovered_node_id)
        else {
            self.undo_stack.pop();
            return false;
        };
        let connection_angles = adjacent_angles_for_fragment_node(entry.fragment, &hovered_node_id);
        let node = &mut entry.fragment.nodes[node_index];

        if !apply_node_label_replacement(node, label, &connection_angles) {
            self.undo_stack.pop();
            return false;
        }

        let node_position = node.position;
        refresh_attached_node_label_geometry_for_node(
            entry.fragment,
            object_translate,
            &hovered_node_id,
            self.options.bond_stroke_world_cm().value(),
        );
        entry.update_bounds();
        let hover_point = crate::Point::new(
            object_translate[0] + node_position[0],
            object_translate[1] + node_position[1],
        );
        self.drag = None;
        self.state.selection = crate::SelectionState::default();
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.preview = None;
        self.state.overlay.hover_endpoint = Some(EndpointHit {
            node_id: hovered_node_id,
            point: hover_point,
            distance: 0.0,
            label_anchor: None,
        });
        true
    }

    fn endpoint_text_session(&self, node_id: &str, _point: Point) -> Option<TextEditSession> {
        let entry = self.state.document.editable_fragment()?;
        let node = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == node_id)?;
        let label = node.label.as_ref();
        let box_value = label.and_then(|label| label.bbox()).map(|bbox| {
            [
                round6(bbox[0] + entry.object.transform.translate[0]),
                round6(bbox[1] + entry.object.transform.translate[1]),
                round6(bbox[2] + entry.object.transform.translate[0]),
                round6(bbox[3] + entry.object.transform.translate[1]),
            ]
        });
        let anchor_point =
            endpoint_label_editor_anchor_world(node, entry.object.transform.translate)
                .unwrap_or_else(|| {
                    attached_node_label_anchor_world(
                        entry.fragment,
                        node_id,
                        entry.object.transform.translate,
                        self.options.bond_stroke_world_cm().value(),
                    )
                });
        let source_runs = label
            .and_then(|label| label.meta.get("sourceRuns"))
            .cloned()
            .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
            .unwrap_or_else(|| label.map(|label| label.runs.clone()).unwrap_or_default());
        let text = if !source_runs.is_empty() {
            runs_text(&source_runs)
        } else {
            label.map(|label| label.text.clone()).unwrap_or_default()
        };
        let font_size = label
            .and_then(|label| label.font_size)
            .or(Some(DEFAULT_TEXT_FONT_SIZE));
        let font_size_world_cm = WorldCm(font_size.unwrap_or(DEFAULT_TEXT_FONT_SIZE));
        let line_height = Some((font_size_world_cm.value() * 1.05).max(font_size_world_cm.value()));
        Some(TextEditSession {
            target: TextEditTarget::EndpointLabel {
                node_id: node_id.to_string(),
                x: anchor_point.x,
                y: anchor_point.y,
            },
            text,
            source_runs,
            font_family: label
                .and_then(|label| label.font_family.clone())
                .or(Some(DEFAULT_TEXT_FONT_FAMILY.to_string())),
            font_size,
            fill: label
                .and_then(|label| label.fill.clone())
                .or(Some(DEFAULT_TEXT_FILL.to_string())),
            align: Some("left".to_string()),
            line_height,
            box_value,
            anchor_offset: box_value.map(|bbox| {
                [
                    round6(anchor_point.x - bbox[0]),
                    round6(anchor_point.y - bbox[1]),
                ]
            }),
            preserve_lines: true,
            default_chemical: true,
        })
    }

    fn text_object_session(&self, object_id: &str) -> Option<TextEditSession> {
        let object = self
            .state
            .document
            .objects
            .iter()
            .find(|object| object.id == object_id && object.object_type == "text")?;
        let payload = &object.payload;
        let source_runs = payload
            .extra
            .get("sourceRuns")
            .cloned()
            .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
            .unwrap_or_else(|| payload_runs_or_text(payload));
        let text = if !source_runs.is_empty() {
            runs_text(&source_runs)
        } else {
            payload_text(payload)
        };
        Some(TextEditSession {
            target: TextEditTarget::TextObject {
                object_id: Some(object_id.to_string()),
                x: object.transform.translate[0],
                y: object.transform.translate[1],
            },
            text,
            source_runs,
            font_family: payload
                .extra
                .get("fontFamily")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .or(Some(DEFAULT_TEXT_FONT_FAMILY.to_string())),
            font_size: payload
                .extra
                .get("fontSize")
                .and_then(Value::as_f64)
                .or(Some(DEFAULT_TEXT_FONT_SIZE)),
            fill: payload
                .extra
                .get("fill")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .or(Some(DEFAULT_TEXT_FILL.to_string())),
            align: payload
                .extra
                .get("align")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .or(Some("left".to_string())),
            line_height: payload
                .extra
                .get("lineHeight")
                .and_then(Value::as_f64)
                .or(Some(DEFAULT_TEXT_LINE_HEIGHT)),
            box_value: payload_box(payload),
            anchor_offset: None,
            preserve_lines: payload
                .extra
                .get("preserveLines")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            default_chemical: false,
        })
    }

    fn build_endpoint_text_edit_layout(
        &self,
        node_id: &str,
        session: &TextEditSession,
        text: String,
        source_runs: Vec<LabelRun>,
        display_runs: Vec<LabelRun>,
        fallback_font_family: &str,
        fallback_font_size: f64,
        fallback_fill: &str,
        line_height: f64,
        selection: Option<TextEditSelectionState>,
    ) -> TextEditLayout {
        let Some(entry) = self.state.document.editable_fragment() else {
            return build_text_object_edit_layout(
                session,
                text,
                source_runs,
                display_runs,
                line_height,
                selection,
            );
        };
        let local_anchor = {
            let anchor = Point::from_world(session.target_world_point());
            [
                round2(anchor.x - entry.object.transform.translate[0]),
                round2(anchor.y - entry.object.transform.translate[1]),
            ]
        };
        let connection_angles = adjacent_angles_for_fragment_node(entry.fragment, node_id);
        let label = make_centered_node_label_from_runs(
            &text,
            local_anchor,
            source_runs.clone(),
            display_runs.clone(),
            fallback_font_family,
            fallback_font_size,
            fallback_fill,
            &connection_angles,
            session,
        );
        build_endpoint_label_edit_layout_from_label(
            text,
            source_runs,
            display_runs,
            &label,
            local_anchor,
            line_height,
            selection,
        )
    }

    fn apply_text_object_edit(
        &mut self,
        object_id: Option<&str>,
        session: &TextEditSession,
    ) -> bool {
        let text = session.text.replace("\r\n", "\n").replace('\r', "\n");
        if text.trim().is_empty() {
            return self.remove_text_object(object_id);
        }
        let source_runs = normalize_source_runs(session, &text);
        let session_font_size = session
            .font_size_world_cm()
            .unwrap_or(WorldCm(DEFAULT_TEXT_FONT_SIZE))
            .value();
        let session_line_height = session
            .line_height_world_cm()
            .unwrap_or(WorldCm(DEFAULT_TEXT_BLOCK_LINE_HEIGHT))
            .value();
        let display_runs = display_runs_from_source_runs(
            &source_runs,
            session.font_family.as_deref().unwrap_or("Arial"),
            session_font_size,
            session.fill.as_deref().unwrap_or("#000000"),
        );
        let (width, height) =
            estimate_text_block_size(&display_runs, session_font_size, session_line_height);
        let (x, y, existing_object_id) = match &session.target {
            TextEditTarget::TextObject { object_id, x, y } => (*x, *y, object_id.clone()),
            _ => return false,
        };
        let target_object_id = existing_object_id.or_else(|| object_id.map(ToString::to_string));

        self.push_undo_snapshot();
        let changed =
            if let Some(target_object_id) = target_object_id {
                let Some(object) =
                    self.state.document.objects.iter_mut().find(|object| {
                        object.id == target_object_id && object.object_type == "text"
                    })
                else {
                    self.undo_stack.pop();
                    return false;
                };
                update_text_object_fields(
                    object,
                    x,
                    y,
                    &text,
                    source_runs,
                    display_runs,
                    session,
                    width,
                    height,
                )
            } else {
                let next_id = self.next_id("obj_text");
                let object = make_text_object(
                    &next_id,
                    x,
                    y,
                    &text,
                    source_runs,
                    display_runs,
                    session,
                    width,
                    height,
                    self.next_text_z_index(),
                );
                self.state.document.objects.push(object);
                true
            };
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        self.state.selection = crate::SelectionState::default();
        self.clear_interaction();
        true
    }

    fn apply_endpoint_text_edit(&mut self, node_id: &str, session: &TextEditSession) -> bool {
        let text = session
            .text
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .replace('\n', " ");
        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let object_translate = entry.object.transform.translate;
        let Some(node_index) = entry
            .fragment
            .nodes
            .iter()
            .position(|node| node.id == node_id)
        else {
            self.undo_stack.pop();
            return false;
        };
        let local_anchor_position = match &session.target {
            TextEditTarget::EndpointLabel { .. } => {
                let anchor = Point::from_world(session.target_world_point());
                [
                    round2(anchor.x - object_translate[0]),
                    round2(anchor.y - object_translate[1]),
                ]
            }
            _ => entry.fragment.nodes[node_index].position,
        };
        let connection_angles = adjacent_angles_for_fragment_node(entry.fragment, node_id);
        let node = &mut entry.fragment.nodes[node_index];
        let changed = apply_node_label_text_edit(
            node,
            &text,
            session,
            &connection_angles,
            local_anchor_position,
        );
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        let node_position = node.position;
        refresh_attached_node_label_geometry_for_node(
            entry.fragment,
            object_translate,
            node_id,
            self.options.bond_stroke_world_cm().value(),
        );
        entry.update_bounds();
        let hover_point = crate::Point::new(
            object_translate[0] + node_position[0],
            object_translate[1] + node_position[1],
        );
        self.drag = None;
        self.state.selection = crate::SelectionState::default();
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.preview = None;
        self.state.overlay.hover_endpoint = Some(EndpointHit {
            node_id: node_id.to_string(),
            point: hover_point,
            distance: 0.0,
            label_anchor: None,
        });
        true
    }

    pub(super) fn remove_text_object(&mut self, object_id: Option<&str>) -> bool {
        let Some(object_id) = object_id else {
            return false;
        };
        let Some(index) = self
            .state
            .document
            .objects
            .iter()
            .position(|object| object.id == object_id && object.object_type == "text")
        else {
            return false;
        };
        self.push_undo_snapshot();
        self.state.document.objects.remove(index);
        self.state.selection = crate::SelectionState::default();
        self.clear_interaction();
        true
    }

    pub(super) fn hit_test_text_object(&self, point: Point) -> Option<(String, [f64; 4])> {
        let mut best: Option<(i32, usize, String)> = None;
        let mut best_bounds: Option<[f64; 4]> = None;
        for (index, object) in self.state.document.objects.iter().enumerate() {
            if object.object_type != "text" || !object.visible {
                continue;
            }
            let Some(bounds) = text_object_world_bounds(object) else {
                continue;
            };
            if point.x < bounds[0]
                || point.x > bounds[2]
                || point.y < bounds[1]
                || point.y > bounds[3]
            {
                continue;
            }
            let candidate = (object.z_index, index, object.id.clone());
            if best.as_ref().is_none_or(|current| {
                candidate.0 > current.0 || (candidate.0 == current.0 && candidate.1 > current.1)
            }) {
                best = Some(candidate);
                best_bounds = Some(bounds);
            }
        }
        best.and_then(|(_, _, object_id)| best_bounds.map(|bounds| (object_id, bounds)))
    }

    pub(super) fn hit_test_endpoint_label_box(&self, point: Point) -> Option<(String, [f64; 4])> {
        let entry = self.state.document.editable_fragment()?;
        let mut best: Option<(f64, String, [f64; 4])> = None;
        for node in &entry.fragment.nodes {
            let Some(bounds) = endpoint_label_world_bounds(node, entry.object.transform.translate)
            else {
                continue;
            };
            if point.x < bounds[0]
                || point.x > bounds[2]
                || point.y < bounds[1]
                || point.y > bounds[3]
            {
                continue;
            }
            let area = (bounds[2] - bounds[0]).abs() * (bounds[3] - bounds[1]).abs();
            if best.as_ref().is_none_or(|current| area < current.0) {
                best = Some((area, node.id.clone(), bounds));
            }
        }
        best.map(|(_, node_id, bounds)| (node_id, bounds))
    }

    fn next_text_z_index(&self) -> i32 {
        self.state
            .document
            .objects
            .iter()
            .map(|object| object.z_index)
            .max()
            .unwrap_or(10)
            + 10
    }
}

pub(super) fn apply_node_label_replacement(
    node: &mut crate::Node,
    label: &str,
    connection_angles: &[f64],
) -> bool {
    if classify_node_label_replacement(label).is_none() {
        return false;
    }
    let session = TextEditSession {
        target: TextEditTarget::EndpointLabel {
            node_id: node.id.clone(),
            x: node.position[0],
            y: node.position[1],
        },
        text: label.to_string(),
        source_runs: Vec::new(),
        font_family: Some(DEFAULT_TEXT_FONT_FAMILY.to_string()),
        font_size: Some(DEFAULT_TEXT_FONT_SIZE),
        fill: Some(DEFAULT_TEXT_FILL.to_string()),
        align: Some("left".to_string()),
        line_height: Some(DEFAULT_TEXT_LINE_HEIGHT),
        box_value: None,
        anchor_offset: None,
        preserve_lines: true,
        default_chemical: true,
    };
    apply_node_label_text_edit(node, label, &session, connection_angles, node.position)
}

pub(super) fn apply_node_label_text_edit(
    node: &mut crate::Node,
    text: &str,
    session: &TextEditSession,
    connection_angles: &[f64],
    anchor_position: [f64; 2],
) -> bool {
    let previous_element = node.element.clone();
    let previous_atomic_number = node.atomic_number;
    let previous_is_placeholder = node.is_placeholder;
    let previous_label = node.label.clone();
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed == "C" {
        let changed = previous_element != "C"
            || previous_atomic_number != 6
            || previous_is_placeholder
            || previous_label.is_some();
        if !changed {
            return false;
        }
        node.element = "C".to_string();
        node.atomic_number = 6;
        node.is_placeholder = false;
        node.label = None;
        return true;
    }

    if let Some(replacement) = classify_node_label_replacement(trimmed) {
        match replacement {
            NodeLabelReplacement::Carbon => {}
            NodeLabelReplacement::Element {
                element,
                atomic_number,
            } => {
                node.element = element.to_string();
                node.atomic_number = atomic_number;
                node.is_placeholder = false;
            }
            NodeLabelReplacement::Abbreviation => {
                node.element = "C".to_string();
                node.atomic_number = 6;
                node.is_placeholder = true;
            }
        }
    } else {
        node.element = "C".to_string();
        node.atomic_number = 6;
        node.is_placeholder = true;
    }

    let source_runs = normalize_source_runs(session, text);
    let session_font_size = session
        .font_size_world_cm()
        .unwrap_or(WorldCm(DEFAULT_TEXT_FONT_SIZE))
        .value();
    let display_runs = display_runs_from_source_runs(
        &source_runs,
        session
            .font_family
            .as_deref()
            .unwrap_or(DEFAULT_TEXT_FONT_FAMILY),
        session_font_size,
        session.fill.as_deref().unwrap_or(DEFAULT_TEXT_FILL),
    );
    let next_label = make_centered_node_label_from_runs(
        text,
        anchor_position,
        source_runs,
        display_runs,
        session
            .font_family
            .as_deref()
            .unwrap_or(DEFAULT_TEXT_FONT_FAMILY),
        session_font_size,
        session.fill.as_deref().unwrap_or(DEFAULT_TEXT_FILL),
        connection_angles,
        session,
    );
    let changed = previous_element != node.element
        || previous_atomic_number != node.atomic_number
        || previous_is_placeholder != node.is_placeholder
        || !same_node_label(previous_label.as_ref(), Some(&next_label));
    if !changed {
        return false;
    }
    node.label = Some(next_label);
    true
}

fn endpoint_label_editor_anchor_world(
    node: &crate::Node,
    object_translate: [f64; 2],
) -> Option<Point> {
    let label = node.label.as_ref()?;
    let bbox = label.bbox()?;
    Some(Point::new(
        bbox[0] + object_translate[0],
        bbox[1] + object_translate[1],
    ))
}

fn polygon_anchor_point(polygon: &[Point]) -> Option<Point> {
    if polygon.is_empty() {
        return None;
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in polygon {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Some(Point::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5))
}

fn payload_text(payload: &crate::ObjectPayload) -> String {
    payload
        .extra
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn payload_box(payload: &crate::ObjectPayload) -> Option<[f64; 4]> {
    payload
        .extra
        .get("box")
        .cloned()
        .and_then(|value| serde_json::from_value::<[f64; 4]>(value).ok())
}

fn payload_runs_or_text(payload: &crate::ObjectPayload) -> Vec<LabelRun> {
    if let Some(value) = payload.extra.get("runs").cloned() {
        if let Ok(runs) = serde_json::from_value::<Vec<LabelRun>>(value) {
            if !runs.is_empty() {
                return runs;
            }
        }
    }
    let text = payload_text(payload);
    if text.is_empty() {
        Vec::new()
    } else {
        vec![LabelRun {
            text,
            font_family: payload
                .extra
                .get("fontFamily")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            font_size: payload.extra.get("fontSize").and_then(Value::as_f64),
            fill: payload
                .extra
                .get("fill")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            script: Some("normal".to_string()),
            face: None,
        }]
    }
}

fn runs_text(runs: &[LabelRun]) -> String {
    runs.iter().map(|run| run.text.as_str()).collect()
}

fn normalize_source_runs(session: &TextEditSession, text: &str) -> Vec<LabelRun> {
    let source_runs = if !session.source_runs.is_empty() {
        session.source_runs.clone()
    } else if text.is_empty() {
        Vec::new()
    } else {
        vec![LabelRun {
            text: text.to_string(),
            font_family: session.font_family.clone(),
            font_size: session.font_size,
            fill: session.fill.clone(),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            script: Some(if session.default_chemical {
                "chemical".to_string()
            } else {
                "normal".to_string()
            }),
            face: None,
        }]
    };
    source_runs
        .into_iter()
        .filter(|run| !run.text.is_empty())
        .map(|mut run| {
            if run.font_family.is_none() {
                run.font_family = session.font_family.clone();
            }
            if run.font_size.is_none() {
                run.font_size = session.font_size;
            }
            if run.fill.is_none() {
                run.fill = session.fill.clone();
            }
            if run.font_weight.is_none() {
                run.font_weight = Some(400);
            }
            if run.font_style.is_none() {
                run.font_style = Some("normal".to_string());
            }
            if run.underline.is_none() {
                run.underline = Some(false);
            }
            if run.script.is_none() {
                run.script = Some(if session.default_chemical {
                    "chemical".to_string()
                } else {
                    "normal".to_string()
                });
            }
            run
        })
        .collect()
}

fn display_runs_from_source_runs(
    source_runs: &[LabelRun],
    fallback_font_family: &str,
    fallback_font_size: f64,
    fallback_fill: &str,
) -> Vec<LabelRun> {
    let mut out = Vec::new();
    for run in source_runs {
        if run.text.is_empty() {
            continue;
        }
        let base = LabelRun {
            text: String::new(),
            font_family: Some(
                run.font_family
                    .clone()
                    .unwrap_or_else(|| fallback_font_family.to_string()),
            ),
            font_size: Some(run.font_size.unwrap_or(fallback_font_size)),
            fill: Some(
                run.fill
                    .clone()
                    .unwrap_or_else(|| fallback_fill.to_string()),
            ),
            font_weight: Some(run.font_weight.unwrap_or(400)),
            font_style: Some(
                run.font_style
                    .clone()
                    .unwrap_or_else(|| "normal".to_string()),
            ),
            underline: Some(run.underline.unwrap_or(false)),
            script: Some("normal".to_string()),
            face: None,
        };
        match run.script.as_deref().unwrap_or("normal") {
            "chemical" => out.extend(expand_chemical_run(&base, &run.text)),
            "subscript" | "superscript" => {
                let mut next = base.clone();
                next.text = run.text.clone();
                next.script = run.script.clone();
                out.push(next);
            }
            _ => {
                let mut next = base.clone();
                next.text = run.text.clone();
                out.push(next);
            }
        }
    }
    merge_adjacent_runs(out)
}

fn merge_adjacent_runs(runs: Vec<LabelRun>) -> Vec<LabelRun> {
    let mut merged: Vec<LabelRun> = Vec::new();
    for run in runs {
        if let Some(previous) = merged.last_mut() {
            if previous.font_family == run.font_family
                && previous.font_size == run.font_size
                && previous.fill == run.fill
                && previous.font_weight == run.font_weight
                && previous.font_style == run.font_style
                && previous.script == run.script
            {
                previous.text.push_str(&run.text);
                continue;
            }
        }
        merged.push(run);
    }
    merged
}

fn expand_chemical_run(base: &LabelRun, text: &str) -> Vec<LabelRun> {
    let chars: Vec<char> = text.chars().collect();
    let mut scripts = vec!["normal"; chars.len()];

    for index in 0..chars.len() {
        let character = chars[index];
        if character.is_ascii_digit() && index > 0 && chars[index - 1].is_ascii_alphabetic() {
            scripts[index] = "subscript";
        }
        if matches!(character, '+' | '-') {
            scripts[index] = "superscript";
            if index > 0 && chars[index - 1].is_ascii_digit() {
                let previous_index = index - 1;
                if previous_index > 0 && !chars[previous_index - 1].is_whitespace() {
                    scripts[previous_index] = "superscript";
                }
            }
        }
    }

    let mut out = Vec::new();
    let mut buffer = String::new();
    let mut active_script = "normal";
    for (index, character) in chars.into_iter().enumerate() {
        let script = scripts[index];
        if !buffer.is_empty() && script != active_script {
            let mut run = base.clone();
            run.text = std::mem::take(&mut buffer);
            run.script = Some(active_script.to_string());
            out.push(run);
        }
        active_script = script;
        buffer.push(character);
    }
    if !buffer.is_empty() {
        let mut run = base.clone();
        run.text = buffer;
        run.script = Some(active_script.to_string());
        out.push(run);
    }
    out
}

#[derive(Clone)]
struct ResolvedTextEditLine {
    x: f64,
    y: f64,
    baseline_y: f64,
    height: f64,
    text_anchor: String,
    runs: Vec<LabelRun>,
}

#[derive(Clone)]
struct ResolvedTextEditCharBox {
    offset: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    line_index: usize,
}

fn normalize_text_edit_selection(
    text: &str,
    selection: Option<&TextEditSelection>,
) -> Option<TextEditSelectionState> {
    let Some(selection) = selection else {
        return None;
    };
    let text_length = text.chars().count();
    let anchor = selection.anchor.min(text_length);
    let focus = selection.focus.min(text_length);
    Some(TextEditSelectionState {
        anchor,
        focus,
        start: anchor.min(focus),
        end: anchor.max(focus),
        collapsed: anchor == focus,
    })
}

fn split_runs_by_line_preserving_empty(runs: &[LabelRun]) -> Vec<Vec<LabelRun>> {
    let mut lines = vec![Vec::new()];
    for run in runs {
        let segments: Vec<&str> = run.text.split('\n').collect();
        for (index, segment) in segments.iter().enumerate() {
            if !segment.is_empty() {
                let mut next_run = run.clone();
                next_run.text = (*segment).to_string();
                lines
                    .last_mut()
                    .expect("line vector always exists")
                    .push(next_run);
            }
            if index + 1 < segments.len() {
                lines.push(Vec::new());
            }
        }
    }
    if lines.is_empty() {
        vec![Vec::new()]
    } else {
        lines
    }
}

fn text_anchor_for_align(align: &str) -> String {
    match align {
        "right" => "end".to_string(),
        "center" => "middle".to_string(),
        _ => "start".to_string(),
    }
}

fn anchor_x_for_align(align: &str, width: f64) -> f64 {
    match align {
        "right" => width,
        "center" => width * 0.5,
        _ => 0.0,
    }
}

fn measure_text_edit_line_width(runs: &[LabelRun], fallback_font_size: f64) -> f64 {
    runs.iter().fold(0.0, |width, run| {
        let run_font_size = run.font_size.unwrap_or(fallback_font_size);
        width
            + run
                .text
                .chars()
                .map(|character| {
                    crate::shared_glyph_metrics(character, run_font_size, run.script.as_deref())
                        .advance
                })
                .sum::<f64>()
    })
}

fn build_text_edit_layout_geometry(
    text: String,
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    lines: Vec<ResolvedTextEditLine>,
    width: f64,
    height: f64,
    line_height: f64,
    anchor_offset: [f64; 2],
    selection: Option<TextEditSelectionState>,
    fallback_font_size: f64,
) -> TextEditLayout {
    let mut layout_lines = Vec::new();
    let mut caret_positions = Vec::new();
    let mut char_boxes = Vec::new();
    let mut offset = 0usize;

    for (line_index, line) in lines.iter().enumerate() {
        let mut caret_offsets = Vec::new();
        let line_start = offset;
        let mut cursor_x = line.x;
        let caret_y = line.y;
        let caret_height = line.height.max(0.0);
        let start_caret = TextEditLayoutCaret {
            offset,
            x: round6(cursor_x),
            y: round6(caret_y),
            height: round6(caret_height),
            line_index,
        };
        caret_offsets.push(TextEditLayoutCaretOffset {
            offset,
            x: round6(cursor_x),
        });
        caret_positions.push(start_caret);

        for run in &line.runs {
            let run_font_size = run.font_size.unwrap_or(fallback_font_size);
            for character in run.text.chars() {
                let metrics =
                    crate::shared_glyph_metrics(character, run_font_size, run.script.as_deref());
                let char_top = line.baseline_y + metrics.top;
                let char_bottom = line.baseline_y + metrics.bottom;
                char_boxes.push(ResolvedTextEditCharBox {
                    offset,
                    x: cursor_x,
                    y: char_top,
                    width: metrics.advance,
                    height: (char_bottom - char_top).max(0.0),
                    line_index,
                });
                cursor_x += metrics.advance;
                offset += 1;
                caret_offsets.push(TextEditLayoutCaretOffset {
                    offset,
                    x: round6(cursor_x),
                });
                caret_positions.push(TextEditLayoutCaret {
                    offset,
                    x: round6(cursor_x),
                    y: round6(caret_y),
                    height: round6(caret_height),
                    line_index,
                });
            }
        }

        let line_end = offset;
        layout_lines.push(TextEditLayoutLine {
            index: line_index,
            x: round6(line.x),
            y: round6(line.y),
            baseline_y: round6(line.baseline_y),
            height: round6(line.height),
            start_offset: line_start,
            end_offset: line_end,
            text_anchor: line.text_anchor.clone(),
            runs: line.runs.clone(),
            caret_offsets,
        });
        if line_index + 1 < lines.len() {
            offset += 1;
        }
    }

    let selection_rects = build_text_edit_selection_rects(&char_boxes, selection.as_ref());
    TextEditLayout {
        text,
        source_runs,
        display_runs,
        lines: layout_lines,
        width: round6(width),
        height: round6(height),
        line_height: round6(line_height),
        anchor_offset: [round6(anchor_offset[0]), round6(anchor_offset[1])],
        caret_positions,
        selection_rects,
        selection,
    }
}

fn build_text_edit_selection_rects(
    char_boxes: &[ResolvedTextEditCharBox],
    selection: Option<&TextEditSelectionState>,
) -> Vec<TextEditLayoutRect> {
    let Some(selection) = selection else {
        return Vec::new();
    };
    if selection.collapsed {
        return Vec::new();
    }
    let mut grouped: Vec<(usize, TextEditLayoutRect)> = Vec::new();
    for entry in char_boxes {
        if entry.offset < selection.start || entry.offset >= selection.end {
            continue;
        }
        if let Some((_, current)) = grouped
            .iter_mut()
            .find(|(line_index, _)| *line_index == entry.line_index)
        {
            current.x = current.x.min(entry.x);
            current.y = current.y.min(entry.y);
            current.width = current.width.max(entry.x + entry.width - current.x);
            current.height = current.height.max(entry.height);
            continue;
        }
        grouped.push((
            entry.line_index,
            TextEditLayoutRect {
                x: entry.x,
                y: entry.y,
                width: entry.width.max(0.0),
                height: entry.height.max(0.0),
            },
        ));
    }
    grouped
        .into_iter()
        .map(|(_, rect)| TextEditLayoutRect {
            x: round6(rect.x),
            y: round6(rect.y),
            width: round6(rect.width.max(0.0)),
            height: round6(rect.height.max(0.0)),
        })
        .collect()
}

fn build_text_object_edit_layout(
    session: &TextEditSession,
    text: String,
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    line_height: f64,
    selection: Option<TextEditSelectionState>,
) -> TextEditLayout {
    let fallback_font_size = session
        .font_size_world_cm()
        .unwrap_or(WorldCm(DEFAULT_TEXT_FONT_SIZE))
        .value();
    let align = session.align.as_deref().unwrap_or("left");
    let line_runs = split_runs_by_line_preserving_empty(&display_runs);
    let line_widths: Vec<f64> = line_runs
        .iter()
        .map(|runs| measure_text_edit_line_width(runs, fallback_font_size))
        .collect();
    let width = round2(
        line_widths
            .iter()
            .copied()
            .fold(TEXT_EDIT_BOX_WIDTH, f64::max),
    );
    let height = round2((line_height * line_runs.len().max(1) as f64).max(line_height));
    let text_anchor = text_anchor_for_align(align);
    let lines = line_runs
        .into_iter()
        .enumerate()
        .map(|(index, runs)| {
            let y = index as f64 * line_height;
            ResolvedTextEditLine {
                x: anchor_x_for_align(align, width),
                y,
                baseline_y: y + fallback_font_size * 0.82,
                height: line_height,
                text_anchor: text_anchor.clone(),
                runs,
            }
        })
        .collect();
    build_text_edit_layout_geometry(
        text,
        source_runs,
        display_runs,
        lines,
        width,
        height,
        line_height,
        [0.0, 0.0],
        selection,
        fallback_font_size,
    )
}

fn build_endpoint_label_edit_layout_from_label(
    text: String,
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    label: &crate::NodeLabel,
    local_anchor: [f64; 2],
    line_height: f64,
    selection: Option<TextEditSelectionState>,
) -> TextEditLayout {
    let fallback_font_size = label.font_size.unwrap_or(DEFAULT_TEXT_FONT_SIZE);
    let box_value = label.bbox().unwrap_or([
        local_anchor[0],
        local_anchor[1] - fallback_font_size * 0.42,
        local_anchor[0] + TEXT_EDIT_BOX_WIDTH,
        local_anchor[1] - fallback_font_size * 0.42 + line_height,
    ]);
    let baseline_x = label.position.map(|value| value[0]).unwrap_or(box_value[0]);
    let first_baseline_y = label
        .position
        .map(|value| value[1])
        .unwrap_or(box_value[1] + fallback_font_size * 0.82);
    let editor_origin_x = baseline_x;
    let editor_origin_y = box_value[1];
    let width = round2((box_value[2] - editor_origin_x).max(TEXT_EDIT_BOX_WIDTH));
    let height = round2((box_value[3] - editor_origin_y).max(line_height));
    let line_runs = if !label.line_runs.is_empty() {
        label.line_runs.clone()
    } else {
        vec![label.runs.clone()]
    };
    let lines = line_runs
        .into_iter()
        .enumerate()
        .map(|(index, runs)| {
            let y = if index == 0 {
                0.0
            } else {
                index as f64 * line_height
            };
            let baseline_y = if index == 0 {
                first_baseline_y - editor_origin_y
            } else {
                y + line_height * 0.82
            };
            ResolvedTextEditLine {
                x: 0.0,
                y,
                baseline_y,
                height: line_height,
                text_anchor: "start".to_string(),
                runs,
            }
        })
        .collect();
    build_text_edit_layout_geometry(
        text,
        source_runs,
        display_runs,
        lines,
        width,
        height,
        line_height,
        [
            local_anchor[0] - editor_origin_x,
            local_anchor[1] - editor_origin_y,
        ],
        selection,
        fallback_font_size,
    )
}

fn estimate_text_block_size(runs: &[LabelRun], font_size: f64, line_height: f64) -> (f64, f64) {
    let mut max_width = font_size * 0.6;
    let mut line_width = 0.0;
    let mut line_count = 1usize;

    for run in runs {
        let script_scale = crate::glyph_kernel::shared_script_scale_factor(run.script.as_deref());
        let run_font_size = run.font_size.unwrap_or(font_size) * script_scale;
        for character in run.text.chars() {
            if character == '\n' {
                max_width = max_width.max(line_width);
                line_width = 0.0;
                line_count += 1;
                continue;
            }
            line_width += estimated_char_width(character, run_font_size);
        }
    }
    max_width = max_width.max(line_width);
    let width = round2((max_width + font_size * 0.24).max(crate::px_to_cm(8.0)));
    let height = round2((line_height * line_count as f64).max(line_height));
    (width, height)
}

fn estimated_char_width(character: char, font_size: f64) -> f64 {
    crate::glyph_kernel::shared_estimated_char_width(character, font_size)
}

#[allow(clippy::too_many_arguments)]
fn update_text_object_fields(
    object: &mut crate::SceneObject,
    x: f64,
    y: f64,
    text: &str,
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    session: &TextEditSession,
    width: f64,
    height: f64,
) -> bool {
    let next_payload = make_text_payload(text, source_runs, display_runs, session, width, height);
    let changed =
        object.transform.translate != [x, y] || object.payload.extra != next_payload.extra;
    if !changed {
        return false;
    }
    object.transform.translate = [round2(x), round2(y)];
    object.payload = next_payload;
    object.style_ref = None;
    object.visible = true;
    object.locked = false;
    true
}

#[allow(clippy::too_many_arguments)]
fn make_text_object(
    object_id: &str,
    x: f64,
    y: f64,
    text: &str,
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    session: &TextEditSession,
    width: f64,
    height: f64,
    z_index: i32,
) -> crate::SceneObject {
    crate::SceneObject {
        id: object_id.to_string(),
        object_type: "text".to_string(),
        name: "text".to_string(),
        visible: true,
        locked: false,
        z_index,
        transform: crate::Transform {
            translate: [round2(x), round2(y)],
            rotate: 0.0,
            scale: [1.0, 1.0],
        },
        style_ref: None,
        meta: Value::Null,
        payload: make_text_payload(text, source_runs, display_runs, session, width, height),
    }
}

fn make_text_payload(
    text: &str,
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    session: &TextEditSession,
    width: f64,
    height: f64,
) -> crate::ObjectPayload {
    let mut extra = std::collections::BTreeMap::new();
    extra.insert("text".to_string(), Value::String(text.to_string()));
    extra.insert(
        "align".to_string(),
        Value::String(session.align.clone().unwrap_or_else(|| "left".to_string())),
    );
    extra.insert("valign".to_string(), Value::String("top".to_string()));
    extra.insert("preserveLines".to_string(), Value::Bool(true));
    extra.insert(
        "fontFamily".to_string(),
        Value::String(
            session
                .font_family
                .clone()
                .unwrap_or_else(|| "Arial".to_string()),
        ),
    );
    extra.insert(
        "fontSize".to_string(),
        json!(round6(
            session
                .font_size_world_cm()
                .unwrap_or(WorldCm(DEFAULT_TEXT_FONT_SIZE))
                .value()
        )),
    );
    extra.insert(
        "fill".to_string(),
        Value::String(
            session
                .fill
                .clone()
                .unwrap_or_else(|| "#000000".to_string()),
        ),
    );
    extra.insert(
        "lineHeight".to_string(),
        json!(round6(
            session
                .line_height
                .unwrap_or(DEFAULT_TEXT_BLOCK_LINE_HEIGHT)
        )),
    );
    extra.insert("box".to_string(), json!([0.0, 0.0, width, height]));
    extra.insert(
        "runs".to_string(),
        serde_json::to_value(display_runs).unwrap_or(Value::Array(Vec::new())),
    );
    extra.insert(
        "sourceRuns".to_string(),
        serde_json::to_value(source_runs).unwrap_or(Value::Array(Vec::new())),
    );
    crate::ObjectPayload {
        resource_ref: None,
        bbox: Some([0.0, 0.0, width, height]),
        extra,
    }
}

pub(super) fn text_object_world_bounds(object: &crate::SceneObject) -> Option<[f64; 4]> {
    let local_box = payload_box(&object.payload).or(object
        .payload
        .bbox
        .map(|bbox| [bbox[0], bbox[1], bbox[2], bbox[3]]))?;
    let x = object.transform.translate[0] + local_box[0];
    let y = object.transform.translate[1] + local_box[1];
    Some([x, y, x + local_box[2], y + local_box[3]])
}

pub(super) fn endpoint_label_world_bounds(
    node: &crate::Node,
    object_translate: [f64; 2],
) -> Option<[f64; 4]> {
    let bbox = node.label.as_ref()?.bbox()?;
    Some([
        round6(bbox[0] + object_translate[0]),
        round6(bbox[1] + object_translate[1]),
        round6(bbox[2] + object_translate[0]),
        round6(bbox[3] + object_translate[1]),
    ])
}

#[derive(Clone, Copy)]
enum NodeLabelReplacement<'a> {
    Carbon,
    Element { element: &'a str, atomic_number: u8 },
    Abbreviation,
}

fn classify_node_label_replacement(label: &str) -> Option<NodeLabelReplacement<'_>> {
    match label {
        "C" => Some(NodeLabelReplacement::Carbon),
        "H" => Some(NodeLabelReplacement::Element {
            element: "H",
            atomic_number: 1,
        }),
        "N" => Some(NodeLabelReplacement::Element {
            element: "N",
            atomic_number: 7,
        }),
        "O" => Some(NodeLabelReplacement::Element {
            element: "O",
            atomic_number: 8,
        }),
        "S" => Some(NodeLabelReplacement::Element {
            element: "S",
            atomic_number: 16,
        }),
        "P" => Some(NodeLabelReplacement::Element {
            element: "P",
            atomic_number: 15,
        }),
        "F" => Some(NodeLabelReplacement::Element {
            element: "F",
            atomic_number: 9,
        }),
        "Cl" => Some(NodeLabelReplacement::Element {
            element: "Cl",
            atomic_number: 17,
        }),
        "Br" => Some(NodeLabelReplacement::Element {
            element: "Br",
            atomic_number: 35,
        }),
        "I" => Some(NodeLabelReplacement::Element {
            element: "I",
            atomic_number: 53,
        }),
        "Si" => Some(NodeLabelReplacement::Element {
            element: "Si",
            atomic_number: 14,
        }),
        "Na" => Some(NodeLabelReplacement::Element {
            element: "Na",
            atomic_number: 11,
        }),
        "B" => Some(NodeLabelReplacement::Element {
            element: "B",
            atomic_number: 5,
        }),
        "D" => Some(NodeLabelReplacement::Element {
            element: "D",
            atomic_number: 1,
        }),
        "Me" => Some(NodeLabelReplacement::Abbreviation),
        "Ph" => Some(NodeLabelReplacement::Abbreviation),
        _ => None,
    }
}

fn make_centered_node_label(text: &str, position: [f64; 2]) -> crate::NodeLabel {
    let font_size = DEFAULT_CENTERED_LABEL_FONT_SIZE;
    let (label_position, label_box) = estimated_centered_label_geometry(text, position, font_size);
    crate::NodeLabel {
        text: text.to_string(),
        source_text: Some(text.to_string()),
        position: Some(label_position),
        box_field: Some(label_box),
        runs: vec![crate::LabelRun {
            text: text.to_string(),
            font_family: Some("Arial".to_string()),
            font_size: Some(font_size),
            fill: Some("#000000".to_string()),
            font_weight: Some(700),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            script: Some("normal".to_string()),
            face: None,
        }],
        line_runs: Vec::new(),
        lines: Vec::new(),
        align: Some("center".to_string()),
        layout: None,
        attachment: None,
        anchor: Some("middle".to_string()),
        font_family: Some("Arial".to_string()),
        fill: Some("#000000".to_string()),
        font_size: Some(font_size),
        glyph_polygons: Vec::new(),
        box_value: Some(label_box),
        meta: serde_json::Value::Null,
    }
}

fn endpoint_session_box_size(session: &TextEditSession) -> Option<(f64, f64)> {
    let [x1, y1, x2, y2] = session.box_value?;
    let width = (x2 - x1).abs();
    let height = (y2 - y1).abs();
    if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
        Some((width, height))
    } else {
        None
    }
}

fn make_centered_node_label_from_runs(
    text: &str,
    position: [f64; 2],
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    font_family: &str,
    font_size: f64,
    fill: &str,
    connection_angles: &[f64],
    session: &TextEditSession,
) -> crate::NodeLabel {
    let decision = decide_label_layout(connection_angles, false, false);
    let layout = layout_label_text(text, &decision);
    let (lines, line_runs) = layout_display_runs(&display_runs, &decision);
    let line_height = (font_size * 1.05).max(font_size);
    let estimated_width = lines
        .iter()
        .zip(line_runs.iter())
        .map(|(_, runs)| estimate_line_runs_width(runs, font_size))
        .fold(font_size * 0.6, f64::max);
    let estimated_height = round2((line_height * lines.len().max(1) as f64).max(line_height));
    let anchor_prefix_width = line_runs
        .get(layout.anchor_line)
        .map(|runs| estimate_prefix_width(runs, layout.anchor_char, font_size))
        .unwrap_or(0.0);
    let anchor_char_width = line_runs
        .get(layout.anchor_line)
        .and_then(|runs| estimate_anchor_char_width(runs, layout.anchor_char, font_size))
        .unwrap_or(font_size * 0.62);
    let anchor_center_x = anchor_prefix_width + anchor_char_width * 0.5;
    let can_use_measured_geometry =
        matches!(decision.flow, LabelFlow::Forward) && lines.len() == 1 && layout.anchor_line == 0;
    let measured_anchor = session
        .anchor_offset_world_cm()
        .map(|value| (value[0].value(), value[1].value()));
    let measured_box_size = endpoint_session_box_size(session);
    let fallback_geometry = || {
        let x1 = round2(position[0] - anchor_center_x);
        let y1 = round2(position[1] - font_size * 0.42 - layout.anchor_line as f64 * line_height);
        let baseline_y = round2(y1 + layout.anchor_line as f64 * line_height + font_size * 0.82);
        (estimated_width, estimated_height, x1, y1, baseline_y)
    };
    let (width, height, mut x1, mut y1, mut baseline_y) = if can_use_measured_geometry {
        if let (Some((anchor_offset_x, anchor_offset_y)), Some((measured_width, measured_height))) =
            (measured_anchor, measured_box_size)
        {
            const MAX_MEASURED_SIZE_RATIO: f64 = 8.0;
            let max_width = estimated_width.max(font_size) * MAX_MEASURED_SIZE_RATIO;
            let max_height = estimated_height.max(font_size) * MAX_MEASURED_SIZE_RATIO;
            let valid_anchor_x = anchor_offset_x.is_finite()
                && anchor_offset_x >= -estimated_width * 0.25
                && anchor_offset_x <= max_width;
            let valid_anchor_y = anchor_offset_y.is_finite()
                && anchor_offset_y >= -estimated_height * 0.25
                && anchor_offset_y <= max_height;
            let valid_size = measured_width.is_finite()
                && measured_height.is_finite()
                && measured_width > 0.0
                && measured_height > 0.0
                && measured_width <= max_width
                && measured_height <= max_height;
            if valid_anchor_x && valid_anchor_y && valid_size {
                let x1 = round2(position[0] - anchor_offset_x);
                let y1 = round2(position[1] - anchor_offset_y);
                let width = round2(measured_width.max(estimated_width));
                let height = round2(measured_height.max(estimated_height));
                let baseline_y = round2(y1 + font_size * 0.82);
                (width, height, x1, y1, baseline_y)
            } else {
                fallback_geometry()
            }
        } else {
            fallback_geometry()
        }
    } else {
        fallback_geometry()
    };
    let mut x2 = round2(x1 + width);
    let mut y2 = round2(y1 + height);
    let mut meta = serde_json::Map::new();
    meta.insert(
        "sourceRuns".to_string(),
        serde_json::to_value(source_runs).unwrap_or(Value::Array(Vec::new())),
    );
    let mut glyph_polygons = build_label_glyph_polygons(
        if line_runs.len() == 1 {
            line_runs.first().map(Vec::as_slice).unwrap_or(&[])
        } else {
            &[]
        },
        if line_runs.len() > 1 { &line_runs } else { &[] },
        [x1, baseline_y],
        Some([x1, y1, x2, y2]),
        font_size,
    );
    if lines.len() == 1 {
        if let Some(current_anchor) = glyph_polygons.get(layout.anchor_char).and_then(|polygon| {
            let points: Vec<_> = polygon
                .iter()
                .map(|point| Point::new(point[0], point[1]))
                .collect();
            polygon_anchor_point(&points)
        }) {
            let dx = round2(position[0] - current_anchor.x);
            let dy = round2(position[1] - current_anchor.y);
            if dx.abs() > crate::EPSILON || dy.abs() > crate::EPSILON {
                x1 = round2(x1 + dx);
                y1 = round2(y1 + dy);
                x2 = round2(x2 + dx);
                y2 = round2(y2 + dy);
                baseline_y = round2(baseline_y + dy);
                for polygon in &mut glyph_polygons {
                    for point in polygon {
                        point[0] = round2(point[0] + dx);
                        point[1] = round2(point[1] + dy);
                    }
                }
            }
        }
    }
    crate::NodeLabel {
        text: layout.rendered_text,
        source_text: Some(text.to_string()),
        position: Some([x1, baseline_y]),
        box_field: Some([x1, y1, x2, y2]),
        runs: if line_runs.len() == 1 {
            line_runs.first().cloned().unwrap_or_default()
        } else {
            Vec::new()
        },
        line_runs: if line_runs.len() > 1 {
            line_runs
        } else {
            Vec::new()
        },
        lines: if lines.len() > 1 {
            lines.clone()
        } else {
            Vec::new()
        },
        align: Some("left".to_string()),
        layout: Some(match decision.flow {
            LabelFlow::StackAbove => "attached-group-above".to_string(),
            _ => "attached-group".to_string(),
        }),
        attachment: Some("node".to_string()),
        anchor: Some("start".to_string()),
        font_family: Some(font_family.to_string()),
        fill: Some(fill.to_string()),
        font_size: Some(font_size),
        glyph_polygons,
        box_value: Some([x1, y1, x2, y2]),
        meta: Value::Object(meta),
    }
}

#[derive(Clone)]
struct StyledGlyph {
    ch: char,
    run: LabelRun,
}

fn layout_display_runs(
    display_runs: &[LabelRun],
    decision: &crate::LabelLayoutDecision,
) -> (Vec<String>, Vec<Vec<LabelRun>>) {
    let groups = split_styled_groups(display_runs);
    if groups.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let lines = match decision.flow {
        LabelFlow::Forward => vec![groups.concat()],
        LabelFlow::Reverse => vec![groups.into_iter().rev().flatten().collect()],
        LabelFlow::StackAbove => {
            if groups.len() > 1 {
                vec![groups[1..].concat(), groups[0].clone()]
            } else {
                vec![groups[0].clone()]
            }
        }
        LabelFlow::StackBelow => {
            if groups.len() > 1 {
                vec![groups[0].clone(), groups[1..].concat()]
            } else {
                vec![groups[0].clone()]
            }
        }
    };
    let line_texts = lines
        .iter()
        .map(|line| line.iter().map(|glyph| glyph.ch).collect::<String>())
        .collect::<Vec<_>>();
    let line_runs = lines
        .iter()
        .map(|line| merge_styled_glyph_runs(line))
        .collect();
    (line_texts, line_runs)
}

fn split_styled_groups(display_runs: &[LabelRun]) -> Vec<Vec<StyledGlyph>> {
    let mut groups = Vec::new();
    let mut current = Vec::new();
    for run in display_runs {
        for ch in run.text.chars() {
            if ch.is_whitespace() {
                continue;
            }
            if ch.is_ascii_uppercase() && !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
            current.push(StyledGlyph {
                ch,
                run: LabelRun {
                    text: ch.to_string(),
                    font_family: run.font_family.clone(),
                    font_size: run.font_size,
                    fill: run.fill.clone(),
                    font_weight: run.font_weight,
                    font_style: run.font_style.clone(),
                    underline: run.underline,
                    script: run.script.clone(),
                    face: run.face,
                },
            });
        }
    }
    if !current.is_empty() {
        groups.push(current);
    }
    groups
}

fn merge_styled_glyph_runs(line: &[StyledGlyph]) -> Vec<LabelRun> {
    let mut runs: Vec<LabelRun> = Vec::new();
    for glyph in line {
        if let Some(previous) = runs.last_mut() {
            if previous.font_family == glyph.run.font_family
                && previous.font_size == glyph.run.font_size
                && previous.fill == glyph.run.fill
                && previous.font_weight == glyph.run.font_weight
                && previous.font_style == glyph.run.font_style
                && previous.underline == glyph.run.underline
                && previous.script == glyph.run.script
            {
                previous.text.push(glyph.ch);
                continue;
            }
        }
        let mut next = glyph.run.clone();
        next.text = glyph.ch.to_string();
        runs.push(next);
    }
    runs
}

fn estimate_line_runs_width(runs: &[LabelRun], fallback_font_size: f64) -> f64 {
    runs.iter().fold(0.0, |width, run| {
        let run_font_size = run.font_size.unwrap_or(fallback_font_size)
            * crate::glyph_kernel::shared_script_scale_factor(run.script.as_deref());
        width
            + run
                .text
                .chars()
                .map(|ch| estimated_char_width(ch, run_font_size))
                .sum::<f64>()
    })
}

fn estimate_prefix_width(runs: &[LabelRun], char_count: usize, fallback_font_size: f64) -> f64 {
    let mut remaining = char_count;
    let mut width = 0.0;
    for run in runs {
        if remaining == 0 {
            break;
        }
        let run_font_size = run.font_size.unwrap_or(fallback_font_size)
            * crate::glyph_kernel::shared_script_scale_factor(run.script.as_deref());
        for ch in run.text.chars() {
            if remaining == 0 {
                break;
            }
            width += estimated_char_width(ch, run_font_size);
            remaining -= 1;
        }
    }
    width
}

fn estimate_anchor_char_width(
    runs: &[LabelRun],
    char_index: usize,
    fallback_font_size: f64,
) -> Option<f64> {
    let mut current_index = 0usize;
    for run in runs {
        let run_font_size = run.font_size.unwrap_or(fallback_font_size)
            * crate::glyph_kernel::shared_script_scale_factor(run.script.as_deref());
        for ch in run.text.chars() {
            if current_index == char_index {
                return Some(estimated_char_width(ch, run_font_size));
            }
            current_index += 1;
        }
    }
    None
}

fn adjacent_angles_for_fragment_node(
    fragment: &crate::MoleculeFragment,
    node_id: &str,
) -> Vec<f64> {
    let Some(node) = fragment.nodes.iter().find(|node| node.id == node_id) else {
        return Vec::new();
    };
    let point = Point::new(node.position[0], node.position[1]);
    let mut out = Vec::new();
    for bond in &fragment.bonds {
        if bond.begin != node_id && bond.end != node_id {
            continue;
        }
        let other_id = if bond.begin == node_id {
            &bond.end
        } else {
            &bond.begin
        };
        let Some(other) = fragment.nodes.iter().find(|node| &node.id == other_id) else {
            continue;
        };
        out.push(crate::angle_between(
            point,
            Point::new(other.position[0], other.position[1]),
        ));
    }
    out
}

fn same_node_label(current: Option<&crate::NodeLabel>, next: Option<&crate::NodeLabel>) -> bool {
    match (current, next) {
        (None, None) => true,
        (Some(current), Some(next)) => {
            current.text == next.text
                && current.align == next.align
                && current.runs == next.runs
                && current.font_family == next.font_family
                && current.font_size == next.font_size
                && current.fill == next.fill
        }
        _ => false,
    }
}

pub(super) fn prune_unconnected_fragment_nodes(fragment: &mut crate::MoleculeFragment) {
    let connected_nodes: BTreeSet<String> = fragment
        .bonds
        .iter()
        .flat_map(|bond| [bond.begin.clone(), bond.end.clone()])
        .collect();
    fragment
        .nodes
        .retain(|node| connected_nodes.contains(&node.id));
}

fn estimated_centered_label_geometry(
    text: &str,
    center: [f64; 2],
    font_size: f64,
) -> ([f64; 2], [f64; 4]) {
    let width = text
        .chars()
        .map(|ch| estimated_char_width(ch, font_size))
        .sum::<f64>()
        .max(crate::glyph_kernel::shared_estimated_char_width(
            'C', font_size,
        ));
    let height = (font_size * 0.84).max(crate::px_to_cm(8.0));
    let half_width = width * 0.5;
    let half_height = height * 0.5;
    let x1 = center[0] - half_width;
    let y1 = center[1] - half_height;
    let x2 = center[0] + half_width;
    let y2 = center[1] + half_height;
    ([center[0], y2], [x1, y1, x2, y2])
}

pub(super) fn refresh_attached_node_label_geometry_for_all_nodes(
    fragment: &mut crate::MoleculeFragment,
    object_translate: [f64; 2],
    stroke_width: f64,
) {
    let node_ids: Vec<_> = fragment.nodes.iter().map(|node| node.id.clone()).collect();
    for node_id in node_ids {
        refresh_attached_node_label_geometry_for_node(
            fragment,
            object_translate,
            &node_id,
            stroke_width,
        );
    }
}

pub(super) fn refresh_attached_node_label_geometry_for_node(
    fragment: &mut crate::MoleculeFragment,
    object_translate: [f64; 2],
    node_id: &str,
    stroke_width: f64,
) {
    let Some(node_index) = fragment.nodes.iter().position(|node| node.id == node_id) else {
        return;
    };
    let Some(next_label) =
        refreshed_attached_node_label(fragment, node_id, object_translate, stroke_width)
    else {
        return;
    };
    fragment.nodes[node_index].label = Some(next_label);
}

fn is_generated_centered_label(label: &crate::NodeLabel) -> bool {
    label.align.as_deref() == Some("center")
        && label.anchor.as_deref() == Some("middle")
        && label.glyph_polygons.is_empty()
        && label.runs.len() == 1
}

fn is_attached_node_label(label: &crate::NodeLabel) -> bool {
    label.attachment.as_deref() == Some("node")
        && label.align.as_deref() == Some("left")
        && label.anchor.as_deref() == Some("start")
}

fn refreshed_attached_node_label(
    fragment: &crate::MoleculeFragment,
    node_id: &str,
    object_translate: [f64; 2],
    stroke_width: f64,
) -> Option<crate::NodeLabel> {
    let node = fragment.nodes.iter().find(|node| node.id == node_id)?;
    let label = node.label.as_ref()?;
    let world_anchor =
        attached_node_label_anchor_world(fragment, node_id, object_translate, stroke_width);
    let local_anchor = [
        round2(world_anchor.x - object_translate[0]),
        round2(world_anchor.y - object_translate[1]),
    ];
    if is_generated_centered_label(label) {
        return Some(make_centered_node_label(&label.text, local_anchor));
    }
    if !is_attached_node_label(label) {
        return None;
    }

    let source_runs = source_runs_from_node_label(label);
    let text = if !source_runs.is_empty() {
        runs_text(&source_runs)
    } else {
        label
            .source_text
            .clone()
            .unwrap_or_else(|| label.text.clone())
    };
    let font_family = label
        .font_family
        .clone()
        .unwrap_or_else(|| DEFAULT_TEXT_FONT_FAMILY.to_string());
    let font_size = WorldCm(label.font_size.unwrap_or(DEFAULT_TEXT_FONT_SIZE)).value();
    let fill = label
        .fill
        .clone()
        .unwrap_or_else(|| DEFAULT_TEXT_FILL.to_string());
    let display_runs = display_runs_from_source_runs(&source_runs, &font_family, font_size, &fill);
    let connection_angles = adjacent_angles_for_fragment_node(fragment, node_id);
    let (anchor_offset, box_value) = current_node_label_editor_geometry(node, object_translate);
    let session = TextEditSession {
        target: TextEditTarget::EndpointLabel {
            node_id: node_id.to_string(),
            x: world_anchor.x,
            y: world_anchor.y,
        },
        text: text.clone(),
        source_runs: source_runs.clone(),
        font_family: Some(font_family.clone()),
        font_size: Some(font_size),
        fill: Some(fill.clone()),
        align: Some("left".to_string()),
        line_height: Some((font_size * 1.05).max(font_size)),
        box_value,
        anchor_offset,
        preserve_lines: true,
        default_chemical: source_runs
            .iter()
            .any(|run| run.script.as_deref() == Some("chemical")),
    };
    Some(make_centered_node_label_from_runs(
        &text,
        local_anchor,
        source_runs,
        display_runs,
        &font_family,
        font_size,
        &fill,
        &connection_angles,
        &session,
    ))
}

fn source_runs_from_node_label(label: &crate::NodeLabel) -> Vec<LabelRun> {
    label
        .meta
        .get("sourceRuns")
        .cloned()
        .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
        .or_else(|| (!label.runs.is_empty()).then(|| label.runs.clone()))
        .unwrap_or_else(|| {
            let text = label
                .source_text
                .clone()
                .unwrap_or_else(|| label.text.clone());
            if text.is_empty() {
                Vec::new()
            } else {
                vec![LabelRun {
                    text,
                    font_family: label.font_family.clone(),
                    font_size: label.font_size,
                    fill: label.fill.clone(),
                    font_weight: Some(400),
                    font_style: Some("normal".to_string()),
                    underline: Some(false),
                    script: Some("normal".to_string()),
                    face: None,
                }]
            }
        })
}

fn current_node_label_editor_geometry(
    node: &crate::Node,
    object_translate: [f64; 2],
) -> (Option<[f64; 2]>, Option<[f64; 4]>) {
    let Some(bounds) = endpoint_label_world_bounds(node, object_translate) else {
        return (None, None);
    };
    let anchor_offset = endpoint_label_editor_anchor_world(node, object_translate)
        .map(|anchor| [round6(anchor.x - bounds[0]), round6(anchor.y - bounds[1])]);
    (anchor_offset, Some(bounds))
}

fn attached_node_label_anchor_world(
    fragment: &crate::MoleculeFragment,
    node_id: &str,
    object_translate: [f64; 2],
    stroke_width: f64,
) -> Point {
    let Some(node) = fragment.nodes.iter().find(|node| node.id == node_id) else {
        return Point::new(object_translate[0], object_translate[1]);
    };
    let node_world = Point::new(
        object_translate[0] + node.position[0],
        object_translate[1] + node.position[1],
    );
    let connected: Vec<_> = fragment
        .bonds
        .iter()
        .filter(|bond| bond.begin == node_id || bond.end == node_id)
        .collect();
    if connected.len() != 1 || connected[0].order != 2 {
        return node_world;
    }
    let bond = connected[0];
    let Some(begin_node) = fragment.nodes.iter().find(|other| other.id == bond.begin) else {
        return node_world;
    };
    let Some(end_node) = fragment.nodes.iter().find(|other| other.id == bond.end) else {
        return node_world;
    };
    let placement = bond
        .double
        .as_ref()
        .map(|double| double.placement)
        .unwrap_or(DoubleBondPlacement::Center);
    if placement == DoubleBondPlacement::Center {
        return node_world;
    }
    let begin_world = Point::new(
        object_translate[0] + begin_node.position[0],
        object_translate[1] + begin_node.position[1],
    );
    let end_world = Point::new(
        object_translate[0] + end_node.position[0],
        object_translate[1] + end_node.position[1],
    );
    let dx = end_world.x - begin_world.x;
    let dy = end_world.y - begin_world.y;
    let length = dx.hypot(dy);
    if length <= crate::EPSILON {
        return node_world;
    }
    let side = if placement == DoubleBondPlacement::Left {
        -1.0
    } else {
        1.0
    };
    let normal_x = -dy / length;
    let normal_y = dx / length;
    let offset =
        0.5 * side_double_center_distance_for_bond_points(
            bond,
            begin_world,
            end_world,
            stroke_width,
            placement,
        ) * side;
    Point::new(
        node_world.x + normal_x * offset,
        node_world.y + normal_y * offset,
    )
}

fn bold_weight_stroke_width_for_engine(stroke_width: f64) -> f64 {
    (crate::BOLD_BOND_WIDTH_CM.value() * (stroke_width / crate::DEFAULT_BOND_STROKE_CM))
        .max(stroke_width)
}

fn bond_line_weight_stroke_width_for_engine(stroke_width: f64, weight: BondLineWeight) -> f64 {
    if weight == BondLineWeight::Bold {
        bold_weight_stroke_width_for_engine(stroke_width)
    } else {
        stroke_width
    }
}

fn side_double_center_distance_for_bond_points(
    bond: &Bond,
    start: Point,
    end: Point,
    stroke_width: f64,
    placement: DoubleBondPlacement,
) -> f64 {
    let outer_weight = match placement {
        DoubleBondPlacement::Left => bond.line_weights.right,
        DoubleBondPlacement::Right => bond.line_weights.left,
        DoubleBondPlacement::Center => BondLineWeight::Normal,
    };
    let main_width = bond_line_weight_stroke_width_for_engine(stroke_width, bond.line_weights.main);
    let outer_width = bond_line_weight_stroke_width_for_engine(stroke_width, outer_weight);
    start.distance(end) * 0.12 + 0.5 * (main_width + outer_width)
}
