#![allow(dead_code)]

use std::path::PathBuf;

pub fn fixture_path(name: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tracked = manifest_dir
        .join("tests")
        .join("fixtures")
        .join("cdxml")
        .join(name);
    if tracked.exists() {
        return tracked;
    }
    let repo_root = manifest_dir.join("../..");
    let published = repo_root.join(name);
    if published.exists() {
        return published;
    }
    repo_root.join("tmp").join(name)
}

pub fn read_cdxml_fixture(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name)).unwrap_or_else(|error| panic!("{name}: {error}"))
}

pub fn cdxml_fixture_exists(name: &str) -> bool {
    fixture_path(name).exists()
}

pub fn read_optional_cdxml_fixture(name: &str) -> Option<String> {
    let path = fixture_path(name);
    match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!(
                "skipping external CDXML fixture test; missing {}",
                path.display()
            );
            None
        }
        Err(error) => panic!("{name}: {error}"),
    }
}
