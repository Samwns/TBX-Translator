use crate::ui::{TbxApp, AppTab};
use crate::i18n::t;
use egui::*;
use std::path::Path;

impl TbxApp {
    pub fn render_translate_tab(&mut self, ui: &mut Ui, _ctx: &Context) {
        let lang = &self.config.ui_language.clone();

        // 1. Engine Selector Pill Tabs
        ui.horizontal(|ui| {
            ui.label(RichText::new("Motor:").color(Color32::from_rgb(166, 173, 200)).strong());

            let renpy_active = self.engine_mode == 0;
            let unity_active = self.engine_mode == 1;

            let renpy_btn = Button::image_and_text(
                egui::Image::new(egui::include_image!("../../../assets/renpy_icon.svg")).max_height(14.0),
                RichText::new("Ren'Py Engine")
                    .color(if renpy_active { Color32::from_rgb(17, 17, 27) } else { Color32::from_rgb(205, 214, 244) })
                    .strong(),
            )
            .fill(if renpy_active { Color32::from_rgb(249, 226, 175) } else { Color32::from_rgb(49, 50, 68) })
            .rounding(Rounding::same(6.0));

            if ui.add(renpy_btn).clicked() {
                self.engine_mode = 0;
                self.config.modo_jogo = "renpy".into();
                self.game_path = self.config.caminho_jogo_renpy.clone();
                self.detect_game_type();
            }

            ui.add_space(8.0); // Espaçamento adicionado aqui!

            let unity_btn = Button::image_and_text(
                egui::Image::new(egui::include_image!("../../../assets/unity_icon.svg")).max_height(14.0),
                RichText::new("Unity Engine")
                    .color(if unity_active { Color32::from_rgb(17, 17, 27) } else { Color32::from_rgb(205, 214, 244) })
                    .strong(),
            )
            .fill(if unity_active { Color32::from_rgb(137, 180, 250) } else { Color32::from_rgb(49, 50, 68) })
            .rounding(Rounding::same(6.0));

            if ui.add(unity_btn).clicked() {
                self.engine_mode = 1;
                self.config.modo_jogo = "unity".into();
                self.game_path = self.config.caminho_jogo_unity.clone();
                self.detect_game_type();
            }
        });

        ui.add_space(10.0);

        // 2. Game Path Input + Browse Button
        ui.group(|ui| {
            ui.label(RichText::new("Executável / Pasta do Jogo:").color(Color32::from_rgb(205, 214, 244)).strong());
            ui.horizontal(|ui| {
                let edit = TextEdit::singleline(&mut self.game_path)
                    .hint_text(t("selecione_pasta", lang))
                    .desired_width(ui.available_width() - 110.0);
                let edit_resp = ui.add(edit);
                if edit_resp.changed() {
                    if self.engine_mode == 0 {
                        self.config.caminho_jogo_renpy = self.game_path.clone();
                    } else {
                        self.config.caminho_jogo_unity = self.game_path.clone();
                    }
                    self.detect_game_type();
                }

                let browse_btn = Button::new(RichText::new("Procurar...").color(Color32::WHITE).strong())
                    .fill(Color32::from_rgb(69, 71, 90));

                if ui.add(browse_btn).clicked() {
                    let mut dialog = rfd::FileDialog::new().set_title("Selecione o Executável do Jogo");
                    if cfg!(target_os = "windows") {
                        dialog = dialog.add_filter("Executáveis (*.exe)", &["exe"]);
                    } else {
                        dialog = dialog.add_filter("Executáveis (*.sh, *.x86_64, executável)", &["sh", "x86_64", "bin", ""]);
                    }

                    if let Some(file) = dialog.pick_file() {
                        self.game_path = file.to_string_lossy().to_string();
                        if self.engine_mode == 0 {
                            self.config.caminho_jogo_renpy = self.game_path.clone();
                        } else {
                            self.config.caminho_jogo_unity = self.game_path.clone();
                        }
                        self.detect_game_type();
                    }
                }
            });

            // Live game type badge
            if let Some(status) = &self.detected_game_type {
                ui.add_space(4.0);
                let col = if status.contains('✓') {
                    Color32::from_rgb(166, 227, 161)
                } else {
                    Color32::from_rgb(243, 139, 168)
                };
                ui.label(RichText::new(status).color(col).small());
            }
        });

        ui.add_space(10.0);

        // 3. Language Selection & Translation Folder Card
        ui.group(|ui| {
            ui.columns(3, |cols| {
                // Col 1: Idioma Origem
                cols[0].label(RichText::new(t("idioma_orig", lang)).color(Color32::from_rgb(166, 173, 200)).small());
                egui::ComboBox::from_id_salt("combo_source_lang")
                    .selected_text(&self.selected_source_lang)
                    .width(cols[0].available_width())
                    .show_ui(&mut cols[0], |ui| {
                        for lang_name in &self.source_languages {
                            ui.selectable_value(&mut self.selected_source_lang, lang_name.to_string(), *lang_name);
                        }
                    });

                // Col 2: Idioma Alvo
                cols[1].label(RichText::new(t("idioma_alvo", lang)).color(Color32::from_rgb(166, 173, 200)).small());
                egui::ComboBox::from_id_salt("combo_target_lang")
                    .selected_text(&self.selected_target_lang)
                    .width(cols[1].available_width())
                    .show_ui(&mut cols[1], |ui| {
                        for lang_name in &self.target_languages {
                            ui.selectable_value(&mut self.selected_target_lang, lang_name.to_string(), *lang_name);
                        }
                    });

                // Col 3: Pasta de Tradução
                cols[2].label(RichText::new(t("pasta_trad", lang)).color(Color32::from_rgb(166, 173, 200)).small());
                let edit_resp = cols[2].add(TextEdit::singleline(&mut self.config.pasta_traducao));
                if edit_resp.lost_focus() {
                    let _ = self.config.salvar();
                }
            });
        });

        ui.add_space(14.0);

        // 4. Action Buttons
        ui.horizontal(|ui| {
            if !self.is_running {
                let t_time = _ctx.input(|i| i.time);
                let pulse = (t_time * 4.0).sin() as f32 * 0.5 + 0.5; // 0.0 to 1.0

                let (btn_text, base_col) = if self.engine_mode == 0 {
                    (t("iniciar_trad_renpy", lang), [249.0, 226.0, 175.0])
                } else {
                    (t("iniciar_trad_unity", lang), [137.0, 180.0, 250.0])
                };

                let p_col = [255.0, 255.0, 255.0];
                let btn_color = Color32::from_rgb(
                    (base_col[0] * (1.0 - pulse * 0.2) + p_col[0] * pulse * 0.2) as u8,
                    (base_col[1] * (1.0 - pulse * 0.2) + p_col[1] * pulse * 0.2) as u8,
                    (base_col[2] * (1.0 - pulse * 0.2) + p_col[2] * pulse * 0.2) as u8,
                );

                let main_btn = Button::new(RichText::new(btn_text).color(Color32::from_rgb(17, 17, 27)).strong().size(14.0))
                    .fill(btn_color)
                    .min_size(vec2(220.0, 38.0))
                    .rounding(Rounding::same(6.0));

                if ui.add(main_btn).clicked() {
                    if self.check_translation_folder_exists() {
                        self.show_overwrite_modal = true;
                    } else {
                        self.start_translation(false);
                    }
                }
                _ctx.request_repaint(); // Animate continuously

                if self.engine_mode == 1 {
                    let inject_btn = Button::new(RichText::new("INJETAR TRADUÇÃO").color(Color32::from_rgb(17, 17, 27)).strong().size(14.0))
                        .fill(Color32::from_rgb(166, 227, 161))
                        .min_size(vec2(180.0, 38.0))
                        .rounding(Rounding::same(6.0));

                    if ui.add(inject_btn).clicked() {
                        self.start_unity_inject();
                    }
                }

                let editor_btn = Button::new(RichText::new(t("abrir_editor", lang)).color(Color32::WHITE).strong().size(14.0))
                    .fill(Color32::from_rgb(49, 50, 68))
                    .min_size(vec2(200.0, 38.0))
                    .rounding(Rounding::same(6.0));

                if ui.add(editor_btn).clicked() {
                    self.editor_state.load_directory(&self.game_path, &self.config.pasta_traducao, self.engine_mode == 1);
                    self.current_tab = AppTab::Editor;
                }
            }
            if self.is_running {
                // Running task indicator and cancel button
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.label(RichText::new(&self.progress_text).color(Color32::from_rgb(203, 166, 247)).strong());

                        let cancel_btn = Button::new(RichText::new("⏹ CANCELAR TRADUÇÃO").color(Color32::WHITE).strong())
                            .fill(Color32::from_rgb(243, 139, 168))
                            .rounding(Rounding::same(8.0))
                            .min_size(vec2(0.0, 40.0));
                        if ui.add(cancel_btn).clicked() {
                            self.show_cancel_modal = true;
                        }
                    });
                });
            }
        });

        // Progress bar if running or active
        if self.is_running || self.progress.1 > 0 {
            ui.add_space(10.0);
            let fraction = if self.progress.1 > 0 {
                self.progress.0 as f32 / self.progress.1 as f32
            } else {
                0.0
            };

            let bar = egui::ProgressBar::new(fraction)
                .text(format!("{:.1}%", fraction * 100.0))
                .animate(self.is_running);
            ui.add(bar);
        }

        ui.add_space(14.0);

        // 5. Quick log preview in translate tab
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("📜 Últimos Eventos:").color(Color32::from_rgb(166, 173, 200)).small().strong());
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Ver Todos os Logs →").clicked() {
                        self.current_tab = AppTab::Logs;
                    }
                });
            });

            ui.add_space(4.0);
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(140.0)
                .stick_to_bottom(true)
                .id_salt("mini_log_scroll")
                .show(ui, |ui| {
                    if let Some(tab) = self.log_tabs.get(self.active_log_tab) {
                        for line in tab.lines.iter().rev().take(15).rev() {
                            ui.label(RichText::new(line).monospace().size(11.0).color(Color32::from_rgb(166, 227, 161)));
                        }
                    }
                });
        });
    }


}
