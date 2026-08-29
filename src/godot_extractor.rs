use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, mpsc::Sender};
use regex::Regex;
use crate::types::UiMsg;
use crate::api;
use crate::godot_pck;
use std::io::{Read, Seek, SeekFrom};

// ----- Regexes compartilhadas (extracao + injecao) -----
macro_rules! shared_regex {
    ($name:ident, $pattern:expr) => {
        #[allow(non_snake_case)]
        fn $name() -> &'static Regex {
            static RE: OnceLock<Regex> = OnceLock::new();
            RE.get_or_init(|| Regex::new($pattern).expect("regex estatica invalida"))
        }
    };
}

shared_regex!(DTL_SPEAKER_RE, r"^(?P<prefix>\s*[^:\r\n]+:\s*)(?P<text>.+?)(?P<suffix>\r?\n)?$");
shared_regex!(DTL_CHOICE_RE, r"^(?P<prefix>\s*-\s+)(?P<text>.+?)(?P<suffix>\r?\n)?$");
shared_regex!(DTL_TEXT_ATTR_RE, r#"(?i)^(?P<before>\s*\[[^\]\r\n]*?\btext\s*=\s*&?#?)(?:(?P<q_d>")(?P<t_d>[^"]*)(?P<a_d>"[^\]\r\n]*\]\s*)|(?P<q_s>')(?P<t_s>[^']*)(?P<a_s>'[^\]\r\n]*\]\s*))(?P<suffix>\r?\n)?$"#);
shared_regex!(TSCN_TEXT_RE, r#"^(?P<prefix>\s*(?:text|placeholder_text|tooltip_text|window_title|title)\s*=\s*")(?P<text>(?:\\.|[^"])*)(?P<suffix>"\s*(?:\r?\n)?)$"#);
shared_regex!(JSON_TEXT_RE, r#"^(?P<prefix>.*?["'](?:text|dialog|dialogue|name|text_en|text_pt|text_es)["']\s*:\s*")(?P<text>(?:\\.|[^"])*)(?P<suffix>".*)$"#);
shared_regex!(CFG_TEXT_RE, r#"^(?P<prefix>.*?\b(?:text|dialog|dialogue|name|string)["']?\s*[=:]\s*")(?P<text>(?:\\.|[^"])*)(?P<suffix>".*)$"#);
shared_regex!(STORY_DIALOGUE_RE, r#"^(?P<prefix>\s*"dialogue"\s*:\s*")(?P<text>(?:\\.|[^"])*)(?P<suffix>".*)$"#);
shared_regex!(PROTECT_RE, r"(\{[^{}\r\n]+\}|\\[A-Za-z]+\[[^\]\r\n]*\])");
shared_regex!(PO_MSGID_RE, r#"^(?P<prefix>msgid\s*")(?P<text>(?:\\.|[^"])*)(?P<suffix>"\s*)$"#);
shared_regex!(PO_MSGSTR_RE, r#"^(?P<prefix>msgstr\s*")(?P<text>(?:\\.|[^"])*)(?P<suffix>"\s*)$"#);
shared_regex!(PO_KEY_RE, r"^[A-Z0-9_]+$");

/// Quando o msgstr do PO está vazio, decide se o msgid é uma chave de UI
/// (ex.: `HOTMENU_SETTINGS`). Se for, tenta resolver o texto real no catálogo
/// binário `.translation` (que mapeia chave → texto). Se o catálogo não
/// resolver (OptimizedTranslation ou chave ausente), retorna `String::new()`
/// para o chamador pular — evita que a chave crua vaze para a UI do jogo
/// como "texto fonte", fazendo o usuário traduzir a chave e recebê-la de
/// volta no lugar do texto real.
/// Resolve o texto-fonte real de um msgid. Tenta primeiro o catálogo binário
/// nativo (`.translation` do locale fonte); se vier vazio, cai para o catálogo
/// cross-locale: mesmo msgid resolvido via `msgstr` de POs de outros idiomas
/// (ex.: es_MX) que sabemos conter o texto fonte equivalente.
fn resolve_po_source(
    msgid: &str,
    native_catalog: &HashMap<String, String>,
    cross_locale_catalog: &HashMap<String, String>,
) -> Option<String> {
    if let Some(real) = native_catalog.get(msgid) {
        if !real.trim().is_empty() {
            return Some(real.clone());
        }
    }
    if let Some(real) = cross_locale_catalog.get(msgid) {
        if !real.trim().is_empty() {
            return Some(real.clone());
        }
    }
    None
}

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
    path.starts_with("locale/") || path.contains("/locale/") || po_file_locale(&path).is_some()
}

/// Extrai o locale de um arquivo `.po`/`.translation` quando o nome do arquivo
/// É o próprio locale (ex.: `translation/en.po`, `po/fr.po`, `locale.en.translation`).
/// Retorna `None` para nomes que não são locale (`strings.po`, `messages.po`).
fn po_file_locale(normalized_path: &str) -> Option<String> {
    let name = normalized_path.rsplit('/').next()?;
    let stem = if let Some(s) = name.strip_suffix(".po") {
        s
    } else if let Some(rest) = name.strip_prefix("locale.") {
        rest.strip_suffix(".translation")?
    } else {
        return None;
    };
    if stem.is_empty() { return None; }
    // Locale: base 2-3 letras, opcionalmente seguida de _REGIAO (ex.: en, pt_BR, es_MX, zh_CN).
    let mut parts = stem.split('_');
    let base = parts.next()?;
    let base_ok = (2..=3).contains(&base.len())
        && base.chars().all(|c| c.is_ascii_lowercase());
    if !base_ok { return None; }
    let region_ok = parts.clone().all(|p| p.len() <= 4 && p.chars().all(|c| c.is_ascii_alphanumeric()));
    if !region_ok { return None; }
    Some(stem.to_string())
}

fn belongs_to_selected_locale(virtual_path: &str, selected_locale: &str) -> bool {
    let path = normalized_resource_path(virtual_path);
    let locale = selected_locale.to_ascii_lowercase();
    if !is_locale_resource(&path) { return selected_locale.eq_ignore_ascii_case("en"); }
    path.ends_with(&format!("locale/po/{locale}.po"))
        || path.ends_with(&format!("locale.{locale}.translation"))
        || po_file_locale(&path).map_or(false, |l| l.eq_ignore_ascii_case(&selected_locale)
            || api::get_lang_code_of_locale(&l) == api::get_lang_code_of_locale(selected_locale))
}

/// Arquivos de tradução do próprio editor/plugins (Dialogue Manager, Dialogic,
/// addons diversos). Eles vivem em `addons/.../l10n/`, `addons/.../translations/`
/// etc. e NÃO fazem parte do conteúdo do jogo — traduzi-los suja a extração
/// com centenas de strings de UI do editor (chines/ucraniano/etc.).
fn is_editor_plugin_l10n(virtual_path: &str) -> bool {
    let p = normalized_resource_path(virtual_path).to_lowercase();
    if !p.contains("addons/") && !p.starts_with(".godot/") {
        return false;
    }
    p.contains("/l10n/")
        || p.contains("/translations/")
        || p.contains("/translation/")
        || p.contains("/i18n/")
        || p.contains("/localization/")
        || p.contains("/locales/")
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

/// Cache do editor (vive em `.godot/`) — nunca contém texto do jogo.
fn is_editor_cache(virtual_path: &str) -> bool {
    let p = normalized_resource_path(virtual_path).to_lowercase();
    p.starts_with(".godot/")
        || p.contains("/.godot/")
        || p.ends_with(".import")
        || p.ends_with(".gdnative") && p.contains("editor")
}

#[allow(dead_code)]
fn is_embedded_dialogue_resource(virtual_path: &str) -> bool {
    let path = normalized_resource_path(virtual_path);
    path.ends_with(".res") && path.rsplit('/').next().is_some_and(|name| name.contains("dialogue"))
}

/// Recursos de cena/recurso binários exportados (.scn, .res) que podem carregar
/// chaves de tradução diretamente (ex.: "MAIN_MENU_PLAY" como texto do botão).
fn is_binary_scene_or_resource(virtual_path: &str) -> bool {
    let path = normalized_resource_path(virtual_path);
    path.ends_with(".scn") || path.ends_with(".res")
}

/// Varre um blob binário procurando strings ASCII que pareçam chaves PO
/// (ALL_CAPS_COM_UNDERSCORE). Retorna chaves únicas em ordem de aparição.
/// Usado para detectar tr-keys embutidos em .scn/.res exportados, quando o
/// jogo chama `tr("MINHA_CHAVE")` com a chave definida como texto do nó.
fn extract_tr_keys_from_binary(data: &[u8]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (idx, byte) in data.iter().copied().chain(std::iter::once(0)).enumerate() {
        let ok = byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_';
        if ok {
            start.get_or_insert(idx);
        } else if let Some(begin) = start.take() {
            if idx > begin {
                if let Ok(s) = std::str::from_utf8(&data[begin..idx]) {
                    // Exige ao menos um underscore para evitar pegar constantes
                    // binárias curtas ("RSRC", "GDSC", etc.) e ruído genérico.
                    if s.len() >= 5 && s.len() <= 64 && s.contains('_') && PO_KEY_RE().is_match(s) {
                        if seen.insert(s.to_string()) {
                            out.push(s.to_string());
                        }
                    }
                }
            }
        }
    }
    out
}

/// Exported `.res` dialogue resources retain their visible UTF-8 strings even
/// though their object structure is binary. Restrict this scanner to resources
/// whose filename is explicitly dialogue-related and reject Godot metadata.
fn extract_dialogue_strings_from_resource(data: &[u8]) -> Vec<String> {
    fn useful(value: &str) -> bool {
        let value = value.trim();
        // Tamanho mínimo 2 e máximo razoável (textos de menus/notas são curtos a moderados).
        if value.len() < 2 || value.len() > 4096 {
            return false;
        }
        // Filtros óbvios de Godot interno / paths.
        if value.starts_with("res://") || value.starts_with("uid://")
            || value.starts_with("local://") || value.contains("metadata/")
            || value.starts_with("NodePath(") || value.starts_with("Vector")
            || value.starts_with("Transform") || value.starts_with("Color(")
        {
            return false;
        }
        const INTERNAL: &[&str] = &[
            "RSRC", "Resource", "GDScript", "Script", "resource_local_to_scene", "resource_name",
            "script", "input", "responses", "next_dialogue_set", "ShopDialogueOptionData",
            "ShopDialogueSetData", "ShopDialogueData",
            "Node", "Node2D", "Control", "CanvasLayer", "MarginContainer",
            "HBoxContainer", "VBoxContainer", "GridContainer",
            "PackedScene", "Texture2D", "Font", "Theme", "StyleBox",
            "anchor_left", "anchor_right", "anchor_top", "anchor_bottom",
            "offset_left", "offset_right", "offset_top", "offset_bottom",
            "texture_filter", "texture_repeat", "stretch_mode",
        ];
        if INTERNAL.contains(&value) { return false; }
        // Identificador puro tipo `some_snake_case` ou `CamelCaseId`: provavelmente
        // nome de nó / propriedade, não texto visível. Aceita se tiver espaço ou
        // pontuação além de `_`/`.`/`/`/`-`.
        let has_letter = value.chars().any(|c| c.is_alphabetic());
        if !has_letter { return false; }
        let looks_like_identifier = value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '/' || c == '-');
        if looks_like_identifier { return false; }
        // Exige pelo menos um caractere comum de texto visível (espaço, pontuação
        // amigável) OU tamanho curto com capital (título de botão "Settings").
        let has_visible_punct = value.chars().any(|c| matches!(c, ' ' | '!' | '?' | ',' | ':' | '\'' | '"' | '(' | ')' | '[' | ']' | '%' | '&'));
        let short_title = value.len() <= 28 && value.chars().next().map_or(false, |c| c.is_uppercase());
        if !has_visible_punct && !short_title { return false; }
        value.chars().filter(|c| c.is_control()).count() == 0
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

/// Converte uma chave tr() ALL_CAPS_SNAKE_CASE ("MAIN_MENU_PLAY") em texto
/// legível para o tradutor ("Play"). Remove prefixos de nomespaco comuns
/// (MENU_, MAIN_MENU_, HOTMENU_, MODIFIERS_, etc.) e aplica Title Case.
fn humanize_tr_key(key: &str) -> String {
    // Prefixos maiores primeiro para evitar cortes parciais.
    const PREFIXES: &[&str] = &[
        "MAIN_MENU_", "PAUSE_MENU_", "HOTMENU_", "MODIFIERS_", "ACHIEVEMENT_",
        "COLLECTABLE_", "POWER_UP_", "LEVEL_EDITOR_", "LEVEL_CREATOR_",
        "LEVEL_SELECT_", "CUTSCENE_", "SCENE_VIEWER_", "PROJECT_SETTINGS_",
        "FILE_DROP_IMAGE_", "DIFFICULTY_MENU_", "DIFFICULTY_", "GAMEPLAY_",
        "CONTROLS_", "WARNING_SCREEN_", "SPLASH_SCREEN_", "CHARACTER_",
        "CREATE_CUTSCENE_", "MODPACK_", "MODS_", "AUDIO_", "VIDEO_", "TOYS_",
        "PHOTO_", "OUTFIT_", "LEVEL_", "MENU_", "GAME_", "SHOP_", "UI_",
        "EXTRAS_", "CREDITS_", "SCORE_", "FAIL_", "HOTKEY_", "MUSIC_",
        "DLC_", "FLAG_", "DIFF_", "PAUSE_", "LOADING_",
        // Cobertura extra para Beat Banger / jogos com menus de calibração e
        // estado de gameplay: garante que `GAMEPLAY_DOWNSCROLL` e
        // `FAIL_LEVEL_FAILED` virem "Downscroll"/"Level Failed" em vez de
        // ficarem crus.
        "CALIBRATION_", "SETTINGS_", "OPTIONS_", "LANGUAGE_", "LOCALE_",
        "GAME_OVER_", "RESULT_", "RESULTS_", "WIN_", "LOSE_", "DEATH_",
        "STAGE_", "BOSS_", "ENEMY_", "ITEM_", "SKILL_", "SPELL_",
    ];
    let mut s = key;
    for p in PREFIXES {
        if let Some(rest) = s.strip_prefix(p) {
            if !rest.is_empty() { s = rest; break; }
        }
    }
    // snake_case para "Title Case" com espacos.
    let mut out = String::with_capacity(s.len());
    let mut uppercase_next = true;
    for ch in s.chars() {
        if ch == '_' {
            out.push(' ');
            uppercase_next = true;
        } else if uppercase_next {
            out.push(ch.to_ascii_uppercase());
            uppercase_next = false;
        } else {
            out.push(ch.to_ascii_lowercase());
        }
    }
    let trimmed = out.trim();
    if trimmed.is_empty() { key.to_string() } else { trimmed.to_string() }
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
    // Aceita POs do locale fonte em QUALQUER pasta (ex.: `locale/po/en.po`,
    // `translation/en.po`), mas apenas os cujo nome bata com o locale fonte.
    let src_base = api::get_lang_code_of_locale(&source_locale);
    let has_selected_po = pck_archive.files.iter().any(|entry| {
        let p = normalized_resource_path(&entry.path);
        po_file_locale(&p).map_or(false, |l| api::get_lang_code_of_locale(&l) == src_base)
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

    // Catálogo cross-locale: msgid → texto resolvido de outros idiomas.
    // Para msgids cujo msgstr no PO fonte está vazio, procuramos a tradução
    // real de outro locale (ex.: es_MX) que contém o texto fonte presente
    // naquele msgid. Serve como "significado da chave" quando não há texto
    // no catálogo binário.
    let mut cross_locale_catalog: HashMap<String, String> = HashMap::new();
    {
        use std::io::Read;
        for entry in pck_archive.files.iter() {
            let p = normalized_resource_path(&entry.path);
            // Só POs cujo nome é um locale e NÃO é o locale fonte.
            let Some(file_locale) = po_file_locale(&p) else { continue };
            if api::get_lang_code_of_locale(&file_locale) == src_base { continue; }
            let mut data = vec![0u8; entry.size as usize];
            if file.seek(SeekFrom::Start(entry.offset)).is_ok() && file.read_exact(&mut data).is_ok() {
                if let Ok(text) = std::str::from_utf8(&data) {
                    for (msgid, msgstr) in parse_po_catalog(text) {
                        if msgid.trim().is_empty() || msgstr.trim().is_empty() { continue; }
                        // Primeiro preenchido ganha; preferimos os que já existam
                        // para não sobrescrever com locale posterior.
                        cross_locale_catalog.entry(msgid).or_insert(msgstr);
                    }
                }
            }
        }
    }
    if !cross_locale_catalog.is_empty() {
        let _ = tx.send(UiMsg::Log(format!(
            "Catálogo cross-locale: {} msgids com significado extraído de outros idiomas.",
            cross_locale_catalog.len()
        )));
    }

    // Scanner binário de recursos exportados (.scn/.res). Pega:
    //  a) textos visuais embutidos (Label.text, Button.text, RichTextLabel.bbcode, etc.)
    //  b) chaves tr() ALL_CAPS usadas como texto (ex.: MAIN_MENU_PLAY)
    // Roda para QUALQUER locale, mas com duas proteções:
    //  - em non-EN, só extrai CHAVES tr() (textos planos geralmente estão em EN
    //    ou no locale do jogo, e misturar causaria ruído);
    //  - exclui .res/.scn que moram em pastas de locale de OUTROS idiomas.
    let is_en = source_locale.eq_ignore_ascii_case("en");
    let mut embedded_text_count = 0usize;
    let mut scene_key_count = 0usize;
    for entry in pck_archive.files.iter().filter(|entry| is_binary_scene_or_resource(&entry.path)) {
        // Não pega traduções de outros idiomas embutidas em pastas locale/<outro>.
        let norm = normalized_resource_path(&entry.path);
        if is_locale_resource(&norm) && !belongs_to_selected_locale(&entry.path, &source_locale) {
            continue;
        }
        let mut data = vec![0u8; entry.size as usize];
        if file.seek(SeekFrom::Start(entry.offset)).is_ok() && file.read_exact(&mut data).is_ok() {
            if is_en {
                for source in extract_dialogue_strings_from_resource(&data) {
                    native_catalog.entry(source.clone()).or_insert(source);
                    embedded_text_count += 1;
                }
            }
            for key in extract_tr_keys_from_binary(&data) {
                if !native_catalog.contains_key(&key) {
                    let human = humanize_tr_key(&key);
                    native_catalog.insert(key.clone(), human);
                    scene_key_count += 1;
                }
            }
        }
    }
    if embedded_text_count > 0 {
        let _ = tx.send(UiMsg::Log(format!(
            "Recursos binários (.scn/.res): {} textos visíveis encontrados.", embedded_text_count
        )));
    }
    if scene_key_count > 0 {
        let _ = tx.send(UiMsg::Log(format!(
            "Chaves de UI embutidas em cenas binárias: {} encontradas (ex.: MAIN_MENU_PLAY).", scene_key_count
        )));
    }

    let valid_exts = [".dtl", ".dialogue", ".tscn", ".tres", ".po", ".json", ".cfg", ".txt"];
    let mut files_to_translate_content: Vec<(String, String)> = Vec::new();

    for entry in pck_archive.files {
        let path_lower = entry.path.to_lowercase();

        if path_lower.contains("credits") || path_lower.contains("patron") || path_lower.contains("supporter") {
            continue;
        }

        let accepted_language = belongs_to_selected_locale(&entry.path, &source_locale) && !is_editor_plugin_l10n(&entry.path) && !is_editor_cache(&entry.path);
        let norm = normalized_resource_path(&entry.path);
        let is_selected_po = po_file_locale(&norm)
            .map_or(false, |l| api::get_lang_code_of_locale(&l) == src_base);
        let accepted_source = (is_story_script(&entry.path) && !is_editor_plugin_l10n(&entry.path) && !is_editor_cache(&entry.path)) || if has_selected_po {
            is_selected_po
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
                            let norm = normalized_resource_path(&virtual_path);
                            let is_selected_po = po_file_locale(&norm)
                                .map_or(false, |l| api::get_lang_code_of_locale(&l) == src_base);
                            let accepted_source = (is_story_script(&virtual_path) && !is_editor_plugin_l10n(&virtual_path) && !is_editor_cache(&virtual_path)) || if has_selected_po {
                                is_selected_po
                            } else {
                                belongs_to_selected_locale(&virtual_path, &source_locale) && !is_editor_plugin_l10n(&virtual_path) && !is_editor_cache(&virtual_path)
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
    // Múltiplos msgids podem compartilhar a MESMA fonte humanizada
    // (ex.: MENU_SHOP e MAIN_MENU_SHOP ambos viram "Shop"). Guardamos todos
    // para a injeção aplicar a tradução em TODOS os msgids equivalentes.
    let mut message_ids: HashMap<String, Vec<String>> = HashMap::new();
    // Mapa id→texto para resolver msgstr vazio apontando para chave de UI
    // (ex.: HOTMENU_SETTINGS) usando o catálogo binário nativo.
    let native_catalog_by_id: HashMap<String, String> = native_catalog.clone();
    for (message_id, source_text) in native_catalog {
        let source = if source_text.trim().is_empty() { message_id.clone() } else { source_text };
        if !source.trim().is_empty() {
            let entry = message_ids.entry(source.clone()).or_default();
            if !entry.contains(&message_id) {
                entry.push(message_id);
            }
            texts_to_translate.push(source);
        }
    }

    // Regexes compartilhadas via OnceLock (compiladas uma unica vez por processo).
    // BBCode (`[wave]`, `[color=...]`, etc.) e tratado segmento a segmento pelo
    // api.rs; aqui apenas variaveis/diretivas de template precisam de protecao.
    let dtl_speaker_re = DTL_SPEAKER_RE();
    let dtl_choice_re = DTL_CHOICE_RE();
    let dtl_text_attr_re = DTL_TEXT_ATTR_RE();
    let tscn_text_re = TSCN_TEXT_RE();
    let json_text_re = JSON_TEXT_RE();
    let cfg_text_re = CFG_TEXT_RE();
    let story_dialogue_re = STORY_DIALOGUE_RE();
    let protect_re = PROTECT_RE();

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
                    // msgstr vazio: se msgid parecer chave de UI, extrai o texto
                    // real do catálogo binário (.translation); se o catálogo
                    // não conhecer a chave, pula para não plantar chave como
                    // fonte traduzível (mantém o comportamento do jogo, que
                    // cai no msgid cru quando não há tradução).
                    if PO_KEY_RE().is_match(&message_id) {
                        match resolve_po_source(&message_id, &native_catalog_by_id, &cross_locale_catalog) {
                            Some(text) => text,
                            None => continue,
                        }
                    } else {
                        message_id.clone()
                    }
                } else {
                    translated_source
                };
                if !source.trim().is_empty() {
                    let entry = message_ids.entry(source.clone()).or_default();
                    if !entry.contains(&message_id) {
                        entry.push(message_id);
                    }
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

    // Aviso sobre colisões de humanização: uma fonte traduzida cobrirá
    // vários msgids no jogo (comportamento desejado), mas logar para debug.
    let colliding: Vec<(String, usize)> = message_ids.iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(k, v)| (k.clone(), v.len()))
        .collect();
    if !colliding.is_empty() {
        let _ = tx.send(UiMsg::Log(format!(
            "[Info] {} fontes humanizadas cobrem múltiplas chaves (ex.: {}).",
            colliding.len(),
            colliding.iter().take(3).map(|(k, n)| format!("{k} ({n}x)")).collect::<Vec<_>>().join(", ")
        )));
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
        let ignored_tags = config.get_active_tags(Some(out_dir.join("tbx_tags.txt")), 2);

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

/// Detecta a versao major da Godot usada pelo jogo.
/// Ordem: (1) strings no executavel, (2) format_version do PCK.
fn detect_godot_version(exe_path: &Path, pck_archive: &godot_pck::PckArchive) -> u8 {
    detect_godot_version_full(exe_path, pck_archive).0
}

/// Detecta a versao completa (major, "4.7-stable") do Godot do jogo.
/// Retorna (major: u8, version_string: String).
fn detect_godot_version_full(exe_path: &Path, pck_archive: &godot_pck::PckArchive) -> (u8, String) {
    if let Ok(mut file) = fs::File::open(exe_path) {
        let mut buf = Vec::new();
        use std::io::Read as _;
        let _ = file.by_ref().take(16 * 1024 * 1024).read_to_end(&mut buf);
        let text: String = buf.iter()
            .filter(|b| b.is_ascii_graphic() || **b == b' ' || **b == b'.' || **b == b'_' || **b == b'-')
            .map(|b| *b as char).collect();
        if let Some(pos) = text.find("Godot Engine v") {
            let rest = &text[pos + "Godot Engine v".len()..];
            let mut ver = String::new();
            for c in rest.chars() {
                if c.is_ascii_alphanumeric() || c == '.' || c == '-' { ver.push(c); }
                else { break; }
            }
            // "4.7.stable.official.5b4e0cb0f" -> normalizar para "4.7-stable"
            let parts: Vec<&str> = ver.split('.').collect();
            if parts.len() >= 2 {
                let major = parts[0].parse::<u8>().unwrap_or(4);
                let minor = parts[1];
                // Parte 3 costuma ser "stable" | "rc1" | "beta3" | etc
                let channel = parts.get(2).copied().unwrap_or("stable");
                let version_str = format!("{}.{}-{}", major, minor, channel);
                return (major, version_str);
            }
            let major = ver.chars().next().and_then(|c| c.to_digit(10)).unwrap_or(4) as u8;
            return (major, format!("{}.0-stable", major));
        }
    }
    let major = if pck_archive.format_version > 1 { 4 } else { 3 };
    (major, format!("{}.0-stable", major))
}

/// Retorna o caminho para um binario Godot standalone compativel com a versao.
/// Faz download para ~/.cache/tbx-translator/godot/{versao}/ se ausente.
fn ensure_godot_binary(version_full: &str, tx: &Sender<UiMsg>) -> Result<PathBuf, String> {
    let base = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("tbx-translator")
        .join("godot")
        .join(version_full);
    
    #[cfg(target_os = "windows")]
    let bin_name = format!("Godot_v{}_win64.exe", version_full);
    #[cfg(target_os = "macos")]
    let bin_name = format!("Godot_v{}_macos.universal", version_full);
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let bin_name = format!("Godot_v{}_linux.x86_64", version_full);
    
    let bin_path = base.join(&bin_name);
    if bin_path.exists() {
        return Ok(bin_path);
    }
    
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let _ = tx.send(UiMsg::Log(format!("[Info] Baixando Godot {} (~70MB, uma vez)...", version_full)));
    
    #[cfg(target_os = "windows")]
    let zip_name = format!("Godot_v{}_win64.exe.zip", version_full);
    #[cfg(target_os = "macos")]
    let zip_name = format!("Godot_v{}_macos.universal.zip", version_full);
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let zip_name = format!("Godot_v{}_linux.x86_64.zip", version_full);
    
    let url = format!("https://github.com/godotengine/godot/releases/download/{}/{}", version_full, zip_name);
    let zip_path = base.join("godot.zip");
    
    // curl para simplificar (reqwest ja esta como dependencia, mas curl e universal)
    let status = std::process::Command::new("curl")
        .args(["-L", "-f", "-sS", "-o", zip_path.to_str().unwrap_or("godot.zip"), &url])
        .status()
        .map_err(|e| format!("curl nao encontrado: {}", e))?;
    if !status.success() {
        return Err(format!("Falha ao baixar Godot de {}", url));
    }
    
    // Extrai com unzip (janela nativa / linux: ambos tem)
    let status = std::process::Command::new("unzip")
        .args(["-o", zip_path.to_str().unwrap_or("godot.zip")])
        .current_dir(&base)
        .status()
        .map_err(|e| format!("unzip nao encontrado: {}", e))?;
    if !status.success() {
        return Err("Falha ao extrair Godot zip".into());
    }
    let _ = fs::remove_file(&zip_path);
    
    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&bin_path, fs::Permissions::from_mode(0o755));
    }
    
    if !bin_path.exists() {
        return Err(format!("Binario Godot nao encontrado apos extracao: {:?}", bin_path));
    }
    let _ = tx.send(UiMsg::Log(format!("[OK] Godot {} pronto em {:?}", version_full, base)));
    Ok(bin_path)
}

fn run_godot_headless_patcher(
    exe_path: &Path,
    translation_json: &Path,
    pck_archive: &godot_pck::PckArchive,
    modified_files: &mut HashMap<String, Vec<u8>>,
    tx: &Sender<UiMsg>,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let parent = exe_path.parent().unwrap_or(Path::new(""));
    let temp_dir = parent.join("tbx_temp_patch");
    let _ = fs::create_dir_all(&temp_dir);
    
    let patcher_script = temp_dir.join("tbx_patcher.gd");
    let (godot_major, godot_ver_str) = detect_godot_version_full(exe_path, pck_archive);
    let godot_4 = godot_major >= 4;
    
    let mut files_to_patch = Vec::new();
    for entry in &pck_archive.files {
        let l = entry.path.to_lowercase();
        if l.ends_with(".res") || l.ends_with(".tres") || l.ends_with(".gdc") {
            files_to_patch.push(entry.path.clone());
        }
    }
    
    if files_to_patch.is_empty() {
        return Ok(());
    }

    let out_dir_escaped = temp_dir.to_string_lossy().replace("\\", "/");
    let json_escaped = translation_json.to_string_lossy().replace("\\", "/");
    
    let mut gdscript = String::new();
    gdscript.push_str("extends SceneTree\n\n");
    gdscript.push_str("func _init():\n");
    gdscript.push_str(&format!("    var out_dir = \"{}\"\n", out_dir_escaped));
    gdscript.push_str(&format!("    var json_path = \"{}\"\n", json_escaped));
    
    if godot_4 {
        gdscript.push_str("    var file = FileAccess.open(json_path, FileAccess.READ)\n");
        gdscript.push_str("    if file == null: quit(1); return\n");
        gdscript.push_str("    var json = JSON.new()\n");
        gdscript.push_str("    json.parse(file.get_as_text())\n");
        gdscript.push_str("    var map = json.data\n");
    } else {
        gdscript.push_str("    var file = File.new()\n");
        gdscript.push_str("    if file.open(json_path, File.READ) != OK: quit(1); return\n");
        gdscript.push_str("    var p = JSON.parse(file.get_as_text())\n");
        gdscript.push_str("    var map = p.result\n");
    }
    
    gdscript.push_str("    var files = [\n");
    for f in &files_to_patch {
        gdscript.push_str(&format!("        \"{}\",\n", f));
    }
    gdscript.push_str("    ]\n");
    
    // G1: preserva o caminho relativo ao res:// ao salvar (evita colisao de nomes).
    if godot_4 {
        gdscript.push_str("    for res_path in files:\n");
        gdscript.push_str("        var res = ResourceLoader.load(res_path)\n");
        gdscript.push_str("        if res != null:\n");
        gdscript.push_str("            var modified = false\n");
        gdscript.push_str("            for prop in res.get_property_list():\n");
        gdscript.push_str("                if prop.type == TYPE_STRING:\n");
        gdscript.push_str("                    var val = res.get(prop.name)\n");
        gdscript.push_str("                    if typeof(val) == TYPE_STRING and map.has(val):\n");
        gdscript.push_str("                        res.set(prop.name, map[val])\n");
        gdscript.push_str("                        modified = true\n");
        gdscript.push_str("            if modified:\n");
        gdscript.push_str("                var rel = res_path.trim_prefix(\"res://\")\n");
        gdscript.push_str("                var dest = out_dir + \"/\" + rel\n");
        gdscript.push_str("                DirAccess.make_dir_recursive_absolute(dest.get_base_dir())\n");
        gdscript.push_str("                ResourceSaver.save(res, dest)\n");
    } else {
        gdscript.push_str("    for res_path in files:\n");
        gdscript.push_str("        var res = ResourceLoader.load(res_path)\n");
        gdscript.push_str("        if res != null:\n");
        gdscript.push_str("            var modified = false\n");
        gdscript.push_str("            for prop in res.get_property_list():\n");
        gdscript.push_str("                if prop.type == TYPE_STRING:\n");
        gdscript.push_str("                    var val = res.get(prop.name)\n");
        gdscript.push_str("                    if typeof(val) == TYPE_STRING and map.has(val):\n");
        gdscript.push_str("                        res.set(prop.name, map[val])\n");
        gdscript.push_str("                        modified = true\n");
        gdscript.push_str("            if modified:\n");
        gdscript.push_str("                var rel = res_path.trim_prefix(\"res://\")\n");
        gdscript.push_str("                var dest = out_dir + \"/\" + rel\n");
        gdscript.push_str("                var dir = Directory.new()\n");
        gdscript.push_str("                dir.make_dir_recursive(dest.get_base_dir())\n");
        gdscript.push_str("                ResourceSaver.save(dest, res)\n");
    }
    // OS.kill força saida sincrona (quit(0) e agendado para o proximo frame,
    // o que deixa plugins nativos do autoload inicializarem e crasharem o processo
    // antes do quit acontecer em jogos com Steam/Discord SDK).
    gdscript.push_str("    OS.kill(OS.get_process_id())\n");
    
    fs::write(&patcher_script, gdscript)?;
    let _ = tx.send(UiMsg::Log("Rodando injeção headless Godot para arquivos binários...".into()));
    
    let arg = if godot_4 { "--headless" } else { "--no-window" };
    let _ = tx.send(UiMsg::Log(format!("Detectado Godot {} ({}) — executando injeção headless...", godot_major, godot_ver_str)));
    
    // G5/G6: resolve o binário nativo do Godot (baixa se precisar) em TODOS os OS.
    // Roda com --main-pack <exe do jogo> para carregar o PCK embutido, e -s <script>
    // para executar o patcher. Não usamos mais o exe do jogo / wine — isso evita
    // o crash dos plugins Steam/Discord e a falta de suporte ao -s.
    let godot_bin = ensure_godot_binary(&godot_ver_str, tx)
        .map_err(|e| format!("ensure_godot_binary: {}", e))?;
    let _ = tx.send(UiMsg::Log(format!("[Info] Usando Godot nativo em {:?}", godot_bin)));
    
    let mut child = std::process::Command::new(&godot_bin)
        .arg(arg)
        .arg("--main-pack")
        .arg(exe_path)
        .arg("-s")
        .arg(&patcher_script)
        .current_dir(parent)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    
    let mut output = None;
    let timeout = std::time::Duration::from_secs(300);
    let start = std::time::Instant::now();
    let mut timed_out = false;
    loop {
        if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            let _ = child.kill();
            return Err("Cancelado pelo usuário".into());
        }
        if let Some(_) = child.try_wait()? {
            output = Some(child.wait_with_output()?);
            break;
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            timed_out = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    
    if timed_out {
        let _ = tx.send(UiMsg::Log(format!(
            "[Aviso] Injeção headless excedeu 300s (wine iniciando prefixo ou jogo abrindo janela). \
             Arquivos .res binários NÃO foram modificados — apenas texto puro (.cfg/.dtl/.po) será injetado."
        )));
        let _ = fs::remove_dir_all(&temp_dir);
        return Ok(());
    }
    
    let output = output.unwrap();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = tx.send(UiMsg::Log(format!("[Aviso] Headless retornou codigo {}: {}", output.status, stderr.lines().take(3).collect::<Vec<_>>().join(" | "))));
    }
        
    // G1: le de volta usando o mesmo caminho relativo usado no patcher.
    let mut modified_count = 0;
    for f in &files_to_patch {
        let rel = f.strip_prefix("res://").unwrap_or(f);
        let patched_path = temp_dir.join(rel);
        if patched_path.exists() {
            if let Ok(data) = fs::read(&patched_path) {
                modified_files.insert(f.clone(), data);
                modified_count += 1;
            }
        }
    }
    
    let _ = tx.send(UiMsg::Log(format!("{} arquivos binários modificados nativamente.", modified_count)));
    let _ = fs::remove_dir_all(&temp_dir);
    
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
    cancelled: Arc<std::sync::atomic::AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = output_folder(game_path, folder_name, target_lang);
    let translated_json = out_dir.join("translation.json");

    if !translated_json.exists() {
        return Err("Nenhum arquivo translation.json encontrado! Faça a extração primeiro.".into());
    }

    let json_content = fs::read_to_string(&translated_json)?;
    let translation_map: HashMap<String, String> = serde_json::from_str(&json_content)?;
    // Backward-compat: formato antigo (HashMap<String,String>) é convertido
    // para o novo (HashMap<String,Vec<String>>).
    let message_ids: HashMap<String, Vec<String>> = fs::read_to_string(out_dir.join("godot_message_ids.json"))
        .ok()
        .and_then(|content| {
            serde_json::from_str::<HashMap<String, Vec<String>>>(&content).ok()
                .or_else(|| serde_json::from_str::<HashMap<String, String>>(&content).ok()
                    .map(|m| m.into_iter().map(|(k, v)| (k, vec![v])).collect()))
        })
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
        let mut resolved = 0usize;
        let mut kept_as_key = 0usize;
        let mut native_translation_map: HashMap<String, String> = HashMap::new();
        for (source, translated) in translation_map.iter() {
            if let Some(ids) = message_ids.get(source) {
                // Uma fonte humanizada pode corresponder a várias chaves
                // (MENU_SHOP / MAIN_MENU_SHOP). Injetamos TODAS.
                for id in ids {
                    native_translation_map.insert(id.clone(), translated.clone());
                    resolved += 1;
                }
            } else {
                // Source JÁ é a chave quando veio de extract_tr_keys_from_binary
                // (source==key). Nesse caso, usar source como msgid é o
                // comportamento correto. Só é "perigoso" quando source é
                // texto real sem mapeamento — contaremos para logar.
                kept_as_key += 1;
                native_translation_map.insert(source.clone(), translated.clone());
            }
        }
        let _ = tx.send(UiMsg::Log(format!(
            "Compilando {} mensagens ({} com ID mapeado, {} usando a própria fonte como msgid).",
            native_translation_map.len(), resolved, kept_as_key
        )));
        let compiled = compile_native_translation(&native_translation_map, &locale)?;
        let pck_path = locate_pck(Path::new(game_path))?;
        let mut native_files_buf = HashMap::new();
        native_files_buf.insert(native_path.clone(), compiled);

        // Preferência: reembutir diretamente no exe/PCK original via gdre_tools
        // (não depende de override.cfg nem de ordem de carregamento de patch PCK).
        // target_lang vem do parâmetro; cobre qualquer idioma, não só PT-BR.
        let target_label = api::get_lang_name(api::get_lang_code(target_lang)).to_string();
        if let Err(e) = embed_patched_file(game_path, &pck_path, &native_files_buf, &locale, &target_label, &tx) {
            let _ = tx.send(UiMsg::Log(format!(
                "[Aviso] Injeção direta via gdre_tools falhou ({e}); gerando patch PCK separado."
            )));
            let pck_name = pck_path.file_stem().and_then(|v| v.to_str()).unwrap_or("game");
            let patch_pck = pck_path.with_file_name(format!("{pck_name}_patch_1.pck"));
            godot_pck::create_patch_pck(&patch_pck, &native_files_buf)?;
            let _ = tx.send(UiMsg::Log(format!(
                "Patch instalado em {}. No menu do jogo, escolha o idioma '{}' para usar PT-BR.",
                patch_pck.display(), locale
            )));
        }
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

    for entry in &pck_archive.files {
        let path_lower = entry.path.to_lowercase();
        if path_lower.contains("credits") || path_lower.contains("patron") || path_lower.contains("supporter") {
            continue;
        }
        if is_editor_plugin_l10n(&entry.path) {
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
                            if !is_editor_plugin_l10n(&virtual_path) {
                                files_to_translate_content.push((virtual_path, content));
                            }
                        }
                    }
                }
            }
        }
    }

    let dtl_speaker_re = DTL_SPEAKER_RE();
    let dtl_choice_re = DTL_CHOICE_RE();
    let dtl_text_attr_re = DTL_TEXT_ATTR_RE();
    let tscn_text_re = TSCN_TEXT_RE();
    let json_text_re = JSON_TEXT_RE();
    let cfg_text_re = CFG_TEXT_RE();
    let po_msgid_re = PO_MSGID_RE();
    let po_msgstr_re = PO_MSGSTR_RE();

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

    // --- INJEÇÃO NATIVA PARA ARQUIVOS BINÁRIOS ---
    if let Err(e) = run_godot_headless_patcher(Path::new(game_path), &translated_json, &pck_archive, &mut modified_files, &tx, cancelled.clone()) {
        let _ = tx.send(UiMsg::Log(format!("[Aviso] A injeção headless Godot falhou: {}", e)));
    }
    // ---------------------------------------------

    let _ = tx.send(UiMsg::Log(format!("Encontrados {} arquivos modificados para salvar no patch.", modified_files.len())));

    let pck_name = pck_path.file_stem().and_then(|s| s.to_str()).unwrap_or("game");
    let patch_pck = pck_path.with_file_name(format!("{}_patch_1.pck", pck_name));

    // Tenta reembutir diretamente no exe/PCK original; se gdre_tools não estiver
    // disponível (ou falhar), cai no patch PCK separado como antes.
    let target_label = api::get_lang_name(api::get_lang_code(target_lang)).to_string();
    // Usamos o próprio target_lang como "tag" no arquivo final. Se o jogo pedir
    // um código de locale específico via requested_locale, quem manda é ele;
    // aqui no DirectPatch vale o idioma-alvo do usuário.
    let tag = api::get_lang_code(target_lang);
    match embed_patched_file(game_path, &pck_path, &modified_files, tag, &target_label, &tx) {
        Ok(()) => {
            let _ = tx.send(UiMsg::Log("Sucesso! Tradução embutida diretamente no jogo (sem override.cfg).".into()));
        }
        Err(e) => {
            let _ = tx.send(UiMsg::Log(format!(
                "[Aviso] Injeção direta via gdre_tools falhou ({e}); gerando patch PCK separado."
            )));
            godot_pck::create_patch_pck(&patch_pck, &modified_files)?;
            let _ = tx.send(UiMsg::Log(format!("Patch gerado: {}. Este modo substitui arquivos de diálogo, sem criar um override.cfg inválido.", patch_pck.display())));
        }
    }
    let _ = tx.send(UiMsg::Done("Injeção Godot concluída!".to_string()));

    Ok(())
}

/// Grava os arquivos modificados diretamente dentro do exe/PCK original usando
/// GDRE Tools (`--pck-patch --embed`). O original nunca é sobrescrito: gera um
/// `<nome>_ptbr.exe|.pck` ao lado, e backup `<nome>.tbx_bak` do original.
fn embed_patched_file(
    game_path: &str,
    pck_path: &Path,
    files: &HashMap<String, Vec<u8>>,
    tag: &str,
    target_label: &str,
    tx: &Sender<UiMsg>,
) -> Result<(), String> {
    if files.is_empty() {
        return Err("Nenhum arquivo para embutir.".into());
    }
    let gdre = crate::gdre_tools::locate()
        .ok_or_else(|| "gdre_tools não encontrado".to_string())?;
    let _ = tx.send(UiMsg::Log(format!("Usando GDRE Tools: {}", gdre.display())));

    // Materializa os arquivos modificados numa pasta temporária.
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    let staging = std::env::temp_dir().join(format!("tbx-gdre-patch-{}-{}", std::process::id(), unique));
    fs::create_dir_all(&staging).map_err(|e| e.to_string())?;

    let result = (|| -> Result<(), String> {
        let mut pairs: Vec<(PathBuf, String)> = Vec::new();
        for (res_path, data) in files {
            let safe = res_path.trim_start_matches("res://").replace('/', "__");
            let local = staging.join(&safe);
            fs::write(&local, data).map_err(|e| format!("Falha ao escrever {}: {e}", local.display()))?;
            pairs.push((local, res_path.clone()));
        }

        // ---- Autoload do seletor de idioma (TBX Locale Menu) ----
        // Estratégia: extrair project.binary -> converter para texto -> editar
        // [autoload] e lista de translations -> re-converter para binário ->
        // incluir junto no patch. Falha aqui não impede o patch principal:
        // registra aviso e segue sem o seletor.
        let _ = tx.send(UiMsg::Log("Registrando autoload do seletor de idioma...".into()));
        match install_locale_menu_autoload(game_path, pck_path, tag, target_label, &staging, &mut pairs, tx) {
            Ok(true) => { let _ = tx.send(UiMsg::Log("Seletor de idioma incluído via autoload.".into())); },
            Ok(false) => { let _ = tx.send(UiMsg::Log("[Aviso] Autoload do seletor não aplicado (jogo sem project.binary acessível?).".into())); },
            Err(e) => { let _ = tx.send(UiMsg::Log(format!("[Aviso] Falha ao registrar autoload: {e} (seguindo sem ele)."))); },
        }

        let game = Path::new(game_path);
        let is_exe_embedded = game_path.to_lowercase().ends_with(".exe") && pck_path == game;

        // Alvo de saída: nunca sobrescrever o original.
        let ext = if is_exe_embedded { "exe" } else { "pck" };
        let stem = game.file_stem().and_then(|s| s.to_str()).unwrap_or("game");
        let output = game.with_file_name(format!("{stem}_{tag}.{ext}"));

        // Backup do original (uma vez).
        let backup = game.with_file_name(format!("{}.tbx_bak", game.file_name().and_then(|s| s.to_str()).unwrap_or("game")));
        if !backup.exists() {
            fs::copy(game, &backup).map_err(|e| format!("Falha ao criar backup {}: {e}", backup.display()))?;
            let _ = tx.send(UiMsg::Log(format!("Backup do original: {}", backup.display())));
        }

        let _ = tx.send(UiMsg::Log(format!(
            "Embutindo {} arquivo(s) traduzido(s) em {} ...", pairs.len(), output.display()
        )));
        let embed_src = if is_exe_embedded { Some(game) } else { None };
        crate::gdre_tools::patch_embed(pck_path, &pairs, embed_src, &output)?;
        let _ = tx.send(UiMsg::Log(format!(
            "Jogo traduzido gerado: {}. Execute este arquivo (o original está intacto).",
            output.display()
        )));
        Ok(())
    })();

    let _ = fs::remove_dir_all(&staging);
    result
}

/// Instala o autoload do seletor de idioma.
///
/// Passos:
///  1. Abre o PCK (ou exe embutido) e lê `project.binary`.
///  2. Converte para texto via gdre (`bin_to_txt`).
///  3. Injeta `TBXLocaleMenu="*res://tbx/tbx_locale_menu.gd"` em `[autoload]`
///     e garante `internationalization/locale/translations` incluindo o
///     arquivo `.translation` traduzido já presente em `files`.
///  4. Converte de volta (`txt_to_bin`) e adiciona `res://project.binary`
///     + `res://tbx/tbx_locale_menu.gd` ao lote de patch.
fn install_locale_menu_autoload(
    game_path: &str,
    pck_path: &Path,
    tag: &str,
    target_label: &str,
    staging: &Path,
    pairs: &mut Vec<(PathBuf, String)>,
    tx: &Sender<UiMsg>,
) -> Result<bool, String> {

    // Localiza project.binary no PCK.
    let mut file = fs::File::open(pck_path).map_err(|e| format!("PCK: {e}"))?;
    let archive = godot_pck::read_pck_header(&mut file)?;
    let entry = match archive.files.iter().find(|e| e.path == "project.binary") {
        Some(e) => e,
        None => return Ok(false),
    };
    let mut bytes = vec![0u8; entry.size as usize];
    file.seek(SeekFrom::Start(entry.offset)).map_err(|e| e.to_string())?;
    file.read_exact(&mut bytes).map_err(|e| e.to_string())?;

    let bin_local = staging.join("project.binary");
    fs::write(&bin_local, &bytes).map_err(|e| e.to_string())?;

    // Converte para texto.
    let out_txt = match crate::gdre_tools::bin_to_txt(&bin_local) {
        Ok(p) => p,
        Err(e) => return Err(format!("bin_to_txt falhou: {e}")),
    };
    let mut text = fs::read_to_string(&out_txt).map_err(|e| e.to_string())?;

    // Edita [autoload].
    if !text.contains("TBXLocaleMenu") {
        let autoload_line = "TBXLocaleMenu=\"*res://tbx/tbx_locale_menu.gd\"";
        if text.contains("[autoload]") {
            text = text.replace("[autoload]", &format!("[autoload]\n{}", autoload_line));
        } else {
            text.push_str("\n[autoload]\n");
            text.push_str(autoload_line);
            text.push_str("\n");
        }
    }

    // Injeta locale translation na lista.
    let target_res = format!("res://locale.{}.translation", tag.to_lowercase());
    if !text.contains(&target_res) {
        if let Some(pos) = text.find("internationalization/locale/translations") {
            // Se já houver lista, adiciona item ao final.
            if let Some(bracket_pos) = text[pos..].find('[').map(|i| pos + i) {
                if let Some(close_bracket) = text[bracket_pos..].find(']').map(|i| bracket_pos + i) {
                    let inside = &text[bracket_pos + 1..close_bracket];
                    let new_item = format!("\"{}\"", target_res);
                    let new_inside = if inside.trim().is_empty() {
                        new_item
                    } else {
                        format!("{}, {}", inside.trim(), new_item)
                    };
                    text.replace_range(bracket_pos + 1..close_bracket, &new_inside);
                }
            }
        } else {
            text.push_str(&format!("\n[internationalization]\nlocale/translations=PackedStringArray(\"{}\")\n", target_res));
        }
    }

    let out_bin = crate::gdre_tools::txt_to_bin(&out_txt).map_err(|e| format!("txt_to_bin falhou: {e}"))?;
    let bin_data = fs::read(&out_bin).map_err(|e| e.to_string())?;
    let staged_bin = staging.join("project.patched.binary");
    fs::write(&staged_bin, &bin_data).map_err(|e| e.to_string())?;
    pairs.push((staged_bin, "res://project.binary".to_string()));

    // Adiciona o .gd do autoload.
    let gd_src = autoload_template_path();
    if !gd_src.is_file() {
        return Err(format!("Template do autoload ausente em {}", gd_src.display()));
    }
    let staged_gd = staging.join("tbx_locale_menu.gd");
    // Substitui placeholders pelo idioma-alvo real (qualquer idioma, não
    // apenas pt_BR). Se o placeholder sumir por engano, ainda assim copia o
    // template puro — o autoload cai nos defaults pt_BR internos dele.
    let mut content = fs::read_to_string(&gd_src).map_err(|e| e.to_string())?;
    let tbx_locale = tag.replace('-', "_");
    let escape_gd = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    content = content
        .replace("__TBX_TARGET_LOCALE__", &escape_gd(&tbx_locale))
        .replace("__TBX_TARGET_LABEL__", &escape_gd(target_label));
    fs::write(&staged_gd, content).map_err(|e| e.to_string())?;
    pairs.push((staged_gd, "res://tbx/tbx_locale_menu.gd".to_string()));

    let _ = tx.send(UiMsg::Log(format!(
        "Autoload TBXLocaleMenu incluído (path: {}).",
        game_path
    )));
    Ok(true)
}

/// Caminho onde o template do autoload é esperado no disco. Primeiro tenta
/// perto do exe do TBX (empacotado), depois o diretório do repo.
fn autoload_template_path() -> PathBuf {
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            cands.push(d.join("TBX_Injector/godot/tbx_locale_menu.gd"));
            cands.push(d.join("../TBX_Injector/godot/tbx_locale_menu.gd"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        cands.push(cwd.join("TBX_Injector/godot/tbx_locale_menu.gd"));
    }
    cands.into_iter().find(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from("TBX_Injector/godot/tbx_locale_menu.gd"))
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
