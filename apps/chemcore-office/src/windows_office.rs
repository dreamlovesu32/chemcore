use std::env;
use std::ffi::c_void;
use std::mem::zeroed;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::Command;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicU32, Ordering};

use windows_sys::core::GUID;
use windows_sys::Win32::Foundation::{
    GlobalFree, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, HGLOBAL, POINTL, RECT, SIZE,
};
use windows_sys::Win32::Graphics::Gdi::{
    CreatePen, DeleteObject, Ellipse, GetStockObject, LineTo, MoveToEx, Rectangle, SelectObject,
    SetBkMode, TextOutW, HDC, HGDIOBJ, NULL_BRUSH, PS_SOLID, TRANSPARENT,
};
use windows_sys::Win32::System::Com::StructuredStorage::{
    CreateILockBytesOnHGlobal, StgCreateDocfile, StgCreateDocfileOnILockBytes, WriteClassStg,
};
use windows_sys::Win32::System::Com::{
    CoInitializeEx, CoRegisterClassObject, CoRevokeClassObject, CoTaskMemAlloc, CoUninitialize,
    CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED, DATADIR_GET, DVASPECT_CONTENT, FORMATETC,
    REGCLS_MULTIPLEUSE, STATSTG, STGC_DEFAULT, STGMEDIUM, STGM_CREATE, STGM_READ, STGM_READWRITE,
    STGM_SHARE_EXCLUSIVE, TYMED_HGLOBAL, TYMED_ISTORAGE,
};
use windows_sys::Win32::System::DataExchange::RegisterClipboardFormatW;
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock};
use windows_sys::Win32::System::Ole::{
    OleFlushClipboard, OleInitialize, OleRegEnumVerbs, OleRegGetMiscStatus, OleRegGetUserType,
    OleSetClipboard, OleUninitialize, ReleaseStgMedium, OBJECTDESCRIPTOR,
    OLEMISC_ACTIVATEWHENVISIBLE, OLEMISC_INSIDEOUT, OLEMISC_RENDERINGISDEVICEINDEPENDENT,
    OLEMISC_SETCLIENTSITEFIRST,
};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, REG_SZ,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, TranslateMessage, MSG,
};

const APP_NAME: &str = "Chemcore";
const DOCUMENT_DISPLAY_NAME: &str = "Chemcore Document";
const PROG_ID: &str = "Chemcore.Document";
const VERSIONED_PROG_ID: &str = "Chemcore.Document.1";
const CLSID_STRING: &str = "{CB69F54F-F21E-44DE-84FB-89D98FECE056}";
const OLE_STREAM_MANIFEST: &str = "ChemcoreManifest";
const OLE_STREAM_DOCUMENT: &str = "ChemcoreDocument";
const OLE_STREAM_PREVIEW_SVG: &str = "ChemcorePreviewSvg";
const CLIPBOARD_FORMAT_EMBED_SOURCE: &str = "Embed Source";
const CLIPBOARD_FORMAT_OBJECT_DESCRIPTOR: &str = "Object Descriptor";
const FORMAT_CHEMCORE_FRAGMENT: &str = "Chemcore Clipboard Fragment";
const FORMAT_CHEMCORE_DOCUMENT_JSON: &str = "Chemcore Document JSON";
const FORMAT_CHEMDRAW_INTERCHANGE: &str = "ChemDraw Interchange Format";
const FORMAT_CDXML_MIME: &str = "chemical/x-cdxml";
const FORMAT_SVG_MIME: &str = "image/svg+xml";
const FORMAT_SVG: &str = "SVG";
const CF_UNICODETEXT: u16 = 13;
const GMEM_MOVEABLE_FLAG: u32 = 0x0002;
const DEFAULT_OBJECT_WIDTH_HIMETRIC: i32 = 6000;
const DEFAULT_OBJECT_HEIGHT_HIMETRIC: i32 = 3000;

const CLSID_CHEMCORE_DOCUMENT: GUID = GUID {
    data1: 0xcb69f54f,
    data2: 0xf21e,
    data3: 0x44de,
    data4: [0x84, 0xfb, 0x89, 0xd9, 0x8f, 0xec, 0xe0, 0x56],
};

const IID_IUNKNOWN: GUID = GUID {
    data1: 0x00000000,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

const IID_ICLASS_FACTORY: GUID = GUID {
    data1: 0x00000001,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

const IID_IDATA_OBJECT: GUID = GUID {
    data1: 0x0000010e,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

const IID_IENUM_FORMATETC: GUID = GUID {
    data1: 0x00000103,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

const IID_IPERSIST: GUID = GUID {
    data1: 0x0000010c,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

const IID_IPERSIST_STORAGE: GUID = GUID {
    data1: 0x0000010a,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

const IID_IOLE_OBJECT: GUID = GUID {
    data1: 0x00000112,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

const IID_IVIEW_OBJECT: GUID = GUID {
    data1: 0x0000010d,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

const IID_IVIEW_OBJECT2: GUID = GUID {
    data1: 0x00000127,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

const IID_IRUNNABLE_OBJECT: GUID = GUID {
    data1: 0x00000126,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

const S_OK: i32 = 0;
const S_FALSE: i32 = 1;
const E_FAIL: i32 = 0x80004005u32 as i32;
const E_POINTER: i32 = 0x80004003u32 as i32;
const E_NOINTERFACE: i32 = 0x80004002u32 as i32;
const E_NOTIMPL: i32 = 0x80004001u32 as i32;
const E_OUTOFMEMORY: i32 = 0x8007000eu32 as i32;
const DV_E_FORMATETC: i32 = 0x80040064u32 as i32;
const DV_E_TYMED: i32 = 0x80040069u32 as i32;
const OLE_E_NOTRUNNING: i32 = 0x80040005u32 as i32;
const CLASS_E_NOAGGREGATION: i32 = 0x80040110u32 as i32;

pub fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_default();
    match command.as_str() {
        "--register-user" => register(RegistrationScope::User),
        "--unregister-user" => unregister(RegistrationScope::User),
        "--register-machine" => register(RegistrationScope::Machine),
        "--unregister-machine" => unregister(RegistrationScope::Machine),
        "--print-registration" => print_registration(),
        "--self-test" => run_self_test(),
        "--copy-clipboard-payload" => {
            let payload_path = args.next().ok_or_else(|| {
                "--copy-clipboard-payload requires a JSON payload path.".to_string()
            })?;
            copy_clipboard_payload(PathBuf::from(payload_path))
        }
        "--serve" | "-Embedding" | "/Embedding" | "--embedding" => run_com_server(),
        "" | "--help" | "-h" | "/?" => {
            print_help();
            Ok(())
        }
        other => Err(format!("Unknown chemcore-office command: {other}")),
    }
}

#[derive(Clone, Copy)]
enum RegistrationScope {
    User,
    Machine,
}

impl RegistrationScope {
    fn root(self) -> HKEY {
        match self {
            Self::User => HKEY_CURRENT_USER,
            Self::Machine => HKEY_LOCAL_MACHINE,
        }
    }

    fn prefix(self) -> &'static str {
        match self {
            Self::User => "HKCU\\Software\\Classes",
            Self::Machine => "HKLM\\Software\\Classes",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::User => "current user",
            Self::Machine => "machine",
        }
    }
}

fn register(scope: RegistrationScope) -> Result<(), String> {
    let server_path = current_server_path()?;
    let server_command = quote_path(&server_path);
    let icon_command = format!("{server_command},0");
    let root = scope.root();

    set_key_default(root, &classes_path(PROG_ID), DOCUMENT_DISPLAY_NAME)?;
    set_key_default(
        root,
        &classes_path(&format!("{PROG_ID}\\CLSID")),
        CLSID_STRING,
    )?;
    set_key_default(
        root,
        &classes_path(&format!("{PROG_ID}\\CurVer")),
        VERSIONED_PROG_ID,
    )?;

    set_key_default(
        root,
        &classes_path(VERSIONED_PROG_ID),
        DOCUMENT_DISPLAY_NAME,
    )?;
    set_key_default(
        root,
        &classes_path(&format!("{VERSIONED_PROG_ID}\\CLSID")),
        CLSID_STRING,
    )?;

    let clsid_path = classes_path(&format!("CLSID\\{CLSID_STRING}"));
    set_key_default(root, &clsid_path, DOCUMENT_DISPLAY_NAME)?;
    set_key_default(root, &format!("{clsid_path}\\ProgID"), VERSIONED_PROG_ID)?;
    set_key_default(
        root,
        &format!("{clsid_path}\\VersionIndependentProgID"),
        PROG_ID,
    )?;
    set_key_default(
        root,
        &format!("{clsid_path}\\LocalServer32"),
        &server_command,
    )?;
    set_key_default(root, &format!("{clsid_path}\\DefaultIcon"), &icon_command)?;
    set_key_default(
        root,
        &format!("{clsid_path}\\AuxUserType\\2"),
        DOCUMENT_DISPLAY_NAME,
    )?;
    set_key_default(root, &format!("{clsid_path}\\Verb\\0"), "&Edit,0,2")?;
    set_key_default(root, &format!("{clsid_path}\\Verb\\1"), "&Open,0,2")?;
    set_key_default(root, &format!("{clsid_path}\\MiscStatus"), "0")?;
    create_key(root, &format!("{clsid_path}\\Insertable"))?;

    println!(
        "Registered {DOCUMENT_DISPLAY_NAME} for {} at {}",
        scope.label(),
        scope.prefix()
    );
    println!("CLSID: {CLSID_STRING}");
    println!("Server: {}", server_path.display());
    Ok(())
}

fn unregister(scope: RegistrationScope) -> Result<(), String> {
    let root = scope.root();
    delete_tree(root, &classes_path(PROG_ID))?;
    delete_tree(root, &classes_path(VERSIONED_PROG_ID))?;
    delete_tree(root, &classes_path(&format!("CLSID\\{CLSID_STRING}")))?;
    println!(
        "Unregistered {DOCUMENT_DISPLAY_NAME} for {} from {}",
        scope.label(),
        scope.prefix()
    );
    Ok(())
}

fn print_registration() -> Result<(), String> {
    let server_path = current_server_path()?;
    println!("{APP_NAME} Office/OLE registration");
    println!("Display name: {DOCUMENT_DISPLAY_NAME}");
    println!("ProgID: {PROG_ID}");
    println!("Versioned ProgID: {VERSIONED_PROG_ID}");
    println!("CLSID: {CLSID_STRING}");
    println!("Server: {}", server_path.display());
    Ok(())
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClipboardPayload {
    chemcore_fragment_json: Option<String>,
    chemcore_document_json: Option<String>,
    cdxml: Option<String>,
    svg: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Clone)]
struct OleObjectPayload {
    chemcore_fragment_json: Option<String>,
    chemcore_document_json: String,
    cdxml: Option<String>,
    svg: String,
    text: Option<String>,
}

impl OleObjectPayload {
    fn blank() -> Self {
        let chemcore_document_json =
            serde_json::to_string(&chemcore_engine::ChemcoreDocument::blank())
                .unwrap_or_else(|_| "{}".to_string());
        Self {
            chemcore_fragment_json: None,
            chemcore_document_json,
            cdxml: None,
            svg: String::from_utf8(ole_preview_svg_stream_payload()).unwrap_or_default(),
            text: Some(DOCUMENT_DISPLAY_NAME.to_string()),
        }
    }

    fn from_clipboard(payload: ClipboardPayload) -> Self {
        let fallback = Self::blank();
        Self {
            chemcore_fragment_json: payload.chemcore_fragment_json,
            chemcore_document_json: payload
                .chemcore_document_json
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(fallback.chemcore_document_json),
            cdxml: payload.cdxml.filter(|value| !value.trim().is_empty()),
            svg: payload
                .svg
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(fallback.svg),
            text: payload
                .text
                .filter(|value| !value.trim().is_empty())
                .or(Some(DOCUMENT_DISPLAY_NAME.to_string())),
        }
    }
}

fn copy_clipboard_payload(payload_path: PathBuf) -> Result<(), String> {
    let json = std::fs::read_to_string(&payload_path).map_err(|error| {
        format!(
            "Failed to read OLE clipboard payload {}: {error}",
            payload_path.display()
        )
    })?;
    let payload: ClipboardPayload = serde_json::from_str(&json)
        .map_err(|error| format!("Invalid OLE clipboard payload JSON: {error}"))?;
    let payload = OleObjectPayload::from_clipboard(payload);

    unsafe {
        let hr = OleInitialize(null());
        if !hresult_succeeded(hr) {
            return Err(format!("OleInitialize failed: 0x{:08X}", hr as u32));
        }

        let mut object = Box::new(ChemcoreOleObject::with_payload(payload));
        object.init_self_references();
        let object = Box::into_raw(object);
        let data_object =
            (&mut (*object).data_object as *mut InterfacePart<DataObjectVtbl>).cast::<c_void>();

        let set_hr = OleSetClipboard(data_object);
        let flush_hr = if hresult_succeeded(set_hr) {
            OleFlushClipboard()
        } else {
            set_hr
        };
        chemcore_object_release(object);
        OleUninitialize();

        if !hresult_succeeded(flush_hr) {
            return Err(format!(
                "Failed to place Chemcore OLE object on clipboard: 0x{:08X}",
                flush_hr as u32
            ));
        }
    }

    println!("{DOCUMENT_DISPLAY_NAME} OLE clipboard payload copied.");
    Ok(())
}

fn run_self_test() -> Result<(), String> {
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
        if !hresult_succeeded(hr) || !guid_eq(&class_id, &CLSID_CHEMCORE_DOCUMENT) {
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
        let embed_format = FORMATETC {
            cfFormat: clipboard_format(CLIPBOARD_FORMAT_EMBED_SOURCE),
            ptd: null_mut(),
            dwAspect: DVASPECT_CONTENT,
            lindex: -1,
            tymed: TYMED_ISTORAGE as u32,
        };
        let query_get_data = (*(*(data_object.cast::<*const DataObjectVtbl>()))).query_get_data;
        let hr = query_get_data(data_object, &embed_format);
        if !hresult_succeeded(hr) {
            part_release::<DataObjectVtbl>(data_object);
            return Err(format!(
                "IDataObject::QueryGetData(Embed Source) failed: 0x{:08X}",
                hr as u32
            ));
        }
        let mut medium = STGMEDIUM::default();
        let get_data = (*(*(data_object.cast::<*const DataObjectVtbl>()))).get_data;
        let hr = get_data(data_object, &embed_format, &mut medium);
        part_release::<DataObjectVtbl>(data_object);
        if !hresult_succeeded(hr) || medium.tymed != TYMED_ISTORAGE as u32 {
            return Err(format!(
                "IDataObject::GetData(Embed Source) failed: 0x{:08X}",
                hr as u32
            ));
        }
        if medium.u.pstg.is_null() {
            return Err("IDataObject::GetData(Embed Source) returned a null storage.".into());
        }
        let document = storage_read_stream(medium.u.pstg, OLE_STREAM_DOCUMENT)?;
        ReleaseStgMedium(&mut medium);
        let document = String::from_utf8(document)
            .map_err(|error| format!("Embedded source document stream is not UTF-8: {error}"))?;
        if !document.contains("\"name\":\"chemcore\"") {
            return Err("Embedded source did not contain a Chemcore document stream.".into());
        }
    }

    run_persist_storage_self_test()?;

    println!("{DOCUMENT_DISPLAY_NAME} COM object self-test passed.");
    Ok(())
}

fn run_persist_storage_self_test() -> Result<(), String> {
    let storage_path = env::temp_dir().join(format!(
        "chemcore-office-self-test-{}.ole",
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
        let manifest = storage_read_stream(storage, OLE_STREAM_MANIFEST)?;
        let preview = storage_read_stream(storage, OLE_STREAM_PREVIEW_SVG)?;

        com_release(persist_storage);
        com_release(storage);

        let document = String::from_utf8(document)
            .map_err(|error| format!("ChemcoreDocument stream is not UTF-8: {error}"))?;
        if !document.contains("\"name\":\"chemcore\"") || !document.contains("\"objects\"") {
            return Err("ChemcoreDocument stream did not contain a blank Chemcore document".into());
        }

        let manifest = String::from_utf8(manifest)
            .map_err(|error| format!("ChemcoreManifest stream is not UTF-8: {error}"))?;
        if !manifest.contains(OLE_STREAM_DOCUMENT) || !manifest.contains(OLE_STREAM_PREVIEW_SVG) {
            return Err("ChemcoreManifest stream did not reference required object streams".into());
        }

        let preview = String::from_utf8(preview)
            .map_err(|error| format!("ChemcorePreviewSvg stream is not UTF-8: {error}"))?;
        if !preview.contains("<svg") || !preview.contains(DOCUMENT_DISPLAY_NAME) {
            return Err("ChemcorePreviewSvg stream did not contain the preview placeholder".into());
        }

        Ok(())
    })();

    let _ = std::fs::remove_file(storage_path);
    result
}

fn current_server_path() -> Result<PathBuf, String> {
    env::current_exe().map_err(|error| format!("Failed to resolve chemcore-office.exe: {error}"))
}

fn quote_path(path: &PathBuf) -> String {
    format!("\"{}\"", path.display())
}

fn classes_path(path: &str) -> String {
    format!("Software\\Classes\\{path}")
}

fn create_key(root: HKEY, subkey: &str) -> Result<(), String> {
    let subkey_w = wide_null(subkey);
    let mut key: HKEY = null_mut();
    let status = unsafe { RegCreateKeyW(root, subkey_w.as_ptr(), &mut key) };
    if status != ERROR_SUCCESS {
        return Err(format!("Failed to create registry key {subkey}: {status}"));
    }
    unsafe {
        RegCloseKey(key);
    }
    Ok(())
}

fn set_key_default(root: HKEY, subkey: &str, value: &str) -> Result<(), String> {
    let subkey_w = wide_null(subkey);
    let mut key: HKEY = null_mut();
    let status = unsafe { RegCreateKeyW(root, subkey_w.as_ptr(), &mut key) };
    if status != ERROR_SUCCESS {
        return Err(format!("Failed to create registry key {subkey}: {status}"));
    }

    let value_w = wide_null(value);
    let bytes = (value_w.len() * std::mem::size_of::<u16>()) as u32;
    let status =
        unsafe { RegSetValueExW(key, null(), 0, REG_SZ, value_w.as_ptr().cast::<u8>(), bytes) };
    unsafe {
        RegCloseKey(key);
    }
    if status != ERROR_SUCCESS {
        return Err(format!(
            "Failed to set default registry value for {subkey}: {status}"
        ));
    }
    Ok(())
}

fn delete_tree(root: HKEY, subkey: &str) -> Result<(), String> {
    let subkey_w = wide_null(subkey);
    let status = unsafe { RegDeleteTreeW(root, subkey_w.as_ptr()) };
    if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
        return Ok(());
    }
    Err(format!("Failed to delete registry tree {subkey}: {status}"))
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wide_path_null(path: &PathBuf) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[repr(C)]
struct ClassFactory {
    vtbl: *const ClassFactoryVtbl,
}

unsafe impl Sync for ClassFactory {}

#[repr(C)]
struct ClassFactoryVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    create_instance:
        unsafe extern "system" fn(*mut c_void, *mut c_void, *const GUID, *mut *mut c_void) -> i32,
    lock_server: unsafe extern "system" fn(*mut c_void, i32) -> i32,
}

static CLASS_FACTORY_VTBL: ClassFactoryVtbl = ClassFactoryVtbl {
    query_interface: class_factory_query_interface,
    add_ref: class_factory_add_ref,
    release: class_factory_release,
    create_instance: class_factory_create_instance,
    lock_server: class_factory_lock_server,
};

static CLASS_FACTORY: ClassFactory = ClassFactory {
    vtbl: &CLASS_FACTORY_VTBL,
};

fn run_com_server() -> Result<(), String> {
    let hr = unsafe { CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED as u32) };
    if !hresult_succeeded(hr) {
        return Err(format!("CoInitializeEx failed: 0x{:08X}", hr as u32));
    }

    let mut registration_cookie = 0;
    let hr = unsafe {
        CoRegisterClassObject(
            &CLSID_CHEMCORE_DOCUMENT,
            (&CLASS_FACTORY as *const ClassFactory)
                .cast_mut()
                .cast::<c_void>(),
            CLSCTX_LOCAL_SERVER,
            REGCLS_MULTIPLEUSE as u32,
            &mut registration_cookie,
        )
    };
    if !hresult_succeeded(hr) {
        unsafe {
            CoUninitialize();
        }
        return Err(format!(
            "CoRegisterClassObject failed for {CLSID_STRING}: 0x{:08X}",
            hr as u32
        ));
    }

    println!("{DOCUMENT_DISPLAY_NAME} COM local server is running.");
    run_message_loop();

    unsafe {
        CoRevokeClassObject(registration_cookie);
        CoUninitialize();
    }
    Ok(())
}

fn run_message_loop() {
    let mut message: MSG = unsafe { zeroed() };
    loop {
        let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        if result <= 0 {
            break;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

unsafe extern "system" fn class_factory_query_interface(
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
    if guid_eq(&*riid, &IID_IUNKNOWN) || guid_eq(&*riid, &IID_ICLASS_FACTORY) {
        *object = this;
        class_factory_add_ref(this);
        return S_OK;
    }
    E_NOINTERFACE
}

unsafe extern "system" fn class_factory_add_ref(_this: *mut c_void) -> u32 {
    2
}

unsafe extern "system" fn class_factory_release(_this: *mut c_void) -> u32 {
    1
}

unsafe extern "system" fn class_factory_create_instance(
    _this: *mut c_void,
    outer: *mut c_void,
    riid: *const GUID,
    object: *mut *mut c_void,
) -> i32 {
    if !object.is_null() {
        *object = null_mut();
    }
    if object.is_null() {
        return E_POINTER;
    }
    if !outer.is_null() {
        return CLASS_E_NOAGGREGATION;
    }
    if riid.is_null() {
        return E_NOINTERFACE;
    }
    let mut instance = Box::new(ChemcoreOleObject::new());
    instance.init_self_references();
    let instance = Box::into_raw(instance);
    let hr = chemcore_object_query_interface(instance, riid, object);
    chemcore_object_release(instance);
    hr
}

unsafe extern "system" fn class_factory_lock_server(_this: *mut c_void, _lock: i32) -> i32 {
    S_OK
}

#[repr(C)]
struct ChemcoreOleObject {
    unknown_vtbl: *const UnknownVtbl,
    ref_count: AtomicU32,
    data_object: InterfacePart<DataObjectVtbl>,
    persist_storage: InterfacePart<PersistStorageVtbl>,
    ole_object: InterfacePart<OleObjectVtbl>,
    view_object2: InterfacePart<ViewObject2Vtbl>,
    runnable_object: InterfacePart<RunnableObjectVtbl>,
    client_site: *mut c_void,
    storage: *mut c_void,
    ole_advise_holder: *mut c_void,
    payload: OleObjectPayload,
    extent_himetric: SIZE,
}

#[repr(C)]
struct InterfacePart<T> {
    vtbl: *const T,
    owner: *mut ChemcoreOleObject,
}

impl ChemcoreOleObject {
    fn new() -> Self {
        Self::with_payload(OleObjectPayload::blank())
    }

    fn with_payload(payload: OleObjectPayload) -> Self {
        Self {
            unknown_vtbl: &UNKNOWN_VTBL,
            ref_count: AtomicU32::new(1),
            data_object: InterfacePart {
                vtbl: &DATA_OBJECT_VTBL,
                owner: null_mut(),
            },
            persist_storage: InterfacePart {
                vtbl: &PERSIST_STORAGE_VTBL,
                owner: null_mut(),
            },
            ole_object: InterfacePart {
                vtbl: &OLE_OBJECT_VTBL,
                owner: null_mut(),
            },
            view_object2: InterfacePart {
                vtbl: &VIEW_OBJECT2_VTBL,
                owner: null_mut(),
            },
            runnable_object: InterfacePart {
                vtbl: &RUNNABLE_OBJECT_VTBL,
                owner: null_mut(),
            },
            client_site: null_mut(),
            storage: null_mut(),
            ole_advise_holder: null_mut(),
            payload,
            extent_himetric: SIZE {
                cx: DEFAULT_OBJECT_WIDTH_HIMETRIC,
                cy: DEFAULT_OBJECT_HEIGHT_HIMETRIC,
            },
        }
    }

    fn init_self_references(&mut self) {
        let owner = self as *mut ChemcoreOleObject;
        self.data_object.owner = owner;
        self.persist_storage.owner = owner;
        self.ole_object.owner = owner;
        self.view_object2.owner = owner;
        self.runnable_object.owner = owner;
    }
}

impl Drop for ChemcoreOleObject {
    fn drop(&mut self) {
        unsafe {
            com_release(self.client_site);
            self.client_site = null_mut();
            com_release(self.ole_advise_holder);
            self.ole_advise_holder = null_mut();
        }
    }
}

#[repr(C)]
struct UnknownVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
}

static UNKNOWN_VTBL: UnknownVtbl = UnknownVtbl {
    query_interface: unknown_query_interface,
    add_ref: unknown_add_ref,
    release: unknown_release,
};

unsafe extern "system" fn unknown_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    object: *mut *mut c_void,
) -> i32 {
    chemcore_object_query_interface(this.cast::<ChemcoreOleObject>(), riid, object)
}

unsafe extern "system" fn unknown_add_ref(this: *mut c_void) -> u32 {
    chemcore_object_add_ref(this.cast::<ChemcoreOleObject>())
}

unsafe extern "system" fn unknown_release(this: *mut c_void) -> u32 {
    chemcore_object_release(this.cast::<ChemcoreOleObject>())
}

fn chemcore_object_query_interface(
    object: *mut ChemcoreOleObject,
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
    chemcore_object_add_ref(object);
    unsafe {
        *out = interface;
    }
    S_OK
}

fn chemcore_object_add_ref(object: *mut ChemcoreOleObject) -> u32 {
    if object.is_null() {
        return 0;
    }
    unsafe { (*object).ref_count.fetch_add(1, Ordering::Relaxed) + 1 }
}

fn chemcore_object_release(object: *mut ChemcoreOleObject) -> u32 {
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

unsafe fn owner_from_part<T>(this: *mut c_void) -> *mut ChemcoreOleObject {
    if this.is_null() {
        return null_mut();
    }
    (*(this.cast::<InterfacePart<T>>())).owner
}

unsafe extern "system" fn part_query_interface<T>(
    this: *mut c_void,
    riid: *const GUID,
    object: *mut *mut c_void,
) -> i32 {
    chemcore_object_query_interface(owner_from_part::<T>(this), riid, object)
}

unsafe extern "system" fn part_add_ref<T>(this: *mut c_void) -> u32 {
    chemcore_object_add_ref(owner_from_part::<T>(this))
}

unsafe extern "system" fn part_release<T>(this: *mut c_void) -> u32 {
    chemcore_object_release(owner_from_part::<T>(this))
}

#[repr(C)]
struct DataObjectVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    get_data: unsafe extern "system" fn(*mut c_void, *const FORMATETC, *mut STGMEDIUM) -> i32,
    get_data_here: unsafe extern "system" fn(*mut c_void, *const FORMATETC, *mut STGMEDIUM) -> i32,
    query_get_data: unsafe extern "system" fn(*mut c_void, *const FORMATETC) -> i32,
    get_canonical_format_etc:
        unsafe extern "system" fn(*mut c_void, *const FORMATETC, *mut FORMATETC) -> i32,
    set_data:
        unsafe extern "system" fn(*mut c_void, *const FORMATETC, *const STGMEDIUM, i32) -> i32,
    enum_format_etc: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> i32,
    d_advise:
        unsafe extern "system" fn(*mut c_void, *const FORMATETC, u32, *mut c_void, *mut u32) -> i32,
    d_unadvise: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    enum_d_advise: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32,
}

static DATA_OBJECT_VTBL: DataObjectVtbl = DataObjectVtbl {
    query_interface: part_query_interface::<DataObjectVtbl>,
    add_ref: part_add_ref::<DataObjectVtbl>,
    release: part_release::<DataObjectVtbl>,
    get_data: data_object_get_data,
    get_data_here: data_object_get_data_here,
    query_get_data: data_object_query_get_data,
    get_canonical_format_etc: data_object_get_canonical_format_etc,
    set_data: data_object_set_data,
    enum_format_etc: data_object_enum_format_etc,
    d_advise: data_object_d_advise,
    d_unadvise: data_object_d_unadvise,
    enum_d_advise: data_object_enum_d_advise,
};

unsafe extern "system" fn data_object_get_data(
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
    write_clipboard_format_to_medium(
        &(*object).payload,
        (*object).extent_himetric,
        &*format,
        medium,
    )
}

unsafe extern "system" fn data_object_get_data_here(
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
    let embed_source = clipboard_format(CLIPBOARD_FORMAT_EMBED_SOURCE);
    if (*format).cfFormat == embed_source && ((*format).tymed & TYMED_ISTORAGE as u32) != 0 {
        let storage = (*medium).u.pstg;
        if storage.is_null() {
            return E_POINTER;
        }
        return write_ole_storage_payload(storage, &(*object).payload);
    }
    DV_E_FORMATETC
}

unsafe extern "system" fn data_object_query_get_data(
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
    if clipboard_format_supported(&(*object).payload, (*object).extent_himetric, &*format) {
        S_OK
    } else {
        DV_E_FORMATETC
    }
}

unsafe extern "system" fn data_object_get_canonical_format_etc(
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

unsafe extern "system" fn data_object_set_data(
    _this: *mut c_void,
    _format: *const FORMATETC,
    _medium: *const STGMEDIUM,
    _release: i32,
) -> i32 {
    E_NOTIMPL
}

unsafe extern "system" fn data_object_enum_format_etc(
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
    let enumerator = Box::new(FormatEtcEnumerator {
        vtbl: &FORMAT_ETC_ENUMERATOR_VTBL,
        ref_count: AtomicU32::new(1),
        formats: ole_clipboard_formats(&(*object).payload, (*object).extent_himetric),
        index: 0,
    });
    *enum_format_etc = Box::into_raw(enumerator).cast::<c_void>();
    S_OK
}

unsafe extern "system" fn data_object_d_advise(
    _this: *mut c_void,
    _format: *const FORMATETC,
    _advf: u32,
    _sink: *mut c_void,
    connection: *mut u32,
) -> i32 {
    if !connection.is_null() {
        *connection = 0;
    }
    E_NOTIMPL
}

unsafe extern "system" fn data_object_d_unadvise(_this: *mut c_void, _connection: u32) -> i32 {
    E_NOTIMPL
}

unsafe extern "system" fn data_object_enum_d_advise(
    _this: *mut c_void,
    enum_advise: *mut *mut c_void,
) -> i32 {
    if !enum_advise.is_null() {
        *enum_advise = null_mut();
    }
    E_NOTIMPL
}

#[repr(C)]
struct FormatEtcEnumerator {
    vtbl: *const FormatEtcEnumeratorVtbl,
    ref_count: AtomicU32,
    formats: Vec<FORMATETC>,
    index: usize,
}

#[repr(C)]
struct FormatEtcEnumeratorVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    next: unsafe extern "system" fn(*mut c_void, u32, *mut FORMATETC, *mut u32) -> i32,
    skip: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    reset: unsafe extern "system" fn(*mut c_void) -> i32,
    clone: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32,
}

static FORMAT_ETC_ENUMERATOR_VTBL: FormatEtcEnumeratorVtbl = FormatEtcEnumeratorVtbl {
    query_interface: format_etc_enum_query_interface,
    add_ref: format_etc_enum_add_ref,
    release: format_etc_enum_release,
    next: format_etc_enum_next,
    skip: format_etc_enum_skip,
    reset: format_etc_enum_reset,
    clone: format_etc_enum_clone,
};

unsafe extern "system" fn format_etc_enum_query_interface(
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

unsafe extern "system" fn format_etc_enum_add_ref(this: *mut c_void) -> u32 {
    if this.is_null() {
        return 0;
    }
    (*(this.cast::<FormatEtcEnumerator>()))
        .ref_count
        .fetch_add(1, Ordering::Relaxed)
        + 1
}

unsafe extern "system" fn format_etc_enum_release(this: *mut c_void) -> u32 {
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

unsafe extern "system" fn format_etc_enum_next(
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

unsafe extern "system" fn format_etc_enum_skip(this: *mut c_void, count: u32) -> i32 {
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

unsafe extern "system" fn format_etc_enum_reset(this: *mut c_void) -> i32 {
    if this.is_null() {
        return E_POINTER;
    }
    (*(this.cast::<FormatEtcEnumerator>())).index = 0;
    S_OK
}

unsafe extern "system" fn format_etc_enum_clone(this: *mut c_void, out: *mut *mut c_void) -> i32 {
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

#[repr(C)]
struct PersistStorageVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    get_class_id: unsafe extern "system" fn(*mut c_void, *mut GUID) -> i32,
    is_dirty: unsafe extern "system" fn(*mut c_void) -> i32,
    init_new: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    load: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    save: unsafe extern "system" fn(*mut c_void, *mut c_void, i32) -> i32,
    save_completed: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    hands_off_storage: unsafe extern "system" fn(*mut c_void) -> i32,
}

static PERSIST_STORAGE_VTBL: PersistStorageVtbl = PersistStorageVtbl {
    query_interface: part_query_interface::<PersistStorageVtbl>,
    add_ref: part_add_ref::<PersistStorageVtbl>,
    release: part_release::<PersistStorageVtbl>,
    get_class_id: persist_storage_get_class_id,
    is_dirty: persist_storage_is_dirty,
    init_new: persist_storage_init_new,
    load: persist_storage_load,
    save: persist_storage_save,
    save_completed: persist_storage_save_completed,
    hands_off_storage: persist_storage_hands_off_storage,
};

unsafe extern "system" fn persist_storage_get_class_id(
    _this: *mut c_void,
    class_id: *mut GUID,
) -> i32 {
    if class_id.is_null() {
        return E_POINTER;
    }
    *class_id = CLSID_CHEMCORE_DOCUMENT;
    S_OK
}

unsafe extern "system" fn persist_storage_is_dirty(_this: *mut c_void) -> i32 {
    S_FALSE
}

unsafe extern "system" fn persist_storage_init_new(this: *mut c_void, storage: *mut c_void) -> i32 {
    let object = owner_from_part::<PersistStorageVtbl>(this);
    if object.is_null() || storage.is_null() {
        return E_POINTER;
    }
    (*object).storage = storage;
    write_ole_storage_payload(storage, &(*object).payload)
}

unsafe extern "system" fn persist_storage_load(this: *mut c_void, storage: *mut c_void) -> i32 {
    let object = owner_from_part::<PersistStorageVtbl>(this);
    if object.is_null() || storage.is_null() {
        return E_POINTER;
    }
    (*object).storage = storage;
    if let Ok(document) = storage_read_stream(storage, OLE_STREAM_DOCUMENT).and_then(|bytes| {
        String::from_utf8(bytes)
            .map_err(|error| format!("ChemcoreDocument stream is not UTF-8: {error}"))
    }) {
        (*object).payload.chemcore_document_json = document;
    }
    if let Ok(svg) = storage_read_stream(storage, OLE_STREAM_PREVIEW_SVG).and_then(|bytes| {
        String::from_utf8(bytes)
            .map_err(|error| format!("ChemcorePreviewSvg stream is not UTF-8: {error}"))
    }) {
        (*object).payload.svg = svg;
    }
    S_OK
}

unsafe extern "system" fn persist_storage_save(
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
    write_ole_storage_payload(storage, &(*object).payload)
}

unsafe extern "system" fn persist_storage_save_completed(
    this: *mut c_void,
    storage: *mut c_void,
) -> i32 {
    let object = owner_from_part::<PersistStorageVtbl>(this);
    if !object.is_null() {
        (*object).storage = storage;
    }
    S_OK
}

unsafe extern "system" fn persist_storage_hands_off_storage(this: *mut c_void) -> i32 {
    let object = owner_from_part::<PersistStorageVtbl>(this);
    if !object.is_null() {
        (*object).storage = null_mut();
    }
    S_OK
}

#[repr(C)]
struct StorageVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    create_stream:
        unsafe extern "system" fn(*mut c_void, *const u16, u32, u32, u32, *mut *mut c_void) -> i32,
    open_stream: unsafe extern "system" fn(
        *mut c_void,
        *const u16,
        *mut c_void,
        u32,
        u32,
        *mut *mut c_void,
    ) -> i32,
    create_storage:
        unsafe extern "system" fn(*mut c_void, *const u16, u32, u32, u32, *mut *mut c_void) -> i32,
    open_storage: unsafe extern "system" fn(
        *mut c_void,
        *const u16,
        *mut c_void,
        u32,
        *mut *mut u16,
        u32,
        *mut *mut c_void,
    ) -> i32,
    copy_to:
        unsafe extern "system" fn(*mut c_void, u32, *const GUID, *mut *mut u16, *mut c_void) -> i32,
    move_element_to:
        unsafe extern "system" fn(*mut c_void, *const u16, *mut c_void, *const u16, u32) -> i32,
    commit: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    revert: unsafe extern "system" fn(*mut c_void) -> i32,
    enum_elements:
        unsafe extern "system" fn(*mut c_void, u32, *mut c_void, u32, *mut *mut c_void) -> i32,
    destroy_element: unsafe extern "system" fn(*mut c_void, *const u16) -> i32,
    rename_element: unsafe extern "system" fn(*mut c_void, *const u16, *const u16) -> i32,
    set_element_times: unsafe extern "system" fn(
        *mut c_void,
        *const u16,
        *const c_void,
        *const c_void,
        *const c_void,
    ) -> i32,
    set_class: unsafe extern "system" fn(*mut c_void, *const GUID) -> i32,
    set_state_bits: unsafe extern "system" fn(*mut c_void, u32, u32) -> i32,
    stat: unsafe extern "system" fn(*mut c_void, *mut STATSTG, u32) -> i32,
}

#[repr(C)]
struct StreamVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    read: unsafe extern "system" fn(*mut c_void, *mut c_void, u32, *mut u32) -> i32,
    write: unsafe extern "system" fn(*mut c_void, *const c_void, u32, *mut u32) -> i32,
    seek: unsafe extern "system" fn(*mut c_void, i64, u32, *mut u64) -> i32,
    set_size: unsafe extern "system" fn(*mut c_void, u64) -> i32,
    copy_to: unsafe extern "system" fn(*mut c_void, *mut c_void, u64, *mut u64, *mut u64) -> i32,
    commit: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    revert: unsafe extern "system" fn(*mut c_void) -> i32,
    lock_region: unsafe extern "system" fn(*mut c_void, u64, u64, u32) -> i32,
    unlock_region: unsafe extern "system" fn(*mut c_void, u64, u64, u32) -> i32,
    stat: unsafe extern "system" fn(*mut c_void, *mut STATSTG, u32) -> i32,
    clone: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32,
}

unsafe fn write_ole_storage_payload(storage: *mut c_void, payload: &OleObjectPayload) -> i32 {
    if storage.is_null() {
        return E_POINTER;
    }
    let hr = WriteClassStg(storage, &CLSID_CHEMCORE_DOCUMENT);
    if !hresult_succeeded(hr) {
        return hr;
    }

    let document = chemcore_document_stream_payload(payload);
    let manifest = match ole_manifest_stream_payload() {
        Ok(manifest) => manifest,
        Err(hr) => return hr,
    };
    let preview = payload.svg.as_bytes();

    let streams = [
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

    storage_commit(storage)
}

fn chemcore_document_stream_payload(payload: &OleObjectPayload) -> Vec<u8> {
    payload.chemcore_document_json.as_bytes().to_vec()
}

unsafe fn create_ole_storage_medium(payload: &OleObjectPayload, medium: *mut STGMEDIUM) -> i32 {
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

    let hr = write_ole_storage_payload(storage, payload);
    if !hresult_succeeded(hr) {
        com_release(storage);
        return hr;
    }

    (*medium).tymed = TYMED_ISTORAGE as u32;
    (*medium).u.pstg = storage;
    (*medium).pUnkForRelease = null_mut();
    S_OK
}

fn hglobal_for_bytes(bytes: &[u8]) -> Result<HGLOBAL, i32> {
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

fn hglobal_for_utf8_nul(value: &str) -> Result<HGLOBAL, i32> {
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(0);
    hglobal_for_bytes(&bytes)
}

fn hglobal_for_utf16_nul(value: &str) -> Result<HGLOBAL, i32> {
    let wide = wide_null(value);
    let bytes = unsafe {
        std::slice::from_raw_parts(
            wide.as_ptr().cast::<u8>(),
            wide.len() * std::mem::size_of::<u16>(),
        )
    };
    hglobal_for_bytes(bytes)
}

fn hglobal_for_object_descriptor(extent: SIZE) -> Result<HGLOBAL, i32> {
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
        (*descriptor).cbSize = total as u32;
        (*descriptor).clsid = CLSID_CHEMCORE_DOCUMENT;
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

fn hglobal_medium(handle: HGLOBAL, medium: *mut STGMEDIUM) -> i32 {
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

fn hglobal_text_medium(value: &str, unicode: bool, medium: *mut STGMEDIUM) -> i32 {
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

fn clipboard_format(name: &str) -> u16 {
    unsafe { RegisterClipboardFormatW(wide_null(name).as_ptr()) as u16 }
}

fn default_misc_status() -> u32 {
    (OLEMISC_INSIDEOUT
        | OLEMISC_ACTIVATEWHENVISIBLE
        | OLEMISC_RENDERINGISDEVICEINDEPENDENT
        | OLEMISC_SETCLIENTSITEFIRST) as u32
}

fn ole_clipboard_formats(payload: &OleObjectPayload, _extent: SIZE) -> Vec<FORMATETC> {
    let mut formats = Vec::new();
    push_format(
        &mut formats,
        clipboard_format(CLIPBOARD_FORMAT_EMBED_SOURCE),
        TYMED_ISTORAGE as u32,
    );
    push_format(
        &mut formats,
        clipboard_format(CLIPBOARD_FORMAT_OBJECT_DESCRIPTOR),
        TYMED_HGLOBAL as u32,
    );
    if payload.chemcore_fragment_json.is_some() {
        push_format(
            &mut formats,
            clipboard_format(FORMAT_CHEMCORE_FRAGMENT),
            TYMED_HGLOBAL as u32,
        );
    }
    push_format(
        &mut formats,
        clipboard_format(FORMAT_CHEMCORE_DOCUMENT_JSON),
        TYMED_HGLOBAL as u32,
    );
    if payload.cdxml.is_some() {
        push_format(
            &mut formats,
            clipboard_format(FORMAT_CHEMDRAW_INTERCHANGE),
            TYMED_HGLOBAL as u32,
        );
        push_format(
            &mut formats,
            clipboard_format(FORMAT_CDXML_MIME),
            TYMED_HGLOBAL as u32,
        );
    }
    if !payload.svg.is_empty() {
        push_format(
            &mut formats,
            clipboard_format(FORMAT_SVG_MIME),
            TYMED_HGLOBAL as u32,
        );
        push_format(
            &mut formats,
            clipboard_format(FORMAT_SVG),
            TYMED_HGLOBAL as u32,
        );
    }
    if payload.text.is_some() {
        push_format(&mut formats, CF_UNICODETEXT, TYMED_HGLOBAL as u32);
    }

    formats.retain(|format| format.cfFormat != 0);
    formats
}

fn push_format(formats: &mut Vec<FORMATETC>, cf_format: u16, tymed: u32) {
    if cf_format == 0 {
        return;
    }
    formats.push(FORMATETC {
        cfFormat: cf_format,
        ptd: null_mut(),
        dwAspect: DVASPECT_CONTENT,
        lindex: -1,
        tymed,
    });
}

unsafe fn clipboard_format_supported(
    payload: &OleObjectPayload,
    extent: SIZE,
    requested: &FORMATETC,
) -> bool {
    ole_clipboard_formats(payload, extent)
        .into_iter()
        .any(|available| {
            available.cfFormat == requested.cfFormat
                && (requested.dwAspect == 0 || requested.dwAspect == DVASPECT_CONTENT)
                && (available.tymed & requested.tymed) != 0
        })
}

unsafe fn write_clipboard_format_to_medium(
    payload: &OleObjectPayload,
    extent: SIZE,
    format: &FORMATETC,
    medium: *mut STGMEDIUM,
) -> i32 {
    if medium.is_null() {
        return E_POINTER;
    }
    if format.cfFormat == clipboard_format(CLIPBOARD_FORMAT_EMBED_SOURCE) {
        if (format.tymed & TYMED_ISTORAGE as u32) == 0 {
            return DV_E_TYMED;
        }
        return create_ole_storage_medium(payload, medium);
    }
    if (format.tymed & TYMED_HGLOBAL as u32) == 0 {
        return DV_E_TYMED;
    }

    if format.cfFormat == clipboard_format(CLIPBOARD_FORMAT_OBJECT_DESCRIPTOR) {
        return match hglobal_for_object_descriptor(extent) {
            Ok(handle) => hglobal_medium(handle, medium),
            Err(hr) => hr,
        };
    }
    if format.cfFormat == clipboard_format(FORMAT_CHEMCORE_FRAGMENT) {
        return payload
            .chemcore_fragment_json
            .as_deref()
            .map(|value| hglobal_text_medium(value, false, medium))
            .unwrap_or(DV_E_FORMATETC);
    }
    if format.cfFormat == clipboard_format(FORMAT_CHEMCORE_DOCUMENT_JSON) {
        return hglobal_text_medium(&payload.chemcore_document_json, false, medium);
    }
    if format.cfFormat == clipboard_format(FORMAT_CHEMDRAW_INTERCHANGE)
        || format.cfFormat == clipboard_format(FORMAT_CDXML_MIME)
    {
        return payload
            .cdxml
            .as_deref()
            .map(|value| hglobal_text_medium(value, false, medium))
            .unwrap_or(DV_E_FORMATETC);
    }
    if format.cfFormat == clipboard_format(FORMAT_SVG_MIME)
        || format.cfFormat == clipboard_format(FORMAT_SVG)
    {
        return hglobal_text_medium(&payload.svg, false, medium);
    }
    if format.cfFormat == CF_UNICODETEXT {
        return payload
            .text
            .as_deref()
            .map(|value| hglobal_text_medium(value, true, medium))
            .unwrap_or(DV_E_FORMATETC);
    }

    DV_E_FORMATETC
}

fn ole_preview_svg_stream_payload() -> Vec<u8> {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="240" height="120" viewBox="0 0 240 120"><rect width="240" height="120" fill="#ffffff"/><path d="M56 68h128" stroke="#111827" stroke-width="4" stroke-linecap="round"/><circle cx="56" cy="68" r="7" fill="#111827"/><circle cx="184" cy="68" r="7" fill="#111827"/><text x="120" y="32" text-anchor="middle" font-family="Arial, sans-serif" font-size="16" fill="#111827">{DOCUMENT_DISPLAY_NAME}</text></svg>"##
    )
    .into_bytes()
}

fn ole_manifest_stream_payload() -> Result<Vec<u8>, i32> {
    serde_json::to_vec(&serde_json::json!({
        "format": "chemcore-ole-object",
        "version": 1,
        "classId": CLSID_STRING,
        "progId": PROG_ID,
        "documentStream": OLE_STREAM_DOCUMENT,
        "previewSvgStream": OLE_STREAM_PREVIEW_SVG
    }))
    .map_err(|_| E_FAIL)
}

unsafe fn storage_write_stream(storage: *mut c_void, name: &str, bytes: &[u8]) -> i32 {
    let mut stream = null_mut();
    let name_w = wide_null(name);
    let storage_vtbl = *(storage.cast::<*const StorageVtbl>());
    let hr = ((*storage_vtbl).create_stream)(
        storage,
        name_w.as_ptr(),
        STGM_CREATE | STGM_READWRITE | STGM_SHARE_EXCLUSIVE,
        0,
        0,
        &mut stream,
    );
    if !hresult_succeeded(hr) || stream.is_null() {
        return hr;
    }

    let hr = stream_write_all(stream, bytes);
    let commit_hr = if hresult_succeeded(hr) {
        stream_commit(stream)
    } else {
        hr
    };
    com_release(stream);
    commit_hr
}

unsafe fn storage_read_stream(storage: *mut c_void, name: &str) -> Result<Vec<u8>, String> {
    let mut stream = null_mut();
    let name_w = wide_null(name);
    let storage_vtbl = *(storage.cast::<*const StorageVtbl>());
    let hr = ((*storage_vtbl).open_stream)(
        storage,
        name_w.as_ptr(),
        null_mut(),
        STGM_READ | STGM_SHARE_EXCLUSIVE,
        0,
        &mut stream,
    );
    if !hresult_succeeded(hr) || stream.is_null() {
        return Err(format!(
            "IStorage::OpenStream({name}) failed: 0x{:08X}",
            hr as u32
        ));
    }
    let result = stream_read_all(stream).map_err(|hr| {
        format!(
            "IStream::Read for stream {name} failed: 0x{:08X}",
            hr as u32
        )
    });
    com_release(stream);
    result
}

unsafe fn storage_commit(storage: *mut c_void) -> i32 {
    let storage_vtbl = *(storage.cast::<*const StorageVtbl>());
    ((*storage_vtbl).commit)(storage, STGC_DEFAULT)
}

unsafe fn stream_write_all(stream: *mut c_void, bytes: &[u8]) -> i32 {
    let stream_vtbl = *(stream.cast::<*const StreamVtbl>());
    let mut offset = 0usize;
    while offset < bytes.len() {
        let remaining = (bytes.len() - offset).min(u32::MAX as usize) as u32;
        let mut written = 0;
        let hr = ((*stream_vtbl).write)(
            stream,
            bytes[offset..].as_ptr().cast::<c_void>(),
            remaining,
            &mut written,
        );
        if !hresult_succeeded(hr) {
            return hr;
        }
        if written == 0 {
            return E_FAIL;
        }
        offset += written as usize;
    }
    S_OK
}

unsafe fn stream_read_all(stream: *mut c_void) -> Result<Vec<u8>, i32> {
    let stream_vtbl = *(stream.cast::<*const StreamVtbl>());
    let mut out = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let mut read = 0;
        let hr = ((*stream_vtbl).read)(
            stream,
            buffer.as_mut_ptr().cast::<c_void>(),
            buffer.len() as u32,
            &mut read,
        );
        if !hresult_succeeded(hr) {
            return Err(hr);
        }
        if read == 0 {
            break;
        }
        out.extend_from_slice(&buffer[..read as usize]);
        if read < buffer.len() as u32 {
            break;
        }
    }
    Ok(out)
}

unsafe fn stream_commit(stream: *mut c_void) -> i32 {
    let stream_vtbl = *(stream.cast::<*const StreamVtbl>());
    ((*stream_vtbl).commit)(stream, STGC_DEFAULT)
}

#[repr(C)]
struct OleObjectVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    set_client_site: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    get_client_site: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32,
    set_host_names: unsafe extern "system" fn(*mut c_void, *const u16, *const u16) -> i32,
    close: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    set_moniker: unsafe extern "system" fn(*mut c_void, u32, *mut c_void) -> i32,
    get_moniker: unsafe extern "system" fn(*mut c_void, u32, u32, *mut *mut c_void) -> i32,
    init_from_data: unsafe extern "system" fn(*mut c_void, *mut c_void, i32, u32) -> i32,
    get_clipboard_data: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> i32,
    do_verb: unsafe extern "system" fn(
        *mut c_void,
        i32,
        *mut c_void,
        *mut c_void,
        i32,
        isize,
        *const c_void,
    ) -> i32,
    enum_verbs: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32,
    update: unsafe extern "system" fn(*mut c_void) -> i32,
    is_up_to_date: unsafe extern "system" fn(*mut c_void) -> i32,
    get_user_class_id: unsafe extern "system" fn(*mut c_void, *mut GUID) -> i32,
    get_user_type: unsafe extern "system" fn(*mut c_void, u32, *mut *mut u16) -> i32,
    set_extent: unsafe extern "system" fn(*mut c_void, u32, *const c_void) -> i32,
    get_extent: unsafe extern "system" fn(*mut c_void, u32, *mut c_void) -> i32,
    advise: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut u32) -> i32,
    unadvise: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    enum_advise: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32,
    get_misc_status: unsafe extern "system" fn(*mut c_void, u32, *mut u32) -> i32,
    set_color_scheme: unsafe extern "system" fn(*mut c_void, *const c_void) -> i32,
}

static OLE_OBJECT_VTBL: OleObjectVtbl = OleObjectVtbl {
    query_interface: part_query_interface::<OleObjectVtbl>,
    add_ref: part_add_ref::<OleObjectVtbl>,
    release: part_release::<OleObjectVtbl>,
    set_client_site: ole_object_set_client_site,
    get_client_site: ole_object_get_client_site,
    set_host_names: ole_object_set_host_names,
    close: ole_object_close,
    set_moniker: ole_object_set_moniker,
    get_moniker: ole_object_get_moniker,
    init_from_data: ole_object_init_from_data,
    get_clipboard_data: ole_object_get_clipboard_data,
    do_verb: ole_object_do_verb,
    enum_verbs: ole_object_enum_verbs,
    update: ole_object_update,
    is_up_to_date: ole_object_is_up_to_date,
    get_user_class_id: ole_object_get_user_class_id,
    get_user_type: ole_object_get_user_type,
    set_extent: ole_object_set_extent,
    get_extent: ole_object_get_extent,
    advise: ole_object_advise,
    unadvise: ole_object_unadvise,
    enum_advise: ole_object_enum_advise,
    get_misc_status: ole_object_get_misc_status,
    set_color_scheme: ole_object_set_color_scheme,
};

unsafe extern "system" fn ole_object_set_client_site(this: *mut c_void, site: *mut c_void) -> i32 {
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    com_release((*object).client_site);
    (*object).client_site = site;
    com_add_ref(site);
    S_OK
}

unsafe extern "system" fn ole_object_get_client_site(
    this: *mut c_void,
    site: *mut *mut c_void,
) -> i32 {
    if site.is_null() {
        return E_POINTER;
    }
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() {
        *site = null_mut();
        return E_POINTER;
    }
    *site = (*object).client_site;
    com_add_ref(*site);
    S_OK
}

unsafe extern "system" fn ole_object_set_host_names(
    _this: *mut c_void,
    _container_app: *const u16,
    _container_object: *const u16,
) -> i32 {
    S_OK
}

unsafe extern "system" fn ole_object_close(_this: *mut c_void, _save_option: i32) -> i32 {
    S_OK
}

unsafe extern "system" fn ole_object_set_moniker(
    _this: *mut c_void,
    _which_moniker: u32,
    _moniker: *mut c_void,
) -> i32 {
    E_NOTIMPL
}

unsafe extern "system" fn ole_object_get_moniker(
    _this: *mut c_void,
    _assign: u32,
    _which_moniker: u32,
    moniker: *mut *mut c_void,
) -> i32 {
    if !moniker.is_null() {
        *moniker = null_mut();
    }
    E_NOTIMPL
}

unsafe extern "system" fn ole_object_init_from_data(
    _this: *mut c_void,
    _data_object: *mut c_void,
    _creation: i32,
    _reserved: u32,
) -> i32 {
    E_NOTIMPL
}

unsafe extern "system" fn ole_object_get_clipboard_data(
    _this: *mut c_void,
    _reserved: u32,
    data_object: *mut *mut c_void,
) -> i32 {
    if !data_object.is_null() {
        *data_object = null_mut();
    }
    E_NOTIMPL
}

unsafe extern "system" fn ole_object_do_verb(
    this: *mut c_void,
    _verb: i32,
    _message: *mut c_void,
    _active_site: *mut c_void,
    _index: i32,
    _parent: isize,
    _position: *const c_void,
) -> i32 {
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    launch_desktop_for_payload(&(*object).payload)
        .map(|_| S_OK)
        .unwrap_or(OLE_E_NOTRUNNING)
}

unsafe extern "system" fn ole_object_enum_verbs(
    _this: *mut c_void,
    enum_verbs: *mut *mut c_void,
) -> i32 {
    if enum_verbs.is_null() {
        return E_POINTER;
    }
    *enum_verbs = null_mut();
    OleRegEnumVerbs(&CLSID_CHEMCORE_DOCUMENT, enum_verbs)
}

unsafe extern "system" fn ole_object_update(_this: *mut c_void) -> i32 {
    S_OK
}

unsafe extern "system" fn ole_object_is_up_to_date(_this: *mut c_void) -> i32 {
    S_OK
}

unsafe extern "system" fn ole_object_get_user_class_id(
    _this: *mut c_void,
    class_id: *mut GUID,
) -> i32 {
    if class_id.is_null() {
        return E_POINTER;
    }
    *class_id = CLSID_CHEMCORE_DOCUMENT;
    S_OK
}

unsafe extern "system" fn ole_object_get_user_type(
    _this: *mut c_void,
    form: u32,
    user_type: *mut *mut u16,
) -> i32 {
    if user_type.is_null() {
        return E_POINTER;
    }
    *user_type = null_mut();
    let hr = OleRegGetUserType(&CLSID_CHEMCORE_DOCUMENT, form, user_type);
    if hresult_succeeded(hr) {
        return hr;
    }
    allocate_com_string(DOCUMENT_DISPLAY_NAME, user_type)
}

unsafe extern "system" fn ole_object_set_extent(
    this: *mut c_void,
    _draw_aspect: u32,
    size: *const c_void,
) -> i32 {
    if size.is_null() {
        return E_POINTER;
    }
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    (*object).extent_himetric = *(size.cast::<SIZE>());
    S_OK
}

unsafe extern "system" fn ole_object_get_extent(
    this: *mut c_void,
    _draw_aspect: u32,
    size: *mut c_void,
) -> i32 {
    if size.is_null() {
        return E_POINTER;
    }
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    *(size.cast::<SIZE>()) = (*object).extent_himetric;
    S_OK
}

unsafe extern "system" fn ole_object_advise(
    _this: *mut c_void,
    _sink: *mut c_void,
    connection: *mut u32,
) -> i32 {
    if !connection.is_null() {
        *connection = 0;
    }
    E_NOTIMPL
}

unsafe extern "system" fn ole_object_unadvise(_this: *mut c_void, _connection: u32) -> i32 {
    E_NOTIMPL
}

unsafe extern "system" fn ole_object_enum_advise(
    _this: *mut c_void,
    enum_advise: *mut *mut c_void,
) -> i32 {
    if !enum_advise.is_null() {
        *enum_advise = null_mut();
    }
    E_NOTIMPL
}

unsafe extern "system" fn ole_object_get_misc_status(
    _this: *mut c_void,
    draw_aspect: u32,
    status: *mut u32,
) -> i32 {
    if status.is_null() {
        return E_POINTER;
    }
    let hr = OleRegGetMiscStatus(&CLSID_CHEMCORE_DOCUMENT, draw_aspect, status);
    if hresult_succeeded(hr) {
        return hr;
    }
    *status = default_misc_status();
    S_OK
}

unsafe extern "system" fn ole_object_set_color_scheme(
    _this: *mut c_void,
    _palette_log: *const c_void,
) -> i32 {
    S_OK
}

#[repr(C)]
struct ViewObject2Vtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    draw: unsafe extern "system" fn(
        *mut c_void,
        u32,
        i32,
        *mut c_void,
        *mut c_void,
        isize,
        isize,
        *const c_void,
        *const c_void,
        *mut c_void,
        usize,
    ) -> i32,
    get_color_set: unsafe extern "system" fn(
        *mut c_void,
        u32,
        i32,
        *mut c_void,
        *mut c_void,
        isize,
        *mut *mut c_void,
    ) -> i32,
    freeze: unsafe extern "system" fn(*mut c_void, u32, i32, *mut c_void, *mut u32) -> i32,
    unfreeze: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    set_advise: unsafe extern "system" fn(*mut c_void, u32, u32, *mut c_void) -> i32,
    get_advise: unsafe extern "system" fn(*mut c_void, *mut u32, *mut u32, *mut *mut c_void) -> i32,
    get_extent: unsafe extern "system" fn(*mut c_void, u32, i32, *mut c_void, *mut c_void) -> i32,
}

static VIEW_OBJECT2_VTBL: ViewObject2Vtbl = ViewObject2Vtbl {
    query_interface: part_query_interface::<ViewObject2Vtbl>,
    add_ref: part_add_ref::<ViewObject2Vtbl>,
    release: part_release::<ViewObject2Vtbl>,
    draw: view_object_draw,
    get_color_set: view_object_get_color_set,
    freeze: view_object_freeze,
    unfreeze: view_object_unfreeze,
    set_advise: view_object_set_advise,
    get_advise: view_object_get_advise,
    get_extent: view_object_get_extent,
};

unsafe extern "system" fn view_object_draw(
    this: *mut c_void,
    draw_aspect: u32,
    _index: i32,
    _aspect: *mut c_void,
    _target_device: *mut c_void,
    _target_dc: isize,
    _draw_dc: isize,
    bounds: *const c_void,
    _window_bounds: *const c_void,
    _continue_fn: *mut c_void,
    _continue_value: usize,
) -> i32 {
    if draw_aspect != DVASPECT_CONTENT {
        return DV_E_FORMATETC;
    }
    if _draw_dc == 0 || bounds.is_null() {
        return E_POINTER;
    }
    let object = owner_from_part::<ViewObject2Vtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    draw_placeholder_preview(_draw_dc as HDC, &*(bounds.cast::<RECT>()));
    S_OK
}

unsafe extern "system" fn view_object_get_color_set(
    _this: *mut c_void,
    _draw_aspect: u32,
    _index: i32,
    _aspect: *mut c_void,
    _target_device: *mut c_void,
    _target_dc: isize,
    color_set: *mut *mut c_void,
) -> i32 {
    if !color_set.is_null() {
        *color_set = null_mut();
    }
    E_NOTIMPL
}

unsafe extern "system" fn view_object_freeze(
    _this: *mut c_void,
    _draw_aspect: u32,
    _index: i32,
    _aspect: *mut c_void,
    freeze_key: *mut u32,
) -> i32 {
    if !freeze_key.is_null() {
        *freeze_key = 0;
    }
    E_NOTIMPL
}

unsafe extern "system" fn view_object_unfreeze(_this: *mut c_void, _freeze_key: u32) -> i32 {
    E_NOTIMPL
}

unsafe extern "system" fn view_object_set_advise(
    _this: *mut c_void,
    _aspects: u32,
    _advf: u32,
    _sink: *mut c_void,
) -> i32 {
    S_OK
}

unsafe extern "system" fn view_object_get_advise(
    _this: *mut c_void,
    aspects: *mut u32,
    advf: *mut u32,
    sink: *mut *mut c_void,
) -> i32 {
    if !aspects.is_null() {
        *aspects = DVASPECT_CONTENT;
    }
    if !advf.is_null() {
        *advf = 0;
    }
    if !sink.is_null() {
        *sink = null_mut();
    }
    S_OK
}

unsafe extern "system" fn view_object_get_extent(
    this: *mut c_void,
    _draw_aspect: u32,
    _index: i32,
    _target_device: *mut c_void,
    size: *mut c_void,
) -> i32 {
    if size.is_null() {
        return E_POINTER;
    }
    let object = owner_from_part::<ViewObject2Vtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    *(size.cast::<SIZE>()) = (*object).extent_himetric;
    S_OK
}

#[repr(C)]
struct RunnableObjectVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    get_running_class: unsafe extern "system" fn(*mut c_void, *mut GUID) -> i32,
    run: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    is_running: unsafe extern "system" fn(*mut c_void) -> i32,
    lock_running: unsafe extern "system" fn(*mut c_void, i32, i32) -> i32,
    set_contained_object: unsafe extern "system" fn(*mut c_void, i32) -> i32,
}

static RUNNABLE_OBJECT_VTBL: RunnableObjectVtbl = RunnableObjectVtbl {
    query_interface: part_query_interface::<RunnableObjectVtbl>,
    add_ref: part_add_ref::<RunnableObjectVtbl>,
    release: part_release::<RunnableObjectVtbl>,
    get_running_class: runnable_object_get_running_class,
    run: runnable_object_run,
    is_running: runnable_object_is_running,
    lock_running: runnable_object_lock_running,
    set_contained_object: runnable_object_set_contained_object,
};

unsafe extern "system" fn runnable_object_get_running_class(
    _this: *mut c_void,
    class_id: *mut GUID,
) -> i32 {
    if class_id.is_null() {
        return E_POINTER;
    }
    *class_id = CLSID_CHEMCORE_DOCUMENT;
    S_OK
}

unsafe extern "system" fn runnable_object_run(
    _this: *mut c_void,
    _bind_context: *mut c_void,
) -> i32 {
    S_OK
}

unsafe extern "system" fn runnable_object_is_running(_this: *mut c_void) -> i32 {
    S_OK
}

unsafe extern "system" fn runnable_object_lock_running(
    _this: *mut c_void,
    _lock: i32,
    _last_unlock_closes: i32,
) -> i32 {
    S_OK
}

unsafe extern "system" fn runnable_object_set_contained_object(
    _this: *mut c_void,
    _contained: i32,
) -> i32 {
    S_OK
}

unsafe fn com_add_ref(interface: *mut c_void) {
    if interface.is_null() {
        return;
    }
    let vtbl = *(interface.cast::<*const UnknownVtbl>());
    ((*vtbl).add_ref)(interface);
}

unsafe fn com_release(interface: *mut c_void) {
    if interface.is_null() {
        return;
    }
    let vtbl = *(interface.cast::<*const UnknownVtbl>());
    ((*vtbl).release)(interface);
}

fn launch_desktop_for_payload(payload: &OleObjectPayload) -> Result<(), String> {
    let payload_path = env::temp_dir().join(format!(
        "chemcore-ole-edit-{}-{}.json",
        std::process::id(),
        monotonic_millis()
    ));
    std::fs::write(&payload_path, &payload.chemcore_document_json)
        .map_err(|error| format!("Failed to write temporary OLE edit payload: {error}"))?;

    let desktop_exe = current_server_path()?.with_file_name("chemcore-desktop.exe");
    if !desktop_exe.exists() {
        return Err(format!(
            "Chemcore desktop executable was not found at {}",
            desktop_exe.display()
        ));
    }

    Command::new(desktop_exe)
        .arg(payload_path)
        .spawn()
        .map_err(|error| format!("Failed to launch Chemcore desktop: {error}"))?;
    Ok(())
}

fn monotonic_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

unsafe fn draw_placeholder_preview(dc: HDC, bounds: &RECT) {
    let width = (bounds.right - bounds.left).max(1);
    let height = (bounds.bottom - bounds.top).max(1);
    let old_brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
    let pen = CreatePen(PS_SOLID, (width.min(height) / 40).max(1), 0x000000);
    let old_pen = SelectObject(dc, pen as HGDIOBJ);

    Rectangle(dc, bounds.left, bounds.top, bounds.right, bounds.bottom);

    let mid_y = bounds.top + height * 58 / 100;
    let left_x = bounds.left + width * 24 / 100;
    let right_x = bounds.left + width * 76 / 100;
    MoveToEx(dc, left_x, mid_y, null_mut());
    LineTo(dc, right_x, mid_y);
    let radius = (width.min(height) / 20).max(3);
    Ellipse(
        dc,
        left_x - radius,
        mid_y - radius,
        left_x + radius,
        mid_y + radius,
    );
    Ellipse(
        dc,
        right_x - radius,
        mid_y - radius,
        right_x + radius,
        mid_y + radius,
    );

    SetBkMode(dc, TRANSPARENT as i32);
    let label = wide_null(DOCUMENT_DISPLAY_NAME);
    TextOutW(
        dc,
        bounds.left + width * 30 / 100,
        bounds.top + height * 18 / 100,
        label.as_ptr(),
        (label.len().saturating_sub(1)) as i32,
    );

    SelectObject(dc, old_pen);
    SelectObject(dc, old_brush);
    DeleteObject(pen as HGDIOBJ);
}

unsafe fn allocate_com_string(value: &str, out: *mut *mut u16) -> i32 {
    if out.is_null() {
        return E_POINTER;
    }
    let wide = wide_null(value);
    let bytes = wide.len() * std::mem::size_of::<u16>();
    let ptr = CoTaskMemAlloc(bytes).cast::<u16>();
    if ptr.is_null() {
        *out = null_mut();
        return E_OUTOFMEMORY;
    }
    std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
    *out = ptr;
    S_OK
}

fn guid_eq(left: &GUID, right: &GUID) -> bool {
    left.data1 == right.data1
        && left.data2 == right.data2
        && left.data3 == right.data3
        && left.data4 == right.data4
}

fn hresult_succeeded(hr: i32) -> bool {
    hr >= 0
}

fn print_help() {
    println!("{APP_NAME} Office/OLE integration server");
    println!();
    println!("Usage:");
    println!("  chemcore-office.exe --register-user");
    println!("  chemcore-office.exe --unregister-user");
    println!("  chemcore-office.exe --register-machine");
    println!("  chemcore-office.exe --unregister-machine");
    println!("  chemcore-office.exe --print-registration");
    println!("  chemcore-office.exe --self-test");
    println!("  chemcore-office.exe --copy-clipboard-payload <payload.json>");
    println!("  chemcore-office.exe --serve");
    println!();
    println!("COM may launch this executable with -Embedding or /Embedding.");
}
