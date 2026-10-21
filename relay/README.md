# Relay service

This Worker connects one foreground CLI host to one native or browser controller.
Both endpoints use the same relay account. The host approves the controller
locally; account recovery grants no stream key or session permission.

This complete Worker/UI source is being prepared in CLI PR #23 after protected
PR #22 merge `78810826cd376588d3303f13bc237309fb5170a8` and accepted native PR #21.
PR #22 accepted inert dependency inputs, not installation or runtime. The
[earlier local checks](../docs/relay-publication.md#current-local-checks) cover
native, WASM, Worker and browser stages and cf1's normal full gate. Both complete
hosted reviews rejected that head on five defects. The
[local review corrections](../docs/relay-publication.md#worker-review-corrections)
still need their own checks and protected adoption; updated UI deployment
remains pending. See [publication prerequisites](../docs/relay-publication.md) for exact
receipts and [deployment status](../docs/remote-deployment.md) for historical
closed uploads. Admission stays closed with a two-account limit. Payments,
commercial collection and buyer exports remain disabled. Approved policy pages
remain unpublished, with effective dates pending publication.

## Trust and data

Native endpoints encrypt through Snow's authenticated Noise channel. The Worker receives binary ciphertext. It can observe account and device references, opaque routing IDs, timing, traffic sizes and connection state. It can deny service. It cannot recover the native endpoint keys from its database.

The browser uses Snow compiled to WebAssembly. The operator serves the browser code and could change it to capture browser secrets or plaintext. The controller page states that boundary before accepting an invitation. Use independently verified installed native clients when protection from a malicious operator matters.

D1 holds verified primary email, GitHub account reference, terms, registered devices, expiring authentication state, hashed connection tickets, payment references and rights tasks. Provider OAuth tokens are discarded after identity verification. Session IP addresses and arbitrary user agents are not retained. No application transcript, plaintext, ciphertext, invitation PSK, endpoint identity key, session title, path or application request ID belongs in service storage or diagnostics.

Optional commercial records live in the separate `PrivacyAuthority`. Collection and release remain disabled with `COMMERCIAL_CONFIG="{}"`. Account identity and payment records are not commercial export sources. See [privacy operations](../docs/relay-privacy.md), [browser control](../docs/relay-web.md) and [native use](../docs/remote-control.md).

## Shared screens

Account, device, policy and controller screens compose public `@ejm/ui` shell,
document, form, action-row and terminal components. Active tokens, generated
styles, fonts and Relay artwork come from public `@ejm/*` exports. React renders
finite fragments only during the build. Runtime scripts clone rendered rows
and transcript entries, then fill text/state. No private sibling source or local
visual primitives are used. Authentication, privacy policy, pairing, transport
and run behavior remain product-owned.

Production `relay.css` imports the public base, static-page and terminal style
entries, in that order, followed by its screen-source directive. Only common
CSS, the appearance bootstrap, two fonts and five selected Relay SVGs are public.
Controller HTML, app, protocol and WASM remain account protected. The public
`/account.css` route is a common-stylesheet alias. No framework runs in the
Worker or browser. See [appearance and budgets](../docs/relay-web.md#shared-appearance-preparation)
and [local visual review](web/ui/README.md#local-visual-review).

Protected ejm-ui merge `79a1aa8b1075f834717bc0c0299dc2c41a22b966`, tree
`c18c74fd336dbad9515bff8ecf22ca980b889a51`, supplies the shared contracts.
CLI PR #22 accepted its selector, thirteen metadata bindings and current-owner
source supplement. The complete 941-file source, lifecycle-disabled installed
packages and production artifacts were checked locally for rejected head cf1.
Its successor still needs its own acceptance. Earlier sample previews and mocked
Worker tests do not qualify the complete runtime or deployment. Keep review
fixtures inert and outside package installs.

## Resource policy

| Bound                                          | Default                                                                            |
| ---------------------------------------------- | ---------------------------------------------------------------------------------- |
| Admitted active accounts                       | Maximum 30, including paid accounts; configured launch limit two, admission closed |
| Rooms per account                              | One, with one host and one controller                                              |
| Share lifetime                                 | Eight hours                                                                        |
| Combined incoming traffic per account, UTC day | 50,000 frames and 64 MiB                                                           |
| Maximum binary frame                           | 16 KiB                                                                             |
| Combined burst per room                        | 100 frames per second                                                              |
| Account ticket/reservation operations          | 300 per UTC day                                                                    |
| Reservation block                              | At most 512 frames and 1 MiB                                                       |
| Connection ticket                              | One use, at most 60 seconds                                                        |
| Connection lease                               | At most 15 minutes, ending at the UTC day boundary                                 |
| Active devices per account                     | 16                                                                                 |
| Pending tickets per account                    | 32                                                                                 |

Reservations debit the account before forwarding. A lost or disconnected allocation stays spent. Reconnecting, hibernation, a second device or another routing ID cannot reset that ledger. Counter updates in socket attachments avoid per-frame application SQL. Quota and ticket state use persistent SQLite in the per-account authority.

A lease is checked on every frame. A revoked account or device cannot renew authority. Existing device/logout authority lasts at most its current lease. Account closure and room stop close both peers before acknowledgement.

A disconnected pair retires together. A lone peer that already used the old cipher transport cannot join a replacement handshake. Both endpoints need fresh sockets, one-use tickets and a fresh Noise handshake. Old close/error callbacks affect only their own pair, so they cannot close a replacement pair. Sockets restored from legacy attachments without pair authority close on upgrade and require reconnect. This uses existing socket attachments and needs no database migration. These are prepared [review corrections](../docs/relay-publication.md#worker-review-corrections), with corrected-source acceptance pending.

Dashboard stop retires that routing ID until its original share expiry, including already-redeemed connections delayed until after stop. Account-scoped retries preserve the deadline and leave a newer share intact. The quota authority retains at most 600 retired IDs across the two UTC days an eight-hour share can overlap, removes expired entries through cleanup and fails closed at capacity. Each Room keeps one persistent expiry fence, restored on eviction, with one deadline cleanup alarm and no per-frame SQL. Early alarms preserve the fence; due cleanup removes the idle account linkage. A fresh share remains eligible. Final-source and live acceptance remain pending.

Room objects use `acceptWebSocket`, attachment restoration and bounded handlers. They have no room polling, outbound WebSocket or recurring timer. Slow-reader buffering is bounded by the platform and checked where the runtime exposes `bufferedAmount`; the service keeps no transcript queue. Idle connections can hibernate. Availability still depends on provider quotas and the host process.

Paid frame/byte limits cannot exceed the beta envelope. Missing prices, credentials or approval configuration fail closed. A purchased plan never bypasses admission capacity.

## Local checks

Use Node 26.12.0 and npm 11.20.0. Rust 1.99.1, the `wasm32-unknown-unknown` target and `wasm-bindgen-cli` 0.2.129 build the controller. The repository tool pins and browser guide describe installation.

From the repository root, run the complete gate:

```sh
./scripts/check
```

Its relay phase requires the current native artifact record, installs the exact npm lock, builds the separate loopback-test WebAssembly, checks TypeScript and the generated Better Auth schema, runs real workerd/D1/Durable Object tests, builds the production browser and a Worker dry run, tests external loopback cookie authentication, and runs native and Chrome integration fixtures. The parent `./scripts/check` builds the current native test executable, captures Cargo's artifact record, and also runs every Rust and terminal gate. It supplies `EJM_NATIVE_TEST_ARTIFACTS` to this phase and removes its temporary record afterward. Missing, unrelated or ambiguous artifacts fail before installation. No native runner selects by directory timestamp or starts Cargo.

For a standalone relay phase, capture a fresh locked native build from the repository root, then retain that record for the phase:

```sh
(
  set -eu
  artifact_record="$(mktemp)"
  trap 'rm -f "$artifact_record"' EXIT
  CARGO_BUILD_JOBS=2 cargo test --locked -p ejm-cli --no-run --message-format=json-render-diagnostics > "$artifact_record"
  EJM_NATIVE_TEST_ARTIFACTS="$artifact_record" ./scripts/check --relay
)
```

Run these commands with the exact source and reviewed harness checkout being checked. Do not reuse an artifact record from another source snapshot. The standalone phase does not replace the root native or PTY gates. Keep total Cargo jobs at two or fewer and run no two Cargo gates against the same target directory.

Useful focused commands from `relay/`, after the locked installation:

```sh
npm run build:ui
npm run check:types
npm run check:schema
npm test -- tests/core.test.ts tests/billing.test.ts tests/retention.test.ts
npm test -- tests/privacy
npm run build:web
./node_modules/.bin/wrangler deploy --dry-run --outdir dist
npm run test:fixture
npm run test:native
npm run test:load
npm run test:web
```

The browser tests use installed Google Chrome through Playwright's `chrome` channel. They use an ephemeral loopback Worker, fixture credentials and an offline native host. Production rejects insecure relay origins; only test builds accept strict loopback HTTP. Test-only WebAssembly output stays in `web/.fixture-dist` and is not a deployment input. No provider sign-in or paid API call is needed.

`scripts/local-fixture.ts` creates isolated D1/DO storage through the official Wrangler test harness. It creates a verified test account, accepts fixture terms and registers a device through the real service APIs. It returns credentials only to the test caller, never to diagnostic output. Native fixture input, ready and status files have mode 0600 and are removed after the run. The runners use the same current-Cargo artifact selector as the PTY supervisor, verify the requested fixture in its test inventory, and invoke that exact ignored test without starting Cargo. The native runner requires successful assertions, then checks every local D1 and Durable Object SQL table and captured relay logs for all recorded application markers, invitation, pairing secret and session ID. Operational auth tokens are expected in the account database but must not appear in logs. Its output contains only counts.

## HTTP and WebSocket API

This describes the prepared v1 service, not a public launch. Follow the
[publication gates](../docs/relay-publication.md) and dated
[deployment record](../docs/remote-deployment.md) before using a release.
All paths below are relative to the operator's exact `RELAY_ORIGIN`.

### Authentication and request rules

Use HTTPS for HTTP and WSS for sockets. Browser HTTP requests use the secure
Better Auth session cookie; native HTTP requests use `Authorization: Bearer`
with the session token from device authorization. An Authorization header takes
precedence over cookies. These session credentials are not connection tickets.
Authenticated browser `/v1` mutations require `Origin: RELAY_ORIGIN`. Native
mutations may omit Origin with a valid bearer credential; any supplied Origin
must match. Sign-in and device approve/deny always require the exact Origin;
device code/token requests may omit it. Parsed bodies are limited to 16 KiB;
use only the documented fields.
The auth adapter also accepts form encoding and returns `413/body_too_large`
for an oversized body. Other routes report their route-specific invalid-body
error. Responses are no-store. Never record tokens, cookies, device codes,
callback URLs or invitations in logs, examples, issue reports or shell history.

Better Auth owns the login and device flow under `/api/auth`:

| Method and path | Request | Result and next step |
| --- | --- | --- |
| `GET /api/auth/get-session` | Session credential, if present | Session data or JSON `null`; this alone does not prove relay admission. |
| `POST /api/auth/sign-in/social` | `{"provider":"github"}`; optional `callbackURL` must be `RELAY_ORIGIN/account` | GitHub authorization URL and state cookie. Complete the normal browser flow; do not manufacture or replay callback parameters. |
| `GET /api/auth/callback/github` | GitHub's state/code redirect | Framework redirect to Account or its fixed error landing; an unexpected non-API callback failure renders HTML with status 500. |
| `POST /api/auth/device/code` | `{"client_id":"ejm-cli-relay","scope":"relay"}` | `device_code`, `user_code`, verification URLs, `expires_in` and `interval`. Codes last 600 seconds; the initial poll interval is 5 seconds. |
| `GET /api/auth/device?user_code=USER_CODE` | Code from the user's own CLI; signed-in browser session for review | Status; the owning user also receives `client_id` and `scope`. Review must show `ejm-cli-relay` and `relay` before approval. |
| `POST /api/auth/device/approve` or `/api/auth/device/deny` | Owning browser session and `{"userCode":"USER_CODE"}` | `{"success":true}` after a pending request is claimed/reviewed. Only approve a code the user requested. |
| `POST /api/auth/device/token` | `client_id`, `device_code`, `grant_type:"urn:ietf:params:oauth:grant-type:device_code"` | `access_token`, `token_type:"Bearer"`, `expires_in`, `scope` after approval. Honor `authorization_pending` and `slow_down`; stop on denial, expiry or invalid grant. |
| `POST /api/auth/sign-out` | Session credential; Origin rules above | Framework session logout. Use `/v1/logout` below for registered relay-device revocation. |

The plugin's OAuth errors have their own `error` string and optional description;
relay errors below use `{"error":{"code":"..."}}`. Branch on fixed codes,
not descriptions. New GitHub signup is disabled while admission is closed,
capacity is zero or retention is unapproved; linked existing users retain login.
Temporary OAuth state and rate-limit writes may still occur while signup is closed.

### Account, device and room routes

Except `/health`, these HTTP routes require a verified session. JSON object
fields shown as required must be present; optional metadata is named explicitly.
Service `expiresAt` values below are Unix milliseconds. Room and device path
IDs use 16 to 128 letters, digits, underscores or hyphens. Values are opaque;
do not infer account ownership from an ID.

| Method and path | Request | Successful response essentials |
| --- | --- | --- |
| `GET /health` | No authentication | `{"service":"ejm-cli-relay","protocol":1}`. It needs no session. |
| `GET /v1/account` | Session credential | Top-level `id`, `email`, `termsVersion`, `requiredTermsVersion`, `deviceId`, `account` and `plan`. `account` may be `null` for a linked but unadmitted identity. |
| `POST /v1/account/terms` | `{"version":"CURRENT_TERMS_VERSION"}` | `accepted:true`, `version` and default-off commercial choices. Needs approved retention, open admission and available capacity, including for an unadmitted or stale-terms user. Verify `account.status:"active"` and `account.terms_version === requiredTermsVersion` with a fresh account GET. |
| `GET /v1/devices` | Session credential | `devices`, newest first, at most 64 records; includes revoked devices. |
| `POST /v1/devices` | `label`; optional `osFamily` and `cliMajor` | `id`; status 201 for a new device or 200 for this session's existing device. Current terms are required; at most 16 devices may be active. Labels allow 1 to 64 Unicode letters/digits, spaces, `.`, `_` or `-`; OS is `macos`, `linux`, `windows` or `other`; CLI major is an integer from 0 through 1000. |
| `POST /v1/devices/DEVICE_ID/revoke` | No parsed body; `{}` is sufficient | `revoked:true`, `existingLeaseMaximumSeconds:900`. Deletes that device's session; unknown/already-revoked IDs return `404/device_not_found`. |
| `POST /v1/logout` | No parsed body; `{}` is sufficient | Same revocation fields for this session's relay device; deletes the session. A lost response leaves server revocation unconfirmed. |
| `POST /v1/account/close` | `{"confirm":"close"}` | `requestId` and one-time private `statusKey`; status 200 only after relay revocation completes. Recipient, operational and billing follow-up can remain pending. Closure promises neither immediate deletion nor recall of recipient copies. |
| `GET /v1/rooms` | Session credential | `rooms:[]` or one `{id,expiresAt}` for this account. Only opaque routing metadata is exposed. |
| `POST /v1/rooms/ROOM_ID/tickets` | `role` (`host` or `controller`) and `deviceId` equal to this session's device ID | `ticket`, `expiresAt`, `room`, `role`, `leaseMaximumSeconds:900`. Requires this session's registered device and current terms. The first host ticket creates the share; a controller cannot create it. One active room per account. |
| `POST /v1/rooms/ROOM_ID/stop` | Registered session device; no parsed body, `{}` is sufficient | `{"stopped":true}` after retiring the owned epoch and closing peers. Owned retries preserve its original deadline; they do not stop a different newer share. |

Account closure uses the [tracked rights workflow](../docs/relay-privacy.md#account-access-correction-and-deletion).
If revocation cannot finish, status 503 still returns `requestId` and `statusKey`
with `relayClosureAcknowledged:false` and
`error:{code:"relay_revocation_pending"}`. Those receipt fields stay at the top
level. The dashboard shows the receipt on its result page without automatic
navigation. Save it before leaving; the service stores only the key's hash.
Pending work remains durable after session revocation.

### One-use tickets and WSS

Connect with `GET /v1/rooms/ROOM_ID/host` or `/controller` and a WebSocket upgrade.
A ticket is a secret 43-character base64url value, valid once for at most
60 seconds and scoped to account, session, device, room and peer role.
Native sockets put it in `Authorization: Bearer TICKET`, without offering a
WebSocket subprotocol. Browser sockets require the exact Origin, the same
registered session cookie that issued the ticket, and exactly two offered
subprotocols: `ejm-relay-v1` and `ticket.TICKET`. They must not send Authorization.
A successful browser upgrade is status 101 and selects only `ejm-relay-v1`;
it never echoes the secret protocol. Socket URLs must have no query string.

Ticket redemption reserves allowance before upgrade. An expired/redeemed ticket
or a lost upgrade needs a fresh ticket, not ticket replay. Issuance, redemption
and reservation attempts share the 300-operation UTC-day budget, including
refused attempts. A lease ends within 15 minutes, at UTC midnight or at the
share's original eight-hour expiry, whichever comes first. Disconnect closes
both peers; reconnect uses fresh tickets and a fresh Noise handshake. Send only
nonempty binary frames of at most 16 KiB. See the [native wire contract](../docs/remote-control.md#bounds-and-wire-compatibility)
and [browser bounds](../docs/relay-web.md#transport-and-bounds) for encrypted
framing, pairing, the 16 KiB UTF-8 prompt bound and outcome lookup. Reconnect
must not resubmit an application mutation whose result is unknown.

### Errors and recovery

These are common relay errors, not an exhaustive framework error catalogue.

| Status/code | Recovery |
| --- | --- |
| `421/configured_origin_required`; `403/origin_required`, `origin_not_allowed` or `browser_origin_required` | Correct the URL/Origin and credential transport; do not follow a credential-bearing redirect. |
| `401/sign_in_required`; `401/registered_device_required` or `403/registered_device_required`; `401/registered_browser_required` | Sign in and register the current session's device. A closed account or revoked device needs operator/user recovery, not a new ticket loop. |
| `400/current_terms_required` or `403/current_terms_required`; `503/admission_closed`, `operational_retention_not_approved` or `relay_capacity_exhausted` | Review current terms and operator status. An open, approved admission window is required for terms admission; capacity counts admitted accounts, not GitHub identities. |
| `400/invalid_device_metadata`, `invalid_ticket_scope` or `confirmation_required`; `409/device_limit` | Correct the documented body or revoke an unused device before registering another. |
| `401/ticket_required` or `invalid_ticket`; `403/account_mismatch`, `device_mismatch` or `account_or_device_inactive` | Obtain a fresh correctly scoped ticket only after checking session, terms, device and account. Never substitute another account's ticket. |
| `409/host_room_required`, `one_room_per_account` or `peer_already_connected` | Start the host share first, stop the old share or close the duplicate peer. Do not create a second room to bypass limits. |
| `403/room_retired` or `room_expired`; `403/room_not_owned` | A stopped/expired epoch cannot reconnect. For a new invitation, stop/start sharing locally; an ownership failure does not authorize stopping another account's room. |
| `429/control_quota_exhausted`, `daily_quota_exhausted` or `ticket_limit`; `429/retirement_capacity_exhausted` or `503/retirement_capacity_exhausted` | Wait for the relevant quota reset/expiry and inspect operator status. Avoid retry storms. |
| `426/websocket_required`; `400/url_credentials_not_allowed`; `503/service_temporarily_unavailable` or `quota_authority_unavailable` | Use the documented upgrade with no query, or back off while service recovers. A failed stop/revocation has not been acknowledged. |

Dashboard stop blocks old tickets and delayed redeemed connections through the
original expiry. It ends relay access, not an already-started run or the local
remote queue; use local `/remote stop` to discard queued prompts, then start a
fresh share. Device revoke/logout may leave an existing lease valid for at most
900 seconds; account closure and acknowledged room stop close peers sooner.
See the existing [privacy/rights route table](../docs/relay-privacy.md#account-access-correction-and-deletion)
for data choices, rights requests, tracking and retained exceptions. Payments
and commercial collection remain disabled in the closed configuration.

## Configuration

`wrangler.jsonc` selects `https://relay.ejmlabs.com`, the dedicated database,
closed admission, a two-account limit, disabled billing/commercial programs and
empty approval objects. Keep `workers_dev` and `preview_urls` disabled. Alternate
origins receive 421. Credentials and invitations bind to the exact HTTPS origin.
The approved [notice](../docs/relay-notice.md) and
[terms](../docs/relay-service-terms.md) remain unpublished; their approval alone
does not activate configuration or establish an effective date.

| Setting                                       | Purpose                                                                          |
| --------------------------------------------- | -------------------------------------------------------------------------------- |
| `RELAY_ORIGIN`                                | Exact HTTPS origin, without a path or trailing slash                             |
| `TERMS_VERSION`                               | Published service terms accepted before admission                                |
| `ADMISSION_ENABLED`                           | `true` only after launch inputs are reviewed; otherwise no new GitHub identities |
| `ADMISSION_LIMIT`                             | Integer from 0 through 30                                                        |
| `RETENTION_CONFIG`                            | Approved operational purpose, expiry and exception policy                        |
| `BETTER_AUTH_SECRET`                          | Secret of at least 32 characters                                                 |
| `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`    | Dedicated GitHub OAuth application                                               |
| `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`  | Processor secrets, absent until configured                                       |
| `STRIPE_MONTHLY_PRICE`, `STRIPE_YEARLY_PRICE` | Server-owned recurring price IDs                                                 |
| `PAID_DAILY_FRAMES`, `PAID_DAILY_BYTES`       | Paid allowance, bounded by beta maxima                                           |
| `COMMERCIAL_CONFIG`                           | Separate approved commercial program, default disabled                           |
| `PRIVACY_OPERATOR_ACCOUNT_IDS`                | Explicit accounts allowed to operate rights and releases                         |

`ASSETS` must point to the production `web/dist` build with `run_worker_first: true`. The Worker authenticates `/control` and every `/control/assets/*` request before calling the asset binding. Do not add a separate public bucket or static route for these files. Controller responses have `no-store`, exact-origin connection policy, a self-only script/style policy and the WebAssembly compilation permission. There is no third-party script, font, analytics or service worker.

The dedicated GitHub app requires callback
`https://relay.ejmlabs.com/api/auth/callback/github`. The three auth secret names
were installed and checked historically without reading their values. The owner
confirmed the real closed-signup callback on the old deployed version; that does
not prove an authenticated account or final-source sign-in. Preserve the existing
secret values. Better Auth owns OAuth, device authorization and session cookies.
The adapter requests `user:email`, selects a verified primary email, discards
provider tokens/name/avatar and disables account linking. Shared per-path database
rate limits avoid storing client IPs, while provider logs remain separate.

New GitHub signup is disabled when admission is not `true`, effective capacity
is zero or operational retention is invalid/unapproved. The callback checks
current configuration even if admission closes after sign-in starts. Linked
existing identities retain sign-in, sign-out and privacy access. Temporary OAuth
state and rate-limit records can still be written. Configuring credentials does
not authorize new accounts or terms admission.

Callback failures use Better Auth's supported error URL and fixed safe messages.
Unexpected non-API callback errors receive HTML status 500. Identity fetches use
manual redirect handling and reject all non-success responses without forwarding
credentials. Keep callback codes/state, cookies and provider responses out of
logs and issues. Retry sign-in from Account after an error. See the historical
[identity correction](../docs/remote-deployment.md#closed-github-identity-request-update).

## Operational retention

`RETENTION_CONFIG` is separate from commercial approval. New service admission requires all fields below, with the actual reviewer, purpose and published notice. Day fields must be whole numbers from 1 through 3650. The durations below match the approved beta proposal, but placeholder approval and notice references must be replaced only after publication and configuration review. This is not active configuration:

```json
{
  "approved": true,
  "version": "YOUR-REVIEWED-POLICY",
  "approvedBy": "YOUR-REVIEWER",
  "accountActivePurpose": "Authenticate active relay accounts and enforce service access, billing and user rights.",
  "unadmittedDays": 7,
  "accountClosedDays": 30,
  "providerIdentityDays": 30,
  "paymentDays": 30,
  "externalNotice": "https://YOUR-RELAY/YOUR-PUBLISHED-NOTICE",
  "legalHoldPolicy": "YOUR-REVIEWED-EXCEPTION-PROCESS"
}
```

Active identity remains while needed for the configured service purpose. Unadmitted identities expire under `unadmittedDays`. Closure revokes sessions, devices and tickets, suppresses transfers and queues subscription cancellation. Hourly cleanup purges closed-account devices, provider identity and payment records at their separate deadlines. A pending cancellation retains the minimal payment references needed to retry while ordinary identity deletion continues. A legal hold requires a category, reason, evidence and expiry in `operational_legal_holds`; it conservatively pauses that account's scheduled purge. The same boundary applies to direct rights deletion: held identity remains, its retained categories and hold expiry are reported as explicit exceptions, and relay/session revocation still proceeds. Rights deletion reports these exceptions instead of claiming full erasure. See the [prepared corrections](../docs/relay-publication.md#worker-review-corrections) and [tracked closure workflow](../docs/relay-privacy.md#account-access-correction-and-deletion).

`/terms` links to `externalNotice` only when the full operational policy passes
admission validation. It labels the service terms and operational-policy versions
separately. Invalid/unapproved policy retains the preview warning. The approved
notice/terms body and effective dates stay unchanged until actual publication;
the current empty policy object remains inactive. Validation checks fields and
scope, not notice availability or legal sufficiency. Verify the live notice
before configuring its reference. Displaying a notice does not open admission.

### Hourly maintenance

The configured cron, `17 * * * *`, runs once an hour at minute 17 and budgets operational cleanup, retention and privacy enqueueing together. One populated invocation is bounded to at most 38 parent D1 statements and 11 internal Durable Object calls, with their combined count below 50. Every statement inside a D1 batch counts separately. The published [free D1 ceiling](https://developers.cloudflare.com/d1/platform/limits/) is 50 queries per Worker invocation, leaving at least 12 query slots of headroom. The cron makes no direct external requests. Durable Object SQL and any configured Stripe reconciliation run in their own bounded invocations; the launch keeps Stripe disabled. These bounds do not prove production CPU time or daily row-read/write headroom.

| Work per invocation                                 | Batch | Maximum attempts per day at hourly cadence |
| --------------------------------------------------- | ----: | -----------------------------------------: |
| Account quota cleanup                               |     2 |                                         48 |
| Pending payment cancellation                        |     1 |                                         24 |
| Operational rights request                          |     1 |                                         24 |
| Eligible closed-account retention                   |     1 |                                         24 |
| Rights requests copied from privacy authority to D1 |     2 |                                         48 |

Expiry deletes also retain their per-category row caps: 500 connection tickets; 200 processed billing events and each expired authentication category; 100 unadmitted identities and expired legal holds; 128 completed or exception D1 rights results. Commercial categories have their own approved retention and bounded Durable Object cleanup.

Unattempted work starts oldest first. Failed cancellation attempts rotate behind earlier unattempted records. Failed quota cleanup, rights work and final quota purges remain durable and retry after an hour while later records can proceed. Pending D1 rights results hold only a retry deadline until the privacy authority acknowledges the actual result. A persisted privacy enqueue cursor survives object eviction, visits later pending requests and wraps to retry failed inserts. Cursor movement never marks a rights request complete. A failed cross-store acknowledgement leaves that request pending.

Hourly capacities are attempt limits, not guaranteed completion rates. Each account may have 32 open rights requests, so the initial two-account launch can have 64; one full operational pass needs at least 64 hourly attempts plus enqueue and failure delays. Before raising admission, review the oldest pending rights deadline and retention eligibility, failed cancellations and quota purges, and actual provider CPU/row usage. Resolve provider or authority failures rather than clearing pending markers or deleting state to remove a backlog. Run `npm test -- tests/cleanup.test.ts tests/retention.test.ts` to check the populated combined budget, authority eviction, repeated draining and failed-purge fairness before changing batches or cadence.

Rights exports include operational identity, device and payment references and clearly labelled reserved allowances. They exclude secrets. The operator must still complete and record required recipient follow-up; completing the operational task does not complete the whole rights request.

Publish how Cloudflare, GitHub, Stripe, backups and legally retained records are handled. Provider backups and recipient copies are not erased by deleting a D1 row. The operator must monitor unresolved rights tasks, overdue requests and cancellation failures.

## Payments

Payments are disabled in the closed preview. The following describes the optional implementation and its activation requirements.

Create monthly and yearly recurring prices in the operator's Stripe account and configure the customer portal. The client sends only `monthly` or `yearly`. The server chooses the price and customer mapping. Before checkout, the account authority reads current Stripe subscriptions. Every nonterminal subscription, including past due or incomplete payment, goes to the customer portal. The local free/paid entitlement never authorizes a second subscription. The authority checks a recorded Checkout session even after its local timeout; a completed session goes to the portal, and only Stripe's confirmed expired state or the completed session's exact subscription becoming terminal permits a replacement. Repeated open checkout reuses one session. An uncertain response retries its persisted Stripe idempotency key, and an expired uncertain request is recovered by its provider metadata. Missing or incomplete provider history fails closed for operator reconciliation rather than starting another subscription. A different pending plan is rejected until that Checkout expires. All payment forms stay on Stripe.

Configure the webhook at `/v1/billing/webhook` for subscription, checkout completion, invoice payment, refund and dispute events handled by `src/billing.ts`. Verification uses untouched request bytes and the official Stripe SDK. Durable event IDs deduplicate delivery. The per-account authority serializes reconciliation with quota allocation and fetches current provider state, so delivery order and redirects cannot extend access.

Active recognized subscriptions grant their absolute provider period end. Cancellation at period end retains purchased access until that time. Failed collection, incomplete or expired subscriptions stop paid allocation. A full refund of the current paid invoice or unresolved/lost dispute puts paid allocation on hold; authoritative resolution can remove the hold. Existing leases keep their bounded expiry. Free eligibility remains subject to account status and admission. Refusing data sales changes none of this.

Account closure first revokes the live relay, then expires open subscription Checkout sessions and reads current Stripe subscriptions, even if no completion webhook has arrived. Existing purchased periods cancel at period end; an incomplete first payment is canceled immediately. Unresolved provider requests keep a durable cancellation task pending, and hourly cleanup retries it. If Stripe's bounded history cannot resolve a saved request, the operator must reconcile that customer and request in Stripe before clearing the task; do not delete or replace the saved request identity. Signed financial events can still reconcile a closed account without restoring service access. This code does not issue an automatic refund or promise one. Publish the operator's actual prices, tax, cancellation and refund terms before checkout is enabled.

## Cost model

Official published limits were checked on October 21, 2026 UTC. They are not a check
of this Cloudflare account's plan, current usage or production CPU. Recheck those
before launch and before increasing admission:

| Provider allowance | Published free limit |
| --- | --- |
| [Workers](https://developers.cloudflare.com/workers/platform/limits/) | 100,000 requests/day, 10 ms CPU and 50 subrequests per invocation, 128 MB per isolate |
| [SQLite Durable Objects](https://developers.cloudflare.com/durable-objects/platform/pricing/) | 100,000 request units/day and 13,000 GB-s/day; five million row reads/day, 100,000 row writes/day, 5 GB total storage |
| [D1](https://developers.cloudflare.com/d1/platform/pricing/) | Five million row reads/day, 100,000 row writes/day and 5 GB total storage; [500 MB per free database](https://developers.cloudflare.com/d1/platform/limits/) |

DO incoming WebSocket messages use a 20:1 compute billing ratio; outgoing messages
add no compute request units. Eligible idle hibernating objects avoid idle duration
charges. Authentication, tickets, quota, cleanup, privacy and other apps sharing
the account also consume capacity. Forwarding counts alone cannot establish free
operation. Production CPU, daily row usage and storage still need measurement.

Keep admission closed until release checks pass, then start at two accounts.
Inspect errors, actual duration, row counts, storage and rights/cleanup deadlines
before any increase. Free quota exhaustion can stop service. An application
limit does not guarantee an absolute spending cap on a paid Cloudflare account.
No plan change is included. A paid upgrade needs a separate operator decision
and its base and usage charges must be reviewed together. Optional processor
fees are separate from hosting; consult [Stripe pricing](https://stripe.com/pricing)
if payments are later proposed. No buyer demand, data-sale revenue or free-tier
availability guarantee is assumed.

## Deployment and rollback

Follow [source publication prerequisites](../docs/relay-publication.md) before
using these deployment commands. This integration has not run them or deployed
the updated UI. A closed UI update and opening the beta are separate steps.

1. Pass complete local/hosted source gates, fresh exact-head human decisions and
   protected adoption. Bind source, UI packages and production artifact hashes.
2. Preserve D1 `e42b7009-c485-4bad-86a7-b14036278ca6`, DO namespaces, custom
   domain and existing auth secrets. Keep admission false and alternate URLs off.
   Read secret names only. Apply only reviewed genuinely pending migrations.
3. Build production `web/dist`, excluding `web/.fixture-dist`, and deploy that
   checked source. Record version, traffic, configuration and artifact comparisons,
   including any source-map or asset limits. Never infer a new match from an old one.
4. Check TLS, health, exact Origin, signed-out recovery and protected asset denials.
   Publish the approved policies through the active docs build and verify their
   content/effective dates before configuring admission. Do not overwrite the
   unmatched live docs site with this older static source.
5. Use the approved operator bootstrap window only after policy/configuration
   gates. Check real GitHub/device login, native/browser pairing, both roles,
   reconnect and revocation at HTTPS/WSS. Close the bootstrap window and complete
   live acceptance before opening the two-account beta. Payments/commercial stay off.

From `relay/`, with the operator's existing authenticated Wrangler session:

```sh
./node_modules/.bin/wrangler secret list
./node_modules/.bin/wrangler deploy
./node_modules/.bin/wrangler deployments list
```

Do not print secret values or enable request/header/body capture, session replay,
Logpush or third-party telemetry. Default observability is disabled. Diagnostics
emit only allowlisted safe categories/status/counts. See the
[deployment record](../docs/remote-deployment.md) for historical receipts.

For rollback, close new admission and select a compatible reviewed version with
`wrangler deployments list`, then use `wrangler rollback VERSION_ID` through the
same pinned local executable. Preserve secrets, D1/DO state and ticket/quota
ledgers. Code rollback does not reverse migrations, payment state, rights evidence
or delivered exports. Never rename/delete namespaces or reset state to retry.
Historical versions lacking the signup guard or share-retirement fences are
unsafe while affected epochs remain valid. Prefer a compatible closed forward
fix, or have owners stop affected shares locally before restoring older code.
Stop rooms on transport incompatibility and pair again when keys/epochs change.

## Dependencies

Runtime dependencies are Better Auth 1.7.11, Better Call 1.4.2 and Stripe 23.0.0, all MIT, from their maintained upstream projects. They own authentication protocols, cookie parsing/serialization and payment signatures/API handling. Cloudflare Wrangler 4.151.1, its official Vitest integration 1.4.4, Workers types 5.20261018.1, Vitest 5.0.5, TypeScript 7.0.2 and tsx 4.23.15 are development tools. Playwright 1.64.1 is the browser fixture runner. The fixture also uses Better Call's signed-cookie helper. Their exact graph is in package-lock.json.

The browser reuses native Snow 0.10.0 through wasm-bindgen 0.2.129. It adds no JavaScript Noise implementation, frontend framework or runtime CDN. The maintained packages add bundle weight in exchange for established authentication, signing and test behavior. `npm ci --ignore-scripts --install-links`, schema drift checks, the browser download budget and Worker dry-run size make that cost reviewable. An npm audit result is a point-in-time advisory check, not a security guarantee. Native crypto/license review is recorded with the shared Rust change.

The shared screen build uses the existing UI owner's Vite 8.3.4, Tailwind 4.3.3
and React 19.3.1 versions as development-only tools. `npm run build:ui` compiles
all templates and common assets into ignored `web/.ui/`. Run it before standalone
type, schema or unit checks that import the Worker. The normal repository gate
builds the fixture first; `npm run check` also builds the shared module before
its existing checks. `web/build.mjs` copies the same generated common assets
and controller HTML into its separate production or fixture directory. The
normal npm install keeps lifecycle scripts disabled and uses `--install-links`
to materialize public `file:` packages. See [UI build notices](UI_BUILD_NOTICES.md).
