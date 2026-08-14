use crate::i18n::t;
use crate::ui::{AppTab, TbxApp};
use egui::*;

impl TbxApp {
    pub(crate) fn check_for_updates(&mut self, silent: bool) {
        if self.update_checking || self.update_downloading {
            return;
        }
        self.update_checking = true;
        self.update_check_silent = silent;
        if !silent {
            self.update_status = t("buscando_atualizacoes", &self.config.ui_language);
        }
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
            if let Err(error) =
                crate::updater::download_apply_and_restart(release, tx.clone(), language).await
            {
                let _ = tx.send(crate::types::UiMsg::UpdateError(error));
            }
        });
    }

    pub fn render_updates_tab(&mut self, ui: &mut Ui) {
        let lang = self.config.ui_language.clone();
        self.update_notice_unread = false;

        ui.horizontal(|ui| {
            if ui.button(t("voltar", &lang)).clicked() {
                self.current_tab = AppTab::Translate;
            }
            ui.add_space(8.0);
            ui.add(
                egui::Image::new(egui::include_image!("../../../assets/update_icon.svg"))
                    .max_height(22.0),
            );
            ui.label(
                RichText::new(t("central_atualizacoes", &lang))
                    .color(Color32::WHITE)
                    .strong()
                    .size(19.0),
            );
        });
        ui.add_space(6.0);
        ui.label(
            RichText::new(t("verificacao_automatica", &lang))
                .small()
                .color(Color32::from_rgb(166, 173, 200)),
        );
        ui.add_space(12.0);

        Frame::group(ui.style())
            .fill(Color32::from_rgb(24, 24, 37))
            .inner_margin(Margin::same(14.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t("versao_atual", &lang)).strong());
                    ui.label(crate::updater::current_version());
                    if self.update_checking {
                        ui.spinner();
                        ui.label(t("buscando_atualizacoes", &lang));
                    }
                });

                if !self.update_status.is_empty() {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(&self.update_status)
                            .color(Color32::from_rgb(166, 227, 161)),
                    );
                }

                if self.update_downloading {
                    ui.add_space(8.0);
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
            });

        ui.add_space(12.0);
        if let Some(release) = self.update_release.clone() {
            let newer = crate::updater::is_newer(&release.tag_name);
            Frame::group(ui.style())
                .fill(Color32::from_rgb(24, 24, 37))
                .inner_margin(Margin::same(14.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(&release.name)
                                    .strong()
                                    .size(17.0)
                                    .color(if newer {
                                        Color32::from_rgb(166, 227, 161)
                                    } else {
                                        Color32::WHITE
                                    }),
                            );
                            ui.label(
                                RichText::new(&release.tag_name)
                                    .small()
                                    .color(Color32::from_rgb(108, 112, 134)),
                            );
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.hyperlink_to(t("abrir_release", &lang), &release.html_url);
                        });
                    });

                    ui.add_space(10.0);
                    if newer {
                        let translations_running =
                            self.running_engines.iter().any(|running| *running);
                        if ui
                            .add_enabled(
                                !self.update_downloading && !translations_running,
                                Button::new(
                                    RichText::new(t("baixar_instalar_reiniciar", &lang))
                                        .strong()
                                        .color(Color32::from_rgb(17, 17, 27)),
                                )
                                .fill(Color32::from_rgb(166, 227, 161))
                                .min_size(vec2(260.0, 36.0)),
                            )
                            .clicked()
                        {
                            self.apply_available_update();
                        }
                        if translations_running {
                            ui.label(
                                RichText::new(t("aguarde_traducoes", &lang))
                                    .small()
                                    .color(Color32::from_rgb(249, 226, 175)),
                            );
                        }
                    } else {
                        ui.label(
                            RichText::new(t("nenhuma_atualizacao", &lang))
                                .color(Color32::from_rgb(166, 227, 161)),
                        );
                    }
                });

            ui.add_space(12.0);
            ui.label(
                RichText::new(t("changelog", &lang))
                    .color(Color32::WHITE)
                    .strong()
                    .size(17.0),
            );
            ui.add_space(6.0);
            Frame::group(ui.style())
                .fill(Color32::from_rgb(24, 24, 37))
                .inner_margin(Margin::same(14.0))
                .show(ui, |ui| {
                    ScrollArea::vertical()
                        .id_salt("github_changelog_screen")
                        .max_height(ui.available_height().max(180.0))
                        .show(ui, |ui| {
                            if release.body.trim().is_empty() {
                                ui.label(t("changelog_vazio", &lang));
                            } else {
                                ui.label(&release.body);
                            }
                        });
                });
        } else if !self.update_checking {
            ui.label(
                RichText::new(t("verificacao_automatica", &lang))
                    .color(Color32::from_rgb(166, 173, 200)),
            );
        }
    }
}
