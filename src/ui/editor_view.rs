use crate::app::PsychicEditorApp;
use crate::ui::code_view::CodeView;
use crate::ui::markdown_view::render_markdown_preview;
use egui::{vec2, Align, Color32, Id, Margin, Stroke};

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
        let markdown_preview = self.markdown_preview;
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
                let is_markdown = tab.syntax_name == "Markdown"
                    || tab
                        .path
                        .as_ref()
                        .and_then(|p| p.extension())
                        .and_then(|e| e.to_str())
                        .is_some_and(|ext| {
                            ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown")
                        });

                if markdown_preview && is_markdown {
                    // Split view: Editor na esquerda, Preview nativo na direita
                    let total_width = ui.available_width();
                    let editor_width = (total_width * 0.5).floor().max(200.0);
                    let preview_width = (total_width - editor_width - 1.0).max(160.0);

                    // Sincronização de scroll bidirecional:
                    // Se o usuário estiver digitando/navegando pelo teclado no editor, o editor comanda.
                    // Se o ponteiro/arraste estiver sobre o preview, o preview comanda.
                    // Caso contrário, o editor comanda.
                    let is_typing_or_nav = ctx.input(|i| {
                        i.events
                            .iter()
                            .any(|e| matches!(e, egui::Event::Key { .. } | egui::Event::Text(_)))
                    });

                    let hover_pos = ctx.input(|i| i.pointer.hover_pos());
                    let press_origin = ctx.input(|i| i.pointer.press_origin());
                    let is_pointer_down = ctx.input(|i| i.pointer.primary_down());

                    let sep_boundary_x = ui.available_rect_before_wrap().min.x + editor_width;
                    let preview_is_driver = if is_typing_or_nav {
                        false
                    } else if is_pointer_down {
                        press_origin.is_some_and(|p| p.x >= sep_boundary_x)
                    } else {
                        hover_pos.is_some_and(|p| p.x >= sep_boundary_x)
                    };

                    let mut code_out = None;
                    let mut preview_out = None;

                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;

                        let out = ui
                            .allocate_ui_with_layout(
                                vec2(editor_width, ui.available_height()),
                                egui::Layout::top_down(Align::Min),
                                |ui| {
                                    let mut code_view = CodeView {
                                        tab,
                                        theme_name: &theme_name,
                                        font_size,
                                        show_minimap,
                                        search_query: &search_query,
                                        search_case_sensitive,
                                        steal_focus_if_idle: !palette_is_open && !search_open,
                                    };
                                    code_view.show(ui)
                                },
                            )
                            .inner;

                        if out.cursor_changed && tab.cursor_line != prev_line {
                            cursor_line_changed = true;
                        }
                        if let Some(msg) = out.status_hint.as_ref() {
                            new_status = Some(msg.clone());
                        }

                        // Divisor vertical
                        let (sep_rect, _) = ui.allocate_exact_size(
                            vec2(1.0, ui.available_height()),
                            egui::Sense::hover(),
                        );
                        ui.painter().line_segment(
                            [sep_rect.left_top(), sep_rect.left_bottom()],
                            Stroke::new(1.0_f32, Color32::from_rgb(45, 48, 58)),
                        );

                        let target_preview_ratio = if preview_is_driver {
                            None
                        } else {
                            Some(out.scroll_ratio)
                        };

                        // Visualização Markdown
                        let p_out = ui
                            .allocate_ui_with_layout(
                                vec2(preview_width, ui.available_height()),
                                egui::Layout::top_down(Align::Min),
                                |ui| {
                                    let po = render_markdown_preview(
                                        ui,
                                        tab.id,
                                        &tab.content,
                                        &theme_name,
                                        font_size,
                                        target_preview_ratio,
                                    );
                                    if po.copied_html {
                                        new_status = Some(
                                            "HTML do Markdown copiado para a área de transferência!"
                                                .to_string(),
                                        );
                                    }
                                    po
                                },
                            )
                            .inner;

                        code_out = Some(out);
                        preview_out = Some(p_out);
                    });

                    if preview_is_driver {
                        if let (Some(code_o), Some(p_o)) = (code_out, preview_out) {
                            if (p_o.scroll_ratio - code_o.scroll_ratio).abs() > 0.002 {
                                let target_code_y = p_o.scroll_ratio * code_o.max_scroll_y;
                                ctx.data_mut(|d| {
                                    d.insert_temp(
                                        Id::new(("sync_code_scroll_y", tab.id)),
                                        target_code_y,
                                    );
                                });
                                ctx.request_repaint();
                            }
                        }
                    }
                } else {
                    // Editor em tela cheia
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
