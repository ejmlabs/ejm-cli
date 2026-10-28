#!/usr/bin/env python3
"""Exercise real terminal input and restoration without credentials or model calls."""

from pathlib import Path
import json
import os
import re
import secrets
import signal
import subprocess
import sys
import tempfile
import time


def supervise(binary: Path, state: str, report: Path, mode: str) -> None:
    import termios

    # Keep the session leader alive until flags are checked. macOS revokes the
    # terminal when its session leader exits, even with the parent slave open.
    original = termios.tcgetattr(0)
    if mode == "entry-signal":
        command = [str(binary), "--relay", "https://relay.invalid", "--state-dir", state,
                   "remote", "migrate-credentials"]
    elif mode.startswith("remote-"):
        os.environ["EJM_REMOTE_TERMINAL_CASE"] = mode.removeprefix("remote-")
        os.environ["EJM_REMOTE_TERMINAL_CONFIG"] = str(Path(state) / "private-input.json")
        command = [str(binary), "--exact", REMOTE_TEST, "--ignored", "--nocapture", "--test-threads=1"]
    else:
        command = [str(binary), "--replay", "--state-dir", state]
    child = subprocess.Popen(command)
    if mode.startswith("remote-"):
        signal.signal(signal.SIGINT, signal.SIG_IGN)
        signal.signal(signal.SIGUSR1, lambda *_: child.send_signal(signal.SIGUSR1))
    signal.signal(signal.SIGTERM, lambda *_: child.send_signal(signal.SIGTERM))
    returncode = child.wait()
    import select

    # macOS marks restored canonical input PENDIN until readiness is checked.
    # Poll without reading so the comparison still checks every terminal flag.
    select.select([0], [], [], 0)
    report.write_text(json.dumps({
        "returncode": returncode,
        "restored": termios.tcgetattr(0) == original,
    }))


def check(binary: Path, terminate: bool, remote_case: str | None = None, remote_entry: bool = False) -> None:
    import fcntl
    import pty
    import select
    import struct
    import termios

    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
    output = bytearray()
    environment = os.environ.copy()
    environment.pop("EJM_CLI_API_KEY", None)
    environment.pop("OPENAI_API_KEY", None)
    environment["TERM"] = "xterm-256color"

    def controlling_terminal() -> None:
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)

    with tempfile.TemporaryDirectory(prefix="ejm-cli-terminal-") as state:
        private_input = secrets.token_urlsafe(32).encode()
        config = Path(state) / "private-input.json"
        if remote_case is not None:
            with open(config, "w", opener=lambda path, flags: os.open(path, flags, 0o600)) as target:
                json.dump({"input": private_input.decode()}, target)
        if remote_case is not None or remote_entry:
            for name in tuple(environment):
                if name.startswith(("EJM_CLI_", "EJM_REMOTE_", "ANTHROPIC_", "CLAUDE_")) or name in {"OPENAI_API_KEY", "OPENAI_BASE_URL"}:
                    environment.pop(name)
        report = Path(state) / "terminal-check.json"
        mode = "entry-signal" if remote_entry else "remote-" + remote_case if remote_case else "replay"
        process = subprocess.Popen(
            [sys.executable, str(Path(__file__).resolve()), "--child", str(binary), state,
             str(report), mode],
            stdin=slave, stdout=slave, stderr=slave,
            env=environment, preexec_fn=controlling_terminal,
        )

        def read_pty() -> bool:
            try:
                data = os.read(master, 65536)
            except OSError:
                return False
            if not data:
                return False
            # Crossterm asks for the cursor when the fullscreen TUI resumes.
            # Include the previous three bytes so split queries receive one reply.
            start = max(0, len(output) - 3)
            output.extend(data)
            queries = output.count(b"\x1b[6n", start)
            if queries:
                os.write(master, b"\x1b[1;1R" * queries)
            return True

        def read_until(needle: bytes, timeout: float = 10, after: int = 0) -> None:
            # Differential draws can insert cursor moves between visible words.
            expected = re.sub(rb"\s+", b"", needle)

            def contains_text() -> bool:
                visible = re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", output[after:])
                return expected in re.sub(rb"\s+", b"", visible)

            deadline = time.monotonic() + timeout
            while not contains_text():
                if time.monotonic() >= deadline:
                    if remote_case is not None or remote_entry:
                        raise AssertionError("remote terminal fixture did not reach its expected stage; private output omitted")
                    raise AssertionError(f"terminal did not show {needle!r}; last output {bytes(output[-1000:])!r}")
                if select.select([master], [], [], 0.05)[0] and not read_pty():
                    break
                if process.poll() is not None:
                    break
            assert contains_text(), f"terminal exited before showing {needle!r}"

        def wait_for_exit(timeout: float) -> None:
            deadline = time.monotonic() + timeout
            while process.poll() is None:
                if time.monotonic() >= deadline:
                    raise subprocess.TimeoutExpired(process.args, timeout)
                # Keep draining while the TUI draws and restores the terminal.
                if select.select([master], [], [], 0.05)[0]:
                    read_pty()

        screen_rows = 30

        def repaint_until(needle: bytes) -> bytes:
            # Resize forces a full paint. A differential paint after completion may
            # contain only the appended letters, which cannot prove the whole draft.
            nonlocal screen_rows
            deadline = time.monotonic() + 0.2
            while time.monotonic() < deadline:
                if select.select([master], [], [], 0.05)[0] and not read_pty():
                    break
            deadline = time.monotonic() + 10
            while True:
                mark = len(output)
                screen_rows = 31 if screen_rows == 30 else 30
                fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", screen_rows, 100, 0, 0))
                try:
                    read_until(needle, timeout=1, after=mark)
                except AssertionError:
                    if process.poll() is not None or time.monotonic() >= deadline:
                        raise
                else:
                    return re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", output[mark:])

        def check_remote_flow() -> None:
            if remote_case == "pairing-replacement":
                read_until(b"original controller fixture")
                os.write(master, b"y")
                # A resize paint proves the replacement remains visible after
                # the old key was parsed and its confirmation was abandoned.
                repaint_until(b"replacement controller fixture")
                os.write(master, b"y")
                repaint_until(b"Pairing boundary passed")
                os.write(master, b"/quit\r")
                return
            if remote_case == "host-signal":
                read_until(b"Relay sign-in")
                flags = termios.tcgetattr(slave)[3]
                assert not flags & (termios.ICANON | termios.ECHO), "host login did not protect terminal input"
                os.write(master, b"\x1b[200~" + private_input + b"\x1b[201~")
                process.send_signal(signal.SIGTERM)
                return
            if remote_case and remote_case.startswith("observer-"):
                read_until(b"Observer blocked")
                if remote_case == "observer-signal":
                    process.send_signal(signal.SIGTERM)
                else:
                    command = {"observer-stop": b"/remote stop\r", "observer-switch": b"/new\r", "observer-quit": b"/quit\r"}[remote_case]
                    os.write(master, command)
                    if remote_case != "observer-quit":
                        expected = b"Sharing stopped" if remote_case == "observer-stop" else b"Sharing ended because the selected session changed"
                        read_until(expected)
                        os.write(master, b"/quit\r")
                return
            read_until(b"Connect to a shared session")
            assert not termios.tcgetattr(slave)[3] & termios.ICANON, "private input did not enter raw mode"
            entered = private_input.ljust(8193, b"x") if remote_case == "error" else private_input
            if remote_case in {"queued-complete", "queued-partial"}:
                # One write lets a buffered reader parse beyond the submitting
                # Enter, including a complete or unfinished second private paste.
                queued = b"\x1b[200~" + entered
                if remote_case == "queued-complete":
                    queued += b"\x1b[201~"
                os.write(master, b"\x1b[200~" + entered + b"\x1b[201~\r" + queued)
                repaint_until(b"Ordinary input ready")
                if remote_case == "queued-partial":
                    os.write(master, b"\x1b[201~")
                os.write(master, b"safe boundary input\r")
                repaint_until(b"Private input boundary passed")
                os.write(master, b"q")
                return
            os.write(master, b"\x1b[200~" + entered + b"\x1b[201~")
            if remote_case == "signal":
                process.send_signal(signal.SIGTERM)
            elif remote_case == "panic":
                process.send_signal(signal.SIGUSR1)
            elif remote_case == "cancel":
                os.write(master, b"\x1b")
            elif remote_case == "interrupt":
                os.write(master, b"\x03")
            elif remote_case == "flow":
                os.write(master, b"\r")
                read_until(b"ejm-remote:v1:")
                os.write(master, b"\r")
                repaint_until(b"Confirm controller identity")
                os.write(master, b"y")
                repaint_until(b"Host paired")
                os.write(master, b"s")
                repaint_until(b"Sharing stopped")
                os.write(master, b"r")
                repaint_until(b"Private pairing fixture")
                os.write(master, b"\r")
                repaint_until(b"Switch shared session")
                os.write(master, b"n")
                repaint_until(b"Selected session changed")
                os.write(master, b"q")

        try:
            if remote_entry:
                read_until(b"Local confirmation")
                flags = termios.tcgetattr(slave)[3]
                assert not flags & termios.ICANON, "protected command did not enter raw mode"
                assert not flags & termios.ECHO, "protected command left terminal echo enabled"
                # A paste event cannot confirm this prompt. Terminate before any
                # credential-store access or network operation can begin.
                os.write(master, b"\x1b[200~" + private_input + b"\x1b[201~")
                process.send_signal(signal.SIGTERM)
            elif remote_case is not None:
                check_remote_flow()
            elif terminate:
                read_until(b"What shall we build?")
                assert not termios.tcgetattr(slave)[3] & termios.ICANON, "TUI must enter raw mode"
                process.send_signal(signal.SIGTERM)
            else:
                read_until(b"What shall we build?")
                assert not termios.tcgetattr(slave)[3] & termios.ICANON, "TUI must enter raw mode"
                os.write(master, b"ejm-cli-pty-check\r")
                repaint_until(b"Demo: ejm-cli-pty-check")
                os.write(master, b"/help\r")
                repaint_until(b"Shortcuts")
                os.write(master, b"\x1b")
                repaint_until(b"completed")
                os.write(master, b"/agents\r")
                repaint_until(b"choose")
                os.write(master, b"\x1b")
                repaint_until(b"ejm-cli-pty-check")
                os.write(master, b"/quit\r")
            wait_for_exit(10)
            while select.select([master], [], [], 0.05)[0]:
                if not read_pty():
                    break
            assert process.returncode == 0, f"terminal supervisor exit status {process.returncode}"
            result = json.loads(report.read_text())
            expected_returncode = 1 if remote_entry else 101 if remote_case == "panic" else 0
            assert result["returncode"] == expected_returncode, f"TUI exit status {result['returncode']}"
            assert result["restored"], "terminal flags were not restored"
            assert b"\x1b[?1049h" in output and b"\x1b[?1049l" in output, "alternate screen was not restored"
            assert b"\x1b[?2004l" in output, "bracketed paste was not disabled"
            assert b"\x1b[?25h" in output, "cursor was not restored"
            if remote_case is not None or remote_entry:
                assert private_input not in output, "private invitation input reached the terminal"
                for match in re.finditer(b"ejm-remote:v1:", output):
                    entered = output.rfind(b"\x1b[?1049h", 0, match.start())
                    left = output.rfind(b"\x1b[?1049l", 0, match.start())
                    assert entered > left, "private invitation reveal escaped the alternate screen"
                if remote_case == "flow":
                    assert b"ejm-remote:v1:" in output, "private invitation reveal was not exercised"
                if remote_case is not None:
                    config.unlink()
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGKILL)
                wait_for_exit(5)
            os.close(master)
            os.close(slave)


REMOTE_TEST = "remote::host_ui::tests::remote_terminal_interaction_fixture"
REMOTE_CASES = ("flow", "queued-complete", "queued-partial", "cancel", "interrupt", "error", "signal", "panic", "host-signal", "observer-stop", "observer-switch", "observer-quit", "observer-signal", "pairing-replacement")


def native_fixture(messages: Path) -> Path:
    """Use the current Cargo test invocation's artifact, including cache hits."""
    manifest = Path(__file__).resolve().parents[1] / "crates/ejm-cli/Cargo.toml"
    candidates = set()
    finished = False
    with messages.open() as source:
        for line in source:
            try:
                message = json.loads(line)
            except json.JSONDecodeError:
                continue  # Cargo's stream also contains ordinary test output.
            if not isinstance(message, dict):
                continue
            if message.get("reason") == "build-finished":
                finished = message.get("success") is True
            if message.get("reason") != "compiler-artifact":
                continue
            target = message.get("target", {})
            profile = message.get("profile", {})
            path = message.get("manifest_path")
            executable = message.get("executable")
            if (isinstance(target, dict) and isinstance(profile, dict)
                    and target.get("name") == "ejm-cli" and target.get("kind") == ["bin"]
                    and profile.get("test") is True and isinstance(path, str)
                    and Path(path).resolve() == manifest and isinstance(executable, str)):
                candidates.add(Path(executable).resolve(strict=True))
    if not finished or len(candidates) != 1:
        raise SystemExit("Expected one native unit-test artifact from the current successful locked Cargo test invocation.")
    fixture = candidates.pop()
    if not fixture.is_file() or not os.access(fixture, os.X_OK):
        raise SystemExit("The current Cargo native unit-test artifact is not executable.")
    listed = subprocess.run([str(fixture), "--list", "--exact", REMOTE_TEST], capture_output=True, text=True, timeout=10, check=False)
    if listed.returncode != 0 or REMOTE_TEST + ": test" not in listed.stdout.splitlines():
        raise SystemExit("The current Cargo native unit-test artifact does not contain the remote PTY fixture.")
    return fixture


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--native-fixture":
        print(native_fixture(Path(sys.argv[2])))
        raise SystemExit(0)
    if os.name != "posix":
        raise SystemExit("PTY restoration checks require Linux or macOS; run them on a supported host.")
    if len(sys.argv) == 6 and sys.argv[1] == "--child":
        supervise(Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4]), sys.argv[5])
        raise SystemExit(0)
    if len(sys.argv) == 3 and sys.argv[1] == "--entry-signal-only":
        check(Path(sys.argv[2]).resolve(strict=True), terminate=True, remote_entry=True)
        print("ejm-cli remote command SIGTERM restoration passed.")
        raise SystemExit(0)
    if len(sys.argv) == 3 and sys.argv[1] == "--remote-only":
        fixture = native_fixture(Path(sys.argv[2]))
        for case in REMOTE_CASES:
            check(fixture, terminate=False, remote_case=case)
        print("ejm-cli remote PTY privacy, lifecycle and restoration passed.")
        raise SystemExit(0)
    if len(sys.argv) not in {2, 3}:
        raise SystemExit("usage: check_terminal.py PATH_TO_EJM_CLI [CARGO_TEST_MESSAGES_JSONL]")
    executable = Path(sys.argv[1]).resolve(strict=True)
    check(executable, terminate=False)
    check(executable, terminate=True)
    check(executable, terminate=True, remote_entry=True)
    if len(sys.argv) == 3:
        fixture = native_fixture(Path(sys.argv[2]))
        for case in REMOTE_CASES:
            check(fixture, terminate=False, remote_case=case)
    print("ejm-cli PTY interaction and terminal restoration passed.")
