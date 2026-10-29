import json
import os
import select
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path


def report(cases: list[tuple[str, str]]) -> str:
    counts = (
        f'tests="{len(cases)}" failures="{sum("<failure" in state for _, state in cases)}" '
        f'errors="{sum("<error" in state for _, state in cases)}" '
        f'skipped="{sum("<skipped" in state for _, state in cases)}"'
    )
    return (
        f'<testsuites {counts}><testsuite name="pkg::suite" {counts}>'
        + "".join(
            f'<testcase classname="pkg::suite" name="{name}">{state}</testcase>'
            for name, state in cases
        )
        + "</testsuite></testsuites>"
    )


class FlakeCheckTests(unittest.TestCase):
    def test_hung_attempt_preserves_incremental_diagnostics(self) -> None:
        for phase in ("cargo-1", "nextest-1", "cargo-2", "nextest-2"):
            with self.subTest(phase=phase), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                cargo = root / "cargo"
                cargo.write_text(
                    '''#!/usr/bin/env python3
import os, sys, time, tomllib
from pathlib import Path
if sys.argv[1:3] == ["nextest", "list"]:
    print("{}")
    raise SystemExit(0)
kind = "nextest" if sys.argv[1] == "nextest" else "cargo"
counter = Path(kind + "-count")
number = int(counter.read_text()) + 1 if counter.exists() else 1
counter.write_text(str(number))
phase = f"{kind}-{number}"
if kind == "nextest":
    config = Path(sys.argv[sys.argv.index("--config-file") + 1])
    report = Path(tomllib.loads(config.read_text())["profile"]["ci"]["junit"]["path"])
    report.write_text(os.environ["FAILURE_REPORT"])
print("diagnostic " + phase, end="", flush=True)
if phase == os.environ["HANG_PHASE"]:
    time.sleep(60)
raise SystemExit(100 if kind == "nextest" else 101)
''', encoding="utf-8"
                )
                cargo.chmod(0o755)
                env = {**os.environ, "PATH": str(root) + os.pathsep + os.environ.get("PATH", ""),
                       "HANG_PHASE": phase, "FAILURE_REPORT": report([("x", "<failure/>")])}
                process = subprocess.Popen(
                    [sys.executable, str(Path(__file__).with_name("flake_check.py")), str(root / "evidence")],
                    cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                    start_new_session=True,
                )
                try:
                    output = b""
                    deadline = time.monotonic() + 10
                    while f"diagnostic {phase}".encode() not in output:
                        remaining = deadline - time.monotonic()
                        self.assertGreater(remaining, 0, "detector did not reach hanging attempt")
                        ready, _, _ = select.select([process.stdout], [], [], remaining)
                        self.assertTrue(ready, "detector did not emit diagnostic")
                        chunk = os.read(process.stdout.fileno(), 8192)
                        self.assertTrue(chunk, "detector exited before hanging attempt")
                        output += chunk
                finally:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=10)
                    process.stdout.close()
                kind, number = phase.split("-")
                log = root / "evidence" / (f"cargo-test-{number}.log" if kind == "cargo" else f"attempt-{number}.log")
                self.assertIn(f"diagnostic {phase}", log.read_text())
                if phase in ("cargo-2", "nextest-2"):
                    self.assertEqual((root / "evidence/attempt-1.xml").read_text(), env["FAILURE_REPORT"])
                    self.assertIn("diagnostic nextest-1", (root / "evidence/attempt-1.log").read_text())

    def test_cargo_success_without_finishing_all_tests_is_incomplete(self) -> None:
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parent.parent) as directory:
            root = Path(directory)
            (root / "src").mkdir()
            (root / "Cargo.toml").write_text(
                '[package]\nname = "fixture"\nversion = "0.1.0"\nedition = "2021"\n[workspace]\n',
                encoding="utf-8",
            )
            (root / "src/lib.rs").write_text(
                '#[test] fn a_exit() { std::process::exit(0); }\n'
                '#[test] #[ignore] fn z_never_runs() {}\n', encoding="utf-8"
            )
            real_cargo = shutil.which("cargo")
            self.assertIsNotNone(real_cargo)
            subprocess.run([real_cargo, "generate-lockfile"], cwd=root, check=True, capture_output=True)
            cargo = root / "cargo"
            cargo.write_text(
                '''#!/usr/bin/env python3
import json, os, subprocess, sys, tomllib
from pathlib import Path
if sys.argv[1:3] == ["nextest", "list"]:
    build = subprocess.run([os.environ["REAL_CARGO"], "test", "--locked", "--no-run", "--message-format=json"], capture_output=True, text=True, check=True)
    artifact = next(json.loads(line) for line in build.stdout.splitlines() if json.loads(line).get("profile", {}).get("test") and json.loads(line).get("executable"))
    suite = {"binary-id": "fixture", "binary-path": artifact["executable"], "status": "listed", "testcases": {name: {"filter-match": {"status": "matches"}} for name in ("a_exit", "z_never_runs")}}
    print(json.dumps({"test-count": 2, "rust-suites": {"fixture": suite}}))
elif sys.argv[1:3] == ["nextest", "run"]:
    config = Path(sys.argv[sys.argv.index("--config-file") + 1])
    output = Path(tomllib.loads(config.read_text())["profile"]["ci"]["junit"]["path"])
    attrs = " tests='2' failures='0' errors='0' skipped='0'"
    output.write_text("<testsuites" + attrs + "><testsuite name='fixture'" + attrs + "><testcase classname='fixture' name='a_exit'/><testcase classname='fixture' name='z_never_runs'/></testsuite></testsuites>")
else:
    os.execv(os.environ["REAL_CARGO"], [os.environ["REAL_CARGO"], *sys.argv[1:]])
''', encoding="utf-8"
            )
            cargo.chmod(0o755)
            env = {**os.environ, "PATH": str(root) + os.pathsep + os.environ.get("PATH", ""),
                   "REAL_CARGO": real_cargo, "RUST_TEST_THREADS": "1", "CARGO_BUILD_JOBS": "2"}
            result = subprocess.run(
                [sys.executable, str(Path(__file__).with_name("flake_check.py")), str(root / "evidence")],
                cwd=root, env=env, capture_output=True, text=True,
            )
            summary = json.loads((root / "evidence/summary.json").read_text())
            self.assertEqual(summary["cargo_test_exit_codes"], [0, 0])
            self.assertTrue(summary["incomplete"])
            self.assertEqual(result.returncode, 1)
            (root / "src/lib.rs").write_text(
                '#[test] fn a_exit() {}\n#[test] #[ignore] fn z_never_runs() {}\n', encoding="utf-8"
            )
            result = subprocess.run(
                [sys.executable, str(Path(__file__).with_name("flake_check.py")), str(root / "complete")],
                cwd=root, env=env, capture_output=True, text=True,
            )
            summary = json.loads((root / "complete/summary.json").read_text())
            self.assertFalse(summary["incomplete"], result.stderr)
            self.assertEqual(result.returncode, 0)

    def test_inventory_and_comparison_through_command(self) -> None:
        cases = {
            "complete": ([('x', ''), ('y', '')], [('x', ''), ('y', '')], False, []),
            "reverse": ([('x', ''), ('y', '')], [('x', '<failure/>'), ('y', '')], False, [['pkg::suite', 'x']]),
            "both_fail": ([('x', '<failure/>'), ('y', '')], [('x', ''), ('y', '<failure/>')], False, [['pkg::suite', 'x'], ['pkg::suite', 'y']]),
            "captured_output": ([('x', '<failure><system-out>test evil ... ok</system-out></failure>'), ('y', '')], [('x', ''), ('y', '')], False, [['pkg::suite', 'x']]),
            "skipped": ([('x', '<failure/>'), ('y', '<skipped/>')], [('x', ''), ('y', '<skipped/>')], True, [['pkg::suite', 'x']]),
            "error": ([('x', '<error/>'), ('y', '')], [('x', ''), ('y', '')], True, []),
            "omitted": ([('x', '<failure/>')], [('x', '')], True, [['pkg::suite', 'x']]),
            "missing_second": ([('x', '<failure/>'), ('y', '')], [('y', '')], True, []),
            "duplicate": ([('x', ''), ('y', '')], [('x', ''), ('x', '<failure/>')], True, []),
            "bad_counts": ([('x', ''), ('y', '')], [('x', ''), ('y', '')], True, []),
            "malformed": ([('x', ''), ('y', '')], [('x', ''), ('y', '')], True, []),
            "old_counts": ([('x', ''), ('y', '')], [('x', ''), ('y', '')], False, []),
            "old_skipped": ([('x', ''), ('y', '')], [('x', '<skipped/>'), ('y', '')], True, []),
            "invalid": ([('x', '<failure/>'), ('y', '')], [('x', ''), ('y', '')], True, [['pkg::suite', 'x']]),
            "cargo_omitted": ([('x', '<failure/>'), ('y', '')], [('x', ''), ('y', '')], True, [['pkg::suite', 'x']]),
            "cargo_wrong_name": ([('x', ''), ('y', '')], [('x', ''), ('y', '')], True, []),
        }
        for name, (first, second, incomplete, mixed) in cases.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                cargo = root / "cargo"
                cargo.write_text(
                    '''#!/usr/bin/env python3
import json, os, sys, tomllib
from pathlib import Path
if sys.argv[1:3] == ["nextest", "list"]:
    print(os.environ["LISTING"])
elif sys.argv[1:3] == ["nextest", "run"]:
    marker = Path("ran")
    first = not marker.exists()
    marker.touch()
    config = Path(sys.argv[sys.argv.index("--config-file") + 1])
    output = Path(tomllib.loads(config.read_text())["profile"]["ci"]["junit"]["path"])
    result = os.environ["FIRST" if first else "SECOND"]
    output.write_text(result)
    raise SystemExit(100 if "<failure" in result else 0)
else:
    print("     Running unittests src/lib.rs (fake)")
    print("running 2 tests\\ntest x ... ok")
    if not os.environ.get("OMIT_CARGO"):
        print("test " + ("z" if os.environ.get("WRONG_CARGO") else "y") + " ... ok\\ntest result: ok.")
''', encoding="utf-8"
                )
                cargo.chmod(0o755)
                listing = {
                    "test-count": 3 if name == "invalid" else 2,
                    "rust-suites": {"pkg::suite": {
                        "binary-id": "pkg::suite", "binary-path": "fake", "status": "listed",
                        "testcases": {item: {"filter-match": {"status": "matches"}} for item in ("x", "y")},
                    }},
                }
                first_report, second_report = report(first), report(second)
                if name == "bad_counts":
                    second_report = second_report.replace('tests="2"', 'tests="3"', 1)
                elif name == "malformed":
                    second_report = "<testsuites>"
                elif name in ("old_counts", "old_skipped"):
                    first_report = first_report.replace(' skipped="0"', '')
                    second_report = second_report.replace(' skipped="0"', '').replace(' skipped="1"', '')
                env = {
                    **os.environ, "PATH": str(root) + os.pathsep + os.environ.get("PATH", ""),
                    "LISTING": json.dumps(listing), "FIRST": first_report, "SECOND": second_report,
                    "OMIT_CARGO": "1" if name == "cargo_omitted" else "",
                    "WRONG_CARGO": "1" if name == "cargo_wrong_name" else "",
                }
                result = subprocess.run(
                    [sys.executable, str(Path(__file__).with_name("flake_check.py")), str(root / "evidence")],
                    cwd=root, env=env, capture_output=True, text=True,
                )
                summary = json.loads((root / "evidence/summary.json").read_text())
                self.assertEqual(summary["incomplete"], incomplete)
                self.assertEqual(summary["mixed"], mixed)
                self.assertEqual(result.returncode, 0 if name in ("complete", "old_counts") else 1)

    def test_second_run_does_not_hide_the_initial_failure(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cargo = root / "cargo"
            cargo.write_text(
                """#!/usr/bin/env python3
import json, sys, tomllib
from pathlib import Path
if sys.argv[1:3] == ["nextest", "list"]:
    print(json.dumps({"test-count": 1, "rust-suites": {"pkg::suite": {"binary-id": "pkg::suite", "binary-path": "fake", "status": "listed", "testcases": {"write": {"filter-match": {"status": "matches"}}}}}}))
    raise SystemExit(0)
if sys.argv[1] == "nextest":
    marker = Path("ran")
    first = not marker.exists()
    marker.touch()
    config = Path(sys.argv[sys.argv.index("--config-file") + 1])
    output = Path(tomllib.loads(config.read_text())["profile"]["ci"]["junit"]["path"])
    state = "<failure/>" if first else ""
    count = "1" if first else "0"
    attrs = " tests='1' failures='" + count + "' errors='0' skipped='0'"
    output.write_text("<testsuites" + attrs + "><testsuite name='pkg::suite'" + attrs
                      + "><testcase classname='pkg::suite' name='write'>" + state
                      + "</testcase></testsuite></testsuites>")
    raise SystemExit(100 if first else 0)
print("     Running unittests src/lib.rs (fake)")
print("running 1 test\\ntest write ... ok\\ntest result: ok.")
raise SystemExit(0)
""",
                encoding="utf-8",
            )
            cargo.chmod(0o755)
            env = {
                **os.environ,
                "PATH": str(root) + os.pathsep + os.environ.get("PATH", ""),
            }
            script = Path(__file__).with_name("flake_check.py")
            result = subprocess.run(
                [sys.executable, str(script), str(root / "evidence")],
                cwd=root,
                env=env,
                capture_output=True,
                text=True,
            )
            summary = json.loads((root / "evidence/summary.json").read_text())
            self.assertEqual(result.returncode, 1)
            self.assertEqual(summary["cargo_test_exit_codes"], [0, 0])
            self.assertEqual(summary["exit_codes"], [100, 0])
            self.assertEqual(summary["doctests"], "not_applicable")
            self.assertFalse(summary["incomplete"])
            self.assertEqual(summary["mixed"], [["pkg::suite", "write"]])

    def test_failed_cargo_gate_stays_failed_when_nextest_passes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cargo = root / "cargo"
            cargo.write_text(
                """#!/usr/bin/env python3
import json, sys, tomllib
from pathlib import Path
if sys.argv[1:3] == ["nextest", "list"]:
    print(json.dumps({"test-count": 1, "rust-suites": {"pkg::suite": {"binary-id": "pkg::suite", "binary-path": "fake", "status": "listed", "testcases": {"write": {"filter-match": {"status": "matches"}}}}}}))
    raise SystemExit(0)
if sys.argv[1] == "nextest":
    config = Path(sys.argv[sys.argv.index("--config-file") + 1])
    output = Path(tomllib.loads(config.read_text())["profile"]["ci"]["junit"]["path"])
    attrs = " tests='1' failures='0' errors='0' skipped='0'"
    output.write_text("<testsuites" + attrs + "><testsuite name='pkg::suite'" + attrs
                      + "><testcase classname='pkg::suite' name='write'/></testsuite></testsuites>")
    raise SystemExit(0)
marker = Path("cargo-ran")
if not marker.exists():
    marker.touch()
    print("shared-process test failed")
    raise SystemExit(101)
print("     Running unittests src/lib.rs (fake)")
print("running 1 test\\ntest write ... ok\\ntest result: ok.")
""",
                encoding="utf-8",
            )
            cargo.chmod(0o755)
            env = {
                **os.environ,
                "PATH": str(root) + os.pathsep + os.environ.get("PATH", ""),
            }
            result = subprocess.run(
                [
                    sys.executable,
                    str(Path(__file__).with_name("flake_check.py")),
                    str(root / "evidence"),
                ],
                cwd=root,
                env=env,
                capture_output=True,
                text=True,
            )
            summary = json.loads((root / "evidence/summary.json").read_text())
            self.assertEqual(result.returncode, 1)
            self.assertEqual(summary["cargo_test_exit_codes"], [101, 0])
            self.assertTrue(summary["cargo_test_inconsistent"])
            self.assertEqual(summary["exit_codes"], [0, 0])
            self.assertEqual(summary["mixed"], [])
            self.assertIn(
                "shared-process test failed",
                (root / "evidence/cargo-test-1.log").read_text(),
            )

    def test_missing_reports_and_build_failures_are_incomplete(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cargo = root / "cargo"
            cargo.write_text(
                "#!/bin/sh\n"
                'if [ "$1" = nextest ]; then echo "test fake ... ok"; exit 101; fi\n'
                "exit 0\n",
                encoding="utf-8",
            )
            cargo.chmod(0o755)
            env = {
                **os.environ,
                "PATH": str(root) + os.pathsep + os.environ.get("PATH", ""),
            }
            result = subprocess.run(
                [
                    sys.executable,
                    str(Path(__file__).with_name("flake_check.py")),
                    str(root / "evidence"),
                ],
                cwd=root,
                env=env,
                capture_output=True,
                text=True,
            )
            summary = json.loads((root / "evidence/summary.json").read_text())
            self.assertNotEqual(result.returncode, 0)
            self.assertTrue(summary["incomplete"])
            self.assertEqual(summary["mixed"], [])


if __name__ == "__main__":
    unittest.main()
