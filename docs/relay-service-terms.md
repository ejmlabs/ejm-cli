# Relay beta service terms

**Version:** `beta-v1`. **Status:** approved for publication; unpublished. **Effective date:** pending actual publication. The privacy notice is [Relay signup and privacy notice](relay-notice.md), version `relay-beta-2026-10-19`.

## Access and use

The operator is ejmlabs, llc, California, United States. Sign in with GitHub, accept the current terms and register only devices you control. Admission and capacity limits apply. Use the service only for systems and sessions you may access. Do not interfere with another user's access, misuse credentials or evade limits.

Pairing can let a controller read the shared session and, when the host grants control, submit work that runs with the host's permissions. Check the host identity and requested role. Keep pairing invitations, local keys and device/session credentials private. Stop a share or revoke a device when it is no longer trusted. Relay authentication alone does not grant an existing encrypted session's keys.

## Beta limits and reliability

This is a free, bounded beta without an availability guarantee. The initial admission cap is two active accounts. Each account has at most one room, one host and one controller. The free daily allowance is 50,000 combined incoming frames and 64 MiB per UTC day, reserved in blocks that can exceed actual traffic. Frames are at most 16 KiB; the limit is 100 frames per second and connection leases last at most fifteen minutes. Shared provider capacity may fail or be exhausted. Keep a local recovery path; local CLI use can continue without the relay.

Service access may stop for closure, revocation, security response, misuse, capacity or changes to the beta. No payment or subscription is activated by these terms. Any future paid plan needs separately published prices and payment/cancellation terms.

## Privacy and trust

Paired endpoints encrypt stream content, while the operator sees account/device records and connection metadata. The operator serves browser code and could change it to capture browser secrets or plaintext. Installed native controllers give a stronger boundary against a malicious operator. The privacy notice describes collection, retention, provider handling and requests. Accepting these terms does not opt into commercial data collection or sale. Those features remain disabled for this beta.

## Closure, changes and concerns

Use the account/privacy controls to close relay access and track deletion requests. Some records and copied rights exports remain under the notice's separate retention rules and exceptions. Sign-out alone does not close an account.

A future terms version requires fresh acceptance for relay access. Existing linked accounts retain sign-in and privacy access while new admission is closed. Send service concerns to support@ejmlabs.com and privacy concerns to privacy@ejmlabs.com. The operator will use the approved runbook to investigate and record follow-up; these contacts do not waive other remedies.

## Governing law and disputes

California law governs these terms to the extent permitted by applicable law. Contact support@ejmlabs.com for informal resolution of service disputes. These terms do not require mandatory arbitration or waive court access, non-waivable remedies or consumer rights.
