use crate::app::PsychicEditorApp;
use crate::icons::{
    ICON_BOLT, ICON_CLOSE, ICON_FOLDER_OPEN, ICON_PLUS, ICON_SAVE, ICON_SEARCH,
};
use crate::palette::{EditorCommand, PaletteMode};
use egui::{Color32, CornerRadius, CursorIcon, Margin, RichText, Sense, Stroke};

impl PsychicEditorApp {
    pub fn render_top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_chrome")
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(24, 26, 31))
                    .inner_margin(Margin::symmetric(10, 5)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Botão da Sidebar
                    let sb_color = if self.sidebar_open {
                        Color32::from_rgb(255, 165, 60)
                    } else {
                        Color32::from_rgb(160, 165, 175)
                    };
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(ICON_FOLDER_OPEN).size(16.5).color(sb_color),
                            )
                            .frame(false),
                        )
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text("Alternar Sidebar (Ctrl+B)")
                        .clicked()
                    {
                        self.execute_command(EditorCommand::ToggleSidebar);
                    }

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Botões de ação com ícones maiores e espaçamento entre eles
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(ICON_PLUS)
                                    .size(16.0)
                                    .color(Color32::from_rgb(200, 205, 215)),
                            )
                            .frame(false),
                        )
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text("Novo Arquivo (Ctrl+N)")
                        .clicked()
                    {
                        self.execute_command(EditorCommand::NewTab);
                    }

                    ui.add_space(6.0);

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(ICON_SAVE)
                                    .size(16.0)
                                    .color(Color32::from_rgb(200, 205, 215)),
                            )
                            .frame(false),
                        )
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text("Salvar (Ctrl+S)")
                        .clicked()
                    {
                        self.execute_command(EditorCommand::Save);
                    }

                    ui.add_space(6.0);

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(ICON_SEARCH)
                                    .size(16.0)
                                    .color(Color32::from_rgb(200, 205, 215)),
                            )
                            .frame(false),
                        )
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text("Buscar e Substituir (Ctrl+F)")
                        .clicked()
                    {
                        self.execute_command(EditorCommand::ToggleSearch);
                    }

                    ui.add_space(6.0);

                    // Apenas o símbolo do raio para o Command Palette
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(ICON_BOLT)
                                    .size(16.5)
                                    .color(Color32::from_rgb(255, 175, 70)),
                            )
                            .frame(false),
                        )
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text("Command Palette (Ctrl+Shift+P) / Goto Anything (Ctrl+P)")
                        .clicked()
                    {
                        self.open_palette(PaletteMode::CommandPalette, String::new());
                    }

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Barra de abas rolável sem exibir a barra de rolagem horizontal
                    egui::ScrollArea::horizontal()
                        .id_salt("tabs_scroll")
                        .scroll_bar_visibility(
                            egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                        )
                        .show(ui, |ui| {
                            if ui.rect_contains_pointer(ui.clip_rect()) {
                                let dy = ui.input(|i| i.smooth_scroll_delta.y);
                                if dy.abs() > 0.0 {
                                    ui.scroll_with_delta(egui::vec2(dy, 0.0));
                                }
                            }

                            let mut tab_to_close = None;
                            let mut tab_to_select = None;

                            for (idx, tab) in self.tabs.iter().enumerate() {
                                let is_active = idx == self.active_tab;
                                let bg = if is_active {
                                    Color32::from_rgb(39, 42, 50)
                                } else {
                                    Color32::from_rgb(29, 31, 37)
                                };

                                let frame = egui::Frame::NONE
                                    .fill(bg)
                                    .corner_radius(CornerRadius {
                                        nw: 5,
                                        ne: 5,
                                        sw: 0,
                                        se: 0,
                                    })
                                    .stroke(if is_active {
                                        Stroke::new(1.0_f32, Color32::from_rgb(255, 155, 50))
                                    } else {
                                        Stroke::new(1.0_f32, Color32::from_rgb(42, 45, 54))
                                    })
                                    .inner_margin(Margin::symmetric(10, 5));

                                frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(tab.icon)
                                                    .size(13.0)
                                                    .color(tab.icon_color),
                                            )
                                            .selectable(false),
                                        )
                                        .on_hover_cursor(CursorIcon::Default);

                                        let label_text = if tab.is_dirty() {
                                            format!("{} •", tab.title)
                                        } else {
                                            tab.title.clone()
                                        };
                                        let text_color = if is_active {
                                            Color32::WHITE
                                        } else {
                                            Color32::from_rgb(170, 175, 185)
                                        };

                                        let resp = ui
                                            .add(
                                                egui::Label::new(
                                                    RichText::new(label_text)
                                                        .size(13.0)
                                                        .color(text_color),
                                                )
                                                .selectable(false)
                                                .sense(Sense::click()),
                                            )
                                            .on_hover_cursor(CursorIcon::Default);

                                        if resp.clicked() {
                                            tab_to_select = Some(idx);
                                        }
                                        if resp.middle_clicked() {
                                            tab_to_close = Some(idx);
                                        }

                                        let close_resp = ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new(ICON_CLOSE)
                                                        .size(10.5)
                                                        .color(Color32::from_rgb(140, 145, 155)),
                                                )
                                                .frame(false),
                                            )
                                            .on_hover_cursor(CursorIcon::Default)
                                            .on_hover_text("Fechar aba (Ctrl+W)");

                                        if close_resp.clicked() {
                                            tab_to_close = Some(idx);
                                        }
                                    });
                                });
                            }

                            if let Some(idx) = tab_to_select {
                                self.select_tab(idx);
                            }
                            if let Some(idx) = tab_to_close {
                                self.close_tab(idx);
                            }
                        });
                });
            });
    }
}
