use crate::ui::{TbxApp, AppTab};
use egui::*;

impl TbxApp {
    pub fn render_font_injector_view(&mut self, ui: &mut Ui, ctx: &Context) {
        ui.horizontal(|ui| {
            if ui.button("← Voltar para Ferramentas").clicked() {
                self.current_tab = AppTab::Tools;
            }
            ui.label(RichText::new("Injetor & Testador de Fontes").color(Color32::WHITE).strong().size(16.0));
        });
        ui.add_space(6.0);
        self.font_injector_state.render_ui(ui, ctx, &self.game_path, &self.config.ui_language);
    }


}
