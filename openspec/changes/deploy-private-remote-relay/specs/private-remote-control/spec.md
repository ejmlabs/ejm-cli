# Spec delta

## ADDED Requirements

### Requirement: Exact queued completion

Queued remote prompts SHALL remain host-owned and revocable until admission. Admission SHALL follow successful completion of the exact target run and the end of its bounded completion observers. The host SHALL recheck the live share, expiry and target before starting queued work. Ordinary native stop and graceful shutdown SHALL await bounded existing relay retirement after immediate local revocation, polling in-flight UI work and revocation together inside the existing five-second timeout.

#### Scenario: Completion observers are still running

- **WHEN** the target run has a durable successful completion while a run-end observer is blocked
- **THEN** accepted queued prompts remain pending and start once each after the completion barrier, including when the target came from the local queue

#### Scenario: Sharing stops during completion

- **WHEN** sharing stops while accepted prompts wait for completion observers
- **THEN** stop acknowledgement removes those prompts before any later admission, including after the observer returns

#### Scenario: Local input and signals during cleanup

- **WHEN** a completion observer is blocked, host startup owns protected input or an ordinary sharing host stops
- **THEN** local revocation, session changes, exit and termination remain responsive, restore the terminal and prevent queued remote admission; graceful cleanup polls in-flight UI work with revocation and awaits existing relay retirement within its bounded drain before abort fallback, and trusted local stop also cancels retained local successors
- **AND** temporary transport reconnect retains the reservation, and failed cleanup does not restore local sharing authority

#### Scenario: A fast local successor overtakes replay

- **WHEN** the target's driver hands off to a queued local successor that finishes before the host observes completion
- **THEN** the immutable completion decision causes the dependent remote chain to be discarded, with no admission based on apparent idleness

### Requirement: Compatible prompt byte limit

Native text mutations SHALL reject blank text and text above 16 KiB of UTF-8 bytes before acknowledgement. A rejected oversized request SHALL NOT discard valid queued prompts or start work.

#### Scenario: A prompt exceeds the limit by one byte

- **WHEN** a controller submits 16,385 UTF-8 bytes while a valid prompt is queued
- **THEN** the oversized request is rejected before queue acknowledgement and the valid prompt can still execute

#### Scenario: Multibyte prompt reaches the limit

- **WHEN** nonblank text contains multibyte characters and is exactly 16,384 UTF-8 bytes
- **THEN** the byte limit permits the request while the remaining host authority checks still apply

### Requirement: Pairing confirmation and restart authority

Each new pairing confirmation SHALL be displayed before accepting a fresh approval key. An enrolled controller SHALL be able to reconnect until share expiry with its pinned identity; this SHALL NOT extend initial enrollment or permit replacement identities.

#### Scenario: A confirmation replaces another panel

- **WHEN** a new candidate replaces an invitation or abandoned confirmation
- **THEN** buffered approval input is purged after displaying that candidate and a fresh key is required

#### Scenario: A paired controller restarts after initial enrollment closes

- **WHEN** the original controller reconnects after the initial enrollment deadline but before share expiry
- **THEN** the host accepts its pinned key and rejects unenrolled or replacement keys

### Requirement: Coherent account persistence

Relay account metadata and credentials SHALL publish atomically through the public vault. Normal login SHALL NOT write a separate metadata file. Logout SHALL preserve a newer saved login.

#### Scenario: Concurrent account replacement

- **WHEN** concurrent logins publish different accounts, or an older logout finishes after a new login
- **THEN** reloading returns one complete account and credential pair, without changing files linked to the legacy metadata path or deleting the newer login

### Requirement: Credential wait cancellation

Vault lock waits SHALL observe operation cancellation and the end of the awaiting caller, using the public vault cancellation contract.

#### Scenario: Shutdown while another writer holds the vault

- **WHEN** the credential operation is cancelled or its awaiting future is dropped during lock contention
- **THEN** the blocking lock waiter exits without waiting for the vault's contention deadline

### Requirement: Authoritative recovery after missing events

A delivered Events gap SHALL initiate the existing Snapshot recovery barrier with at most one request while recovery is active. The controller SHALL suspend actions and incremental Events until the Snapshot supplies authoritative run, role, cursor and mutation high-water state. Recovery SHALL preserve uncertain actions without resubmitting them. Missing events SHALL NOT establish run authority.

#### Scenario: A gap omits run completion

- **WHEN** the controller misses RunFinished in a server-truncated batch
- **THEN** it requests one authoritative Snapshot, suppresses intervening incremental Events and applies current run, role and sequence authority before enabling subsequent work, while preserving any uncertain action for outcome recovery
- **AND** a gap inside a Snapshot remains visible without causing a refresh loop

### Requirement: Snapshot and admission authority

A snapshot's run authority SHALL reflect every event through its delivered cursor. Mutation admission SHALL check current share expiry after lock contention and asynchronous refresh; receipt time SHALL NOT extend authority.

#### Scenario: Completion arrives during snapshot construction

- **WHEN** the delivered snapshot includes a completion journaled after the initial authority refresh
- **THEN** its active run state includes that completion and permits an otherwise valid ordinary Submit

#### Scenario: A request waits across expiry

- **WHEN** a mutation received before expiry waits for authority or journal refresh until the share expires
- **THEN** admission rejects it without starting or changing a run or recording a successful mutation
