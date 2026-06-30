use assert_cmd::Command;
use at::history::{HistoryDb, HistorySource, PathKind};
use std::path::PathBuf;

#[test]
fn help_mentions_core_commands() {
    Command::cargo_bin("at")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("menu"))
        .stdout(predicates::str::contains("recent"))
        .stdout(predicates::str::contains("search"));
}

#[test]
fn search_accepts_optional_query() {
    Command::cargo_bin("at")
        .unwrap()
        .args(["search", "--shell", "nightlight"])
        .assert()
        .success()
        .stdout(predicates::str::contains("query=nightlight"));
}

#[test]
fn shell_print_outputs_functions() {
    Command::cargo_bin("at")
        .unwrap()
        .args(["shell", "print"])
        .assert()
        .success()
        .stdout(predicates::str::contains("@()"))
        .stdout(predicates::str::contains("@search()"));
}

#[test]
fn shell_hook_outputs_cd_recorder() {
    Command::cargo_bin("at")
        .unwrap()
        .args(["shell", "hook"])
        .assert()
        .success()
        .stdout(predicates::str::contains("_atflow_record_cd"))
        .stdout(predicates::str::contains("recent-record"));
}

#[test]
fn recent_record_writes_shell_cd_history() {
    let data_home = tempfile::tempdir().unwrap();
    let recorded_path = PathBuf::from("/tmp/atflow-cli-smoke");
    let db_path = data_home.path().join("at").join("history.sqlite");

    Command::cargo_bin("at")
        .unwrap()
        .args(["recent-record", recorded_path.to_str().unwrap()])
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
