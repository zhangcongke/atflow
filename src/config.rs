use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ui::theme::ThemeName;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub open: OpenConfig,
    pub search: SearchConfig,
    pub history: HistoryConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub theme: ThemeName,
    pub max_recent: usize,
    pub start_from_git_root: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenConfig {
    pub editor: String,
    pub gui_editor: String,
    pub file_opener: String,
    pub prefer_terminal_editor: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchConfig {
    #[serde(default)]
    pub root_mode: SearchRootMode,
    pub roots: Vec<String>,
    pub ignore: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SearchRootMode {
    #[default]
    Invocation,
    Configured,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryConfig {
    pub record_atflow_opens: bool,
    pub record_shell_cd: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                theme: ThemeName::Mist,
                max_recent: 100,
                start_from_git_root: true,
            },
            open: OpenConfig {
                editor: "nvim".to_owned(),
                gui_editor: "code".to_owned(),
                file_opener: "xdg-open".to_owned(),
                prefer_terminal_editor: true,
            },
            search: SearchConfig {
                root_mode: SearchRootMode::Invocation,
                roots: vec![
                    "~/work".to_owned(),
                    "~/code".to_owned(),
                    "~/Documents".to_owned(),
                ],
                ignore: vec![
                    ".git".to_owned(),
                    "node_modules".to_owned(),
                    "__pycache__".to_owned(),
                    ".venv".to_owned(),
                    "target".to_owned(),
                    "dist".to_owned(),
                ],
            },
            history: HistoryConfig {
                record_atflow_opens: true,
                record_shell_cd: false,
            },
        }
    }
}

impl Config {
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path
            .try_exists()
            .with_context(|| format!("failed to inspect config {}", path.display()))?
        {
            return Ok(Self::default());
        }

        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read config {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("failed to parse config {}", path.display()))
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create config dir {}", parent.display()))?;
        }

        let text = toml::to_string_pretty(self).context("failed to serialize config")?;
        fs::write(path, text).with_context(|| format!("failed to write config {}", path.display()))
    }
}

pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("at")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_matches_mvp_defaults() {
        let config = Config::default();
        assert_eq!(config.general.theme, ThemeName::Mist);
        assert_eq!(config.general.max_recent, 100);
        assert!(config.general.start_from_git_root);
        assert_eq!(config.open.editor, "nvim");
        assert_eq!(config.open.gui_editor, "code");
        assert_eq!(config.open.file_opener, "xdg-open");
        assert!(config.open.prefer_terminal_editor);
        assert_eq!(config.search.root_mode, SearchRootMode::Invocation);
        assert_eq!(config.search.roots, ["~/work", "~/code", "~/Documents"]);
        assert_eq!(
            config.search.ignore,
            [
                ".git",
                "node_modules",
                "__pycache__",
                ".venv",
                "target",
                "dist"
            ]
        );
        assert!(config.history.record_atflow_opens);
        assert!(!config.history.record_shell_cd);
    }

    #[test]
    fn old_configs_without_search_root_mode_still_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"
[general]
theme = "mist"
max_recent = 100
start_from_git_root = true

[open]
editor = "nvim"
gui_editor = "code"
file_opener = "xdg-open"
prefer_terminal_editor = true

[search]
roots = ["~/work"]
ignore = [".git"]

[history]
record_atflow_opens = true
record_shell_cd = false
"#,
        )
        .unwrap();

        let config = Config::load_or_default(&path).unwrap();

        assert_eq!(config.search.root_mode, SearchRootMode::Invocation);
        assert_eq!(config.search.roots, ["~/work"]);
    }

    #[test]
    fn load_missing_config_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        assert_eq!(Config::load_or_default(&path).unwrap(), Config::default());
    }

    #[test]
    fn saves_and_loads_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("config.toml");
        let mut config = Config::default();
        config.general.theme = ThemeName::Paper;
        config.history.record_shell_cd = true;

        config.save_to(&path).unwrap();
        let loaded = Config::load_or_default(&path).unwrap();

        assert_eq!(loaded, config);
    }

    #[test]
    fn invalid_toml_reports_parse_context() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "not valid toml").unwrap();

        let error = Config::load_or_default(&path).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("failed to parse config"));
        assert!(message.contains(&path.display().to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn load_config_reports_existence_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::os::unix::fs::symlink(&path, &path).unwrap();

        let error = Config::load_or_default(&path).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("failed to inspect config"));
        assert!(message.contains(&path.display().to_string()));
    }
}
