use crate::palette::{EditorCommand, PaletteMode, PathCompletionCache};
use crate::sidebar::WorkspaceTree;
use crate::state::{ProjectSessionState, SavedTabEntry};
use crate::tab::DocumentTab;
use egui::{Key, KeyboardShortcut, Modifiers};
use std::env;
use std::path::{Path, PathBuf};

pub struct PsychicEditorApp {
    pub tabs: Vec<DocumentTab>,
    pub active_tab: usize,
    pub next_tab_id: u64,
    pub untitled_counter: usize,

    pub sidebar_open: bool,
    pub workspace: WorkspaceTree,

    pub theme_name: String,
    pub font_size: f32,
    pub word_wrap: bool,

    // Search & Replace bar
    pub search_open: bool,
    pub search_query: String,
    pub replace_query: String,
    pub search_case_sensitive: bool,
    pub request_search_focus: bool,

    // Sublime Command Palette / Quick Open / Prompts
    pub palette_mode: PaletteMode,
    pub palette_query: String,
    pub palette_selected: usize,
    pub palette_scroll_to_selected: bool,
    pub request_palette_focus: bool,
    pub palette_just_opened: bool,
    pub path_cache: PathCompletionCache,

    pub status_message: String,
    pub persist_session: bool,
}

impl PsychicEditorApp {
    pub fn from_args(args: &[String], persist_session: bool) -> Self {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut workspace_root = cwd;
        let mut cli_tabs = Vec::new();
        let mut next_tab_id = 1u64;

        for arg in args {
            let path = PathBuf::from(arg);
            if path.is_dir() {
                workspace_root = path.canonicalize().unwrap_or(path);
            } else if path.exists() {
                if let Ok(tab) = DocumentTab::from_file(next_tab_id, path.clone()) {
                    next_tab_id += 1;
                    cli_tabs.push(tab);
                }
            } else {
                let mut tab = DocumentTab::new_untitled(next_tab_id, 1);
                next_tab_id += 1;
                tab.set_path_metadata(path);
                cli_tabs.push(tab);
            }
        }

        let saved_session = if persist_session {
            ProjectSessionState::load_for_workspace(&workspace_root)
        } else {
            None
        };

        let mut sidebar_open = true;
        let mut show_hidden = true;
        let mut theme_name = "base16-eighties.dark".to_string();
        let mut font_size = 14.5;
        let mut word_wrap = false;
        let mut active_tab = 0usize;
        let mut tabs = cli_tabs;

        if let Some(session) = saved_session {
            sidebar_open = session.sidebar_open;
            show_hidden = session.show_hidden;
            theme_name = session.theme_name.clone();
            font_size = session.font_size;
            word_wrap = session.word_wrap;

            if tabs.is_empty() {
                let (restored, restored_active, new_next_id) = session.restore_tabs(next_tab_id);
                tabs = restored;
                active_tab = restored_active;
                next_tab_id = new_next_id;
            }
        }

        let mut untitled_counter = 1;
        if tabs.is_empty() {
            tabs.push(DocumentTab::new_untitled(next_tab_id, untitled_counter));
            next_tab_id += 1;
            untitled_counter += 1;
        }

        let mut workspace = WorkspaceTree::new(workspace_root);
        if workspace.show_hidden != show_hidden {
            workspace.show_hidden = show_hidden;
            workspace.rebuild_index();
        }

        Self {
            tabs,
            active_tab,
            next_tab_id,
            untitled_counter,
            sidebar_open,
            workspace,
            theme_name,
            font_size,
            word_wrap,
            search_open: false,
            search_query: String::new(),
            replace_query: String::new(),
            search_case_sensitive: false,
            request_search_focus: false,
            palette_mode: PaletteMode::Closed,
            palette_query: String::new(),
            palette_selected: 0,
            palette_scroll_to_selected: false,
            request_palette_focus: false,
            palette_just_opened: false,
            path_cache: PathCompletionCache::default(),
            status_message: "Pronto • Ctrl+P: Abrir Rápido | Ctrl+Shift+P: Comandos".to_string(),
            persist_session,
        }
    }

    pub fn build_session_state(&self) -> ProjectSessionState {
        let open_tabs = self
            .tabs
            .iter()
            .filter_map(|t| {
                t.path.as_ref().map(|p| SavedTabEntry {
                    path: p.clone(),
                    cursor_line: t.cursor_line,
                    cursor_col: t.cursor_col,
                })
            })
            .collect();

        let active_file = self.active_tab_ref().and_then(|t| t.path.clone());

        ProjectSessionState {
            workspace_root: self.workspace.root.clone(),
            sidebar_open: self.sidebar_open,
            show_hidden: self.workspace.show_hidden,
            theme_name: self.theme_name.clone(),
            font_size: self.font_size,
            word_wrap: self.word_wrap,
            active_file,
            open_tabs,
        }
    }

    pub fn save_session_state(&self) {
        if !self.persist_session {
            return;
        }
        let state = self.build_session_state();
        let _ = state.save_for_workspace();
    }

    pub fn open_palette(&mut self, mode: PaletteMode, initial_query: String) {
        self.palette_mode = mode;
        self.palette_query = initial_query;
        self.palette_selected = 0;
        self.palette_scroll_to_selected = true;
        self.request_palette_focus = true;
        self.palette_just_opened = true;
        self.path_cache.invalidate();
    }

    pub fn close_palette(&mut self) {
        self.palette_mode = PaletteMode::Closed;
        self.palette_query.clear();
        self.palette_selected = 0;
        self.palette_scroll_to_selected = false;
        self.palette_just_opened = false;
    }

    pub fn select_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() && self.active_tab != idx {
            self.active_tab = idx;
            self.save_session_state();
        }
    }

    pub fn execute_command(&mut self, cmd: EditorCommand) {
        match cmd {
            EditorCommand::NewTab => {
                let tab = DocumentTab::new_untitled(self.next_tab_id, self.untitled_counter);
                self.next_tab_id += 1;
                self.untitled_counter += 1;
                self.tabs.push(tab);
                self.active_tab = self.tabs.len() - 1;
                self.status_message = "Novo arquivo criado.".to_string();
                self.save_session_state();
            }
            EditorCommand::QuickOpenPrompt => {
                self.open_palette(PaletteMode::QuickOpen, String::new());
            }
            EditorCommand::OpenFilePrompt => {
                let default_dir = self
                    .active_tab_ref()
                    .and_then(|t| t.path.as_ref())
                    .and_then(|p| p.parent())
                    .map(|p| format!("{}/", p.display()))
                    .unwrap_or_else(|| format!("{}/", self.workspace.root.display()));
                self.open_palette(PaletteMode::OpenPath, default_dir);
            }
            EditorCommand::OpenFolderPrompt => {
                let default_dir = format!("{}/", self.workspace.root.display());
                self.open_palette(PaletteMode::OpenFolderPath, default_dir);
            }
            EditorCommand::Save => {
                self.save_active_tab();
            }
            EditorCommand::SaveAs => {
                let initial = self
                    .active_tab_ref()
                    .and_then(|t| t.path.as_ref())
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| {
                        let name = self
                            .active_tab_ref()
                            .map(|t| t.title.as_str())
                            .unwrap_or("untitled.txt");
                        format!("{}/{}", self.workspace.root.display(), name)
                    });
                self.open_palette(PaletteMode::SaveAsPath, initial);
            }
            EditorCommand::RevertFile => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    match tab.revert_from_disk() {
                        Ok(()) => {
                            self.status_message = format!("Recarregado do disco: {}", tab.title);
                        }
                        Err(e) => self.status_message = e,
                    }
                }
            }
            EditorCommand::CloseActiveTab => {
                self.close_tab(self.active_tab);
            }
            EditorCommand::CloseOtherTabs => {
                if let Some(keep) = self.tabs.get(self.active_tab).cloned() {
                    self.tabs.clear();
                    self.tabs.push(keep);
                    self.active_tab = 0;
                    self.status_message = "Outras abas fechadas.".to_string();
                    self.save_session_state();
                }
            }
            EditorCommand::CloseAllTabs => {
                self.tabs.clear();
                self.tabs
                    .push(DocumentTab::new_untitled(self.next_tab_id, self.untitled_counter));
                self.next_tab_id += 1;
                self.untitled_counter += 1;
                self.active_tab = 0;
                self.status_message = "Todas as abas foram fechadas.".to_string();
                self.save_session_state();
            }
            EditorCommand::DuplicateLine => {
                self.duplicate_current_line();
            }
            EditorCommand::ToggleLineEnding => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.line_ending = if tab.line_ending == "LF" { "CRLF" } else { "LF" };
                    self.status_message = format!("Quebra de linha: {}", tab.line_ending);
                }
            }
            EditorCommand::ToggleSidebar => {
                self.sidebar_open = !self.sidebar_open;
                self.save_session_state();
            }
            EditorCommand::ToggleSearch => {
                self.search_open = !self.search_open;
                if self.search_open {
                    self.request_search_focus = true;
                }
            }
            EditorCommand::ToggleWordWrap => {
                self.word_wrap = !self.word_wrap;
                self.status_message = if self.word_wrap {
                    "Word Wrap: Ativado".to_string()
                } else {
                    "Word Wrap: Desativado".to_string()
                };
                self.save_session_state();
            }
            EditorCommand::GotoLinePrompt => {
                self.open_palette(PaletteMode::GotoLine, String::new());
            }
            EditorCommand::ChangeSyntaxPrompt => {
                self.open_palette(PaletteMode::SyntaxSelector, String::new());
            }
            EditorCommand::SetSyntax(syn) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.syntax_name = syn.clone();
                    self.status_message = format!("Sintaxe alterada para: {}", syn);
                }
            }
            EditorCommand::SetTheme(theme) => {
                self.theme_name = theme.clone();
                self.status_message = format!("Tema alterado para: {}", theme);
                self.save_session_state();
            }
            EditorCommand::FontSizeIncrease => {
                self.font_size = (self.font_size + 1.0).min(36.0);
                self.save_session_state();
            }
            EditorCommand::FontSizeDecrease => {
                self.font_size = (self.font_size - 1.0).max(9.0);
                self.save_session_state();
            }
            EditorCommand::FontSizeReset => {
                self.font_size = 14.5;
                self.save_session_state();
            }
        }
    }

    pub fn active_tab_ref(&self) -> Option<&DocumentTab> {
        self.tabs.get(self.active_tab)
    }

    pub fn duplicate_current_line(&mut self) {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };
        let target_line = tab.cursor_line.max(1);
        let mut lines: Vec<&str> = tab.content.split('\n').collect();
        let idx = (target_line - 1).min(lines.len().saturating_sub(1));
        if let Some(&line_str) = lines.get(idx) {
            lines.insert(idx + 1, line_str);
            tab.content = lines.join("\n");
            tab.mark_content_edited();
            tab.set_cursor_line_col(idx + 2, tab.cursor_col);
            self.status_message = "Linha duplicada.".to_string();
        }
    }

    pub fn open_file_path(&mut self, path: PathBuf) {
        let expanded = expand_tilde(&path);
        if expanded.is_dir() {
            // Salva sessão do projeto anterior antes de trocar de pasta
            self.save_session_state();
            let canonical = expanded.canonicalize().unwrap_or(expanded);
            self.workspace.set_root(canonical.clone());
            self.sidebar_open = true;

            if self.persist_session {
                if let Some(saved) = ProjectSessionState::load_for_workspace(&canonical) {
                    let (restored, restored_active, next_id) =
                        saved.restore_tabs(self.next_tab_id);
                    if !restored.is_empty() {
                        self.tabs = restored;
                        self.active_tab = restored_active;
                        self.next_tab_id = next_id;
                    }
                }
            }
            self.status_message = format!("Pasta aberta: {}", canonical.display());
            self.save_session_state();
            return;
        }

        for (idx, tab) in self.tabs.iter().enumerate() {
            if tab.matches_path(&expanded) {
                self.active_tab = idx;
                self.status_message = format!("Arquivo focado: {}", expanded.display());
                self.save_session_state();
                return;
            }
        }

        match DocumentTab::from_file(self.next_tab_id, expanded.clone()) {
            Ok(tab) => {
                self.next_tab_id += 1;
                if self.tabs.len() == 1
                    && self.tabs[0].path.is_none()
                    && self.tabs[0].content.is_empty()
                {
                    self.tabs[0] = tab;
                    self.active_tab = 0;
                } else {
                    self.tabs.push(tab);
                    self.active_tab = self.tabs.len() - 1;
                }
                self.status_message = format!("Aberto: {}", expanded.display());
                self.save_session_state();
            }
            Err(err) => {
                self.status_message = err;
            }
        }
    }

    pub fn save_active_tab(&mut self) {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };
        if tab.path.is_none() {
            self.execute_command(EditorCommand::SaveAs);
            return;
        }
        match tab.save() {
            Ok(()) => {
                if let Some(p) = &tab.path {
                    self.status_message = format!("Salvo: {}", p.display());
                }
                self.workspace.rebuild_index();
                self.save_session_state();
            }
            Err(e) => {
                self.status_message = e;
            }
        }
    }

    pub fn close_tab(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        self.tabs.remove(idx);
        if self.tabs.is_empty() {
            self.tabs
                .push(DocumentTab::new_untitled(self.next_tab_id, self.untitled_counter));
            self.next_tab_id += 1;
            self.untitled_counter += 1;
            self.active_tab = 0;
        } else if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        self.save_session_state();
    }

    pub fn find_next_in_active_tab(&mut self) {
        if self.search_query.is_empty() {
            return;
        }
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };

        let haystack = if self.search_case_sensitive {
            tab.content.clone()
        } else {
            tab.content.to_lowercase()
        };
        let needle = if self.search_case_sensitive {
            self.search_query.clone()
        } else {
            self.search_query.to_lowercase()
        };

        let mut matches = Vec::new();
        for (byte_idx, _) in haystack.match_indices(&needle) {
            let start_char = tab.content[..byte_idx].chars().count();
            let len_chars = needle.chars().count();
            matches.push((start_char, start_char + len_chars));
        }

        if matches.is_empty() {
            self.status_message = format!("Nenhuma ocorrência para '{}'", self.search_query);
            return;
        }

        let current_pos = tab
            .char_index_for_line(tab.cursor_line)
            .saturating_add(tab.cursor_col);
        let next = matches
            .iter()
            .find(|(start, _)| *start >= current_pos)
            .or_else(|| matches.first())
            .copied();

        if let Some((start, end)) = next {
            tab.pending_cursor_char = Some((start, end));
            tab.update_cursor_stats(start, end - start);
            self.status_message = format!("{} ocorrência(s) encontrada(s)", matches.len());
        }
    }

    pub fn replace_all_in_active_tab(&mut self) {
        if self.search_query.is_empty() {
            return;
        }
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };
        let count = tab.content.matches(&self.search_query).count();
        if count > 0 {
            tab.content = tab.content.replace(&self.search_query, &self.replace_query);
            tab.mark_content_edited();
            self.status_message = format!("Substituídas {} ocorrência(s).", count);
        } else {
            self.status_message = "Nenhuma ocorrência exata encontrada.".to_string();
        }
    }

    pub fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let ctrl_shift_p = KeyboardShortcut::new(Modifiers::CTRL | Modifiers::SHIFT, Key::P);
        let ctrl_shift_s = KeyboardShortcut::new(Modifiers::CTRL | Modifiers::SHIFT, Key::S);
        let ctrl_shift_d = KeyboardShortcut::new(Modifiers::CTRL | Modifiers::SHIFT, Key::D);
        let ctrl_p = KeyboardShortcut::new(Modifiers::CTRL, Key::P);
        let ctrl_n = KeyboardShortcut::new(Modifiers::CTRL, Key::N);
        let ctrl_o = KeyboardShortcut::new(Modifiers::CTRL, Key::O);
        let ctrl_k = KeyboardShortcut::new(Modifiers::CTRL, Key::K);
        let ctrl_s = KeyboardShortcut::new(Modifiers::CTRL, Key::S);
        let ctrl_w = KeyboardShortcut::new(Modifiers::CTRL, Key::W);
        let ctrl_b = KeyboardShortcut::new(Modifiers::CTRL, Key::B);
        let ctrl_f = KeyboardShortcut::new(Modifiers::CTRL, Key::F);
        let ctrl_h = KeyboardShortcut::new(Modifiers::CTRL, Key::H);
        let ctrl_g = KeyboardShortcut::new(Modifiers::CTRL, Key::G);
        let ctrl_plus = KeyboardShortcut::new(Modifiers::CTRL, Key::Equals);
        let ctrl_minus = KeyboardShortcut::new(Modifiers::CTRL, Key::Minus);
        let ctrl_zero = KeyboardShortcut::new(Modifiers::CTRL, Key::Num0);
        let alt_z = KeyboardShortcut::new(Modifiers::ALT, Key::Z);

        ctx.input_mut(|i| {
            if i.consume_shortcut(&ctrl_shift_p) {
                self.open_palette(PaletteMode::CommandPalette, String::new());
            } else if i.consume_shortcut(&ctrl_shift_s) {
                self.execute_command(EditorCommand::SaveAs);
            } else if i.consume_shortcut(&ctrl_shift_d) {
                self.execute_command(EditorCommand::DuplicateLine);
            } else if i.consume_shortcut(&ctrl_p) {
                self.open_palette(PaletteMode::QuickOpen, String::new());
            } else if i.consume_shortcut(&ctrl_n) {
                self.execute_command(EditorCommand::NewTab);
            } else if i.consume_shortcut(&ctrl_o) {
                self.execute_command(EditorCommand::OpenFilePrompt);
            } else if i.consume_shortcut(&ctrl_k) {
                self.execute_command(EditorCommand::OpenFolderPrompt);
            } else if i.consume_shortcut(&ctrl_s) {
                self.execute_command(EditorCommand::Save);
            } else if i.consume_shortcut(&ctrl_w) {
                self.execute_command(EditorCommand::CloseActiveTab);
            } else if i.consume_shortcut(&ctrl_b) {
                self.execute_command(EditorCommand::ToggleSidebar);
            } else if i.consume_shortcut(&ctrl_f) || i.consume_shortcut(&ctrl_h) {
                self.search_open = true;
                self.request_search_focus = true;
            } else if i.consume_shortcut(&ctrl_g) {
                self.execute_command(EditorCommand::GotoLinePrompt);
            } else if i.consume_shortcut(&ctrl_plus) {
                self.execute_command(EditorCommand::FontSizeIncrease);
            } else if i.consume_shortcut(&ctrl_minus) {
                self.execute_command(EditorCommand::FontSizeDecrease);
            } else if i.consume_shortcut(&ctrl_zero) {
                self.execute_command(EditorCommand::FontSizeReset);
            } else if i.consume_shortcut(&alt_z) {
                self.execute_command(EditorCommand::ToggleWordWrap);
            } else if i.key_pressed(Key::Escape) {
                if self.palette_mode != PaletteMode::Closed {
                    self.close_palette();
                } else if self.search_open {
                    self.search_open = false;
                }
            }
        });
    }
}

impl eframe::App for PsychicEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        self.render_top_bar(ctx);
        self.render_status_bar(ctx);
        self.render_search_bar(ctx);
        self.render_sidebar(ctx);
        self.render_editor_central(ctx);
        self.render_palette_modal(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_session_state();
    }
}

pub fn expand_tilde(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/") {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    } else if s == "~" {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home);
        }
    }
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_commands_and_tab_lifecycle() {
        let mut app = PsychicEditorApp::from_args(&[], false);
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.active_tab, 0);

        app.execute_command(EditorCommand::NewTab);
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab, 1);

        app.execute_command(EditorCommand::NewTab);
        assert_eq!(app.tabs.len(), 3);

        app.execute_command(EditorCommand::CloseOtherTabs);
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.active_tab, 0);

        app.execute_command(EditorCommand::CloseAllTabs);
        assert_eq!(app.tabs.len(), 1);
    }

    #[test]
    fn test_app_duplicate_line_and_search_replace() {
        let mut app = PsychicEditorApp::from_args(&[], false);
        app.tabs[0].content = "linha alfa\nlinha beta".to_string();
        app.tabs[0].mark_content_edited();
        app.tabs[0].set_cursor_line_col(1, 3);

        app.execute_command(EditorCommand::DuplicateLine);
        assert_eq!(
            app.tabs[0].content,
            "linha alfa\nlinha alfa\nlinha beta"
        );
        assert_eq!(app.tabs[0].cursor_line, 2);

        app.search_query = "alfa".to_string();
        app.replace_query = "gama".to_string();
        app.replace_all_in_active_tab();
        assert_eq!(
            app.tabs[0].content,
            "linha gama\nlinha gama\nlinha beta"
        );

        app.search_query = "beta".to_string();
        app.find_next_in_active_tab();
        assert_eq!(app.tabs[0].cursor_line, 3);
    }

    #[test]
    fn test_app_view_toggles_and_zoom() {
        let mut app = PsychicEditorApp::from_args(&[], false);
        let init_font = app.font_size;
        app.execute_command(EditorCommand::FontSizeIncrease);
        assert!(app.font_size > init_font);
        app.execute_command(EditorCommand::FontSizeDecrease);
        assert_eq!(app.font_size, init_font);
        app.execute_command(EditorCommand::FontSizeReset);
        assert_eq!(app.font_size, 14.5);

        assert!(!app.word_wrap);
        app.execute_command(EditorCommand::ToggleWordWrap);
        assert!(app.word_wrap);

        assert_eq!(app.tabs[0].line_ending, "LF");
        app.execute_command(EditorCommand::ToggleLineEnding);
        assert_eq!(app.tabs[0].line_ending, "CRLF");

        app.execute_command(EditorCommand::SetSyntax("Rust".to_string()));
        assert_eq!(app.tabs[0].syntax_name, "Rust");

        app.execute_command(EditorCommand::SetTheme("base16-mocha.dark".to_string()));
        assert_eq!(app.theme_name, "base16-mocha.dark");
    }

    #[test]
    fn test_expand_tilde() {
        let normal = Path::new("/etc/hosts");
        assert_eq!(expand_tilde(normal), PathBuf::from("/etc/hosts"));
        let tilde = expand_tilde(Path::new("~/.config"));
        assert!(!tilde.to_string_lossy().starts_with('~'));
    }

    #[test]
    fn test_headless_ui_rendering_all_panels_and_palette_modes() {
        let ctx = egui::Context::default();
        let mut app = PsychicEditorApp::from_args(&["Cargo.toml".to_string()], false);
        app.search_open = true;

        let modes = [
            (PaletteMode::Closed, ""),
            (PaletteMode::CommandPalette, "save"),
            (PaletteMode::QuickOpen, "main"),
            (PaletteMode::QuickOpen, ":10"),
            (PaletteMode::GotoLine, "5"),
            (PaletteMode::SyntaxSelector, "Rust"),
            (PaletteMode::OpenPath, "./"),
            (PaletteMode::SaveAsPath, "./test.txt"),
            (PaletteMode::OpenFolderPath, "./"),
        ];

        for (mode, query) in modes {
            if mode == PaletteMode::Closed {
                app.close_palette();
            } else {
                app.open_palette(mode, query.to_string());
            }

            let mut raw_input = egui::RawInput::default();
            raw_input.events.push(egui::Event::Key {
                key: Key::ArrowDown,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            });

            let _ = ctx.run(raw_input, |ctx| {
                app.handle_shortcuts(ctx);
                app.render_top_bar(ctx);
                app.render_status_bar(ctx);
                app.render_search_bar(ctx);
                app.render_sidebar(ctx);
                app.render_editor_central(ctx);
                app.render_palette_modal(ctx);
            });
        }

        let state = app.build_session_state();
        assert!(!state.open_tabs.is_empty());
    }

    #[test]
    fn test_palette_does_not_close_on_opening_click_and_closes_on_outside_click() {
        let ctx = egui::Context::default();
        let mut app = PsychicEditorApp::from_args(&[], false);

        // Frame 1: simula clique no botão da barra superior que abre o Command Palette
        let mut click_down = egui::RawInput::default();
        let pos = egui::pos2(10.0, 10.0);
        click_down.events.push(egui::Event::PointerMoved(pos));
        click_down.events.push(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        let _ = ctx.run(click_down, |ctx| {
            app.render_top_bar(ctx);
        });

        let mut click_up = egui::RawInput::default();
        click_up.events.push(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        let _ = ctx.run(click_up, |ctx| {
            // No mesmo frame do clique que abriu o palette, render_palette_modal é chamado
            app.open_palette(PaletteMode::CommandPalette, String::new());
            app.render_palette_modal(ctx);
        });

        assert_eq!(
            app.palette_mode,
            PaletteMode::CommandPalette,
            "Command Palette deve permanecer aberto no frame em que o botão foi clicado"
        );

        // Frame subsequente: clique fora do modal deve fechá-lo
        let mut outside_down = egui::RawInput::default();
        let outside_pos = egui::pos2(5.0, 5.0);
        outside_down
            .events
            .push(egui::Event::PointerMoved(outside_pos));
        outside_down.events.push(egui::Event::PointerButton {
            pos: outside_pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        let _ = ctx.run(outside_down, |ctx| {
            app.render_palette_modal(ctx);
        });

        let mut outside_up = egui::RawInput::default();
        outside_up.events.push(egui::Event::PointerButton {
            pos: outside_pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        let _ = ctx.run(outside_up, |ctx| {
            app.render_palette_modal(ctx);
        });

        assert_eq!(
            app.palette_mode,
            PaletteMode::Closed,
            "Command Palette deve fechar ao clicar fora em um frame posterior"
        );
    }

    #[test]
    fn test_keyboard_shortcuts_and_escape() {
        let ctx = egui::Context::default();
        let mut app = PsychicEditorApp::from_args(&[], false);

        let shortcuts = [
            (Modifiers::CTRL | Modifiers::SHIFT, Key::P, PaletteMode::CommandPalette),
            (Modifiers::CTRL, Key::P, PaletteMode::QuickOpen),
            (Modifiers::CTRL, Key::O, PaletteMode::OpenPath),
            (Modifiers::CTRL, Key::K, PaletteMode::OpenFolderPath),
            (Modifiers::CTRL, Key::G, PaletteMode::GotoLine),
            (Modifiers::CTRL | Modifiers::SHIFT, Key::S, PaletteMode::SaveAsPath),
        ];

        for (mods, key, expected_mode) in shortcuts {
            let mut input = egui::RawInput::default();
            input.modifiers = mods;
            input.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: mods,
            });
            let _ = ctx.run(input, |ctx| {
                app.handle_shortcuts(ctx);
            });
            assert_eq!(app.palette_mode, expected_mode);

            // Escape fecha o modal
            let mut esc_input = egui::RawInput::default();
            esc_input.events.push(egui::Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            });
            let _ = ctx.run(esc_input, |ctx| {
                app.handle_shortcuts(ctx);
            });
            assert_eq!(app.palette_mode, PaletteMode::Closed);
        }
    }

    #[test]
    fn test_palette_enter_actions_and_tab_completion() {
        let ctx = egui::Context::default();
        let mut app = PsychicEditorApp::from_args(&["Cargo.toml".to_string()], false);

        // 1. GotoLine via Enter
        app.open_palette(PaletteMode::GotoLine, "3".to_string());
        let mut enter_input = egui::RawInput::default();
        enter_input.events.push(egui::Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        });
        let _ = ctx.run(enter_input.clone(), |ctx| {
            app.render_palette_modal(ctx);
        });
        assert_eq!(app.palette_mode, PaletteMode::Closed);
        assert_eq!(app.tabs[0].cursor_line, 3);

        // 2. QuickOpen :2 via Enter
        app.open_palette(PaletteMode::QuickOpen, ":2".to_string());
        let _ = ctx.run(enter_input.clone(), |ctx| {
            app.render_palette_modal(ctx);
        });
        assert_eq!(app.palette_mode, PaletteMode::Closed);
        assert_eq!(app.tabs[0].cursor_line, 2);

        // 3. SyntaxSelector via Enter
        app.open_palette(PaletteMode::SyntaxSelector, "Rust".to_string());
        let _ = ctx.run(enter_input.clone(), |ctx| {
            app.render_palette_modal(ctx);
        });
        assert_eq!(app.palette_mode, PaletteMode::Closed);
        assert_eq!(app.tabs[0].syntax_name, "Rust");

        // 4. OpenPath com Tab completion
        app.open_palette(PaletteMode::OpenPath, "./".to_string());
        let mut tab_input = egui::RawInput::default();
        tab_input.events.push(egui::Event::Key {
            key: Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        });
        let _ = ctx.run(tab_input, |ctx| {
            app.render_palette_modal(ctx);
        });
        assert_ne!(app.palette_query, "./");
    }

    #[test]
    fn test_open_file_path_and_tab_wheel_scroll() {
        let ctx = egui::Context::default();
        let mut app = PsychicEditorApp::from_args(&[], false);

        // Abre Cargo.toml (substitui aba untitled vazia)
        app.open_file_path(PathBuf::from("Cargo.toml"));
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.tabs[0].title, "Cargo.toml");

        // Abre README.md em uma 2ª aba
        app.open_file_path(PathBuf::from("README.md"));
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab, 1);

        // Reabrir Cargo.toml foca na aba 0 sem duplicar
        app.open_file_path(PathBuf::from("Cargo.toml"));
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab, 0);

        // Simula rodinha do mouse sobre a barra de abas
        let mut scroll_input = egui::RawInput::default();
        scroll_input
            .events
            .push(egui::Event::PointerMoved(egui::pos2(220.0, 12.0)));
        scroll_input.events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -40.0),
            modifiers: Modifiers::NONE,
        });
        let _ = ctx.run(scroll_input, |ctx| {
            app.render_top_bar(ctx);
        });
    }

    #[test]
    fn test_holding_arrow_down_keeps_focus_on_palette_input_and_allows_typing() {
        let ctx = egui::Context::default();
        let mut app = PsychicEditorApp::from_args(&[], false);
        app.open_palette(PaletteMode::CommandPalette, String::new());

        // Simula segurar a seta para baixo rapidamente por 15 frames seguidos (repeat: true)
        for _ in 0..15 {
            let mut repeat_input = egui::RawInput::default();
            repeat_input.events.push(egui::Event::Key {
                key: Key::ArrowDown,
                physical_key: None,
                pressed: true,
                repeat: true,
                modifiers: Modifiers::NONE,
            });
            let _ = ctx.run(repeat_input, |ctx| {
                app.render_editor_central(ctx);
                app.render_palette_modal(ctx);
            });
        }

        // Em seguida, digita "save" sem clicar com o mouse
        let mut text_input = egui::RawInput::default();
        text_input
            .events
            .push(egui::Event::Text("save".to_string()));
        let _ = ctx.run(text_input, |ctx| {
            app.render_editor_central(ctx);
            app.render_palette_modal(ctx);
        });

        assert_eq!(
            app.palette_query, "save",
            "Digitar após segurar seta para baixo deve ir direto para o input do Command Palette"
        );
    }
}
