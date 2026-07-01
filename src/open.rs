use std::ffi::OsStr;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorCommand {
    pub command: String,
    pub fallback_from: Option<String>,
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

pub fn default_editor_command() -> String {
    if let Ok(editor) = std::env::var("EDITOR")
        && !editor.trim().is_empty()
    {
        return editor;
    }

    first_available_editor(std::env::var_os("PATH").as_deref()).unwrap_or_else(|| "vi".to_owned())
}

pub fn resolve_editor_command(preferred: &str) -> EditorCommand {
    resolve_editor_command_with_path(preferred, std::env::var_os("PATH").as_deref())
}

fn resolve_editor_command_with_path(preferred: &str, path_env: Option<&OsStr>) -> EditorCommand {
    let trimmed = preferred.trim();
    if !trimmed.is_empty() && command_exists(trimmed, path_env) {
        return EditorCommand {
            command: trimmed.to_owned(),
            fallback_from: None,
        };
    }

    let fallback = first_available_editor(path_env).unwrap_or_else(|| {
        if trimmed.is_empty() {
            "vi".to_owned()
        } else {
            trimmed.to_owned()
        }
    });
    EditorCommand {
        command: fallback,
        fallback_from: (!trimmed.is_empty()).then(|| trimmed.to_owned()),
    }
}

fn first_available_editor(path_env: Option<&OsStr>) -> Option<String> {
    ["nvim", "vim", "vi", "nano"]
        .into_iter()
        .find(|command| command_exists(command, path_env))
        .map(str::to_owned)
}

fn command_exists(command: &str, path_env: Option<&OsStr>) -> bool {
    let path = Path::new(command);
    if path.components().count() > 1 {
        return is_executable_file(path);
    }

    let Some(path_env) = path_env else {
        return false;
    };
    std::env::split_paths(path_env).any(|dir| is_executable_file(&dir.join(command)))
}

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    fn write_executable(path: &Path) {
        std::fs::write(path, "").unwrap();
        #[cfg(unix)]
        {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

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

    #[test]
    fn default_editor_uses_available_path_editor_without_editor_env() {
        let dir = tempfile::tempdir().unwrap();
        write_executable(&dir.path().join("vim"));

        assert_eq!(
            first_available_editor(Some(dir.path().as_os_str())),
            Some("vim".to_owned())
        );
    }

    #[test]
    fn editor_command_falls_back_when_preferred_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        write_executable(&dir.path().join("vim"));

        assert_eq!(
            resolve_editor_command_with_path("missing-editor", Some(dir.path().as_os_str())),
            EditorCommand {
                command: "vim".to_owned(),
                fallback_from: Some("missing-editor".to_owned()),
            }
        );
    }

    #[test]
    fn editor_command_keeps_available_preferred_editor() {
        let dir = tempfile::tempdir().unwrap();
        write_executable(&dir.path().join("hx"));

        assert_eq!(
            resolve_editor_command_with_path("hx", Some(dir.path().as_os_str())),
            EditorCommand {
                command: "hx".to_owned(),
                fallback_from: None,
            }
        );
    }
}
