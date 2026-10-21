# relay-accounts-and-plans Specification


## Purpose

Require a verified relay identity and support paid access without mixing account, payment, model-provider or endpoint encryption authority.

## ADDED Requirements

### Requirement: Verified relay account
Relay use SHALL require a verified account identity and acceptance of service terms. Operational identity and verification provenance SHALL be stored separately from optional commercial profile data. Account creation SHALL NOT enable data sales.

#### Scenario: New account
- **WHEN** a user completes verified sign-in and accepts service terms
- **THEN** the account can use its permitted free or paid relay allowance while all commercial data choices remain off

### Requirement: Bounded native device login
Native sign-in SHALL use a maintained device-authorization flow with explicit browser approval, cancellation, expiry and polling limits. The host SHALL validate authorization origins before opening or displaying them. Bearer tokens and secret device codes SHALL NOT enter arguments, logs or the transcript.

#### Scenario: Cancelled or unapproved login
- **WHEN** approval is denied, the code expires or the user cancels
- **THEN** polling ends, temporary secrets are discarded and no authenticated account is reported

### Requirement: Separate relay credentials
Relay credentials SHALL stay separate from model-provider and package-registry credentials. Persistent secrets SHALL use the shared encrypted vault with a key protected by a native OS service. Origin and purpose namespaces SHALL stay separate. Direct per-secret OS access SHALL be limited to controlled preparation or legacy import after older writers stop. If persistence is unavailable, the client SHALL offer an explicit memory-only session or fail clearly without plaintext fallback.

#### Scenario: Provider login exists but relay login does not
- **WHEN** a user has a saved model-provider or registry account
- **THEN** it does not authorize the hosted relay and its credential is never sent to relay endpoints

#### Scenario: Secret store is unavailable
- **WHEN** persistent relay login or endpoint-key storage cannot use the shared vault or native key service
- **THEN** the client does not silently write secrets to a configuration file

#### Scenario: Prepare a persistent endpoint
- **WHEN** no endpoint identity exists in the vault
- **THEN** controlled preparation checks legacy storage only after local confirmation that older writers stopped, preserves any legacy identity and verifies the committed destination before cleanup

#### Scenario: A prepared endpoint key is missing
- **WHEN** the identity migration marker exists but its key is absent
- **THEN** ordinary use and repeated preparation require recovery and never generate a replacement

### Requirement: Account and device revocation
Users SHALL be able to list safe relay device metadata, revoke devices, log out and close their account. Account recovery or administrator actions SHALL NOT recover endpoint private keys, enroll a controller or bypass local pairing.

#### Scenario: Account recovery
- **WHEN** an account is recovered on a new device
- **THEN** the device can authenticate to account services but cannot decrypt or control an existing shared session until locally paired

#### Scenario: Fresh browser account stops sharing
- **WHEN** a verified account session without a registered transport device stops a share owned by that account
- **THEN** the account service retires that share under its mutation-origin checks without enrolling a controller or granting decryption or execution authority

### Requirement: Server-owned plans and checkout
The server SHALL choose prices, plan identifiers and account mapping for hosted checkout. Customers SHALL use the payment processor for payment details and subscription management. Missing billing configuration SHALL disable checkout clearly without granting a paid plan.

#### Scenario: Client tampers with plan details
- **WHEN** a client supplies a different price, amount or account identifier
- **THEN** checkout uses only the authenticated account and a server-allowed plan or rejects the request

### Requirement: Verified idempotent billing
Entitlements SHALL change only after authenticated payment-provider evidence. Webhooks SHALL verify the original request body, deduplicate event identifiers and reconcile authoritative payment state despite duplicate or unordered delivery. Checkout redirects SHALL NOT grant access.

#### Scenario: Forged success or webhook
- **WHEN** a browser opens a success URL without payment or a webhook has an invalid signature
- **THEN** no paid entitlement is created

#### Scenario: Duplicate and reordered events
- **WHEN** payment, cancellation and retry events arrive more than once or out of order
- **THEN** the final entitlement matches authoritative provider state and no access period is granted twice

### Requirement: Enforced entitlement lifecycle
Server-side access SHALL follow the effective plan, payment state, expiry and capacity policy. Cancellation, payment failure, refund and dispute behavior SHALL be documented. Paid users SHALL retain the same data-sale refusal and withdrawal choices.

#### Scenario: Subscription ends
- **WHEN** the paid access period expires or the payment provider confirms a state that revokes access
- **THEN** new paid allocations stop and existing connection authority expires under the relay lease policy

#### Scenario: Customer refuses data sales
- **WHEN** a customer declines or withdraws every commercial data choice
- **THEN** the purchased plan, price and relay allowance remain unchanged

### Requirement: Minimal billing records
Account storage SHALL retain only payment references and state needed for access, reconciliation and required recordkeeping. Card data, payment identities and provider-sourced authentication data SHALL be excluded from commercial exports.

#### Scenario: Commercial export includes a billing join
- **WHEN** an export path attempts to include payment or authentication identity fields
- **THEN** the export fails eligibility validation without releasing those fields
