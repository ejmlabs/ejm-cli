# Relay signup and privacy notice

**Version:** `relay-beta-2026-10-19`. **Status:** approved for publication; unpublished. **Effective date:** pending actual publication.

## Operator and service

The operator is **ejmlabs, llc**, California, United States. Contact **support@ejmlabs.com** for service help and **privacy@ejmlabs.com** for privacy requests.

The relay connects your CLI host to your paired native or browser controller. GitHub sign-in creates an authentication identity; accepting the current [service terms](relay-service-terms.md) separately requests relay admission. Capacity can be full. New identities are rejected while admission is closed or operational retention is unapproved. Existing linked identities can still sign in, sign out and use privacy controls. Temporary OAuth state and rate-limit records can still be written.

## Information and encryption

The service uses your GitHub account ID and verified primary email to identify you. It keeps internal account IDs, terms versions and dates, device IDs and labels, authentication sessions, approval state, routing IDs, connection timing, traffic sizes, quota reservations and privacy records. Optional OS-family and CLI-major-version fields describe registered devices. Reservations can exceed delivered traffic.

GitHub profile and email responses and an access token are used during sign-in. The app does not persist GitHub access, refresh or ID tokens, profile names or avatars. Relay session credentials are separate. The app omits client IP and user-agent values from authentication storage; providers still receive connection information.

Paired endpoints encrypt session traffic. The relay forwards ciphertext and does not persist session payloads or endpoint stream keys. The operator can inspect operational records. **The operator serves browser-controller code and could change it to capture a pairing secret or plaintext. An installed native controller provides a stronger boundary against a malicious operator.** Keep invitations private and approve only devices you control. Endpoint compromise, local history, tools and model-provider handling remain separate risks.

The first beta has no payments, buyer exports or collection of optional profile/usage data for sale. Account creation and terms acceptance do not consent to sales. Necessary operational counters remain. The browser controller has no third-party analytics script. Configured Worker observability is off; this does not mean providers keep no logs.

## Retention for this beta

| Record | Rule |
| --- | --- |
| Active identity | Keep while the account is active to authenticate accounts/devices, route encrypted connections, enforce service access and limits, preserve preferences and handle requests. |
| Identity not admitted to the relay | Eligible for deletion seven days after creation. |
| Closed-account devices | Eligible 30 days after closure. |
| Closed-account GitHub identity and email | Eligible 30 days after closure; a deletion request may remove these earlier. |
| Payment references | Eligible 30 days after closure, while payments remain disabled. Any future payment launch needs a separate policy review. |
| Final operational account ID | Eligible after the longest closed-account period, subject to unresolved cancellation, holds and successful quota-authority purge. |

Expiry makes records eligible for bounded hourly cleanup, not deletion at an exact minute. Backlog or failed work delays removal. Sign-in sessions last seven days and can renew with activity; device approvals expire after ten minutes, and connection tickets after 60 seconds. Quota cleanup keeps the current and two prior UTC days.

Separate restricted records have longer implemented defaults for a fresh privacy authority: **rights records and attached access/export results, consent evidence and disclosure evidence: 2,190 days; server-side anonymous preferences: 730 days; browser preference cookie: at most 400 days; operational queue results: eligible 30 days after request creation once completed or marked exception**. Pending tasks survive that expiry. Rights-record expiry is measured from the request date, and unfinished requests survive expiry; active-account opt-outs persist. A previously saved validated privacy policy takes precedence over fresh defaults. The operational periods above do not shorten these rules.

An access/export result can copy your email, provider ID, device and accounting records into the restricted rights record. Removing their original operational rows does not remove that copy before its separate expiry. A documented operational hold pauses that account's scheduled purge. The approved process records its reason, evidence and expiry, reviews it daily and removes it when no longer needed. Pending cancellation keeps references needed for retry. Provider copies and backups follow separate processes; closure does not prove immediate erasure everywhere.

## Privacy choices and requests

Use `/privacy` while signed in for preferences, downloads and tracked access/export, correction, deletion or closure requests. Tracked access/export includes operational records beyond the immediate privacy download. The application acknowledges immediately and records a 30-day response deadline while the commercial program is off; operator follow-up is required. This is not a promise that automation completes every request or legal duty.

Save the private status key returned once. `/privacy/rights` uses it to track that request after account closure. Do not send the key, session credentials, invitations or export contents by email. For follow-up, send only your request ID to privacy@ejmlabs.com. Closing revokes relay access. Deletion can also stop access and may retain stated exceptions. Provider-owned identity corrections need a new verified sign-in or operator review.

`/do-not-sell-or-share` works without an account. Global Privacy Control saves a browser opt-out and applies it to a known account. Clearing cookies can lose an anonymous preference identifier; account opt-outs persist. Refusal does not change price, allowance or eligibility. Any future commercial program needs separate review, named-recipient disclosures and choices.

## Hosting and identity providers

**Cloudflare:** Its [privacy policy](https://www.cloudflare.com/privacypolicy/) describes processing IP addresses, routing information and traffic-derived security data. It operates globally and may transfer or access information across countries. Retention depends on purpose and legal obligations, rather than one fixed duration. This notice promises neither processing only in the United States nor a fixed provider deletion deadline.

**Cloudflare's customer DPA:** The [published addendum](https://www.cloudflare.com/cloudflare-customer-dpa/) describes processing instructions, assistance and deletion or return when services end, with legal exceptions. Its publication does not establish which contractual terms apply to this operator's account.

**GitHub:** GitHub handles your identity and sign-in under its [privacy statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) and own terms. It processes account, device and service-use information in multiple locations. Its retention depends on service purposes and obligations. Closing the relay account does not close GitHub or determine GitHub's deletion schedule.

These provider summaries were checked on 2026-10-19. No fixed backup lifetime, residency commitment or operator contract status has been verified here.
