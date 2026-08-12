use super::*;

pub(super) fn register(scope: RegistrationScope) -> Result<(), String> {
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
    for prog_id in FILE_ASSOCIATION_PROG_IDS {
        set_key_default(
            root,
            &classes_path(&format!("{prog_id}\\CLSID")),
            CLSID_STRING,
        )?;
        register_std_file_editing(root, prog_id, &server_command)?;
    }

    println!(
        "Registered {DOCUMENT_DISPLAY_NAME} for {} at {}",
        scope.label(),
        scope.prefix()
    );
    println!("CLSID: {CLSID_STRING}");
    println!("Server: {}", server_path.display());
    Ok(())
}

pub(super) fn register_std_file_editing(
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

pub(super) fn register_data_formats(root: HKEY, clsid_path: &str) -> Result<(), String> {
    let data_formats = format!("{clsid_path}\\DataFormats");
    set_key_default(
        root,
        &format!("{data_formats}\\DefaultFile"),
        FORMAT_CHEMSEMA_NATIVE,
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
        &format!("{FORMAT_CHEMSEMA_NATIVE},1,1,1"),
    )?;
    set_key_default(
        root,
        &format!("{get_set}\\6"),
        &format!("{FORMAT_CHEMSEMA_DOCUMENT_JSON},1,1,1"),
    )?;
    Ok(())
}

pub(super) fn unregister(scope: RegistrationScope) -> Result<(), String> {
    let root = scope.root();
    delete_tree(root, &classes_path(PROG_ID))?;
    delete_tree(root, &classes_path(VERSIONED_PROG_ID))?;
    delete_tree(root, &classes_path(&format!("CLSID\\{CLSID_STRING}")))?;
    for prog_id in FILE_ASSOCIATION_PROG_IDS {
        delete_tree(root, &classes_path(&format!("{prog_id}\\CLSID")))?;
        delete_tree(
            root,
            &classes_path(&format!("{prog_id}\\Protocol\\StdFileEditing")),
        )?;
    }
    println!(
        "Unregistered {DOCUMENT_DISPLAY_NAME} for {} from {}",
        scope.label(),
        scope.prefix()
    );
    Ok(())
}

pub(super) fn print_registration() -> Result<(), String> {
    let server_path = current_server_path()?;
    println!("{APP_NAME} Office/OLE registration");
    println!("Display name: {DOCUMENT_DISPLAY_NAME}");
    println!("ProgID: {PROG_ID}");
    println!("Versioned ProgID: {VERSIONED_PROG_ID}");
    println!("CLSID: {CLSID_STRING}");
    println!("Server: {}", server_path.display());
    Ok(())
}

pub(super) fn current_server_path() -> Result<PathBuf, String> {
    env::current_exe().map_err(|error| format!("Failed to resolve chemsema-office.exe: {error}"))
}

pub(super) fn quote_path(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

pub(super) fn classes_path(path: &str) -> String {
    format!("Software\\Classes\\{path}")
}

pub(super) fn create_key(root: HKEY, subkey: &str) -> Result<(), String> {
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

pub(super) fn set_key_default(root: HKEY, subkey: &str, value: &str) -> Result<(), String> {
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

pub(super) fn set_named_string(
    root: HKEY,
    subkey: &str,
    name: &str,
    value: &str,
) -> Result<(), String> {
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

pub(super) fn delete_tree(root: HKEY, subkey: &str) -> Result<(), String> {
    let subkey_w = wide_null(subkey);
    let status = unsafe { RegDeleteTreeW(root, subkey_w.as_ptr()) };
    if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
        return Ok(());
    }
    Err(format!("Failed to delete registry tree {subkey}: {status}"))
}

pub(super) fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(super) fn wide_path_null(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
