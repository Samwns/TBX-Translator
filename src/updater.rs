use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;

use crate::types::UiMsg;

const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/Samwns/TBX-Translator/releases/latest";

#[derive(Clone, Debug, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    pub digest: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub name: String,
    #[serde(default)]
    pub body: String,
    pub html_url: String,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Clone, Copy, Debug)]
enum InstallKind {
    #[cfg(target_os = "windows")]
    WindowsInstaller,
    #[cfg(target_os = "windows")]
    WindowsPortable,
    #[cfg(target_os = "linux")]
    AppImage,
    #[cfg(target_os = "linux")]
    Deb,
    #[cfg(target_os = "linux")]
    Rpm,
    #[cfg(target_os = "linux")]
    Arch,
}

pub fn current_version() -> String {
    match option_env!("TBX_BUILD_NUMBER") {
        Some(build) if !build.is_empty() => {
            let base = env!("CARGO_PKG_VERSION")
                .split('-')
                .next()
                .unwrap_or(env!("CARGO_PKG_VERSION"));
            format!("{}-build-{}", base, build)
        }
        _ => env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn numeric_version(value: &str) -> Vec<u64> {
    value
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|piece| !piece.is_empty())
        .filter_map(|piece| piece.parse().ok())
        .collect()
}

pub fn is_newer(remote: &str) -> bool {
    let remote_numbers = numeric_version(remote);
    let current_numbers = numeric_version(&current_version());
    let count = remote_numbers.len().max(current_numbers.len());
    for index in 0..count {
        let remote_part = remote_numbers.get(index).copied().unwrap_or(0);
        let current_part = current_numbers.get(index).copied().unwrap_or(0);
        if remote_part != current_part {
            return remote_part > current_part;
        }
    }

    // A stable release with the same numeric version supersedes a local alpha.
    current_version().contains("alpha") && !remote.contains("alpha")
}

pub async fn check_latest() -> Result<ReleaseInfo, String> {
    Client::new()
        .get(LATEST_RELEASE_URL)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", format!("TBX-Translator/{}", current_version()))
        .send()
        .await
        .map_err(|error| format!("Não foi possível consultar o GitHub: {error}"))?
        .error_for_status()
        .map_err(|error| format!("O GitHub recusou a consulta: {error}"))?
        .json::<ReleaseInfo>()
        .await
        .map_err(|error| format!("Resposta de atualização inválida: {error}"))
}

fn detect_install_kind() -> Result<InstallKind, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("Não foi possível localizar o programa: {error}"))?;

    #[cfg(target_os = "windows")]
    {
        let location = executable.to_string_lossy().to_ascii_lowercase();
        if location.contains("\\program files\\") || location.contains("/program files/") {
            Ok(InstallKind::WindowsInstaller)
        } else {
            Ok(InstallKind::WindowsPortable)
        }
    }

    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("APPIMAGE").is_some() {
            return Ok(InstallKind::AppImage);
        }
        let os_release = fs::read_to_string("/etc/os-release")
            .unwrap_or_default()
            .to_ascii_lowercase();
        if os_release.contains("id=arch") || os_release.contains("id_like=arch") {
            Ok(InstallKind::Arch)
        } else if os_release.contains("id=fedora") || os_release.contains("id_like=\"rhel fedora\"") {
            Ok(InstallKind::Rpm)
        } else if executable.starts_with("/opt/tbx-translator")
            || os_release.contains("id=ubuntu")
            || os_release.contains("id=debian")
            || os_release.contains("id_like=debian")
        {
            Ok(InstallKind::Deb)
        } else {
            Err("Formato da instalação não reconhecido. Use AppImage, pacote do sistema ou ZIP do Windows.".to_string())
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = executable;
        Err("Atualização automática ainda não está disponível neste sistema.".to_string())
    }
}

fn asset_name(kind: InstallKind) -> &'static str {
    match kind {
        #[cfg(target_os = "windows")]
        InstallKind::WindowsInstaller => "TBX-Translator-Setup.exe",
        #[cfg(target_os = "windows")]
        InstallKind::WindowsPortable => "TBX-Translator-Windows-x64.zip",
        #[cfg(target_os = "linux")]
        InstallKind::AppImage => "TBX-Translator-x86_64.AppImage",
        #[cfg(target_os = "linux")]
        InstallKind::Deb => "TBX-Translator-Debian-Ubuntu-amd64.deb",
        #[cfg(target_os = "linux")]
        InstallKind::Rpm => "TBX-Translator-Fedora-x86_64.rpm",
        #[cfg(target_os = "linux")]
        InstallKind::Arch => "TBX-Translator-Arch-x86_64.pkg.tar.zst",
    }
}

async fn download_asset(
    asset: &ReleaseAsset,
    destination: &Path,
    tx: &Sender<UiMsg>,
) -> Result<(), String> {
    let client = Client::new();
    let mut response = client
        .get(&asset.browser_download_url)
        .header("Accept", "application/octet-stream")
        .header("User-Agent", format!("TBX-Translator/{}", current_version()))
        .send()
        .await
        .map_err(|error| format!("Falha ao iniciar o download: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Falha ao baixar a atualização: {error}"))?;

    let total = response.content_length().unwrap_or(asset.size);
    let mut downloaded = 0u64;
    let mut output = File::create(destination)
        .map_err(|error| format!("Não foi possível criar o arquivo temporário: {error}"))?;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("Download interrompido: {error}"))?
    {
        output
            .write_all(&chunk)
            .map_err(|error| format!("Não foi possível salvar a atualização: {error}"))?;
        downloaded += chunk.len() as u64;
        let _ = tx.send(UiMsg::UpdateProgress(downloaded, total));
    }
    output
        .flush()
        .map_err(|error| format!("Não foi possível finalizar o download: {error}"))?;
    Ok(())
}

fn verify_digest(path: &Path, expected: Option<&str>) -> Result<(), String> {
    let expected = expected
        .and_then(|digest| digest.strip_prefix("sha256:"))
        .ok_or_else(|| "A release não informou o SHA-256 do arquivo.".to_string())?;
    let mut file = File::open(path)
        .map_err(|error| format!("Não foi possível verificar a atualização: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Falha ao calcular SHA-256: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err("A atualização baixada falhou na verificação SHA-256 e foi recusada.".to_string())
    }
}

fn temp_update_dir() -> Result<PathBuf, String> {
    let directory = std::env::temp_dir().join(format!("tbx-update-{}", std::process::id()));
    if directory.exists() {
        fs::remove_dir_all(&directory)
            .map_err(|error| format!("Não foi possível limpar a atualização anterior: {error}"))?;
    }
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Não foi possível preparar a atualização: {error}"))?;
    Ok(directory)
}

#[cfg(target_os = "linux")]
fn launch_linux_updater(kind: InstallKind, package: &Path, temp_dir: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let script = temp_dir.join("tbx-apply-update.sh");
    let script_body = match kind {
        InstallKind::AppImage => r#"#!/bin/sh
pid="$1"; package="$2"; target="$3"
while kill -0 "$pid" 2>/dev/null; do sleep 0.2; done
chmod +x "$package" || exit 1
mv -f "$package" "$target" || exit 1
rm -f -- "$0"
exec "$target"
"#,
        InstallKind::Deb => r#"#!/bin/sh
pid="$1"; package="$2"; target="$3"
while kill -0 "$pid" 2>/dev/null; do sleep 0.2; done
pkexec dpkg -i "$package" || exit 1
rm -rf -- "$(dirname "$package")"
exec "$target"
"#,
        InstallKind::Rpm => r#"#!/bin/sh
pid="$1"; package="$2"; target="$3"
while kill -0 "$pid" 2>/dev/null; do sleep 0.2; done
pkexec rpm -U --replacepkgs "$package" || exit 1
rm -rf -- "$(dirname "$package")"
exec "$target"
"#,
        InstallKind::Arch => r#"#!/bin/sh
pid="$1"; package="$2"; target="$3"
while kill -0 "$pid" 2>/dev/null; do sleep 0.2; done
pkexec pacman -U --noconfirm "$package" || exit 1
rm -rf -- "$(dirname "$package")"
exec "$target"
"#,
    };
    fs::write(&script, script_body)
        .map_err(|error| format!("Não foi possível criar o aplicador: {error}"))?;
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("Não foi possível autorizar o aplicador: {error}"))?;

    let current_exe = if matches!(kind, InstallKind::AppImage) {
        std::env::var_os("APPIMAGE")
            .map(PathBuf::from)
            .or_else(|| std::env::current_exe().ok())
            .ok_or_else(|| "Não foi possível localizar o AppImage atual.".to_string())?
    } else {
        std::env::current_exe()
            .map_err(|error| format!("Não foi possível localizar o executável: {error}"))?
    };
    Command::new(&script)
        .arg(std::process::id().to_string())
        .arg(package)
        .arg(current_exe)
        .spawn()
        .map_err(|error| format!("Não foi possível iniciar o aplicador: {error}"))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn extract_portable_zip(archive: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(archive)
        .map_err(|error| format!("Não foi possível abrir o ZIP: {error}"))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|error| format!("ZIP da atualização inválido: {error}"))?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)
            .map_err(|error| format!("Falha ao ler o ZIP: {error}"))?;
        let Some(enclosed) = entry.enclosed_name() else { continue };
        let relative: PathBuf = enclosed.components().skip(1).collect();
        if relative.as_os_str().is_empty() { continue; }
        let output = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output).map_err(|error| error.to_string())?;
        } else {
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let mut output_file = File::create(&output).map_err(|error| error.to_string())?;
            std::io::copy(&mut entry, &mut output_file).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_windows_updater(kind: InstallKind, package: &Path, temp_dir: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let current_exe = std::env::current_exe()
        .map_err(|error| format!("Não foi possível localizar o executável: {error}"))?;
    let target_dir = current_exe.parent()
        .ok_or_else(|| "Diretório do programa não encontrado.".to_string())?;
    let script = temp_dir.join("tbx-apply-update.ps1");
    let script_body = match kind {
        InstallKind::WindowsInstaller => r#"
param($ProcessId, $Package, $TargetDir, $Executable)
Wait-Process -Id $ProcessId -ErrorAction SilentlyContinue
$process = Start-Process -FilePath $Package -ArgumentList '/VERYSILENT','/CLOSEAPPLICATIONS','/SUPPRESSMSGBOXES','/NORESTART' -Verb RunAs -Wait -PassThru
if ($process.ExitCode -eq 0) { Start-Process -FilePath $Executable }
"#,
        InstallKind::WindowsPortable => r#"
param($ProcessId, $SourceDir, $TargetDir, $Executable)
Wait-Process -Id $ProcessId -ErrorAction SilentlyContinue
Get-ChildItem -LiteralPath $SourceDir | Copy-Item -Destination $TargetDir -Recurse -Force
Start-Process -FilePath $Executable
"#,
    };
    fs::write(&script, script_body)
        .map_err(|error| format!("Não foi possível criar o aplicador: {error}"))?;

    let source = if matches!(kind, InstallKind::WindowsPortable) {
        let extracted = temp_dir.join("portable");
        fs::create_dir_all(&extracted).map_err(|error| error.to_string())?;
        extract_portable_zip(package, &extracted)?;
        extracted
    } else {
        package.to_path_buf()
    };

    Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .arg(std::process::id().to_string())
        .arg(source)
        .arg(target_dir)
        .arg(&current_exe)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|error| format!("Não foi possível iniciar o aplicador: {error}"))?;
    Ok(())
}

pub async fn download_apply_and_restart(
    release: ReleaseInfo,
    tx: Sender<UiMsg>,
    language: String,
) -> Result<(), String> {
    let kind = detect_install_kind()?;
    let wanted_name = asset_name(kind);
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name.eq_ignore_ascii_case(wanted_name))
        .ok_or_else(|| format!("A release não contém o pacote {wanted_name}."))?;
    let temp_dir = temp_update_dir()?;
    let package = temp_dir.join(&asset.name);

    let _ = tx.send(UiMsg::UpdateStatus(format!(
        "{}: {}...",
        crate::i18n::t("baixando_atualizacao", &language),
        asset.name
    )));
    download_asset(asset, &package, &tx).await?;
    let _ = tx.send(UiMsg::UpdateStatus(crate::i18n::t(
        "verificando_integridade",
        &language,
    )));
    verify_digest(&package, asset.digest.as_deref())?;
    let _ = tx.send(UiMsg::UpdateStatus(crate::i18n::t(
        "aplicando_reiniciando",
        &language,
    )));

    #[cfg(target_os = "linux")]
    launch_linux_updater(kind, &package, &temp_dir)?;
    #[cfg(target_os = "windows")]
    launch_windows_updater(kind, &package, &temp_dir)?;

    // The helper can only replace the executable after this process exits.
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::numeric_version;

    #[test]
    fn parses_release_build_number() {
        assert_eq!(numeric_version("v0.0.2-build-20"), vec![0, 0, 2, 20]);
    }
}
