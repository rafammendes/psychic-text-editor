use crate::app::PsychicEditorApp;
use crate::icons::ICON_GEAR;
use crate::palette::{EditorCommand, PaletteMode};
use egui::{Align, Color32, CursorIcon, Layout, Margin, RichText, Stroke};

impl PsychicEditorApp {
    pub fn render_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(22, 24, 29))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(40, 44, 52)))
                    .inner_margin(Margin::symmetric(10, 3)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (line, col, sel, line_ending, syntax_name, total_lines) = self
                        .active_tab_ref()
                        .map(|t| {
                            (
                                t.cursor_line,
                                t.cursor_col,
                                t.selection_len,
                                t.line_ending,
                                t.syntax_name.clone(),
                                t.cached_lines(),
                            )
                        })
                        .unwrap_or((1, 1, 0, "LF", "Plain Text".to_string(), 1));

                    let pos_text = if sel > 0 {
                        format!(
                            "Line {}, Column {} ({} selected) • {} lines",
                            line, col, sel, total_lines
                        )
                    } else {
                        format!("Line {}, Column {} • {} lines", line, col, total_lines)
                    };

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(pos_text)
                                    .size(12.0)
                                    .color(Color32::from_rgb(190, 195, 205)),
                            )
                            .frame(false),
                        )
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text("Ir para linha (Ctrl+G)")
                        .clicked()
                    {
                        self.open_palette(PaletteMode::GotoLine, String::new());
                    }

                    ui.separator();
                    ui.add(
                        egui::Label::new(
                            RichText::new(&self.status_message)
                                .size(12.0)
                                .color(Color32::from_rgb(145, 155, 170)),
                        )
                        .selectable(false),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(format!("{} {}", ICON_GEAR, syntax_name))
                                        .size(12.0)
                                        .color(Color32::from_rgb(255, 175, 80)),
                                )
                                .frame(false),
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .on_hover_text("Clique para alterar a sintaxe")
                            .clicked()
                        {
                            self.open_palette(PaletteMode::SyntaxSelector, String::new());
                        }

                        ui.separator();

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(line_ending)
                                        .size(12.0)
                                        .color(Color32::from_rgb(180, 185, 195)),
                                )
                                .frame(false),
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .on_hover_text("Alternar quebra de linha LF / CRLF")
                            .clicked()
                        {
                            self.execute_command(EditorCommand::ToggleLineEnding);
                        }

                        ui.separator();
                        ui.add(
                            egui::Label::new(
                                RichText::new("UTF-8")
                                    .size(12.0)
                                    .color(Color32::from_rgb(165, 170, 180)),
                            )
                            .selectable(false),
                        );
                        ui.separator();
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("{:.0}pt", self.font_size))
                                    .size(12.0)
                                    .color(Color32::from_rgb(165, 170, 180)),
                            )
                            .selectable(false),
                        );
                    });
                });
            });
    }
}
