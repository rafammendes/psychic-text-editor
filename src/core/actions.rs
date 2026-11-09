use crate::core::buffer::TextBuffer;
use crate::core::history::{EditHistory, EditKind};
use crate::core::selection::{Selection, SelectionSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveDirection {
    Left,
    Right,
    WordLeft,
    WordRight,
    Up,
    Down,
    SmartHome,
    LineEnd,
    PageUp(usize),
    PageDown(usize),
    DocStart,
    DocEnd,
}

/// Move todos os cursores na direção indicada, mantendo ou expandindo a seleção (`extend`).
pub fn move_selections(
    buffer: &TextBuffer,
    selections: &mut SelectionSet,
    dir: MoveDirection,
    extend: bool,
) {
    let max_len = buffer.len_chars();
    for sel in selections.as_mut_slice() {
        let new_head = match dir {
            MoveDirection::Left => {
                sel.preferred_col = None;
                if !extend && !sel.is_empty() {
                    sel.start()
                } else {
                    sel.head.saturating_sub(1)
                }
            }
            MoveDirection::Right => {
                sel.preferred_col = None;
                if !extend && !sel.is_empty() {
                    sel.end()
                } else {
                    (sel.head + 1).min(max_len)
                }
            }
            MoveDirection::WordLeft => {
                sel.preferred_col = None;
                prev_word_boundary(buffer, sel.head)
            }
            MoveDirection::WordRight => {
                sel.preferred_col = None;
                next_word_boundary(buffer, sel.head)
            }
            MoveDirection::Up => move_vertical(buffer, sel, -1),
            MoveDirection::Down => move_vertical(buffer, sel, 1),
            MoveDirection::PageUp(rows) => move_vertical(buffer, sel, -(rows.max(1) as isize)),
            MoveDirection::PageDown(rows) => move_vertical(buffer, sel, rows.max(1) as isize),
            MoveDirection::SmartHome => {
                sel.preferred_col = None;
                let (line_1, _) = buffer.char_to_line_col(sel.head);
                let line_0 = line_1 - 1;
                let line_start = buffer.line_start_char(line_0);
                let line_str = buffer.line_text(line_0);
                let non_ws = line_str
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .count();
                let smart_pos = line_start + non_ws;
                if sel.head == smart_pos {
                    line_start
                } else {
                    smart_pos
                }
            }
            MoveDirection::LineEnd => {
                sel.preferred_col = None;
                let (line_1, _) = buffer.char_to_line_col(sel.head);
                buffer.line_end_char(line_1 - 1)
            }
            MoveDirection::DocStart => {
                sel.preferred_col = None;
                0
            }
            MoveDirection::DocEnd => {
                sel.preferred_col = None;
                max_len
            }
        };

        sel.head = new_head;
        if !extend {
            sel.anchor = new_head;
        }
    }
    selections.normalize(max_len);
}

fn move_vertical(buffer: &TextBuffer, sel: &mut Selection, line_delta: isize) -> usize {
    let (line_1, col_1) = buffer.char_to_line_col(sel.head);
    let pref_col = *sel.preferred_col.get_or_insert(col_1);
    let total_lines = buffer.len_lines() as isize;
    let target_line_1 = ((line_1 as isize) + line_delta).clamp(1, total_lines) as usize;
    buffer.line_col_to_char(target_line_1, pref_col)
}

pub fn prev_word_boundary(buffer: &TextBuffer, mut pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    while pos > 0 {
        let ch = buffer.char_at(pos - 1).unwrap_or(' ');
        if !ch.is_whitespace() {
            break;
        }
        pos -= 1;
    }
    if pos == 0 {
        return 0;
    }
    let first_is_word = is_word_char(buffer.char_at(pos - 1).unwrap_or(' '));
    while pos > 0 {
        let ch = buffer.char_at(pos - 1).unwrap_or(' ');
        if ch.is_whitespace() || is_word_char(ch) != first_is_word {
            break;
        }
        pos -= 1;
    }
    pos
}

pub fn next_word_boundary(buffer: &TextBuffer, mut pos: usize) -> usize {
    let len = buffer.len_chars();
    if pos >= len {
        return len;
    }
    while pos < len {
        let ch = buffer.char_at(pos).unwrap_or(' ');
        if !ch.is_whitespace() {
            break;
        }
        pos += 1;
    }
    if pos >= len {
        return len;
    }
    let first_is_word = is_word_char(buffer.char_at(pos).unwrap_or(' '));
    while pos < len {
        let ch = buffer.char_at(pos).unwrap_or(' ');
        if ch.is_whitespace() || is_word_char(ch) != first_is_word {
            break;
        }
        pos += 1;
    }
    pos
}

#[inline]
pub fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// Retorna o intervalo `(start, end)` da palavra sob `pos`.
pub fn word_range_at(buffer: &TextBuffer, pos: usize) -> (usize, usize) {
    let len = buffer.len_chars();
    if len == 0 {
        return (0, 0);
    }
    let idx = pos.min(len.saturating_sub(1));
    let ch = buffer.char_at(idx).unwrap_or(' ');
    if ch == '\n' {
        return (idx, idx);
    }
    let target_class = is_word_char(ch);
    let mut start = idx;
    while start > 0 {
        let prev = buffer.char_at(start - 1).unwrap_or('\n');
        if prev == '\n' || is_word_char(prev) != target_class || prev.is_whitespace() != ch.is_whitespace() {
            break;
        }
        start -= 1;
    }
    let mut end = idx;
    while end < len {
        let next = buffer.char_at(end).unwrap_or('\n');
        if next == '\n' || is_word_char(next) != target_class || next.is_whitespace() != ch.is_whitespace() {
            break;
        }
        end += 1;
    }
    (start, end)
}

/// Insere texto em todas as seleções ativas (com suporte a auto-fechamento de pares `(`, `[`, `{`, `"`, `'`).
pub fn insert_text(
    buffer: &mut TextBuffer,
    selections: &mut SelectionSet,
    history: &mut EditHistory,
    text: &str,
) {
    if text.is_empty() {
        return;
    }

    let is_single_char = text.chars().count() == 1;
    let ch_opt = if is_single_char {
        text.chars().next()
    } else {
        None
    };
    let coalesce = is_single_char
        && ch_opt.map_or(false, |c| !c.is_whitespace() && !matches!(c, '(' | ')' | '[' | ']' | '{' | '}'));

    history.record(
        buffer,
        selections,
        if coalesce {
            EditKind::InsertChar
        } else {
            EditKind::Command
        },
        coalesce,
    );

    // Verifica se é um par de auto-fechamento
    if let Some(open_ch) = ch_opt {
        if let Some(close_ch) = matching_closing_pair(open_ch) {
            let mut delta: isize = 0;
            for sel in selections.as_mut_slice() {
                let start = ((sel.start() as isize) + delta).max(0) as usize;
                let end = ((sel.end() as isize) + delta).max(0) as usize;

                if start < end {
                    // Envolve a seleção com o par! (ex: selecionar `foo` e digitar `"` -> `"foo"`)
                    let open_s = open_ch.to_string();
                    let close_s = close_ch.to_string();
                    buffer.insert(end, &close_s);
                    buffer.insert(start, &open_s);
                    sel.anchor = start + 1;
                    sel.head = end + 1;
                    delta += 2;
                    continue;
                }

                // Se digitar `"` ou `)` exatamente sobre o caractere de fechamento existente, apenas avança
                if (open_ch == '"' || open_ch == '\'') && buffer.char_at(start) == Some(open_ch) {
                    sel.anchor = start + 1;
                    sel.head = start + 1;
                    continue;
                }

                let pair_str = format!("{}{}", open_ch, close_ch);
                buffer.insert(start, &pair_str);
                sel.anchor = start + 1;
                sel.head = start + 1;
                sel.preferred_col = None;
                delta += 2;
            }
            selections.normalize(buffer.len_chars());
            return;
        }

        // Se digitar `)`, `]`, `}` sobre o próprio fechamento e sem seleção, apenas pula por cima
        if matches!(open_ch, ')' | ']' | '}') {
            let all_match_next = selections
                .as_slice()
                .iter()
                .all(|s| s.is_empty() && buffer.char_at(s.head) == Some(open_ch));
            if all_match_next {
                for sel in selections.as_mut_slice() {
                    sel.head += 1;
                    sel.anchor = sel.head;
                }
                selections.normalize(buffer.len_chars());
                return;
            }
        }
    }

    let inserted_chars = text.chars().count() as isize;
    let mut delta: isize = 0;

    for sel in selections.as_mut_slice() {
        let start = ((sel.start() as isize) + delta).max(0) as usize;
        let end = ((sel.end() as isize) + delta).max(0) as usize;
        let removed = (end - start) as isize;

        if start < end {
            buffer.remove(start..end);
        }
        buffer.insert(start, text);

        let new_caret = (start as isize + inserted_chars).max(0) as usize;
        sel.anchor = new_caret;
        sel.head = new_caret;
        sel.preferred_col = None;
        delta += inserted_chars - removed;
    }

    selections.normalize(buffer.len_chars());
}

fn matching_closing_pair(ch: char) -> Option<char> {
    match ch {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' => Some('"'),
        '\'' => Some('\''),
        _ => None,
    }
}

/// Insere quebra de linha com auto-indentação inteligente em cada cursor.
pub fn insert_newline_auto_indent(
    buffer: &mut TextBuffer,
    selections: &mut SelectionSet,
    history: &mut EditHistory,
) {
    history.record(buffer, selections, EditKind::Command, false);
    let mut delta: isize = 0;

    for sel in selections.as_mut_slice() {
        let start = ((sel.start() as isize) + delta).max(0) as usize;
        let end = ((sel.end() as isize) + delta).max(0) as usize;
        let removed = (end - start) as isize;

        if start < end {
            buffer.remove(start..end);
        }

        let (line_1, _) = buffer.char_to_line_col(start);
        let line_str = buffer.line_text(line_1 - 1);
        let base_indent: String = line_str
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();

        let prev_char = if start > 0 {
            buffer.char_at(start - 1)
        } else {
            None
        };
        let next_char = buffer.char_at(start);

        // Se estiver entre `{}` ou `[]` ou `()`, expande o bloco em duas linhas!
        if matches!(
            (prev_char, next_char),
            (Some('{'), Some('}')) | (Some('['), Some(']')) | (Some('('), Some(')'))
        ) {
            let to_insert = format!("\n{}    \n{}", base_indent, base_indent);
            let inserted_len = to_insert.chars().count() as isize;
            buffer.insert(start, &to_insert);
            let caret_pos = start + 1 + base_indent.chars().count() + 4;
            sel.anchor = caret_pos;
            sel.head = caret_pos;
            sel.preferred_col = None;
            delta += inserted_len - removed;
            continue;
        }

        let extra_indent = if matches!(prev_char, Some('{' | '[' | '(' | ':')) {
            "    "
        } else {
            ""
        };
        let to_insert = format!("\n{}{}", base_indent, extra_indent);
        let inserted_len = to_insert.chars().count() as isize;
        buffer.insert(start, &to_insert);

        let new_caret = (start as isize + inserted_len).max(0) as usize;
        sel.anchor = new_caret;
        sel.head = new_caret;
        sel.preferred_col = None;
        delta += inserted_len - removed;
    }

    selections.normalize(buffer.len_chars());
}

pub fn delete_backward(
    buffer: &mut TextBuffer,
    selections: &mut SelectionSet,
    history: &mut EditHistory,
    by_word: bool,
) {
    history.record(buffer, selections, EditKind::Delete, false);
    let mut delta: isize = 0;

    for sel in selections.as_mut_slice() {
        let start = ((sel.start() as isize) + delta).max(0) as usize;
        let end = ((sel.end() as isize) + delta).max(0) as usize;

        let (del_start, del_end) = if start < end {
            (start, end)
        } else if start == 0 {
            (0, 0)
        } else if by_word {
            (prev_word_boundary(buffer, start), start)
        } else {
            // Se apagar a abertura de um par vazio `()`, `[]`, `{}`, `""`, apaga os dois!
            let prev = buffer.char_at(start - 1);
            let next = buffer.char_at(start);
            if let (Some(p), Some(n)) = (prev, next) {
                if matching_closing_pair(p) == Some(n) {
                    (start - 1, start + 1)
                } else {
                    (start - 1, start)
                }
            } else {
                (start - 1, start)
            }
        };

        if del_start < del_end {
            buffer.remove(del_start..del_end);
            let removed = (del_end - del_start) as isize;
            delta -= removed;
        }
        sel.anchor = del_start;
        sel.head = del_start;
        sel.preferred_col = None;
    }

    selections.normalize(buffer.len_chars());
}

pub fn delete_forward(
    buffer: &mut TextBuffer,
    selections: &mut SelectionSet,
    history: &mut EditHistory,
    by_word: bool,
) {
    history.record(buffer, selections, EditKind::Delete, false);
    let mut delta: isize = 0;

    for sel in selections.as_mut_slice() {
        let start = ((sel.start() as isize) + delta).max(0) as usize;
        let end = ((sel.end() as isize) + delta).max(0) as usize;
        let max_len = buffer.len_chars();

        let (del_start, del_end) = if start < end {
            (start, end)
        } else if start >= max_len {
            (start, start)
        } else if by_word {
            (start, next_word_boundary(buffer, start))
        } else {
            (start, start + 1)
        };

        if del_start < del_end {
            buffer.remove(del_start..del_end);
            let removed = (del_end - del_start) as isize;
            delta -= removed;
        }
        sel.anchor = del_start;
        sel.head = del_start;
        sel.preferred_col = None;
    }

    selections.normalize(buffer.len_chars());
}

/// Indenta (`Tab` ou `Ctrl+]`) ou desindenta (`Shift+Tab` ou `Ctrl+[`).
pub fn indent_or_unindent(
    buffer: &mut TextBuffer,
    selections: &mut SelectionSet,
    history: &mut EditHistory,
    unindent: bool,
) {
    // Se for Tab simples e nenhuma seleção cruzar múltiplas linhas, insere 4 espaços nos cursores
    if !unindent {
        let any_multiline = selections.as_slice().iter().any(|s| {
            !s.is_empty() && buffer.char_to_line_col(s.start()).0 != buffer.char_to_line_col(s.end()).0
        });
        if !any_multiline {
            insert_text(buffer, selections, history, "    ");
            return;
        }
    }

    history.record(buffer, selections, EditKind::Command, false);
    let mut target_lines = std::collections::BTreeSet::new();
    for sel in selections.as_slice() {
        let (l_start, _) = buffer.char_to_line_col(sel.start());
        let (l_end, _) = buffer.char_to_line_col(sel.end());
        for line_1 in l_start..=l_end {
            target_lines.insert(line_1 - 1);
        }
    }

    // Aplica de baixo para cima para não invalidar índices das linhas anteriores
    for &line_0 in target_lines.iter().rev() {
        let line_start = buffer.line_start_char(line_0);
        if unindent {
            let line_str = buffer.line_text(line_0);
            let remove_count = line_str
                .chars()
                .take(4)
                .take_while(|&c| c == ' ' || c == '\t')
                .count();
            if remove_count > 0 {
                buffer.remove(line_start..line_start + remove_count);
                adjust_selections_after_line_edit(selections, line_start, -(remove_count as isize));
            }
        } else {
            buffer.insert(line_start, "    ");
            adjust_selections_after_line_edit(selections, line_start, 4);
        }
    }

    selections.normalize(buffer.len_chars());
}

fn adjust_selections_after_line_edit(selections: &mut SelectionSet, edit_pos: usize, delta: isize) {
    for sel in selections.as_mut_slice() {
        if sel.anchor >= edit_pos {
            sel.anchor = ((sel.anchor as isize) + delta).max(edit_pos as isize) as usize;
        }
        if sel.head >= edit_pos {
            sel.head = ((sel.head as isize) + delta).max(edit_pos as isize) as usize;
        }
    }
}

/// Alterna comentário de linha (`Ctrl+/`) para todas as linhas selecionadas.
pub fn toggle_line_comment(
    buffer: &mut TextBuffer,
    selections: &mut SelectionSet,
    history: &mut EditHistory,
    syntax_name: &str,
) {
    let prefix = match syntax_name {
        "TOML / Config"
        | "INI / Systemd / Conf"
        | "Shell / Bash / Zsh"
        | "YAML"
        | "Python" => "# ",
        "Lua" => "-- ",
        _ => "// ",
    };
    let prefix_trimmed = prefix.trim_end();

    history.record(buffer, selections, EditKind::Command, false);

    let mut target_lines = std::collections::BTreeSet::new();
    for sel in selections.as_slice() {
        let (l_start, _) = buffer.char_to_line_col(sel.start());
        let (l_end, _) = buffer.char_to_line_col(sel.end());
        for line_1 in l_start..=l_end {
            target_lines.insert(line_1 - 1);
        }
    }

    let all_commented = target_lines.iter().all(|&line_0| {
        let text = buffer.line_text(line_0);
        let trimmed = text.trim_start();
        trimmed.starts_with(prefix_trimmed)
    });

    for &line_0 in target_lines.iter().rev() {
        let line_start = buffer.line_start_char(line_0);
        let text = buffer.line_text(line_0);
        let leading_ws = text.chars().take_while(|&c| c == ' ' || c == '\t').count();
        let comment_pos = line_start + leading_ws;

        if all_commented {
            let after_ws = &text[text
                .char_indices()
                .nth(leading_ws)
                .map_or(text.len(), |(i, _)| i)..];
            let remove_chars = if after_ws.starts_with(prefix) {
                prefix.chars().count()
            } else if after_ws.starts_with(prefix_trimmed) {
                prefix_trimmed.chars().count()
            } else {
                0
            };
            if remove_chars > 0 {
                buffer.remove(comment_pos..comment_pos + remove_chars);
                adjust_selections_after_line_edit(
                    selections,
                    comment_pos,
                    -(remove_chars as isize),
                );
            }
        } else {
            let insert_chars = prefix.chars().count() as isize;
            buffer.insert(comment_pos, prefix);
            adjust_selections_after_line_edit(selections, comment_pos, insert_chars);
        }
    }

    selections.normalize(buffer.len_chars());
}

/// `Ctrl+D` do Sublime Text:
/// - Se o cursor primário estiver vazio, seleciona a palavra sob o cursor.
/// - Se já houver uma seleção, busca a próxima ocorrência no documento e adiciona um novo cursor!
pub fn select_next_occurrence(buffer: &TextBuffer, selections: &mut SelectionSet) -> bool {
    let primary = selections.primary();
    if primary.is_empty() {
        let (w_start, w_end) = word_range_at(buffer, primary.head);
        if w_start < w_end {
            let last_idx = selections.primary_index();
            if let Some(s) = selections.as_mut_slice().get_mut(last_idx) {
                *s = Selection::new(w_start, w_end);
            }
            selections.normalize(buffer.len_chars());
            return true;
        }
        return false;
    }

    let needle = buffer.slice_to_string(primary.range());
    if needle.is_empty() {
        return false;
    }

    let full_text = buffer.to_string_full();
    let search_from_char = selections
        .as_slice()
        .last()
        .map_or(primary.end(), |s| s.end());

    // Converte char index para byte index para busca rápida, ou varre ocorrências por char
    let mut all_matches = Vec::new();
    for (byte_idx, _) in full_text.match_indices(&needle) {
        let start_char = full_text[..byte_idx].chars().count();
        let end_char = start_char + needle.chars().count();
        all_matches.push(Selection::new(start_char, end_char));
    }

    let next_match = all_matches
        .iter()
        .find(|m| {
            m.start() >= search_from_char
                && !selections
                    .as_slice()
                    .iter()
                    .any(|s| s.start() == m.start() && s.end() == m.end())
        })
        .or_else(|| {
            all_matches.iter().find(|m| {
                !selections
                    .as_slice()
                    .iter()
                    .any(|s| s.start() == m.start() && s.end() == m.end())
            })
        })
        .copied();

    if let Some(found) = next_match {
        selections.push(found, buffer.len_chars());
        true
    } else {
        false
    }
}

/// `Alt+F3` do Sublime Text: seleciona todas as ocorrências da palavra/seleção atual no arquivo!
pub fn select_all_occurrences(buffer: &TextBuffer, selections: &mut SelectionSet) -> usize {
    let primary = selections.primary();
    let range = if primary.is_empty() {
        let (ws, we) = word_range_at(buffer, primary.head);
        ws..we
    } else {
        primary.range()
    };

    let needle = buffer.slice_to_string(range);
    if needle.is_empty() {
        return 0;
    }

    let full_text = buffer.to_string_full();
    let needle_chars = needle.chars().count();
    let mut matches = Vec::new();
    for (byte_idx, _) in full_text.match_indices(&needle) {
        let start_char = full_text[..byte_idx].chars().count();
        matches.push(Selection::new(start_char, start_char + needle_chars));
    }

    let count = matches.len();
    if count > 0 {
        *selections = SelectionSet::from_vec(matches, count - 1, buffer.len_chars());
    }
    count
}

/// `Ctrl+Shift+L` do Sublime Text: divide uma seleção multi-linha em um cursor no fim de cada linha.
pub fn split_selection_into_lines(buffer: &TextBuffer, selections: &mut SelectionSet) {
    let mut new_sels = Vec::new();
    for sel in selections.as_slice() {
        let (start_line_1, _) = buffer.char_to_line_col(sel.start());
        let (end_line_1, _) = buffer.char_to_line_col(sel.end());
        if start_line_1 == end_line_1 {
            new_sels.push(*sel);
        } else {
            for line_1 in start_line_1..=end_line_1 {
                let line_0 = line_1 - 1;
                let l_start = buffer.line_start_char(line_0).max(sel.start());
                let l_end = buffer.line_end_char(line_0).min(sel.end());
                new_sels.push(Selection::new(l_start, l_end));
            }
        }
    }
    let last = new_sels.len().saturating_sub(1);
    *selections = SelectionSet::from_vec(new_sels, last, buffer.len_chars());
}

/// `Ctrl+Alt+↑` / `Ctrl+Alt+↓`: adiciona um novo cursor na linha acima ou abaixo.
pub fn add_cursor_vertical(buffer: &TextBuffer, selections: &mut SelectionSet, up: bool) {
    let base = if up {
        selections
            .as_slice()
            .first()
            .copied()
            .unwrap_or_else(|| selections.primary())
    } else {
        selections
            .as_slice()
            .last()
            .copied()
            .unwrap_or_else(|| selections.primary())
    };

    let (line_1, col_1) = buffer.char_to_line_col(base.head);
    let target_line_1 = if up {
        line_1.saturating_sub(1).max(1)
    } else {
        (line_1 + 1).min(buffer.len_lines())
    };
    let pos = buffer.line_col_to_char(target_line_1, col_1);
    selections.push(Selection::caret(pos), buffer.len_chars());
}

/// Move a(s) linha(s) atual(is) para cima ou para baixo (`Ctrl+Shift+↑` / `Ctrl+Shift+↓`).
pub fn move_lines_vertical(
    buffer: &mut TextBuffer,
    selections: &mut SelectionSet,
    history: &mut EditHistory,
    up: bool,
) {
    let primary = selections.primary();
    let (line_1, col_1) = buffer.char_to_line_col(primary.head);
    let total_lines = buffer.len_lines();

    if (up && line_1 <= 1) || (!up && line_1 >= total_lines) {
        return;
    }

    history.record(buffer, selections, EditKind::Command, false);
    let mut lines: Vec<String> = (0..total_lines).map(|i| buffer.line_text(i)).collect();
    let idx = line_1 - 1;
    let swap_idx = if up { idx - 1 } else { idx + 1 };
    lines.swap(idx, swap_idx);

    buffer.replace_all(&lines.join("\n"));
    let new_pos = buffer.line_col_to_char(swap_idx + 1, col_1);
    selections.set_single(Selection::caret(new_pos), buffer.len_chars());
}

/// Apaga a linha atual (`Ctrl+Shift+K`).
pub fn delete_current_lines(
    buffer: &mut TextBuffer,
    selections: &mut SelectionSet,
    history: &mut EditHistory,
) {
    history.record(buffer, selections, EditKind::Delete, false);
    let primary = selections.primary();
    let (line_1, col_1) = buffer.char_to_line_col(primary.head);
    let total_lines = buffer.len_lines();
    if total_lines <= 1 {
        buffer.replace_all("");
        selections.set_single(Selection::caret(0), 0);
        return;
    }
    let mut lines: Vec<String> = (0..total_lines).map(|i| buffer.line_text(i)).collect();
    let idx = (line_1 - 1).min(lines.len() - 1);
    lines.remove(idx);
    buffer.replace_all(&lines.join("\n"));
    let new_line_1 = line_1.min(buffer.len_lines());
    let new_pos = buffer.line_col_to_char(new_line_1, col_1);
    selections.set_single(Selection::caret(new_pos), buffer.len_chars());
}

/// Copia o texto das seleções (ou a linha inteira se houver apenas 1 cursor sem seleção).
pub fn copy_selections(buffer: &TextBuffer, selections: &SelectionSet) -> String {
    if selections.len() == 1 && selections.primary().is_empty() {
        let (line_1, _) = buffer.char_to_line_col(selections.primary().head);
        return format!("{}\n", buffer.line_text(line_1 - 1));
    }

    let parts: Vec<String> = selections
        .as_slice()
        .iter()
        .map(|s| buffer.slice_to_string(s.range()))
        .collect();
    parts.join("\n")
}

/// Cola texto em todas as seleções (distribuindo 1 linha por cursor se a quantidade de linhas coincidir).
pub fn paste_text(
    buffer: &mut TextBuffer,
    selections: &mut SelectionSet,
    history: &mut EditHistory,
    clipboard_text: &str,
) {
    if clipboard_text.is_empty() {
        return;
    }
    let normalized = clipboard_text.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized.split('\n').collect();

    if selections.is_multi() && lines.len() == selections.len() {
        history.record(buffer, selections, EditKind::Command, false);
        let mut delta: isize = 0;
        for (idx, sel) in selections.as_mut_slice().iter_mut().enumerate() {
            let start = ((sel.start() as isize) + delta).max(0) as usize;
            let end = ((sel.end() as isize) + delta).max(0) as usize;
            let removed = (end - start) as isize;
            if start < end {
                buffer.remove(start..end);
            }
            let piece = lines[idx];
            let ins_len = piece.chars().count() as isize;
            buffer.insert(start, piece);
            let new_caret = (start as isize + ins_len).max(0) as usize;
            sel.anchor = new_caret;
            sel.head = new_caret;
            delta += ins_len - removed;
        }
        selections.normalize(buffer.len_chars());
    } else {
        insert_text(buffer, selections, history, &normalized);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_cursor_insert_and_ctrl_d() {
        let mut buf = TextBuffer::from_str("let foo = 1;\nlet bar = foo + foo;\n");
        let mut sel = SelectionSet::single(Selection::caret(5)); // dentro do 1º `foo`
        let mut hist = EditHistory::new();

        // 1º Ctrl+D seleciona a palavra `foo`
        assert!(select_next_occurrence(&buf, &mut sel));
        assert_eq!(sel.len(), 1);
        assert_eq!(buf.slice_to_string(sel.primary().range()), "foo");

        // 2º Ctrl+D adiciona o 2º `foo`
        assert!(select_next_occurrence(&buf, &mut sel));
        assert_eq!(sel.len(), 2);

        // 3º Ctrl+D adiciona o 3º `foo`
        assert!(select_next_occurrence(&buf, &mut sel));
        assert_eq!(sel.len(), 3);

        // Digita "valor" nos 3 cursores simultaneamente!
        insert_text(&mut buf, &mut sel, &mut hist, "valor");
        assert_eq!(
            buf.to_string_full(),
            "let valor = 1;\nlet bar = valor + valor;\n"
        );
    }

    #[test]
    fn test_toggle_line_comment_and_indent() {
        let mut buf = TextBuffer::from_str("fn a() {\n    let x = 1;\n}");
        let mut sel = SelectionSet::single(Selection::new(0, 15));
        let mut hist = EditHistory::new();

        toggle_line_comment(&mut buf, &mut sel, &mut hist, "Rust");
        assert!(buf.line_text(0).starts_with("// "));
        assert!(buf.line_text(1).trim_start().starts_with("// "));

        // Toggle novamente descomenta
        toggle_line_comment(&mut buf, &mut sel, &mut hist, "Rust");
        assert_eq!(buf.line_text(0), "fn a() {");
        assert_eq!(buf.line_text(1), "    let x = 1;");

        indent_or_unindent(&mut buf, &mut sel, &mut hist, false);
        assert_eq!(buf.line_text(0), "    fn a() {");
        indent_or_unindent(&mut buf, &mut sel, &mut hist, true);
        assert_eq!(buf.line_text(0), "fn a() {");
    }

    #[test]
    fn test_auto_indent_and_auto_pair() {
        let mut buf = TextBuffer::from_str("fn main() ");
        let mut sel = SelectionSet::single(Selection::caret(buf.len_chars()));
        let mut hist = EditHistory::new();

        // Digita `{` -> insere `{}` com cursor no meio
        insert_text(&mut buf, &mut sel, &mut hist, "{");
        assert_eq!(buf.to_string_full(), "fn main() {}");
        assert_eq!(sel.primary().head, 11);

        // Pressiona Enter entre `{}` -> expande com indentação de 4 espaços!
        insert_newline_auto_indent(&mut buf, &mut sel, &mut hist);
        assert_eq!(buf.to_string_full(), "fn main() {\n    \n}");
        assert_eq!(buf.char_to_line_col(sel.primary().head), (2, 5));
    }

    #[test]
    fn test_select_all_split_lines_and_move_lines() {
        let mut buf = TextBuffer::from_str("aa\nbb\naa\n");
        let mut sel = SelectionSet::single(Selection::caret(0));
        let mut hist = EditHistory::new();

        assert_eq!(select_all_occurrences(&buf, &mut sel), 2);
        assert_eq!(sel.len(), 2);

        sel.set_single(Selection::new(0, 5), buf.len_chars());
        split_selection_into_lines(&buf, &mut sel);
        assert_eq!(sel.len(), 2);

        sel.set_single(Selection::caret(0), buf.len_chars());
        move_lines_vertical(&mut buf, &mut sel, &mut hist, false);
        assert_eq!(buf.line_text(0), "bb");
        assert_eq!(buf.line_text(1), "aa");
        move_lines_vertical(&mut buf, &mut sel, &mut hist, true);
        assert_eq!(buf.line_text(0), "aa");
    }

    #[test]
    fn test_all_move_directions_and_smart_home() {
        let buf = TextBuffer::from_str("    primeira linha\ncurta\n    terceira linha longa");
        let mut sel = SelectionSet::single(Selection::caret(10));

        // SmartHome vai primeiro para o início da indentação (col 5 -> char 4), depois para col 1 (char 0)
        move_selections(&buf, &mut sel, MoveDirection::SmartHome, false);
        assert_eq!(sel.primary().head, 4);
        move_selections(&buf, &mut sel, MoveDirection::SmartHome, false);
        assert_eq!(sel.primary().head, 0);

        // LineEnd vai para o fim da linha 1 (char 18)
        move_selections(&buf, &mut sel, MoveDirection::LineEnd, false);
        assert_eq!(sel.primary().head, 18);

        // Down e Down novamente preservam preferred_col (mesmo passando por "curta")
        move_selections(&buf, &mut sel, MoveDirection::Down, false);
        assert_eq!(buf.char_to_line_col(sel.primary().head), (2, 6));
        move_selections(&buf, &mut sel, MoveDirection::Down, false);
        assert_eq!(buf.char_to_line_col(sel.primary().head), (3, 19));

        // Up volta para a linha 2
        move_selections(&buf, &mut sel, MoveDirection::Up, false);
        assert_eq!(buf.char_to_line_col(sel.primary().head).0, 2);

        // WordLeft e WordRight
        move_selections(&buf, &mut sel, MoveDirection::DocStart, false);
        assert_eq!(sel.primary().head, 0);
        move_selections(&buf, &mut sel, MoveDirection::WordRight, false);
        assert_eq!(sel.primary().head, 12);
        move_selections(&buf, &mut sel, MoveDirection::WordLeft, false);
        assert_eq!(sel.primary().head, 4);

        // Left / Right com e sem extend
        move_selections(&buf, &mut sel, MoveDirection::Right, true);
        assert_eq!(sel.primary(), Selection::new(4, 5));
        move_selections(&buf, &mut sel, MoveDirection::Left, false);
        assert_eq!(sel.primary(), Selection::caret(4));

        // PageDown / PageUp / DocEnd
        move_selections(&buf, &mut sel, MoveDirection::PageDown(10), false);
        assert_eq!(buf.char_to_line_col(sel.primary().head).0, 3);
        move_selections(&buf, &mut sel, MoveDirection::PageUp(10), false);
        assert_eq!(buf.char_to_line_col(sel.primary().head).0, 1);
        move_selections(&buf, &mut sel, MoveDirection::DocEnd, false);
        assert_eq!(sel.primary().head, buf.len_chars());
    }

    #[test]
    fn test_delete_backward_forward_and_auto_pair_wrap_and_skip() {
        let mut buf = TextBuffer::from_str("hello world");
        let mut sel = SelectionSet::single(Selection::new(0, 5)); // "hello"
        let mut hist = EditHistory::new();

        // Envolve a seleção "hello" com aspas `"` -> `"hello" world`
        insert_text(&mut buf, &mut sel, &mut hist, "\"");
        assert_eq!(buf.to_string_full(), "\"hello\" world");
        assert_eq!(sel.primary(), Selection::new(1, 6));

        // Coloca cursor antes do `"` de fechamento e digita `"` -> apenas avança por cima
        sel.set_single(Selection::caret(6), buf.len_chars());
        insert_text(&mut buf, &mut sel, &mut hist, "\"");
        assert_eq!(sel.primary().head, 7);

        // Insere `(` -> `()` e depois digita `)` -> pula por cima
        insert_text(&mut buf, &mut sel, &mut hist, "(");
        assert_eq!(buf.slice_to_string(7..9), "()");
        assert_eq!(sel.primary().head, 8);
        insert_text(&mut buf, &mut sel, &mut hist, ")");
        assert_eq!(sel.primary().head, 9);

        // Volta 1 char para ficar dentro de `()` e pressiona Backspace -> remove o par inteiro `()`!
        sel.set_single(Selection::caret(8), buf.len_chars());
        delete_backward(&mut buf, &mut sel, &mut hist, false);
        assert_eq!(buf.to_string_full(), "\"hello\" world");

        // Ctrl+Backspace (by_word) no fim do buffer apaga "world"
        sel.set_single(Selection::caret(buf.len_chars()), buf.len_chars());
        delete_backward(&mut buf, &mut sel, &mut hist, true);
        assert_eq!(buf.to_string_full(), "\"hello\" ");

        // Ctrl+Delete (by_word) no início apaga `"hello"`
        sel.set_single(Selection::caret(0), buf.len_chars());
        delete_forward(&mut buf, &mut sel, &mut hist, true);
        delete_forward(&mut buf, &mut sel, &mut hist, false);
    }

    #[test]
    fn test_vertical_cursors_delete_lines_and_multi_cursor_clipboard() {
        let mut buf = TextBuffer::from_str("um\ndois\ntres");
        let mut sel = SelectionSet::single(Selection::caret(0));
        let mut hist = EditHistory::new();

        // Adiciona cursores abaixo (Ctrl+Alt+Down)
        add_cursor_vertical(&buf, &mut sel, false);
        add_cursor_vertical(&buf, &mut sel, false);
        assert_eq!(sel.len(), 3);

        // Cola 3 linhas distribuindo 1 linha em cada cursor!
        paste_text(&mut buf, &mut sel, &mut hist, "A-\nB-\nC-");
        assert_eq!(buf.to_string_full(), "A-um\nB-dois\nC-tres");

        // Copy com 1 cursor vazio copia a linha inteira com \n
        sel.set_single(Selection::caret(0), buf.len_chars());
        assert_eq!(copy_selections(&buf, &sel), "A-um\n");

        // Delete line (Ctrl+Shift+K)
        delete_current_lines(&mut buf, &mut sel, &mut hist);
        assert_eq!(buf.to_string_full(), "B-dois\nC-tres");
        delete_current_lines(&mut buf, &mut sel, &mut hist);
        delete_current_lines(&mut buf, &mut sel, &mut hist);
        assert_eq!(buf.to_string_full(), "");
    }
}
