use anyhow::{Context, Result, bail};
use std::io::{self, BufRead, Write};

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
    config_from_answers_with_base(Config::default(), answers)
}

pub fn config_from_answers_with_base(mut config: Config, answers: &InitAnswers) -> Config {
    config.open.editor = answers.editor.clone();
    config.search.roots = answers.search_roots.clone();
    config.general.theme = answers.theme;
    config.general.start_from_git_root = answers.start_from_git_root;
    config.history.record_shell_cd = answers.enable_cd_hook;
    config
}

pub fn run_init() -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_init_with(&mut stdin.lock(), &mut stdout.lock())
}

fn run_init_with<R: BufRead, W: Write>(input: &mut R, output: &mut W) -> Result<()> {
    writeln!(output, "Atflow setup")?;

    let defaults = InitAnswers::default();
    let answers = InitAnswers {
        install_shell_functions: prompt_bool(
            input,
            output,
            "Print shell functions",
            defaults.install_shell_functions,
        )?,
        enable_cd_hook: prompt_bool(input, output, "Enable cd hook", defaults.enable_cd_hook)?,
        editor: prompt_string(input, output, "Editor", &defaults.editor)?,
        search_roots: prompt_roots(input, output, &defaults.search_roots)?,
        theme: prompt_theme(input, output, defaults.theme)?,
        start_from_git_root: prompt_bool(
            input,
            output,
            "Start flow from git root",
            defaults.start_from_git_root,
        )?,
    };

    let path = default_config_path();
    let base = Config::load_or_default(&path)?;
    config_from_answers_with_base(base, &answers).save_to(&path)?;
    writeln!(output, "Config saved to {}", path.display())?;

    if answers.install_shell_functions {
        writeln!(output)?;
        writeln!(output, "Add this to your shell profile:")?;
        writeln!(output, "{}", crate::shell::functions_block())?;
        writeln!(output, "Restart your shell or source your profile.")?;
    }

    if answers.enable_cd_hook {
        writeln!(output)?;
        writeln!(output, "{}", crate::shell::cd_hook_block())?;
    }

    Ok(())
}

fn prompt_bool<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    label: &str,
    default: bool,
) -> Result<bool> {
    let suffix = if default { "Y/n" } else { "y/N" };
    loop {
        match prompt_line(input, output, &format!("{label} [{suffix}]: "))?
            .to_lowercase()
            .as_str()
        {
            "" => return Ok(default),
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => writeln!(output, "Please answer yes or no.")?,
        }
    }
}

fn prompt_string<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    label: &str,
    default: &str,
) -> Result<String> {
    let answer = prompt_line(input, output, &format!("{label} [{default}]: "))?;
    if answer.is_empty() {
        Ok(default.to_owned())
    } else {
        Ok(answer)
    }
}

fn prompt_roots<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    default: &[String],
) -> Result<Vec<String>> {
    let joined = default.join(", ");
    let answer = prompt_line(input, output, &format!("Search roots [{joined}]: "))?;
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

fn prompt_theme<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    default: ThemeName,
) -> Result<ThemeName> {
    let default_number = match default {
        ThemeName::Mist => 1,
        ThemeName::Ink => 2,
        ThemeName::Paper => 3,
    };

    loop {
        match prompt_line(
            input,
            output,
            &format!("Theme 1) Mist 2) Ink 3) Paper [{default_number}]: "),
        )?
        .as_str()
        {
            "" => return Ok(default),
            "1" => return Ok(ThemeName::Mist),
            "2" => return Ok(ThemeName::Ink),
            "3" => return Ok(ThemeName::Paper),
            _ => writeln!(output, "Please choose 1, 2, or 3.")?,
        }
    }
}

fn prompt_line<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    prompt: &str,
) -> Result<String> {
    write!(output, "{prompt}")?;
    output.flush().context("failed to flush prompt")?;

    let mut answer = String::new();
    let bytes = input
        .read_line(&mut answer)
        .context("failed to read prompt")?;
    if bytes == 0 {
        bail!("no input provided for init prompt");
    }

    Ok(answer.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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

    #[test]
    fn config_from_answers_with_base_preserves_unowned_fields() {
        let mut base = Config::default();
        base.general.max_recent = 42;
        base.open.gui_editor = "zed".to_owned();
        base.open.file_opener = "gio open".to_owned();
        base.search.ignore = vec![".cache".to_owned(), "vendor".to_owned()];

        let answers = InitAnswers {
            install_shell_functions: true,
            enable_cd_hook: true,
            editor: "hx".to_owned(),
            search_roots: vec!["~/src".to_owned()],
            theme: ThemeName::Ink,
            start_from_git_root: false,
        };

        let config = config_from_answers_with_base(base, &answers);

        assert_eq!(config.general.max_recent, 42);
        assert_eq!(config.open.gui_editor, "zed");
        assert_eq!(config.open.file_opener, "gio open");
        assert_eq!(config.search.ignore, [".cache", "vendor"]);
        assert_eq!(config.open.editor, "hx");
        assert_eq!(config.search.roots, ["~/src"]);
        assert_eq!(config.general.theme, ThemeName::Ink);
        assert!(!config.general.start_from_git_root);
        assert!(config.history.record_shell_cd);
    }

    #[test]
    fn prompt_line_errors_on_eof() {
        let mut input = Cursor::new("");
        let mut output = Vec::new();

        let error = prompt_line(&mut input, &mut output, "Editor [nvim]: ").unwrap_err();

        assert!(error.to_string().contains("no input"));
    }

    #[test]
    fn prompt_bool_reprompts_after_invalid_input() {
        let mut input = Cursor::new("maybe\nyes\n");
        let mut output = Vec::new();

        let answer = prompt_bool(&mut input, &mut output, "Print shell functions", false).unwrap();

        assert!(answer);
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("Please answer yes or no.")
        );
    }

    #[test]
    fn prompt_theme_reprompts_after_invalid_input() {
        let mut input = Cursor::new("mist\n2\n");
        let mut output = Vec::new();

        let theme = prompt_theme(&mut input, &mut output, ThemeName::Mist).unwrap();

        assert_eq!(theme, ThemeName::Ink);
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("Please choose 1, 2, or 3.")
        );
    }
}
