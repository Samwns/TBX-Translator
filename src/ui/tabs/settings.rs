use crate::ui::{TbxApp, AppTab};
use crate::i18n::t;
use egui::*;
use crate::ui::toggle_ui;

impl TbxApp {
    fn check_for_updates(&mut self) {
        if self.update_checking || self.update_downloading {
            return;
        }
        self.update_checking = true;
        self.update_status = t("buscando_atualizacoes", &self.config.ui_language);
        self.update_progress = (0, 0);
        let tx = self.tx.clone();
        tokio::spawn(async move {
            match crate::updater::check_latest().await {
                Ok(release) => {
                    let _ = tx.send(crate::types::UiMsg::UpdateFound(release));
                }
                Err(error) => {
                    let _ = tx.send(crate::types::UiMsg::UpdateError(error));
                }
            }
        });
    }

    fn apply_available_update(&mut self) {
        if self.update_downloading || self.running_engines.iter().any(|running| *running) {
            return;
        }
        let Some(release) = self.update_release.clone() else { return };
        if !crate::updater::is_newer(&release.tag_name) {
            return;
        }
        self.update_downloading = true;
        self.update_progress = (0, 0);
        self.update_status = t("atualizacao_em_andamento", &self.config.ui_language);
        let tx = self.tx.clone();
        let language = self.config.ui_language.clone();
        tokio::spawn(async move {
            if let Err(error) = crate::updater::download_apply_and_restart(release, tx.clone(), language).await {
                let _ = tx.send(crate::types::UiMsg::UpdateError(error));
            }
        });
    }

    pub fn render_settings_tab(&mut self, ui: &mut Ui) {
        let lang = &self.config.ui_language.clone();

        ui.label(RichText::new(t("config_geral", lang)).color(Color32::WHITE).strong().size(16.0));
        ui.add_space(8.0);

        ui.group(|ui| {
            // UI Language
            ui.horizontal(|ui| {
                ui.label(RichText::new(t("idioma_app", lang)).color(Color32::from_rgb(205, 214, 244)).strong());

                let selected_display = crate::api::get_lang_name(&self.config.ui_language);

                egui::ComboBox::from_id_salt("ui_language_combo")
                    .selected_text(selected_display)
                    .show_ui(ui, |ui| {
                        // ScrollArea is handled automatically by ComboBox for large lists in egui
                        for lang_name in crate::api::ALL_LANGUAGES {
                            let code = crate::api::get_lang_code(lang_name);
                            ui.selectable_value(&mut self.config.ui_language, code.to_string(), *lang_name);
                        }
                    });
            });

            ui.add_space(8.0);

            // API Engine
            ui.horizontal(|ui| {
                ui.label(RichText::new("Motor de Tradução (API):").color(Color32::from_rgb(205, 214, 244)).strong());
                ui.label(RichText::new("Google Translator (Nativo Gratuito)").color(Color32::from_rgb(166, 227, 161)).strong());
            });

            ui.add_space(8.0);

            // Multi-thread toggle
            toggle_ui(ui, &mut self.config.usar_multi_trad, &t("ativar_multi", lang));
            if self.config.usar_multi_trad {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t("qtd_threads", lang)).color(Color32::from_rgb(166, 173, 200)));
                    let mut threads_str = self.config.threads_ativas.to_string();
                    if ui.add(TextEdit::singleline(&mut threads_str).desired_width(60.0)).changed() {
                        if let Ok(val) = threads_str.parse::<u32>() {
                            self.config.threads_ativas = val.clamp(1, 4);
                        }
                    }
                    ui.label(RichText::new("(Recomendado: 3; limite seguro: 4)").color(Color32::from_rgb(108, 112, 134)).small());
                });
            }

            ui.add_space(8.0);

            // Engine settings modal button
            if ui.button("⚙ Configurações Extras dos Motores...").clicked() {
                self.show_engine_modal = true;
            }

            ui.add_space(8.0);
            ui.colored_label(Color32::from_rgb(249, 226, 175), t("aviso_ip", lang));
        });

        ui.add_space(14.0);

        ui.label(
            RichText::new(t("atualizacoes", lang))
                .color(Color32::WHITE)
                .strong()
                .size(16.0),
        );
        ui.add_space(8.0);
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{}: {}",
                    t("versao_atual", lang),
                    crate::updater::current_version()
                ));

                let check_text = if self.update_checking {
                    t("buscando_atualizacoes", lang)
                } else {
                    t("buscar_atualizacoes", lang)
                };
                if ui
                    .add_enabled(
                        !self.update_checking && !self.update_downloading,
                        Button::new(check_text),
                    )
                    .clicked()
                {
                    self.check_for_updates();
                }
            });

            if !self.update_status.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new(&self.update_status).color(Color32::from_rgb(166, 227, 161)));
            }

            if self.update_downloading {
                let (downloaded, total) = self.update_progress;
                let fraction = if total > 0 {
                    (downloaded as f32 / total as f32).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                ui.add(
                    ProgressBar::new(fraction)
                        .show_percentage()
                        .animate(total == 0),
                );
            }

            if let Some(release) = self.update_release.clone() {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&release.name).strong());
                    ui.hyperlink_to(t("abrir_release", lang), &release.html_url);
                });

                if crate::updater::is_newer(&release.tag_name) {
                    let translations_running = self.running_engines.iter().any(|running| *running);
                    if ui
                        .add_enabled(
                            !self.update_downloading && !translations_running,
                            Button::new(
                                RichText::new(t("baixar_instalar_reiniciar", lang))
                                    .strong()
                                    .color(Color32::from_rgb(17, 17, 27)),
                            )
                            .fill(Color32::from_rgb(137, 180, 250)),
                        )
                        .clicked()
                    {
                        self.apply_available_update();
                    }
                    if translations_running {
                        ui.label(
                            RichText::new(t("aguarde_traducoes", lang))
                                .small()
                                .color(Color32::from_rgb(249, 226, 175)),
                        );
                    }
                } else {
                    ui.label(t("nenhuma_atualizacao", lang));
                }

                if !release.body.trim().is_empty() {
                    ui.separator();
                    ui.label(RichText::new(t("changelog", lang)).strong());
                    ScrollArea::vertical()
                        .id_salt("github_changelog")
                        .max_height(220.0)
                        .show(ui, |ui| {
                            ui.label(&release.body);
                        });
                }
            }
        });

        ui.add_space(14.0);

        let save_btn = Button::new(RichText::new(t("salvar_config", lang)).color(Color32::from_rgb(17, 17, 27)).strong())
            .fill(Color32::from_rgb(166, 227, 161))
            .min_size(vec2(220.0, 36.0))
            .rounding(Rounding::same(6.0));

        if ui.add(save_btn).clicked() {
            self.config.idioma_origem = self.selected_source_lang.clone();
            self.config.idioma_alvo = self.selected_target_lang.clone();
            let _ = self.config.salvar();
            self.show_alert_modal = Some((
                false,
                "Configurações".to_string(),
                "Configurações salvas com sucesso em ~/.tbx-translator/config.properties!".to_string(),
            ));
        }
    }


}
