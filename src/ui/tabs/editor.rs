use crate::ui::{TbxApp, AppTab};
use egui::*;

impl TbxApp {
    pub fn render_editor_view(&mut self, ui: &mut Ui, ctx: &Context) {
        ui.horizontal(|ui| {
            if ui.button("← Voltar para Tradução").clicked() {
                self.current_tab = AppTab::Translate;
            }
            ui.label(RichText::new("Editor Manual de Textos").color(Color32::WHITE).strong().size(16.0));
        });
        ui.add_space(6.0);
        self.editor_state.render_ui(ui, ctx);
    }


}
