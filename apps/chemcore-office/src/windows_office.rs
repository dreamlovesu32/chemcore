use std::env;
use std::ffi::c_void;
use std::mem::zeroed;
use std::path::PathBuf;
use std::ptr::{null, null_mut};

use windows_sys::core::GUID;
use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Com::{
    CoInitializeEx, CoRegisterClassObject, CoRevokeClassObject, CoUninitialize,
    CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED, REGCLS_MULTIPLEUSE,
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

const S_OK: i32 = 0;
const E_POINTER: i32 = 0x80004003u32 as i32;
const E_NOINTERFACE: i32 = 0x80004002u32 as i32;
const E_NOTIMPL: i32 = 0x80004001u32 as i32;
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
    _riid: *const GUID,
    object: *mut *mut c_void,
) -> i32 {
    if !object.is_null() {
        *object = null_mut();
    }
    if !outer.is_null() {
        return CLASS_E_NOAGGREGATION;
    }
    E_NOTIMPL
}

unsafe extern "system" fn class_factory_lock_server(_this: *mut c_void, _lock: i32) -> i32 {
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
    println!("  chemcore-office.exe --serve");
    println!();
    println!("COM may launch this executable with -Embedding or /Embedding.");
}
