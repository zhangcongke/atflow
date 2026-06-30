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
