#!/usr/bin/env python3
"""Run native and relay gates with pinned inputs and no provider calls."""

from pathlib import Path
import os
import shutil
import subprocess
import sys
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def run(arguments: list[str], *, cwd: Path = ROOT, env: dict[str, str] | None = None) -> None:
    executable = shutil.which(arguments[0])
    if executable is None:
        raise SystemExit(f"Missing {arguments[0]}. See docs/contributing.md.")
    subprocess.run([executable, *arguments[1:]], cwd=cwd, env=env, check=True)


def fixture_environment(messages: Path) -> dict[str, str]:
    # Use the same reviewed selector as the PTY gate before any npm install.
    subprocess.run(
        [sys.executable, str(ROOT / "scripts/check_terminal.py"), "--native-fixture", str(messages)],
        cwd=ROOT, check=True, stdout=subprocess.DEVNULL,
    )
    return {
        **os.environ,
        "EJM_NATIVE_TEST_ARTIFACTS": str(messages.resolve()),
        "EJM_FIXTURE_PYTHON": sys.executable,
    }


def relay(messages: Path) -> None:
    environment = fixture_environment(messages)
    tools = tomllib.loads((ROOT / "tools.toml").read_text())
    for program, expected in (("node", "v" + tools["node"]["version"]), ("npm", tools["npm"]["version"])):
        executable = shutil.which(program)
        if executable is None:
            raise SystemExit(f"Missing {program}. See docs/contributing.md.")
        actual = subprocess.check_output([executable, "--version"], cwd=ROOT, text=True).strip()
        if actual != expected:
            raise SystemExit(f"Relay checks require {program} {expected}; found {actual}.")
    for arguments in (
        ["npm", "ci", "--ignore-scripts", "--install-links"],
        ["npm", "run", "build:web", "--", "--fixture"],
        ["npm", "run", "check"],
    ):
        run(arguments, cwd=ROOT / "relay", env=environment)


def native_tests(messages: Path) -> None:
    # Tee Cargo's current compiler-artifact records without another build.
    # Test stdout remains visible, including on failure. JSONL stays local.
    cargo = shutil.which("cargo")
    if cargo is None:
        raise SystemExit("Missing cargo. See docs/contributing.md.")
    command = [cargo, "test", "--workspace", "--all-targets", "--locked", "--message-format=json-render-diagnostics"]
    with messages.open("w", encoding="utf-8") as output:
        messages.chmod(0o600)
        with subprocess.Popen(command, cwd=ROOT, stdout=subprocess.PIPE, text=True, encoding="utf-8") as child:
            assert child.stdout is not None
            for line in child.stdout:
                output.write(line)
                sys.stdout.write(line)
            result = child.wait()
        if result:
            raise subprocess.CalledProcessError(result, command)


def full() -> None:
    if os.name == "nt":
        raise SystemExit("The full native terminal gate requires Linux or macOS; Windows terminal acceptance is separate. See docs/contributing.md.")
    with tempfile.TemporaryDirectory(prefix="ejm-cli-check-") as temporary:
        messages = Path(temporary) / "cargo-tests.jsonl"
        run([sys.executable, "scripts/check_architecture.py"])
        run(["cargo", "fmt", "--all", "--", "--check"])
        run(["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"])
        native_tests(messages)
        run(["cargo", "build", "--workspace", "--release", "--locked"])
        run(["cargo", "doc", "--workspace", "--no-deps", "--locked"], env={**os.environ, "RUSTDOCFLAGS": "-D warnings"})
        run(["cargo", "machete", "--with-metadata"])
        run(["cargo", "deny", "--locked", "--all-features", "check"])
        target = Path(os.environ.get("CARGO_TARGET_DIR", "target"))
        run([sys.executable, "scripts/check_terminal.py", str(target / "release/ejm-cli"), str(messages)])
        relay(messages)


def main() -> None:
    os.environ["CARGO_BUILD_JOBS"] = "2"
    if sys.argv[1:] == ["--relay"]:
        messages = os.environ.get("EJM_NATIVE_TEST_ARTIFACTS")
        if not messages:
            raise SystemExit("Set EJM_NATIVE_TEST_ARTIFACTS to the current locked Cargo test JSONL, or run ./scripts/check.")
        relay((ROOT / messages).resolve())
    elif len(sys.argv) == 1:
        full()
    else:
        raise SystemExit("Usage: scripts/check [--relay]")


if __name__ == "__main__":
    main()
