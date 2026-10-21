#![expect(
    clippy::unwrap_used,
    reason = "An invalid deterministic fixture must fail the integration test immediately."
)]

use std::{
    path::Path,
    process::{Command, Output},
};

#[path = "cli/packages.rs"]
mod packages;

fn command(state: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ejm-cli"));
    command
        .arg("--state-dir")
        .arg(state)
        .arg("--replay")
        .env_remove("EJM_CLI_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("EJM_REGISTRY_URL");
    command
}

fn cli(state: &Path, args: &[&str]) -> Output {
    command(state).args(args).output().unwrap()
}

fn json_events(output: &Output) -> Vec<serde_json::Value> {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn echo_streams_and_resume_outputs_only_the_new_turn() {
    let state = tempfile::tempdir().unwrap();
    let first = json_events(&cli(state.path(), &["run", "first prompt", "--json"]));
    assert_eq!(
        first
            .iter()
            .filter(|event| event["type"] == "run_finished")
            .count(),
        1
    );
    assert!(first.iter().any(|event| event["type"] == "text_delta"));
    let session = first[0]["session_id"].as_str().unwrap();
    let second = json_events(&cli(
        state.path(),
        &["--session", session, "run", "second prompt", "--json"],
    ));
    assert_eq!(
        second
            .iter()
            .filter(|event| event["type"] == "run_started")
            .count(),
        1
    );
    assert!(
        second
            .iter()
            .any(|event| event["prompt"] == "second prompt")
    );
    assert!(!second.iter().any(|event| event["prompt"] == "first prompt"));
    let mut previous = 0;
    for event in second {
        let sequence = event["sequence"].as_u64().unwrap();
        assert!(sequence > previous, "events must be ordered and unique");
        previous = sequence;
    }
    let output = cli(state.path(), &["sessions", "--json"]);
    let sessions: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(sessions.len(), 1);
}

#[test]
fn default_tui_requires_terminal_and_doctor_reports_grants() {
    let state = tempfile::tempdir().unwrap();
    let output = cli(state.path(), &[]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("TUI needs a terminal"));
    let output = cli(state.path(), &["doctor", "--allow-write"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Grants: read + write"));
    assert!(text.contains("not a security sandbox"));
}

#[test]
fn denied_shell_tool_fails_and_does_not_execute() {
    let state = tempfile::tempdir().unwrap();
    let fixture = state.path().join("responses.json");
    let marker = state.path().join("must-not-exist");
    std::fs::write(
        &fixture,
        serde_json::to_vec(&serde_json::json!([
            { "content": "", "tool_calls": [{
                "id": "denied", "name": "shell",
                "arguments": { "command": format!("touch {}", marker.display()) }
            }] },
            { "content": "Done" }
        ]))
        .unwrap(),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ejm-cli"))
        .arg("--state-dir")
        .arg(state.path().join("sessions"))
        .arg("--replay-file")
        .arg(&fixture)
        .args(["run", "test denied tool", "--json"])
        .output()
        .unwrap();
    let events: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(!marker.exists());
    assert!(events.iter().any(|event| event["type"] == "run_started"));
    assert!(events.iter().any(|event| {
        (event["type"] == "tool_finished" && event["output"]["is_error"] == true)
            || (event["type"] == "run_finished"
                && event["summary"]["status"] == "failed"
                && event["summary"]["error"]
                    .as_str()
                    .is_some_and(|error| error.contains("not granted")))
    }));
}

#[test]
fn accounts_are_local_and_anthropic_signin_stays_in_the_official_client() {
    let state = tempfile::tempdir().unwrap();
    let output = cli(state.path(), &["accounts", "--json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::json!([])
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(state.path().join("auth"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700);
    }
    let output = cli(state.path(), &["login", "anthropic"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("official Claude Code"));
    assert!(text.contains("does not collect"));
    assert!(text.contains("ANTHROPIC_API_KEY"));
    let output = cli(state.path(), &["logout", "missing-registration"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no matching saved account"));
}

#[test]
fn native_model_discovery_rejects_endpoint_overrides_before_accessing_credentials() {
    let state = tempfile::tempdir().unwrap();
    for provider in ["responses", "chatgpt"] {
        let output = cli(
            state.path(),
            &[
                "--provider",
                provider,
                "--base-url",
                "https://other.test/v1",
                "models",
            ],
        );
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("fixed official endpoint"));
        assert!(!state.path().join("auth").exists());
    }
}

#[test]
fn relay_metadata_and_rejected_controller_do_not_initialize_provider_or_journal() {
    let state = tempfile::tempdir().unwrap();
    let missing_replay = state.path().join("provider-must-not-be-opened.json");
    let command = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ejm-cli"));
        command
            .arg("--state-dir")
            .arg(state.path())
            .arg("--replay-file")
            .arg(&missing_replay)
            .env_remove("EJM_CLI_API_KEY")
            .env_remove("OPENAI_API_KEY")
            .env_remove("EJM_CLI_PROVIDER")
            .stdin(std::process::Stdio::null());
        command
    };
    let accounts = command().args(["remote", "accounts"]).output().unwrap();
    assert!(
        accounts.status.success(),
        "{}",
        String::from_utf8_lossy(&accounts.stderr)
    );
    assert!(String::from_utf8_lossy(&accounts.stdout).contains("No saved relay account."));
    let connect = command()
        .args(["--relay", "https://relay.example", "remote", "connect"])
        .output()
        .unwrap();
    assert!(!connect.status.success());
    assert!(String::from_utf8_lossy(&connect.stderr).contains("terminal"));
    assert_eq!(std::fs::read_dir(state.path()).unwrap().count(), 0);
}
