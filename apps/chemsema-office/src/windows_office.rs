use std::collections::BTreeMap;
use std::env;
use std::ffi::{c_void, OsString};
use std::io::{Cursor, Read, Write};
use std::mem::zeroed;
use std::net::{SocketAddr, TcpStream};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime};

use base64::Engine as _;
use chemsema_engine::PT_PER_CM;
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
    CreateOleAdviseHolder, OleCreateFromData, OleFlushClipboard, OleGetClipboard, OleInitialize,
    OleRegEnumVerbs, OleRegGetMiscStatus, OleRegGetUserType, OleSave, OleSetClipboard,
    OleUninitialize, ReleaseStgMedium, CF_ENHMETAFILE, CF_METAFILEPICT, OBJECTDESCRIPTOR,
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

mod clipboard_formats;
mod data_object;
mod desktop_launch;
mod emf_preview;
mod ole_object;
mod payload;
mod persist_file;
mod presentation_storage;
mod registration;
mod server;
mod storage;
mod view_object;

use clipboard_formats::*;
use data_object::*;
use desktop_launch::*;
use emf_preview::{
    draw_payload_preview, draw_placeholder_preview, enhanced_metafile_bits_for_office_payload,
    enhanced_metafile_bits_for_payload, enhanced_metafile_for_office_payload,
    extent_himetric_for_payload, hglobal_for_metafile_pict, ole_presentation_stream_for_payload,
    preview_bounds_debug_report,
};
use ole_object::*;
pub(crate) use payload::write_emf_payload_json;
use payload::*;
use persist_file::*;
use presentation_storage::*;
use registration::*;
use server::*;
use storage::*;
use view_object::*;

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
        "--read-clipboard-payload" => {
            unsafe {
                FreeConsole();
            }
            let output_path = args.next().ok_or_else(|| {
                "--read-clipboard-payload requires an output JSON path.".to_string()
            })?;
            read_clipboard_payload(PathBuf::from(output_path))
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
        other => Err(format!("Unknown chemsema-office command: {other}")),
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClipboardPayload {
    chemsema_fragment_json: Option<String>,
    chemsema_document_json: Option<String>,
    render_list_json: Option<String>,
    cdxml: Option<String>,
    svg: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Clone)]
struct OleObjectPayload {
    chemsema_fragment_json: Option<String>,
    chemsema_document_json: String,
    document_was_supplied: bool,
    render_list_json: Option<String>,
    cdxml: Option<String>,
    svg: String,
    svg_was_supplied: bool,
    text: Option<String>,
}

impl OleObjectPayload {
    fn blank() -> Self {
        let chemsema_document_json =
            serde_json::to_string(&chemsema_engine::ChemSemaDocument::blank())
                .unwrap_or_else(|_| "{}".to_string());
        Self {
            chemsema_fragment_json: None,
            chemsema_document_json,
            document_was_supplied: false,
            render_list_json: None,
            cdxml: None,
            svg: String::from_utf8(ole_preview_svg_stream_payload()).unwrap_or_default(),
            svg_was_supplied: false,
            text: None,
        }
    }

    fn from_clipboard(payload: ClipboardPayload) -> Self {
        let default = Self::blank();
        let cdxml = payload.cdxml.filter(|value| !value.trim().is_empty());
        let supplied_svg = payload.svg.filter(|value| !value.trim().is_empty());
        let document_was_supplied = payload
            .chemsema_document_json
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
        let document_json = payload
            .chemsema_document_json
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(default.chemsema_document_json);
        let generated_svg = chemsema_engine::parse_document_json(&document_json)
            .ok()
            .map(|document| chemsema_engine::document_to_svg(&document))
            .filter(|value| !value.trim().is_empty());
        let has_preview_svg = supplied_svg.is_some() || generated_svg.is_some();
        Self {
            chemsema_fragment_json: payload.chemsema_fragment_json,
            chemsema_document_json: document_json,
            document_was_supplied,
            render_list_json: payload
                .render_list_json
                .filter(|value| !value.trim().is_empty()),
            cdxml: cdxml.clone(),
            svg: supplied_svg
                .clone()
                .or(generated_svg)
                .unwrap_or(default.svg),
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

    fn clipboard_payload(&self) -> ClipboardPayload {
        ClipboardPayload {
            chemsema_fragment_json: self.chemsema_fragment_json.clone(),
            chemsema_document_json: self
                .document_was_supplied
                .then(|| self.chemsema_document_json.clone()),
            render_list_json: self.render_list_json.clone(),
            cdxml: self.cdxml.clone(),
            svg: self.svg_was_supplied.then(|| self.svg.clone()),
            text: self.text.clone(),
        }
    }
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

#[repr(C)]
struct ChemSemaOleObject {
    unknown_vtbl: *const UnknownVtbl,
    ref_count: AtomicU32,
    data_object: InterfacePart<DataObjectVtbl>,
    persist_storage: InterfacePart<PersistStorageVtbl>,
    persist_file: InterfacePart<PersistFileVtbl>,
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
    current_file: Option<PathBuf>,
}

#[repr(C)]
struct InterfacePart<T> {
    vtbl: *const T,
    owner: *mut ChemSemaOleObject,
}

impl ChemSemaOleObject {
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
            persist_file: InterfacePart {
                vtbl: &PERSIST_FILE_VTBL,
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
            current_file: None,
        }
    }

    fn init_self_references(&mut self) {
        let owner = self as *mut ChemSemaOleObject;
        self.data_object.owner = owner;
        self.persist_storage.owner = owner;
        self.persist_file.owner = owner;
        self.ole_object.owner = owner;
        self.view_object2.owner = owner;
        self.runnable_object.owner = owner;
    }
}

impl Drop for ChemSemaOleObject {
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

#[repr(C)]
struct PersistFileVtbl {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    get_class_id: unsafe extern "system" fn(*mut c_void, *mut GUID) -> i32,
    is_dirty: unsafe extern "system" fn(*mut c_void) -> i32,
    load: unsafe extern "system" fn(*mut c_void, *const u16, u32) -> i32,
    save: unsafe extern "system" fn(*mut c_void, *const u16, i32) -> i32,
    save_completed: unsafe extern "system" fn(*mut c_void, *const u16) -> i32,
    get_cur_file: unsafe extern "system" fn(*mut c_void, *mut *mut u16) -> i32,
}

static PERSIST_FILE_VTBL: PersistFileVtbl = PersistFileVtbl {
    query_interface: part_query_interface::<PersistFileVtbl>,
    add_ref: part_add_ref::<PersistFileVtbl>,
    release: part_release::<PersistFileVtbl>,
    get_class_id: persist_file_get_class_id,
    is_dirty: persist_file_is_dirty,
    load: persist_file_load,
    save: persist_file_save,
    save_completed: persist_file_save_completed,
    get_cur_file: persist_file_get_cur_file,
};

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

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn ole_clipboard_formats_prefer_embedded_object_over_visual_defaults() {
        let payload = OleObjectPayload {
            chemsema_fragment_json: Some("{\"nodes\":[],\"bonds\":[]}".to_string()),
            chemsema_document_json:
                "{\"document\":{\"name\":\"chemsema\"},\"objects\":[],\"resources\":{}}".to_string(),
            document_was_supplied: true,
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
        assert!(format_names.contains(FORMAT_CHEMSEMA_NATIVE));
        assert!(format_names.contains(FORMAT_CHEMSEMA_DOCUMENT_JSON));
        assert!(format_names.contains(FORMAT_CDXML_MIME));
        assert!(format_names.contains("CF_ENHMETAFILE"));
        assert!(
            !format_names.contains(FORMAT_CHEMDRAW_INTERCHANGE),
            "ChemSema should not advertise ChemDraw's native clipboard format with a CDXML payload"
        );
        assert!(
            !format_names.contains(FORMAT_SVG_MIME),
            "Word's default Paste prefers SVG as a plain image instead of embedding the OLE object"
        );
        assert!(
            !format_names.contains(FORMAT_SVG),
            "Word's default Paste prefers SVG as a plain image instead of embedding the OLE object"
        );
        assert!(format_names.contains("CF_UNICODETEXT"));
        assert!(format_names.contains(CLIPBOARD_FORMAT_HTML));
    }

    #[test]
    fn clipboard_payload_without_svg_generates_preview_svg_from_document() {
        let document_json = serde_json::to_string(&chemsema_engine::ChemSemaDocument::blank())
            .expect("blank document should serialize");
        let payload = OleObjectPayload::from_clipboard(ClipboardPayload {
            chemsema_fragment_json: None,
            chemsema_document_json: Some(document_json),
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
    fn clipboard_html_carries_the_complete_portable_payload() {
        let payload = OleObjectPayload::from_clipboard(ClipboardPayload {
            chemsema_fragment_json: Some("{\"nodes\":[],\"bonds\":[]}".to_string()),
            chemsema_document_json: Some(
                serde_json::to_string(&chemsema_engine::ChemSemaDocument::blank()).unwrap(),
            ),
            render_list_json: None,
            cdxml: Some("<CDXML></CDXML>".to_string()),
            svg: None,
            text: Some("<CDXML></CDXML>".to_string()),
        });
        let html = clipboard_html(&payload);
        let marker = "data-chemsema-payload-base64=\"";
        let tail = html.split_once(marker).expect("payload marker").1;
        let encoded = tail.split_once('"').expect("payload terminator").0;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .expect("payload base64");
        let portable: ClipboardPayload =
            serde_json::from_slice(&decoded).expect("portable payload json");
        assert!(portable.chemsema_fragment_json.is_some());
        assert!(portable.chemsema_document_json.is_some());
        assert_eq!(portable.cdxml.as_deref(), Some("<CDXML></CDXML>"));
    }

    #[test]
    fn word_docx_uses_natural_orig_size_and_fitted_display_extent() {
        let document_json = serde_json::to_string(&chemsema_engine::ChemSemaDocument::blank())
            .expect("blank document should serialize");
        let payload = OleObjectPayload {
            chemsema_fragment_json: None,
            chemsema_document_json: document_json,
            document_was_supplied: true,
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
        let document_json = serde_json::to_string(&chemsema_engine::ChemSemaDocument::blank())
            .expect("blank document should serialize");
        let payload = OleObjectPayload {
            chemsema_fragment_json: None,
            chemsema_document_json: document_json,
            document_was_supplied: true,
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
        let dev_server = PathBuf::from(r"C:\ChemSemaDev\chemsema\target\debug\chemsema-office.exe");
        assert_eq!(
            desktop_exe_candidates_for_server_path(&dev_server)[0],
            PathBuf::from(r"C:\ChemSemaDev\chemsema\target\debug\chemsema-desktop.exe")
        );

        let resource_server =
            PathBuf::from(r"C:\Program Files\ChemSema\resources\chemsema-office.exe");
        let candidates = desktop_exe_candidates_for_server_path(&resource_server);
        assert!(candidates.contains(&PathBuf::from(
            r"C:\Program Files\ChemSema\resources\chemsema-desktop.exe"
        )));
        assert!(candidates.contains(&PathBuf::from(
            r"C:\Program Files\ChemSema\chemsema-desktop.exe"
        )));
        assert!(candidates.contains(&PathBuf::from(r"C:\Program Files\ChemSema\ChemSema.exe")));
    }

    fn assert_size_eq(actual: Option<SIZE>, expected: SIZE) {
        let actual = actual.expect("expected a presentation extent");
        assert_eq!(actual.cx, expected.cx);
        assert_eq!(actual.cy, expected.cy);
    }
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
