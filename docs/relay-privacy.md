# Relay data choices and rights

This reference describes the prepared Worker source on protected CLI PR #22. Both complete reviews of Worker head cf1 reported defects; the [local corrections and remaining gates](relay-publication.md#worker-review-corrections) are recorded separately from its earlier checks. Protected adoption and live acceptance remain pending. This source activates no commercial program.

The relay requires an operational account. Commercial participation is optional and off by default. Refusing or withdrawing does not change relay access, price or allowance. The account email, GitHub identity, devices, quotas and billing records stay outside the sale dataset.

The [signup and privacy notice](relay-notice.md) and [service terms](relay-service-terms.md) are approved for publication. Publication and their effective date remain pending.

The commercial program remains disabled unless `COMMERCIAL_CONFIG` parses and every launch prerequisite is present. No production buyer, price, demand or revenue is assumed. Tests use named fixture recipients that must never be deployed as real buyers.

## User controls

Open `/privacy` while signed in to inspect choices, withdraw choices, download privacy records or submit a tracked rights request. Eligibility, named-buyer consent and direct profile controls appear only when the commercial program is configured and the account is eligible. A profile form also needs current profile consent. An explicit rejoin form needs an eligible opted-out account and no live GPC signal. Every profile and usage checkbox starts unchecked. Each choice names its buyer, purpose, fields, notice version and buyer retention period. The accepted scope also binds the notice URL, contract evidence and approval interval. Changing a buyer or scope blocks transfer until a fresh applicable choice.

`/do-not-sell-or-share` works without an account or identity check. A valid `Sec-GPC: 1` signal saves an opt-out for the browser and any known account. A first-party, HttpOnly, SameSite cookie preserves the browser preference when the signal disappears. The cookie lasts 400 days and is renewed when the saved preference is applied; production uses a Secure cookie restricted to the relay host. The server-side browser record follows its configured retention period of at least 366 days, while an account opt-out remains durable for the active account. Signing in with that preference also suppresses account sales. Clearing an anonymous browser cookie removes the browser's durable identifier; it does not remove an account opt-out.

The service suppresses new opt-in solicitation for at least 366 days after opt-out. A user may ask to rejoin. A separate confirmation follows review of current notices. Confirmation removes the opt-out, then each buyer and category still needs fresh consent. Active GPC blocks rejoining. A new session or an absent GPC header cannot restore consent.

Unknown age or a self-reported minor blocks collection and transfer. The gate records adult affirmation. It does not ask for a birth date. Relay eligibility follows its separate account policy.

## Commercial fields

| Category | Allowed fields | Source and description |
| --- | --- | --- |
| Direct professional profile | Role, industry, organization-size band, ISO country code | Bounded choices entered by the user; stored with `direct-user` provenance; labeled `self-reported` |
| Optional coarse usage | OS family, CLI major version, monthly connected-hour, traffic and active-day bands | Validated optional device metadata and measured relay forwarding counters; estimates remain separate and labeled |
| Buyer identifier | Random identifier per buyer and account | Restricted mapping supports rights requests; it is personal data |

No names, email, contact lists, employer name, provider profile fields, enrichment data, payment data, device fingerprints, exact session traces, raw IPs or precise locations enter commercial storage or projections. No prompts, commands, output, tools, paths, files, repository names, secrets or ciphertext enter these storage paths. Adding an unknown field fails validation. A provider-derived profile fails even when its value happens to match an allowed direct choice. Collection does not prefill GitHub fields.

Commercial usage collection is limited to the union of fields covered by current approved buyer usage consents. A bounded monthly accumulator retains only the metadata and counters needed for those fields. For example, permission for OS family alone does not authorize traffic, connected-time or day-membership counters in the commercial accumulator. Raw frame counts used to detect an active day are transient; they are not retained as commercial usage. The accumulator holds no per-session trace or application data. Reserved quota is never treated as measured forwarding.

A flush includes its measurement block's start time. A block spanning a usage consent or eligibility change is dropped. Adding a buyer or changing usage permission resets retained usage accumulators, so later consent cannot cover earlier collection. Withdrawal deletes accumulated usage when usage permission changes. Monthly merges include only unexpired rows and preserve the earliest included contribution's expiry. Later samples can therefore expire sooner than their own retention limit; they cannot extend older samples. Expired usage is excluded from account access and buyer exports even before cleanup deletes its row. Exports include only coarse bands and a measurement or estimate label. Idle time that the relay did not observe is not inferred as measured connection time.

All account-linked rows and buyer pseudonyms are treated as personal data. The service makes no anonymity claim.

## Export authorization

The first release offers bounded manual JSON exports. It has no buyer login, unrestricted buyer query or automatic delivery API. Operator account IDs come from `PRIVACY_OPERATOR_ACCOUNT_IDS`; leaving it unset authorizes no operator. Ordinary account credentials cannot prepare or release an export.

1. An operator chooses an approved buyer, purpose, category, allowed fields and at most 128 candidate accounts, and affirms approval.
2. Preparation stores a manifest, candidate IDs and privacy generations. It stores no downloadable data file.
3. Release constructs the current allowed projection in memory and computes its SHA-256 digest. The serialized export is limited to 256 KiB.
4. A synchronous transaction in the single SQLite `PrivacyAuthority` rechecks every candidate generation, current eligibility, accepted scope and buyer approval, then commits the disclosure ledger and manifest state together.
5. The response is returned only after that transaction commits. There is no later download URL or retryable download authority.

This transaction is the release boundary. Withdrawal or deletion committed first blocks the whole release, including one already constructing a projection. If release commits first, later withdrawal cannot recall authorized bytes. It instead triggers the recorded recipient follow-up. A network interruption after commit may mean the operator did not receive the response; the same manifest cannot release again. Prepare a fresh manifest from current eligibility if another attempt is needed.

A restricted ledger records recipient, purpose, category, field set, notice version, affected accounts, digest and release time. Terminal data and payload JSON are absent. A maximum of 512 retained disclosures per account bounds complete recipient tracking; further releases fail until the applicable retention cleanup makes room.

Cohort output requires a canonical full eligible cohort, one profile field, at least 20 subjects and at least 10 subjects in every cell. The entire report is suppressed if a small cell would be exposed. No arbitrary filters or intersections are offered. A persistent overlap registry blocks another release involving any member of a retained cohort, including a changed buyer or field. A cohort release uses the same generation transaction and ledger boundary. These controls do not prove anonymity.

## Account access, correction and deletion

| Route | Function |
| --- | --- |
| `GET /api/privacy` | Current account choices and named notices |
| `GET /api/privacy/access` | Machine-readable privacy record, summaries and recent evidence |
| `GET /api/privacy/evidence?before=TIMESTAMP&id=ID` | Complete evidence export in bounded pages; the response returns the next cursor |
| `POST /api/privacy/profile` | Enter or correct a directly supplied profile |
| `GET /api/privacy/rights` | Current account's tracked requests, deadlines, results and exceptions |
| `POST /api/privacy/rights/request` | Request access, export, correction, deletion or account closure |
| `POST /v1/account/close` | Dashboard closure through the same tracked rights workflow |
| `GET /privacy/rights` | Public form to track a request with its saved private key after account closure |
| `POST /api/privacy/rights/status` | Read one request and an optional numbered operational export part using its ID and private status key in the body |

Authenticated mutations require a same-origin `Origin` header. Every query derives account ownership from the authenticated session. No buyer route can request a broad account export.

A rights request is acknowledged and recorded before returning. Its response includes a random 256-bit private status key once. Save this key to track the request after account closure. Only its SHA-256 hash is stored. The key grants access to that request alone, restores no account or relay authority, and is never placed in a URL. Keep it private. The policy chosen for its jurisdiction sets acknowledgement, response and maximum extension periods. While commercial activation is disabled or a jurisdiction is unknown, the conservative fallback acknowledges immediately and sets a 30-day response deadline with no automatic extension. Unknown jurisdiction still needs operator review. A configured extension requires a notice before the original response deadline and can occur only once. Overdue requests stay visible in account status and the scheduled cleanup report.

Dashboard closure creates the durable rights request, recipient follow-up and operational work before session revocation. Its result page displays the request ID and one-time private status key without automatic navigation. Save both before leaving; signing in again cannot recover the key. The key belongs in neither logs nor URLs.

The authority durably records pending operational tasks. An idempotent D1 queue reaches identity, devices, quota, payment references and account storage. Hourly cleanup retries enqueueing pending tasks if an earlier queue write failed. Closing an account suppresses all transfers, revokes relay authority and closes active rooms before successful acknowledgment. If relay revocation cannot finish, the response reports the pending closure with status 503 and still supplies the private request key; it does not claim closure succeeded. A failed D1 task enqueue does not lose the durable authority task or its tracking key. Profile correction and all consent changes invalidate prepared exports.

A request cannot be marked complete while operational work or recipient follow-up is pending. Each recorded recipient must be confirmed or have an explained exception. Operators cannot silently drop a recipient. Restricted consent, disclosure, rights and statutory payment evidence use their own policies and are never reintroduced into sale projections. Buyer copies, provider logs and backups need recorded contractual or provider follow-up; the service does not promise that every remote copy is technically erased at once.

A live operational hold also applies to direct rights deletion. Held identity remains available for the recorded purpose, with its retained categories and hold expiry reported as explicit exceptions. The hold does not preserve relay access: account, session, device and room authority is still revoked.

### Operational export parts

Tracked access and export requests include all retained device history within
their capture window, including revoked devices. The dashboard's 64-row list is
not a full history export. Hourly work saves at most 64 devices per part and one
part per attempt. Requests with more history need more invocations; monitor the
request deadline and use bounded catch-up if needed.

Export validation uses the registration label rule: up to 64 Unicode code
points from the permitted letters, digits and separators. Supplementary-plane
letters remain intact. This character limit is separate from the existing
48 KiB serialized UTF-8 part limit.

The first claim records a fixed upper `(created_at,id)` watermark. Later parts
continue after their stored cursor in that order, including tied timestamps.
Devices added beyond the watermark belong to a later request. This is a
paginated capture over a window, not a transactionally frozen snapshot of all
operational stores. Other metadata reflects its recorded capture time.

Parts are stored in the existing privacy authority before queue progress can
advance. Retrying a saved part returns its original acknowledgement. A lost
reply, failed queue update or authority eviction cannot justify skipping it or
marking incomplete history complete. Operational state stays pending until the
terminal part is acknowledged; recipient work still has its separate gate.

Use `/privacy/rights` with the saved request ID and status key. Leave the part
field blank for status and the first available part, or enter a zero-based part
number returned by the status response. For JSON clients, POST a body containing
`requestId`, `statusKey` and optional nonnegative integer `part` to
`/api/privacy/rights/status`, with the exact same-origin `Origin` header. Never
put the key in a URL, command argument, log or shared screenshot.

The response retains the request status fields. `operational_result.devices`
contains the selected bounded part. Its `deviceExport` manifest reports saved
part and device counts, capture bounds and whether capture has completed;
`part` and `nextPart` identify paging. Save each part, then retrieve the next
available part until the manifest reports completion. An unfinished manifest
means more scheduled work is needed. `nextPart` may still be in preparation;
only numbers below `partCount` are saved. No missing part is evidence of completion.
Older unpaged results have no `deviceExport` manifest and do not prove full
device history. Request a fresh export for a paged capture.

Each serialized part stays below 48 KiB UTF-8. Total storage and the number of
attempts grow with history. Parts inherit the request's original rights expiry,
which retries do not extend. Pending requests remain protected. Completed
expired parts are inaccessible even before bounded cleanup removes them.
After parts have been stored, recovery must use code that can deliver and clean
them; older single-result versions are not compatible. The additive D1 paging
index requires no account-data rewrite.

## Retention and bounded cleanup

The hourly job uses expiry indexes and deletes at most 128 rows from each privacy category per invocation, plus at most 128 due profile purges and closed-account tombstones. Backlogs are operational work, not a claim of instant deletion. Monitor the oldest due row and run bounded catch-up before a published deadline is missed. Operational quotas and webhook records have separate cleanup in the relay service.

Profile eligibility checks inspect at most 128 subject rows per hourly pass,
including subjects without profiles. A durable primary-key cursor advances past
unchanged eligible subjects, so they cannot block a later profile whose buyer
scope changed. Scan progress and deadline updates commit together and survive
authority eviction. A full scan can require several hourly passes.

Repeated withdrawal keeps or shortens the current profile deletion deadline.
It cannot move that deadline later. Fresh valid consent or an explicit eligible
profile update can establish a new retention baseline under the existing rules.

Export-part cleanup inspects at most 128 indexed candidates before checking
request protection. A durable cursor advances past pending parts without
deleting them, so they cannot cause an unbounded scan or starve later completed
parts. The cursor and deletions commit together.

| Category | Technical rule |
| --- | --- |
| Application contents and ciphertext | Never stored |
| Optional usage accumulators | At most 90 days; monthly rows retain the earliest included contribution's expiry, so newer samples may expire sooner. Usage withdrawal removes the current accumulator immediately. |
| Withdrawn direct profile | Immediately excluded from sale; purge within the configured period, no more than 30 days |
| Prepared manifests | Configured lifetime, at most 24 hours; invalidated immediately by a privacy change |
| Rejoin challenges | 10 minutes |
| Anonymous browser preference | Configured server period of at least 366 days; browser cookie lasts 400 days and is renewed when applied |
| Consent evidence | Category-specific configured period of at least 366 days |
| Disclosure ledger and buyer mappings | Configured justified period; mappings renewed for each retained disclosure |
| Rights evidence | Configured justified period; open requests remain available until resolved |
| Operational export parts | Original request rights expiry; pending requests protected; completed expired parts denied before bounded removal |
| Active-account preference | While the account is active; closing starts the configured restricted evidence retention period |
| Closed-account tombstone | Maximum configured consent/disclosure/rights period, then bounded removal once open rights work ends |
| Completed operational task result | Configured category period; pending work is retained until resolved |

The disabled program uses conservative restricted-record defaults for rights operations. Before commercial activation, explicitly set each retention field and review it against actual obligations. A longer restricted-record policy does not authorize keeping sale data longer. External provider logs, Stripe records, D1/DO backups and restoration effects need their own documented retention and deletion handling.

## Launch configuration and approval

Leave `COMMERCIAL_CONFIG` unset to keep collection for sale and exports disabled. Setting a boolean alone cannot enable the program. The last validated retention policy is saved in the privacy authority, so disabling activation does not silently lengthen the policy for retained records. The parser requires all of these records:

- An operator approval reference and published notice reference.
- An applicability decision and applicable prior risk-assessment evidence. If an assessment is not required, the applicability record must explain the decision; use a recorded decision reference, not an invented completed assessment.
- Rights-operation, recipient-operation and provider-retention runbooks.
- A documented reidentification and overlap assessment. No field declares the output anonymous.
- Jurisdiction policies with evidence and bounded acknowledgement, response and extension days.
- Explicit category-specific retention values.
- At least one real named buyer with purpose, notice URL/version, category fields, retention, contract evidence and a current approval interval.

Unknown keys, invalid fields, missing evidence, unsafe notice URLs, empty buyers or invalid periods fail closed. Approval references must point to reviewed operator records; the code checks presence and scope, not whether an underlying legal assessment is correct. Real approvals are launch inputs outside the local test suite.

Before approving a named buyer, record a contract covering permitted uses, retention, onward transfer, security, rights contacts, correction/deletion follow-up, incident handling, audit evidence and restrictions on reidentification. Record a person who will handle rights requests, the acknowledgement and response clock, recipient notification, exceptions, overdue escalation and restoration from backups.

Before collecting for sale, assess actual jurisdictions and sources. Review CCPA applicability and covered risk assessments, GDPR consent scope where relevant, and whether the actual business fits a data-broker definition. Do not infer exemption from a small launch or infer broker status from this design alone. GitHub API identity data remains excluded from resale. Treat expected buyer demand and revenue as hypotheses until supported by real buyer evidence.

## Local evidence

`npm test -- tests/privacy/privacy.test.ts` exercises the production privacy service inside the local Workers runtime using actual Durable Object SQLite storage. It covers consent scope, excluded fields and provenance, adult/default-off gates, GPC persistence, rejoining, withdrawal/deletion racing release, one-time manifests, rights isolation and deadlines, quality labels, retention indexes and cohort suppression. `npm run check:types` checks the integrated relay types. Local fixtures do not activate real sales, prove a legal assessment or demonstrate a deployed recipient process.

## Shared screens

The shared appearance source keeps privacy forms as native POST forms
that work without JavaScript, including opt-out, withdrawal, rights tracking
and rejoin confirmation. Public Header, Footer, Page, DocumentContent, Disclosure
and NativeCheckbox components now supply the shell, prose, disclosures and native
form controls alongside the existing shared buttons, inputs and selects. Required
checkboxes retain native form and label semantics. NativeCheckbox uses the public
`@ejm/ui/components/NativeCheckbox` import. The common stylesheet compiles
`@ejm/ui/styles.css`, then the opt-in `@ejm/ui/styles-static.css` and
`@ejm/ui/styles-terminal.css`; Relay adds no local visual primitives. The same
build-rendered components, fonts and semantic light/dark tokens serve the account
and controller. Styles,
theme boot and font assets come from the relay origin under its fixed public
allowlist. No inline style or CSP exception is needed.

A normal browser GET to `/privacy` accepting HTML receives a branded sign-in
page with status 401 when signed out. API and JSON fetch authentication errors
remain JSON at 401. Failed native form submissions keep their error status and
show a safe support label in HTML. Private records and exports still require
authentication. Public opt-out and private-key rights tracking remain separate.
Disabled commercial collection shows no eligibility, profile or rejoin form;
rights, withdrawal, opt-out and account closure remain available.

The static primitives are accepted through protected UI PR #11 merge
`79a1aa8b1075f834717bc0c0299dc2c41a22b966`, following historical artwork/source
baseline `24935aa85355ea99f49bdbecc4186a5e95049393`. CLI PR #22 accepted the consumer source pin and readable install context through
protected merge `78810826cd376588d3303f13bc237309fb5170a8`. That inert
prerequisite proves no installation or Worker runtime. Earlier Worker installation
and local gate results remain historical; the corrected source still needs its
own checks, hosted adoption and release. See
[publication prerequisites](relay-publication.md#worker-review-corrections).
React is build-only; the product retains authentication, privacy policy and HTTP behavior.
This presentation change neither activates collection nor changes the
approved notice, terms, retention configuration or their publication state.
