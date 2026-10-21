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
- **WHEN** an authenticated user closes their account
- **THEN** new relay authority and commercial transfers stop, pending exports are invalidated, and the user receives a tracked deletion status including separately retained records

#### Scenario: Data was already delivered
- **WHEN** a rights request concerns a prior buyer export
- **THEN** the service uses the disclosure ledger and recipient contract process to track required follow-up without falsely promising technical erasure of every buyer copy

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
- **THEN** sales remain blocked until a distinct confirmation succeeds and no active opt-out signal suppresses eligibility

#### Scenario: New buyer belongs to an existing category
- **WHEN** a buyer in a previously described category was not named in the accepted consent
- **THEN** that buyer receives no records until its named purpose and fields are covered by fresh consent
