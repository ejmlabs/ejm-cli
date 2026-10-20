# private-remote-control Specification


## Purpose

Let a user control one explicitly shared CLI session from another installed client while the relay remains unable to decrypt application traffic or grant execution authority.

## Requirements

### Requirement: Explicit foreground sharing
The host SHALL share only a locally selected session after an explicit action. Sharing SHALL remain visible and end when the host exits, the local user stops sharing, or the selected session changes. One host process SHALL own the harness and journal.

#### Scenario: Sharing starts and stops locally
- **WHEN** the user starts sharing an open session and later stops sharing or exits
- **THEN** the host displays the sharing role and controller presence while active, closes all associated channels at stop, and rejects later commands

#### Scenario: Another process already owns the journal
- **WHEN** a foreground remote host tries to open a journal owned by another process
- **THEN** it fails clearly without bypassing the lock or attaching directly to journal files

### Requirement: Authenticated device pairing
Pairing SHALL use an expiring, locally generated high-entropy invitation and authenticated endpoint identities. The controller SHALL verify the invited host identity. The host SHALL enroll at most one controller identity per invitation. Invitation secrets SHALL never enter service requests, arguments, logs or terminal history.

#### Scenario: Account access without the invitation
- **WHEN** a valid account or relay administrator attempts to connect without the invitation secret and trusted endpoint identity
- **THEN** application authentication fails before session data or control is available

#### Scenario: Stolen or reused pairing
- **WHEN** a different controller presents an expired or consumed invitation
- **THEN** the host rejects pairing and permits local revocation of the original sharing session

### Requirement: End-to-end application confidentiality
Clients SHALL encrypt and authenticate every application request, reply and event before transmission. Only the paired devices SHALL hold decryption keys. Prompts, output, tool arguments, approvals, paths, titles and internal session/run identifiers SHALL stay inside ciphertext.

#### Scenario: Relay observes a session
- **WHEN** the relay records forwarded frames and all service-side account records
- **THEN** neither record set contains application plaintext, invitation secrets, endpoint private keys or sufficient material to decrypt the stream

#### Scenario: Relay alters traffic
- **WHEN** a relay changes ciphertext, substitutes an endpoint or negotiates an unsupported protocol
- **THEN** the receiving client rejects the channel without executing an application request

### Requirement: Host-owned permissions
Sharing SHALL default to view access. Control SHALL require a local host grant and remain restricted to the shared session and existing tool grants. Remote commands SHALL NOT change providers, credentials, package grants, approvals, workspace scope or sharing authority.

#### Scenario: Viewer sends an action
- **WHEN** a view-only controller submits a prompt, steer, queue or cancel request
- **THEN** the host denies it without changing session state

#### Scenario: Controller attempts escalation
- **WHEN** a controller requests broader execution grants or a different workspace
- **THEN** the host rejects the request and retains local approval requirements

### Requirement: Run-bound control
The host SHALL serialize local and remote actions and validate the shared session and expected run state before dispatch. Steer, queue and cancel SHALL target an exact run. Stale requests SHALL fail without acting on another run.

#### Scenario: Delayed cancel reaches a newer run
- **WHEN** a cancel request names a finished run while another run is active
- **THEN** the host reports stale state and leaves the new run running

#### Scenario: Local action changes state first
- **WHEN** local input changes the expected session state before a remote submit is dispatched
- **THEN** the remote request fails with refreshed state instead of submitting against an unintended run

#### Scenario: Output progresses during control
- **WHEN** output advances after a controller observes the current run without any intervening run-state change
- **THEN** control of that exact run remains valid, while a future cursor or an older run-state view fails

### Requirement: Reconnect without repeated effects
Every reconnect SHALL authenticate a fresh encrypted channel. Display recovery SHALL use bounded durable event replay. Mutating requests SHALL carry unique identifiers, and the host SHALL reject replayed identifiers. Clients SHALL NOT automatically retry mutations whose result is unknown.

#### Scenario: Connection drops after submit
- **WHEN** a submitted prompt might have executed before the reply was lost
- **THEN** the controller restores events and reports the uncertain request outcome without resubmitting it

#### Scenario: A paired controller process restarts
- **WHEN** the same controller identity reconnects to an existing share from a new process
- **THEN** the encrypted snapshot supplies the last admitted action number and new actions use higher numbers, without replaying previous effects or confusing old results with new requests

#### Scenario: Old ciphertext is replayed
- **WHEN** traffic from an old connection or a repeated mutation identifier arrives
- **THEN** the host rejects it without repeating the operation

### Requirement: Bounded native interaction
The native controller SHALL present session events and control results without interpreting untrusted terminal control sequences. Buffers, replay pages, pending requests and reconnect attempts SHALL be bounded. Cancellation and errors SHALL restore terminal state and preserve an unsent draft.

#### Scenario: Slow controller and large output
- **WHEN** output exceeds a controller's buffer or a cursor falls behind retained events
- **THEN** the controller reports the gap and reloads bounded state instead of growing memory without limit

#### Scenario: Failed prompt submission
- **WHEN** a connection or validation error prevents a known-unexecuted submission
- **THEN** its draft remains available and no pasted text is submitted automatically

#### Scenario: Private Unix input is still buffered
- **WHEN** complete or partial invitation input remains buffered as a private Unix pairing screen closes
- **THEN** the client discards that input before ordinary editing or restoring echo, so it cannot enter the following composer or history

### Requirement: Accurate trust statement
Documentation SHALL state that relay confidentiality assumes untampered installed clients. It SHALL disclose observable metadata, endpoint compromise, malicious updates and relay denial of service. A service-delivered browser controller SHALL NOT be presented as offering the same protection against its operator.

#### Scenario: Privacy claim is shown
- **WHEN** the product describes end-to-end encryption
- **THEN** it distinguishes encrypted application traffic from visible account and connection metadata and makes no claim that the operator is cryptographically unable to alter a future client release

### Requirement: Revocation of pending work
Ending sharing SHALL discard undispatched remote actions and queued prompts before acknowledging revocation. Admission and revocation SHALL share one ordered authority boundary. Work already started SHALL continue under local control unless separately cancelled; ending sharing SHALL NOT imply rollback of completed effects.

#### Scenario: Sharing stops with remote prompts pending
- **WHEN** the local user stops sharing while remote prompts are waiting to start
- **THEN** pending remote prompts are removed before the stop action is acknowledged, no further remote admission occurs, and the UI identifies any already running work

### Requirement: Authenticated browser control
The product SHALL provide a small account-protected web dashboard and live controller using the user's relay account. Browser control SHALL retain local host pairing, host permission, encrypted application frames, bounded text presentation and the same run-bound mutation contract. The browser SHALL NOT send invitation material to HTTP endpoints or retain it in URLs, local storage, service workers or analytics.

#### Scenario: Signed-in user controls a shared session
- **WHEN** a user signs in, pastes a current invitation into the controller page and the host confirms their controller identity
- **THEN** the page displays session events and permits only the locally granted actions through the encrypted protocol

#### Scenario: Unsigned visitor or recovered account
- **WHEN** a visitor lacks a valid relay account session or lacks the private invitation and host confirmation
- **THEN** the browser cannot access application data or control the host

#### Scenario: Browser controller trust is described
- **WHEN** the web page offers live control
- **THEN** it states before pairing that the operator supplies its code and may alter it, while distinguishing the installed client's stronger protection against the relay operator
