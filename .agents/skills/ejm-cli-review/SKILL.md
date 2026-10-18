---
name: ejm-cli-review
description: Review a ejm-cli patch for correctness, terminal recovery, resource bounds, public API boundaries, and maintainability.
---

# ejm-cli review

Read `AGENTS.md` and [the contributor guide](../../../docs/contributing.md). Establish the requested diff or revision. Review its callers and tests before judging the patch.

Trace one complete user action: parse or type, submit through the public harness, receive events, paint, cancel, and exit. Check failure paths, resumed history, slow consumers, Unicode editing, terminal escapes, denied effects and background sessions.

Look for concrete ways state can become inconsistent. Prefer one owner for each state transition, a small public interface, and traits only at real variable seams. Reject renderer code that starts commands or makes model calls. Check every new allocation and queue for a bound.

Run the focused proof first, then `./scripts/check`. Report actionable findings with severity, affected path and line, a reproducible condition, and the user effect. Keep speculative style advice separate from confirmed defects. Do not claim a successful gate you did not run.
