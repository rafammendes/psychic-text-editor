use crate::app::{expand_tilde, PsychicEditorApp};
use crate::highlighter::list_syntaxes;
use crate::icons::{file_icon_and_color, ICON_FOLDER_OPEN};
use crate::palette::{built_in_commands, fuzzy_match_score, EditorCommand, PaletteMode};
use egui::{
    Align, Color32, CornerRadius, CursorIcon, FontFamily, FontId, Key, Layout, Margin, RichText,
    Sense, Stroke, Vec2,
};
use std::path::{Path, PathBuf};

impl PsychicEditorApp {
    pub fn render_palette_modal(&mut self, ctx: &egui::Context) {
        if self.palette_mode == PaletteMode::Closed {
            return;
        }

        let mut should_close_outside = false;

        let win_resp = egui::Window::new("command_palette_popup")
            .id(egui::Id::new("psychic_command_palette_window"))
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_TOP, Vec2::new(0.0, 42.0))
            .fixed_size(Vec2::new(640.0, 350.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(Color32::from_rgb(28, 31, 38))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(8))
                    .stroke(Stroke::new(0.5_f32, Color32::from_rgb(68, 75, 90))),
            )
            .show(ctx, |ui| {
                let mut down_pressed = false;
                while ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowDown)) {
                    down_pressed = true;
                }
                let mut up_pressed = false;
                while ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowUp)) {
                    up_pressed = true;
                }
                let mut page_down = false;
                while ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::PageDown)) {
                    page_down = true;
                }
                let mut page_up = false;
                while ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::PageUp)) {
                    page_up = true;
                }
                let mut tab_pressed = false;
                while ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Tab)) {
                    tab_pressed = true;
                }
                let enter_pressed = ui.input(|i| i.key_pressed(Key::Enter));

                // Mantém o foco travado no input durante toda a vida do Command Palette
                let input_id = egui::Id::new("psychic_palette_query_input");
                ui.memory_mut(|m| m.request_focus(input_id));

                let prev_query = self.palette_query.clone();
                let input_resp = egui::Frame::NONE
                    .fill(Color32::from_rgb(21, 23, 29))
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::symmetric(8, 6))
                    .stroke(Stroke::new(0.5_f32, Color32::from_rgb(62, 68, 82)))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.palette_query)
                                .id(input_id)
                                .desired_width(f32::INFINITY)
                                .font(FontId::new(14.5, FontFamily::Monospace))
                                .frame(false)
                                .lock_focus(true)
                                .hint_text(match self.palette_mode {
                                    PaletteMode::CommandPalette => {
                                        "Digite um comando (ex: save, theme, syntax, duplicate)..."
                                    }
                                    PaletteMode::QuickOpen => {
                                        "Digite o nome do arquivo ou :42 para pular para a linha..."
                                    }
                                    PaletteMode::GotoLine => "Número da linha (ex: 120)...",
                                    PaletteMode::SyntaxSelector => {
                                        "Filtrar linguagem (ex: Rust, TOML, Shell, JSON)..."
                                    }
                                    PaletteMode::OpenPath
                                    | PaletteMode::SaveAsPath
                                    | PaletteMode::OpenFolderPath => {
                                        "/caminho/do/arquivo (Tab autocompleta, Enter confirma)..."
                                    }
                                    PaletteMode::Closed => "",
                                }),
                        )
                    })
                    .inner;

                // Trava setas verticais, horizontais e Tab no input para o egui nunca mover o foco
                input_resp.request_focus();
                ui.memory_mut(|m| {
                    m.set_focus_lock_filter(
                        input_id,
                        egui::EventFilter {
                            tab: true,
                            horizontal_arrows: true,
                            vertical_arrows: true,
                            escape: false,
                        },
                    );
                });

                let query_changed = self.palette_query != prev_query;
                if query_changed {
                    self.palette_selected = 0;
                    self.palette_scroll_to_selected = true;
                }

                self.request_palette_focus = false;

                if down_pressed || up_pressed || page_down || page_up {
                    self.palette_scroll_to_selected = true;
                }

                let nav = PaletteNavInput {
                    down_pressed,
                    up_pressed,
                    page_down,
                    page_up,
                    tab_pressed,
                    enter_pressed,
                };

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);

                match self.palette_mode.clone() {
                    PaletteMode::CommandPalette => self.render_command_list(ui, nav),
                    PaletteMode::QuickOpen => self.render_quick_open_list(ui, nav),
                    PaletteMode::GotoLine => self.render_goto_line_prompt(ui, nav),
                    PaletteMode::SyntaxSelector => self.render_syntax_selector_list(ui, nav),
                    PaletteMode::OpenPath
                    | PaletteMode::SaveAsPath
                    | PaletteMode::OpenFolderPath => self.render_path_prompt(ui, nav),
                    PaletteMode::Closed => {}
                }
            });

        if self.palette_just_opened {
            self.palette_just_opened = false;
        } else if let Some(inner) = win_resp {
            if inner.response.clicked_elsewhere() {
                should_close_outside = true;
            }
        }

        if should_close_outside {
            self.close_palette();
        }
    }

    fn apply_keyboard_navigation(&mut self, nav: PaletteNavInput, item_count: usize) {
        if item_count == 0 {
            self.palette_selected = 0;
            return;
        }
        if self.palette_selected >= item_count {
            self.palette_selected = item_count - 1;
        }
        if nav.down_pressed {
            self.palette_selected = (self.palette_selected + 1) % item_count;
        } else if nav.up_pressed {
            self.palette_selected = self
                .palette_selected
                .checked_sub(1)
                .unwrap_or(item_count - 1);
        } else if nav.page_down {
            self.palette_selected = (self.palette_selected + 8).min(item_count - 1);
        } else if nav.page_up {
            self.palette_selected = self.palette_selected.saturating_sub(8);
        }
    }

    fn render_command_list(&mut self, ui: &mut egui::Ui, nav: PaletteNavInput) {
        let cmds = built_in_commands();
        let mut scored: Vec<(i32, usize)> = cmds
            .iter()
            .enumerate()
            .filter_map(|(idx, c)| {
                let full_text = format!("{}: {}", c.category, c.label);
                fuzzy_match_score(&self.palette_query, &full_text).map(|score| (score, idx))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));

        self.apply_keyboard_navigation(nav, scored.len());

        let mut chosen_cmd = None;
        if nav.enter_pressed {
            if let Some((_, cmd_idx)) = scored.get(self.palette_selected) {
                chosen_cmd = Some(cmds[*cmd_idx].command.clone());
            }
        }

        let scroll_to_sel = self.palette_scroll_to_selected;
        self.palette_scroll_to_selected = false;

        egui::ScrollArea::vertical()
            .id_salt("palette_cmd_scroll")
            .max_height(280.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (list_idx, (_, cmd_idx)) in scored.iter().enumerate() {
                    let item = &cmds[*cmd_idx];
                    let is_sel = list_idx == self.palette_selected;

                    let row_resp = render_palette_row(ui, is_sel, |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("{}:", item.category))
                                    .size(12.5)
                                    .color(Color32::from_rgb(255, 165, 70)),
                            )
                            .selectable(false),
                        );
                        ui.add(
                            egui::Label::new(
                                RichText::new(&item.label)
                                    .size(13.5)
                                    .color(if is_sel {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(215, 220, 230)
                                    }),
                            )
                            .selectable(false),
                        );

                        if !item.shortcut.is_empty() {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(item.shortcut)
                                            .size(12.0)
                                            .color(Color32::from_rgb(160, 170, 185)),
                                    )
                                    .selectable(false),
                                );
                            });
                        }
                    });

                    if is_sel && scroll_to_sel {
                        row_resp.scroll_to_me(None);
                    }
                    if row_resp.clicked() {
                        chosen_cmd = Some(item.command.clone());
                    }
                }
            });

        if let Some(cmd) = chosen_cmd {
            self.close_palette();
            self.execute_command(cmd);
        }
    }

    fn render_quick_open_list(&mut self, ui: &mut egui::Ui, nav: PaletteNavInput) {
        let query = self.palette_query.trim().to_string();
        if let Some(line_str) = query.strip_prefix(':') {
            let max_lines = self
                .active_tab_ref()
                .map(|t| t.cached_lines())
                .unwrap_or(1);
            ui.add(
                egui::Label::new(
                    RichText::new(format!(
                        "Pressione Enter para pular para a linha (1..={})",
                        max_lines
                    ))
                    .color(Color32::from_rgb(180, 185, 195)),
                )
                .selectable(false),
            );
            if nav.enter_pressed {
                if let Ok(line_num) = line_str.trim().parse::<usize>() {
                    if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                        tab.set_cursor_line_col(line_num, 1);
                    }
                    self.save_session_state();
                    self.close_palette();
                }
            }
            return;
        }

        let (file_part, line_part) = if let Some((f, l)) = query.split_once(':') {
            (f, l.parse::<usize>().ok())
        } else {
            (query.as_str(), None)
        };

        let root = self.workspace.root.clone();
        let open_paths: Vec<PathBuf> = self
            .tabs
            .iter()
            .filter_map(|t| t.path.clone())
            .collect();

        let mut scored: Vec<(i32, PathBuf, String, bool)> = self
            .workspace
            .indexed_files
            .iter()
            .filter_map(|p| {
                let rel = p
                    .strip_prefix(&root)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .to_string();
                let is_open = open_paths.iter().any(|op| op == p);
                fuzzy_match_score(file_part, &rel).map(|score| {
                    let boost = if is_open { 80 } else { 0 };
                    (score + boost, p.clone(), rel, is_open)
                })
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.truncate(80);

        self.apply_keyboard_navigation(nav, scored.len());

        let mut chosen_path = None;
        if nav.enter_pressed {
            if let Some((_, p, _, _)) = scored.get(self.palette_selected) {
                chosen_path = Some(p.clone());
            } else if !file_part.is_empty() {
                chosen_path = Some(PathBuf::from(file_part));
            }
        }

        let scroll_to_sel = self.palette_scroll_to_selected;
        self.palette_scroll_to_selected = false;

        egui::ScrollArea::vertical()
            .id_salt("palette_quick_open_scroll")
            .max_height(280.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (list_idx, (_, path, rel, is_open)) in scored.iter().enumerate() {
                    let is_sel = list_idx == self.palette_selected;
                    let (icon, color) = file_icon_and_color(Some(path));

                    let row_resp = render_palette_row(ui, is_sel, |ui| {
                        ui.add(
                            egui::Label::new(RichText::new(icon).size(13.5).color(color))
                                .selectable(false),
                        );
                        ui.add(
                            egui::Label::new(
                                RichText::new(rel).size(13.5).color(if is_sel {
                                    Color32::WHITE
                                } else {
                                    Color32::from_rgb(215, 220, 230)
                                }),
                            )
                            .selectable(false),
                        );
                        if *is_open {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new("• aberto")
                                            .size(11.5)
                                            .color(Color32::from_rgb(255, 175, 80)),
                                    )
                                    .selectable(false),
                                );
                            });
                        }
                    });

                    if is_sel && scroll_to_sel {
                        row_resp.scroll_to_me(None);
                    }
                    if row_resp.clicked() {
                        chosen_path = Some(path.clone());
                    }
                }
            });

        if let Some(path) = chosen_path {
            self.close_palette();
            self.open_file_path(path);
            if let Some(target_line) = line_part {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.set_cursor_line_col(target_line, 1);
                }
                self.save_session_state();
            }
        }
    }

    fn render_goto_line_prompt(&mut self, ui: &mut egui::Ui, nav: PaletteNavInput) {
        let max_lines = self
            .active_tab_ref()
            .map(|t| t.cached_lines())
            .unwrap_or(1);
        ui.add(
            egui::Label::new(
                RichText::new(format!(
                    "Digite o número da linha (1 a {}) e pressione Enter:",
                    max_lines
                ))
                .color(Color32::from_rgb(190, 195, 205)),
            )
            .selectable(false),
        );
        if nav.enter_pressed {
            if let Ok(line_num) = self.palette_query.trim().parse::<usize>() {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.set_cursor_line_col(line_num, 1);
                    self.status_message = format!("Linha {}", tab.cursor_line);
                }
                self.save_session_state();
                self.close_palette();
            }
        }
    }

    fn render_syntax_selector_list(&mut self, ui: &mut egui::Ui, nav: PaletteNavInput) {
        let syntaxes = list_syntaxes();
        let mut scored: Vec<(i32, String)> = syntaxes
            .into_iter()
            .filter_map(|s| fuzzy_match_score(&self.palette_query, &s).map(|score| (score, s)))
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));

        self.apply_keyboard_navigation(nav, scored.len());

        let mut chosen_syn = None;
        if nav.enter_pressed {
            if let Some((_, s)) = scored.get(self.palette_selected) {
                chosen_syn = Some(s.clone());
            }
        }

        let scroll_to_sel = self.palette_scroll_to_selected;
        self.palette_scroll_to_selected = false;

        egui::ScrollArea::vertical()
            .id_salt("palette_syntax_scroll")
            .max_height(280.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (idx, (_, syn)) in scored.iter().enumerate() {
                    let is_sel = idx == self.palette_selected;
                    let row_resp = render_palette_row(ui, is_sel, |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(syn).size(13.5).color(if is_sel {
                                    Color32::WHITE
                                } else {
                                    Color32::from_rgb(215, 220, 230)
                                }),
                            )
                            .selectable(false),
                        );
                    });

                    if is_sel && scroll_to_sel {
                        row_resp.scroll_to_me(None);
                    }
                    if row_resp.clicked() {
                        chosen_syn = Some(syn.clone());
                    }
                }
            });

        if let Some(syn) = chosen_syn {
            self.close_palette();
            self.execute_command(EditorCommand::SetSyntax(syn));
        }
    }

    fn render_path_prompt(&mut self, ui: &mut egui::Ui, nav: PaletteNavInput) {
        let mode = self.palette_mode.clone();
        let current_input = expand_tilde(Path::new(self.palette_query.trim()));
        let (browse_dir, filter_prefix) = if current_input.is_dir()
            && (self.palette_query.ends_with('/') || self.palette_query.trim() == "~")
        {
            (current_input.clone(), String::new())
        } else {
            let parent = current_input
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| self.workspace.root.clone());
            let prefix = current_input
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();
            (parent, prefix)
        };

        let all_entries = self.path_cache.get_entries(&browse_dir).to_vec();
        let filtered: Vec<PathBuf> = all_entries
            .into_iter()
            .filter(|p| {
                if filter_prefix.is_empty() {
                    return true;
                }
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.to_lowercase().contains(&filter_prefix))
                    .unwrap_or(false)
            })
            .collect();

        self.apply_keyboard_navigation(nav, filtered.len());

        // Tab autocompleta o item selecionado na lista
        if nav.tab_pressed {
            if let Some(item) = filtered.get(self.palette_selected) {
                if item.is_dir() {
                    self.palette_query = format!("{}/", item.display());
                } else {
                    self.palette_query = item.display().to_string();
                }
                self.request_palette_focus = true;
            }
        }

        if nav.enter_pressed && !self.palette_query.trim().is_empty() {
            let target = expand_tilde(Path::new(self.palette_query.trim()));
            self.close_palette();
            match mode {
                PaletteMode::OpenPath | PaletteMode::OpenFolderPath => {
                    self.open_file_path(target);
                }
                PaletteMode::SaveAsPath => {
                    if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                        match tab.save_as(target.clone()) {
                            Ok(()) => {
                                self.status_message = format!("Salvo em: {}", target.display());
                                self.workspace.rebuild_index();
                                self.save_session_state();
                            }
                            Err(e) => self.status_message = e,
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        let scroll_to_sel = self.palette_scroll_to_selected;
        self.palette_scroll_to_selected = false;

        ui.add(
            egui::Label::new(
                RichText::new("Setas ↑/↓ navegam • Tab autocompleta • Enter confirma:")
                    .size(12.0)
                    .color(Color32::from_rgb(160, 165, 175)),
            )
            .selectable(false),
        );

        egui::ScrollArea::vertical()
            .id_salt("palette_path_scroll")
            .max_height(255.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (idx, item) in filtered.iter().enumerate() {
                    let is_sel = idx == self.palette_selected;
                    let (icon, color, disp) = if item.is_dir() {
                        (
                            ICON_FOLDER_OPEN,
                            Color32::from_rgb(210, 195, 145),
                            format!("{}/", item.display()),
                        )
                    } else {
                        let (ic, col) = file_icon_and_color(Some(item));
                        (ic, col, item.display().to_string())
                    };

                    let row_resp = render_palette_row(ui, is_sel, |ui| {
                        ui.add(
                            egui::Label::new(RichText::new(icon).size(13.5).color(color))
                                .selectable(false),
                        );
                        ui.add(
                            egui::Label::new(
                                RichText::new(disp).size(13.5).color(if is_sel {
                                    Color32::WHITE
                                } else {
                                    Color32::from_rgb(210, 215, 225)
                                }),
                            )
                            .selectable(false),
                        );
                    });

                    if is_sel && scroll_to_sel {
                        row_resp.scroll_to_me(None);
                    }
                    if row_resp.clicked() {
                        if item.is_dir() && mode != PaletteMode::OpenFolderPath {
                            self.palette_query = format!("{}/", item.display());
                            self.request_palette_focus = true;
                        } else {
                            self.palette_query = item.display().to_string();
                            self.request_palette_focus = true;
                        }
                    }
                }
            });
    }
}

#[derive(Clone, Copy)]
struct PaletteNavInput {
    down_pressed: bool,
    up_pressed: bool,
    page_down: bool,
    page_up: bool,
    tab_pressed: bool,
    enter_pressed: bool,
}

fn render_palette_row(
    ui: &mut egui::Ui,
    is_selected: bool,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let bg = if is_selected {
        Color32::from_rgb(48, 54, 68)
    } else {
        Color32::TRANSPARENT
    };

    let frame = egui::Frame::NONE
        .fill(bg)
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(8, 5));

    let resp = frame
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                add_contents(ui);
            });
        })
        .response;

    ui.interact(resp.rect, resp.id.with("row_click"), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand)
}
