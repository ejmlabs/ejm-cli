#![expect(
    clippy::unwrap_used,
    reason = "An invalid deterministic fixture must fail the integration test immediately."
)]

use std::{
    path::Path,
    process::{Command, Output},
};

fn cli(state: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ejm-cli"))
        .arg("--state-dir")
        .arg(state)
        .arg("--replay")
        .args(args)
        .env_remove("EJM_CLI_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .output()
        .unwrap()
}

/// A real provider command. This helper does not insert `--replay`.
fn provider_cli(state: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ejm-cli"));
    command
        .arg("--state-dir")
        .arg(state)
        .env_remove("EJM_CLI_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("EJM_CLI_BASE_URL")
        .env_remove("HTTP_PROXY")
        .env_remove("http_proxy")
        .env_remove("ALL_PROXY")
        .env_remove("all_proxy")
        .env_remove("HTTPS_PROXY")
        .env_remove("https_proxy")
        .env_remove("NO_PROXY")
        .env_remove("no_proxy");
    command
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn assert_cleartext_rejected(output: &Output, raw_url: &str) {
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("ejm-cli: provider base URL must use https"),
        "{stderr}"
    );
    assert!(stderr.contains("--base-url"), "{stderr}");
    assert!(stderr.contains("EJM_CLI_BASE_URL"), "{stderr}");
    assert!(!stderr.contains(raw_url), "{stderr}");
    assert!(!stderr.contains("secret"), "{stderr}");
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("fixed official endpoint"), "{stderr}");
        assert!(!stderr.contains("must use https"), "{stderr}");
        assert!(!state.path().join("auth").exists());
        let output = cli(
            state.path(),
            &[
                "--provider",
                provider,
                "--base-url",
                "http://api.example.invalid/v1",
                "models",
            ],
        );
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("fixed official endpoint"), "{stderr}");
        assert!(!stderr.contains("must use https"), "{stderr}");
        assert!(!state.path().join("auth").exists());
    }
}

#[test]
fn https_base_url_from_the_flag_is_the_usable_endpoint() {
    let state = tempfile::tempdir().unwrap();
    let output = provider_cli(state.path())
        .args(["doctor", "--base-url", "https://api.deepseek.com/v1"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(
        stdout.contains("Endpoint: https://api.deepseek.com/v1\n"),
        "{stdout}"
    );
    assert!(!stdout.contains("rejected"), "{stdout}");
}

#[test]
fn https_base_url_from_the_environment_is_the_usable_endpoint() {
    let state = tempfile::tempdir().unwrap();
    let output = provider_cli(state.path())
        .args(["doctor"])
        .env("EJM_CLI_BASE_URL", "https://api.deepseek.com/v1")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(
        stdout.contains("Endpoint: https://api.deepseek.com/v1\n"),
        "{stdout}"
    );
    assert!(!stdout.contains("rejected"), "{stdout}");
}

#[test]
fn http_remote_base_url_from_the_flag_exits_before_the_key() {
    let state = tempfile::tempdir().unwrap();
    let url = "http://api.example.invalid/v1";
    for provider in ["openai", "anthropic"] {
        for command in [
            vec!["--provider", provider, "--base-url", url, "run", "hello"],
            vec!["--provider", provider, "--base-url", url, "doctor"],
        ] {
            let output = provider_cli(state.path()).args(&command).output().unwrap();
            assert_cleartext_rejected(&output, url);
            if command.last() == Some(&"doctor") {
                let stdout = text(&output.stdout);
                assert!(
                    stdout.contains(&format!(
                        "Endpoint: {url} (rejected). HTTPS is required except for loopback hosts."
                    )),
                    "{stdout}"
                );
            }
        }
    }
}

#[test]
fn http_remote_base_url_from_the_environment_exits_before_the_key() {
    let state = tempfile::tempdir().unwrap();
    let url = "http://api.example.invalid/v1";
    for provider in ["openai", "anthropic"] {
        for command in [
            vec!["--provider", provider, "run", "hello"],
            vec!["--provider", provider, "doctor"],
        ] {
            let output = provider_cli(state.path())
                .args(&command)
                .env("EJM_CLI_BASE_URL", url)
                .output()
                .unwrap();
            assert_cleartext_rejected(&output, url);
        }
    }
}

#[test]
fn http_loopback_base_url_is_the_usable_endpoint() {
    let state = tempfile::tempdir().unwrap();
    let output = provider_cli(state.path())
        .args(["doctor", "--base-url", "http://127.0.0.1:9/v1"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(
        stdout.contains("Endpoint: http://127.0.0.1:9/v1\n"),
        "{stdout}"
    );
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.starts_with("Endpoint:"))
            .count(),
        1,
        "{stdout}"
    );
    assert!(!stdout.contains("rejected"), "{stdout}");
}

#[test]
fn lookalike_and_private_http_hosts_are_rejected() {
    let state = tempfile::tempdir().unwrap();
    for url in [
        "http://localhost.evil.invalid/v1",
        "http://127.0.0.1.nip.invalid/v1",
        "http://foo.localhost/v1",
        "http://192.168.1.10:11434/v1",
        "http://0.0.0.0/v1",
        "http://10.1.2.3/v1",
        "http://172.16.0.1/v1",
    ] {
        let output = provider_cli(state.path())
            .args(["run", "--base-url", url, "hello"])
            .output()
            .unwrap();
        assert_cleartext_rejected(&output, url);
    }
}

#[test]
fn other_schemes_are_invalid_base_urls() {
    let state = tempfile::tempdir().unwrap();
    for url in [
        "ftp://files.example.invalid/v1",
        "file:///tmp/a",
        "ws://api.example.invalid/v1",
        "api.example.invalid/v1",
    ] {
        let output = provider_cli(state.path())
            .args(["run", "--base-url", url, "hello"])
            .output()
            .unwrap();
        let stderr = text(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{url}: {stderr}");
        assert!(
            stderr.contains("invalid provider base URL"),
            "{url}: {stderr}"
        );
        assert!(!stderr.contains("must use https"), "{url}: {stderr}");
        assert!(!stderr.contains(url), "{url}: {stderr}");
    }
}

#[test]
fn insecure_url_with_a_key_present_does_not_print_the_key() {
    let state = tempfile::tempdir().unwrap();
    let key = "ejm-cli-test-key-9f3c";
    let url = "http://api.example.invalid/v1";
    let output = provider_cli(state.path())
        .args(["run", "hello"])
        .env("EJM_CLI_API_KEY", key)
        .env("EJM_CLI_BASE_URL", url)
        .output()
        .unwrap();
    assert_cleartext_rejected(&output, url);
    let combined = format!("{}{}", text(&output.stdout), text(&output.stderr));
    assert!(!combined.contains(key), "{combined}");
    assert!(
        !combined.contains("provider connection failed"),
        "{combined}"
    );
}

#[test]
fn insecure_url_with_no_key_reports_the_base_url() {
    let state = tempfile::tempdir().unwrap();
    let output = provider_cli(state.path())
        .args([
            "run",
            "--base-url",
            "http://api.example.invalid/v1",
            "hello",
        ])
        .output()
        .unwrap();
    assert_cleartext_rejected(&output, "http://api.example.invalid/v1");
    assert!(!text(&output.stderr).contains("set EJM_CLI_API_KEY"));
}

#[test]
fn doctor_marks_an_insecure_endpoint_rejected() {
    let state = tempfile::tempdir().unwrap();
    let url = "http://api.example.invalid/v1";
    let output = provider_cli(state.path())
        .args(["doctor"])
        .env("EJM_CLI_BASE_URL", url)
        .output()
        .unwrap();
    assert_cleartext_rejected(&output, url);
    let stdout = text(&output.stdout);
    assert!(
        stdout.contains(&format!(
            "Endpoint: {url} (rejected). HTTPS is required except for loopback hosts."
        )),
        "{stdout}"
    );
    assert!(!stdout.contains("ejm-cli-test-key"), "{stdout}");
    let output = provider_cli(state.path())
        .args([
            "doctor",
            "--base-url",
            "http://user:secret@api.example.invalid/v1",
        ])
        .output()
        .unwrap();
    let stderr = text(&output.stderr);
    let stdout = text(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("invalid provider base URL"), "{stderr}");
    assert!(!stderr.contains("secret"), "{stderr}");
    assert!(!stdout.contains("secret"), "{stdout}");
    assert!(!stdout.contains("Endpoint:"), "{stdout}");
    let injected = "http://localhost/v1\nProvider credentials: configured (value hidden)";
    let output = provider_cli(state.path())
        .args(["doctor", "--base-url", injected])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.starts_with("Endpoint:"))
            .count(),
        1,
        "{stdout}"
    );
    assert!(
        !stdout
            .lines()
            .any(|line| line == "Provider credentials: configured (value hidden)"),
        "{stdout}"
    );
}

#[test]
fn anthropic_without_an_override_uses_the_https_default() {
    let state = tempfile::tempdir().unwrap();
    let output = provider_cli(state.path())
        .args(["doctor", "--provider", "anthropic"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(
        stdout.contains("Endpoint: https://api.anthropic.com/v1\n"),
        "{stdout}"
    );
    assert!(!stdout.contains("rejected"), "{stdout}");
}

#[test]
fn replay_and_sessions_ignore_an_unused_cleartext_base_url() {
    let state = tempfile::tempdir().unwrap();
    let output = provider_cli(state.path())
        .args([
            "--replay",
            "--base-url",
            "http://api.example.invalid/v1",
            "run",
            "hello",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(text(&output.stdout).contains("hello"));
    let sessions = tempfile::tempdir().unwrap();
    let output = provider_cli(sessions.path())
        .args(["sessions"])
        .env("EJM_CLI_BASE_URL", "http://api.example.invalid/v1")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(text(&output.stdout).contains("No sessions yet"));
    assert!(!text(&output.stderr).contains("must use https"));
    let output = provider_cli(state.path())
        .args(["--replay", "doctor"])
        .env("EJM_CLI_BASE_URL", "http://api.example.invalid/v1")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(
        stdout.contains("Endpoint: (not used: offline replay)"),
        "{stdout}"
    );
    assert!(!text(&output.stderr).contains("must use https"));
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
