use std::collections::HashSet;
use regex::Regex;

/// Candidato extraído do dump do jogo.
#[derive(Debug, Clone)]
pub struct RenpyCandidate {
    pub text: String,
    pub file: String,
    pub kind: String,
    /// Identificador nativo do RenPy (`s.identifier`) quando originado de um
    /// nó AST. Permite gerar `translate <lang> <id>:` em vez de old/new, o
    /// que funciona de forma robusta mesmo quando o script estava dentro de
    /// um .rpa e melhora a filtragem (chave estável independente do texto).
    pub identifier: Option<String>,
}

pub fn parse_dump_content(dump_content: &str) -> Vec<RenpyCandidate> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    // Expressões regulares para filtrar ruídos que o Python pode ter jogado no dump
    let re_bad = Regex::new(r"(?i)^(gui/|#|v\d+\.\d+|http|www\.|\.png|\.jpg|\.ogg|\.mp3|\.wav)").unwrap();

    for line in dump_content.lines() {
        // Formato antigo (3 campos): arquivo|||tipo|||texto
        // Formato novo (4 campos): arquivo|||tipo|||id|||texto
        let parts: Vec<&str> = line.splitn(4, "|||").collect();
        let (file, kind, ident, raw_text) = match parts.len() {
            4 => (parts[0].to_string(), parts[1].to_string(), Some(parts[2].to_string()), parts[3]),
            3 => {
                // Repartir em 3 a partir da linha inteira para preservar texto com "|||"
                let p3: Vec<&str> = line.splitn(3, "|||").collect();
                (p3[0].to_string(), p3[1].to_string(), None, p3[2])
            }
            _ => continue,
        };

        // Normalizar escapes vindos do Python e o placeholder do separador
        let text = raw_text
            .replace("{{pipe3}}", "|||")
            .replace("\\n", "\n")
            .replace("\\\"", "\"")
            .replace("\\'", "'");
        let trimmed = text.trim();
        let file_lower = file.to_lowercase();

        // Skip interfaces bundled for authoring/debugging rather than players.
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
            continue;
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
        if !trimmed.contains(' ') {
            let has_letters = trimmed.chars().any(|c| c.is_alphabetic());
            let has_numbers_or_underscore = trimmed.chars().any(|c| c.is_ascii_digit() || c == '_');
            if has_letters && has_numbers_or_underscore {
                continue;
            }
        }

        // Ignorar textos que são APENAS formatação renpy (ex: "{b}{/b}")
        let text_no_tags = Regex::new(r"\{.*?\}").unwrap().replace_all(trimmed, "");
        if text_no_tags.trim().is_empty() {
            continue;
        }

        // Chave de dedup: usa o ID quando existir (mesmo texto = mesma linha
        // de diálogo), senão arquivo+texto.
        let key = match &ident {
            Some(id) if !id.is_empty() => format!("id:{}", id),
            _ => format!("{}|{}", file, trimmed),
        };
        if !seen.contains(&key) {
            seen.insert(key);
            candidates.push(RenpyCandidate {
                text,
                file,
                kind,
                identifier: ident.filter(|s| !s.is_empty()),
            });
        }
    }

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_old_and_new_format() {
        let dump = "script.rpy|||dialogo|||Hello world!\nscript.rpy|||dialogo|||abc123def456|||Olá mundo\n";
        let out = parse_dump_content(dump);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].identifier, None);
        assert_eq!(out[0].text, "Hello world!");
        assert_eq!(out[1].identifier.as_deref(), Some("abc123def456"));
        assert_eq!(out[1].text, "Olá mundo");
    }

    #[test]
    fn dedups_by_identifier() {
        let dump = "a.rpy|||dialogo|||id_x|||Texto\nb.rpy|||dialogo|||id_x|||Texto\n";
        let out = parse_dump_content(dump);
        assert_eq!(out.len(), 1);
    }
}
