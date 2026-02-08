use eyre::ContextCompat;
use std::path::{Path, PathBuf};

fn main() -> eyre::Result<()> {
    println!("cargo:rerun-if-env-changed=PROXY_DLL");
    let dll = std::env::var("PROXY_DLL").unwrap_or("version.dll".to_string());

    // First check if the provided DLL is already a path
    let final_path = if std::fs::exists(&dll)? {
        PathBuf::from(dll)
    } else {
        let existing_path = Path::new("C:\\Windows\\System32\\");
        existing_path.join(&dll)
    };

    forward_dll::forward_dll(final_path.to_str().context("Could not convert to string")?).unwrap();
    Ok(())
}
