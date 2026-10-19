# Spec Delta

## Purpose

Keep contribution, maintainer review, documentation, and security policy consistent as the repositories become public.

## ADDED Requirements

### Requirement: Human admission precedes costly work
Every PR commit MUST receive an explicit readiness decision from a current human maintainer before CI or AI review runs. PR creation, pushes, labels from outsiders, bot comments, and stale readiness decisions MUST NOT start costly work.

#### Scenario: An outsider opens or updates a PR
- **WHEN** an outsider opens a PR or pushes another commit
- **THEN** quality checks and AI review wait for a human readiness decision on that exact head

#### Scenario: A maintainer marks the head ready
- **WHEN** a current human maintainer explicitly admits the current PR head
- **THEN** bounded quality and two AI passes can run once for that admission

### Requirement: Public automation isolates untrusted changes
Checks MUST run on ephemeral hosted runners with minimal permissions. Untrusted PR code MUST NOT receive write tokens or model credentials. Review scope, runtime, concurrency, and output MUST be bounded. Repeated or missing events MUST NOT authorize extra AI runs.

#### Scenario: A submitted workflow tries to spend budget
- **WHEN** a PR changes workflow definitions or scripts
- **THEN** trusted admission and review code still come from reviewed immutable standards and base code

### Requirement: Maintainer approval binds to the current commit
Every pull request to main MUST have approval from a current human maintainer and all required checks passing. A maintainer SHALL be able to approve their own current commit explicitly without bypassing checks.

#### Scenario: Maintainer approves a current commit
- **WHEN** a maintainer approves the current head through a review or an explicit approval comment
- **THEN** the approval gate accepts that commit, including when the maintainer is the author

#### Scenario: The head changes
- **WHEN** a new commit is pushed after approval
- **THEN** the previous approval cannot authorize the new head

#### Scenario: An outsider submits an approval
- **WHEN** an account without current maintainer permission approves
- **THEN** the approval gate remains blocked

### Requirement: Checks fail closed
Main MUST require passing quality, repository standards, and both AI review passes. Missing credentials, incomplete reviews, stale results, or failed checks MUST prevent merging.

#### Scenario: A required review cannot run
- **WHEN** the AI reviewer lacks credentials or either review pass fails
- **THEN** its required gate does not report success

#### Scenario: A review has no findings but is incomplete
- **WHEN** either review lacks required source, cannot finish, or omits JSON boolean `complete: true`
- **THEN** the installed completion guard rejects the result even if its findings list is empty

### Requirement: Shared caller updates include exact source context
A shared workflow pin update MUST include bounded, readable source evidence for the exact installed and proposed implementations. The evidence MUST identify commits, trees, source modes, blob IDs, SHA-256 digests, sizes, and original paths. It MUST remain nonexecuted source data and MUST NOT grant the model network access or repository credentials.

#### Scenario: A caller revision changes
- **WHEN** a PR changes the shared workflow revision
- **THEN** both actual reviewers can inspect both exact implementations from the admitted source archive, including generated workflows, readable sources, review contracts, and templates

#### Scenario: Review evidence names an earlier caller pair
- **WHEN** the archived evidence does not match the trusted installed and proposed caller revisions
- **THEN** it does not establish completion of the workflow comparison

### Requirement: Public guidance is discoverable
The repository MUST provide consistent entry points for setup, development, documentation, contribution, support, vulnerability reporting, license terms, and release status.

#### Scenario: A contributor opens the repository
- **WHEN** a contributor follows the README links
- **THEN** the linked documents exist and distinguish available source from published artifacts

### Requirement: Package grants have distinct arguments
Package enable commands MUST parse without panic and MUST keep package provider and widget grants separate from global host provider and widget selectors. Trust and named-export validation MUST remain explicit.

#### Scenario: Host selectors and package grants appear together
- **WHEN** a user passes global `--provider` and `--widget` options with package `--grant-provider` and `--grant-widget` options
- **THEN** each value reaches its intended configuration without mixing grants or causing a parser panic

#### Scenario: A package is enabled without trust or a declared export
- **WHEN** a user omits trust or requests an undeclared export
- **THEN** the command fails without enabling the package
