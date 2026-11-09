use ropey::Rope;
use std::ops::Range;

#[derive(Clone, Debug)]
pub struct TextBuffer {
    rope: Rope,
    version: u64,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            version: 1,
        }
    }

    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            version: 1,
        }
    }

    pub fn from_rope(rope: Rope, version: u64) -> Self {
        Self { rope, version }
    }

    #[inline]
    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    #[inline]
    pub fn version(&self) -> u64 {
        self.version
    }

    #[inline]
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    #[inline]
    #[allow(dead_code)]
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    #[inline]
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    #[inline]
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines().max(1)
    }

    /// Converte índice global de caractere (0-based) para `(linha_1based, coluna_1based)` em O(log n).
    pub fn char_to_line_col(&self, char_idx: usize) -> (usize, usize) {
        let clamped = char_idx.min(self.len_chars());
        let line_0 = self.rope.char_to_line(clamped);
        let line_start = self.rope.line_to_char(line_0);
        let col_0 = clamped.saturating_sub(line_start);
        (line_0 + 1, col_0 + 1)
    }

    /// Converte `(linha_1based, coluna_1based)` para índice global de caractere (0-based) em O(log n).
    pub fn line_col_to_char(&self, line_1based: usize, col_1based: usize) -> usize {
        let total_lines = self.len_lines();
        let line_0 = line_1based.saturating_sub(1).min(total_lines.saturating_sub(1));
        let line_start = self.rope.line_to_char(line_0);
        let line_len = self.line_len_without_newline(line_0);
        let col_0 = col_1based.saturating_sub(1).min(line_len);
        line_start + col_0
    }

    /// Retorna o índice inicial de caractere da linha `line_0` (0-based).
    pub fn line_start_char(&self, line_0: usize) -> usize {
        let clamped = line_0.min(self.len_lines().saturating_sub(1));
        self.rope.line_to_char(clamped)
    }

    /// Retorna o índice final de caractere da linha `line_0` (0-based), sem incluir o `\n`.
    pub fn line_end_char(&self, line_0: usize) -> usize {
        let start = self.line_start_char(line_0);
        start + self.line_len_without_newline(line_0)
    }

    /// Quantidade de caracteres na linha `line_0` (0-based) excluindo `\n` ou `\r\n` final.
    pub fn line_len_without_newline(&self, line_0: usize) -> usize {
        if line_0 >= self.rope.len_lines() {
            return 0;
        }
        let slice = self.rope.line(line_0);
        let mut len = slice.len_chars();
        if len > 0 && slice.char(len - 1) == '\n' {
            len -= 1;
            if len > 0 && slice.char(len - 1) == '\r' {
                len -= 1;
            }
        }
        len
    }

    /// Retorna o texto da linha `line_0` (0-based) sem o `\n` final.
    pub fn line_text(&self, line_0: usize) -> String {
        if line_0 >= self.rope.len_lines() {
            return String::new();
        }
        let slice = self.rope.line(line_0);
        let len = self.line_len_without_newline(line_0);
        slice.slice(0..len).to_string()
    }

    pub fn slice_to_string(&self, range: Range<usize>) -> String {
        let len = self.len_chars();
        let start = range.start.min(len);
        let end = range.end.min(len).max(start);
        self.rope.slice(start..end).to_string()
    }

    pub fn char_at(&self, char_idx: usize) -> Option<char> {
        if char_idx < self.len_chars() {
            Some(self.rope.char(char_idx))
        } else {
            None
        }
    }

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        if text.is_empty() {
            return;
        }
        let pos = char_idx.min(self.len_chars());
        self.rope.insert(pos, text);
        self.version = self.version.wrapping_add(1);
    }

    pub fn remove(&mut self, range: Range<usize>) {
        let len = self.len_chars();
        let start = range.start.min(len);
        let end = range.end.min(len).max(start);
        if start < end {
            self.rope.remove(start..end);
            self.version = self.version.wrapping_add(1);
        }
    }

    pub fn replace_all(&mut self, new_text: &str) {
        self.rope = Rope::from_str(new_text);
        self.version = self.version.wrapping_add(1);
    }

    pub fn to_string_full(&self) -> String {
        self.rope.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rope_buffer_line_col_conversions_and_edits() {
        let mut buf = TextBuffer::from_str("primeira\nsegunda linha\nterceira");
        assert!(!buf.is_empty());
        assert!(buf.len_bytes() > 0);
        assert_eq!(buf.len_lines(), 3);
        assert_eq!(buf.line_text(0), "primeira");
        assert_eq!(buf.line_text(1), "segunda linha");
        assert_eq!(buf.line_text(2), "terceira");

        assert_eq!(buf.char_to_line_col(0), (1, 1));
        assert_eq!(buf.char_to_line_col(8), (1, 9));
        assert_eq!(buf.char_to_line_col(9), (2, 1));
        assert_eq!(buf.char_to_line_col(12), (2, 4));

        assert_eq!(buf.line_col_to_char(2, 4), 12);
        assert_eq!(buf.line_col_to_char(1, 999), 8);

        let v0 = buf.version();
        buf.insert(9, "NOVA ");
        assert!(buf.version() > v0);
        assert_eq!(buf.line_text(1), "NOVA segunda linha");

        buf.remove(9..14);
        assert_eq!(buf.line_text(1), "segunda linha");
    }
}
