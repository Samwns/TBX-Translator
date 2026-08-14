use crate::ui::{AppTab, TbxApp};
use crate::i18n::t;
use egui::*;

impl TbxApp {
    pub fn render_custom_title_bar(&mut self, ui: &mut Ui, ctx: &Context) {
        let bar_color = Color32::from_rgb(17, 17, 27); // #11111b
        let underline_color = if self.engine_mode == 0 {
            Color32::from_rgb(249, 226, 175) // Ren'Py Gold
        } else {
            Color32::from_rgb(137, 180, 250) // Unity Blue
        };

        Frame::none()
            .fill(bar_color)
            .rounding(0.0)
            .inner_margin(Margin::symmetric(14.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // App Logo Icon & Title
                    ui.add(egui::Image::new(egui::include_image!("../../assets/com.tbx.translator.svg")).max_height(20.0));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("TBX TRANSLATOR")
                            .color(Color32::WHITE)
                            .strong()
                            .size(15.0),
                    );

                    // Drag area takes remaining space before window controls
                    let drag_resp = ui.allocate_response(
                        vec2(ui.available_width() - 80.0, 32.0),
                        egui::Sense::click_and_drag(),
                    );
                    if drag_resp.drag_started() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }

                    // Window controls: Minimize and Close
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        let (close_rect, close_resp) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::click());
                        if close_resp.hovered() {
                            ui.painter().rect_filled(close_rect, Rounding::same(4.0), Color32::from_rgba_unmultiplied(243, 139, 168, 60));
                        }
                        let close_color = if close_resp.hovered() { Color32::WHITE } else { Color32::from_rgb(243, 139, 168) };
                        ui.painter().text(close_rect.center(), Align2::CENTER_CENTER, "X", FontId::proportional(14.0), close_color);
                        if close_resp.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }

                        let (min_rect, min_resp) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::click());
                        if min_resp.hovered() {
                            ui.painter().rect_filled(min_rect, Rounding::same(4.0), Color32::from_rgba_unmultiplied(166, 173, 200, 60));
                        }
                        let min_color = if min_resp.hovered() { Color32::WHITE } else { Color32::from_rgb(166, 173, 200) };
                        ui.painter().text(min_rect.center(), Align2::CENTER_CENTER, "—", FontId::proportional(14.0), min_color);
                        if min_resp.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });
            });

        // Dynamic colored underline bar
        ui.painter().rect_filled(
            egui::Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), 2.0)),
            Rounding::ZERO,
            underline_color,
        );
        ui.add_space(2.0);
    }

    pub fn render_top_navigation_bar(&mut self, ui: &mut Ui, ctx: &Context) {
        let lang = &self.config.ui_language;

        Frame::none()
            .fill(Color32::from_rgb(24, 24, 37)) // #181825
            .inner_margin(Margin::symmetric(14.0, 10.0)) // increased top/bottom padding
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // StackSwitcher-like segmented control
                    ui.spacing_mut().item_spacing.x = 0.0; // No gap between tabs

                    let nav_tab = |ui: &mut Ui, target: AppTab, icon_source: egui::ImageSource<'_>, label: &str, current: &mut AppTab, is_first: bool, is_last: bool| {
                        let active = *current == target;

                        // Animate transition for the active state
                        let anim_t = ctx.animate_bool_with_time(ui.id().with("tab_anim").with(target as usize), active, 0.2);

                        // Interpolate background color
                        let bg_r = (24.0 * (1.0 - anim_t) + 69.0 * anim_t) as u8;
                        let bg_g = (24.0 * (1.0 - anim_t) + 71.0 * anim_t) as u8;
                        let bg_b = (37.0 * (1.0 - anim_t) + 90.0 * anim_t) as u8;
                        let bg_color = Color32::from_rgb(bg_r, bg_g, bg_b);

                        let mut rounding = Rounding::ZERO;
                        if is_first { rounding.nw = 6.0; rounding.sw = 6.0; }
                        if is_last { rounding.ne = 6.0; rounding.se = 6.0; }

                        let btn = Button::image_and_text(
                            egui::Image::new(icon_source).max_height(16.0),
                            RichText::new(label)
                                .color(if active { Color32::WHITE } else { Color32::from_rgb(166, 173, 200) })
                                .strong(),
                        )
                        .fill(bg_color)
                        .rounding(rounding)
                        .min_size(vec2(100.0, 32.0)); // Make tabs thicker and uniform

                        if ui.add(btn).clicked() {
                            *current = target;
                        }
                    };

                    // The tabs
                    nav_tab(ui, AppTab::Translate, egui::include_image!("../../assets/folder_icon.svg"), &t("aba_traduzir", lang), &mut self.current_tab, true, false);
                    nav_tab(ui, AppTab::Logs, egui::include_image!("../../assets/logs_icon.svg"), &t("aba_logs", lang), &mut self.current_tab, false, false);
                    nav_tab(ui, AppTab::Tools, egui::include_image!("../../assets/tools_icon.svg"), &t("aba_tools", lang), &mut self.current_tab, false, false);
                    nav_tab(ui, AppTab::Settings, egui::include_image!("../../assets/settings_icon.svg"), &t("aba_config", lang), &mut self.current_tab, false, true);

                    // Contextual active tools (detached from segmented control)
                    if self.current_tab == AppTab::Editor {
                        ui.add_space(12.0);
                        let active_btn = Button::image_and_text(
                            egui::Image::new(egui::include_image!("../../assets/tools_icon.svg")).max_height(16.0),
                            RichText::new("Editor de Textos").color(Color32::from_rgb(203, 166, 247)).strong()
                        )
                        .fill(Color32::from_rgb(49, 50, 68))
                        .rounding(Rounding::same(6.0))
                        .min_size(vec2(0.0, 32.0));
                        ui.add(active_btn);
                    }

                    if self.current_tab == AppTab::FontInjector {
                        ui.add_space(12.0);
                        let active_btn = Button::image_and_text(
                            egui::Image::new(egui::include_image!("../../assets/font_icon.svg")).max_height(16.0),
                            RichText::new("Injetor de Fontes").color(Color32::from_rgb(166, 227, 161)).strong()
                        )
                        .fill(Color32::from_rgb(49, 50, 68))
                        .rounding(Rounding::same(6.0))
                        .min_size(vec2(0.0, 32.0));
                        ui.add(active_btn);
                    }

                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.vertical(|ui| {
                            ui.with_layout(egui::Layout::right_to_left(Align::Min), |ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "v{} | by samwns",
                                        crate::updater::current_version()
                                    ))
                                    .color(Color32::from_rgb(108, 112, 134))
                                    .small(),
                                );
                            });
                            if let Some(release) = self
                                .update_release
                                .as_ref()
                                .filter(|release| crate::updater::is_newer(&release.tag_name))
                            {
                                ui.with_layout(egui::Layout::right_to_left(Align::Min), |ui| {
                                    let message = format!(
                                        "↑ {}: {}",
                                        t("nova_versao_disponivel", lang),
                                        release.tag_name
                                    );
                                    if ui
                                        .add(
                                            Label::new(
                                                RichText::new(message)
                                                    .color(Color32::from_rgb(166, 227, 161))
                                                    .small()
                                                    .strong(),
                                            )
                                            .sense(Sense::click()),
                                        )
                                        .on_hover_text(t("atualizacoes", lang))
                                        .clicked()
                                    {
                                        self.current_tab = AppTab::Settings;
                                    }
                                });
                            }
                        });
                    });
                });
            });

        ui.separator();
    }


}
