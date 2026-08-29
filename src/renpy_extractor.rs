// TBX Translator - renpy_extractor.rs
// Creator: samwns
// Ren'Py extractor: injects a Python dump script, runs the game invisibly,
// reads the dump, and sends batches to the translation API.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use regex::Regex;
use tokio::time::sleep;
use walkdir::WalkDir;

use crate::api;
use crate::ui::UiMsg;

enum RenpyTextPart {
    Fixed(String),
    Translatable(usize),
}

const RENPY_CONTROL_PATTERN: &str =
    r"(?m)(\{[^{}\r\n]{0,120}\}|\[[^\[\]\r\n]{0,80}\]|^[A-Za-z_][A-Za-z0-9_]*=[^{}\r\n]{1,80}\})";

pub fn language_identifier(folder: &str) -> String {
    let mut identifier = String::new();
    
    // Replace common accents manually to avoid underscores
    let unaccented = folder.trim().to_lowercase()
        .replace('á', "a").replace('à', "a").replace('ã', "a").replace('â', "a")
        .replace('é', "e").replace('è', "e").replace('ê', "e")
        .replace('í', "i").replace('ì', "i").replace('î', "i")
        .replace('ó', "o").replace('ò', "o").replace('õ', "o").replace('ô', "o")
        .replace('ú', "u").replace('ù', "u").replace('û', "u")
        .replace('ç', "c").replace('ñ', "n");
        
    for character in unaccented.chars() {
        let normalized = if character.is_ascii_alphanumeric() {
            character
        } else {
            '_'
        };
        if normalized != '_' || !identifier.ends_with('_') {
            identifier.push(normalized);
        }
    }
    let identifier = identifier.trim_matches('_').to_string();
    let mut identifier = if identifier.is_empty() {
        "portuguese".to_string()
    } else {
        identifier
    };
    if identifier
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        identifier.insert_str(0, "lang_");
    }
    identifier
}

fn split_renpy_text(
    text: &str,
    control_re: &Regex,
    translatable: &mut Vec<String>,
) -> Vec<RenpyTextPart> {
    let mut parts = Vec::new();
    let mut cursor = 0usize;
    for control in control_re.find_iter(text) {
        if control.start() > cursor {
            let visible = &text[cursor..control.start()];
            if !visible.is_empty() {
                let index = translatable.len();
                translatable.push(visible.to_string());
                parts.push(RenpyTextPart::Translatable(index));
            }
        }
        parts.push(RenpyTextPart::Fixed(control.as_str().to_string()));
        cursor = control.end();
    }
    if cursor < text.len() {
        let visible = &text[cursor..];
        let index = translatable.len();
        translatable.push(visible.to_string());
        parts.push(RenpyTextPart::Translatable(index));
    }
    parts
}

fn rebuild_renpy_text(parts: &[RenpyTextPart], translations: &[String]) -> String {
    let mut rebuilt = String::new();
    for part in parts {
        match part {
            RenpyTextPart::Fixed(value) => rebuilt.push_str(value),
            RenpyTextPart::Translatable(index) => {
                if let Some(value) = translations.get(*index) {
                    rebuilt.push_str(value);
                }
            }
        }
    }
    rebuilt
}

// The Python injection script is embedded at compile time
const DUMP_SCRIPT: &str = include_str!("desired_python.py");

pub fn resolve_renpy_paths(path_str: &str) -> Result<(PathBuf, PathBuf), String> {
    let p = Path::new(path_str);
    if !p.exists() {
        return Err("O caminho especificado não existe.".to_string());
    }

    if p.is_file() {
        let base_dir = p.parent().unwrap_or(p).to_path_buf();
        let game_dir = base_dir.join("game");
        if game_dir.is_dir() {
            return Ok((base_dir, game_dir));
        }
        return Err(format!("A pasta 'game' do Ren'Py não foi encontrada em '{}'.", base_dir.display()));
    }

    if p.file_name().map_or(false, |n| n == "game") {
        let base_dir = p.parent().unwrap_or(p).to_path_buf();
        return Ok((base_dir, p.to_path_buf()));
    }

    let game_dir = p.join("game");
    if game_dir.is_dir() {
        return Ok((p.to_path_buf(), game_dir));
    }

    Err(format!("A pasta 'game' do Ren'Py não foi encontrada em '{}'.", p.display()))
}

pub async fn extract_texts(
    executable: &str,
    translation_folder: &str,
    source_lang: &str,
    target_lang: &str,
    keep_structure: bool,
    translate_character_names: bool,
    threads: u32,
    _api_engine: &str,
    tx: std::sync::mpsc::Sender<UiMsg>,
    cancelled: Arc<AtomicBool>,
    overwrite: bool,
    config: crate::app_config::AppConfig,
) -> Result<(), String> {
    let (_base_dir, game_dir) = resolve_renpy_paths(executable)?;

    let language_id = language_identifier(translation_folder);
    let tl_dir = game_dir.join("tl").join(&language_id);
    let temp_dir = game_dir.join("tl").join("tbx_temp");

    // Cleanup. O Ren'Py aborta com "A translation for X already exists" se um
    // .rpy antigo no tl/<idioma> definir a MESMA string que vamos reescrever.
    // Como cada tradução TBX sobrescreve o conteúdo da pasta, removemos sempre
    // os .rpy gerados previamente, mesmo quando o usuário desmarcou "overwrite"
    // (overwrite só controla re-download da API, não a higiene do tl/).
    if tl_dir.exists() {
        let _ = fs::remove_dir_all(&tl_dir);
        let _ = tx.send(UiMsg::Log(format!(
            "[Ren'Py] Limpei a pasta inteira game/tl/{}/ antes de reescrever.",
            language_id
        )));
    }
    if overwrite {
        let _ = fs::remove_dir_all(&tl_dir);
    }
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&tl_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    // Remove old injection artifacts
    for name in &[
        "tbx_boot.rpy", "tbx_boot.rpyc", "tbx_dumper.rpy", "tbx_dumper.rpyc",
        "tpg_boot.rpy", "tpg_boot.rpyc", "tpg_dumper.rpy", "tpg_dumper.rpyc"
    ] {
        let _ = fs::remove_file(game_dir.join(name));
    }
    let legacy_temp_root = game_dir.join("tl");
    if let Ok(entries) = fs::read_dir(&legacy_temp_root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if name == "tpg_temp" || name.starts_with("tpg_temp_") {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
    if language_id != translation_folder.trim() {
        let _ = tx.send(UiMsg::Log(format!(
            "[Ren'Py] Nome do idioma normalizado para '{}' (identificador compatível com Ren'Py).",
            language_id
        )));
    }

    let _ = tx.send(UiMsg::Log("[Motor Dump] Preparando injeção via Python...".into()));

    // Write injection script
    let injection_script = game_dir.join("tbx_dumper.rpy");
    fs::write(&injection_script, DUMP_SCRIPT).map_err(|e| e.to_string())?;
    let _ = tx.send(UiMsg::Log("[Motor Dump] Script de injeção escrito. Iniciando jogo invisível...".into()));

    // Spawn game process
    let mut proc = spawn_renpy_hidden(executable).map_err(|e| format!("Falha ao iniciar o jogo: {}", e))?;

    // Wait for game to finish or cancellation
    let mut wait_after_exit = 0;
    let dump_file = temp_dir.join("dump.txt");
    let mut extraction_cancelled = false;

    loop {
        if cancelled.load(Ordering::SeqCst) {
            let _ = proc.kill();
            let _ = fs::remove_file(&injection_script);
            let _ = tx.send(UiMsg::Log("[Aviso] Extração cancelada pelo usuário.".into()));
            extraction_cancelled = true;
            break;
        }

        if dump_file.exists() {
            sleep(Duration::from_millis(1000)).await; // wait for file to flush
            let _ = proc.kill(); // clean up if wrapper lingered
            break;
        }

        match proc.try_wait() {
            Ok(Some(_)) => {
                wait_after_exit += 1;
                if wait_after_exit > 30 { // 15 seconds wait after process exited
                    return Err("Falha: O jogo fechou sem gerar o arquivo dump.txt. (Talvez o jogo tenha crashado. Verifique tbx_renpy.log)".to_string());
                }
            },
            Ok(None) => {
                // Still running
            },
            Err(e) => return Err(format!("Erro ao monitorar processo: {}", e)),
        }
        sleep(Duration::from_millis(500)).await;
    }

    if extraction_cancelled {
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = tx.send(UiMsg::Cancelled);
        return Ok(());
    }

    // Ler o dump com tolerancia a truncamento (processo pode ser morto no
    // meio da escrita). Se leitura falhar, devolver erro claro.
    let dump_content = fs::read_to_string(&dump_file).map_err(|e| e.to_string())?;

    let candidates = crate::renpy_parser::parse_dump_content(&dump_content);

    let _ = tx.send(UiMsg::Log(format!("[Diagnóstico] Dump Engine: {} linhas brutas, {} candidatos limpos e parseados.", dump_content.lines().count(), candidates.len())));

    // Filter + deduplicate. O parser ja deduplica pelo "id" nativo do RenPy
    // quando presente; aqui so precisamos filtrar por tipo e re-dedup de
    // seguranca.
    let mut seen: HashSet<String> = HashSet::new();
    let mut dialogues: Vec<crate::renpy_parser::RenpyCandidate> = Vec::new();

    for cand in candidates {
        if cand.kind == "nome" && !translate_character_names {
            continue;
        }
        let key = match &cand.identifier {
            Some(id) => format!("id:{}", id),
            None => format!("{}|{}", cand.file, cand.text),
        };
        if seen.insert(key) {
            dialogues.push(cand);
        }
    }

    let _ = tx.send(UiMsg::Log(format!("[Extração] {} textos únicos para traduzir.", dialogues.len())));

    let total = dialogues.len();
    let batch_size = 64usize;
    let mut processed = 0usize;

    // File writers
    let mut writers: HashMap<String, Vec<(String, String)>> = HashMap::new();

    let client = reqwest::Client::new();
    let mut src_code = api::get_lang_code(source_lang);
    let tgt_code = api::get_lang_code(target_lang);
    let mut detected_mismatch = false;
    let mut detection_attempts = 0;
    // Separate items that can be resolved instantly via standard dictionary
    // from items that need online translation API
    let mut to_translate_indices: Vec<usize> = Vec::new();
    let mut resolved_translations: Vec<Option<String>> = vec![None; dialogues.len()];
    let mut was_cancelled = false;
    let ignored_tags = config.get_active_tags(Some(game_dir.join("tl").join("tbx_tags.txt")), 0);

    for (idx, cand) in dialogues.iter().enumerate() {
        if let Some(std_trans) = crate::dictionary::lookup(&cand.text, tgt_code) {
            resolved_translations[idx] = Some(std_trans.to_string());
        } else {
            to_translate_indices.push(idx);
        }
    }

    let dict_hits = dialogues.len() - to_translate_indices.len();
    if dict_hits > 0 {
        let _ = tx.send(UiMsg::Log(format!("[Dicionário Padrão] {} termos de interface/menu pré-traduzidos instantaneamente.", dict_hits)));
    }

    // Ren'Py commands and interpolation variables are never sent to the API.
    // The third alternative also catches malformed dumps such as `sc=4}Text`.
    let renpy_control_re = Regex::new(RENPY_CONTROL_PATTERN).unwrap();

    for chunk_indices in to_translate_indices.chunks(batch_size) {
        if cancelled.load(Ordering::SeqCst) {
            was_cancelled = true;
            let _ = tx.send(UiMsg::Log("[Aviso] Cancelamento solicitado pelo usuário...".to_string()));
            break;
        }

        let chunk_items: Vec<&crate::renpy_parser::RenpyCandidate> = chunk_indices.iter()
            .map(|&idx| &dialogues[idx])
            .collect();

        let mut texts = Vec::new();
        let prepared: Vec<Vec<RenpyTextPart>> = chunk_items
            .iter()
            .map(|cand| split_renpy_text(&cand.text, &renpy_control_re, &mut texts))
            .collect();

        if !detected_mismatch && src_code != "auto" && detection_attempts < 15 {
            if let Some(sample) = texts.iter().filter(|t| t.len() > 15).max_by_key(|t| t.len()) {
                detection_attempts += 1;
                if let Some(detected) = api::detect_language(&client, sample).await {
                    let _ = tx.send(UiMsg::Log(format!("[DEBUG] Detectando idioma de '{}'... Resultado: {}", sample, detected)));
                    if detected != src_code && detected != "auto" {
                        src_code = api::get_lang_code(crate::api::get_lang_name(&detected));
                        let _ = tx.send(UiMsg::DetectedLanguageMismatch(detected));
                        detected_mismatch = true;
                    }
                }
            }
        }

        let translated = api::translate_batch_concurrent(&client, &texts, src_code, tgt_code, threads as usize, config.usar_traducao_pivo, &ignored_tags).await
            .unwrap_or_else(|_| vec![]);

        for (i, &orig_idx) in chunk_indices.iter().enumerate() {
            let cand = &dialogues[orig_idx];
            let original = &cand.text;
            let raw = if translated.len() == texts.len() {
                rebuild_renpy_text(&prepared[i], &translated)
            } else {
                original.clone()
            };
            let raw = if raw.trim().is_empty() { original.clone() } else { raw.trim().to_string() };
            let trad = raw;
            let _ = tx.send(UiMsg::Log(format!("  [OK] {} -> {}", original.replace('\n', " "), trad.replace('\n', " "))));
            resolved_translations[orig_idx] = Some(trad);
        }

        processed += chunk_indices.len();
        let _ = tx.send(UiMsg::Progress(processed, total));
    }

    // Assemble final translations into writer files. Separamos entradas com
    // identificador nativo do RenPy (traduzidas via `translate <lang> <id>:`)
    // das demais (mantidas no formato old/new, que funciona como fallback
    // para interfaces e menus sem id).
    let mut id_writers: HashMap<String, Vec<(String, String, String)>> = HashMap::new();
    let mut legacy_writers: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut seen_legacy_strings: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (i, cand) in dialogues.iter().enumerate() {
        let trad = resolved_translations[i].clone().unwrap_or_else(|| cand.text.clone());
        let target_file = if keep_structure {
            let mut f = cand.file.clone();
            if f.ends_with(".rpyc") { f = f[..f.len()-5].to_string() + ".rpy"; }
            f
        } else {
            "script.rpy".to_string()
        };

        if let Some(id) = &cand.identifier {
            id_writers.entry(target_file)
                .or_default()
                .push((id.clone(), cand.text.clone(), trad));
        } else {
            if !seen_legacy_strings.contains(&cand.text) {
                seen_legacy_strings.insert(cand.text.clone());
                legacy_writers.entry(target_file)
                    .or_default()
                    .push((cand.text.clone(), trad));
            }
        }
    }

    // Write .rpy files (entradas com ID primeiro, old/new como fallback)
    let mut all_targets: std::collections::HashSet<String> = std::collections::HashSet::new();
    all_targets.extend(id_writers.keys().cloned());
    all_targets.extend(legacy_writers.keys().cloned());

    for target_file in all_targets {
        let out_path = tl_dir.join(&target_file);
        if let Some(parent) = out_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut content = String::new();

        if let Some(entries) = id_writers.get(&target_file) {
            // Bloco nativo por identifier: mais estavel que old/new e
            // funciona mesmo com scripts dentro de .rpa.
            for (id, _orig, trad) in entries {
                content.push_str(&format!("translate {} {}:\n", language_id, id));
                content.push_str(&format!("    _ \"{}\"\n\n", escape_renpy(trad)));
            }
        }

        if let Some(pairs) = legacy_writers.get(&target_file) {
            content.push_str(&format!("translate {} strings:\n\n", language_id));
            for (original, trad) in pairs {
                content.push_str(&format!("    old \"{}\"\n", escape_renpy(original)));
                content.push_str(&format!("    new \"{}\"\n\n", escape_renpy(trad)));
            }
        }

        let _ = fs::write(&out_path, &content);
        let _ = tx.send(UiMsg::Log(format!("   -> {} criado.", target_file)));
    }

    // Register an optional language without overriding the player's current
    // preference. The generated overlay gives games without a language menu a
    // safe selector and lists every language already known by Ren'Py.
    let boot_script = game_dir.join("tbx_boot.rpy");
    let language_label_name = api::get_lang_name(api::get_lang_code(target_lang));
    
    let _ = tx.send(UiMsg::Log(
        "[Ren'Py] Instalando Mod Universal de Idiomas (Força Bruta)...".to_string(),
    ));

    let escaped_language_id = escape_renpy(&language_id);
    let boot_content = format!(
        r##"init 999 python:
    try:
        if getattr(persistent, "tbx_language_set", None) != "{0}":
            _preferences.language = "{0}"
            persistent.tbx_language_set = "{0}"
    except:
        pass

    try:
        tbx_previous_say_filter = config.say_menu_text_filter
        def tbx_translate_filter(txt):
            if txt:
                try:
                    if tbx_previous_say_filter:
                        txt = tbx_previous_say_filter(txt)
                    return __(txt)
                except:
                    pass
            return txt
        config.say_menu_text_filter = tbx_translate_filter
    except:
        pass

    try:
        if not hasattr(renpy, "tbx_original_input"):
            renpy.tbx_original_input = renpy.input
        def tbx_input(prompt, *args, **kwargs):
            return renpy.tbx_original_input(__(prompt), *args, **kwargs)
        renpy.input = tbx_input
    except:
        pass
"##,
        escaped_language_id,
    );
    let _ = fs::write(boot_script, boot_content);

    let mod_script = game_dir.join("99_tbx_language_manager.rpy");
    let mod_content = r##"# ==========================================
# MOD UNIVERSAL DE IDIOMAS (FORÇA BRUTA)
# ==========================================

init 999 python:
    # 1. Pega os idiomas escondidos no jogo
    def obter_idiomas():
        langs = list(renpy.known_languages())
        if None not in langs:
            langs.insert(0, None)
        return langs

    # 2. Cria uma camada invisível e indestrutível por cima de tudo no motor
    if "camada_idioma_mod" not in config.layers:
        config.layers.append("camada_idioma_mod")

    def _tbx_tela_de_config_ativa():
        # Botao so na tela de configuracoes/preferencias do jogo
        for nome in ("preferences", "prefs", "config", "settings", "opcoes", "configuracoes"):
            try:
                if renpy.get_screen(nome):
                    return True
            except Exception:
                pass
        return False

    def manter_botao_ativo():
        mostrar = _tbx_tela_de_config_ativa()
        on = renpy.get_screen("botao_flutuante_idioma", layer="camada_idioma_mod")
        if mostrar and not on:
            renpy.show_screen("botao_flutuante_idioma", _layer="camada_idioma_mod")
        elif not mostrar and on:
            renpy.hide_screen("botao_flutuante_idioma", layer="camada_idioma_mod")

    config.interact_callbacks.append(manter_botao_ativo)

    # 4. Mantém o atalho de teclado 'L' como redundância
    config.keymap["abrir_menu_idioma"] = ["l", "L"]
    config.underlay.append(renpy.Keymap(abrir_menu_idioma=lambda: renpy.run(ShowMenu("menu_idiomas_universal"))))

# A TELA DO BOTÃO
screen botao_flutuante_idioma():
    # Visivel apenas na tela de configuracoes
    if _tbx_tela_de_config_ativa():
        textbutton "🌐 Idioma":
            align (0.98, 0.02)
            text_size 22
            action ShowMenu("menu_idiomas_universal")

# O MENU DE IDIOMAS EM SI
screen menu_idiomas_universal():
    tag menu
    modal True
    add "#000a"
    
    frame:
        align (0.5, 0.5)
        padding (40, 40)
        
        vbox:
            spacing 20
            text "Selecione o Idioma / Language" size 30 xalign 0.5 bold True
            
            for lang in obter_idiomas():
                $ nome = lang.capitalize() if lang else "Original / Default"
                textbutton nome:
                    xalign 0.5
                    text_size 25
                    action [Language(lang), Return()]
            
            null height 20
            textbutton "Fechar / Close":
                xalign 0.5
                text_size 25
                action Return()
"##;
    let _ = fs::write(&mod_script, mod_content);

    // Cleanup temp
    let _ = fs::remove_file(game_dir.join("tbx_dumper.rpy"));
    let _ = fs::remove_file(game_dir.join("tbx_dumper.rpyc"));
    let _ = fs::remove_dir_all(&temp_dir);

    if was_cancelled {
        let _ = tx.send(UiMsg::Log("[Aviso] A tradução foi cancelada. Os arquivos traduzidos até o momento foram salvos.".to_string()));
        let _ = tx.send(UiMsg::Cancelled);
    } else {
        let _ = tx.send(UiMsg::Log("[Concluído] Tradução Ren'Py finalizada com sucesso!".into()));
        let _ = tx.send(UiMsg::Done("Tradução concluída.".to_string()));
    }
    Ok(())
}

pub fn spawn_renpy_hidden(executable: &str) -> std::io::Result<std::process::Child> {
    use std::process::Stdio;

    let target_path = Path::new(executable);
    let base_dir = if target_path.is_file() {
        target_path.parent().unwrap_or(target_path).to_path_buf()
    } else if target_path.file_name().map_or(false, |n| n == "game") {
        target_path.parent().unwrap_or(target_path).to_path_buf()
    } else {
        target_path.to_path_buf()
    };

    let log_path = base_dir.join("tbx_renpy.log");
    let stdout = std::fs::File::create(&log_path).ok().map_or(Stdio::null(), Stdio::from);
    let stderr = std::fs::File::create(&log_path).ok().map_or(Stdio::null(), Stdio::from);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let ensure_executable = |p: &Path| {
            if let Ok(metadata) = std::fs::metadata(p) {
                let mut perms = metadata.permissions();
                if perms.mode() & 0o111 == 0 {
                    perms.set_mode(perms.mode() | 0o755);
                    let _ = std::fs::set_permissions(p, perms);
                }
            }
        };

        let is_exe = target_path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("exe"));

        // 1. Se o arquivo selecionado for um script nativo (.sh) ou binário ELF linux (não .exe)
        if target_path.is_file() && !is_exe {
            ensure_executable(target_path);
            let mut cmd = if target_path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("sh")) {
                let mut c = crate::paths::hidden_command("sh");
                c.arg(target_path);
                c
            } else {
                crate::paths::hidden_command(target_path)
            };
            return cmd
                .current_dir(&base_dir)
                .env("RENPY_DISABLE_SOUND", "1")
                .env("RENPY_SKIP_SPLASHSCREEN", "1")
                .stdout(stdout)
                .stderr(stderr)
                .spawn();
        }

        // 2. Se for um .exe ou pasta, procurar launcher nativo (.sh) correspondente na base_dir
        if let Some(stem) = target_path.file_stem() {
            let candidate_sh = base_dir.join(format!("{}.sh", stem.to_string_lossy()));
            if candidate_sh.is_file() {
                ensure_executable(&candidate_sh);
                return crate::paths::hidden_command("sh")
                    .arg(&candidate_sh)
                    .current_dir(&base_dir)
                    .env("RENPY_DISABLE_SOUND", "1")
                    .env("RENPY_SKIP_SPLASHSCREEN", "1")
                    .stdout(stdout)
                    .stderr(stderr)
                    .spawn();
            }
        }

        // Procurar qualquer script .sh na pasta do jogo
        if let Ok(entries) = std::fs::read_dir(&base_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("sh")) {
                    ensure_executable(&p);
                    return crate::paths::hidden_command("sh")
                        .arg(&p)
                        .current_dir(&base_dir)
                        .env("RENPY_DISABLE_SOUND", "1")
                        .env("RENPY_SKIP_SPLASHSCREEN", "1")
                        .stdout(stdout)
                        .stderr(stderr)
                        .spawn();
                }
            }
        }

        // Procurar binários em lib/py3-linux-* ou lib/linux-*
        let lib_dirs = [
            "lib/py3-linux-x86_64",
            "lib/linux-x86_64",
            "lib/py2-linux-x86_64",
            "lib/linux-i686",
            "lib/py3-linux-aarch64",
            "lib/linux-aarch64",
        ];
        for sub in &lib_dirs {
            let lib_path = base_dir.join(sub);
            if lib_path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&lib_path) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_file() && p.extension().is_none() {
                            ensure_executable(&p);
                            return crate::paths::hidden_command(&p)
                                .arg(&base_dir)
                                .current_dir(&base_dir)
                                .env("RENPY_DISABLE_SOUND", "1")
                                .env("RENPY_SKIP_SPLASHSCREEN", "1")
                                .stdout(stdout)
                                .stderr(stderr)
                                .spawn();
                        }
                    }
                }
            }
        }

        // 3. Se for um .exe e não houver executável Linux nativo, tentar Wine se disponível
        if is_exe || target_path.is_file() {
            let wine_installed = std::process::Command::new("wine")
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_or(false, |s| s.success());

            if wine_installed {
                return crate::paths::hidden_command("wine")
                    .arg(target_path)
                    .current_dir(&base_dir)
                    .env("WINEDEBUG", "-all")
                    .env("RENPY_DISABLE_SOUND", "1")
                    .env("RENPY_SKIP_SPLASHSCREEN", "1")
                    .stdout(stdout)
                    .stderr(stderr)
                    .spawn();
            }

            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "O arquivo '{}' é um executável Windows (.exe). No Linux Debian, instale o Wine (sudo apt install wine) ou selecione o script .sh nativo do jogo.",
                    target_path.display()
                ),
            ));
        }

        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "Nenhum executável ou script inicializador (.sh) do Ren'Py foi encontrado em '{}'.",
                base_dir.display()
            ),
        ));
    }

    #[cfg(not(unix))]
    {
        crate::paths::hidden_command(executable)
            .current_dir(&base_dir)
            .env("RENPY_DISABLE_SOUND", "1")
            .env("RENPY_SKIP_SPLASHSCREEN", "1")
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
    }
}


fn escape_renpy(text: &str) -> String {
    text.replace('\\', r"\\")
        .replace('\r', "")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_only_visible_renpy_segments_are_prepared() {
        let re = Regex::new(RENPY_CONTROL_PATTERN).unwrap();
        let mut visible = Vec::new();
        let parts = split_renpy_text("{sc=4}What is wrong, [c_name]?{/sc}", &re, &mut visible);
        assert_eq!(visible, vec!["What is wrong, ", "?"]);

        let translated = vec!["O que há de errado, ".to_string(), "?".to_string()];
        assert_eq!(
            rebuild_renpy_text(&parts, &translated),
            "{sc=4}O que há de errado, [c_name]?{/sc}"
        );
    }

    #[test]
    fn test_format_expression_is_not_sent_for_translation() {
        let re = Regex::new(RENPY_CONTROL_PATTERN).unwrap();
        let mut visible = Vec::new();
        let parts = split_renpy_text("time: [_viewers.moved_time:>.2f] s", &re, &mut visible);
        assert_eq!(visible, vec!["time: ", " s"]);
        let translated = vec!["tempo: ".to_string(), " s".to_string()];
        assert_eq!(
            rebuild_renpy_text(&parts, &translated),
            "tempo: [_viewers.moved_time:>.2f] s"
        );
    }

    #[test]
    fn test_language_identifier_is_valid_for_renpy() {
        assert_eq!(language_identifier("Portuguese BR"), "portuguese_br");
        assert_eq!(language_identifier("  123 PT-BR  "), "lang_123_pt_br");
        assert_eq!(language_identifier(""), "portuguese");
    }

    #[test]
    fn test_static_language_menu_is_extended_once() {
        let directory = std::env::temp_dir().join(format!(
            "tbx-renpy-menu-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let screen = directory.join("screens.rpy");
        fs::write(
            &screen,
            "screen preferences():\n    vbox:\n        textbutton \"English\":\n            action Language(None)\n        textbutton \"Español\":\n            action Language(\"spanish\")\n",
        )
        .unwrap();

        let first = integrate_language_menu(&directory, "portuguese", "Portuguese").unwrap();
        let second = integrate_language_menu(&directory, "portuguese", "Portuguese").unwrap();
        let content = fs::read_to_string(&screen).unwrap();
        assert_eq!(first, (1, false));
        assert_eq!(second, (1, false));
        assert_eq!(content.matches("TBX_LANGUAGE_OPTION:portuguese").count(), 1);
        assert!(screen.with_file_name("screens.rpy.tbx_backup").exists());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn test_change_language_menu_is_extended_once() {
        let directory = std::env::temp_dir().join(format!(
            "tbx-renpy-change-language-menu-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let script = directory.join("Initial.rpy");
        fs::write(
            &script,
            "label splashscreen:\n    menu:\n        \"English\":\n            $ renpy.change_language(\"English\")\n\n        \"Español (Default)\":\n            $ renpy.change_language(None)\n\n        \"ukrainian\":\n            $ renpy.change_language(\"ukrainian\")\n\n    return\n",
        )
        .unwrap();

        let first = integrate_language_menu(&directory, "portuguese", "Portuguese").unwrap();
        let second = integrate_language_menu(&directory, "portuguese", "Portuguese").unwrap();
        let content = fs::read_to_string(&script).unwrap();
        assert_eq!(first, (1, false));
        assert_eq!(second, (1, false));
        assert_eq!(content.matches("TBX_LANGUAGE_OPTION:portuguese").count(), 1);
        assert!(content.contains("$ renpy.change_language(\"portuguese\")"));
        assert!(script.with_file_name("Initial.rpy.tbx_backup").exists());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn test_resolve_renpy_paths_variants() {
        let temp = std::env::temp_dir().join(format!("tbx-renpy-resolve-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let _ = fs::create_dir_all(temp.join("game"));

        let exe_file = temp.join("Game.exe");
        fs::write(&exe_file, b"MZ...").unwrap();

        // 1. Passando o .exe
        let (base, game) = resolve_renpy_paths(&exe_file.to_string_lossy()).unwrap();
        assert_eq!(base, temp);
        assert_eq!(game, temp.join("game"));

        // 2. Passando o diretório raiz
        let (base2, game2) = resolve_renpy_paths(&temp.to_string_lossy()).unwrap();
        assert_eq!(base2, temp);
        assert_eq!(game2, temp.join("game"));

        // 3. Passando a própria pasta game/
        let (base3, game3) = resolve_renpy_paths(&temp.join("game").to_string_lossy()).unwrap();
        assert_eq!(base3, temp);
        assert_eq!(game3, temp.join("game"));

        let _ = fs::remove_dir_all(&temp);
    }
}
