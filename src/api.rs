use reqwest::Client;
use serde_json::Value;

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

async fn translate_one(
    client: &Client,
    line: &str,
    from_lang: &str,
    to_lang: &str,
) -> Result<String, String> {
    let encoded = urlencoding::encode(line);
    let uri = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
        from_lang, to_lang, encoded
    );

    let res = client
        .get(&uri)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await;

    match res {
        Ok(response) => {
            if response.status().as_u16() == 429 {
                return Err("IP Blocked (429)".to_string());
            }

            if let Ok(json_text) = response.text().await {
                if let Ok(v) = serde_json::from_str::<Value>(&json_text) {
                    if let Some(arr) = v.as_array() {
                        if let Some(first) = arr.get(0).and_then(|a| a.as_array()) {
                            let translated: String = first.iter()
                                .filter_map(|segment| segment.as_array()
                                    .and_then(|values| values.first())
                                    .and_then(|value| value.as_str()))
                                .collect();
                            if !translated.is_empty() {
                                return Ok(translated);
                            }
                        }
                    }
                }

                let start = json_text.find('"').unwrap_or(0) + 1;
                if start > 1 {
                    let mut end = json_text[start..].find('"').unwrap_or(0) + start;
                    while end > start && json_text.as_bytes()[end - 1] == b'\\' {
                        end = json_text[end + 1..].find('"').unwrap_or(0) + end + 1;
                    }
                    let extracted = &json_text[start..end];
                    return Ok(extracted.replace("\\n", " ").replace("\\\"", "\""));
                }
            }
            Ok(line.to_string())
        }
        Err(_) => Ok(line.to_string()),
    }
}

/// Translate one visual line at a time and restore its exact line terminator.
/// This preserves instruction lists/dialogue formatting in both Ren'Py and
/// Unity while still splitting an individual oversized line safely.
async fn translate_preserving_lines(
    client: &Client,
    text: &str,
    from_lang: &str,
    to_lang: &str,
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

        if line.trim().is_empty() {
            translated.push_str(line);
        } else {
            for piece in split_for_translation(line) {
                translated.push_str(&translate_one(client, &piece, from_lang, to_lang).await?);
            }
        }
        translated.push_str(carriage_return);
        translated.push_str(terminator);
    }
    Ok(translated)
}

pub async fn translate_batch(
    client: &Client,
    texts: &[String],
    _url: &str,
    from_lang: &str,
    to_lang: &str,
) -> Result<Vec<String>, String> {
    let mut translations = Vec::with_capacity(texts.len());

    for text in texts {
        let line = text.as_str();
        if line.trim().is_empty() {
            translations.push(String::new());
            continue;
        }

        translations.push(translate_preserving_lines(client, line, from_lang, to_lang).await?);
    }

    Ok(translations)
}

pub fn get_lang_code(name: &str) -> &'static str {
    match name {
        "Alemão" => "de",
        "Chinês (Simplificado)" => "zh-CN",
        "Coreano" => "ko",
        "Espanhol" => "es",
        "Francês" => "fr",
        "Inglês" => "en",
        "Italiano" => "it",
        "Japonês" => "ja",
        "Português" => "pt",
        "Russo" => "ru",
        _ => "auto",
    }
}

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
