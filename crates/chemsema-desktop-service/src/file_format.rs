use crate::*;

pub(crate) fn normalize_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, String> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        return Err("Path is empty.".to_string());
    }
    Ok(path.to_path_buf())
}

pub(crate) fn file_name_for_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")
        .to_string()
}

pub(crate) fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub(crate) fn paths_equal(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

pub(crate) fn normalize_document_format(format: &str) -> String {
    match format
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase()
        .as_str()
    {
        "ccjz" => "ccjz",
        "ccjs" | "json" => "ccjs",
        "cdxml" => "cdxml",
        "cdx" => "cdx",
        "sdf" | "sd" => "sdf",
        "svg" => "svg",
        _ => "",
    }
    .to_string()
}

pub(crate) fn document_format_for_path(path: &Path) -> String {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "ccjz" => "ccjz",
        "ccjs" | "json" => "ccjs",
        "cdxml" => "cdxml",
        "cdx" => "cdx",
        "sdf" | "sd" => "sdf",
        "svg" => "svg",
        _ => "ccjz",
    }
    .to_string()
}

pub(crate) fn is_ole_edit_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            lower.starts_with("chemsema-ole-edit-") && lower.ends_with(".ccjs")
        })
        .unwrap_or(false)
}

pub(crate) fn ole_edit_document_text(text: &str) -> Option<String> {
    let payload: OleEditDocumentPayload = serde_json::from_str(text).ok()?;
    payload
        .chemsema_document_json
        .filter(|value| !value.trim().is_empty())
}

pub(crate) fn document_format_for_path_and_bytes(path: &Path, bytes: &[u8]) -> String {
    let format = document_format_for_path(path);
    if format != "ccjz" && (bytes.starts_with(&[0x1f, 0x8b]) || bytes.starts_with(b"PK\x03\x04")) {
        return "ccjz".to_string();
    }
    format
}

pub(crate) fn looks_like_cdxml(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("<CDXML") || trimmed.starts_with("<?xml") && trimmed.contains("<CDXML")
}
