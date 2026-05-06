use chemcore_engine::{Engine, RenderBoundsScope};
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
