#!/usr/bin/env python3
"""Check ejm-cli package boundaries and project skill routing with the standard library."""

from pathlib import Path
import re
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
ALLOWED_DEPENDENCIES = {
    "anyhow", "clap", "crossterm", "ejm-harness-auth", "ejm-harness-coding", "ejm-harness-exec", "ejm-harness",
    "ratatui", "serde_json", "tokio", "tokio-util", "unicode-width",
    # Host-only sign-in URL validation and OS browser launch.
    "url", "open",
    # Maintained HTTP parsing and bounded loopback callback responses.
    "hyper", "hyper-util", "http-body-util",
    # Alacritty's ANSI parser with a fixed OSC buffer and no default features.
    "vte",
    # Owned terminal parsing, bounded buffer cleanup and safe native input modes.
    "termwiz", "zeroize", "rustix", "crossterm_winapi", "winsafe",
    # Reusable package installation/runtime ports and validated, portable manifests.
    "ejm-harness-packages", "ejm-harness-package-spec", "serde",
}
REQUIRED_SKILLS = {
    "ejm-cli-change", "ejm-cli-review", "ejm-cli-terminal", "ejm-cli-theme", "ejm-cli-auth",
    "ejm-cli-packages", "ejm-cli-performance", "ejm-cli-testing", "ejm-cli-dependencies",
    "ejm-cli-harness", "ejm-cli-docs", "ejm-cli-site", "ejm-cli-release",
}
OPENSPEC_SKILLS = {
    "openspec-explore", "openspec-propose", "openspec-update-change",
    "openspec-apply-change", "openspec-sync-specs", "openspec-archive-change",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def command_version(command: list[str]) -> str:
    try:
        output = subprocess.check_output(command, text=True)
    except (OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"Could not run {' '.join(command)}: {error}. See docs/contributing.md.") from error
    for word in output.split():
        if re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", word):
            return word
    raise SystemExit(f"Could not read a version from {' '.join(command)}: {output.strip()!r}.")


def check_toolchain() -> None:
    tools = tomllib.loads((ROOT / "tools.toml").read_text())
    expected = tools["rust"]["version"]
    pinned = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]
    require(pinned["channel"] == expected, "Keep rust-toolchain.toml and tools.toml in sync.")
    require(set(tools["rust"]["components"]) <= set(pinned["components"]), "Pin the required Rust components.")
    actual = command_version(["rustc", "--version"])
    require(actual == expected, f"Rust {expected} is required; found {actual}. See docs/contributing.md.")
    minimum = tuple(map(int, tools["python"]["minimum"].split(".")))
    require(sys.version_info >= minimum, f"Python {tools['python']['minimum']} or later is required.")
    for name in ("cargo-deny", "cargo-machete"):
        version = command_version(["cargo", name.removeprefix("cargo-"), "--version"])
        require(version == tools[name]["version"], f"Install {name} {tools[name]['version']}; found {version}.")


def check_packages() -> None:
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text())
    require(workspace["workspace"]["members"] == ["crates/ejm-cli"], "ejm-cli must remain a separate workspace.")
    manifest = tomllib.loads((ROOT / "crates/ejm-cli/Cargo.toml").read_text())
    require(manifest["lints"]["workspace"] is True, "The CLI must inherit workspace lints.")
    extra = set(manifest["dependencies"]) - ALLOWED_DEPENDENCIES
    for target in manifest.get("target", {}).values():
        extra.update(set(target.get("dependencies", {})) - ALLOWED_DEPENDENCIES)
    require(not extra, f"Review new dependencies and update this boundary check with a reason: {sorted(extra)}")
    require(manifest["dependencies"]["vte"]["default-features"] is False, "Keep VTE's fixed OSC buffer by disabling default features.")
    for dependency in ("ejm-harness", "ejm-harness-coding", "ejm-harness-exec", "ejm-harness-auth", "ejm-harness-packages", "ejm-harness-package-spec"):
        configured = workspace["workspace"]["dependencies"][dependency]
        require(configured["path"].startswith("../ejm-harness/"), f"{dependency} must come from the independent sibling harness project.")
    require((ROOT / "Cargo.lock").is_file(), "Commit Cargo.lock and use --locked.")
    for path in (ROOT / "crates/ejm-cli/src").glob("*.rs"):
        source = path.read_text().split("#[cfg(test)]", 1)[0]
        require(not re.search(r"\bCommand::new\s*\(", source), f"{path}: arbitrary process execution belongs in ejm-harness.")
        require(not re.search(r"(?:std|tokio)::process::(?:Command|Child)\b", source) or path.name == "browser.rs", f"{path}: only the browser host may supervise an OS URL launcher.")
        require("impl Provider for" not in source and "impl Tool for" not in source, f"{path}: providers and tools belong in ejm-harness.")
        require("open::" not in source or path.name == "browser.rs", f"{path}: only the browser host may open the browser.")
        require(not re.search(r"#\s*!\s*\[\s*(?:allow|expect)\b", source), f"{path}: production lint exceptions must be scoped to an item with a reason.")


def check_skills() -> None:
    skill_root = ROOT / ".agents/skills"
    skills = {path.parent.name for path in skill_root.glob("*/SKILL.md")}
    require(skills == REQUIRED_SKILLS | OPENSPEC_SKILLS, "Keep the skill routing inventory in sync with .agents/skills.")
    routing = (ROOT / "AGENTS.md").read_text()
    for name in sorted(skills):
        require(f"`{name}`" in routing, f"AGENTS.md must route to the {name} skill.")
        path = skill_root / name / "SKILL.md"
        text = path.read_text()
        require(text.startswith("---\n"), f"{path}: missing frontmatter")
        parts = text.split("---", 2)
        require(len(parts) == 3, f"{path}: unclosed frontmatter")
        fields = dict(re.findall(r"^(name|description):\s*(.+)$", parts[1], re.MULTILINE))
        require(fields.get("name") == name, f"{path}: name must match its folder")
        require(bool(fields.get("description")), f"{path}: add a routing description")
        require("TODO" not in text and "[insert" not in text.lower(), f"{path}: remove unfinished scaffold text")
        for target in re.findall(r"\[[^\]]+\]\(([^)]+)\)", parts[2]):
            if "://" not in target and not target.startswith("#"):
                require((path.parent / target.split("#", 1)[0]).exists(), f"{path}: broken reference {target}")


if __name__ == "__main__":
    check_toolchain()
    check_packages()
    check_skills()
    print("ejm-cli architecture and skill checks passed.")
