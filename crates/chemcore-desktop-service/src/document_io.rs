use crate::*;

impl DesktopDocumentService {
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
        } else if format == "cdx" {
            cdx_to_cdxml(&bytes)?
        } else {
            String::from_utf8(bytes).map_err(|error| {
                format!("Failed to read {} as UTF-8 text: {error}", path.display())
            })?
        };
        let text = if is_ole_edit_path(&path) {
            ole_edit_document_text(&text).unwrap_or(text)
        } else {
            text
        };
        // Normalize by content after decoding so dragged CDXML files without a
        // trusted extension still open through the chemical import path.
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
        if !is_ole_edit_path(&path) {
            self.add_recent_file(path);
        }
        Ok(opened)
    }

    pub fn write_document_file<P: AsRef<Path>>(
        &mut self,
        path: P,
        content: &str,
        format: Option<&str>,
    ) -> Result<DesktopSavedDocument, String> {
        let path = normalize_path(path)?;
        if let Some(parent) = output_parent_path(&path) {
            fs::create_dir_all(parent).map_err(|error| {
                format!("Failed to create directory {}: {error}", parent.display())
            })?;
            if !parent.is_dir() {
                return Err(format!(
                    "Failed to verify output directory {} after creating it.",
                    parent.display()
                ));
            }
        }
        let format = format
            .map(normalize_document_format)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| document_format_for_path(&path));
        let expected_bytes;
        if format == "ccjz" {
            let bytes = compress_gzip_text(content)?;
            expected_bytes = bytes.len() as u64;
            fs::write(&path, bytes.as_slice())
                .map_err(|error| format!("Failed to write {}: {error}", path.display()))?;
        } else if format == "cdx" {
            let bytes = cdxml_to_cdx(content)?;
            expected_bytes = bytes.len() as u64;
            fs::write(&path, bytes.as_slice())
                .map_err(|error| format!("Failed to write {}: {error}", path.display()))?;
        } else {
            expected_bytes = content.len() as u64;
            fs::write(&path, content.as_bytes())
                .map_err(|error| format!("Failed to write {}: {error}", path.display()))?;
        }
        verify_written_file_exact(&path, expected_bytes)?;
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

fn verify_written_file_exact(path: &Path, expected_bytes: u64) -> Result<(), String> {
    let metadata = fs::metadata(path).map_err(|error| {
        format!(
            "Failed to verify saved document {} after writing: {error}",
            path.display()
        )
    })?;
    if !metadata.is_file() {
        return Err(format!(
            "Failed to verify saved document {} after writing: path is not a regular file.",
            path.display()
        ));
    }
    let bytes = metadata.len();
    if bytes != expected_bytes {
        return Err(format!(
            "Failed to verify saved document {} after writing: file has {bytes} bytes, expected {expected_bytes}.",
            path.display()
        ));
    }
    Ok(())
}

fn output_parent_path(path: &Path) -> Option<&Path> {
    let parent = path.parent()?;
    if parent.as_os_str().is_empty() || parent.components().next().is_none() {
        None
    } else {
        Some(parent)
    }
}
