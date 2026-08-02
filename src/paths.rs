use std::path::PathBuf;

/// Directory containing the executable and its companion resources.
/// During `cargo run`, this falls back to the project directory.
pub fn app_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
        .filter(|path| path.join("assets").is_dir() || path.join("unity_static_extractor").is_dir())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn asset_path(name: &str) -> PathBuf {
    app_root().join("assets").join(name)
}
