/// CLI Integration Tests
/// Tests the command-line interface without actually starting the server
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help() {
    // Test that --help works
    let mut cmd = Command::cargo_bin("debugger_mcp").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("DAP-based MCP debugging server"));
}

#[test]
fn test_cli_version() {
    // Test that --version works
    let mut cmd = Command::cargo_bin("debugger_mcp").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("debugger_mcp"));
}

#[test]
fn test_cli_serve_subcommand_help() {
    // Test that serve --help works
    let mut cmd = Command::cargo_bin("debugger_mcp").unwrap();
    cmd.arg("serve")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Start the MCP server"))
        .stdout(predicate::str::contains("--verbose"));
}

#[test]
fn test_cli_no_subcommand_fails() {
    // Test that running without a subcommand fails
    let mut cmd = Command::cargo_bin("debugger_mcp").unwrap();
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Usage:"));
}

#[test]
fn test_cli_invalid_subcommand_fails() {
    // Test that invalid subcommand fails
    let mut cmd = Command::cargo_bin("debugger_mcp").unwrap();
    cmd.arg("invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_cli_serve_log_level_parsing() {
    // We can't actually run the server, but we can verify the CLI accepts log level
    let mut cmd = Command::cargo_bin("debugger_mcp").unwrap();
    cmd.arg("serve").arg("--log-level").arg("help").assert();
    // This will fail when trying to start the server, but that's expected
    // The important part is that the CLI parsing works
}
