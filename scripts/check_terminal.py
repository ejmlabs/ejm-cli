#!/usr/bin/env python3
"""Exercise real terminal input and restoration without credentials or model calls."""

from pathlib import Path
import json
import os
import re
import signal
import subprocess
import sys
import tempfile
import time


def supervise(binary: Path, state: str, report: Path) -> None:
    import termios

    # Keep the session leader alive until flags are checked. macOS revokes the
    # terminal when its session leader exits, even with the parent slave open.
    original = termios.tcgetattr(0)
    child = subprocess.Popen([str(binary), "--replay", "--state-dir", state])
    signal.signal(signal.SIGTERM, lambda *_: child.send_signal(signal.SIGTERM))
    returncode = child.wait()
    report.write_text(json.dumps({
        "returncode": returncode,
        "restored": termios.tcgetattr(0) == original,
    }))


def check(binary: Path, terminate: bool) -> None:
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
        report = Path(state) / "terminal-check.json"
        process = subprocess.Popen(
            [sys.executable, str(Path(__file__).resolve()), "--child", str(binary), state, str(report)],
            stdin=slave, stdout=slave, stderr=slave,
            env=environment, preexec_fn=controlling_terminal,
        )

        def read_until(needle: bytes, timeout: float = 10, after: int = 0) -> None:
            # Differential draws can insert cursor moves between visible words.
            expected = re.sub(rb"\s+", b"", needle)

            def contains_text() -> bool:
                visible = re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", output[after:])
                return expected in re.sub(rb"\s+", b"", visible)

            deadline = time.monotonic() + timeout
            while not contains_text():
                if time.monotonic() >= deadline:
                    raise AssertionError(f"terminal did not show {needle!r}; last output {bytes(output[-1000:])!r}")
                if select.select([master], [], [], 0.05)[0]:
                    try:
                        output.extend(os.read(master, 65536))
                    except OSError:
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
                    try:
                        output.extend(os.read(master, 65536))
                    except OSError:
                        pass

        try:
            read_until(b"What shall we build?")
            assert not termios.tcgetattr(slave)[3] & termios.ICANON, "TUI must enter raw mode"
            if terminate:
                process.send_signal(signal.SIGTERM)
            else:
                mark = len(output)
                os.write(master, b"ejm-cli-pty-check\r")
                read_until(b"Demo: ejm-cli-pty-check", after=mark)
                mark = len(output)
                os.write(master, b"/help\r")
                read_until(b"Conversation", after=mark)
                mark = len(output)
                os.write(master, b"\x1b")
                read_until(b"completed", after=mark)
                mark = len(output)
                os.write(master, b"/agents\r")
                read_until(b"choose", after=mark)
                mark = len(output)
                os.write(master, b"\x1b")
                read_until(b"ejm-cli-pty-check", after=mark)
                os.write(master, b"/quit\r")
            wait_for_exit(10)
            while select.select([master], [], [], 0.05)[0]:
                try:
                    data = os.read(master, 65536)
                except OSError:
                    break
                if not data:
                    break
                output.extend(data)
            assert process.returncode == 0, f"terminal supervisor exit status {process.returncode}"
            result = json.loads(report.read_text())
            assert result["returncode"] == 0, f"TUI exit status {result['returncode']}"
            assert result["restored"], "terminal flags were not restored"
            assert b"\x1b[?1049h" in output and b"\x1b[?1049l" in output, "alternate screen was not restored"
            assert b"\x1b[?2004l" in output, "bracketed paste was not disabled"
            assert b"\x1b[?25h" in output, "cursor was not restored"
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGKILL)
                wait_for_exit(5)
            os.close(master)
            os.close(slave)


if __name__ == "__main__":
    if os.name != "posix":
        raise SystemExit("PTY restoration checks require Linux or macOS; run them on a supported host.")
    if len(sys.argv) == 5 and sys.argv[1] == "--child":
        supervise(Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4]))
        raise SystemExit(0)
    if len(sys.argv) != 2:
        raise SystemExit("usage: check_terminal.py PATH_TO_EJM_CLI")
    executable = Path(sys.argv[1]).resolve(strict=True)
    check(executable, terminate=False)
    check(executable, terminate=True)
    print("ejm-cli PTY interaction and terminal restoration passed.")
