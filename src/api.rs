use reqwest::Client;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};
use regex::Regex;

fn global_request_limiter() -> &'static tokio::sync::Semaphore {
    static LIMITER: OnceLock<tokio::sync::Semaphore> = OnceLock::new();
    LIMITER.get_or_init(|| tokio::sync::Semaphore::new(4))
}

type TranslationCache = HashMap<(String, String, String), String>;

fn translation_cache() -> &'static Mutex<TranslationCache> {
    static CACHE: OnceLock<Mutex<TranslationCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

// Keep the URL query comfortably below common proxy/server limits. The limit
// is measured after percent encoding, which matters for accented/CJK text.
const MAX_ENCODED_TEXT_LEN: usize = 1_800;

fn encoded_len(text: &str) -> usize {
    urlencoding::encode(text).len()
}

fn split_oversized_unit(unit: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    for ch in unit.chars() {
        let mut candidate = current.clone();
        candidate.push(ch);
        if !current.is_empty() && encoded_len(&candidate) > MAX_ENCODED_TEXT_LEN {
            parts.push(std::mem::take(&mut current));
        }
        current.push(ch);
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

/// Split only when necessary, preserving every character and favoring natural
/// sentence/word boundaries. The caller joins translated pieces in order.
fn split_for_translation(text: &str) -> Vec<String> {
    if encoded_len(text) <= MAX_ENCODED_TEXT_LEN {
        return vec![text.to_string()];
    }

    let mut parts = Vec::new();
    let mut current = String::new();
    for unit in text.split_inclusive(|ch: char| ch.is_whitespace() || matches!(ch, '.' | '!' | '?' | ';' | ':')) {
        if encoded_len(unit) > MAX_ENCODED_TEXT_LEN {
            if !current.is_empty() {
                parts.push(std::mem::take(&mut current));
            }
            parts.extend(split_oversized_unit(unit));
            continue;
        }

        let mut candidate = current.clone();
        candidate.push_str(unit);
        if !current.is_empty() && encoded_len(&candidate) > MAX_ENCODED_TEXT_LEN {
            parts.push(std::mem::take(&mut current));
        }
        current.push_str(unit);
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

pub async fn detect_language(client: &Client, text: &str) -> Option<String> {
    let encoded = urlencoding::encode(text);
    let uri = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=en&dt=t&q={}",
        encoded
    );

    if let Ok(res) = client.get(&uri).header("User-Agent", "Mozilla/5.0").send().await {
        if let Ok(json_text) = res.text().await {
            if let Ok(v) = serde_json::from_str::<Value>(&json_text) {
                if let Some(arr) = v.as_array() {
                    if let Some(lang) = arr.get(2).and_then(|v| v.as_str()) {
                        return Some(lang.to_string());
                    }
                }
            }
        }
    }
    None
}

async fn translate_one(
    client: &Client,
    line: &str,
    from_lang: &str,
    to_lang: &str,
) -> Result<String, String> {
    translate_one_internal(client, line, from_lang, to_lang).await
}

async fn translate_one_internal(
    client: &Client,
    line: &str,
    from_lang: &str,
    to_lang: &str,
) -> Result<String, String> {
    let cache_key = (from_lang.to_string(), to_lang.to_string(), line.to_string());
    if let Ok(cache) = translation_cache().lock() {
        if let Some(translated) = cache.get(&cache_key) {
            return Ok(translated.clone());
        }
    }

    let encoded = urlencoding::encode(line);
    let uri = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
        from_lang, to_lang, encoded
    );

    for attempt in 0..3u64 {
        // Shared by every engine/job in this process. Running Ren'Py, Unity and
        // Godot together therefore never multiplies the outbound burst above 4.
        let _permit = global_request_limiter().acquire().await
            .map_err(|_| "Limitador global de tradução foi encerrado".to_string())?;
        let res = client
            .get(&uri)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await;

        match res {
        Ok(response) => {
            if (response.status().as_u16() == 429 || response.status().is_server_error()) && attempt < 2 {
                drop(_permit);
                tokio::time::sleep(std::time::Duration::from_millis(750 * (1 << attempt))).await;
                continue;
            }
            if !response.status().is_success() {
                return Err(format!("Google Translate respondeu HTTP {}", response.status()));
            }

            let json_text = response.text().await.map_err(|e| format!("Não foi possível ler a resposta da tradução: {e}"))?;
            if let Ok(v) = serde_json::from_str::<Value>(&json_text) {
                    if let Some(arr) = v.as_array() {
                        if let Some(first) = arr.get(0).and_then(|a| a.as_array()) {
                            let translated: String = first.iter()
                                .filter_map(|segment| segment.as_array()
                                    .and_then(|values| values.first())
                                    .and_then(|value| value.as_str()))
                                .collect();

                            if !translated.is_empty() {
                                if let Ok(mut cache) = translation_cache().lock() {
                                    cache.insert(cache_key.clone(), translated.clone());
                                }
                                return Ok(translated);
                            }
                        }
                    }
                }

            return Err("Google Translate retornou uma resposta inválida; nenhum texto foi marcado como traduzido.".to_string());
        }
        Err(_error) if attempt < 2 => {
            drop(_permit);
            tokio::time::sleep(std::time::Duration::from_millis(500 * (1 << attempt))).await;
            continue;
        }
        Err(error) => return Err(format!("Não foi possível acessar Google Translate: {error}")),
        }
    }
    Err("O tradutor não respondeu após três tentativas".to_string())
}

async fn translate_visible_segment(
    client: &Client,
    text: &str,
    from_lang: &str,
    to_lang: &str,
    usar_pivo: bool,
) -> Result<String, String> {
    let without_left = text.trim_start_matches(char::is_whitespace);
    let left_len = text.len() - without_left.len();
    let core = without_left.trim_end_matches(char::is_whitespace);
    let right_start = left_len + core.len();
    if core.is_empty() {
        return Ok(text.to_string());
    }
    let translated = if usar_pivo && from_lang != "en" && to_lang != "en" {
        let en = translate_one(client, core, from_lang, "en").await?;
        translate_one(client, &en, "en", to_lang).await?
    } else {
        translate_one(client, core, from_lang, to_lang).await?
    };
    Ok(format!("{}{}{}", &text[..left_len], translated, &text[right_start..]))
}

/// Translate one visual line at a time and restore its exact line terminator.
/// This preserves instruction lists/dialogue formatting in both Ren'Py and
/// Unity while still splitting an individual oversized line safely.
async fn translate_preserving_lines(
    client: &Client,
    text: &str,
    from_lang: &str,
    to_lang: &str,
    usar_pivo: bool,
    tags_ignoradas: &[String],
) -> Result<String, String> {
    let mut translated = String::new();
    for raw_line in text.split_inclusive('\n') {
        let (line_with_optional_cr, terminator) = match raw_line.strip_suffix('\n') {
            Some(line) => (line, "\n"),
            None => (raw_line, ""),
        };
        let (line, carriage_return) = match line_with_optional_cr.strip_suffix('\r') {
            Some(line) => (line, "\r"),
            None => (line_with_optional_cr, ""),
        };

        translated.push_str(&translate_preserving_markup(client, line, from_lang, to_lang, usar_pivo, tags_ignoradas).await?);
        translated.push_str(carriage_return);
        translated.push_str(terminator);
    }
    Ok(translated)
}

fn find_next_markup(text: &str, protected_words: &[String]) -> Option<(usize, usize)> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(?x)
        \[[^\]]*\] |
        \{[^}]*\} |
        <[^>]*> |
        %\([^)]+\)[sdf] |
        %[sdf]
    ").unwrap());
    
    let mut earliest: Option<(usize, usize)> = None;
    
    // Find earliest regex match
    if let Some(mat) = re.find(text) {
        earliest = Some((mat.start(), mat.end()));
    }
    
    // Find earliest protected word match
    for word in protected_words {
        let trimmed = word.trim();
        if trimmed.is_empty() { continue; }
        
        if let Some(pos) = text.find(trimmed) {
            if earliest.is_none() || pos < earliest.unwrap().0 {
                earliest = Some((pos, pos + trimmed.len()));
            }
        }
    }
    
    earliest
}

/// Translate only visible text and copy Godot/RichText BBCode tags byte for
/// byte. Example: `[wave][color=pink]Hello[/color][/wave]` sends only `Hello`
/// to the service and reconstructs the original tag structure afterwards.
async fn translate_preserving_markup(
    client: &Client,
    text: &str,
    from_lang: &str,
    to_lang: &str,
    usar_pivo: bool,
    tags_ignoradas: &[String],
) -> Result<String, String> {
    if text.trim().is_empty() {
        return Ok(text.to_string());
    }

    let mut output = String::with_capacity(text.len());
    let mut cursor = 0usize;
    while let Some((relative_open, relative_close)) = find_next_markup(&text[cursor..], tags_ignoradas) {
        let open = cursor + relative_open;
        let close = cursor + relative_close;
        
        let visible = &text[cursor..open];
        for piece in split_for_translation(visible) {
            if !piece.is_empty() {
                output.push_str(&translate_visible_segment(client, &piece, from_lang, to_lang, usar_pivo).await?);
            }
        }

        output.push_str(&text[open..close]);
        cursor = close;
    }

    for piece in split_for_translation(&text[cursor..]) {
        if !piece.is_empty() {
            output.push_str(&translate_visible_segment(client, &piece, from_lang, to_lang, usar_pivo).await?);
        }
    }
    Ok(output)
}

pub async fn translate_batch(
    client: &Client,
    texts: &[String],
    _url: &str,
    from_lang: &str,
    to_lang: &str,
    usar_pivo: bool,
    tags_ignoradas: &[String],
) -> Result<Vec<String>, String> {
    let mut translations = Vec::with_capacity(texts.len());

    for text in texts {
        let line = text.as_str();
        if line.trim().is_empty() {
            translations.push(String::new());
            continue;
        }

        translations.push(translate_preserving_lines(client, line, from_lang, to_lang, usar_pivo, tags_ignoradas).await?);
    }

    Ok(translations)
}

const PACK_SEPARATOR: &str = "\n⟦TBXITEM⟧\n";
const PACK_MAX_ITEMS: usize = 16;
const PACK_MAX_ENCODED_LEN: usize = 1_500;

#[derive(Clone)]
struct PackedItem {
    index: usize,
    masked: String,
    left_space: String,
    right_space: String,
    tags: Vec<(String, String)>,
}

fn prepare_packed_item(index: usize, text: &str, tags_ignoradas: &[String]) -> PackedItem {
    let without_left = text.trim_start_matches(char::is_whitespace);
    let left_len = text.len() - without_left.len();
    let core = without_left.trim_end_matches(char::is_whitespace);
    let right_start = left_len + core.len();
    let mut masked = String::with_capacity(core.len());
    let mut tags = Vec::new();
    let mut cursor = 0usize;

    while let Some((relative_open, relative_close)) = find_next_markup(&core[cursor..], tags_ignoradas) {
        let open = cursor + relative_open;
        let close = cursor + relative_close;
        
        masked.push_str(&core[cursor..open]);
        let token = format!("⟦TBXT{index:04}_{:03}⟧", tags.len());
        tags.push((token.clone(), core[open..close].to_string()));
        masked.push_str(&token);
        
        cursor = close;
    }
    masked.push_str(&core[cursor..]);

    PackedItem {
        index,
        masked,
        left_space: text[..left_len].to_string(),
        right_space: text[right_start..].to_string(),
        tags,
    }
}

async fn translate_packed_group(
    client: &Client,
    items: &[PackedItem],
    from_lang: &str,
    to_lang: &str,
    usar_pivo: bool,
) -> Result<Vec<(usize, String)>, String> {
    let payload = items
        .iter()
        .map(|item| item.masked.as_str())
        .collect::<Vec<_>>()
        .join(PACK_SEPARATOR);
    if encoded_len(&payload) > MAX_ENCODED_TEXT_LEN {
        return Err("Lote excedeu o limite seguro da URL".to_string());
    }
    
    let translated = if usar_pivo && from_lang != "en" && to_lang != "en" {
        let en = translate_one(client, &payload, from_lang, "en").await?;
        translate_one(client, &en, "en", to_lang).await?
    } else {
        translate_one(client, &payload, from_lang, to_lang).await?
    };
    let pieces: Vec<&str> = translated.split("⟦TBXITEM⟧").collect();
    if pieces.len() != items.len() {
        return Err("O serviço alterou os separadores do lote".to_string());
    }

    let mut output = Vec::with_capacity(items.len());
    for (item, piece) in items.iter().zip(pieces) {
        let mut restored = piece.trim_matches('\n').to_string();
        for (token, tag) in &item.tags {
            if !restored.contains(token) {
                return Err("O serviço alterou um marcador de formatação".to_string());
            }
            restored = restored.replace(token, tag);
        }
        output.push((
            item.index,
            format!("{}{}{}", item.left_space, restored, item.right_space),
        ));
    }
    Ok(output)
}

/// Packs up to 16 dialogues into each request and processes a bounded number
/// of packs simultaneously. A shared ceiling of four outbound requests keeps
/// concurrent engine jobs from multiplying the burst rate.
pub async fn translate_batch_concurrent(
    client: &Client,
    texts: &[String],
    from_lang: &str,
    to_lang: &str,
    requested_concurrency: usize,
    usar_pivo: bool,
    tags_ignoradas: &[String],
) -> Result<Vec<String>, String> {
    let concurrency = requested_concurrency.clamp(1, 4);
    if texts.len() <= 1 {
        return translate_batch(client, texts, "", from_lang, to_lang, usar_pivo, tags_ignoradas).await;
    }

    let mut results: Vec<Option<String>> = vec![None; texts.len()];
    let mut groups: Vec<Vec<PackedItem>> = Vec::new();
    let mut current_group = Vec::new();
    let mut current_encoded_len = 0usize;

    for (index, text) in texts.iter().enumerate() {
        if text.trim().is_empty() {
            results[index] = Some(String::new());
            continue;
        }
        let item = prepare_packed_item(index, text, tags_ignoradas);
        let item_len = encoded_len(&item.masked)
            + if current_group.is_empty() { 0 } else { encoded_len(PACK_SEPARATOR) };
        if !current_group.is_empty()
            && (current_group.len() >= PACK_MAX_ITEMS
                || current_encoded_len + item_len > PACK_MAX_ENCODED_LEN)
        {
            groups.push(std::mem::take(&mut current_group));
            current_encoded_len = 0;
        }
        current_encoded_len += encoded_len(&item.masked)
            + if current_group.is_empty() { 0 } else { encoded_len(PACK_SEPARATOR) };
        current_group.push(item);
    }
    if !current_group.is_empty() {
        groups.push(current_group);
    }

    let mut next = 0usize;
    let mut jobs = tokio::task::JoinSet::new();

    while next < groups.len() || !jobs.is_empty() {
        while next < groups.len() && jobs.len() < concurrency {
            let group_number = next;
            let group = groups[next].clone();
            let client = client.clone();
            let from = from_lang.to_string();
            let to = to_lang.to_string();
            let stagger = (group_number % concurrency) as u64 * 60;
            let usar_pivo_c = usar_pivo;
            let tags_ignoradas_c = tags_ignoradas.to_vec();
            jobs.spawn(async move {
                if stagger > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(stagger)).await;
                }
                match translate_packed_group(&client, &group, &from, &to, usar_pivo_c).await {
                    Ok(translated) => Ok::<_, String>(translated),
                    Err(error)
                        if error == "O serviço alterou os separadores do lote"
                            || error == "O serviço alterou um marcador de formatação"
                            || error == "Lote excedeu o limite seguro da URL" =>
                    {
                        let mut fallback = Vec::with_capacity(group.len());
                        for item in group {
                            let original = format!("{}{}{}", item.left_space, item.masked, item.right_space);
                            let mut translated = translate_preserving_lines(&client, &original, &from, &to, usar_pivo_c, &tags_ignoradas_c).await?;
                            for (token, tag) in &item.tags {
                                translated = translated.replace(token, tag);
                            }
                            fallback.push((item.index, translated));
                        }
                        Ok(fallback)
                    }
                    Err(error) => Err(error),
                }
            });
            next += 1;
        }

        if let Some(joined) = jobs.join_next().await {
            let translated_group = joined
                .map_err(|error| format!("Tarefa de tradução interrompida: {error}"))??;
            for (index, translated) in translated_group {
                results[index] = Some(translated);
            }
        }
    }

    results.into_iter()
        .map(|value| value.ok_or_else(|| "Resposta de tradução ausente".to_string()))
        .collect()
}

pub fn get_lang_code(name: &str) -> &'static str {
    match name {
        "Afrikaans" | "af" => "af",
        "Albanian" | "sq" => "sq",
        "Amharic" | "am" => "am",
        "Arabic" | "ar" => "ar",
        "Armenian" | "hy" => "hy",
        "Azerbaijani" | "az" => "az",
        "Basque" | "eu" => "eu",
        "Belarusian" | "be" => "be",
        "Bengali" | "bn" => "bn",
        "Bosnian" | "bs" => "bs",
        "Bulgarian" | "bg" => "bg",
        "Catalan" | "ca" => "ca",
        "Cebuano" | "ceb" => "ceb",
        "Chichewa" | "ny" => "ny",
        "Chinese (Simplified)" | "zh-CN" => "zh-CN",
        "Chinese (Traditional)" | "zh-TW" => "zh-TW",
        "Corsican" | "co" => "co",
        "Croatian" | "hr" => "hr",
        "Czech" | "cs" => "cs",
        "Danish" | "da" => "da",
        "Dutch" | "nl" => "nl",
        "English" | "Inglês" | "ingles" | "inglês" | "en" => "en",
        "Esperanto" | "eo" => "eo",
        "Estonian" | "et" => "et",
        "Filipino" | "tl" => "tl",
        "Finnish" | "fi" => "fi",
        "French" | "fr" => "fr",
        "Frisian" | "fy" => "fy",
        "Galician" | "gl" => "gl",
        "Georgian" | "ka" => "ka",
        "German" | "de" => "de",
        "Greek" | "el" => "el",
        "Gujarati" | "gu" => "gu",
        "Haitian Creole" | "ht" => "ht",
        "Hausa" | "ha" => "ha",
        "Hawaiian" | "haw" => "haw",
        "Hebrew" | "iw" => "iw",
        "Hindi" | "hi" => "hi",
        "Hmong" | "hmn" => "hmn",
        "Hungarian" | "hu" => "hu",
        "Icelandic" | "is" => "is",
        "Igbo" | "ig" => "ig",
        "Indonesian" | "id" => "id",
        "Irish" | "ga" => "ga",
        "Italian" | "it" => "it",
        "Japanese" | "Japonês" | "japones" | "japonês" | "ja" => "ja",
        "Javanese" | "jw" => "jw",
        "Kannada" | "kn" => "kn",
        "Kazakh" | "kk" => "kk",
        "Khmer" | "km" => "km",
        "Korean" | "ko" => "ko",
        "Kurdish (Kurmanji)" | "ku" => "ku",
        "Kyrgyz" | "ky" => "ky",
        "Lao" | "lo" => "lo",
        "Latin" | "la" => "la",
        "Latvian" | "lv" => "lv",
        "Lithuanian" | "lt" => "lt",
        "Luxembourgish" | "lb" => "lb",
        "Macedonian" | "mk" => "mk",
        "Malagasy" | "mg" => "mg",
        "Malay" | "ms" => "ms",
        "Malayalam" | "ml" => "ml",
        "Maltese" | "mt" => "mt",
        "Maori" | "mi" => "mi",
        "Marathi" | "mr" => "mr",
        "Mongolian" | "mn" => "mn",
        "Myanmar (Burmese)" | "my" => "my",
        "Nepali" | "ne" => "ne",
        "Norwegian" | "no" => "no",
        "Pashto" | "ps" => "ps",
        "Persian" | "fa" => "fa",
        "Polish" | "pl" => "pl",
        "Portuguese" | "pt" | "Português" | "português" | "portuguese" => "pt",
        "Punjabi" | "pa" => "pa",
        "Romanian" | "ro" => "ro",
        "Russian" | "Russo" | "russo" | "ru" => "ru",
        "Samoan" | "sm" => "sm",
        "Scots Gaelic" | "gd" => "gd",
        "Serbian" | "sr" => "sr",
        "Sesotho" | "st" => "st",
        "Shona" | "sn" => "sn",
        "Sindhi" | "sd" => "sd",
        "Sinhala" | "si" => "si",
        "Slovak" | "sk" => "sk",
        "Slovenian" | "sl" => "sl",
        "Somali" | "so" => "so",
        "Spanish" | "Espanhol" | "espanhol" | "es" => "es",
        "Sundanese" | "su" => "su",
        "Swahili" | "sw" => "sw",
        "Swedish" | "sv" => "sv",
        "Tajik" | "tg" => "tg",
        "Tamil" | "ta" => "ta",
        "Telugu" | "te" => "te",
        "Thai" | "th" => "th",
        "Turkish" | "tr" => "tr",
        "Ukrainian" | "uk" => "uk",
        "Urdu" | "ur" => "ur",
        "Uzbek" | "uz" => "uz",
        "Vietnamese" | "vi" => "vi",
        "Welsh" | "cy" => "cy",
        "Xhosa" | "xh" => "xh",
        "Yiddish" | "yi" => "yi",
        "Yoruba" | "yo" => "yo",
        "Zulu" | "zu" => "zu",
        _ => "auto",
    }
}

pub fn get_lang_name(code: &str) -> &'static str {
    let base_code = code.split('_').next().unwrap_or(code);
    match base_code {
        "af" => "Afrikaans",
        "sq" => "Albanian",
        "am" => "Amharic",
        "ar" => "Arabic",
        "hy" => "Armenian",
        "az" => "Azerbaijani",
        "eu" => "Basque",
        "be" => "Belarusian",
        "bn" => "Bengali",
        "bs" => "Bosnian",
        "bg" => "Bulgarian",
        "ca" => "Catalan",
        "ceb" => "Cebuano",
        "ny" => "Chichewa",
        "zh-CN" => "Chinese (Simplified)",
        "zh-TW" => "Chinese (Traditional)",
        "co" => "Corsican",
        "hr" => "Croatian",
        "cs" => "Czech",
        "da" => "Danish",
        "nl" => "Dutch",
        "en" => "English",
        "eo" => "Esperanto",
        "et" => "Estonian",
        "tl" => "Filipino",
        "fi" => "Finnish",
        "fr" => "French",
        "fy" => "Frisian",
        "gl" => "Galician",
        "ka" => "Georgian",
        "de" => "German",
        "el" => "Greek",
        "gu" => "Gujarati",
        "ht" => "Haitian Creole",
        "ha" => "Hausa",
        "haw" => "Hawaiian",
        "iw" => "Hebrew",
        "hi" => "Hindi",
        "hmn" => "Hmong",
        "hu" => "Hungarian",
        "is" => "Icelandic",
        "ig" => "Igbo",
        "id" => "Indonesian",
        "ga" => "Irish",
        "it" => "Italian",
        "ja" => "Japanese",
        "jw" => "Javanese",
        "kn" => "Kannada",
        "kk" => "Kazakh",
        "km" => "Khmer",
        "ko" => "Korean",
        "ku" => "Kurdish (Kurmanji)",
        "ky" => "Kyrgyz",
        "lo" => "Lao",
        "la" => "Latin",
        "lv" => "Latvian",
        "lt" => "Lithuanian",
        "lb" => "Luxembourgish",
        "mk" => "Macedonian",
        "mg" => "Malagasy",
        "ms" => "Malay",
        "ml" => "Malayalam",
        "mt" => "Maltese",
        "mi" => "Maori",
        "mr" => "Marathi",
        "mn" => "Mongolian",
        "my" => "Myanmar (Burmese)",
        "ne" => "Nepali",
        "no" => "Norwegian",
        "ps" => "Pashto",
        "fa" => "Persian",
        "pl" => "Polish",
        "pt" => "Portuguese",
        "pa" => "Punjabi",
        "ro" => "Romanian",
        "ru" => "Russian",
        "sm" => "Samoan",
        "gd" => "Scots Gaelic",
        "sr" => "Serbian",
        "st" => "Sesotho",
        "sn" => "Shona",
        "sd" => "Sindhi",
        "si" => "Sinhala",
        "sk" => "Slovak",
        "sl" => "Slovenian",
        "so" => "Somali",
        "es" => "Spanish",
        "su" => "Sundanese",
        "sw" => "Swahili",
        "sv" => "Swedish",
        "tg" => "Tajik",
        "ta" => "Tamil",
        "te" => "Telugu",
        "th" => "Thai",
        "tr" => "Turkish",
        "uk" => "Ukrainian",
        "ur" => "Urdu",
        "uz" => "Uzbek",
        "vi" => "Vietnamese",
        "cy" => "Welsh",
        "xh" => "Xhosa",
        "yi" => "Yiddish",
        "yo" => "Yoruba",
        "zu" => "Zulu",
        _ => "Detectar Automaticamente",
    }
}

pub const ALL_LANGUAGES: &[&str] = &[
    "Afrikaans",
    "Albanian",
    "Amharic",
    "Arabic",
    "Armenian",
    "Azerbaijani",
    "Basque",
    "Belarusian",
    "Bengali",
    "Bosnian",
    "Bulgarian",
    "Catalan",
    "Cebuano",
    "Chichewa",
    "Chinese (Simplified)",
    "Chinese (Traditional)",
    "Corsican",
    "Croatian",
    "Czech",
    "Danish",
    "Dutch",
    "English",
    "Esperanto",
    "Estonian",
    "Filipino",
    "Finnish",
    "French",
    "Frisian",
    "Galician",
    "Georgian",
    "German",
    "Greek",
    "Gujarati",
    "Haitian Creole",
    "Hausa",
    "Hawaiian",
    "Hebrew",
    "Hindi",
    "Hmong",
    "Hungarian",
    "Icelandic",
    "Igbo",
    "Indonesian",
    "Irish",
    "Italian",
    "Japanese",
    "Javanese",
    "Kannada",
    "Kazakh",
    "Khmer",
    "Korean",
    "Kurdish (Kurmanji)",
    "Kyrgyz",
    "Lao",
    "Latin",
    "Latvian",
    "Lithuanian",
    "Luxembourgish",
    "Macedonian",
    "Malagasy",
    "Malay",
    "Malayalam",
    "Maltese",
    "Maori",
    "Marathi",
    "Mongolian",
    "Myanmar (Burmese)",
    "Nepali",
    "Norwegian",
    "Pashto",
    "Persian",
    "Polish",
    "Portuguese",
    "Punjabi",
    "Romanian",
    "Russian",
    "Samoan",
    "Scots Gaelic",
    "Serbian",
    "Sesotho",
    "Shona",
    "Sindhi",
    "Sinhala",
    "Slovak",
    "Slovenian",
    "Somali",
    "Spanish",
    "Sundanese",
    "Swahili",
    "Swedish",
    "Tajik",
    "Tamil",
    "Telugu",
    "Thai",
    "Turkish",
    "Ukrainian",
    "Urdu",
    "Uzbek",
    "Vietnamese",
    "Welsh",
    "Xhosa",
    "Yiddish",
    "Yoruba",
    "Zulu",
];

#[cfg(test)]
mod tests {
    use super::{encoded_len, split_for_translation, MAX_ENCODED_TEXT_LEN};

    #[test]
    fn splits_large_text_without_losing_characters() {
        let text = format!("{}{}", "Olá, mundo! ".repeat(400), "漢字".repeat(400));
        let parts = split_for_translation(&text);
        assert!(parts.len() > 1);
        assert!(parts.iter().all(|part| encoded_len(part) <= MAX_ENCODED_TEXT_LEN));
        assert_eq!(parts.concat(), text);
    }

    #[test]
    fn splitting_keeps_each_original_line_intact() {
        let text = "Primeira linha\n\nSegunda linha\r\nTerceira";
        let lines: Vec<_> = text.split_inclusive('\n').collect();
        assert_eq!(lines, vec!["Primeira linha\n", "\n", "Segunda linha\r\n", "Terceira"]);
    }
}
