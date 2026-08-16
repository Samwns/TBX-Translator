#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::process::Stdio;
#[cfg(target_os = "linux")]
use std::sync::OnceLock;

#[derive(Clone, Copy)]
pub enum AppSound {
    Click,
    Success,
    Cancel,
    Error,
    Notification,
}

impl AppSound {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn file_name(self) -> &'static str {
        match self {
            Self::Click => "click.wav",
            Self::Success => "success.wav",
            Self::Cancel => "cancel.wav",
            Self::Error => "error.wav",
            Self::Notification => "notification.wav",
        }
    }

    #[cfg(target_os = "windows")]
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Click => include_bytes!("../assets/sounds/click.wav"),
            Self::Success => include_bytes!("../assets/sounds/success.wav"),
            Self::Cancel => include_bytes!("../assets/sounds/cancel.wav"),
            Self::Error => include_bytes!("../assets/sounds/error.wav"),
            Self::Notification => include_bytes!("../assets/sounds/notification.wav"),
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn command_in_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| directory.join(name).is_file())
    })
}

#[cfg(target_os = "linux")]
fn linux_player() -> Option<&'static str> {
    static PLAYER: OnceLock<Option<&'static str>> = OnceLock::new();
    *PLAYER.get_or_init(|| {
        ["pw-play", "paplay", "aplay"]
            .into_iter()
            .find(|name| command_in_path(name))
    })
}

#[cfg(target_os = "linux")]
fn play_native(sound: AppSound) {
    let Some(player) = linux_player() else { return };
    let path = crate::paths::asset_path("sounds").join(sound.file_name());
    let mut command = crate::paths::hidden_command(player);
    if player == "aplay" {
        command.arg("-q");
    }
    let _ = command
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

#[cfg(target_os = "macos")]
fn play_native(sound: AppSound) {
    if !command_in_path("afplay") {
        return;
    }
    let path = crate::paths::asset_path("sounds").join(sound.file_name());
    let _ = crate::paths::hidden_command("afplay")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

#[cfg(target_os = "windows")]
fn play_native(sound: AppSound) {
    const SND_ASYNC: u32 = 0x0001;
    const SND_NODEFAULT: u32 = 0x0002;
    const SND_MEMORY: u32 = 0x0004;
    #[link(name = "winmm")]
    extern "system" {
        fn PlaySoundA(sound: *const u8, module: *mut std::ffi::c_void, flags: u32) -> i32;
    }
    unsafe {
        PlaySoundA(
            sound.bytes().as_ptr(),
            std::ptr::null_mut(),
            SND_ASYNC | SND_NODEFAULT | SND_MEMORY,
        );
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn play_native(_sound: AppSound) {}

pub fn play(sound: AppSound, enabled: bool) {
    if enabled {
        play_native(sound);
    }
}
