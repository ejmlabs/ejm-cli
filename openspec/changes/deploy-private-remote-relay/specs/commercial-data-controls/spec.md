# commercial-data-controls Specification


## Purpose

Support voluntary licensing of bounded professional profile and relay usage data while preserving encrypted session privacy and making collection, transfer, withdrawal and deletion observable.

## ADDED Requirements

### Requirement: Separate voluntary choices
Commercial profile and usage participation SHALL each default off and require an affirmative, versioned choice separate from service terms, billing and marketing. Refusal or withdrawal SHALL leave relay access, price and allowance unchanged. The notice SHALL identify fields, purposes, retention and named buyers before collection for sale. A buyer relying on the original consent SHALL be named in it.

#### Scenario: Signup or payment without commercial participation
- **WHEN** a user creates an account, buys access or leaves optional choices unchecked
- **THEN** no commercial collection or sale permission is inferred

#### Scenario: Scope changes
- **WHEN** a buyer, purpose or field exceeds the accepted notice
- **THEN** affected transfers remain blocked until a new applicable choice is recorded

### Requirement: Bounded fields and provenance
Commercial data SHALL use an explicit field allowlist and provenance. Profile fields SHALL be directly supplied through bounded choices. Usage SHALL contain only consented coarse relay statistics and optional OS family or CLI major version. Provider identity SHALL NOT be copied, inferred or prefilled into the commercial profile.

#### Scenario: User volunteers a profile
- **WHEN** a participating adult supplies role, industry, organisation-size band or country
- **THEN** the stored values are marked self-reported and linked to the accepted notice and permitted recipients

#### Scenario: Provider data enters an export
- **WHEN** a record or field is sourced from GitHub, another identity provider or enrichment data
- **THEN** commercial eligibility validation rejects it even if its value matches a directly supplied field

#### Scenario: Consent covers only part of usage data
- **WHEN** current usage consent covers only a subset of available fields
- **THEN** collection retains the consented counters and available authenticated optional metadata, including OS family and CLI major, without storing unrelated traffic, duration or device metadata

### Requirement: Excluded commercial data
Commercial storage and exports SHALL exclude application contents, prompts, output, commands, keys, credentials, files, repository names, raw IPs, precise locations, exact session traces, device fingerprints, contact details, authentication identity and payment data. No endpoint SHALL offer a broader arbitrary account export to buyers.

#### Scenario: Forbidden field is added
- **WHEN** an export or collection schema includes an unapproved or excluded field
- **THEN** the operation fails closed rather than silently including it

### Requirement: Buyer-specific export authorization
Each sale export SHALL require an approved recipient, permitted purpose, field set, retention terms and current user eligibility. Eligibility SHALL be rechecked when releasing the export. Withdrawal, deletion and opt-out SHALL cancel staged exports and retries. Releases SHALL be bounded and recorded in a restricted disclosure ledger.

#### Scenario: Consent is withdrawn after preparation
- **WHEN** a user withdraws after a dataset is staged but before release
- **THEN** the staged artifact is invalidated and cannot be downloaded or retried with that user's records

#### Scenario: Usage expires while a disclosure is prepared
- **WHEN** an included usage row expires or is removed during asynchronous disclosure preparation
- **THEN** the final disclosure transaction revalidates the included rows and their deadlines and refuses to release expired or missing data

#### Scenario: A retained profile expires before disclosure commits
- **WHEN** a profile reaches its purge deadline before cohort counting or while an individual or cohort disclosure awaits its digest
- **THEN** shared eligibility refuses the release, no ledger or cohort entry is committed, and expired profiles are excluded before the canonical universe limit

#### Scenario: Buyer is unapproved
- **WHEN** an operator requests export to an unknown recipient or unsupported purpose
- **THEN** release is denied without producing a downloadable dataset

### Requirement: Persistent opt-out signals
The service SHALL provide a public sale/share opt-out route without requiring login or identity verification for the opt-out itself. It SHALL honor Global Privacy Control for the browser and any known account, show confirmation, preserve the preference after the signal disappears, and suppress solicitation to opt back in for twelve months.

#### Scenario: Global Privacy Control is present
- **WHEN** the service receives a valid opt-out preference signal
- **THEN** known applicable sales permissions are suppressed and the response confirms the opt-out without requiring account creation

#### Scenario: Later visit lacks the signal
- **WHEN** the browser returns without the previous opt-out signal
- **THEN** its recorded opt-out remains in force and no sale permission is restored

### Requirement: User access and deletion
Users SHALL be able to inspect, correct, export and request deletion of account, profile and retained usage data. Rights workflows SHALL cover staging and recipients, track completion and retained exceptions, and enforce applicable acknowledgement and response deadlines with overdue detection and extension notices. Identity checks for access or deletion SHALL NOT block sale opt-out.

#### Scenario: Account deletion is requested
- **WHEN** an authenticated user requests account deletion or closure
- **THEN** relay authority and commercial transfers stop before successful acknowledgement, pending exports are invalidated, and the user receives tracked deletion status including separately retained records

#### Scenario: Data was already delivered
- **WHEN** a rights request concerns a prior buyer export
- **THEN** the service uses the disclosure ledger and recipient contract process to track required follow-up without falsely promising technical erasure of every buyer copy

#### Scenario: Consent is withdrawn after a committed disclosure
- **WHEN** an account withdraws a category for one or all buyers after release
- **THEN** the same transaction durably queues matching recipient-only follow-up without another rights request, duplicated work, loss of pending progress, or a relay-access change, even when the user-request limit is full

#### Scenario: Device history exceeds one rights export part
- **WHEN** retained device history within an access or export capture window exceeds one bounded part
- **THEN** the existing workflow stores every part durably, remains pending until terminal acknowledgement, survives retries and authority eviction without duplicate or missing parts, and permits private-key retrieval of each part without putting the key in a URL

#### Scenario: Registered labels use supplementary Unicode letters
- **WHEN** an access or export capture contains a device label accepted by registration with up to 64 Unicode code points
- **THEN** the same label validator accepts it during export and every delivered part preserves the original label within the existing UTF-8 byte bound

#### Scenario: Export parts reach their retention deadline
- **WHEN** a completed request's original rights retention expires
- **THEN** its parts become inaccessible and bounded cleanup removes them without extending expiry on retry or deleting pending work

#### Scenario: Expired pending parts precede completed parts
- **WHEN** protected expired parts fill a cleanup batch ahead of completed expired parts
- **THEN** a durable cursor bounds inspected candidates and advances past protected parts so later eligible parts are removed without deleting pending work

#### Scenario: Account closes from the dashboard
- **WHEN** an authenticated user confirms dashboard account closure after a prior disclosure
- **THEN** durable close-rights, recipient and operational work exist before session revocation, and the response exposes a private tracking key the user can retain

#### Scenario: An operational identity hold is active
- **WHEN** deletion work reaches an account with an unexpired operational preservation hold, including one committed after retention selection but before a destructive D1 statement
- **THEN** relay and session authority remain revoked while each destructive statement protects held identity, device, payment and account records and the tracked result reports the retained exception

### Requirement: Adult commercial participation
The initial commercial program SHALL accept only users who affirm adult eligibility through a proportionate age gate. Unknown or known minor eligibility SHALL block commercial collection and transfer. Full birth dates SHALL NOT be collected solely for this gate.

#### Scenario: Age is unknown or ineligible
- **WHEN** a user has not affirmed adult eligibility or is known to be a minor
- **THEN** the commercial program remains unavailable while relay access follows its separate account policy

### Requirement: Defined retention
Every operational and commercial data category SHALL have a configured retention rule and bounded expiry cleanup. Withdrawn commercial profiles and usage SHALL be removed under the published schedule. Restricted consent, disclosure and payment records SHALL have separate justified retention and SHALL NOT reenter sale datasets.

#### Scenario: Records expire
- **WHEN** a category's retention period ends
- **THEN** bounded cleanup removes its expired records and dependent staging while retaining only documented exceptions

#### Scenario: Eligible profiles fill the first scan page
- **WHEN** unchanged eligible subjects precede a profile whose buyer scope no longer permits retention
- **THEN** bounded scans resume past the unchanged page across authority recreation and shorten the later profile's deadline without changing eligible profiles' generations or deadlines

#### Scenario: A user repeats profile withdrawal
- **WHEN** a user repeats withdrawal without intervening fresh valid consent or an eligible profile update
- **THEN** the earliest current deletion deadline remains in force and the profile is removed when that deadline becomes due

#### Scenario: Opt-out follows profile withdrawal
- **WHEN** a first opt-out or GPC signal suppresses participation after profile withdrawal without intervening fresh valid consent
- **THEN** the earlier profile deletion deadline and original withdrawal time remain in force, and cleanup removes the profile when that deadline becomes due

#### Scenario: More samples enter the same monthly aggregate
- **WHEN** later samples are added to a live monthly aggregate
- **THEN** earlier contributions keep their original expiry bound and later collection cannot extend it

#### Scenario: Cleanup has not removed an expired aggregate
- **WHEN** access, export or a new sample reaches an expired monthly row
- **THEN** access and export exclude it, and new collection starts from zero rather than merging expired data

### Requirement: Honest data descriptions
Account-linked statistics and buyer-specific pseudonyms SHALL be treated as personal data. Reports SHALL distinguish self-reported fields, measurements and estimates. Cohort output SHALL suppress small or revealing combinations, and no output SHALL be called anonymous without a documented reidentification assessment.

#### Scenario: Usage is estimated from reservations
- **WHEN** a summary is based on reserved quota rather than measured forwarded traffic
- **THEN** it is labeled as an estimate and not represented as actual user activity

### Requirement: Commercial release gates
Data release SHALL remain disabled until notices, recipient terms, retention, rights handling and applicable jurisdiction assessments are configured and approved. Applicable risk assessments SHALL be completed before covered processing begins. Data demand and revenue SHALL remain unvalidated assumptions until supported by buyer evidence.

#### Scenario: Missing launch prerequisites
- **WHEN** commercial participation or export is enabled without required configuration or approval evidence
- **THEN** the service refuses collection for sale or export and reports the missing prerequisite

### Requirement: Deliberate rejoining after opt-out
A previously opted-out user SHALL rejoin only through a user-initiated request followed by a separate affirmative confirmation after reviewing the current notice. Active opt-out preference signals SHALL continue to suppress sale eligibility. A missing signal or a newly issued session SHALL NOT constitute consent.

#### Scenario: User requests to rejoin
- **WHEN** an opted-out user starts rejoining the commercial program
- **THEN** sales remain blocked until a distinct confirmation succeeds and no active opt-out signal suppresses eligibility; fresh unchecked consent choices then become available without restoring prior consent or clearing solicitation suppression

#### Scenario: New buyer belongs to an existing category
- **WHEN** a buyer in a previously described category was not named in the accepted consent
- **THEN** that buyer receives no records until its named purpose and fields are covered by fresh consent
