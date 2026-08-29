use crate::ui::TbxApp;
use egui::*;

use crate::ui::toggle_ui;
use crate::ui::t;
use std::path::PathBuf;

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
            let title = if self.engine_modal_single_mode {
                let engine_name = match self.engine_modal_tab {
                    0 => "Ren'Py",
                    1 => "Unity",
                    _ => "Godot",
                };
                format!("Configurações Extras: {}", engine_name)
            } else {
                "Configurações Extras dos Motores".to_string()
            };

            egui::Window::new(title.clone())
                .id(egui::Id::new("EngineModal_V2"))
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    // Custom Title Bar
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&title).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let close_btn = egui::Button::new(RichText::new(" X ").color(Color32::WHITE).strong())
                                .fill(Color32::from_rgb(237, 135, 150)) // Red background
                                .rounding(Rounding::same(4.0));
                            if ui.add(close_btn).clicked() {
                                self.show_engine_modal = false;
                            }
                        });
                    });
                    ui.separator();
                    ui.add_space(4.0);

                    if !self.engine_modal_single_mode {
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
                                .fill(r_bg).rounding(Rounding { nw: 6.0, sw: 6.0, ne: 0.0, se: 0.0 }).min_size(vec2(90.0, 26.0));
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
                                .fill(u_bg).rounding(Rounding::ZERO).min_size(vec2(90.0, 26.0));
                            if ui.add(u_btn).clicked() { self.engine_modal_tab = 1; }

                            let godot_active = self.engine_modal_tab == 2;
                            let g_anim = ctx.animate_bool_with_time(ui.id().with("g_tab"), godot_active, 0.2);
                            let g_bg = Color32::from_rgb(
                                (24.0 * (1.0 - g_anim) + 49.0 * g_anim) as u8,
                                (24.0 * (1.0 - g_anim) + 50.0 * g_anim) as u8,
                                (37.0 * (1.0 - g_anim) + 68.0 * g_anim) as u8,
                            );
                            let g_text_color = Color32::from_rgb(
                                (166.0 * (1.0 - g_anim) + 137.0 * g_anim) as u8,
                                (173.0 * (1.0 - g_anim) + 180.0 * g_anim) as u8,
                                (200.0 * (1.0 - g_anim) + 250.0 * g_anim) as u8,
                            );

                            let g_btn = Button::image_and_text(
                                egui::Image::new(egui::include_image!("../../assets/godot_icon.svg")).max_height(14.0),
                                RichText::new("Godot").color(g_text_color).strong(),
                            )
                                .fill(g_bg).rounding(Rounding { nw: 0.0, sw: 0.0, ne: 6.0, se: 6.0 }).min_size(vec2(90.0, 26.0));
                            if ui.add(g_btn).clicked() { self.engine_modal_tab = 2; }
                        });
                        ui.add_space(8.0);
                    }

                    // Fade animation for content change
                    let _fade = ctx.animate_bool_with_time(ui.id().with("tab_fade").with(self.engine_modal_tab), true, 0.2);

                    ui.allocate_ui(vec2(280.0, 0.0), |ui| {
                        ui.vertical(|ui| {
                            if self.engine_modal_tab == 0 {
                                ui.horizontal(|ui| {
                                    toggle_ui(ui, &mut self.config.manter_estrutura_original, "Manter estrutura original na pasta tl");
                                    if ui.add(Button::new(" (i) ").small().fill(Color32::from_rgb(49, 50, 68))).clicked() {
                                        self.show_info_modal = Some(("Manter Estrutura".into(), "Quando ativado, o tradutor recriará a estrutura original de pastas e arquivos (ex: script.rpy, screens.rpy) dentro da pasta 'tl'. Se desativado, o Ren'Py agrupará tudo em menos arquivos.".into()));
                                    }
                                });
                                ui.add_space(4.0);
                                toggle_ui(ui, &mut self.config.preservar_nomes_renpy, "Proteger variáveis [nome]");
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    toggle_ui(ui, &mut self.config.traduzir_nomes_personagens_renpy, "Traduzir nomes dos personagens");
                                    if ui.add(Button::new(" (i) ").small().fill(Color32::from_rgb(49, 50, 68))).clicked() {
                                        self.show_info_modal = Some(("Traduzir Nomes".into(), "Permite que os nomes dos personagens, definidos no Character(), sejam enviados para tradução (ex: 'Woodman' vira 'Lenhador'). Se desmarcado, os nomes serão sempre mantidos no original.".into()));
                                    }
                                });
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    toggle_ui(ui, &mut self.config.usar_traducao_pivo, "Tradução Pivô (Nativo -> Inglês -> Alvo)");
                                    if ui.add(Button::new(" (i) ").small().fill(Color32::from_rgb(49, 50, 68))).clicked() {
                                        self.show_info_modal = Some(("Tradução Pivô".into(), "A tradução pivô faz com que o texto seja primeiro traduzido para Inglês, e depois do Inglês para o idioma final. Isso geralmente resulta em traduções de melhor qualidade para idiomas difíceis (como Japonês ou Coreano).".into()));
                                    }
                                });
                                

                            } else if self.engine_modal_tab == 1 {
                                ui.label(RichText::new("Integração:").color(Color32::from_rgb(137, 180, 250)).strong());
                                ui.label("Injeção Dinâmica: Usa UnityPy para extração e BepInEx (Hooking em Memória) para injeção.");
                                ui.add_space(8.0);
                                ui.label(RichText::new("Compatibilidade:").color(Color32::from_rgb(137, 180, 250)).strong());
                                ui.label("Suporte universal a jogos Mono e IL2CPP (Mod BepInEx 6 Bleeding Edge)");
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    toggle_ui(ui, &mut self.config.usar_bepinex_6, "Instalar BepInEx 6 (Bleeding Edge)");
                                    if ui.add(egui::Button::new(" (i) ").small().fill(egui::Color32::from_rgb(49, 50, 68))).clicked() {
                                        self.show_info_modal = Some(("BepInEx 6".into(), "Recomendado apenas para Unity 2022+ ou quando o hook nativo do BepInEx 5 falha no Windows/Linux. Os arquivos do BepInEx 6 devem estar na pasta 'third_party' do diretório do tradutor.".into()));
                                    }
                                });
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    toggle_ui(ui, &mut self.config.substituir_todas_fontes_unity, "Substituir fonte do jogo todas as fontes");
                                    if ui.add(egui::Button::new(" (i) ").small().fill(egui::Color32::from_rgb(49, 50, 68))).clicked() {
                                        self.show_info_modal = Some(("Substituir Todas as Fontes".into(), "Se ativado, a fonte universal irá sobrescrever completamente todas as fontes do jogo. Se desativado (Padrão), a fonte universal funcionará apenas como um fallback para fornecer os acentos que faltam, preservando o estilo original do jogo.".into()));
                                    }
                                });
                            } else {
                                ui.label(RichText::new("Integração:").color(Color32::from_rgb(137, 180, 250)).strong());
                                ui.label("Extração de arquivos .pck, parsing de strings CSV e injetor automático.");
                                ui.add_space(8.0);
                                toggle_ui(ui, &mut self.config.usar_traducao_pivo, "Tradução Pivô (Nativo -> Inglês -> Alvo)");
                            }
                            
                            ui.add_space(12.0);
                            ui.separator();
                            ui.add_space(8.0);
                            
                            let tags_btn = egui::Button::new(RichText::new("Gerenciar Nomes e Tags Protegidas...").color(Color32::WHITE).strong())
                                .fill(ui.visuals().selection.bg_fill)
                                .min_size(vec2(ui.available_width(), 32.0))
                                .rounding(Rounding::same(6.0));
                            if ui.add(tags_btn).clicked() {
                                self.show_tags_modal = true;
                                // Try loading game tags if any game is selected
                                let mut game_tags_path = PathBuf::from(&self.game_path);
                                if game_tags_path.is_file() {
                                    game_tags_path = game_tags_path.parent().unwrap_or(&game_tags_path).to_path_buf();
                                }
                                if self.engine_mode == 0 {
                                    game_tags_path = game_tags_path.join("game").join("tl").join("tbx_tags.txt");
                                } else if self.engine_mode == 1 {
                                    game_tags_path = game_tags_path.join("TBX_Workspace").join("tbx_tags.txt");
                                } else {
                                    game_tags_path = game_tags_path.join("TBX_Workspace_Godot").join("tbx_tags.txt");
                                }
                                self.tags_jogo = std::fs::read_to_string(&game_tags_path).unwrap_or_default();
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
        self.render_tags_modal(ctx);
        self.render_info_modal(ctx);
        self.render_inject_modal(ctx);
        self.render_create_patch_modal(ctx);
    }
}

impl TbxApp {
    pub fn render_themes_modal(&mut self, ctx: &egui::Context) {
        if !self.show_themes_modal {
            return;
        }

        let themes = crate::themes::AppTheme::all();
        let current_id = self.config.theme_id.clone();

        let mut apply_id: Option<String> = None;

        egui::Window::new("Temas de Cores")
            .id(egui::Id::new("themes_modal"))
            .title_bar(false)
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
                        let close_btn = egui::Button::new(RichText::new(" X ").color(Color32::WHITE).strong())
                            .fill(Color32::from_rgb(237, 135, 150)) // Red background
                            .rounding(Rounding::same(4.0));
                        if ui.add(close_btn).clicked() {
                            self.show_themes_modal = false;
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
    }

    fn apply_theme(&mut self, theme: &crate::themes::AppTheme, ctx: &egui::Context) {
        self.config.theme_id = theme.id.to_string();
        self.config.salvar();
        crate::ui::setup_theme_visuals(ctx, theme);
    }

    pub fn render_info_modal(&mut self, ctx: &egui::Context) {
        let mut close = false;
        if let Some((title, desc)) = &self.show_info_modal {
            egui::Window::new(title.clone())
                .id(egui::Id::new("InfoModal"))
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.label(RichText::new(desc).color(Color32::from_rgb(205, 214, 244)));
                    ui.add_space(8.0);
                    if ui.button("Entendi").clicked() {
                        close = true;
                    }
                });
        }
        if close {
            self.show_info_modal = None;
        }
    }

    pub fn render_tags_modal(&mut self, ctx: &egui::Context) {
        if !self.show_tags_modal { return; }

        let mut close = false;
        egui::Window::new("Gerenciador de Nomes e Tags")
            .id(egui::Id::new("tags_modal"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Nomes e Tags Protegidas").strong().size(15.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let close_btn = egui::Button::new(RichText::new(" X ").color(Color32::WHITE).strong())
                            .fill(Color32::from_rgb(237, 135, 150))
                            .rounding(Rounding::same(4.0));
                        if ui.add(close_btn).clicked() {
                            close = true;
                        }
                    });
                });
                ui.separator();
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tags_modal_tab, 0, "Padrão (Global)");
                    ui.selectable_value(&mut self.tags_modal_tab, 1, "Personalizada");
                    ui.selectable_value(&mut self.tags_modal_tab, 2, "Jogo Específico");
                });
                ui.separator();
                ui.add_space(6.0);

                if self.tags_modal_tab == 0 {
                    ui.label(RichText::new("Tags padrão (esta lista é baseada no motor selecionado e não é modificável).").small());
                    ui.add_space(4.0);
                    let mut default_tags = match self.engine_mode {
                        0 => "[name]\n[player_name]\n{b}\n{/b}\n{i}\n{/i}\n{u}\n{/u}\n{color}\n{/color}\n{w}\n{p}\n{nw}".to_string(),
                        1 => "<b>\n</b>\n<i>\n</i>\n<u>\n</u>\n<color>\n</color>\n<size>\n</size>".to_string(),
                        2 => "[b]\n[/b]\n[i]\n[/i]\n[u]\n[/u]\n[color]\n[/color]".to_string(),
                        _ => String::new(),
                    };
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        ui.add_enabled(false, egui::TextEdit::multiline(&mut default_tags).desired_width(ui.available_width()));
                    });
                } else if self.tags_modal_tab == 1 {
                    ui.horizontal(|ui| {
                        toggle_ui(ui, &mut self.config.usa_tags_personalizadas, "Ativar Tags Personalizadas");
                    });
                    ui.label(RichText::new("Sua lista global de tags. Aplicada em todos os jogos.").small());
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        ui.add(egui::TextEdit::multiline(&mut self.tags_personalizadas).desired_width(ui.available_width()));
                    });
                } else {
                    ui.horizontal(|ui| {
                        toggle_ui(ui, &mut self.config.usa_tags_jogo, "Ativar Tags do Jogo");
                    });
                    ui.label(RichText::new("Lista salva na pasta do jogo selecionado atualmente.").small());
                    if self.game_path.is_empty() {
                        ui.colored_label(Color32::from_rgb(243, 139, 168), "Nenhum jogo selecionado na aba 'Traduzir'!");
                    } else {
                        ui.horizontal(|ui| {
                            if ui.button("Escanear (.rpy)").clicked() {
                                // Escanear .rpy files no game path
                                let mut names = Vec::new();
                                if let Ok(re) = regex::Regex::new(r#"Character\s*\(\s*["']([^"']+)["']"#) {
                                    let mut search_path = PathBuf::from(&self.game_path);
                                    if search_path.is_file() {
                                        search_path = search_path.parent().unwrap_or(&search_path).to_path_buf();
                                    }
                                    if self.engine_mode == 0 {
                                        search_path = search_path.join("game");
                                    }
                                    
                                    if search_path.exists() {
                                        for entry in walkdir::WalkDir::new(&search_path).into_iter().flatten() {
                                            if entry.path().extension().map_or(false, |e| e == "rpy") {
                                                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                                                    for cap in re.captures_iter(&content) {
                                                        names.push(cap[1].to_string());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                names.sort();
                                names.dedup();
                                
                                let mut current_tags: Vec<String> = self.tags_jogo.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                                for name in names {
                                    if !current_tags.contains(&name) {
                                        current_tags.push(name);
                                    }
                                }
                                self.tags_jogo = current_tags.join("\n");
                            }
                            
                            if ui.button("Importar").clicked() {
                                if let Some(path) = rfd::FileDialog::new().add_filter("Text", &["txt"]).pick_file() {
                                    if let Ok(content) = std::fs::read_to_string(path) {
                                        self.tags_jogo = content;
                                    }
                                }
                            }
                            
                            if ui.button("Exportar").clicked() {
                                if let Some(path) = rfd::FileDialog::new().add_filter("Text", &["txt"]).save_file() {
                                    let _ = std::fs::write(path, &self.tags_jogo);
                                }
                            }
                        });

                        ui.add_space(4.0);
                        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                            ui.add(egui::TextEdit::multiline(&mut self.tags_jogo).desired_width(ui.available_width()));
                        });
                    }
                }
            });

        if close {
            self.show_tags_modal = false;
            // Save custom tags when closing
            let _ = std::fs::write("tags_personalizadas.txt", &self.tags_personalizadas);
            
            // Save game tags
            if !self.game_path.is_empty() {
                let mut game_tags_path = PathBuf::from(&self.game_path);
                if game_tags_path.is_file() {
                    game_tags_path = game_tags_path.parent().unwrap_or(&game_tags_path).to_path_buf();
                }
                if self.engine_mode == 0 {
                    game_tags_path = game_tags_path.join("game").join("tl").join("tbx_tags.txt");
                } else if self.engine_mode == 1 {
                    game_tags_path = game_tags_path.join("TBX_Workspace").join("tbx_tags.txt");
                } else {
                    game_tags_path = game_tags_path.join("TBX_Workspace_Godot").join("tbx_tags.txt");
                }
                if let Some(parent) = game_tags_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(game_tags_path, &self.tags_jogo);
            }
            self.config.salvar();
        }
    }

    pub fn render_inject_modal(&mut self, ctx: &egui::Context) {
        let mut close = false;
        let mut do_inject_dynamic = false;
        let mut do_inject_static = false;

        if self.show_inject_modal {
            egui::Window::new("Opções de Injeção")
                .id(egui::Id::new("InjectModal"))
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Escolha o método de injeção da tradução:").strong().size(15.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let close_btn = egui::Button::new(RichText::new(" X ").color(Color32::WHITE).strong())
                                .fill(Color32::from_rgb(237, 135, 150))
                                .rounding(Rounding::same(4.0));
                            if ui.add(close_btn).clicked() {
                                close = true;
                            }
                        });
                    });
                    ui.separator();
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.use_dynamic_inject, true, "Dinâmica (BepInEx)");
                        ui.label(RichText::new("ℹ").color(Color32::from_rgb(137, 180, 250)))
                            .on_hover_text("Recomendado. Usa BepInEx para traduzir o jogo em tempo real na memória. Mais seguro.");
                    });
                    
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.use_dynamic_inject, false, "Estática (Nativo)");
                        ui.label(RichText::new("ℹ").color(Color32::from_rgb(137, 180, 250)))
                            .on_hover_text("Usa UnityPy para modificar os arquivos originais do jogo. Útil quando o BepInEx não funciona.");
                    });

                    ui.add_space(16.0);
                    
                    ui.horizontal(|ui| {
                        let btn = egui::Button::new(RichText::new("INJETAR").strong().color(Color32::WHITE))
                            .fill(ui.visuals().selection.bg_fill)
                            .min_size(egui::vec2(100.0, 30.0));
                        if ui.add(btn).clicked() {
                            if self.use_dynamic_inject {
                                do_inject_dynamic = true;
                            } else {
                                do_inject_static = true;
                            }
                            close = true;
                        }
                    });
                });
        }

        if close {
            self.show_inject_modal = false;
        }
        
        if do_inject_dynamic {
            self.start_unity_dynamic_inject();
        } else if do_inject_static {
            self.start_unity_inject();
        }
    }

    pub fn render_create_patch_modal(&mut self, ctx: &egui::Context) {
        let mut close = false;
        let mut confirm = false;

        if self.show_create_patch_modal {
            egui::Window::new("Criar Patch")
                .id(egui::Id::new("CreatePatchModal"))
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Image::new(egui::include_image!("../../assets/language_icon.svg")).max_size(egui::vec2(20.0, 20.0)));
                        ui.label(RichText::new("Criar patch de tradução").strong().size(15.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let close_btn = egui::Button::new(RichText::new(" X ").color(Color32::WHITE).strong())
                                .fill(Color32::from_rgb(237, 135, 150))
                                .rounding(Rounding::same(4.0));
                            if ui.add(close_btn).clicked() {
                                close = true;
                            }
                        });
                    });
                    ui.separator();
                    ui.add_space(8.0);

                    // Método de injeção
                    // Ren'Py não tem injetor — o patch é sempre o tl/<idioma>. Esconde a pergunta.
                    if self.engine_mode != 0 {
                        ui.label("Método de injeção:");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.patch_method_embed, true, "Completo (com injetor)");
                            ui.label(RichText::new("ℹ").color(Color32::from_rgb(137, 180, 250)))
                                .on_hover_text("Inclui tudo que o usuário precisa: BepInEx + plugin (Unity) ou o .exe já patchado (Godot).");
                        });
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.patch_method_embed, false, "Somente arquivos de tradução");
                            ui.label(RichText::new("ℹ").color(Color32::from_rgb(137, 180, 250)))
                                .on_hover_text("Apenas os arquivos traduzidos, para quem já tem o injetor instalado.");
                        });
                        ui.add_space(12.0);
                    }

                    // Pasta de destino
                    ui.label("Salvar em:");
                    ui.horizontal(|ui| {
                        let mut txt = self.patch_dest_dir.clone();
                        if ui.add(egui::TextEdit::singleline(&mut txt).desired_width(280.0)).changed() {
                            self.patch_dest_dir = txt;
                        }
                        if ui.button("📂").on_hover_text("Escolher pasta...").clicked() {
                            if let Ok(Some(p)) = crate::ui::dialogs::pick_folder("Pasta de destino do patch") {
                                self.patch_dest_dir = p.to_string_lossy().to_string();
                            }
                        }
                    });

                    ui.add_space(12.0);

                    // Formato
                    ui.label("Formato:");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.patch_as_zip, true, "Arquivo .zip");
                        ui.radio_value(&mut self.patch_as_zip, false, "Pasta normal");
                    });

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        let btn = egui::Button::new(RichText::new("CRIAR PATCH").strong().color(Color32::WHITE))
                            .fill(ui.visuals().selection.bg_fill)
                            .min_size(egui::vec2(120.0, 30.0));
                        if ui.add(btn).clicked() {
                            confirm = true;
                            close = true;
                        }
                    });
                });
        }

        if close {
            self.show_create_patch_modal = false;
        }
        if confirm {
            self.start_create_patch();
        }
    }
}
