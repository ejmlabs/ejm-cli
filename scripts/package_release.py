#!/usr/bin/env python3
"""Archive a built ejm-cli binary, record SHA-256 digests and smoke the archive.

Usage: package_release.py TARGET BINARY OUT_DIR
"""
import hashlib
import io
import os
import re
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGETS = {"x86_64-unknown-linux-gnu", "aarch64-apple-darwin"}


def fail(message):
    print(f"package_release: {message}", file=sys.stderr)
    sys.exit(1)


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def workspace_version():
    match = re.search(r'(?m)^version = "([^"]+)"', (ROOT / "Cargo.toml").read_text())
    return match.group(1) if match else fail("no workspace version in Cargo.toml")


def add(archive, name, data, mode):
    info = tarfile.TarInfo(name)
    info.size, info.mode, info.mtime = len(data), mode, 0
    archive.addfile(info, io.BytesIO(data))


def package(target, binary, out_dir):
    version = (ROOT / "VERSION").read_text().strip()
    if version != workspace_version():
        fail("VERSION and the workspace version differ")
    revision = (ROOT / ".github" / "harness-revision").read_text().strip()
    if not re.fullmatch(r"[0-9a-f]{40}", revision):
        fail("harness revision is not a full commit SHA")
    stem = f"ejm-cli-{version}-{target}"
    out_dir.mkdir(parents=True, exist_ok=True)
    archive_path = out_dir / f"{stem}.tar.gz"
    files = [
        (f"{stem}/ejm-cli", binary.read_bytes(), 0o755),
        (f"{stem}/LICENSE", (ROOT / "LICENSE").read_bytes(), 0o644),
        (f"{stem}/THIRD_PARTY_NOTICES.md", (ROOT / "THIRD_PARTY_NOTICES.md").read_bytes(), 0o644),
        (f"{stem}/HARNESS_REVISION", f"{revision}\n".encode(), 0o644),
    ]
    with archive_path.open("wb") as raw, tarfile.open(
        fileobj=raw, mode="w:gz", format=tarfile.PAX_FORMAT, compresslevel=9
    ) as archive:
        for name, data, mode in files:
            add(archive, name, data, mode)
    return version, stem, archive_path


def smoke(version, stem, archive_path):
    with tempfile.TemporaryDirectory() as scratch:
        with tarfile.open(archive_path) as archive:
            archive.extractall(scratch, filter="data")
        binary = Path(scratch) / stem / "ejm-cli"
        home = Path(scratch) / "home"
        home.mkdir()
        env = {"PATH": os.environ.get("PATH", ""), "HOME": str(home), "NO_COLOR": "1"}
        reported = subprocess.run([binary, "--version"], env=env, capture_output=True, text=True, check=False)
        if reported.returncode != 0 or version not in reported.stdout.split():
            fail(f"--version printed {reported.stdout!r}, expected {version}")
        print(reported.stdout.strip())
        replay = subprocess.run(
            [binary, "--replay", "run", "Hello"], env=env, cwd=scratch, capture_output=True, text=True, check=False
        )
        if replay.returncode != 0 or not replay.stdout.strip():
            fail(f"--replay run failed with status {replay.returncode}: {replay.stderr.strip()}")
        print(replay.stdout.strip())


def main():
    if len(sys.argv) != 4 or sys.argv[1] not in TARGETS:
        fail(f"usage: package_release.py {{{'|'.join(sorted(TARGETS))}}} BINARY OUT_DIR")
    out_dir = Path(sys.argv[3])
    version, stem, archive_path = package(sys.argv[1], Path(sys.argv[2]), out_dir)
    smoke(version, stem, archive_path)
    lines = [f"{sha256(path)}  {path.name}\n" for path in sorted(out_dir.iterdir()) if path.name != "SHA256SUMS"]
    (out_dir / "SHA256SUMS").write_text("".join(lines))
    print("".join(lines), end="")


if __name__ == "__main__":
    main()
