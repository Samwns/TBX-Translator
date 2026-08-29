use std::path::Path;
use std::fs;
use reqwest::Client;
use crate::types::UiMsg;

// A DLL do nosso Mod nativo em C# compilado
const TBX_INJECTOR_DLL: &[u8] = include_bytes!("../TBX_Injector/bin/Release/netstandard2.0/TBX_Injector.dll");
// A fonte TTF é compatível com TMP dinâmico em uma faixa muito maior de
// versões do Unity do que um AssetBundle pré-compilado.
const DEFAULT_FALLBACK_TTF: &[u8] = include_bytes!("../assets/NotoSansJP.ttf");

async fn download_and_extract(url: &str, dest_dir: &Path, tx: &std::sync::mpsc::Sender<UiMsg>) -> Result<(), String> {
    let _ = tx.send(UiMsg::Log(format!("Baixando: {}", url.split('/').last().unwrap_or("arquivo"))));
    
    let client = Client::builder().user_agent("TBX-Translator/1.0").build().unwrap();
    let resp = client.get(url).send().await.map_err(|e| format!("Falha no download: {}", e))?;
    
    if !resp.status().is_success() {
        return Err(format!("Erro HTTP: {}", resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| format!("Falha ao ler bytes: {}", e))?;
    let _ = tx.send(UiMsg::Log("Extraindo arquivos...".into()));

    extract_zip_bytes(bytes.to_vec(), dest_dir, tx)
}

fn extract_zip_bytes(bytes: Vec<u8>, dest_dir: &Path, _tx: &std::sync::mpsc::Sender<UiMsg>) -> Result<(), String> {
    let reader = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(reader).map_err(|e| format!("Erro lendo ZIP: {}", e))?;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
        let outpath = dest_dir.join(file.enclosed_name().unwrap_or(Path::new("")));

        if file.name().ends_with('/') {
            let _ = fs::create_dir_all(&outpath);
        } else {
            if let Some(p) = outpath.parent() {
                let _ = fs::create_dir_all(p);
            }
            let mut outfile = fs::File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub async fn install_dynamic_translation(
    executable: &str,
    translation_folder: &str,
    target_lang: &str,
    tx: std::sync::mpsc::Sender<UiMsg>,
    backend: &str, // "Mono" ou "IL2CPP"
    usar_bepinex_6: bool,
    substituir_todas_fontes_unity: bool,
) -> Result<(), String> {
    // TBX_Injector.dll is a Mono/BepInEx 5 plugin. BepInEx 6 and IL2CPP need
    // their own builds and loading the current DLL there silently fails.
    if backend != "Mono" {
        return Err(
            "A injeção dinâmica TBX atual suporta somente Unity Mono. Jogos IL2CPP precisam do plugin IL2CPP próprio; use a injeção nativa por enquanto."
                .into(),
        );
    }
    if usar_bepinex_6 {
        let _ = tx.send(UiMsg::Log(
            "[Aviso] BepInEx 6 foi solicitado, mas TBX_Injector.dll é compatível com BepInEx 5. Instalando BepInEx 5 para evitar uma instalação que não carrega o mod."
                .into(),
        ));
    }
    let _ = tx.send(UiMsg::Log(format!(
        "Iniciando Instalação Dinâmica BepInEx 5.4.23.5 ({})",
        backend
    )));

    let exe_path = Path::new(executable);
    let game_dir = exe_path.parent().ok_or("Caminho do jogo inválido")?;

    let out_dir = crate::unity_extractor::output_folder(executable, translation_folder, target_lang);
    let translated_json = out_dir.join("translated_texts.json");

    if !translated_json.exists() {
        return Err("JSON de tradução não encontrado! Realize a extração/tradução primeiro.".into());
    }

    let bepinex_url = "https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.5/BepInEx_win_x64_5.4.23.5.zip";
    download_and_extract(bepinex_url, game_dir, &tx).await?;

    // Injetar o nosso próprio Mod TBX_Injector
    let plugins_dir = game_dir.join("BepInEx").join("plugins");
    let _ = fs::create_dir_all(&plugins_dir);
    
    let tbx_mod_path = plugins_dir.join("TBX_Injector.dll");
    let _ = tx.send(UiMsg::Log("Injetando Mod TBX_Injector nativo...".into()));
    fs::write(&tbx_mod_path, TBX_INJECTOR_DLL).map_err(|e| format!("Erro ao copiar TBX_Injector.dll: {}", e))?;

    let bepinex_dir = game_dir.join("BepInEx");

    // Forçar a exibição do terminal do BepInEx
    let config_dir = bepinex_dir.join("config");
    let _ = fs::create_dir_all(&config_dir);
    let bepinex_cfg = config_dir.join("BepInEx.cfg");
    
    // A fonte selecionada pelo Font Injector fica neste caminho. Se ainda não
    // houver uma seleção do usuário, instalar uma TTF padrão válida.
    let tbx_config_dir = config_dir.join("TBX_Injector");
    let _ = fs::create_dir_all(&tbx_config_dir);
    let custom_font_path = tbx_config_dir.join("custom_font_bundle");
    if custom_font_path.is_file() && fs::metadata(&custom_font_path).map(|m| m.len()).unwrap_or(0) == 0 {
        let _ = fs::remove_file(&custom_font_path);
        let _ = tx.send(UiMsg::Log(
            "[Aviso] Removido custom_font_bundle vazio; ele não contém uma fonte utilizável.".into(),
        ));
    }
    let fallback_font_path = tbx_config_dir.join("fallback_font.ttf");
    if !fallback_font_path.is_file() || fs::metadata(&fallback_font_path).map(|m| m.len()).unwrap_or(0) == 0 {
        fs::write(&fallback_font_path, DEFAULT_FALLBACK_TTF)
            .map_err(|e| format!("Erro ao instalar fallback_font.ttf: {}", e))?;
        let _ = tx.send(UiMsg::Log(
            "Fonte universal TTF instalada. O plugin a converterá para TMP em tempo de execução.".into(),
        ));
    } else {
        let _ = tx.send(UiMsg::Log(
            "Usando fallback_font.ttf já selecionada pelo usuário.".into(),
        ));
    }
    
    // Configurar o font_config.json para passar a opção de substituir todas as fontes
    let translation_cfg_dir = bepinex_dir.join("Translation");
    let _ = fs::create_dir_all(&translation_cfg_dir);
    let font_cfg_path = translation_cfg_dir.join("font_config.json");
    let config_json = serde_json::json!({
        "fallbackFontName": "Noto Sans",
        "fontSizeMultiplier": 1.0,
        "fontSizeOffset": 0,
        "replaceAllFonts": substituir_todas_fontes_unity
    });
    let _ = fs::write(&font_cfg_path, serde_json::to_string_pretty(&config_json).unwrap_or_default());
    if !bepinex_cfg.exists() {
        let _ = fs::write(&bepinex_cfg, "[Logging.Console]\nEnabled = true\n");
    } else if let Ok(mut cfg_content) = fs::read_to_string(&bepinex_cfg) {
        if !cfg_content.contains("[Logging.Console]") {
            cfg_content.push_str("\n[Logging.Console]\nEnabled = true\n");
            let _ = fs::write(&bepinex_cfg, cfg_content);
        } else {
            let new_cfg = cfg_content.replace("Enabled = false", "Enabled = true");
            let _ = fs::write(&bepinex_cfg, new_cfg);
        }
    }

    let trans_dir = bepinex_dir.join("Translation").join(target_lang).join("Text");
    let _ = fs::create_dir_all(&trans_dir);
    
    let json_out = trans_dir.join("translated_texts.json");
    if translated_json.exists() {
        let _ = fs::copy(&translated_json, &json_out);
    }

    let _ = tx.send(UiMsg::Log("Instalação do Mod TBX Concluída! Inicie o jogo normalmente.".into()));
    
    if cfg!(unix) && executable.to_lowercase().ends_with(".exe") {
        let _ = tx.send(UiMsg::Log("\n[IMPORTANTE LINUX/PROTON] Como você está rodando um jogo de Windows no Linux,".into()));
        let _ = tx.send(UiMsg::Log("você PRECISA configurar o override do winhttp para o BepInEx iniciar!".into()));
        let _ = tx.send(UiMsg::Log("Na Steam, vá em Propriedades -> Opções de Inicialização e coloque:".into()));
        let _ = tx.send(UiMsg::Log("WINEDLLOVERRIDES=\"winhttp=n,b\" %command%\n".into()));
    }

    let _ = tx.send(UiMsg::Done("Mod Dinâmico Instalado!".into()));

    Ok(())
}
