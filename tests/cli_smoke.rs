use assert_cmd::Command as AssertCommand;
use at::cli::{Cli, Command as CliCommand};
use at::config::Config;
use at::history::{HistoryDb, HistorySource, PathKind};
use clap::Parser;
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
        .arg("setting")
        .env("XDG_CONFIG_HOME", config_home.path())
        .assert()
        .success()
        .stdout(format!("{}\n", expected.display()));
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
