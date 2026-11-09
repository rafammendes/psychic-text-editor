mod app;
mod highlighter;
mod icons;
mod palette;
mod sidebar;
mod state;
mod tab;
mod ui;

use app::PsychicEditorApp;
use eframe::egui;
use egui::{Color32, FontData, FontDefinitions, FontFamily, Stroke};
use std::env;
use std::sync::Arc;

const JETBRAINS_NERD_FONT: &[u8] =
    include_bytes!("../assets/fonts/JetBrainsMonoNerdFont-Regular.ttf");

fn main() -> eframe::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Psychic Text Editor")
            .with_app_id("psychic-text-editor")
            .with_decorations(true)
            .with_inner_size([1240.0, 800.0])
            .with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Psychic Text Editor",
        native_options,
        Box::new(move |cc| {
            configure_fonts(&cc.egui_ctx);
            configure_sublime_visuals(&cc.egui_ctx);
            Ok(Box::new(PsychicEditorApp::from_args(&args, true)))
        }),
    )
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "JetBrainsMonoNerdFont".to_owned(),
        Arc::new(FontData::from_static(JETBRAINS_NERD_FONT)),
    );

    if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
        mono.insert(0, "JetBrainsMonoNerdFont".to_owned());
    }
    if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
        prop.insert(0, "JetBrainsMonoNerdFont".to_owned());
    }

    ctx.set_fonts(fonts);
}

fn configure_sublime_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = Color32::from_rgb(30, 33, 40);
    visuals.panel_fill = Color32::from_rgb(35, 38, 46);
    visuals.extreme_bg_color = Color32::from_rgb(28, 30, 36);
    visuals.faint_bg_color = Color32::from_rgb(39, 42, 50);
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(255, 155, 50, 75);
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::from_rgb(255, 175, 80));
    ctx.set_visuals(visuals);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configure_fonts_and_visuals() {
        let ctx = egui::Context::default();
        configure_fonts(&ctx);
        configure_sublime_visuals(&ctx);
    }

    #[test]
    fn test_rapid_typing_highlight_speed() {
        let source = include_str!("app.rs");
        let start = std::time::Instant::now();
        for i in 0..10 {
            let modified = format!("{} // char {}", source, i);
            let job = highlighter::compute_fast_layout_job(
                &modified,
                "Rust",
                "base16-eighties.dark",
                14.5,
            );
            assert!(!job.sections.is_empty());
        }
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 50);
    }
}
