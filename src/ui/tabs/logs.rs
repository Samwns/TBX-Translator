use crate::ui::TbxApp;
use egui::*;

impl TbxApp {
    pub fn render_logs_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Console de Eventos").color(Color32::WHITE).strong().size(16.0));

            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                let clear_button = Button::image_and_text(
                    egui::Image::new(egui::include_image!("../../../assets/trash_icon.svg"))
                        .max_size(vec2(14.0, 14.0)),
                    "Limpar logs",
                );
                if ui.add(clear_button).clicked() {
                    if let Some(tab) = self.log_tabs.get_mut(self.active_log_tab) {
                        tab.lines.clear();
                    }
                }
                let copy_button = Button::image_and_text(
                    egui::Image::new(egui::include_image!("../../../assets/copy_icon.svg"))
                        .max_size(vec2(14.0, 14.0)),
                    "Copiar logs",
                );
                if ui.add(copy_button).clicked() {
                    if let Some(tab) = self.log_tabs.get(self.active_log_tab) {
                        let text = tab.lines.join("\n");
                        ui.output_mut(|o| o.copied_text = text);
                    }
                }
            });
        });

        ui.add_space(6.0);

        // Tab buttons for each log session
        ui.horizontal(|ui| {
            let mut tab_to_close: Option<usize> = None;

            for (idx, tab) in self.log_tabs.iter().enumerate() {
                let is_active = self.active_log_tab == idx;
                let bg_color = if is_active { Color32::from_rgb(49, 50, 68) } else { Color32::from_rgb(17, 17, 27) };
                let text_color = if is_active { Color32::WHITE } else { Color32::from_rgb(166, 173, 200) };

                let mut close_clicked = false;
                let mut close_btn_rect = None;
                let frame_resp = Frame::none()
                    .fill(bg_color)
                    .rounding(Rounding::same(6.0))
                    .inner_margin(Margin::symmetric(10.0, 4.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            ui.label(RichText::new(&tab.title).color(text_color).strong());

                            if tab.closable {
                                let (rect, response) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::click());
                                close_btn_rect = Some(rect);

                                if response.hovered() {
                                    ui.painter().rect_filled(rect, Rounding::same(4.0), Color32::from_rgba_unmultiplied(243, 139, 168, 60));
                                }

                                let text_color = if response.hovered() { Color32::WHITE } else { Color32::from_rgb(243, 139, 168) };
                                ui.painter().text(rect.center(), Align2::CENTER_CENTER, "X", FontId::proportional(12.0), text_color);

                                if response.clicked() {
                                    tab_to_close = Some(idx);
                                    close_clicked = true;
                                }
                            }
                        });
                    }).response;

                if !close_clicked {
                    let mut should_interact = true;
                    if let Some(pos) = ui.ctx().pointer_hover_pos() {
                        if let Some(rect) = close_btn_rect {
                            if rect.contains(pos) {
                                should_interact = false;
                            }
                        }
                    }
                    if should_interact {
                        let interact_resp = ui.interact(frame_resp.rect, ui.id().with(format!("tab_interact_{}", idx)), Sense::click());
                        if interact_resp.clicked() {
                            self.active_log_tab = idx;
                        }
                    }
                }
            }

            if let Some(close_idx) = tab_to_close {
                if self.log_tabs.len() > 1 {
                    self.log_tabs.remove(close_idx);
                    if self.active_log_tab >= self.log_tabs.len() {
                        self.active_log_tab = self.log_tabs.len() - 1;
                    }
                }
            }
        });

        ui.add_space(6.0);

        // Monospace Console Body
        Frame::none()
            .fill(Color32::from_rgb(11, 11, 18)) // #0b0b12
            .inner_margin(Margin::same(10.0))
            .rounding(Rounding::same(6.0))
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .id_salt("full_log_console")
                    .show(ui, |ui| {
                        if let Some(tab) = self.log_tabs.get(self.active_log_tab) {
                            if tab.lines.is_empty() {
                                ui.label(RichText::new("Aguardando execução...").color(Color32::from_rgb(108, 112, 134)));
                            } else {
                                for line in &tab.lines {
                                    let col = if line.contains("[Erro") || line.contains("Falha") {
                                        Color32::from_rgb(243, 139, 168)
                                    } else if line.contains("[Aviso") {
                                        Color32::from_rgb(249, 226, 175)
                                    } else if line.contains("[Concluído") || line.contains("[Sistema]") {
                                        Color32::from_rgb(166, 227, 161)
                                    } else {
                                        Color32::from_rgb(205, 214, 244)
                                    };
                                    ui.label(RichText::new(line).monospace().size(12.0).color(col));
                                }
                            }
                        }
                    });
            });
    }


}
