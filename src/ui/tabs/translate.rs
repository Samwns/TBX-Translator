use crate::ui::{TbxApp, AppTab, dialogs};
use crate::i18n::t;
use egui::*;

impl TbxApp {
    pub fn render_translate_tab(&mut self, ui: &mut Ui, ctx: &Context) {
        let lang = &self.config.ui_language.clone();

        // 1. Engine selector: independent floating pills, detached from the top tabs.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;

            let renpy_active = self.engine_mode == 0;
            let unity_active = self.engine_mode == 1;
            let godot_active = self.engine_mode == 2;
            let engine_selector_id = ui.id();

            let idle = ui.visuals().widgets.inactive.bg_fill;
            let animated_fill = |id: &str, active: bool, selected: Color32| {
                let amount = ctx.animate_bool_with_time(engine_selector_id.with(id), active, 0.18);
                Color32::from_rgb(
                    egui::lerp(idle.r() as f32..=selected.r() as f32, amount) as u8,
                    egui::lerp(idle.g() as f32..=selected.g() as f32, amount) as u8,
                    egui::lerp(idle.b() as f32..=selected.b() as f32, amount) as u8,
                )
            };

            let renpy_btn = Button::image_and_text(
                egui::Image::new(egui::include_image!("../../../assets/renpy_icon.svg")).max_height(18.0),
                RichText::new("Ren'Py")
                    .color(if renpy_active { ui.visuals().window_fill } else { ui.visuals().text_color() })
                    .strong(),
            )
            .fill(animated_fill("renpy_engine_pill", renpy_active, ui.visuals().selection.bg_fill))
            .rounding(Rounding { nw: 6.0, sw: 6.0, ne: 0.0, se: 0.0 })
            .min_size(vec2(132.0, 40.0));

            if ui.add(renpy_btn).clicked() {
                self.engine_mode = 0;
                self.config.modo_jogo = "renpy".into();
                self.game_path = self.config.caminho_jogo_renpy.clone();
                self.detect_game_type();
            }

            let unity_btn = Button::image_and_text(
                egui::Image::new(egui::include_image!("../../../assets/unity_icon.svg")).max_height(18.0),
                RichText::new("Unity")
                    .color(if unity_active { ui.visuals().window_fill } else { ui.visuals().text_color() })
                    .strong(),
            )
            .fill(animated_fill("unity_engine_pill", unity_active, ui.visuals().selection.bg_fill))
            .rounding(Rounding::ZERO)
            .min_size(vec2(132.0, 40.0));

            if ui.add(unity_btn).clicked() {
                self.engine_mode = 1;
                self.config.modo_jogo = "unity".into();
                self.game_path = self.config.caminho_jogo_unity.clone();
                self.detect_game_type();
            }
            let godot_btn = Button::image_and_text(
                egui::Image::new(egui::include_image!("../../../assets/godot_icon.svg")).max_height(18.0),
                RichText::new("Godot")
                    .color(if godot_active { ui.visuals().window_fill } else { ui.visuals().text_color() })
                    .strong(),
            )
            .fill(animated_fill("godot_engine_pill", godot_active, ui.visuals().selection.bg_fill))
            .rounding(Rounding { nw: 0.0, sw: 0.0, ne: 6.0, se: 6.0 })
            .min_size(vec2(132.0, 40.0));

            if ui.add(godot_btn).clicked() {
                self.engine_mode = 2;
                self.config.modo_jogo = "godot".into();
                self.game_path = self.config.caminho_jogo_godot.clone();
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
                    } else if self.engine_mode == 1 {
                        self.config.caminho_jogo_unity = self.game_path.clone();
                    } else {
                        self.config.caminho_jogo_godot = self.game_path.clone();
                    }
                    self.detect_game_type();
                }
                if edit_resp.lost_focus() {
                    let _ = self.config.salvar();
                }

                let browse_btn = Button::new(RichText::new("Procurar...").color(Color32::WHITE).strong())
                    .fill(Color32::from_rgb(69, 71, 90));

                if ui.add(browse_btn).clicked() {
                    match dialogs::pick_game_file() {
                        Ok(Some(file)) => {
                            self.game_path = file.to_string_lossy().to_string();
                            if self.engine_mode == 0 {
                                self.config.caminho_jogo_renpy = self.game_path.clone();
                            } else if self.engine_mode == 1 {
                                self.config.caminho_jogo_unity = self.game_path.clone();
                            } else {
                                self.config.caminho_jogo_godot = self.game_path.clone();
                            }
                            self.detect_game_type();
                            self.config.salvar();
                        }
                        Ok(None) => {}
                        Err(error) => self.show_alert_modal = Some((true, "Falha ao abrir seletor".into(), error)),
                    }
                }
            });

            // Live game type badge
            if let Some(status) = &self.detected_game_type {
                ui.add_space(4.0);
                let detected = status.to_lowercase().contains("detectado");
                let col = if detected {
                    Color32::from_rgb(166, 227, 161)
                } else {
                    Color32::from_rgb(243, 139, 168)
                };
                ui.horizontal(|ui| {
                    if detected {
                        ui.add(
                            egui::Image::new(egui::include_image!(
                                "../../../assets/check_icon.svg"
                            ))
                            .max_size(vec2(13.0, 13.0)),
                        );
                    }
                    ui.label(RichText::new(status).color(col).small());
                });
            }
        });

        ui.add_space(10.0);

        // 3. Language Selection & Translation Folder Card
        ui.group(|ui| {
            ui.columns(3, |cols| {
                // Col 1: Idioma Origem
                cols[0].label(RichText::new(t("idioma_orig", lang)).color(Color32::from_rgb(166, 173, 200)).small());
                let mut source_changed = false;
                egui::ComboBox::from_id_salt("combo_source_lang")
                    .selected_text(&self.selected_source_lang)
                    .width(cols[0].available_width())
                    .show_ui(&mut cols[0], |ui| {
                        for lang_name in &self.source_languages {
                            if ui.selectable_value(&mut self.selected_source_lang, lang_name.to_string(), *lang_name).changed() {
                                source_changed = true;
                            }
                        }
                    });
                if source_changed {
                    self.config.idioma_origem = self.selected_source_lang.clone();
                    let _ = self.config.salvar();
                }

                // Col 2: Idioma Alvo
                cols[1].label(RichText::new(t("idioma_alvo", lang)).color(Color32::from_rgb(166, 173, 200)).small());
                let mut target_changed = false;
                egui::ComboBox::from_id_salt("combo_target_lang")
                    .selected_text(&self.selected_target_lang)
                    .width(cols[1].available_width())
                    .show_ui(&mut cols[1], |ui| {
                        for lang_name in &self.target_languages {
                            if ui.selectable_value(&mut self.selected_target_lang, lang_name.to_string(), *lang_name).changed() {
                                target_changed = true;
                            }
                        }
                    });
                if target_changed {
                    self.config.idioma_alvo = self.selected_target_lang.clone();
                    self.config.pasta_traducao = crate::renpy_extractor::language_identifier(&self.selected_target_lang);
                    let _ = self.config.salvar();
                }

                // Col 3: Pasta de Tradução
                let example_folder = crate::renpy_extractor::language_identifier(&self.selected_target_lang);
                let label_text = t("pasta_trad", lang)
                    .replace("portuguese", &example_folder)
                    .replace("Portuguese", &example_folder);
                    
                cols[2].label(RichText::new(label_text).color(Color32::from_rgb(166, 173, 200)).small());
                let edit_resp = cols[2].add(TextEdit::singleline(&mut self.config.pasta_traducao));
                if edit_resp.lost_focus() {
                    let _ = self.config.salvar();
                }
            });
        });

        ui.add_space(14.0);

        // Godot has a native translation registry in exported projects. Reusing
        // a registered locale is reliable; exported games do not load arbitrary
        // override.cfg autoload definitions.
        if self.engine_mode == 2 {
            ui.group(|ui| {
                ui.label(RichText::new("Instalação Godot").color(Color32::from_rgb(166, 227, 161)).strong());
                let mut setting_changed = false;
                ui.horizontal(|ui| {
                    ui.label("Estratégia:");
                    let response = egui::ComboBox::from_id_salt("godot_injection_mode")
                        .selected_text(match self.config.godot_injection_mode.as_str() {
                            "force_slot" => "Forçar idioma nativo",
                            "direct_patch" => "Patch direto de arquivos",
                            _ => "Automático (recomendado)",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.config.godot_injection_mode, "auto".into(), "Automático (recomendado)");
                            ui.selectable_value(&mut self.config.godot_injection_mode, "force_slot".into(), "Forçar idioma nativo");
                            ui.selectable_value(&mut self.config.godot_injection_mode, "direct_patch".into(), "Patch direto de arquivos");
                        });
                    setting_changed |= response.response.changed();
                });
                if !self.godot_native_locales.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label("Slot do jogo:");
                        let response = egui::ComboBox::from_id_salt("godot_force_locale")
                            .selected_text(&self.config.godot_force_locale)
                            .show_ui(ui, |ui| {
                                for locale in &self.godot_native_locales {
                                    ui.selectable_value(&mut self.config.godot_force_locale, locale.clone(), locale);
                                }
                            });
                        setting_changed |= response.response.changed();
                    });
                    ui.label(RichText::new(format!(
                        "Idiomas nativos detectados: {}. A extração usa apenas o Idioma original; o slot acima define onde instalar o PT-BR.",
                        self.godot_native_locales.join(", ")
                    )).small().color(Color32::from_rgb(166, 173, 200)));
                } else {
                    ui.label(RichText::new("Nenhum .translation nativo detectado; Automático usará Patch direto.").small().color(Color32::from_rgb(249, 226, 175)));
                }
                if setting_changed {
                    self.config.salvar();
                }
            });
            ui.add_space(14.0);
        }

        // 4. Progress bar (moved to top)
        let current_engine_running = self.running_engines[self.engine_mode as usize];
        let current_progress = self.engine_progress[self.engine_mode as usize];
        
        if current_engine_running || current_progress.1 > 0 {
            let fraction = if current_progress.1 > 0 {
                current_progress.0 as f32 / current_progress.1 as f32
            } else {
                0.0
            };

            let text = format!("{:.1}%", fraction * 100.0);
            self.draw_progress_bar(ui, fraction, &text);
            ui.add_space(14.0);
        }

        // 5. Action Buttons
        ui.vertical(|ui| {
            ui.horizontal_wrapped(|ui| {
                if !current_engine_running {
                    let t_time = ctx.input(|i| i.time);
                    let pulse = (t_time * 4.0).sin() as f32 * 0.5 + 0.5; // 0.0 to 1.0

                    let accent = ui.visuals().selection.bg_fill;
                    let base_col = [accent.r() as f32, accent.g() as f32, accent.b() as f32];
                    let btn_text = if self.engine_mode == 0 {
                        t("iniciar_trad_renpy", lang)
                    } else if self.engine_mode == 1 {
                        t("iniciar_trad_unity", lang)
                    } else {
                        t("iniciar_trad_godot", lang)
                    };

                    let p_col = [255.0, 255.0, 255.0];
                    let btn_color = Color32::from_rgb(
                        (base_col[0] * (1.0 - pulse * 0.2) + p_col[0] * pulse * 0.2) as u8,
                        (base_col[1] * (1.0 - pulse * 0.2) + p_col[1] * pulse * 0.2) as u8,
                        (base_col[2] * (1.0 - pulse * 0.2) + p_col[2] * pulse * 0.2) as u8,
                    );

                    let main_btn = egui::Button::new(RichText::new(btn_text).color(ui.visuals().window_fill).strong().size(14.0))
                        .fill(btn_color)
                        .min_size(egui::vec2(220.0, 38.0))
                        .rounding(Rounding::same(6.0));

                    if ui.add(main_btn).clicked() {
                        if self.check_translation_folder_exists() {
                            self.show_overwrite_modal = true;
                        } else {
                            self.start_translation(false);
                        }
                    }
                    ctx.request_repaint(); // Animate continuously

                    // INJETAR sempre visível: o pipeline de injeção valida
                    // internamente a presença da tradução e mostra erro claro.
                    // Ren'Py injeta automaticamente durante a tradução (gera game/tl/),
                    // então o botão "Injetar" é redundante — escondemos nesse motor.
                    if self.engine_mode != 0 {
                        ui.add_space(8.0);
                        let inject_icon = egui::Image::new(egui::include_image!("../../../assets/inject_icon.svg"))
                            .max_height(18.0)
                            .tint(ui.visuals().window_fill);
                        let inject_btn = egui::Button::image_and_text(
                            inject_icon,
                            RichText::new(t("inj_trad", lang))
                                .color(ui.visuals().window_fill).strong().size(14.0))
                            .fill(ui.visuals().selection.bg_fill)
                            .min_size(egui::vec2(180.0, 38.0))
                            .rounding(Rounding::same(6.0));

                        if ui.add(inject_btn).clicked() {
                            if self.engine_mode == 1 {
                                self.show_inject_modal = true;
                            } else {
                                self.start_godot_inject();
                            }
                        }
                    }

                    // PATCH cria o pacote de tradução pronto-para-uso.
                    // Só aparece depois que a tradução já foi extraída/gerada:
                    // - Unity/Godot: JSON dentro de TBX_Workspace_<pasta_traducao>.
                    // - Ren'Py: pasta game/tl/<lang>/ criada pelo injetor.
                    let has_translation = !self.game_path.is_empty() && {
                        let exe_path = std::path::Path::new(&self.game_path);
                        let base = exe_path.parent().unwrap_or(std::path::Path::new("."));
                        match self.engine_mode {
                            0 => {
                                let game_dir = if base.join("game").is_dir() {
                                    base.join("game")
                                } else {
                                    base.to_path_buf()
                                };
                                // Ren'Py grava a tradução em game/tl/<language_identifier(pasta_traducao)>
                                // (ex.: "brazilian"), não em tl/<lang_code>.
                                let lang_id = crate::renpy_extractor::language_identifier(&self.config.pasta_traducao);
                                let tl_dir = game_dir.join("tl").join(&lang_id);
                                tl_dir.is_dir() && std::fs::read_dir(&tl_dir)
                                    .map(|mut it| it.next().is_some())
                                    .unwrap_or(false)
                            }
                            1 => crate::unity_extractor::output_folder(&self.game_path, &self.config.pasta_traducao, &self.config.idioma_alvo)
                                .join("translated_texts.json").is_file(),
                            _ => crate::godot_extractor::output_folder(&self.game_path, &self.config.pasta_traducao, &self.config.idioma_alvo)
                                .join("translation.json").is_file(),
                        }
                    };

                    if has_translation {
                        ui.add_space(8.0);
                        let patch_icon = egui::Image::new(egui::include_image!("../../../assets/language_icon.svg"))
                            .max_height(18.0)
                            .tint(ui.visuals().window_fill);
                        let patch_btn = egui::Button::image_and_text(
                            patch_icon,
                            RichText::new(t("criar_patch", lang))
                                .color(ui.visuals().window_fill).strong().size(14.0))
                            .fill(ui.visuals().selection.bg_fill)
                            .min_size(egui::vec2(160.0, 38.0))
                            .rounding(Rounding::same(6.0));

                        if ui.add(patch_btn).clicked() {
                            // Abre o modal: método de injeção + pasta de destino +
                            // formato (zip ou pasta) — depois dispara `start_create_patch`.
                            self.show_create_patch_modal = true;
                        }
                    }

                    ui.add_space(8.0);
                    let editor_btn = egui::Button::image_and_text(
                        egui::Image::new(egui::include_image!("../../../assets/edit_icon.svg")).max_height(18.0).tint(ui.visuals().text_color()),
                        RichText::new(t("abrir_editor", lang)).color(ui.visuals().text_color()).strong().size(14.0)
                    )
                        .fill(ui.visuals().widgets.inactive.bg_fill)
                        .min_size(egui::vec2(200.0, 38.0))
                        .rounding(Rounding::same(6.0));

                    if ui.add(editor_btn).clicked() {
                        self.editor_state.load_directory(&self.game_path, &self.config.pasta_traducao, self.engine_mode as u8);
                        self.current_tab = AppTab::Editor;
                    }

                    ui.add_space(8.0);
                    let settings_btn = egui::Button::image(
                        egui::Image::new(egui::include_image!("../../../assets/settings_icon.svg"))
                            .max_height(20.0)
                            .tint(ui.visuals().text_color())
                    )
                    .min_size(egui::vec2(38.0, 38.0))
                    .rounding(Rounding::same(6.0));

                    if ui.add(settings_btn).clicked() {
                        self.show_engine_modal = true;
                        self.engine_modal_tab = self.engine_mode as usize;
                    }
                } else {
                    let cancel_btn = Button::image_and_text(
                        egui::Image::new(egui::include_image!("../../../assets/stop_icon.svg"))
                            .max_size(vec2(15.0, 15.0)),
                        RichText::new("CANCELAR TRADUÇÃO").color(Color32::WHITE).strong(),
                    )
                        .fill(Color32::from_rgb(243, 139, 168))
                        .rounding(Rounding::same(8.0))
                        .min_size(vec2(0.0, 40.0));
                    if ui.add(cancel_btn).clicked() {
                        self.show_cancel_modal = true;
                    }
                }
            });
        });

        ui.add_space(14.0);

        // 5. Quick log preview in translate tab
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::Image::new(egui::include_image!("../../../assets/history_icon.svg"))
                        .max_size(vec2(14.0, 14.0)),
                );
                ui.label(RichText::new("Últimos eventos:").color(Color32::from_rgb(166, 173, 200)).small().strong());
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let button = Button::image_and_text(
                        egui::Image::new(egui::include_image!("../../../assets/arrow_right_icon.svg"))
                            .max_size(vec2(13.0, 13.0)),
                        "Ver todos os logs",
                    );
                    if ui.add(button).clicked() {
                        self.current_tab = AppTab::Logs;
                    }
                });
            });

            ui.add_space(4.0);
            let log_height = ui.available_height() - 8.0;
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(if log_height > 60.0 { log_height } else { 140.0 })
                .stick_to_bottom(true)
                .id_salt("mini_log_scroll")
                .show(ui, |ui| {
                    let engine_log_idx = self.engine_log_tabs[self.engine_mode as usize];
                    if let Some(tab) = self.log_tabs.get(engine_log_idx) {
                        for line in tab.lines.iter().rev().take(15).rev() {
                            ui.label(RichText::new(line).monospace().size(11.0).color(Color32::from_rgb(166, 227, 161)));
                        }
                    }
                });
        });
    }


}
