use crate::ui::TbxApp;
use egui::*;

use crate::ui::toggle_ui;
use crate::ui::t;

impl TbxApp {
    pub fn render_modals(&mut self, ctx: &Context) {
        // Exibido uma única vez depois que a versão instalada muda.
        if self.show_post_update_changelog {
            let lang = self.config.ui_language.clone();
            let version = crate::updater::current_version();
            let changelog = self.post_update_changelog.clone();
            let mut acknowledged = false;

            egui::Window::new(t("central_atualizacoes", &lang))
                .id(egui::Id::new("post_update_changelog"))
                .collapsible(false)
                .resizable(true)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .default_width(560.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Image::new(egui::include_image!(
                                "../../assets/update_icon.svg"
                            ))
                            .max_size(vec2(30.0, 30.0)),
                        );
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(t("central_atualizacoes", &lang))
                                    .size(20.0)
                                    .strong()
                                    .color(Color32::from_rgb(249, 226, 175)),
                            );
                            ui.label(
                                RichText::new(format!("TBX Translator v{version}"))
                                    .color(Color32::from_rgb(166, 173, 200)),
                            );
                        });
                    });
                    ui.add_space(10.0);
                    ui.separator();
                    ScrollArea::vertical().max_height(350.0).show(ui, |ui| {
                        ui.label(RichText::new(changelog).size(14.0));
                    });
                    ui.separator();
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                Button::new(RichText::new("OK").strong())
                                    .fill(Color32::from_rgb(166, 227, 161))
                                    .min_size(vec2(92.0, 32.0)),
                            )
                            .clicked()
                        {
                            acknowledged = true;
                        }
                    });
                });

            if acknowledged {
                self.show_post_update_changelog = false;
                self.config.ultima_versao_exibida = version;
                self.config.salvar();
            }
        }

        // 1. Overwrite Dialog Modal
        if self.show_overwrite_modal {
            egui::Window::new("Pasta de Tradução Já Existe")
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.label("A pasta de tradução para este jogo já contém arquivos extraídos.");
                    ui.label("O que você deseja fazer?");
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Sobrescrever (Limpar)").color(Color32::from_rgb(243, 139, 168)).strong()).clicked() {
                            self.show_overwrite_modal = false;
                            self.start_translation(true);
                        }

                        if ui.button(RichText::new("Atualizar (Manter Existente)").color(Color32::from_rgb(166, 227, 161)).strong()).clicked() {
                            self.show_overwrite_modal = false;
                            self.start_translation(false);
                        }

                        if ui.button("Cancelar").clicked() {
                            self.show_overwrite_modal = false;
                        }
                    });
                });
        }

        // 2. Engine Extras Modal
        if self.show_engine_modal {
            egui::Window::new("Configurações Extras dos Motores")
                .id(egui::Id::new("EngineModal_V2"))
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        
                        let renpy_active = self.engine_modal_tab == 0;
                        let r_anim = ctx.animate_bool_with_time(ui.id().with("r_tab"), renpy_active, 0.2);
                        let r_bg = Color32::from_rgb(
                            (24.0 * (1.0 - r_anim) + 49.0 * r_anim) as u8,
                            (24.0 * (1.0 - r_anim) + 50.0 * r_anim) as u8,
                            (37.0 * (1.0 - r_anim) + 68.0 * r_anim) as u8,
                        );
                        let r_text_color = Color32::from_rgb(
                            (166.0 * (1.0 - r_anim) + 249.0 * r_anim) as u8,
                            (173.0 * (1.0 - r_anim) + 226.0 * r_anim) as u8,
                            (200.0 * (1.0 - r_anim) + 175.0 * r_anim) as u8,
                        );

                        let r_btn = Button::image_and_text(
                            egui::Image::new(egui::include_image!("../../assets/renpy_icon.svg")).max_height(14.0),
                            RichText::new("Ren'Py").color(r_text_color).strong(),
                        )
                            .fill(r_bg).rounding(Rounding { nw: 6.0, sw: 6.0, ne: 0.0, se: 0.0 }).min_size(vec2(130.0, 26.0));
                        if ui.add(r_btn).clicked() { self.engine_modal_tab = 0; }

                        let unity_active = self.engine_modal_tab == 1;
                        let u_anim = ctx.animate_bool_with_time(ui.id().with("u_tab"), unity_active, 0.2);
                        let u_bg = Color32::from_rgb(
                            (24.0 * (1.0 - u_anim) + 49.0 * u_anim) as u8,
                            (24.0 * (1.0 - u_anim) + 50.0 * u_anim) as u8,
                            (37.0 * (1.0 - u_anim) + 68.0 * u_anim) as u8,
                        );
                        let u_text_color = Color32::from_rgb(
                            (166.0 * (1.0 - u_anim) + 137.0 * u_anim) as u8,
                            (173.0 * (1.0 - u_anim) + 180.0 * u_anim) as u8,
                            (200.0 * (1.0 - u_anim) + 250.0 * u_anim) as u8,
                        );

                        let u_btn = Button::image_and_text(
                            egui::Image::new(egui::include_image!("../../assets/unity_icon.svg")).max_height(14.0),
                            RichText::new("Unity").color(u_text_color).strong(),
                        )
                            .fill(u_bg).rounding(Rounding { nw: 0.0, sw: 0.0, ne: 6.0, se: 6.0 }).min_size(vec2(130.0, 26.0));
                        if ui.add(u_btn).clicked() { self.engine_modal_tab = 1; }
                    });

                    ui.add_space(8.0);

                    // Fade animation for content change
                    let _fade = ctx.animate_bool_with_time(ui.id().with("tab_fade").with(self.engine_modal_tab), true, 0.2);

                    ui.allocate_ui(vec2(280.0, 0.0), |ui| {
                        ui.vertical(|ui| {
                            if self.engine_modal_tab == 0 {
                                toggle_ui(ui, &mut self.config.manter_estrutura_original, "Manter estrutura original na pasta tl");
                                ui.add_space(4.0);
                                toggle_ui(ui, &mut self.config.preservar_nomes_renpy, "Proteger variáveis [nome] (sempre ativo)");
                                ui.add_space(4.0);
                                toggle_ui(ui, &mut self.config.traduzir_nomes_personagens_renpy, "Traduzir nomes dos personagens");
                            } else {
                            ui.label(RichText::new("Integração:").color(Color32::from_rgb(137, 180, 250)).strong());
                            ui.label("Utiliza extração direta de Assets (UABE / UnityPy)");
                            ui.add_space(8.0);
                            ui.label(RichText::new("Compatibilidade:").color(Color32::from_rgb(137, 180, 250)).strong());
                            ui.label("Compatível com TextAssets, MonoBehaviours e Fontes");
                        }
                    });

                    ui.add_space(12.0);
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        let btn = Button::new(RichText::new("FECHAR").color(Color32::WHITE).strong())
                            .fill(Color32::from_rgb(88, 91, 112))
                            .min_size(vec2(100.0, 32.0))
                            .rounding(Rounding::same(6.0));
                        if ui.add(btn).clicked() {
                            self.show_engine_modal = false;
                        }
                    });
                    });
                });
        }

        // 3. Alert / Done Modal
        let mut alert_to_close = false;
        if let Some((is_error, title, msg)) = &self.show_alert_modal {
            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    let col = if *is_error { Color32::from_rgb(243, 139, 168) } else { Color32::from_rgb(166, 227, 161) };
                    ui.label(RichText::new(msg).color(col).strong());
                    ui.add_space(8.0);
                    if ui.button("OK").clicked() {
                        alert_to_close = true;
                    }
                });
        }

        if alert_to_close {
            self.show_alert_modal = None;
        }

        // 4. Cancel Confirmation Modal
        if self.show_cancel_modal {
            egui::Window::new("Aviso de Cancelamento")
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.label(RichText::new("Você tem certeza que deseja cancelar a extração/tradução?").strong().color(Color32::from_rgb(250, 179, 135))); // Peach
                    ui.label("O processo será interrompido e arquivos pela metade podem ser gerados.");
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Sim, Cancelar").color(Color32::from_rgb(243, 139, 168)).strong()).clicked() {
                            self.show_cancel_modal = false;
                            self.cancel_current_task();
                        }

                        if ui.button("Não, Continuar").clicked() {
                            self.show_cancel_modal = false;
                        }
                    });
                });
        }

        // 5. Themes Selector Modal
        self.render_themes_modal(ctx);
    }
}

impl TbxApp {
    pub fn render_themes_modal(&mut self, ctx: &egui::Context) {
        if !self.show_themes_modal {
            return;
        }

        let themes = crate::themes::AppTheme::all();
        let current_id = self.config.theme_id.clone();

        let mut close = false;
        let mut apply_id: Option<String> = None;

        egui::Window::new("Temas de Cores")
            .id(egui::Id::new("themes_modal"))
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .default_width(620.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Image::new(egui::include_image!("../../assets/brush_icon.svg"))
                            .max_size(egui::vec2(22.0, 22.0)),
                    );
                    ui.label(egui::RichText::new("Escolha um tema visual").strong().size(15.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Fechar").clicked() {
                            close = true;
                        }
                    });
                });
                ui.separator();
                ui.add_space(6.0);

                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .id_salt("themes_scroll")
                    .show(ui, |ui| {
                        egui::Grid::new("themes_grid")
                            .num_columns(4)
                            .spacing([8.0, 8.0])
                            .show(ui, |ui| {
                                for (i, theme) in themes.iter().enumerate() {
                                    let is_active = theme.id == current_id;

                                    let card_fill = if is_active {
                                        egui::Color32::from_rgb(49, 74, 63)
                                    } else {
                                        theme.mantle
                                    };
                                    let card_stroke = if is_active {
                                        egui::Stroke::new(2.0, theme.accent)
                                    } else {
                                        egui::Stroke::new(1.0, theme.border)
                                    };

                                    let (card_rect, card_resp) = ui.allocate_exact_size(
                                        egui::vec2(136.0, 110.0),
                                        egui::Sense::click(),
                                    );

                                    if card_resp.clicked() {
                                        apply_id = Some(theme.id.to_string());
                                    }

                                    let painter = ui.painter();

                                    // Card background
                                    painter.rect_filled(card_rect, egui::Rounding::same(8.0), card_fill);
                                    painter.rect_stroke(card_rect, egui::Rounding::same(8.0), card_stroke);

                                    // Mini window preview
                                    let preview_rect = egui::Rect::from_min_size(
                                        card_rect.min + egui::vec2(8.0, 8.0),
                                        egui::vec2(120.0, 44.0),
                                    );
                                    painter.rect_filled(preview_rect, egui::Rounding::same(5.0), theme.base);

                                    // Title bar
                                    let title_rect = egui::Rect::from_min_size(
                                        preview_rect.min,
                                        egui::vec2(120.0, 12.0),
                                    );
                                    painter.rect_filled(title_rect, egui::Rounding { nw: 5.0, ne: 5.0, sw: 0.0, se: 0.0 }, theme.crust);

                                    // Traffic light dots
                                    let dot_colors = [
                                        egui::Color32::from_rgb(243, 139, 168),
                                        egui::Color32::from_rgb(249, 226, 175),
                                        egui::Color32::from_rgb(166, 227, 161),
                                    ];
                                    for (d, &dot_color) in dot_colors.iter().enumerate() {
                                        painter.circle_filled(
                                            egui::pos2(title_rect.min.x + 6.0 + d as f32 * 9.0, title_rect.center().y),
                                            3.0,
                                            dot_color,
                                        );
                                    }

                                    // Color swatches
                                    let swatch_y = title_rect.max.y + 6.0;
                                    let swatch_colors = [theme.surface0, theme.surface1, theme.accent, theme.accent2, theme.text];
                                    for (s, &col) in swatch_colors.iter().enumerate() {
                                        painter.rect_filled(
                                            egui::Rect::from_min_size(
                                                egui::pos2(preview_rect.min.x + 6.0 + s as f32 * 22.0, swatch_y),
                                                egui::vec2(18.0, 18.0),
                                            ),
                                            egui::Rounding::same(4.0),
                                            col,
                                        );
                                    }

                                    // Accent color indicator dot + name
                                    painter.circle_filled(
                                        card_rect.min + egui::vec2(14.0, 65.0),
                                        4.0,
                                        theme.accent,
                                    );
                                    painter.text(
                                        card_rect.min + egui::vec2(24.0, 58.0),
                                        egui::Align2::LEFT_TOP,
                                        theme.name,
                                        egui::FontId::proportional(11.0),
                                        theme.text,
                                    );

                                    // Badge
                                    let badge_rect = egui::Rect::from_min_size(
                                        card_rect.min + egui::vec2(8.0, 80.0),
                                        egui::vec2(120.0, 22.0),
                                    );
                                    if is_active {
                                        painter.rect_filled(badge_rect, egui::Rounding::same(4.0), theme.accent);
                                        painter.text(badge_rect.center(), egui::Align2::CENTER_CENTER,
                                            "Ativo", egui::FontId::proportional(11.0), egui::Color32::from_rgb(17, 17, 27));
                                    } else {
                                        let hover_col = if card_resp.hovered() { theme.surface1 } else { theme.surface0 };
                                        painter.rect_filled(badge_rect, egui::Rounding::same(4.0), hover_col);
                                        painter.text(badge_rect.center(), egui::Align2::CENTER_CENTER,
                                            "Aplicar", egui::FontId::proportional(11.0), theme.text);
                                    }

                                    if (i + 1) % 4 == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
            });

        if let Some(id) = apply_id {
            let all = crate::themes::AppTheme::all();
            if let Some(theme) = all.iter().find(|t| t.id == id) {
                self.apply_theme(theme, ctx);
            }
        }
        if close {
            self.show_themes_modal = false;
        }
    }

    fn apply_theme(&mut self, theme: &crate::themes::AppTheme, ctx: &egui::Context) {
        self.config.theme_id = theme.id.to_string();
        self.config.salvar();
        crate::ui::setup_theme_visuals(ctx, theme);
    }
}
