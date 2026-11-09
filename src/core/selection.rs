use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
    pub preferred_col: Option<usize>,
}

impl Selection {
    #[inline]
    pub fn caret(pos: usize) -> Self {
        Self {
            anchor: pos,
            head: pos,
            preferred_col: None,
        }
    }

    #[inline]
    pub fn new(anchor: usize, head: usize) -> Self {
        Self {
            anchor,
            head,
            preferred_col: None,
        }
    }

    #[inline]
    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    #[inline]
    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.end() - self.start()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    #[inline]
    pub fn range(&self) -> Range<usize> {
        self.start()..self.end()
    }

    pub fn clamp(mut self, max_len: usize) -> Self {
        self.anchor = self.anchor.min(max_len);
        self.head = self.head.min(max_len);
        self
    }

    pub fn overlaps_or_touches(&self, other: &Self) -> bool {
        if self.is_empty() && other.is_empty() {
            return self.head == other.head;
        }
        if self.is_empty() {
            return self.head >= other.start() && self.head <= other.end();
        }
        if other.is_empty() {
            return other.head >= self.start() && other.head <= self.end();
        }
        self.start() <= other.end() && other.start() <= self.end()
    }

    pub fn merge(&self, other: &Self) -> Self {
        let start = self.start().min(other.start());
        let end = self.end().max(other.end());
        if self.head >= self.anchor {
            Self::new(start, end)
        } else {
            Self::new(end, start)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionSet {
    selections: Vec<Selection>,
    primary_index: usize,
}

impl Default for SelectionSet {
    fn default() -> Self {
        Self::single(Selection::caret(0))
    }
}

impl SelectionSet {
    pub fn single(sel: Selection) -> Self {
        Self {
            selections: vec![sel],
            primary_index: 0,
        }
    }

    pub fn from_vec(selections: Vec<Selection>, primary_index: usize, max_len: usize) -> Self {
        let mut set = Self {
            selections,
            primary_index,
        };
        set.normalize(max_len);
        set
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.selections.len()
    }

    #[inline]
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.selections.is_empty()
    }

    #[inline]
    pub fn is_multi(&self) -> bool {
        self.selections.len() > 1
    }

    #[inline]
    pub fn primary(&self) -> Selection {
        self.selections
            .get(self.primary_index)
            .copied()
            .or_else(|| self.selections.first().copied())
            .unwrap_or_else(|| Selection::caret(0))
    }

    #[inline]
    pub fn primary_index(&self) -> usize {
        self.primary_index
    }

    #[inline]
    pub fn as_slice(&self) -> &[Selection] {
        &self.selections
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [Selection] {
        &mut self.selections
    }

    pub fn set_single(&mut self, sel: Selection, max_len: usize) {
        self.selections.clear();
        self.selections.push(sel.clamp(max_len));
        self.primary_index = 0;
    }

    pub fn collapse_to_primary(&mut self) {
        let p = self.primary();
        self.selections.clear();
        self.selections.push(p);
        self.primary_index = 0;
    }

    pub fn add_or_toggle_caret(&mut self, pos: usize, max_len: usize) {
        let clamped = pos.min(max_len);
        if self.selections.len() > 1 {
            if let Some(idx) = self
                .selections
                .iter()
                .position(|s| s.is_empty() && s.head == clamped)
            {
                self.selections.remove(idx);
                if self.primary_index >= self.selections.len() {
                    self.primary_index = self.selections.len() - 1;
                }
                return;
            }
        }
        self.selections.push(Selection::caret(clamped));
        self.primary_index = self.selections.len() - 1;
        self.normalize(max_len);
    }

    pub fn push(&mut self, sel: Selection, max_len: usize) {
        self.selections.push(sel.clamp(max_len));
        self.primary_index = self.selections.len() - 1;
        self.normalize(max_len);
    }

    pub fn normalize(&mut self, max_len: usize) {
        if self.selections.is_empty() {
            self.selections.push(Selection::caret(0));
            self.primary_index = 0;
            return;
        }

        for s in &mut self.selections {
            *s = s.clamp(max_len);
        }

        let primary_head = self.primary().head;
        self.selections
            .sort_by(|a, b| a.start().cmp(&b.start()).then(a.end().cmp(&b.end())));

        let mut merged: Vec<Selection> = Vec::with_capacity(self.selections.len());
        for sel in self.selections.drain(..) {
            if let Some(last) = merged.last_mut() {
                if last.overlaps_or_touches(&sel) {
                    *last = last.merge(&sel);
                    continue;
                }
            }
            merged.push(sel);
        }

        self.primary_index = merged
            .iter()
            .position(|s| s.head == primary_head || (s.start() <= primary_head && primary_head <= s.end()))
            .unwrap_or(merged.len() - 1);
        self.selections = merged;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_normalize_and_merge() {
        let mut set = SelectionSet::from_vec(
            vec![
                Selection::new(10, 15),
                Selection::new(2, 6),
                Selection::new(5, 8),
                Selection::caret(20),
                Selection::caret(20),
            ],
            0,
            100,
        );

        assert!(!set.is_empty());
        assert_eq!(set.len(), 3);
        assert_eq!(set.as_slice()[0], Selection::new(2, 8));
        assert_eq!(set.as_slice()[1], Selection::new(10, 15));
        assert_eq!(set.as_slice()[2], Selection::caret(20));

        set.add_or_toggle_caret(20, 100);
        assert_eq!(set.len(), 2, "Clicar em cima de um cursor existente remove-o");

        set.collapse_to_primary();
        assert_eq!(set.len(), 1);
    }
}
