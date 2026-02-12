use assert_cmd::Command;

#[test]
fn help_prints() {
    Command::cargo_bin("ubl")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn llm_help() {
    Command::cargo_bin("ubl")
        .unwrap()
        .args(["llm", "--help"])
        .assert()
        .success();
}

#[test]
fn llm_complete_help() {
    Command::cargo_bin("ubl")
        .unwrap()
        .args(["llm", "complete", "--help"])
        .assert()
        .success();
}

#[test]
fn llm_judge_help() {
    Command::cargo_bin("ubl")
        .unwrap()
        .args(["llm", "judge", "--help"])
        .assert()
        .success();
}

#[test]
fn pricing_help() {
    Command::cargo_bin("ubl")
        .unwrap()
        .args(["pricing", "--help"])
        .assert()
        .success();
}

#[test]
fn pricing_price_help() {
    Command::cargo_bin("ubl")
        .unwrap()
        .args(["pricing", "price", "--help"])
        .assert()
        .success();
}

#[test]
fn pricing_quote_help() {
    Command::cargo_bin("ubl")
        .unwrap()
        .args(["pricing", "quote", "--help"])
        .assert()
        .success();
}

#[test]
fn pricing_invoice_help() {
    Command::cargo_bin("ubl")
        .unwrap()
        .args(["pricing", "invoice", "--help"])
        .assert()
        .success();
}

#[test]
fn cap_help() {
    Command::cargo_bin("ubl")
        .unwrap()
        .args(["cap", "--help"])
        .assert()
        .success();
}

#[test]
fn permit_help() {
    Command::cargo_bin("ubl")
        .unwrap()
        .args(["permit", "--help"])
        .assert()
        .success();
}

#[test]
fn tdln_help() {
    Command::cargo_bin("ubl")
        .unwrap()
        .args(["tdln", "--help"])
        .assert()
        .success();
}

#[test]
fn tdln_policy_help() {
    Command::cargo_bin("ubl")
        .unwrap()
        .args(["tdln", "policy", "--help"])
        .assert()
        .success();
}

#[test]
fn tdln_runtime_help() {
    Command::cargo_bin("ubl")
        .unwrap()
        .args(["tdln", "runtime", "--help"])
        .assert()
        .success();
}

#[test]
fn llm_engine_help() {
    Command::cargo_bin("ubl")
        .unwrap()
        .args(["llm", "engine", "--help"])
        .assert()
        .success();
}

#[test]
fn llm_smart_help() {
    Command::cargo_bin("ubl")
        .unwrap()
        .args(["llm", "smart", "--help"])
        .assert()
        .success();
}
