use crate::app::PsychicEditorApp;
use crate::ui::code_view::CodeView;
use egui::{Color32, Margin, Stroke};

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
        let show_minimap = self.show_minimap;
        let palette_is_open = self.palette_mode != crate::palette::PaletteMode::Closed;
        let search_open = self.search_open;
        let search_query = if search_open {
            self.search_query.clone()
        } else {
            String::new()
        };
        let search_case_sensitive = self.search_case_sensitive;

        let mut cursor_line_changed = false;
        let mut new_status: Option<String> = None;

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

                let prev_line = tab.cursor_line;
                let mut code_view = CodeView {
                    tab,
                    theme_name: &theme_name,
                    font_size,
                    show_minimap,
                    search_query: &search_query,
                    search_case_sensitive,
                    steal_focus_if_idle: !palette_is_open && !search_open,
                };

                let out = code_view.show(ui);
                if out.cursor_changed && tab.cursor_line != prev_line {
                    cursor_line_changed = true;
                }
                if let Some(msg) = out.status_hint {
                    new_status = Some(msg);
                }
            });

        if let Some(msg) = new_status {
            self.status_message = msg;
        }
        if cursor_line_changed {
            self.save_session_state();
        }
    }
}
