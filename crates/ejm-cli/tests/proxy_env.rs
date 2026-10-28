#![expect(
    clippy::unwrap_used,
    reason = "An invalid deterministic fixture must fail the integration test immediately."
)]

use std::{
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    process::{Command, Output},
    sync::mpsc,
    thread,
    time::Duration,
};

const KEY: &str = "ejm-cli-proxy-key-9f3c";
const PROXY_SECRET: &str = "proxy-secret-value";

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

fn bind() -> (TcpListener, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

fn proxy_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/{PROXY_SECRET}")
}

fn assert_secret_free(output: &Output) {
    let combined = format!("{}{}", text(&output.stdout), text(&output.stderr));
    assert!(!combined.contains(KEY), "API key was printed");
    assert!(!combined.contains(PROXY_SECRET), "proxy value was printed");
}

fn assert_proxy_idle(listener: &TcpListener) {
    let accepted = listener.accept();
    assert!(
        matches!(accepted, Err(ref error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "plain http loopback contacted the proxy"
    );
}

/// Accept one request and answer immediately so `run` cannot sit on the client timeout.
fn expect_connection() -> (u16, mpsc::Receiver<()>) {
    let (listener, port) = bind();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .unwrap();
        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer);
        let _ = stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        let _ = sender.send(());
    });
    (port, receiver)
}

#[test]
fn unset_proxy_allows_http_loopback() {
    let state = tempfile::tempdir().unwrap();
    let output = provider_cli(state.path())
        .args(["doctor", "--base-url", "http://127.0.0.1:9/v1"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(text(&output.stdout).contains("Endpoint: http://127.0.0.1:9/v1\n"));
}

#[test]
fn set_proxy_refuses_before_the_key_and_does_not_connect() {
    for variable in [
        "HTTP_PROXY",
        "http_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        let state = tempfile::tempdir().unwrap();
        let (listener, port) = bind();
        listener.set_nonblocking(true).unwrap();
        let output = provider_cli(state.path())
            .args(["run", "--base-url", "http://127.0.0.1:9/v1", "hello"])
            .env(variable, proxy_url(port))
            .env("EJM_CLI_API_KEY", KEY)
            .output()
            .unwrap();
        let stderr = text(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{variable}: {stderr}");
        assert!(stderr.contains(variable), "{variable}: {stderr}");
        assert!(stderr.contains("NO_PROXY"), "{stderr}");
        assert!(!stderr.contains("set EJM_CLI_API_KEY"), "{stderr}");
        assert!(!stderr.contains("provider connection failed"), "{stderr}");
        assert_secret_free(&output);
        assert_proxy_idle(&listener);
    }
}

#[test]
fn no_proxy_exact_ip_reaches_loopback_and_skips_the_proxy() {
    let state = tempfile::tempdir().unwrap();
    let (target_port, connected) = expect_connection();
    let (proxy, proxy_port) = bind();
    proxy.set_nonblocking(true).unwrap();
    let base = format!("http://127.0.0.1:{target_port}/v1");
    let output = provider_cli(state.path())
        .args(["run", "--base-url", &base, "hello"])
        .env("HTTP_PROXY", proxy_url(proxy_port))
        .env("NO_PROXY", "127.0.0.1")
        .env("EJM_CLI_API_KEY", KEY)
        .output()
        .unwrap();
    assert!(
        connected.recv_timeout(Duration::from_secs(5)).is_ok(),
        "loopback target received no connection"
    );
    assert_proxy_idle(&proxy);
    assert_secret_free(&output);
    assert!(!text(&output.stderr).contains("while HTTP_PROXY is set"));
}

#[test]
fn no_proxy_star_is_not_coverage() {
    let state = tempfile::tempdir().unwrap();
    let (proxy, port) = bind();
    proxy.set_nonblocking(true).unwrap();
    let output = provider_cli(state.path())
        .args(["run", "--base-url", "http://localhost:9/v1", "hello"])
        .env("ALL_PROXY", proxy_url(port))
        .env("NO_PROXY", "*")
        .env("EJM_CLI_API_KEY", KEY)
        .output()
        .unwrap();
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("ALL_PROXY"), "{stderr}");
    assert!(!stderr.contains("set EJM_CLI_API_KEY"), "{stderr}");
    assert!(!stderr.contains("provider connection failed"), "{stderr}");
    assert_secret_free(&output);
    assert_proxy_idle(&proxy);

    let (proxy, port) = bind();
    proxy.set_nonblocking(true).unwrap();
    let output = provider_cli(state.path())
        .args(["run", "--base-url", "http://127.0.0.1:9/v1", "hello"])
        .env("HTTP_PROXY", proxy_url(port))
        .env("NO_PROXY", "*")
        .env("EJM_CLI_API_KEY", KEY)
        .output()
        .unwrap();
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("HTTP_PROXY"), "{stderr}");
    assert_secret_free(&output);
    assert_proxy_idle(&proxy);
}

#[test]
fn bracketed_ipv6_no_proxy_refuses_before_the_key() {
    let state = tempfile::tempdir().unwrap();
    let (listener, port) = bind();
    listener.set_nonblocking(true).unwrap();
    let output = provider_cli(state.path())
        .args(["run", "--base-url", "http://[::1]:9/v1", "hello"])
        .env("HTTP_PROXY", proxy_url(port))
        .env("NO_PROXY", "[::1]")
        .env("EJM_CLI_API_KEY", KEY)
        .output()
        .unwrap();
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("HTTP_PROXY"), "{stderr}");
    assert!(!stderr.contains("set EJM_CLI_API_KEY"), "{stderr}");
    assert!(!stderr.contains("provider connection failed"), "{stderr}");
    assert_secret_free(&output);
    assert_proxy_idle(&listener);
}

#[test]
fn no_proxy_loopback_ip_covers_expanded_ipv6() {
    let state = tempfile::tempdir().unwrap();
    let output = provider_cli(state.path())
        .args(["doctor", "--base-url", "http://[0:0:0:0:0:0:0:1]/v1"])
        .env("http_proxy", "http://127.0.0.1:9/proxy-secret-value")
        .env("no_proxy", "::1")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_secret_free(&output);
    let output = provider_cli(state.path())
        .args(["doctor", "--base-url", "http://127.0.0.1:9/v1"])
        .env("HTTP_PROXY", "http://127.0.0.1:9/proxy-secret-value")
        .env("NO_PROXY", "localhost")
        .output()
        .unwrap();
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("HTTP_PROXY"), "{stderr}");
    assert_secret_free(&output);
}
