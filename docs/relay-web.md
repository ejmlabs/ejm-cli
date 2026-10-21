# Browser controller

The Worker source provides `/account` for account, device and opaque room
management, and `/control` for one shared CLI session. The browser controller
uses the account used to register the host. An invitation alone grants no relay
access. This complete Worker/UI source is being prepared in CLI PR #23 after
protected PR #22 merge `78810826cd376588d3303f13bc237309fb5170a8` and accepted
native PR #21. PR #22 accepted inert dependency inputs. The Worker's
[earlier local checks](relay-publication.md#current-local-checks) cover native,
Worker, WASM and browser stages and cf1's normal full gate. The
[first corrections](relay-publication.md#worker-review-corrections) passed their
normal local gate at 9336, but both complete hosted reviews found further defects.
The [second corrections](relay-publication.md#second-worker-review-corrections)
passed their normal local gate at 2de6. Its first hosted review found two more
defects and its second review timed out without a report. The
[third corrections](relay-publication.md#third-worker-review-corrections) remain
prepared, with fresh hosted review, protected adoption and deployment pending. See
[publication status](relay-publication.md) and [deployment status](remote-deployment.md).

Controller HTML and every `/control/assets/` file require the verified account
cookie. A signed-out HTML request to `/control` receives a branded sign-in page
at 401. API, JSON and asset requests keep JSON denials. The recovery page loads
no controller code or WASM.

## Trust and pairing

The browser reuses Snow `0.10.0` compiled to WebAssembly, with the native
`Noise_XXpsk3_25519_ChaChaPoly_SHA256` pattern and prologue. It pins the host's
static key before sending Noise message three. The host must then approve the
controller's full SHA-256 fingerprint locally before dispatching requests.

**The relay operator serves browser code and could change it to capture a
pairing secret or plaintext.** The controller states this before pairing and
requires acknowledgment. An independently verified installed native controller
provides a stronger boundary against a malicious operator. Both endpoints,
native builds and updates remain trusted. This work claims no formal crypto audit.

To connect when your operator has opened access:

1. Sign in at `/account` and accept the current service terms.
2. Start sharing the selected session from the [host CLI](remote-control.md#sign-in-and-host).
3. Open `/control`, read the trust statement and paste the private invitation.
4. Compare the full browser fingerprint on the host and approve it locally.

The invitation is cleared from the field when pairing starts. It never enters
an HTTP API or URL. Its PSK, controller identity and Noise state remain in tab
memory. The controller uses no analytics, service worker or persistent pairing
storage. The shared appearance bootstrap can read an existing theme choice;
it writes no new preference. Disconnect clears reachable invitation, prompt,
key and transcript buffers. JavaScript strings and browser copies prevent a
forensic erasure guarantee.

Both endpoints retire the old cipher transport when either peer disconnects.
A lone old peer cannot join a replacement handshake, and late close/error
callbacks from that old pair cannot close its replacement. Reconnect needs fresh
sockets, one-use tickets and a new Noise handshake on both endpoints. Sockets
with legacy attachments close on a Worker upgrade. A Room's account owner and
original expiry persist independently of sockets; disconnect and reconnect do
not release ownership or extend that deadline. Legacy Rooms without ownership
state reject old tickets and connections. Start a fresh host share with a new
invitation and routing ID rather than trying to recover disconnected ownership.
These prepared [review corrections](relay-publication.md#second-worker-review-corrections)
require no D1 migration; corrected-source acceptance remains pending.

For a compatible owned share, reconnect in the same tab retains the controller
identity and mutation sequence while using new Noise ephemeral keys, nonces and
counters. Reload or navigation clears pairing and creates a new identity, which
the old share rejects. Start a new host share and pair its new invitation after
a reload.

## Access and actions

View invitations disable mutation controls. Control invitations permit submit,
steer, queue and cancel. Each action identifies the displayed session, active
run and event cursor. The host checks that authority and keeps its existing
grants and execution policy. The page cannot change providers, credentials,
packages, grants or execution approvals.

Only one action waits for an outcome at a time. A transport failure keeps its
sequence and instruction. Reconnect requests a snapshot and that sequence's
outcome, without resubmitting the mutation. An unknown outcome blocks further
actions: inspect the host before making a new share. Rejected actions restore
their draft when the composer is empty.

A queued acknowledgment is revocable, not execution confirmation. The host
waits for exact-run completion, discards a dependent chain after a local-successor
handoff and rechecks expiry before admission. The browser does not track the
queue's later final result; check the transcript's run-start **Prompt** entry.
See the [native queue contract](remote-control.md#native-controller).

Host stop, expiry, revocation and transport loss disable actions. A lease lasts
at most fifteen minutes. The same tab may reconnect to an active share with a
fresh ticket. Initial enrollment lasts five minutes; a share lasts at most eight
hours. A fresh signed-in dashboard session for the owning account can use
**Stop sharing** without enrolling a browser controller device. It retires the
routing ID through its original expiry, including while no peers are connected.
Browser stop still requires the exact Origin. It grants no stream key, controller
approval or execution authority. This account-level action does not cancel an
already-started run or discard the local remote queue; use local `/remote stop`
to discard queued prompts, then create a fresh share.

## Transport and bounds

Pairing registers a `Browser controller` device through the authenticated
same-origin API and reuses that tab's device reference across reconnects. Each
connection requests a one-use ticket scoped to account, session, device, room
and role. The socket offers `ejm-relay-v1` and `ticket.TICKET`. The Worker
requires the exact Origin and same session cookie, consumes the ticket once
and selects only `ejm-relay-v1`. Tickets never enter URLs, selected protocols,
UI or logs. Native clients retain their Authorization-header transport.

Encrypted product messages use the native 22-byte `EJMR` header and ordered
chunks. Malformed messages, gaps, replay and failed authentication close the channel.

| Bound | Limit |
| --- | --- |
| Encrypted WebSocket frame | 16 KiB |
| Plain product chunk | 16,346 bytes |
| Reassembled message | 1 MiB |
| Prompt | 16 KiB of UTF-8 |
| Replay page | 512 events |
| Rendered transcript | 256 KiB of UTF-8 and 1,024 blocks |
| Browser send buffer | 256 KiB before refusing more work |
| Production WASM | 1,572,864 bytes before HTTP compression |

Prompt validation counts UTF-8 bytes, while the textarea hint counts UTF-16 code
units. Multibyte text can reach the byte limit sooner; an oversized draft stays
editable. Transcript rendering uses text nodes and removes nonprinting controls.
It interprets neither HTML nor terminal escapes. Truncation and replay gaps remain
visible. Recovery never repeats mutations or restores omitted/deleted history.

The Worker uses no-store, same-origin referrers and a self-only script/style
CSP, with `wasm-unsafe-eval` for compilation and only its own secure WebSocket
endpoint. Native privacy forms retain their exact Origin without a Referer
fallback. Cross-origin referrers remain withheld.

## Shared appearance preparation

All visuals use public ejm-ui components, active tokens and generated styles.
Relay owns screen content, state and host adapters. ejm-ui owns the reusable
shell, document, form, action-row and terminal presentation. Production CSS is:

```css
@import "@ejm/ui/styles.css";
@import "@ejm/ui/styles-static.css";
@import "@ejm/ui/styles-terminal.css";
@source "./screens.tsx";
```

The build uses public shell `Header`, `Footer`, `Disclosure`, `SkipLink`, layout
`Page`, `DocumentContent`, `ActionRows`, `Facts`, `SectionHead`, `Text`, standard
buttons/inputs/selects, `NativeCheckbox` and `TerminalWorkspace` exports. It
renders finite fragments at build time. Runtime scripts clone rendered account
rows and transcript entries, filling text and state without React. No private
sibling imports or local visual primitives are used.

The controller follows the native CLI: two compact header lines, a continuous
monospace transcript, one status line and a rounded composer. Connection holds
full fingerprint/session details, expiry, reconnect and disconnect. It opens
while host approval is pending and closes after the first approved snapshot.
Initial pairing keeps the trust statement and required acknowledgment visible;
the composer appears after pairing starts. The account page leads with the
controller, sessions and devices; settings use native disclosures.

Relay artwork comes from public `@ejm/tokens/assets/projects/relay/` exports.
Account/policy headers preserve the outlined logo's proportions. The controller
uses the compact mark and name; native host and transcript speaker remain
`ejm-cli`. Five selected SVGs, two fonts, common CSS and the appearance bootstrap
form the fixed public asset allowlist. Controller application assets stay private.

Protected UI merge `79a1aa8b1075f834717bc0c0299dc2c41a22b966`, tree
`c18c74fd336dbad9515bff8ecf22ca980b889a51`, owns these contracts. CLI PR #22
accepted the source selector and readable inputs. The complete 941-file source,
lifecycle-disabled package installation and production artifacts were checked
locally for this Worker. Shared CSS measured 13,807 bytes gzip, templates 48,345
raw bytes, bootstrap 289 bytes gzip and fonts 34,536 raw bytes, with no framework
runtime chunks. The comparable minified Worker presentation delta is 788 bytes
gzip within 32,768. Local checks do not establish hosted acceptance or deployment.

## Build and verification

Use the tools pinned in `tools.toml`: Rust `1.99.1`, wasm-bindgen CLI `0.2.129`,
Node `26.12.0` and npm `11.20.0`. Check out the selected UI and harness source
as siblings before installing. From the repository root:

```sh
rustup target add --toolchain 1.99.1 wasm32-unknown-unknown
cargo install --locked --version 0.2.129 wasm-bindgen-cli
cd relay
npm ci --ignore-scripts --install-links
npm run build:web
```

`web/build.mjs` builds the shared screens and strict TypeScript, compiles the
browser crate and runs the matching wasm-bindgen CLI. Production output is
`web/dist`; fixture-only debug WASM goes to `web/.fixture-dist`. Only that debug
build allows numeric HTTP loopback. Never deploy fixture output. `--source-only`
is for frontend iteration and is not a release build.

Snow's alloc-only public features use Curve25519, ChaCha20-Poly1305, SHA-2 and
getrandom's maintained browser backend. The selected WASM graph excludes the
harness, native execution, persistence and native TLS. Its source adds no
JavaScript Noise implementation, runtime CDN or alternate cryptography engine.

The complete acceptance entry point is `./scripts/check` from the repository
root. It binds a native executable from that invocation's Cargo artifact record,
then runs native/PTY, WASM, Worker and real Chrome fixtures. Missing or ambiguous
artifacts fail; fixture runners do not select by timestamps or invoke Cargo.
They use ephemeral workerd, actual local D1/DO storage, Better Auth fixture
sessions and a native offline replay host. Fixture files are private and removed
on completion, with no external sign-in, paid API calls or browser recordings.
See [local operations](../relay/README.md#local-checks).

For an offline layout preview, follow
[local visual review](../relay/web/ui/README.md#local-visual-review). It serves
actual built templates with sample data and disabled actions. It loads no
account/controller script, WebSocket or encryption engine. A preview does not
verify sign-in, pairing, execution or release.
