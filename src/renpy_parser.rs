use std::collections::HashSet;
use regex::Regex;

pub fn parse_dump_content(dump_content: &str) -> Vec<(String, String, String)> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    // Expressões regulares para filtrar ruídos que o Python pode ter jogado no dump
    let re_bad = Regex::new(r"(?i)^(gui/|#|v\d+\.\d+|http|www\.|\.png|\.jpg|\.ogg|\.mp3|\.wav)").unwrap();

    for line in dump_content.lines() {
        let parts: Vec<&str> = line.splitn(3, "|||").collect();
        let (file, kind, raw_text) = if parts.len() == 3 {
            (parts[0].to_string(), parts[1].to_string(), parts[2])
        } else if parts.len() == 2 {
            (parts[0].to_string(), "dialogo".to_string(), parts[1])
        } else {
            continue;
        };

        // Normalizar escapes vindos do Python
        let text = raw_text.replace("\\n", "\n").replace("\\\"", "\"").replace("\\'", "'");
        let trimmed = text.trim();
        let file_lower = file.to_lowercase();

        // Skip interfaces bundled for authoring/debugging rather than players.
        // Their labels (warper, spline, keyframes, etc.) were previously mixed
        // with game dialogue when a project shipped development tools.
        let developer_tool_file = [
            "action_editor",
            "actioneditor",
            "warper",
            "spline_editor",
            "developer",
        ]
        .iter()
        .any(|marker| file_lower.contains(marker));
        if kind == "interface" && developer_tool_file {
            continue;
        }

        if trimmed.is_empty() {
            continue;
        }

        // Filtros avançados para evitar traduzir variáveis de sistema
        if trimmed.len() <= 1 {
            continue; // Evitar traduzir letras solitárias como "A", "B"
        }

        // Ignorar textos puramente matemáticos, pontuações ou cores hex
        if trimmed.chars().all(|c| c.is_numeric() || c.is_ascii_punctuation() || c.is_whitespace() || c == '#') {
            continue;
        }

        // Ignorar caminhos de arquivos ou links
        if re_bad.is_match(trimmed) {
            continue;
        }

        // Heurística para variáveis internas (ex: chupard1, my_variable)
        // Se a string NÃO tem espaços, MAS tem números ou underscores no meio das letras,
        // é quase certeza que é um ID/Label do motor e não um texto legível.
        if !trimmed.contains(' ') {
            let has_letters = trimmed.chars().any(|c| c.is_alphabetic());
            let has_numbers_or_underscore = trimmed.chars().any(|c| c.is_ascii_digit() || c == '_');

            // Exceções: se for só número (já pego antes), ou se for algo normal.
            if has_letters && has_numbers_or_underscore {
                continue;
            }
        }

        // Ignorar textos que são APENAS formatação renpy (ex: "{b}{/b}")
        let text_no_tags = Regex::new(r"\{.*?\}").unwrap().replace_all(trimmed, "");
        if text_no_tags.trim().is_empty() {
            continue;
        }

        // Adicionar apenas únicos por arquivo para otimizar envio à API
        let key = format!("{}|{}", file, trimmed);
        if !seen.contains(&key) {
            seen.insert(key);
            candidates.push((text, file, kind));
        }
    }

    candidates
}
