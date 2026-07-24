use super::*;

pub(super) unsafe fn write_ole_storage_payload(
    storage: *mut c_void,
    payload: &OleObjectPayload,
    presentation_extent: SIZE,
) -> i32 {
    if storage.is_null() {
        return E_POINTER;
    }
    let hr = WriteClassStg(storage, &CLSID_CHEMSEMA_DOCUMENT);
    if !hresult_succeeded(hr) {
        return hr;
    }
    let user_type = wide_null(DOCUMENT_DISPLAY_NAME);
    let hr = WriteFmtUserTypeStg(
        storage,
        clipboard_format(FORMAT_CHEMSEMA_NATIVE),
        user_type.as_ptr(),
    );
    if !hresult_succeeded(hr) {
        return hr;
    }

    let contents = ole_contents_stream_payload(payload);
    let ole = ole_stream_payload();
    let obj_info = ole_obj_info_stream_payload();
    let document = chemsema_document_stream_payload(payload);
    let manifest = match ole_manifest_stream_payload(presentation_extent) {
        Ok(manifest) => manifest,
        Err(hr) => return hr,
    };
    let preview = payload.svg.as_bytes();

    let streams = [
        (OLE_STREAM_OLE, ole.as_slice()),
        (OLE_STREAM_CONTENTS, contents.as_slice()),
        (OLE_STREAM_OBJ_INFO, obj_info.as_slice()),
        (OLE_STREAM_MANIFEST, manifest.as_slice()),
        (OLE_STREAM_DOCUMENT, document.as_slice()),
        (OLE_STREAM_PREVIEW_SVG, preview),
    ];
    for (name, bytes) in streams {
        let hr = storage_write_stream(storage, name, bytes);
        if !hresult_succeeded(hr) {
            return hr;
        }
    }
    if let Some(cdxml) = payload.cdxml.as_deref() {
        let hr = storage_write_stream(storage, OLE_STREAM_SOURCE_CDXML, cdxml.as_bytes());
        if !hresult_succeeded(hr) {
            return hr;
        }
    }
    if let Ok(presentation) =
        ole_presentation_stream_for_payload(payload, presentation_extent, CF_ENHMETAFILE)
    {
        let hr = storage_write_stream(storage, OLE_STREAM_PRESENTATION_EMF, &presentation);
        if !hresult_succeeded(hr) {
            return hr;
        }
    }
    if let Ok(enhanced_print) = enhanced_metafile_bits_for_payload(payload, presentation_extent) {
        let hr = storage_write_stream(storage, OLE_STREAM_ENHANCED_PRINT, &enhanced_print);
        if !hresult_succeeded(hr) {
            return hr;
        }
    }
    storage_commit(storage)
}

pub(super) fn chemsema_document_stream_payload(payload: &OleObjectPayload) -> Vec<u8> {
    payload.chemsema_document_json.as_bytes().to_vec()
}

pub(super) fn ole_contents_stream_payload(payload: &OleObjectPayload) -> Vec<u8> {
    payload.chemsema_document_json.as_bytes().to_vec()
}

pub(super) fn ole_obj_info_stream_payload() -> [u8; 6] {
    // ODT: ODTPersist1=0, cf=0x0003 (metafile/EMF), ODTPersist2=0x0001
    // (fEMF). Word's own RTF clipboard stream uses this pre-cache value.
    [0x00, 0x00, 0x03, 0x00, 0x01, 0x00]
}

pub(super) fn ole_stream_payload() -> [u8; 20] {
    let mut bytes = [0u8; 20];
    bytes[0..4].copy_from_slice(&0x0200_0001u32.to_le_bytes());
    bytes[4..8].copy_from_slice(&0x0000_0008u32.to_le_bytes());
    bytes
}

pub(super) unsafe fn create_ole_storage_medium(
    payload: &OleObjectPayload,
    extent: SIZE,
    medium: *mut STGMEDIUM,
) -> i32 {
    if medium.is_null() {
        return E_POINTER;
    }
    *medium = STGMEDIUM::default();

    let mut lock_bytes = null_mut();
    let hr = CreateILockBytesOnHGlobal(null_mut(), 1, &mut lock_bytes);
    if !hresult_succeeded(hr) || lock_bytes.is_null() {
        return hr;
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
        return hr;
    }

    let hr = save_ole_object_storage(storage, payload, extent);
    if !hresult_succeeded(hr) {
        com_release(storage);
        return hr;
    }

    (*medium).tymed = TYMED_ISTORAGE as u32;
    (*medium).u.pstg = storage;
    (*medium).pUnkForRelease = null_mut();
    S_OK
}

pub(super) unsafe fn save_ole_object_storage(
    storage: *mut c_void,
    payload: &OleObjectPayload,
    presentation_extent: SIZE,
) -> i32 {
    let mut object = Box::new(ChemSemaOleObject::with_payload(payload.clone()));
    object.extent_himetric = presentation_extent;
    object.init_self_references();
    let object = Box::into_raw(object);
    let persist_storage =
        (&mut (*object).persist_storage as *mut InterfacePart<PersistStorageVtbl>).cast::<c_void>();
    let hr = OleSave(persist_storage, storage, 0);
    chemsema_object_release(object);
    if hresult_succeeded(hr) {
        storage_commit(storage)
    } else {
        hr
    }
}

pub(super) unsafe fn write_native_clipboard_storage_payload(
    storage: *mut c_void,
    payload: &OleObjectPayload,
) -> i32 {
    if storage.is_null() {
        return E_POINTER;
    }
    let hr = WriteClassStg(storage, &CLSID_CHEMSEMA_DOCUMENT);
    if !hresult_succeeded(hr) {
        return hr;
    }
    let user_type = wide_null(DOCUMENT_DISPLAY_NAME);
    let hr = WriteFmtUserTypeStg(
        storage,
        clipboard_format(FORMAT_CHEMSEMA_NATIVE),
        user_type.as_ptr(),
    );
    if !hresult_succeeded(hr) {
        return hr;
    }
    let hr = storage_write_stream(
        storage,
        OLE_STREAM_CONTENTS,
        &ole_contents_stream_payload(payload),
    );
    if !hresult_succeeded(hr) {
        return hr;
    }
    let hr = storage_write_stream(storage, OLE_STREAM_OLE, &ole_stream_payload());
    if !hresult_succeeded(hr) {
        return hr;
    }
    let hr = storage_write_stream(storage, OLE_STREAM_OBJ_INFO, &ole_obj_info_stream_payload());
    if !hresult_succeeded(hr) {
        return hr;
    }
    if let Ok(enhanced_print) =
        enhanced_metafile_bits_for_payload(payload, payload.extent_himetric())
    {
        let hr = storage_write_stream(storage, OLE_STREAM_ENHANCED_PRINT, &enhanced_print);
        if !hresult_succeeded(hr) {
            return hr;
        }
    }
    if let Ok(presentation) =
        ole_presentation_stream_for_payload(payload, payload.extent_himetric(), CF_ENHMETAFILE)
    {
        let hr = storage_write_stream(storage, OLE_STREAM_PRESENTATION_EMF, &presentation);
        if !hresult_succeeded(hr) {
            return hr;
        }
    }
    storage_commit(storage)
}

pub(super) fn native_clipboard_storage_file_bytes_for_payload(
    payload: &OleObjectPayload,
) -> Result<Vec<u8>, String> {
    let storage_path = env::temp_dir().join(format!(
        "chemsema-office-native-{}-{}.ole",
        std::process::id(),
        unique_temp_suffix()
    ));
    let result = unsafe {
        let mut storage = null_mut();
        let storage_path_w = wide_path_null(&storage_path);
        let hr = StgCreateDocfile(
            storage_path_w.as_ptr(),
            STGM_CREATE | STGM_READWRITE | STGM_SHARE_EXCLUSIVE,
            0,
            &mut storage,
        );
        if !hresult_succeeded(hr) || storage.is_null() {
            Err(format!(
                "StgCreateDocfile for Native clipboard payload failed: 0x{:08X}",
                hr as u32
            ))
        } else {
            let hr = write_native_clipboard_storage_payload(storage, payload);
            com_release(storage);
            if !hresult_succeeded(hr) {
                Err(format!(
                    "Saving Native clipboard payload failed: 0x{:08X}",
                    hr as u32
                ))
            } else {
                std::fs::read(&storage_path).map_err(|error| {
                    format!(
                        "Failed to read generated Native clipboard payload {}: {error}",
                        storage_path.display()
                    )
                })
            }
        }
    };
    let _ = std::fs::remove_file(storage_path);
    result
}

pub(super) fn hglobal_for_native_clipboard_payload(
    payload: &OleObjectPayload,
) -> Result<HGLOBAL, i32> {
    let bytes = native_clipboard_storage_file_bytes_for_payload(payload).map_err(|_| E_FAIL)?;
    hglobal_for_bytes(&bytes)
}

pub(super) fn hglobal_for_bytes(bytes: &[u8]) -> Result<HGLOBAL, i32> {
    unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE_FLAG, bytes.len());
        if handle.is_null() {
            return Err(E_OUTOFMEMORY);
        }
        let target = GlobalLock(handle).cast::<u8>();
        if target.is_null() {
            GlobalFree(handle);
            return Err(E_FAIL);
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
        GlobalUnlock(handle);
        Ok(handle)
    }
}

pub(super) fn hglobal_for_utf8_nul(value: &str) -> Result<HGLOBAL, i32> {
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(0);
    hglobal_for_bytes(&bytes)
}

pub(super) fn hglobal_for_utf16_nul(value: &str) -> Result<HGLOBAL, i32> {
    let wide = wide_null(value);
    let bytes = unsafe {
        std::slice::from_raw_parts(
            wide.as_ptr().cast::<u8>(),
            wide.len() * std::mem::size_of::<u16>(),
        )
    };
    hglobal_for_bytes(bytes)
}

pub(super) fn hglobal_for_object_descriptor(extent: SIZE) -> Result<HGLOBAL, i32> {
    let user_type = wide_null(DOCUMENT_DISPLAY_NAME);
    let source = wide_null(APP_NAME);
    let descriptor_size = std::mem::size_of::<OBJECTDESCRIPTOR>();
    let user_bytes = user_type.len() * std::mem::size_of::<u16>();
    let source_bytes = source.len() * std::mem::size_of::<u16>();
    let total = descriptor_size + user_bytes + source_bytes;
    unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE_FLAG, total);
        if handle.is_null() {
            return Err(E_OUTOFMEMORY);
        }
        let target = GlobalLock(handle).cast::<u8>();
        if target.is_null() {
            GlobalFree(handle);
            return Err(E_FAIL);
        }
        std::ptr::write_bytes(target, 0, total);
        let descriptor = target.cast::<OBJECTDESCRIPTOR>();
        (*descriptor).cbSize = descriptor_size as u32;
        (*descriptor).clsid = CLSID_CHEMSEMA_DOCUMENT;
        (*descriptor).dwDrawAspect = DVASPECT_CONTENT;
        (*descriptor).sizel = extent;
        (*descriptor).pointl = POINTL { x: 0, y: 0 };
        (*descriptor).dwStatus = default_misc_status();
        (*descriptor).dwFullUserTypeName = descriptor_size as u32;
        (*descriptor).dwSrcOfCopy = (descriptor_size + user_bytes) as u32;
        std::ptr::copy_nonoverlapping(
            user_type.as_ptr().cast::<u8>(),
            target.add(descriptor_size),
            user_bytes,
        );
        std::ptr::copy_nonoverlapping(
            source.as_ptr().cast::<u8>(),
            target.add(descriptor_size + user_bytes),
            source_bytes,
        );
        GlobalUnlock(handle);
        Ok(handle)
    }
}

pub(super) fn hglobal_for_word_rtf_object(payload: &OleObjectPayload) -> Result<HGLOBAL, i32> {
    let rtf = word_rtf_object_for_payload(payload).map_err(|_| E_FAIL)?;
    hglobal_for_utf8_nul(&rtf)
}

pub(super) fn word_rtf_object_for_payload(payload: &OleObjectPayload) -> Result<String, String> {
    let natural_extent = payload.extent_himetric();
    let display_extent = fit_extent_himetric_to_word_body(natural_extent);
    let emf = enhanced_metafile_bits_for_office_payload(payload, natural_extent)
        .map_err(|hr| format!("Failed to render Word RTF EMF preview: 0x{:08X}", hr as u32))?;
    let ole = ole_storage_file_bytes_for_payload(payload, natural_extent)?;
    let objdata = word_rtf_objdata_bytes(&ole, &emf)?;
    let width_twips = points_to_twips(himetric_to_points(natural_extent.cx));
    let height_twips = points_to_twips(himetric_to_points(natural_extent.cy));
    let scale_x = rtf_percent_scale(display_extent.cx, natural_extent.cx);
    let scale_y = rtf_percent_scale(display_extent.cy, natural_extent.cy);
    let objdata_hex = rtf_hex_lines(&objdata);
    let emf_hex = rtf_hex_lines(&emf);

    let mut rtf = String::new();
    rtf.push_str("{\\rtf1\\ansi\\ansicpg1252\\deff0");
    rtf.push_str("{\\fonttbl{\\f0\\fnil Arial;}}");
    rtf.push_str("\\viewkind4\\uc1\\pard\\plain\\f0\\fs22");
    rtf.push_str("{\\object\\objemb");
    rtf.push_str(&format!("\\objw{width_twips}\\objh{height_twips}"));
    rtf.push_str(&format!("{{\\*\\objclass {VERSIONED_PROG_ID}}}"));
    rtf.push_str(&format!("{{\\*\\objdata {objdata_hex}}}"));
    rtf.push_str("{\\result {\\*\\shppict{\\pict");
    rtf.push_str("{\\*\\picprop\\shplid1025");
    rtf.push_str("{\\sp{\\sn shapeType}{\\sv 75}}");
    rtf.push_str("{\\sp{\\sn fLine}{\\sv 0}}");
    rtf.push('}');
    rtf.push_str(&format!("\\picscalex{scale_x}\\picscaley{scale_y}"));
    rtf.push_str(&format!(
        "\\picw{}\\pich{}\\picwgoal{width_twips}\\pichgoal{height_twips}\\emfblip ",
        natural_extent.cx, natural_extent.cy
    ));
    rtf.push_str(&emf_hex);
    rtf.push_str("}}}");
    rtf.push_str("}\\par}");
    Ok(rtf)
}

pub(super) fn rtf_percent_scale(display: i32, natural: i32) -> i32 {
    if natural <= 0 {
        return 100;
    }
    ((display.max(1) as f64 / natural as f64) * 100.0)
        .round()
        .clamp(1.0, 1000.0) as i32
}

pub(super) fn word_rtf_objdata_bytes(
    ole_storage: &[u8],
    presentation_emf: &[u8],
) -> Result<Vec<u8>, String> {
    if ole_storage.len() > u32::MAX as usize {
        return Err("OLE storage is too large for RTF objdata.".into());
    }
    if presentation_emf.len() > u32::MAX as usize {
        return Err("RTF presentation EMF is too large for RTF objdata.".into());
    }
    let class_name = VERSIONED_PROG_ID.as_bytes();
    let class_name_len = class_name
        .len()
        .checked_add(1)
        .ok_or_else(|| "OLE class name length overflowed.".to_string())?;
    if class_name_len > u32::MAX as usize {
        return Err("OLE class name is too large for RTF objdata.".into());
    }

    let mut bytes =
        Vec::with_capacity(44 + class_name_len * 2 + ole_storage.len() + presentation_emf.len());
    bytes.extend_from_slice(&0x0000_0501u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&(class_name_len as u32).to_le_bytes());
    bytes.extend_from_slice(class_name);
    bytes.push(0);
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&(ole_storage.len() as u32).to_le_bytes());
    bytes.extend_from_slice(ole_storage);
    bytes.extend_from_slice(&0x0000_0501u32.to_le_bytes());
    bytes.extend_from_slice(&5u32.to_le_bytes());
    bytes.extend_from_slice(&(class_name_len as u32).to_le_bytes());
    bytes.extend_from_slice(class_name);
    bytes.push(0);
    bytes.extend_from_slice(&(CF_ENHMETAFILE as u32).to_le_bytes());
    bytes.extend_from_slice(&(presentation_emf.len() as u32).to_le_bytes());
    bytes.extend_from_slice(presentation_emf);
    Ok(bytes)
}

pub(super) fn rtf_hex_lines(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2 + bytes.len() / 32 + 1);
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 && index % 32 == 0 {
            out.push('\n');
        }
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

pub(super) fn word_docx_package_for_payload(payload: &OleObjectPayload) -> Result<Vec<u8>, String> {
    let natural_extent = payload.extent_himetric();
    let display_extent = fit_extent_himetric_to_word_body(natural_extent);
    let emf = enhanced_metafile_bits_for_office_payload(payload, display_extent).map_err(|hr| {
        format!(
            "Failed to render Word OOXML EMF preview: 0x{:08X}",
            hr as u32
        )
    })?;
    let ole = ole_storage_file_bytes_for_payload(payload, natural_extent)?;
    let display_width_pt = himetric_to_points(display_extent.cx);
    let display_height_pt = himetric_to_points(display_extent.cy);
    let natural_width_twips = points_to_twips(himetric_to_points(natural_extent.cx));
    let natural_height_twips = points_to_twips(himetric_to_points(natural_extent.cy));

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip_add_text(
            &mut zip,
            options,
            "[Content_Types].xml",
            &word_content_types_xml(),
        )?;
        zip_add_text(&mut zip, options, "_rels/.rels", &word_root_rels_xml())?;
        zip_add_text(
            &mut zip,
            options,
            "docProps/core.xml",
            &word_core_props_xml(),
        )?;
        zip_add_text(&mut zip, options, "docProps/app.xml", &word_app_props_xml())?;
        zip_add_text(
            &mut zip,
            options,
            "word/document.xml",
            &word_document_xml(
                display_width_pt,
                display_height_pt,
                natural_width_twips,
                natural_height_twips,
            ),
        )?;
        zip_add_text(
            &mut zip,
            options,
            "word/_rels/document.xml.rels",
            &word_document_rels_xml(),
        )?;
        zip_add_bytes(&mut zip, options, "word/media/image1.emf", &emf)?;
        zip_add_bytes(&mut zip, options, "word/embeddings/oleObject1.bin", &ole)?;
        zip.finish()
            .map_err(|error| format!("Failed to finish Word OOXML package: {error}"))?;
    }
    Ok(cursor.into_inner())
}

pub(super) fn zip_add_text<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    options: zip::write::SimpleFileOptions,
    name: &str,
    text: &str,
) -> Result<(), String> {
    zip_add_bytes(zip, options, name, text.as_bytes())
}

pub(super) fn zip_add_bytes<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    options: zip::write::SimpleFileOptions,
    name: &str,
    bytes: &[u8],
) -> Result<(), String> {
    zip.start_file(name, options)
        .map_err(|error| format!("Failed to add {name} to Word OOXML package: {error}"))?;
    zip.write_all(bytes)
        .map_err(|error| format!("Failed to write {name} to Word OOXML package: {error}"))
}

pub(super) fn word_content_types_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="bin" ContentType="application/vnd.openxmlformats-officedocument.oleObject"/><Default Extension="emf" ContentType="image/x-emf"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/><Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/></Types>"#.to_string()
}

pub(super) fn word_root_rels_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>"#.to_string()
}

pub(super) fn word_document_rels_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.emf"/><Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject" Target="embeddings/oleObject1.bin"/></Relationships>"#.to_string()
}

pub(super) fn word_core_props_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title>ChemSema Document</dc:title><dc:creator>ChemSema</dc:creator><cp:lastModifiedBy>ChemSema</cp:lastModifiedBy></cp:coreProperties>"#.to_string()
}

pub(super) fn word_app_props_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><Application>ChemSema</Application></Properties>"#.to_string()
}

pub(super) fn word_document_xml(
    width_pt: f64,
    height_pt: f64,
    width_twips: i32,
    height_twips: i32,
) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:v="urn:schemas-microsoft-com:vml" xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:object w:dxaOrig="{width_twips}" w:dyaOrig="{height_twips}"><v:shapetype id="_x0000_t75" coordsize="21600,21600" o:spt="75" o:preferrelative="t" path="m@4@5l@4@11@9@11@9@5xe" filled="f" stroked="f"><v:stroke joinstyle="miter"/><v:formulas><v:f eqn="if lineDrawn pixelLineWidth 0"/><v:f eqn="sum @0 1 0"/><v:f eqn="sum 0 0 @1"/><v:f eqn="prod @2 1 2"/><v:f eqn="prod @3 21600 pixelWidth"/><v:f eqn="prod @3 21600 pixelHeight"/><v:f eqn="sum @0 0 1"/><v:f eqn="prod @6 1 2"/><v:f eqn="prod @7 21600 pixelWidth"/><v:f eqn="sum @8 21600 0"/><v:f eqn="prod @7 21600 pixelHeight"/><v:f eqn="sum @10 21600 0"/></v:formulas><v:path o:extrusionok="f" gradientshapeok="t" o:connecttype="rect"/><o:lock v:ext="edit" aspectratio="t"/></v:shapetype><v:shape id="_x0000_i1025" type="#_x0000_t75" style="width:{:.1}pt;height:{:.1}pt" o:ole=""><v:imagedata r:id="rId4" o:title=""/></v:shape><o:OLEObject Type="Embed" ProgID="{VERSIONED_PROG_ID}" ShapeID="_x0000_i1025" DrawAspect="Content" ObjectID="_chemsema0001" r:id="rId5"/></w:object></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1800" w:bottom="1440" w:left="1800" w:header="851" w:footer="992" w:gutter="0"/></w:sectPr></w:body></w:document>"##,
        width_pt, height_pt
    )
}

pub(super) fn himetric_to_points(value: i32) -> f64 {
    (value.max(1) as f64 / HIMETRIC_PER_CM) * PT_PER_CM
}

pub(super) fn fit_extent_himetric_to_word_body(extent: SIZE) -> SIZE {
    let width_pt = himetric_to_points(extent.cx);
    let height_pt = himetric_to_points(extent.cy);
    let max_width_pt = WORD_A4_BODY_WIDTH_CM * PT_PER_CM;
    if width_pt <= max_width_pt || width_pt <= 0.0 || height_pt <= 0.0 {
        return extent;
    }
    let scale = max_width_pt / width_pt;
    SIZE {
        cx: points_to_himetric(width_pt * scale),
        cy: points_to_himetric(height_pt * scale),
    }
}

pub(super) fn points_to_twips(value: f64) -> i32 {
    (value * 20.0).round().clamp(1.0, i32::MAX as f64) as i32
}

pub(super) fn points_to_himetric(value: f64) -> i32 {
    ((value / PT_PER_CM) * HIMETRIC_PER_CM)
        .round()
        .clamp(MIN_OBJECT_EXTENT_HIMETRIC as f64, i32::MAX as f64) as i32
}

pub(super) fn ole_storage_file_bytes_for_payload(
    payload: &OleObjectPayload,
    presentation_extent: SIZE,
) -> Result<Vec<u8>, String> {
    let storage_path = env::temp_dir().join(format!(
        "chemsema-office-docx-{}-{}.ole",
        std::process::id(),
        unique_temp_suffix()
    ));
    let result = unsafe {
        let mut storage = null_mut();
        let storage_path_w = wide_path_null(&storage_path);
        let hr = StgCreateDocfile(
            storage_path_w.as_ptr(),
            STGM_CREATE | STGM_READWRITE | STGM_SHARE_EXCLUSIVE,
            0,
            &mut storage,
        );
        if !hresult_succeeded(hr) || storage.is_null() {
            Err(format!(
                "StgCreateDocfile for Word OOXML OLE embedding failed: 0x{:08X}",
                hr as u32
            ))
        } else {
            let hr = save_ole_object_storage(storage, payload, presentation_extent);
            com_release(storage);
            if !hresult_succeeded(hr) {
                Err(format!(
                    "Saving Word OOXML OLE embedding failed: 0x{:08X}",
                    hr as u32
                ))
            } else {
                std::fs::read(&storage_path).map_err(|error| {
                    format!(
                        "Failed to read generated Word OOXML OLE storage {}: {error}",
                        storage_path.display()
                    )
                })
            }
        }
    };
    let _ = std::fs::remove_file(storage_path);
    result
}

pub(super) fn unique_temp_suffix() -> String {
    let sequence = TEMP_SUFFIX_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{timestamp}-{sequence}")
}

pub(super) fn enhanced_print_is_emf(bytes: &[u8]) -> bool {
    bytes.len() >= 44
        && u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) == 1
        && bytes[40..44] == *b" EMF"
}

pub(super) fn hglobal_medium(handle: HGLOBAL, medium: *mut STGMEDIUM) -> i32 {
    if medium.is_null() {
        unsafe {
            GlobalFree(handle);
        }
        return E_POINTER;
    }
    unsafe {
        *medium = STGMEDIUM::default();
        (*medium).tymed = TYMED_HGLOBAL as u32;
        (*medium).u.hGlobal = handle;
        (*medium).pUnkForRelease = null_mut();
    }
    S_OK
}

pub(super) fn metafile_pict_medium(handle: HGLOBAL, medium: *mut STGMEDIUM) -> i32 {
    if medium.is_null() {
        unsafe {
            GlobalFree(handle);
        }
        return E_POINTER;
    }
    unsafe {
        *medium = STGMEDIUM::default();
        (*medium).tymed = TYMED_MFPICT as u32;
        (*medium).u.hMetaFilePict = handle.cast();
        (*medium).pUnkForRelease = null_mut();
    }
    S_OK
}

pub(super) fn enhanced_metafile_medium(handle: *mut c_void, medium: *mut STGMEDIUM) -> i32 {
    if medium.is_null() {
        unsafe {
            DeleteEnhMetaFile(handle);
        }
        return E_POINTER;
    }
    unsafe {
        *medium = STGMEDIUM::default();
        (*medium).tymed = TYMED_ENHMF as u32;
        (*medium).u.hEnhMetaFile = handle;
        (*medium).pUnkForRelease = null_mut();
    }
    S_OK
}

pub(super) fn hglobal_text_medium(value: &str, unicode: bool, medium: *mut STGMEDIUM) -> i32 {
    let handle = if unicode {
        hglobal_for_utf16_nul(value)
    } else {
        hglobal_for_utf8_nul(value)
    };
    match handle {
        Ok(handle) => hglobal_medium(handle, medium),
        Err(hr) => hr,
    }
}
