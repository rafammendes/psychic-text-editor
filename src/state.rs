use crate::tab::DocumentTab;
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct SavedTabEntry {
    pub path: PathBuf,
    pub cursor_line: usize,
    pub cursor_col: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectSessionState {
    pub workspace_root: PathBuf,
    pub sidebar_open: bool,
    pub show_hidden: bool,
    pub theme_name: String,
    pub font_size: f32,
    pub word_wrap: bool,
    pub active_file: Option<PathBuf>,
    pub open_tabs: Vec<SavedTabEntry>,
}

impl Default for ProjectSessionState {
    fn default() -> Self {
        Self {
            workspace_root: PathBuf::from("."),
            sidebar_open: true,
            show_hidden: true,
            theme_name: "base16-eighties.dark".to_string(),
            font_size: 14.5,
            word_wrap: false,
            active_file: None,
            open_tabs: Vec::new(),
        }
    }
}

impl ProjectSessionState {
    pub fn state_file_for_workspace(workspace_root: &Path) -> PathBuf {
        let base_dir = env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share"))
            })
            .unwrap_or_else(env::temp_dir)
            .join("psychic-text-editor")
            .join("sessions");

        Self::state_file_in_dir(&base_dir, workspace_root)
    }

    pub fn state_file_in_dir(sessions_dir: &Path, workspace_root: &Path) -> PathBuf {
        let canonical = workspace_root
            .canonicalize()
            .unwrap_or_else(|_| workspace_root.to_path_buf());
        let mut hasher = DefaultHasher::new();
        canonical.to_string_lossy().hash(&mut hasher);
        let hash = hasher.finish();
        sessions_dir.join(format!("project_{:016x}.state", hash))
    }

    pub fn serialize(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "workspace_root={}\n",
            escape_val(&self.workspace_root.to_string_lossy())
        ));
        out.push_str(&format!("sidebar_open={}\n", self.sidebar_open));
        out.push_str(&format!("show_hidden={}\n", self.show_hidden));
        out.push_str(&format!("theme_name={}\n", escape_val(&self.theme_name)));
        out.push_str(&format!("font_size={}\n", self.font_size));
        out.push_str(&format!("word_wrap={}\n", self.word_wrap));
        if let Some(active) = &self.active_file {
            out.push_str(&format!(
                "active_file={}\n",
                escape_val(&active.to_string_lossy())
            ));
        }
        for tab in &self.open_tabs {
            out.push_str(&format!(
                "tab={}:{}:{}\n",
                tab.cursor_line,
                tab.cursor_col,
                escape_val(&tab.path.to_string_lossy())
            ));
        }
        out
    }

    pub fn deserialize(content: &str) -> Self {
        let mut state = Self::default();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, val)) = line.split_once('=') else {
                continue;
            };
            match key {
                "workspace_root" => state.workspace_root = PathBuf::from(unescape_val(val)),
                "sidebar_open" => state.sidebar_open = val == "true",
                "show_hidden" => state.show_hidden = val == "true",
                "theme_name" => state.theme_name = unescape_val(val),
                "font_size" => {
                    if let Ok(sz) = val.parse::<f32>() {
                        state.font_size = sz.clamp(9.0, 36.0);
                    }
                }
                "word_wrap" => state.word_wrap = val == "true",
                "active_file" => {
                    let p = unescape_val(val);
                    if !p.is_empty() {
                        state.active_file = Some(PathBuf::from(p));
                    }
                }
                "tab" => {
                    let mut parts = val.splitn(3, ':');
                    if let (Some(l_str), Some(c_str), Some(p_str)) =
                        (parts.next(), parts.next(), parts.next())
                    {
                        let cursor_line = l_str.parse::<usize>().unwrap_or(1).max(1);
                        let cursor_col = c_str.parse::<usize>().unwrap_or(1).max(1);
                        let path = PathBuf::from(unescape_val(p_str));
                        state.open_tabs.push(SavedTabEntry {
                            path,
                            cursor_line,
                            cursor_col,
                        });
                    }
                }
                _ => {}
            }
        }
        state
    }

    pub fn save_to_file(&self, state_file: &Path) -> Result<(), String> {
        if let Some(parent) = state_file.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Falha ao criar pasta de sessão: {}", e))?;
        }
        fs::write(state_file, self.serialize())
            .map_err(|e| format!("Falha ao salvar sessão: {}", e))
    }

    pub fn load_from_file(state_file: &Path) -> Option<Self> {
        let raw = fs::read_to_string(state_file).ok()?;
        Some(Self::deserialize(&raw))
    }

    pub fn save_for_workspace(&self) -> Result<(), String> {
        let path = Self::state_file_for_workspace(&self.workspace_root);
        self.save_to_file(&path)
    }

    pub fn load_for_workspace(workspace_root: &Path) -> Option<Self> {
        let path = Self::state_file_for_workspace(workspace_root);
        Self::load_from_file(&path)
    }

    /// Restaura as abas abertas a partir da sessão salva, ignorando com segurança arquivos
    /// que tenham sido excluídos ou movidos antes da reabertura do projeto.
    pub fn restore_tabs(&self, mut next_tab_id: u64) -> (Vec<DocumentTab>, usize, u64) {
        let mut restored_tabs = Vec::new();
        let mut active_idx = 0usize;

        for entry in &self.open_tabs {
            // Segurança: se o arquivo foi excluído do disco antes de reabrir o editor, ignora sem falhar
            if !entry.path.is_file() {
                continue;
            }
            if let Ok(mut tab) = DocumentTab::from_file(next_tab_id, entry.path.clone()) {
                next_tab_id += 1;
                tab.set_cursor_line_col(entry.cursor_line, entry.cursor_col);
                if let Some(active_path) = &self.active_file {
                    if tab.matches_path(active_path) {
                        active_idx = restored_tabs.len();
                    }
                }
                restored_tabs.push(tab);
            }
        }

        if active_idx >= restored_tabs.len() {
            active_idx = 0;
        }

        (restored_tabs, active_idx, next_tab_id)
    }
}

fn escape_val(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n")
}

fn unescape_val(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_and_deserialize_session_state() {
        let state = ProjectSessionState {
            workspace_root: PathBuf::from("/tmp/meu_projeto"),
            sidebar_open: false,
            show_hidden: true,
            theme_name: "base16-mocha.dark".to_string(),
            font_size: 16.0,
            word_wrap: true,
            active_file: Some(PathBuf::from("/tmp/meu_projeto/src/lib.rs")),
            open_tabs: vec![
                SavedTabEntry {
                    path: PathBuf::from("/tmp/meu_projeto/Cargo.toml"),
                    cursor_line: 8,
                    cursor_col: 3,
                },
                SavedTabEntry {
                    path: PathBuf::from("/tmp/meu_projeto/src/lib.rs"),
                    cursor_line: 42,
                    cursor_col: 12,
                },
            ],
        };

        let text = state.serialize();
        let restored = ProjectSessionState::deserialize(&text);
        assert_eq!(restored, state);
    }

    #[test]
    fn test_restore_tabs_skips_deleted_files_and_restores_cursor_line() {
        let temp_dir = env::temp_dir().join("psychic_session_restore_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let file_alive = temp_dir.join("alive.rs");
        let file_deleted = temp_dir.join("deleted.rs");

        fs::write(&file_alive, "fn a() {}\nfn b() {}\nfn c() {\n    let x = 10;\n}\n").unwrap();
        fs::write(&file_deleted, "will be deleted").unwrap();

        let state = ProjectSessionState {
            workspace_root: temp_dir.clone(),
            active_file: Some(file_alive.clone()),
            open_tabs: vec![
                SavedTabEntry {
                    path: file_deleted.clone(),
                    cursor_line: 1,
                    cursor_col: 1,
                },
                SavedTabEntry {
                    path: file_alive.clone(),
                    cursor_line: 4,
                    cursor_col: 9,
                },
            ],
            ..Default::default()
        };

        // Simula exclusão do arquivo antes de reabrir o projeto
        fs::remove_file(&file_deleted).unwrap();

        let state_file = ProjectSessionState::state_file_in_dir(&temp_dir, &temp_dir);
        state.save_to_file(&state_file).unwrap();
        let loaded = ProjectSessionState::load_from_file(&state_file).unwrap();

        let (tabs, active_idx, next_id) = loaded.restore_tabs(1);
        assert_eq!(tabs.len(), 1, "Deleted file must be skipped safely");
        assert_eq!(active_idx, 0);
        assert_eq!(next_id, 2);
        assert_eq!(tabs[0].cursor_line, 4);
        assert_eq!(tabs[0].cursor_col, 9);
        assert!(tabs[0].pending_cursor_char.is_some());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_session_escape_malformed_lines_and_all_files_deleted() {
        let raw = "# comentario\nlinha_invalida_sem_igual\nfont_size=999.0\nworkspace_root=/tmp/a\\\\b\\nc\ntab=10:5:/tmp/nao_existe_1.rs\ntab=20:2:/tmp/nao_existe_2.rs\nactive_file=/tmp/nao_existe_2.rs\n";
        let state = ProjectSessionState::deserialize(raw);
        assert_eq!(state.font_size, 36.0);
        assert_eq!(state.workspace_root, PathBuf::from("/tmp/a\\b\nc"));
        assert_eq!(state.open_tabs.len(), 2);

        // Todos os arquivos foram deletados antes de reabrir: deve retornar vazio sem pânico
        let (restored, active_idx, next_id) = state.restore_tabs(5);
        assert!(restored.is_empty());
        assert_eq!(active_idx, 0);
        assert_eq!(next_id, 5);
    }

    #[test]
    fn test_save_and_load_for_workspace() {
        let temp_dir = env::temp_dir().join("psychic_workspace_session_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let state = ProjectSessionState {
            workspace_root: temp_dir.clone(),
            theme_name: "base16-eighties.dark".to_string(),
            font_size: 15.0,
            ..Default::default()
        };

        let res = state.save_for_workspace();
        assert!(res.is_ok());

        let loaded = ProjectSessionState::load_for_workspace(&temp_dir);
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.font_size, 15.0);
        assert_eq!(loaded.theme_name, "base16-eighties.dark");

        let session_file = ProjectSessionState::state_file_for_workspace(&temp_dir);
        let _ = fs::remove_file(session_file);
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
