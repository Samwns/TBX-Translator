// TBX Translator - font_injector.rs
// Creator: samwns
// Pure Rust Font Injector & Preview using rusttype + eframe/egui

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::thread;

use egui::{Color32, ColorImage, Context, TextureHandle, TextureOptions, Ui, Vec2};
use rusttype::{point, Font, Scale};

#[derive(Clone)]
pub struct OnlineFont {
    pub name: &'static str,
    pub style: &'static str,
    pub url: &'static str,
}

pub const CURATED_FONTS: &[OnlineFont] = &[
    OnlineFont { name: "Roboto", style: "Moderno / Limpo", url: "https://github.com/googlefonts/roboto/raw/main/src/hinted/Roboto-Regular.ttf" },
    OnlineFont { name: "Open Sans", style: "Moderno / Leitura", url: "https://github.com/googlefonts/opensans/raw/main/fonts/ttf/OpenSans-Regular.ttf" },
    OnlineFont { name: "Oswald", style: "Títulos / Impacto", url: "https://raw.githubusercontent.com/google/fonts/main/ofl/oswald/Oswald%5Bwght%5D.ttf" },
    OnlineFont { name: "Press Start 2P", style: "Pixel Art / Retro", url: "https://raw.githubusercontent.com/google/fonts/main/ofl/pressstart2p/PressStart2P-Regular.ttf" },
    OnlineFont { name: "VT323", style: "Terminal / Pixel Art", url: "https://raw.githubusercontent.com/google/fonts/main/ofl/vt323/VT323-Regular.ttf" },
    OnlineFont { name: "Cinzel", style: "Fantasia / RPG / Serif", url: "https://raw.githubusercontent.com/google/fonts/main/ofl/cinzel/Cinzel%5Bwght%5D.ttf" },
    OnlineFont { name: "Creepster", style: "Terror / Halloween", url: "https://raw.githubusercontent.com/google/fonts/main/ofl/creepster/Creepster-Regular.ttf" },
    OnlineFont { name: "Comic Neue", style: "Casual / HQ", url: "https://raw.githubusercontent.com/google/fonts/main/ofl/comicneue/ComicNeue-Regular.ttf" },
    OnlineFont { name: "Ubuntu", style: "Linux / UI", url: "https://raw.githubusercontent.com/google/fonts/main/ufl/ubuntu/Ubuntu-Regular.ttf" },
];

pub struct FontInjectorState {
    pub engine_tab: usize, // 0 = Ren'Py, 1 = Unity
    pub is_scanning: bool,
    pub renpy_fonts: Vec<String>,
    pub unity_fonts: Vec<String>,
    pub godot_fonts: Vec<String>,
    pub test_texts: HashMap<String, String>,
    pub textures: HashMap<String, (String, TextureHandle)>,
    pub unity_atlas_textures: HashMap<String, TextureHandle>,
    pub status_message: Option<(bool, String)>, // (is_error, message)
    pub show_catalog_for: Option<String>,
    pub is_downloading: bool,
    pub action_to_perform: Option<(String, PathBuf)>,
    pub catalog_search: String,
    pub previewing_font: Option<OnlineFont>,
    pub preview_text: String,
    pub downloading_for_preview: bool,
    pub downloading_font: Option<OnlineFont>,
    rx: Option<Receiver<ScanResult>>,
    rx_dl: Option<Receiver<Result<PathBuf, String>>>,
}

enum ScanResult {
    Renpy(Result<Vec<String>, String>),
    Unity(Result<Vec<String>, String>),
    Godot(Result<Vec<String>, String>),
}

impl Default for FontInjectorState {
    fn default() -> Self {
        Self {
            engine_tab: 0,
            is_scanning: false,
            renpy_fonts: Vec::new(),
            unity_fonts: Vec::new(),
            godot_fonts: Vec::new(),
            test_texts: HashMap::new(),
            textures: HashMap::new(),
            unity_atlas_textures: HashMap::new(),
            status_message: None,
            show_catalog_for: None,
            is_downloading: false,
            action_to_perform: None,
            catalog_search: String::new(),
            previewing_font: None,
            preview_text: "Teste: Áá Çç 123".to_string(),
            downloading_for_preview: false,
            downloading_font: None,
            rx: None,
            rx_dl: None,
        }
    }
}

impl FontInjectorState {
    pub fn set_engine_mode(&mut self, mode: u32) {
        self.engine_tab = if mode == 1 { 1 } else { 0 };
    }

    pub fn check_async_messages(&mut self, ctx: &Context) {
        if let Some(rx) = &self.rx {
            while let Ok(msg) = rx.try_recv() {
                self.is_scanning = false;
                match msg {
                    ScanResult::Renpy(Ok(fonts)) => {
                        self.renpy_fonts = fonts;
                        self.status_message = None;
                    }
                    ScanResult::Renpy(Err(err)) => {
                        self.status_message = Some((true, format!("Erro ao escanear fontes Ren'Py: {}", err)));
                    }
                    ScanResult::Unity(Ok(fonts)) => {
                        self.unity_fonts = fonts;
                        self.status_message = None;
                    }
                    ScanResult::Unity(Err(err)) => {
                        self.status_message = Some((true, format!("Erro ao escanear fontes Unity: {}", err)));
                    }
                    ScanResult::Godot(Ok(fonts)) => {
                        self.godot_fonts = fonts;
                        self.status_message = None;
                    }
                    ScanResult::Godot(Err(err)) => {
                        self.status_message = Some((true, format!("Erro ao escanear fontes Godot: {}", err)));
                    }
                }
                ctx.request_repaint();
            }
        }

        if let Some(rx_dl) = &self.rx_dl {
            while let Ok(res) = rx_dl.try_recv() {
                self.is_downloading = false;
                match res {
                    Ok(path) => {
                        if self.downloading_for_preview {
                            self.previewing_font = self.downloading_font.take();
                            self.status_message = None;
                        } else {
                            self.status_message = Some((false, "Fonte baixada com sucesso!".to_string()));
                            if let Some(target_font) = self.show_catalog_for.take() {
                                self.action_to_perform = Some((target_font, path));
                            }
                        }
                    }
                    Err(e) => {
                        self.status_message = Some((true, format!("Falha no download da fonte: {}", e)));
                    }
                }
                ctx.request_repaint();
            }
        }
    }

    pub fn start_download_font(&mut self, font: OnlineFont, for_preview: bool) {
        self.is_downloading = true;
        self.downloading_for_preview = for_preview;
        self.downloading_font = Some(font.clone());
        self.status_message = Some((false, format!("Baixando {}...", font.name)));
        let (tx, rx) = channel();
        self.rx_dl = Some(rx);
        
        let url = font.url.to_string();
        let name = font.name.to_string();
        
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let res = rt.block_on(async {
                let client = reqwest::Client::builder().user_agent("TBX-Translator/1.0").build().unwrap();
                let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
                if !resp.status().is_success() {
                    return Err(format!("HTTP Erro: {}", resp.status()));
                }
                let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
                
                let cache_dir = dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("tbx_translator")
                    .join("fonts");
                    
                let _ = fs::create_dir_all(&cache_dir);
                let file_path = cache_dir.join(format!("{}.ttf", name.replace(" ", "_")));
                
                fs::write(&file_path, bytes).map_err(|e| e.to_string())?;
                Ok(file_path)
            });
            let _ = tx.send(res);
        });
    }

    pub fn start_scan_renpy(&mut self, game_path: String) {
        self.is_scanning = true;
        self.status_message = None;
        let (tx, rx) = channel();
        self.rx = Some(rx);

        thread::spawn(move || {
            let res = scan_renpy_fonts(&game_path);
            let _ = tx.send(ScanResult::Renpy(res));
        });
    }

    pub fn start_scan_godot(&mut self, game_path: String) {
        self.is_scanning = true;
        self.status_message = None;
        let (tx, rx) = channel();
        self.rx = Some(rx);
        thread::spawn(move || {
            let res = scan_godot_fonts(&game_path);
            let _ = tx.send(ScanResult::Godot(res));
        });
    }

    pub fn start_scan_unity(&mut self, game_path: String) {
        self.is_scanning = true;
        self.status_message = None;
        let (tx, rx) = channel();
        self.rx = Some(rx);

        thread::spawn(move || {
            let res = scan_unity_fonts(&game_path);
            let _ = tx.send(ScanResult::Unity(res));
        });
    }

    pub fn render_ui(&mut self, ui: &mut Ui, ctx: &Context, game_path: &str, lang: &str) {
        self.check_async_messages(ctx);

        if game_path.trim().is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.colored_label(Color32::from_rgb(243, 139, 168), crate::i18n::t("erro_sem_pasta", lang));
            });
            return;
        }

        // Header and Engine Tabs
        ui.horizontal(|ui| {
            let renpy_active = self.engine_tab == 0;
            let unity_active = self.engine_tab == 1;
            let godot_active = self.engine_tab == 2;

            let renpy_btn = egui::Button::image_and_text(
                egui::Image::new(egui::include_image!("../assets/renpy_icon.svg"))
                    .max_size(egui::vec2(14.0, 14.0)),
                egui::RichText::new("Ren'Py")
                    .color(if renpy_active { Color32::from_rgb(249, 226, 175) } else { Color32::from_rgb(166, 173, 200) })
                    .strong()
            ).fill(if renpy_active { Color32::from_rgb(49, 50, 68) } else { Color32::from_rgb(17, 17, 27) });

            if ui.add(renpy_btn).clicked() {
                self.engine_tab = 0;
                self.status_message = None;
            }

            let unity_btn = egui::Button::image_and_text(
                egui::Image::new(egui::include_image!("../assets/unity_icon.svg"))
                    .max_size(egui::vec2(14.0, 14.0)),
                egui::RichText::new("Unity")
                    .color(if unity_active { Color32::from_rgb(137, 180, 250) } else { Color32::from_rgb(166, 173, 200) })
                    .strong()
            ).fill(if unity_active { Color32::from_rgb(49, 50, 68) } else { Color32::from_rgb(17, 17, 27) });

            if ui.add(unity_btn).clicked() {
                self.engine_tab = 1;
                self.status_message = None;
            }

            let godot_btn = egui::Button::image_and_text(
                egui::Image::new(egui::include_image!("../assets/godot_icon.svg"))
                    .max_size(egui::vec2(14.0, 14.0)),
                egui::RichText::new("Godot")
                    .color(if godot_active { Color32::from_rgb(166, 227, 161) } else { Color32::from_rgb(166, 173, 200) })
                    .strong()
            ).fill(if godot_active { Color32::from_rgb(49, 50, 68) } else { Color32::from_rgb(17, 17, 27) });

            if ui.add(godot_btn).clicked() {
                self.engine_tab = 2;
                self.status_message = None;
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        if let Some((is_err, msg)) = &self.status_message {
            let col = if *is_err { Color32::from_rgb(243, 139, 168) } else { Color32::from_rgb(166, 227, 161) };
            ui.horizontal(|ui| {
                let icon = if *is_err {
                    egui::include_image!("../assets/alert_icon.svg")
                } else {
                    egui::include_image!("../assets/check_icon.svg")
                };
                ui.add(egui::Image::new(icon).max_size(egui::vec2(15.0, 15.0)));
                ui.label(egui::RichText::new(msg).color(col).strong());
            });
            ui.add_space(6.0);
        }

        if self.engine_tab == 0 {
            self.render_renpy_tab(ui, ctx, game_path);
        } else if self.engine_tab == 1 {
            self.render_unity_tab(ui, ctx, game_path);
        } else if self.engine_tab == 2 {
            self.render_godot_tab(ui, ctx, game_path);
        }

        self.render_catalog_modal(ctx);
    }

    fn render_catalog_modal(&mut self, ctx: &Context) {
        let mut close = false;
        let mut do_download = None;

        if self.show_catalog_for.is_some() {
            egui::Window::new("Catálogo de Fontes (Google Fonts)")
                .id(egui::Id::new("CatalogModal"))
                .collapsible(false)
                .resizable(false)
                .default_width(450.0)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Selecione uma fonte para baixar e substituir:").strong().size(15.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let close_btn = egui::Button::new(egui::RichText::new(" X ").color(Color32::WHITE).strong())
                                .fill(Color32::from_rgb(237, 135, 150))
                                .rounding(egui::Rounding::same(4.0));
                            if ui.add(close_btn).clicked() {
                                close = true;
                            }
                        });
                    });
                    ui.separator();
                    ui.add_space(8.0);

                    if self.is_downloading {
                        ui.vertical_centered(|ui| {
                            ui.spinner();
                            ui.add_space(4.0);
                            if self.downloading_for_preview {
                                ui.label(egui::RichText::new("Baixando fonte para visualização...").color(Color32::from_rgb(137, 180, 250)).strong());
                            } else {
                                ui.label(egui::RichText::new("Baixando fonte...").color(Color32::from_rgb(137, 180, 250)).strong());
                            }
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🔍 Busca:").color(Color32::from_rgb(166, 173, 200)));
                            ui.text_edit_singleline(&mut self.catalog_search);
                        });
                        ui.add_space(8.0);
                        
                        let current_preview = self.previewing_font.clone();
                        if let Some(preview_font) = current_preview {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(format!("Visualizando: {}", preview_font.name)).color(Color32::from_rgb(249, 226, 175)).strong());
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("Fechar").clicked() {
                                            self.previewing_font = None;
                                        }
                                    });
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Texto:");
                                    ui.text_edit_singleline(&mut self.preview_text);
                                });
                                
                                let cache_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("tbx_translator").join("fonts");
                                let file_path = cache_dir.join(format!("{}.ttf", preview_font.name.replace(" ", "_")));
                                if let Ok(bytes) = std::fs::read(&file_path) {
                                    if let Some(font_obj) = rusttype::Font::try_from_vec(bytes) {
                                        if let Some(tex) = crate::font_injector::rasterize_text_preview(&font_obj, &self.preview_text, 80.0) {
                                            ui.add_space(8.0);
                                            let aspect = tex.size[0] as f32 / tex.size[1] as f32;
                                            let handle = ctx.load_texture("catalog_preview", tex, Default::default());
                                            ui.add(egui::Image::new(&handle).fit_to_exact_size(Vec2::new(350.0, 350.0 / aspect)));
                                        } else {
                                            ui.label(egui::RichText::new("Falha ao gerar visualização.").color(Color32::from_rgb(243, 139, 168)));
                                        }
                                    }
                                }
                            });
                            ui.add_space(8.0);
                        }

                        let search_query = self.catalog_search.to_lowercase();
                        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                            for font in CURATED_FONTS {
                                if !search_query.is_empty() && !font.name.to_lowercase().contains(&search_query) && !font.style.to_lowercase().contains(&search_query) {
                                    continue;
                                }
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(egui::RichText::new(font.name).strong().size(16.0).color(Color32::WHITE));
                                            ui.label(egui::RichText::new(font.style).small().color(Color32::from_rgb(166, 173, 200)));
                                        });
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            let dl_btn = egui::Button::new(egui::RichText::new("⬇ Baixar e Substituir").strong().color(Color32::from_rgb(17, 17, 27)))
                                                .fill(Color32::from_rgb(166, 227, 161));
                                            if ui.add(dl_btn).clicked() {
                                                do_download = Some((font.clone(), false));
                                            }
                                            
                                            let prev_btn = egui::Button::new(egui::RichText::new("👁 Visualizar").strong().color(Color32::from_rgb(17, 17, 27)))
                                                .fill(Color32::from_rgb(137, 180, 250));
                                            if ui.add(prev_btn).clicked() {
                                                let cache_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("tbx_translator").join("fonts");
                                                let file_path = cache_dir.join(format!("{}.ttf", font.name.replace(" ", "_")));
                                                if file_path.exists() {
                                                    self.previewing_font = Some(font.clone());
                                                } else {
                                                    do_download = Some((font.clone(), true));
                                                }
                                            }
                                        });
                                    });
                                });
                                ui.add_space(4.0);
                            }
                        });
                    }
                });
        }

        if let Some((font, for_preview)) = do_download {
            self.start_download_font(font, for_preview);
        }
        if close {
            self.show_catalog_for = None;
            self.previewing_font = None;
        }
    }

    fn render_renpy_tab(&mut self, ui: &mut Ui, ctx: &Context, game_path: &str) {
        ui.horizontal(|ui| {
            let scan_label = if self.is_scanning {
                "Escaneando fontes (aguarde)..."
            } else if self.renpy_fonts.is_empty() {
                "Escanear fontes do jogo"
            } else {
                "Escanear novamente"
            };

            let btn = egui::Button::image_and_text(
                egui::Image::new(egui::include_image!("../assets/search_icon.svg"))
                    .max_size(egui::vec2(15.0, 15.0)),
                egui::RichText::new(scan_label).color(Color32::from_rgb(17, 17, 27)).strong(),
            )
                .fill(Color32::from_rgb(166, 227, 161));

            if ui.add_enabled(!self.is_scanning, btn).clicked() {
                self.start_scan_renpy(game_path.to_string());
            }
        });

        ui.add_space(8.0);

        let fonts = self.renpy_fonts.clone();

        egui::ScrollArea::vertical().id_salt("renpy_fonts_scroll").show(ui, |ui| {
            if fonts.is_empty() && !self.is_scanning {
                ui.label(egui::RichText::new("Nenhuma fonte escaneada ainda. Clique no botão acima para listar as fontes embutidas.").color(Color32::from_rgb(166, 173, 200)));
            } else {
                let mut action_to_perform: Option<(String, PathBuf)> = None;
                let mut action_export: Option<String> = None;

                for font_path in &fonts {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(font_path).color(Color32::WHITE).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let btn = egui::Button::new(egui::RichText::new("Substituir").color(Color32::from_rgb(17, 17, 27)).strong())
                                    .fill(Color32::from_rgb(137, 180, 250));
                                if ui.add(btn).clicked() {
                                    if let Ok(Some(file)) = crate::ui::dialogs::pick_font_file("Selecione a Nova Fonte") {
                                        action_to_perform = Some((font_path.clone(), file));
                                    }
                                }

                                let btn_cat = egui::Button::new(egui::RichText::new("Catálogo").color(Color32::from_rgb(17, 17, 27)).strong())
                                    .fill(Color32::from_rgb(166, 227, 161));
                                if ui.add(btn_cat).clicked() {
                                    self.show_catalog_for = Some(font_path.clone());
                                }

                                let btn_ext = egui::Button::new(egui::RichText::new("Extrair original").color(Color32::from_rgb(17, 17, 27)).strong())
                                    .fill(Color32::from_rgb(203, 166, 247));
                                if ui.add(btn_ext).clicked() {
                                    action_export = Some(font_path.clone());
                                }
                            });
                        });

                        // Interactive font preview
                        let current_text = self.test_texts.entry(font_path.clone()).or_insert_with(|| "Prévia da fonte original: Áá Çç 123".to_string());
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Teste:").color(Color32::from_rgb(203, 166, 247)));
                            ui.text_edit_singleline(current_text);
                        });

                        // Render preview image
                        let font_file_name = Path::new(font_path).file_name().and_then(|s| s.to_str()).unwrap_or("");
                        let mut base_dir = PathBuf::from(game_path);
                        if base_dir.is_file() {
                            if let Some(p) = base_dir.parent() {
                                base_dir = p.to_path_buf();
                            }
                        }
                        let dumped_font_path = base_dir.join("game").join("tbx_temp_fonts").join(font_file_name);

                        if dumped_font_path.exists() {
                            let text_val = current_text.clone();
                            let tex = self.get_or_create_preview_texture(ctx, font_path, &dumped_font_path, &text_val);
                            if let Some(texture) = tex {
                                ui.image((texture.id(), texture.size_vec2()));
                            }
                        }
                    });
                    ui.add_space(6.0);
                }

                if let Some((target_font, new_font_file)) = action_to_perform {
                    match inject_renpy_individual(game_path, &new_font_file, &target_font) {
                        Ok(_) => {
                            self.status_message = Some((false, format!("Fonte '{}' substituída com sucesso!", target_font)));
                        }
                        Err(e) => {
                            self.status_message = Some((true, format!("Erro ao substituir fonte: {}", e)));
                        }
                    }
                }

                if let Some(target_font) = action_export {
                    if let Ok(Some(folder)) = crate::ui::dialogs::pick_folder("Selecione a pasta para salvar") {
                        let font_file_name = Path::new(&target_font).file_name().and_then(|s| s.to_str()).unwrap_or("");
                        let mut base_dir = PathBuf::from(game_path);
                        if base_dir.is_file() {
                            if let Some(p) = base_dir.parent() {
                                base_dir = p.to_path_buf();
                            }
                        }
                        let dumped_font_path = base_dir.join("game").join("tbx_temp_fonts").join(font_file_name);

                        if dumped_font_path.exists() {
                            let dest = folder.join(font_file_name);
                            match fs::copy(&dumped_font_path, &dest) {
                                Ok(_) => self.status_message = Some((false, format!("Fonte extraída para:\n{}", dest.display()))),
                                Err(e) => self.status_message = Some((true, format!("Erro ao extrair fonte: {}", e))),
                            }
                        } else {
                            self.status_message = Some((true, "A fonte temporária não foi encontrada.".to_string()));
                        }
                    }
                }
            }
        });
    }

    fn render_godot_tab(&mut self, ui: &mut Ui, _ctx: &Context, game_path: &str) {
        ui.horizontal(|ui| {
            let scan_label = if self.is_scanning {
                "Escaneando fontes (aguarde)..."
            } else if self.godot_fonts.is_empty() {
                "Escanear fontes do PCK (Godot)"
            } else {
                "Escanear novamente"
            };

            let btn = egui::Button::image_and_text(
                egui::Image::new(egui::include_image!("../assets/search_icon.svg"))
                    .max_size(egui::vec2(15.0, 15.0)),
                egui::RichText::new(scan_label).color(Color32::from_rgb(17, 17, 27)).strong(),
            )
                .fill(Color32::from_rgb(166, 227, 161));

            if ui.add_enabled(!self.is_scanning, btn).clicked() {
                self.start_scan_godot(game_path.to_string());
            }
        });

        ui.add_space(8.0);

        let fonts = self.godot_fonts.clone();

        egui::ScrollArea::vertical().id_salt("godot_fonts_scroll").show(ui, |ui| {
            if fonts.is_empty() && !self.is_scanning {
                ui.label(egui::RichText::new("Nenhuma fonte Godot encontrada. Selecione o executável/PCK e escaneie.").color(Color32::from_rgb(166, 173, 200)));
            } else {
                let mut action_to_perform: Option<(String, PathBuf)> = None;

                for font_path in &fonts {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(font_path).color(Color32::WHITE).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let btn = egui::Button::new(egui::RichText::new("Substituir").color(Color32::from_rgb(17, 17, 27)).strong())
                                    .fill(Color32::from_rgb(137, 180, 250));
                                if ui.add(btn).clicked() {
                                    if let Ok(Some(file)) = crate::ui::dialogs::pick_font_file("Selecione a Nova Fonte") {
                                        action_to_perform = Some((font_path.clone(), file));
                                    }
                                }

                                let btn_cat = egui::Button::new(egui::RichText::new("Catálogo").color(Color32::from_rgb(17, 17, 27)).strong())
                                    .fill(Color32::from_rgb(166, 227, 161));
                                if ui.add(btn_cat).clicked() {
                                    self.show_catalog_for = Some(font_path.clone());
                                }
                            });
                        });
                    });
                    ui.add_space(6.0);
                }

                if let Some((target_font, new_font_file)) = action_to_perform {
                    match inject_godot_individual(game_path, &new_font_file, &target_font) {
                        Ok(_) => {
                            self.status_message = Some((false, format!("Fonte '{}' substituída com sucesso via patch!", target_font)));
                        }
                        Err(e) => {
                            self.status_message = Some((true, format!("Erro ao substituir fonte: {}", e)));
                        }
                    }
                }
            }
        });
    }

    fn render_unity_tab(&mut self, ui: &mut Ui, ctx: &Context, game_path: &str) {
        ui.horizontal(|ui| {
            let scan_label = if self.is_scanning {
                "Escaneando fontes Unity (aguarde)..."
            } else if self.unity_fonts.is_empty() {
                "Escanear fontes do jogo (Unity)"
            } else {
                "Escanear novamente (Unity)"
            };

            let btn = egui::Button::image_and_text(
                egui::Image::new(egui::include_image!("../assets/search_icon.svg"))
                    .max_size(egui::vec2(15.0, 15.0)),
                egui::RichText::new(scan_label).color(Color32::from_rgb(17, 17, 27)).strong(),
            )
                .fill(Color32::from_rgb(137, 180, 250));

            if ui.add_enabled(!self.is_scanning, btn).clicked() {
                self.start_scan_unity(game_path.to_string());
            }
        });

        ui.add_space(8.0);
        
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Pacote Externo:").strong().color(Color32::from_rgb(166, 173, 200)));
            let btn_bundle = egui::Button::new(egui::RichText::new("Carregar AssetBundle (.unity3d)").strong().color(Color32::from_rgb(17, 17, 27)))
                .fill(Color32::from_rgb(203, 166, 247));
            if ui.add(btn_bundle).on_hover_text("Carrega um AssetBundle contendo um TMP_FontAsset.").clicked() {
                if let Ok(Some(file)) = crate::ui::dialogs::pick_assetbundle_file("Selecione o AssetBundle da Fonte") {
                    let game_dir = if Path::new(game_path).is_file() { Path::new(game_path).parent().unwrap_or(Path::new("")) } else { Path::new(game_path) };
                    let config_dir = game_dir.join("BepInEx").join("config").join("TBX_Injector");
                    let _ = fs::create_dir_all(&config_dir);
                    let dest_font = config_dir.join("custom_font_bundle");
                    match std::fs::copy(&file, &dest_font) {
                        Ok(_) => self.status_message = Some((false, "AssetBundle de fonte carregado com sucesso!".to_string())),
                        Err(e) => self.status_message = Some((true, format!("Erro ao copiar AssetBundle: {}", e))),
                    }
                }
            }
        });

        ui.add_space(8.0);

        let fonts = self.unity_fonts.clone();

        egui::ScrollArea::vertical().id_salt("unity_fonts_scroll").show(ui, |ui| {
            if fonts.is_empty() && !self.is_scanning {
                ui.label(egui::RichText::new("Nenhuma fonte Unity escaneada. Clique no botão acima para buscar fontes nos arquivos .assets.").color(Color32::from_rgb(166, 173, 200)));
            } else {
                let mut action_replace: Option<(String, PathBuf)> = None;
                let mut action_export: Option<String> = None;

                for font_id in &fonts {
                    let font_parts: Vec<&str> = font_id.splitn(4, '|').collect();
                    let is_embedded = font_parts.len() == 4 && font_parts[0] == "EMBEDDED";
                    let display_name = if font_parts.len() == 4 {
                        let kind = if is_embedded { "TTF/OTF incorporada" } else { "TextMeshPro/SDF" };
                        format!("{}  —  {} ({})", font_parts[2], font_parts[1], kind)
                    } else {
                        font_id.clone()
                    };

                    let f_path = if is_embedded {
                        format!("{}|{}|{}", font_parts[1], font_parts[2], font_parts[3])
                    } else {
                        String::new()
                    };

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&display_name).color(Color32::WHITE).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if is_embedded {
                                    let btn_rep = egui::Button::new(egui::RichText::new("Substituir").color(Color32::from_rgb(17, 17, 27)).strong())
                                        .fill(Color32::from_rgb(137, 180, 250));
                                    if ui.add(btn_rep).clicked() {
                                        if let Ok(Some(file)) = crate::ui::dialogs::pick_font_file("Selecione a Nova Fonte") {
                                            action_replace = Some((f_path.clone(), file));
                                        }
                                    }

                                    let btn_cat = egui::Button::new(egui::RichText::new("Catálogo").color(Color32::from_rgb(17, 17, 27)).strong())
                                        .fill(Color32::from_rgb(166, 227, 161));
                                    if ui.add(btn_cat).clicked() {
                                        self.show_catalog_for = Some(f_path.clone());
                                    }

                                    let btn_ext = egui::Button::new(egui::RichText::new("Extrair original").color(Color32::from_rgb(17, 17, 27)).strong())
                                        .fill(Color32::from_rgb(203, 166, 247));
                                    if ui.add(btn_ext).clicked() {
                                        action_export = Some(f_path.clone());
                                    }
                                } else {
                                    let btn_rep = egui::Button::new(egui::RichText::new("Substituir (Dinâmico)").color(Color32::from_rgb(17, 17, 27)).strong())
                                        .fill(Color32::from_rgb(249, 226, 175));
                                    if ui.add(btn_rep).on_hover_text("Define esta fonte como Fallback Global via BepInEx (Afeta todos os textos do jogo).").clicked() {
                                        if let Ok(Some(file)) = crate::ui::dialogs::pick_font_file("Selecione a Nova Fonte Global") {
                                            action_replace = Some(("GLOBAL_BEPINEX_FONT".to_string(), file));
                                        }
                                    }

                                    let btn_cat = egui::Button::new(egui::RichText::new("Catálogo").color(Color32::from_rgb(17, 17, 27)).strong())
                                        .fill(Color32::from_rgb(166, 227, 161));
                                    if ui.add(btn_cat).clicked() {
                                        self.show_catalog_for = Some("GLOBAL_BEPINEX_FONT".to_string());
                                    }
                                    ui.colored_label(Color32::from_rgb(166, 173, 200), "TMP / SDF");
                                }
                            });
                        });

                        if is_embedded {
                            // Render preview for embedded TTF
                            let current_text = self.test_texts.entry(font_id.clone()).or_insert_with(|| "Prévia Unity: Áá Çç 123".to_string());
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Teste:").color(Color32::from_rgb(203, 166, 247)));
                                ui.text_edit_singleline(current_text);
                            });

                            if let Ok(extracted_path) = export_unity_original_font(game_path, &f_path) {
                                let text_val = current_text.clone();
                                let tex = self.get_or_create_preview_texture(ctx, font_id, &extracted_path, &text_val);
                                if let Some(texture) = tex {
                                    ui.image((texture.id(), texture.size_vec2()));
                                }
                            }
                        } else if font_parts.len() == 4 {
                            // TMP Atlas preview
                            if let Some(tex) = self.get_or_create_tmp_atlas_texture(ctx, game_path, font_parts[1], font_parts[3]) {
                                ui.label(egui::RichText::new("Prévia do Atlas TMP SDF:").color(Color32::from_rgb(166, 173, 200)));
                                ui.image((tex.id(), Vec2::new(260.0, 120.0)));
                            }
                        }
                    });
                    ui.add_space(6.0);
                }

                if let Some((target_font, new_font_file)) = action_replace {
                    if target_font == "GLOBAL_BEPINEX_FONT" {
                        // Global BepInEx injection
                        let game_dir = if Path::new(game_path).is_file() { Path::new(game_path).parent().unwrap_or(Path::new("")) } else { Path::new(game_path) };
                        let config_dir = game_dir.join("BepInEx").join("config").join("TBX_Injector");
                        if let Err(e) = fs::create_dir_all(&config_dir) {
                            self.status_message = Some((true, format!("Erro ao criar diretório do BepInEx: {}", e)));
                        } else {
                            let dest_font = config_dir.join("fallback_font.ttf");
                            match fs::copy(&new_font_file, &dest_font) {
                                Ok(_) => {
                                    // Remove the custom_font_bundle if they are choosing an OS font so the OS font takes priority
                                    let _ = fs::remove_file(config_dir.join("custom_font_bundle"));
                                    
                                    // Update font_config.json in BepInEx/Translation where Plugin.cs expects it
                                    let translation_dir = game_dir.join("BepInEx").join("Translation");
                                    let _ = fs::create_dir_all(&translation_dir);
                                    let config_path = translation_dir.join("font_config.json");
                                    
                                    let font_name = new_font_file.file_stem().unwrap_or_default().to_string_lossy().to_string().replace("-Regular", "");
                                    
                                    let mut config_json = serde_json::json!({
                                        "fallbackFontName": font_name,
                                        "fontSizeMultiplier": 1.0,
                                        "fontSizeOffset": 0
                                    });
                                    if let Ok(existing_content) = fs::read_to_string(&config_path) {
                                        if let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(&existing_content) {
                                            parsed["fallbackFontName"] = serde_json::Value::String(font_name);
                                            config_json = parsed;
                                        }
                                    }
                                    let _ = fs::write(&config_path, serde_json::to_string_pretty(&config_json).unwrap_or_default());
                                    
                                    // Open the directory so the user can install the font
                                    #[cfg(target_os = "windows")]
                                    let _ = std::process::Command::new("explorer").arg(config_dir.to_str().unwrap()).spawn();
                                    
                                    self.status_message = Some((false, "Fonte configurada! IMPORTANTE: Instale o arquivo .ttf no Windows (dois cliques) para o Unity reconhecer!".to_string()));
                                }
                                Err(e) => {
                                    self.status_message = Some((true, format!("Erro ao copiar arquivo da fonte: {}", e)));
                                }
                            }
                        }
                    } else {
                        match inject_unity_individual(game_path, &new_font_file, &target_font) {
                            Ok(_) => {
                                self.status_message = Some((false, "Fonte Unity substituída com sucesso!".to_string()));
                            }
                            Err(e) => {
                                self.status_message = Some((true, format!("Erro ao substituir fonte Unity: {}", e)));
                            }
                        }
                    }
                }

                if let Some(target_font) = action_export {
                    if let Ok(Some(folder)) = crate::ui::dialogs::pick_folder("Selecione a pasta para salvar") {
                        match export_unity_original_font(game_path, &target_font) {
                            Ok(p) => {
                                if let Some(file_name) = p.file_name() {
                                    let dest = folder.join(file_name);
                                    match fs::copy(&p, &dest) {
                                        Ok(_) => self.status_message = Some((false, format!("Fonte original extraída para:\n{}", dest.display()))),
                                        Err(e) => self.status_message = Some((true, format!("Erro ao copiar fonte Unity: {}", e))),
                                    }
                                } else {
                                    self.status_message = Some((true, "Erro ao identificar o nome do arquivo da fonte Unity.".to_string()));
                                }
                            }
                            Err(e) => {
                                self.status_message = Some((true, format!("Erro ao extrair fonte Unity: {}", e)));
                            }
                        }
                    }
                }
            }
        });
    }

    pub fn get_or_create_preview_texture(
        &mut self,
        ctx: &Context,
        key: &str,
        font_file: &Path,
        text: &str,
    ) -> Option<TextureHandle> {
        if let Some((cached_text, handle)) = self.textures.get(key) {
            if cached_text == text {
                return Some(handle.clone());
            }
        }

        let font_data = fs::read(font_file).ok()?;
        let font = Font::try_from_vec(font_data)?;
        let color_image = rasterize_text_preview(&font, text, 28.0)?;
        let handle = ctx.load_texture(format!("font_preview_{}", key), color_image, TextureOptions::LINEAR);
        self.textures.insert(key.to_string(), (text.to_string(), handle.clone()));
        Some(handle)
    }

    fn get_or_create_tmp_atlas_texture(
        &mut self,
        ctx: &Context,
        game_path: &str,
        asset_path: &str,
        path_id: &str,
    ) -> Option<TextureHandle> {
        let key = format!("{}_{}", asset_path, path_id);
        if let Some(handle) = self.unity_atlas_textures.get(&key) {
            return Some(handle.clone());
        }

        let png_path = export_tmp_atlas_preview(game_path, asset_path, path_id).ok()?;
        let img = image::open(&png_path).ok()?.to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        let pixels = img.into_raw();
        let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
        let handle = ctx.load_texture(format!("tmp_atlas_{}", key), color_image, TextureOptions::LINEAR);
        self.unity_atlas_textures.insert(key, handle.clone());
        Some(handle)
    }
}

fn rasterize_text_preview(font: &Font, text: &str, font_size: f32) -> Option<ColorImage> {
    if text.is_empty() {
        return None;
    }

    let scale = Scale::uniform(font_size);
    let v_metrics = font.v_metrics(scale);
    let glyphs: Vec<_> = font.layout(text, scale, point(0.0, v_metrics.ascent)).collect();
    let width = glyphs.iter().map(|g| g.position().x + g.unpositioned().h_metrics().advance_width).last().unwrap_or(0.0).ceil().max(1.0) as usize;
    let height = (v_metrics.ascent - v_metrics.descent).ceil().max(1.0) as usize;

    let mut img_data = vec![0u8; width * height * 4];
    for g in glyphs {
        if let Some(bb) = g.pixel_bounding_box() {
            g.draw(|x, y, v| {
                let px = x as i32 + bb.min.x;
                let py = y as i32 + bb.min.y;
                if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                    let idx = (py as usize * width + px as usize) * 4;
                    let alpha = (v * 255.0) as u8;
                    img_data[idx] = 255;
                    img_data[idx + 1] = 255;
                    img_data[idx + 2] = 255;
                    img_data[idx + 3] = img_data[idx + 3].max(alpha);
                }
            });
        }
    }

    Some(ColorImage::from_rgba_unmultiplied([width, height], &img_data))
}

fn scan_unpacked_renpy_fonts(game_dir: &Path) -> Vec<String> {
    let output_dir = game_dir.join("tbx_temp_fonts");
    let _ = fs::create_dir_all(&output_dir);
    let mut fonts = Vec::new();

    for entry in walkdir::WalkDir::new(game_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if path.starts_with(&output_dir) {
            continue;
        }
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !matches!(extension.as_str(), "ttf" | "otf" | "woff" | "woff2") {
            continue;
        }
        let Ok(relative) = path.strip_prefix(game_dir) else { continue };
        let internal_path = relative.to_string_lossy().replace('\\', "/");
        if let Some(file_name) = path.file_name() {
            let _ = fs::copy(path, output_dir.join(file_name));
        }
        fonts.push(internal_path);
    }

    fonts.sort();
    fonts.dedup();
    fonts
}

#[cfg(test)]
mod renpy_font_scan_tests {
    use super::scan_unpacked_renpy_fonts;

    #[test]
    fn finds_unpacked_fonts_without_starting_renpy() {
        let root = std::env::temp_dir().join(format!(
            "tbx-font-scan-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let fonts_dir = root.join("fonts");
        std::fs::create_dir_all(&fonts_dir).unwrap();
        std::fs::write(fonts_dir.join("GameFont.ttf"), b"font-test").unwrap();

        let result = scan_unpacked_renpy_fonts(&root);

        assert_eq!(result, vec!["fonts/GameFont.ttf"]);
        assert!(root.join("tbx_temp_fonts/GameFont.ttf").is_file());
        let _ = std::fs::remove_dir_all(root);
    }
}

pub fn scan_renpy_fonts(game_path_str: &str) -> Result<Vec<String>, String> {
    let original_path = PathBuf::from(game_path_str);
    let mut base_dir = original_path.clone();
    let mut executable = original_path.clone();

    if base_dir.is_file() {
        if let Some(p) = base_dir.parent() {
            base_dir = p.to_path_buf();
        }
    } else {
        let name = base_dir.file_name().unwrap_or_default().to_string_lossy().to_string();
        if cfg!(target_os = "windows") {
            executable = base_dir.join(format!("{}.exe", name));
        } else {
            executable = base_dir.join(name);
        }
    }

    if !executable.exists() {
        if let Ok(entries) = fs::read_dir(&base_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let path = entry.path();
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if cfg!(target_os = "windows") && ext == "exe" {
                            executable = path;
                            break;
                        } else if !cfg!(target_os = "windows") && (ext == "sh" || ext == "x86_64") {
                            executable = path;
                            break;
                        }
                    }
                }
            }
        }
    }

    if !executable.exists() {
        return Err("Executável do Ren'Py não encontrado. Selecione o executável do jogo (.exe / .sh) em vez da pasta.".to_string());
    }

    let game_dir = base_dir.join("game");
    if !game_dir.exists() {
        return Err("Pasta 'game' não encontrada.".to_string());
    }
    // Most Ren'Py projects keep fonts unpacked under game/fonts. Reading them
    // directly is immediate and works even when an older game cannot finish
    // booting. The runtime scan below remains as a complement for RPA assets.
    let mut unpacked_fonts = scan_unpacked_renpy_fonts(&game_dir);
    // Remove only temporary artifacts left by versions that still used the
    // old TPG prefix. Backups are intentionally preserved.
    let _ = fs::remove_dir_all(game_dir.join("tpg_temp_fonts"));
    for legacy_name in &[
        "tpg_fonts.json",
        "tpg_fonts.json.done",
        "tpg_font_dumper.rpy",
        "tpg_font_dumper.rpyc",
    ] {
        let _ = fs::remove_file(game_dir.join(legacy_name));
    }

    if !unpacked_fonts.is_empty() {
        return Ok(unpacked_fonts);
    }

    let dumper_script = r#"
init 999 python:
    import json
    import os
    import sys
    import io
    fonts = []

    font_dir = os.path.join(renpy.config.basedir, "game", "tbx_temp_fonts")
    try:
        os.makedirs(font_dir)
    except Exception:
        pass

    for f in renpy.list_files():
        fl = f.lower()
        if fl.endswith((".ttf", ".otf", ".woff", ".woff2")):
            fonts.append(f)
            try:
                content = renpy.file(f).read()
                out_path = os.path.join(font_dir, os.path.basename(f))
                with open(out_path, "wb") as out_f:
                    out_f.write(content)
            except:
                pass
    try:
        with io.open(renpy.config.basedir + "/game/tbx_fonts.json", "w", encoding="utf-8") as out:
            json.dump(fonts, out, ensure_ascii=False, indent=4)
        with open(renpy.config.basedir + "/game/tbx_fonts.json.done", "w") as f:
            f.write("1")
    except Exception as e:      pass
    renpy.quit()
"#;

    let dumper_path = game_dir.join("tbx_font_dumper.rpy");
    fs::write(&dumper_path, dumper_script).map_err(|e| format!("Falha ao escrever dumper: {}", e))?;

    let json_path = game_dir.join("tbx_fonts.json");
    let _ = fs::remove_file(&json_path);

    let mut proc = crate::renpy_extractor::spawn_renpy_hidden(&executable.to_string_lossy())
        .map_err(|e| format!("Falha ao iniciar Ren'Py: {}", e))?;

    let mut wait_count = 0;
    while !game_dir.join("tbx_fonts.json.done").exists() {
        if wait_count > 30 { // 15 seconds
            let _ = proc.kill();
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
        wait_count += 1;
    }

    let done_exists = game_dir.join("tbx_fonts.json.done").exists();

    let _ = proc.kill(); // Ensure it closes

    let _ = fs::remove_file(&dumper_path);
    let _ = fs::remove_file(game_dir.join("tbx_font_dumper.rpyc"));
    if done_exists {
        let _ = fs::remove_file(game_dir.join("tbx_fonts.json.done"));
    } else {
        if !unpacked_fonts.is_empty() {
            return Ok(unpacked_fonts);
        }
        let runtime_log = base_dir.join("tbx_renpy.log");
        let detail = fs::read_to_string(runtime_log)
            .ok()
            .and_then(|log| log.lines().rev().find(|line| !line.trim().is_empty()).map(str::to_owned))
            .unwrap_or_else(|| "sem detalhes no log do Ren'Py".to_string());
        return Err(format!(
            "O jogo não concluiu a varredura de fontes. Detalhe: {detail}"
        ));
    }

    if !json_path.exists() {
        return Err("Arquivo JSON não gerado pelo motor.".to_string());
    }

    let content = fs::read_to_string(&json_path).map_err(|e| format!("Erro ao ler JSON: {}", e))?;
    let _ = fs::remove_file(&json_path);

    let runtime_fonts: Vec<String> = serde_json::from_str(&content).map_err(|e| format!("JSON inválido: {}", e))?;
    unpacked_fonts.extend(runtime_fonts);
    unpacked_fonts.sort();
    unpacked_fonts.dedup();
    Ok(unpacked_fonts)
}

pub fn inject_renpy_individual(game_path_str: &str, user_font_path: &Path, target_internal_path: &str) -> Result<(), String> {
    let mut base_dir = PathBuf::from(game_path_str);
    if base_dir.is_file() {
        if let Some(p) = base_dir.parent() {
            base_dir = p.to_path_buf();
        }
    }
    let game_dir = base_dir.join("game");
    if !game_dir.exists() {
        return Err("Pasta 'game' não encontrada.".to_string());
    }

    let target_full_path = game_dir.join(target_internal_path);
    if let Some(parent) = target_full_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Falha ao criar diretório: {}", e))?;
    }

    if target_full_path.exists() {
        let backup_path = game_dir.join(format!("{}.tbx_backup", target_internal_path));
        if !backup_path.exists() {
            let _ = fs::copy(&target_full_path, &backup_path);
        }
    }

    fs::copy(user_font_path, &target_full_path).map_err(|e| format!("Falha ao copiar fonte: {}", e))?;
    Ok(())
}

pub fn scan_unity_fonts(game_path_str: &str) -> Result<Vec<String>, String> {
    let mut base_dir = PathBuf::from(game_path_str);
    if base_dir.is_file() {
        if let Some(p) = base_dir.parent() {
            base_dir = p.to_path_buf();
        }
    }

    let mut command = crate::unity_extractor::get_unity_extractor_command()?;
    let out = command
        .arg("font-scan")
        .arg(&base_dir.to_string_lossy().to_string())
        .output()
        .map_err(|e| format!("Falha ao chamar extrator Unity: {}", e))?;

    if !out.status.success() {
        return Err(format!("Extrator UABEA falhou: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let txt = String::from_utf8_lossy(&out.stdout);
    let mut fonts = Vec::new();
    for line in txt.lines() {
        if line.starts_with("[FONT_SCAN] ") {
            let parts: Vec<&str> = line["[FONT_SCAN] ".len()..].splitn(4, '|').collect();
            if parts.len() == 4 && matches!(parts[0], "EMBEDDED" | "TMP") {
                fonts.push(format!("{}|{}|{}|{}", parts[0], parts[1], parts[2], parts[3]));
            }
        }
    }

    Ok(fonts)
}

pub fn inject_unity_individual(game_path_str: &str, user_font_path: &Path, target_internal_path: &str) -> Result<(), String> {
    let parts: Vec<&str> = target_internal_path.splitn(3, '|').collect();
    if parts.len() != 3 {
        return Err("Formato de fonte Unity inválido.".to_string());
    }
    let asset_file = parts[0];
    let font_name = parts[1];
    let path_id = parts[2];
    let font_locator = format!("{}|{}", asset_file, path_id);

    let mut base_dir = PathBuf::from(game_path_str);
    if base_dir.is_file() {
        if let Some(p) = base_dir.parent() {
            base_dir = p.to_path_buf();
        }
    }

    let mut command = crate::unity_extractor::get_unity_extractor_command()?;
    let out = command
        .arg("font-inject")
        .arg(&base_dir.to_string_lossy().to_string())
        .arg(font_locator)
        .arg(font_name)
        .arg(&user_font_path.to_string_lossy().to_string())
        .output()
        .map_err(|e| format!("Falha ao chamar extrator Unity: {}", e))?;

    let txt = String::from_utf8_lossy(&out.stdout);
    if txt.contains("[SUCCESS]") {
        Ok(())
    } else {
        Err(format!("Falha na injeção C#:\n{}", txt))
    }
}

pub fn export_unity_original_font(game_path_str: &str, target_internal_path: &str) -> Result<PathBuf, String> {
    let parts: Vec<&str> = target_internal_path.splitn(3, '|').collect();
    if parts.len() != 3 {
        return Err("Formato de fonte Unity inválido.".to_string());
    }
    let mut base_dir = PathBuf::from(game_path_str);
    if base_dir.is_file() {
        base_dir = base_dir.parent().ok_or("Pasta do jogo inválida.")?.to_path_buf();
    }
    let output_dir = base_dir.join("tbx_temp_fonts");
    let locator = format!("{}|{}", parts[0], parts[2]);
    let mut command = crate::unity_extractor::get_unity_extractor_command()?;
    let out = command
        .arg("font-export")
        .arg(&base_dir)
        .arg(locator)
        .arg(&output_dir)
        .output()
        .map_err(|e| format!("Falha ao chamar UABEA: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() || !text.contains("[SUCCESS]") {
        return Err(format!("Falha ao extrair fonte com UABEA:\n{}", text));
    }
    text.lines()
        .find_map(|line| line.strip_prefix("[SUCCESS] "))
        .map(PathBuf::from)
        .ok_or("UABEA não retornou o arquivo extraído.".to_string())
}

pub fn export_tmp_atlas_preview(game_path_str: &str, asset_path: &str, path_id: &str) -> Result<PathBuf, String> {
    let mut base_dir = PathBuf::from(game_path_str);
    if base_dir.is_file() {
        base_dir = base_dir.parent().ok_or("Pasta do jogo inválida.")?.to_path_buf();
    }
    let output_dir = base_dir.join("tbx_temp_fonts");
    let mut command = crate::unity_extractor::get_unity_extractor_command()?;
    let output = command
        .arg("tmp-atlas-export")
        .arg(&base_dir)
        .arg(asset_path)
        .arg(path_id)
        .arg(&output_dir)
        .output()
        .map_err(|e| format!("Falha ao chamar UABEA: {e}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() || !text.contains("[SUCCESS]") {
        return Err(text.trim().to_string());
    }
    text.lines()
        .find_map(|line| line.strip_prefix("[SUCCESS] "))
        .map(PathBuf::from)
        .ok_or("UABEA não retornou a prévia do atlas.".to_string())
        .and_then(|ppm_path| {
            let png_path = ppm_path.with_extension("png");
            image::open(&ppm_path)
                .map_err(|e| format!("Falha ao abrir atlas temporário: {e}"))?
                .save(&png_path)
                .map_err(|e| format!("Falha ao converter atlas para PNG: {e}"))?;
            let _ = fs::remove_file(ppm_path);
            Ok(png_path)
        })
}


pub fn scan_godot_fonts(game_path_str: &str) -> Result<Vec<String>, String> {
    let pck_path = crate::godot_extractor::locate_pck(Path::new(game_path_str))?;
    let mut file = fs::File::open(&pck_path).map_err(|e| format!("Erro ao abrir arquivo PCK ({}): {}", pck_path.display(), e))?;
    let archive = crate::godot_pck::read_pck_header(&mut file).map_err(|e| format!("Erro no PCK: {}", e))?;

    let mut fonts = Vec::new();
    for entry in archive.files {
        let path = entry.path.to_lowercase();
        if path.ends_with(".ttf") || path.ends_with(".otf") {
            fonts.push(entry.path.clone());
        }
    }

    Ok(fonts)
}

pub fn inject_godot_individual(game_path_str: &str, user_font_path: &Path, target_internal_path: &str) -> Result<(), String> {
    let target_path = PathBuf::from(game_path_str);
    let pck_path = crate::godot_extractor::locate_pck(&target_path)?;
    let pck_name = pck_path.file_stem().and_then(|s| s.to_str()).unwrap_or("game");
    let patch_pck = pck_path.with_file_name(format!("{}_TBX_Font_Patch.pck", pck_name));

    let font_data = fs::read(user_font_path).map_err(|e| format!("Falha ao ler nova fonte: {}", e))?;

    let mut files_to_add = HashMap::new();
    files_to_add.insert(target_internal_path.to_string(), font_data);

    crate::godot_pck::create_patch_pck(&patch_pck, &files_to_add).map_err(|e| format!("Falha ao gerar PCK patch de fonte: {}", e))?;

    Ok(())
}
