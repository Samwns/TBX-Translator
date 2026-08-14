use crate::ui::TbxApp;
use crate::i18n::t;
use egui::*;
use crate::ui::toggle_ui;

impl TbxApp {
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

            toggle_ui(
                ui,
                &mut self.config.efeitos_sonoros,
                &t("efeitos_sonoros", lang),
            );

            ui.add_space(8.0);

            // Engine settings modal button
            if ui.button("⚙ Configurações Extras dos Motores...").clicked() {
                self.show_engine_modal = true;
            }

            ui.add_space(8.0);
            ui.colored_label(Color32::from_rgb(249, 226, 175), t("aviso_ip", lang));
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
