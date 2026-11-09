use egui::Color32;
use std::path::Path;

pub const ICON_FOLDER_CLOSED: &str = "\u{f07b}"; // 
pub const ICON_FOLDER_OPEN: &str = "\u{f07c}"; // 
pub const ICON_FILE_TEXT: &str = "\u{f15c}"; // 
pub const ICON_FILE_CODE: &str = "\u{f1c9}"; // 
pub const ICON_SEARCH: &str = "\u{f002}"; // 
pub const ICON_TERMINAL: &str = "\u{f120}"; // 
pub const ICON_GEAR: &str = "\u{f013}"; // 
pub const ICON_SAVE: &str = "\u{f0c7}"; // 
pub const ICON_PLUS: &str = "\u{f067}"; // 
pub const ICON_CLOSE: &str = "\u{f00d}"; // 
pub const ICON_BOLT: &str = "\u{f0e7}"; // 
pub const ICON_BRANCH: &str = "\u{e725}"; // 

pub fn file_icon_and_color(path: Option<&Path>) -> (&'static str, Color32) {
    let Some(p) = path else {
        return (ICON_FILE_TEXT, Color32::from_rgb(180, 185, 195));
    };

    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "rs" => ("\u{e7a8}", Color32::from_rgb(222, 165, 132)), //  Rust
        "toml" | "ini" | "conf" | "cfg" | "service" => {
            (ICON_GEAR, Color32::from_rgb(160, 190, 230))
        }
        "json" | "yaml" | "yml" => ("\u{e60b}", Color32::from_rgb(241, 224, 90)), // 
        "md" | "markdown" => ("\u{e73e}", Color32::from_rgb(81, 154, 186)),       // 
        "sh" | "bash" | "zsh" | "fish" => (ICON_TERMINAL, Color32::from_rgb(137, 224, 81)),
        "py" => ("\u{e73c}", Color32::from_rgb(53, 114, 165)), // 
        "js" | "mjs" | "cjs" => ("\u{e74e}", Color32::from_rgb(241, 224, 90)), // 
        "ts" | "tsx" => ("\u{e628}", Color32::from_rgb(49, 120, 198)), // 
        "html" | "htm" => ("\u{e736}", Color32::from_rgb(227, 76, 38)), // 
        "css" | "scss" => ("\u{e749}", Color32::from_rgb(86, 61, 124)), // 
        "c" | "h" => ("\u{e61e}", Color32::from_rgb(168, 185, 204)), // 
        "cpp" | "hpp" | "cc" => ("\u{e61d}", Color32::from_rgb(243, 75, 125)), // 
        "go" => ("\u{e627}", Color32::from_rgb(0, 173, 216)),  // 
        "lua" => ("\u{e620}", Color32::from_rgb(0, 0, 128)),   // 
        "lock" => ("\u{f023}", Color32::from_rgb(150, 150, 150)), // 
        _ => {
            if name.starts_with(".git") {
                (ICON_BRANCH, Color32::from_rgb(240, 80, 50))
            } else if name.starts_with('.') || name.ends_with("rc") {
                (ICON_GEAR, Color32::from_rgb(160, 190, 230))
            } else {
                (ICON_FILE_CODE, Color32::from_rgb(180, 185, 195))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icons_for_known_extensions() {
        let (icon_none, _) = file_icon_and_color(None);
        assert_eq!(icon_none, ICON_FILE_TEXT);

        let (icon_rs, _) = file_icon_and_color(Some(Path::new("src/main.rs")));
        assert_eq!(icon_rs, "\u{e7a8}");

        let (icon_conf, _) = file_icon_and_color(Some(Path::new("/etc/nginx.conf")));
        assert_eq!(icon_conf, ICON_GEAR);

        let (icon_sh, _) = file_icon_and_color(Some(Path::new("deploy.sh")));
        assert_eq!(icon_sh, ICON_TERMINAL);

        let (icon_git, _) = file_icon_and_color(Some(Path::new(".gitignore")));
        assert_eq!(icon_git, ICON_BRANCH);

        let (icon_rc, _) = file_icon_and_color(Some(Path::new(".bashrc")));
        assert_eq!(icon_rc, ICON_GEAR);

        let (icon_other, _) = file_icon_and_color(Some(Path::new("notes.unknown")));
        assert_eq!(icon_other, ICON_FILE_CODE);
    }

    #[test]
    fn test_icons_for_web_and_systems_languages() {
        for (file, expected) in [
            ("app.json", "\u{e60b}"),
            ("README.md", "\u{e73e}"),
            ("script.py", "\u{e73c}"),
            ("index.js", "\u{e74e}"),
            ("comp.tsx", "\u{e628}"),
            ("page.html", "\u{e736}"),
            ("style.css", "\u{e749}"),
            ("kern.c", "\u{e61e}"),
            ("engine.cpp", "\u{e61d}"),
            ("main.go", "\u{e627}"),
            ("init.lua", "\u{e620}"),
            ("Cargo.lock", "\u{f023}"),
        ] {
            let (ic, _) = file_icon_and_color(Some(Path::new(file)));
            assert_eq!(ic, expected, "Failed icon match for {}", file);
        }
    }
}
