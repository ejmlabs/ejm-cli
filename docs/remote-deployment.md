# Relay deployment and launch status

This record separates historical closed uploads from the Worker/UI source now
being integrated on protected CLI parent
`78810826cd376588d3303f13bc237309fb5170a8`. No updated shared-component UI
deployment or authenticated live control acceptance is claimed here. Local
stage checks are recorded separately. See [source publication prerequisites](relay-publication.md).

## Current state

| Area | Recorded state |
| --- | --- |
| Relay origin | `https://relay.ejmlabs.com`, Worker `ejm-cli-relay` |
| Dedicated D1 | `ejm-cli-relay`, ID `e42b7009-c485-4bad-86a7-b14036278ca6`, WNAM |
| Admission | Closed; initial limit two active accounts |
| Authentication | Dedicated GitHub OAuth credentials installed; owner-confirmed real callback reached closed signup |
| Live encrypted control | Authenticated account/device, native/browser pairing, reconnect and revocation remain unverified for the new source |
| Policies | Notice and terms approved for publication, unpublished; effective dates pending |
| Payments/commercial collection | Disabled; no commercial program or buyer exports activated |
| New source | Accepted native PR #21 and inert UI prerequisite PR #22; [current local stages](relay-publication.md#current-local-checks) passed; hosted adoption pending |
| Public docs | Local source only; active site's exact source/assets are unresolved; publication unverified |

The recorded D1 schema comparison covered 15 application tables and 23 explicit
indexes. It was historical schema-only evidence. Secret-installation checks
read no SQL rows and do not refresh that comparison. Inspect migration state
before applying only genuinely pending migrations; never recreate the database
as a retry.

## Historical closed deployments

These are dated receipts for earlier source. They are not rollback endorsements
or proof that current shared UI assets are deployed.

| Date, UTC | Version | Evidence and limits |
| --- | --- | --- |
| October 19, 17:23 | `36c620d3-4921-445f-a46d-319b412f760a` | Owner-installed `GITHUB_CLIENT_ID` and `GITHUB_CLIENT_SECRET` appeared alongside `BETTER_AUTH_SECRET`. Ten anonymous checks and an OAuth-start check passed. Later executable JavaScript matched exact source `3420060`; static assets were not freshly checked. |
| October 19, 18:01 | `cdf2a5c2-16f4-4223-b17e-d12a82a74309` | Callback-recovery tree `b024de8b1d57fa244719f762bd09c755c386644f` passed its historical macOS gate with harness `943aa862`. Ten anonymous and three fixed recovery checks passed. Active executable JavaScript matched; the nonexecutable source map differed. |
| October 19, 18:50 | `e11a58f0-74fa-4601-9212-7a3ae6a3f53a` | Identity correction tree `464d782e50eb9525ed4e2c05b8e2bb6b564d258e` passed its historical macOS gate and supplied deployment `ccc4d9ae-f8ab-47b3-88f5-c8491295d848` at 100 percent traffic. Executable JavaScript matched; source map differed and assets were not freshly checked. Owner later reached the expected closed-signup message. |

The historical identity-correction gate passed 116 native unit tests, six
integration tests, five WASM tests, 98 Worker tests and nine Chromium cases on
macOS arm64. Those counts apply to that named old source/harness, not the new
accepted native graph or current combined Worker source. Earlier build/upload
time and startup figures prove neither production CPU nor free-tier headroom.

No public launch, authenticated session or live control was established by those
receipts. They preserved the custom domain, D1/DO identities, closed admission,
two-account limit, disabled alternate origins, empty policy approval objects and
three auth secret names. No secret rotation, plan upgrade or schema reset was
part of those closed updates.

## OAuth installation checks

The dedicated app uses callback
`https://relay.ejmlabs.com/api/auth/callback/github`. A same-origin OAuth start
confirmed the GitHub target, generated callback, minimal email scope, state,
cookie and PKCE S256 without following a redirect or retaining secrets. This
proved configuration presence and request construction only. The owner's later
closed-signup result established the real callback/client-secret exchange for
that historical version; it did not create an admitted account.

Better Auth owns OAuth, device authorization and sessions. The adapter selects
a verified primary GitHub email and discards provider tokens, names and avatars.
New signup stays disabled when admission is closed, capacity is zero or
operational retention lacks approval. Existing linked identities keep sign-in,
sign-out and privacy access. Temporary OAuth state/rate-limit rows can still
exist while new accounts are closed.

## Closed GitHub identity request update

The reported download of a file named `github` was not reproduced exactly.
Local fixtures exposed empty callback failures. The callback-only recovery
uses Better Auth's supported error URL for expected failures and HTML status
500 for unexpected non-API callback errors, preserving redirects/cookies and
fixed safe messages. Do not log callback URLs, codes, state or downloaded files.

The next owner retry showed the fixed identity error. The old adapter used
`redirect: "error"`, which pinned workerd rejected before either GitHub identity
request. The correction uses supported manual handling, rejects non-success
responses and sends no bearer token to redirect targets. Thirteen focused
fixture cases passed before the historical complete gate and closed update.
The owner then saw: “New relay accounts are not being accepted. Existing
accounts can still sign in.” Retain the safeguard and fixed recovery in the
final source. Do not broaden scopes, replace working credentials or open
admission to diagnose this resolved source defect.

## Final-source deployment and recovery

1. Pass the complete source gate, fresh hosted reviews and protected adoption.
   Bind the selected UI source, installed packages, release source and production
   artifacts. Keep billing/commercial settings off and admission closed.
2. Preserve the existing domain, database, Durable Object namespaces and auth
   secret. Read secret names only; do not reinstall or rotate values to check
   presence. Review schema state and genuinely pending migrations.
3. Build `web/dist`, excluding debug `web/.fixture-dist`, then deploy that same
   checked source. Record the new version, traffic, configuration, hashes and
   exact artifact-comparison limits. A historical executable match is insufficient.
4. Repeat TLS/health, exact-origin, dashboard and anonymous protected-asset
   denials while admission stays closed.
5. Publish approved policy routes through the active docs source/build, verify
   their bytes and effective dates, then install the approved notice/retention
   configuration. Keep payments/commercial programs disabled.
6. Complete the approved operator-account bootstrap. Verify real GitHub/device
   login, both controller roles, host approval, encrypted output, reconnect
   without duplicate mutations, stop and revocation at the actual HTTPS/WSS
   origin before bounded admission.

See [relay operations](../relay/README.md#deployment-and-rollback) for commands
and compatibility checks. A code rollback does not reverse migrations, rights
records or payment state. Preserve secrets, ticket/quota ledgers and retirement
fences. Historical code without the signup guard or stop fences is unsafe while
old epochs remain valid; prefer a compatible closed forward fix. Never reset
D1/DO state to make rollback succeed.

A source-built UI deployment while admission stays closed is a preparation
release. It does not publish policy pages, complete live encrypted control
acceptance or open the service.
