use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use ejm_harness_packages::{PackageGrants, PackageManager, pack_directory};
use rustix::process::{Pid, Signal, kill_process, test_kill_process};
use serde_json::json;

use super::{ProviderFixture, claude_fixture, command};

const BOUND: Duration = Duration::from_secs(20);

struct StartupCli {
    child: Option<Child>,
    markers: Vec<PathBuf>,
}

impl StartupCli {
    fn spawn(command: &mut Command, markers: Vec<PathBuf>) -> Self {
        Self {
            child: Some(
                command
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap(),
            ),
            markers,
        }
    }

    fn signal(&self, signal: Signal) {
        let pid = Pid::from_raw(i32::try_from(self.child.as_ref().unwrap().id()).unwrap()).unwrap();
        // Only signal the CLI. Its cleanup must stop each interpreter itself.
        kill_process(pid, signal).unwrap();
    }

    fn wait(&mut self) -> Output {
        let deadline = Instant::now() + BOUND;
        while self.child.as_mut().unwrap().try_wait().unwrap().is_none() {
            assert!(Instant::now() < deadline, "CLI startup cleanup timed out");
            thread::sleep(Duration::from_millis(10));
        }
        self.child.take().unwrap().wait_with_output().unwrap()
    }

    fn wait_marker(&mut self, marker: &Path) -> Pid {
        let deadline = Instant::now() + BOUND;
        loop {
            if let Ok(raw) = fs::read_to_string(marker)
                && let Ok(raw) = raw.parse::<i32>()
            {
                return Pid::from_raw(raw).unwrap();
            }
            assert!(
                self.child.as_mut().unwrap().try_wait().unwrap().is_none(),
                "CLI exited before {}: {}",
                marker.display(),
                String::from_utf8_lossy(&self.wait().stderr)
            );
            assert!(
                Instant::now() < deadline,
                "startup marker missing: {}",
                marker.display()
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn assert_reaped(&mut self) {
        for marker in &self.markers {
            let raw = fs::read_to_string(marker).unwrap().parse::<i32>().unwrap();
            let pid = Pid::from_raw(raw).unwrap();
            assert_eq!(
                test_kill_process(pid),
                Err(rustix::io::Errno::SRCH),
                "startup child {pid} outlived CLI cleanup"
            );
        }
        self.markers.clear();
    }
}

impl Drop for StartupCli {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        for marker in &self.markers {
            if let Ok(raw) = fs::read_to_string(marker)
                && let Ok(raw) = raw.parse::<i32>()
                && let Some(pid) = Pid::from_raw(raw)
            {
                let _ = kill_process(pid, Signal::KILL);
            }
        }
    }
}

fn fixture_root() -> tempfile::TempDir {
    // Protected capacity roots must use their physical path on macOS.
    #[cfg(target_os = "macos")]
    let root = Path::new("/private/tmp");
    #[cfg(not(target_os = "macos"))]
    let root = Path::new("/tmp");
    tempfile::Builder::new()
        .prefix("ejm-cli-startup-")
        .tempdir_in(root)
        .unwrap()
}

#[derive(Clone, Copy)]
enum Language {
    Node,
    Python,
}

fn package(state: &Path, language: Language, blocked: bool, fail: bool) -> PathBuf {
    let name = match language {
        Language::Node => "node",
        Language::Python => "python",
    };
    let source = state.join(format!("source-{name}"));
    fs::create_dir(&source).unwrap();
    let marker = state.join(format!("{name}-pid"));
    let marker_literal = serde_json::to_string(&marker).unwrap();
    let (entry, body) = match language {
        Language::Node => (
            "extension.mjs",
            format!(
                "import {{ writeFileSync }} from 'node:fs';\n\
                 writeFileSync({marker_literal}, String(process.pid));\n\
                 {}{}export function hello() {{ return 'hello'; }}\n",
                if blocked {
                    "while (true) await new Promise(resolve => setTimeout(resolve, 10));\n"
                } else {
                    ""
                },
                if fail {
                    "throw new Error('startup import fixture rejected');\n"
                } else {
                    ""
                },
            ),
        ),
        Language::Python => (
            "extension.py",
            format!(
                "import os, time\nfrom pathlib import Path\n\
                 Path({marker_literal}).write_text(str(os.getpid()))\n\
                 {}{}def hello(args, ctx):\n    return 'hello'\n",
                if blocked {
                    "while True:\n    time.sleep(0.01)\n"
                } else {
                    ""
                },
                if fail {
                    "raise RuntimeError('startup import fixture rejected')\n"
                } else {
                    ""
                },
            ),
        ),
    };
    fs::write(source.join(entry), body).unwrap();
    fs::write(
        source.join(ejm_harness_package_spec::MANIFEST_FILE),
        serde_json::to_vec(&json!({
            "schema_version": 1,
            "name": format!("@fixture/{name}"),
            "version": "1.0.0",
            "description": "CLI startup lifecycle fixture",
            "runtime": {"kind": name, "entry": entry},
            "exports": {"commands": [{"name": "hello", "description": "Hello", "handler": "hello"}]}
        }))
        .unwrap(),
    )
    .unwrap();
    let manager = PackageManager::new(state.join("packages")).unwrap();
    manager
        .install(&pack_directory(&source).unwrap(), "local:startup-fixture")
        .unwrap();
    manager
        .enable(
            &format!("@fixture/{name}"),
            PackageGrants {
                commands: vec!["hello".into()],
                ..PackageGrants::default()
            },
        )
        .unwrap();
    marker
}

fn run_command(state: &Path, endpoint: &str) -> Command {
    let mut cli = command(state, &state.join("missing-claude-client"));
    cli.args(["--capacity", "fixed", "--base-url", endpoint])
        .env("EJM_CLI_API_KEY", "local-fixture-only");
    cli
}

#[test]
fn startup_signals_wait_for_package_interpreters() {
    let provider = ProviderFixture::new(vec![]);
    for language in [Language::Node, Language::Python] {
        for signal in [Signal::INT, Signal::TERM] {
            let state = fixture_root();
            let marker = package(state.path(), language, true, false);
            let mut cli = StartupCli::spawn(
                run_command(state.path(), provider.endpoint()).args(["run", "hello"]),
                vec![marker.clone()],
            );
            assert!(test_kill_process(cli.wait_marker(&marker)).is_ok());
            cli.signal(signal);
            let output = cli.wait();
            assert!(!output.status.success());
            let diagnostic = String::from_utf8_lossy(&output.stderr);
            assert!(diagnostic.contains("startup cancelled"), "{diagnostic}");
            cli.assert_reaped();
        }
    }
}

#[test]
fn startup_failures_reap_all_loaded_package_hosts() {
    #[derive(Clone, Copy, Debug)]
    enum Failure {
        Import,
        Theme,
        Provider,
    }

    let provider = ProviderFixture::new(vec![]);
    for failure in [Failure::Import, Failure::Theme, Failure::Provider] {
        let state = fixture_root();
        let markers = vec![
            package(state.path(), Language::Node, false, false),
            package(
                state.path(),
                Language::Python,
                false,
                matches!(failure, Failure::Import),
            ),
        ];
        let mut command = run_command(state.path(), provider.endpoint());
        let expected = match failure {
            Failure::Import => "startup import fixture rejected",
            Failure::Theme => {
                command.args(["--theme", "missing-theme"]);
                "choose an unambiguous installed theme"
            }
            Failure::Provider => {
                command.args(["--package-provider", "@fixture/missing:provider"]);
                "no granted package export named"
            }
        };
        let mut cli = StartupCli::spawn(command.args(["run", "hello"]), markers);
        let output = cli.wait();
        assert!(!output.status.success());
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(diagnostic.contains(expected), "{failure:?}: {diagnostic}");
        cli.assert_reaped();
    }
}

#[test]
fn normal_shutdown_reaps_all_loaded_package_hosts() {
    let provider = ProviderFixture::new(vec![json!({"content": "startup fixture completed"})]);
    let state = fixture_root();
    let markers = vec![
        package(state.path(), Language::Node, false, false),
        package(state.path(), Language::Python, false, false),
    ];
    let mut cli = StartupCli::spawn(
        run_command(state.path(), provider.endpoint()).args(["run", "hello"]),
        markers,
    );
    let output = cli.wait();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("startup fixture completed"));
    cli.assert_reaped();
}

#[test]
fn startup_signal_cancels_provider_preflight_and_reaps_packages() {
    let state = fixture_root();
    let executable = claude_fixture(state.path(), "success");
    fs::write(state.path().join("claude-client/status-wait"), "wait").unwrap();
    let package_pid = package(state.path(), Language::Node, false, false);
    let status_pid = state.path().join("claude-client/status-pid");
    let mut cli = StartupCli::spawn(
        command(state.path(), &executable).args([
            "--capacity",
            "fixed",
            "--provider",
            "claude",
            "run",
            "hello",
        ]),
        vec![package_pid.clone(), status_pid.clone()],
    );
    assert!(test_kill_process(cli.wait_marker(&status_pid)).is_ok());
    assert!(test_kill_process(cli.wait_marker(&package_pid)).is_ok());
    cli.signal(Signal::TERM);
    let output = cli.wait();
    assert!(!output.status.success());
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains("startup cancelled"), "{diagnostic}");
    cli.assert_reaped();
}
