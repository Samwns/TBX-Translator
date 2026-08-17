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
    for character in folder.trim().chars() {
        let normalized = if character.is_ascii_alphanumeric() {
            character.to_ascii_lowercase()
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

fn integrate_language_menu(
    game_dir: &Path,
    language_id: &str,
    language_label: &str,
) -> Result<(usize, bool), String> {
    let marker = format!("# TBX_LANGUAGE_OPTION:{language_id}");
    let language_call = Regex::new(&format!(
        r#"Language\s*\(\s*["']{}["']"#,
        regex::escape(language_id)
    ))
    .map_err(|error| error.to_string())?;
    let change_language_call = Regex::new(&format!(
        r#"change_language\s*\(\s*["']{}["']"#,
        regex::escape(language_id)
    ))
    .map_err(|error| error.to_string())?;
    let any_change_language_call = Regex::new(r"renpy\s*\.\s*change_language\s*\(")
        .map_err(|error| error.to_string())?;
    let mut patched_menus = 0usize;
    let mut dynamic_menu_found = false;

    for entry in WalkDir::new(game_dir)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            name != "tl" && name != "cache" && name != "saves"
        })
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file()
            || path.extension().and_then(|value| value.to_str()) != Some("rpy")
            || path.file_name().and_then(|value| value.to_str()).is_some_and(|name| {
                name.starts_with("tbx_") || name.starts_with("tpg_")
            })
        {
            continue;
        }

        let Ok(content) = fs::read_to_string(path) else { continue };
        if content.contains("renpy.known_languages()") {
            dynamic_menu_found = true;
            continue;
        }
        if content.contains(&marker)
            || language_call.is_match(&content)
            || change_language_call.is_match(&content)
        {
            patched_menus += 1;
            continue;
        }

        let newline = if content.contains("\r\n") { "\r\n" } else { "\n" };
        let lines: Vec<&str> = content.lines().collect();
        let mut language_buttons = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("textbutton") {
                continue;
            }
            let button_indent = line.len() - trimmed.len();
            let indent = line[..button_indent].to_string();
            if trimmed.contains("action") && trimmed.contains("Language(") {
                language_buttons.push((index, index, indent));
                continue;
            }

            // Also recognize block-style buttons:
            // textbutton "English":
            //     action Language(None)
            let mut end = index;
            let mut has_language_action = false;
            for (next_index, next_line) in lines.iter().enumerate().skip(index + 1) {
                let next_trimmed = next_line.trim_start();
                let next_indent = next_line.len() - next_trimmed.len();
                if !next_trimmed.is_empty() && next_indent <= button_indent {
                    break;
                }
                end = next_index;
                if next_trimmed.starts_with("action") && next_trimmed.contains("Language(") {
                    has_language_action = true;
                }
            }
            if has_language_action {
                language_buttons.push((index, end, indent));
            }
        }
        let mut insertions: Vec<(usize, String)> = Vec::new();
        for (position, (_, end, indent)) in language_buttons.iter().enumerate() {
            let next_is_same_group = language_buttons.get(position + 1).is_some_and(|next| {
                next.0 <= *end + 2 && next.2.as_str() == indent.as_str()
            });
            if !next_is_same_group {
                insertions.push((
                    *end,
                    format!(
                        "{indent}{marker}{newline}{indent}textbutton \"{}\" action Language(\"{}\"){newline}",
                        escape_renpy(language_label),
                        escape_renpy(language_id)
                    ),
                ));
            }
        }

        // Some games implement their language selector as a regular Ren'Py
        // `menu` whose choices call `renpy.change_language(...)`. Find each
        // choice block and append the new language to the same group.
        let mut language_choices: Vec<(usize, usize, String)> = Vec::new();
        for (action_index, action_line) in lines.iter().enumerate() {
            if !any_change_language_call.is_match(action_line) {
                continue;
            }
            let action_trimmed = action_line.trim_start();
            let action_indent = action_line.len() - action_trimmed.len();
            for choice_index in (0..action_index).rev() {
                let choice_line = lines[choice_index];
                let choice_trimmed = choice_line.trim_start();
                if choice_trimmed.is_empty() {
                    continue;
                }
                let choice_indent_len = choice_line.len() - choice_trimmed.len();
                if choice_indent_len >= action_indent {
                    continue;
                }
                if choice_trimmed.ends_with(':')
                    && (choice_trimmed.starts_with('"') || choice_trimmed.starts_with('\''))
                {
                    let mut end = choice_index;
                    for (next_index, next_line) in lines.iter().enumerate().skip(choice_index + 1) {
                        let next_trimmed = next_line.trim_start();
                        let next_indent = next_line.len() - next_trimmed.len();
                        if !next_trimmed.is_empty() && next_indent <= choice_indent_len {
                            break;
                        }
                        end = next_index;
                    }
                    language_choices.push((
                        choice_index,
                        end,
                        choice_line[..choice_indent_len].to_string(),
                    ));
                }
                break;
            }
        }
        language_choices.sort_by_key(|choice| choice.0);
        language_choices.dedup_by_key(|choice| choice.0);
        for (position, (_, end, indent)) in language_choices.iter().enumerate() {
            let next_is_same_group = language_choices.get(position + 1).is_some_and(|next| {
                next.0 <= *end + 2 && next.2.as_str() == indent.as_str()
            });
            if !next_is_same_group {
                insertions.push((
                    *end,
                    format!(
                        "{indent}{marker}{newline}{indent}\"{}\":{newline}{indent}    $ renpy.change_language(\"{}\"){newline}",
                        escape_renpy(language_label),
                        escape_renpy(language_id)
                    ),
                ));
            }
        }

        if insertions.is_empty() {
            continue;
        }

        let mut rewritten = String::with_capacity(content.len() + insertions.len() * 160);
        for (index, line) in lines.iter().enumerate() {
            rewritten.push_str(line);
            rewritten.push_str(newline);
            for (_, snippet) in insertions.iter().filter(|(end, _)| *end == index) {
                rewritten.push_str(snippet);
                patched_menus += 1;
            }
        }

        let backup_name = format!(
            "{}.tbx_backup",
            path.file_name().and_then(|value| value.to_str()).unwrap_or("screen.rpy")
        );
        let backup = path.with_file_name(backup_name);
        if !backup.exists() {
            fs::copy(path, &backup).map_err(|error| {
                format!("Falha ao criar backup de {}: {error}", path.display())
            })?;
        }
        fs::write(path, rewritten)
            .map_err(|error| format!("Falha ao atualizar {}: {error}", path.display()))?;
    }

    Ok((patched_menus, dynamic_menu_found))
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
) -> Result<(), String> {
    let (_base_dir, game_dir) = resolve_renpy_paths(executable)?;

    let language_id = language_identifier(translation_folder);
    let tl_dir = game_dir.join("tl").join(&language_id);
    let temp_dir = game_dir.join("tl").join("tbx_temp");

    // Cleanup
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

    // Parse dump using the advanced Rust Parser
    let dump_content = fs::read_to_string(&dump_file).map_err(|e| e.to_string())?;

    let candidates = crate::renpy_parser::parse_dump_content(&dump_content);

    let _ = tx.send(UiMsg::Log(format!("[Diagnóstico] Dump Engine: {} linhas brutas, {} candidatos limpos e parseados.", dump_content.lines().count(), candidates.len())));

    // Filter + deduplicate
    let mut seen: HashSet<String> = HashSet::new();
    let mut dialogues: Vec<(String, String, String)> = Vec::new();

    for (text, file, kind) in candidates {
        if kind == "nome" && !translate_character_names {
            continue;
        }
        if seen.insert(text.clone()) {
            dialogues.push((text, file, kind));
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

    for (idx, (text, _, _)) in dialogues.iter().enumerate() {
        if let Some(std_trans) = crate::dictionary::lookup(text, tgt_code) {
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

        let chunk_items: Vec<&(String, String, String)> = chunk_indices.iter()
            .map(|&idx| &dialogues[idx])
            .collect();

        let mut texts = Vec::new();
        let prepared: Vec<Vec<RenpyTextPart>> = chunk_items
            .iter()
            .map(|(text, _, _)| split_renpy_text(text, &renpy_control_re, &mut texts))
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

        let translated = api::translate_batch_concurrent(&client, &texts, src_code, tgt_code, threads as usize).await
            .unwrap_or_else(|_| vec![]);

        for (i, &orig_idx) in chunk_indices.iter().enumerate() {
            let (original, _, _) = &dialogues[orig_idx];
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

    // Assemble final translations into writer files
    for (i, (original, file, _)) in dialogues.iter().enumerate() {
        let trad = resolved_translations[i].clone().unwrap_or_else(|| original.clone());
        let target_file = if keep_structure {
            let mut f = file.clone();
            if f.ends_with(".rpyc") { f = f[..f.len()-5].to_string() + ".rpy"; }
            f
        } else {
            "script.rpy".to_string()
        };

        writers.entry(target_file).or_default().push((original.clone(), trad));
    }

    // Write .rpy files
    for (target_file, pairs) in &writers {
        let out_path = tl_dir.join(target_file);
        if let Some(parent) = out_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut content = format!("translate {} strings:\n\n", language_id);
        for (original, trad) in pairs {
            content.push_str(&format!("    old \"{}\"\n", escape_renpy(original)));
            content.push_str(&format!("    new \"{}\"\n\n", escape_renpy(trad)));
        }
        let _ = fs::write(&out_path, &content);
        let _ = tx.send(UiMsg::Log(format!("   -> {} criado.", target_file)));
    }

    // Register an optional language without overriding the player's current
    // preference. The generated overlay gives games without a language menu a
    // safe selector and lists every language already known by Ren'Py.
    let boot_script = game_dir.join("tbx_boot.rpy");
    let language_label_name = api::get_lang_name(api::get_lang_code(target_lang));
    let (patched_menus, dynamic_menu_found) =
        match integrate_language_menu(&game_dir, &language_id, language_label_name) {
            Ok(result) => result,
            Err(error) => {
                let _ = tx.send(UiMsg::Log(format!(
                    "[Ren'Py Aviso] Não foi possível integrar o menu original: {error}. Usando seletor complementar."
                )));
                (0, false)
            }
        };
    let use_fallback_selector = patched_menus == 0 && !dynamic_menu_found;
    if dynamic_menu_found {
        let _ = tx.send(UiMsg::Log(format!(
            "[Ren'Py] O menu usa renpy.known_languages(); '{}' será listado automaticamente.",
            language_id
        )));
    } else if patched_menus > 0 {
        let _ = tx.send(UiMsg::Log(format!(
            "[Ren'Py] Idioma '{}' integrado em {} lista(s) de idiomas existente(s).",
            language_id, patched_menus
        )));
    } else {
        let _ = tx.send(UiMsg::Log(
            "[Ren'Py] O jogo não possui lista de idiomas editável; seletor complementar instalado.".to_string(),
        ));
    }

    let language_label = escape_renpy(language_label_name);
    let escaped_language_id = escape_renpy(&language_id);
    let fallback_flag = if use_fallback_selector { "True" } else { "False" };
    let boot_content = format!(
        r##"init 999 python:
    tbx_language_labels = dict([("{0}", "{1}")])
    tbx_use_language_overlay = {2}

    try:
        if getattr(persistent, "tbx_language_set", None) != "{0}":
            _preferences.language = "{0}"
            persistent.tbx_language_set = "{0}"
    except:
        pass

    def tbx_language_name(language):
        return tbx_language_labels.get(language, language.replace("_", " ").title())

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

    try:
        if tbx_use_language_overlay and "tbx_language_access" not in config.overlay_screens:
            config.overlay_screens.append("tbx_language_access")
    except:
        pass

screen tbx_language_access():
    zorder 9998
    
    key "l" action ShowMenu("tbx_language_selector")
    key "L" action ShowMenu("tbx_language_selector")

    if main_menu or getattr(store, '_menu', False):
        textbutton _("Idioma"):
            action ShowMenu("tbx_language_selector")
            xalign 0.98
            yalign 0.02
            text_size 24
            text_outlines [ (1, "#000", 0, 0) ]

screen tbx_language_selector():
    modal True
    zorder 9999

    add Solid("#00000099")

    frame:
        align (0.5, 0.5)
        padding (30, 24)

        vbox:
            spacing 10
            label _("Idioma") xalign 0.5
            textbutton _("Original") action [Language(None), Hide("tbx_language_selector")] xalign 0.5

            for tbx_lang in sorted(renpy.known_languages()):
                textbutton tbx_language_name(tbx_lang) action [Language(tbx_lang), Hide("tbx_language_selector")] xalign 0.5

            textbutton _("Fechar") action Hide("tbx_language_selector") xalign 0.5
"##,
        escaped_language_id,
        language_label,
        fallback_flag,
    );
    let _ = fs::write(boot_script, boot_content);

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
