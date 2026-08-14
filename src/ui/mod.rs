pub mod title_bar;
pub mod modals;
pub mod tabs;

// TBX Translator - ui.rs
// Creator: samwns
// Pure Rust Cross-Platform UI using eframe and egui

pub use crate::types::UiMsg;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

use egui::{
    Color32, Context, Frame, Margin, Rounding, Stroke, Visuals,
};

use crate::app_config::AppConfig;
use crate::editor_ui::EditorState;
use crate::font_injector::FontInjectorState;
use crate::i18n::t;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum AppTab {
    Translate,
    Logs,
    Tools,
    Settings,
    Editor,
    FontInjector,
}

#[derive(Clone, Debug)]
pub struct LogTab {
    pub title: String,
    pub lines: Vec<String>,
    pub closable: bool,
}

fn scoped_sender(engine: usize, global: Sender<UiMsg>) -> Sender<UiMsg> {
    let (local, receiver) = channel();
    thread::spawn(move || {
        while let Ok(message) = receiver.recv() {
            let scoped = match message {
                UiMsg::Done(text) => UiMsg::EngineDone(engine, text),
                UiMsg::Error(text) => UiMsg::EngineError(engine, text),
                UiMsg::Cancelled => UiMsg::EngineCancelled(engine),
                UiMsg::Log(text) => UiMsg::EngineLog(engine, text),
                UiMsg::Progress(done, total) => UiMsg::EngineProgress(engine, done, total),
                other => other,
            };
            if global.send(scoped).is_err() { break; }
        }
    });
    local
}

pub fn toggle_ui(ui: &mut egui::Ui, on: &mut bool, text: &str) -> egui::Response {
    let desired_size = egui::vec2(36.0, 20.0);

    ui.horizontal(|ui| {
        let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
        if response.clicked() {
            *on = !*on;
            response.mark_changed();
        }

        if ui.is_rect_visible(rect) {
            let how_on = ui.ctx().animate_bool_with_time(response.id, *on, 0.2);
            let visuals = ui.style().interact_selectable(&response, *on);

            let rect = rect.expand(visuals.expansion);
            let radius = 0.5 * rect.height();

            let bg_color = if *on {
                Color32::from_rgb(166, 227, 161) // Green for ON
            } else {
                Color32::from_rgb(88, 91, 112) // Gray for OFF
            };

            ui.painter().rect(rect, Rounding::same(radius), bg_color, Stroke::NONE);

            let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
            let center = egui::pos2(circle_x, rect.center().y);

            ui.painter().circle(center, 0.75 * radius, ui.visuals().window_fill(), Stroke::new(1.0, Color32::from_rgb(166, 173, 200)));
        }

        ui.label(text);
        response
    }).inner
}

pub struct TbxApp {
    pub config: AppConfig,
    pub current_tab: AppTab,
    pub engine_mode: u32, // 0 = Ren'Py, 1 = Unity
    pub game_path: String,
    pub detected_game_type: Option<String>,
    pub is_running: bool,
    pub running_engines: [bool; 3],
    pub engine_log_tabs: [usize; 3],
    pub engine_progress: [(usize, usize); 3],
    pub progress: (usize, usize),
    pub progress_text: String,
    pub cancelled: Arc<AtomicBool>,
    pub cancelled_engines: [Arc<AtomicBool>; 3],

    // Channels
    pub tx: Sender<UiMsg>,
    pub rx: Receiver<UiMsg>,

    // Logs
    pub log_tabs: Vec<LogTab>,
    pub active_log_tab: usize,

    // Sub-views
    pub editor_state: EditorState,
    pub font_injector_state: FontInjectorState,

    // Modals
    pub show_overwrite_modal: bool,
    pub show_engine_modal: bool,
    pub show_cancel_modal: bool,
    pub engine_modal_tab: usize, // 0 = RenPy, 1 = Unity
    pub show_alert_modal: Option<(bool, String, String)>, // (is_error, title, message)

    // Cached languages
    pub source_languages: Vec<&'static str>,
    pub target_languages: Vec<&'static str>,
    pub selected_source_lang: String,
    pub selected_target_lang: String,
    /// Locale slots already registered in the selected Godot export.
    pub godot_native_locales: Vec<String>,

    // Application updater
    pub update_release: Option<crate::updater::ReleaseInfo>,
    pub update_checking: bool,
    pub update_downloading: bool,
    pub update_status: String,
    pub update_progress: (u64, u64),
}

impl TbxApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        setup_custom_styles(&cc.egui_ctx);

        let config = AppConfig::carregar();
        let (tx, rx) = channel();

        let initial_engine = if config.modo_jogo == "unity" {
            1
        } else if config.modo_jogo == "godot" {
            2
        } else {
            0
        };
        let initial_path = match initial_engine {
            1 => config.caminho_jogo_unity.clone(),
            2 => config.caminho_jogo_godot.clone(),
            _ => config.caminho_jogo_renpy.clone(),
        };

        let mut app = Self {
            selected_source_lang: config.idioma_origem.clone(),
            selected_target_lang: config.idioma_alvo.clone(),
            config,
            current_tab: AppTab::Translate,
            engine_mode: initial_engine,
            game_path: initial_path,
            detected_game_type: None,
            is_running: false,
            running_engines: [false; 3],
            engine_log_tabs: [0; 3],
            engine_progress: [(0, 0); 3],
            progress: (0, 0),
            progress_text: String::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
            cancelled_engines: std::array::from_fn(|_| Arc::new(AtomicBool::new(false))),
            tx,
            rx,
            log_tabs: vec![LogTab {
                title: "Geral".to_string(),
                lines: vec!["[Sistema] TBX Translator iniciado com sucesso.".to_string()],
                closable: false,
            }],
            active_log_tab: 0,
            editor_state: EditorState::new(),
            font_injector_state: FontInjectorState::default(),
            show_overwrite_modal: false,
            show_engine_modal: false,
            show_cancel_modal: false,
            engine_modal_tab: 0,
            show_alert_modal: None,
            source_languages: {
                let mut langs = vec!["Detectar Automaticamente"];
                langs.extend_from_slice(crate::api::ALL_LANGUAGES);
                langs
            },
            target_languages: crate::api::ALL_LANGUAGES.to_vec(),
            godot_native_locales: Vec::new(),
            update_release: None,
            update_checking: false,
            update_downloading: false,
            update_status: String::new(),
            update_progress: (0, 0),
        };

        app.detect_game_type();
        app
    }

    pub fn detect_game_type(&mut self) {
        let path_str = self.game_path.trim();
        if path_str.is_empty() {
            self.detected_game_type = None;
            return;
        }

        let path = Path::new(path_str);
        if !path.exists() {
            self.detected_game_type = Some("Caminho não encontrado".to_string());
            return;
        }

        let parent = if path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path
        };

        if self.engine_mode == 0 {
            if parent.join("game").is_dir() {
                self.detected_game_type = Some("Jogo Ren'Py Detectado ✓".to_string());
            } else {
                self.detected_game_type = Some("Pasta 'game' do Ren'Py não encontrada".to_string());
            }
        } else if self.engine_mode == 1 {
            if let Some(backend) = crate::unity_extractor::detect_unity_backend(path_str) {
                self.detected_game_type = Some(format!("Unity Detectado ({}) ✓", backend));
            } else {
                self.detected_game_type = Some("Estrutura Unity (*_Data) não encontrada".to_string());
            }
        } else if self.engine_mode == 2 {
            self.godot_native_locales = crate::godot_extractor::detect_native_locales(path_str)
                .map(|info| info.locale_codes)
                .unwrap_or_default();
            if path_str.to_lowercase().ends_with(".pck") {
                self.detected_game_type = Some("Arquivo PCK do Godot Detectado ✓".to_string());
            } else if path.is_file() && path_str.to_lowercase().ends_with(".exe") {
                let pck_path = path.with_extension("pck");
                if pck_path.exists() {
                    self.detected_game_type = Some("Jogo Godot Detectado (via PCK adjacente) ✓".to_string());
                } else {
                    self.detected_game_type = Some("Jogo Godot Detectado (PCK possivelmente embutido no .exe) ✓".to_string());
                }
            } else {
                self.detected_game_type = Some("Arquivo PCK ou Executável não reconhecido".to_string());
            }
        }
    }

    pub fn append_log(&mut self, text: String) {
        if let Some(tab) = self.log_tabs.get_mut(self.active_log_tab) {
            tab.lines.push(text);
        }
    }

    pub fn create_task_log_tab(&mut self, title: String) {
        let existing = self.log_tabs.iter().position(|t| t.title == title);
        if let Some(idx) = existing {
            self.active_log_tab = idx;
        } else {
            self.log_tabs.push(LogTab {
                title,
                lines: Vec::new(),
                closable: true,
            });
            self.active_log_tab = self.log_tabs.len() - 1;
        }
    }

    pub fn check_incoming_messages(&mut self, ctx: &Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                UiMsg::Log(line) => {
                    self.append_log(line);
                }
                UiMsg::Progress(done, total) => {
                    self.progress = (done, total);
                    self.progress_text = format!("Traduzindo: {} / {} itens", done, total);
                }
                UiMsg::Done(summary) => {
                    self.is_running = false;
                    self.progress = (0, 0);
                    self.append_log(format!("[Concluído] {}", summary));
                    self.show_alert_modal = Some((false, "Sucesso".to_string(), summary));
                }
                UiMsg::Error(err) => {
                    self.is_running = false;
                    self.progress = (0, 0);
                    self.append_log(format!("[Erro] {}", err));
                    self.show_alert_modal = Some((true, "Erro".to_string(), err));
                }
                UiMsg::Cancelled => {
                    self.is_running = false;
                    self.progress = (0, 0);
                    self.append_log("[Aviso] Processo cancelado pelo usuário.".to_string());
                }
                UiMsg::EngineDone(engine, summary) => {
                    self.running_engines[engine] = false;
                    self.engine_progress[engine] = (0, 0);
                    self.is_running = self.running_engines.iter().any(|running| *running);
                    self.progress = (0, 0);
                    if let Some(tab) = self.log_tabs.get_mut(self.engine_log_tabs[engine]) {
                        tab.lines.push(format!("[Concluído] {}", summary));
                    }
                    self.show_alert_modal = Some((false, "Sucesso".to_string(), summary));
                }
                UiMsg::EngineError(engine, error) => {
                    self.running_engines[engine] = false;
                    self.engine_progress[engine] = (0, 0);
                    self.is_running = self.running_engines.iter().any(|running| *running);
                    self.progress = (0, 0);
                    if let Some(tab) = self.log_tabs.get_mut(self.engine_log_tabs[engine]) {
                        tab.lines.push(format!("[Erro] {}", error));
                    }
                    self.show_alert_modal = Some((true, "Erro".to_string(), error));
                }
                UiMsg::EngineCancelled(engine) => {
                    self.running_engines[engine] = false;
                    self.engine_progress[engine] = (0, 0);
                    self.is_running = self.running_engines.iter().any(|running| *running);
                    self.progress = (0, 0);
                    if let Some(tab) = self.log_tabs.get_mut(self.engine_log_tabs[engine]) {
                        tab.lines.push("[Aviso] Processo cancelado pelo usuário.".to_string());
                    }
                }
                UiMsg::EngineLog(engine, line) => {
                    if let Some(tab) = self.log_tabs.get_mut(self.engine_log_tabs[engine]) {
                        tab.lines.push(line);
                    }
                }
                UiMsg::EngineProgress(engine, done, total) => {
                    self.engine_progress[engine] = (done, total);
                    if self.engine_mode as usize == engine {
                        self.progress = (done, total);
                        self.progress_text = format!("Traduzindo: {} / {} itens", done, total);
                    }
                }
                UiMsg::DetectedLanguageMismatch(lang_code) => {
                    let lang_name = crate::api::get_lang_name(&lang_code);
                    if self.selected_source_lang != lang_name && lang_name != "Detectar Automaticamente" {
                        self.selected_source_lang = lang_name.to_string();
                        self.show_alert_modal = Some((
                            false,
                            "Idioma Detectado Automaticamente".to_string(),
                            format!("O idioma original do jogo foi detectado como {}. A seleção foi alterada automaticamente para garantir a melhor tradução.", lang_name)
                        ));
                    }
                }
                UiMsg::UpdateFound(release) => {
                    self.update_checking = false;
                    if crate::updater::is_newer(&release.tag_name) {
                        self.update_status = format!(
                            "{}: {}",
                            t("nova_versao_disponivel", &self.config.ui_language),
                            release.name
                        );
                    } else {
                        self.update_status = format!(
                            "{} ({})",
                            t("versao_mais_recente", &self.config.ui_language),
                            crate::updater::current_version()
                        );
                    }
                    self.update_release = Some(release);
                }
                UiMsg::UpdateStatus(status) => {
                    self.update_status = status;
                }
                UiMsg::UpdateProgress(done, total) => {
                    self.update_progress = (done, total);
                }
                UiMsg::UpdateError(error) => {
                    self.update_checking = false;
                    self.update_downloading = false;
                    self.update_status = error.clone();
                    self.show_alert_modal = Some((true, "Atualização".to_string(), error));
                }
            }
            ctx.request_repaint();
        }
    }

    pub fn start_translation(&mut self, overwrite: bool) {
        let exe = self.game_path.trim().to_string();
        if exe.is_empty() {
            self.show_alert_modal = Some((
                true,
                "Atenção".to_string(),
                t("erro_sem_pasta", &self.config.ui_language).to_string(),
            ));
            return;
        }

        let engine_index = self.engine_mode as usize;
        if self.running_engines[engine_index] { return; }
        self.running_engines[engine_index] = true;
        self.engine_progress[engine_index] = (0, 0);
        self.is_running = true;
        self.progress = (0, 0);
        self.progress_text = "Iniciando processamento...".to_string();
        self.cancelled = Arc::new(AtomicBool::new(false));
        self.cancelled_engines[engine_index] = self.cancelled.clone();

        let filename = Path::new(&exe)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Jogo")
            .to_string();
        self.create_task_log_tab(filename);
        self.engine_log_tabs[engine_index] = self.active_log_tab;

        let tx = scoped_sender(engine_index, self.tx.clone());
        let cancelled = self.cancelled.clone();
        let folder = self.config.pasta_traducao.clone();
        let src_lang = crate::api::get_lang_code(&self.selected_source_lang).to_string();
        let tgt_lang = crate::api::get_lang_code(&self.selected_target_lang).to_string();
        let keep_struct = self.config.manter_estrutura_original;
        let trans_names = self.config.traduzir_nomes_personagens_renpy;
        let threads = if self.config.usar_multi_trad { self.config.threads_ativas } else { 1 };
        let engine_type = self.engine_mode;
        let api_engine = self.config.motor_api.clone();

        thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(UiMsg::Log(format!("[Erro] Falha ao criar runtime Tokio: {}", e)));
                    let _ = tx.send(UiMsg::Done(format!("Erro interno: {}", e)));
                    return;
                }
            };

            rt.block_on(async {
                if engine_type == 0 {
                    let res = crate::renpy_extractor::extract_texts(
                        &exe,
                        &folder,
                        &src_lang,
                        &tgt_lang,
                        keep_struct,
                        trans_names,
                        threads,
                        &api_engine,
                        tx.clone(),
                        cancelled,
                        overwrite,
                    )
                    .await;

                    match res {
                        Ok(_) => {
                            // The extractor will send UiMsg::Cancelled or UiMsg::Done on its own.
                        }
                        Err(e) => {
                            if e.to_string().contains("Cancelado") {
                                let _ = tx.send(UiMsg::Cancelled);
                            } else {
                                let _ = tx.send(UiMsg::Log(format!("[Erro] {}", e)));
                                let _ = tx.send(UiMsg::Error(format!("Falha na tradução: {}", e)));
                            }
                        }
                    }
                } else if engine_type == 1 {
                    let res = crate::unity_extractor::extract_texts(
                        &exe,
                        &folder,
                        &src_lang,
                        &tgt_lang,
                        threads,
                        &api_engine,
                        tx.clone(),
                        cancelled,
                        overwrite,
                    )
                    .await;

                    match res {
                        Ok(_) => {
                            // The extractor will send UiMsg::Cancelled or UiMsg::Done on its own.
                        }
                        Err(e) => {
                            if e.to_string().contains("Cancelado") {
                                let _ = tx.send(UiMsg::Cancelled);
                            } else {
                                let _ = tx.send(UiMsg::Log(format!("[Erro Unity] {}", e)));
                                let _ = tx.send(UiMsg::Error(format!("Falha na extração Unity: {}", e)));
                            }
                        }
                    }
                } else if engine_type == 2 {
                    let res = crate::godot_extractor::extract_texts(
                        &exe,
                        &folder,
                        &src_lang,
                        &tgt_lang,
                        threads,
                        &api_engine,
                        tx.clone(),
                        cancelled,
                        overwrite,
                    )
                    .await;

                    match res {
                        Ok(_) => {
                            // The extractor will send UiMsg::Cancelled or UiMsg::Done on its own.
                        }
                        Err(e) => {
                            if e.to_string().contains("Cancelado") {
                                let _ = tx.send(UiMsg::Cancelled);
                            } else {
                                let _ = tx.send(UiMsg::Log(format!("[Erro Godot] {}", e)));
                                let _ = tx.send(UiMsg::Error(format!("Falha na extração Godot: {}", e)));
                            }
                        }
                    }
                }
            });
        });
    }

    pub fn start_unity_inject(&mut self) {
        let exe = self.game_path.trim().to_string();
        if exe.is_empty() {
            self.show_alert_modal = Some((
                true,
                "Atenção".to_string(),
                t("erro_sem_pasta", &self.config.ui_language).to_string(),
            ));
            return;
        }

        let engine_index = 1usize;
        if self.running_engines[engine_index] { return; }
        self.running_engines[engine_index] = true;
        self.engine_progress[engine_index] = (0, 0);
        self.is_running = true;
        self.progress = (0, 0);
        self.progress_text = "Injetando tradução Unity...".to_string();

        let filename = format!(
            "{} (Injeção)",
            Path::new(&exe).file_name().and_then(|s| s.to_str()).unwrap_or("Jogo")
        );
        self.create_task_log_tab(filename);
        self.engine_log_tabs[engine_index] = self.active_log_tab;

        let tx = scoped_sender(engine_index, self.tx.clone());
        let folder = self.config.pasta_traducao.clone();
        let tgt_lang = self.selected_target_lang.clone();

        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let res = crate::unity_extractor::inject_texts(&exe, &folder, &tgt_lang, tx.clone()).await;
                match res {
                    Ok(_) => {
                        let _ = tx.send(UiMsg::Done("Injeção Unity (XUnity AutoTranslator) realizada com sucesso!".into()));
                    }
                    Err(e) => {
                        let _ = tx.send(UiMsg::Log(format!("[Erro] {}", e)));
                        let _ = tx.send(UiMsg::Error(format!("Falha na injeção: {}", e)));
                    }
                }
            });
        });
    }

    pub fn start_godot_inject(&mut self) {
        let exe = self.game_path.trim().to_string();
        if exe.is_empty() {
            self.show_alert_modal = Some((
                true,
                "Atenção".to_string(),
                t("erro_sem_pasta", &self.config.ui_language).to_string(),
            ));
            return;
        }

        let engine_index = 2usize;
        if self.running_engines[engine_index] { return; }
        self.running_engines[engine_index] = true;
        self.engine_progress[engine_index] = (0, 0);
        self.is_running = true;
        self.progress = (0, 0);
        self.progress_text = "Injetando tradução Godot...".to_string();

        let filename = format!(
            "{} (Injeção)",
            Path::new(&exe).file_name().and_then(|s| s.to_str()).unwrap_or("Jogo")
        );
        self.create_task_log_tab(filename);
        self.engine_log_tabs[engine_index] = self.active_log_tab;

        let tx = scoped_sender(engine_index, self.tx.clone());
        let folder = self.config.pasta_traducao.clone();
        let src_lang = self.selected_source_lang.clone();
        let tgt_lang = self.selected_target_lang.clone();
        let strategy = crate::godot_extractor::InjectionStrategy::from_config(&self.config.godot_injection_mode);
        let locale = self.config.godot_force_locale.clone();

        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let res = crate::godot_extractor::inject_translation(&exe, &folder, &src_lang, &tgt_lang, strategy, &locale, tx.clone()).await;
                match res {
                    Ok(_) => {
                        // Success message is sent by inject_translation
                    }
                    Err(e) => {
                        let _ = tx.send(UiMsg::Log(format!("[Erro Godot] {}", e)));
                        let _ = tx.send(UiMsg::Error(format!("Falha na injeção Godot: {}", e)));
                    }
                }
            });
        });
    }

    pub fn cancel_current_task(&mut self) {
        self.cancelled_engines[self.engine_mode as usize].store(true, Ordering::SeqCst);
        self.append_log("[Aviso] Cancelamento solicitado pelo usuário...".to_string());
    }

    #[allow(dead_code)]
    pub fn handle_messages(&mut self, ctx: &Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                crate::types::UiMsg::Log(text) => {
                    self.append_log(text);
                }
                crate::types::UiMsg::Progress(current, total) => {
                    self.progress = (current, total);
                    if total > 0 {
                        self.progress_text = format!("Processando... {} / {}", current, total);
                    }
                }
                crate::types::UiMsg::Done(text) => {
                    self.is_running = false;
                    self.progress_text = text.clone();
                    self.append_log(format!("[Concluído] {}", text));

                    self.show_alert_modal = Some((
                        false,
                        "Sucesso".to_string(),
                        text,
                    ));
                }
                crate::types::UiMsg::Error(text) => {
                    self.is_running = false;
                    self.progress_text = "Erro na operação".to_string();
                    self.append_log(format!("[Erro] {}", text));

                    self.show_alert_modal = Some((
                        true,
                        "Erro".to_string(),
                        text,
                    ));
                }
                crate::types::UiMsg::DetectedLanguageMismatch(lang) => {
                    self.append_log(format!("[Aviso] Idioma fonte auto-detectado como {}", lang));
                }
                crate::types::UiMsg::Cancelled => {
                    self.is_running = false;
                    self.progress_text = "Operação Cancelada".to_string();
                }
                crate::types::UiMsg::EngineDone(engine, text) => {
                    self.running_engines[engine] = false;
                    self.is_running = self.running_engines.iter().any(|running| *running);
                    self.progress_text = text;
                }
                crate::types::UiMsg::EngineError(engine, text) => {
                    self.running_engines[engine] = false;
                    self.is_running = self.running_engines.iter().any(|running| *running);
                    self.progress_text = text;
                }
                crate::types::UiMsg::EngineCancelled(engine) => {
                    self.running_engines[engine] = false;
                    self.is_running = self.running_engines.iter().any(|running| *running);
                    self.progress_text = "Operação Cancelada".to_string();
                }
                crate::types::UiMsg::EngineLog(engine, text) => {
                    if let Some(tab) = self.log_tabs.get_mut(self.engine_log_tabs[engine]) {
                        tab.lines.push(text);
                    }
                }
                crate::types::UiMsg::EngineProgress(engine, current, total) => {
                    self.engine_progress[engine] = (current, total);
                }
                crate::types::UiMsg::UpdateFound(release) => {
                    self.update_checking = false;
                    self.update_release = Some(release);
                }
                crate::types::UiMsg::UpdateStatus(status) => {
                    self.update_status = status;
                }
                crate::types::UiMsg::UpdateProgress(current, total) => {
                    self.update_progress = (current, total);
                }
                crate::types::UiMsg::UpdateError(error) => {
                    self.update_checking = false;
                    self.update_downloading = false;
                    self.update_status = error;
                }
            }
            ctx.request_repaint();
        }
    }
    pub fn check_translation_folder_exists(&self) -> bool {
        let base_path = PathBuf::from(&self.game_path);
        let parent_dir = if base_path.is_file() {
            base_path.parent().unwrap_or(&base_path).to_path_buf()
        } else {
            base_path
        };

        if self.engine_mode == 0 {
            let safe_folder = crate::renpy_extractor::language_identifier(&self.config.pasta_traducao);
            parent_dir.join("game").join("tl").join(safe_folder).exists()
        } else if self.engine_mode == 1 {
            let safe_name = if self.config.pasta_traducao.trim().is_empty() {
                "portuguese"
            } else {
                self.config.pasta_traducao.trim()
            };
            parent_dir.join(format!("TBX_Workspace_{}", safe_name)).exists()
        } else {
            let safe_name = if self.config.pasta_traducao.trim().is_empty() {
                "portuguese"
            } else {
                self.config.pasta_traducao.trim()
            };
            parent_dir.join(format!("TBX_Workspace_Godot_{}", safe_name)).exists()
        }
    }
}

impl eframe::App for TbxApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.check_incoming_messages(ctx);

        // Custom frameless window root container
        egui::CentralPanel::default()
            .frame(
                Frame::none()
                    .fill(Color32::from_rgb(30, 30, 46))
                    .inner_margin(Margin::same(0.0)),
            )
            .show(ctx, |ui| {
                // Wrapper frame for the actual app content
                Frame::none()
                    .fill(Color32::from_rgb(30, 30, 46)) // #1e1e2e
                    .rounding(0.0)
                    .show(ui, |ui| {
                        self.render_custom_title_bar(ui, ctx);
                        self.render_top_navigation_bar(ui, ctx);

                        // Main body area
                        Frame::none()
                            .fill(Color32::from_rgb(30, 30, 46))
                            .inner_margin(Margin::same(16.0))
                            .rounding(0.0)
                            .show(ui, |ui| {
                                ui.set_min_size(ui.available_size());
                                match self.current_tab {
                        AppTab::Translate => self.render_translate_tab(ui, ctx),
                        AppTab::Logs => self.render_logs_tab(ui),
                        AppTab::Tools => self.render_tools_tab(ui, ctx),
                        AppTab::Settings => self.render_settings_tab(ui),
                        AppTab::Editor => self.render_editor_view(ui, ctx),
                        AppTab::FontInjector => self.render_font_injector_view(ui, ctx),
                    }
                    });

                        self.render_modals(ctx);
                    });
            });
    }
}

fn setup_custom_styles(ctx: &Context) {
    let mut fonts = egui::FontDefinitions::default();
    // Support for CJK (Japanese / Shift-JIS) characters if the font is available in assets
    if let Ok(font_bytes) = std::fs::read("assets/NotoSansJP.ttf") {
        fonts.font_data.insert("cjk".to_owned(), egui::FontData::from_owned(font_bytes));
        if let Some(prop) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            prop.push("cjk".to_owned());
        }
        if let Some(mono) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            mono.push("cjk".to_owned());
        }
    }
    ctx.set_fonts(fonts);

    let mut visuals = Visuals::dark();

    // Catppuccin Mocha Palette
    visuals.override_text_color = Some(Color32::from_rgb(205, 214, 244)); // #cdd6f4
    visuals.panel_fill = Color32::from_rgb(30, 30, 46); // #1e1e2e
    visuals.window_fill = Color32::from_rgb(24, 24, 37); // #181825
    visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(69, 71, 90)); // #45475a
    visuals.window_rounding = Rounding::same(8.0);

    // Widget styling
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(24, 24, 37);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(49, 50, 68));
    visuals.widgets.noninteractive.rounding = Rounding::same(6.0);

    visuals.widgets.inactive.bg_fill = Color32::from_rgb(49, 50, 68);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(69, 71, 90));
    visuals.widgets.inactive.rounding = Rounding::same(6.0);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(69, 71, 90);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(137, 180, 250));
    visuals.widgets.hovered.rounding = Rounding::same(6.0);

    visuals.widgets.active.bg_fill = Color32::from_rgb(88, 91, 112);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, Color32::from_rgb(203, 166, 247));
    visuals.widgets.active.rounding = Rounding::same(6.0);

    visuals.selection.bg_fill = Color32::from_rgb(69, 71, 90);
    visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(137, 180, 250));

    ctx.set_visuals(visuals);
}

pub fn run_app(icon_data: Option<std::sync::Arc<egui::IconData>>) -> Result<(), eframe::Error> {
    let mut builder = egui::ViewportBuilder::default()
        .with_title("TBX Translator")
        .with_inner_size([880.0, 620.0])
        .with_min_inner_size([700.0, 500.0])
        .with_decorations(false) // Frameless with custom dark title bar
        .with_transparent(true);

    if let Some(icon) = icon_data {
        builder = builder.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport: builder,
        ..Default::default()
    };

    eframe::run_native(
        "TBX Translator",
        options,
        Box::new(|cc| Ok(Box::new(TbxApp::new(cc)))),
    )
}
