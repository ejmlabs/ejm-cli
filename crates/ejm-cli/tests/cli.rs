#![expect(
    clippy::unwrap_used,
    reason = "An invalid deterministic fixture must fail the integration test immediately."
)]

use std::{
    path::Path,
    process::{Command, Output},
    sync::OnceLock,
};

#[path = "cli/packages.rs"]
mod packages;
mod provider_fixture;
#[cfg(unix)]
#[path = "cli/startup.rs"]
mod startup;
use provider_fixture::ProviderFixture;

fn command(state: &Path, claude: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ejm-cli"));
    command
        .arg("--state-dir")
        .arg(state)
        .arg("--claude-path")
        .arg(claude)
        .env("HOME", state)
        .env("USERPROFILE", state)
        .env("LOCALAPPDATA", state.join("local"))
        .env("APPDATA", state.join("roaming"))
        .env("XDG_CONFIG_HOME", state.join("config"))
        .env("XDG_DATA_HOME", state.join("data"))
        .env_remove("EJM_REGISTRY_URL");
    for (name, _) in std::env::vars_os() {
        let key = name.to_string_lossy();
        if key.starts_with("EJM_CLI_")
            || key.starts_with("ANTHROPIC_")
            || key.starts_with("CLAUDE_")
            || matches!(
                key.as_ref(),
                "OPENAI_API_KEY" | "OPENAI_BASE_URL" | "CLAUDECODE"
            )
        {
            command.env_remove(name);
        }
    }
    command.env("CLAUDE_CONFIG_DIR", state.join("official-claude"));
    command
}

fn cli_with_claude(state: &Path, args: &[&str], claude: &Path) -> Output {
    command(state, claude).args(args).output().unwrap()
}

fn cli(state: &Path, args: &[&str]) -> Output {
    let claude = state.join("missing-claude-client");
    if args.contains(&"run") {
        let fixture = ProviderFixture::new(vec![serde_json::json!({"content":"Fixture response"})]);
        command(state, &claude)
            .args(["--base-url", fixture.endpoint()])
            .env("EJM_CLI_API_KEY", "local-fixture-only")
            .args(args)
            .output()
            .unwrap()
    } else {
        cli_with_claude(state, args, &claude)
    }
}

fn claude_fixture(state: &Path, mode: &str) -> std::path::PathBuf {
    static NATIVE_FIXTURE: OnceLock<std::path::PathBuf> = OnceLock::new();
    let source = NATIVE_FIXTURE.get_or_init(|| {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let status = Command::new(env!("CARGO"))
            .current_dir(manifest)
            .args([
                "build",
                "--locked",
                "--example",
                "claude-fixture",
                "--jobs",
                "2",
            ])
            .status()
            .unwrap();
        assert!(status.success(), "native Claude fixture build failed");
        Path::new(env!("CARGO_BIN_EXE_ejm-cli"))
            .parent()
            .unwrap()
            .join("examples")
            .join(format!("claude-fixture{}", std::env::consts::EXE_SUFFIX))
    });
    let directory = state.join("claude-client");
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(directory.join("mode"), mode).unwrap();
    let executable = directory.join(format!("Claude fixture ü{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(source, &executable).unwrap();
    executable
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
fn configured_provider_streams_and_resume_outputs_only_the_new_turn() {
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
fn sessions_lists_every_page_as_text_and_one_json_array_and_reports_output_errors() {
    let state = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let expected = runtime.block_on(async {
        let harness = ejm_harness::Harness::with_optional_provider(
            ejm_harness::HarnessConfig {
                state_dir: state.path().into(),
                ..ejm_harness::HarnessConfig::default()
            },
            None,
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let mut ids = std::collections::HashSet::new();
        for index in 0..125 {
            let session = harness
                .create_session(ejm_harness::SessionOptions {
                    cwd: state.path().into(),
                    title: format!("Saved {index}\nsecond line\u{1b}[31m"),
                })
                .await
                .unwrap();
            ids.insert(session.id);
        }
        harness.shutdown().await.unwrap();
        ids
    });

    let json = cli(state.path(), &["sessions", "--json"]);
    assert!(json.status.success(), "{json:?}");
    let sessions: Vec<ejm_harness::Session> = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(sessions.len(), 125);
    let ids = sessions
        .into_iter()
        .map(|session| session.id)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids, expected);

    let output = cli(state.path(), &["sessions"]);
    assert!(output.status.success(), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(!text.contains('\u{1b}'));
    let rows = text.lines().skip(1).collect::<Vec<_>>();
    assert_eq!(rows.len(), 125);
    let ids = rows
        .iter()
        .map(|line| line.split_whitespace().next().unwrap().to_owned())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids, expected);

    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let failed = command(state.path(), &state.path().join("missing-claude-client"))
        .args(["sessions", "--json"])
        .stdout(writer)
        .output()
        .unwrap();
    assert!(!failed.status.success(), "{failed:?}");
    assert_ne!(failed.stderr, [] as [u8; 0]);
}

#[test]
fn sessions_reports_invalid_storage_in_text_and_json_modes() {
    let state = tempfile::tempdir().unwrap();
    let invalid = state.path().join("regular-file");
    std::fs::write(&invalid, "preserve").unwrap();
    for args in [&["sessions"][..], &["sessions", "--json"][..]] {
        let output = cli(&invalid, args);
        assert!(!output.status.success());
        assert_ne!(output.stderr, [] as [u8; 0]);
        assert_eq!(output.stdout, [] as [u8; 0]);
    }
    assert_eq!(std::fs::read_to_string(invalid).unwrap(), "preserve");
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
    let marker = state.path().join("must-not-exist");
    let fixture = ProviderFixture::new(vec![
        serde_json::json!(
            { "content": "", "tool_calls": [{
                "id": "denied", "name": "shell",
                "arguments": { "command": format!("touch {}", marker.display()) }
            }] }),
        serde_json::json!({ "content": "Done" }),
    ]);
    let output = command(
        &state.path().join("sessions"),
        &state.path().join("missing-claude-client"),
    )
    .args(["--base-url", fixture.endpoint()])
    .env("EJM_CLI_API_KEY", "local-fixture-only")
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
fn removed_demo_flags_fail_without_creating_state() {
    let state = tempfile::tempdir().unwrap();
    for args in [vec!["--replay"], vec!["--replay-file", "responses.json"]] {
        let sessions = state.path().join("must-not-exist");
        let output = command(&sessions, &state.path().join("missing-client"))
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(!sessions.exists());
        assert_eq!(output.stdout, [] as [u8; 0]);
    }
}

#[test]
fn unavailable_claude_preserves_account_listing_and_anthropic_guidance() {
    let state = tempfile::tempdir().unwrap();
    let output = cli(state.path(), &["accounts", "--json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let accounts: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(accounts.len(), 1, "no native account should be invented");
    assert_eq!(accounts[0]["provider"], "claude");
    assert_eq!(accounts[0]["state"], "unavailable");
    assert!(
        !state.path().join("auth").exists(),
        "read-only listing created credential state"
    );
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
fn unavailable_provider_vault_still_lists_safe_claude_status() {
    let state = tempfile::tempdir().unwrap();
    let executable = claude_fixture(state.path(), "success");
    std::fs::write(state.path().join("auth"), "PRIVATE_VAULT_BLOCKER").unwrap();
    let output = cli_with_claude(state.path(), &["accounts", "--json"], &executable);
    assert!(output.status.success());
    let entries: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry["provider"] == "chatgpt" && entry["state"] == "unavailable")
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry["provider"] == "claude" && entry["state"] == "available")
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("PRIVATE_VAULT_BLOCKER"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("PRIVATE_VAULT_BLOCKER"));
}

#[test]
fn claude_shell_login_and_logout_use_safe_official_account_status() {
    let state = tempfile::tempdir().unwrap();
    let executable = claude_fixture(state.path(), "success");
    let run = |args: &[&str]| cli_with_claude(state.path(), args, &executable);
    let output = run(&["accounts", "--json"]);
    assert!(output.status.success());
    let accounts: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(accounts[0]["loggedIn"], false);

    let output = run(&["login", "claude"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = run(&["accounts", "--json"]);
    assert!(output.status.success());
    let accounts: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0]["provider"], "claude");
    assert_eq!(accounts[0]["state"], "available");
    assert_eq!(accounts[0]["loggedIn"], true);
    assert_eq!(accounts[0]["authMethod"], "claude.ai");
    assert_eq!(accounts[0]["email"], "terminal-fixture@example.test");
    assert!(accounts[0].get("accessToken").is_none());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("CLAUDE_FIXTURE_PRIVATE_STATUS"));

    let output = run(&["logout", "claude"]);
    assert!(output.status.success());
    let output = run(&["accounts", "--json"]);
    let accounts: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(accounts[0]["loggedIn"], false);
    let invocations =
        std::fs::read_to_string(state.path().join("claude-client/invocations.jsonl")).unwrap();
    let invocations: Vec<Vec<String>> = invocations
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(
        invocations
            .iter()
            .any(|args| args == &["auth", "login", "--claudeai"])
    );
    assert!(invocations.iter().any(|args| args == &["auth", "logout"]));
}

#[test]
fn claude_shell_login_rejects_failure_and_non_subscription_status() {
    for (mode, source) in [("failure", None), ("success", Some("api_key"))] {
        let state = tempfile::tempdir().unwrap();
        let executable = claude_fixture(state.path(), mode);
        if let Some(source) = source {
            std::fs::write(
                state.path().join("claude-client/status.json"),
                serde_json::to_vec(&serde_json::json!({
                    "loggedIn": true,
                    "authMethod": source,
                    "accessToken": "CLAUDE_FIXTURE_PRIVATE_STATUS"
                }))
                .unwrap(),
            )
            .unwrap();
            std::fs::write(state.path().join("claude-client/status-exit"), "0").unwrap();
        }
        let output = cli_with_claude(state.path(), &["login", "claude"], &executable);
        assert!(!output.status.success());
        assert!(!String::from_utf8_lossy(&output.stdout).contains("CLAUDE_FIXTURE_PRIVATE_STATUS"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("CLAUDE_FIXTURE_PRIVATE_STATUS"));
    }
}

#[test]
fn malformed_claude_status_remains_unavailable_without_exposing_client_output() {
    let state = tempfile::tempdir().unwrap();
    let executable = claude_fixture(state.path(), "success");
    std::fs::write(
        state.path().join("claude-client/status.json"),
        "CLAUDE_FIXTURE_PRIVATE_STATUS",
    )
    .unwrap();
    std::fs::write(state.path().join("claude-client/status-exit"), "0").unwrap();
    let output = cli_with_claude(state.path(), &["accounts", "--json"], &executable);
    assert!(output.status.success());
    let accounts: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(accounts[0]["state"], "unavailable");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("CLAUDE_FIXTURE_PRIVATE_STATUS"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("CLAUDE_FIXTURE_PRIVATE_STATUS"));
}

#[test]
fn claude_api_options_fail_before_starting_the_official_client() {
    for option in [
        ["--base-url", "https://other.test/v1"],
        ["--account", "other-registration"],
        ["--api-key-env", "OTHER_API_KEY"],
    ] {
        let state = tempfile::tempdir().unwrap();
        let executable = claude_fixture(state.path(), "success");
        let output = command(state.path(), &executable)
            .args(["--provider", "claude"])
            .args(option)
            .args(["run", "must not execute"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            !state
                .path()
                .join("claude-client/invocations.jsonl")
                .exists()
        );
    }
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
fn relay_accounts_and_rejected_connect_do_not_initialize_provider_or_journal() {
    let state = tempfile::tempdir().unwrap();
    let output = command(state.path(), &state.path().join("missing-client"))
        .args(["--provider", "claude", "remote", "accounts"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(std::fs::read_dir(state.path()).unwrap().next().is_none());
    let output = command(state.path(), &state.path().join("missing-client"))
        .args([
            "--relay",
            "https://relay.example",
            "--provider",
            "claude",
            "remote",
            "connect",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("interactive terminal"));
    assert!(std::fs::read_dir(state.path()).unwrap().next().is_none());
}
