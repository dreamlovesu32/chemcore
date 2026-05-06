use chemcore_engine::{Engine, RenderBoundsScope};
use serde::Serialize;
use std::collections::BTreeMap;

pub type SessionId = u64;

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

#[derive(Default)]
pub struct DesktopDocumentService {
    next_session_id: SessionId,
    sessions: BTreeMap<SessionId, Engine>,
}

impl DesktopDocumentService {
    pub fn new() -> Self {
        Self {
            next_session_id: 1,
            sessions: BTreeMap::new(),
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

    pub fn document_cdxml(&self, session_id: SessionId) -> Result<String, String> {
        Ok(self.session(session_id)?.document_cdxml())
    }

    pub fn document_svg(&self, session_id: SessionId) -> Result<String, String> {
        Ok(self.session(session_id)?.document_svg())
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

fn parse_render_bounds_scope(scope: &str) -> RenderBoundsScope {
    match scope {
        "document" => RenderBoundsScope::Document,
        "selection" => RenderBoundsScope::Selection,
        _ => RenderBoundsScope::All,
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
}
