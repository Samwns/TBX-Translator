use reqwest::Client;
use serde_json::Value;

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
                    // Quick and dirty JSON parsing to extract the translation string
                    if let Ok(v) = serde_json::from_str::<Value>(&json_text) {
                        if let Some(arr) = v.as_array() {
                            if let Some(first) = arr.get(0).and_then(|a| a.as_array()) {
                                if let Some(trans) = first.get(0).and_then(|a| a.as_array()) {
                                    if let Some(t) = trans.get(0).and_then(|s| s.as_str()) {
                                        translations.push(t.to_string());
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                    
                    // Fallback manual parsing if serde extraction failed like in Java:
                    let start = json_text.find('"').unwrap_or(0) + 1;
                    if start > 1 {
                        let mut end = json_text[start..].find('"').unwrap_or(0) + start;
                        while json_text.as_bytes()[end - 1] == b'\\' {
                            end = json_text[end + 1..].find('"').unwrap_or(0) + end + 1;
                        }
                        let extracted = &json_text[start..end];
                        // Unescape manually (naive)
                        let unescaped = extracted.replace("\\n", " ").replace("\\\"", "\"");
                        translations.push(unescaped);
                    } else {
                        translations.push(line.to_string());
                    }
                } else {
                    translations.push(line.to_string());
                }
            }
            Err(_) => {
                translations.push(line.to_string());
            }
        }
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
