use egui::text::{LayoutJob, LayoutSection, TextFormat};
use egui::{Color32, FontFamily, FontId, Galley};
use std::path::Path;
use std::sync::Arc;

pub const AVAILABLE_THEMES: &[(&str, &str)] = &[
    ("base16-eighties.dark", "Sublime Monokai / Eighties Dark"),
    ("base16-mocha.dark", "Sublime Mariana / Mocha Dark"),
    ("base16-ocean.dark", "Sublime Ocean Dark"),
    ("Solarized (dark)", "Solarized Dark"),
    ("InspiredGitHub", "Sublime Light (GitHub)"),
];

const SUPPORTED_SYNTAXES: &[&str] = &[
    "Rust",
    "TOML / Config",
    "INI / Systemd / Conf",
    "Shell / Bash / Zsh",
    "JSON",
    "YAML",
    "Markdown",
    "Python",
    "JavaScript / TypeScript",
    "C / C++",
    "Go",
    "HTML / XML",
    "CSS",
    "Lua",
    "Plain Text",
];

pub fn list_syntaxes() -> Vec<String> {
    SUPPORTED_SYNTAXES.iter().map(|s| (*s).to_string()).collect()
}

pub fn detect_syntax_name(path: Option<&Path>, first_line: &str) -> String {
    if let Some(p) = path {
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let file_name = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        match ext.as_str() {
            "rs" => return "Rust".to_string(),
            "toml" | "lock" => return "TOML / Config".to_string(),
            "conf" | "cfg" | "ini" | "service" | "socket" | "timer" | "desktop" => {
                return "INI / Systemd / Conf".to_string()
            }
            "sh" | "bash" | "zsh" | "fish" | "env" | "profile" => {
                return "Shell / Bash / Zsh".to_string()
            }
            "json" | "jsonc" => return "JSON".to_string(),
            "yaml" | "yml" => return "YAML".to_string(),
            "md" | "markdown" => return "Markdown".to_string(),
            "py" => return "Python".to_string(),
            "js" | "mjs" | "cjs" | "ts" | "tsx" | "jsx" => {
                return "JavaScript / TypeScript".to_string()
            }
            "c" | "h" | "cpp" | "hpp" | "cc" | "cxx" => return "C / C++".to_string(),
            "go" => return "Go".to_string(),
            "html" | "htm" | "xml" | "svg" => return "HTML / XML".to_string(),
            "css" | "scss" => return "CSS".to_string(),
            "lua" => return "Lua".to_string(),
            _ => {}
        }

        if file_name.starts_with(".bashrc")
            || file_name.starts_with(".zshrc")
            || file_name.starts_with(".profile")
            || file_name.ends_with("rc")
        {
            return "Shell / Bash / Zsh".to_string();
        }
        if file_name == "cargo.lock" || file_name == "cargo.toml" {
            return "TOML / Config".to_string();
        }
    }

    if first_line.starts_with("#!") {
        if first_line.contains("python") {
            return "Python".to_string();
        }
        if first_line.contains("sh") || first_line.contains("bash") || first_line.contains("zsh") {
            return "Shell / Bash / Zsh".to_string();
        }
    }

    "Plain Text".to_string()
}

#[derive(Clone, Copy)]
struct ThemePalette {
    normal: Color32,
    comment: Color32,
    string: Color32,
    keyword: Color32,
    ty: Color32,
    function: Color32,
    number: Color32,
    macro_attr: Color32,
    punctuation: Color32,
}

impl ThemePalette {
    fn for_name(name: &str) -> Self {
        match name {
            "base16-mocha.dark" => Self {
                normal: Color32::from_rgb(216, 222, 233),
                comment: Color32::from_rgb(128, 139, 155),
                string: Color32::from_rgb(153, 199, 148),
                keyword: Color32::from_rgb(197, 148, 197),
                ty: Color32::from_rgb(250, 200, 99),
                function: Color32::from_rgb(102, 153, 204),
                number: Color32::from_rgb(249, 145, 87),
                macro_attr: Color32::from_rgb(95, 179, 179),
                punctuation: Color32::from_rgb(192, 197, 206),
            },
            "base16-ocean.dark" => Self {
                normal: Color32::from_rgb(192, 197, 206),
                comment: Color32::from_rgb(101, 115, 126),
                string: Color32::from_rgb(163, 190, 140),
                keyword: Color32::from_rgb(180, 142, 173),
                ty: Color32::from_rgb(235, 203, 139),
                function: Color32::from_rgb(143, 161, 179),
                number: Color32::from_rgb(208, 135, 112),
                macro_attr: Color32::from_rgb(150, 181, 180),
                punctuation: Color32::from_rgb(171, 178, 191),
            },
            "Solarized (dark)" => Self {
                normal: Color32::from_rgb(147, 161, 161),
                comment: Color32::from_rgb(88, 110, 117),
                string: Color32::from_rgb(42, 161, 152),
                keyword: Color32::from_rgb(133, 153, 0),
                ty: Color32::from_rgb(181, 137, 0),
                function: Color32::from_rgb(38, 139, 210),
                number: Color32::from_rgb(211, 54, 130),
                macro_attr: Color32::from_rgb(203, 75, 22),
                punctuation: Color32::from_rgb(131, 148, 150),
            },
            "InspiredGitHub" => Self {
                normal: Color32::from_rgb(50, 50, 50),
                comment: Color32::from_rgb(130, 138, 148),
                string: Color32::from_rgb(24, 118, 54),
                keyword: Color32::from_rgb(167, 29, 93),
                ty: Color32::from_rgb(0, 134, 179),
                function: Color32::from_rgb(121, 93, 163),
                number: Color32::from_rgb(0, 128, 128),
                macro_attr: Color32::from_rgb(190, 90, 0),
                punctuation: Color32::from_rgb(80, 80, 80),
            },
            _ => Self {
                normal: Color32::from_rgb(248, 248, 242),
                comment: Color32::from_rgb(125, 132, 145),
                string: Color32::from_rgb(230, 219, 116),
                keyword: Color32::from_rgb(249, 38, 114),
                ty: Color32::from_rgb(102, 217, 239),
                function: Color32::from_rgb(166, 226, 46),
                number: Color32::from_rgb(174, 129, 255),
                macro_attr: Color32::from_rgb(253, 151, 31),
                punctuation: Color32::from_rgb(210, 215, 222),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct LineCacheKey {
    text_hash: u64,
    text_len: usize,
    syntax_name: String,
    theme_name: String,
    font_size_bits: u32,
}

#[derive(Clone, Default)]
struct LineGalleyCache {
    map: std::collections::HashMap<LineCacheKey, Arc<Galley>>,
}

/// Faz o layout e syntax highlight de uma única linha visível com cache O(1).
pub fn layout_line_galley(
    ui: &egui::Ui,
    line_text: &str,
    syntax_name: &str,
    theme_name: &str,
    font_size: f32,
) -> Arc<Galley> {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    line_text.hash(&mut hasher);
    let key = LineCacheKey {
        text_hash: hasher.finish(),
        text_len: line_text.len(),
        syntax_name: syntax_name.to_string(),
        theme_name: theme_name.to_string(),
        font_size_bits: font_size.to_bits(),
    };

    let cache_id = egui::Id::new("psychic_line_galley_cache");
    if let Some(cached) = ui.ctx().data_mut(|d| {
        let cache = d.get_temp_mut_or_default::<LineGalleyCache>(cache_id);
        cache.map.get(&key).cloned()
    }) {
        return cached;
    }

    let mut job = compute_fast_layout_job(line_text, syntax_name, theme_name, font_size);
    job.wrap.max_width = f32::INFINITY;
    let galley = ui.fonts(|f| f.layout_job(job));

    ui.ctx().data_mut(|d| {
        let cache = d.get_temp_mut_or_default::<LineGalleyCache>(cache_id);
        if cache.map.len() >= 1024 {
            cache.map.clear();
        }
        cache.map.insert(key, Arc::clone(&galley));
    });

    galley
}

/// Mapeia emojis de "bolinhas/quadrados coloridos" para um glifo geométrico monocromático
/// presente na fonte embutida e a cor correspondente. O `egui` não renderiza emojis coloridos
/// (os glifos de fallback são monocromáticos ou ausentes), então os desenhamos tingidos.
pub fn color_marker_glyph(c: char) -> Option<(char, Color32)> {
    let circle = '\u{25CF}'; // ●
    let square = '\u{25A0}'; // ■
    Some(match c {
        '🔴' => (circle, Color32::from_rgb(240, 80, 80)),
        '🟠' => (circle, Color32::from_rgb(255, 152, 48)),
        '🟡' => (circle, Color32::from_rgb(248, 208, 64)),
        '🟢' => (circle, Color32::from_rgb(88, 204, 108)),
        '🔵' => (circle, Color32::from_rgb(76, 148, 245)),
        '🟣' => (circle, Color32::from_rgb(170, 110, 235)),
        '🟤' => (circle, Color32::from_rgb(160, 110, 80)),
        '⚪' => (circle, Color32::from_rgb(236, 238, 242)),
        '⚫' => (circle, Color32::from_rgb(105, 108, 116)),
        '🟥' => (square, Color32::from_rgb(240, 80, 80)),
        '🟧' => (square, Color32::from_rgb(255, 152, 48)),
        '🟨' => (square, Color32::from_rgb(248, 208, 64)),
        '🟩' => (square, Color32::from_rgb(88, 204, 108)),
        '🟦' => (square, Color32::from_rgb(76, 148, 245)),
        '🟪' => (square, Color32::from_rgb(170, 110, 235)),
        '🟫' => (square, Color32::from_rgb(160, 110, 80)),
        '⬜' => (square, Color32::from_rgb(236, 238, 242)),
        '⬛' => (square, Color32::from_rgb(105, 108, 116)),
        _ => return None,
    })
}

#[inline]
pub fn contains_color_marker(text: &str) -> bool {
    text.chars()
        .any(|c| color_marker_glyph(c).is_some() || is_variation_selector(c))
}

/// Seletores de variação (ex.: U+FE0F de `🖥️`) não têm glifo na fonte e apareceriam como um
/// quadrado de "caractere ausente"; outros editores os tratam como largura zero.
#[inline]
pub fn is_variation_selector(c: char) -> bool {
    ('\u{FE00}'..='\u{FE0F}').contains(&c)
}

/// Substitui emojis de marcadores coloridos por glifos geométricos tingidos, preservando as
/// demais seções (e a contagem de caracteres: 1 emoji → 1 glifo).
pub fn colorize_color_markers(job: LayoutJob) -> LayoutJob {
    if !contains_color_marker(&job.text) {
        return job;
    }
    let mut out = job.clone();
    out.text.clear();
    out.sections.clear();
    for section in &job.sections {
        let slice = &job.text[section.byte_range.clone()];
        let mut first = true;
        let mut plain_start = out.text.len();
        let push_plain = |out: &mut LayoutJob, start: usize, leading: f32| {
            let end = out.text.len();
            if end > start {
                out.sections.push(LayoutSection {
                    leading_space: leading,
                    byte_range: start..end,
                    format: section.format.clone(),
                });
            }
        };
        for c in slice.chars() {
            if let Some((glyph, color)) = color_marker_glyph(c) {
                push_plain(
                    &mut out,
                    plain_start,
                    if first { section.leading_space } else { 0.0 },
                );
                first = false;
                let start = out.text.len();
                out.text.push(glyph);
                let mut fmt = section.format.clone();
                fmt.color = color;
                fmt.italics = false;
                fmt.font_id = FontId::new(fmt.font_id.size, FontFamily::Monospace);
                out.sections.push(LayoutSection {
                    leading_space: 0.0,
                    byte_range: start..out.text.len(),
                    format: fmt,
                });
                plain_start = out.text.len();
            } else if is_variation_selector(c) {
                // Mantém 1 char → 1 char; U+200B é invisível e tem largura zero no egui.
                out.text.push('\u{200B}');
            } else {
                out.text.push(c);
            }
        }
        push_plain(
            &mut out,
            plain_start,
            if first { section.leading_space } else { 0.0 },
        );
    }
    out
}

pub fn compute_fast_layout_job(
    text: &str,
    syntax_name: &str,
    theme_name: &str,
    font_size: f32,
) -> LayoutJob {
    colorize_color_markers(compute_fast_layout_job_raw(
        text,
        syntax_name,
        theme_name,
        font_size,
    ))
}

fn compute_fast_layout_job_raw(
    text: &str,
    syntax_name: &str,
    theme_name: &str,
    font_size: f32,
) -> LayoutJob {
    let font_id = FontId::new(font_size, FontFamily::Monospace);
    if text.is_empty() {
        return LayoutJob::default();
    }

    let palette = ThemePalette::for_name(theme_name);

    if syntax_name == "Plain Text" || text.len() > 500_000 {
        return LayoutJob::single_section(
            text.to_owned(),
            TextFormat {
                font_id,
                color: palette.normal,
                ..Default::default()
            },
        );
    }

    let is_config_like = matches!(
        syntax_name,
        "TOML / Config" | "INI / Systemd / Conf" | "YAML" | "JSON" | "Markdown"
    );
    let hash_comments =
        is_config_like || matches!(syntax_name, "Shell / Bash / Zsh" | "Python");

    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut sections: Vec<LayoutSection> = Vec::with_capacity(len / 12 + 16);
    let mut i = 0usize;

    while i < len {
        let b = bytes[i];

        // 1. Whitespace: inherit previous section color to avoid splitting sections
        if b.is_ascii_whitespace() {
            let start = i;
            i += 1;
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            push_section(
                &mut sections,
                start,
                i,
                palette.normal,
                false,
                &font_id,
                true,
            );
            continue;
        }

        // 2. Line comments
        if (b == b'/' && i + 1 < len && bytes[i + 1] == b'/')
            || (b == b'#' && (hash_comments || (i + 1 < len && bytes[i + 1] == b'!')))
            || (b == b';' && syntax_name == "INI / Systemd / Conf")
            || (b == b'-' && syntax_name == "Lua" && i + 1 < len && bytes[i + 1] == b'-')
        {
            if b == b'#'
                && syntax_name == "Rust"
                && i + 1 < len
                && (bytes[i + 1] == b'['
                    || (bytes[i + 1] == b'!' && i + 2 < len && bytes[i + 2] == b'['))
            {
                let start = i;
                while i < len && bytes[i] != b'\n' && bytes[i] != b']' {
                    i += 1;
                }
                if i < len && bytes[i] == b']' {
                    i += 1;
                }
                push_section(
                    &mut sections,
                    start,
                    i,
                    palette.macro_attr,
                    false,
                    &font_id,
                    false,
                );
                continue;
            }

            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            push_section(
                &mut sections,
                start,
                i,
                palette.comment,
                false,
                &font_id,
                false,
            );
            continue;
        }

        // 3. Block comments `/* ... */`
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(len);
            push_section(
                &mut sections,
                start,
                i,
                palette.comment,
                false,
                &font_id,
                false,
            );
            continue;
        }

        // 4. Rust attributes `#[...]`
        if b == b'#' && syntax_name == "Rust" {
            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            push_section(
                &mut sections,
                start,
                i,
                palette.macro_attr,
                false,
                &font_id,
                false,
            );
            continue;
        }

        // 5. Section header in TOML / INI (`[section]`) at start of line
        if b == b'[' && is_config_like && is_at_line_start(bytes, i) {
            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            push_section(&mut sections, start, i, palette.ty, false, &font_id, false);
            continue;
        }

        // 6. Strings `"..."` or `'...'` or `` `...` ``
        if b == b'"' || b == b'\'' || b == b'`' {
            if b == b'\'' && syntax_name == "Rust" {
                if i + 2 < len && bytes[i + 1] != b'\\' && bytes[i + 2] != b'\'' {
                    let start = i;
                    i += 1;
                    while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                        i += 1;
                    }
                    push_section(
                        &mut sections,
                        start,
                        i,
                        palette.macro_attr,
                        false,
                        &font_id,
                        false,
                    );
                    continue;
                }
            }

            let quote = b;
            let start = i;
            i += 1;
            while i < len {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(len);
                    continue;
                }
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                if bytes[i] == b'\n' && quote == b'\'' {
                    break;
                }
                i += 1;
            }
            push_section(
                &mut sections,
                start,
                i,
                palette.string,
                false,
                &font_id,
                false,
            );
            continue;
        }

        // 7. Numbers
        if b.is_ascii_digit() {
            let start = i;
            i += 1;
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.' || bytes[i] == b'_')
            {
                i += 1;
            }
            push_section(
                &mut sections,
                start,
                i,
                palette.number,
                false,
                &font_id,
                false,
            );
            continue;
        }

        // 8. Shell / Config variables `$VAR`
        if b == b'$' {
            let start = i;
            i += 1;
            while i < len
                && (bytes[i].is_ascii_alphanumeric()
                    || bytes[i] == b'_'
                    || bytes[i] == b'{'
                    || bytes[i] == b'}')
            {
                i += 1;
            }
            push_section(
                &mut sections,
                start,
                i,
                palette.macro_attr,
                false,
                &font_id,
                false,
            );
            continue;
        }

        // 9. Identifiers, Keywords, Types, Functions, Config Keys
        if b.is_ascii_alphabetic() || b == b'_' {
            let start = i;
            i += 1;
            while i < len
                && (bytes[i].is_ascii_alphanumeric()
                    || bytes[i] == b'_'
                    || (is_config_like && bytes[i] == b'-'))
            {
                i += 1;
            }
            let word = &text[start..i];

            if i < len && bytes[i] == b'!' {
                i += 1;
                push_section(
                    &mut sections,
                    start,
                    i,
                    palette.macro_attr,
                    false,
                    &font_id,
                    false,
                );
                continue;
            }

            if is_config_like {
                let mut look = i;
                while look < len && (bytes[look] == b' ' || bytes[look] == b'\t') {
                    look += 1;
                }
                if look < len && (bytes[look] == b'=' || bytes[look] == b':') {
                    push_section(
                        &mut sections,
                        start,
                        i,
                        palette.function,
                        false,
                        &font_id,
                        false,
                    );
                    continue;
                }
            }

            let color = if is_keyword(word) {
                palette.keyword
            } else if is_builtin_or_constant(word) {
                palette.number
            } else if is_type_name(word) {
                palette.ty
            } else if i < len && bytes[i] == b'(' {
                palette.function
            } else {
                palette.normal
            };

            push_section(&mut sections, start, i, color, false, &font_id, false);
            continue;
        }

        // 10. Multi-byte UTF-8 or punctuation
        let start = i;
        if b < 128 {
            i += 1;
            push_section(
                &mut sections,
                start,
                i,
                palette.punctuation,
                false,
                &font_id,
                false,
            );
        } else {
            let ch_len = text[i..].chars().next().map_or(1, char::len_utf8);
            i += ch_len;
            push_section(
                &mut sections,
                start,
                i,
                palette.normal,
                false,
                &font_id,
                false,
            );
        }
    }

    LayoutJob {
        text: text.to_owned(),
        sections,
        ..Default::default()
    }
}

#[inline]
fn push_section(
    sections: &mut Vec<LayoutSection>,
    start: usize,
    end: usize,
    color: Color32,
    italics: bool,
    font_id: &FontId,
    inherit_previous_if_possible: bool,
) {
    if let Some(last) = sections.last_mut() {
        if last.byte_range.end == start
            && (inherit_previous_if_possible
                || (last.format.color == color && last.format.italics == italics))
        {
            last.byte_range.end = end;
            return;
        }
    }
    sections.push(LayoutSection {
        leading_space: 0.0,
        byte_range: start..end,
        format: TextFormat {
            font_id: font_id.clone(),
            color,
            italics,
            ..Default::default()
        },
    });
}

#[inline]
fn is_at_line_start(bytes: &[u8], mut idx: usize) -> bool {
    while idx > 0 {
        idx -= 1;
        if bytes[idx] == b'\n' {
            return true;
        }
        if bytes[idx] != b' ' && bytes[idx] != b'\t' {
            return false;
        }
    }
    true
}

#[inline]
fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "fn" | "let"
            | "mut"
            | "pub"
            | "struct"
            | "enum"
            | "impl"
            | "trait"
            | "for"
            | "in"
            | "while"
            | "loop"
            | "if"
            | "else"
            | "match"
            | "return"
            | "use"
            | "mod"
            | "const"
            | "static"
            | "async"
            | "await"
            | "where"
            | "type"
            | "ref"
            | "move"
            | "unsafe"
            | "extern"
            | "crate"
            | "super"
            | "as"
            | "break"
            | "continue"
            | "dyn"
            | "def"
            | "class"
            | "import"
            | "from"
            | "pass"
            | "lambda"
            | "yield"
            | "with"
            | "try"
            | "except"
            | "finally"
            | "raise"
            | "function"
            | "var"
            | "export"
            | "default"
            | "switch"
            | "case"
            | "new"
            | "typeof"
            | "interface"
            | "package"
            | "func"
            | "defer"
            | "go"
            | "select"
            | "then"
            | "fi"
            | "do"
            | "done"
            | "elif"
            | "esac"
            | "local"
    )
}

#[inline]
fn is_builtin_or_constant(word: &str) -> bool {
    matches!(
        word,
        "true"
            | "false"
            | "True"
            | "False"
            | "None"
            | "Some"
            | "Ok"
            | "Err"
            | "null"
            | "nil"
            | "undefined"
            | "self"
            | "Self"
            | "this"
    )
}

#[inline]
fn is_type_name(word: &str) -> bool {
    if matches!(
        word,
        "bool"
            | "char"
            | "str"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "f32"
            | "f64"
            | "int"
            | "float"
            | "double"
            | "void"
            | "string"
            | "byte"
    ) {
        return true;
    }
    let first = word.as_bytes()[0];
    first.is_ascii_uppercase() && word.bytes().any(|b| b.is_ascii_lowercase())
}

#[cfg(test)]
pub fn build_gutter_layout_job(
    line_count: usize,
    current_line: usize,
    font_size: f32,
) -> LayoutJob {
    use std::fmt::Write as _;
    let digits = line_count.to_string().len().max(3);
    let line_stride = digits + 4;
    let mut text = String::with_capacity(line_count * line_stride);
    let mut cur_start = 0usize;
    let mut cur_end = 0usize;

    for line_num in 1..=line_count {
        if line_num == current_line {
            cur_start = text.len();
        }
        if line_num < line_count {
            let _ = writeln!(text, " {:>width$}  ", line_num, width = digits);
        } else {
            let _ = write!(text, " {:>width$}  ", line_num, width = digits);
        }
        if line_num == current_line {
            cur_end = text.len();
        }
    }

    let font_id = FontId::new(font_size, FontFamily::Monospace);
    let dim_format = TextFormat {
        font_id: font_id.clone(),
        color: Color32::from_rgb(100, 108, 122),
        ..Default::default()
    };
    let active_format = TextFormat {
        font_id,
        color: Color32::from_rgb(255, 175, 80),
        ..Default::default()
    };

    let mut sections = Vec::with_capacity(3);
    if cur_start > 0 {
        sections.push(LayoutSection {
            leading_space: 0.0,
            byte_range: 0..cur_start,
            format: dim_format.clone(),
        });
    }
    if cur_end > cur_start {
        sections.push(LayoutSection {
            leading_space: 0.0,
            byte_range: cur_start..cur_end,
            format: active_format,
        });
    }
    if cur_end < text.len() {
        sections.push(LayoutSection {
            leading_space: 0.0,
            byte_range: cur_end..text.len(),
            format: dim_format,
        });
    }

    LayoutJob {
        text,
        sections,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_syntax_all_supported_types() {
        for (file, expected) in [
            ("main.rs", "Rust"),
            ("Cargo.toml", "TOML / Config"),
            ("/etc/nginx.conf", "INI / Systemd / Conf"),
            ("script.sh", "Shell / Bash / Zsh"),
            ("package.json", "JSON"),
            ("docker-compose.yml", "YAML"),
            ("README.md", "Markdown"),
            ("app.py", "Python"),
            ("index.tsx", "JavaScript / TypeScript"),
            ("kernel.c", "C / C++"),
            ("server.go", "Go"),
            ("index.html", "HTML / XML"),
            ("theme.css", "CSS"),
            ("init.lua", "Lua"),
            (".bashrc", "Shell / Bash / Zsh"),
            ("unknown.xyz", "Plain Text"),
        ] {
            assert_eq!(detect_syntax_name(Some(Path::new(file)), ""), expected);
        }

        assert_eq!(
            detect_syntax_name(None, "#!/usr/bin/env python3"),
            "Python"
        );
        assert_eq!(
            detect_syntax_name(None, "#!/bin/bash"),
            "Shell / Bash / Zsh"
        );
    }

    #[test]
    fn test_lexer_highlights_rust_and_configs_without_italic_skew() {
        let code = r#"
#[derive(Debug)]
pub struct Config<'a> {
    pub name: &'a str, //  Pasta
    pub count: usize,
}
/* Block comment */
fn main() {
    let x = 42;
    println!("hello \"world\"");
}
"#;
        for (theme_id, _) in AVAILABLE_THEMES {
            let job = compute_fast_layout_job(code, "Rust", theme_id, 14.0);
            assert!(!job.sections.is_empty());
            // Verify no section uses synthetic italics
            assert!(job.sections.iter().all(|s| !s.format.italics));
        }

        let toml_code = "[package]\nname = \"psychic\"\n# comment\n$HOME\n";
        let toml_job =
            compute_fast_layout_job(toml_code, "TOML / Config", "base16-eighties.dark", 14.0);
        assert!(!toml_job.sections.is_empty());
    }

    #[test]
    fn test_gutter_layout_job_has_at_most_3_sections() {
        let job = build_gutter_layout_job(500, 250, 14.5);
        assert_eq!(job.sections.len(), 3);

        let job_first = build_gutter_layout_job(500, 1, 14.5);
        assert_eq!(job_first.sections.len(), 2);

        let job_last = build_gutter_layout_job(500, 500, 14.5);
        assert_eq!(job_last.sections.len(), 2);
    }

    #[test]
    fn test_color_markers_are_tinted_and_preserve_text_layout() {
        let src = "- 🔴 P0 — crítico\n- 🟠 P1\n- ⚪ P3 `código`";
        let job = compute_fast_layout_job(src, "Markdown", "base16-eighties.dark", 14.0);
        assert_eq!(job.text.chars().count(), src.chars().count());
        assert!(!contains_color_marker(&job.text));
        let dots: Vec<_> = job
            .sections
            .iter()
            .filter(|s| &job.text[s.byte_range.clone()] == "●")
            .collect();
        assert_eq!(dots.len(), 3);
        assert_ne!(dots[0].format.color, dots[1].format.color);
        // seções continuam cobrindo todo o texto, sem lacunas
        let covered: usize = job.sections.iter().map(|s| s.byte_range.len()).sum();
        assert_eq!(covered, job.text.len());
    }

    #[test]
    fn test_variation_selector_is_not_rendered_as_missing_glyph_box() {
        let src = "## 🖥\u{FE0F} 4. Interface";
        let job = compute_fast_layout_job(src, "Markdown", "base16-eighties.dark", 14.0);
        assert!(!job.text.contains('\u{FE0F}'));
        assert_eq!(job.text.chars().count(), src.chars().count());
        let covered: usize = job.sections.iter().map(|s| s.byte_range.len()).sum();
        assert_eq!(covered, job.text.len());
    }
}
