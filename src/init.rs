use anyhow::{Context, Result, bail};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use crate::config::{Config, SearchRootMode, default_config_path};
use crate::ui::theme::ThemeName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitAnswers {
    pub install_shell_functions: bool,
    pub enable_cd_hook: bool,
    pub editor: String,
    pub search_root_mode: SearchRootMode,
    pub search_roots: Vec<String>,
    pub theme: ThemeName,
    pub start_from_git_root: bool,
}

impl Default for InitAnswers {
    fn default() -> Self {
        Self {
            install_shell_functions: true,
            enable_cd_hook: false,
            editor: crate::open::default_editor_command(),
            search_root_mode: SearchRootMode::Invocation,
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
    config.search.root_mode = answers.search_root_mode;
    config.search.roots = answers.search_roots.clone();
    config.general.theme = answers.theme;
    config.general.start_from_git_root = answers.start_from_git_root;
    config.history.record_shell_cd = answers.enable_cd_hook;
    config
}

pub fn answers_from_config(config: &Config) -> InitAnswers {
    InitAnswers {
        install_shell_functions: true,
        enable_cd_hook: config.history.record_shell_cd,
        editor: config.open.editor.clone(),
        search_root_mode: config.search.root_mode,
        search_roots: config.search.roots.clone(),
        theme: config.general.theme,
        start_from_git_root: config.general.start_from_git_root,
    }
}

pub fn run_init() -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_init_with(&mut stdin.lock(), &mut stdout.lock())
}

fn run_init_with<R: BufRead, W: Write>(input: &mut R, output: &mut W) -> Result<()> {
    writeln!(output, "AtFlow setup")?;

    let path = default_config_path();
    let has_config = path
        .try_exists()
        .with_context(|| format!("failed to inspect config {}", path.display()))?;
    let base = if has_config {
        Config::load_or_default(&path)?
    } else {
        Config::default()
    };
    let defaults = if has_config {
        answers_from_config(&base)
    } else {
        InitAnswers::default()
    };
    let answers = InitAnswers {
        install_shell_functions: prompt_bool(
            input,
            output,
            "Install shell shortcuts",
            defaults.install_shell_functions,
        )?,
        enable_cd_hook: prompt_bool(input, output, "Enable cd hook", defaults.enable_cd_hook)?,
        editor: prompt_string(input, output, "Editor", &defaults.editor)?,
        search_root_mode: prompt_search_root_mode(input, output, defaults.search_root_mode)?,
        search_roots: prompt_roots(input, output, &defaults.search_roots)?,
        theme: prompt_theme(input, output, defaults.theme)?,
        start_from_git_root: prompt_bool(
            input,
            output,
            "Start flow from git root",
            defaults.start_from_git_root,
        )?,
    };

    config_from_answers_with_base(base, &answers).save_to(&path)?;
    writeln!(output, "Config saved to {}", path.display())?;

    if answers.install_shell_functions || answers.enable_cd_hook {
        let shell_path = shell_integration_path(&path);
        write_shell_integration(
            &shell_path,
            answers.install_shell_functions,
            answers.enable_cd_hook,
        )?;
        let profile_path = default_profile_path();
        let updated = update_shell_profile(&profile_path, &shell_path)?;

        writeln!(output)?;
        writeln!(
            output,
            "Installed shell integration at {}",
            shell_path.display()
        )?;
        if updated {
            writeln!(output, "Updated shell profile {}", profile_path.display())?;
        } else {
            writeln!(
                output,
                "Shell profile already sources {}",
                shell_path.display()
            )?;
        }
        if answers.enable_cd_hook {
            writeln!(output, "Enabled cd hook _atflow_record_cd.")?;
        }
        writeln!(
            output,
            "Restart your shell or run: . {}",
            crate::shell::shell_quote(&shell_path.display().to_string())
        )?;
    }

    Ok(())
}

fn shell_integration_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("shell.sh")
}

fn write_shell_integration(
    path: &Path,
    include_functions: bool,
    include_cd_hook: bool,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create shell integration dir {}",
                parent.display()
            )
        })?;
    }

    let mut text = String::from("# Generated by AtFlow. Re-run `at init` to update this file.\n");
    if include_functions {
        text.push_str(crate::shell::functions_block());
        text.push('\n');
    }
    if include_cd_hook {
        if include_functions {
            text.push('\n');
        }
        text.push_str(crate::shell::cd_hook_block());
        text.push('\n');
    }

    fs::write(path, text)
        .with_context(|| format!("failed to write shell integration {}", path.display()))
}

fn default_profile_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    profile_path_for(&std::env::var("SHELL").unwrap_or_default(), &home)
}

fn profile_path_for(shell: &str, home: &Path) -> PathBuf {
    if shell.ends_with("zsh") {
        home.join(".zshrc")
    } else if shell.ends_with("bash") {
        home.join(".bashrc")
    } else {
        home.join(".profile")
    }
}

fn update_shell_profile(profile_path: &Path, shell_path: &Path) -> Result<bool> {
    if let Some(parent) = profile_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create shell profile dir {}", parent.display()))?;
    }

    let source_line = format!(
        ". {}",
        crate::shell::shell_quote(&shell_path.display().to_string())
    );
    let current = fs::read_to_string(profile_path).unwrap_or_default();
    if current.lines().any(|line| line.trim() == source_line) {
        return Ok(false);
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(profile_path)
        .with_context(|| format!("failed to update shell profile {}", profile_path.display()))?;
    if !current.is_empty() && !current.ends_with('\n') {
        writeln!(file)?;
    }
    writeln!(file, "# AtFlow shell integration")?;
    writeln!(file, "{source_line}")?;
    Ok(true)
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

fn prompt_search_root_mode<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    default: SearchRootMode,
) -> Result<SearchRootMode> {
    let default_number = match default {
        SearchRootMode::Invocation => 1,
        SearchRootMode::Configured => 2,
    };

    loop {
        match prompt_line(
            input,
            output,
            &format!("Search root mode 1) Invocation cwd 2) Configured roots [{default_number}]: "),
        )?
        .as_str()
        {
            "" => return Ok(default),
            "1" => return Ok(SearchRootMode::Invocation),
            "2" => return Ok(SearchRootMode::Configured),
            _ => writeln!(output, "Please choose 1 or 2.")?,
        }
    }
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
            search_root_mode: SearchRootMode::Configured,
            search_roots: vec!["~/src".to_owned(), "/opt/projects".to_owned()],
            theme: ThemeName::Paper,
            start_from_git_root: false,
        };

        let config = config_from_answers(&answers);

        assert_eq!(config.open.editor, "hx");
        assert_eq!(config.search.root_mode, SearchRootMode::Configured);
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
            search_root_mode: SearchRootMode::Configured,
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
        assert_eq!(config.search.root_mode, SearchRootMode::Configured);
        assert_eq!(config.search.roots, ["~/src"]);
        assert_eq!(config.general.theme, ThemeName::Ink);
        assert!(!config.general.start_from_git_root);
        assert!(config.history.record_shell_cd);
    }

    #[test]
    fn answers_from_config_uses_existing_wizard_values_as_defaults() {
        let mut base = Config::default();
        base.open.editor = "vim".to_owned();
        base.search.root_mode = SearchRootMode::Configured;
        base.search.roots = vec!["~/projects".to_owned()];
        base.general.theme = ThemeName::Ink;
        base.general.start_from_git_root = false;
        base.history.record_shell_cd = true;

        let answers = answers_from_config(&base);

        assert!(answers.install_shell_functions);
        assert!(answers.enable_cd_hook);
        assert_eq!(answers.editor, "vim");
        assert_eq!(answers.search_root_mode, SearchRootMode::Configured);
        assert_eq!(answers.search_roots, ["~/projects"]);
        assert_eq!(answers.theme, ThemeName::Ink);
        assert!(!answers.start_from_git_root);

        let config = config_from_answers_with_base(base, &answers);
        assert_eq!(config.open.editor, "vim");
        assert_eq!(config.search.root_mode, SearchRootMode::Configured);
        assert_eq!(config.search.roots, ["~/projects"]);
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

    #[test]
    fn prompt_search_root_mode_reprompts_after_invalid_input() {
        let mut input = Cursor::new("cwd\n2\n");
        let mut output = Vec::new();

        let mode =
            prompt_search_root_mode(&mut input, &mut output, SearchRootMode::Invocation).unwrap();

        assert_eq!(mode, SearchRootMode::Configured);
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("Please choose 1 or 2.")
        );
    }
}
