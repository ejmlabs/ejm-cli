#!/usr/bin/env python3
"""Compare independent nextest reports without turning a failure green."""

import json
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from collections import Counter
from collections.abc import Iterable
from pathlib import Path


def outcomes(path: Path) -> dict[tuple[str, str], str]:
    """Return per-test results from a complete nextest JUnit report."""
    # shortcut: Reports over 50 MiB are refused; raise this bound if suites exceed it.
    if path.stat().st_size > 50 * 1024 * 1024:
        raise ValueError("test report exceeds 50 MiB")
    root = ET.parse(path).getroot()
    if root.tag != "testsuites":
        raise ValueError("not a nextest JUnit report")
    results = {}
    for suite in root.findall("testsuite"):
        suite_name = suite.get("name")
        if not suite_name:
            raise ValueError("test suite has no name")
        suite_results = []
        for case in suite.findall("testcase"):
            name = case.get("name")
            if not name or case.get("classname") != suite_name:
                raise ValueError("test case has no matching identity")
            key = (suite_name, name)
            if key in results:
                raise ValueError(f"duplicate test result: {key}")
            states = [child.tag for child in case if child.tag in ("failure", "error", "skipped")]
            if len(states) > 1 or any(child.tag.startswith(("flaky", "rerun")) for child in case):
                raise ValueError(f"ambiguous test result: {key}")
            status = states[0] if states else "passed"
            results[key] = status
            suite_results.append(status)
        check_counts(suite, suite_results)
    if not results:
        raise ValueError("report contains no tests")
    check_counts(root, results.values())
    return results


def check_counts(element: ET.Element, statuses: Iterable[str]) -> None:
    counts = Counter(statuses)
    expected = {
        "tests": sum(counts.values()),
        "failures": counts["failure"],
        "errors": counts["error"],
        "skipped": counts["skipped"],
    }
    for key, value in expected.items():
        # Older nextest omits the skipped count when it emits no skipped cases.
        if key == "skipped" and key not in element.attrib and value == 0:
            continue
        try:
            actual = int(element.attrib[key])
        except (KeyError, ValueError) as error:
            raise ValueError(f"missing or invalid {key} count") from error
        if actual != value:
            raise ValueError(f"{key} count does not match reported tests")


def mixed_tests(first: Path, second: Path) -> list[tuple[str, str]]:
    """Compare results present and comparable in both reports."""
    a, b = outcomes(first), outcomes(second)
    return sorted(key for key in a.keys() & b.keys() if {a[key], b[key]} == {"passed", "failure"})


def inventory(path: Path) -> set[tuple[str, str]]:
    """Read the platform's full nextest test list."""
    data = json.loads(path.read_text(encoding="utf-8"))
    results = set()
    for suite_id, suite in data["rust-suites"].items():
        if suite["binary-id"] != suite_id or suite["status"] != "listed":
            raise ValueError("test inventory contains an unlisted suite")
        for name, case in suite["testcases"].items():
            if case["filter-match"]["status"] != "matches":
                raise ValueError("test inventory contains a filtered test")
            results.add((suite_id, name))
    if not results or len(results) != data["test-count"]:
        raise ValueError("test inventory count does not match listed tests")
    return results


def cargo_complete(log: Path, listing: Path) -> None:
    suites = json.loads(listing.read_text(encoding="utf-8"))["rust-suites"]
    binaries = {str(Path(suite["binary-path"]).resolve()): suite_id for suite_id, suite in suites.items()}
    if len(binaries) != len(suites):
        raise ValueError("test inventory contains duplicate binary paths")
    expected = inventory(listing)
    results = set()
    seen = set()
    active = None
    for line in log.read_text(encoding="utf-8").splitlines():
        header = re.fullmatch(r"\s*Running .+ \((.+)\)", line)
        if header:
            if active is not None:
                raise ValueError("Cargo test binary did not finish")
            active = binaries.get(str(Path(header[1]).resolve()))
            if active is None or active in seen:
                raise ValueError("unexpected or repeated Cargo test binary")
            seen.add(active)
        elif line.lstrip().startswith("Doc-tests "):
            if active is not None:
                raise ValueError("Cargo test binary did not finish")
        elif active is not None:
            case = re.fullmatch(r"test (.+) \.\.\. (ok|FAILED|ignored)", line)
            if case:
                key = (active, case[1])
                if case[2] == "ignored" or key in results:
                    raise ValueError("Cargo test was ignored or repeated")
                results.add(key)
            elif line.startswith("test result: "):
                active = None
    if active is not None or results != expected or seen != set(suites):
        raise ValueError("Cargo test results are missing or differ from the inventory")


def run(command: list[str], log: Path) -> int | None:
    with log.open("w", encoding="utf-8") as stream:
        try:
            process = subprocess.Popen(
                command,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                encoding="utf-8",
                errors="replace",
            )
        except OSError as error:
            print(f"Could not start {command[0]}: {error}", file=stream)
            return None
        assert process.stdout is not None
        for line in process.stdout:
            sys.stdout.write(line)
            sys.stdout.flush()
            stream.write(line)
        return process.wait()


def main() -> int:
    if len(sys.argv) != 2:
        raise SystemExit("usage: python3 scripts/flake_check.py OUTPUT_DIRECTORY")
    output = Path(sys.argv[1]).resolve()
    output.mkdir()
    with (output / "inventory.json").open("w", encoding="utf-8") as listing, (
        output / "inventory.log"
    ).open("w", encoding="utf-8") as log:
        try:
            inventory_code = subprocess.run(
                [
                    "cargo", "nextest", "list", "--locked", "--workspace",
                    "--run-ignored", "all", "--ignore-default-filter",
                    "--message-format", "json",
                    "--user-config-file", "none",
                ],
                stdout=listing,
                stderr=log,
                check=False,
            ).returncode
        except OSError as error:
            print(f"Could not list tests: {error}", file=log)
            inventory_code = None
    baseline = []
    attempts = []
    for number in (1, 2):
        baseline.append(
            run(
                [
                    "cargo",
                    "test",
                    "--locked",
                    "--workspace",
                    "--no-fail-fast",
                    "--color",
                    "never",
                    "--",
                    "--include-ignored",
                ],
                output / f"cargo-test-{number}.log",
            )
        )
        report = output / f"attempt-{number}.xml"
        config = output / f"attempt-{number}.toml"
        config.write_text(
            "[profile.ci.junit]\npath = "
            + json.dumps(str(report))
            + '\nstore-failure-output = false\n',
            encoding="utf-8",
        )
        attempts.append(
            run(
                [
                    "cargo",
                    "nextest",
                    "run",
                    "--locked",
                    "--workspace",
                    "--run-ignored",
                    "all",
                    "--ignore-default-filter",
                    "--no-fail-fast",
                    "--retries",
                    "0",
                    "--color",
                    "never",
                    "--profile",
                    "ci",
                    "--config-file",
                    str(config),
                    "--user-config-file",
                    "none",
                    "--no-tests",
                    "fail",
                ],
                output / f"attempt-{number}.log",
            )
        )
    incomplete = inventory_code != 0 or any(code is None for code in baseline) or any(
        code not in (0, 100) for code in attempts
    )
    mixed = []
    try:
        first = outcomes(output / "attempt-1.xml")
        second = outcomes(output / "attempt-2.xml")
        mixed = sorted(
            key for key in first.keys() & second.keys()
            if {first[key], second[key]} == {"passed", "failure"}
        )
        expected = inventory(output / "inventory.json")
        if any(
            results.keys() != expected
            or any(status in ("skipped", "error") for status in results.values())
            for results in (first, second)
        ):
            raise ValueError("test results are skipped, missing, or differ from the inventory")
        for number, code in enumerate(attempts, 1):
            has_failures = "failure" in outcomes(output / f"attempt-{number}.xml").values()
            if has_failures != (code == 100):
                raise ValueError("nextest exit code does not match its report")
    except (OSError, ET.ParseError, ValueError, KeyError, TypeError) as error:
        print(f"Cannot compare test results: {error}", file=sys.stderr)
        incomplete = True
    for number in (1, 2):
        try:
            cargo_complete(output / f"cargo-test-{number}.log", output / "inventory.json")
        except (OSError, ValueError, KeyError, TypeError) as error:
            print(f"Cannot verify Cargo attempt {number}: {error}", file=sys.stderr)
            incomplete = True
    baseline_differed = all(code is not None for code in baseline) and (baseline[0] == 0) != (
        baseline[1] == 0
    )
    summary = {
        "cargo_test_exit_codes": baseline,
        "inventory_exit_code": inventory_code,
        "cargo_test_inconsistent": baseline_differed,
        "exit_codes": attempts,
        "doctests": "not_applicable",
        "mixed": mixed,
        "incomplete": incomplete,
    }
    (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    for suite, name in mixed:
        print(f"Flaky test: {suite} :: {name}", file=sys.stderr)
    if baseline_differed:
        print(
            "Cargo test suite outcomes differed; inspect cargo-test logs.",
            file=sys.stderr,
        )
    print(f"Test evidence: {output}")
    return int(incomplete or any(code != 0 for code in baseline + attempts))


if __name__ == "__main__":
    sys.exit(main())
