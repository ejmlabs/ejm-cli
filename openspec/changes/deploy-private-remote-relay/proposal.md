# Proposal

## Why

The relay and browser controller pass local checks but have no hosted CI result or Cloudflare deployment. The user has requested both, with free-tier hosting where possible.

## What Changes

- Publish reviewable CLI and dependency revisions through existing protected workflows, repair the scoped shared CI prerequisite hook, and record hosted results for exact revisions. Combine the bounded bootstrap, terminal and command foundations in one prelude that retains the current reviewed harness pin and callers; adopt the hook before the new harness API pin and its native consumers.
- Configure one Cloudflare Worker for the relay and protected browser UI, a dedicated D1 database, SQLite Durable Objects, the exact HTTPS origin, and safe production assets.
- Keep payments and commercial collection disabled. Keep account admission closed until dedicated OAuth credentials and reviewed operational notices are configured, then start with two accounts.
- Bound scheduled database work to the provider's free per-invocation limits and retain retryable work across scheduled invocations.
- Verify live TLS, authentication, protected assets, device approval, pairing, reconnect and revocation. Publish setup, limits, recovery and actual release evidence in the affected docs.

## Capabilities

### New Capabilities

None. This change releases the implemented relay rather than adding another remote-control product.

### Modified Capabilities

- `cloudflare-relay`: Make the bounded free-launch contract explicit for per-invocation database work and conservative production admission.

## Impact

Changes affect relay cleanup, deployment configuration, CI setup, dependency revision pins, release evidence, canonical relay documentation and the public docs sources. The shared workflow needs its own reviewed central change. The later native CLI layer depends on new private harness APIs whose protected publication is authorized. The prelude retains the current reviewed harness source; adopt the new API pin only after the central hook supplies its Linux native prerequisites. Existing maintainer readiness, approval, branch protection and paid-review gates remain in force. No repository visibility change, Cloudflare plan upgrade, payment activation or commercial activation is included. Concurrent site, cross-platform and model-selection work must be preserved.
