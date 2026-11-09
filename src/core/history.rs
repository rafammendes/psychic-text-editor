use crate::core::buffer::TextBuffer;
use crate::core::selection::SelectionSet;
use ropey::Rope;
use std::time::{Duration, Instant};

const MAX_HISTORY_ENTRIES: usize = 300;
const COALESCE_WINDOW: Duration = Duration::from_millis(600);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    InsertChar,
    Delete,
    Command,
}

#[derive(Clone, Debug)]
struct HistoryEntry {
    rope: Rope,
    selections: SelectionSet,
    kind: EditKind,
    timestamp: Instant,
}

#[derive(Clone, Debug, Default)]
pub struct EditHistory {
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
}

impl EditHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(
        &mut self,
        buffer: &TextBuffer,
        selections: &SelectionSet,
        kind: EditKind,
        coalesce: bool,
    ) {
        self.redo_stack.clear();
        let now = Instant::now();

        if coalesce && kind == EditKind::InsertChar {
            if let Some(last) = self.undo_stack.last_mut() {
                if last.kind == EditKind::InsertChar
                    && now.duration_since(last.timestamp) <= COALESCE_WINDOW
                {
                    last.timestamp = now;
                    return;
                }
            }
        }

        if self.undo_stack.len() >= MAX_HISTORY_ENTRIES {
            self.undo_stack.remove(0);
        }

        self.undo_stack.push(HistoryEntry {
            rope: buffer.rope().clone(), // O(1) structural sharing in ropey!
            selections: selections.clone(),
            kind,
            timestamp: now,
        });
    }

    pub fn undo(&mut self, buffer: &mut TextBuffer, selections: &mut SelectionSet) -> bool {
        let Some(prev) = self.undo_stack.pop() else {
            return false;
        };

        self.redo_stack.push(HistoryEntry {
            rope: buffer.rope().clone(),
            selections: selections.clone(),
            kind: prev.kind,
            timestamp: Instant::now(),
        });

        let next_ver = buffer.version().wrapping_add(1);
        *buffer = TextBuffer::from_rope(prev.rope, next_ver);
        *selections = prev.selections;
        selections.normalize(buffer.len_chars());
        true
    }

    pub fn redo(&mut self, buffer: &mut TextBuffer, selections: &mut SelectionSet) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };

        self.undo_stack.push(HistoryEntry {
            rope: buffer.rope().clone(),
            selections: selections.clone(),
            kind: next.kind,
            timestamp: Instant::now(),
        });

        let next_ver = buffer.version().wrapping_add(1);
        *buffer = TextBuffer::from_rope(next.rope, next_ver);
        *selections = next.selections;
        selections.normalize(buffer.len_chars());
        true
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::selection::Selection;

    #[test]
    fn test_undo_redo_with_coalescing() {
        let mut buf = TextBuffer::from_str("");
        let mut sel = SelectionSet::single(Selection::caret(0));
        let mut hist = EditHistory::new();

        // Digita "oi" rapidamente (agrupado no mesmo undo)
        hist.record(&buf, &sel, EditKind::InsertChar, true);
        buf.insert(0, "o");
        sel.set_single(Selection::caret(1), buf.len_chars());

        hist.record(&buf, &sel, EditKind::InsertChar, true);
        buf.insert(1, "i");
        sel.set_single(Selection::caret(2), buf.len_chars());

        assert_eq!(buf.to_string_full(), "oi");
        assert!(hist.undo(&mut buf, &mut sel));
        assert_eq!(buf.to_string_full(), "");
        assert_eq!(sel.primary().head, 0);

        assert!(hist.redo(&mut buf, &mut sel));
        assert_eq!(buf.to_string_full(), "oi");
        assert_eq!(sel.primary().head, 2);

        hist.clear();
        assert!(!hist.undo(&mut buf, &mut sel));
    }
}
