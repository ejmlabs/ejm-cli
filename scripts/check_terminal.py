#!/usr/bin/env python3
"""Exercise real terminal input and restoration without credentials or model calls."""

import json
import math
import os
import re
import secrets
import shutil
import signal
import sqlite3
import subprocess
import sys
import tempfile
import time
from pathlib import Path

LOGIN_OUTPUT = b"CLAUDE_FIXTURE_LOGIN_OUTPUT"
PRIVATE_STATUS = b"CLAUDE_FIXTURE_PRIVATE_STATUS"


def supervise(binary: Path, state: str, report: Path, mode: str) -> None:
    import termios

    # Keep the session leader alive until flags are checked. macOS revokes the
    # terminal when its session leader exits, even with the parent slave open.
    original = termios.tcgetattr(0)
    if mode.startswith("remote-"):
        os.environ["EJM_REMOTE_TERMINAL_CASE"] = mode.removeprefix("remote-")
        os.environ["EJM_REMOTE_TERMINAL_CONFIG"] = str(Path(state) / "private-input.json")
        command = [str(binary), "remote_terminal_interaction_fixture", "--ignored", "--nocapture", "--test-threads=1"]
    elif mode == "unconfigured":
        command = [
            str(binary), "--state-dir", state, "--plain", "--no-color",
            "--claude-path", str(Path(state) / "claude-client" / "claude-fixture"),
        ]
    else:
        config = Path(state) / "terminal-fixture.json"
        config.write_text(json.dumps({"state": state,
                                      "claude": str(Path(state) / "claude-client" / "claude-fixture")}))
        os.environ["EJM_TEST_TERMINAL_CONFIG"] = str(config)
        command = [str(binary), "configured_terminal_interaction_fixture", "--ignored", "--nocapture", "--test-threads=1"]
    child = subprocess.Popen(command)
    # The foreground group also contains this PTY supervisor. Keep it alive
    # while Ctrl+C reaches the app and its inherited-stdio sign-in child.
    signal.signal(signal.SIGINT, signal.SIG_IGN)
    signal.signal(signal.SIGTERM, lambda *_: child.send_signal(signal.SIGTERM))
    signal.signal(signal.SIGUSR1, lambda *_: child.send_signal(signal.SIGUSR1))
    returncode = child.wait()
    import select

    # macOS marks restored canonical input PENDIN until readiness is checked.
    # Poll without reading so the comparison still checks every terminal flag.
    select.select([0], [], [], 0)
    report.write_text(json.dumps({
        "returncode": returncode,
        "restored": termios.tcgetattr(0) == original,
    }))


def check(
    binary: Path, terminate: bool, unconfigured: bool = False, claude_auth: bool = False,
    remote_case: str | None = None,
) -> None:
    import fcntl
    import pty
    import select
    import struct
    import termios

    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
    output = bytearray()

    def controlling_terminal() -> None:
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)

    with tempfile.TemporaryDirectory(prefix="ejm-cli-terminal-") as state:
        directory = Path(state)
        fixture = directory / "claude-client"
        fixture.mkdir()
        project = Path(__file__).resolve().parents[1]
        target = project / os.environ.get("CARGO_TARGET_DIR", "target")
        shutil.copy2(target / "debug/examples/claude-fixture", fixture / "claude-fixture")
        (fixture / "mode").write_text("wait")
        home = directory / "home"
        home.mkdir()
        private_input = secrets.token_urlsafe(32).encode()
        if remote_case is not None:
            config = directory / "private-input.json"
            with open(config, "w", opener=lambda path, flags: os.open(path, flags, 0o600)) as target:
                json.dump({"input": private_input.decode()}, target)
        environment = os.environ.copy()
        for name in tuple(environment):
            if name.startswith(("EJM_CLI_", "EJM_REMOTE_", "ANTHROPIC_", "CLAUDE_")) or name in {
                "OPENAI_API_KEY", "OPENAI_BASE_URL", "CLAUDECODE",
            }:
                environment.pop(name)
        environment.update({
            "TERM": "xterm-256color",
            "HOME": str(home),
            "USERPROFILE": str(home),
            "LOCALAPPDATA": str(home / "local"),
            "APPDATA": str(home / "roaming"),
            "XDG_CONFIG_HOME": str(home / "config"),
            "XDG_DATA_HOME": str(home / "data"),
            "CLAUDE_CONFIG_DIR": str(home / "claude"),
        })
        report = Path(state) / "terminal-check.json"
        process = subprocess.Popen(
            [sys.executable, str(Path(__file__).resolve()), "--child", str(binary), state,
             str(report), "remote-" + remote_case if remote_case else "unconfigured" if unconfigured else "configured"],
            stdin=slave, stdout=slave, stderr=slave,
            env=environment, preexec_fn=controlling_terminal,  # noqa: PLW1509 - this supervisor is single threaded.
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
                    if remote_case is not None:
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

        def command(text: bytes, expected: bytes) -> None:
            os.write(master, text + b"\r")
            repaint_until(expected)

        def open_login() -> Path:
            # Fail before ChatGPT auth if a navigation regression selects it.
            # Keep this blocker until the intended flow or dismissal is proven.
            blocker = directory / "auth"
            blocker.touch(mode=0o600, exist_ok=False)
            invocations = fixture / "invocations.jsonl"
            before = invocations.read_bytes() if invocations.exists() else b""
            command(b"/login", b"OpenAI-compatible API")
            after = invocations.read_bytes() if invocations.exists() else b""
            assert after == before, "opening the login picker started the provider client"
            return blocker

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

        def begin_login(mode: str) -> int:
            (fixture / "mode").write_text(mode)
            mark = len(output)
            blocker = open_login()
            os.write(master, b"\x1b[B\r")
            read_until(LOGIN_OUTPUT, after=mark)
            blocker.unlink()
            marker = output.index(LOGIN_OUTPUT, mark)
            entered = output.rfind(b"\x1b[?1049h", 0, marker)
            left = output.rfind(b"\x1b[?1049l", 0, marker)
            assert left > entered, "official sign-in output entered the alternate screen"
            login = json.loads((fixture / "login.json").read_text())
            assert login["terminal"], "official sign-in did not inherit terminal stdio"
            assert login["canonical"] and login["echo"], "TUI did not release terminal input"
            return login["pid"]

        def assert_login_reaped(pid: int) -> None:
            deadline = time.monotonic() + 2
            while True:
                try:
                    os.kill(pid, 0)
                except ProcessLookupError:
                    return
                assert time.monotonic() < deadline, "official sign-in child was not reaped"
                # Keep servicing the PTY while the host completes child cleanup.
                if select.select([master], [], [], 0.05)[0]:
                    read_pty()

        def assert_tui_returned(needle: bytes) -> None:
            frame = repaint_until(needle)
            assert not termios.tcgetattr(slave)[3] & termios.ICANON, "TUI did not restore raw mode"
            assert output.rfind(b"\x1b[?1049h") > output.rfind(b"\x1b[?1049l"), "TUI did not restore alternate screen"
            assert LOGIN_OUTPUT not in frame, "official sign-in output entered the conversation"

        login_pid = None
        try:
            if remote_case is not None:
                check_remote_flow()
            elif claude_auth:
                read_until(b"Internal terminal fixture ready.")
                assert not termios.tcgetattr(slave)[3] & termios.ICANON, "TUI must enter raw mode"
                if terminate:
                    login_pid = begin_login("raw-wait")
                    process.send_signal(signal.SIGTERM)
                else:
                    for mode in ("failure", "wait", "raw-wait"):
                        login_pid = begin_login(mode)
                        if mode != "failure":
                            os.write(master, b"\x03")
                        assert_tui_returned(b"> /login")
                        assert process.poll() is None, "sign-in cancellation exited the app"
                        assert_login_reaped(login_pid)
                        os.write(master, b"\x15")
                        command(b"/status", b"Connection: ready")
                        repaint_until(b"Model: @fixture/terminal:provider")
                        prompt = f"claude-{mode}-keeps-replay".encode()
                        command(prompt, b"Demo: " + prompt)

                    login_pid = begin_login("wait")
                    os.write(master, b"complete\r")
                    assert_tui_returned(b"terminal-fixture@example.test")
                    assert_login_reaped(login_pid)
                    command(b"/status", b"Connection: ready")
                    repaint_until(b"claude sonnet")
                    command(b"/accounts", b"No saved accounts.")
                    command(b"/logout claude", b"Signed out of Claude")
                    command(b"/status", b"Connection: not connected")
                    os.write(master, b"/quit\r")
            elif terminate:
                read_until(b"No model connection" if unconfigured else b"Internal terminal fixture ready.")
                assert not termios.tcgetattr(slave)[3] & termios.ICANON, "TUI must enter raw mode"
                process.send_signal(signal.SIGTERM)
            else:
                read_until(b"No model connection" if unconfigured else b"Internal terminal fixture ready.")
                assert not termios.tcgetattr(slave)[3] & termios.ICANON, "TUI must enter raw mode"
                if unconfigured:
                    command(b"unconfigured-pty-draft", b"No model connection")
                    repaint_until(b"> unconfigured-pty-draft")
                    os.write(master, b"\x15")
                    command(b"/model openai fixture-model", b"set EJM_CLI_API_KEY")
                    repaint_until(b"> /model openai fixture-model")
                    os.write(master, b"\x15")
                else:
                    command(b"ejm-cli-pty-check", b"Demo: ejm-cli-pty-check")

                os.write(master, b"/")
                repaint_until(b"commands: Up/Down choose")
                os.write(master, b"mod")
                repaint_until(b"> /mod")
                os.write(master, b"\t")
                repaint_until(b"> /model ")
                os.write(master, b"\x15/mod")
                repaint_until(b"commands: Up/Down choose")
                os.write(master, b"\x1b")
                dismissed = repaint_until(b"> /mod")
                assert b"commands: Up/Down" not in dismissed, "Escape did not dismiss the picker"
                os.write(master, b"\x15")

                os.write(master, b"/resume\r\x1b")
                # Failed commands retain their draft and command picker. Dismiss
                # the picker before checking the error it can otherwise cover.
                repaint_until(b"Usage: /resume ID")
                repaint_until(b"> /resume")
                os.write(master, b"\x15")
                blocker = open_login()
                os.write(master, b"\x1b[B\x1b")
                dismissed = repaint_until(b"> /login")
                assert b"OpenAI-compatible API" not in dismissed, "Escape did not dismiss the login picker"
                blocker.unlink()
                os.write(master, b"\x15")
                blocker = open_login()
                os.write(master, b"\x1b[6~\r")
                repaint_until(b"Anthropic API access uses")
                blocker.unlink()
                command(b"/accounts", b"No saved accounts.")
                repaint_until(b"signed out")
                connection = b"Connection: not connected" if unconfigured else b"Connection: ready"
                command(b"/status", connection)
                repaint_until(b"Models: 0 active, 0 queued")
                read_until(b"Tools: 0 active, 0 queued")
                command(b"/help", b"Conversation")
                os.write(master, b"\x1b")
                repaint_until(connection)
                command(b"/agents", b"choose")
                os.write(master, b"\x1b")
                repaint_until(connection)
                os.write(master, b"/quit\r")
            wait_for_exit(10)
            while select.select([master], [], [], 0.05)[0]:
                if not read_pty():
                    break
            assert process.returncode == 0, f"terminal supervisor exit status {process.returncode}"
            result = json.loads(report.read_text())
            expected_returncode = 101 if remote_case == "panic" else 0
            assert result["returncode"] == expected_returncode, f"TUI exit status {result['returncode']}"
            assert result["restored"], "terminal flags were not restored"
            assert b"\x1b[?1049h" in output and b"\x1b[?1049l" in output, "alternate screen was not restored"
            assert b"\x1b[?2004l" in output, "bracketed paste was not disabled"
            assert b"\x1b[?25h" in output, "cursor was not restored"
            assert PRIVATE_STATUS not in output, "private official status fields reached the terminal"
            if remote_case is not None:
                assert private_input not in output, "private invitation input reached the terminal"
                for match in re.finditer(b"ejm-remote:v1:", output):
                    entered = output.rfind(b"\x1b[?1049h", 0, match.start())
                    left = output.rfind(b"\x1b[?1049l", 0, match.start())
                    assert entered > left, "private invitation reveal escaped the alternate screen"
                if remote_case == "flow":
                    assert b"ejm-remote:v1:" in output, "private invitation reveal was not exercised"
                config.unlink()
            if login_pid is not None:
                assert_login_reaped(login_pid)
                # Inspect only fresh fixture sessions, never a user's state.
                with sqlite3.connect((directory / "sessions.sqlite3").as_uri() + "?mode=ro", uri=True) as database:
                    for row in database.iterdump():
                        assert LOGIN_OUTPUT.decode() not in row, "sign-in output entered durable session data"
                        assert PRIVATE_STATUS.decode() not in row, "private status fields entered durable session data"
                for path in directory.rglob("*"):
                    if path.is_file() and path.suffix in {".json", ".jsonl"}:
                        assert LOGIN_OUTPUT not in path.read_bytes(), f"sign-in output persisted in {path.name}"
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGKILL)
                wait_for_exit(5)
            os.close(master)
            os.close(slave)


REMOTE_TEST = "remote::host_ui::tests::remote_terminal_interaction_fixture"
REMOTE_CASES = ("flow", "queued-complete", "queued-partial", "cancel", "interrupt", "error", "signal", "panic", "host-signal", "observer-stop", "observer-switch", "observer-quit", "observer-signal", "pairing-replacement")

def fresh_stop_acknowledgement(before: str, after: str, title: str, marker: str) -> str | None:
    """Require a new acknowledgement after a selected session's staged draft."""
    acknowledgements = ("Stopping", "cancelled")
    if (title not in "\n".join(before.splitlines()[:2])
            or marker not in "\n".join(before.splitlines()[-4:])
            or any(text in before for text in acknowledgements)
            or title not in "\n".join(after.splitlines()[:2])
            or marker in after):
        return None
    return next((text for text in acknowledgements if text in after), None)


def latency_sample_verdict(values: list, limit: int) -> dict:
    """Describe the retained samples without inferring missing coverage."""
    result = {"count": len(values), "budget_ms": 50, "p50_ms": None, "p95_ms": None}
    if len(values) > limit or any(type(value) not in (int, float)
                                 or not math.isfinite(value) or value < 0 for value in values):
        return result | {"status": "invalid_samples"}
    if not values:
        return result | {"status": "no_samples"}
    ordered = sorted(values)
    result.update(p50_ms=ordered[(len(ordered) - 1) // 2],
                  p95_ms=ordered[math.ceil(len(ordered) * .95) - 1])
    return result | {"status": "within_budget" if result["p95_ms"] <= 50 else "over_budget"}


def stop_sample_validation(report: dict) -> dict:
    """Cross-check command proofs with the fixture's retired-session count."""
    if "operator_stop" not in report:
        return {"status": "unavailable", "reasons": ["operator Stop settings are missing"]}
    samples = [sample for sample in report.get("trace", []) if sample["action"] == "stop_acknowledgement"]
    expected = report.get("requested_samples") if report["operator_stop"] else 0
    retired = report.get("fixture", {}).get("operator_stopped_sessions")
    sessions = {sample.get("session_id") for sample in samples}
    reasons = []
    if report["operator_stop"] and (type(expected) is not int or not 1 <= expected <= 100):
        reasons.append("requested Stop sample count is missing or outside its bound")
    if len(samples) != expected or len(sessions) != expected or None in sessions:
        reasons.append("Stop samples do not cover distinct requested sessions")
    if retired != expected:
        reasons.append("retired-session count differs from requested Stop samples")
    for sample in samples:
        proof = sample.get("proof", {})
        if (sample.get("command") != "/stop"
                or type(sample.get("sent_ns")) is not int
                or type(sample.get("observed_ns")) is not int
                or not 0 <= sample["sent_ns"] <= sample["observed_ns"]
                or not all(isinstance(proof.get(key), str) and proof[key]
                           for key in ("before", "after", "title", "marker"))
                or fresh_stop_acknowledgement(proof["before"], proof["after"],
                                               proof["title"], proof["marker"]) is None):
            reasons.append("Stop sample lacks a fresh selected-session command acknowledgement")
            break
    cancelled = report.get("fixture", {}).get("cancelled")
    if expected and (type(cancelled) is not int or cancelled < 1):
        reasons.append("no cancellation completed during the measured work window")
    return {"status": "failed" if reasons else "passed", "requested": expected,
            "samples": len(samples), "distinct_sessions": len(sessions),
            "retired_sessions": retired, "cancelled_during_work": cancelled, "reasons": reasons}


def write_measurement_report(path: Path, report: dict) -> None:
    """Save workload and trace verdicts before rejecting failed work."""
    fixture = report.get("fixture")
    if fixture is not None:
        report["workload_status"] = "failed" if fixture["failed"] or fixture["drain_failed"] else "passed"
    trace_status = "disabled"
    trace_reasons = []
    if report.get("event_trace"):
        trace_fixture = fixture if isinstance(fixture, dict) else {}
        integrity = trace_fixture.get("paint_trace_integrity")
        counters = ("subscriptions", "selection_changes", "generation_resets", "lag_signals",
                    "reported_lag_total", "closed_signals", "observer_error_count",
                    "missing_commit_stamps", "uncovered_saved_markers")
        accounting_available = (
            isinstance(integrity, dict) and integrity.get("enabled") is True
            and all(type(integrity.get(name)) is int and integrity[name] >= 0 for name in counters)
            and isinstance(integrity.get("observer_error_samples"), list)
            and all(isinstance(sample, str) for sample in integrity["observer_error_samples"])
        )
        if not accounting_available:
            trace_reasons.append("paint trace accounting is missing or unavailable")
        else:
            if integrity["lag_signals"] or integrity["reported_lag_total"]:
                trace_reasons.append("selected subscription reported lag; its totals are not an exact lost-event count")
            if integrity["closed_signals"]:
                trace_reasons.append("selected subscription closed before observation stopped")
            if integrity["observer_error_count"] or integrity["observer_error_samples"]:
                trace_reasons.append("selected-view trace recorded an error")
            if integrity["missing_commit_stamps"]:
                trace_reasons.append("selected markers arrived without a durable commit stamp")
            if integrity["uncovered_saved_markers"]:
                trace_reasons.append("selected saved or recovery markers have no monotonic timing origin")
        omitted = trace_fixture.get("paint_trace_omitted")
        if type(omitted) is not int or omitted < 0:
            trace_reasons.append("paint omission accounting is missing or unavailable")
        elif omitted:
            trace_reasons.append("selected paint samples were omitted or remain pending")
        samples = trace_fixture.get("selected_event_receive_to_terminal_write_ns")
        if not isinstance(samples, list):
            trace_reasons.append("selected paint samples are missing or unavailable")
        commit_samples = trace_fixture.get("selected_event_commit_to_terminal_write_ns")
        if not isinstance(commit_samples, list):
            trace_reasons.append("selected commit paint samples are missing or unavailable")
        elif isinstance(samples, list) and len(commit_samples) != len(samples):
            trace_reasons.append("selected commit paint samples do not cover the retained receipt samples")
        if trace_reasons:
            trace_status = "incomplete"
        elif not samples:
            trace_status = "no_samples"
            trace_reasons.append("no selected event paint samples were recorded")
        else:
            trace_status = "diagnostic_only"
            trace_reasons.append("commit timing excludes precommit journal wait and physical display; first-paint correlation, selection coverage, and trace overhead are unverified")
        if accounting_available and integrity["selection_changes"]:
            trace_reasons.append("selection changes bound observation; loss between selections is unmeasured")
    report["event_trace_status"] = trace_status
    report["event_trace_reasons"] = trace_reasons[:8]
    report["selected_event_latency_acceptance"] = "unverified"
    report["operator_stop_validation"] = stop_sample_validation(report)
    trace = report.get("trace", [])
    sample_verdicts = {}
    for name, action in (("input_to_observed_paint", "input_paint"),
                         ("stop_to_observed_acknowledgement", "stop_acknowledgement")):
        samples = [sample for sample in trace if sample["action"] == action]
        valid = all(type(sample.get("sent_ns")) is int and type(sample.get("observed_ns")) is int
                    and 0 <= sample["sent_ns"] <= sample["observed_ns"] for sample in samples)
        values = [(sample["observed_ns"] - sample["sent_ns"]) / 1e6 for sample in samples] if valid else [None]
        sample_verdicts[name] = latency_sample_verdict(values, 100)
    for name, field in (("selected_receipt_to_terminal_write", "selected_event_receive_to_terminal_write_ns"),
                        ("selected_commit_to_terminal_write", "selected_event_commit_to_terminal_write_ns")):
        values = fixture.get(field, []) if isinstance(fixture, dict) else []
        valid = isinstance(values, list) and all(type(value) is int and value >= 0 for value in values)
        sample_verdicts[name] = latency_sample_verdict([value / 1e6 for value in values] if valid else [None], 1000)
    report["latency_sample_verdicts"] = sample_verdicts
    report["latency_sample_scope"] = {
        "coverage": "retained samples only; incomplete coverage cannot establish population p95",
        "event_trace_integrity": trace_status,
        "limits": ["event clocks start at subscription receipt or after durable commit and end after terminal write",
                   "precommit journal work and physical display are not measured",
                   "selection, saved/recovery markers, omissions and first-write correlation limit event coverage",
                   "PTY observation includes reader/emulator cost; tracing cost requires a paired comparison"],
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2) + "\n")
    if fixture is not None and report["workload_status"] == "failed":
        raise AssertionError(
            f"live workload recorded {fixture['failed']} failures and "
            f"{fixture['drain_failed']} drain failures; retained report: {path}"
        )
    if report["operator_stop_validation"]["status"] == "failed":
        raise AssertionError(f"operator Stop proof failed; retained report: {path}")


def measure_sessions(arguments: list[str], live: bool = False) -> None:
    """Measure saved sessions or the public offline live-agent workload in a PTY."""
    import argparse
    import base64
    import fcntl
    import hashlib
    import platform
    import pty
    import select
    import struct
    import termios

    parser = argparse.ArgumentParser(description=measure_sessions.__doc__)
    parser.add_argument("binary", type=lambda value: Path(value).resolve(strict=True))
    parser.add_argument("--state-dir", required=not live, type=lambda value: Path(value).resolve(strict=True))
    parser.add_argument("--session", required=not live)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--samples", type=int, default=20, choices=range(1, 101), metavar="1..100")
    parser.add_argument("--idle-seconds", type=float, default=60)
    parser.add_argument("--slow-reader-ms", type=float, default=100)
    if live:
        parser.add_argument("--helper", required=True, type=lambda value: Path(value).resolve(strict=True))
        parser.add_argument("--fixture-root", required=True, type=lambda value: Path(value).resolve())
        parser.add_argument("--capacity", choices=("fixed", "auto"), default="fixed")
        parser.add_argument("--model-requests", type=int, default=8, choices=range(1, 257), metavar="1..256")
        parser.add_argument("--jobs", type=int, default=4, choices=range(1, 129), metavar="1..128")
        parser.add_argument("--capacity-root", type=lambda value: Path(value).absolute(),
                            help="private shared fixture budget; defaults to FIXTURE_ROOT/capacity")
        parser.add_argument("--agents", type=int, default=100, choices=(100, 500, 1000, 4096))
        parser.add_argument("--seconds", type=int, default=30, choices=range(5, 121), metavar="5..120")
        parser.add_argument("--headless", action="store_true")
        parser.add_argument("--no-event-trace", action="store_true")
        parser.add_argument("--no-stop", action="store_true")
    args = parser.parse_args(arguments)
    if not 0 <= args.idle_seconds <= 300 or not 0 <= args.slow_reader_ms <= 5000:
        parser.error("idle seconds must be 0..300 and slow-reader milliseconds must be 0..5000")
    environment = os.environ.copy()
    for name in tuple(environment):
        if name.startswith(("EJM_CLI_", "EJM_REMOTE_", "ANTHROPIC_", "CLAUDE_")) or name in {
            "OPENAI_API_KEY", "OPENAI_BASE_URL", "CLAUDECODE",
        }:
            environment.pop(name)
    environment["TERM"] = "xterm-256color"
    if live:
        if args.fixture_root.exists():
            parser.error("live measurements require a new fixture root")
        args.fixture_root.mkdir(parents=True)
        capacity_root = args.capacity_root if args.capacity_root is not None else args.fixture_root / "capacity"
        config = args.fixture_root / "config.json"
        config.write_text(json.dumps({"root": str(args.fixture_root), "helper": str(args.helper),
                                      "capacity": args.capacity, "model_requests": args.model_requests,
                                      "jobs": args.jobs, "capacity_root": str(capacity_root),
                                      "agents": args.agents, "ui": not args.headless,
                                      "trace": not args.no_event_trace and not args.headless,
                                      "journal_bytes": 2 * 1024 * 1024 * 1024,
                                      "seconds": args.seconds + 10}))
        environment["EJM_CAPACITY_TERMINAL_CONFIG"] = str(config)
        command = [str(args.binary), "capacity_terminal_fixture", "--ignored", "--nocapture", "--test-threads=1"]
        sessions = []
    else:
        common = [str(args.binary), "--state-dir", str(args.state_dir), "--plain", "--no-color"]
        # The existing public list-all command checks its complete JSON-array
        # contract before timing the selected terminal process.
        sessions = json.loads(subprocess.check_output([*common, "sessions", "--json"], env=environment))
        if len(sessions) < 100 or args.session not in {row["id"] for row in sessions}:
            parser.error("use a retained harness fixture with at least 100 sessions and its first_session_id")
        command = [*common, "--session", args.session]

    def row_needle(session: dict) -> bytes | tuple[bytes, ...]:
        if live:
            # The selected ID also appears in the header and search query.
            # Require a complete picker row after its page request returns.
            return tuple(f"{session['id'][:8]} [{status}] {session['title']}".encode()
                         for status in ("idle", "running", "interrupted"))
        return f"{session['id'][:8]} [idle] {session['title']}".encode()

    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))

    def controlling_terminal() -> None:
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)

    output = bytearray()
    screen = ""
    trace = []
    report = None
    emulator = subprocess.Popen(
        ["node", str(Path(__file__).with_name("terminal_screen.mjs"))],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
    )
    emulator.stdin.write("\n")
    emulator.stdin.flush()
    if not emulator.stdout.readline():
        raise RuntimeError("terminal observer could not start; install the documented site dependencies")
    started = time.perf_counter_ns()
    process = subprocess.Popen(
        command, stdin=slave, stdout=slave, stderr=slave,
        env=environment, preexec_fn=controlling_terminal,  # noqa: PLW1509 - this observer is single threaded.
    )

    def read_pty(timeout: float) -> None:
        nonlocal screen
        if not select.select([master], [], [], timeout)[0]:
            return
        try:
            data = os.read(master, 65536)
        except OSError:
            if process.poll() is not None:
                return
            raise
        start = max(0, len(output) - 3)
        output.extend(data)
        if len(output) > 4 * 1024 * 1024:
            raise AssertionError("terminal measurement exceeded its 4 MiB capture bound")
        queries = output.count(b"\x1b[6n", start)
        if queries:
            os.write(master, b"\x1b[1;1R" * queries)
        emulator.stdin.write(base64.b64encode(data).decode() + "\n")
        emulator.stdin.flush()
        reply = emulator.stdout.readline()
        if not reply:
            raise RuntimeError("terminal observer exited during the measurement")
        screen = "\n".join(json.loads(reply))

    def drain(seconds: float) -> None:
        deadline = time.perf_counter() + seconds
        while time.perf_counter() < deadline:
            read_pty(min(0.01, max(0, deadline - time.perf_counter())))

    def until(needle: bytes | tuple[bytes, ...], begin: int, action: str, present: bool = True) -> None:
        needles = needle if isinstance(needle, tuple) else (needle,)
        expected = [re.sub(rb"\s+", b"", item) for item in needles]
        deadline = time.perf_counter() + 10
        while True:
            visible = re.sub(rb"\s+", b"", screen.encode())
            if any(item in visible for item in expected) == present:
                painted = time.perf_counter_ns()
                trace.append({"action": action, "sent_ns": begin, "observed_ns": painted,
                              "latency_ms": (painted - begin) / 1_000_000})
                return
            if process.poll() is not None or time.perf_counter() >= deadline:
                raise AssertionError(f"terminal measurement did not paint {action}: {bytes(output[-1000:])!r}")
            read_pty(0.001)

    def timed_input(keys: bytes, needle: bytes | tuple[bytes, ...], action: str, delay: float = 0, present: bool = True) -> None:
        drain(0.03)
        begin = time.perf_counter_ns()
        os.write(master, keys)
        if delay:
            time.sleep(delay)
        until(needle, begin, action, present)

    def timed_stop(session: dict, index: int) -> None:
        marker = f"stop-ready-{index:03d}-{session['id'][:8]}"
        timed_input(b"\x15" + marker.encode(), marker.encode(), "stop_command_ready")
        before = screen
        if any(text in before for text in ("Stopping", "cancelled")):
            raise AssertionError("Stop acknowledgement was already visible before the command")
        begin = time.perf_counter_ns()
        os.write(master, b"\x15/stop\r")
        deadline = time.perf_counter() + 10
        while True:
            acknowledgement = fresh_stop_acknowledgement(before, screen, session["title"], marker)
            if acknowledgement is not None:
                observed = time.perf_counter_ns()
                trace.append({"action": "stop_acknowledgement", "sent_ns": begin, "observed_ns": observed,
                              "latency_ms": (observed - begin) / 1e6, "session_id": session["id"],
                              "command": "/stop", "acknowledgement": acknowledgement,
                              "proof": {"before": before, "after": screen, "title": session["title"],
                                        "marker": marker}})
                return
            if process.poll() is not None or time.perf_counter() >= deadline:
                raise AssertionError("terminal did not paint a fresh selected-session Stop acknowledgement")
            read_pty(0.001)

    def tree_snapshot() -> dict:
        # ps is an external observer. Do not put a polling task in the product.
        sampled = time.perf_counter_ns()
        try:
            result = subprocess.run(
                ["ps", "-A", "-o", "pid=,ppid=,rss=,time="],
                text=True, capture_output=True, check=False,
            )
        except OSError as error:
            return {"sampled_ns": sampled, "available": False, "error": str(error)}
        if result.returncode != 0:
            return {"sampled_ns": sampled, "available": False, "error": result.stderr.strip()}
        rows = {}
        for line in result.stdout.splitlines():
            pid, parent, rss, cpu = line.split()
            seconds = 0.0
            for part in cpu.split(":"):
                seconds = seconds * 60 + float(part)
            rows[int(pid)] = {"pid": int(pid), "parent": int(parent), "rss_bytes": int(rss) * 1024,
                              "cpu_seconds": seconds}
        selected = {process.pid}
        while True:
            children = {pid for pid, row in rows.items() if row["parent"] in selected}
            if children <= selected:
                break
            selected.update(children)
        members = [rows[pid] for pid in sorted(selected) if pid in rows]
        return {"sampled_ns": sampled, "available": True, "members": members,
                "rss_bytes": sum(row["rss_bytes"] for row in members),
                "cpu_seconds": sum(row["cpu_seconds"] for row in members)}

    try:
        fixture_work = None
        if live:
            deadline = time.perf_counter() + 120
            ready = args.fixture_root / "ready.json"
            while not ready.exists():
                if process.poll() is not None or time.perf_counter() >= deadline:
                    raise AssertionError(f"live fixture preparation failed: {bytes(output[-2000:])!r}")
                read_pty(0.01)
            data = json.loads(ready.read_text())
            sessions = sorted(data["sessions"], key=lambda row: (row["created_at_ms"], row["id"]), reverse=True)
            args.session = data["first_session_id"]
            if not args.headless:
                until(b"Ctrl+C stop/quit", started, "prepared_fixture_first_paint")
        else:
            until(b"Ctrl+C stop/quit", started, "first_paint")
        drain(0.1)
        initial = tree_snapshot()
        if live:
            workload_started = time.perf_counter()
            (args.fixture_root / "start").touch()
            drain(0.2)
        current = args.session
        for index in range(0 if live and args.headless else args.samples):
            marker = f"draft-{index:03d}-capacity".encode()
            timed_input(b"\x15" + marker, marker, "input_paint")
            timed_input(b"\x13", row_needle(sessions[0]), "picker_open")
            # The second page contains distinct rows. Wait for the returned
            # data, since an initial loading paint alone proves no navigation.
            timed_input(b"\x1b[6~", row_needle(sessions[50]), "picker_next_page")
            query = args.session.encode()
            found = next(row for row in sessions if row["id"] == args.session)
            timed_input(query, row_needle(found), "picker_search")
            # A switch to another session makes the header change observable.
            other = (sessions[(index + 1) % len(sessions)] if live
                     else sessions[1] if sessions[0]["id"] == current else sessions[0])
            timed_input(b"\x1b", row_needle(found), "picker_close", present=False)
            timed_input(b"\x15/resume " + other["id"].encode() + b"\r", other["title"].encode(), "session_switch")
            current = other["id"]
            if live and not args.no_stop:
                timed_stop(other, index)
        if not live or not args.headless:
            if live:
                selected = next(row for row in sessions if row["id"] == args.session)
                timed_input(b"\x15/resume " + args.session.encode() + b"\r", selected["title"].encode(),
                            "selected_session_restore")
            slow_marker = b"slow-reader-capacity"
            timed_input(b"\x15" + slow_marker, slow_marker, "slow_reader_input_paint", args.slow_reader_ms / 1000)
            os.write(master, b"\x15")
        drain(0.2)
        idle_start = tree_snapshot()
        if live:
            drain(max(0, args.seconds - (time.perf_counter() - workload_started)))
            (args.fixture_root / "stop").touch()
        else:
            drain(args.idle_seconds)
        idle_end = tree_snapshot()
        idle_cpu = None
        if (idle_start["available"] and idle_end["available"]
                and {row["pid"] for row in idle_start["members"]} == {row["pid"] for row in idle_end["members"]}):
            elapsed = (idle_end["sampled_ns"] - idle_start["sampled_ns"]) / 1_000_000_000
            if elapsed:
                idle_cpu = 100 * (idle_end["cpu_seconds"] - idle_start["cpu_seconds"]) / elapsed
        if not live:
            os.write(master, b"\x04")
        deadline = time.perf_counter() + 10
        while process.poll() is None and time.perf_counter() < deadline:
            read_pty(0.01)
        if process.poll() != 0:
            raise AssertionError(f"terminal measurement exited with {process.poll()}")
        if live:
            fixture_work = json.loads((args.fixture_root / "result.json").read_text())
        digest = hashlib.sha256()
        with args.binary.open("rb") as binary:
            for chunk in iter(lambda: binary.read(1024 * 1024), b""):
                digest.update(chunk)
        actions = {}
        for action in {sample["action"] for sample in trace}:
            values = sorted(sample["latency_ms"] for sample in trace if sample["action"] == action)
            actions[action] = {"count": len(values), "p50_ms": values[(len(values) - 1) // 2],
                               "p95_ms": values[max(0, (len(values) * 95 + 99) // 100 - 1)]}
        report = {
            "schema": 1, "scope": ("public offline live-agent fixture" if live
                                      else "saved-session terminal fixture; no concurrent managed agents"),
            "platform": {"system": platform.system(), "release": platform.release(), "machine": platform.machine(),
                         "logical_cpus": os.cpu_count()},
            "binary": str(args.binary), "binary_bytes": args.binary.stat().st_size, "binary_sha256": digest.hexdigest(),
            "command": command, "saved_sessions": len(sessions), "trace_clock": "perf_counter_ns",
            "method": "external PTY output through xterm.js screen; 1 ms read wait; no production trace instrumentation",
            "limitations": ["uncontrolled host load",
                            "process snapshots do not establish peak RSS or capture exited descendants", "no I/O measurement",
                            "slow-reader timing includes the intentional reader delay", "observer overhead not isolated"],
            "summary": actions, "trace": trace,
            "requested_samples": args.samples,
            "process_tree": {"initial": initial, "idle_start": idle_start, "idle_end": idle_end,
                             "idle_seconds": args.idle_seconds, "idle_percent_one_core": idle_cpu},
            "terminal_bytes_read": len(output), "remote_api_calls": 0,
        }
        if live:
            helper_digest = hashlib.sha256()
            with args.helper.open("rb") as helper:
                for chunk in iter(lambda: helper.read(1024 * 1024), b""):
                    helper_digest.update(chunk)
            report.update({"agents": args.agents, "headless": args.headless, "fixture": fixture_work,
                           "capacity": args.capacity, "model_requests": args.model_requests,
                           "jobs": args.jobs, "capacity_root": str(capacity_root),
                           "event_trace": not args.no_event_trace and not args.headless,
                           "operator_stop": not args.headless and not args.no_stop,
                           "fixture_root": str(args.fixture_root), "requested_work_seconds": args.seconds,
                           "helper_binary": {"path": str(args.helper), "bytes": args.helper.stat().st_size,
                                             "sha256": helper_digest.hexdigest()}})
            report["limitations"].extend([
                "release-profile libtest composition; not ordinary CLI startup",
                "separate event clocks start after durable commit and at subscription receipt; both end after terminal write; first-write and selection coverage remain unverified",
                "one configured capacity setting per report; repeat runs for an auto-vs-fixed sweep",
                "workload duration excludes session preparation but includes operator-driven cancellation",
                "completion counts and latency end when the fixture controller observes completion; drain counts bound cutoff uncertainty",
            ])
        else:
            report["limitations"].extend(["no active background-agent event flood", "no executing work or Stop timing"])
        write_measurement_report(args.output, report)
        print(json.dumps({"saved_sessions": len(sessions), "summary": actions, "idle_percent_one_core": idle_cpu,
                          **({"workload_status": report["workload_status"]} if live else {}),
                          "output": str(args.output)}))
    except Exception as error:
        if report is None:
            partial = {"schema": 1, "measurement_status": "failed", "measurement_error": str(error),
                       "command": command, "trace_clock": "perf_counter_ns", "trace": trace,
                       "terminal_output_tail": bytes(output[-2000:]).decode("utf-8", errors="replace")}
            write_measurement_report(args.output, partial)
        raise
    finally:
        if process.poll() is None:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait(timeout=5)
        emulator.stdin.close()
        emulator.wait(timeout=5)
        emulator.stdout.close()
        os.close(master)
        os.close(slave)


if __name__ == "__main__":
    if os.name != "posix":
        raise SystemExit("PTY restoration checks require Linux or macOS; run them on a supported host.")
    if len(sys.argv) == 6 and sys.argv[1] == "--child":
        supervise(Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4]), sys.argv[5])
        raise SystemExit(0)
    if len(sys.argv) > 1 and sys.argv[1] == "--measure-saved":
        measure_sessions(sys.argv[2:])
        raise SystemExit(0)
    if len(sys.argv) > 1 and sys.argv[1] == "--measure-live":
        measure_sessions(sys.argv[2:], live=True)
        raise SystemExit(0)
    if len(sys.argv) == 3 and sys.argv[1] == "--remote-only":
        fixture = Path(sys.argv[2]).resolve(strict=True)
        for case in ("flow", "queued-complete", "queued-partial", "cancel", "interrupt", "error", "signal", "panic"):
            check(fixture, terminate=False, remote_case=case)
        print("ejm-cli remote PTY privacy, lifecycle and restoration passed.")
        raise SystemExit(0)
    if len(sys.argv) != 3:
        raise SystemExit("usage: check_terminal.py PATH_TO_EJM_CLI PATH_TO_NATIVE_TEST_BINARY")
    executable = Path(sys.argv[1]).resolve(strict=True)
    fixture = Path(sys.argv[2]).resolve(strict=True)
    check(fixture, terminate=False)
    check(fixture, terminate=True)
    check(executable, terminate=False, unconfigured=True)
    check(fixture, terminate=False, claude_auth=True)
    check(fixture, terminate=True, claude_auth=True)
    for case in ("flow", "queued-complete", "queued-partial", "cancel", "interrupt", "error", "signal", "panic"):
        check(fixture, terminate=False, remote_case=case)
    print("ejm-cli PTY interaction and terminal restoration passed.")
