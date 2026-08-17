use crate::ui::{TbxApp, AppTab};
use egui::*;

impl TbxApp {
    pub fn render_editor_view(&mut self, ui: &mut Ui, ctx: &Context) {
        ui.horizontal(|ui| {
            let back_button = Button::image_and_text(
                egui::Image::new(egui::include_image!("../../../assets/arrow_left_icon.svg"))
                    .max_size(vec2(14.0, 14.0)),
                "Voltar para tradução",
            );
            if ui.add(back_button).clicked() {
                self.current_tab = AppTab::Translate;
            }
            ui.label(RichText::new("Editor Manual de Textos").color(Color32::WHITE).strong().size(16.0));
        });
        ui.add_space(6.0);
        self.editor_state.render_ui(ui, ctx, &mut self.tags_jogo);
    }


}
