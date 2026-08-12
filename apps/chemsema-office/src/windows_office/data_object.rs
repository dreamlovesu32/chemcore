use super::*;

pub(super) unsafe fn replace_object_storage(object: *mut ChemSemaOleObject, storage: *mut c_void) {
    if object.is_null() || (*object).storage == storage {
        return;
    }
    com_release((*object).storage);
    (*object).storage = null_mut();
    if !storage.is_null() {
        com_add_ref(storage);
        (*object).storage = storage;
    }
}

pub(super) unsafe extern "system" fn unknown_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    object: *mut *mut c_void,
) -> i32 {
    chemsema_object_query_interface(this.cast::<ChemSemaOleObject>(), riid, object)
}

pub(super) unsafe extern "system" fn unknown_add_ref(this: *mut c_void) -> u32 {
    chemsema_object_add_ref(this.cast::<ChemSemaOleObject>())
}

pub(super) unsafe extern "system" fn unknown_release(this: *mut c_void) -> u32 {
    chemsema_object_release(this.cast::<ChemSemaOleObject>())
}

pub(super) fn chemsema_object_query_interface(
    object: *mut ChemSemaOleObject,
    riid: *const GUID,
    out: *mut *mut c_void,
) -> i32 {
    if out.is_null() {
        return E_POINTER;
    }
    unsafe {
        *out = null_mut();
    }
    if object.is_null() || riid.is_null() {
        return E_NOINTERFACE;
    }
    let riid = unsafe { &*riid };
    let interface = unsafe {
        if guid_eq(riid, &IID_IUNKNOWN) {
            object.cast::<c_void>()
        } else if guid_eq(riid, &IID_IDATA_OBJECT) {
            (&mut (*object).data_object as *mut InterfacePart<DataObjectVtbl>).cast::<c_void>()
        } else if guid_eq(riid, &IID_IPERSIST) || guid_eq(riid, &IID_IPERSIST_STORAGE) {
            (&mut (*object).persist_storage as *mut InterfacePart<PersistStorageVtbl>)
                .cast::<c_void>()
        } else if guid_eq(riid, &IID_IPERSIST_FILE) {
            (&mut (*object).persist_file as *mut InterfacePart<PersistFileVtbl>).cast::<c_void>()
        } else if guid_eq(riid, &IID_IOLE_OBJECT) {
            (&mut (*object).ole_object as *mut InterfacePart<OleObjectVtbl>).cast::<c_void>()
        } else if guid_eq(riid, &IID_IVIEW_OBJECT) || guid_eq(riid, &IID_IVIEW_OBJECT2) {
            (&mut (*object).view_object2 as *mut InterfacePart<ViewObject2Vtbl>).cast::<c_void>()
        } else if guid_eq(riid, &IID_IRUNNABLE_OBJECT) {
            (&mut (*object).runnable_object as *mut InterfacePart<RunnableObjectVtbl>)
                .cast::<c_void>()
        } else {
            null_mut()
        }
    };
    if interface.is_null() {
        return E_NOINTERFACE;
    }
    chemsema_object_add_ref(object);
    unsafe {
        *out = interface;
    }
    S_OK
}

pub(super) fn chemsema_object_add_ref(object: *mut ChemSemaOleObject) -> u32 {
    if object.is_null() {
        return 0;
    }
    unsafe { (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1 }
}

pub(super) fn chemsema_object_release(object: *mut ChemSemaOleObject) -> u32 {
    if object.is_null() {
        return 0;
    }
    let next = unsafe { (*object).ref_count.fetch_sub(1, Ordering::Release) - 1 };
    if next == 0 {
        std::sync::atomic::fence(Ordering::Acquire);
        unsafe {
            drop(Box::from_raw(object));
        }
    }
    next
}

pub(super) unsafe fn owner_from_part<T>(this: *mut c_void) -> *mut ChemSemaOleObject {
    if this.is_null() {
        return null_mut();
    }
    (*(this.cast::<InterfacePart<T>>())).owner
}

pub(super) unsafe extern "system" fn part_query_interface<T>(
    this: *mut c_void,
    riid: *const GUID,
    object: *mut *mut c_void,
) -> i32 {
    chemsema_object_query_interface(owner_from_part::<T>(this), riid, object)
}

pub(super) unsafe extern "system" fn part_add_ref<T>(this: *mut c_void) -> u32 {
    chemsema_object_add_ref(owner_from_part::<T>(this))
}

pub(super) unsafe extern "system" fn part_release<T>(this: *mut c_void) -> u32 {
    chemsema_object_release(owner_from_part::<T>(this))
}

pub(super) unsafe extern "system" fn data_object_get_data(
    this: *mut c_void,
    format: *const FORMATETC,
    medium: *mut STGMEDIUM,
) -> i32 {
    if format.is_null() || medium.is_null() {
        return E_POINTER;
    }
    let object = owner_from_part::<DataObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    let hr = write_clipboard_format_to_medium(
        &(*object).payload,
        (*object).extent_himetric,
        &*format,
        medium,
    );
    log_format_request("IDataObject::GetData", &*format, hr);
    hr
}

pub(super) unsafe extern "system" fn data_object_get_data_here(
    this: *mut c_void,
    format: *const FORMATETC,
    medium: *mut STGMEDIUM,
) -> i32 {
    if format.is_null() || medium.is_null() {
        return E_POINTER;
    }
    let object = owner_from_part::<DataObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    let embedded_object = clipboard_format(CLIPBOARD_FORMAT_EMBEDDED_OBJECT);
    let embed_source = clipboard_format(CLIPBOARD_FORMAT_EMBED_SOURCE);
    if ((*format).cfFormat == embedded_object || (*format).cfFormat == embed_source)
        && ((*format).tymed & TYMED_ISTORAGE as u32) != 0
    {
        let storage = (*medium).u.pstg;
        if storage.is_null() {
            return E_POINTER;
        }
        let hr = save_ole_object_storage(storage, &(*object).payload, (*object).extent_himetric);
        log_format_request("IDataObject::GetDataHere", &*format, hr);
        return hr;
    }
    log_format_request("IDataObject::GetDataHere", &*format, DV_E_FORMATETC);
    DV_E_FORMATETC
}

pub(super) unsafe extern "system" fn data_object_query_get_data(
    this: *mut c_void,
    format: *const FORMATETC,
) -> i32 {
    if format.is_null() {
        return E_POINTER;
    }
    let object = owner_from_part::<DataObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    let hr = if clipboard_format_supported(&(*object).payload, (*object).extent_himetric, &*format)
    {
        S_OK
    } else {
        DV_E_FORMATETC
    };
    log_format_request("IDataObject::QueryGetData", &*format, hr);
    hr
}

pub(super) unsafe extern "system" fn data_object_get_canonical_format_etc(
    _this: *mut c_void,
    _format_in: *const FORMATETC,
    format_out: *mut FORMATETC,
) -> i32 {
    if format_out.is_null() {
        return E_POINTER;
    }
    (*format_out).ptd = null_mut();
    0x00040130
}

pub(super) unsafe extern "system" fn data_object_set_data(
    _this: *mut c_void,
    _format: *const FORMATETC,
    _medium: *const STGMEDIUM,
    _release: i32,
) -> i32 {
    E_NOTIMPL
}

pub(super) unsafe extern "system" fn data_object_enum_format_etc(
    this: *mut c_void,
    direction: u32,
    enum_format_etc: *mut *mut c_void,
) -> i32 {
    if enum_format_etc.is_null() {
        return E_POINTER;
    }
    *enum_format_etc = null_mut();
    if direction != DATADIR_GET as u32 {
        return E_NOTIMPL;
    }
    let object = owner_from_part::<DataObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    log_ole_event("IDataObject::EnumFormatEtc(DATADIR_GET)");
    let enumerator = Box::new(FormatEtcEnumerator {
        vtbl: &FORMAT_ETC_ENUMERATOR_VTBL,
        ref_count: AtomicU32::new(1),
        formats: ole_clipboard_formats(&(*object).payload, (*object).extent_himetric),
        index: 0,
    });
    *enum_format_etc = Box::into_raw(enumerator).cast::<c_void>();
    S_OK
}

pub(super) unsafe extern "system" fn data_object_d_advise(
    this: *mut c_void,
    format: *const FORMATETC,
    advf: u32,
    sink: *mut c_void,
    connection: *mut u32,
) -> i32 {
    if format.is_null() || connection.is_null() {
        return E_POINTER;
    }
    *connection = 0;
    let object = owner_from_part::<DataObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    if (*object).data_advise_holder.is_null() {
        let hr = CreateDataAdviseHolder(&mut (*object).data_advise_holder);
        if !hresult_succeeded(hr) {
            log_format_request("IDataObject::DAdvise(CreateDataAdviseHolder)", &*format, hr);
            return hr;
        }
    }
    let holder_vtbl = *((*object)
        .data_advise_holder
        .cast::<*const DataAdviseHolderVtbl>());
    let data_object =
        (&mut (*object).data_object as *mut InterfacePart<DataObjectVtbl>).cast::<c_void>();
    let hr = ((*holder_vtbl).advise)(
        (*object).data_advise_holder,
        data_object,
        format.cast_mut(),
        advf,
        sink,
        connection,
    );
    log_format_request("IDataObject::DAdvise", &*format, hr);
    hr
}

pub(super) unsafe extern "system" fn data_object_d_unadvise(
    this: *mut c_void,
    connection: u32,
) -> i32 {
    let object = owner_from_part::<DataObjectVtbl>(this);
    if object.is_null() || (*object).data_advise_holder.is_null() {
        return E_POINTER;
    }
    let holder_vtbl = *((*object)
        .data_advise_holder
        .cast::<*const DataAdviseHolderVtbl>());
    let hr = ((*holder_vtbl).unadvise)((*object).data_advise_holder, connection);
    log_ole_event(&format!(
        "IDataObject::DUnadvise({connection}) -> 0x{:08X}",
        hr as u32
    ));
    hr
}

pub(super) unsafe extern "system" fn data_object_enum_d_advise(
    this: *mut c_void,
    enum_advise: *mut *mut c_void,
) -> i32 {
    if enum_advise.is_null() {
        return E_POINTER;
    }
    *enum_advise = null_mut();
    let object = owner_from_part::<DataObjectVtbl>(this);
    if object.is_null() || (*object).data_advise_holder.is_null() {
        return S_FALSE;
    }
    let holder_vtbl = *((*object)
        .data_advise_holder
        .cast::<*const DataAdviseHolderVtbl>());
    let hr = ((*holder_vtbl).enum_advise)((*object).data_advise_holder, enum_advise);
    log_ole_event(&format!("IDataObject::EnumDAdvise -> 0x{:08X}", hr as u32));
    hr
}

pub(super) unsafe extern "system" fn format_etc_enum_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    object: *mut *mut c_void,
) -> i32 {
    if object.is_null() {
        return E_POINTER;
    }
    *object = null_mut();
    if riid.is_null() {
        return E_NOINTERFACE;
    }
    if guid_eq(&*riid, &IID_IUNKNOWN) || guid_eq(&*riid, &IID_IENUM_FORMATETC) {
        *object = this;
        format_etc_enum_add_ref(this);
        return S_OK;
    }
    E_NOINTERFACE
}

pub(super) unsafe extern "system" fn format_etc_enum_add_ref(this: *mut c_void) -> u32 {
    if this.is_null() {
        return 0;
    }
    (*(this.cast::<FormatEtcEnumerator>()))
        .ref_count
        .fetch_add(1, Ordering::Relaxed)
        + 1
}

pub(super) unsafe extern "system" fn format_etc_enum_release(this: *mut c_void) -> u32 {
    if this.is_null() {
        return 0;
    }
    let next = (*(this.cast::<FormatEtcEnumerator>()))
        .ref_count
        .fetch_sub(1, Ordering::Release)
        - 1;
    if next == 0 {
        std::sync::atomic::fence(Ordering::Acquire);
        drop(Box::from_raw(this.cast::<FormatEtcEnumerator>()));
    }
    next
}

pub(super) unsafe extern "system" fn format_etc_enum_next(
    this: *mut c_void,
    count: u32,
    out: *mut FORMATETC,
    fetched: *mut u32,
) -> i32 {
    if this.is_null() || out.is_null() {
        return E_POINTER;
    }
    if count != 1 && fetched.is_null() {
        return E_POINTER;
    }
    let enumerator = &mut *(this.cast::<FormatEtcEnumerator>());
    let mut copied = 0;
    while copied < count && enumerator.index < enumerator.formats.len() {
        *out.add(copied as usize) = enumerator.formats[enumerator.index];
        log_format_request(
            "IEnumFORMATETC::Next",
            &enumerator.formats[enumerator.index],
            S_OK,
        );
        enumerator.index += 1;
        copied += 1;
    }
    if !fetched.is_null() {
        *fetched = copied;
    }
    if copied == count {
        S_OK
    } else {
        S_FALSE
    }
}

pub(super) unsafe extern "system" fn format_etc_enum_skip(this: *mut c_void, count: u32) -> i32 {
    if this.is_null() {
        return E_POINTER;
    }
    let enumerator = &mut *(this.cast::<FormatEtcEnumerator>());
    enumerator.index = (enumerator.index + count as usize).min(enumerator.formats.len());
    if enumerator.index < enumerator.formats.len() {
        S_OK
    } else {
        S_FALSE
    }
}

pub(super) unsafe extern "system" fn format_etc_enum_reset(this: *mut c_void) -> i32 {
    if this.is_null() {
        return E_POINTER;
    }
    (*(this.cast::<FormatEtcEnumerator>())).index = 0;
    S_OK
}

pub(super) unsafe extern "system" fn format_etc_enum_clone(
    this: *mut c_void,
    out: *mut *mut c_void,
) -> i32 {
    if this.is_null() || out.is_null() {
        return E_POINTER;
    }
    let enumerator = &*(this.cast::<FormatEtcEnumerator>());
    let clone = Box::new(FormatEtcEnumerator {
        vtbl: &FORMAT_ETC_ENUMERATOR_VTBL,
        ref_count: AtomicU32::new(1),
        formats: enumerator.formats.clone(),
        index: enumerator.index,
    });
    *out = Box::into_raw(clone).cast::<c_void>();
    S_OK
}

pub(super) unsafe extern "system" fn persist_storage_get_class_id(
    _this: *mut c_void,
    class_id: *mut GUID,
) -> i32 {
    if class_id.is_null() {
        return E_POINTER;
    }
    *class_id = CLSID_CHEMSEMA_DOCUMENT;
    S_OK
}

pub(super) unsafe extern "system" fn persist_storage_is_dirty(this: *mut c_void) -> i32 {
    let object = owner_from_part::<PersistStorageVtbl>(this);
    if !object.is_null() && (*object).dirty {
        S_OK
    } else {
        S_FALSE
    }
}

pub(super) unsafe extern "system" fn persist_storage_init_new(
    this: *mut c_void,
    storage: *mut c_void,
) -> i32 {
    let object = owner_from_part::<PersistStorageVtbl>(this);
    if object.is_null() || storage.is_null() {
        return E_POINTER;
    }
    replace_object_storage(object, storage);
    let hr = write_ole_storage_payload(storage, &(*object).payload, (*object).extent_himetric);
    if hresult_succeeded(hr) {
        (*object).dirty = false;
    }
    log_ole_event(&format!("IPersistStorage::InitNew -> 0x{:08X}", hr as u32));
    hr
}

pub(super) unsafe extern "system" fn persist_storage_load(
    this: *mut c_void,
    storage: *mut c_void,
) -> i32 {
    let object = owner_from_part::<PersistStorageVtbl>(this);
    if object.is_null() || storage.is_null() {
        return E_POINTER;
    }
    replace_object_storage(object, storage);
    if let Ok(document) = storage_read_stream(storage, OLE_STREAM_DOCUMENT).and_then(|bytes| {
        String::from_utf8(bytes)
            .map_err(|error| format!("ChemSemaDocument stream is not UTF-8: {error}"))
    }) {
        (*object).payload.chemsema_document_json = document;
    } else if let Ok(document) =
        storage_read_stream(storage, OLE_STREAM_CONTENTS).and_then(|bytes| {
            String::from_utf8(bytes)
                .map_err(|error| format!("CONTENTS stream is not UTF-8: {error}"))
        })
    {
        (*object).payload.chemsema_document_json = document;
    }
    if let Ok(svg) = storage_read_stream(storage, OLE_STREAM_PREVIEW_SVG).and_then(|bytes| {
        String::from_utf8(bytes)
            .map_err(|error| format!("ChemSemaPreviewSvg stream is not UTF-8: {error}"))
    }) {
        (*object).payload.svg = svg;
    }
    if let Ok(cdxml) = storage_read_stream(storage, OLE_STREAM_SOURCE_CDXML).and_then(|bytes| {
        String::from_utf8(bytes)
            .map_err(|error| format!("ChemSemaSourceCdxml stream is not UTF-8: {error}"))
    }) {
        (*object).payload.cdxml = Some(cdxml);
    }
    (*object).extent_himetric =
        storage_presentation_extent(storage).unwrap_or_else(|| (*object).payload.extent_himetric());
    (*object).dirty = false;
    log_ole_event("IPersistStorage::Load -> 0x00000000");
    S_OK
}

pub(super) unsafe extern "system" fn persist_storage_save(
    this: *mut c_void,
    storage: *mut c_void,
    _same_as_load: i32,
) -> i32 {
    if storage.is_null() {
        return E_POINTER;
    }
    let object = owner_from_part::<PersistStorageVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    let hr = write_ole_storage_payload(storage, &(*object).payload, (*object).extent_himetric);
    if hresult_succeeded(hr) {
        (*object).dirty = false;
    }
    log_ole_event(&format!("IPersistStorage::Save -> 0x{:08X}", hr as u32));
    hr
}

pub(super) unsafe extern "system" fn persist_storage_save_completed(
    this: *mut c_void,
    storage: *mut c_void,
) -> i32 {
    let object = owner_from_part::<PersistStorageVtbl>(this);
    if !object.is_null() {
        replace_object_storage(object, storage);
    }
    S_OK
}

pub(super) unsafe extern "system" fn persist_storage_hands_off_storage(this: *mut c_void) -> i32 {
    let object = owner_from_part::<PersistStorageVtbl>(this);
    if !object.is_null() {
        replace_object_storage(object, null_mut());
    }
    S_OK
}
