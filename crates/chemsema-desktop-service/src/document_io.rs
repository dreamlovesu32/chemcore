use crate::*;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

static SAVE_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

impl DesktopDocumentService {
    pub fn read_recovery_journal<P: AsRef<Path>>(
        &self,
        document_path: P,
    ) -> Result<Option<String>, String> {
        let path = recovery_journal_path(&normalize_path(document_path)?);
        match fs::read_to_string(&path) {
            Ok(content) => Ok(Some(content)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!(
                "Failed to read recovery journal {}: {error}",
                path.display()
            )),
        }
    }

    pub fn write_recovery_journal<P: AsRef<Path>>(
        &self,
        document_path: P,
        content: &str,
    ) -> Result<(), String> {
        let path = recovery_journal_path(&normalize_path(document_path)?);
        write_document_bytes_atomically(&path, content.as_bytes())
    }

    pub fn delete_recovery_journal<P: AsRef<Path>>(&self, document_path: P) -> Result<(), String> {
        let path = recovery_journal_path(&normalize_path(document_path)?);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!(
                "Failed to delete recovery journal {}: {error}",
                path.display()
            )),
        }
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
            decode_ccjz(&bytes)?
        } else if format == "cdx" {
            cdx_to_cdxml(&bytes)?
        } else {
            decode_document_text(&bytes, &format, &path)?
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
        if format == "ccjz" {
            write_ccjz_document_atomically(&path, content)?;
            self.add_recent_file(path.clone());
            return Ok(DesktopSavedDocument {
                file_name: file_name_for_path(&path),
                path: path_to_string(&path),
                format,
            });
        }
        let bytes = if format == "cdx" {
            cdxml_to_cdx(content)?
        } else {
            content.as_bytes().to_vec()
        };
        write_document_bytes_atomically(&path, &bytes)?;
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

fn write_ccjz_document_atomically(path: &Path, content: &str) -> Result<(), String> {
    let parent = output_parent_path(path).unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document");
    let (temp_path, mut output) = (0..100)
        .find_map(|_| {
            let sequence = SAVE_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let candidate = parent.join(format!(
                ".{file_name}.chemsema-save-{}-{sequence}.tmp",
                std::process::id()
            ));
            match OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(file) => Some(Ok((candidate, file))),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => None,
                Err(error) => Some(Err(format!(
                    "Failed to create temporary save file {}: {error}",
                    candidate.display()
                ))),
            }
        })
        .ok_or_else(|| "Failed to allocate a unique temporary save file".to_string())??;
    let result = (|| {
        if path.is_file() {
            let previous_file = OpenOptions::new().read(true).open(path).map_err(|error| {
                format!("Failed to open previous CCJZ {}: {error}", path.display())
            })?;
            let mut previous = CcjzReader::open(previous_file, DecodeLimits::default())?;
            write_ccjz_reusing(
                &mut previous,
                &mut output,
                content,
                DEFAULT_SCENE_CHUNK_RECORDS,
                &[],
                &[],
            )?;
        } else {
            write_ccjz(&mut output, content, DEFAULT_SCENE_CHUNK_RECORDS, &[])?;
        }
        output.sync_all().map_err(|error| {
            format!(
                "Failed to flush temporary CCJZ {}: {error}",
                temp_path.display()
            )
        })?;
        drop(output);
        let verify_file = OpenOptions::new()
            .read(true)
            .open(&temp_path)
            .map_err(|error| {
                format!(
                    "Failed to verify saved CCJZ {}: {error}",
                    temp_path.display()
                )
            })?;
        CcjzReader::open(verify_file, DecodeLimits::default())?;
        replace_file_atomically(&temp_path, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn recovery_journal_path(document_path: &Path) -> PathBuf {
    let mut value = document_path.as_os_str().to_os_string();
    value.push(".journal");
    PathBuf::from(value)
}

fn decode_document_text(bytes: &[u8], format: &str, path: &Path) -> Result<String, String> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(text.to_string()),
        Err(utf8_error) if format == "cdxml" => {
            // Real-world ChemDraw XML sometimes declares UTF-8 while carrying
            // one or two legacy Windows-1252 punctuation bytes. Preserve a
            // strict UTF-8 path first, then use the narrow Windows-1252 compatibility branch.
            let (text, _, had_errors) = WINDOWS_1252.decode(bytes);
            if had_errors {
                Err(format!(
                    "Failed to read {} as UTF-8 or Windows-1252 CDXML text: {utf8_error}",
                    path.display()
                ))
            } else {
                Ok(text.into_owned())
            }
        }
        Err(error) => Err(format!(
            "Failed to read {} as UTF-8 text: {error}",
            path.display()
        )),
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

fn write_document_bytes_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = output_parent_path(path).unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document");
    let temp_path = (0..100)
        .find_map(|_| {
            let sequence = SAVE_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let candidate = parent.join(format!(
                ".{file_name}.chemsema-save-{}-{sequence}.tmp",
                std::process::id()
            ));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(file) => Some(Ok((candidate, file))),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => None,
                Err(error) => Some(Err(format!(
                    "Failed to create temporary save file {}: {error}",
                    candidate.display()
                ))),
            }
        })
        .ok_or_else(|| "Failed to allocate a unique temporary save file".to_string())??;
    let (temp_path, mut file) = temp_path;
    let result = (|| {
        file.write_all(bytes).map_err(|error| {
            format!(
                "Failed to write temporary save file {}: {error}",
                temp_path.display()
            )
        })?;
        file.sync_all().map_err(|error| {
            format!(
                "Failed to flush temporary save file {}: {error}",
                temp_path.display()
            )
        })?;
        drop(file);
        verify_written_file_exact(&temp_path, bytes.len() as u64)?;
        replace_file_atomically(&temp_path, path)?;
        verify_written_file_exact(path, bytes.len() as u64)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(not(windows))]
fn replace_file_atomically(temp_path: &Path, path: &Path) -> Result<(), String> {
    fs::rename(temp_path, path).map_err(|error| {
        format!(
            "Failed to atomically replace {} with verified temporary file: {error}",
            path.display()
        )
    })
}

#[cfg(windows)]
fn replace_file_atomically(temp_path: &Path, path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, ReplaceFileW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        REPLACEFILE_WRITE_THROUGH,
    };

    let wide = |value: &Path| {
        value
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>()
    };
    let temp = wide(temp_path);
    let target = wide(path);
    let ok = unsafe {
        if path.exists() {
            ReplaceFileW(
                target.as_ptr(),
                temp.as_ptr(),
                std::ptr::null(),
                REPLACEFILE_WRITE_THROUGH,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } else {
            MoveFileExW(
                temp.as_ptr(),
                target.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        }
    };
    if ok == 0 {
        return Err(format!(
            "Failed to atomically replace {} with verified temporary file: {}",
            path.display(),
            std::io::Error::last_os_error()
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

#[cfg(test)]
mod ccjz_save_tests {
    use super::*;
    use chemsema_container::{Attachment, CcjzReader, DecodeLimits};

    #[test]
    fn atomic_ccjz_resave_preserves_opaque_attachments() {
        let path = std::env::temp_dir().join(format!(
            "chemsema-desktop-cow-{}-{}.ccjz",
            std::process::id(),
            SAVE_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let payload = b"desktop-preserved-fid";
        let hash = "3a5d8308d0437d8713d353f630fb12f6b19bf5ed596825b0381057e9ba9ee565";
        let mut document: serde_json::Value =
            serde_json::from_str(&Engine::new().document_json().expect("document serializes"))
                .expect("document parses");
        document["resources"]["fid"] = serde_json::json!({
            "type": "nmr-fid",
            "encoding": "opaque",
            "data": {
                "storage": "ccjz-attachment",
                "mediaType": "application/vnd.chemsema.nmr-fid",
                "byteLength": payload.len(),
                "sha256": hash
            }
        });
        let result = (|| -> Result<(), String> {
            let mut initial = OpenOptions::new()
                .write(true)
                .read(true)
                .create_new(true)
                .open(&path)
                .map_err(|error| error.to_string())?;
            chemsema_container::write_ccjz(
                &mut initial,
                &document.to_string(),
                DEFAULT_SCENE_CHUNK_RECORDS,
                &[Attachment {
                    id: "fid",
                    media_type: "application/vnd.chemsema.nmr-fid",
                    extension: "fid",
                    bytes: payload,
                }],
            )?;
            drop(initial);

            document["document"]["title"] = serde_json::json!("edited title");
            write_ccjz_document_atomically(&path, &document.to_string())?;
            let file = OpenOptions::new()
                .read(true)
                .open(&path)
                .map_err(|error| error.to_string())?;
            let mut reader = CcjzReader::open(file, DecodeLimits::default())?;
            assert_eq!(reader.read_attachment("fid")?, payload);
            let reopened: serde_json::Value = serde_json::from_str(&decode_ccjz(
                &fs::read(&path).map_err(|error| error.to_string())?,
            )?)
            .map_err(|error| error.to_string())?;
            assert_eq!(reopened, document);
            Ok(())
        })();
        let _ = fs::remove_file(&path);
        result.expect("desktop copy-on-write save succeeds");
    }
}
