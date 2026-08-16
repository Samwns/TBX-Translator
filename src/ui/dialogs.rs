use std::path::PathBuf;


/// Diálogo seguro para seleção do executável ou arquivo PCK do jogo.
pub fn pick_game_file() -> Result<Option<PathBuf>, String> {
    #[cfg(target_os = "linux")]
    {
        // 1. Tenta zenity (comum no GNOME/XFCE/Debian/Ubuntu)
        if let Ok(result) = crate::paths::hidden_command("zenity")
            .args(["--file-selection", "--title=Selecione o Executável ou PCK do Jogo"])
            .output()
        {
            if result.status.success() {
                let path = String::from_utf8_lossy(&result.stdout).trim().to_owned();
                return Ok((!path.is_empty()).then(|| PathBuf::from(path)));
            }
            return Ok(None); // Usuário cancelou ou fechou
        }

        // 2. Tenta kdialog (KDE Plasma)
        if let Ok(result) = crate::paths::hidden_command("kdialog")
            .args(["--getopenfilename", ".", "* | Todos os arquivos", "--title", "Selecione o Executável ou PCK do Jogo"])
            .output()
        {
            if result.status.success() {
                let path = String::from_utf8_lossy(&result.stdout).trim().to_owned();
                return Ok((!path.is_empty()).then(|| PathBuf::from(path)));
            }
            return Ok(None);
        }
    }

    // Fallback multiplataforma (rfd) protegido contra pânico/abort
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        rfd::FileDialog::new()
            .set_title("Selecione o Executável ou PCK do Jogo")
            .pick_file()
    }))
    .map_err(|_| "O seletor de arquivos falhou. Digite ou cole o caminho do jogo no campo acima.".to_string())
}

/// Diálogo seguro para seleção de pastas.
pub fn pick_folder(title: &str) -> Result<Option<PathBuf>, String> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(result) = crate::paths::hidden_command("zenity")
            .args(["--file-selection", "--directory", &format!("--title={title}")])
            .output()
        {
            if result.status.success() {
                let path = String::from_utf8_lossy(&result.stdout).trim().to_owned();
                return Ok((!path.is_empty()).then(|| PathBuf::from(path)));
            }
            return Ok(None);
        }

        if let Ok(result) = crate::paths::hidden_command("kdialog")
            .args(["--getexistingdirectory", ".", "--title", title])
            .output()
        {
            if result.status.success() {
                let path = String::from_utf8_lossy(&result.stdout).trim().to_owned();
                return Ok((!path.is_empty()).then(|| PathBuf::from(path)));
            }
            return Ok(None);
        }
    }

    let title_owned = title.to_string();
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        rfd::FileDialog::new()
            .set_title(&title_owned)
            .pick_folder()
    }))
    .map_err(|_| "O seletor de pastas falhou.".to_string())
}

/// Diálogo seguro para seleção de arquivos de fonte (TTF, OTF, WOFF).
pub fn pick_font_file(title: &str) -> Result<Option<PathBuf>, String> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(result) = crate::paths::hidden_command("zenity")
            .args([
                "--file-selection",
                &format!("--title={title}"),
                "--file-filter=Fontes (*.ttf, *.otf, *.woff, *.woff2) | *.ttf *.otf *.woff *.woff2 *.TTF *.OTF *.WOFF *.WOFF2",
                "--file-filter=Todos os arquivos | *",
            ])
            .output()
        {
            if result.status.success() {
                let path = String::from_utf8_lossy(&result.stdout).trim().to_owned();
                return Ok((!path.is_empty()).then(|| PathBuf::from(path)));
            }
            return Ok(None);
        }

        if let Ok(result) = crate::paths::hidden_command("kdialog")
            .args([
                "--getopenfilename",
                ".",
                "*.ttf *.otf *.woff *.woff2 | Fontes (*.ttf, *.otf, *.woff, *.woff2)",
                "--title",
                title,
            ])
            .output()
        {
            if result.status.success() {
                let path = String::from_utf8_lossy(&result.stdout).trim().to_owned();
                return Ok((!path.is_empty()).then(|| PathBuf::from(path)));
            }
            return Ok(None);
        }
    }

    let title_owned = title.to_string();
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        rfd::FileDialog::new()
            .set_title(&title_owned)
            .add_filter("Fontes (*.ttf, *.otf, *.woff, *.woff2)", &["ttf", "otf", "woff", "woff2"])
            .pick_file()
    }))
    .map_err(|_| "O seletor de fontes falhou.".to_string())
}
