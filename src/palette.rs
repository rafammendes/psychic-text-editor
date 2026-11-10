use crate::highlighter::AVAILABLE_THEMES;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub enum PaletteMode {
    Closed,
    CommandPalette,
    QuickOpen,
    GotoLine,
    SyntaxSelector,
    OpenPath,
    SaveAsPath,
    OpenFolderPath,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EditorCommand {
    NewTab,
    QuickOpenPrompt,
    Save,
    SaveAs,
    RevertFile,
    OpenFilePrompt,
    OpenFolderPrompt,
    CloseActiveTab,
    CloseOtherTabs,
    CloseAllTabs,
    DuplicateLine,
    DeleteLine,
    MoveLineUp,
    MoveLineDown,
    ToggleComment,
    SelectNextOccurrence,
    SelectAllOccurrences,
    SplitSelectionIntoLines,
    Undo,
    Redo,
    ToggleLineEnding,
    ToggleSidebar,
    ToggleMinimap,
    ToggleMarkdownPreview,
    CopyMarkdownHtml,
    ToggleSearch,
    ToggleWordWrap,
    GotoLinePrompt,
    ChangeSyntaxPrompt,
    SetSyntax(String),
    SetTheme(String),
    FontSizeIncrease,
    FontSizeDecrease,
    FontSizeReset,
}

#[derive(Clone, Debug)]
pub struct CommandItem {
    pub category: &'static str,
    pub label: String,
    pub shortcut: &'static str,
    pub command: EditorCommand,
}

pub fn built_in_commands() -> Vec<CommandItem> {
    let mut cmds = vec![
        CommandItem {
            category: "Goto",
            label: "Goto Anything / Quick Open File...".to_string(),
            shortcut: "Ctrl+P",
            command: EditorCommand::QuickOpenPrompt,
        },
        CommandItem {
            category: "Goto",
            label: "Goto Line... (Ir para Linha)".to_string(),
            shortcut: "Ctrl+G",
            command: EditorCommand::GotoLinePrompt,
        },
        CommandItem {
            category: "File",
            label: "New File (Novo Arquivo)".to_string(),
            shortcut: "Ctrl+N",
            command: EditorCommand::NewTab,
        },
        CommandItem {
            category: "File",
            label: "Open File by Path... (Abrir por Caminho)".to_string(),
            shortcut: "Ctrl+O",
            command: EditorCommand::OpenFilePrompt,
        },
        CommandItem {
            category: "File",
            label: "Open Folder / Project... (Abrir Pasta)".to_string(),
            shortcut: "Ctrl+K",
            command: EditorCommand::OpenFolderPrompt,
        },
        CommandItem {
            category: "File",
            label: "Save (Salvar)".to_string(),
            shortcut: "Ctrl+S",
            command: EditorCommand::Save,
        },
        CommandItem {
            category: "File",
            label: "Save As... (Salvar Como)".to_string(),
            shortcut: "Ctrl+Shift+S",
            command: EditorCommand::SaveAs,
        },
        CommandItem {
            category: "File",
            label: "Revert File from Disk (Recarregar do Disco)".to_string(),
            shortcut: "",
            command: EditorCommand::RevertFile,
        },
        CommandItem {
            category: "File",
            label: "Close Tab (Fechar Aba Atual)".to_string(),
            shortcut: "Ctrl+W",
            command: EditorCommand::CloseActiveTab,
        },
        CommandItem {
            category: "File",
            label: "Close Other Tabs (Fechar Outras Abas)".to_string(),
            shortcut: "",
            command: EditorCommand::CloseOtherTabs,
        },
        CommandItem {
            category: "File",
            label: "Close All Tabs (Fechar Todas as Abas)".to_string(),
            shortcut: "",
            command: EditorCommand::CloseAllTabs,
        },
        CommandItem {
            category: "Edit",
            label: "Duplicate Line (Duplicar Linha Atual)".to_string(),
            shortcut: "Ctrl+Shift+D",
            command: EditorCommand::DuplicateLine,
        },
        CommandItem {
            category: "Edit",
            label: "Delete Line (Remover Linha Atual)".to_string(),
            shortcut: "Ctrl+Shift+K",
            command: EditorCommand::DeleteLine,
        },
        CommandItem {
            category: "Edit",
            label: "Move Line Up (Mover Linha para Cima)".to_string(),
            shortcut: "Ctrl+Shift+Up",
            command: EditorCommand::MoveLineUp,
        },
        CommandItem {
            category: "Edit",
            label: "Move Line Down (Mover Linha para Baixo)".to_string(),
            shortcut: "Ctrl+Shift+Down",
            command: EditorCommand::MoveLineDown,
        },
        CommandItem {
            category: "Edit",
            label: "Toggle Comment (Comentar / Descomentar Linha)".to_string(),
            shortcut: "Ctrl+/",
            command: EditorCommand::ToggleComment,
        },
        CommandItem {
            category: "Selection",
            label: "Select Next Occurrence / Add Cursor (Próxima Ocorrência)".to_string(),
            shortcut: "Ctrl+D",
            command: EditorCommand::SelectNextOccurrence,
        },
        CommandItem {
            category: "Selection",
            label: "Select All Occurrences (Selecionar Todas as Ocorrências)".to_string(),
            shortcut: "Alt+F3",
            command: EditorCommand::SelectAllOccurrences,
        },
        CommandItem {
            category: "Selection",
            label: "Split Selection into Lines (Um Cursor por Linha)".to_string(),
            shortcut: "Ctrl+Shift+L",
            command: EditorCommand::SplitSelectionIntoLines,
        },
        CommandItem {
            category: "Edit",
            label: "Undo (Desfazer)".to_string(),
            shortcut: "Ctrl+Z",
            command: EditorCommand::Undo,
        },
        CommandItem {
            category: "Edit",
            label: "Redo (Refazer)".to_string(),
            shortcut: "Ctrl+Y",
            command: EditorCommand::Redo,
        },
        CommandItem {
            category: "Edit",
            label: "Toggle Line Ending LF / CRLF".to_string(),
            shortcut: "",
            command: EditorCommand::ToggleLineEnding,
        },
        CommandItem {
            category: "Find",
            label: "Find & Replace (Buscar e Substituir)".to_string(),
            shortcut: "Ctrl+F / Ctrl+H",
            command: EditorCommand::ToggleSearch,
        },
        CommandItem {
            category: "View",
            label: "Toggle Sidebar (Mostrar/Ocultar Barra Lateral)".to_string(),
            shortcut: "Ctrl+B",
            command: EditorCommand::ToggleSidebar,
        },
        CommandItem {
            category: "View",
            label: "Toggle Minimap (Mostrar/Ocultar Minimapa)".to_string(),
            shortcut: "Alt+M",
            command: EditorCommand::ToggleMinimap,
        },
        CommandItem {
            category: "View",
            label: "Toggle Markdown Preview (Visualizar Markdown Lado a Lado)".to_string(),
            shortcut: "Ctrl+Shift+M",
            command: EditorCommand::ToggleMarkdownPreview,
        },
        CommandItem {
            category: "Markdown",
            label: "Copy Markdown as HTML (Copiar como HTML)".to_string(),
            shortcut: "",
            command: EditorCommand::CopyMarkdownHtml,
        },
        CommandItem {
            category: "View",
            label: "Toggle Word Wrap (Quebra de Linha Automática)".to_string(),
            shortcut: "Alt+Z",
            command: EditorCommand::ToggleWordWrap,
        },
        CommandItem {
            category: "Syntax",
            label: "Set Syntax... (Escolher Linguagem)".to_string(),
            shortcut: "",
            command: EditorCommand::ChangeSyntaxPrompt,
        },
        CommandItem {
            category: "View",
            label: "Zoom In (Aumentar Fonte)".to_string(),
            shortcut: "Ctrl+=",
            command: EditorCommand::FontSizeIncrease,
        },
        CommandItem {
            category: "View",
            label: "Zoom Out (Diminuir Fonte)".to_string(),
            shortcut: "Ctrl+-",
            command: EditorCommand::FontSizeDecrease,
        },
        CommandItem {
            category: "View",
            label: "Reset Zoom (Tamanho Padrão da Fonte)".to_string(),
            shortcut: "Ctrl+0",
            command: EditorCommand::FontSizeReset,
        },
    ];

    for (theme_id, display_name) in AVAILABLE_THEMES {
        cmds.push(CommandItem {
            category: "Theme",
            label: (*display_name).to_string(),
            shortcut: "",
            command: EditorCommand::SetTheme((*theme_id).to_string()),
        });
    }

    cmds
}

pub fn fuzzy_match_score(query: &str, candidate: &str) -> Option<i32> {
    let q_trimmed = query.trim();
    if q_trimmed.is_empty() {
        return Some(0);
    }
    let q_lower = q_trimmed.to_lowercase();
    let c_lower = candidate.to_lowercase();

    if c_lower == q_lower {
        return Some(2000);
    }

    if let Some(pos) = c_lower.find(&q_lower) {
        let mut bonus = 1000 - (pos as i32) * 5 - (candidate.len() as i32);
        // Bonus if matching right after a path separator `/` or space
        if pos == 0
            || matches!(
                c_lower.as_bytes().get(pos.wrapping_sub(1)),
                Some(b'/' | b' ' | b':' | b'_' | b'-')
            )
        {
            bonus += 150;
        }
        return Some(bonus);
    }

    // Subsequence fuzzy match
    let mut score = 0i32;
    let mut c_chars = c_lower.chars().enumerate();
    let mut last_idx = 0usize;

    for q_ch in q_lower.chars() {
        if q_ch.is_whitespace() {
            continue;
        }
        let mut found = false;
        for (idx, c_ch) in c_chars.by_ref() {
            if c_ch == q_ch {
                found = true;
                if idx == 0 || idx == last_idx + 1 {
                    score += 20;
                } else {
                    score -= (idx - last_idx) as i32;
                }
                last_idx = idx;
                break;
            }
        }
        if !found {
            return None;
        }
    }

    Some(score)
}

/// Cache para evitar chamar `fs::read_dir` a cada frame quando o modal de caminho está aberto
#[derive(Clone, Default)]
pub struct PathCompletionCache {
    pub cached_dir: Option<PathBuf>,
    pub entries: Vec<PathBuf>,
}

impl PathCompletionCache {
    pub fn get_entries(&mut self, dir: &std::path::Path) -> &[PathBuf] {
        if self.cached_dir.as_deref() != Some(dir) {
            self.cached_dir = Some(dir.to_path_buf());
            self.entries.clear();
            if let Ok(read_dir) = std::fs::read_dir(dir) {
                self.entries = read_dir.flatten().map(|e| e.path()).collect();
                self.entries.sort_by(|a, b| {
                    b.is_dir()
                        .cmp(&a.is_dir())
                        .then_with(|| a.file_name().cmp(&b.file_name()))
                });
            }
        }
        &self.entries
    }

    pub fn invalidate(&mut self) {
        self.cached_dir = None;
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match_ranking() {
        let exact = fuzzy_match_score("Save", "Save").unwrap();
        let prefix = fuzzy_match_score("Save", "Save As...").unwrap();
        let subseq = fuzzy_match_score("sva", "Save As...").unwrap();

        assert!(exact > prefix);
        assert!(prefix > subseq);
        assert!(fuzzy_match_score("zzz", "Save As...").is_none());
        assert_eq!(fuzzy_match_score("   ", "Anything"), Some(0));
    }

    #[test]
    fn test_built_in_commands_not_empty() {
        let cmds = built_in_commands();
        assert!(cmds.len() >= 18);
        assert!(cmds.iter().any(|c| c.command == EditorCommand::Save));
        assert!(cmds.iter().any(|c| c.command == EditorCommand::DuplicateLine));
    }

    #[test]
    fn test_path_completion_cache_reads_once() {
        let temp_dir = std::env::temp_dir().join("psychic_path_cache_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("pasta_a")).unwrap();
        std::fs::write(temp_dir.join("arquivo.txt"), "ok").unwrap();

        let mut cache = PathCompletionCache::default();
        let list = cache.get_entries(&temp_dir);
        assert_eq!(list.len(), 2);
        assert!(list[0].is_dir(), "Directories should be sorted first");

        cache.invalidate();
        assert!(cache.cached_dir.is_none());
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
