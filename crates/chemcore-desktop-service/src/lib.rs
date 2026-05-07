use chemcore_engine::{
    ArrowCurve, ArrowEndpointStyle, ArrowHeadSize, ArrowNoGo, ArrowVariant, BondVariant,
    BracketKind, Engine, Point, PointerEvent, RenderBoundsScope, RenderPrimitive, RenderRole,
    ShapeKind, ShapeStyle, TextEditLayoutRequest, TextEditSession, Tool, ToolState, WorldCm,
    WorldPoint,
};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub type SessionId = u64;

const MAX_RECENT_FILES: usize = 10;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RenderBounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl From<[f64; 4]> for RenderBounds {
    fn from(bounds: [f64; 4]) -> Self {
        Self {
            min_x: bounds[0],
            min_y: bounds[1],
            max_x: bounds[2],
            max_y: bounds[3],
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopOpenedDocument {
    pub path: String,
    pub file_name: String,
    pub format: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopRecentFile {
    pub path: String,
    pub file_name: String,
    #[serde(default)]
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSavedDocument {
    pub path: String,
    pub file_name: String,
    pub format: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DesktopEngineSnapshotMode {
    State,
    Interaction,
    Selection,
    Document,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopEngineSnapshot {
    pub document_json: Option<String>,
    pub state_json: Option<String>,
    pub render_list_json: Option<String>,
    pub all_bounds_json: Option<String>,
    pub document_bounds_json: Option<String>,
    pub selection_bounds_json: Option<String>,
    pub document_colors_json: Option<String>,
    pub document_style_preset: Option<String>,
    pub can_undo: Option<bool>,
    pub can_redo: Option<bool>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecentFilesStore {
    files: Vec<DesktopRecentFile>,
}

#[derive(Default)]
pub struct DesktopDocumentService {
    next_session_id: SessionId,
    sessions: BTreeMap<SessionId, Engine>,
    recent_files: Vec<DesktopRecentFile>,
    recent_store_path: Option<PathBuf>,
}

impl DesktopDocumentService {
    pub fn new() -> Self {
        let recent_store_path = default_recent_store_path();
        let recent_files = recent_store_path
            .as_ref()
            .map(|path| load_recent_files(path))
            .unwrap_or_default();
        Self {
            next_session_id: 1,
            sessions: BTreeMap::new(),
            recent_files,
            recent_store_path,
        }
    }

    pub fn create_session(&mut self) -> SessionId {
        let session_id = self.next_session_id;
        self.next_session_id += 1;
        self.sessions.insert(session_id, Engine::new());
        session_id
    }

    pub fn free_session(&mut self, session_id: SessionId) -> bool {
        self.sessions.remove(&session_id).is_some()
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn load_document_json(&mut self, session_id: SessionId, json: &str) -> Result<(), String> {
        self.session_mut(session_id)?.load_document_json(json)
    }

    pub fn load_document_cdxml(
        &mut self,
        session_id: SessionId,
        cdxml: &str,
    ) -> Result<(), String> {
        self.session_mut(session_id)?.load_cdxml_document(cdxml)
    }

    pub fn document_json(&self, session_id: SessionId) -> Result<String, String> {
        self.session(session_id)?
            .document_json()
            .map_err(|error| error.to_string())
    }

    pub fn state_json(&self, session_id: SessionId) -> Result<String, String> {
        self.session(session_id)?
            .state_json()
            .map_err(|error| error.to_string())
    }

    pub fn render_list_json(&self, session_id: SessionId) -> Result<String, String> {
        serde_json::to_string(&self.session(session_id)?.render_list())
            .map_err(|error| error.to_string())
    }

    pub fn render_bounds_json(&self, session_id: SessionId, scope: &str) -> Result<String, String> {
        serde_json::to_string(&self.render_bounds(session_id, scope)?)
            .map_err(|error| error.to_string())
    }

    pub fn render_bounds(
        &self,
        session_id: SessionId,
        scope: &str,
    ) -> Result<Option<RenderBounds>, String> {
        Ok(self
            .session(session_id)?
            .render_bounds(parse_render_bounds_scope(scope))
            .map(RenderBounds::from))
    }

    pub fn snapshot_json(
        &self,
        session_id: SessionId,
        mode: DesktopEngineSnapshotMode,
    ) -> Result<String, String> {
        let session = self.session(session_id)?;
        let include_document = mode == DesktopEngineSnapshotMode::Document;
        let include_render = mode != DesktopEngineSnapshotMode::State;
        let include_all_bounds = mode == DesktopEngineSnapshotMode::Document;
        let include_document_bounds = mode == DesktopEngineSnapshotMode::Document;
        let include_selection_bounds = matches!(
            mode,
            DesktopEngineSnapshotMode::Interaction
                | DesktopEngineSnapshotMode::Selection
                | DesktopEngineSnapshotMode::Document
        );

        let primitives = if include_render {
            Some(session.render_list())
        } else {
            None
        };
        let render_list_json = primitives
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| error.to_string())?;

        let snapshot = DesktopEngineSnapshot {
            document_json: include_document
                .then(|| session.document_json())
                .transpose()
                .map_err(|error| error.to_string())?,
            state_json: Some(session.state_json().map_err(|error| error.to_string())?),
            render_list_json,
            all_bounds_json: bounds_json_for_snapshot(
                primitives.as_deref(),
                RenderBoundsScope::All,
                include_all_bounds,
            )?,
            document_bounds_json: bounds_json_for_snapshot(
                primitives.as_deref(),
                RenderBoundsScope::Document,
                include_document_bounds,
            )?,
            selection_bounds_json: bounds_json_for_snapshot(
                primitives.as_deref(),
                RenderBoundsScope::Selection,
                include_selection_bounds,
            )?,
            document_colors_json: include_document
                .then(|| serde_json::to_string(&session.document_colors()))
                .transpose()
                .map_err(|error| error.to_string())?,
            document_style_preset: Some(session.document_style_preset().to_string()),
            can_undo: Some(session.can_undo()),
            can_redo: Some(session.can_redo()),
        };
        serde_json::to_string(&snapshot).map_err(|error| error.to_string())
    }

    pub fn document_cdxml(&self, session_id: SessionId) -> Result<String, String> {
        Ok(self.session(session_id)?.document_cdxml())
    }

    pub fn document_svg(&self, session_id: SessionId) -> Result<String, String> {
        Ok(self.session(session_id)?.document_svg())
    }

    pub fn document_colors_json(&self, session_id: SessionId) -> Result<String, String> {
        serde_json::to_string(&self.session(session_id)?.document_colors())
            .map_err(|error| error.to_string())
    }

    pub fn set_tool(
        &mut self,
        session_id: SessionId,
        active_tool: &str,
        bond_variant: &str,
    ) -> Result<(), String> {
        let session = self.session_mut(session_id)?;
        let current = session.state().tool.clone();
        session.set_tool_state(ToolState {
            active_tool: parse_tool(active_tool),
            bond_variant: parse_bond_variant(bond_variant),
            arrow_variant: current.arrow_variant,
            arrow_head_size: current.arrow_head_size,
            arrow_curve: current.arrow_curve,
            arrow_head_style: current.arrow_head_style,
            arrow_tail_style: current.arrow_tail_style,
            arrow_head: current.arrow_head,
            arrow_tail: current.arrow_tail,
            arrow_bold: current.arrow_bold,
            arrow_no_go: current.arrow_no_go,
            shape_kind: current.shape_kind,
            shape_style: current.shape_style,
            shape_color: current.shape_color,
            bracket_kind: current.bracket_kind,
            symbol_kind: current.symbol_kind,
            template: current.template,
        });
        Ok(())
    }

    pub fn set_shape_options(
        &mut self,
        session_id: SessionId,
        kind: &str,
        style: &str,
        color: &str,
    ) -> Result<(), String> {
        let session = self.session_mut(session_id)?;
        let mut tool = session.state().tool.clone();
        tool.shape_kind = parse_shape_kind(kind);
        tool.shape_style = parse_shape_style(style);
        tool.shape_color = color.to_string();
        session.set_tool_state(tool);
        Ok(())
    }

    pub fn set_template(&mut self, session_id: SessionId, template: &str) -> Result<(), String> {
        let session = self.session_mut(session_id)?;
        let mut tool = session.state().tool.clone();
        tool.template = template.to_string();
        session.set_tool_state(tool);
        Ok(())
    }

    pub fn set_bracket_options(&mut self, session_id: SessionId, kind: &str) -> Result<(), String> {
        let session = self.session_mut(session_id)?;
        let mut tool = session.state().tool.clone();
        tool.bracket_kind = parse_bracket_kind(kind);
        session.set_tool_state(tool);
        Ok(())
    }

    pub fn set_symbol_options(&mut self, session_id: SessionId, kind: &str) -> Result<(), String> {
        let session = self.session_mut(session_id)?;
        let mut tool = session.state().tool.clone();
        tool.symbol_kind = parse_bracket_kind(kind);
        session.set_tool_state(tool);
        Ok(())
    }

    pub fn set_document_style_preset(
        &mut self,
        session_id: SessionId,
        preset: &str,
    ) -> Result<(), String> {
        self.session_mut(session_id)?
            .set_document_style_preset(preset);
        Ok(())
    }

    pub fn document_style_preset(&self, session_id: SessionId) -> Result<String, String> {
        Ok(self
            .session(session_id)?
            .document_style_preset()
            .to_string())
    }

    pub fn object_settings_dialog_json(&self, session_id: SessionId) -> Result<String, String> {
        Ok(self.session(session_id)?.object_settings_dialog_json())
    }

    pub fn apply_object_settings_dialog_json(
        &mut self,
        session_id: SessionId,
        settings_json: &str,
    ) -> Result<bool, String> {
        self.session_mut(session_id)?
            .apply_object_settings_dialog_json(settings_json)
    }

    pub fn set_arrow_options(
        &mut self,
        session_id: SessionId,
        variant: &str,
        head_size: &str,
        head: bool,
        tail: bool,
        bold: bool,
    ) -> Result<(), String> {
        let session = self.session_mut(session_id)?;
        let mut tool = session.state().tool.clone();
        tool.arrow_variant = parse_arrow_variant(variant);
        tool.arrow_head_size = parse_arrow_head_size(head_size);
        tool.arrow_curve = ArrowCurve::Arc270;
        tool.arrow_head_style = if head {
            ArrowEndpointStyle::Full
        } else {
            ArrowEndpointStyle::None
        };
        tool.arrow_tail_style = if tail {
            ArrowEndpointStyle::Full
        } else {
            ArrowEndpointStyle::None
        };
        tool.arrow_head = head;
        tool.arrow_tail = tail;
        tool.arrow_bold = bold;
        tool.arrow_no_go = ArrowNoGo::None;
        session.set_tool_state(tool);
        Ok(())
    }

    pub fn set_arrow_endpoint_options(
        &mut self,
        session_id: SessionId,
        variant: &str,
        head_size: &str,
        curve: &str,
        head_style: &str,
        tail_style: &str,
        no_go: &str,
        bold: bool,
    ) -> Result<(), String> {
        let session = self.session_mut(session_id)?;
        let mut tool = session.state().tool.clone();
        tool.arrow_variant = parse_arrow_variant(variant);
        tool.arrow_head_size = parse_arrow_head_size(head_size);
        tool.arrow_curve = parse_arrow_curve(curve);
        tool.arrow_head_style = parse_arrow_endpoint_style(head_style);
        tool.arrow_tail_style = parse_arrow_endpoint_style(tail_style);
        tool.arrow_head = tool.arrow_head_style != ArrowEndpointStyle::None;
        tool.arrow_tail = tool.arrow_tail_style != ArrowEndpointStyle::None;
        tool.arrow_no_go = parse_arrow_no_go(no_go);
        tool.arrow_bold = bold;
        session.set_tool_state(tool);
        Ok(())
    }

    pub fn apply_arrow_options_to_selection(
        &mut self,
        session_id: SessionId,
        variant: &str,
        head_size: &str,
        head: bool,
        tail: bool,
        bold: bool,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .apply_arrow_options_to_selection(
                parse_arrow_variant(variant),
                parse_arrow_head_size(head_size),
                ArrowCurve::Arc270,
                if head {
                    ArrowEndpointStyle::Full
                } else {
                    ArrowEndpointStyle::None
                },
                if tail {
                    ArrowEndpointStyle::Full
                } else {
                    ArrowEndpointStyle::None
                },
                head,
                tail,
                bold,
                ArrowNoGo::None,
            ))
    }

    pub fn apply_arrow_endpoint_options_to_selection(
        &mut self,
        session_id: SessionId,
        variant: &str,
        head_size: &str,
        curve: &str,
        head_style: &str,
        tail_style: &str,
        no_go: &str,
        bold: bool,
    ) -> Result<bool, String> {
        let head_style = parse_arrow_endpoint_style(head_style);
        let tail_style = parse_arrow_endpoint_style(tail_style);
        Ok(self
            .session_mut(session_id)?
            .apply_arrow_options_to_selection(
                parse_arrow_variant(variant),
                parse_arrow_head_size(head_size),
                parse_arrow_curve(curve),
                head_style,
                tail_style,
                head_style != ArrowEndpointStyle::None,
                tail_style != ArrowEndpointStyle::None,
                bold,
                parse_arrow_no_go(no_go),
            ))
    }

    pub fn pointer_move(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
        alt_key: bool,
    ) -> Result<(), String> {
        self.session_mut(session_id)?
            .pointer_move(pointer_event(x, y, None, alt_key));
        Ok(())
    }

    pub fn pointer_down(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
        alt_key: bool,
    ) -> Result<(), String> {
        self.session_mut(session_id)?
            .pointer_down(pointer_event(x, y, Some(0), alt_key));
        Ok(())
    }

    pub fn pointer_up(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
        alt_key: bool,
    ) -> Result<(), String> {
        self.session_mut(session_id)?
            .pointer_up(pointer_event(x, y, Some(0), alt_key));
        Ok(())
    }

    pub fn select_at_point(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
        additive: bool,
    ) -> Result<(), String> {
        self.session_mut(session_id)?
            .select_at_point(point(x, y), additive);
        Ok(())
    }

    pub fn select_component_at_point(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
        additive: bool,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .select_component_at_point(point(x, y), additive))
    }

    pub fn select_in_rect(
        &mut self,
        session_id: SessionId,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        additive: bool,
    ) -> Result<(), String> {
        self.session_mut(session_id)?
            .select_in_rect(point(x1, y1), point(x2, y2), additive);
        Ok(())
    }

    pub fn select_in_polygon_json(
        &mut self,
        session_id: SessionId,
        points_json: &str,
        additive: bool,
    ) -> Result<(), String> {
        let raw_points: Vec<[f64; 2]> =
            serde_json::from_str(points_json).map_err(|error| error.to_string())?;
        let points = raw_points
            .into_iter()
            .map(|candidate| point(candidate[0], candidate[1]))
            .collect();
        self.session_mut(session_id)?
            .select_in_polygon(points, additive);
        Ok(())
    }

    pub fn select_all(&mut self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.select_all())
    }

    pub fn clear_selection(&mut self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.clear_selection())
    }

    pub fn context_hit_test_json(
        &self,
        session_id: SessionId,
        x: f64,
        y: f64,
    ) -> Result<String, String> {
        Ok(self.session(session_id)?.context_hit_test_json(point(x, y)))
    }

    pub fn context_menu_json(
        &self,
        session_id: SessionId,
        hit_json: &str,
        has_paste: bool,
    ) -> Result<String, String> {
        Ok(self
            .session(session_id)?
            .context_menu_json(hit_json, has_paste))
    }

    pub fn selection_contains_point(
        &self,
        session_id: SessionId,
        x: f64,
        y: f64,
    ) -> Result<bool, String> {
        Ok(self
            .session(session_id)?
            .selection_contains_point(point(x, y)))
    }

    pub fn hover_arrow_action(
        &self,
        session_id: SessionId,
        x: f64,
        y: f64,
    ) -> Result<String, String> {
        Ok(self
            .session(session_id)?
            .hover_arrow_action_at_point(point(x, y))
            .to_string())
    }

    pub fn begin_hover_arrow_edit(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
    ) -> Result<String, String> {
        Ok(self
            .session_mut(session_id)?
            .begin_hover_arrow_edit(point(x, y))
            .to_string())
    }

    pub fn update_hover_arrow_edit(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
        alt_key: bool,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .update_hover_arrow_edit(point(x, y), alt_key))
    }

    pub fn finish_hover_arrow_edit(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
        alt_key: bool,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .finish_hover_arrow_edit(point(x, y), alt_key))
    }

    pub fn hover_shape_action(
        &self,
        session_id: SessionId,
        x: f64,
        y: f64,
    ) -> Result<String, String> {
        Ok(self
            .session(session_id)?
            .hover_shape_action_at_point(point(x, y))
            .to_string())
    }

    pub fn begin_hover_shape_edit(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
    ) -> Result<String, String> {
        Ok(self
            .session_mut(session_id)?
            .begin_hover_shape_edit(point(x, y))
            .to_string())
    }

    pub fn update_hover_shape_edit(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
        alt_key: bool,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .update_hover_shape_edit(point(x, y), alt_key))
    }

    pub fn finish_hover_shape_edit(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
        alt_key: bool,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .finish_hover_shape_edit(point(x, y), alt_key))
    }

    pub fn active_arrow_edit_degrees(&self, session_id: SessionId) -> Result<f64, String> {
        Ok(self.session(session_id)?.active_arrow_edit_degrees())
    }

    pub fn begin_selection_move(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
        additive: bool,
        alt_key: bool,
    ) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.begin_selection_move_at_point(
            point(x, y),
            additive,
            alt_key,
        ))
    }

    pub fn update_selection_move(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
        alt_key: bool,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .update_selection_move(point(x, y), alt_key))
    }

    pub fn finish_selection_move(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
        alt_key: bool,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .finish_selection_move(point(x, y), alt_key))
    }

    pub fn begin_selection_rotate(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .begin_selection_rotate(point(x, y)))
    }

    pub fn update_selection_rotate(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
        alt_key: bool,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .update_selection_rotate(point(x, y), alt_key))
    }

    pub fn finish_selection_rotate(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
        alt_key: bool,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .finish_selection_rotate(point(x, y), alt_key))
    }

    pub fn begin_selection_resize(
        &mut self,
        session_id: SessionId,
        handle: &str,
        x: f64,
        y: f64,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .begin_selection_resize(handle, point(x, y)))
    }

    pub fn update_selection_resize(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .update_selection_resize(point(x, y)))
    }

    pub fn finish_selection_resize(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .finish_selection_resize(point(x, y)))
    }

    pub fn apply_selection_arrange_command(
        &mut self,
        session_id: SessionId,
        command: &str,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .apply_selection_arrange_command(command))
    }

    pub fn scale_selection(&mut self, session_id: SessionId, percent: f64) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.scale_selection(percent))
    }

    pub fn rotate_selection_degrees(
        &mut self,
        session_id: SessionId,
        degrees: f64,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .rotate_selection_degrees(degrees))
    }

    pub fn selection_numeric_dialog_json(
        &self,
        session_id: SessionId,
        kind: &str,
    ) -> Result<String, String> {
        Ok(self
            .session(session_id)?
            .selection_numeric_dialog_json(kind))
    }

    pub fn apply_selection_numeric_dialog_json(
        &mut self,
        session_id: SessionId,
        payload_json: &str,
    ) -> Result<bool, String> {
        self.session_mut(session_id)?
            .apply_selection_numeric_dialog_json(payload_json)
    }

    pub fn apply_selection_order_command(
        &mut self,
        session_id: SessionId,
        command: &str,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .apply_selection_order_command(command))
    }

    pub fn group_selection(&mut self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.group_selection())
    }

    pub fn ungroup_selection(&mut self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.ungroup_selection())
    }

    pub fn apply_color_to_selection(
        &mut self,
        session_id: SessionId,
        color: &str,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .apply_color_to_selection(color))
    }

    pub fn apply_shape_style_to_selection(
        &mut self,
        session_id: SessionId,
        style: &str,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .apply_shape_style_to_selection(style))
    }

    pub fn apply_bracket_kind_to_selection(
        &mut self,
        session_id: SessionId,
        kind: &str,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .apply_bracket_kind_to_selection(kind))
    }

    pub fn apply_line_style_to_selection(
        &mut self,
        session_id: SessionId,
        style: &str,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .apply_line_style_to_selection(style))
    }

    pub fn apply_bond_style_to_selection(
        &mut self,
        session_id: SessionId,
        style: &str,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .apply_bond_style_to_selection(style))
    }

    pub fn apply_text_style_to_selection(
        &mut self,
        session_id: SessionId,
        command: &str,
        value: &str,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .apply_text_style_to_selection(command, value))
    }

    pub fn set_chemical_check_for_selection(
        &mut self,
        session_id: SessionId,
        enabled: bool,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .set_chemical_check_for_selection(enabled))
    }

    pub fn expand_labels_in_selection(&mut self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.expand_labels_in_selection())
    }

    pub fn center_selection_on_page(&mut self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.center_selection_on_page())
    }

    pub fn clear_interaction(&mut self, session_id: SessionId) -> Result<(), String> {
        self.session_mut(session_id)?.clear_interaction();
        Ok(())
    }

    pub fn undo(&mut self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.undo())
    }

    pub fn redo(&mut self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.redo())
    }

    pub fn can_undo(&self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session(session_id)?.can_undo())
    }

    pub fn can_redo(&self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session(session_id)?.can_redo())
    }

    pub fn delete_selection(&mut self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.delete_selection())
    }

    pub fn copy_selection(&mut self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.copy_selection())
    }

    pub fn has_clipboard(&self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session(session_id)?.has_clipboard())
    }

    pub fn clipboard_selection_json(
        &self,
        session_id: SessionId,
    ) -> Result<Option<String>, String> {
        self.session(session_id)?.clipboard_selection_json()
    }

    pub fn clipboard_document_json(&self, session_id: SessionId) -> Result<Option<String>, String> {
        self.session(session_id)?.clipboard_document_json()
    }

    pub fn cut_selection(&mut self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.cut_selection())
    }

    pub fn paste_clipboard(&mut self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.paste_clipboard())
    }

    pub fn paste_clipboard_json(
        &mut self,
        session_id: SessionId,
        json: &str,
    ) -> Result<bool, String> {
        self.session_mut(session_id)?.paste_clipboard_json(json)
    }

    pub fn replace_hovered_endpoint_label(
        &mut self,
        session_id: SessionId,
        label: &str,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .replace_hovered_endpoint_label(label))
    }

    pub fn begin_text_edit(
        &mut self,
        session_id: SessionId,
        x: f64,
        y: f64,
    ) -> Result<Option<String>, String> {
        self.session_mut(session_id)?
            .begin_text_edit(point(x, y))
            .map(|session| serde_json::to_string(&session).map_err(|error| error.to_string()))
            .transpose()
    }

    pub fn apply_text_edit(
        &mut self,
        session_id: SessionId,
        session_json: &str,
    ) -> Result<bool, String> {
        let session: TextEditSession =
            serde_json::from_str(session_json).map_err(|error| error.to_string())?;
        Ok(self.session_mut(session_id)?.apply_text_edit(session))
    }

    pub fn preview_text_runs(
        &self,
        session_id: SessionId,
        session_json: &str,
    ) -> Result<String, String> {
        let session: TextEditSession =
            serde_json::from_str(session_json).map_err(|error| error.to_string())?;
        let (source_runs, display_runs) = self.session(session_id)?.preview_text_runs(&session);
        serde_json::to_string(&serde_json::json!({
            "sourceRuns": source_runs,
            "displayRuns": display_runs,
        }))
        .map_err(|error| error.to_string())
    }

    pub fn preview_text_edit_layout(
        &self,
        session_id: SessionId,
        request_json: &str,
    ) -> Result<String, String> {
        let request: TextEditLayoutRequest =
            serde_json::from_str(request_json).map_err(|error| error.to_string())?;
        serde_json::to_string(&self.session(session_id)?.preview_text_edit_layout(&request))
            .map_err(|error| error.to_string())
    }

    pub fn read_document_file<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<DesktopOpenedDocument, String> {
        let path = normalize_path(path)?;
        let bytes = fs::read(&path)
            .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
        let format = document_format_for_path_and_bytes(&path, &bytes);
        let text = if format == "ccjz" {
            decompress_gzip_text(&bytes)?
        } else {
            String::from_utf8(bytes).map_err(|error| {
                format!("Failed to read {} as UTF-8 text: {error}", path.display())
            })?
        };
        let format = if format == "text" && looks_like_cdxml(&text) {
            "cdxml".to_string()
        } else if format == "text" {
            "ccjs".to_string()
        } else {
            format
        };
        let opened = DesktopOpenedDocument {
            file_name: file_name_for_path(&path),
            path: path_to_string(&path),
            format,
            text,
        };
        self.add_recent_file(path);
        Ok(opened)
    }

    pub fn write_document_file<P: AsRef<Path>>(
        &mut self,
        path: P,
        content: &str,
        format: Option<&str>,
    ) -> Result<DesktopSavedDocument, String> {
        let path = normalize_path(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("Failed to create directory {}: {error}", parent.display())
            })?;
        }
        let format = format
            .map(normalize_document_format)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| document_format_for_path(&path));
        if format == "ccjz" {
            let bytes = compress_gzip_text(content)?;
            fs::write(&path, bytes)
                .map_err(|error| format!("Failed to write {}: {error}", path.display()))?;
        } else {
            fs::write(&path, content)
                .map_err(|error| format!("Failed to write {}: {error}", path.display()))?;
        }
        self.add_recent_file(path.clone());
        Ok(DesktopSavedDocument {
            file_name: file_name_for_path(&path),
            path: path_to_string(&path),
            format,
        })
    }

    pub fn recent_files(&self) -> Vec<DesktopRecentFile> {
        self.recent_files
            .iter()
            .map(|entry| DesktopRecentFile {
                path: entry.path.clone(),
                file_name: entry.file_name.clone(),
                exists: Path::new(&entry.path).is_file(),
            })
            .collect()
    }

    pub fn clear_recent_files(&mut self) -> Result<(), String> {
        self.recent_files.clear();
        self.save_recent_files()
    }

    fn session(&self, session_id: SessionId) -> Result<&Engine, String> {
        self.sessions
            .get(&session_id)
            .ok_or_else(|| format!("Unknown desktop engine session: {session_id}"))
    }

    fn session_mut(&mut self, session_id: SessionId) -> Result<&mut Engine, String> {
        self.sessions
            .get_mut(&session_id)
            .ok_or_else(|| format!("Unknown desktop engine session: {session_id}"))
    }

    fn add_recent_file(&mut self, path: PathBuf) {
        let path_string = path_to_string(&path);
        self.recent_files
            .retain(|entry| !paths_equal(&entry.path, &path_string));
        self.recent_files.insert(
            0,
            DesktopRecentFile {
                file_name: file_name_for_path(&path),
                path: path_string,
                exists: path.is_file(),
            },
        );
        self.recent_files.truncate(MAX_RECENT_FILES);
        let _ = self.save_recent_files();
    }

    fn save_recent_files(&self) -> Result<(), String> {
        let Some(path) = &self.recent_store_path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "Failed to create recent-file directory {}: {error}",
                    parent.display()
                )
            })?;
        }
        let store = RecentFilesStore {
            files: self.recent_files(),
        };
        let json = serde_json::to_string_pretty(&store).map_err(|error| error.to_string())?;
        fs::write(path, format!("{json}\n"))
            .map_err(|error| format!("Failed to write {}: {error}", path.display()))
    }
}

fn parse_render_bounds_scope(scope: &str) -> RenderBoundsScope {
    match scope {
        "document" => RenderBoundsScope::Document,
        "selection" => RenderBoundsScope::Selection,
        _ => RenderBoundsScope::All,
    }
}

fn bounds_json_for_snapshot(
    primitives: Option<&[RenderPrimitive]>,
    scope: RenderBoundsScope,
    include: bool,
) -> Result<Option<String>, String> {
    if !include {
        return Ok(None);
    }
    let bounds = primitives.and_then(|items| {
        chemcore_engine::render_primitives_bounds(
            items
                .iter()
                .filter(|primitive| render_bounds_scope_accepts(scope, primitive)),
        )
        .map(RenderBounds::from)
    });
    serde_json::to_string(&bounds)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn render_bounds_scope_accepts(scope: RenderBoundsScope, primitive: &RenderPrimitive) -> bool {
    match scope {
        RenderBoundsScope::All => true,
        RenderBoundsScope::Document => {
            let role = render_primitive_role(primitive);
            role != RenderRole::DocumentKnockout
                && !render_role_is_selection(role)
                && !render_role_is_hover(role)
                && !render_role_is_preview(role)
        }
        RenderBoundsScope::Selection => render_role_is_selection(render_primitive_role(primitive)),
    }
}

fn render_primitive_role(primitive: &RenderPrimitive) -> RenderRole {
    match primitive {
        RenderPrimitive::Line { role, .. }
        | RenderPrimitive::Circle { role, .. }
        | RenderPrimitive::Polygon { role, .. }
        | RenderPrimitive::Rect { role, .. }
        | RenderPrimitive::Ellipse { role, .. }
        | RenderPrimitive::Polyline { role, .. }
        | RenderPrimitive::Path { role, .. }
        | RenderPrimitive::FilledPath { role, .. }
        | RenderPrimitive::Text { role, .. } => *role,
    }
}

fn render_role_is_selection(role: RenderRole) -> bool {
    matches!(
        role,
        RenderRole::SelectionBox
            | RenderRole::SelectionBond
            | RenderRole::SelectionBondDot
            | RenderRole::SelectionNode
            | RenderRole::SelectionTextBox
    )
}

fn render_role_is_hover(role: RenderRole) -> bool {
    matches!(
        role,
        RenderRole::HoverEndpoint
            | RenderRole::HoverLabelGlyph
            | RenderRole::HoverBondCenter
            | RenderRole::HoverArrowCenter
            | RenderRole::HoverArrowHandle
            | RenderRole::HoverShapeHandle
            | RenderRole::HoverTextBox
    )
}

fn render_role_is_preview(role: RenderRole) -> bool {
    matches!(role, RenderRole::PreviewBond | RenderRole::PreviewEnd)
}

fn point(x: f64, y: f64) -> Point {
    Point::from_world(WorldPoint::new(WorldCm(x), WorldCm(y)))
}

fn pointer_event(x: f64, y: f64, button: Option<u8>, alt_key: bool) -> PointerEvent {
    PointerEvent::from_world_point(WorldPoint::new(WorldCm(x), WorldCm(y)), button, alt_key)
}

fn parse_tool(value: &str) -> Tool {
    match value {
        "bond" => Tool::Bond,
        "arrow" => Tool::Arrow,
        "bracket" => Tool::Bracket,
        "symbol" => Tool::Symbol,
        "delete" => Tool::Delete,
        "text" => Tool::Text,
        "shape" => Tool::Shape,
        "templates" => Tool::Templates,
        _ => Tool::Select,
    }
}

fn parse_bracket_kind(value: &str) -> BracketKind {
    match value {
        "square" => BracketKind::Square,
        "curly" => BracketKind::Curly,
        "double-dagger" | "doubleDagger" => BracketKind::DoubleDagger,
        "dagger" => BracketKind::Dagger,
        "circle-plus" | "circlePlus" => BracketKind::CirclePlus,
        "plus" => BracketKind::Plus,
        "radical-cation" | "radicalCation" => BracketKind::RadicalCation,
        "lone-pair" | "lonePair" => BracketKind::LonePair,
        "circle-minus" | "circleMinus" => BracketKind::CircleMinus,
        "minus" => BracketKind::Minus,
        "radical-anion" | "radicalAnion" => BracketKind::RadicalAnion,
        "electron" => BracketKind::Electron,
        _ => BracketKind::Round,
    }
}

fn parse_arrow_variant(value: &str) -> ArrowVariant {
    match value {
        "curved" => ArrowVariant::Curved,
        "curved-mirror" => ArrowVariant::CurvedMirror,
        "hollow" => ArrowVariant::Hollow,
        "open" => ArrowVariant::Open,
        _ => ArrowVariant::Solid,
    }
}

fn parse_shape_kind(value: &str) -> ShapeKind {
    match value {
        "ellipse" => ShapeKind::Ellipse,
        "round-rect" | "roundRect" => ShapeKind::RoundRect,
        "rect" => ShapeKind::Rect,
        _ => ShapeKind::Circle,
    }
}

fn parse_shape_style(value: &str) -> ShapeStyle {
    match value {
        "dashed" => ShapeStyle::Dashed,
        "shaded" => ShapeStyle::Shaded,
        "filled" => ShapeStyle::Filled,
        "shadowed" | "shadow" => ShapeStyle::Shadowed,
        _ => ShapeStyle::Solid,
    }
}

fn parse_arrow_curve(value: &str) -> ArrowCurve {
    match value {
        "180" | "arc-180" | "arc180" => ArrowCurve::Arc180,
        "120" | "arc-120" | "arc120" => ArrowCurve::Arc120,
        "90" | "arc-90" | "arc90" => ArrowCurve::Arc90,
        _ => ArrowCurve::Arc270,
    }
}

fn parse_arrow_head_size(value: &str) -> ArrowHeadSize {
    match value {
        "large" => ArrowHeadSize::Large,
        "medium" => ArrowHeadSize::Medium,
        "small" => ArrowHeadSize::Small,
        _ => ArrowHeadSize::Small,
    }
}

fn parse_arrow_endpoint_style(value: &str) -> ArrowEndpointStyle {
    match value {
        "full" => ArrowEndpointStyle::Full,
        "left" | "top" | "half-left" => ArrowEndpointStyle::Left,
        "right" | "bottom" | "half-right" => ArrowEndpointStyle::Right,
        _ => ArrowEndpointStyle::None,
    }
}

fn parse_arrow_no_go(value: &str) -> ArrowNoGo {
    match value {
        "cross" => ArrowNoGo::Cross,
        "hash" => ArrowNoGo::Hash,
        _ => ArrowNoGo::None,
    }
}

fn parse_bond_variant(value: &str) -> BondVariant {
    match value {
        "double" => BondVariant::Double,
        "triple" => BondVariant::Triple,
        "dashed" => BondVariant::Dashed,
        "dashed-double" => BondVariant::DashedDouble,
        "bold" => BondVariant::Bold,
        "bold-dashed" => BondVariant::BoldDashed,
        "wedge" => BondVariant::Wedge,
        "hashed-wedge" => BondVariant::HashedWedge,
        _ => BondVariant::Single,
    }
}

fn default_recent_store_path() -> Option<PathBuf> {
    dirs::data_dir().map(|path| {
        path.join("Chemcore")
            .join("desktop")
            .join("recent-files.json")
    })
}

fn load_recent_files(path: &Path) -> Vec<DesktopRecentFile> {
    let Ok(json) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(store) = serde_json::from_str::<RecentFilesStore>(&json) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for entry in store.files {
        if entry.path.trim().is_empty()
            || files
                .iter()
                .any(|existing: &DesktopRecentFile| paths_equal(&existing.path, &entry.path))
        {
            continue;
        }
        let path = PathBuf::from(&entry.path);
        files.push(DesktopRecentFile {
            file_name: if entry.file_name.trim().is_empty() {
                file_name_for_path(&path)
            } else {
                entry.file_name
            },
            exists: path.is_file(),
            path: entry.path,
        });
        if files.len() >= MAX_RECENT_FILES {
            break;
        }
    }
    files
}

fn normalize_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, String> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        return Err("Path is empty.".to_string());
    }
    Ok(path.to_path_buf())
}

fn file_name_for_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")
        .to_string()
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn paths_equal(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn normalize_document_format(format: &str) -> String {
    match format
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase()
        .as_str()
    {
        "ccjz" => "ccjz",
        "ccjs" => "ccjs",
        "cdxml" => "cdxml",
        "svg" => "svg",
        _ => "",
    }
    .to_string()
}

fn document_format_for_path(path: &Path) -> String {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "ccjz" => "ccjz",
        "ccjs" => "ccjs",
        "cdxml" => "cdxml",
        "svg" => "svg",
        _ => "ccjz",
    }
    .to_string()
}

fn document_format_for_path_and_bytes(path: &Path, bytes: &[u8]) -> String {
    let format = document_format_for_path(path);
    if format != "ccjz" && bytes.starts_with(&[0x1f, 0x8b]) {
        return "ccjz".to_string();
    }
    format
}

fn looks_like_cdxml(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("<CDXML") || trimmed.starts_with("<?xml") && trimmed.contains("<CDXML")
}

fn decompress_gzip_text(bytes: &[u8]) -> Result<String, String> {
    let mut decoder = GzDecoder::new(bytes);
    let mut text = String::new();
    decoder
        .read_to_string(&mut text)
        .map_err(|error| format!("Failed to decompress .ccjz data: {error}"))?;
    Ok(text)
}

fn compress_gzip_text(text: &str) -> Result<Vec<u8>, String> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(text.as_bytes())
        .map_err(|error| format!("Failed to compress .ccjz data: {error}"))?;
    encoder
        .finish()
        .map_err(|error| format!("Failed to finish .ccjz compression: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn creates_and_frees_native_engine_sessions() {
        let mut service = DesktopDocumentService::new();
        let first = service.create_session();
        let second = service.create_session();

        assert_ne!(first, second);
        assert_eq!(service.session_count(), 2);
        assert!(service.free_session(first));
        assert!(!service.free_session(first));
        assert_eq!(service.session_count(), 1);
    }

    #[test]
    fn exposes_document_and_render_json_for_blank_session() {
        let mut service = DesktopDocumentService::new();
        let session_id = service.create_session();

        let document: Value =
            serde_json::from_str(&service.document_json(session_id).unwrap()).unwrap();
        let render_list: Value =
            serde_json::from_str(&service.render_list_json(session_id).unwrap()).unwrap();
        let bounds: Value =
            serde_json::from_str(&service.render_bounds_json(session_id, "all").unwrap()).unwrap();

        assert_eq!(document["document"]["title"], "Untitled");
        assert!(render_list.as_array().is_some());
        assert!(bounds.is_null() || bounds["minX"].is_number());
    }

    #[test]
    fn rejects_unknown_sessions() {
        let service = DesktopDocumentService::new();
        assert!(service.document_json(42).is_err());
    }

    #[test]
    fn native_session_accepts_editing_commands() {
        let mut service = DesktopDocumentService::new();
        let session_id = service.create_session();

        service.set_tool(session_id, "bond", "single").unwrap();
        service.pointer_down(session_id, 20.0, 20.0, false).unwrap();
        service.pointer_up(session_id, 20.0, 20.0, false).unwrap();

        let document: Value =
            serde_json::from_str(&service.document_json(session_id).unwrap()).unwrap();
        let render_list: Value =
            serde_json::from_str(&service.render_list_json(session_id).unwrap()).unwrap();

        assert!(document["objects"].as_array().unwrap().len() >= 1);
        assert!(render_list.as_array().unwrap().len() >= 1);
        assert!(service.can_undo(session_id).unwrap());
        assert!(service.undo(session_id).unwrap());
    }

    #[test]
    fn native_session_exposes_selection_and_color_commands() {
        let mut service = DesktopDocumentService::new();
        let session_id = service.create_session();

        service.set_tool(session_id, "bond", "single").unwrap();
        service.pointer_down(session_id, 20.0, 20.0, false).unwrap();
        service.pointer_up(session_id, 20.0, 20.0, false).unwrap();
        assert!(!service.has_clipboard(session_id).unwrap());
        assert!(service.select_all(session_id).unwrap());
        assert!(service.copy_selection(session_id).unwrap());
        assert!(service.has_clipboard(session_id).unwrap());
        service
            .select_in_rect(session_id, 0.0, 0.0, 120.0, 80.0, false)
            .unwrap();

        let selection_bounds: Value =
            serde_json::from_str(&service.render_bounds_json(session_id, "selection").unwrap())
                .unwrap();
        assert!(selection_bounds["minX"].is_number());
        assert!(service
            .apply_color_to_selection(session_id, "#336699")
            .unwrap());
        assert!(service
            .document_colors_json(session_id)
            .unwrap()
            .contains("#336699"));
    }

    #[test]
    fn native_session_exposes_group_and_order_commands() {
        let mut service = DesktopDocumentService::new();
        let session_id = service.create_session();
        service
            .load_document_json(
                session_id,
                &serde_json::json!({
                    "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
                    "document": {
                        "id": "doc_native_group",
                        "title": "native group",
                        "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
                    },
                    "styles": {
                        "style_shape": { "kind": "shape", "stroke": "#000000", "strokeWidth": 1.0 }
                    },
                    "objects": [
                        {
                            "id": "shape_a",
                            "type": "shape",
                            "zIndex": 10,
                            "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                            "styleRef": "style_shape",
                            "payload": { "bbox": [0.0, 0.0, 20.0, 10.0], "kind": "rect" }
                        },
                        {
                            "id": "shape_b",
                            "type": "shape",
                            "zIndex": 20,
                            "transform": { "translate": [50.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                            "styleRef": "style_shape",
                            "payload": { "bbox": [0.0, 0.0, 30.0, 10.0], "kind": "rect" }
                        }
                    ],
                    "resources": {}
                })
                .to_string(),
            )
            .unwrap();
        service
            .select_in_rect(session_id, 0.0, 0.0, 90.0, 60.0, false)
            .unwrap();
        assert!(service
            .apply_selection_order_command(session_id, "bring-front")
            .unwrap());
        assert!(service.group_selection(session_id).unwrap());
        let document: Value = serde_json::from_str(&service.document_json(session_id).unwrap())
            .expect("document json");
        let group = document["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|object| object["type"] == "group")
            .expect("group object");
        assert_eq!(group["children"].as_array().unwrap().len(), 2);
        assert!(service.ungroup_selection(session_id).unwrap());
    }

    #[test]
    fn native_session_supports_text_edit_preview_and_commit() {
        let mut service = DesktopDocumentService::new();
        let session_id = service.create_session();

        service.set_tool(session_id, "text", "single").unwrap();
        service.pointer_down(session_id, 30.0, 30.0, false).unwrap();
        service.pointer_up(session_id, 30.0, 30.0, false).unwrap();
        let session_json = service
            .begin_text_edit(session_id, 30.0, 30.0)
            .unwrap()
            .expect("text edit session");
        let mut session: Value = serde_json::from_str(&session_json).unwrap();
        session["text"] = Value::String("Native".to_string());
        session["sourceRuns"] = serde_json::json!([{
            "text": "Native",
            "fontFamily": "Arial",
            "fontSize": 10.0,
            "fill": "#000000"
        }]);
        let updated_session_json = serde_json::to_string(&session).unwrap();

        let preview = service
            .preview_text_edit_layout(
                session_id,
                &serde_json::json!({
                    "session": session,
                    "selection": null
                })
                .to_string(),
            )
            .unwrap();
        assert!(preview.contains("Native"));
        assert!(service
            .apply_text_edit(session_id, &updated_session_json)
            .unwrap());
        assert!(service
            .document_json(session_id)
            .unwrap()
            .contains("Native"));
    }

    #[test]
    fn native_session_supports_shape_hover_edit_commands() {
        let mut service = DesktopDocumentService::new();
        let session_id = service.create_session();

        service.set_tool(session_id, "shape", "single").unwrap();
        service
            .set_shape_options(session_id, "circle", "solid", "#000000")
            .unwrap();
        service.pointer_down(session_id, 40.0, 40.0, false).unwrap();
        service.pointer_move(session_id, 60.0, 40.0, false).unwrap();
        service.pointer_up(session_id, 60.0, 40.0, false).unwrap();

        assert_eq!(
            service.hover_shape_action(session_id, 60.0, 40.0).unwrap(),
            "circle-radius"
        );
        assert_eq!(
            service
                .begin_hover_shape_edit(session_id, 60.0, 40.0)
                .unwrap(),
            "circle-radius"
        );
        assert!(service
            .update_hover_shape_edit(session_id, 80.0, 40.0, false)
            .unwrap());
        assert!(service
            .finish_hover_shape_edit(session_id, 80.0, 40.0, false)
            .unwrap());

        let document: Value =
            serde_json::from_str(&service.document_json(session_id).unwrap()).unwrap();
        let shape = document["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|object| object["type"] == "shape")
            .expect("shape object should exist");
        assert_eq!(
            shape["payload"]["majorAxisEnd"],
            serde_json::json!([80.0, 40.0])
        );
    }

    #[test]
    fn detects_document_format_from_paths() {
        assert_eq!(document_format_for_path(Path::new("sample.ccjz")), "ccjz");
        assert_eq!(document_format_for_path(Path::new("sample.ccjs")), "ccjs");
        assert_eq!(document_format_for_path(Path::new("sample.cdxml")), "cdxml");
        assert_eq!(document_format_for_path(Path::new("sample.svg")), "svg");
        assert_eq!(document_format_for_path(Path::new("sample")), "ccjz");
    }

    #[test]
    fn gzip_round_trip_preserves_document_text() {
        let text = "{\"format\":{\"name\":\"chemcore\"}}\n";
        let compressed = compress_gzip_text(text).unwrap();
        assert!(compressed.starts_with(&[0x1f, 0x8b]));
        assert_eq!(decompress_gzip_text(&compressed).unwrap(), text);
    }
}
