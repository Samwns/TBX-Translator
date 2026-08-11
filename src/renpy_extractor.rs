// TPG Translator - renpy_extractor.rs
// Creator: samwns
// Ren'Py extractor: injects a Python dump script, runs the game invisibly,
// reads the dump, and sends batches to the translation API.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use regex::Regex;
use tokio::time::sleep;

use crate::api;
use crate::ui::UiMsg;

// The Python injection script is embedded at compile time
const DUMP_SCRIPT: &str = include_str!("desired_python.py");

pub async fn extract_texts(
    executable: &str,
    translation_folder: &str,
    source_lang: &str,
    target_lang: &str,
    keep_structure: bool,
    translate_character_names: bool,
    _threads: u32,
    api_engine: &str,
    tx: std::sync::mpsc::Sender<UiMsg>,
    cancelled: Arc<AtomicBool>,
    overwrite: bool,
) -> Result<(), String> {
    let exe_path = Path::new(executable);
    let game_dir = exe_path
        .parent()
        .ok_or("Não foi possível determinar o diretório do jogo")?
        .join("game");

    if !game_dir.exists() {
        return Err("A pasta 'game' não foi encontrada próximo ao executável.".to_string());
    }

    let tl_dir = game_dir.join("tl").join(translation_folder);
    let temp_dir = game_dir.join("tl").join("tpg_temp");

    // Cleanup
    if overwrite {
        let _ = fs::remove_dir_all(&tl_dir);
    }
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&tl_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    // Remove old injection artifacts
    for name in &["tpg_boot.rpy", "tpg_boot.rpyc", "tpg_dumper.rpy", "tpg_dumper.rpyc"] {
        let _ = fs::remove_file(game_dir.join(name));
    }

    // Write injection script
    let injection_script = game_dir.join("tpg_dumper.rpy");
    fs::write(&injection_script, DUMP_SCRIPT).map_err(|e| e.to_string())?;
    let _ = tx.send(UiMsg::Log("[Motor Dump] Script de injeção escrito. Iniciando jogo...".into()));

    // Spawn game process
    let mut proc = spawn_renpy_hidden(executable).map_err(|e| format!("Falha ao iniciar o jogo: {}", e))?;

    // Wait for game to finish or cancellation
    loop {
        if cancelled.load(Ordering::SeqCst) {
            let _ = proc.kill();
            let _ = fs::remove_file(&injection_script);
            let _ = tx.send(UiMsg::Log("[Aviso] Extração cancelada pelo usuário.".into()));
            return Ok(());
        }
        match proc.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => sleep(Duration::from_millis(100)).await,
            Err(e) => return Err(format!("Erro ao monitorar processo: {}", e)),
        }
    }

    let dump_file = temp_dir.join("dump.txt");
    if !dump_file.exists() {
        return Err("Falha: O jogo fechou sem gerar o arquivo dump.txt.".to_string());
    }

    // Parse dump
    let dump_content = fs::read_to_string(&dump_file).map_err(|e| e.to_string())?;
    let mut candidates: Vec<(String, String, String)> = Vec::new(); // (text, file, type)

    for line in dump_content.lines() {
        let parts: Vec<&str> = line.splitn(3, "|||").collect();
        if parts.len() == 3 {
            let text = normalize_dump_text(parts[2]);
            candidates.push((text, parts[0].to_string(), parts[1].to_string()));
        } else if parts.len() == 2 {
            let text = normalize_dump_text(parts[1]);
            candidates.push((text, parts[0].to_string(), "dialogo".to_string()));
        }
    }

    let _ = tx.send(UiMsg::Log(format!("[Diagnóstico] Dump: {} linhas brutas, {} candidatos", dump_content.lines().count(), candidates.len())));

    // Filter + deduplicate
    let mut seen: HashSet<String> = HashSet::new();
    let mut dialogues: Vec<(String, String, String)> = Vec::new();

    for (text, file, kind) in candidates {
        if kind != "interface" {
            if let Some(_reason) = filter_reason(&text) {
                continue;
            }
        }
        if kind == "nome" && !translate_character_names {
            continue;
        }
        if seen.insert(text.clone()) {
            dialogues.push((text, file, kind));
        }
    }

    let _ = tx.send(UiMsg::Log(format!("[Extração] {} textos únicos para traduzir.", dialogues.len())));

    let total = dialogues.len();
    let batch_size = 20usize;
    let mut processed = 0usize;

    // File writers
    let mut writers: HashMap<String, Vec<(String, String)>> = HashMap::new();

    let client = reqwest::Client::new();
    let src_code = api::get_lang_code(source_lang);
    let tgt_code = api::get_lang_code(target_lang);
    let api_url = if api_engine.contains("Apps Script") {
        "" // future extension
    } else {
        "Integrado"
    };

    // Separate items that can be resolved instantly via standard dictionary
    // from items that need online translation API
    let mut to_translate_indices: Vec<usize> = Vec::new();
    let mut resolved_translations: Vec<Option<String>> = vec![None; dialogues.len()];

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

    for chunk_indices in to_translate_indices.chunks(batch_size) {
        if cancelled.load(Ordering::SeqCst) { break; }

        let chunk_items: Vec<&(String, String, String)> = chunk_indices.iter()
            .map(|&idx| &dialogues[idx])
            .collect();

        let texts: Vec<String> = chunk_items.iter()
            .map(|(t, _, _)| protect_renpy_tags(t))
            .collect();

        let translated = api::translate_batch(&client, &texts, api_url, src_code, tgt_code).await
            .unwrap_or_else(|_| vec![]);

        for (i, &orig_idx) in chunk_indices.iter().enumerate() {
            let (original, _, _) = &dialogues[orig_idx];
            let raw = translated.get(i).cloned().unwrap_or_default();
            let raw = if raw.trim().is_empty() { original.clone() } else { raw.trim().to_string() };
            let trad = restore_renpy_tags(&raw, original);
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

        let _ = tx.send(UiMsg::Log(format!("  [OK] {} -> {}", original.replace('\n', " "), trad.replace('\n', " "))));
        writers.entry(target_file).or_default().push((original.clone(), trad));
    }

    // Write .rpy files
    for (target_file, pairs) in &writers {
        let out_path = tl_dir.join(target_file);
        if let Some(parent) = out_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut content = format!("translate {} strings:\n\n", translation_folder);
        for (original, trad) in pairs {
            content.push_str(&format!("    old \"{}\"\n", escape_renpy(original)));
            content.push_str(&format!("    new \"{}\"\n\n", escape_renpy(trad)));
        }
        let _ = fs::write(&out_path, &content);
        let _ = tx.send(UiMsg::Log(format!("   -> {} criado.", target_file)));
    }

    // Write boot patch
    let boot_script = game_dir.join("tpg_boot.rpy");
    let boot_content = format!(
        "init 999 python:\n\
         \x20   try:\n\
         \x20       _preferences.language = \"{0}\"\n\
         \x20   except:\n\
         \x20       pass\n\
         \x20   try:\n\
         \x20       config.language = \"{0}\"\n\
         \x20   except:\n\
         \x20       pass\n\
         \x20   try:\n\
         \x20       def tpg_translate_filter(txt):\n\
         \x20           if txt:\n\
         \x20               try:\n\
         \x20                   return __(txt)\n\
         \x20               except:\n\
         \x20                   pass\n\
         \x20           return txt\n\
         \x20       config.say_menu_text_filter = tpg_translate_filter\n\
         \x20   except:\n\
         \x20       pass\n\
         \x20   try:\n\
         \x20       old_input = renpy.input\n\
         \x20       def tpg_input(prompt, *args, **kwargs):\n\
         \x20           return old_input(__(prompt), *args, **kwargs)\n\
         \x20       renpy.input = tpg_input\n\
         \x20   except:\n\
         \x20       pass\n",
        translation_folder
    );
    let _ = fs::write(boot_script, boot_content);

    // Cleanup temp
    let _ = fs::remove_file(&injection_script);
    let _ = fs::remove_file(game_dir.join("tpg_dumper.rpyc"));
    let _ = fs::remove_dir_all(&temp_dir);

    let _ = tx.send(UiMsg::Log("[Concluído] Tradução Ren'Py finalizada com sucesso!".into()));
    Ok(())
}

pub fn spawn_renpy_hidden(executable: &str) -> std::io::Result<std::process::Child> {
    use std::process::Stdio;
    crate::paths::hidden_command(executable)
        .env("RENPY_DISABLE_SOUND", "1")
        .env("RENPY_SKIP_SPLASHSCREEN", "1")
        .env("RENPY_RENDERER", "sw")
        .env("SDL_VIDEODRIVER", "dummy")
        .env("SDL_AUDIODRIVER", "dummy")
        .stdout(Stdio::null()).stderr(Stdio::null())
        .spawn()
}

fn normalize_dump_text(text: &str) -> String {
    text.replace("\\n", "\n")
        .replace("\r", "")
        .replace("\\r", "")
        .trim()
        .to_string()
}

fn filter_reason(text: &str) -> Option<&'static str> {
    let s = text.trim();
    if s.is_empty() { return Some("vazio"); }
    if s.len() < 2 { return Some("curto"); }
    if s.len() > 1000 { return Some("longo"); }
    if !s.chars().any(|c| c.is_alphabetic()) { return Some("sem_letra"); }

    let lower = s.to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") { return Some("url"); }
    if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".ogg") { return Some("arquivo"); }

    let letters = s.chars().filter(|c| c.is_alphabetic()).count();
    let len = s.len().max(1);
    if letters * 100 / len < 20 { return Some("poucas_letras"); }

    None
}

pub fn protect_renpy_tags(text: &str) -> String {
    // Protect both {...} tags and [...] variables (such as [plural], [mc_nombre], [her], etc.)
    let re = Regex::new(r"(\{[^{}\r\n]{1,120}\}|\[[^\[\]\r\n]{1,80}\])").unwrap();
    let mut idx = 0usize;
    re.replace_all(text, |_: &regex::Captures| {
        let marker = format!("_TBXVAR{}_", idx);
        idx += 1;
        marker
    }).to_string()
}

pub fn restore_renpy_tags(translated: &str, original: &str) -> String {
    let re = Regex::new(r"(\{[^{}\r\n]{1,120}\}|\[[^\[\]\r\n]{1,80}\])").unwrap();
    let tags: Vec<String> = re.find_iter(original).map(|m| m.as_str().to_string()).collect();
    let mut result = translated.to_string();

    for (i, tag) in tags.iter().enumerate() {
        // Tolerant regex matching _TBXVAR0_, _ TBXVAR0 _, _TBXVAR 0_, TBXVAR0, etc.
        let pattern = format!(r"(?:_+|\b)\s*TBX(?:VAR|TAG)\s*{}\s*(?:_+|\b)", i);
        if let Ok(marker_re) = Regex::new(&pattern) {
            result = marker_re.replace_all(&result, tag.as_str()).to_string();
        }

        // Backward compatibility for numeric 777000777 pattern
        let old_pattern = format!(r"7\s*7\s*7[\s.,]*{}\s*7\s*7\s*7", format!("{:03}", i).chars().map(|c| c.to_string()).collect::<Vec<_>>().join(r"[\s.,]*"));
        if let Ok(old_re) = Regex::new(&old_pattern) {
            result = old_re.replace_all(&result, tag.as_str()).to_string();
        }
    }

    result
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
    fn test_protect_and_restore_renpy_interpolations() {
        let original = "No one said anything. All your [plural] looked at you at the same time.";
        let protected = protect_renpy_tags(original);
        assert_eq!(protected, "No one said anything. All your _TBXVAR0_ looked at you at the same time.");

        // Simulate translation preserving placeholder
        let simulated_trans = "Ninguém disse nada. Todas as suas _TBXVAR0_ olharam para você ao mesmo tempo.";
        let restored = restore_renpy_tags(simulated_trans, original);
        assert_eq!(restored, "Ninguém disse nada. Todas as suas [plural] olharam para você ao mesmo tempo.");
    }

    #[test]
    fn test_protect_and_restore_multiple_tags_and_vars() {
        let original = "Olá [mc_nombre], você tem {b}[count]{/b} mensagens de {color=#ff0000}[sender]{/color}!";
        let protected = protect_renpy_tags(original);
        assert_eq!(protected, "Olá _TBXVAR0_, você tem _TBXVAR1__TBXVAR2__TBXVAR3_ mensagens de _TBXVAR4__TBXVAR5__TBXVAR6_!");

        let simulated_trans = "Hello _TBXVAR0_, you have _TBXVAR1__TBXVAR2__TBXVAR3_ messages from _TBXVAR4__TBXVAR5__TBXVAR6_!";
        let restored = restore_renpy_tags(simulated_trans, original);
        assert_eq!(restored, "Hello [mc_nombre], you have {b}[count]{/b} messages from {color=#ff0000}[sender]{/color}!");
    }

    #[test]
    fn test_restore_tolerant_spacing() {
        let original = "Hello [plural] and [mc_nombre]!";
        let simulated_with_spaces = "Olá _ TBXVAR0 _ e _TBXVAR 1_!";
        let restored = restore_renpy_tags(simulated_with_spaces, original);
        assert_eq!(restored, "Olá [plural] e [mc_nombre]!");
    }
}
