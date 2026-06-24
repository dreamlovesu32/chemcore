use chemcore_engine::{
    cdx_to_cdxml, cdxml_to_cdx, ArrowCurve, ArrowEndpointStyle, ArrowHeadSize, ArrowNoGo,
    ArrowVariant, BondVariant, BracketKind, Engine, OrbitalPhase, OrbitalStyle, OrbitalTemplate,
    Point, PointerEvent, RenderBoundsScope, RenderPrimitive, RenderRole, ShapeKind, ShapeStyle,
    TextEditLayoutRequest, TextEditSession, Tool, ToolState, WorldPoint, WorldPt,
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

mod file_format;
mod recent_files;
mod render_bounds;
mod tool_parsing;

use file_format::*;
use recent_files::*;
use render_bounds::*;
use tool_parsing::*;
mod document_io;

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
    DocumentState,
    Document,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopEngineSnapshot {
    pub document_json: Option<String>,
    pub state_json: Option<String>,
    pub render_list_json: Option<String>,
    pub interaction_render_list_json: Option<String>,
    pub all_bounds_json: Option<String>,
    pub document_bounds_json: Option<String>,
    pub selection_bounds_json: Option<String>,
    pub selection_chemistry_summary_json: Option<String>,
    pub document_colors_json: Option<String>,
    pub document_style_preset: Option<String>,
    pub revision: Option<u64>,
    pub last_command_result_json: Option<String>,
    pub can_undo: Option<bool>,
    pub can_redo: Option<bool>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecentFilesStore {
    files: Vec<DesktopRecentFile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OleEditDocumentPayload {
    chemcore_document_json: Option<String>,
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

    pub fn load_document_cdx(&mut self, session_id: SessionId, cdx: &[u8]) -> Result<(), String> {
        self.session_mut(session_id)?.load_cdx_document(cdx)
    }

    pub fn load_document_sdf(&mut self, session_id: SessionId, sdf: &str) -> Result<(), String> {
        self.session_mut(session_id)?.load_sdf_document(sdf)
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

    pub fn interaction_render_list_json(&self, session_id: SessionId) -> Result<String, String> {
        serde_json::to_string(&self.session(session_id)?.interaction_render_list())
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
        let include_document = matches!(
            mode,
            DesktopEngineSnapshotMode::Document | DesktopEngineSnapshotMode::DocumentState
        );
        let include_render = mode == DesktopEngineSnapshotMode::Document;
        let include_interaction_render = matches!(
            mode,
            DesktopEngineSnapshotMode::Interaction
                | DesktopEngineSnapshotMode::Selection
                | DesktopEngineSnapshotMode::Document
        );
        let include_all_bounds = mode == DesktopEngineSnapshotMode::Document;
        let include_document_bounds = mode == DesktopEngineSnapshotMode::Document;
        let include_selection_bounds = matches!(
            mode,
            DesktopEngineSnapshotMode::Interaction
                | DesktopEngineSnapshotMode::Selection
                | DesktopEngineSnapshotMode::Document
        );
        let include_selection_summary = matches!(
            mode,
            DesktopEngineSnapshotMode::Selection | DesktopEngineSnapshotMode::Document
        );

        let primitives = if include_render {
            Some(session.render_list())
        } else {
            None
        };
        let interaction_primitives = if include_interaction_render {
            Some(session.interaction_render_list())
        } else {
            None
        };
        let render_list_json = primitives
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| error.to_string())?;
        let interaction_render_list_json = interaction_primitives
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
            interaction_render_list_json,
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
            selection_bounds_json: selection_bounds_json_for_snapshot(
                session,
                include_selection_bounds,
            )?,
            selection_chemistry_summary_json: include_selection_summary
                .then(|| session.selection_chemistry_summary_json()),
            document_colors_json: include_document
                .then(|| serde_json::to_string(&session.document_colors()))
                .transpose()
                .map_err(|error| error.to_string())?,
            document_style_preset: Some(session.document_style_preset().to_string()),
            revision: Some(session.revision()),
            last_command_result_json: Some(
                session
                    .last_command_result_json()
                    .map_err(|error| error.to_string())?,
            ),
            can_undo: Some(session.can_undo()),
            can_redo: Some(session.can_redo()),
        };
        serde_json::to_string(&snapshot).map_err(|error| error.to_string())
    }

    pub fn document_cdxml(&self, session_id: SessionId) -> Result<String, String> {
        Ok(self.session(session_id)?.document_cdxml())
    }

    pub fn document_cdx(&self, session_id: SessionId) -> Result<Vec<u8>, String> {
        self.session(session_id)?.document_cdx()
    }

    pub fn document_sdf(&self, session_id: SessionId) -> Result<String, String> {
        self.session(session_id)?.document_sdf()
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
            orbital_template: current.orbital_template,
            orbital_style: current.orbital_style,
            orbital_phase: current.orbital_phase,
            orbital_color: current.orbital_color,
            bracket_kind: current.bracket_kind,
            symbol_kind: current.symbol_kind,
            element_symbol: current.element_symbol,
            element_atomic_number: current.element_atomic_number,
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

    pub fn set_orbital_options(
        &mut self,
        session_id: SessionId,
        template: &str,
        style: &str,
        phase: &str,
        color: &str,
    ) -> Result<(), String> {
        let session = self.session_mut(session_id)?;
        let mut tool = session.state().tool.clone();
        tool.orbital_template = parse_orbital_template(template);
        tool.orbital_style = parse_orbital_style(style);
        tool.orbital_phase = parse_orbital_phase(phase);
        tool.orbital_color = color.to_string();
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

    pub fn set_element_options(
        &mut self,
        session_id: SessionId,
        symbol: &str,
        atomic_number: u8,
    ) -> Result<(), String> {
        let session = self.session_mut(session_id)?;
        let mut tool = session.state().tool.clone();
        tool.element_symbol = symbol.to_string();
        tool.element_atomic_number = atomic_number;
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

    pub fn toolbar_color_palette_json(
        &self,
        session_id: SessionId,
        custom_colors_json: &str,
    ) -> Result<String, String> {
        Ok(self
            .session(session_id)?
            .toolbar_color_palette_json(custom_colors_json))
    }

    pub fn color_dialog_palette_json(
        &self,
        session_id: SessionId,
        current_color: &str,
        custom_colors_json: &str,
    ) -> Result<String, String> {
        Ok(self
            .session(session_id)?
            .color_dialog_palette_json(current_color, custom_colors_json))
    }

    pub fn text_symbol_palette_json(&self, session_id: SessionId) -> Result<String, String> {
        Ok(self.session(session_id)?.text_symbol_palette_json())
    }

    pub fn element_palette_json(&self, session_id: SessionId) -> Result<String, String> {
        Ok(self.session(session_id)?.element_palette_json())
    }

    pub fn apply_element_palette_json(
        &mut self,
        session_id: SessionId,
        selection_json: &str,
    ) -> Result<bool, String> {
        self.session_mut(session_id)?
            .apply_element_palette_json(selection_json)
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

    pub fn link_selection(&mut self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.link_selection())
    }

    pub fn unlink_selection(&mut self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.unlink_selection())
    }

    pub fn join_selection(&mut self, session_id: SessionId) -> Result<bool, String> {
        Ok(self.session_mut(session_id)?.join_selection())
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

    pub fn apply_orbital_template_to_selection(
        &mut self,
        session_id: SessionId,
        template: &str,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .apply_orbital_template_to_selection(template))
    }

    pub fn apply_orbital_style_to_selection(
        &mut self,
        session_id: SessionId,
        style: &str,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .apply_orbital_style_to_selection(style))
    }

    pub fn apply_orbital_phase_to_selection(
        &mut self,
        session_id: SessionId,
        phase: &str,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .apply_orbital_phase_to_selection(phase))
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

    pub fn apply_hovered_bond_style(
        &mut self,
        session_id: SessionId,
        style: &str,
    ) -> Result<bool, String> {
        Ok(self
            .session_mut(session_id)?
            .apply_hovered_bond_style(style))
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

    pub fn execute_command_json(
        &mut self,
        session_id: SessionId,
        command_json: &str,
    ) -> Result<String, String> {
        self.session_mut(session_id)?
            .execute_command_json(command_json)
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

    pub fn apply_bracket_label_text(
        &mut self,
        session_id: SessionId,
        bracket_id: &str,
        session_json: &str,
    ) -> Result<bool, String> {
        let session: TextEditSession =
            serde_json::from_str(session_json).map_err(|error| error.to_string())?;
        Ok(self
            .session_mut(session_id)?
            .apply_bracket_label_text(bracket_id, session))
    }

    pub fn pending_graphic_object_id(&self, session_id: SessionId) -> Result<String, String> {
        Ok(self
            .session(session_id)?
            .pending_graphic_object_id()
            .unwrap_or_default()
            .to_string())
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
        assert_eq!(document_format_for_path(Path::new("sample.cdx")), "cdx");
        assert_eq!(document_format_for_path(Path::new("sample.sdf")), "sdf");
        assert_eq!(document_format_for_path(Path::new("sample.sd")), "sdf");
        assert_eq!(document_format_for_path(Path::new("sample.svg")), "svg");
        assert_eq!(document_format_for_path(Path::new("sample")), "ccjz");
    }

    #[test]
    fn native_session_reads_and_writes_sdf_documents() {
        let mut service = DesktopDocumentService::new();
        let session_id = service.create_session();
        let sdf = concat!(
            "Ethanol\n",
            "  ChemCore\n",
            "\n",
            "  3  2  0  0  0  0            999 V2000\n",
            "    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n",
            "    1.5000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n",
            "    3.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n",
            "  1  2  1  0  0  0  0\n",
            "  2  3  1  0  0  0  0\n",
            "M  END\n",
            "$$$$\n",
        );
        service.load_document_sdf(session_id, sdf).unwrap();
        let document: Value =
            serde_json::from_str(&service.document_json(session_id).unwrap()).unwrap();
        assert_eq!(document["objects"][0]["type"], "molecule");

        let exported = service.document_sdf(session_id).unwrap();
        assert!(exported.contains("M  END"));
        assert!(exported.ends_with("$$$$\n"));
    }

    #[test]
    fn cdx_file_round_trip_uses_binary_storage_and_cdxml_text_payload() {
        let mut service = DesktopDocumentService::new();
        let path = std::env::temp_dir().join(format!(
            "chemcore-cdx-round-trip-{}-{}.cdx",
            std::process::id(),
            1
        ));
        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="ChemCore" BoundingBox="0 0 100 80" LabelFont="3" LabelSize="10" CaptionFont="3" CaptionSize="10" LineWidth="1" BoldWidth="4" BondLength="18">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 100 80">
    <fragment id="2" BoundingBox="10 10 50 20">
      <n id="3" p="10 10" Element="6"/>
      <n id="4" p="50 20" Element="8"/>
      <b id="5" B="3" E="4" Order="1"/>
    </fragment>
  </page>
</CDXML>
"#;
        service
            .write_document_file(&path, cdxml, Some("cdx"))
            .expect("cdx should write");
        let bytes = fs::read(&path).expect("cdx file should exist");
        assert!(bytes.starts_with(b"VjCD0100"));
        let opened = service.read_document_file(&path).expect("cdx should read");
        let _ = fs::remove_file(&path);

        assert_eq!(opened.format, "cdx");
        assert!(opened.text.contains("<CDXML"));
        assert!(opened.text.contains("<fragment"));
    }

    #[test]
    fn detects_ole_edit_transient_paths() {
        assert!(is_ole_edit_path(Path::new(
            "chemcore-ole-edit-123-456.ccjs"
        )));
        assert!(is_ole_edit_path(Path::new(
            r"C:\Temp\chemcore-ole-edit-123-456.ccjs"
        )));
        assert!(!is_ole_edit_path(Path::new(
            "chemcore-ole-edit-123-456.ccjz"
        )));
        assert!(!is_ole_edit_path(Path::new("regular-document.ccjs")));
    }

    #[test]
    fn reads_ole_edit_payload_as_document_text() {
        let mut service = DesktopDocumentService::new();
        let path = std::env::temp_dir().join(format!(
            "chemcore-ole-edit-test-{}-{}.ccjs",
            std::process::id(),
            1
        ));
        let document_json = r#"{"document":{"title":"OLE payload"},"objects":[],"resources":{}}"#;
        let payload = serde_json::json!({
            "chemcoreDocumentJson": document_json,
            "renderListJson": "[]",
            "cdxml": "<CDXML></CDXML>"
        });
        fs::write(&path, serde_json::to_string(&payload).unwrap()).unwrap();
        let opened = service.read_document_file(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(opened.format, "ccjs");
        assert_eq!(opened.text, document_json);
    }

    #[test]
    fn gzip_round_trip_preserves_document_text() {
        let text = "{\"format\":{\"name\":\"chemcore\"}}\n";
        let compressed = compress_gzip_text(text).unwrap();
        assert!(compressed.starts_with(&[0x1f, 0x8b]));
        assert_eq!(decompress_gzip_text(&compressed).unwrap(), text);
    }
}
