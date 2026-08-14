use crate::ui::{TbxApp, AppTab};
use crate::i18n::t;
use egui::*;

impl TbxApp {
    pub fn render_tools_tab(&mut self, ui: &mut Ui, _ctx: &Context) {
        let lang = &self.config.ui_language;

        ui.label(RichText::new(t("aba_tools", lang)).color(Color32::WHITE).strong().size(16.0));
        ui.label(RichText::new(t("ferramentas_desc", lang)).color(Color32::from_rgb(166, 173, 200)));

        ui.add_space(16.0);

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🔤 Injetor de Fontes:").color(Color32::from_rgb(137, 180, 250)).strong());
                ui.label(RichText::new(t("fonte_info", lang)).color(Color32::from_rgb(166, 173, 200)));
            });

            ui.add_space(6.0);

            let font_btn = Button::new(RichText::new(t("btn_font", lang)).color(Color32::from_rgb(17, 17, 27)).strong())
                .fill(Color32::from_rgb(166, 227, 161))
                .min_size(vec2(320.0, 36.0))
                .rounding(Rounding::same(6.0));

            if ui.add(font_btn).clicked() {
                self.font_injector_state.set_engine_mode(self.engine_mode);
                self.current_tab = AppTab::FontInjector;
            }
        });
    }


}
