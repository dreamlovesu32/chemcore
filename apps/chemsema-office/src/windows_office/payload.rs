use super::*;

pub(super) fn copy_clipboard_payload(payload_path: PathBuf) -> Result<(), String> {
    let payload = read_ole_object_payload(&payload_path)?;

    unsafe {
        let hr = OleInitialize(null());
        if !hresult_succeeded(hr) {
            return Err(format!("OleInitialize failed: 0x{:08X}", hr as u32));
        }

        let mut object = Box::new(ChemSemaOleObject::with_payload(payload));
        object.init_self_references();
        let object = Box::into_raw(object);
        let data_object =
            (&mut (*object).data_object as *mut InterfacePart<DataObjectVtbl>).cast::<c_void>();

        // Flush the COM data object immediately so Word can paste an owned OLE
        // payload even after this helper process exits.
        let set_hr = OleSetClipboard(data_object);
        let flush_hr = if hresult_succeeded(set_hr) {
            OleFlushClipboard()
        } else {
            set_hr
        };
        chemsema_object_release(object);
        OleUninitialize();

        if !hresult_succeeded(flush_hr) {
            return Err(format!(
                "Failed to place ChemSema OLE object on clipboard: 0x{:08X}",
                flush_hr as u32
            ));
        }
    }

    Ok(())
}

pub(super) fn read_clipboard_payload(output_path: PathBuf) -> Result<(), String> {
    unsafe {
        let hr = OleInitialize(null());
        if !hresult_succeeded(hr) {
            return Err(format!("OleInitialize failed: 0x{:08X}", hr as u32));
        }
        let mut data_object = null_mut();
        let clipboard_hr = OleGetClipboard(&mut data_object);
        let result = if hresult_succeeded(clipboard_hr) && !data_object.is_null() {
            payload_from_data_object(data_object)
                .map(|payload| payload.clipboard_payload())
                .and_then(|payload| {
                    serde_json::to_vec(&payload)
                        .map_err(|error| format!("Failed to serialize clipboard payload: {error}"))
                })
                .and_then(|json| {
                    std::fs::write(&output_path, json).map_err(|error| {
                        format!(
                            "Failed to write clipboard payload {}: {error}",
                            output_path.display()
                        )
                    })
                })
        } else {
            Err(format!(
                "OleGetClipboard failed: 0x{:08X}",
                clipboard_hr as u32
            ))
        };
        if !data_object.is_null() {
            com_release(data_object);
        }
        OleUninitialize();
        result
    }
}

pub(super) fn read_ole_object_payload(payload_path: &PathBuf) -> Result<OleObjectPayload, String> {
    let json = std::fs::read_to_string(payload_path).map_err(|error| {
        format!(
            "Failed to read OLE clipboard payload {}: {error}",
            payload_path.display()
        )
    })?;
    let payload: ClipboardPayload = serde_json::from_str(&json)
        .map_err(|error| format!("Invalid OLE clipboard payload JSON: {error}"))?;
    Ok(OleObjectPayload::from_clipboard(payload))
}

pub(super) fn ole_edit_session_payload_json(payload: &OleObjectPayload) -> Result<String, String> {
    serde_json::to_string_pretty(&serde_json::json!({
        "chemsemaFragmentJson": payload.chemsema_fragment_json.clone(),
        "chemsemaDocumentJson": payload.chemsema_document_json.clone(),
        "renderListJson": payload.render_list_json.clone(),
        "cdxml": payload.cdxml.clone(),
        "svg": payload.svg.clone(),
        "text": payload.text.clone(),
    }))
    .map_err(|error| format!("Failed to serialize OLE edit session payload: {error}"))
}

pub(super) fn ole_object_payload_from_edit_session_text(
    text: &str,
) -> Result<OleObjectPayload, String> {
    if text.trim().is_empty() {
        return Err("OLE edit session payload was empty.".into());
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
        if value.get("chemsemaDocumentJson").is_some() {
            let payload: ClipboardPayload = serde_json::from_value(value)
                .map_err(|error| format!("Invalid OLE edit session payload JSON: {error}"))?;
            if payload
                .chemsema_document_json
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                return Ok(OleObjectPayload::from_clipboard(payload));
            }
            return Err("OLE edit session payload did not contain chemsemaDocumentJson.".into());
        }
    }
    Ok(OleObjectPayload::from_clipboard(ClipboardPayload {
        chemsema_fragment_json: None,
        chemsema_document_json: Some(text.to_string()),
        render_list_json: None,
        cdxml: None,
        svg: None,
        text: None,
    }))
}

pub(super) fn write_word_docx_payload(
    payload_path: PathBuf,
    output_path: PathBuf,
) -> Result<(), String> {
    let payload = read_ole_object_payload(&payload_path)?;
    let package = word_docx_package_for_payload(&payload)?;
    std::fs::write(&output_path, package).map_err(|error| {
        format!(
            "Failed to write Word OOXML package {}: {error}",
            output_path.display()
        )
    })?;
    println!(
        "{DOCUMENT_DISPLAY_NAME} Word OOXML package written to {}.",
        output_path.display()
    );
    Ok(())
}

pub(super) fn write_emf_payload(payload_path: PathBuf, output_path: PathBuf) -> Result<(), String> {
    let payload = read_ole_object_payload(&payload_path)?;
    write_emf_payload_object(&output_path, &payload)?;
    println!(
        "{DOCUMENT_DISPLAY_NAME} EMF written to {}.",
        output_path.display()
    );
    Ok(())
}

pub(crate) fn write_emf_payload_json(
    output_path: &std::path::Path,
    payload_json: &str,
) -> Result<(), String> {
    let payload: ClipboardPayload = serde_json::from_str(payload_json)
        .map_err(|error| format!("Invalid EMF payload JSON: {error}"))?;
    write_emf_payload_object(output_path, &OleObjectPayload::from_clipboard(payload))
}

pub(super) fn write_emf_payload_object(
    output_path: &std::path::Path,
    payload: &OleObjectPayload,
) -> Result<(), String> {
    let extent = payload.extent_himetric();
    let emf = enhanced_metafile_bits_for_payload(payload, extent)
        .map_err(|hr| format!("Failed to render EMF: 0x{:08X}", hr as u32))?;
    std::fs::write(output_path, emf)
        .map_err(|error| format!("Failed to write EMF {}: {error}", output_path.display()))?;
    Ok(())
}

pub(super) fn write_preview_bounds_payload(
    payload_path: PathBuf,
    output_path: PathBuf,
) -> Result<(), String> {
    let payload = read_ole_object_payload(&payload_path)?;
    let extent = payload.extent_himetric();
    let report = preview_bounds_debug_report(&payload, extent);
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("Failed to serialize preview bounds report: {error}"))?;
    std::fs::write(&output_path, json).map_err(|error| {
        format!(
            "Failed to write preview bounds report {}: {error}",
            output_path.display()
        )
    })?;
    println!(
        "{DOCUMENT_DISPLAY_NAME} preview bounds report written to {}.",
        output_path.display()
    );
    Ok(())
}

pub(super) fn run_self_test() -> Result<(), String> {
    unsafe {
        let factory = (&CLASS_FACTORY as *const ClassFactory)
            .cast_mut()
            .cast::<c_void>();
        let mut ole_object = null_mut();
        let hr =
            class_factory_create_instance(factory, null_mut(), &IID_IOLE_OBJECT, &mut ole_object);
        if !hresult_succeeded(hr) || ole_object.is_null() {
            return Err(format!(
                "IClassFactory::CreateInstance(IOleObject) failed: 0x{:08X}",
                hr as u32
            ));
        }

        let required_interfaces = [
            ("IDataObject", IID_IDATA_OBJECT),
            ("IPersistStorage", IID_IPERSIST_STORAGE),
            ("IViewObject2", IID_IVIEW_OBJECT2),
            ("IRunnableObject", IID_IRUNNABLE_OBJECT),
        ];
        for (name, iid) in required_interfaces {
            let mut interface = null_mut();
            let hr = part_query_interface::<OleObjectVtbl>(ole_object, &iid, &mut interface);
            if !hresult_succeeded(hr) || interface.is_null() {
                part_release::<OleObjectVtbl>(ole_object);
                return Err(format!(
                    "QueryInterface({name}) failed: 0x{:08X}",
                    hr as u32
                ));
            }
            com_release(interface);
        }

        let mut class_id = GUID::default();
        let get_class_id = (*(*(ole_object.cast::<*const OleObjectVtbl>()))).get_user_class_id;
        let hr = get_class_id(ole_object, &mut class_id);
        part_release::<OleObjectVtbl>(ole_object);
        if !hresult_succeeded(hr) || !guid_eq(&class_id, &CLSID_CHEMSEMA_DOCUMENT) {
            return Err(format!(
                "IOleObject::GetUserClassID failed: 0x{:08X}",
                hr as u32
            ));
        }

        let mut data_object = null_mut();
        let hr =
            class_factory_create_instance(factory, null_mut(), &IID_IDATA_OBJECT, &mut data_object);
        if !hresult_succeeded(hr) || data_object.is_null() {
            return Err(format!(
                "IClassFactory::CreateInstance(IDataObject) failed: 0x{:08X}",
                hr as u32
            ));
        }
        let embedded_format = FORMATETC {
            cfFormat: clipboard_format(CLIPBOARD_FORMAT_EMBEDDED_OBJECT),
            ptd: null_mut(),
            dwAspect: DVASPECT_CONTENT,
            lindex: -1,
            tymed: TYMED_ISTORAGE as u32,
        };
        let embed_source_format = FORMATETC {
            cfFormat: clipboard_format(CLIPBOARD_FORMAT_EMBED_SOURCE),
            ptd: null_mut(),
            dwAspect: DVASPECT_CONTENT,
            lindex: -1,
            tymed: TYMED_ISTORAGE as u32,
        };
        let query_get_data = (*(*(data_object.cast::<*const DataObjectVtbl>()))).query_get_data;
        let hr = query_get_data(data_object, &embedded_format);
        if !hresult_succeeded(hr) {
            part_release::<DataObjectVtbl>(data_object);
            return Err(format!(
                "IDataObject::QueryGetData(Embedded Object) failed: 0x{:08X}",
                hr as u32
            ));
        }
        let hr = query_get_data(data_object, &embed_source_format);
        if !hresult_succeeded(hr) {
            part_release::<DataObjectVtbl>(data_object);
            return Err(format!(
                "IDataObject::QueryGetData(Embed Source) failed: 0x{:08X}",
                hr as u32
            ));
        }
        let mut medium = STGMEDIUM::default();
        let get_data = (*(*(data_object.cast::<*const DataObjectVtbl>()))).get_data;
        let hr = get_data(data_object, &embedded_format, &mut medium);
        if !hresult_succeeded(hr) || medium.tymed != TYMED_ISTORAGE as u32 {
            part_release::<DataObjectVtbl>(data_object);
            return Err(format!(
                "IDataObject::GetData(Embedded Object) failed: 0x{:08X}",
                hr as u32
            ));
        }
        if medium.u.pstg.is_null() {
            part_release::<DataObjectVtbl>(data_object);
            return Err("IDataObject::GetData(Embedded Object) returned a null storage.".into());
        }
        let document = storage_read_stream(medium.u.pstg, OLE_STREAM_DOCUMENT)
            .or_else(|_| storage_read_stream(medium.u.pstg, OLE_STREAM_CONTENTS))?;
        ReleaseStgMedium(&mut medium);
        let document = String::from_utf8(document)
            .map_err(|error| format!("Embedded source document stream is not UTF-8: {error}"))?;
        if !document.contains("\"name\":\"chemsema\"") {
            return Err("Embedded source did not contain a ChemSema document stream.".into());
        }

        run_ole_create_from_data_self_test(data_object)?;
        part_release::<DataObjectVtbl>(data_object);
    }

    run_persist_storage_self_test()?;
    run_ole_edit_session_update_self_test()?;
    run_word_docx_package_self_test()?;

    println!("{DOCUMENT_DISPLAY_NAME} COM object self-test passed.");
    Ok(())
}

pub(super) unsafe fn run_ole_create_from_data_self_test(
    data_object: *mut c_void,
) -> Result<(), String> {
    let ole_init_hr = OleInitialize(null());
    if !hresult_succeeded(ole_init_hr) {
        return Err(format!(
            "OleInitialize for OleCreateFromData self-test failed: 0x{:08X}",
            ole_init_hr as u32
        ));
    }
    let mut lock_bytes = null_mut();
    let hr = CreateILockBytesOnHGlobal(null_mut(), 1, &mut lock_bytes);
    if !hresult_succeeded(hr) || lock_bytes.is_null() {
        OleUninitialize();
        return Err(format!(
            "CreateILockBytesOnHGlobal for OleCreateFromData self-test failed: 0x{:08X}",
            hr as u32
        ));
    }
    let mut storage = null_mut();
    let hr = StgCreateDocfileOnILockBytes(
        lock_bytes,
        STGM_CREATE | STGM_READWRITE | STGM_SHARE_EXCLUSIVE,
        0,
        &mut storage,
    );
    com_release(lock_bytes);
    if !hresult_succeeded(hr) || storage.is_null() {
        OleUninitialize();
        return Err(format!(
            "StgCreateDocfileOnILockBytes for OleCreateFromData self-test failed: 0x{:08X}",
            hr as u32
        ));
    }

    let render_format = FORMATETC {
        cfFormat: CF_ENHMETAFILE,
        ptd: null_mut(),
        dwAspect: DVASPECT_CONTENT,
        lindex: -1,
        tymed: TYMED_ENHMF as u32,
    };
    let mut ole_object = null_mut();
    let hr = OleCreateFromData(
        data_object,
        &IID_IOLE_OBJECT,
        OLERENDER_FORMAT as u32,
        &render_format,
        null_mut(),
        storage,
        &mut ole_object,
    );
    if !ole_object.is_null() {
        com_release(ole_object);
    }
    com_release(storage);
    OleUninitialize();
    if !hresult_succeeded(hr) {
        return Err(format!(
            "OleCreateFromData self-test failed: 0x{:08X}",
            hr as u32
        ));
    }
    Ok(())
}

pub(super) fn run_persist_storage_self_test() -> Result<(), String> {
    let storage_path = env::temp_dir().join(format!(
        "chemsema-office-self-test-{}.ole",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&storage_path);

    let result = (|| unsafe {
        let mut storage = null_mut();
        let storage_path_w = wide_path_null(&storage_path);
        let hr = StgCreateDocfile(
            storage_path_w.as_ptr(),
            STGM_CREATE | STGM_READWRITE | STGM_SHARE_EXCLUSIVE,
            0,
            &mut storage,
        );
        if !hresult_succeeded(hr) || storage.is_null() {
            return Err(format!(
                "StgCreateDocfile for OLE storage self-test failed: 0x{:08X}",
                hr as u32
            ));
        }

        let mut persist_storage = null_mut();
        let factory = (&CLASS_FACTORY as *const ClassFactory)
            .cast_mut()
            .cast::<c_void>();
        let hr = class_factory_create_instance(
            factory,
            null_mut(),
            &IID_IPERSIST_STORAGE,
            &mut persist_storage,
        );
        if !hresult_succeeded(hr) || persist_storage.is_null() {
            com_release(storage);
            return Err(format!(
                "IClassFactory::CreateInstance(IPersistStorage) failed: 0x{:08X}",
                hr as u32
            ));
        }

        let init_new = (*(*(persist_storage.cast::<*const PersistStorageVtbl>()))).init_new;
        let hr = init_new(persist_storage, storage);
        if !hresult_succeeded(hr) {
            com_release(persist_storage);
            com_release(storage);
            return Err(format!(
                "IPersistStorage::InitNew storage self-test failed: 0x{:08X}",
                hr as u32
            ));
        }

        let document = storage_read_stream(storage, OLE_STREAM_DOCUMENT)?;
        let contents = storage_read_stream(storage, OLE_STREAM_CONTENTS)?;
        let manifest = storage_read_stream(storage, OLE_STREAM_MANIFEST)?;
        let preview = storage_read_stream(storage, OLE_STREAM_PREVIEW_SVG)?;
        let presentation_emf = storage_read_stream(storage, OLE_STREAM_PRESENTATION_EMF)?;
        let enhanced_print = storage_read_stream(storage, OLE_STREAM_ENHANCED_PRINT)?;

        com_release(persist_storage);
        com_release(storage);

        let document = String::from_utf8(document)
            .map_err(|error| format!("ChemSemaDocument stream is not UTF-8: {error}"))?;
        let contents = String::from_utf8(contents)
            .map_err(|error| format!("CONTENTS stream is not UTF-8: {error}"))?;
        if !document.contains("\"name\":\"chemsema\"") || !document.contains("\"objects\"") {
            return Err("ChemSemaDocument stream did not contain a blank ChemSema document".into());
        }
        if contents != document {
            return Err("CONTENTS stream did not match ChemSemaDocument stream".into());
        }

        let manifest = String::from_utf8(manifest)
            .map_err(|error| format!("ChemSemaManifest stream is not UTF-8: {error}"))?;
        if !manifest.contains(OLE_STREAM_DOCUMENT) || !manifest.contains(OLE_STREAM_PREVIEW_SVG) {
            return Err("ChemSemaManifest stream did not reference required object streams".into());
        }

        let preview = String::from_utf8(preview)
            .map_err(|error| format!("ChemSemaPreviewSvg stream is not UTF-8: {error}"))?;
        if !preview.contains("<svg") || !preview.contains(DOCUMENT_DISPLAY_NAME) {
            return Err("ChemSemaPreviewSvg stream did not contain the preview placeholder".into());
        }
        if presentation_emf.len() <= 40 {
            return Err("OLE EMF presentation stream was unexpectedly empty.".into());
        }
        if !enhanced_print_is_emf(&enhanced_print) {
            return Err("Enhanced print stream did not contain an EMF payload.".into());
        }

        Ok(())
    })();

    let _ = std::fs::remove_file(storage_path);
    result
}

pub(super) fn run_ole_edit_session_update_self_test() -> Result<(), String> {
    let storage_path = env::temp_dir().join(format!(
        "chemsema-office-edit-session-self-test-{}.ole",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&storage_path);

    let result = (|| unsafe {
        let mut storage = null_mut();
        let storage_path_w = wide_path_null(&storage_path);
        let hr = StgCreateDocfile(
            storage_path_w.as_ptr(),
            STGM_CREATE | STGM_READWRITE | STGM_SHARE_EXCLUSIVE,
            0,
            &mut storage,
        );
        if !hresult_succeeded(hr) || storage.is_null() {
            return Err(format!(
                "StgCreateDocfile for OLE edit session self-test failed: 0x{:08X}",
                hr as u32
            ));
        }

        let mut persist_storage = null_mut();
        let factory = (&CLASS_FACTORY as *const ClassFactory)
            .cast_mut()
            .cast::<c_void>();
        let hr = class_factory_create_instance(
            factory,
            null_mut(),
            &IID_IPERSIST_STORAGE,
            &mut persist_storage,
        );
        if !hresult_succeeded(hr) || persist_storage.is_null() {
            com_release(storage);
            return Err(format!(
                "IClassFactory::CreateInstance(IPersistStorage) edit session self-test failed: 0x{:08X}",
                hr as u32
            ));
        }

        let init_new = (*(*(persist_storage.cast::<*const PersistStorageVtbl>()))).init_new;
        let hr = init_new(persist_storage, storage);
        if !hresult_succeeded(hr) {
            com_release(persist_storage);
            com_release(storage);
            return Err(format!(
                "IPersistStorage::InitNew edit session self-test failed: 0x{:08X}",
                hr as u32
            ));
        }

        let object = owner_from_part::<PersistStorageVtbl>(persist_storage);
        if object.is_null() {
            com_release(persist_storage);
            com_release(storage);
            return Err("OLE edit session self-test could not resolve object owner.".into());
        }

        let mut document_json: serde_json::Value =
            serde_json::from_str(&(*object).payload.chemsema_document_json).map_err(|error| {
                format!("Initial OLE document JSON self-test parse failed: {error}")
            })?;
        document_json["document"]["title"] =
            serde_json::Value::String("ChemSema OLE edit session self-test".into());
        let document_json = serde_json::to_string(&document_json).map_err(|error| {
            format!("Edited OLE document JSON self-test serialize failed: {error}")
        })?;
        let edit_payload = OleObjectPayload::from_clipboard(ClipboardPayload {
            chemsema_fragment_json: None,
            chemsema_document_json: Some(document_json),
            render_list_json: None,
            cdxml: Some("<CDXML></CDXML>".to_string()),
            svg: None,
            text: Some("<CDXML></CDXML>".to_string()),
        });
        let edit_payload_json = ole_edit_session_payload_json(&edit_payload)?;

        let hr = apply_ole_edit_session_update(object, &edit_payload_json);
        if !hresult_succeeded(hr) {
            com_release(persist_storage);
            com_release(storage);
            return Err(format!(
                "OLE edit session update self-test failed: 0x{:08X}",
                hr as u32
            ));
        }

        let is_dirty = (*(*(persist_storage.cast::<*const PersistStorageVtbl>()))).is_dirty;
        let dirty_hr = is_dirty(persist_storage);
        if dirty_hr != S_OK {
            com_release(persist_storage);
            com_release(storage);
            return Err(format!(
                "OLE edit session update self-test did not mark object dirty: 0x{:08X}",
                dirty_hr as u32
            ));
        }

        let stored_document = storage_read_stream(storage, OLE_STREAM_DOCUMENT)?;
        let stored_document = String::from_utf8(stored_document)
            .map_err(|error| format!("Edited ChemSemaDocument stream is not UTF-8: {error}"))?;
        if !stored_document.contains("ChemSema OLE edit session self-test") {
            com_release(persist_storage);
            com_release(storage);
            return Err("OLE edit session update self-test did not update storage.".into());
        }
        let presentation_emf = storage_read_stream(storage, OLE_STREAM_PRESENTATION_EMF)?;
        if presentation_emf.len() <= 40 {
            com_release(persist_storage);
            com_release(storage);
            return Err(
                "OLE edit session update self-test did not update presentation EMF.".into(),
            );
        }
        let enhanced_print = storage_read_stream(storage, OLE_STREAM_ENHANCED_PRINT)?;
        if !enhanced_print_is_emf(&enhanced_print) {
            com_release(persist_storage);
            com_release(storage);
            return Err(
                "OLE edit session update self-test did not update enhanced print EMF.".into(),
            );
        }

        com_release(persist_storage);
        com_release(storage);
        Ok(())
    })();

    let _ = std::fs::remove_file(storage_path);
    result
}

pub(super) fn run_word_docx_package_self_test() -> Result<(), String> {
    let payload = OleObjectPayload::blank();
    let package = word_docx_package_for_payload(&payload)?;
    let reader = Cursor::new(package);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|error| format!("Generated Word OOXML package is not a zip: {error}"))?;
    let mut names = Vec::new();
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|error| format!("Failed to inspect Word OOXML package entry: {error}"))?;
        names.push(file.name().to_string());
    }
    for required in [
        "[Content_Types].xml",
        "word/document.xml",
        "word/_rels/document.xml.rels",
        "word/media/image1.emf",
        "word/embeddings/oleObject1.bin",
    ] {
        if !names.iter().any(|name| name == required) {
            return Err(format!(
                "Generated Word OOXML package is missing {required}."
            ));
        }
    }
    let mut document_xml = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|error| format!("Generated Word OOXML package has no document.xml: {error}"))?
        .read_to_string(&mut document_xml)
        .map_err(|error| format!("Failed to read generated document.xml: {error}"))?;
    if !document_xml.contains("ProgID=\"ChemSema.Document.1\"")
        || !document_xml.contains("r:id=\"rId4\"")
        || !document_xml.contains("r:id=\"rId5\"")
    {
        return Err(
            "Generated Word OOXML document does not link both EMF preview and OLE embedding."
                .into(),
        );
    }
    Ok(())
}
