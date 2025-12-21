use assert_cmd::Command;
use predicates::prelude::*;

fn groove() -> Command {
    Command::cargo_bin("groove").unwrap()
}

#[test]
fn test_help() {
    groove()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("GrooveHQ CLI"));
}

#[test]
fn test_version() {
    groove()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("groove"));
}

#[test]
fn test_missing_token() {
    groove()
        .arg("me")
        .env_remove("GROOVEHQ_API_TOKEN")
        .assert()
        .failure()
        .stderr(predicate::str::contains("API token not found"));
}

#[test]
fn test_config_path() {
    groove()
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.toml"));
}

#[test]
fn test_invalid_subcommand() {
    groove().arg("invalid").assert().failure();
}

#[test]
fn test_conversation_help() {
    groove()
        .args(["conversation", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("view"))
        .stdout(predicate::str::contains("reply"))
        .stdout(predicate::str::contains("close"))
        .stdout(predicate::str::contains("open"))
        .stdout(predicate::str::contains("snooze"))
        .stdout(predicate::str::contains("assign"))
        .stdout(predicate::str::contains("unassign"))
        .stdout(predicate::str::contains("add-tag"))
        .stdout(predicate::str::contains("remove-tag"))
        .stdout(predicate::str::contains("note"));
}

#[test]
fn test_folder_help() {
    groove()
        .args(["folder", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"));
}

#[test]
fn test_tag_help() {
    groove()
        .args(["tag", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"));
}

#[test]
fn test_canned_replies_help() {
    groove()
        .args(["canned-replies", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("show"));
}

#[test]
fn test_config_help() {
    groove()
        .args(["config", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("set-token"))
        .stdout(predicate::str::contains("path"));
}

#[test]
fn test_quiet_flag() {
    groove()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--quiet"))
        .stdout(predicate::str::contains("Suppress success messages"));
}

#[test]
fn test_format_flag() {
    groove()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--format"))
        .stdout(predicate::str::contains("table"))
        .stdout(predicate::str::contains("json"))
        .stdout(predicate::str::contains("compact"));
}

#[test]
fn test_conversation_aliases() {
    // 'conv' should work
    groove().args(["conv", "--help"]).assert().success();

    // 'c' should work
    groove().args(["c", "--help"]).assert().success();
}

#[test]
fn test_folder_alias() {
    groove().args(["f", "--help"]).assert().success();
}

#[test]
fn test_tag_alias() {
    groove().args(["t", "--help"]).assert().success();
}

#[test]
fn test_canned_replies_alias() {
    groove().args(["canned", "--help"]).assert().success();
}

#[test]
fn test_conversation_list_aliases() {
    // 'ls' should work
    groove().args(["conv", "ls", "--help"]).assert().success();

    // 'l' should work
    groove().args(["conv", "l", "--help"]).assert().success();
}

#[test]
fn test_conversation_reply_alias() {
    groove().args(["conv", "r", "--help"]).assert().success();
}

#[test]
fn test_conversation_reply_canned_flag() {
    groove()
        .args(["conv", "reply", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--canned"));
}
