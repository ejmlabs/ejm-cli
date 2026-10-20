# Remote control

This guide describes native candidate source based on protected CLI PR #20 merge
`9e3f8ef11dc083c31b651b9dae15a98e086e3359`. Exact-source normal and hosted
acceptance must precede adoption; binary distribution and live acceptance remain
pending. Use this workflow only after building the reviewed native source.

Remote access shares one selected CLI session through an account-gated relay.
Local use needs no relay account. Native preparation began on protected CLI
`c8e2239392f71df82cbef6166b1827c8e5f5f1c5`. This candidate selects
harness merge `42e4f9cf33bd3525aec344cfdffabc251271f782` and its exact
358-file completion API tree `826e0231...`. Earlier lock and gate results do not
accept this revision. A compatible account service is required;
no relay or browser UI is included. See [candidate status](native-remote-publication.md)
for the blocked reviews and required fresh gates. The workflow below applies
after those prerequisites pass.

## Sign in and host

Set your operator's HTTPS origin. There is no built-in relay endpoint:

```sh
export EJM_CLI_RELAY_URL=https://YOUR-RELAY
ejm-cli remote migrate-credentials
ejm-cli remote login
ejm-cli --replay remote host
```

Before first persistent use, stop older host/controller processes and run the
preparation command, even on a fresh installation. Confirm locally only after
those writers have stopped. It preserves any legacy identity or creates a new
one when none exists. See the recovery steps below.

On Linux and macOS, Ctrl-C or SIGTERM cancels remote commands and releases
their protected terminal mode. Signal ownership begins before memory-only host
sign-in and initial sharing. The [corrected local macOS gate](native-remote-publication.md)
covered the production loop, startup PTY and restoration. The final native
source after the protected docs/UI prerequisite still needs its normal gate,
hosted platform acceptance and release checks.

Sign in through GitHub, approve the device code you requested, and accept the
relay service terms. The relay requests the verified primary email with the
GitHub email scope. Provider and package-registry accounts remain separate.
The host and controller must use the same relay account.

Hosting grants **view access by default**. To grant prompt submission,
steering, queueing and cancellation:

```sh
ejm-cli --replay remote host --control
```

Replay is an offline echo provider. Omit `--replay` and configure a model
to use a real provider. The host's existing file grants, approvals and tool
policy apply to all remotely requested work.

The host shows a private invitation on its alternate screen. It stays out of
arguments, composer history and durable events. Transfer it over a private
channel. First enrollment expires after five minutes; the share lasts at most
eight hours. A previously enrolled controller can restart with its same saved
identity until share expiry. The host rejects new or replacement keys after the
enrollment deadline. Compare the current controller fingerprint on both endpoints
and press a fresh Y after it is displayed; buffered approval from an earlier
panel cannot approve it. Account recovery never replaces this confirmation.

In a normal local TUI, after endpoint preparation and a saved relay login, use:

| Action | Command |
| --- | --- |
| Share read-only | `/remote share` or `/remote share view` |
| Grant control | `/remote share control` |
| Inspect sharing | `/remote status` |
| Revoke sharing | `/remote stop` |

The sharing badge remains visible after the invitation panel closes.
Switching the selected session or exiting the host stops sharing. Stopping
discards prompts still in the remote queue. A run that already started
continues locally unless the host cancels it.

## Native controller

```sh
ejm-cli remote migrate-credentials
ejm-cli remote login
ejm-cli remote connect
```

Paste the invitation only into the protected prompt. The controller prints its
public fingerprint for comparison. It never opens a model provider, file
tools or a local harness journal.

With control access, enter a prompt to start a turn. During a turn, ordinary
Enter queues a prompt. `/steer TEXT` or Ctrl+K steers that exact run.
`/stop` or Ctrl+C cancels that run. View access permits no mutation.
Use `/status` to refresh retained events and check a pending result.
Use `/quit` to leave.

Queued prompts stay in the host until the exact target succeeds and its
bounded completion observers return. A durable finished event alone does not
release them. The host polls the retained completion without blocking terminal
input, then rechecks the share, expiry and current target before admission.
Local `/stop` also cancels retained run cleanup and its queued local successors;
remote stop remains bound to the controller's exact run. If detailed completion reports a
local queued successor, the host discards the dependent remote chain even if
that successor has already finished. Stopping or replacing the share
during that wait discards undispatched prompts. Text above 16,384 UTF-8 bytes
is rejected before acknowledgement and leaves valid pending prompts intact.

Every action names the expected session, run and event cursor. A run-state
change invalidates an older view, including an intervening completed turn.
Output-only progress keeps controls valid for the same exact run. A future
cursor fails. The controller retains a failed draft; refresh and inspect the
current session before choosing a new action.
Snapshot run authority covers every event through its delivered cursor. The host
checks current share expiry after waiting for authority and journal refresh;
receiving an action before expiry does not extend its permission.

Reconnect uses a fresh encrypted handshake, restores retained events and asks
for pending results. It never repeats a mutation. An unknown result can mean an
action ran but its cached result was evicted. Inspect the output before making
another request. After retries end, the terminal keeps the draft until you quit.

## Credentials and recovery

Native relay tokens and endpoint identity keys use the public harness encrypted
vault at the state directory's `auth` root. macOS Keychain, Windows Credential
Manager or an unlocked Linux Secret Service protects its key. The harness owns
native access controls, locking, authenticated encryption and atomic writes.
Linux needs the distribution's libdbus, a session D-Bus and an unlocked Secret
Service. A locked, denied or corrupt store fails closed. Cancellation or caller
drop cancels a vault lock wait through the public harness API; it cannot interrupt
an OS key-store call already in progress. Restore the original native key and
vault together if key access is lost; do not generate a replacement key for an
existing encrypted vault. There is no plaintext fallback.
Without native key access, use an explicit temporary sign-in and identity in the
foreground process:

```sh
ejm-cli --memory-only --replay remote host --control
ejm-cli --memory-only remote connect
```

Each command signs in for that process. A standalone memory-only login ends
when its process exits. Saved metadata contains only relay origin, account ID
and device ID. One local state directory caches one relay account at a time.
Each HTTPS origin has a separate credential namespace, with distinct
`device-session` and `endpoint-identity` purposes under `ejm-cli-relay`. Provider
and registry credentials cannot be used for relay access. Account/device
metadata is bound inside the encrypted token record as well as the separate
non-secret cache. Mismatched metadata requires sign-in again.

### Prepare or upgrade persistent relay credentials

Run this preparation once per relay origin before persistent host or controller
use, including on a fresh installation. Normal endpoint loading reads only the
vault and never probes individual legacy OS entries.

1. Stop every older host and controller process that could write this origin's
   keyring entries. The new vault lock does not exclude those older clients.
2. Run `ejm-cli remote migrate-credentials` and confirm locally that the older
   writers have stopped. Declining leaves both stores unchanged.
3. Keep the existing state directory, including `relay-account.json`, until
   migration finishes. Import retains the origin, account ID, device ID and
   endpoint fingerprint. If no legacy or destination identity exists, this step
   creates one. Conflicting destinations fail without legacy deletion.
4. The CLI commits and reloads the destination and migration markers before
   deleting legacy `relay-account-v1/device-session` and
   `identity/native-noise-v1` entries. If cleanup fails, keep using the vault and
   retry migration while older writers remain stopped.

Logout removes the token and retains its migration marker, so leftover legacy
tokens cannot be imported again. A missing marked endpoint key requires
recovery, rather than silent identity replacement. Direct downgrade to an older
writer is unsupported. If older writers cannot be stopped, defer the upgrade or
explicitly choose a new memory-only foreground session. Memory-only mode never
imports or saves credentials and creates a fresh endpoint identity each time.

```sh
ejm-cli remote accounts
ejm-cli remote devices
ejm-cli remote devices --revoke DEVICE_ID
ejm-cli remote logout
```

Logout requests server revocation and removes the local credential. A network
failure is reported as unconfirmed revocation. Use your operator's account page,
when available, to revoke it when the service returns. Active relay leases last
at most fifteen minutes;
explicit room stop closes them sooner. Recovering an account grants relay
admission only. It cannot recover a native private key or existing share key.

## Encryption and trust

Native endpoints use `Noise_XXpsk3_25519_ChaChaPoly_SHA256` through Snow 0.10.
The invitation binds protocol version, relay origin, random routing ID, role,
host public key, pairing deadline and a random 256-bit PSK. The controller
pins the host key, and the host confirms and pins the first controller.
Reconnect retains that pin while creating fresh ephemeral channel keys.
Tampered, replayed, downgraded or wrong-key frames close the channel.

The relay and Cloudflare terminate HTTPS but forward only handshake bytes and
application ciphertext. They can observe account identity, coarse devices,
addresses visible to the network, timing, size, routing IDs and connection
activity. They can deny service and see a host public identity through an
active Noise handshake. They cannot recover honest native endpoint stream
plaintext from the forwarding protocol.

A compromised endpoint or altered client can read its own plaintext. The
native installation and update channel must be trusted. Earlier local contract
tests are recorded separately; they do not accept this candidate.
Snow states that it has not received a formal audit. Reachable
invitation/plaintext buffers are cleared where practical; Snow does not promise
forensic erasure of all internal key state.

## Bounds and wire compatibility

The wire protocol is version 1. Unsupported versions fail closed. Peers must
use compatible versions. The native client sends each one-use connection ticket
in a sensitive Authorization header. Neither tickets nor invitations belong in
URLs or logs.

Limits include 16 KiB ciphertext frames, 1 MiB message assembly, 512 events per
replay page, 16 KiB UTF-8 prompts, eight queued remote prompts and a 128-result
mutation cache. The host's mutation high-water mark survives cache eviction
for the share. Every encrypted snapshot supplies that mark so a returning
controller starts above it. Action numbers are bounded to the JavaScript
safe-integer range. Old request IDs cannot execute again. Oversized output produces
an explicit gap; the view remains bounded. Event cursors recover retained
output, not omitted or deleted history.

If the native display falls behind or receives a server event gap, it pauses
newer live batches and recovers an authoritative snapshot and retained journal
output. A gap alone never establishes Idle. Each full replay page waits for the
display cursor before the next read. A short or late snapshot cannot resume live
output while a newer suppressed sequence remains unrecovered. Control results
still arrive through the bounded channel; recovery never resubmits a mutation.
Final-source local and hosted checks remain required.

## Relay prerequisites and evidence

The service must implement compatible account/device authorization, one-use
header tickets, room admission, stop and revocation routes, and bounded
WebSocket forwarding. Host and controller use the same relay account. Native
account requests verify HTTPS certificates and refuse redirects. Native
transport uses WSS with Rustls chain and hostname checks against native roots.
An empty loaded trust store fails closed. Local `SSL_CERT_FILE` and
`SSL_CERT_DIR` override platform roots; see [TLS dependencies](remote-dependencies.md#package-boundaries).
This root loader does not use the full OS platform verifier.
The loopback HTTP exception is compiled only for local tests.

Relay operation, browser delivery, account capacity, daily quotas, payment and
commercial policies belong to the separate Worker/browser publication layer.
This source preparation does not deploy that service or activate data collection.
Free hosting and production latency have not been measured for this candidate.
See [native dependencies](remote-dependencies.md) and
[publication gates](native-remote-publication.md). There is no claim that a live
service, public binary or updated docs site is available.
