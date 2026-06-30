use assert_cmd::Command;

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
