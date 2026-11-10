use crate::highlighter::{
    colorize_color_markers, compute_fast_layout_job, contains_color_marker, EmojiLayout,
};
use egui::text::{LayoutJob, LayoutSection, TextFormat};
use egui::{
    vec2, Align, Color32, CornerRadius, FontFamily, FontId, Id, Margin, RichText, Stroke, Ui,
};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Converte Markdown para HTML puro usando pulldown-cmark
pub fn markdown_to_html(markdown_text: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

    let parser = Parser::new_ext(markdown_text, options);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    html_output
}

#[derive(Clone, Debug)]
struct InlineSpan {
    text: String,
    bold: bool,
    italic: bool,
    strikethrough: bool,
    code: bool,
    link: Option<String>,
}

#[derive(Default)]
struct InlineCollector {
    spans: Vec<InlineSpan>,
    bold_depth: usize,
    italic_depth: usize,
    strike_depth: usize,
    current_link: Option<String>,
}

impl InlineCollector {
    fn push_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.spans.push(InlineSpan {
            text: text.to_string(),
            bold: self.bold_depth > 0,
            italic: self.italic_depth > 0,
            strikethrough: self.strike_depth > 0,
            code: false,
            link: self.current_link.clone(),
        });
    }

    fn push_code(&mut self, code: &str) {
        if code.is_empty() {
            return;
        }
        self.spans.push(InlineSpan {
            text: code.to_string(),
            bold: false,
            italic: false,
            strikethrough: false,
            code: true,
            link: self.current_link.clone(),
        });
    }

    fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    fn clear(&mut self) {
        self.spans.clear();
    }
}

pub struct MarkdownPreviewOutput {
    pub copied_html: bool,
    pub scroll_y: f32,
    pub max_scroll_y: f32,
    pub scroll_ratio: f32,
}

/// Renderiza o documento Markdown em tempo real como widgets nativos no `egui`
pub fn render_markdown_preview(
    ui: &mut Ui,
    tab_id: u64,
    markdown_text: &str,
    theme_name: &str,
    font_size: f32,
    target_scroll_ratio: Option<f32>,
) -> MarkdownPreviewOutput {
    let mut output = MarkdownPreviewOutput {
        copied_html: false,
        scroll_y: 0.0,
        max_scroll_y: 0.0,
        scroll_ratio: 0.0,
    };

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

    let parser = Parser::new_ext(markdown_text, options);

    // Barra de cabeçalho do Preview (Título + Botão Copiar HTML)
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(" Markdown Preview")
                .size(13.0)
                .color(Color32::from_rgb(102, 178, 255)),
        );
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            if ui
                .button(RichText::new("📋 Copiar HTML").size(11.5))
                .on_hover_text("Converte e copia o documento como código HTML para o clipboard")
                .clicked()
            {
                let html = markdown_to_html(markdown_text);
                ui.ctx().copy_text(html);
                output.copied_html = true;
            }
        });
    });
    ui.add_space(4.0);
    ui.separator();

    let max_scroll_id = Id::new(("markdown_max_scroll", tab_id));
    let prev_max_scroll = ui
        .ctx()
        .data(|d| d.get_temp::<f32>(max_scroll_id))
        .unwrap_or(0.0);

    // Área rolável do documento com margem de leitura
    let mut scroll_area = egui::ScrollArea::vertical()
        .id_salt(("markdown_preview_scroll", tab_id))
        .auto_shrink([false, false]);

    if let Some(target_ratio) = target_scroll_ratio {
        scroll_area = scroll_area.vertical_scroll_offset(target_ratio * prev_max_scroll);
    }

    let scroll_out = scroll_area.show(ui, |ui| {
            egui::Frame::NONE
                .fill(Color32::from_rgb(32, 35, 43))
                .inner_margin(Margin::symmetric(20, 16))
                .show(ui, |ui| {
                    let mut in_code_block: Option<(String, String)> = None;
                    let mut current_heading: Option<HeadingLevel> = None;
                    let mut in_blockquote = 0usize;
                    let mut list_stack: Vec<(bool, u64)> = Vec::new(); // (is_ordered, counter)
                    let mut in_table = false;
                    let mut table_headers: Vec<String> = Vec::new();
                    let mut table_rows: Vec<Vec<String>> = Vec::new();
                    let mut current_table_row: Vec<String> = Vec::new();
                    let mut current_cell = String::new();
                    let mut in_table_head = false;
                    let mut task_marker: Option<bool> = None;

                    let mut collector = InlineCollector::default();

                    for event in parser {
                        // Se estivermos dentro de um bloco de código cercado
                        if let Some((_, ref mut code_acc)) = in_code_block {
                            match event {
                                Event::Text(t) => {
                                    code_acc.push_str(&t);
                                }
                                Event::End(TagEnd::CodeBlock) => {
                                    let (lang, code) = in_code_block.take().unwrap();
                                    render_code_block(ui, &lang, &code, theme_name, font_size);
                                }
                                _ => {}
                            }
                            continue;
                        }

                        // Se estivermos dentro de uma tabela
                        if in_table {
                            match event {
                                Event::Start(Tag::TableHead) => {
                                    in_table_head = true;
                                    current_table_row.clear();
                                }
                                Event::End(TagEnd::TableHead) => {
                                    in_table_head = false;
                                    table_headers = std::mem::take(&mut current_table_row);
                                }
                                Event::Start(Tag::TableRow) => {
                                    current_table_row.clear();
                                }
                                Event::End(TagEnd::TableRow) => {
                                    if !in_table_head {
                                        table_rows.push(std::mem::take(&mut current_table_row));
                                    }
                                }
                                Event::Start(Tag::TableCell) => {
                                    current_cell.clear();
                                }
                                Event::End(TagEnd::TableCell) => {
                                    current_table_row.push(std::mem::take(&mut current_cell));
                                }
                                Event::Text(t) | Event::Code(t) => {
                                    current_cell.push_str(&t);
                                }
                                Event::End(TagEnd::Table) => {
                                    render_table(ui, &table_headers, &table_rows, font_size);
                                    in_table = false;
                                    table_headers.clear();
                                    table_rows.clear();
                                }
                                _ => {}
                            }
                            continue;
                        }

                        match event {
                            Event::Start(tag) => match tag {
                                Tag::Heading { level, .. } => {
                                    current_heading = Some(level);
                                    collector.clear();
                                }
                                Tag::Paragraph => {
                                    collector.clear();
                                }
                                Tag::BlockQuote(_) => {
                                    in_blockquote += 1;
                                }
                                Tag::CodeBlock(kind) => {
                                    let lang = match kind {
                                        CodeBlockKind::Fenced(l) => l.to_string(),
                                        CodeBlockKind::Indented => String::new(),
                                    };
                                    in_code_block = Some((lang, String::new()));
                                }
                                Tag::List(start) => {
                                    let is_ordered = start.is_some();
                                    let start_num = start.unwrap_or(1);
                                    list_stack.push((is_ordered, start_num));
                                }
                                Tag::Item => {
                                    collector.clear();
                                    task_marker = None;
                                }
                                Tag::Table(_) => {
                                    in_table = true;
                                    table_headers.clear();
                                    table_rows.clear();
                                }
                                Tag::Emphasis => {
                                    collector.italic_depth += 1;
                                }
                                Tag::Strong => {
                                    collector.bold_depth += 1;
                                }
                                Tag::Strikethrough => {
                                    collector.strike_depth += 1;
                                }
                                Tag::Link { dest_url, .. } => {
                                    collector.current_link = Some(dest_url.to_string());
                                }
                                _ => {}
                            },
                            Event::End(tag_end) => match tag_end {
                                TagEnd::Heading(level) => {
                                    render_heading(
                                        ui,
                                        level,
                                        &collector.spans,
                                        font_size,
                                        theme_name,
                                    );
                                    collector.clear();
                                    current_heading = None;
                                }
                                TagEnd::Paragraph => {
                                    if !collector.is_empty() {
                                        render_paragraph(
                                            ui,
                                            &collector.spans,
                                            font_size,
                                            in_blockquote > 0,
                                        );
                                        collector.clear();
                                    }
                                }
                                TagEnd::BlockQuote(_) => {
                                    in_blockquote = in_blockquote.saturating_sub(1);
                                    ui.add_space(4.0);
                                }
                                TagEnd::Item => {
                                    let depth = list_stack.len().saturating_sub(1);
                                    let (prefix, is_ordered) =
                                        if let Some((ordered, count)) = list_stack.last_mut()
                                        {
                                            if *ordered {
                                                let p = format!("{}. ", count);
                                                *count += 1;
                                                (p, true)
                                            } else {
                                                ("• ".to_string(), false)
                                            }
                                        } else {
                                            ("• ".to_string(), false)
                                        };

                                    let final_prefix = if let Some(checked) = task_marker.take() {
                                        if checked {
                                            "☑ ".to_string()
                                        } else {
                                            "☐ ".to_string()
                                        }
                                    } else {
                                        prefix
                                    };

                                    render_list_item(
                                        ui,
                                        depth,
                                        &final_prefix,
                                        is_ordered,
                                        &collector.spans,
                                        font_size,
                                    );
                                    collector.clear();
                                }
                                TagEnd::List(_) => {
                                    list_stack.pop();
                                    ui.add_space(4.0);
                                }
                                TagEnd::Emphasis => {
                                    collector.italic_depth =
                                        collector.italic_depth.saturating_sub(1);
                                }
                                TagEnd::Strong => {
                                    collector.bold_depth =
                                        collector.bold_depth.saturating_sub(1);
                                }
                                TagEnd::Strikethrough => {
                                    collector.strike_depth =
                                        collector.strike_depth.saturating_sub(1);
                                }
                                TagEnd::Link => {
                                    collector.current_link = None;
                                }
                                _ => {}
                            },
                            Event::Text(t) => {
                                collector.push_text(&t);
                            }
                            Event::Code(c) => {
                                collector.push_code(&c);
                            }
                            Event::TaskListMarker(checked) => {
                                task_marker = Some(checked);
                            }
                            Event::Rule => {
                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(8.0);
                            }
                            Event::SoftBreak => {
                                collector.push_text(" ");
                            }
                            Event::HardBreak => {
                                collector.push_text("\n");
                            }
                            _ => {}
                        }
                    }

                    // Se sobrou algum parágrafo aberto no fim do arquivo
                    if !collector.is_empty() {
                        if let Some(level) = current_heading {
                            render_heading(ui, level, &collector.spans, font_size, theme_name);
                        } else {
                            render_paragraph(ui, &collector.spans, font_size, in_blockquote > 0);
                        }
                    }
                });
        });

    let content_height = scroll_out.content_size.y;
    let viewport_height = scroll_out.inner_rect.height();
    let current_max_scroll = (content_height - viewport_height).max(0.0);
    let scroll_y = scroll_out.state.offset.y;
    let scroll_ratio = if current_max_scroll > 0.0 {
        (scroll_y / current_max_scroll).clamp(0.0, 1.0)
    } else {
        0.0
    };

    ui.ctx()
        .data_mut(|d| d.insert_temp(max_scroll_id, current_max_scroll));

    if prev_max_scroll == 0.0
        && current_max_scroll > 0.0
        && target_scroll_ratio.is_some_and(|r| r > 0.0)
    {
        ui.ctx().request_repaint();
    }

    output.scroll_y = scroll_y;
    output.max_scroll_y = current_max_scroll;
    output.scroll_ratio = scroll_ratio;

    output
}

fn render_heading(
    ui: &mut Ui,
    level: HeadingLevel,
    spans: &[InlineSpan],
    base_font_size: f32,
    _theme_name: &str,
) {
    let (scale, add_sep, color) = match level {
        HeadingLevel::H1 => (
            1.65,
            true,
            Color32::from_rgb(255, 175, 80), // Sublime accent orange
        ),
        HeadingLevel::H2 => (
            1.38,
            true,
            Color32::from_rgb(240, 244, 252),
        ),
        HeadingLevel::H3 => (
            1.20,
            false,
            Color32::from_rgb(230, 235, 245),
        ),
        HeadingLevel::H4 => (
            1.08,
            false,
            Color32::from_rgb(215, 220, 230),
        ),
        HeadingLevel::H5 => (
            0.98,
            false,
            Color32::from_rgb(180, 185, 195),
        ),
        HeadingLevel::H6 => (
            0.90,
            false,
            Color32::from_rgb(150, 155, 165),
        ),
    };

    ui.add_space(10.0);

    let text_full: String = spans.iter().map(|s| s.text.as_str()).collect();
    if contains_color_marker(&text_full) {
        show_job(
            ui,
            marker_job(&text_full, base_font_size * scale, color, false, false),
        );
    } else {
        ui.label(
            RichText::new(text_full)
                .size(base_font_size * scale)
                .strong()
                .color(color),
        );
    }

    if add_sep {
        ui.add_space(2.0);
        ui.separator();
        ui.add_space(4.0);
    } else {
        ui.add_space(4.0);
    }
}

fn render_paragraph(ui: &mut Ui, spans: &[InlineSpan], font_size: f32, in_blockquote: bool) {
    let has_links_or_code = spans.iter().any(|s| s.link.is_some() || s.code);

    if in_blockquote {
        egui::Frame::NONE
            .fill(Color32::from_rgb(27, 30, 37))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 50, 62)))
            .corner_radius(CornerRadius::same(3))
            .inner_margin(Margin {
                left: 12,
                right: 8,
                top: 6,
                bottom: 6,
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Barra vertical de destaque do quote
                    let (rect, _) = ui.allocate_exact_size(vec2(3.0, 18.0), egui::Sense::hover());
                    ui.painter().rect_filled(
                        rect,
                        1.0,
                        Color32::from_rgb(255, 175, 80),
                    );
                    ui.add_space(4.0);
                    render_spans_flow(ui, spans, font_size, has_links_or_code, true);
                });
            });
        ui.add_space(4.0);
        return;
    }

    render_spans_flow(ui, spans, font_size, has_links_or_code, false);
    ui.add_space(6.0);
}

fn render_spans_flow(
    ui: &mut Ui,
    spans: &[InlineSpan],
    font_size: f32,
    has_links_or_code: bool,
    italic_override: bool,
) {
    if !has_links_or_code {
        // Fast path: monta um único LayoutJob para o parágrafo inteiro com tipografia perfeita
        let mut job = LayoutJob::default();
        let default_color = Color32::from_rgb(220, 225, 235);
        let font_id = FontId::new(font_size, FontFamily::Proportional);

        for span in spans {
            let start = job.text.len();
            job.text.push_str(&span.text);
            let end = job.text.len();

            let mut fmt = TextFormat {
                font_id: font_id.clone(),
                color: default_color,
                italics: span.italic || italic_override,
                ..Default::default()
            };
            if span.bold {
                fmt.color = Color32::WHITE;
            }
            if span.strikethrough {
                fmt.strikethrough = Stroke::new(1.0_f32, default_color);
            }
            job.sections.push(LayoutSection {
                leading_space: 0.0,
                byte_range: start..end,
                format: fmt,
            });
        }
        show_job(ui, job);
    } else {
        // Flow path: suporta links clicáveis e badges de código inline
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(3.0, 2.0);
            for span in spans {
                if let Some(ref url) = span.link {
                    ui.hyperlink_to(
                        RichText::new(&span.text)
                            .size(font_size)
                            .color(Color32::from_rgb(102, 178, 255)),
                        url,
                    );
                } else if span.code {
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(42, 46, 56))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(58, 64, 78)))
                        .corner_radius(CornerRadius::same(3))
                        .inner_margin(Margin::symmetric(5, 1))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(&span.text)
                                    .monospace()
                                    .size(font_size * 0.90)
                                    .color(Color32::from_rgb(250, 205, 120)),
                            );
                        });
                } else if contains_color_marker(&span.text) {
                    let color = if span.bold {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(220, 225, 235)
                    };
                    show_job(
                        ui,
                        marker_job(
                        &span.text,
                        font_size,
                        color,
                        span.italic || italic_override,
                        span.strikethrough,
                    ),
                    );
                } else {
                    let mut rt = RichText::new(&span.text)
                        .size(font_size)
                        .color(Color32::from_rgb(220, 225, 235));
                    if span.bold {
                        rt = rt.strong().color(Color32::WHITE);
                    }
                    if span.italic || italic_override {
                        rt = rt.italics();
                    }
                    if span.strikethrough {
                        rt = rt.strikethrough();
                    }
                    ui.label(rt);
                }
            }
        });
    }
}

/// Monta um `LayoutJob` de uma única cor onde emojis de marcadores coloridos (🔴🟠🟡…)
/// são desenhados como glifos geométricos tingidos.
fn marker_job(text: &str, size: f32, color: Color32, italics: bool, strike: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.text = text.to_string();
    job.sections.push(LayoutSection {
        leading_space: 0.0,
        byte_range: 0..text.len(),
        format: TextFormat {
            font_id: FontId::new(size, FontFamily::Proportional),
            color,
            italics,
            strikethrough: if strike {
                Stroke::new(1.0_f32, color)
            } else {
                Stroke::NONE
            },
            ..Default::default()
        },
    });
    job
}

/// Mostra um `LayoutJob` no preview, desenhando emojis coloridos (imagens) quando houver.
fn show_job(ui: &mut Ui, job: LayoutJob) {
    let emojis: Vec<char> = crate::emoji::color_emoji_positions(&job.text)
        .into_iter()
        .map(|(_, c)| c)
        .collect();
    let measure = |f: &FontId| {
        let w = ui.fonts(|fo| fo.glyph_width(f, crate::emoji::PLACEHOLDER));
        (f.size * crate::emoji::EMOJI_SCALE - w).max(0.0)
    };
    let job = colorize_color_markers(job, &EmojiLayout::Flow(&measure));
    if emojis.is_empty() {
        ui.label(job);
        return;
    }
    let (pos, galley, _resp) = egui::Label::new(job).layout_in_ui(ui);
    ui.painter().add(egui::epaint::TextShape::new(
        pos,
        galley.clone(),
        ui.visuals().text_color(),
    ));
    crate::emoji::paint_galley_emojis(ui.painter(), ui.ctx(), pos, &galley, &emojis, false);
}

fn render_list_item(
    ui: &mut Ui,
    depth: usize,
    prefix: &str,
    is_ordered: bool,
    spans: &[InlineSpan],
    font_size: f32,
) {
    let indent_px = (depth as f32) * 18.0 + 8.0;
    ui.horizontal(|ui| {
        ui.add_space(indent_px);
        let prefix_color = if is_ordered {
            Color32::from_rgb(255, 175, 80)
        } else {
            Color32::from_rgb(160, 170, 185)
        };
        ui.label(
            RichText::new(prefix)
                .size(font_size)
                .strong()
                .color(prefix_color),
        );
        let has_links_or_code = spans.iter().any(|s| s.link.is_some() || s.code);
        render_spans_flow(ui, spans, font_size, has_links_or_code, false);
    });
    ui.add_space(2.0);
}

fn render_code_block(
    ui: &mut Ui,
    lang: &str,
    code: &str,
    theme_name: &str,
    font_size: f32,
) {
    ui.add_space(4.0);
    egui::Frame::NONE
        .fill(Color32::from_rgb(24, 26, 32))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(46, 50, 62)))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            // Cabeçalho da caixa de código: Linguagem + Botão Copiar
            ui.horizontal(|ui| {
                let display_lang = if lang.is_empty() {
                    "PLAIN TEXT"
                } else {
                    lang.trim()
                };
                ui.label(
                    RichText::new(display_lang.to_uppercase())
                        .size(10.5)
                        .monospace()
                        .color(Color32::from_rgb(140, 150, 165)),
                );
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("📋 Copiar").clicked() {
                        ui.ctx().copy_text(code.to_string());
                    }
                });
            });
            ui.add_space(2.0);
            ui.separator();
            ui.add_space(4.0);

            // Conteúdo do código com Syntax Highlighting nativo do Psychic!
            let syntax_name = detect_syntax_from_lang(lang);
            let job = compute_fast_layout_job(code, syntax_name, theme_name, font_size * 0.92);
            let emojis: Vec<char> = crate::emoji::color_emoji_positions(code)
                .into_iter()
                .map(|(_, c)| c)
                .collect();
            if emojis.is_empty() {
                ui.label(job);
            } else {
                let (pos, galley, _resp) = egui::Label::new(job).layout_in_ui(ui);
                ui.painter().add(egui::epaint::TextShape::new(
                    pos,
                    galley.clone(),
                    ui.visuals().text_color(),
                ));
                crate::emoji::paint_galley_emojis(
                    ui.painter(),
                    ui.ctx(),
                    pos,
                    &galley,
                    &emojis,
                    true,
                );
            }
        });
    ui.add_space(8.0);
}

fn render_table(
    ui: &mut Ui,
    headers: &[String],
    rows: &[Vec<String>],
    font_size: f32,
) {
    if headers.is_empty() && rows.is_empty() {
        return;
    }
    ui.add_space(4.0);
    egui::Frame::NONE
        .fill(Color32::from_rgb(27, 29, 36))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(48, 52, 64)))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            egui::Grid::new(ui.next_auto_id())
                .striped(true)
                .spacing(vec2(16.0, 6.0))
                .show(ui, |ui| {
                    if !headers.is_empty() {
                        for h in headers {
                            if contains_color_marker(h) {
                                show_job(
                                    ui,
                                    marker_job(
                                        h,
                                        font_size,
                                        Color32::from_rgb(255, 175, 80),
                                        false,
                                        false,
                                    ),
                                );
                            } else {
                                ui.label(
                                    RichText::new(h)
                                        .size(font_size)
                                        .strong()
                                        .color(Color32::from_rgb(255, 175, 80)),
                                );
                            }
                        }
                        ui.end_row();
                    }
                    for row in rows {
                        for cell in row {
                            if contains_color_marker(cell) {
                                show_job(
                                    ui,
                                    marker_job(
                                    cell,
                                    font_size * 0.95,
                                    Color32::from_rgb(220, 225, 235),
                                    false,
                                    false,
                                ),
                                );
                            } else {
                                ui.label(
                                    RichText::new(cell)
                                        .size(font_size * 0.95)
                                        .color(Color32::from_rgb(220, 225, 235)),
                                );
                            }
                        }
                        ui.end_row();
                    }
                });
        });
    ui.add_space(8.0);
}

fn detect_syntax_from_lang(lang: &str) -> &'static str {
    match lang.trim().to_lowercase().as_str() {
        "rs" | "rust" => "Rust",
        "toml" => "TOML / Config",
        "ini" | "conf" | "cfg" => "INI / Systemd / Conf",
        "sh" | "bash" | "zsh" | "shell" => "Shell / Bash / Zsh",
        "json" | "jsonc" => "JSON",
        "yaml" | "yml" => "YAML",
        "md" | "markdown" => "Markdown",
        "py" | "python" => "Python",
        "js" | "javascript" | "ts" | "typescript" | "tsx" | "jsx" => "JavaScript / TypeScript",
        "c" | "cpp" | "c++" | "h" | "hpp" => "C / C++",
        "go" | "golang" => "Go",
        "html" | "htm" | "xml" => "HTML / XML",
        "css" | "scss" => "CSS",
        "lua" => "Lua",
        _ => "Plain Text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_to_html_conversion() {
        let md = "# Titulo 1\n\nTexto com **negrito** e *italico*.\n\n```rust\nfn main() {}\n```\n";
        let html = markdown_to_html(md);
        assert!(html.contains("<h1>Titulo 1</h1>"));
        assert!(html.contains("<strong>negrito</strong>"));
        assert!(html.contains("<em>italico</em>"));
        assert!(html.contains("<code class=\"language-rust\">"));
    }

    #[test]
    fn test_marker_job_replaces_emoji_with_colored_glyphs() {
        let job = marker_job("🔴 P0 e 🟡 P2", 14.0, Color32::WHITE, false, false);
        assert!(job.text.contains("🔴 P0 e 🟡 P2"));
        let processed = colorize_color_markers(job, &EmojiLayout::Grid);
        assert_eq!(processed.text.chars().count(), "🔴 P0 e 🟡 P2".chars().count());
    }

    #[test]
    fn test_render_markdown_color_markers_everywhere() {
        let ctx = egui::Context::default();
        let markdown = "# 🔴 Título\n\n- 🟠 item **🟡 negrito** `x`\n\n| a |\n|---|\n| 🟢 |\n\n⚪ texto\n";
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = render_markdown_preview(ui, 9, markdown, "base16-eighties.dark", 14.0, None);
            });
        });
    }

    #[test]
    fn test_detect_syntax_from_lang() {
        assert_eq!(detect_syntax_from_lang("rust"), "Rust");
        assert_eq!(detect_syntax_from_lang("py"), "Python");
        assert_eq!(detect_syntax_from_lang("unknown"), "Plain Text");
    }

    #[test]
    fn test_render_markdown_preview_headless() {
        let ctx = egui::Context::default();
        let markdown = r#"
# Heading 1
## Heading 2
### Heading 3

This is a paragraph with **bold**, *italic*, ~~strikethrough~~, and `inline code`.
Here is a [link](https://github.com/rafammendes/psychic-text-editor).

> This is a blockquote.

- Item 1
- Item 2
  - Subitem 2.1
- [x] Done task
- [ ] Todo task

1. First
2. Second

| Header 1 | Header 2 |
| :--- | :--- |
| Cell 1 | Cell 2 |

```rust
fn psychic_test() {
    let x = 42;
}
```

---
The end.
"#;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let out = render_markdown_preview(
                    ui,
                    1,
                    markdown,
                    "base16-eighties.dark",
                    14.0,
                    Some(0.0),
                );
                assert!(!out.copied_html);
                assert!(out.scroll_ratio >= 0.0 && out.scroll_ratio <= 1.0);
            });
        });
    }

    #[test]
    fn test_render_all_headings_h1_to_h6() {
        let ctx = egui::Context::default();
        let markdown = r#"
# Heading 1
## Heading 2
### Heading 3
#### Heading 4
##### Heading 5
###### Heading 6
"#;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let out = render_markdown_preview(
                    ui,
                    2,
                    markdown,
                    "base16-eighties.dark",
                    14.0,
                    None,
                );
                assert_eq!(out.scroll_ratio, 0.0);
            });
        });
    }

    #[test]
    fn test_render_markdown_tables_and_empty() {
        let ctx = egui::Context::default();
        let markdown = r#"
| Coluna A | Coluna B | Coluna C |
| :--- | :---: | ---: |
| 10 | 20 | 30 |
| alpha | beta | gamma |
"#;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let out = render_markdown_preview(
                    ui,
                    3,
                    markdown,
                    "base16-eighties.dark",
                    14.0,
                    None,
                );
                assert!(!out.copied_html);

                // Testa render_table com tabela vazia diretamente
                render_table(ui, &[], &[], 14.0);
            });
        });
    }

    #[test]
    fn test_render_markdown_code_blocks_and_syntax() {
        let ctx = egui::Context::default();
        let markdown = r#"
```rust
pub fn foo() -> i32 { 100 }
```

```python
def bar():
    return 42
```

```toml
[package]
name = "psychic"
```

```json
{"status": "ok"}
```

```
código sem linguagem definida
```
"#;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let out = render_markdown_preview(
                    ui,
                    4,
                    markdown,
                    "base16-eighties.dark",
                    14.0,
                    None,
                );
                assert!(!out.copied_html);
            });
        });
    }

    #[test]
    fn test_render_markdown_nested_task_lists_and_blockquotes() {
        let ctx = egui::Context::default();
        let markdown = r#"
> Citação com `código inline` e [link externo](https://example.com).
> Segunda linha da citação.

1. Primeiro ordenado
2. Segundo ordenado com subitem:
   - [x] Tarefa concluída
   - [ ] Tarefa pendente
   - Subitem com **negrito** e *itálico*
"#;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let out = render_markdown_preview(
                    ui,
                    5,
                    markdown,
                    "base16-eighties.dark",
                    14.0,
                    Some(0.5),
                );
                assert!(!out.copied_html);
            });
        });
    }

    #[test]
    fn test_render_markdown_scroll_ratio_persistence() {
        let ctx = egui::Context::default();
        let mut lines = Vec::new();
        for i in 1..=80 {
            lines.push(format!("### Parágrafo de teste {}\n\nTexto longo que ocupa espaço vertical na tela para testar scroll.", i));
        }
        let markdown = lines.join("\n\n");

        let mut input1 = egui::RawInput::default();
        input1.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 400.0)));

        // Frame 1: calcula max_scroll e armazena em temp data
        let _ = ctx.run(input1, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let out = render_markdown_preview(
                    ui,
                    6,
                    &markdown,
                    "base16-eighties.dark",
                    14.0,
                    Some(0.5),
                );
                assert!(!out.copied_html);
            });
        });

        let mut input2 = egui::RawInput::default();
        input2.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 400.0)));

        // Frame 2: agora com max_scroll conhecido, o target_scroll_ratio 0.5 deve ser aplicado
        let _ = ctx.run(input2, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let out = render_markdown_preview(
                    ui,
                    6,
                    &markdown,
                    "base16-eighties.dark",
                    14.0,
                    Some(0.5),
                );
                assert!(out.max_scroll_y > 0.0);
                assert!(out.scroll_y > 0.0);
            });
        });
    }
}
