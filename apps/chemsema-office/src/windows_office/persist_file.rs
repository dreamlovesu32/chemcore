use super::*;

pub(super) unsafe extern "system" fn persist_file_get_class_id(
    _this: *mut c_void,
    class_id: *mut GUID,
) -> i32 {
    if class_id.is_null() {
        return E_POINTER;
    }
    *class_id = CLSID_CHEMSEMA_DOCUMENT;
    S_OK
}

pub(super) unsafe extern "system" fn persist_file_is_dirty(this: *mut c_void) -> i32 {
    let object = owner_from_part::<PersistFileVtbl>(this);
    if !object.is_null() && (*object).dirty {
        S_OK
    } else {
        S_FALSE
    }
}

pub(super) unsafe extern "system" fn persist_file_load(
    this: *mut c_void,
    file_name: *const u16,
    _mode: u32,
) -> i32 {
    let object = owner_from_part::<PersistFileVtbl>(this);
    if object.is_null() || file_name.is_null() {
        return E_POINTER;
    }
    let path = wide_path(file_name);
    match ole_object_payload_from_document_path(&path) {
        Ok(payload) => {
            (*object).payload = payload;
            (*object).extent_himetric = (*object).payload.extent_himetric();
            (*object).dirty = false;
            (*object).current_file = Some(path.clone());
            log_ole_event(&format!(
                "IPersistFile::Load({}) -> 0x00000000",
                path.display()
            ));
            S_OK
        }
        Err(error) => {
            log_ole_event(&format!(
                "IPersistFile::Load({}) failed: {error}",
                path.display()
            ));
            E_FAIL
        }
    }
}

pub(super) unsafe extern "system" fn persist_file_save(
    _this: *mut c_void,
    _file_name: *const u16,
    _remember: i32,
) -> i32 {
    E_NOTIMPL
}

pub(super) unsafe extern "system" fn persist_file_save_completed(
    this: *mut c_void,
    file_name: *const u16,
) -> i32 {
    let object = owner_from_part::<PersistFileVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    if !file_name.is_null() {
        (*object).current_file = Some(wide_path(file_name));
    }
    S_OK
}

pub(super) unsafe extern "system" fn persist_file_get_cur_file(
    this: *mut c_void,
    file_name: *mut *mut u16,
) -> i32 {
    if file_name.is_null() {
        return E_POINTER;
    }
    *file_name = null_mut();
    let object = owner_from_part::<PersistFileVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    let Some(path) = (*object).current_file.as_ref() else {
        return S_FALSE;
    };
    let wide = wide_path_null(path);
    let bytes = wide.len() * std::mem::size_of::<u16>();
    let output = CoTaskMemAlloc(bytes).cast::<u16>();
    if output.is_null() {
        return E_OUTOFMEMORY;
    }
    std::ptr::copy_nonoverlapping(wide.as_ptr(), output, wide.len());
    *file_name = output;
    S_OK
}

unsafe fn wide_path(value: *const u16) -> PathBuf {
    let mut len = 0;
    while *value.add(len) != 0 {
        len += 1;
    }
    PathBuf::from(OsString::from_wide(std::slice::from_raw_parts(value, len)))
}

pub(super) fn ole_object_payload_from_document_path(
    path: &Path,
) -> Result<OleObjectPayload, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    ole_object_payload_from_document_bytes(path, &bytes)
}

pub(super) fn ole_object_payload_from_document_bytes(
    path: &Path,
    bytes: &[u8],
) -> Result<OleObjectPayload, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut engine = chemsema_engine::Engine::new();
    let source_text = || {
        String::from_utf8(bytes.to_vec())
            .map_err(|error| format!("{} is not UTF-8: {error}", path.display()))
    };
    match extension.as_str() {
        "ccjs" | "json" => engine.load_document_json(&source_text()?)?,
        "ccjz" => {
            let document = chemsema_container::decode_ccjz(bytes)?;
            engine.load_document_json(&document)?;
        }
        "cdxml" => engine.load_cdxml_document(&source_text()?)?,
        "cdx" => engine.load_cdx_document(bytes)?,
        "sdf" | "sd" => engine.load_sdf_document(&source_text()?)?,
        _ => {
            return Err(format!(
                "Unsupported ChemSema document extension: .{extension}"
            ))
        }
    }
    let document_json = engine.document_json().map_err(|error| error.to_string())?;
    let cdxml = engine.document_cdxml();
    let render_list_json = serde_json::to_string(&engine.render_list())
        .map_err(|error| format!("Failed to serialize render list: {error}"))?;
    Ok(OleObjectPayload::from_clipboard(ClipboardPayload {
        chemsema_fragment_json: None,
        chemsema_document_json: Some(document_json),
        render_list_json: Some(render_list_json),
        cdxml: Some(cdxml.clone()),
        svg: Some(engine.document_svg()),
        text: Some(cdxml),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_engine() -> chemsema_engine::Engine {
        let mut engine = chemsema_engine::Engine::new();
        engine
            .load_cdxml_document(
                r#"<CDXML BondLength="14.4"><page id="1"><fragment id="2"><n id="3" p="0 0"/><n id="4" p="14.4 0"/><b id="5" B="3" E="4"/></fragment></page></CDXML>"#,
            )
            .unwrap();
        engine
    }

    fn assert_file_payload(extension: &str, bytes: &[u8]) {
        let path = PathBuf::from(format!("sample.{extension}"));
        let payload = ole_object_payload_from_document_bytes(&path, bytes)
            .unwrap_or_else(|error| panic!("{extension} file payload: {error}"));
        assert!(payload.document_was_supplied, "{extension}");
        assert!(payload.svg_was_supplied, "{extension}");
        assert!(payload.render_list_json.is_some(), "{extension}");
    }

    #[test]
    fn every_desktop_document_format_builds_an_editable_file_payload() {
        let engine = sample_engine();
        let document_json = engine.document_json().unwrap();
        assert_file_payload("ccjs", document_json.as_bytes());
        assert_file_payload(
            "ccjz",
            &chemsema_container::encode_ccjz(&document_json).unwrap(),
        );
        assert_file_payload("cdxml", engine.document_cdxml().as_bytes());
        assert_file_payload("cdx", &engine.document_cdx().unwrap());
        let sdf = engine.document_sdf().unwrap();
        assert_file_payload("sdf", sdf.as_bytes());
        assert_file_payload("sd", sdf.as_bytes());
    }
}
