# cloudflare-relay Specification


## Purpose

Provide a low-cost hosted path between paired native clients while forwarding only ciphertext and bounding the resources consumed by each account and connection.

## ADDED Requirements

### Requirement: Account-gated relay admission
Each endpoint SHALL authenticate an active relay account before joining a room. A room SHALL belong durably to one account through its original share deadline, independently of connected transports, and admit at most one host and one controller for that account. Transport admission SHALL NOT grant access to encrypted content or host permissions.

#### Scenario: Unauthenticated or cross-account connection
- **WHEN** an endpoint lacks a valid room credential or belongs to another account
- **THEN** the relay rejects its connection before room admission

#### Scenario: Another account names a disconnected share
- **WHEN** another account requests host authority or stop for a share whose transports have disconnected
- **THEN** its durable owner and original deadline remain unchanged, and the other account cannot connect or retire that share

### Requirement: Scoped one-use connection tickets
Connection tickets SHALL be short-lived, single-use and bound to account, registered device, room and role. Redemption SHALL be atomic and survive hibernation. Tokens SHALL NOT appear in URLs, command arguments, analytics or application logs.

#### Scenario: Ticket replay or role substitution
- **WHEN** a consumed host ticket is reused or presented for a controller role or another room
- **THEN** the connection is rejected without replacing an admitted peer

### Requirement: Opaque bounded forwarding
The relay SHALL forward bounded binary ciphertext only to the admitted opposite endpoint. It SHALL NOT decrypt, interpret or persist application frames. Oversized frames, full queues and repeated rate violations SHALL close or throttle the offending connection with a clear client status.

#### Scenario: Peer sends invalid or excessive traffic
- **WHEN** an endpoint sends an oversized frame, unbounded burst or traffic without a peer
- **THEN** the relay enforces its published policy without accumulating an unbounded transcript or queue

#### Scenario: An expired connection is replaced before its callback arrives
- **WHEN** an old pair disconnects or expires and a fresh pair is admitted before an old close or error callback arrives
- **THEN** old callbacks and surviving old peers cannot close or receive frames from the replacement pair, including after hibernation

### Requirement: Idle connection efficiency
Idle room connections SHALL remain eligible for platform hibernation without application polling or recurring room timers. Traffic handlers SHALL return promptly after bounded forwarding. Hibernation SHALL preserve admitted identities, roles, expiry and counters.

#### Scenario: Room wakes after inactivity
- **WHEN** a connected room hibernates and later receives a frame
- **THEN** it restores the same admission and quota state without admitting an extra endpoint or resetting limits

### Requirement: Durable account quotas
Admission SHALL reserve bounded account-wide frame and byte allowances before forwarding. Usage reservations SHALL persist across rooms, reconnects and process replacement. Unavailable quota state SHALL deny new allocations. Client-reported usage SHALL NOT establish paid entitlements or remaining quota.

#### Scenario: Reconnect or multiple rooms try to reset quota
- **WHEN** an account exhausts its allowance and reconnects or opens another room
- **THEN** all connections remain within the same daily allowance and excess traffic is refused

#### Scenario: Quota reservation fails
- **WHEN** the quota authority is unavailable or a reservation cannot be committed
- **THEN** no unreserved traffic is forwarded

### Requirement: Bounded free launch
The default deployment SHALL use the provider's free plan, bounded beta admission and published per-account limits. It SHALL reserve capacity for account, billing and cleanup work and fail visibly when capacity is unavailable. Paid service entitlement SHALL NOT override a hosting capacity limit.

#### Scenario: Free capacity is exhausted
- **WHEN** application admission limits or provider quotas prevent a connection
- **THEN** the client reports temporary relay capacity exhaustion and the host session continues locally

#### Scenario: Operator considers upgrading
- **WHEN** hosting guidance describes a paid upgrade
- **THEN** it separates the provider's base charge and usage charges from payment fees and makes no promise of unlimited free hosting or an absolute application spending cap


#### Scenario: Scheduled work exceeds one free invocation
- **WHEN** cleanup or rights work has more pending records than one free-plan invocation can process
- **THEN** the service processes a bounded oldest-first batch within database query and external subrequest limits and leaves the remaining work durable for later invocations

#### Scenario: Initial production admission
- **WHEN** the operator deploys a new relay
- **THEN** admission remains closed until authentication and operational notices are configured, then starts with a conservative account limit and disabled unconfigured commercial and payment features

### Requirement: Lease expiry and account revocation
The relay SHALL stop renewing access after logout, device revocation, account suspension or loss of entitlement. Existing connection authority SHALL expire within fifteen minutes. Every forwarded frame SHALL check expiry. Local host revocation SHALL take effect immediately regardless of relay behavior.

#### Scenario: Account or subscription is revoked
- **WHEN** account authority ends while a room remains connected
- **THEN** further leases are refused and forwarding ends no later than the current lease expiry

#### Scenario: Account quota revocation succeeds but Room closure fails
- **WHEN** account closure revokes quota authority but cannot obtain the Room retirement acknowledgement
- **THEN** the existing workflow retains and retries the closure target until acknowledgement or its original share deadline invalidates every transport lease, and an expired target cannot retire a later owner of the same routing ID

### Requirement: Content-free operations
Service diagnostics SHALL use an explicit field allowlist and bounded retention. They SHALL exclude payloads, credentials, invitation material and host application identifiers. Operational connection data SHALL remain separate from opt-in commercial summaries.

#### Scenario: Failure and error reporting
- **WHEN** authentication, transport or forwarding fails
- **THEN** diagnostic records contain a safe error category and permitted service metadata without request bodies, authorization headers or ciphertext dumps

### Requirement: Origin-bound credentials
Clients SHALL use HTTPS and WSS with valid certificate verification. Invitations SHALL match the explicitly configured relay origin. Credentials SHALL be bound to that origin and service audience. Native clients SHALL reject credential-bearing redirects and SHALL NOT reuse saved tokens across relay origins.

#### Scenario: Invitation names an attacker relay
- **WHEN** an invitation or redirect points to a different origin or an insecure transport
- **THEN** the client rejects it without transmitting a saved bearer token or connection ticket

### Requirement: Protected browser relay admission
Browser control pages and their assets SHALL require the same authenticated relay account. Browser WebSockets SHALL redeem short-lived one-use tickets from an approved subprotocol header with exact-origin checks, without URL credentials or echoing the secret subprotocol. The native Authorization header transport SHALL remain supported.

#### Scenario: Browser attempts a room connection
- **WHEN** a browser connects with an absent, reused, cross-account or wrong-role ticket or an unapproved origin
- **THEN** the service denies admission without replacing a peer or exposing session contents
