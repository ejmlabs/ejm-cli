#![expect(
    clippy::unwrap_used,
    reason = "An invalid deterministic fixture must fail the integration test immediately."
)]

use std::{
    path::Path,
    process::{Command, Output},
};

fn cli(state: &Path, args: &[&str]) -> Output {
    cli_env(state, args, &[])
}

fn cli_env(state: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ejm-cli"));
    command
        .arg("--state-dir")
        .arg(state)
        .arg("--replay")
        .args(args)
        .env_remove("EJM_CLI_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("EJM_CLI_REGISTRY_URL");
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().unwrap()
}

fn output_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
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

fn write_session(state: &Path, origin: &str, token: &str) -> std::path::PathBuf {
    let packages = state.join("packages");
    std::fs::create_dir_all(&packages).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&packages, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let path = packages.join("registry-session.json");
    std::fs::write(
        &path,
        serde_json::json!({"origin": origin, "token": token}).to_string(),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    path
}

fn quiet_listener(host: &str) -> std::net::TcpListener {
    let listener = std::net::TcpListener::bind((host, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    listener
}

fn assert_no_connection(listener: &std::net::TcpListener) {
    let idle = listener
        .accept()
        .is_err_and(|error| error.kind() == std::io::ErrorKind::WouldBlock);
    assert!(idle, "registry command opened a connection");
}

fn spawn_registry(status: u16, body: &'static str) -> (String, std::sync::mpsc::Receiver<String>) {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_http_request(&mut stream);
        let reason = if status == 200 { "OK" } else { "Error" };
        let response = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        std::io::Write::write_all(&mut stream, response.as_bytes()).unwrap();
        sender.send(request).unwrap();
    });
    (origin, receiver)
}

fn read_http_request(stream: &mut std::net::TcpStream) -> String {
    use std::io::Read;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();
    let mut data = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        if data.len() > 16_384 {
            break;
        }
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                data.extend_from_slice(&buf[..n]);
                if let Some(end) = data.windows(4).position(|window| window == b"\r\n\r\n") {
                    let end = end + 4;
                    let headers = String::from_utf8_lossy(&data[..end]);
                    let length = content_length(&headers);
                    if data.len() >= end + length {
                        break;
                    }
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&data).into_owned()
}

fn content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
        .min(8192)
}

fn hidden(output: &Output, secret: &str) {
    assert!(
        !output_text(&output.stdout).contains(secret)
            && !output_text(&output.stderr).contains(secret),
        "registry token appeared in command output"
    );
}

#[test]
fn registry_help_names_the_override_and_discard_command() {
    let state = tempfile::tempdir().unwrap();
    for command in [
        ["package", "--help"],
        ["doctor", "--help"],
        ["run", "--help"],
    ] {
        let output = cli(state.path(), &command);
        assert!(output.status.success(), "{}", output_text(&output.stderr));
        let text = output_text(&output.stdout);
        assert!(text.contains("--registry"));
        assert!(text.contains("EJM_CLI_REGISTRY_URL"));
        assert!(text.contains("Does not change model endpoints"));
    }
    let output = cli(state.path(), &["package", "--help"]);
    assert!(output_text(&output.stdout).contains("discard-retired-session"));
}

#[test]
fn doctor_reports_the_registry_choice_without_connecting() {
    let state = tempfile::tempdir().unwrap();
    let output = cli(state.path(), &["doctor"]);
    assert!(output.status.success(), "{}", output_text(&output.stderr));
    let text = output_text(&output.stdout);
    assert!(text.contains("Registry: https://registry.ejmlabs.com (default)"));
    assert!(!text.contains("EJM_CLI_REGISTRY_URL"));
    assert!(!text.contains("--registry"));

    let output = cli_env(state.path(), &["doctor"], &[("EJM_CLI_REGISTRY_URL", "")]);
    assert!(output.status.success());
    assert!(
        output_text(&output.stdout).contains("Registry: https://registry.ejmlabs.com (default)")
    );

    let output = cli_env(
        state.path(),
        &["doctor"],
        &[("EJM_CLI_REGISTRY_URL", "http://127.0.0.1:8787")],
    );
    assert!(output.status.success(), "{}", output_text(&output.stderr));
    assert!(
        output_text(&output.stdout)
            .contains("Registry: http://127.0.0.1:8787 (EJM_CLI_REGISTRY_URL)")
    );

    let output = cli_env(
        state.path(),
        &["doctor"],
        &[("EJM_CLI_REGISTRY_URL", "https://harness.meetgerry.ai")],
    );
    assert_eq!(output.status.code(), Some(1));
    let text = output_text(&output.stdout);
    assert!(text.contains("Registry: rejected"));
    assert!(text.contains("meetgerry.ai registry is retired"));
    assert!(text.contains("https://registry.ejmlabs.com"));
    assert!(text.contains("EJM_CLI_REGISTRY_URL"));
    assert!(text.contains("Grants:"));
    assert!(text.contains("not a security sandbox"));
    assert!(!output_text(&output.stderr).contains("ejm-cli:"));

    let secret = "doctor-retired-token";
    write_session(state.path(), "https://harness.meetgerry.ai/", secret);
    let output = cli(state.path(), &["doctor"]);
    assert!(output.status.success(), "{}", output_text(&output.stderr));
    let text = output_text(&output.stdout);
    assert!(text.contains("ejm-cli package discard-retired-session"));
    assert!(text.contains("not sent or reused"));
    hidden(&output, secret);
}

#[test]
fn package_commands_validate_registry_overrides_before_any_request() {
    let state = tempfile::tempdir().unwrap();
    let (origin, requests) = spawn_registry(200, r#"{"packages":[]}"#);
    let output = cli_env(
        state.path(),
        &["--registry", &origin, "package", "search", "hello"],
        &[("EJM_CLI_REGISTRY_URL", "https://harness.meetgerry.ai")],
    );
    assert!(output.status.success(), "{}", output_text(&output.stderr));
    let text = output_text(&output.stdout);
    assert!(text.contains(&format!("Registry: {origin} (--registry)")));
    assert!(text.contains("\"packages\""));
    let request = requests
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    assert!(request.contains("GET /v1/packages"));
    assert!(request.contains("hello"));

    let listener = quiet_listener("127.0.0.1");
    let output = cli(
        state.path(),
        &[
            "--registry",
            "http://registry.example.com",
            "package",
            "search",
            "hello",
        ],
    );
    assert!(!output.status.success());
    let error = output_text(&output.stderr);
    assert!(error.contains("https"));
    assert!(error.contains("--registry"));
    assert!(error.contains("EJM_CLI_REGISTRY_URL"));
    assert_no_connection(&listener);

    for raw in ["https://harness.meetgerry.ai", "https://other.meetgerry.ai"] {
        let output = cli_env(
            state.path(),
            &["package", "search", "hello"],
            &[("EJM_CLI_REGISTRY_URL", raw)],
        );
        assert!(!output.status.success());
        let error = output_text(&output.stderr);
        assert!(
            error.contains("meetgerry.ai registry is retired"),
            "{error}"
        );
        assert!(error.contains("https://registry.ejmlabs.com"), "{error}");
        assert!(error.contains("EJM_CLI_REGISTRY_URL"), "{error}");
    }

    let output = cli(
        state.path(),
        &[
            "--registry",
            "https://ejmlabs.com/registry",
            "package",
            "search",
            "hello",
        ],
    );
    assert!(!output.status.success());
    assert!(output_text(&output.stderr).contains("plain origin"));

    for raw in ["http://0.0.0.0:9", "http://192.168.1.10:9"] {
        let output = cli(
            state.path(),
            &["--registry", raw, "package", "search", "hello"],
        );
        assert!(!output.status.success());
        assert!(output_text(&output.stderr).contains("https"), "{raw}");
    }
}

#[test]
fn pinned_registry_client_still_rejects_other_loopback_addresses() {
    let state = tempfile::tempdir().unwrap();
    let listener = quiet_listener("127.0.0.2");
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let output = cli(
        state.path(),
        &["--registry", &origin, "package", "search", "hello"],
    );
    assert!(!output.status.success());
    assert!(output_text(&output.stdout).contains(&format!("Registry: {origin} (--registry)")));
    assert!(output_text(&output.stderr).contains("registry requires HTTPS"));
    assert_no_connection(&listener);
}

#[test]
fn logout_removes_only_the_session_for_the_origin_in_effect() {
    let state = tempfile::tempdir().unwrap();
    let (origin, requests) = spawn_registry(200, "{}");
    let secret = "logout-token-do-not-print";
    let path = write_session(state.path(), &format!("{origin}/"), secret);
    let output = cli(state.path(), &["--registry", &origin, "package", "logout"]);
    assert!(output.status.success(), "{}", output_text(&output.stderr));
    assert!(output_text(&output.stdout).contains("Registry session revoked and removed."));
    hidden(&output, secret);
    assert!(!path.exists());
    let request = requests
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    assert!(request.contains("POST /api/auth/sign-out"));
    assert!(
        request
            .to_ascii_lowercase()
            .contains(&format!("bearer {secret}"))
    );

    let output = cli(state.path(), &["--registry", &origin, "package", "logout"]);
    assert!(output.status.success(), "{}", output_text(&output.stderr));
    assert!(output_text(&output.stdout).contains(&format!("No registry session for {origin}.")));

    let (origin, requests) = spawn_registry(500, "{}");
    let secret = "logout-failed-token";
    let path = write_session(state.path(), &format!("{origin}/"), secret);
    let output = cli(state.path(), &["--registry", &origin, "package", "logout"]);
    assert!(!output.status.success());
    assert!(
        output_text(&output.stderr).contains("local session removed; remote revocation failed")
    );
    hidden(&output, secret);
    assert!(!path.exists());
    let request = requests
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    assert!(request.contains("POST /api/auth/sign-out"));

    let listener = quiet_listener("127.0.0.1");
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let secret = "other-origin-token";
    let path = write_session(state.path(), "https://registry.ejmlabs.com/", secret);
    let output = cli(state.path(), &["--registry", &origin, "package", "logout"]);
    assert!(output.status.success(), "{}", output_text(&output.stderr));
    assert!(output_text(&output.stdout).contains(&format!("No registry session for {origin}.")));
    hidden(&output, secret);
    assert!(path.exists());
    assert_no_connection(&listener);

    let output = cli(state.path(), &["--registry", &origin, "package", "whoami"]);
    assert!(!output.status.success());
    assert!(output_text(&output.stderr).contains(&format!("no registry session for {origin}")));
    hidden(&output, secret);
    assert_no_connection(&listener);
}

#[test]
fn discard_retired_session_is_local_and_login_stays_blocked_until_then() {
    let state = tempfile::tempdir().unwrap();
    let listener = quiet_listener("127.0.0.1");
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let secret = "legacy-token-do-not-send";
    let path = write_session(state.path(), "https://harness.meetgerry.ai/", secret);

    let output = cli(
        state.path(),
        &["--registry", &origin, "package", "login", "--no-browser"],
    );
    assert!(!output.status.success());
    let error = output_text(&output.stderr);
    assert!(error.contains("the meetgerry.ai registry is retired"));
    assert!(error.contains("not sent or reused"));
    assert!(error.contains("ejm-cli package discard-retired-session"));
    hidden(&output, secret);
    assert!(path.exists());
    assert_no_connection(&listener);

    let output = cli_env(
        state.path(),
        &["package", "discard-retired-session"],
        &[("EJM_CLI_REGISTRY_URL", "https://other.meetgerry.ai")],
    );
    assert!(output.status.success(), "{}", output_text(&output.stderr));
    assert!(output_text(&output.stdout).contains("No request was sent."));
    hidden(&output, secret);
    assert!(!path.exists());

    let (origin, requests) = spawn_registry(200, "{}");
    let output = cli(
        state.path(),
        &["--registry", &origin, "package", "login", "--no-browser"],
    );
    assert!(!output.status.success());
    assert!(!output_text(&output.stderr).contains("was not sent"));
    let request = requests
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    assert!(request.contains("POST /api/auth/device/code"));
    assert!(!request.contains(secret));

    let kept = write_session(state.path(), "https://registry.ejmlabs.com/", secret);
    let output = cli(state.path(), &["package", "discard-retired-session"]);
    assert!(output.status.success(), "{}", output_text(&output.stderr));
    assert!(output_text(&output.stdout).contains("Left it in place."));
    hidden(&output, secret);
    assert!(kept.exists());

    std::fs::remove_file(&kept).unwrap();
    let output = cli(state.path(), &["package", "discard-retired-session"]);
    assert!(output.status.success());
    assert!(output_text(&output.stdout).contains("No leftover registry session file."));
}

#[test]
fn invalid_registry_environment_does_not_block_replay() {
    let state = tempfile::tempdir().unwrap();
    let output = cli_env(
        state.path(),
        &["run", "registry must not matter", "--json"],
        &[("EJM_CLI_REGISTRY_URL", "https://harness.meetgerry.ai")],
    );
    assert!(output.status.success(), "{}", output_text(&output.stderr));
    assert!(!output_text(&output.stderr).contains("meetgerry"));
    assert!(output_text(&output.stdout).contains("run_finished"));
}

#[test]
fn doctor_hides_a_registry_userinfo_secret() {
    let state = tempfile::tempdir().unwrap();
    let output = cli_env(
        state.path(),
        &["doctor"],
        &[(
            "EJM_CLI_REGISTRY_URL",
            "https://user:secret-registry-password@registry.example",
        )],
    );
    assert_eq!(output.status.code(), Some(1));
    let text = combined(&output);
    assert!(text.contains("https://registry.example"), "{text}");
    assert!(!text.contains("(invalid registry URL)"), "{text}");
    hidden(&output, "secret-registry-password");
    hidden(&output, "user:secret-registry-password");
    assert_eq!(registry_lines(&text), 1, "{text}");
}

#[test]
fn doctor_rejected_registry_value_cannot_forge_a_second_line() {
    let state = tempfile::tempdir().unwrap();
    let output = cli_env(
        state.path(),
        &["doctor"],
        &[(
            "EJM_CLI_REGISTRY_URL",
            "https://evil.example\nRegistry: https://registry.ejmlabs.com (default)",
        )],
    );
    assert_eq!(output.status.code(), Some(1));
    let text = combined(&output);
    assert_eq!(registry_lines(&text), 1, "{text}");
    assert!(
        !text
            .lines()
            .any(|line| line.trim() == "Registry: https://registry.ejmlabs.com (default)"),
        "{text}"
    );
    assert!(text.contains("(invalid registry URL)"), "{text}");
    assert!(!text.contains("evil.example"), "{text}");
}

#[test]
fn doctor_rejected_registry_value_drops_query_and_fragment() {
    let state = tempfile::tempdir().unwrap();
    let output = cli_env(
        state.path(),
        &["doctor"],
        &[(
            "EJM_CLI_REGISTRY_URL",
            "https://registry.example/index?access=query-secret#fragment-secret",
        )],
    );
    assert_eq!(output.status.code(), Some(1));
    let text = combined(&output);
    assert!(text.contains("https://registry.example/index"), "{text}");
    hidden(&output, "query-secret");
    hidden(&output, "fragment-secret");
    assert!(!text.contains('?'), "{text}");
    assert!(!text.contains('#'), "{text}");
    assert_eq!(registry_lines(&text), 1, "{text}");
}

#[test]
fn doctor_unparsed_registry_value_prints_a_placeholder() {
    let state = tempfile::tempdir().unwrap();
    let output = cli_env(
        state.path(),
        &["doctor"],
        &[(
            "EJM_CLI_REGISTRY_URL",
            "not-a-url sentinel-raw-registry-value",
        )],
    );
    assert_eq!(output.status.code(), Some(1));
    let text = combined(&output);
    assert!(text.contains("(invalid registry URL)"), "{text}");
    assert!(!text.contains("sentinel-raw-registry-value"), "{text}");
    assert_eq!(registry_lines(&text), 1, "{text}");
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        output_text(&output.stdout),
        output_text(&output.stderr)
    )
}

fn registry_lines(text: &str) -> usize {
    text.lines()
        .filter(|line| line.contains("Registry:"))
        .count()
}

#[cfg(unix)]
#[derive(Clone, Copy)]
enum SessionHazard {
    Symlink,
    Directory,
    Loose,
    Malformed,
    ExtraFields,
    MissingOrigin,
    Oversize,
}

#[cfg(unix)]
#[derive(Clone, Copy)]
enum SessionCommand {
    Discard,
    Login,
    Logout,
    Whoami,
}

#[cfg(unix)]
fn assert_session_hazard(hazard: SessionHazard, command: SessionCommand) {
    let state = tempfile::tempdir().unwrap();
    let packages = state.path().join("packages");
    std::fs::create_dir_all(&packages).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&packages, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let packages = packages.canonicalize().unwrap();
    let session = packages.join("registry-session.json");
    let secret = match hazard {
        SessionHazard::Symlink => "symlink-target-secret",
        SessionHazard::Directory => "directory-child-secret",
        SessionHazard::Loose => "loose-mode-secret",
        SessionHazard::Malformed => "malformed-json-secret",
        SessionHazard::ExtraFields => "extra-field-secret",
        SessionHazard::MissingOrigin => "missing-origin-secret",
        SessionHazard::Oversize => "oversize-session-secret",
    };
    let preserved = plant_session(&packages, &session, hazard, secret);
    let listener = quiet_listener("127.0.0.1");
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let output = run_session_command(state.path(), command, &origin);
    hidden(&output, secret);
    assert_no_connection(&listener);
    let text = combined(&output);
    let session_text = session.display().to_string();
    match (hazard, command) {
        (SessionHazard::Directory, _) => {
            assert!(!output.status.success(), "{text}");
            assert!(text.contains(&session_text), "{text}");
            assert!(text.contains("is a directory"), "{text}");
            assert!(text.contains("does not delete directories"), "{text}");
            assert!(std::fs::symlink_metadata(&session).unwrap().is_dir());
        }
        (_, SessionCommand::Discard) => {
            assert!(output.status.success(), "{text}");
            assert!(text.contains("No request was sent."), "{text}");
            assert!(
                std::fs::symlink_metadata(&session).is_err(),
                "legacy path still exists"
            );
        }
        _ => {
            assert!(!output.status.success(), "{text}");
            assert!(text.contains(&session_text), "{text}");
            assert!(
                text.contains("ejm-cli package discard-retired-session"),
                "{text}"
            );
            assert!(
                std::fs::symlink_metadata(&session).is_ok(),
                "legacy path was removed"
            );
        }
    }
    if let Some((path, bytes)) = preserved {
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}

#[cfg(unix)]
fn plant_session(
    packages: &Path,
    session: &Path,
    hazard: SessionHazard,
    secret: &str,
) -> Option<(std::path::PathBuf, Vec<u8>)> {
    use std::os::unix::fs::PermissionsExt;
    match hazard {
        SessionHazard::Symlink => {
            let outside = packages.parent().unwrap().join("outside-secret.txt");
            let bytes = secret.as_bytes().to_vec();
            std::fs::write(&outside, &bytes).unwrap();
            std::os::unix::fs::symlink(&outside, session).unwrap();
            assert!(!outside.starts_with(packages));
            Some((outside, bytes))
        }
        SessionHazard::Directory => {
            std::fs::create_dir(session).unwrap();
            let child = session.join("keep.txt");
            let bytes = secret.as_bytes().to_vec();
            std::fs::write(&child, &bytes).unwrap();
            Some((child, bytes))
        }
        SessionHazard::Loose => {
            std::fs::write(session, secret).unwrap();
            std::fs::set_permissions(session, std::fs::Permissions::from_mode(0o644)).unwrap();
            None
        }
        SessionHazard::Malformed => {
            write_private(session, &format!("{{not-json {secret}"));
            None
        }
        SessionHazard::ExtraFields => {
            write_private(
                session,
                &serde_json::json!({
                    "origin": "https://registry.ejmlabs.com/",
                    "token": secret,
                    "extra": 1
                })
                .to_string(),
            );
            None
        }
        SessionHazard::MissingOrigin => {
            write_private(session, &serde_json::json!({ "token": secret }).to_string());
            None
        }
        SessionHazard::Oversize => {
            let mut body = secret.as_bytes().to_vec();
            body.resize(8193, b'x');
            write_private(session, &String::from_utf8(body).unwrap());
            None
        }
    }
}

#[cfg(unix)]
fn write_private(path: &Path, contents: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, contents).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
}

#[cfg(unix)]
fn run_session_command(state: &Path, command: SessionCommand, origin: &str) -> Output {
    match command {
        SessionCommand::Discard => cli_env(
            state,
            &["package", "discard-retired-session"],
            &[("EJM_CLI_REGISTRY_URL", origin)],
        ),
        SessionCommand::Login => cli(
            state,
            &["--registry", origin, "package", "login", "--no-browser"],
        ),
        SessionCommand::Logout => cli(state, &["--registry", origin, "package", "logout"]),
        SessionCommand::Whoami => cli(state, &["--registry", origin, "package", "whoami"]),
    }
}

#[cfg(unix)]
#[test]
fn discard_retired_session_unlinks_a_symlink_without_reading_its_target() {
    assert_session_hazard(SessionHazard::Symlink, SessionCommand::Discard);
}

#[cfg(unix)]
#[test]
fn discard_retired_session_refuses_to_delete_a_directory() {
    assert_session_hazard(SessionHazard::Directory, SessionCommand::Discard);
}

#[cfg(unix)]
#[test]
fn discard_retired_session_unlinks_a_loose_session_file() {
    assert_session_hazard(SessionHazard::Loose, SessionCommand::Discard);
}

#[cfg(unix)]
#[test]
fn discard_retired_session_unlinks_malformed_json() {
    assert_session_hazard(SessionHazard::Malformed, SessionCommand::Discard);
    assert_session_hazard(SessionHazard::ExtraFields, SessionCommand::Discard);
}

#[cfg(unix)]
#[test]
fn discard_retired_session_unlinks_a_session_missing_an_origin() {
    assert_session_hazard(SessionHazard::MissingOrigin, SessionCommand::Discard);
}

#[cfg(unix)]
#[test]
fn discard_retired_session_unlinks_an_oversize_session_file() {
    assert_session_hazard(SessionHazard::Oversize, SessionCommand::Discard);
}

#[cfg(unix)]
#[test]
fn login_names_discard_for_a_symlink_session() {
    assert_session_hazard(SessionHazard::Symlink, SessionCommand::Login);
}

#[cfg(unix)]
#[test]
fn login_names_a_directory_session_path() {
    assert_session_hazard(SessionHazard::Directory, SessionCommand::Login);
}

#[cfg(unix)]
#[test]
fn login_names_discard_for_a_loose_session_file() {
    assert_session_hazard(SessionHazard::Loose, SessionCommand::Login);
}

#[cfg(unix)]
#[test]
fn login_names_discard_for_malformed_session_json() {
    assert_session_hazard(SessionHazard::Malformed, SessionCommand::Login);
}

#[cfg(unix)]
#[test]
fn login_names_discard_for_a_session_missing_an_origin() {
    assert_session_hazard(SessionHazard::MissingOrigin, SessionCommand::Login);
}

#[cfg(unix)]
#[test]
fn login_names_discard_for_an_oversize_session_file() {
    assert_session_hazard(SessionHazard::Oversize, SessionCommand::Login);
}

#[cfg(unix)]
#[test]
fn logout_names_discard_for_a_symlink_session() {
    assert_session_hazard(SessionHazard::Symlink, SessionCommand::Logout);
}

#[cfg(unix)]
#[test]
fn logout_names_a_directory_session_path() {
    assert_session_hazard(SessionHazard::Directory, SessionCommand::Logout);
}

#[cfg(unix)]
#[test]
fn logout_names_discard_for_a_loose_session_file() {
    assert_session_hazard(SessionHazard::Loose, SessionCommand::Logout);
}

#[cfg(unix)]
#[test]
fn logout_names_discard_for_malformed_session_json() {
    assert_session_hazard(SessionHazard::Malformed, SessionCommand::Logout);
}

#[cfg(unix)]
#[test]
fn logout_names_discard_for_a_session_missing_an_origin() {
    assert_session_hazard(SessionHazard::MissingOrigin, SessionCommand::Logout);
}

#[cfg(unix)]
#[test]
fn logout_names_discard_for_an_oversize_session_file() {
    assert_session_hazard(SessionHazard::Oversize, SessionCommand::Logout);
}

#[cfg(unix)]
#[test]
fn whoami_names_discard_for_a_symlink_session() {
    assert_session_hazard(SessionHazard::Symlink, SessionCommand::Whoami);
}

#[cfg(unix)]
#[test]
fn whoami_names_a_directory_session_path() {
    assert_session_hazard(SessionHazard::Directory, SessionCommand::Whoami);
}

#[cfg(unix)]
#[test]
fn whoami_names_discard_for_a_loose_session_file() {
    assert_session_hazard(SessionHazard::Loose, SessionCommand::Whoami);
}

#[cfg(unix)]
#[test]
fn whoami_names_discard_for_malformed_session_json() {
    assert_session_hazard(SessionHazard::Malformed, SessionCommand::Whoami);
}

#[cfg(unix)]
#[test]
fn whoami_names_discard_for_a_session_missing_an_origin() {
    assert_session_hazard(SessionHazard::MissingOrigin, SessionCommand::Whoami);
}

#[cfg(unix)]
#[test]
fn whoami_names_discard_for_an_oversize_session_file() {
    assert_session_hazard(SessionHazard::Oversize, SessionCommand::Whoami);
}
