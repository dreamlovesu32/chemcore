use std::collections::BTreeMap;
use std::env;
use std::ffi::c_void;
use std::io::{Cursor, Read, Write};
use std::mem::zeroed;
use std::net::{SocketAddr, TcpStream};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime};

use chemcore_engine::PT_PER_CM;
use windows_sys::core::GUID;
use windows_sys::Win32::Foundation::{
    GetLastError, GlobalFree, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, HGLOBAL, POINTL, RECT, SIZE,
};
use windows_sys::Win32::Graphics::Gdi::{DeleteEnhMetaFile, HDC};
use windows_sys::Win32::System::Com::StructuredStorage::{
    CreateILockBytesOnHGlobal, StgCreateDocfile, StgCreateDocfileOnILockBytes, WriteClassStg,
    WriteFmtUserTypeStg,
};
use windows_sys::Win32::System::Com::{
    CoRegisterClassObject, CoRevokeClassObject, CoTaskMemAlloc, CLSCTX_LOCAL_SERVER, DATADIR_GET,
    DVASPECT_CONTENT, FORMATETC, REGCLS_MULTIPLEUSE, STATSTG, STGC_DEFAULT, STGMEDIUM, STGM_CREATE,
    STGM_READ, STGM_READWRITE, STGM_SHARE_EXCLUSIVE, TYMED_ENHMF, TYMED_HGLOBAL, TYMED_ISTORAGE,
    TYMED_MFPICT,
};
use windows_sys::Win32::System::Console::FreeConsole;
use windows_sys::Win32::System::DataExchange::RegisterClipboardFormatW;
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock};
use windows_sys::Win32::System::Ole::{
    CreateOleAdviseHolder, OleCreateFromData, OleFlushClipboard, OleInitialize, OleRegEnumVerbs,
    OleRegGetMiscStatus, OleRegGetUserType, OleSave, OleSetClipboard, OleUninitialize,
    ReleaseStgMedium, CF_ENHMETAFILE, CF_METAFILEPICT, OBJECTDESCRIPTOR,
    OLEMISC_RENDERINGISDEVICEINDEPENDENT, OLEMISC_SETCLIENTSITEFIRST, OLERENDER_FORMAT,
};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, REG_SZ,
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, DispatchMessageW, GetMessageW, KillTimer, PostThreadMessageW,
    SetTimer, TranslateMessage, MSG, SW_SHOWNORMAL,
};

static TEMP_SUFFIX_COUNTER: AtomicU64 = AtomicU64::new(1);

mod emf_preview;

use emf_preview::{
    draw_payload_preview, draw_placeholder_preview, enhanced_metafile_bits_for_payload,
    enhanced_metafile_for_payload, extent_himetric_for_payload, hglobal_for_metafile_pict,
    ole_presentation_stream_for_payload, preview_bounds_debug_report,
};
mod constants;

use constants::*;

#[link(name = "ole32")]
unsafe extern "system" {
    fn CreateDataAdviseHolder(holder: *mut *mut c_void) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentThreadId() -> u32;
}

#[derive(Debug, Clone)]
struct OleEditSession {
    path: PathBuf,
    object: usize,
    last_modified: Option<SystemTime>,
}

static OLE_EDIT_SESSIONS: OnceLock<Mutex<BTreeMap<String, OleEditSession>>> = OnceLock::new();
static OLE_EDIT_PENDING_UPDATES: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static OLE_EDIT_MAIN_THREAD_ID: OnceLock<u32> = OnceLock::new();

fn ole_edit_sessions() -> &'static Mutex<BTreeMap<String, OleEditSession>> {
    OLE_EDIT_SESSIONS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn ole_edit_pending_updates() -> &'static Mutex<Vec<String>> {
    OLE_EDIT_PENDING_UPDATES.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_default();
    log_ole_event(&format!("launch command: {command}"));
    match command.as_str() {
        "--register-user" => register(RegistrationScope::User),
        "--unregister-user" => unregister(RegistrationScope::User),
        "--register-machine" => register(RegistrationScope::Machine),
        "--unregister-machine" => unregister(RegistrationScope::Machine),
        "--print-registration" => print_registration(),
        "--self-test" => run_self_test(),
        "--copy-clipboard-payload" => {
            unsafe {
                FreeConsole();
            }
            let payload_path = args.next().ok_or_else(|| {
                "--copy-clipboard-payload requires a JSON payload path.".to_string()
            })?;
            copy_clipboard_payload(PathBuf::from(payload_path))
        }
        "--write-word-docx-payload" => {
            let payload_path = args.next().ok_or_else(|| {
                "--write-word-docx-payload requires a JSON payload path.".to_string()
            })?;
            let output_path = args.next().ok_or_else(|| {
                "--write-word-docx-payload requires an output .docx path.".to_string()
            })?;
            write_word_docx_payload(PathBuf::from(payload_path), PathBuf::from(output_path))
        }
        "--write-emf-payload" => {
            let payload_path = args
                .next()
                .ok_or_else(|| "--write-emf-payload requires a JSON payload path.".to_string())?;
            let output_path = args
                .next()
                .ok_or_else(|| "--write-emf-payload requires an output .emf path.".to_string())?;
            write_emf_payload(PathBuf::from(payload_path), PathBuf::from(output_path))
        }
        "--write-preview-bounds-payload" => {
            let payload_path = args.next().ok_or_else(|| {
                "--write-preview-bounds-payload requires a JSON payload path.".to_string()
            })?;
            let output_path = args.next().ok_or_else(|| {
                "--write-preview-bounds-payload requires an output .json path.".to_string()
            })?;
            write_preview_bounds_payload(PathBuf::from(payload_path), PathBuf::from(output_path))
        }
        "--serve" | "-Embedding" | "/Embedding" | "--embedding" => {
            unsafe {
                FreeConsole();
            }
            run_com_server()
        }
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
    set_named_string(root, &clsid_path, "AppID", CLSID_STRING)?;
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
    set_named_string(
        root,
        &format!("{clsid_path}\\LocalServer32"),
        "ServerExecutable",
        &server_path.to_string_lossy(),
    )?;
    set_key_default(root, &format!("{clsid_path}\\LocalServer"), &server_command)?;
    set_key_default(root, &format!("{clsid_path}\\InprocHandler32"), "ole32.dll")?;
    set_key_default(root, &format!("{clsid_path}\\DefaultIcon"), &icon_command)?;
    set_key_default(
        root,
        &format!("{clsid_path}\\AuxUserType\\2"),
        DOCUMENT_DISPLAY_NAME,
    )?;
    set_key_default(
        root,
        &format!("{clsid_path}\\AuxUserType\\3"),
        DOCUMENT_DISPLAY_NAME,
    )?;
    set_key_default(root, &format!("{clsid_path}\\Verb\\0"), "&Edit,0,2")?;
    set_key_default(root, &format!("{clsid_path}\\Verb\\1"), "&Open,0,2")?;
    let misc_status = default_misc_status().to_string();
    set_key_default(root, &format!("{clsid_path}\\MiscStatus"), &misc_status)?;
    set_key_default(root, &format!("{clsid_path}\\MiscStatus\\1"), &misc_status)?;
    register_data_formats(root, &clsid_path)?;
    create_key(root, &format!("{clsid_path}\\Insertable"))?;
    create_key(
        root,
        &format!("{clsid_path}\\Implemented Categories\\{{40FC6ED3-2438-11CF-A3DB-080036F12502}}"),
    )?;
    register_std_file_editing(root, PROG_ID, &server_command)?;
    register_std_file_editing(root, VERSIONED_PROG_ID, &server_command)?;

    println!(
        "Registered {DOCUMENT_DISPLAY_NAME} for {} at {}",
        scope.label(),
        scope.prefix()
    );
    println!("CLSID: {CLSID_STRING}");
    println!("Server: {}", server_path.display());
    Ok(())
}

fn register_std_file_editing(
    root: HKEY,
    prog_id: &str,
    server_command: &str,
) -> Result<(), String> {
    let std_file_editing = classes_path(&format!("{prog_id}\\Protocol\\StdFileEditing"));
    set_key_default(root, &format!("{std_file_editing}\\Server"), server_command)?;
    set_key_default(root, &format!("{std_file_editing}\\Verb\\0"), "&Edit,0,2")?;
    set_key_default(root, &format!("{std_file_editing}\\Verb\\1"), "&Open,0,2")?;
    Ok(())
}

fn register_data_formats(root: HKEY, clsid_path: &str) -> Result<(), String> {
    let data_formats = format!("{clsid_path}\\DataFormats");
    set_key_default(
        root,
        &format!("{data_formats}\\DefaultFile"),
        FORMAT_CHEMCORE_NATIVE,
    )?;
    let get_set = format!("{data_formats}\\GetSet");
    set_key_default(root, &format!("{get_set}\\0"), "14,1,64,1")?;
    set_key_default(root, &format!("{get_set}\\1"), "Embedded Object,1,8,1")?;
    set_key_default(root, &format!("{get_set}\\2"), "Embed Source,1,8,1")?;
    set_key_default(root, &format!("{get_set}\\3"), "Object Descriptor,1,1,1")?;
    set_key_default(root, &format!("{get_set}\\4"), "Native,1,1,1")?;
    set_key_default(
        root,
        &format!("{get_set}\\5"),
        &format!("{FORMAT_CHEMCORE_NATIVE},1,1,1"),
    )?;
    set_key_default(
        root,
        &format!("{get_set}\\6"),
        &format!("{FORMAT_CHEMCORE_DOCUMENT_JSON},1,1,1"),
    )?;
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
    render_list_json: Option<String>,
    cdxml: Option<String>,
    svg: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Clone)]
struct OleObjectPayload {
    chemcore_fragment_json: Option<String>,
    chemcore_document_json: String,
    render_list_json: Option<String>,
    cdxml: Option<String>,
    svg: String,
    svg_was_supplied: bool,
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
            render_list_json: None,
            cdxml: None,
            svg: String::from_utf8(ole_preview_svg_stream_payload()).unwrap_or_default(),
            svg_was_supplied: false,
            text: None,
        }
    }

    fn from_clipboard(payload: ClipboardPayload) -> Self {
        let fallback = Self::blank();
        let cdxml = payload.cdxml.filter(|value| !value.trim().is_empty());
        let supplied_svg = payload.svg.filter(|value| !value.trim().is_empty());
        let document_json = payload
            .chemcore_document_json
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(fallback.chemcore_document_json);
        let generated_svg = chemcore_engine::parse_document_json(&document_json)
            .ok()
            .map(|document| chemcore_engine::document_to_svg(&document))
            .filter(|value| !value.trim().is_empty());
        let has_preview_svg = supplied_svg.is_some() || generated_svg.is_some();
        Self {
            chemcore_fragment_json: payload.chemcore_fragment_json,
            chemcore_document_json: document_json,
            render_list_json: payload
                .render_list_json
                .filter(|value| !value.trim().is_empty()),
            cdxml: cdxml.clone(),
            svg: supplied_svg
                .clone()
                .or(generated_svg)
                .unwrap_or(fallback.svg),
            svg_was_supplied: has_preview_svg,
            text: payload
                .text
                .filter(|value| !value.trim().is_empty())
                .or(cdxml),
        }
    }

    fn extent_himetric(&self) -> SIZE {
        extent_himetric_for_payload(self).unwrap_or(SIZE {
            cx: DEFAULT_OBJECT_WIDTH_HIMETRIC,
            cy: DEFAULT_OBJECT_HEIGHT_HIMETRIC,
        })
    }
}

fn copy_clipboard_payload(payload_path: PathBuf) -> Result<(), String> {
    let payload = read_ole_object_payload(&payload_path)?;

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

        // Flush the COM data object immediately so Word can paste an owned OLE
        // payload even after this helper process exits.
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

    Ok(())
}

fn read_ole_object_payload(payload_path: &PathBuf) -> Result<OleObjectPayload, String> {
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

fn ole_edit_session_payload_json(payload: &OleObjectPayload) -> Result<String, String> {
    serde_json::to_string_pretty(&serde_json::json!({
        "chemcoreFragmentJson": payload.chemcore_fragment_json.clone(),
        "chemcoreDocumentJson": payload.chemcore_document_json.clone(),
        "renderListJson": payload.render_list_json.clone(),
        "cdxml": payload.cdxml.clone(),
        "svg": payload.svg.clone(),
        "text": payload.text.clone(),
    }))
    .map_err(|error| format!("Failed to serialize OLE edit session payload: {error}"))
}

fn ole_object_payload_from_edit_session_text(text: &str) -> Result<OleObjectPayload, String> {
    if text.trim().is_empty() {
        return Err("OLE edit session payload was empty.".into());
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
        if value.get("chemcoreDocumentJson").is_some() {
            let payload: ClipboardPayload = serde_json::from_value(value)
                .map_err(|error| format!("Invalid OLE edit session payload JSON: {error}"))?;
            if payload
                .chemcore_document_json
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                return Ok(OleObjectPayload::from_clipboard(payload));
            }
            return Err("OLE edit session payload did not contain chemcoreDocumentJson.".into());
        }
    }
    Ok(OleObjectPayload::from_clipboard(ClipboardPayload {
        chemcore_fragment_json: None,
        chemcore_document_json: Some(text.to_string()),
        render_list_json: None,
        cdxml: None,
        svg: None,
        text: None,
    }))
}

fn write_word_docx_payload(payload_path: PathBuf, output_path: PathBuf) -> Result<(), String> {
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

fn write_emf_payload(payload_path: PathBuf, output_path: PathBuf) -> Result<(), String> {
    let payload = read_ole_object_payload(&payload_path)?;
    let extent = payload.extent_himetric();
    let emf = enhanced_metafile_bits_for_payload(&payload, extent).map_err(|hr| {
        format!(
            "Failed to render EMF preview for {}: 0x{:08X}",
            payload_path.display(),
            hr as u32
        )
    })?;
    std::fs::write(&output_path, emf).map_err(|error| {
        format!(
            "Failed to write EMF preview {}: {error}",
            output_path.display()
        )
    })?;
    println!(
        "{DOCUMENT_DISPLAY_NAME} EMF preview written to {}.",
        output_path.display()
    );
    Ok(())
}

fn write_preview_bounds_payload(payload_path: PathBuf, output_path: PathBuf) -> Result<(), String> {
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
        if !document.contains("\"name\":\"chemcore\"") {
            return Err("Embedded source did not contain a Chemcore document stream.".into());
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

unsafe fn run_ole_create_from_data_self_test(data_object: *mut c_void) -> Result<(), String> {
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
        let contents = storage_read_stream(storage, OLE_STREAM_CONTENTS)?;
        let manifest = storage_read_stream(storage, OLE_STREAM_MANIFEST)?;
        let preview = storage_read_stream(storage, OLE_STREAM_PREVIEW_SVG)?;
        let presentation_emf = storage_read_stream(storage, OLE_STREAM_PRESENTATION_EMF)?;
        let enhanced_print = storage_read_stream(storage, OLE_STREAM_ENHANCED_PRINT)?;

        com_release(persist_storage);
        com_release(storage);

        let document = String::from_utf8(document)
            .map_err(|error| format!("ChemcoreDocument stream is not UTF-8: {error}"))?;
        let contents = String::from_utf8(contents)
            .map_err(|error| format!("CONTENTS stream is not UTF-8: {error}"))?;
        if !document.contains("\"name\":\"chemcore\"") || !document.contains("\"objects\"") {
            return Err("ChemcoreDocument stream did not contain a blank Chemcore document".into());
        }
        if contents != document {
            return Err("CONTENTS stream did not match ChemcoreDocument stream".into());
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

fn run_ole_edit_session_update_self_test() -> Result<(), String> {
    let storage_path = env::temp_dir().join(format!(
        "chemcore-office-edit-session-self-test-{}.ole",
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
            serde_json::from_str(&(*object).payload.chemcore_document_json).map_err(|error| {
                format!("Initial OLE document JSON self-test parse failed: {error}")
            })?;
        document_json["document"]["title"] =
            serde_json::Value::String("Chemcore OLE edit session self-test".into());
        let document_json = serde_json::to_string(&document_json).map_err(|error| {
            format!("Edited OLE document JSON self-test serialize failed: {error}")
        })?;
        let edit_payload = OleObjectPayload::from_clipboard(ClipboardPayload {
            chemcore_fragment_json: None,
            chemcore_document_json: Some(document_json),
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
            .map_err(|error| format!("Edited ChemcoreDocument stream is not UTF-8: {error}"))?;
        if !stored_document.contains("Chemcore OLE edit session self-test") {
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

fn run_word_docx_package_self_test() -> Result<(), String> {
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
    if !document_xml.contains("ProgID=\"Chemcore.Document.1\"")
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

fn set_named_string(root: HKEY, subkey: &str, name: &str, value: &str) -> Result<(), String> {
    let subkey_w = wide_null(subkey);
    let mut key: HKEY = null_mut();
    let status = unsafe { RegCreateKeyW(root, subkey_w.as_ptr(), &mut key) };
    if status != ERROR_SUCCESS {
        return Err(format!("Failed to create registry key {subkey}: {status}"));
    }

    let name_w = wide_null(name);
    let value_w = wide_null(value);
    let bytes = (value_w.len() * std::mem::size_of::<u16>()) as u32;
    let status = unsafe {
        RegSetValueExW(
            key,
            name_w.as_ptr(),
            0,
            REG_SZ,
            value_w.as_ptr().cast::<u8>(),
            bytes,
        )
    };
    unsafe {
        RegCloseKey(key);
    }
    if status != ERROR_SUCCESS {
        return Err(format!(
            "Failed to set registry value {subkey}\\{name}: {status}"
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
    let main_thread_id = unsafe { GetCurrentThreadId() };
    let _ = OLE_EDIT_MAIN_THREAD_ID.set(main_thread_id);
    log_ole_event(&format!("COM server main thread id {main_thread_id}"));
    log_ole_event("COM server initializing OLE");
    let hr = unsafe { OleInitialize(null()) };
    if !hresult_succeeded(hr) {
        log_ole_event(&format!("OleInitialize failed: 0x{:08X}", hr as u32));
        return Err(format!("OleInitialize failed: 0x{:08X}", hr as u32));
    }

    let mut registration_cookie = 0;
    log_ole_event("COM server registering class object");
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
            OleUninitialize();
        }
        log_ole_event(&format!(
            "CoRegisterClassObject failed for {CLSID_STRING}: 0x{:08X}",
            hr as u32
        ));
        return Err(format!(
            "CoRegisterClassObject failed for {CLSID_STRING}: 0x{:08X}",
            hr as u32
        ));
    }

    log_ole_event("COM server class object registered");
    unsafe {
        SetTimer(null_mut(), OLE_EDIT_TIMER_ID, OLE_EDIT_POLL_MS, None);
    }
    run_message_loop();

    log_ole_event("COM server message loop exited");
    unsafe {
        KillTimer(null_mut(), OLE_EDIT_TIMER_ID);
        CoRevokeClassObject(registration_cookie);
        OleUninitialize();
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
        if message.message == WM_TIMER && message.wParam == OLE_EDIT_TIMER_ID {
            poll_ole_edit_sessions();
            continue;
        }
        if message.message == WM_OLE_EDIT_SESSION_CHANGED {
            log_ole_event("COM server received OLE edit session change message");
            apply_pending_ole_edit_session_updates();
            poll_ole_edit_sessions();
            continue;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

fn register_ole_edit_session(
    session_id: String,
    path: PathBuf,
    object: *mut ChemcoreOleObject,
) -> Result<(), String> {
    if object.is_null() {
        return Err("OLE edit session cannot register a null object.".to_string());
    }
    let last_modified = file_modified_time(&path);
    chemcore_object_add_ref(object);
    let mut sessions = ole_edit_sessions()
        .lock()
        .map_err(|error| error.to_string())?;
    sessions.insert(
        session_id.clone(),
        OleEditSession {
            path: path.clone(),
            object: object as usize,
            last_modified,
        },
    );
    unsafe {
        SetTimer(null_mut(), OLE_EDIT_TIMER_ID, OLE_EDIT_POLL_MS, None);
    }
    start_ole_edit_file_watcher(session_id, path, last_modified);
    Ok(())
}

fn ole_edit_session_path_for_object(object: *mut ChemcoreOleObject) -> Option<PathBuf> {
    if object.is_null() {
        return None;
    }
    let object = object as usize;
    let sessions = ole_edit_sessions().lock().ok()?;
    sessions
        .values()
        .find(|session| session.object == object && session.path.exists())
        .map(|session| session.path.clone())
}

fn file_modified_time(path: &PathBuf) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

fn ole_edit_session_notify_path(path: &PathBuf) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("chemcore-ole-edit.ccjs");
    path.with_file_name(format!("{file_name}.notify.json"))
}

fn write_ole_edit_session_notify_file(path: &PathBuf) -> Result<(), String> {
    let Some(thread_id) = OLE_EDIT_MAIN_THREAD_ID.get().copied() else {
        return Ok(());
    };
    let notify_path = ole_edit_session_notify_path(path);
    let payload = serde_json::to_string_pretty(&serde_json::json!({
        "threadId": thread_id,
    }))
    .map_err(|error| format!("Failed to serialize OLE edit notification payload: {error}"))?;
    std::fs::write(&notify_path, format!("{payload}\n")).map_err(|error| {
        format!(
            "Failed to write OLE edit notification file {}: {error}",
            notify_path.display()
        )
    })
}

fn start_ole_edit_file_watcher(
    session_id: String,
    path: PathBuf,
    initial_modified: Option<SystemTime>,
) {
    let Some(thread_id) = OLE_EDIT_MAIN_THREAD_ID.get().copied() else {
        return;
    };
    thread::spawn(move || {
        let mut last_modified = initial_modified;
        loop {
            thread::sleep(Duration::from_millis(u64::from(OLE_EDIT_POLL_MS)));
            let Some(modified) = file_modified_time(&path) else {
                break;
            };
            if last_modified.is_some_and(|last| modified <= last) {
                continue;
            }
            last_modified = Some(modified);
            if let Ok(mut pending) = ole_edit_pending_updates().lock() {
                if !pending.iter().any(|value| value == &session_id) {
                    pending.push(session_id.clone());
                }
            }
            log_ole_event(&format!(
                "OLE edit watcher noticed change for {session_id} at {}",
                path.display()
            ));
            unsafe {
                let posted = PostThreadMessageW(thread_id, WM_OLE_EDIT_SESSION_CHANGED, 0, 0);
                if posted == 0 {
                    log_ole_event(&format!(
                        "PostThreadMessageW({thread_id}, WM_OLE_EDIT_SESSION_CHANGED) failed: {}",
                        GetLastError()
                    ));
                }
            }
        }
    });
}

fn poll_ole_edit_sessions() {
    let candidates = {
        let Ok(sessions) = ole_edit_sessions().lock() else {
            return;
        };
        sessions
            .iter()
            .filter_map(|(session_id, session)| {
                let modified = file_modified_time(&session.path)?;
                if session.last_modified.is_some_and(|last| modified <= last) {
                    return None;
                }
                Some((
                    session_id.clone(),
                    session.path.clone(),
                    session.object,
                    modified,
                ))
            })
            .collect::<Vec<_>>()
    };

    for (session_id, path, object, modified) in candidates {
        let document_json = match std::fs::read_to_string(&path) {
            Ok(value) => value,
            Err(error) => {
                log_ole_event(&format!(
                    "Failed to read OLE edit session {} from {}: {error}",
                    session_id,
                    path.display()
                ));
                continue;
            }
        };
        let hr = unsafe {
            apply_ole_edit_session_update(object as *mut ChemcoreOleObject, &document_json)
        };
        if hresult_succeeded(hr) {
            if let Ok(mut sessions) = ole_edit_sessions().lock() {
                if let Some(session) = sessions.get_mut(&session_id) {
                    session.last_modified = Some(modified);
                }
            }
        }
        log_ole_event(&format!(
            "OLE edit session {session_id} update -> 0x{:08X}",
            hr as u32
        ));
    }
}

fn apply_pending_ole_edit_session_updates() {
    let session_ids = {
        let Ok(mut pending) = ole_edit_pending_updates().lock() else {
            return;
        };
        std::mem::take(&mut *pending)
    };
    if session_ids.is_empty() {
        log_ole_event("OLE edit pending update queue was empty");
    } else {
        log_ole_event(&format!(
            "Applying {} pending OLE edit session update(s)",
            session_ids.len()
        ));
    }
    for session_id in session_ids {
        apply_ole_edit_session_update_by_id(&session_id);
    }
}

fn apply_ole_edit_session_update_by_id(session_id: &str) {
    let Some((path, object, modified)) = ({
        let Ok(sessions) = ole_edit_sessions().lock() else {
            return;
        };
        sessions.get(session_id).and_then(|session| {
            let modified = file_modified_time(&session.path)?;
            Some((session.path.clone(), session.object, modified))
        })
    }) else {
        log_ole_event(&format!(
            "OLE edit session {session_id} was not found for pending update"
        ));
        return;
    };
    let document_json = match std::fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            log_ole_event(&format!(
                "Failed to read OLE edit session {} from {}: {error}",
                session_id,
                path.display()
            ));
            return;
        }
    };
    log_ole_event(&format!(
        "Read OLE edit session {session_id} from {} ({} bytes)",
        path.display(),
        document_json.len()
    ));
    let hr =
        unsafe { apply_ole_edit_session_update(object as *mut ChemcoreOleObject, &document_json) };
    if hresult_succeeded(hr) {
        if let Ok(mut sessions) = ole_edit_sessions().lock() {
            if let Some(session) = sessions.get_mut(session_id) {
                session.last_modified = Some(modified);
            }
        }
    }
    log_ole_event(&format!(
        "OLE edit session {session_id} update -> 0x{:08X}",
        hr as u32
    ));
}

unsafe fn apply_ole_edit_session_update(
    object: *mut ChemcoreOleObject,
    document_json: &str,
) -> i32 {
    log_ole_event("Applying OLE edit session payload to object");
    if object.is_null() {
        return E_POINTER;
    }
    if document_json.trim().is_empty() {
        return E_FAIL;
    }
    let payload = match ole_object_payload_from_edit_session_text(document_json) {
        Ok(payload) => payload,
        Err(error) => {
            log_ole_event(&format!("Invalid OLE edit session payload: {error}"));
            return E_FAIL;
        }
    };
    log_ole_event("Built OLE edit session payload");
    (*object).payload = payload;
    (*object).extent_himetric = (*object).payload.extent_himetric();
    (*object).dirty = true;

    if !(*object).storage.is_null() {
        log_ole_event("Writing OLE edit session payload to storage");
        let hr = write_ole_storage_payload(
            (*object).storage,
            &(*object).payload,
            (*object).extent_himetric,
        );
        log_ole_event(&format!(
            "Write OLE edit session payload to storage -> 0x{:08X}",
            hr as u32
        ));
        if !hresult_succeeded(hr) {
            return hr;
        }
    }
    log_ole_event("Notifying OLE container about edit session payload");
    notify_ole_object_changed(object);
    log_ole_event("Finished notifying OLE container about edit session payload");
    S_OK
}

unsafe fn notify_ole_object_changed(object: *mut ChemcoreOleObject) {
    if object.is_null() {
        return;
    }
    if !(*object).ole_advise_holder.is_null() {
        let holder_vtbl = *((*object)
            .ole_advise_holder
            .cast::<*const OleAdviseHolderVtbl>());
        ((*holder_vtbl).send_on_save)((*object).ole_advise_holder);
    }
    if !(*object).data_advise_holder.is_null() {
        let holder_vtbl = *((*object)
            .data_advise_holder
            .cast::<*const DataAdviseHolderVtbl>());
        let data_object =
            (&mut (*object).data_object as *mut InterfacePart<DataObjectVtbl>).cast::<c_void>();
        let hr =
            ((*holder_vtbl).send_on_data_change)((*object).data_advise_holder, data_object, 0, 0);
        log_ole_event(&format!(
            "IDataAdviseHolder::SendOnDataChange -> 0x{:08X}",
            hr as u32
        ));
    }
    if !(*object).view_advise_sink.is_null()
        && ((*object).view_advise_aspects == 0
            || ((*object).view_advise_aspects & DVASPECT_CONTENT) != 0)
    {
        let sink_vtbl = *((*object).view_advise_sink.cast::<*const AdviseSinkVtbl>());
        ((*sink_vtbl).on_view_change)((*object).view_advise_sink, DVASPECT_CONTENT, -1);
        log_ole_event("IAdviseSink::OnViewChange(DVASPECT_CONTENT)");
    }
    if !(*object).client_site.is_null() {
        let site_vtbl = *((*object).client_site.cast::<*const OleClientSiteVtbl>());
        let hr = ((*site_vtbl).save_object)((*object).client_site);
        log_ole_event(&format!(
            "IOleClientSite::SaveObject -> 0x{:08X}",
            hr as u32
        ));
        let hr = ((*site_vtbl).show_object)((*object).client_site);
        log_ole_event(&format!(
            "IOleClientSite::ShowObject -> 0x{:08X}",
            hr as u32
        ));
        let hr = ((*site_vtbl).request_new_object_layout)((*object).client_site);
        log_ole_event(&format!(
            "IOleClientSite::RequestNewObjectLayout -> 0x{:08X}",
            hr as u32
        ));
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
    log_ole_event("IClassFactory::CreateInstance");
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
    log_ole_event(&format!(
        "IClassFactory::CreateInstance -> 0x{:08X}",
        hr as u32
    ));
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
    data_advise_holder: *mut c_void,
    view_advise_sink: *mut c_void,
    view_advise_aspects: u32,
    view_advise_flags: u32,
    payload: OleObjectPayload,
    extent_himetric: SIZE,
    dirty: bool,
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
        let extent_himetric = payload.extent_himetric();
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
            data_advise_holder: null_mut(),
            view_advise_sink: null_mut(),
            view_advise_aspects: 0,
            view_advise_flags: 0,
            payload,
            extent_himetric,
            dirty: false,
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
            com_release(self.storage);
            self.storage = null_mut();
            com_release(self.ole_advise_holder);
            self.ole_advise_holder = null_mut();
            com_release(self.data_advise_holder);
            self.data_advise_holder = null_mut();
            com_release(self.view_advise_sink);
            self.view_advise_sink = null_mut();
        }
    }
}

unsafe fn replace_object_storage(object: *mut ChemcoreOleObject, storage: *mut c_void) {
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
    let hr = write_clipboard_format_to_medium(
        &(*object).payload,
        (*object).extent_himetric,
        &*format,
        medium,
    );
    log_format_request("IDataObject::GetData", &*format, hr);
    hr
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
    let hr = if clipboard_format_supported(&(*object).payload, (*object).extent_himetric, &*format)
    {
        S_OK
    } else {
        DV_E_FORMATETC
    };
    log_format_request("IDataObject::QueryGetData", &*format, hr);
    hr
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

unsafe extern "system" fn data_object_d_advise(
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

unsafe extern "system" fn data_object_d_unadvise(this: *mut c_void, connection: u32) -> i32 {
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

unsafe extern "system" fn data_object_enum_d_advise(
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

#[repr(C)]
struct DataAdviseHolderVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    advise: unsafe extern "system" fn(
        *mut c_void,
        *mut c_void,
        *mut FORMATETC,
        u32,
        *mut c_void,
        *mut u32,
    ) -> i32,
    unadvise: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    enum_advise: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32,
    send_on_data_change: unsafe extern "system" fn(*mut c_void, *mut c_void, u32, u32) -> i32,
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

unsafe extern "system" fn persist_storage_is_dirty(this: *mut c_void) -> i32 {
    let object = owner_from_part::<PersistStorageVtbl>(this);
    if !object.is_null() && (*object).dirty {
        S_OK
    } else {
        S_FALSE
    }
}

unsafe extern "system" fn persist_storage_init_new(this: *mut c_void, storage: *mut c_void) -> i32 {
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

unsafe extern "system" fn persist_storage_load(this: *mut c_void, storage: *mut c_void) -> i32 {
    let object = owner_from_part::<PersistStorageVtbl>(this);
    if object.is_null() || storage.is_null() {
        return E_POINTER;
    }
    replace_object_storage(object, storage);
    if let Ok(document) = storage_read_stream(storage, OLE_STREAM_DOCUMENT).and_then(|bytes| {
        String::from_utf8(bytes)
            .map_err(|error| format!("ChemcoreDocument stream is not UTF-8: {error}"))
    }) {
        (*object).payload.chemcore_document_json = document;
    } else if let Ok(document) =
        storage_read_stream(storage, OLE_STREAM_CONTENTS).and_then(|bytes| {
            String::from_utf8(bytes)
                .map_err(|error| format!("CONTENTS stream is not UTF-8: {error}"))
        })
    {
        (*object).payload.chemcore_document_json = document;
    }
    if let Ok(svg) = storage_read_stream(storage, OLE_STREAM_PREVIEW_SVG).and_then(|bytes| {
        String::from_utf8(bytes)
            .map_err(|error| format!("ChemcorePreviewSvg stream is not UTF-8: {error}"))
    }) {
        (*object).payload.svg = svg;
    }
    if let Ok(cdxml) = storage_read_stream(storage, OLE_STREAM_SOURCE_CDXML).and_then(|bytes| {
        String::from_utf8(bytes)
            .map_err(|error| format!("ChemcoreSourceCdxml stream is not UTF-8: {error}"))
    }) {
        (*object).payload.cdxml = Some(cdxml);
    }
    (*object).extent_himetric =
        storage_presentation_extent(storage).unwrap_or_else(|| (*object).payload.extent_himetric());
    (*object).dirty = false;
    log_ole_event("IPersistStorage::Load -> 0x00000000");
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
    let hr = write_ole_storage_payload(storage, &(*object).payload, (*object).extent_himetric);
    if hresult_succeeded(hr) {
        (*object).dirty = false;
    }
    log_ole_event(&format!("IPersistStorage::Save -> 0x{:08X}", hr as u32));
    hr
}

unsafe extern "system" fn persist_storage_save_completed(
    this: *mut c_void,
    storage: *mut c_void,
) -> i32 {
    let object = owner_from_part::<PersistStorageVtbl>(this);
    if !object.is_null() {
        replace_object_storage(object, storage);
    }
    S_OK
}

unsafe extern "system" fn persist_storage_hands_off_storage(this: *mut c_void) -> i32 {
    let object = owner_from_part::<PersistStorageVtbl>(this);
    if !object.is_null() {
        replace_object_storage(object, null_mut());
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

unsafe fn write_ole_storage_payload(
    storage: *mut c_void,
    payload: &OleObjectPayload,
    presentation_extent: SIZE,
) -> i32 {
    if storage.is_null() {
        return E_POINTER;
    }
    let hr = WriteClassStg(storage, &CLSID_CHEMCORE_DOCUMENT);
    if !hresult_succeeded(hr) {
        return hr;
    }
    let user_type = wide_null(DOCUMENT_DISPLAY_NAME);
    let hr = WriteFmtUserTypeStg(
        storage,
        clipboard_format(FORMAT_CHEMCORE_NATIVE),
        user_type.as_ptr(),
    );
    if !hresult_succeeded(hr) {
        return hr;
    }

    let contents = ole_contents_stream_payload(payload);
    let ole = ole_stream_payload();
    let obj_info = ole_obj_info_stream_payload();
    let document = chemcore_document_stream_payload(payload);
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

fn chemcore_document_stream_payload(payload: &OleObjectPayload) -> Vec<u8> {
    payload.chemcore_document_json.as_bytes().to_vec()
}

fn ole_contents_stream_payload(payload: &OleObjectPayload) -> Vec<u8> {
    payload.chemcore_document_json.as_bytes().to_vec()
}

fn ole_obj_info_stream_payload() -> [u8; 6] {
    // ODT: ODTPersist1=0, cf=0x0003 (metafile/EMF), ODTPersist2=0x0001
    // (fEMF). Word's own RTF clipboard stream uses this pre-cache value.
    [0x00, 0x00, 0x03, 0x00, 0x01, 0x00]
}

fn ole_stream_payload() -> [u8; 20] {
    let mut bytes = [0u8; 20];
    bytes[0..4].copy_from_slice(&0x0200_0001u32.to_le_bytes());
    bytes[4..8].copy_from_slice(&0x0000_0008u32.to_le_bytes());
    bytes
}

unsafe fn create_ole_storage_medium(
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

unsafe fn save_ole_object_storage(
    storage: *mut c_void,
    payload: &OleObjectPayload,
    presentation_extent: SIZE,
) -> i32 {
    let mut object = Box::new(ChemcoreOleObject::with_payload(payload.clone()));
    object.extent_himetric = presentation_extent;
    object.init_self_references();
    let object = Box::into_raw(object);
    let persist_storage =
        (&mut (*object).persist_storage as *mut InterfacePart<PersistStorageVtbl>).cast::<c_void>();
    let hr = OleSave(persist_storage, storage, 0);
    chemcore_object_release(object);
    if hresult_succeeded(hr) {
        storage_commit(storage)
    } else {
        hr
    }
}

unsafe fn write_native_clipboard_storage_payload(
    storage: *mut c_void,
    payload: &OleObjectPayload,
) -> i32 {
    if storage.is_null() {
        return E_POINTER;
    }
    let hr = WriteClassStg(storage, &CLSID_CHEMCORE_DOCUMENT);
    if !hresult_succeeded(hr) {
        return hr;
    }
    let user_type = wide_null(DOCUMENT_DISPLAY_NAME);
    let hr = WriteFmtUserTypeStg(
        storage,
        clipboard_format(FORMAT_CHEMCORE_NATIVE),
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

fn native_clipboard_storage_file_bytes_for_payload(
    payload: &OleObjectPayload,
) -> Result<Vec<u8>, String> {
    let storage_path = env::temp_dir().join(format!(
        "chemcore-office-native-{}-{}.ole",
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

fn hglobal_for_native_clipboard_payload(payload: &OleObjectPayload) -> Result<HGLOBAL, i32> {
    let bytes = native_clipboard_storage_file_bytes_for_payload(payload).map_err(|_| E_FAIL)?;
    hglobal_for_bytes(&bytes)
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
        (*descriptor).cbSize = descriptor_size as u32;
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

fn hglobal_for_word_rtf_object(payload: &OleObjectPayload) -> Result<HGLOBAL, i32> {
    let rtf = word_rtf_object_for_payload(payload).map_err(|_| E_FAIL)?;
    hglobal_for_utf8_nul(&rtf)
}

fn word_rtf_object_for_payload(payload: &OleObjectPayload) -> Result<String, String> {
    let natural_extent = payload.extent_himetric();
    let display_extent = fit_extent_himetric_to_word_body(natural_extent);
    let emf = enhanced_metafile_bits_for_payload(payload, natural_extent)
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
    rtf.push_str("}");
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

fn rtf_percent_scale(display: i32, natural: i32) -> i32 {
    if natural <= 0 {
        return 100;
    }
    ((display.max(1) as f64 / natural as f64) * 100.0)
        .round()
        .clamp(1.0, 1000.0) as i32
}

fn word_rtf_objdata_bytes(ole_storage: &[u8], presentation_emf: &[u8]) -> Result<Vec<u8>, String> {
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

fn rtf_hex_lines(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2 + bytes.len() / 32 + 1);
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 && index % 32 == 0 {
            out.push('\n');
        }
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn word_docx_package_for_payload(payload: &OleObjectPayload) -> Result<Vec<u8>, String> {
    let natural_extent = payload.extent_himetric();
    let display_extent = fit_extent_himetric_to_word_body(natural_extent);
    let emf = enhanced_metafile_bits_for_payload(payload, display_extent).map_err(|hr| {
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

fn zip_add_text<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    options: zip::write::SimpleFileOptions,
    name: &str,
    text: &str,
) -> Result<(), String> {
    zip_add_bytes(zip, options, name, text.as_bytes())
}

fn zip_add_bytes<W: Write + std::io::Seek>(
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

fn word_content_types_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="bin" ContentType="application/vnd.openxmlformats-officedocument.oleObject"/><Default Extension="emf" ContentType="image/x-emf"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/><Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/></Types>"#.to_string()
}

fn word_root_rels_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>"#.to_string()
}

fn word_document_rels_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.emf"/><Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject" Target="embeddings/oleObject1.bin"/></Relationships>"#.to_string()
}

fn word_core_props_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title>Chemcore Document</dc:title><dc:creator>Chemcore</dc:creator><cp:lastModifiedBy>Chemcore</cp:lastModifiedBy></cp:coreProperties>"#.to_string()
}

fn word_app_props_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><Application>Chemcore</Application></Properties>"#.to_string()
}

fn word_document_xml(width_pt: f64, height_pt: f64, width_twips: i32, height_twips: i32) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:v="urn:schemas-microsoft-com:vml" xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:object w:dxaOrig="{width_twips}" w:dyaOrig="{height_twips}"><v:shapetype id="_x0000_t75" coordsize="21600,21600" o:spt="75" o:preferrelative="t" path="m@4@5l@4@11@9@11@9@5xe" filled="f" stroked="f"><v:stroke joinstyle="miter"/><v:formulas><v:f eqn="if lineDrawn pixelLineWidth 0"/><v:f eqn="sum @0 1 0"/><v:f eqn="sum 0 0 @1"/><v:f eqn="prod @2 1 2"/><v:f eqn="prod @3 21600 pixelWidth"/><v:f eqn="prod @3 21600 pixelHeight"/><v:f eqn="sum @0 0 1"/><v:f eqn="prod @6 1 2"/><v:f eqn="prod @7 21600 pixelWidth"/><v:f eqn="sum @8 21600 0"/><v:f eqn="prod @7 21600 pixelHeight"/><v:f eqn="sum @10 21600 0"/></v:formulas><v:path o:extrusionok="f" gradientshapeok="t" o:connecttype="rect"/><o:lock v:ext="edit" aspectratio="t"/></v:shapetype><v:shape id="_x0000_i1025" type="#_x0000_t75" style="width:{:.1}pt;height:{:.1}pt" o:ole=""><v:imagedata r:id="rId4" o:title=""/></v:shape><o:OLEObject Type="Embed" ProgID="{VERSIONED_PROG_ID}" ShapeID="_x0000_i1025" DrawAspect="Content" ObjectID="_chemcore0001" r:id="rId5"/></w:object></w:r></w:p><w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1800" w:bottom="1440" w:left="1800" w:header="851" w:footer="992" w:gutter="0"/></w:sectPr></w:body></w:document>"##,
        width_pt, height_pt
    )
}

fn himetric_to_points(value: i32) -> f64 {
    (value.max(1) as f64 / HIMETRIC_PER_CM) * PT_PER_CM
}

fn fit_extent_himetric_to_word_body(extent: SIZE) -> SIZE {
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

fn points_to_twips(value: f64) -> i32 {
    (value * 20.0).round().clamp(1.0, i32::MAX as f64) as i32
}

fn points_to_himetric(value: f64) -> i32 {
    ((value / PT_PER_CM) * HIMETRIC_PER_CM)
        .round()
        .clamp(MIN_OBJECT_EXTENT_HIMETRIC as f64, i32::MAX as f64) as i32
}

fn ole_storage_file_bytes_for_payload(
    payload: &OleObjectPayload,
    presentation_extent: SIZE,
) -> Result<Vec<u8>, String> {
    let storage_path = env::temp_dir().join(format!(
        "chemcore-office-docx-{}-{}.ole",
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

fn unique_temp_suffix() -> String {
    let sequence = TEMP_SUFFIX_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{timestamp}-{sequence}")
}

fn enhanced_print_is_emf(bytes: &[u8]) -> bool {
    bytes.len() >= 44
        && u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) == 1
        && bytes[40..44] == *b" EMF"
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

fn metafile_pict_medium(handle: HGLOBAL, medium: *mut STGMEDIUM) -> i32 {
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

fn enhanced_metafile_medium(handle: *mut c_void, medium: *mut STGMEDIUM) -> i32 {
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

fn known_clipboard_format_name(format: u16) -> &'static str {
    if format == clipboard_format(CLIPBOARD_FORMAT_EMBED_SOURCE) {
        CLIPBOARD_FORMAT_EMBED_SOURCE
    } else if format == clipboard_format(CLIPBOARD_FORMAT_EMBEDDED_OBJECT) {
        CLIPBOARD_FORMAT_EMBEDDED_OBJECT
    } else if format == clipboard_format(CLIPBOARD_FORMAT_NATIVE) {
        CLIPBOARD_FORMAT_NATIVE
    } else if format == clipboard_format(CLIPBOARD_FORMAT_OBJECT_DESCRIPTOR) {
        CLIPBOARD_FORMAT_OBJECT_DESCRIPTOR
    } else if format == clipboard_format(CLIPBOARD_FORMAT_RTF) {
        CLIPBOARD_FORMAT_RTF
    } else if format == clipboard_format(FORMAT_CHEMCORE_NATIVE) {
        FORMAT_CHEMCORE_NATIVE
    } else if format == CF_ENHMETAFILE {
        "CF_ENHMETAFILE"
    } else if format == CF_METAFILEPICT {
        "CF_METAFILEPICT"
    } else if format == clipboard_format(FORMAT_CHEMCORE_FRAGMENT) {
        FORMAT_CHEMCORE_FRAGMENT
    } else if format == clipboard_format(FORMAT_CHEMCORE_DOCUMENT_JSON) {
        FORMAT_CHEMCORE_DOCUMENT_JSON
    } else if format == clipboard_format(FORMAT_CHEMDRAW_INTERCHANGE) {
        FORMAT_CHEMDRAW_INTERCHANGE
    } else if format == clipboard_format(FORMAT_CDXML_MIME) {
        FORMAT_CDXML_MIME
    } else if format == clipboard_format(FORMAT_SVG_MIME) {
        FORMAT_SVG_MIME
    } else if format == clipboard_format(FORMAT_SVG) {
        FORMAT_SVG
    } else if format == CF_UNICODETEXT_FORMAT {
        "CF_UNICODETEXT"
    } else {
        "unknown"
    }
}

unsafe fn log_format_request(prefix: &str, format: &FORMATETC, hr: i32) {
    log_ole_event(&format!(
        "{prefix}: cf={} ({}) aspect={} lindex={} tymed=0x{:X} -> 0x{:08X}",
        format.cfFormat,
        known_clipboard_format_name(format.cfFormat),
        format.dwAspect,
        format.lindex,
        format.tymed,
        hr as u32
    ));
}

fn default_misc_status() -> u32 {
    (OLEMISC_RENDERINGISDEVICEINDEPENDENT | OLEMISC_SETCLIENTSITEFIRST) as u32
}

fn ole_clipboard_formats(payload: &OleObjectPayload, _extent: SIZE) -> Vec<FORMATETC> {
    let mut formats = Vec::new();
    push_format(
        &mut formats,
        clipboard_format(CLIPBOARD_FORMAT_EMBEDDED_OBJECT),
        TYMED_ISTORAGE as u32,
    );
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
    push_format(
        &mut formats,
        clipboard_format(CLIPBOARD_FORMAT_NATIVE),
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
        clipboard_format(FORMAT_CHEMCORE_NATIVE),
        TYMED_HGLOBAL as u32,
    );
    push_format(
        &mut formats,
        clipboard_format(FORMAT_CHEMCORE_DOCUMENT_JSON),
        TYMED_HGLOBAL as u32,
    );
    if payload.cdxml.is_some() {
        push_format(
            &mut formats,
            clipboard_format(FORMAT_CDXML_MIME),
            TYMED_HGLOBAL as u32,
        );
    }
    push_format(&mut formats, CF_ENHMETAFILE, TYMED_ENHMF as u32);

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
    if format.cfFormat == clipboard_format(CLIPBOARD_FORMAT_EMBEDDED_OBJECT)
        || format.cfFormat == clipboard_format(CLIPBOARD_FORMAT_EMBED_SOURCE)
    {
        if (format.tymed & TYMED_ISTORAGE as u32) == 0 {
            return DV_E_TYMED;
        }
        return create_ole_storage_medium(payload, extent, medium);
    }
    if format.cfFormat == CF_ENHMETAFILE {
        if (format.tymed & TYMED_ENHMF as u32) == 0 {
            return DV_E_TYMED;
        }
        return match enhanced_metafile_for_payload(payload, extent) {
            Ok(handle) => enhanced_metafile_medium(handle, medium),
            Err(hr) => hr,
        };
    }
    if format.cfFormat == CF_METAFILEPICT {
        if (format.tymed & TYMED_MFPICT as u32) == 0 {
            return DV_E_TYMED;
        }
        return match hglobal_for_metafile_pict(payload, extent) {
            Ok(handle) => metafile_pict_medium(handle, medium),
            Err(hr) => hr,
        };
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
    if format.cfFormat == clipboard_format(CLIPBOARD_FORMAT_RTF) {
        return match hglobal_for_word_rtf_object(payload) {
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
    if format.cfFormat == clipboard_format(CLIPBOARD_FORMAT_NATIVE) {
        return match hglobal_for_native_clipboard_payload(payload) {
            Ok(handle) => hglobal_medium(handle, medium),
            Err(hr) => hr,
        };
    }
    if format.cfFormat == clipboard_format(FORMAT_CHEMCORE_NATIVE) {
        return hglobal_text_medium(&payload.chemcore_document_json, false, medium);
    }
    if format.cfFormat == clipboard_format(FORMAT_CHEMCORE_DOCUMENT_JSON) {
        return hglobal_text_medium(&payload.chemcore_document_json, false, medium);
    }
    if format.cfFormat == clipboard_format(FORMAT_CDXML_MIME) {
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
    if format.cfFormat == CF_UNICODETEXT_FORMAT {
        return payload
            .text
            .as_deref()
            .map(|value| hglobal_text_medium(value, true, medium))
            .unwrap_or(DV_E_FORMATETC);
    }
    DV_E_FORMATETC
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn ole_clipboard_formats_prefer_embedded_object_over_visual_fallbacks() {
        let payload = OleObjectPayload {
            chemcore_fragment_json: Some("{\"nodes\":[],\"bonds\":[]}".to_string()),
            chemcore_document_json:
                "{\"document\":{\"name\":\"chemcore\"},\"objects\":[],\"resources\":{}}".to_string(),
            render_list_json: None,
            cdxml: Some("<CDXML></CDXML>".to_string()),
            svg: "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>".to_string(),
            svg_was_supplied: true,
            text: Some("<CDXML></CDXML>".to_string()),
        };
        let format_names: BTreeSet<_> = ole_clipboard_formats(&payload, SIZE::default())
            .into_iter()
            .map(|format| known_clipboard_format_name(format.cfFormat))
            .collect();
        assert!(format_names.contains(CLIPBOARD_FORMAT_EMBEDDED_OBJECT));
        assert!(format_names.contains(CLIPBOARD_FORMAT_EMBED_SOURCE));
        assert!(format_names.contains(CLIPBOARD_FORMAT_OBJECT_DESCRIPTOR));
        assert!(
            !format_names.contains(CLIPBOARD_FORMAT_RTF),
            "Word should use the embedded OLE object path so it can fit oversized objects itself"
        );
        assert!(format_names.contains(CLIPBOARD_FORMAT_NATIVE));
        assert!(format_names.contains(FORMAT_CHEMCORE_NATIVE));
        assert!(format_names.contains(FORMAT_CHEMCORE_DOCUMENT_JSON));
        assert!(format_names.contains(FORMAT_CDXML_MIME));
        assert!(format_names.contains("CF_ENHMETAFILE"));
        assert!(
            !format_names.contains(FORMAT_CHEMDRAW_INTERCHANGE),
            "Chemcore should not advertise ChemDraw's native clipboard format with a CDXML payload"
        );
        assert!(
            !format_names.contains(FORMAT_SVG_MIME),
            "Word's default Paste prefers SVG as a plain image instead of embedding the OLE object"
        );
        assert!(
            !format_names.contains(FORMAT_SVG),
            "Word's default Paste prefers SVG as a plain image instead of embedding the OLE object"
        );
        assert!(
            !format_names.contains("CF_UNICODETEXT"),
            "Word's default Paste may choose plain text instead of embedding the OLE object"
        );
    }

    #[test]
    fn clipboard_payload_without_svg_generates_preview_svg_from_document() {
        let document_json = serde_json::to_string(&chemcore_engine::ChemcoreDocument::blank())
            .expect("blank document should serialize");
        let payload = OleObjectPayload::from_clipboard(ClipboardPayload {
            chemcore_fragment_json: None,
            chemcore_document_json: Some(document_json),
            render_list_json: None,
            cdxml: None,
            svg: None,
            text: None,
        });
        assert!(payload.svg.contains("<svg"));
        assert!(
            payload.svg_was_supplied,
            "generated preview SVG should drive the same preview bounds path as supplied SVG"
        );
        assert!(
            emf_preview::preview_source_bounds(&payload).is_some(),
            "generated preview SVG should provide OLE preview source bounds"
        );
        assert!(
            !payload.svg.contains(DOCUMENT_DISPLAY_NAME),
            "generated preview should not fall back to the placeholder svg"
        );
    }

    #[test]
    fn word_docx_uses_natural_orig_size_and_fitted_display_extent() {
        let document_json = serde_json::to_string(&chemcore_engine::ChemcoreDocument::blank())
            .expect("blank document should serialize");
        let payload = OleObjectPayload {
            chemcore_fragment_json: None,
            chemcore_document_json: document_json,
            render_list_json: None,
            cdxml: None,
            svg: r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 400"></svg>"#
                .to_string(),
            svg_was_supplied: true,
            text: None,
        };
        let natural_extent = payload.extent_himetric();
        let display_extent = fit_extent_himetric_to_word_body(natural_extent);
        let natural_width_twips = points_to_twips(himetric_to_points(natural_extent.cx));
        let natural_height_twips = points_to_twips(himetric_to_points(natural_extent.cy));
        let display_width_pt = himetric_to_points(display_extent.cx);
        let display_height_pt = himetric_to_points(display_extent.cy);

        let package = word_docx_package_for_payload(&payload).expect("docx should be generated");
        let mut archive =
            zip::ZipArchive::new(Cursor::new(package)).expect("docx package should be a zip");
        let mut document_xml = String::new();
        archive
            .by_name("word/document.xml")
            .expect("document.xml should exist")
            .read_to_string(&mut document_xml)
            .expect("document.xml should be UTF-8");

        assert!(
            document_xml.contains(&format!(
                r#"w:dxaOrig="{natural_width_twips}" w:dyaOrig="{natural_height_twips}""#
            )),
            "Word reset-size metadata must preserve the natural OLE object size"
        );
        assert!(
            document_xml.contains(&format!(
                r#"style="width:{display_width_pt:.1}pt;height:{display_height_pt:.1}pt""#
            )),
            "Word's visible shape should still be fitted to the document body"
        );
    }

    #[test]
    fn word_rtf_clipboard_uses_natural_extent() {
        let document_json = serde_json::to_string(&chemcore_engine::ChemcoreDocument::blank())
            .expect("blank document should serialize");
        let payload = OleObjectPayload {
            chemcore_fragment_json: None,
            chemcore_document_json: document_json,
            render_list_json: None,
            cdxml: None,
            svg: r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 400"></svg>"#
                .to_string(),
            svg_was_supplied: true,
            text: None,
        };
        let natural_extent = payload.extent_himetric();
        let display_extent = fit_extent_himetric_to_word_body(natural_extent);
        let natural_width_twips = points_to_twips(himetric_to_points(natural_extent.cx));
        let natural_height_twips = points_to_twips(himetric_to_points(natural_extent.cy));
        let scale_x = rtf_percent_scale(display_extent.cx, natural_extent.cx);
        let scale_y = rtf_percent_scale(display_extent.cy, natural_extent.cy);

        let rtf = word_rtf_object_for_payload(&payload).expect("RTF should be generated");

        assert!(
            rtf.contains(&format!(
                "\\objw{natural_width_twips}\\objh{natural_height_twips}"
            )),
            "clipboard RTF should preserve natural OLE object size and let Word decide paste scaling"
        );
        assert!(
            rtf.contains(&format!(
                "\\picwgoal{natural_width_twips}\\pichgoal{natural_height_twips}"
            )),
            "clipboard RTF preview should preserve natural EMF goal size"
        );
        assert!(
            rtf.contains(&format!("\\picscalex{scale_x}\\picscaley{scale_y}")),
            "clipboard RTF should use picture scaling for the fitted display size"
        );
    }

    #[test]
    fn ole_manifest_persists_presentation_extent() {
        let extent = SIZE {
            cx: 12_220,
            cy: 5_250,
        };
        let manifest = ole_manifest_stream_payload(extent).expect("manifest should serialize");

        assert_size_eq(presentation_extent_from_manifest(&manifest), extent);
    }

    #[test]
    fn ole_presentation_extent_parser_accepts_native_and_word_stream_layouts() {
        let extent = SIZE {
            cx: 12_220,
            cy: 5_250,
        };
        let mut native = vec![0u8; 40];
        native[28..32].copy_from_slice(&extent.cx.to_le_bytes());
        native[32..36].copy_from_slice(&extent.cy.to_le_bytes());
        assert_size_eq(
            presentation_extent_from_ole_presentation_stream(&native),
            extent,
        );

        let mut word = vec![0u8; 128];
        word[12..16].copy_from_slice(&1u32.to_le_bytes());
        word[40..44].copy_from_slice(&1u32.to_le_bytes());
        word[72..76].copy_from_slice(&extent.cx.to_le_bytes());
        word[76..80].copy_from_slice(&extent.cy.to_le_bytes());
        assert_size_eq(
            presentation_extent_from_ole_presentation_stream(&word),
            extent,
        );
    }

    #[test]
    fn desktop_exe_candidates_cover_dev_and_tauri_resource_layouts() {
        let dev_server = PathBuf::from(r"C:\ChemcoreDev\chemcore\target\debug\chemcore-office.exe");
        assert_eq!(
            desktop_exe_candidates_for_server_path(&dev_server)[0],
            PathBuf::from(r"C:\ChemcoreDev\chemcore\target\debug\chemcore-desktop.exe")
        );

        let resource_server =
            PathBuf::from(r"C:\Program Files\Chemcore\resources\chemcore-office.exe");
        let candidates = desktop_exe_candidates_for_server_path(&resource_server);
        assert!(candidates.contains(&PathBuf::from(
            r"C:\Program Files\Chemcore\resources\chemcore-desktop.exe"
        )));
        assert!(candidates.contains(&PathBuf::from(
            r"C:\Program Files\Chemcore\chemcore-desktop.exe"
        )));
        assert!(candidates.contains(&PathBuf::from(r"C:\Program Files\Chemcore\Chemcore.exe")));
    }

    fn assert_size_eq(actual: Option<SIZE>, expected: SIZE) {
        let actual = actual.expect("expected a presentation extent");
        assert_eq!(actual.cx, expected.cx);
        assert_eq!(actual.cy, expected.cy);
    }
}

fn ole_preview_svg_stream_payload() -> Vec<u8> {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="240" height="120" viewBox="0 0 240 120"><rect width="240" height="120" fill="#ffffff"/><path d="M56 68h128" stroke="#111827" stroke-width="4" stroke-linecap="round"/><circle cx="56" cy="68" r="7" fill="#111827"/><circle cx="184" cy="68" r="7" fill="#111827"/><text x="120" y="32" text-anchor="middle" font-family="Arial, sans-serif" font-size="16" fill="#111827">{DOCUMENT_DISPLAY_NAME}</text></svg>"##
    )
    .into_bytes()
}

fn ole_manifest_stream_payload(presentation_extent: SIZE) -> Result<Vec<u8>, i32> {
    serde_json::to_vec(&serde_json::json!({
        "format": "chemcore-ole-object",
        "version": 1,
        "classId": CLSID_STRING,
        "progId": PROG_ID,
        "presentationExtentHimetric": {
            "cx": presentation_extent.cx,
            "cy": presentation_extent.cy
        },
        "documentStream": OLE_STREAM_DOCUMENT,
        "previewSvgStream": OLE_STREAM_PREVIEW_SVG,
        "enhancedPrintStream": OLE_STREAM_ENHANCED_PRINT,
        "presentationStreams": [
            OLE_STREAM_PRESENTATION_EMF
        ]
    }))
    .map_err(|_| E_FAIL)
}

unsafe fn storage_presentation_extent(storage: *mut c_void) -> Option<SIZE> {
    storage_read_stream(storage, OLE_STREAM_MANIFEST)
        .ok()
        .and_then(|bytes| presentation_extent_from_manifest(&bytes))
        .or_else(|| {
            storage_read_stream(storage, OLE_STREAM_PRESENTATION_EMF)
                .ok()
                .and_then(|bytes| presentation_extent_from_ole_presentation_stream(&bytes))
        })
        .or_else(|| {
            storage_read_stream(storage, OLE_STREAM_PRESENTATION_EMF_WORD)
                .ok()
                .and_then(|bytes| presentation_extent_from_ole_presentation_stream(&bytes))
        })
        .or_else(|| {
            storage_read_stream(storage, OLE_STREAM_ENHANCED_PRINT)
                .ok()
                .and_then(|bytes| presentation_extent_from_emf_bits(&bytes))
        })
}

fn presentation_extent_from_manifest(bytes: &[u8]) -> Option<SIZE> {
    let manifest: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let extent = manifest.get("presentationExtentHimetric")?;
    let cx = extent.get("cx")?.as_i64()?;
    let cy = extent.get("cy")?.as_i64()?;
    valid_presentation_extent(cx, cy)
}

fn presentation_extent_from_ole_presentation_stream(bytes: &[u8]) -> Option<SIZE> {
    read_size_at(bytes, 28).or_else(|| {
        let emf_signature = 1u32.to_le_bytes();
        bytes
            .windows(4)
            .enumerate()
            .filter(|(_, window)| *window == emf_signature)
            .find_map(|(emf_offset, _)| presentation_extent_from_emf_bits(&bytes[emf_offset..]))
    })
}

fn presentation_extent_from_emf_bits(bytes: &[u8]) -> Option<SIZE> {
    read_size_at(bytes, 32)
}

fn read_size_at(bytes: &[u8], offset: usize) -> Option<SIZE> {
    let cx = i32::from_le_bytes(bytes.get(offset..offset + 4)?.try_into().ok()?);
    let cy = i32::from_le_bytes(bytes.get(offset + 4..offset + 8)?.try_into().ok()?);
    valid_presentation_extent(cx as i64, cy as i64)
}

fn valid_presentation_extent(cx: i64, cy: i64) -> Option<SIZE> {
    if (MIN_OBJECT_EXTENT_HIMETRIC as i64..=1_000_000).contains(&cx)
        && (MIN_OBJECT_EXTENT_HIMETRIC as i64..=1_000_000).contains(&cy)
    {
        Some(SIZE {
            cx: cx as i32,
            cy: cy as i32,
        })
    } else {
        None
    }
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
struct OleAdviseHolderVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    advise: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut u32) -> i32,
    unadvise: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    enum_advise: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32,
    send_on_rename: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
    send_on_save: unsafe extern "system" fn(*mut c_void) -> i32,
    send_on_close: unsafe extern "system" fn(*mut c_void) -> i32,
}

#[repr(C)]
struct OleClientSiteVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    save_object: unsafe extern "system" fn(*mut c_void) -> i32,
    get_moniker: unsafe extern "system" fn(*mut c_void, u32, u32, *mut *mut c_void) -> i32,
    get_container: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32,
    show_object: unsafe extern "system" fn(*mut c_void) -> i32,
    on_show_window: unsafe extern "system" fn(*mut c_void, i32) -> i32,
    request_new_object_layout: unsafe extern "system" fn(*mut c_void) -> i32,
}

#[repr(C)]
struct AdviseSinkVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    on_data_change: unsafe extern "system" fn(*mut c_void, *const FORMATETC, *const STGMEDIUM),
    on_view_change: unsafe extern "system" fn(*mut c_void, u32, i32),
    on_rename: unsafe extern "system" fn(*mut c_void, *mut c_void),
    on_save: unsafe extern "system" fn(*mut c_void),
    on_close: unsafe extern "system" fn(*mut c_void),
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
    this: *mut c_void,
    data_object: *mut c_void,
    _creation: i32,
    _reserved: u32,
) -> i32 {
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() || data_object.is_null() {
        return E_POINTER;
    }
    match payload_from_data_object(data_object) {
        Ok(payload) => {
            (*object).payload = payload;
            (*object).extent_himetric = (*object).payload.extent_himetric();
            S_OK
        }
        Err(error) => {
            log_ole_event(&format!("IOleObject::InitFromData failed: {error}"));
            E_FAIL
        }
    }
}

unsafe extern "system" fn ole_object_get_clipboard_data(
    this: *mut c_void,
    _reserved: u32,
    data_object: *mut *mut c_void,
) -> i32 {
    if data_object.is_null() {
        return E_POINTER;
    }
    *data_object = null_mut();
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    *data_object =
        (&mut (*object).data_object as *mut InterfacePart<DataObjectVtbl>).cast::<c_void>();
    chemcore_object_add_ref(object);
    log_ole_event("IOleObject::GetClipboardData -> IDataObject");
    S_OK
}

unsafe fn payload_from_data_object(data_object: *mut c_void) -> Result<OleObjectPayload, String> {
    if data_object.is_null() {
        return Err("IDataObject pointer was null.".to_string());
    }
    let get_data = (*(*(data_object.cast::<*const DataObjectVtbl>()))).get_data;

    for format_name in [
        CLIPBOARD_FORMAT_EMBEDDED_OBJECT,
        CLIPBOARD_FORMAT_EMBED_SOURCE,
    ] {
        let format = FORMATETC {
            cfFormat: clipboard_format(format_name),
            ptd: null_mut(),
            dwAspect: DVASPECT_CONTENT,
            lindex: -1,
            tymed: TYMED_ISTORAGE as u32,
        };
        let mut medium = STGMEDIUM::default();
        let hr = get_data(data_object, &format, &mut medium);
        if !hresult_succeeded(hr) {
            continue;
        }
        let result = if medium.tymed == TYMED_ISTORAGE as u32 && !medium.u.pstg.is_null() {
            payload_from_storage(medium.u.pstg)
        } else {
            Err(format!(
                "IDataObject::GetData({format_name}) returned unexpected medium."
            ))
        };
        ReleaseStgMedium(&mut medium);
        if result.is_ok() {
            return result;
        }
    }

    let mut payload = OleObjectPayload::blank();
    let mut populated = false;

    if let Some(document) =
        text_payload_from_data_object(data_object, FORMAT_CHEMCORE_NATIVE, false)?.or(
            text_payload_from_data_object(data_object, FORMAT_CHEMCORE_DOCUMENT_JSON, false)?,
        )
    {
        payload.chemcore_document_json = document;
        populated = true;
    }
    if let Some(fragment) =
        text_payload_from_data_object(data_object, FORMAT_CHEMCORE_FRAGMENT, false)?
    {
        payload.chemcore_fragment_json = Some(fragment);
        populated = true;
    }
    if let Some(cdxml) =
        text_payload_from_data_object(data_object, FORMAT_CHEMDRAW_INTERCHANGE, false)?.or(
            text_payload_from_data_object(data_object, FORMAT_CDXML_MIME, false)?,
        )
    {
        if payload.text.is_none() {
            payload.text = Some(cdxml.clone());
        }
        payload.cdxml = Some(cdxml);
        populated = true;
    }
    if let Some(svg) = text_payload_from_data_object(data_object, FORMAT_SVG_MIME, false)?.or(
        text_payload_from_data_object(data_object, FORMAT_SVG, false)?,
    ) {
        payload.svg = svg;
        payload.svg_was_supplied = true;
        populated = true;
    }
    if let Some(text) =
        text_payload_from_data_object_by_id(data_object, CF_UNICODETEXT_FORMAT, true)?
    {
        payload.text = Some(text);
        populated = true;
    }

    if populated {
        Ok(payload)
    } else {
        Err("Chemcore payload was not available from IDataObject.".to_string())
    }
}

unsafe fn payload_from_storage(storage: *mut c_void) -> Result<OleObjectPayload, String> {
    if storage.is_null() {
        return Err("IStorage pointer was null.".to_string());
    }
    let mut payload = OleObjectPayload::blank();
    let mut populated = false;

    if let Ok(document) = storage_read_stream(storage, OLE_STREAM_DOCUMENT).and_then(|bytes| {
        String::from_utf8(bytes)
            .map_err(|error| format!("ChemcoreDocument stream is not UTF-8: {error}"))
    }) {
        payload.chemcore_document_json = document;
        populated = true;
    } else if let Ok(document) =
        storage_read_stream(storage, OLE_STREAM_CONTENTS).and_then(|bytes| {
            String::from_utf8(bytes)
                .map_err(|error| format!("CONTENTS stream is not UTF-8: {error}"))
        })
    {
        payload.chemcore_document_json = document;
        populated = true;
    }
    if let Ok(svg) = storage_read_stream(storage, OLE_STREAM_PREVIEW_SVG).and_then(|bytes| {
        String::from_utf8(bytes)
            .map_err(|error| format!("ChemcorePreviewSvg stream is not UTF-8: {error}"))
    }) {
        payload.svg = svg;
        payload.svg_was_supplied = true;
        populated = true;
    }
    if let Ok(cdxml) = storage_read_stream(storage, OLE_STREAM_SOURCE_CDXML).and_then(|bytes| {
        String::from_utf8(bytes)
            .map_err(|error| format!("ChemcoreSourceCdxml stream is not UTF-8: {error}"))
    }) {
        payload.text = Some(cdxml.clone());
        payload.cdxml = Some(cdxml);
        populated = true;
    }

    if populated {
        Ok(payload)
    } else {
        Err("Chemcore payload storage did not contain any readable streams.".to_string())
    }
}

unsafe fn text_payload_from_data_object(
    data_object: *mut c_void,
    format_name: &str,
    unicode: bool,
) -> Result<Option<String>, String> {
    text_payload_from_data_object_by_id(data_object, clipboard_format(format_name), unicode)
}

unsafe fn text_payload_from_data_object_by_id(
    data_object: *mut c_void,
    cf_format: u16,
    unicode: bool,
) -> Result<Option<String>, String> {
    if data_object.is_null() || cf_format == 0 {
        return Ok(None);
    }
    let get_data = (*(*(data_object.cast::<*const DataObjectVtbl>()))).get_data;
    let format = FORMATETC {
        cfFormat: cf_format,
        ptd: null_mut(),
        dwAspect: DVASPECT_CONTENT,
        lindex: -1,
        tymed: TYMED_HGLOBAL as u32,
    };
    let mut medium = STGMEDIUM::default();
    let hr = get_data(data_object, &format, &mut medium);
    if !hresult_succeeded(hr) {
        return Ok(None);
    }
    let result = if medium.tymed == TYMED_HGLOBAL as u32 && !medium.u.hGlobal.is_null() {
        read_hglobal_text(medium.u.hGlobal, unicode)
    } else {
        Ok(None)
    };
    ReleaseStgMedium(&mut medium);
    result
}

unsafe fn read_hglobal_text(handle: HGLOBAL, unicode: bool) -> Result<Option<String>, String> {
    if handle.is_null() {
        return Ok(None);
    }
    let size = GlobalSize(handle);
    if size == 0 {
        return Ok(None);
    }
    let source = GlobalLock(handle);
    if source.is_null() {
        return Err("Failed to lock HGLOBAL text payload.".to_string());
    }
    let result = if unicode {
        let len = size / std::mem::size_of::<u16>();
        let wide = std::slice::from_raw_parts(source.cast::<u16>(), len);
        let end = wide
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(wide.len());
        String::from_utf16(&wide[..end])
            .map(Some)
            .map_err(|error| format!("Failed to decode UTF-16 HGLOBAL payload: {error}"))
    } else {
        let bytes = std::slice::from_raw_parts(source.cast::<u8>(), size);
        let end = bytes
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(bytes.len());
        String::from_utf8(bytes[..end].to_vec())
            .map(Some)
            .map_err(|error| format!("Failed to decode UTF-8 HGLOBAL payload: {error}"))
    };
    GlobalUnlock(handle);
    result
}

unsafe extern "system" fn ole_object_do_verb(
    this: *mut c_void,
    verb: i32,
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
    log_ole_event(&format!("IOleObject::DoVerb({verb})"));
    match launch_desktop_for_object(object) {
        Ok(()) => {
            log_ole_event(&format!("IOleObject::DoVerb({verb}) -> 0x00000000"));
            S_OK
        }
        Err(error) => {
            log_ole_event(&format!("DoVerb({verb}) failed: {error}"));
            OLE_E_NOTRUNNING
        }
    }
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
    let requested = *(size.cast::<SIZE>());
    let natural = (*object).payload.extent_himetric();
    if requested.cx == DEFAULT_OBJECT_WIDTH_HIMETRIC
        && requested.cy == DEFAULT_OBJECT_HEIGHT_HIMETRIC
        && (natural.cx != requested.cx || natural.cy != requested.cy)
    {
        (*object).extent_himetric = natural;
    } else {
        (*object).extent_himetric = requested;
    }
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
    this: *mut c_void,
    sink: *mut c_void,
    connection: *mut u32,
) -> i32 {
    if connection.is_null() {
        return E_POINTER;
    }
    *connection = 0;
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    if (*object).ole_advise_holder.is_null() {
        let hr = CreateOleAdviseHolder(&mut (*object).ole_advise_holder);
        if !hresult_succeeded(hr) {
            return hr;
        }
    }
    let holder_vtbl = *((*object)
        .ole_advise_holder
        .cast::<*const OleAdviseHolderVtbl>());
    ((*holder_vtbl).advise)((*object).ole_advise_holder, sink, connection)
}

unsafe extern "system" fn ole_object_unadvise(this: *mut c_void, connection: u32) -> i32 {
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() || (*object).ole_advise_holder.is_null() {
        return E_POINTER;
    }
    let holder_vtbl = *((*object)
        .ole_advise_holder
        .cast::<*const OleAdviseHolderVtbl>());
    ((*holder_vtbl).unadvise)((*object).ole_advise_holder, connection)
}

unsafe extern "system" fn ole_object_enum_advise(
    this: *mut c_void,
    enum_advise: *mut *mut c_void,
) -> i32 {
    if enum_advise.is_null() {
        return E_POINTER;
    }
    *enum_advise = null_mut();
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() || (*object).ole_advise_holder.is_null() {
        return S_FALSE;
    }
    let holder_vtbl = *((*object)
        .ole_advise_holder
        .cast::<*const OleAdviseHolderVtbl>());
    ((*holder_vtbl).enum_advise)((*object).ole_advise_holder, enum_advise)
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
    if !draw_payload_preview(
        _draw_dc as HDC,
        &*(bounds.cast::<RECT>()),
        &(*object).payload,
    ) {
        draw_placeholder_preview(_draw_dc as HDC, &*(bounds.cast::<RECT>()));
    }
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
    this: *mut c_void,
    aspects: u32,
    advf: u32,
    sink: *mut c_void,
) -> i32 {
    let object = owner_from_part::<ViewObject2Vtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    com_release((*object).view_advise_sink);
    (*object).view_advise_sink = null_mut();
    (*object).view_advise_aspects = aspects;
    (*object).view_advise_flags = advf;
    if !sink.is_null() {
        com_add_ref(sink);
        (*object).view_advise_sink = sink;
    }
    log_ole_event(&format!(
        "IViewObject2::SetAdvise(aspects=0x{aspects:X}, advf=0x{advf:X}, sink={})",
        if sink.is_null() { "null" } else { "set" }
    ));
    S_OK
}

unsafe extern "system" fn view_object_get_advise(
    this: *mut c_void,
    aspects: *mut u32,
    advf: *mut u32,
    sink: *mut *mut c_void,
) -> i32 {
    let object = owner_from_part::<ViewObject2Vtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    if !aspects.is_null() {
        *aspects = (*object).view_advise_aspects;
    }
    if !advf.is_null() {
        *advf = (*object).view_advise_flags;
    }
    if !sink.is_null() {
        *sink = (*object).view_advise_sink;
        com_add_ref(*sink);
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

unsafe fn launch_desktop_for_object(object: *mut ChemcoreOleObject) -> Result<(), String> {
    if object.is_null() {
        return Err("Cannot launch OLE editor for a null object.".to_string());
    }
    if let Some(payload_path) = ole_edit_session_path_for_object(object) {
        log_ole_event(&format!(
            "Reusing OLE edit session payload at {}",
            payload_path.display()
        ));
        write_ole_edit_session_notify_file(&payload_path)?;
        let desktop_exe = resolve_desktop_exe()?;
        ensure_desktop_dev_server_for_debug_exe(&desktop_exe)?;
        log_ole_event(&format!(
            "Activating Chemcore desktop for existing OLE edit session from {}",
            desktop_exe.display()
        ));
        unsafe {
            AllowSetForegroundWindow(ASFW_ANY_PROCESS);
        }
        launch_desktop_process(&desktop_exe, &payload_path)?;
        return Ok(());
    }
    let session_id = format!(
        "{}-{}-{}",
        OLE_EDIT_SESSION_PREFIX,
        std::process::id(),
        monotonic_millis()
    );
    let payload_path = env::temp_dir().join(format!("{session_id}.ccjs"));
    let payload_json = ole_edit_session_payload_json(&(*object).payload)?;
    std::fs::write(&payload_path, payload_json)
        .map_err(|error| format!("Failed to write temporary OLE edit payload: {error}"))?;
    write_ole_edit_session_notify_file(&payload_path)?;
    log_ole_event(&format!(
        "Wrote OLE edit session {session_id} payload to {}",
        payload_path.display()
    ));
    register_ole_edit_session(session_id, payload_path.clone(), object)?;

    let desktop_exe = resolve_desktop_exe()?;
    ensure_desktop_dev_server_for_debug_exe(&desktop_exe)?;

    log_ole_event(&format!(
        "Launching Chemcore desktop from {}",
        desktop_exe.display()
    ));
    unsafe {
        AllowSetForegroundWindow(ASFW_ANY_PROCESS);
    }
    launch_desktop_process(&desktop_exe, &payload_path)?;
    Ok(())
}

fn resolve_desktop_exe() -> Result<PathBuf, String> {
    if let Ok(override_path) = env::var(CHEMCORE_DESKTOP_ENV) {
        let override_path = PathBuf::from(override_path);
        if override_path.exists() {
            log_ole_event(&format!(
                "Using {CHEMCORE_DESKTOP_ENV} desktop executable at {}",
                override_path.display()
            ));
            return Ok(override_path);
        }
        log_ole_event(&format!(
            "{CHEMCORE_DESKTOP_ENV} points to missing desktop executable at {}",
            override_path.display()
        ));
    }

    let server_path = current_server_path()?;
    let candidates = desktop_exe_candidates_for_server_path(&server_path);
    if let Some(candidate) = candidates.iter().find(|path| path.exists()) {
        return Ok(candidate.clone());
    }

    let searched = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "Chemcore desktop executable was not found. Searched: {searched}"
    ))
}

fn desktop_exe_candidates_for_server_path(server_path: &PathBuf) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let Some(server_dir) = server_path.parent() else {
        return candidates;
    };

    push_desktop_exe_candidates(&mut candidates, &server_dir.to_path_buf());
    if server_dir
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("resources"))
    {
        if let Some(app_dir) = server_dir.parent() {
            push_desktop_exe_candidates(&mut candidates, &app_dir.to_path_buf());
        }
    }
    candidates
}

fn push_desktop_exe_candidates(candidates: &mut Vec<PathBuf>, dir: &PathBuf) {
    for name in [CHEMCORE_DESKTOP_EXE_NAME, "Chemcore.exe", "chemcore.exe"] {
        let candidate = dir.join(name);
        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }
}

fn launch_desktop_process(desktop_exe: &PathBuf, payload_path: &PathBuf) -> Result<(), String> {
    let mut command = Command::new(desktop_exe);
    command.arg(payload_path);
    if let Some(parent) = desktop_exe.parent() {
        command.current_dir(parent);
    }

    match command.spawn() {
        Ok(_) => Ok(()),
        Err(error) => {
            log_ole_event(&format!(
                "CreateProcess launch failed for {}: {error}",
                desktop_exe.display()
            ));
            shell_execute_desktop(desktop_exe, payload_path).map_err(|shell_error| {
                format!(
                    "Failed to launch Chemcore desktop with CreateProcess ({error}) and ShellExecuteW ({shell_error})"
                )
            })
        }
    }
}

fn shell_execute_desktop(desktop_exe: &PathBuf, payload_path: &PathBuf) -> Result<(), String> {
    let operation = wide_null("open");
    let file = wide_path_null(desktop_exe);
    let parameters = wide_null(&quote_path(payload_path));
    let directory = desktop_exe
        .parent()
        .map(|parent| wide_path_null(&parent.to_path_buf()));
    let directory_ptr = directory
        .as_ref()
        .map(|value| value.as_ptr())
        .unwrap_or(null());
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            parameters.as_ptr(),
            directory_ptr,
            SW_SHOWNORMAL,
        )
    } as isize;
    if result > 32 {
        log_ole_event(&format!(
            "ShellExecuteW launch succeeded for {}",
            desktop_exe.display()
        ));
        return Ok(());
    }
    Err(format!("ShellExecuteW returned {result}"))
}

fn ensure_desktop_dev_server_for_debug_exe(desktop_exe: &PathBuf) -> Result<(), String> {
    let Some(debug_dir) = desktop_exe.parent() else {
        return Ok(());
    };
    if debug_dir.file_name().and_then(|name| name.to_str()) != Some("debug") {
        return Ok(());
    }
    let Some(target_dir) = debug_dir.parent() else {
        return Ok(());
    };
    if target_dir.file_name().and_then(|name| name.to_str()) != Some("target") {
        return Ok(());
    }
    let Some(repo_root) = target_dir.parent() else {
        return Ok(());
    };
    let server_script = repo_root.join("scripts").join("desktop-dev-server.mjs");
    if !server_script.exists() {
        return Ok(());
    }
    if desktop_dev_server_is_ready() {
        return Ok(());
    }

    log_ole_event(&format!(
        "Starting desktop dev server from {}",
        server_script.display()
    ));
    Command::new("node")
        .arg(server_script)
        .current_dir(repo_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW_FLAG)
        .spawn()
        .map_err(|error| format!("Failed to start Chemcore desktop dev server: {error}"))?;

    for _ in 0..30 {
        if desktop_dev_server_is_ready() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(format!(
        "Chemcore desktop dev server did not start at http://{DESKTOP_DEV_SERVER_ADDR}/"
    ))
}

fn desktop_dev_server_is_ready() -> bool {
    let Ok(addr) = DESKTOP_DEV_SERVER_ADDR.parse::<SocketAddr>() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok()
}

fn log_ole_event(message: &str) {
    let line = format!(
        "[{} pid={}] {message}\n",
        monotonic_millis(),
        std::process::id()
    );
    let temp_path = env::temp_dir().join("chemcore-office.log");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(temp_path)
        .and_then(|mut file| file.write_all(line.as_bytes()));

    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(parent.join("chemcore-office.log"))
                .and_then(|mut file| file.write_all(line.as_bytes()));
        }
    }
}

fn monotonic_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
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
    println!("  chemcore-office.exe --write-word-docx-payload <payload.json> <output.docx>");
    println!("  chemcore-office.exe --write-emf-payload <payload.json> <output.emf>");
    println!("  chemcore-office.exe --write-preview-bounds-payload <payload.json> <output.json>");
    println!("  chemcore-office.exe --serve");
    println!();
    println!("COM may launch this executable with -Embedding or /Embedding.");
}
