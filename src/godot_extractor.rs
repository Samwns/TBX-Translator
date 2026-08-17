use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc::Sender};
use regex::Regex;
use crate::types::UiMsg;
use crate::api;
use crate::godot_pck;
use std::io::{Read, Seek, SeekFrom};

use std::time::{SystemTime, UNIX_EPOCH};

/// How the translation is installed. `ForceNativeSlot` is the safe choice for
/// exported games: it replaces a locale the game already registers, so no
/// unsupported `override.cfg`/autoload trick is necessary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InjectionStrategy {
    Auto,
    ForceNativeSlot,
    DirectPatch,
}

impl InjectionStrategy {
    pub fn from_config(value: &str) -> Self {
        match value {
            "force_slot" => Self::ForceNativeSlot,
            "direct_patch" => Self::DirectPatch,
            _ => Self::Auto,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct GodotLocaleInfo {
    pub translation_paths: Vec<String>,
    pub locale_codes: Vec<String>,
}

pub fn detect_native_locales(game_path: &str) -> Result<GodotLocaleInfo, String> {
    let pck_path = locate_pck(Path::new(game_path))?;
    let mut file = fs::File::open(&pck_path).map_err(|e| format!("Falha ao abrir PCK: {e}"))?;
    let archive = godot_pck::read_pck_header(&mut file)?;
    let mut info = GodotLocaleInfo::default();
    for entry in archive.files {
        let lower = entry.path.to_ascii_lowercase();
        if lower.ends_with(".translation") {
            if let Some(name) = Path::new(&entry.path).file_name().and_then(|s| s.to_str()) {
                // locale.en.translation -> en. This intentionally only offers
                // slots that are already registered by the exported project.
                if let Some(locale) = name.strip_prefix("locale.").and_then(|s| s.strip_suffix(".translation")) {
                    info.locale_codes.push(locale.to_string());
                    info.translation_paths.push(entry.path);
                }
            }
        }
    }
    info.locale_codes.sort();
    info.locale_codes.dedup();
    Ok(info)
}

/// Maps the language selected in TBX to the exact locale filename exported by
/// the game (for example, `Spanish` -> `es_MX` when that is the only slot).
pub fn resolve_native_locale(selected_language: &str, available: &[String]) -> Option<String> {
    let requested = api::get_lang_code(selected_language).to_ascii_lowercase();
    available.iter().find(|code| code.eq_ignore_ascii_case(&requested)).cloned()
        .or_else(|| available.iter().find(|code| code.to_ascii_lowercase().starts_with(&(requested.clone() + "_"))).cloned())
}

fn normalized_resource_path(virtual_path: &str) -> String {
    virtual_path
        .replace('\\', "/")
        .trim_start_matches("res://")
        .trim_start_matches('/')
        .to_ascii_lowercase()
}

fn is_locale_resource(virtual_path: &str) -> bool {
    let path = normalized_resource_path(virtual_path);
    path.starts_with("locale/") || path.contains("/locale/")
}

fn belongs_to_selected_locale(virtual_path: &str, selected_locale: &str) -> bool {
    let path = normalized_resource_path(virtual_path);
    let locale = selected_locale.to_ascii_lowercase();
    if !is_locale_resource(&path) { return selected_locale.eq_ignore_ascii_case("en"); }
    path.ends_with(&format!("locale/po/{locale}.po"))
        || path.ends_with(&format!("locale.{locale}.translation"))
}

pub fn locate_pck(exe_path: &Path) -> Result<PathBuf, String> {
    if !exe_path.exists() {
        return Err("Arquivo PCK ou Executável não encontrado.".into());
    }
    // Se o arquivo selecionado já é .pck
    if exe_path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pck")) {
        return Ok(exe_path.to_path_buf());
    }

    // Verifica .pck adjacente com mesmo stem (ex: game.pck para game.exe ou game.x86_64)
    let external_pck = exe_path.with_extension("pck");
    if external_pck.is_file() {
        return Ok(external_pck);
    }

    // Verifica <nome_completo>.pck (ex: BeatBanger.x86_64.pck)
    if let Some(filename) = exe_path.file_name() {
        let direct_pck = exe_path.with_file_name(format!("{}.pck", filename.to_string_lossy()));
        if direct_pck.is_file() {
            return Ok(direct_pck);
        }
    }

    // Se for um arquivo executável, pode conter PCK embutido
    if exe_path.is_file() {
        return Ok(exe_path.to_path_buf());
    }

    // Se for uma pasta, procura qualquer .pck dentro dela
    if exe_path.is_dir() {
        if let Ok(entries) = fs::read_dir(exe_path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pck")) {
                    return Ok(p);
                }
            }
        }
    }

    Ok(exe_path.to_path_buf())
}

fn po_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

fn po_quoted_value(line: &str) -> Option<String> {
    let quote = line.find('"')?;
    serde_json::from_str::<String>(&line[quote..]).ok()
}

/// Parse singular PO entries, including multiline msgid/msgstr values.
fn parse_po_catalog(content: &str) -> Vec<(String, String)> {
    #[derive(Clone, Copy)]
    enum Field { Id, Str }
    let mut entries = Vec::new();
    let mut msgid = String::new();
    let mut msgstr = String::new();
    let mut field: Option<Field> = None;

    let flush = |entries: &mut Vec<(String, String)>, id: &mut String, value: &mut String| {
        if !id.is_empty() {
            entries.push((std::mem::take(id), std::mem::take(value)));
        } else {
            value.clear();
        }
    };

    for raw in content.lines().chain(std::iter::once("")) {
        let line = raw.trim();
        if line.starts_with("msgid ") {
            flush(&mut entries, &mut msgid, &mut msgstr);
            msgid = po_quoted_value(line).unwrap_or_default();
            field = Some(Field::Id);
        } else if line.starts_with("msgstr ") {
            msgstr = po_quoted_value(line).unwrap_or_default();
            field = Some(Field::Str);
        } else if line.starts_with('"') {
            if let Some(value) = po_quoted_value(line) {
                match field {
                    Some(Field::Id) => msgid.push_str(&value),
                    Some(Field::Str) => msgstr.push_str(&value),
                    None => {}
                }
            }
        } else if line.is_empty() {
            flush(&mut entries, &mut msgid, &mut msgstr);
            field = None;
        }
    }
    entries
}

fn dump_native_translation(data: &[u8]) -> Result<HashMap<String, String>, String> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    let work = std::env::temp_dir().join(format!("tbx-godot-catalog-{}-{}", std::process::id(), unique));
    fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let result = (|| {
        fs::write(work.join("project.godot"), "[application]\nconfig/name=\"TBX Catalog Reader\"\n")
            .map_err(|e| e.to_string())?;
        fs::write(work.join("locale.translation"), data).map_err(|e| e.to_string())?;
        let script = "extends SceneTree\nfunc _init():\n\tvar translation = ResourceLoader.load(\"res://locale.translation\")\n\tif translation == null:\n\t\tquit(1)\n\t\treturn\n\tvar catalog = {}\n\tfor message_id in translation.get_message_list():\n\t\tcatalog[message_id] = translation.get_message(message_id)\n\tvar output = FileAccess.open(\"res://catalog.json\", FileAccess.WRITE)\n\toutput.store_string(JSON.stringify(catalog))\n\toutput.close()\n\tquit()\n";
        fs::write(work.join("dump_translation.gd"), script).map_err(|e| e.to_string())?;
        let output = crate::paths::hidden_command("godot").args(["--headless", "--path"]).arg(&work)
            .args(["--script", "res://dump_translation.gd"]).output()
            .map_err(|e| format!("Godot não encontrado para ler o catálogo nativo: {e}"))?;
        if !output.status.success() {
            return Err(format!("Godot não conseguiu abrir o catálogo nativo: {}", String::from_utf8_lossy(&output.stderr)));
        }
        let json = fs::read_to_string(work.join("catalog.json")).map_err(|e| e.to_string())?;
        let catalog: HashMap<String, String> = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        if catalog.is_empty() {
            return Err("OptimizedTranslation não expõe sua lista de IDs; usando PO e recursos de diálogo do jogo".into());
        }
        Ok(catalog)
    })();
    let _ = fs::remove_dir_all(&work);
    result
}

fn is_story_script(virtual_path: &str) -> bool {
    let path = normalized_resource_path(virtual_path);
    path.starts_with("data/") && path.contains("/cutscene/") && path.ends_with("/script.cfg")
}

fn is_embedded_dialogue_resource(virtual_path: &str) -> bool {
    let path = normalized_resource_path(virtual_path);
    path.ends_with(".res") && path.rsplit('/').next().is_some_and(|name| name.contains("dialogue"))
}

/// Exported `.res` dialogue resources retain their visible UTF-8 strings even
/// though their object structure is binary. Restrict this scanner to resources
/// whose filename is explicitly dialogue-related and reject Godot metadata.
fn extract_dialogue_strings_from_resource(data: &[u8]) -> Vec<String> {
    fn useful(value: &str) -> bool {
        let value = value.trim();
        if value.len() < 2 || value.starts_with("res://") || value.starts_with("uid://")
            || value.starts_with("local://") || value.contains("metadata/") {
            return false;
        }
        const INTERNAL: &[&str] = &[
            "RSRC", "Resource", "Script", "resource_local_to_scene", "resource_name",
            "script", "input", "responses", "next_dialogue_set", "ShopDialogueOptionData",
            "ShopDialogueSetData", "ShopDialogueData",
        ];
        if INTERNAL.contains(&value) { return false; }
        let looks_like_identifier = value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        !looks_like_identifier
            && value.chars().any(|c| c.is_alphabetic())
            && value.chars().filter(|c| c.is_control()).count() == 0
    }

    let mut found = Vec::new();
    let mut start = None;
    for (index, byte) in data.iter().copied().chain(std::iter::once(0)).enumerate() {
        if (0x20..=0x7e).contains(&byte) {
            start.get_or_insert(index);
        } else if let Some(begin) = start.take() {
            if index > begin {
                if let Ok(value) = std::str::from_utf8(&data[begin..index]) {
                    if useful(value) { found.push(value.trim().to_string()); }
                }
            }
        }
    }
    found
}

fn compile_native_translation(map: &HashMap<String, String>, locale: &str) -> Result<Vec<u8>, String> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    let work = std::env::temp_dir().join(format!("tbx-godot-locale-{}-{}", std::process::id(), unique));
    fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let result = (|| {
        fs::write(work.join("project.godot"), "[application]\nconfig/name=\"TBX Translation Builder\"\n")
            .map_err(|e| e.to_string())?;
        let mut po = format!("msgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\"Language: {locale}\\n\"\n\n");
        for (source, translated) in map {
            if source.trim().is_empty() || translated.trim().is_empty() { continue; }
            po.push_str(&format!("msgid \"{}\"\nmsgstr \"{}\"\n\n", po_escape(source), po_escape(translated)));
        }
        fs::write(work.join("locale.po"), po).map_err(|e| e.to_string())?;
        let script = "extends SceneTree\nfunc _init():\n\tvar translation = load(\"res://locale.po\")\n\tif translation == null:\n\t\tpush_error(\"Could not import locale.po\")\n\t\tquit(1)\n\t\treturn\n\tvar error = ResourceSaver.save(translation, \"res://locale.translation\")\n\tquit(error)\n";
        fs::write(work.join("build_translation.gd"), script).map_err(|e| e.to_string())?;
        let import = crate::paths::hidden_command("godot").args(["--headless", "--editor", "--path"]).arg(&work).arg("--import").output()
            .map_err(|e| format!("Godot não encontrado para compilar a tradução nativa: {e}"))?;
        if !import.status.success() { return Err(format!("Godot não conseguiu importar o PO: {}", String::from_utf8_lossy(&import.stderr))); }
        let build = crate::paths::hidden_command("godot").args(["--headless", "--path"]).arg(&work).args(["--script", "res://build_translation.gd"]).output()
            .map_err(|e| format!("Falha ao executar o compilador Godot: {e}"))?;
        if !build.status.success() { return Err(format!("Godot não conseguiu gerar .translation: {}", String::from_utf8_lossy(&build.stderr))); }
        fs::read(work.join("locale.translation")).map_err(|e| format!("Godot não gerou locale.translation: {e}"))
    })();
    let _ = fs::remove_dir_all(&work);
    result
}

pub fn output_folder(executable: &str, translation_folder: &str, target_lang_name: &str) -> PathBuf {
    let parent = Path::new(executable).parent().unwrap_or(Path::new("."));
    let name = if translation_folder.trim().is_empty() { target_lang_name } else { translation_folder.trim() };
    let safe_name = name.replace(['/', '\\'], "_");
    parent.join(format!("TBX_Workspace_Godot_{}", safe_name))
}

pub async fn extract_texts(
    game_path: &str,
    folder_name: &str,
    source_lang: &str,
    target_lang: &str,
    threads: u32,
    api_engine: &str,
    tx: Sender<UiMsg>,
    cancelled: Arc<AtomicBool>,
    overwrite: bool,
    config: crate::app_config::AppConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let exe_path = Path::new(game_path);
    let pck_ext = exe_path.with_extension("pck");

    let mut pck_path = exe_path.to_path_buf();

    if game_path.to_lowercase().ends_with(".exe") {
        if pck_ext.exists() {
            let exe_size = std::fs::metadata(&exe_path).map(|m| m.len()).unwrap_or(0);
            let pck_size = std::fs::metadata(&pck_ext).map(|m| m.len()).unwrap_or(0);
            if pck_size > exe_size || exe_size < 100_000_000 {
                pck_path = pck_ext;
            }
        }
    }

    if !pck_path.exists() {
        return Err("Arquivo PCK ou Executável não encontrado.".into());
    }

    let out_dir = output_folder(game_path, folder_name, target_lang);
    if !out_dir.exists() {
        fs::create_dir_all(&out_dir)?;
    }
    let translated_json = out_dir.join("translation.json");

    if overwrite {
        fs::write(&translated_json, "{}")?;
    let _ = tx.send(UiMsg::Log(format!("Tradução anterior limpa: {}", translated_json.display())));
    }

    let _ = tx.send(UiMsg::Log(format!("Lendo cabeçalho do PCK: {}", pck_path.display())));

    let mut file = fs::File::open(&pck_path).map_err(|e| format!("Falha ao abrir PCK: {}", e))?;
    let pck_archive = godot_pck::read_pck_header(&mut file)?;

    let available_locales: Vec<String> = pck_archive.files.iter().filter_map(|entry| {
        Path::new(&entry.path).file_name().and_then(|s| s.to_str())
            .and_then(|name| name.strip_prefix("locale.").and_then(|s| s.strip_suffix(".translation")))
            .map(str::to_string)
    }).collect();
    let source_locale = resolve_native_locale(source_lang, &available_locales)
        .unwrap_or_else(|| api::get_lang_code(source_lang).to_string());
    let _ = tx.send(UiMsg::Log(format!("Idioma selecionado: {source_lang} → usando somente locale '{source_locale}'.")));

    // If the export includes the source PO, it is the authoritative catalogue.
    // Reading scenes/configs as well would collect property names, sample data,
    // and often embedded copies of every locale.
    let has_selected_po = pck_archive.files.iter().any(|entry| {
        normalized_resource_path(&entry.path).ends_with(&format!("locale/po/{}.po", source_locale.to_ascii_lowercase()))
    });

    let selected_translation_suffix = format!("locale.{}.translation", source_locale.to_ascii_lowercase());
    let mut native_catalog = HashMap::new();
    if let Some(entry) = pck_archive.files.iter().find(|entry| {
        normalized_resource_path(&entry.path).ends_with(&selected_translation_suffix)
    }) {
        let mut data = vec![0u8; entry.size as usize];
        if file.seek(SeekFrom::Start(entry.offset)).is_ok() && file.read_exact(&mut data).is_ok() {
            match dump_native_translation(&data) {
                Ok(catalog) => {
                    let _ = tx.send(UiMsg::Log(format!("Catálogo binário '{}': {} mensagens.", entry.path, catalog.len())));
                    native_catalog = catalog;
                }
                Err(error) => {
                    let _ = tx.send(UiMsg::Log(format!("[Aviso] Não foi possível ler '{}': {}", entry.path, error)));
                }
            }
        }
    }

    if source_locale.eq_ignore_ascii_case("en") {
        let mut embedded_dialogue_count = 0usize;
        for entry in pck_archive.files.iter().filter(|entry| is_embedded_dialogue_resource(&entry.path)) {
            let mut data = vec![0u8; entry.size as usize];
            if file.seek(SeekFrom::Start(entry.offset)).is_ok() && file.read_exact(&mut data).is_ok() {
                for source in extract_dialogue_strings_from_resource(&data) {
                    native_catalog.entry(source.clone()).or_insert(source);
                    embedded_dialogue_count += 1;
                }
            }
        }
        let _ = tx.send(UiMsg::Log(format!(
            "Recursos binários de diálogo: {} textos visíveis encontrados.", embedded_dialogue_count
        )));
    }

    let valid_exts = [".dtl", ".dialogue", ".tscn", ".tres", ".po", ".json", ".cfg", ".txt"];
    let mut files_to_translate_content: Vec<(String, String)> = Vec::new();

    for entry in pck_archive.files {
        let path_lower = entry.path.to_lowercase();

        if path_lower.contains("credits") || path_lower.contains("patron") || path_lower.contains("supporter") {
            continue;
        }

        let accepted_language = belongs_to_selected_locale(&entry.path, &source_locale);
        let accepted_source = is_story_script(&entry.path) || if has_selected_po {
            normalized_resource_path(&entry.path).ends_with(&format!("locale/po/{}.po", source_locale.to_ascii_lowercase()))
        } else {
            accepted_language
        };
        if valid_exts.iter().any(|ext| path_lower.ends_with(ext)) && accepted_source {
            let mut data = vec![0u8; entry.size as usize];
            if file.seek(SeekFrom::Start(entry.offset)).is_ok() && file.read_exact(&mut data).is_ok() {
                files_to_translate_content.push((entry.path.clone(), String::from_utf8_lossy(&data).to_string()));
            }
        }
    }

    // Load external files
    if let Some(parent) = pck_path.parent() {
        for entry in walkdir::WalkDir::new(parent).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let path_lower = entry.path().to_string_lossy().to_lowercase();
                if path_lower.contains("credits") || path_lower.contains("patron") || path_lower.contains("supporter") {
                    continue;
                }
                if path_lower.ends_with(".exe") || path_lower.ends_with(".pck") || path_lower.contains("translation.json") || path_lower.contains("tbx_") || path_lower.ends_with(".txt") {
                    continue;
                }
                if valid_exts.iter().any(|ext| path_lower.ends_with(ext)) {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(relative_path) = entry.path().strip_prefix(parent) {
                            let virtual_path = format!("res://{}", relative_path.to_string_lossy().replace("\\", "/"));
                            let accepted_source = is_story_script(&virtual_path) || if has_selected_po {
                                normalized_resource_path(&virtual_path).ends_with(&format!("locale/po/{}.po", source_locale.to_ascii_lowercase()))
                            } else {
                                belongs_to_selected_locale(&virtual_path, &source_locale)
                            };
                            if accepted_source {
                                files_to_translate_content.push((virtual_path, content));
                            }
                        }
                    }
                }
            }
        }
    }

    let _ = tx.send(UiMsg::Log(format!("Encontrados {} arquivos/recursos traduzíveis.", files_to_translate_content.len())));
    for (path, _) in &files_to_translate_content {
        let _ = tx.send(UiMsg::Log(format!("Fonte selecionada: {path}")));
    }

    let mut texts_to_translate: Vec<String> = Vec::new();
    let mut message_ids: HashMap<String, String> = HashMap::new();
    for (message_id, source_text) in native_catalog {
        let source = if source_text.trim().is_empty() { message_id.clone() } else { source_text };
        if !source.trim().is_empty() {
            message_ids.insert(source.clone(), message_id);
            texts_to_translate.push(source);
        }
    }

    // Regexes
    let dtl_speaker_re = Regex::new(r"^(?P<prefix>\s*[^:\r\n]+:\s*)(?P<text>.+?)(?P<suffix>\r?\n)?$").unwrap();
    let dtl_choice_re = Regex::new(r"^(?P<prefix>\s*-\s+)(?P<text>.+?)(?P<suffix>\r?\n)?$").unwrap();
    let dtl_text_attr_re = Regex::new(r#"(?i)^(?P<before>\s*\[[^\]\r\n]*?\btext\s*=\s*&?#?)(?:(?P<q_d>")(?P<t_d>[^"]*)(?P<a_d>"[^\]\r\n]*\]\s*)|(?P<q_s>')(?P<t_s>[^']*)(?P<a_s>'[^\]\r\n]*\]\s*))(?P<suffix>\r?\n)?$"#).unwrap();
    let tscn_text_re = Regex::new(r#"^(?P<prefix>\s*(?:text|placeholder_text|tooltip_text|window_title|title)\s*=\s*")(?P<text>(?:\\.|[^"])*)(?P<suffix>"\s*(?:\r?\n)?)$"#).unwrap();
    let json_text_re = Regex::new(r#"^(?P<prefix>.*?["'](?:text|dialog|dialogue|name|text_en|text_pt|text_es)["']\s*:\s*")(?P<text>(?:\\.|[^"])*)(?P<suffix>".*)$"#).unwrap();
    let cfg_text_re = Regex::new(r#"^(?P<prefix>.*?\b(?:text|dialog|dialogue|name|string)["']?\s*[=:]\s*")(?P<text>(?:\\.|[^"])*)(?P<suffix>".*)$"#).unwrap();
    let story_dialogue_re = Regex::new(r#"^(?P<prefix>\s*"dialogue"\s*:\s*")(?P<text>(?:\\.|[^"])*)(?P<suffix>".*)$"#).unwrap();
    // BBCode (`[wave]`, `[color=...]`, etc.) is handled segment-by-segment by
    // api.rs. Only template variables/directives need placeholder protection.
    let protect_re = Regex::new(r"(\{[^{}\r\n]+\}|\\[A-Za-z]+\[[^\]\r\n]*\])").unwrap();

    for (virtual_path, content) in &files_to_translate_content {
        if cancelled.load(Ordering::SeqCst) {
            return Err("Cancelado".into());
        }

        let path_lower = virtual_path.to_lowercase();
        let suffix = if path_lower.ends_with(".dtl") { ".dtl" }
                     else if path_lower.ends_with(".dialogue") { ".dialogue" }
                     else if path_lower.ends_with(".json") { ".json" }
                     else if path_lower.ends_with(".cfg") { ".cfg" }
                     else if path_lower.ends_with(".txt") { ".txt" }
                     else if path_lower.ends_with(".po") { ".po" }
                     else { ".tscn" };

        if suffix == ".po" {
            for (message_id, translated_source) in parse_po_catalog(content) {
                let source = if translated_source.trim().is_empty() {
                    message_id.clone()
                } else {
                    translated_source
                };
                if !source.trim().is_empty() {
                    message_ids.insert(source.clone(), message_id);
                    texts_to_translate.push(source);
                }
            }
            continue;
        }

        for line in content.lines() {
            let stripped = line.trim();
            if stripped.is_empty() || stripped.starts_with('#') { continue; }

            if suffix == ".dtl" {
                if let Some(caps) = dtl_text_attr_re.captures(line) {
                    if let Some(text) = caps.name("t_d").or_else(|| caps.name("t_s")) {
                        texts_to_translate.push(text.as_str().to_string());
                    }
                    continue;
                }
                if let Some(caps) = dtl_choice_re.captures(line) {
                    texts_to_translate.push(caps["text"].to_string());
                    continue;
                }
                let is_cmd = stripped.starts_with("set ") || stripped.starts_with("if ") || stripped.starts_with("elif ") || stripped.starts_with("else") || stripped.starts_with("join ") || stripped.starts_with("leave ") || stripped.starts_with("jump ") || stripped.starts_with("label ");
                if let Some(caps) = dtl_speaker_re.captures(line) {
                    if !is_cmd {
                        texts_to_translate.push(caps["text"].to_string());
                    }
                    continue;
                }
                if !is_cmd && stripped.len() > 1 && stripped.chars().any(|c| c.is_alphabetic()) && !stripped.contains('=') && !stripped.starts_with('[') {
                    texts_to_translate.push(stripped.to_string());
                }
            } else if suffix == ".dialogue" {
                let is_cmd = stripped.starts_with('~') || stripped.starts_with("=>") || stripped.starts_with("do ") || stripped.starts_with("if ") || stripped.starts_with("elif ") || stripped.starts_with("else") || stripped.starts_with("match ");
                if !is_cmd {
                    let mut text = stripped;
                    if text.starts_with("- ") {
                        text = text[2..].trim();
                    }
                    if text.len() > 1 && text.chars().any(|c| c.is_alphabetic()) {
                        texts_to_translate.push(text.to_string());
                    }
                }
            } else if suffix == ".tscn" {
                if let Some(caps) = tscn_text_re.captures(line) {
                    texts_to_translate.push(caps["text"].to_string());
                }
            } else if suffix == ".json" {
                if let Some(caps) = json_text_re.captures(line) {
                    texts_to_translate.push(caps["text"].to_string());
                }
            } else if suffix == ".cfg" {
                let captures = if is_story_script(virtual_path) {
                    // Story files are structured dictionaries. Only dialogue is
                    // visible; character/src/audio/path fields are metadata.
                    story_dialogue_re.captures(line)
                } else {
                    cfg_text_re.captures(line)
                };
                if let Some(caps) = captures {
                    texts_to_translate.push(caps["text"].to_string());
                }
            } else if suffix == ".txt" {
                if line.len() > 1 && line.chars().any(|c| c.is_alphabetic()) {
                    texts_to_translate.push(line.to_string());
                }
            }
        }
    }

    let mut unique_texts: Vec<String> = texts_to_translate.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect();
    unique_texts.retain(|s| s.len() > 1 && s.chars().any(|c| c.is_alphabetic()));

    let _ = tx.send(UiMsg::Log(format!("Extração concluída: {} textos únicos encontrados para traduzir.", unique_texts.len())));

    if unique_texts.is_empty() {
        return Err(format!(
            "Nenhum texto foi encontrado para o idioma '{}'. Escolha um idioma existente no jogo ou use Patch direto.",
            source_lang
        ).into());
    }

    let message_ids_path = out_dir.join("godot_message_ids.json");
    fs::write(&message_ids_path, serde_json::to_string_pretty(&message_ids)?)?;
    let _ = tx.send(UiMsg::Log(format!("{} IDs nativos preservados em {}.", message_ids.len(), message_ids_path.display())));

    let tgt_code = api::get_lang_code(target_lang);

    // Load existing translation map if present and overwrite is not requested
    let mut translation_map: HashMap<String, String> = HashMap::new();
    if translated_json.exists() && !overwrite {
        if let Ok(content) = fs::read_to_string(&translated_json) {
            if let Ok(map) = serde_json::from_str(&content) {
                translation_map = map;
            }
        }
    }

    let mut to_translate: Vec<String> = Vec::new();
    let mut dict_hits = 0;

    for text in &unique_texts {
        if !translation_map.contains_key(text) {
            if let Some(std_trans) = crate::dictionary::lookup(text, tgt_code) {
                translation_map.insert(text.clone(), std_trans.to_string());
                dict_hits += 1;
            } else {
                to_translate.push(text.clone());
            }
        }
    }

    if dict_hits > 0 {
        let _ = tx.send(UiMsg::Log(format!("[Dicionário Padrão] {} termos resolvidos instantaneamente.", dict_hits)));
    }

    let batch_size = 64usize;
    let mut processed = 0usize;
    let total = to_translate.len();
    let mut was_cancelled = false;
    let mut translation_failures: Vec<String> = Vec::new();
    let client = reqwest::Client::new();
    let src_code = api::get_lang_code(source_lang);

    let total_chunks = (total + batch_size - 1) / batch_size;
    for (chunk_idx, chunk) in to_translate.chunks(batch_size).enumerate() {
        if cancelled.load(Ordering::SeqCst) {
            let _ = tx.send(UiMsg::Log("Cancelamento solicitado...".into()));
            was_cancelled = true;
            break;
        }

        let _ = tx.send(UiMsg::Log(format!(
            "Traduzindo lote {} de {} ({} blocos)...",
            chunk_idx + 1,
            total_chunks,
            chunk.len()
        )));

        let mut protected_chunks: Vec<(String, Vec<(String, String)>)> = Vec::new();
        for original in chunk {
            let protected = original.clone();
            let mut replacements: Vec<(String, String)> = Vec::new();

            let mut token_idx = 0;
            let protected_tmp = protect_re.replace_all(&protected, |caps: &regex::Captures| {
                let var = caps[0].to_string();
                let placeholder = format!("TBXVAR{}", token_idx);
                token_idx += 1;
                replacements.push((var, placeholder.clone()));
                placeholder
            }).to_string();

            protected_chunks.push((protected_tmp, replacements));
        }

        let strings_to_translate: Vec<String> = protected_chunks.iter().map(|(s, _)| s.clone()).collect();
        let ignored_tags = config.get_active_tags(Some(out_dir.join("tbx_tags.txt")));

        if let Ok(translated_chunk) = api::translate_batch_concurrent(&client, &strings_to_translate, src_code, tgt_code, threads as usize, config.usar_traducao_pivo, &ignored_tags).await {
            for (idx, mut trad) in translated_chunk.into_iter().enumerate() {
                let original = &chunk[idx];
                let (_, replacements) = &protected_chunks[idx];
                for (orig_var, placeholder) in replacements {
                    trad = trad.replace(placeholder, orig_var);
                }
                let _ = tx.send(UiMsg::Log(format!("  [OK] {} -> {}", original.replace('\n', " "), trad.replace('\n', " "))));
                translation_map.insert(original.to_string(), trad);
            }
        } else {
            let _ = tx.send(UiMsg::Log("Lote recusado pelo tradutor; tentando item por item.".into()));
            for (idx, original) in chunk.iter().enumerate() {
                if cancelled.load(Ordering::SeqCst) { was_cancelled = true; break; }
                let (protected, replacements) = &protected_chunks[idx];
                match api::translate_batch(&client, &vec![protected.clone()], api_engine, src_code, tgt_code, config.usar_traducao_pivo, &ignored_tags).await {
                    Ok(mut res) => {
                        if let Some(mut trad) = res.pop() {
                            for (orig_var, placeholder) in replacements {
                                trad = trad.replace(placeholder, orig_var);
                            }
                            let _ = tx.send(UiMsg::Log(format!("  [OK] {} -> {}", original.replace('\n', " "), trad.replace('\n', " "))));
                            translation_map.insert(original.to_string(), trad);
                        }
                    }
                    Err(error) => translation_failures.push(error),
                }
            }
        }

        processed += chunk.len();
        let _ = tx.send(UiMsg::Progress(processed, total));

        // Save intermediate progress
        if let Ok(json) = serde_json::to_string_pretty(&translation_map) {
            let _ = fs::write(&translated_json, json);
        }
    }

    if let Ok(json) = serde_json::to_string_pretty(&translation_map) {
        let _ = fs::write(&translated_json, json);
    }

    if was_cancelled {
        let _ = tx.send(UiMsg::Log(format!("[Aviso] Extração cancelada. Os textos foram salvos em {}.", translated_json.display())));
        let _ = tx.send(UiMsg::Cancelled);
    } else if !translation_failures.is_empty() {
        return Err(format!(
            "{} texto(s) não foram traduzidos. Verifique a conexão ou o bloqueio do Google Translate. Primeiro erro: {}",
            translation_failures.len(), translation_failures[0]
        ).into());
    } else {
        let _ = tx.send(UiMsg::Log(format!("Sucesso! Textos extraídos e traduzidos salvos em: {}", translated_json.display())));
        let _ = tx.send(UiMsg::Done("Extração concluída! Verifique o Editor de Tradução se quiser fazer ajustes manuais antes de Injetar.".to_string()));
    }

    Ok(())
}

pub async fn inject_translation(
    game_path: &str,
    folder_name: &str,
    source_lang: &str,
    target_lang: &str,
    requested_strategy: InjectionStrategy,
    requested_locale: &str,
    tx: Sender<UiMsg>,
) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = output_folder(game_path, folder_name, target_lang);
    let translated_json = out_dir.join("translation.json");

    if !translated_json.exists() {
        return Err("Nenhum arquivo translation.json encontrado! Faça a extração primeiro.".into());
    }

    let json_content = fs::read_to_string(&translated_json)?;
    let translation_map: HashMap<String, String> = serde_json::from_str(&json_content)?;
    let message_ids: HashMap<String, String> = fs::read_to_string(out_dir.join("godot_message_ids.json"))
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default();

    let locales = detect_native_locales(game_path).unwrap_or_default();
    let strategy = match requested_strategy {
        InjectionStrategy::Auto if !locales.locale_codes.is_empty() => InjectionStrategy::ForceNativeSlot,
        InjectionStrategy::Auto => InjectionStrategy::DirectPatch,
        selected => selected,
    };

    if strategy == InjectionStrategy::ForceNativeSlot {
        let locale = locales.locale_codes.iter().any(|code| code == requested_locale)
            .then(|| requested_locale.to_string())
            .or_else(|| resolve_native_locale(source_lang, &locales.locale_codes))
            .ok_or_else(|| "O idioma original selecionado não existe nos idiomas nativos deste jogo. Use Patch direto.".to_string())?;
        if locales.locale_codes.is_empty() {
            return Err("O jogo não possui um idioma nativo reutilizável. Use Patch direto.".into());
        }
        let native_path = locales.translation_paths.iter().find(|path| {
            path.ends_with(&format!("locale.{locale}.translation"))
        }).ok_or("Não foi encontrado o arquivo .translation do idioma selecionado.")?;
    let _ = tx.send(UiMsg::Log(format!("Compilando PT-BR no slot '{locale}' já registrado pelo jogo...")));
        // The resource keeps the original locale code. Consequently the game's
        // own language menu continues to work; selecting this slot displays PT-BR.
        let native_translation_map: HashMap<String, String> = translation_map.iter()
            .map(|(source, translated)| {
                (message_ids.get(source).cloned().unwrap_or_else(|| source.clone()), translated.clone())
            })
            .collect();
        let _ = tx.send(UiMsg::Log(format!(
            "Compilando {} mensagens ({} com ID original preservado).",
            native_translation_map.len(), message_ids.len()
        )));
        let compiled = compile_native_translation(&native_translation_map, &locale)?;
        let pck_path = locate_pck(Path::new(game_path))?;
        let pck_name = pck_path.file_stem().and_then(|v| v.to_str()).unwrap_or("game");
        let patch_pck = pck_path.with_file_name(format!("{pck_name}_patch_1.pck"));
        let mut native_files = HashMap::new();
        native_files.insert(native_path.clone(), compiled);
        godot_pck::create_patch_pck(&patch_pck, &native_files)?;
        let _ = tx.send(UiMsg::Log(format!(
            "Patch instalado em {}. No menu do jogo, escolha o idioma '{}' para usar PT-BR.",
            patch_pck.display(), locale
        )));
        let _ = tx.send(UiMsg::Done("Injeção Godot nativa concluída!".to_string()));
        return Ok(());
    }

    let exe_path = Path::new(game_path);
    let pck_ext = exe_path.with_extension("pck");

    let mut pck_path = exe_path.to_path_buf();

    if game_path.to_lowercase().ends_with(".exe") {
        if pck_ext.exists() {
            let exe_size = std::fs::metadata(&exe_path).map(|m| m.len()).unwrap_or(0);
            let pck_size = std::fs::metadata(&pck_ext).map(|m| m.len()).unwrap_or(0);
            if pck_size > exe_size || exe_size < 100_000_000 {
                pck_path = pck_ext;
            }
        }
    }

    if !pck_path.exists() {
        return Err("Arquivo PCK ou Executável não encontrado.".into());
    }

    let _ = tx.send(UiMsg::Log(format!("Lendo cabeçalho do PCK: {}", pck_path.display())));

    let mut file = fs::File::open(&pck_path).map_err(|e| format!("Falha ao abrir PCK: {}", e))?;
    let pck_archive = godot_pck::read_pck_header(&mut file)?;

    let valid_exts = [".dtl", ".dialogue", ".tscn", ".tres", ".po", ".json", ".cfg", ".txt"];
    let mut files_to_translate_content: Vec<(String, String)> = Vec::new();

    for entry in pck_archive.files {
        let path_lower = entry.path.to_lowercase();
        if path_lower.contains("credits") || path_lower.contains("patron") || path_lower.contains("supporter") {
            continue;
        }
        if valid_exts.iter().any(|ext| path_lower.ends_with(ext)) {
            let mut data = vec![0u8; entry.size as usize];
            if file.seek(SeekFrom::Start(entry.offset)).is_ok() && file.read_exact(&mut data).is_ok() {
                files_to_translate_content.push((entry.path.clone(), String::from_utf8_lossy(&data).to_string()));
            }
        }
    }

    if let Some(parent) = pck_path.parent() {
        for entry in walkdir::WalkDir::new(parent).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let path_lower = entry.path().to_string_lossy().to_lowercase();
                if path_lower.contains("credits") || path_lower.contains("patron") || path_lower.contains("supporter") {
                    continue;
                }
                if path_lower.ends_with(".exe") || path_lower.ends_with(".pck") || path_lower.contains("translation.json") || path_lower.contains("tbx_") || path_lower.ends_with(".txt") {
                    continue;
                }
                if valid_exts.iter().any(|ext| path_lower.ends_with(ext)) {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(relative_path) = entry.path().strip_prefix(parent) {
                            let virtual_path = format!("res://{}", relative_path.to_string_lossy().replace("\\", "/"));
                            files_to_translate_content.push((virtual_path, content));
                        }
                    }
                }
            }
        }
    }

    let dtl_speaker_re = Regex::new(r"^(?P<prefix>\s*[^:\r\n]+:\s*)(?P<text>.+?)(?P<suffix>\r?\n)?$").unwrap();
    let dtl_choice_re = Regex::new(r"^(?P<prefix>\s*-\s+)(?P<text>.+?)(?P<suffix>\r?\n)?$").unwrap();
    let dtl_text_attr_re = Regex::new(r#"(?i)^(?P<before>\s*\[[^\]\r\n]*?\btext\s*=\s*&?#?)(?:(?P<q_d>")(?P<t_d>[^"]*)(?P<a_d>"[^\]\r\n]*\]\s*)|(?P<q_s>')(?P<t_s>[^']*)(?P<a_s>'[^\]\r\n]*\]\s*))(?P<suffix>\r?\n)?$"#).unwrap();
    let tscn_text_re = Regex::new(r#"^(?P<prefix>\s*(?:text|placeholder_text|tooltip_text|window_title|title)\s*=\s*")(?P<text>(?:\\.|[^"])*)(?P<suffix>"\s*(?:\r?\n)?)$"#).unwrap();
    let json_text_re = Regex::new(r#"^(?P<prefix>.*?["'](?:text|dialog|dialogue|name|text_en|text_pt|text_es)["']\s*:\s*")(?P<text>(?:\\.|[^"])*)(?P<suffix>".*)$"#).unwrap();
    let cfg_text_re = Regex::new(r#"^(?P<prefix>.*?\b(?:text|dialog|dialogue|name|string)["']?\s*[=:]\s*")(?P<text>(?:\\.|[^"])*)(?P<suffix>".*)$"#).unwrap();
    let po_msgid_re = Regex::new(r#"^(?P<prefix>msgid\s*")(?P<text>(?:\\.|[^"])*)(?P<suffix>"\s*)$"#).unwrap();
    let po_msgstr_re = Regex::new(r#"^(?P<prefix>msgstr\s*")(?P<text>(?:\\.|[^"])*)(?P<suffix>"\s*)$"#).unwrap();

    let mut modified_files = HashMap::new();
    let mut total_modified = 0;

    for (virtual_path, content) in files_to_translate_content {
        let path = &virtual_path;
        let suffix = if path.to_lowercase().ends_with(".dtl") { ".dtl" }
                     else if path.to_lowercase().ends_with(".dialogue") { ".dialogue" }
                     else if path.to_lowercase().ends_with(".json") { ".json" }
                     else if path.to_lowercase().ends_with(".cfg") { ".cfg" }
                     else if path.to_lowercase().ends_with(".txt") { ".txt" }
                     else if path.to_lowercase().ends_with(".po") { ".po" }
                     else { ".tscn" };

        if suffix == ".po" {
            let mut reconstructed = String::new();
            let mut last_msgid = String::new();
            for line in content.lines() {
                let mut line_end = "\n";
                if content.contains("\r\n") { line_end = "\r\n"; }

                let stripped = line.trim();
                if stripped.is_empty() || stripped.starts_with('#') {
                    reconstructed.push_str(line);
                    reconstructed.push_str(line_end);
                    continue;
                }

                if let Some(caps) = po_msgid_re.captures(line) {
                    last_msgid = caps["text"].to_string();
                    reconstructed.push_str(line);
                    reconstructed.push_str(line_end);
                    continue;
                }

                if let Some(caps) = po_msgstr_re.captures(line) {
                    let msgstr = caps["text"].to_string();
                    let lookup_key = if !msgstr.is_empty() {
                        msgstr.clone()
                    } else {
                        last_msgid.clone()
                    };

                    if !lookup_key.is_empty() {
                        if let Some(trad) = translation_map.get(&lookup_key) {
                            reconstructed.push_str(&format!("{}msgstr \"{}\"{}", caps.name("prefix").unwrap().as_str().replace("msgstr", ""), trad.replace("\"", "\\\""), line_end));
                        } else {
                            reconstructed.push_str(line);
                            reconstructed.push_str(line_end);
                        }
                    } else {
                        reconstructed.push_str(line);
                        reconstructed.push_str(line_end);
                    }
                    continue;
                }

                reconstructed.push_str(line);
                reconstructed.push_str(line_end);
            }
            if reconstructed != content {
                modified_files.insert(path.to_string(), reconstructed.into_bytes());
                total_modified += 1;
            }
            continue;
        }

        let mut new_lines = Vec::new();
        let mut was_modified = false;

        for line in content.lines() {
            let mut line_end = "\n";
            if content.contains("\r\n") { line_end = "\r\n"; }

            let stripped = line.trim();
            if stripped.is_empty() || stripped.starts_with('#') {
                new_lines.push(format!("{}{}", line, line_end));
                continue;
            }

            let mut out_line = line.to_string();

            if suffix == ".dtl" {
                if let Some(caps) = dtl_text_attr_re.captures(line) {
                    let text = caps.name("t_d").or_else(|| caps.name("t_s")).unwrap().as_str();
                    let quote = caps.name("q_d").or_else(|| caps.name("q_s")).unwrap().as_str();
                    let after = caps.name("a_d").or_else(|| caps.name("a_s")).unwrap().as_str();

                    if let Some(trad) = translation_map.get(text) {
                        out_line = format!("{}{}{}{}", &caps["before"], quote, trad, after);
                        was_modified = true;
                    }
                } else if let Some(caps) = dtl_choice_re.captures(line) {
                    if let Some(trad) = translation_map.get(&caps["text"]) {
                        out_line = format!("{}{}", &caps["prefix"], trad);
                        was_modified = true;
                    }
                } else if let Some(caps) = dtl_speaker_re.captures(line) {
                    let is_cmd = stripped.starts_with("set ") || stripped.starts_with("if ") || stripped.starts_with("elif ") || stripped.starts_with("else") || stripped.starts_with("join ") || stripped.starts_with("leave ") || stripped.starts_with("jump ") || stripped.starts_with("label ");
                    if !is_cmd {
                        if let Some(trad) = translation_map.get(&caps["text"]) {
                            out_line = format!("{}{}", &caps["prefix"], trad);
                            was_modified = true;
                        }
                    }
                } else {
                    let is_cmd = stripped.starts_with("set ") || stripped.starts_with("if ") || stripped.starts_with("elif ") || stripped.starts_with("else") || stripped.starts_with("join ") || stripped.starts_with("leave ") || stripped.starts_with("jump ") || stripped.starts_with("label ");
                    if !is_cmd && stripped.len() > 1 && stripped.chars().any(|c| c.is_alphabetic()) && !stripped.contains('=') && !stripped.starts_with('[') {
                        if let Some(trad) = translation_map.get(stripped) {
                            let indent = line.chars().take_while(|c| c.is_whitespace()).collect::<String>();
                            out_line = format!("{}{}", indent, trad);
                            was_modified = true;
                        }
                    }
                }
            } else if suffix == ".dialogue" {
                let is_cmd = stripped.starts_with('~') || stripped.starts_with("=>") || stripped.starts_with("do ") || stripped.starts_with("if ") || stripped.starts_with("elif ") || stripped.starts_with("else") || stripped.starts_with("match ");
                if !is_cmd {
                    let mut text = stripped;
                    let mut prefix = "";
                    if text.starts_with("- ") {
                        prefix = "- ";
                        text = text[2..].trim();
                    }
                    if let Some(trad) = translation_map.get(text) {
                        let indent = line.chars().take_while(|c| c.is_whitespace()).collect::<String>();
                        out_line = format!("{}{}{}", indent, prefix, trad);
                        was_modified = true;
                    }
                }
            } else if suffix == ".tscn" {
                if let Some(caps) = tscn_text_re.captures(line) {
                    if let Some(trad) = translation_map.get(&caps["text"]) {
                        out_line = format!("{}{}{}", &caps["prefix"], trad, &caps["suffix"].trim());
                        was_modified = true;
                    }
                }
            } else if suffix == ".json" {
                if let Some(caps) = json_text_re.captures(line) {
                    if let Some(trad) = translation_map.get(&caps["text"]) {
                        out_line = format!("{}{}{}", &caps["prefix"], trad, &caps["suffix"].trim());
                        was_modified = true;
                    }
                }
            } else if suffix == ".cfg" {
                if let Some(caps) = cfg_text_re.captures(line) {
                    if let Some(trad) = translation_map.get(&caps["text"]) {
                        out_line = format!("{}{}{}", &caps["prefix"], trad, &caps["suffix"].trim());
                        was_modified = true;
                    }
                }
            } else if suffix == ".txt" {
                if let Some(trad) = translation_map.get(line) {
                    out_line = trad.clone();
                    was_modified = true;
                }
            }
            new_lines.push(format!("{}{}", out_line, line_end));
        }

        if was_modified {
            let new_content = new_lines.join("");
            modified_files.insert(path.clone(), new_content.into_bytes());
            total_modified += 1;
        }
    }

    let _ = tx.send(UiMsg::Log(format!("Encontrados {} arquivos modificados para salvar no patch.", total_modified)));

    let pck_name = pck_path.file_stem().and_then(|s| s.to_str()).unwrap_or("game");
    let patch_pck = pck_path.with_file_name(format!("{}_patch_1.pck", pck_name));

    godot_pck::create_patch_pck(&patch_pck, &modified_files)?;
    let _ = tx.send(UiMsg::Log(format!("Sucesso! Patch gerado: {}. Este modo substitui arquivos de diálogo, sem criar um override.cfg inválido.", patch_pck.display())));
    let _ = tx.send(UiMsg::Done("Injeção Godot concluída!".to_string()));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiline_po_values_and_ids() {
        let catalog = r#"
msgid "MODIFIERS_NOTE_SPEED_DESC"
msgstr ""
"Change the speed at which the notes arrive. "
"This will not change the song."

msgid "ACHIEVEMENT_DESC"
msgstr "Beat a level"
" for the first time"
"#;
        let entries = parse_po_catalog(catalog);
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|(id, value)| {
            id == "MODIFIERS_NOTE_SPEED_DESC" && value.starts_with("Change the speed")
        }));
        assert!(entries.iter().any(|(id, value)| {
            id == "ACHIEVEMENT_DESC" && value == "Beat a level for the first time"
        }));
    }

    #[test]
    fn extracts_only_visible_strings_from_binary_dialogue_resource() {
        let resource = b"RSRC\0resource_name\0How long have you been running this place?\0Long enough.\0res://dialogue.gd\0";
        let values = extract_dialogue_strings_from_resource(resource);
        assert!(values.iter().any(|value| value == "How long have you been running this place?"));
        assert!(values.iter().any(|value| value == "Long enough."));
        assert!(!values.iter().any(|value| value == "resource_name"));
    }

    #[test]
    fn locate_pck_finds_adjacent_and_linux_binaries() {
        let temp = std::env::temp_dir().join(format!("tbx-locate-pck-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        let _ = std::fs::create_dir_all(&temp);

        let linux_bin = temp.join("Game.x86_64");
        let pck_file = temp.join("Game.pck");
        std::fs::write(&linux_bin, b"ELF...").unwrap();
        std::fs::write(&pck_file, b"GDPC...").unwrap();

        let located = locate_pck(&linux_bin).unwrap();
        assert_eq!(located, pck_file);

        let direct = locate_pck(&pck_file).unwrap();
        assert_eq!(direct, pck_file);

        let _ = std::fs::remove_dir_all(&temp);
    }
}
