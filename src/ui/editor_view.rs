use crate::app::PsychicEditorApp;
use crate::highlighter::{layout_code_galley, layout_gutter_galley};
use egui::{Color32, FontFamily, FontId, Margin, Stroke};

impl PsychicEditorApp {
    pub fn render_sidebar(&mut self, ctx: &egui::Context) {
        if !self.sidebar_open {
            return;
        }

        let active_path = self.active_tab_ref().and_then(|t| t.path.clone());
        let mut file_to_open = None;

        egui::SidePanel::left("workspace_sidebar")
            .resizable(true)
            .default_width(240.0)
            .width_range(160.0..=450.0)
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(28, 30, 36))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(40, 44, 52)))
                    .inner_margin(Margin::symmetric(8, 8)),
            )
            .show(ctx, |ui| {
                file_to_open = self.workspace.ui(ui, active_path.as_deref());
            });

        if let Some(path) = file_to_open {
            self.open_file_path(path);
        }
    }

    pub fn render_editor_central(&mut self, ctx: &egui::Context) {
        let theme_name = self.theme_name.clone();
        let font_size = self.font_size;
        let word_wrap = self.word_wrap;
        let palette_is_open = self.palette_mode != crate::palette::PaletteMode::Closed;

        let mut cursor_line_changed = false;

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(35, 38, 46))
                    .inner_margin(Margin::symmetric(0, 0)),
            )
            .show(ctx, |ui| {
                let Some(tab) = self.tabs.get_mut(self.active_tab) else {
                    return;
                };

                let syntax_name = tab.syntax_name.clone();
                let line_count = tab.line_count();
                let current_line = tab.cursor_line;

                egui::ScrollArea::both()
                    .id_salt(("editor_scroll", tab.id))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;

                            // Line Numbers Gutter (Sublime style) com Galley em cache
                            egui::Frame::NONE
                                .fill(Color32::from_rgb(31, 34, 41))
                                .inner_margin(Margin {
                                    left: 6,
                                    right: 6,
                                    top: 8,
                                    bottom: 8,
                                })
                                .show(ui, |ui| {
                                    let gutter_galley = layout_gutter_galley(
                                        ui,
                                        line_count,
                                        current_line,
                                        font_size,
                                    );
                                    ui.add(egui::Label::new(gutter_galley).selectable(false));
                                });

                            // Code Area com lexer nativo zero-regex + Galley ring-cache
                            let mut layouter =
                                move |ui: &egui::Ui, string: &str, wrap_width: f32| {
                                    let max_w = if word_wrap {
                                        wrap_width
                                    } else {
                                        f32::INFINITY
                                    };
                                    layout_code_galley(
                                        ui,
                                        string,
                                        &syntax_name,
                                        &theme_name,
                                        font_size,
                                        max_w,
                                    )
                                };

                            let editor_id = egui::Id::new(("code_editor_area", tab.id));
                            let desired_width = if word_wrap {
                                ui.available_width().max(200.0)
                            } else {
                                ui.available_width().max(2000.0)
                            };

                            let output = egui::Frame::NONE
                                .inner_margin(Margin {
                                    left: 8,
                                    right: 12,
                                    top: 8,
                                    bottom: 8,
                                })
                                .show(ui, |ui| {
                                    egui::TextEdit::multiline(&mut tab.content)
                                        .id(editor_id)
                                        .font(FontId::new(font_size, FontFamily::Monospace))
                                        .code_editor()
                                        .desired_rows(32)
                                        .desired_width(desired_width)
                                        .lock_focus(true)
                                        .frame(false)
                                        .layouter(&mut layouter)
                                        .show(ui)
                                })
                                .inner;

                            if output.response.changed() {
                                tab.mark_content_edited();
                            }

                            // Aplica pulo de cursor pendente (Goto Line / Find / Restauração de sessão)
                            if let Some((start_char, end_char)) = tab.pending_cursor_char.take() {
                                let mut state = output.state;
                                let ccursor_range = egui::text::CCursorRange::two(
                                    egui::text::CCursor::new(start_char),
                                    egui::text::CCursor::new(end_char),
                                );
                                state.cursor.set_char_range(Some(ccursor_range));
                                state.store(ui.ctx(), editor_id);
                                if !palette_is_open {
                                    output.response.request_focus();
                                }

                                // Rola suavemente até a linha do cursor restaurado/buscado
                                let cursor = output
                                    .galley
                                    .from_ccursor(egui::text::CCursor::new(start_char));
                                let cursor_rect = output.galley.pos_from_cursor(&cursor);
                                let global_rect = cursor_rect
                                    .translate(output.galley_pos.to_vec2())
                                    .expand(24.0);
                                ui.scroll_to_rect(global_rect, Some(egui::Align::Center));
                            } else if let Some(range) = output.cursor_range {
                                let prev_line = tab.cursor_line;
                                let primary = range.primary.ccursor.index;
                                let secondary = range.secondary.ccursor.index;
                                let sel_len = primary.abs_diff(secondary);
                                tab.update_cursor_stats(primary, sel_len);
                                if tab.cursor_line != prev_line {
                                    cursor_line_changed = true;
                                }
                            }
                        });
                    });
            });

        if cursor_line_changed {
            self.save_session_state();
        }
    }
}
