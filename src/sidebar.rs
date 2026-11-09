use crate::icons::{file_icon_and_color, ICON_FOLDER_CLOSED, ICON_FOLDER_OPEN};
use egui::{Color32, CursorIcon, RichText, Ui};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct CachedFileItem {
    pub name: String,
    pub path: PathBuf,
    pub icon: &'static str,
    pub icon_color: Color32,
}

type DirCacheEntry = (Vec<(String, PathBuf)>, Vec<CachedFileItem>);

pub struct WorkspaceTree {
    pub root: PathBuf,
    pub expanded_dirs: BTreeSet<PathBuf>,
    pub show_hidden: bool,
    pub indexed_files: Vec<PathBuf>,
    dir_cache: HashMap<PathBuf, DirCacheEntry>,
}

impl WorkspaceTree {
    pub fn new(root: PathBuf) -> Self {
        let mut expanded_dirs = BTreeSet::new();
        expanded_dirs.insert(root.clone());
        let mut tree = Self {
            root,
            expanded_dirs,
            show_hidden: true,
            indexed_files: Vec::new(),
            dir_cache: HashMap::new(),
        };
        tree.rebuild_index();
        tree
    }

    pub fn set_root(&mut self, root: PathBuf) {
        self.root = root.clone();
        self.expanded_dirs.clear();
        self.expanded_dirs.insert(root);
        self.rebuild_index();
    }

    pub fn rebuild_index(&mut self) {
        self.indexed_files.clear();
        self.dir_cache.clear();
        let root = self.root.clone();
        let show_hidden = self.show_hidden;
        collect_files(&root, show_hidden, 0, &mut self.indexed_files);
    }

    pub fn ui(&mut self, ui: &mut Ui, active_file: Option<&Path>) -> Option<PathBuf> {
        let mut clicked_file = None;

        ui.horizontal(|ui| {
            let folder_name = self
                .root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_else(|| self.root.to_str().unwrap_or("PROJETO"))
                .to_uppercase();

            ui.add(
                egui::Label::new(
                    RichText::new(format!("{} {}", ICON_FOLDER_OPEN, folder_name))
                        .strong()
                        .size(12.0)
                        .color(Color32::from_rgb(160, 170, 185)),
                )
                .selectable(false),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("⟳")
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .on_hover_text("Atualizar árvore de arquivos")
                    .clicked()
                {
                    self.rebuild_index();
                }
                let dot_label = if self.show_hidden { ".*" } else { "·" };
                if ui
                    .small_button(dot_label)
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .on_hover_text("Mostrar/Ocultar arquivos ocultos (dotfiles)")
                    .clicked()
                {
                    self.show_hidden = !self.show_hidden;
                    self.rebuild_index();
                }
            });
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let root = self.root.clone();
                self.render_dir_entries(ui, &root, 0, active_file, &mut clicked_file);
            });

        clicked_file
    }

    pub fn get_or_load_dir(&mut self, dir: &Path) -> DirCacheEntry {
        if let Some(cached) = self.dir_cache.get(dir) {
            return cached.clone();
        }

        let mut dirs = Vec::new();
        let mut files = Vec::new();

        if let Ok(read_dir) = fs::read_dir(dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                if !self.show_hidden && name.starts_with('.') {
                    continue;
                }
                if matches!(name.as_str(), ".git" | "target" | "node_modules")
                    && path.is_dir()
                    && !self.show_hidden
                {
                    continue;
                }
                if path.is_dir() {
                    dirs.push((name, path));
                } else {
                    let (icon, icon_color) = file_icon_and_color(Some(&path));
                    files.push(CachedFileItem {
                        name,
                        path,
                        icon,
                        icon_color,
                    });
                }
            }
        }

        dirs.sort_by(|a, b| a.0.to_ascii_lowercase().cmp(&b.0.to_ascii_lowercase()));
        files.sort_by(|a, b| {
            a.name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
        });

        let entry = (dirs, files);
        self.dir_cache.insert(dir.to_path_buf(), entry.clone());
        entry
    }

    fn render_dir_entries(
        &mut self,
        ui: &mut Ui,
        dir: &Path,
        depth: usize,
        active_file: Option<&Path>,
        clicked_file: &mut Option<PathBuf>,
    ) {
        let (dirs, files) = self.get_or_load_dir(dir);
        let indent = (depth as f32) * 14.0;

        for (name, path) in dirs {
            let is_open = self.expanded_dirs.contains(&path);
            let icon = if is_open {
                ICON_FOLDER_OPEN
            } else {
                ICON_FOLDER_CLOSED
            };

            ui.horizontal(|ui| {
                ui.add_space(indent);
                let resp = ui
                    .selectable_label(
                        false,
                        RichText::new(format!("{}  {}", icon, name))
                            .size(13.5)
                            .color(Color32::from_rgb(210, 195, 145)),
                    )
                    .on_hover_cursor(CursorIcon::PointingHand);
                if resp.clicked() {
                    if is_open {
                        self.expanded_dirs.remove(&path);
                    } else {
                        self.expanded_dirs.insert(path.clone());
                    }
                }
            });

            if is_open {
                self.render_dir_entries(ui, &path, depth + 1, active_file, clicked_file);
            }
        }

        for item in files {
            let is_selected = active_file.map_or(false, |af| af == item.path);

            ui.horizontal(|ui| {
                ui.add_space(indent + 4.0);
                ui.add(
                    egui::Label::new(RichText::new(item.icon).size(13.5).color(item.icon_color))
                        .selectable(false),
                );
                let text_color = if is_selected {
                    Color32::from_rgb(255, 255, 255)
                } else {
                    Color32::from_rgb(200, 205, 215)
                };
                let resp = ui
                    .selectable_label(
                        is_selected,
                        RichText::new(&item.name).size(13.5).color(text_color),
                    )
                    .on_hover_cursor(CursorIcon::PointingHand);
                if resp.clicked() {
                    *clicked_file = Some(item.path);
                }
            });
        }
    }
}

fn collect_files(dir: &Path, show_hidden: bool, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 6 || out.len() >= 2500 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        if matches!(
            name.as_str(),
            ".git" | "target" | "node_modules" | ".cache" | "dist" | "build"
        ) {
            continue;
        }
        if path.is_dir() {
            subdirs.push(path);
        } else if path.is_file() {
            out.push(path);
        }
    }
    for sub in subdirs {
        collect_files(&sub, show_hidden, depth + 1, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_tree_indexing_and_hidden_filter() {
        let temp_dir = std::env::temp_dir().join("psychic_sidebar_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(temp_dir.join("src")).unwrap();
        fs::create_dir_all(temp_dir.join("target")).unwrap();

        fs::write(temp_dir.join(".env"), "SECRET=1").unwrap();
        fs::write(temp_dir.join("src/lib.rs"), "pub fn hello() {}").unwrap();
        fs::write(temp_dir.join("target/ignored.rs"), "// ignored").unwrap();

        let mut tree = WorkspaceTree::new(temp_dir.clone());
        assert!(tree.show_hidden);
        assert_eq!(tree.indexed_files.len(), 2);

        let (dirs, files) = tree.get_or_load_dir(&temp_dir);
        assert_eq!(dirs.len(), 2);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, ".env");

        // Toggle show_hidden = false
        tree.show_hidden = false;
        tree.rebuild_index();
        assert_eq!(tree.indexed_files.len(), 1);
        let (_, visible_files) = tree.get_or_load_dir(&temp_dir);
        assert!(visible_files.is_empty());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
