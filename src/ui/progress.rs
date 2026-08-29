use eframe::egui;

pub fn draw_custom_progress(ui: &mut egui::Ui, fraction: f32, text: &str) {
    let height = 24.0;
    let (rect, _response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );

    let visuals = ui.style().visuals.clone();
    
    // Fundo da barra
    let bg_color = visuals.extreme_bg_color;
    ui.painter().rect_filled(rect, 4.0, bg_color);

    // Progresso
    let fill_color = visuals.selection.bg_fill;
    let mut fill_rect = rect;
    let clamped_fraction = fraction.clamp(0.0, 1.0);
    fill_rect.set_width(rect.width() * clamped_fraction);

    let time = ui.input(|i| i.time);

    // Se fraction > 0, desenha a barra cheia com uma cor bonita
    if clamped_fraction > 0.0 {
        // Efeito de pulso suave na cor
        let pulse = (time * 3.0).sin() as f32 * 0.15 + 0.85;
        let mut animated_color = fill_color;
        animated_color[0] = (animated_color[0] as f32 * pulse) as u8;
        animated_color[1] = (animated_color[1] as f32 * pulse) as u8;
        animated_color[2] = (animated_color[2] as f32 * pulse) as u8;

        ui.painter().rect_filled(fill_rect, 4.0, animated_color);
    } else {
        // Estado indeterminado: barra fica "pulsando"
        let pulse = (time * 4.0).sin() as f32 * 0.5 + 0.5;
        let mut ind_color = visuals.widgets.inactive.bg_fill;
        ind_color[0] = (ind_color[0] as f32 * pulse) as u8;
        ind_color[1] = (ind_color[1] as f32 * pulse) as u8;
        ind_color[2] = (ind_color[2] as f32 * pulse) as u8;
        ui.painter().rect_filled(rect, 4.0, ind_color);
        ui.ctx().request_repaint(); // Continua animando
    }

    if clamped_fraction > 0.0 {
        ui.ctx().request_repaint();
    }

    // Texto flutuando com a barra
    let font_id = egui::FontId::proportional(14.0);
    
    // Simula o tamanho para saber onde colocar
    let galley_temp = ui.painter().layout_no_wrap(text.to_string(), font_id.clone(), egui::Color32::WHITE);
    let mut text_pos = fill_rect.right_center();
    
    // Ajusta pra ficar dentro da barra, na pontinha
    text_pos.x -= galley_temp.size().x + 8.0;
    
    let mut text_color = egui::Color32::WHITE;

    // Se a barra estiver muito pequena, bota o texto pro lado de fora
    if fill_rect.width() < galley_temp.size().x + 16.0 {
        text_pos.x = fill_rect.right_center().x + 8.0;
        text_color = visuals.text_color(); // Cor do tema (fora da barra colorida)
    }
    
    text_pos.y -= galley_temp.size().y / 2.0;

    let galley = ui.painter().layout_no_wrap(text.to_string(), font_id, text_color);
    ui.painter().galley(text_pos, galley, text_color);
}
