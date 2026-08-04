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

/// Cria um `std::process::Command` com a flag `CREATE_NO_WINDOW` no Windows,
/// impedindo que janelas pretas de terminal (cmd.exe) fiquem piscando na interface.
pub fn hidden_command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    #[allow(unused_mut)]
    let mut cmd = std::process::Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd
}
