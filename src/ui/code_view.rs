use crate::core::actions::{
    add_cursor_vertical, copy_selections, delete_backward, delete_current_lines, delete_forward,
    indent_or_unindent, insert_newline_auto_indent, insert_text, move_lines_vertical,
    move_selections, paste_text, select_all_occurrences, select_next_occurrence,
    split_selection_into_lines, toggle_line_comment, word_range_at, MoveDirection,
};
use crate::core::selection::Selection;
use crate::highlighter::layout_line_galley;
use crate::tab::DocumentTab;
use egui::{
    pos2, vec2, Align, Color32, CursorIcon, Event, EventFilter, FontFamily, FontId, Id, ImeEvent,
    Key, Modifiers, PointerButton, Pos2, Rect, Sense, Stroke, StrokeKind, Ui,
};

pub struct CodeViewOutput {
    pub text_changed: bool,
    pub cursor_changed: bool,
    pub status_hint: Option<String>,
}

#[derive(Clone, Default)]
struct CodeViewTempState {
    ime_preedit: String,
    drag_mode: DragSelectMode,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum DragSelectMode {
    #[default]
    Char,
    Word(usize, usize),
    Line(usize),
}

pub struct CodeView<'a> {
    pub tab: &'a mut DocumentTab,
    pub theme_name: &'a str,
    pub font_size: f32,
    pub show_minimap: bool,
    pub search_query: &'a str,
    pub search_case_sensitive: bool,
    pub steal_focus_if_idle: bool,
}

impl<'a> CodeView<'a> {
    pub fn show(&mut self, ui: &mut Ui) -> CodeViewOutput {
        let mut output = CodeViewOutput {
            text_changed: false,
            cursor_changed: false,
            status_hint: None,
        };

        let editor_id = Id::new(("psychic_custom_code_view", self.tab.id));
        let state_id = Id::new(("psychic_custom_code_state", self.tab.id));
        let mut temp_state = ui
            .ctx()
            .data_mut(|d| d.get_temp::<CodeViewTempState>(state_id))
            .unwrap_or_default();

        let font_id = FontId::new(self.font_size, FontFamily::Monospace);
        let char_width = ui
            .fonts(|f| f.glyph_width(&font_id, ' '))
            .max(self.font_size * 0.58);
        let line_height = (self.font_size * 1.40).round().max(15.0);

        let total_lines = self.tab.buffer.len_lines().max(1);
        let digits = total_lines.to_string().len().max(3);
        let gutter_width = ((digits as f32 + 3.5) * char_width).round();
        let code_pad_left = 10.0;
        let code_pad_top = 6.0;

        // Aplica pulo de cursor pendente (Goto Line / Find / Restauração de sessão)
        if let Some((start_char, end_char)) = self.tab.pending_cursor_char.take() {
            let max_len = self.tab.buffer.len_chars();
            self.tab
                .selections
                .set_single(Selection::new(start_char, end_char), max_len);
            self.tab.sync_cursor_from_selections();
            self.tab.scroll_to_primary = true;
            output.cursor_changed = true;
        }

        // Mantém o foco no editor quando nenhum modal/input externo está focado
        if self.steal_focus_if_idle {
            let focused = ui.ctx().memory(|m| m.focused());
            if focused.is_none() {
                ui.ctx().memory_mut(|m| m.request_focus(editor_id));
            }
        }

        let has_focus = ui.ctx().memory(|m| m.has_focus(editor_id));
        if has_focus {
            ui.ctx().memory_mut(|m| {
                m.set_focus_lock_filter(
                    editor_id,
                    EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                );
            });
        }

        // Processa eventos de teclado e clipboard quando o widget está focado
        let prev_version = self.tab.buffer.version();
        let prev_primary = self.tab.selections.primary();
        let prev_sel_count = self.tab.selections.len();

        let visible_rows_approx =
            ((ui.available_height() / line_height).floor() as usize).max(5);

        if has_focus {
            self.handle_keyboard_and_clipboard(
                ui,
                visible_rows_approx,
                &mut temp_state,
                &mut output,
            );
        }

        if self.tab.buffer.version() != prev_version {
            self.tab.on_buffer_edited();
            self.tab.scroll_to_primary = true;
            output.text_changed = true;
            output.cursor_changed = true;
        } else if self.tab.selections.primary() != prev_primary
            || self.tab.selections.len() != prev_sel_count
        {
            self.tab.sync_cursor_from_selections();
            output.cursor_changed = true;
        }

        // Calcula largura disponível descontando o minimapa (se ativo)
        let full_rect = ui.available_rect_before_wrap();
        let minimap_width = if self.show_minimap && full_rect.width() > 340.0 {
            84.0
        } else {
            0.0
        };

        let mut target_scroll_y: Option<f32> = None;

        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let main_width = (full_rect.width() - minimap_width).max(120.0);

            ui.allocate_ui_with_layout(
                vec2(main_width, full_rect.height()),
                egui::Layout::top_down(Align::Min),
                |ui| {
                    let mut scroll_area = egui::ScrollArea::both()
                        .id_salt(("custom_code_scroll", self.tab.id))
                        .auto_shrink([false, false]);

                    if let Some(y) = ui
                        .ctx()
                        .data_mut(|d| d.remove_temp::<f32>(Id::new(("minimap_scroll_y", self.tab.id))))
                    {
                        scroll_area = scroll_area.vertical_scroll_offset(y);
                    }

                    let scroll_out = scroll_area.show_viewport(ui, |ui, viewport| {
                        let total_lines_now = self.tab.buffer.len_lines().max(1);
                        let first_line =
                            ((viewport.min.y - code_pad_top) / line_height).floor().max(0.0)
                                as usize;
                        let last_line = (((viewport.max.y - code_pad_top) / line_height).ceil()
                            as usize
                            + 1)
                            .min(total_lines_now.saturating_sub(1));

                        // Estima largura máxima das linhas visíveis para a barra horizontal
                        let mut max_line_chars = 80usize;
                        for line_0 in first_line..=last_line {
                            max_line_chars =
                                max_line_chars.max(self.tab.buffer.line_len_without_newline(line_0));
                        }

                        let virtual_width = (gutter_width
                            + code_pad_left
                            + (max_line_chars as f32 + 16.0) * char_width)
                            .max(viewport.width());
                        let virtual_height =
                            (code_pad_top * 2.0 + (total_lines_now as f32 + 4.0) * line_height)
                                .max(viewport.height());

                        let (canvas_rect, response) = ui.allocate_exact_size(
                            vec2(virtual_width, virtual_height),
                            Sense::click_and_drag(),
                        );

                        // Define cursor I-beam quando sobre a área de código e seta padrão sobre o gutter
                        if let Some(hover_pos) = response.hover_pos() {
                            let gutter_right =
                                canvas_rect.min.x + viewport.min.x + gutter_width;
                            if hover_pos.x > gutter_right {
                                ui.ctx().set_cursor_icon(CursorIcon::Text);
                            } else {
                                ui.ctx().set_cursor_icon(CursorIcon::Default);
                            }
                        }

                        // Interações de mouse (Clique, Ctrl+Clique, Duplo-Clique, Triplo-Clique, Arraste)
                        self.handle_mouse_interaction(
                            ui,
                            &response,
                            editor_id,
                            canvas_rect,
                            viewport,
                            gutter_width,
                            code_pad_left,
                            code_pad_top,
                            char_width,
                            line_height,
                            &mut temp_state,
                            &mut output,
                        );

                        // Rola automaticamente até o cursor primário quando solicitado
                        if self.tab.scroll_to_primary {
                            self.tab.scroll_to_primary = false;
                            let primary = self.tab.selections.primary();
                            let (line_1, col_1) = self.tab.buffer.char_to_line_col(primary.head);
                            let caret_x = canvas_rect.min.x
                                + gutter_width
                                + code_pad_left
                                + (col_1.saturating_sub(1) as f32) * char_width;
                            let caret_y = canvas_rect.min.y
                                + code_pad_top
                                + (line_1.saturating_sub(1) as f32) * line_height;
                            let target_rect = Rect::from_min_size(
                                pos2(caret_x, caret_y),
                                vec2(char_width * 2.0, line_height),
                            )
                            .expand2(vec2(char_width * 4.0, line_height * 1.5));
                            ui.scroll_to_rect(target_rect, None);
                        }

                        // Pintura virtualizada das camadas visíveis
                        self.paint_visible_layers(
                            ui,
                            canvas_rect,
                            viewport,
                            first_line,
                            last_line,
                            gutter_width,
                            code_pad_left,
                            code_pad_top,
                            char_width,
                            line_height,
                            &font_id,
                            has_focus,
                            &temp_state,
                        );

                        (first_line, last_line, virtual_height)
                    });

                    let (first_line, last_line, virtual_height) = scroll_out.inner;
                    let scroll_y = scroll_out.state.offset.y;

                    // Salva métricas da viewport para o minimapa
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(
                            Id::new(("viewport_lines", self.tab.id)),
                            (first_line, last_line, scroll_y, virtual_height),
                        );
                    });
                },
            );

            // Renderiza Minimapa interativo na direita se habilitado
            if minimap_width > 0.0 {
                let (first_line, last_line, _scroll_y, virtual_height) = ui
                    .ctx()
                    .data(|d| d.get_temp::<(usize, usize, f32, f32)>(Id::new(("viewport_lines", self.tab.id))))
                    .unwrap_or((0, 40, 0.0, 1000.0));

                if let Some(new_y) = self.paint_minimap(
                    ui,
                    minimap_width,
                    full_rect.height(),
                    first_line,
                    last_line,
                    virtual_height,
                ) {
                    target_scroll_y = Some(new_y);
                }
            }
        });

        if let Some(new_y) = target_scroll_y {
            ui.ctx()
                .data_mut(|d| d.insert_temp(Id::new(("minimap_scroll_y", self.tab.id)), new_y));
            ui.ctx().request_repaint();
        }

        ui.ctx()
            .data_mut(|d| d.insert_temp(state_id, temp_state));

        output
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_mouse_interaction(
        &mut self,
        ui: &Ui,
        response: &egui::Response,
        editor_id: Id,
        canvas_rect: Rect,
        viewport: Rect,
        gutter_width: f32,
        code_pad_left: f32,
        code_pad_top: f32,
        char_width: f32,
        line_height: f32,
        temp_state: &mut CodeViewTempState,
        output: &mut CodeViewOutput,
    ) {
        let pointer_down_now = ui.input(|i| i.pointer.button_pressed(PointerButton::Primary));
        let modifiers = ui.input(|i| i.modifiers);
        let max_len = self.tab.buffer.len_chars();

        if pointer_down_now && response.hovered() {
            ui.ctx().memory_mut(|m| m.request_focus(editor_id));
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let gutter_right = canvas_rect.min.x + viewport.min.x + gutter_width;
                let clicked_line_0 = self.pos_to_line_0(pos, canvas_rect, code_pad_top, line_height);

                if pos.x <= gutter_right {
                    // Clique no gutter seleciona a linha inteira
                    let l_start = self.tab.buffer.line_start_char(clicked_line_0);
                    let l_end = (self.tab.buffer.line_end_char(clicked_line_0) + 1).min(max_len);
                    self.tab
                        .selections
                        .set_single(Selection::new(l_start, l_end), max_len);
                    temp_state.drag_mode = DragSelectMode::Line(clicked_line_0);
                } else if response.triple_clicked() {
                    let l_start = self.tab.buffer.line_start_char(clicked_line_0);
                    let l_end = (self.tab.buffer.line_end_char(clicked_line_0) + 1).min(max_len);
                    self.tab
                        .selections
                        .set_single(Selection::new(l_start, l_end), max_len);
                    temp_state.drag_mode = DragSelectMode::Line(clicked_line_0);
                } else if response.double_clicked() {
                    let char_idx = self.pos_to_char_idx(
                        pos,
                        canvas_rect,
                        gutter_width,
                        code_pad_left,
                        code_pad_top,
                        char_width,
                        line_height,
                    );
                    let (w_start, w_end) = word_range_at(&self.tab.buffer, char_idx);
                    self.tab
                        .selections
                        .set_single(Selection::new(w_start, w_end), max_len);
                    temp_state.drag_mode = DragSelectMode::Word(w_start, w_end);
                } else {
                    let char_idx = self.pos_to_char_idx(
                        pos,
                        canvas_rect,
                        gutter_width,
                        code_pad_left,
                        code_pad_top,
                        char_width,
                        line_height,
                    );
                    temp_state.drag_mode = DragSelectMode::Char;

                    if modifiers.ctrl || modifiers.command {
                        self.tab.selections.add_or_toggle_caret(char_idx, max_len);
                        output.status_hint = Some(format!(
                            "{} cursores ativos",
                            self.tab.selections.len()
                        ));
                    } else if modifiers.shift {
                        let p_idx = self.tab.selections.primary_index();
                        if let Some(sel) = self.tab.selections.as_mut_slice().get_mut(p_idx) {
                            sel.head = char_idx;
                            sel.preferred_col = None;
                        }
                        self.tab.selections.normalize(max_len);
                    } else {
                        self.tab
                            .selections
                            .set_single(Selection::caret(char_idx), max_len);
                    }
                }
                self.tab.sync_cursor_from_selections();
                output.cursor_changed = true;
            }
        } else if response.dragged_by(PointerButton::Primary) {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let char_idx = self.pos_to_char_idx(
                    pos,
                    canvas_rect,
                    gutter_width,
                    code_pad_left,
                    code_pad_top,
                    char_width,
                    line_height,
                );
                let p_idx = self.tab.selections.primary_index();
                match temp_state.drag_mode {
                    DragSelectMode::Char => {
                        if let Some(sel) = self.tab.selections.as_mut_slice().get_mut(p_idx) {
                            sel.head = char_idx;
                            sel.preferred_col = None;
                        }
                    }
                    DragSelectMode::Word(orig_start, orig_end) => {
                        let (w_start, w_end) = word_range_at(&self.tab.buffer, char_idx);
                        if let Some(sel) = self.tab.selections.as_mut_slice().get_mut(p_idx) {
                            if w_start < orig_start {
                                sel.anchor = orig_end;
                                sel.head = w_start;
                            } else {
                                sel.anchor = orig_start;
                                sel.head = w_end;
                            }
                        }
                    }
                    DragSelectMode::Line(orig_line_0) => {
                        let cur_line_0 =
                            self.pos_to_line_0(pos, canvas_rect, code_pad_top, line_height);
                        if let Some(sel) = self.tab.selections.as_mut_slice().get_mut(p_idx) {
                            if cur_line_0 < orig_line_0 {
                                sel.anchor =
                                    (self.tab.buffer.line_end_char(orig_line_0) + 1).min(max_len);
                                sel.head = self.tab.buffer.line_start_char(cur_line_0);
                            } else {
                                sel.anchor = self.tab.buffer.line_start_char(orig_line_0);
                                sel.head =
                                    (self.tab.buffer.line_end_char(cur_line_0) + 1).min(max_len);
                            }
                        }
                    }
                }
                self.tab.selections.normalize(max_len);
                self.tab.sync_cursor_from_selections();
                output.cursor_changed = true;
            }
        }
    }

    fn pos_to_line_0(
        &self,
        pos: Pos2,
        canvas_rect: Rect,
        code_pad_top: f32,
        line_height: f32,
    ) -> usize {
        let rel_y = (pos.y - canvas_rect.min.y - code_pad_top).max(0.0);
        let line_0 = (rel_y / line_height).floor() as usize;
        line_0.min(self.tab.buffer.len_lines().saturating_sub(1))
    }

    #[allow(clippy::too_many_arguments)]
    fn pos_to_char_idx(
        &self,
        pos: Pos2,
        canvas_rect: Rect,
        gutter_width: f32,
        code_pad_left: f32,
        code_pad_top: f32,
        char_width: f32,
        line_height: f32,
    ) -> usize {
        let line_0 = self.pos_to_line_0(pos, canvas_rect, code_pad_top, line_height);
        let code_origin_x = canvas_rect.min.x + gutter_width + code_pad_left;
        let rel_x = (pos.x - code_origin_x).max(0.0);
        let col_0 = (rel_x / char_width).round() as usize;
        self.tab.buffer.line_col_to_char(line_0 + 1, col_0 + 1)
    }

    fn handle_keyboard_and_clipboard(
        &mut self,
        ui: &mut Ui,
        visible_rows: usize,
        temp_state: &mut CodeViewTempState,
        output: &mut CodeViewOutput,
    ) {
        let events = ui.input(|i| i.events.clone());

        for event in events {
            match event {
                Event::Copy => {
                    let copied = copy_selections(&self.tab.buffer, &self.tab.selections);
                    ui.ctx().copy_text(copied);
                }
                Event::Cut => {
                    let copied = copy_selections(&self.tab.buffer, &self.tab.selections);
                    ui.ctx().copy_text(copied);
                    if self.tab.selections.len() == 1 && self.tab.selections.primary().is_empty() {
                        delete_current_lines(
                            &mut self.tab.buffer,
                            &mut self.tab.selections,
                            &mut self.tab.history,
                        );
                    } else {
                        delete_backward(
                            &mut self.tab.buffer,
                            &mut self.tab.selections,
                            &mut self.tab.history,
                            false,
                        );
                    }
                }
                Event::Paste(text) => {
                    paste_text(
                        &mut self.tab.buffer,
                        &mut self.tab.selections,
                        &mut self.tab.history,
                        &text,
                    );
                }
                Event::Ime(ime_ev) => match ime_ev {
                    ImeEvent::Preedit(text) => {
                        temp_state.ime_preedit = text;
                    }
                    ImeEvent::Commit(text) => {
                        temp_state.ime_preedit.clear();
                        if !text.is_empty() {
                            insert_text(
                                &mut self.tab.buffer,
                                &mut self.tab.selections,
                                &mut self.tab.history,
                                &text,
                            );
                        }
                    }
                    ImeEvent::Disabled | ImeEvent::Enabled => {
                        temp_state.ime_preedit.clear();
                    }
                },
                Event::Text(text) => {
                    let filtered: String = text
                        .chars()
                        .filter(|c| !c.is_control() || *c == '\t')
                        .collect();
                    if !filtered.is_empty() {
                        insert_text(
                            &mut self.tab.buffer,
                            &mut self.tab.selections,
                            &mut self.tab.history,
                            &filtered,
                        );
                    }
                }
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    self.handle_key_press(key, modifiers, visible_rows, output);
                }
                _ => {}
            }
        }
    }

    fn handle_key_press(
        &mut self,
        key: Key,
        modifiers: Modifiers,
        visible_rows: usize,
        output: &mut CodeViewOutput,
    ) {
        let ctrl = modifiers.ctrl || modifiers.command;
        let shift = modifiers.shift;
        let alt = modifiers.alt;

        match key {
            // Sublime Multi-Cursor: Ctrl+D (próxima ocorrência)
            Key::D if ctrl && !shift && !alt => {
                if select_next_occurrence(&self.tab.buffer, &mut self.tab.selections) {
                    self.tab.scroll_to_primary = true;
                    output.status_hint = Some(format!(
                        "{} seleção(ões) ativa(s)",
                        self.tab.selections.len()
                    ));
                }
            }
            // Sublime Select All Occurrences: Alt+F3
            Key::F3 if alt && !ctrl => {
                let count = select_all_occurrences(&self.tab.buffer, &mut self.tab.selections);
                if count > 0 {
                    output.status_hint = Some(format!("{} ocorrências selecionadas", count));
                }
            }
            // Sublime Split Selection into Lines: Ctrl+Shift+L
            Key::L if ctrl && shift && !alt => {
                split_selection_into_lines(&self.tab.buffer, &mut self.tab.selections);
                output.status_hint = Some(format!(
                    "Dividido em {} cursores",
                    self.tab.selections.len()
                ));
            }
            // Sublime Column Cursor: Ctrl+Alt+Up / Ctrl+Alt+Down
            Key::ArrowUp if ctrl && alt && !shift => {
                add_cursor_vertical(&self.tab.buffer, &mut self.tab.selections, true);
                self.tab.scroll_to_primary = true;
                output.status_hint = Some(format!(
                    "{} cursores ativos",
                    self.tab.selections.len()
                ));
            }
            Key::ArrowDown if ctrl && alt && !shift => {
                add_cursor_vertical(&self.tab.buffer, &mut self.tab.selections, false);
                self.tab.scroll_to_primary = true;
                output.status_hint = Some(format!(
                    "{} cursores ativos",
                    self.tab.selections.len()
                ));
            }
            // Move Lines Up/Down: Ctrl+Shift+Up / Ctrl+Shift+Down
            Key::ArrowUp if ctrl && shift && !alt => {
                move_lines_vertical(
                    &mut self.tab.buffer,
                    &mut self.tab.selections,
                    &mut self.tab.history,
                    true,
                );
            }
            Key::ArrowDown if ctrl && shift && !alt => {
                move_lines_vertical(
                    &mut self.tab.buffer,
                    &mut self.tab.selections,
                    &mut self.tab.history,
                    false,
                );
            }
            // Delete Line: Ctrl+Shift+K
            Key::K if ctrl && shift && !alt => {
                delete_current_lines(
                    &mut self.tab.buffer,
                    &mut self.tab.selections,
                    &mut self.tab.history,
                );
                output.status_hint = Some("Linha removida.".to_string());
            }
            // Toggle Line Comment: Ctrl+/
            Key::Slash if ctrl && !alt => {
                let syn = self.tab.syntax_name.clone();
                toggle_line_comment(
                    &mut self.tab.buffer,
                    &mut self.tab.selections,
                    &mut self.tab.history,
                    &syn,
                );
            }
            // Undo: Ctrl+Z
            Key::Z if ctrl && !shift && !alt => {
                if self
                    .tab
                    .history
                    .undo(&mut self.tab.buffer, &mut self.tab.selections)
                {
                    output.status_hint = Some("Desfeito (Undo)".to_string());
                }
            }
            // Redo: Ctrl+Y ou Ctrl+Shift+Z
            Key::Y if ctrl && !shift && !alt => {
                if self
                    .tab
                    .history
                    .redo(&mut self.tab.buffer, &mut self.tab.selections)
                {
                    output.status_hint = Some("Refeito (Redo)".to_string());
                }
            }
            Key::Z if ctrl && shift && !alt => {
                if self
                    .tab
                    .history
                    .redo(&mut self.tab.buffer, &mut self.tab.selections)
                {
                    output.status_hint = Some("Refeito (Redo)".to_string());
                }
            }
            // Select All: Ctrl+A
            Key::A if ctrl && !shift && !alt => {
                let max_len = self.tab.buffer.len_chars();
                self.tab
                    .selections
                    .set_single(Selection::new(0, max_len), max_len);
            }
            // Escape: colapsa múltiplos cursores para um único cursor
            Key::Escape => {
                if self.tab.selections.is_multi() {
                    self.tab.selections.collapse_to_primary();
                    output.status_hint = Some("Cursores recolhidos para 1.".to_string());
                } else if !self.tab.selections.primary().is_empty() {
                    let head = self.tab.selections.primary().head;
                    self.tab
                        .selections
                        .set_single(Selection::caret(head), self.tab.buffer.len_chars());
                }
            }
            // Edição básica
            Key::Backspace => {
                delete_backward(
                    &mut self.tab.buffer,
                    &mut self.tab.selections,
                    &mut self.tab.history,
                    ctrl,
                );
            }
            Key::Delete => {
                delete_forward(
                    &mut self.tab.buffer,
                    &mut self.tab.selections,
                    &mut self.tab.history,
                    ctrl,
                );
            }
            Key::Enter => {
                insert_newline_auto_indent(
                    &mut self.tab.buffer,
                    &mut self.tab.selections,
                    &mut self.tab.history,
                );
            }
            Key::Tab => {
                indent_or_unindent(
                    &mut self.tab.buffer,
                    &mut self.tab.selections,
                    &mut self.tab.history,
                    shift,
                );
            }
            // Navegação de cursores
            Key::ArrowLeft if !alt => {
                let dir = if ctrl {
                    MoveDirection::WordLeft
                } else {
                    MoveDirection::Left
                };
                move_selections(&self.tab.buffer, &mut self.tab.selections, dir, shift);
                self.tab.scroll_to_primary = true;
            }
            Key::ArrowRight if !alt => {
                let dir = if ctrl {
                    MoveDirection::WordRight
                } else {
                    MoveDirection::Right
                };
                move_selections(&self.tab.buffer, &mut self.tab.selections, dir, shift);
                self.tab.scroll_to_primary = true;
            }
            Key::ArrowUp if !ctrl && !alt => {
                move_selections(
                    &self.tab.buffer,
                    &mut self.tab.selections,
                    MoveDirection::Up,
                    shift,
                );
                self.tab.scroll_to_primary = true;
            }
            Key::ArrowDown if !ctrl && !alt => {
                move_selections(
                    &self.tab.buffer,
                    &mut self.tab.selections,
                    MoveDirection::Down,
                    shift,
                );
                self.tab.scroll_to_primary = true;
            }
            Key::Home => {
                let dir = if ctrl {
                    MoveDirection::DocStart
                } else {
                    MoveDirection::SmartHome
                };
                move_selections(&self.tab.buffer, &mut self.tab.selections, dir, shift);
                self.tab.scroll_to_primary = true;
            }
            Key::End => {
                let dir = if ctrl {
                    MoveDirection::DocEnd
                } else {
                    MoveDirection::LineEnd
                };
                move_selections(&self.tab.buffer, &mut self.tab.selections, dir, shift);
                self.tab.scroll_to_primary = true;
            }
            Key::PageUp => {
                move_selections(
                    &self.tab.buffer,
                    &mut self.tab.selections,
                    MoveDirection::PageUp(visible_rows),
                    shift,
                );
                self.tab.scroll_to_primary = true;
            }
            Key::PageDown => {
                move_selections(
                    &self.tab.buffer,
                    &mut self.tab.selections,
                    MoveDirection::PageDown(visible_rows),
                    shift,
                );
                self.tab.scroll_to_primary = true;
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_visible_layers(
        &self,
        ui: &Ui,
        canvas_rect: Rect,
        viewport: Rect,
        first_line: usize,
        last_line: usize,
        gutter_width: f32,
        code_pad_left: f32,
        code_pad_top: f32,
        char_width: f32,
        line_height: f32,
        font_id: &FontId,
        has_focus: bool,
        temp_state: &CodeViewTempState,
    ) {
        let painter = ui.painter_at(canvas_rect);
        let code_origin_x = canvas_rect.min.x + gutter_width + code_pad_left;
        let gutter_left_x = canvas_rect.min.x + viewport.min.x;
        let visible_right_x = canvas_rect.min.x + viewport.max.x;

        // Conjunto de linhas que possuem cursor ativo
        let mut active_cursor_lines = std::collections::BTreeSet::new();
        for sel in self.tab.selections.as_slice() {
            let (line_1, _) = self.tab.buffer.char_to_line_col(sel.head);
            active_cursor_lines.insert(line_1 - 1);
        }

        // 1. Destaque sutil das linhas com cursor ativo (highlight_line)
        for &line_0 in &active_cursor_lines {
            if line_0 >= first_line && line_0 <= last_line {
                let y = canvas_rect.min.y + code_pad_top + (line_0 as f32) * line_height;
                let row_rect = Rect::from_min_max(
                    pos2(gutter_left_x + gutter_width, y),
                    pos2(visible_right_x, y + line_height),
                );
                painter.rect_filled(
                    row_rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 10),
                );
            }
        }

        // Palavra atualmente selecionada para destacar ocorrências gêmeas nas linhas visíveis
        let primary_sel = self.tab.selections.primary();
        let highlight_word = if !primary_sel.is_empty() && primary_sel.len() >= 2 && primary_sel.len() <= 48 {
            let s = self.tab.buffer.slice_to_string(primary_sel.range());
            if !s.contains('\n') && s.chars().all(|c| !c.is_whitespace()) {
                Some(s)
            } else {
                None
            }
        } else {
            None
        };

        // 2. Guias de indentação, destaques de busca/ocorrências, seleções e texto com syntax highlight
        for line_0 in first_line..=last_line {
            let line_y = canvas_rect.min.y + code_pad_top + (line_0 as f32) * line_height;
            let line_str = self.tab.buffer.line_text(line_0);
            let line_start_char = self.tab.buffer.line_start_char(line_0);
            let line_len_chars = self.tab.buffer.line_len_without_newline(line_0);
            let line_end_char = line_start_char + line_len_chars;

            // 2a. Guias verticais de indentação a cada 4 espaços
            let leading_spaces = line_str.chars().take_while(|&c| c == ' ').count();
            if leading_spaces >= 4 {
                let levels = leading_spaces / 4;
                for lvl in 1..=levels {
                    let gx = code_origin_x + ((lvl * 4 - 4) as f32) * char_width + 1.0;
                    painter.line_segment(
                        [pos2(gx, line_y + 1.0), pos2(gx, line_y + line_height - 1.0)],
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 16)),
                    );
                }
            }

            // 2b. Destaques de ocorrências da palavra selecionada
            if let Some(ref word) = highlight_word {
                for (byte_idx, _) in line_str.match_indices(word.as_str()) {
                    let col_start = line_str[..byte_idx].chars().count();
                    let col_end = col_start + word.chars().count();
                    let abs_start = line_start_char + col_start;
                    let abs_end = line_start_char + col_end;
                    if abs_start == primary_sel.start() && abs_end == primary_sel.end() {
                        continue;
                    }
                    let rx1 = code_origin_x + (col_start as f32) * char_width;
                    let rx2 = code_origin_x + (col_end as f32) * char_width;
                    let rect =
                        Rect::from_min_max(pos2(rx1, line_y + 1.0), pos2(rx2, line_y + line_height - 1.0));
                    painter.rect(
                        rect,
                        2.0,
                        Color32::from_rgba_unmultiplied(180, 205, 255, 18),
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(180, 205, 255, 65)),
                        StrokeKind::Inside,
                    );
                }
            }

            // 2c. Destaques da barra de busca (Ctrl+F)
            if !self.search_query.is_empty() {
                let (hay, needle) = if self.search_case_sensitive {
                    (line_str.clone(), self.search_query.to_string())
                } else {
                    (line_str.to_lowercase(), self.search_query.to_lowercase())
                };
                let needle_chars = needle.chars().count();
                if needle_chars > 0 {
                    for (byte_idx, _) in hay.match_indices(&needle) {
                        let col_start = hay[..byte_idx].chars().count();
                        let col_end = col_start + needle_chars;
                        let rx1 = code_origin_x + (col_start as f32) * char_width;
                        let rx2 = code_origin_x + (col_end as f32) * char_width;
                        let rect = Rect::from_min_max(
                            pos2(rx1, line_y + 1.0),
                            pos2(rx2, line_y + line_height - 1.0),
                        );
                        painter.rect(
                            rect,
                            2.0,
                            Color32::from_rgba_unmultiplied(255, 200, 50, 50),
                            Stroke::new(1.0_f32, Color32::from_rgb(255, 200, 60)),
                            StrokeKind::Inside,
                        );
                    }
                }
            }

            // 2d. Retângulos de todas as seleções ativas que cruzam esta linha
            for sel in self.tab.selections.as_slice() {
                if sel.is_empty() {
                    continue;
                }
                let s_start = sel.start();
                let s_end = sel.end();
                if s_end <= line_start_char || s_start > line_end_char {
                    continue;
                }
                let col_start = s_start.saturating_sub(line_start_char);
                let col_end = if s_end > line_end_char {
                    line_len_chars + 1
                } else {
                    s_end.saturating_sub(line_start_char)
                };
                if col_end > col_start {
                    let rx1 = code_origin_x + (col_start as f32) * char_width;
                    let rx2 = code_origin_x + (col_end as f32) * char_width;
                    let sel_rect =
                        Rect::from_min_max(pos2(rx1, line_y), pos2(rx2, line_y + line_height));
                    painter.rect(
                        sel_rect,
                        2.0,
                        Color32::from_rgba_unmultiplied(255, 155, 50, 72),
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 175, 80, 120)),
                        StrokeKind::Inside,
                    );
                }
            }

            // 2e. Texto da linha com Syntax Highlight (Galley em cache por linha)
            if !line_str.is_empty() {
                let galley = layout_line_galley(
                    ui,
                    &line_str,
                    &self.tab.syntax_name,
                    self.theme_name,
                    self.font_size,
                );
                let text_y = line_y + ((line_height - galley.size().y) * 0.5).max(0.0);
                painter.galley(pos2(code_origin_x, text_y), galley, Color32::WHITE);
            }
        }

        // 3. Cursores ativos (Carets) + Preedit IME
        if has_focus {
            let is_multi = self.tab.selections.is_multi();
            for (idx, sel) in self.tab.selections.as_slice().iter().enumerate() {
                let (line_1, col_1) = self.tab.buffer.char_to_line_col(sel.head);
                let line_0 = line_1 - 1;
                if line_0 < first_line || line_0 > last_line {
                    continue;
                }
                let cx = code_origin_x + ((col_1 - 1) as f32) * char_width;
                let cy = canvas_rect.min.y + code_pad_top + (line_0 as f32) * line_height;

                let caret_color = if idx == self.tab.selections.primary_index() {
                    Color32::from_rgb(255, 175, 80)
                } else {
                    Color32::from_rgb(102, 217, 239)
                };

                let caret_rect =
                    Rect::from_min_size(pos2(cx, cy + 1.0), vec2(2.0, line_height - 2.0));
                painter.rect_filled(caret_rect, 1.0, caret_color);

                if is_multi {
                    // Pequeno marcador superior no cursor para destacar modo multi-cursor
                    painter.rect_filled(
                        Rect::from_min_size(pos2(cx - 1.0, cy + 1.0), vec2(4.0, 2.0)),
                        0.0,
                        caret_color,
                    );
                }

                if idx == self.tab.selections.primary_index() && !temp_state.ime_preedit.is_empty()
                {
                    let pw = (temp_state.ime_preedit.chars().count() as f32) * char_width;
                    let pre_rect = Rect::from_min_size(pos2(cx, cy), vec2(pw, line_height));
                    painter.rect_filled(pre_rect, 2.0, Color32::from_rgb(45, 50, 62));
                    painter.text(
                        pos2(cx, cy + 1.0),
                        egui::Align2::LEFT_TOP,
                        &temp_state.ime_preedit,
                        font_id.clone(),
                        Color32::from_rgb(255, 210, 110),
                    );
                    painter.line_segment(
                        [
                            pos2(cx, cy + line_height - 1.0),
                            pos2(cx + pw, cy + line_height - 1.0),
                        ],
                        Stroke::new(1.5_f32, Color32::from_rgb(255, 175, 80)),
                    );
                }
            }
        }

        // 4. Gutter de números de linha fixado na borda esquerda da viewport
        let gutter_rect = Rect::from_min_max(
            pos2(gutter_left_x, canvas_rect.min.y + viewport.min.y),
            pos2(
                gutter_left_x + gutter_width,
                canvas_rect.min.y + viewport.max.y,
            ),
        );
        painter.rect_filled(gutter_rect, 0.0, Color32::from_rgb(31, 34, 41));
        painter.line_segment(
            [
                pos2(gutter_rect.max.x, gutter_rect.min.y),
                pos2(gutter_rect.max.x, gutter_rect.max.y),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(42, 46, 56)),
        );

        let total_lines = self.tab.buffer.len_lines().max(1);
        let digits = total_lines.to_string().len().max(3);

        for line_0 in first_line..=last_line {
            let line_num = line_0 + 1;
            let line_y = canvas_rect.min.y + code_pad_top + (line_0 as f32) * line_height;
            let is_active = active_cursor_lines.contains(&line_0);
            let num_color = if is_active {
                Color32::from_rgb(255, 175, 80)
            } else {
                Color32::from_rgb(100, 108, 122)
            };
            let label = format!(" {:>width$}", line_num, width = digits);
            painter.text(
                pos2(gutter_left_x + 4.0, line_y + line_height * 0.5),
                egui::Align2::LEFT_CENTER,
                label,
                font_id.clone(),
                num_color,
            );
        }
    }

    fn paint_minimap(
        &self,
        ui: &mut Ui,
        width: f32,
        height: f32,
        first_visible_line: usize,
        last_visible_line: usize,
        virtual_height: f32,
    ) -> Option<f32> {
        let (rect, response) =
            ui.allocate_exact_size(vec2(width, height), Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, 0.0, Color32::from_rgb(29, 32, 39));
        painter.line_segment(
            [rect.left_top(), rect.left_bottom()],
            Stroke::new(1.0_f32, Color32::from_rgb(42, 46, 56)),
        );

        let total_lines = self.tab.buffer.len_lines().max(1);
        let row_step = 2.5_f32;
        let max_minimap_rows = ((height - 8.0) / row_step).floor().max(1.0) as usize;
        let start_line_0 = if total_lines <= max_minimap_rows {
            0
        } else {
            let center_line = (first_visible_line + last_visible_line) / 2;
            center_line
                .saturating_sub(max_minimap_rows / 2)
                .min(total_lines.saturating_sub(max_minimap_rows))
        };
        let end_line_0 = (start_line_0 + max_minimap_rows).min(total_lines);

        // Desenha janela da viewport atual no minimapa
        let vis_top_idx = first_visible_line.saturating_sub(start_line_0);
        let vis_bot_idx = (last_visible_line + 1)
            .saturating_sub(start_line_0)
            .max(vis_top_idx + 2);
        let vy1 = (rect.min.y + 4.0 + (vis_top_idx as f32) * row_step).min(rect.max.y);
        let vy2 = (rect.min.y + 4.0 + (vis_bot_idx as f32) * row_step).min(rect.max.y);
        let slider_rect = Rect::from_min_max(pos2(rect.min.x + 1.0, vy1), pos2(rect.max.x, vy2));
        painter.rect_filled(
            slider_rect,
            2.0,
            Color32::from_rgba_unmultiplied(255, 175, 80, 26),
        );
        painter.rect_stroke(
            slider_rect,
            2.0,
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 175, 80, 70)),
            StrokeKind::Inside,
        );

        // Desenha representação em miniatura das linhas de código
        for (i, line_0) in (start_line_0..end_line_0).enumerate() {
            let y = rect.min.y + 4.0 + (i as f32) * row_step;
            let line_str = self.tab.buffer.line_text(line_0);
            let trimmed = line_str.trim_start();
            if trimmed.is_empty() {
                continue;
            }
            let indent = (line_str.len() - trimmed.len()).min(32);
            let len = trimmed.len().min(60);

            let bar_color = if trimmed.starts_with("//") || trimmed.starts_with('#') {
                Color32::from_rgba_unmultiplied(115, 125, 140, 140)
            } else if trimmed.starts_with("fn ")
                || trimmed.starts_with("pub ")
                || trimmed.starts_with("impl ")
                || trimmed.starts_with("struct ")
            {
                Color32::from_rgba_unmultiplied(249, 110, 140, 180)
            } else {
                Color32::from_rgba_unmultiplied(185, 195, 210, 145)
            };

            let x1 = (rect.min.x + 6.0 + (indent as f32) * 0.9).min(rect.max.x - 4.0);
            let x2 = (x1 + (len as f32) * 1.05).min(rect.max.x - 4.0);
            if x2 > x1 {
                painter.line_segment([pos2(x1, y), pos2(x2, y)], Stroke::new(1.4_f32, bar_color));
            }
        }

        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }

        // Clicar ou arrastar no minimapa rola diretamente para a posição correspondente
        if response.clicked() || response.dragged_by(PointerButton::Primary) {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let rel = ((pos.y - rect.min.y - 4.0) / row_step).max(0.0) as usize;
                let clicked_line = (start_line_0 + rel).min(total_lines.saturating_sub(1));
                let ratio = (clicked_line as f32) / (total_lines.max(1) as f32);
                let target_y = (ratio * virtual_height - height * 0.35).max(0.0);
                return Some(target_y);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::PsychicEditorApp;

    #[test]
    fn test_custom_code_view_typing_multicursor_and_undo_in_ui() {
        let ctx = egui::Context::default();
        let mut app = PsychicEditorApp::from_args(&[], false);
        app.tabs[0].content = "let item = 10;\nlet outro = item + item;\n".to_string();
        app.tabs[0].mark_content_edited();
        app.tabs[0].set_cursor_line_col(1, 5); // sobre `item`

        // Frame 1: Renderiza para adquirir foco automático no CodeView
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            app.render_editor_central(ctx);
        });

        // Frame 2: Pressiona Ctrl+D duas vezes para selecionar 2 ocorrências de `item` e digita `valor`
        let mut input = egui::RawInput::default();
        for _ in 0..2 {
            input.events.push(Event::Key {
                key: Key::D,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::CTRL,
            });
        }
        input.events.push(Event::Text("valor".to_string()));

        let _ = ctx.run(input, |ctx| {
            app.render_editor_central(ctx);
            app.render_status_bar(ctx);
        });

        assert_eq!(app.tabs[0].selections.len(), 2);
        assert_eq!(
            app.tabs[0].content,
            "let valor = 10;\nlet outro = valor + item;\n"
        );

        // Frame 3: Pressiona Ctrl+Z (Undo)
        let mut undo_input = egui::RawInput::default();
        undo_input.events.push(Event::Key {
            key: Key::Z,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::CTRL,
        });
        let _ = ctx.run(undo_input, |ctx| {
            app.render_editor_central(ctx);
        });
        assert_eq!(
            app.tabs[0].content,
            "let item = 10;\nlet outro = item + item;\n"
        );
    }

    #[test]
    fn test_code_view_all_keyboard_shortcuts_ime_and_clipboard() {
        let ctx = egui::Context::default();
        let mut app = PsychicEditorApp::from_args(&[], false);
        app.tabs[0].content = "    fn alpha() {\n        let x = alpha;\n    }\n".to_string();
        app.tabs[0].syntax_name = "Rust".to_string();
        app.tabs[0].mark_content_edited();
        app.tabs[0].set_cursor_line_col(1, 9); // sobre `alpha`

        // Frame 1: Foco inicial
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            app.render_editor_central(ctx);
        });

        // Frame 2: Alt+F3 (seleciona todas as ocorrências de `alpha`), Copy, Escape (recolhe para 1 cursor)
        let mut input = egui::RawInput::default();
        input.events.push(Event::Key {
            key: Key::F3,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::ALT,
        });
        input.events.push(Event::Copy);
        input.events.push(Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        });
        // Mais um Escape recolhe a seleção única para caret
        input.events.push(Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        });

        let _ = ctx.run(input, |ctx| {
            app.render_editor_central(ctx);
        });
        assert_eq!(app.tabs[0].selections.len(), 1);
        assert!(app.tabs[0].selections.primary().is_empty());

        // Frame 3: Testa IME (Preedit + Commit), Ctrl+/, Ctrl+Shift+Up/Down, Ctrl+Alt+Up/Down, Tab/Shift+Tab
        let mut input2 = egui::RawInput::default();
        input2
            .events
            .push(Event::Ime(ImeEvent::Preedit("ã".to_string())));
        let _ = ctx.run(input2, |ctx| {
            app.render_editor_central(ctx);
        });

        let mut input3 = egui::RawInput::default();
        input3
            .events
            .push(Event::Ime(ImeEvent::Commit("ã".to_string())));
        input3.events.push(Event::Ime(ImeEvent::Disabled));
        input3.events.push(Event::Key {
            key: Key::Backspace,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        });
        input3.events.push(Event::Key {
            key: Key::Slash,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::CTRL,
        });
        input3.events.push(Event::Key {
            key: Key::ArrowDown,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::CTRL | Modifiers::SHIFT,
        });
        input3.events.push(Event::Key {
            key: Key::ArrowUp,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::CTRL | Modifiers::SHIFT,
        });
        input3.events.push(Event::Key {
            key: Key::ArrowDown,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::CTRL | Modifiers::ALT,
        });
        input3.events.push(Event::Key {
            key: Key::ArrowUp,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::CTRL | Modifiers::ALT,
        });
        for nav_key in [
            Key::Home,
            Key::End,
            Key::ArrowLeft,
            Key::ArrowRight,
            Key::ArrowDown,
            Key::ArrowUp,
            Key::PageDown,
            Key::PageUp,
            Key::Tab,
            Key::Delete,
            Key::Enter,
        ] {
            input3.events.push(Event::Key {
                key: nav_key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            });
        }
        // Ctrl+Y e Ctrl+Shift+Z (Redo), Ctrl+A, Ctrl+Shift+L, Paste, Cut, Ctrl+Shift+K
        input3.events.push(Event::Key {
            key: Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::CTRL,
        });
        input3.events.push(Event::Key {
            key: Key::L,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::CTRL | Modifiers::SHIFT,
        });
        input3.events.push(Event::Paste("novo".to_string()));
        input3.events.push(Event::Cut);
        input3.events.push(Event::Key {
            key: Key::K,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::CTRL | Modifiers::SHIFT,
        });
        input3.events.push(Event::Key {
            key: Key::Z,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::CTRL,
        });
        input3.events.push(Event::Key {
            key: Key::Y,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::CTRL,
        });
        input3.events.push(Event::Key {
            key: Key::Z,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::CTRL | Modifiers::SHIFT,
        });

        let _ = ctx.run(input3, |ctx| {
            app.render_editor_central(ctx);
        });
    }

    #[test]
    fn test_code_view_mouse_clicks_gutter_and_minimap() {
        let ctx = egui::Context::default();
        let mut app = PsychicEditorApp::from_args(&[], false);
        app.tabs[0].content = (1..=60)
            .map(|i| format!("    fn linha_{}() {{ let v = {}; }}", i, i))
            .collect::<Vec<_>>()
            .join("\n");
        app.tabs[0].syntax_name = "Rust".to_string();
        app.tabs[0].mark_content_edited();
        app.search_open = true;
        app.search_query = "linha_2".to_string();

        // Clique na área de código (x=180, y=60)
        let mut click_code = egui::RawInput::default();
        let code_pos = pos2(180.0, 60.0);
        click_code.events.push(Event::PointerMoved(code_pos));
        click_code.events.push(Event::PointerButton {
            pos: code_pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        let _ = ctx.run(click_code, |ctx| {
            app.render_editor_central(ctx);
        });

        // Ctrl+Clique em outra linha (x=180, y=100) adiciona 2º cursor!
        let mut ctrl_click = egui::RawInput::default();
        let pos2_code = pos2(180.0, 100.0);
        ctrl_click.modifiers = Modifiers::CTRL;
        ctrl_click.events.push(Event::PointerMoved(pos2_code));
        ctrl_click.events.push(Event::PointerButton {
            pos: pos2_code,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::CTRL,
        });
        let _ = ctx.run(ctrl_click, |ctx| {
            app.render_editor_central(ctx);
        });
        assert_eq!(app.tabs[0].selections.len(), 2);

        // Clique no Gutter (x=12, y=60) seleciona a linha inteira
        let mut gutter_click = egui::RawInput::default();
        let gutter_pos = pos2(12.0, 60.0);
        gutter_click.events.push(Event::PointerMoved(gutter_pos));
        gutter_click.events.push(Event::PointerButton {
            pos: gutter_pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        let _ = ctx.run(gutter_click, |ctx| {
            app.render_editor_central(ctx);
        });
        assert!(!app.tabs[0].selections.primary().is_empty());
    }
}
