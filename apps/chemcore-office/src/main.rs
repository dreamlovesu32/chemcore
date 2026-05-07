#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
mod windows_office;

#[cfg(not(target_os = "windows"))]
mod windows_office {
    pub fn run() -> Result<(), String> {
        Err("chemcore-office is currently only supported on Windows.".to_string())
    }
}

fn main() {
    if let Err(error) = windows_office::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
