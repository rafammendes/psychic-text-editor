use crate::core::buffer::TextBuffer;
use crate::core::history::EditHistory;
use crate::core::selection::{Selection, SelectionSet};
use crate::highlighter::detect_syntax_name;
use crate::icons::file_icon_and_color;
use egui::Color32;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct DocumentTab {
    pub id: u64,
    pub path: Option<PathBuf>,
    pub title: String,
    pub content: String,
    pub saved_content: String,
    pub buffer: TextBuffer,
    pub selections: SelectionSet,
    pub history: EditHistory,
    pub syntax_name: String,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub selection_len: usize,
    pub pending_cursor_char: Option<(usize, usize)>,
    pub scroll_to_primary: bool,
    pub line_ending: &'static str,

    // Ícone pré-computado para evitar alocações de String na barra de abas a 60 FPS
    pub icon: &'static str,
    pub icon_color: Color32,

    // Caches O(1) para evitar varreduras desnecessárias por frame
    cached_line_count: usize,
    last_counted_len: usize,
    cached_dirty: bool,
    last_cursor_char: usize,
}

impl DocumentTab {
    pub fn new_untitled(id: u64, index: usize) -> Self {
        let (icon, icon_color) = file_icon_and_color(None);
        Self {
            id,
            path: None,
            title: format!("untitled-{}", index),
            content: String::new(),
            saved_content: String::new(),
            buffer: TextBuffer::new(),
            selections: SelectionSet::default(),
            history: EditHistory::new(),
            syntax_name: "Plain Text".to_string(),
            cursor_line: 1,
            cursor_col: 1,
            selection_len: 0,
            pending_cursor_char: None,
            scroll_to_primary: false,
            line_ending: "LF",
            icon,
            icon_color,
            cached_line_count: 1,
            last_counted_len: 0,
            cached_dirty: false,
            last_cursor_char: 0,
        }
    }

    pub fn from_file(id: u64, path: PathBuf) -> Result<Self, String> {
        let raw = fs::read_to_string(&path)
            .map_err(|e| format!("Erro ao abrir '{}': {}", path.display(), e))?;

        let line_ending = if raw.contains("\r\n") { "CRLF" } else { "LF" };
        let normalized = if line_ending == "CRLF" {
            raw.replace("\r\n", "\n")
        } else {
            raw
        };

        let first_line = normalized.lines().next().unwrap_or("");
        let syntax_name = detect_syntax_name(Some(&path), first_line);
        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("arquivo")
            .to_string();
        let (icon, icon_color) = file_icon_and_color(Some(&path));

        let buffer = TextBuffer::from_str(&normalized);
        let cached_line_count = buffer.len_lines();
        let last_counted_len = normalized.len();

        Ok(Self {
            id,
            path: Some(path),
            title,
            saved_content: normalized.clone(),
            content: normalized,
            buffer,
            selections: SelectionSet::default(),
            history: EditHistory::new(),
            syntax_name,
            cursor_line: 1,
            cursor_col: 1,
            selection_len: 0,
            pending_cursor_char: None,
            scroll_to_primary: false,
            line_ending,
            icon,
            icon_color,
            cached_line_count,
            last_counted_len,
            cached_dirty: false,
            last_cursor_char: 0,
        })
    }

    /// Chamado quando `self.buffer` (`ropey::Rope`) é editado pelo widget customizado `CodeView`.
    pub fn on_buffer_edited(&mut self) {
        self.content = self.buffer.to_string_full();
        self.cached_line_count = self.buffer.len_lines();
        self.last_counted_len = self.content.len();
        self.cached_dirty =
            self.content.len() != self.saved_content.len() || self.content != self.saved_content;
        self.sync_cursor_from_selections();
    }

    /// Sincroniza `cursor_line`, `cursor_col` e `selection_len` a partir de `self.selections` em O(log n).
    pub fn sync_cursor_from_selections(&mut self) {
        let primary = self.selections.primary();
        let (line, col) = self.buffer.char_to_line_col(primary.head);
        self.cursor_line = line;
        self.cursor_col = col;
        self.selection_len = self
            .selections
            .as_slice()
            .iter()
            .map(Selection::len)
            .sum();
        self.last_cursor_char = primary.head;
    }

    /// Chamado quando `self.content` é modificado diretamente (ex: Replace All ou testes).
    pub fn mark_content_edited(&mut self) {
        self.buffer = TextBuffer::from_str(&self.content);
        self.selections.normalize(self.buffer.len_chars());
        self.cached_line_count = self.buffer.len_lines();
        self.last_counted_len = self.content.len();
        self.cached_dirty =
            self.content.len() != self.saved_content.len() || self.content != self.saved_content;
        self.sync_cursor_from_selections();
    }

    /// Retorna se o arquivo possui alterações não salvas em O(1) nos frames ociosos.
    pub fn is_dirty(&self) -> bool {
        if self.content.len() != self.last_counted_len {
            return self.content.len() != self.saved_content.len()
                || self.content != self.saved_content;
        }
        self.cached_dirty
    }

    pub fn set_path_metadata(&mut self, path: PathBuf) {
        self.title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("arquivo")
            .to_string();
        let first_line = self.buffer.line_text(0);
        self.syntax_name = detect_syntax_name(Some(&path), &first_line);
        let (icon, icon_color) = file_icon_and_color(Some(&path));
        self.icon = icon;
        self.icon_color = icon_color;
        self.path = Some(path);
    }

    pub fn save(&mut self) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Err("Arquivo ainda não possui caminho definido.".to_string());
        };
        self.save_as(path.clone())
    }

    pub fn save_as(&mut self, path: PathBuf) -> Result<(), String> {
        let to_write = if self.line_ending == "CRLF" {
            self.content.replace('\n', "\r\n")
        } else {
            self.content.clone()
        };

        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = fs::create_dir_all(parent);
            }
        }

        fs::write(&path, to_write)
            .map_err(|e| format!("Erro ao salvar '{}': {}", path.display(), e))?;

        self.saved_content = self.content.clone();
        self.cached_dirty = false;
        self.set_path_metadata(path);
        Ok(())
    }

    pub fn revert_from_disk(&mut self) -> Result<(), String> {
        let Some(path) = self.path.clone() else {
            return Err("Este buffer ainda não está associado a um arquivo no disco.".to_string());
        };
        let reloaded = Self::from_file(self.id, path)?;
        let prev_line = self.cursor_line;
        let prev_col = self.cursor_col;
        *self = reloaded;
        self.set_cursor_line_col(prev_line, prev_col);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn line_count(&mut self) -> usize {
        if self.content.len() != self.last_counted_len {
            self.mark_content_edited();
        }
        self.cached_line_count
    }

    pub fn cached_lines(&self) -> usize {
        self.cached_line_count
    }

    pub fn update_cursor_stats(&mut self, char_index: usize, selection_len: usize) {
        self.selection_len = selection_len;
        if char_index == self.last_cursor_char && self.content.len() == self.last_counted_len {
            return;
        }
        self.last_cursor_char = char_index;
        let (line, col) = self.buffer.char_to_line_col(char_index);
        self.cursor_line = line;
        self.cursor_col = col;
    }

    pub fn char_index_for_line(&self, target_line_1based: usize) -> usize {
        self.char_index_for_line_col(target_line_1based, 1)
    }

    pub fn char_index_for_line_col(
        &self,
        target_line_1based: usize,
        target_col_1based: usize,
    ) -> usize {
        if target_line_1based > self.buffer.len_lines() {
            return self.buffer.len_chars();
        }
        self.buffer
            .line_col_to_char(target_line_1based.max(1), target_col_1based.max(1))
    }

    pub fn set_cursor_line_col(&mut self, line: usize, col: usize) {
        let clamped_line = line.clamp(1, self.cached_line_count.max(1));
        let char_pos = self.char_index_for_line_col(clamped_line, col.max(1));
        self.selections
            .set_single(Selection::caret(char_pos), self.buffer.len_chars());
        self.pending_cursor_char = Some((char_pos, char_pos));
        self.scroll_to_primary = true;
        self.sync_cursor_from_selections();
    }

    pub fn matches_path(&self, other: &Path) -> bool {
        let Some(p) = &self.path else {
            return false;
        };
        if p == other {
            return true;
        }
        match (p.canonicalize(), other.canonicalize()) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_untitled_tab_defaults_and_dirty_tracking() {
        let mut tab = DocumentTab::new_untitled(1, 1);
        assert_eq!(tab.title, "untitled-1");
        assert!(!tab.is_dirty());
        assert_eq!(tab.line_count(), 1);

        tab.content.push_str("linha 1\nlinha 2");
        tab.mark_content_edited();
        assert!(tab.is_dirty());
        assert_eq!(tab.line_count(), 2);
        assert_eq!(tab.cached_lines(), 2);
    }

    #[test]
    fn test_char_index_for_line_and_col() {
        let mut tab = DocumentTab::new_untitled(1, 1);
        tab.content = "abc\ndefgh\nij".to_string();
        tab.mark_content_edited();

        assert_eq!(tab.char_index_for_line_col(1, 1), 0);
        assert_eq!(tab.char_index_for_line_col(1, 3), 2);
        // Col beyond line 1 length clamps to end of line 1
        assert_eq!(tab.char_index_for_line_col(1, 99), 3);
        assert_eq!(tab.char_index_for_line_col(2, 1), 4);
        assert_eq!(tab.char_index_for_line_col(2, 4), 7);
        assert_eq!(tab.char_index_for_line_col(3, 2), 11);
        assert_eq!(tab.char_index_for_line_col(99, 1), 12);
    }

    #[test]
    fn test_save_load_and_revert_file() {
        let temp_dir = std::env::temp_dir().join("psychic_editor_tab_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("config.toml");

        let mut tab = DocumentTab::new_untitled(10, 1);
        tab.content = "key = \"value\"\nport = 8080\n".to_string();
        tab.mark_content_edited();
        tab.save_as(file_path.clone()).unwrap();

        assert!(!tab.is_dirty());
        assert_eq!(tab.title, "config.toml");
        assert_eq!(tab.syntax_name, "TOML / Config");
        assert!(tab.matches_path(&file_path));

        // Modify in memory, then revert from disk
        tab.content = "modificado".to_string();
        tab.mark_content_edited();
        assert!(tab.is_dirty());

        tab.revert_from_disk().unwrap();
        assert!(!tab.is_dirty());
        assert!(tab.content.contains("port = 8080"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_crlf_normalization_and_preservation() {
        let temp_dir = std::env::temp_dir().join("psychic_editor_crlf_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("win.ini");

        fs::write(&file_path, "[section]\r\na=1\r\n").unwrap();
        let mut tab = DocumentTab::from_file(1, file_path.clone()).unwrap();
        assert_eq!(tab.line_ending, "CRLF");
        assert_eq!(tab.content, "[section]\na=1\n");

        tab.save().unwrap();
        let raw_on_disk = fs::read_to_string(&file_path).unwrap();
        assert_eq!(raw_on_disk, "[section]\r\na=1\r\n");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_tab_error_cases_and_same_length_dirty_check() {
        let mut tab = DocumentTab::new_untitled(1, 1);
        assert!(tab.save().is_err());
        assert!(tab.revert_from_disk().is_err());
        assert!(!tab.matches_path(Path::new("/tmp/qualquer.txt")));
        assert!(DocumentTab::from_file(2, PathBuf::from("/caminho/inexistente/xyz.rs")).is_err());

        tab.saved_content = "abcd".to_string();
        tab.content = "abce".to_string();
        tab.mark_content_edited();
        assert!(
            tab.is_dirty(),
            "Modificação mantendo o mesmo tamanho deve marcar como dirty"
        );
    }
}
