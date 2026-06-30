use anyhow::{Context, Result};
use std::io::{self, Write};

use crate::config::{Config, default_config_path};
use crate::ui::theme::ThemeName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitAnswers {
    pub install_shell_functions: bool,
    pub enable_cd_hook: bool,
    pub editor: String,
    pub search_roots: Vec<String>,
    pub theme: ThemeName,
    pub start_from_git_root: bool,
}

impl Default for InitAnswers {
    fn default() -> Self {
        Self {
            install_shell_functions: true,
            enable_cd_hook: false,
            editor: std::env::var("EDITOR")
                .ok()
                .filter(|editor| !editor.trim().is_empty())
                .unwrap_or_else(|| "nvim".to_owned()),
            search_roots: vec![
                "~/work".to_owned(),
                "~/code".to_owned(),
                "~/Documents".to_owned(),
            ],
            theme: ThemeName::Mist,
            start_from_git_root: true,
        }
    }
}

pub fn config_from_answers(answers: &InitAnswers) -> Config {
    let mut config = Config::default();
    config.open.editor = answers.editor.clone();
    config.search.roots = answers.search_roots.clone();
    config.general.theme = answers.theme;
    config.general.start_from_git_root = answers.start_from_git_root;
    config.history.record_shell_cd = answers.enable_cd_hook;
    config
}

pub fn run_init() -> Result<()> {
    println!("Atflow setup");

    let defaults = InitAnswers::default();
    let answers = InitAnswers {
        install_shell_functions: prompt_bool(
            "Install shell functions",
            defaults.install_shell_functions,
        )?,
        enable_cd_hook: prompt_bool("Enable cd hook", defaults.enable_cd_hook)?,
        editor: prompt_string("Editor", &defaults.editor)?,
        search_roots: prompt_roots(&defaults.search_roots)?,
        theme: prompt_theme(defaults.theme)?,
        start_from_git_root: prompt_bool("Start flow from git root", defaults.start_from_git_root)?,
    };

    config_from_answers(&answers).save_to(&default_config_path())?;

    if answers.install_shell_functions {
        println!();
        println!("{}", crate::shell::functions_block());
    }

    if answers.enable_cd_hook {
        println!();
        println!("{}", crate::shell::cd_hook_block());
    }

    Ok(())
}

fn prompt_bool(label: &str, default: bool) -> Result<bool> {
    let suffix = if default { "Y/n" } else { "y/N" };
    match prompt_line(&format!("{label} [{suffix}]: "))?
        .to_lowercase()
        .as_str()
    {
        "" => Ok(default),
        "y" | "yes" => Ok(true),
        "n" | "no" => Ok(false),
        _ => Ok(default),
    }
}

fn prompt_string(label: &str, default: &str) -> Result<String> {
    let answer = prompt_line(&format!("{label} [{default}]: "))?;
    if answer.is_empty() {
        Ok(default.to_owned())
    } else {
        Ok(answer)
    }
}

fn prompt_roots(default: &[String]) -> Result<Vec<String>> {
    let joined = default.join(", ");
    let answer = prompt_line(&format!("Search roots [{joined}]: "))?;
    if answer.is_empty() {
        return Ok(default.to_vec());
    }

    Ok(answer
        .split(',')
        .map(str::trim)
        .filter(|root| !root.is_empty())
        .map(str::to_owned)
        .collect())
}

fn prompt_theme(default: ThemeName) -> Result<ThemeName> {
    let default_number = match default {
        ThemeName::Mist => 1,
        ThemeName::Ink => 2,
        ThemeName::Paper => 3,
    };
    match prompt_line(&format!(
        "Theme 1) Mist 2) Ink 3) Paper [{default_number}]: "
    ))?
    .as_str()
    {
        "" => Ok(default),
        "1" => Ok(ThemeName::Mist),
        "2" => Ok(ThemeName::Ink),
        "3" => Ok(ThemeName::Paper),
        _ => Ok(default),
    }
}

fn prompt_line(prompt: &str) -> Result<String> {
    print!("{prompt}");
    io::stdout().flush().context("failed to flush prompt")?;

    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .context("failed to read prompt")?;
    Ok(answer.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_from_answers_updates_wizard_fields() {
        let answers = InitAnswers {
            install_shell_functions: false,
            enable_cd_hook: true,
            editor: "hx".to_owned(),
            search_roots: vec!["~/src".to_owned(), "/opt/projects".to_owned()],
            theme: ThemeName::Paper,
            start_from_git_root: false,
        };

        let config = config_from_answers(&answers);

        assert_eq!(config.open.editor, "hx");
        assert_eq!(config.search.roots, ["~/src", "/opt/projects"]);
        assert_eq!(config.general.theme, ThemeName::Paper);
        assert!(!config.general.start_from_git_root);
        assert!(config.history.record_shell_cd);
        assert_eq!(config.open.gui_editor, "code");
        assert!(config.history.record_atflow_opens);
    }
}
