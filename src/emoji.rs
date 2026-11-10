//! Emojis coloridos no editor e no preview.
//!
//! O `egui` rasteriza fontes apenas em monocromático, então emojis ficam sem cor (ou como
//! quadrado "ausente"). Aqui lemos os bitmaps coloridos (`CBDT/CBLC`) da fonte *Noto Color Emoji*
//! instalada no sistema, decodificamos o PNG de cada emoji e o desenhamos como textura por cima de
//! um caractere reservado (NBSP). Se a fonte não existir, o chamador usa o fallback tingido.

use egui::{ColorImage, Context, Id, TextureHandle, TextureId, TextureOptions};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use ttf_parser::{Face, RasterImageFormat};

/// Caractere reservado (largura de 1 célula) no lugar de cada emoji que será desenhado como imagem.
pub const PLACEHOLDER: char = '\u{00A0}';

const FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/noto/NotoColorEmoji.ttf",
    "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
    "/usr/share/fonts/google-noto-emoji/NotoColorEmoji.ttf",
    "/usr/share/fonts/google-noto-color-emoji/NotoColorEmoji.ttf",
    "/usr/share/fonts/TTF/NotoColorEmoji.ttf",
    "/usr/share/fonts/noto-color-emoji/NotoColorEmoji.ttf",
    "/usr/local/share/fonts/NotoColorEmoji.ttf",
];

fn font_bytes() -> Option<&'static [u8]> {
    static FONT: OnceLock<Option<&'static [u8]>> = OnceLock::new();
    *FONT.get_or_init(|| {
        let mut paths: Vec<std::path::PathBuf> = Vec::new();
        if let Some(p) = std::env::var_os("PSYCHIC_EMOJI_FONT") {
            paths.push(p.into());
        }
        if let Some(home) = std::env::var_os("HOME") {
            paths.push(std::path::Path::new(&home).join(".local/share/fonts/NotoColorEmoji.ttf"));
        }
        paths.extend(FONT_CANDIDATES.iter().map(std::path::PathBuf::from));
        for p in paths {
            if let Ok(bytes) = std::fs::read(&p) {
                if Face::parse(&bytes, 0).is_ok() {
                    return Some(&*Box::leak(bytes.into_boxed_slice()));
                }
            }
        }
        None
    })
}

fn face() -> Option<Face<'static>> {
    Face::parse(font_bytes()?, 0).ok()
}

/// `true` se a fonte de emojis coloridos está disponível no sistema.
pub fn is_available() -> bool {
    font_bytes().is_some()
}

fn default_emoji_presentation(c: char) -> bool {
    matches!(c as u32,
        0x231A | 0x231B | 0x23E9..=0x23EC | 0x23F0 | 0x23F3 | 0x25FD | 0x25FE | 0x2614 | 0x2615
        | 0x2648..=0x2653 | 0x267F | 0x2693 | 0x26A1 | 0x26AA | 0x26AB | 0x26BD | 0x26BE
        | 0x26C4 | 0x26C5 | 0x26CE | 0x26D4 | 0x26EA | 0x26F2 | 0x26F3 | 0x26F5 | 0x26FA
        | 0x26FD | 0x2705 | 0x270A | 0x270B | 0x2728 | 0x274C | 0x274E | 0x2753..=0x2755
        | 0x2757 | 0x2795..=0x2797 | 0x27B0 | 0x27BF | 0x2B1B | 0x2B1C | 0x2B50 | 0x2B55)
}

fn has_bitmap(c: char) -> bool {
    static CACHE: OnceLock<Mutex<HashMap<char, bool>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(v) = cache.lock().ok().and_then(|m| m.get(&c).copied()) {
        return v;
    }
    let ok = face().is_some_and(|f| {
        f.glyph_index(c)
            .and_then(|gid| f.glyph_raster_image(gid, 128))
            .is_some_and(|img| matches!(img.format, RasterImageFormat::PNG))
    });
    if let Ok(mut m) = cache.lock() {
        m.insert(c, ok);
    }
    ok
}

/// O caractere deve ser desenhado como emoji colorido (imagem)? `next` é o caractere seguinte
/// (para detectar o seletor de variação U+FE0F que pede a apresentação emoji).
pub fn is_color_emoji(c: char, next: Option<char>) -> bool {
    if c.is_ascii() || matches!(c, '←' | '↑' | '→' | '↓' | '↔' | '—' | '–' | '•' | '“' | '”') {
        return false;
    }
    let cp = c as u32;
    let candidate = (0x1F000..=0x1FAFF).contains(&cp)
        || (0x2600..=0x27BF).contains(&cp)
        || (0x2300..=0x23FF).contains(&cp)
        || (0x2B00..=0x2BFF).contains(&cp)
        || default_emoji_presentation(c)
        || (next == Some('\u{FE0F}') && (0x203C..=0x3299).contains(&cp));
    candidate && is_available() && has_bitmap(c)
}

/// Posições `(índice do char, emoji)` dos emojis coloridos de um texto.
pub fn color_emoji_positions(text: &str) -> Vec<(usize, char)> {
    if text.is_ascii() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut it = text.chars().enumerate().peekable();
    while let Some((i, c)) = it.next() {
        if is_color_emoji(c, it.peek().map(|&(_, n)| n)) {
            out.push((i, c));
        }
    }
    out
}

fn decode(c: char) -> Option<ColorImage> {
    let f = face()?;
    let gid = f.glyph_index(c)?;
    let img = f.glyph_raster_image(gid, 128)?;
    if !matches!(img.format, RasterImageFormat::PNG) {
        return None;
    }
    let mut decoder = png::Decoder::new(std::io::Cursor::new(img.data));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width as usize, info.height as usize);
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => buf[..w * h * 4].to_vec(),
        png::ColorType::Rgb => buf[..w * h * 3]
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        _ => return None,
    };
    // Redução 2x (média 2×2) para um serrilhado menor em tamanhos pequenos.
    if w >= 96 && h >= 96 {
        let (nw, nh) = (w / 2, h / 2);
        let mut out = vec![0u8; nw * nh * 4];
        for y in 0..nh {
            for x in 0..nw {
                let mut acc = [0u32; 4];
                let mut alpha_w = 0u32;
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let i = ((2 * y + dy) * w + 2 * x + dx) * 4;
                    let a = rgba[i + 3] as u32;
                    acc[0] += rgba[i] as u32 * a;
                    acc[1] += rgba[i + 1] as u32 * a;
                    acc[2] += rgba[i + 2] as u32 * a;
                    acc[3] += a;
                    alpha_w += a;
                }
                let o = (y * nw + x) * 4;
                if alpha_w > 0 {
                    out[o] = (acc[0] / alpha_w) as u8;
                    out[o + 1] = (acc[1] / alpha_w) as u8;
                    out[o + 2] = (acc[2] / alpha_w) as u8;
                }
                out[o + 3] = (acc[3] / 4) as u8;
            }
        }
        return Some(ColorImage::from_rgba_unmultiplied([nw, nh], &out));
    }
    Some(ColorImage::from_rgba_unmultiplied([w, h], &rgba))
}

#[derive(Clone, Default)]
struct EmojiTextures(HashMap<char, Option<TextureHandle>>);

/// Textura colorida do emoji (carregada sob demanda e mantida em cache no `Context`).
pub fn texture(ctx: &Context, c: char) -> Option<TextureId> {
    let id = Id::new("psychic_emoji_textures");
    let cached = ctx.data_mut(|d| d.get_temp_mut_or_default::<EmojiTextures>(id).0.get(&c).cloned());
    if let Some(entry) = cached {
        return entry.map(|t| t.id());
    }
    let handle = decode(c).map(|img| {
        ctx.load_texture(format!("emoji_{:x}", c as u32), img, TextureOptions::LINEAR)
    });
    let tex_id = handle.as_ref().map(|t| t.id());
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<EmojiTextures>(id).0.insert(c, handle);
    });
    tex_id
}

/// Escala do emoji em relação ao tamanho da fonte no preview de Markdown.
pub const EMOJI_SCALE: f32 = 1.25;

/// Pinta as imagens dos emojis sobre os glifos PLACEHOLDER reservados no `galley`.
pub fn paint_galley_emojis(
    painter: &egui::Painter,
    ctx: &Context,
    origin: egui::Pos2,
    galley: &egui::Galley,
    emojis: &[char],
    grid: bool,
) {
    let mut it = emojis.iter();
    for row in &galley.rows {
        for g in &row.glyphs {
            if g.chr != PLACEHOLDER {
                continue;
            }
            let Some(&emoji) = it.next() else {
                return;
            };
            let font_size = galley
                .job
                .sections
                .get(g.section_index as usize)
                .map_or(14.0, |s| s.format.font_id.size);

            let (x0, size) = if grid {
                let size = (font_size * 1.15).round();
                (g.pos.x + (g.advance_width - size) * 0.5, size)
            } else {
                let size = (font_size * EMOJI_SCALE).round();
                (g.pos.x - (size - g.advance_width).max(0.0), size)
            };
            let rect = egui::Rect::from_min_size(
                egui::pos2(origin.x + x0, origin.y + row.rect.center().y - size / 2.0),
                egui::vec2(size, size),
            );
            paint(painter, ctx, emoji, rect);
        }
    }
}

/// Desenha o emoji colorido em `rect`. Retorna `false` se não houver imagem.
pub fn paint(painter: &egui::Painter, ctx: &Context, c: char, rect: egui::Rect) -> bool {
    let Some(tex) = texture(ctx, c) else {
        return false;
    };
    painter.image(
        tex,
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_and_plain_symbols_are_not_color_emoji() {
        assert!(!is_color_emoji('a', None));
        assert!(!is_color_emoji('→', None));
        assert!(!is_color_emoji('—', None));
        assert!(color_emoji_positions("texto simples → ok").is_empty());
    }

    #[test]
    fn test_color_emoji_detection_and_texture_when_font_available() {
        if !is_available() {
            return;
        }
        assert!(is_color_emoji('🧭', None));
        assert!(is_color_emoji('✅', None));
        assert!(is_color_emoji('✏', Some('\u{FE0F}')));
        let pos = color_emoji_positions("a 🧭 b 🏗\u{FE0F} c");
        assert_eq!(pos, vec![(2, '🧭'), (6, '🏗')]);

        let ctx = Context::default();
        let _ = ctx.run(egui::RawInput::default(), |_| {});
        assert!(texture(&ctx, '🧭').is_some());
        assert!(texture(&ctx, '🟠').is_some());
        assert!(texture(&ctx, 'a').is_none());
    }

    #[test]
    fn test_probe_todo_emojis() {
        if !is_available() {
            return;
        }
        for c in "—–•“”←↑→↓↔[]()#*-".chars() {
            println!("Punctuation {} -> has_bitmap: {}", c, has_bitmap(c));
        }
        let symbols = "✅🏗✏🔍🧭🖥📁⚙🎨🛠📦🐛🎯🔴🟠🟡⚪🚀💡📌⚠✨";
        for c in symbols.chars() {
            let ok = has_bitmap(c);
            println!("Emoji: {} (U+{:04X}) -> has_bitmap: {}", c, c as u32, ok);
            assert!(ok, "Missing bitmap for {}", c);
        }
    }
}
