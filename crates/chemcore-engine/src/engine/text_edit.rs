use super::{
    EditorCommand, Engine, PendingSelectTarget, TextEditCommandTarget, TextEditLayoutRequest,
};
use crate::{
    build_label_glyph_polygons, decide_label_layout, layout_label_text, round2, round6, Bond,
    BondLineWeight, DoubleBondPlacement, EndpointHit, LabelFlow, LabelRun, Point, WorldPoint,
    WorldPt,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const DEFAULT_TEXT_FONT_FAMILY: &str = "Arial";
const DEFAULT_TEXT_FONT_SIZE: f64 = crate::DEFAULT_TEXT_FONT_SIZE_PT;
const DEFAULT_TEXT_FILL: &str = "#000000";
const DEFAULT_TEXT_LINE_HEIGHT: f64 = crate::DEFAULT_TEXT_LINE_HEIGHT_PT;
const DEFAULT_TEXT_BLOCK_LINE_HEIGHT: f64 = crate::DEFAULT_TEXT_BLOCK_LINE_HEIGHT_PT;
const DEFAULT_CENTERED_LABEL_FONT_SIZE: f64 = crate::DEFAULT_CENTERED_LABEL_FONT_SIZE_PT;
const TEXT_EDIT_BOX_WIDTH: f64 = crate::px_to_pt(8.0);
const IMPLICIT_HYDROGEN_LABEL_META_KEY: &str = "implicitHydrogenLabel";

#[path = "text_edit/geometry.rs"]
mod geometry;
#[path = "text_edit/labels.rs"]
mod labels;
#[path = "text_edit/layout.rs"]
mod layout;
#[path = "text_edit/runs.rs"]
mod runs;

use self::geometry::*;
use self::labels::*;
pub(crate) use self::labels::{
    element_symbol_info, formula_hydrogen_count_for_node, implicit_hydrogen_label_text_for_count,
    mark_shortcut_implicit_hydrogen_label, refresh_attached_node_label_geometry_for_all_nodes,
    refresh_attached_node_label_geometry_for_node,
    refresh_element_valence_recognition_for_all_nodes, standalone_element_hydrogen_count,
};
use self::layout::*;
use self::runs::*;

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
                WorldPoint::new(WorldPt(*x), WorldPt(*y))
            }
        }
    }
}

impl TextEditSession {
    pub const fn font_size_world_pt(&self) -> Option<WorldPt> {
        match self.font_size {
            Some(value) => Some(WorldPt(value)),
            None => None,
        }
    }

    pub const fn line_height_world_pt(&self) -> Option<WorldPt> {
        match self.line_height {
            Some(value) => Some(WorldPt(value)),
            None => None,
        }
    }

    pub const fn target_world_point(&self) -> WorldPoint {
        self.target.world_point()
    }

    pub const fn anchor_offset_world_pt(&self) -> Option<[WorldPt; 2]> {
        match self.anchor_offset {
            Some([x, y]) => Some([WorldPt(x), WorldPt(y)]),
            None => None,
        }
    }
}

pub(crate) fn make_periodic_element_node_label(text: &str, position: [f64; 2]) -> crate::NodeLabel {
    let font_size = DEFAULT_CENTERED_LABEL_FONT_SIZE;
    let session = TextEditSession {
        target: TextEditTarget::EndpointLabel {
            node_id: String::new(),
            x: position[0],
            y: position[1],
        },
        text: text.to_string(),
        source_runs: Vec::new(),
        font_family: Some(DEFAULT_TEXT_FONT_FAMILY.to_string()),
        font_size: Some(font_size),
        fill: Some(DEFAULT_TEXT_FILL.to_string()),
        align: Some("center".to_string()),
        line_height: Some(DEFAULT_TEXT_LINE_HEIGHT),
        box_value: None,
        anchor_offset: None,
        preserve_lines: false,
        default_chemical: true,
    };
    let source_runs = normalize_source_runs(&session, text);
    let display_runs = display_runs_from_source_runs(
        &source_runs,
        DEFAULT_TEXT_FONT_FAMILY,
        font_size,
        DEFAULT_TEXT_FILL,
    );
    make_centered_node_label_from_runs(
        text,
        position,
        source_runs,
        display_runs,
        DEFAULT_TEXT_FONT_FAMILY,
        font_size,
        DEFAULT_TEXT_FILL,
        &[],
        &session,
        false,
        false,
        None,
    )
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
        children: Vec::new(),
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
    let align = session.align.clone().unwrap_or_else(|| "left".to_string());
    let local_box = text_object_box_for_align(&align, width, height);
    extra.insert("text".to_string(), Value::String(text.to_string()));
    extra.insert("align".to_string(), Value::String(align));
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
                .font_size_world_pt()
                .unwrap_or(WorldPt(DEFAULT_TEXT_FONT_SIZE))
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
    extra.insert(
        "box".to_string(),
        json!([
            round6(local_box[0]),
            round6(local_box[1]),
            round6(local_box[2]),
            round6(local_box[3])
        ]),
    );
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
        bbox: Some([
            round6(local_box[0]),
            round6(local_box[1]),
            round6(local_box[2]),
            round6(local_box[3]),
        ]),
        extra,
    }
}

pub(crate) fn text_object_world_bounds(object: &crate::SceneObject) -> Option<[f64; 4]> {
    let local_box = payload_box(&object.payload).or(object
        .payload
        .bbox
        .map(|bbox| [bbox[0], bbox[1], bbox[2], bbox[3]]))?;
    let x = object.transform.translate[0] + local_box[0];
    let y = object.transform.translate[1] + local_box[1];
    if object.transform.rotate.abs() > crate::EPSILON {
        let center =
            crate::Point::new(object.transform.translate[0], object.transform.translate[1]);
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for point in [
            crate::Point::new(x, y),
            crate::Point::new(x + local_box[2], y),
            crate::Point::new(x + local_box[2], y + local_box[3]),
            crate::Point::new(x, y + local_box[3]),
        ] {
            let point = rotate_text_bounds_point(point, center, object.transform.rotate);
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }
        return Some([min_x, min_y, max_x, max_y]);
    }
    Some([x, y, x + local_box[2], y + local_box[3]])
}

fn rotate_text_bounds_point(
    point: crate::Point,
    center: crate::Point,
    degrees: f64,
) -> crate::Point {
    let radians = degrees.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    crate::Point::new(
        center.x + dx * cos - dy * sin,
        center.y + dx * sin + dy * cos,
    )
}

pub(crate) fn endpoint_label_world_bounds(
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
        let target = match &session.target {
            TextEditTarget::TextObject { object_id, .. } => TextEditCommandTarget::TextObject {
                object_id: object_id.clone(),
            },
            TextEditTarget::EndpointLabel { node_id, .. } => TextEditCommandTarget::EndpointLabel {
                node_id: node_id.clone(),
            },
        };
        self.with_command(EditorCommand::ApplyTextEdit { target }, |engine| {
            engine.apply_text_edit_untracked(session)
        })
    }

    fn apply_text_edit_untracked(&mut self, session: TextEditSession) -> bool {
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
            .font_size_world_pt()
            .unwrap_or(WorldPt(DEFAULT_TEXT_FONT_SIZE))
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
            .font_size_world_pt()
            .unwrap_or(WorldPt(DEFAULT_TEXT_FONT_SIZE))
            .value();
        let fallback_fill = session.fill.as_deref().unwrap_or(DEFAULT_TEXT_FILL);
        let line_height = session
            .line_height_world_pt()
            .unwrap_or(WorldPt(DEFAULT_TEXT_LINE_HEIGHT))
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
        self.with_command(
            EditorCommand::ReplaceHoveredEndpointLabel {
                label: label.to_string(),
            },
            |engine| engine.replace_hovered_endpoint_label_untracked(label),
        )
    }

    fn replace_hovered_endpoint_label_untracked(&mut self, label: &str) -> bool {
        let Some(hovered_node_id) = self
            .state
            .overlay
            .hover_endpoint
            .as_ref()
            .map(|hit| hit.node_id.clone())
        else {
            return false;
        };

        self.replace_node_label_untracked(&hovered_node_id, label)
    }

    pub(super) fn replace_node_label_untracked(&mut self, node_id: &str, label: &str) -> bool {
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
        let connection_angles = adjacent_angles_for_fragment_node(entry.fragment, node_id);
        let node = &mut entry.fragment.nodes[node_index];

        if !apply_node_label_replacement(node, label, &connection_angles) {
            self.undo_stack.pop();
            return false;
        }
        mark_shortcut_implicit_hydrogen_label(node, label);

        let node_position = node.position;
        refresh_attached_node_label_geometry_for_node(
            entry.fragment,
            object_translate,
            node_id,
            self.options.bond_stroke_world_pt().value(),
        );
        entry.update_bounds();
        let hover_point = crate::Point::new(
            object_translate[0] + node_position[0],
            object_translate[1] + node_position[1],
        );
        self.drag = None;
        self.state.selection = crate::SelectionState::default();
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_arrow = None;
        self.state.overlay.hover_shape = None;
        self.state.overlay.preview = None;
        self.state.overlay.hover_text_box = None;
        self.state.overlay.hover_endpoint = Some(EndpointHit {
            node_id: node_id.to_string(),
            point: hover_point,
            distance: 0.0,
            label_anchor: None,
        });
        self.note_pending_select_target(PendingSelectTarget::MoleculeNode(node_id.to_string()));
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
        let connection_angles = adjacent_angles_for_fragment_node(entry.fragment, node_id);
        let anchor_point = endpoint_label_editor_anchor_world(
            node,
            entry.object.transform.translate,
            &connection_angles,
        )
        .unwrap_or_else(|| {
            attached_node_label_anchor_world(
                entry.fragment,
                node_id,
                entry.object.transform.translate,
                self.options.bond_stroke_world_pt().value(),
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
        let font_size_world_pt = WorldPt(font_size.unwrap_or(DEFAULT_TEXT_FONT_SIZE));
        let line_height = Some((font_size_world_pt.value() * 1.05).max(font_size_world_pt.value()));
        let default_chemical = label
            .map(|_| source_runs_are_chemical(&source_runs))
            .unwrap_or(true);
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
            default_chemical,
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
        let (preview_connection_angles, editing_session) = if connection_angles.is_empty() {
            (connection_angles.as_slice(), session.clone())
        } else {
            (
                &[][..],
                TextEditSession {
                    box_value: None,
                    anchor_offset: None,
                    ..session.clone()
                },
            )
        };
        let label = make_centered_node_label_from_runs(
            &text,
            local_anchor,
            source_runs.clone(),
            display_runs.clone(),
            fallback_font_family,
            fallback_font_size,
            fallback_fill,
            preview_connection_angles,
            &editing_session,
            false,
            false,
            None,
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
            .font_size_world_pt()
            .unwrap_or(WorldPt(DEFAULT_TEXT_FONT_SIZE))
            .value();
        let session_line_height = session
            .line_height_world_pt()
            .unwrap_or(WorldPt(DEFAULT_TEXT_BLOCK_LINE_HEIGHT))
            .value();
        let display_runs = display_runs_from_source_runs(
            &source_runs,
            session.font_family.as_deref().unwrap_or("Arial"),
            session_font_size,
            session.fill.as_deref().unwrap_or("#000000"),
        );
        let (estimated_width, estimated_height) =
            estimate_text_block_size(&display_runs, session_font_size, session_line_height);
        let width = round2(
            session
                .box_value
                .map(|bbox| bbox[2].max(0.0))
                .unwrap_or(0.0)
                .max(estimated_width),
        );
        let height = round2(
            session
                .box_value
                .map(|bbox| bbox[3].max(0.0))
                .unwrap_or(0.0)
                .max(estimated_height),
        );
        let (x, y, existing_object_id) = match &session.target {
            TextEditTarget::TextObject { object_id, x, y } => (*x, *y, object_id.clone()),
            _ => return false,
        };
        let target_object_id = existing_object_id.or_else(|| object_id.map(ToString::to_string));

        self.push_undo_snapshot();
        let mut changed_text_object_id = target_object_id.clone();
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
                changed_text_object_id = Some(next_id.clone());
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
        if let Some(object_id) = changed_text_object_id {
            self.note_pending_select_target(PendingSelectTarget::TextObject(object_id));
        }
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
            self.options.bond_stroke_world_pt().value(),
        );
        entry.update_bounds();
        let hover_point = crate::Point::new(
            object_translate[0] + node_position[0],
            object_translate[1] + node_position[1],
        );
        self.drag = None;
        self.state.selection = crate::SelectionState::default();
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_shape = None;
        self.state.overlay.preview = None;
        self.state.overlay.hover_endpoint = Some(EndpointHit {
            node_id: node_id.to_string(),
            point: hover_point,
            distance: 0.0,
            label_anchor: None,
        });
        self.note_pending_select_target(PendingSelectTarget::MoleculeNode(node_id.to_string()));
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
    if classify_node_label_replacement_for_connection_count(label, connection_angles.len())
        .is_none()
    {
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
    let previous_meta = node.meta.clone();
    let previous_implicit_hydrogen_label_meta = previous_label
        .as_ref()
        .and_then(implicit_hydrogen_label_meta)
        .cloned();
    let previous_source_text = previous_label
        .as_ref()
        .map(label_source_text)
        .unwrap_or_default();
    let trimmed = text.trim();
    let source_runs = normalize_source_runs(session, text);
    let is_chemical_label = source_runs_are_chemical(&source_runs);
    if trimmed.is_empty() || (is_chemical_label && trimmed == "C") {
        let changed = previous_element != "C"
            || previous_atomic_number != 6
            || previous_is_placeholder
            || previous_label.is_some()
            || label_recognition_meta_from_node(node).is_some();
        if !changed {
            return false;
        }
        node.element = "C".to_string();
        node.atomic_number = 6;
        node.num_hydrogens = 0;
        node.is_placeholder = false;
        node.label = None;
        set_node_label_recognition_meta(node, None);
        set_node_implicit_hydrogen_label_meta(node, None);
        return true;
    }

    let mut label_recognition_meta = None;
    let connection_count = connection_angles.len();
    if is_chemical_label {
        if let Some(replacement) =
            classify_node_label_replacement_for_connection_count(trimmed, connection_count)
        {
            match replacement {
                NodeLabelReplacement::Carbon => {}
                NodeLabelReplacement::Element {
                    element,
                    atomic_number,
                } => {
                    node.element = element.to_string();
                    node.atomic_number = atomic_number;
                    node.num_hydrogens = 0;
                    node.is_placeholder = false;
                }
                NodeLabelReplacement::Abbreviation => {
                    node.element = "C".to_string();
                    node.atomic_number = 6;
                    node.num_hydrogens = 0;
                    node.is_placeholder = true;
                    label_recognition_meta =
                        crate::recognized_abbreviation_meta_for_connection_count(
                            trimmed,
                            connection_count,
                        );
                }
            }
        } else {
            node.element = "C".to_string();
            node.atomic_number = 6;
            node.num_hydrogens = 0;
            node.is_placeholder = true;
            label_recognition_meta = Some(crate::invalid_abbreviation_meta(trimmed));
        }
    } else {
        node.element = "C".to_string();
        node.atomic_number = 6;
        node.num_hydrogens = 0;
        node.is_placeholder = true;
    }
    set_node_label_recognition_meta(node, label_recognition_meta.clone());

    let session_font_size = session
        .font_size_world_pt()
        .unwrap_or(WorldPt(DEFAULT_TEXT_FONT_SIZE))
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
    let mut next_label = make_centered_node_label_from_runs(
        text,
        anchor_position,
        source_runs.clone(),
        display_runs,
        session
            .font_family
            .as_deref()
            .unwrap_or(DEFAULT_TEXT_FONT_FAMILY),
        session_font_size,
        session.fill.as_deref().unwrap_or(DEFAULT_TEXT_FILL),
        connection_angles,
        session,
        false,
        false,
        None,
    );
    set_label_recognition_meta(&mut next_label, label_recognition_meta);
    let implicit_hydrogen_label_meta = previous_implicit_hydrogen_label_meta.map(|meta| {
        let user_edited =
            implicit_hydrogen_label_user_edited(&meta) || previous_source_text != text;
        implicit_hydrogen_label_meta_value(
            implicit_hydrogen_label_source(&meta).unwrap_or("shortcut"),
            user_edited,
        )
    });
    set_label_implicit_hydrogen_label_meta(&mut next_label, implicit_hydrogen_label_meta.clone());
    set_node_implicit_hydrogen_label_meta(node, implicit_hydrogen_label_meta);
    let changed = previous_element != node.element
        || previous_atomic_number != node.atomic_number
        || previous_is_placeholder != node.is_placeholder
        || previous_meta != node.meta
        || !same_node_label(previous_label.as_ref(), Some(&next_label));
    if !changed {
        return false;
    }
    node.label = Some(next_label);
    true
}
