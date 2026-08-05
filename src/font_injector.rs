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

pub struct FontInjectorState {
    pub engine_tab: usize, // 0 = Ren'Py, 1 = Unity
    pub is_scanning: bool,
    pub renpy_fonts: Vec<String>,
    pub unity_fonts: Vec<String>,
    pub test_texts: HashMap<String, String>,
    pub textures: HashMap<String, (String, TextureHandle)>,
    pub unity_atlas_textures: HashMap<String, TextureHandle>,
    pub status_message: Option<(bool, String)>, // (is_error, message)
    rx: Option<Receiver<ScanResult>>,
}

enum ScanResult {
    Renpy(Result<Vec<String>, String>),
    Unity(Result<Vec<String>, String>),
}

impl Default for FontInjectorState {
    fn default() -> Self {
        Self {
            engine_tab: 0,
            is_scanning: false,
            renpy_fonts: Vec::new(),
            unity_fonts: Vec::new(),
            test_texts: HashMap::new(),
            textures: HashMap::new(),
            unity_atlas_textures: HashMap::new(),
            status_message: None,
            rx: None,
        }
    }
}

impl FontInjectorState {
    pub fn new() -> Self {
        Self::default()
    }

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
                }
                ctx.request_repaint();
            }
        }
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

            let renpy_btn = egui::Button::new(
                egui::RichText::new("Ren'Py")
                    .color(if renpy_active { Color32::from_rgb(249, 226, 175) } else { Color32::from_rgb(166, 173, 200) })
                    .strong()
            ).fill(if renpy_active { Color32::from_rgb(49, 50, 68) } else { Color32::from_rgb(17, 17, 27) });

            if ui.add(renpy_btn).clicked() {
                self.engine_tab = 0;
                self.status_message = None;
            }

            let unity_btn = egui::Button::new(
                egui::RichText::new("Unity")
                    .color(if unity_active { Color32::from_rgb(137, 180, 250) } else { Color32::from_rgb(166, 173, 200) })
                    .strong()
            ).fill(if unity_active { Color32::from_rgb(49, 50, 68) } else { Color32::from_rgb(17, 17, 27) });

            if ui.add(unity_btn).clicked() {
                self.engine_tab = 1;
                self.status_message = None;
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        if let Some((is_err, msg)) = &self.status_message {
            let col = if *is_err { Color32::from_rgb(243, 139, 168) } else { Color32::from_rgb(166, 227, 161) };
            ui.label(egui::RichText::new(msg).color(col).strong());
            ui.add_space(6.0);
        }

        if self.engine_tab == 0 {
            self.render_renpy_tab(ui, ctx, game_path);
        } else {
            self.render_unity_tab(ui, ctx, game_path);
        }
    }

    fn render_renpy_tab(&mut self, ui: &mut Ui, ctx: &Context, game_path: &str) {
        ui.horizontal(|ui| {
            let scan_label = if self.is_scanning {
                "⏳ Escaneando fontes (aguarde)..."
            } else if self.renpy_fonts.is_empty() {
                "🔍 Escanear Fontes do Jogo"
            } else {
                "🔄 Escanear Novamente"
            };

            let btn = egui::Button::new(egui::RichText::new(scan_label).color(Color32::from_rgb(17, 17, 27)).strong())
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

                for font_path in &fonts {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(font_path).color(Color32::WHITE).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let btn = egui::Button::new(egui::RichText::new("Substituir").color(Color32::from_rgb(17, 17, 27)).strong())
                                    .fill(Color32::from_rgb(137, 180, 250));
                                if ui.add(btn).clicked() {
                                    if let Some(file) = rfd::FileDialog::new()
                                        .set_title("Selecione a Nova Fonte")
                                        .add_filter("Fontes (*.ttf, *.otf)", &["ttf", "otf"])
                                        .pick_file()
                                    {
                                        action_to_perform = Some((font_path.clone(), file));
                                    }
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
                        let dumped_font_path = base_dir.join("game").join("tpg_temp_fonts").join(font_file_name);

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
                            self.status_message = Some((false, format!("✅ Fonte '{}' substituída com sucesso!", target_font)));
                        }
                        Err(e) => {
                            self.status_message = Some((true, format!("❌ Erro ao substituir fonte: {}", e)));
                        }
                    }
                }
            }
        });
    }

    fn render_unity_tab(&mut self, ui: &mut Ui, ctx: &Context, game_path: &str) {
        ui.horizontal(|ui| {
            let scan_label = if self.is_scanning {
                "⏳ Escaneando fontes Unity (aguarde)..."
            } else if self.unity_fonts.is_empty() {
                "🔍 Escanear Fontes do Jogo (Unity)"
            } else {
                "🔄 Escanear Novamente (Unity)"
            };

            let btn = egui::Button::new(egui::RichText::new(scan_label).color(Color32::from_rgb(17, 17, 27)).strong())
                .fill(Color32::from_rgb(137, 180, 250));

            if ui.add_enabled(!self.is_scanning, btn).clicked() {
                self.start_scan_unity(game_path.to_string());
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
                                        .fill(Color32::from_rgb(166, 227, 161));
                                    if ui.add(btn_rep).clicked() {
                                        if let Some(file) = rfd::FileDialog::new()
                                            .set_title("Selecione a Nova Fonte")
                                            .add_filter("Fontes (*.ttf, *.otf)", &["ttf", "otf"])
                                            .pick_file()
                                        {
                                            action_replace = Some((f_path.clone(), file));
                                        }
                                    }

                                    let btn_ext = egui::Button::new(egui::RichText::new("Extrair original").color(Color32::from_rgb(17, 17, 27)).strong())
                                        .fill(Color32::from_rgb(203, 166, 247));
                                    if ui.add(btn_ext).clicked() {
                                        action_export = Some(f_path.clone());
                                    }
                                } else {
                                    ui.colored_label(Color32::from_rgb(166, 173, 200), "TMP / SDF (Atlas)");
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
                    match inject_unity_individual(game_path, &new_font_file, &target_font) {
                        Ok(_) => {
                            self.status_message = Some((false, "✅ Fonte Unity substituída com sucesso!".to_string()));
                        }
                        Err(e) => {
                            self.status_message = Some((true, format!("❌ Erro ao substituir fonte Unity: {}", e)));
                        }
                    }
                }

                if let Some(target_font) = action_export {
                    match export_unity_original_font(game_path, &target_font) {
                        Ok(p) => {
                            self.status_message = Some((false, format!("✅ Fonte original extraída em:\n{}", p.display())));
                        }
                        Err(e) => {
                            self.status_message = Some((true, format!("❌ Erro ao extrair fonte Unity: {}", e)));
                        }
                    }
                }
            }
        });
    }

    fn get_or_create_preview_texture(
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
        let color_image = rasterize_text_preview(&font, text)?;
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

fn rasterize_text_preview(font: &Font, text: &str) -> Option<ColorImage> {
    if text.is_empty() {
        return None;
    }

    let scale = Scale::uniform(28.0);
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

    let dumper_script = r#"
init 999 python:
    import json
    import os
    import sys
    fonts = []
    
    font_dir = os.path.join(renpy.config.basedir, "game", "tpg_temp_fonts")
    os.makedirs(font_dir, exist_ok=True)
    
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
        with open(renpy.config.basedir + "/game/tpg_fonts.json", "w", encoding="utf-8") as out:
            json.dump(fonts, out, ensure_ascii=False, indent=4)
    except:
        pass
    renpy.quit()
"#;

    let dumper_path = game_dir.join("tpg_font_dumper.rpy");
    fs::write(&dumper_path, dumper_script).map_err(|e| format!("Falha ao escrever dumper: {}", e))?;

    let json_path = game_dir.join("tpg_fonts.json");
    let _ = fs::remove_file(&json_path);

    let mut proc = crate::renpy_extractor::spawn_renpy_hidden(&executable.to_string_lossy())
        .map_err(|e| format!("Falha ao iniciar Ren'Py: {}", e))?;

    let _ = proc.wait();

    let _ = fs::remove_file(&dumper_path);
    let _ = fs::remove_file(game_dir.join("tpg_font_dumper.rpyc"));

    if !json_path.exists() {
        return Err("Arquivo JSON não gerado pelo motor.".to_string());
    }

    let content = fs::read_to_string(&json_path).map_err(|e| format!("Erro ao ler JSON: {}", e))?;
    let _ = fs::remove_file(&json_path);

    let fonts: Vec<String> = serde_json::from_str(&content).map_err(|e| format!("JSON inválido: {}", e))?;
    Ok(fonts)
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
        let backup_path = game_dir.join(format!("{}.tpg_backup", target_internal_path));
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

    let script_path = crate::paths::app_root().join("unity_static_extractor");
    let packaged = script_path.join(if cfg!(windows) { "unity_static_extractor.exe" } else { "unity_static_extractor" });
    let mut command = if packaged.is_file() {
        crate::paths::hidden_command(packaged)
    } else {
        let mut command = crate::paths::hidden_command("dotnet");
        command.arg("run").arg("--");
        command
    };
    let out = command
        .arg("font-scan")
        .arg(&base_dir.to_string_lossy().to_string())
        .current_dir(&script_path)
        .output()
        .map_err(|e| format!("Falha ao chamar C#: {}", e))?;

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

    let script_path = crate::paths::app_root().join("unity_static_extractor");
    let packaged = script_path.join(if cfg!(windows) { "unity_static_extractor.exe" } else { "unity_static_extractor" });
    let mut command = if packaged.is_file() {
        crate::paths::hidden_command(packaged)
    } else {
        let mut command = crate::paths::hidden_command("dotnet");
        command.arg("run").arg("--");
        command
    };
    let out = command
        .arg("font-inject")
        .arg(&base_dir.to_string_lossy().to_string())
        .arg(font_locator)
        .arg(font_name)
        .arg(&user_font_path.to_string_lossy().to_string())
        .current_dir(&script_path)
        .output()
        .map_err(|e| format!("Falha ao chamar C#: {}", e))?;

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
    let output_dir = base_dir.join("tpg_temp_fonts");
    let locator = format!("{}|{}", parts[0], parts[2]);
    let script_path = crate::paths::app_root().join("unity_static_extractor");
    let packaged = script_path.join(if cfg!(windows) { "unity_static_extractor.exe" } else { "unity_static_extractor" });
    let mut command = if packaged.is_file() {
        crate::paths::hidden_command(packaged)
    } else {
        let mut command = crate::paths::hidden_command("dotnet");
        command.arg("run").arg("--");
        command
    };
    let out = command
        .arg("font-export")
        .arg(&base_dir)
        .arg(locator)
        .arg(&output_dir)
        .current_dir(&script_path)
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
    let output_dir = base_dir.join("tpg_temp_fonts");
    let script_path = crate::paths::app_root().join("unity_static_extractor");
    let packaged = script_path.join(if cfg!(windows) { "unity_static_extractor.exe" } else { "unity_static_extractor" });
    let mut command = if packaged.is_file() {
        crate::paths::hidden_command(packaged)
    } else {
        let mut command = crate::paths::hidden_command("dotnet");
        command.arg("run").arg("--");
        command
    };
    let output = command
        .arg("tmp-atlas-export")
        .arg(&base_dir)
        .arg(asset_path)
        .arg(path_id)
        .arg(&output_dir)
        .current_dir(&script_path)
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
