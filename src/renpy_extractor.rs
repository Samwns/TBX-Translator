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
    threads: u32,
    api_engine: &str,
    tx: gtk4::glib::Sender<UiMsg>,
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
            if let Some(reason) = filter_reason(&text) {
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

    for chunk in dialogues.chunks(batch_size) {
        if cancelled.load(Ordering::SeqCst) { break; }

        let texts: Vec<String> = chunk.iter()
            .map(|(t, _, _)| protect_renpy_tags(t))
            .collect();

        let translated = api::translate_batch(&client, &texts, api_url, src_code, tgt_code).await
            .unwrap_or_else(|_| vec![]);

        for (i, (original, file, _)) in chunk.iter().enumerate() {
            let raw = translated.get(i).cloned().unwrap_or_default();
            let raw = if raw.trim().is_empty() { original.clone() } else { raw.trim().to_string() };
            let trad = restore_renpy_tags(&raw, original);

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

        processed += chunk.len();
        let _ = tx.send(UiMsg::Progress(processed, total));
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
    use std::process::{Command, Stdio};
    Command::new(executable)
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



fn protect_renpy_tags(text: &str) -> String {
    // Protect both {...} tags and [...] variables
    let re = Regex::new(r"(\{[^{}\r\n]{1,120}\}|\[[^\[\]\r\n]{1,80}\])").unwrap();
    let mut idx = 0usize;
    re.replace_all(text, |_: &regex::Captures| {
        let marker = format!("777{:03}777", idx);
        idx += 1;
        marker
    }).to_string()
}

fn restore_renpy_tags(translated: &str, original: &str) -> String {
    let re = Regex::new(r"(\{[^{}\r\n]{1,120}\}|\[[^\[\]\r\n]{1,80}\])").unwrap();
    let tags: Vec<String> = re.find_iter(original).map(|m| m.as_str().to_string()).collect();
    let mut result = translated.to_string();
    for (i, tag) in tags.iter().enumerate() {
        let marker_re = Regex::new(&format!(r"7\s*7\s*7\s*{}\s*7\s*7\s*7", 
            format!("{:03}", i).chars().map(|c| c.to_string()).collect::<Vec<_>>().join(r"\s*")
        )).unwrap();
        result = marker_re.replace_all(&result, tag.as_str()).to_string();
    }
    result
}

fn escape_renpy(text: &str) -> String {
    text.replace('\\', r"\\")
        .replace('\r', "")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}
