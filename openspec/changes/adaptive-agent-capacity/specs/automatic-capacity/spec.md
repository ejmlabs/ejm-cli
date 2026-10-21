## Purpose

Let the CLI use the harness's automatic capacity controls without manual trial and error, while showing effective limits, host coordination, and local learning clearly.

## ADDED Requirements

### Requirement: Harness owns native supervision

The CLI SHALL delegate native child ownership, lifetime links, admission, cleanup, and recovery to public harness contracts. It SHALL provide only a narrow early executable entry for private helper mode before ordinary setup. Waiting agents SHALL NOT create a helper or process per agent. Existing grants and public contracts SHALL remain valid.

#### Scenario: Enter private native helper mode

- **WHEN** the harness launches the current executable with its private helper invocation and inherited handles
- **THEN** the public harness adapter handles it before normal argument, auth, runtime, or terminal setup
- **AND** normal CLI commands retain their existing behavior

#### Scenario: A CLI client exits during native work

- **WHEN** the client dies while a controlled command or package import is active
- **THEN** the harness lifetime links trigger owned-child cleanup and retain its protected resources through cleanup
- **AND** the CLI reports the supported native recovery state without replaying an uncertain launch or effect

#### Scenario: Waiting agents remain cheap

- **WHEN** hundreds or thousands of managed agents wait for work or admission
- **THEN** the CLI creates no native process per waiting agent
- **AND** expensive native work waits for shared admission

#### Scenario: Helper startup or reuse competes for the remaining working set

- **WHEN** several helper domains compete for capacity during startup or reuse
- **THEN** the public harness runtime stages bounded demand without a resource lease
- **AND** unused helper readiness holds end whenever resource activation must wait
- **AND** the shared admission plan preserves a useful dependent turn without adding per-job shared records
- **AND** package startup counts its full resident and import peak once

### Requirement: Native recovery respects OS ownership

Shared-owner replacement SHALL require release of the previous OS ownership lock. Missed heartbeats alone SHALL NOT authorize takeover. The CLI SHALL expose public native restrictions as enforced, monitored, unsupported, or unknown. It SHALL NOT add a per-job lifecycle log, replay uncertain launches, or claim portable containment of arbitrary detached descendants.

#### Scenario: Paused owner still holds its lock

- **WHEN** a participant misses heartbeats while its OS ownership lock remains held
- **THEN** the CLI reports stale contact or waiting from the shared harness
- **AND** it does not establish a second owner or repeat the participant's work

#### Scenario: Prior OS owner releases ownership

- **WHEN** the prior owner's OS ownership lock is released
- **THEN** takeover follows the public harness recovery contract
- **AND** an uncertain effect remains interrupted rather than being replayed

#### Scenario: Arbitrary detached descendants on macOS

- **WHEN** a granted arbitrary command detaches beyond supported lifetime links on macOS
- **THEN** docs and status describe the available monitoring and recovery limits
- **AND** they do not claim that a missing parent PID or empty sampled tree proves every descendant ended

### Requirement: Automatic capacity after verified activation

After platform activation gates pass, the CLI SHALL select automatic capacity unless fixed mode is selected. Before activation, capacity omission SHALL retain legacy process-local fixed behavior; explicit modes SHALL opt into shared coordination. The harness SHALL own admission, observations, provider feedback, and learning. Managed-agent count SHALL remain distinct from executing work.

#### Scenario: Launch without capacity settings

- **WHEN** the user starts an interactive session or a noninteractive run without capacity settings after platform activation
- **THEN** the CLI selects automatic capacity
- **AND** the harness chooses operating targets from the available constraints and observations
- **AND** the previous fixed defaults do not become implicit automatic ceilings

#### Scenario: Default activation remains gated

- **WHEN** runtime recovery or measurement gates remain unverified for a platform
- **THEN** omitted capacity retains legacy behavior on that platform
- **AND** the CLI documents explicit coordinated opt-in without claiming the automatic default is active

#### Scenario: Many parked or waiting agents

- **WHEN** hundreds or thousands of managed agents are parked or waiting on a provider
- **THEN** the CLI reports managed agents separately from runnable work, in-flight requests, and executing tools
- **AND** it does not allocate a terminal view, polling task, or running-tool reservation for every parked agent

### Requirement: Coordinated package startup and lifetime

Before package code loads or expensive startup allocation, the CLI SHALL prepare a public harness capacity context and use it for package runtimes and final harness composition. Startup, resident language-host memory, imports, reloads, and run work SHALL share one governor and balanced host grants. Coordination SHALL remain valid through success, cancellation, failure, and shutdown.

#### Scenario: Lazy language host waits for capacity

- **WHEN** an enabled code package first needs its Node or Python host and capacity is unavailable
- **THEN** host launch, import, and their expensive preparation wait for admission through the prepared shared context
- **AND** waiting remains cancellable
- **AND** unused language hosts are not started or assigned resident host memory grants

#### Scenario: Compose the final harness after package loading

- **WHEN** package loading succeeds and the CLI constructs its final harness and executor
- **THEN** all use the same prepared capacity context and governor
- **AND** package startup does not create a second controller, independent host budget, or private-source dependency

#### Scenario: Language host remains resident between callbacks

- **WHEN** a Node or Python host stays alive after import or a callback completes
- **THEN** its resident memory remains charged to the shared budget until verified host exit
- **AND** its native lifetime link remains valid through callbacks, reload, drain, and shutdown
- **AND** completed startup or callback execution does not retain an unnecessary running-work slot
- **AND** a later callback does not charge the same resident memory twice

#### Scenario: Reload needs temporary capacity

- **WHEN** package import or reload requires additional work, memory, or another host generation
- **THEN** the incremental peak is admitted through the same shared context before the expensive phase starts
- **AND** any still-live host generation remains accounted for until its verified exit

#### Scenario: Startup or composition fails or is cancelled

- **WHEN** host startup, import, reload, or final harness composition fails or is cancelled
- **THEN** the CLI stops further startup work and completes the owning runtime's cleanup with the prepared context still valid
- **AND** acquired grants are released once their protected resources are known to be free
- **AND** uncertain surviving children remain accounted for through the harness recovery contract

#### Scenario: Shut down after package use

- **WHEN** the CLI shuts down after loading code packages
- **THEN** the shared context outlives pending callbacks, language hosts, and final harness cleanup
- **AND** shutdown balances the grants after verified cleanup without resetting the shared ledger

### Requirement: Safe automatic activation

The CLI SHALL activate a platform's automatic default only after native recovery evidence and the representative workload's critical throughput, responsiveness and shared-resource gates pass. Existing evidence MAY cover unchanged runtime and dependency paths. Unmet critical gates SHALL block that platform alone. Native evidence SHALL remain platform-specific.

#### Scenario: Participant crashes during ordinary host use

- **WHEN** an idle or busy participant dies during the native macOS recovery gate
- **THEN** surviving participants resume useful work within the documented recovery bound without reclaiming unproven live child resources
- **AND** the evidence records macOS native limits without implying Linux or Windows coverage

#### Scenario: Crash recovery does not meet the gate

- **WHEN** a claimed platform cannot prove safe, bounded useful progress after participant death
- **THEN** activation of the CLI automatic default waits for a verified harness lifecycle revision
- **AND** neither the CLI nor recovery instructions delete the shared ledger to bypass the problem

#### Scenario: Runtime overhead exceeds a scale budget

- **WHEN** native helper or resident-generation cost causes the representative workload to exceed shared resource bounds or miss its critical useful-throughput budget
- **THEN** the CLI reports the missed budget and measured scope
- **AND** default activation remains gated pending a justified and verified runtime revision

#### Scenario: Another platform remains unverified

- **WHEN** one platform passes its critical gates while another lacks native ownership or recovery evidence
- **THEN** only the verified platform may activate its automatic default
- **AND** the other platform retains its prior default and truthful capability status without blocking the verified platform

### Requirement: Explicit ceilings and fixed mode

The CLI SHALL accept explicit session, model, tool, and tool-memory ceilings and an explicit fixed capacity mode. Explicit ceilings SHALL remain hard upper bounds in either mode. Fixed mode SHALL use the former fixed defaults for omitted values and SHALL remain subject to shared host admission, provider limits, and execution grants.

#### Scenario: Bound automatic operation

- **WHEN** the user supplies one or more existing numeric capacity flags in automatic mode
- **THEN** those values bound the matching resource
- **AND** the remaining resource targets remain automatic
- **AND** a lower operating target does not change the configured ceiling

#### Scenario: Reproduce the former fixed envelope

- **WHEN** the user selects fixed mode and omits numeric capacity flags
- **THEN** the configured process envelope is 100 managed sessions, 8 model requests, 4 tools, and 512 MiB of aggregate admitted tool memory
- **AND** the CLI explains when host or provider constraints prevent using the full envelope

#### Scenario: Invalid capacity setting

- **WHEN** a capacity value is zero, outside its supported range, or incompatible with another selected setting
- **THEN** the CLI reports the relevant option and valid usage before starting a run
- **AND** it does not silently replace an explicit value with a larger limit

### Requirement: Shared host participation

Participating CLI processes SHALL use the harness's common host capacity scope even when their session-state directories differ. The CLI SHALL distinguish coordinated reservations from sampled external load and SHALL display unavailable or degraded coordination without claiming complete host enforcement.

#### Scenario: Separate session directories on one host

- **WHEN** two participating CLI or worker processes use different session-state directories in the same supported host and user resource domain
- **THEN** their host resource reservations draw from one shared budget
- **AND** neither CLI creates its own host scheduler or host-capacity registry

#### Scenario: Other applications consume resources

- **WHEN** unrelated applications or older nonparticipating clients consume host resources
- **THEN** the CLI presents their observed effect as sampled external load
- **AND** it does not describe their resource use as coordinated reservations

#### Scenario: Coordination is lost

- **WHEN** the harness cannot safely coordinate new host work
- **THEN** the CLI reports the availability problem and the resulting wait reason
- **AND** Stop and local input remain usable while the harness holds new shared admission

### Requirement: Truthful capacity presentation

The CLI SHALL present capacity mode, scope, observation availability, managed agents, active and queued model and tool work, operating targets, hard ceilings, and the reason work waits. Unknown or stale observations SHALL be labelled. Rendering SHALL use public harness snapshots without collecting host measurements or advancing the tuning loop.

#### Scenario: Work waits below a user ceiling

- **WHEN** the user configured a ceiling higher than the current safe or useful operating target
- **THEN** the status explains the lower target and the constraint or learning state that set it
- **AND** it preserves the distinction between the target, the hard ceiling, and active work

#### Scenario: Observation is unavailable

- **WHEN** a supported resource observation is unavailable or stale
- **THEN** capacity status identifies the missing or stale observation and any fallback in use
- **AND** it does not invent a percentage or imply that an unobserved constraint is healthy

#### Scenario: Refresh terminal status

- **WHEN** the terminal redraws or the user requests status repeatedly
- **THEN** status reads remain bounded and use the latest harness capacity snapshot
- **AND** those reads do not trigger additional probes, admissions, or learning updates

### Requirement: Local learning controls

The CLI SHALL explain that capacity learning stays local, is scoped by the harness, and can be reset through a local command. Reset SHALL use the public harness interface, preserve explicit settings, provider rate accounting, sessions, credentials, and execution grants, and take effect without treating old observations as new evidence.

#### Scenario: Reuse learned capacity

- **WHEN** a later CLI process starts in a compatible host and workload scope with valid learned data
- **THEN** the harness can use that data to choose an initial operating target
- **AND** status identifies the learned or cold-start state without promising optimal capacity

#### Scenario: Reset local learning

- **WHEN** the local user runs `ejm-cli capacity reset`
- **THEN** the harness clears the applicable learned capacity profile
- **AND** the CLI reports the reset scope and outcome
- **AND** active reservations, configured ceilings, sessions, credentials, and grants remain valid
- **AND** provider rate budgets remain governed by provider reset conditions rather than the learning reset

#### Scenario: Remote controller attempts capacity mutation

- **WHEN** a remote controller requests a capacity reset or a change to host policy or execution grants
- **THEN** the existing remote session command authority does not permit the action
- **AND** capacity tuning does not expand tool or shell permissions
