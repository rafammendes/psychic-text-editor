use crate::app::PsychicEditorApp;
use crate::icons::{ICON_CLOSE, ICON_SEARCH};
use egui::{Align, Color32, Key, Layout, Margin, RichText, Stroke};

impl PsychicEditorApp {
    pub fn render_search_bar(&mut self, ctx: &egui::Context) {
        if !self.search_open {
            return;
        }

        egui::TopBottomPanel::bottom("search_panel")
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(30, 33, 40))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(55, 60, 72)))
                    .inner_margin(Margin::symmetric(10, 6)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("{} Find:", ICON_SEARCH))
                                .size(13.0)
                                .color(Color32::from_rgb(200, 205, 215)),
                        )
                        .selectable(false),
                    );
                    let find_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .desired_width(220.0)
                            .hint_text("Buscar texto..."),
                    );
                    if self.request_search_focus {
                        find_resp.request_focus();
                        self.request_search_focus = false;
                    }
                    if find_resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        self.find_next_in_active_tab();
                        find_resp.request_focus();
                    }

                    if ui.button("Find Next").clicked() {
                        self.find_next_in_active_tab();
                    }

                    ui.checkbox(&mut self.search_case_sensitive, "Aa");

                    ui.separator();

                    ui.add(
                        egui::Label::new(
                            RichText::new("Replace:")
                                .size(13.0)
                                .color(Color32::from_rgb(200, 205, 215)),
                        )
                        .selectable(false),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.replace_query)
                            .desired_width(180.0)
                            .hint_text("Substituir por..."),
                    );
                    if ui.button("Replace All").clicked() {
                        self.replace_all_in_active_tab();
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button(ICON_CLOSE).clicked() {
                            self.search_open = false;
                        }
                    });
                });
            });
    }
}
