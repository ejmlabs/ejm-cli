use std::{convert::Infallible, path::Path, thread::JoinHandle, time::Duration};

use http_body_util::Full;
use hyper::{Response, body::Bytes, service::service_fn};
use serde_json::json;

use super::{cli, command};

#[test]
fn package_registry_commands_require_selection_before_credentials_or_network() {
    let state = tempfile::tempdir().unwrap();
    for args in [
        vec!["search", "query"],
        vec!["info", "@fixture/package", "1.0.0"],
        vec!["install", "@fixture/package@1.0.0"],
        vec!["update", "@fixture/package", "1.0.1"],
        vec!["login", "--no-browser"],
        vec!["logout"],
        vec!["migrate-credentials", "--old-clients-stopped"],
        vec!["whoami"],
        vec!["publisher", "fixture", "--display-name", "Fixture"],
        vec!["publish"],
    ] {
        let output = cli(state.path(), &[vec!["package"], args].concat());
        assert!(!output.status.success(), "{output:?}");
        let diagnostic = String::from_utf8(output.stderr).unwrap();
        assert!(diagnostic.contains("EJM_REGISTRY_URL"), "{diagnostic}");
        assert!(diagnostic.contains("--registry"), "{diagnostic}");
    }
    assert!(!state.path().join("packages").exists());
}

#[test]
fn package_registry_rejects_unsafe_origins_before_credentials() {
    let state = tempfile::tempdir().unwrap();
    for (origin, expected) in [
        ("http://registry.example", "registry requires HTTPS"),
        (
            "https://user:fixture-secret@registry.example",
            "registry must be a plain origin without credentials",
        ),
    ] {
        let output = cli(state.path(), &["package", "whoami", "--registry", origin]);
        assert!(!output.status.success(), "{output:?}");
        let diagnostic = String::from_utf8(output.stderr).unwrap();
        assert!(diagnostic.contains(expected), "{diagnostic}");
        assert!(!diagnostic.contains("fixture-secret"), "{diagnostic}");
    }
    assert!(!state.path().join("packages").exists());
}

fn registry_server() -> (String, JoinHandle<()>) {
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                let stream = tokio::time::timeout(Duration::from_secs(5), listener.accept())
                    .await
                    .unwrap()
                    .unwrap()
                    .0;
                let service = service_fn(
                    |request: hyper::Request<hyper::body::Incoming>| async move {
                        assert_eq!(request.uri(), "/v1/packages?q=query&limit=20");
                        assert!(request.headers().get("authorization").is_none());
                        Ok::<_, Infallible>(
                            Response::builder()
                                .header("content-type", "application/json")
                                .body(Full::new(Bytes::from("{\"packages\":[]}")))
                                .unwrap(),
                        )
                    },
                );
                tokio::time::timeout(
                    Duration::from_secs(5),
                    hyper::server::conn::http1::Builder::new()
                        .serve_connection(hyper_util::rt::TokioIo::new(stream), service),
                )
                .await
                .unwrap()
                .unwrap();
            });
    });
    (origin, server)
}

#[test]
fn package_registry_flag_environment_and_flag_precedence_select_only_the_chosen_origin() {
    let state = tempfile::tempdir().unwrap();
    for mode in 0..3 {
        let (origin, server) = registry_server();
        let mut cli = command(state.path());
        cli.arg("package");
        match mode {
            0 => {
                cli.args(["--registry", &origin, "search", "query"]);
            }
            1 => {
                cli.env("EJM_REGISTRY_URL", &origin)
                    .args(["search", "query"]);
            }
            _ => {
                cli.env("EJM_REGISTRY_URL", "http://unselected.example")
                    .args(["search", "query", "--registry", &origin]);
            }
        }
        let output = cli.output().unwrap();
        server.join().unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
            json!({"packages": []})
        );
    }
}

fn local_package(source: &Path) {
    std::fs::create_dir(source).unwrap();
    std::fs::write(
        source.join(ejm_harness_package_spec::MANIFEST_FILE),
        serde_json::to_vec(&json!({
            "schema_version": 1,
            "name": "@fixture/local",
            "version": "1.0.0",
            "description": "Local package fixture",
            "exports": {}
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn package_local_pack_install_and_trust_work_without_a_registry() {
    let state = tempfile::tempdir().unwrap();
    let source = state.path().join("source");
    local_package(&source);
    let archive = state.path().join("package.tgz");
    let output = command(state.path())
        .args(["package", "pack"])
        .arg(&source)
        .arg("--output")
        .arg(&archive)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    for path in [&source, &archive] {
        let output = command(state.path())
            .args(["package", "install"])
            .arg(path)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
    }
    let output = cli(state.path(), &["package", "list"]);
    assert!(output.status.success(), "{output:?}");
    let installed: Vec<ejm_harness_packages::PackageRecord> =
        serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].name, "@fixture/local");
    assert!(!installed[0].enabled);
    for args in [
        vec!["package", "enable", "@fixture/local", "--trust"],
        vec!["package", "disable", "@fixture/local"],
        vec!["package", "remove", "@fixture/local"],
    ] {
        let output = cli(state.path(), &args);
        assert!(output.status.success(), "{output:?}");
    }
    let output = cli(state.path(), &["package", "list"]);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        json!([])
    );
}
