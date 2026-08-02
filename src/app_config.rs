// TPG Translator - app_config.rs
// Creator: samwns
// Persistent configuration saved to ~/.tpg-translator/config.properties

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub caminho_jogo_renpy: String,
    pub caminho_jogo_unity: String,
    pub pasta_traducao: String,
    pub idioma_origem: String,
    pub idioma_alvo: String,
    pub motor_api: String,
    pub modo_jogo: String,          // "renpy" or "unity"
    pub usar_multi_trad: bool,
    pub manter_estrutura_original: bool,
    pub preservar_nomes_renpy: bool,
    pub traduzir_nomes_personagens_renpy: bool,
    pub threads_ativas: u32,
    pub ui_language: String, // "pt_BR" or "en_US"
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            caminho_jogo_renpy: String::new(),
            caminho_jogo_unity: String::new(),
            pasta_traducao: "portuguese".into(),
            idioma_origem: "Detectar Automaticamente".into(),
            idioma_alvo: "Português".into(),
            motor_api: "Google Translator".into(),
            modo_jogo: "renpy".into(),
            usar_multi_trad: false,
            manter_estrutura_original: true,
            preservar_nomes_renpy: true,
            traduzir_nomes_personagens_renpy: false,
            threads_ativas: 5,
            ui_language: "pt_BR".into(),
        }
    }
}

impl AppConfig {
    fn config_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".tpg-translator").join("config.properties")
    }

    pub fn config_path_str() -> String {
        Self::config_path().to_string_lossy().to_string()
    }

    pub fn carregar() -> Self {
        let path = Self::config_path();
        let mut cfg = AppConfig::default();

        let Ok(file) = fs::File::open(&path) else {
            return cfg;
        };

        let mut props: HashMap<String, String> = HashMap::new();
        for line in BufReader::new(file).lines().flatten() {
            let line = line.trim().to_string();
            if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                props.insert(k.trim().to_string(), v.trim().to_string());
            }
        }

        if let Some(v) = props.get("caminhoJogoRenpy")                 { cfg.caminho_jogo_renpy = v.clone(); }
        if let Some(v) = props.get("caminhoJogoUnity")                 { cfg.caminho_jogo_unity = v.clone(); }
        if let Some(v) = props.get("pastaTraducao")                    { cfg.pasta_traducao = v.clone(); }
        if let Some(v) = props.get("idiomaOrigem")                     { cfg.idioma_origem = v.clone(); }
        if let Some(v) = props.get("idiomaAlvo")                       { cfg.idioma_alvo = v.clone(); }
        if let Some(v) = props.get("motorApi")                         { cfg.motor_api = v.clone(); }
        if let Some(v) = props.get("modoJogo")                         { cfg.modo_jogo = v.clone(); }
        if let Some(v) = props.get("usarMultiTrad")                    { cfg.usar_multi_trad = v == "true"; }
        if let Some(v) = props.get("manterEstruturaOriginal")          { cfg.manter_estrutura_original = v == "true"; }
        if let Some(v) = props.get("preservarNomesRenpy")              { cfg.preservar_nomes_renpy = v == "true"; }
        if let Some(v) = props.get("traduzirNomesPersonagensRenpy")    { cfg.traduzir_nomes_personagens_renpy = v == "true"; }
        if let Some(v) = props.get("threadsAtivas")                    { cfg.threads_ativas = v.parse().unwrap_or(5); }
        if let Some(v) = props.get("uiLanguage")                       { cfg.ui_language = v.clone(); }

        cfg
    }

    pub fn salvar(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let Ok(mut file) = fs::File::create(&path) else { return; };

        let _ = writeln!(file, "# TPG Translator - Config");
        let _ = writeln!(file, "# Creator: samwns");
        let _ = writeln!(file, "caminhoJogoRenpy={}", self.caminho_jogo_renpy);
        let _ = writeln!(file, "caminhoJogoUnity={}", self.caminho_jogo_unity);
        let _ = writeln!(file, "pastaTraducao={}", self.pasta_traducao);
        let _ = writeln!(file, "idiomaOrigem={}", self.idioma_origem);
        let _ = writeln!(file, "idiomaAlvo={}", self.idioma_alvo);
        let _ = writeln!(file, "motorApi={}", self.motor_api);
        let _ = writeln!(file, "modoJogo={}", self.modo_jogo);
        let _ = writeln!(file, "usarMultiTrad={}", self.usar_multi_trad);
        let _ = writeln!(file, "manterEstruturaOriginal={}", self.manter_estrutura_original);
        let _ = writeln!(file, "preservarNomesRenpy={}", self.preservar_nomes_renpy);
        let _ = writeln!(file, "traduzirNomesPersonagensRenpy={}", self.traduzir_nomes_personagens_renpy);
        let _ = writeln!(file, "threadsAtivas={}", self.threads_ativas);
        let _ = writeln!(file, "uiLanguage={}", self.ui_language);
    }
}
