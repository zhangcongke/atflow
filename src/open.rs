use std::path::{Path, PathBuf};

use crate::config::Config;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenAction {
    Cd(PathBuf),
    Editor { command: String, path: PathBuf },
    System { command: String, path: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
    Default,
    Editor,
    System,
}

pub fn resolve_open_action(
    path: &Path,
    is_dir: bool,
    mode: OpenMode,
    config: &Config,
) -> OpenAction {
    if is_dir {
        return OpenAction::Cd(path.to_path_buf());
    }

    match mode {
        OpenMode::Editor => OpenAction::Editor {
            command: config.open.editor.clone(),
            path: path.to_path_buf(),
        },
        OpenMode::System => OpenAction::System {
            command: config.open.file_opener.clone(),
            path: path.to_path_buf(),
        },
        OpenMode::Default => {
            if is_text_or_code(path) {
                OpenAction::Editor {
                    command: config.open.editor.clone(),
                    path: path.to_path_buf(),
                }
            } else {
                OpenAction::System {
                    command: config.open.file_opener.clone(),
                    path: path.to_path_buf(),
                }
            }
        }
    }
}

pub fn is_text_or_code(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    matches!(
        extension.as_str(),
        "rs" | "py"
            | "js"
            | "ts"
            | "tsx"
            | "jsx"
            | "md"
            | "txt"
            | "toml"
            | "json"
            | "yaml"
            | "yml"
            | "sh"
            | "zsh"
            | "bash"
            | "html"
            | "css"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directories_resolve_to_cd() {
        let config = Config::default();
        assert_eq!(
            resolve_open_action(Path::new("/tmp/project"), true, OpenMode::Default, &config),
            OpenAction::Cd(PathBuf::from("/tmp/project"))
        );
    }

    #[test]
    fn rust_files_use_editor_by_default() {
        let config = Config::default();
        assert_eq!(
            resolve_open_action(Path::new("/tmp/main.rs"), false, OpenMode::Default, &config),
            OpenAction::Editor {
                command: config.open.editor.clone(),
                path: PathBuf::from("/tmp/main.rs")
            }
        );
    }

    #[test]
    fn images_use_system_opener_by_default() {
        let config = Config::default();
        assert_eq!(
            resolve_open_action(Path::new("/tmp/map.png"), false, OpenMode::Default, &config),
            OpenAction::System {
                command: "xdg-open".to_owned(),
                path: PathBuf::from("/tmp/map.png")
            }
        );
    }

    #[test]
    fn explicit_editor_mode_uses_editor_for_non_text_files() {
        let config = Config::default();
        assert_eq!(
            resolve_open_action(Path::new("/tmp/map.png"), false, OpenMode::Editor, &config),
            OpenAction::Editor {
                command: config.open.editor.clone(),
                path: PathBuf::from("/tmp/map.png")
            }
        );
    }

    #[test]
    fn explicit_system_mode_uses_system_opener_for_code_files() {
        let config = Config::default();
        assert_eq!(
            resolve_open_action(Path::new("/tmp/main.rs"), false, OpenMode::System, &config),
            OpenAction::System {
                command: config.open.file_opener.clone(),
                path: PathBuf::from("/tmp/main.rs")
            }
        );
    }

    #[test]
    fn directories_resolve_to_cd_with_explicit_modes() {
        let config = Config::default();
        assert_eq!(
            resolve_open_action(Path::new("/tmp/project"), true, OpenMode::Editor, &config),
            OpenAction::Cd(PathBuf::from("/tmp/project"))
        );
        assert_eq!(
            resolve_open_action(Path::new("/tmp/project"), true, OpenMode::System, &config),
            OpenAction::Cd(PathBuf::from("/tmp/project"))
        );
    }

    #[test]
    fn uppercase_text_and_code_extensions_use_editor_by_default() {
        let config = Config::default();
        for path in ["/tmp/README.MD", "/tmp/main.RS", "/tmp/config.JSON"] {
            assert_eq!(
                resolve_open_action(Path::new(path), false, OpenMode::Default, &config),
                OpenAction::Editor {
                    command: config.open.editor.clone(),
                    path: PathBuf::from(path)
                }
            );
        }
    }
}
