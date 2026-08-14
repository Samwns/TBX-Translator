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
    }
}
