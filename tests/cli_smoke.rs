use assert_cmd::Command as AssertCommand;
use at::cli::{Cli, Command as CliCommand};
use at::config::Config;
use at::history::{HistoryDb, HistorySource, PathKind};
use at::ui::theme::ThemeName;
use clap::Parser;
use predicates::prelude::PredicateBooleanExt;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

#[test]
fn help_mentions_core_commands() {
    AssertCommand::cargo_bin("at")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("menu"))
        .stdout(predicates::str::contains("recent"))
        .stdout(predicates::str::contains("search"));
}

#[test]
fn search_accepts_optional_query_without_launching_tui() {
    let cli = Cli::try_parse_from(["at", "search", "--shell", "nightlight", "loader"]).unwrap();
    let Some(CliCommand::Search { shell, query }) = cli.command else {
        panic!("expected search command");
    };

    assert!(shell);
    assert_eq!(
        CliCommand::search_query(&query).as_deref(),
        Some("nightlight loader")
    );

    let cli = Cli::try_parse_from(["at", "search"]).unwrap();
    let Some(CliCommand::Search { shell, query }) = cli.command else {
        panic!("expected search command");
    };

    assert!(!shell);
    assert_eq!(CliCommand::search_query(&query), None);
}

#[test]
fn setting_prints_config_path() {
    let config_home = tempfile::tempdir().unwrap();
    let expected = config_home.path().join("at").join("config.toml");

    AssertCommand::cargo_bin("at")
        .unwrap()
        .args(["setting", "--path"])
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success()
        .stdout(format!("{}\n", expected.display()));
}

#[test]
fn setting_opens_config_in_configured_editor() {
    let config_home = tempfile::tempdir().unwrap();
    let bin_dir = tempfile::tempdir().unwrap();
    let editor_log = bin_dir.path().join("editor.log");
    let editor = bin_dir.path().join("capture-editor");
    let config_path = config_home.path().join("at").join("config.toml");
    let mut config = Config::default();
    config.open.editor = editor.display().to_string();
    config.save_to(&config_path).unwrap();

    fs::write(
        &editor,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$1\" > '{}'\n",
            editor_log.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&editor, fs::Permissions::from_mode(0o755)).unwrap();

    AssertCommand::cargo_bin("at")
        .unwrap()
        .arg("setting")
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success()
        .stdout("");

    assert_eq!(
        fs::read_to_string(editor_log).unwrap().trim(),
        config_path.display().to_string()
    );
}

#[test]
fn init_accepts_newline_defaults_and_writes_config() {
    let home = tempfile::tempdir().unwrap();
    let config_home = tempfile::tempdir().unwrap();
    let config_path = config_home.path().join("at").join("config.toml");

    AssertCommand::cargo_bin("at")
        .unwrap()
        .arg("init")
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .env("XDG_CONFIG_HOME", config_home.path())
        .env_remove("EDITOR")
        .write_stdin("\n\n\n\n\n\n")
        .assert()
        .success()
        .stdout(predicates::str::contains("Atflow setup"))
        .stdout(predicates::str::contains("Install shell shortcuts"))
        .stdout(predicates::str::contains(format!(
            "Config saved to {}",
            config_path.display()
        )))
        .stdout(predicates::str::contains("Installed shell integration"))
        .stdout(predicates::str::contains("_atflow_record_cd").not());

    assert!(config_path.is_file());

    let config = Config::load_or_default(&config_path).unwrap();
    assert!(!config.history.record_shell_cd);
    assert!(config.general.start_from_git_root);
    assert_eq!(config.general.theme, ThemeName::Mist);
    assert_eq!(config.open.editor, "nvim");
    assert_eq!(config.search.roots, ["~/work", "~/code", "~/Documents"]);
}

#[test]
fn init_installs_shell_shortcuts_into_bash_profile() {
    let home = tempfile::tempdir().unwrap();
    let config_home = home.path().join(".config");
    let config_path = config_home.join("at").join("config.toml");
    let shell_path = config_home.join("at").join("shell.sh");
    let profile_path = home.path().join(".bashrc");

    AssertCommand::cargo_bin("at")
        .unwrap()
        .arg("init")
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .env("XDG_CONFIG_HOME", &config_home)
        .env_remove("EDITOR")
        .write_stdin("\n\n\n\n\n\n")
        .assert()
        .success()
        .stdout(predicates::str::contains(format!(
            "Config saved to {}",
            config_path.display()
        )))
        .stdout(predicates::str::contains(format!(
            "Installed shell integration at {}",
            shell_path.display()
        )))
        .stdout(predicates::str::contains(format!(
            "Updated shell profile {}",
            profile_path.display()
        )));

    let shell_script = fs::read_to_string(&shell_path).unwrap();
    assert!(shell_script.contains("@()"));
    assert!(shell_script.contains("@flow()"));
    assert!(!shell_script.contains("_atflow_record_cd"));

    let profile = fs::read_to_string(&profile_path).unwrap();
    assert!(profile.contains(&format!(
        ". {}",
        at::shell::shell_quote(&shell_path.display().to_string())
    )));
}

#[test]
fn init_fresh_config_uses_editor_env_default() {
    let home = tempfile::tempdir().unwrap();
    let config_home = tempfile::tempdir().unwrap();
    let config_path = config_home.path().join("at").join("config.toml");

    AssertCommand::cargo_bin("at")
        .unwrap()
        .arg("init")
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .env("XDG_CONFIG_HOME", config_home.path())
        .env("EDITOR", "hx")
        .write_stdin("\n\n\n\n\n\n")
        .assert()
        .success();

    let config = Config::load_or_default(&config_path).unwrap();
    assert_eq!(config.open.editor, "hx");
}

#[test]
fn init_cd_hook_only_prints_hook_guidance_and_saves_cd_history() {
    let home = tempfile::tempdir().unwrap();
    let config_home = tempfile::tempdir().unwrap();
    let config_path = config_home.path().join("at").join("config.toml");

    AssertCommand::cargo_bin("at")
        .unwrap()
        .arg("init")
        .env("HOME", home.path())
        .env("SHELL", "/bin/bash")
        .env("XDG_CONFIG_HOME", config_home.path())
        .write_stdin("n\ny\n\n\n\n\n")
        .assert()
        .success()
        .stdout(predicates::str::contains("@()").not())
        .stdout(predicates::str::contains("Installed shell integration"))
        .stdout(predicates::str::contains("_atflow_record_cd"));

    let config = Config::load_or_default(&config_path).unwrap();
    assert!(config.history.record_shell_cd);
}

#[test]
fn init_empty_stdin_fails_without_writing_config() {
    let config_home = tempfile::tempdir().unwrap();
    let config_path = config_home.path().join("at").join("config.toml");

    AssertCommand::cargo_bin("at")
        .unwrap()
        .arg("init")
        .env("XDG_CONFIG_HOME", config_home.path())
        .write_stdin("")
        .assert()
        .failure()
        .stdout(predicates::str::contains("@()").not());

    assert!(!config_path.exists());
}

#[test]
fn shell_print_outputs_functions() {
    AssertCommand::cargo_bin("at")
        .unwrap()
        .args(["shell", "print"])
        .assert()
        .success()
        .stdout(predicates::str::contains("@()"))
        .stdout(predicates::str::contains("@search()"));
}

#[test]
fn shell_hook_outputs_cd_recorder() {
    AssertCommand::cargo_bin("at")
        .unwrap()
        .args(["shell", "hook"])
        .assert()
        .success()
        .stdout(predicates::str::contains("_atflow_record_cd"))
        .stdout(predicates::str::contains("recent-record"));
}

#[test]
fn recent_record_defaults_to_noop_without_config() {
    let config_home = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let recorded_path = PathBuf::from("/tmp/atflow-cli-smoke");
    let db_path = data_home.path().join("at").join("history.sqlite");

    AssertCommand::cargo_bin("at")
        .unwrap()
        .args(["recent-record", recorded_path.to_str().unwrap()])
        .env("XDG_CONFIG_HOME", config_home.path())
        .env("XDG_DATA_HOME", data_home.path())
        .assert()
        .success()
        .stdout("");

    assert!(!db_path.exists());
}

#[test]
fn recent_record_writes_shell_cd_history_when_enabled() {
    let config_home = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let recorded_path = PathBuf::from("/tmp/atflow-cli-smoke");
    let config_path = config_home.path().join("at").join("config.toml");
    let db_path = data_home.path().join("at").join("history.sqlite");
    let mut config = Config::default();
    config.history.record_shell_cd = true;
    config.save_to(&config_path).unwrap();

    AssertCommand::cargo_bin("at")
        .unwrap()
        .args(["recent-record", recorded_path.to_str().unwrap()])
        .env("XDG_CONFIG_HOME", config_home.path())
        .env("XDG_DATA_HOME", data_home.path())
        .assert()
        .success()
        .stdout("");

    assert!(db_path.is_file());

    let db = HistoryDb::open(&db_path).unwrap();
    let recent = db.recent_dirs(10).unwrap();

    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].path, recorded_path);
    assert_eq!(recent[0].kind, PathKind::Dir);
    assert_eq!(recent[0].source, HistorySource::ShellCdHook);
}
