# Spec Delta

## Purpose

Let ejm-cli users rely on the ejmlabs registry by default, optionally point package commands at a safe alternative registry, and never send a registry session to an origin it was not issued for.

## ADDED Requirements

### Requirement: Registry origin selection
The CLI MUST choose the registry origin for `package` commands in this order: `--registry`, then `EJM_CLI_REGISTRY_URL`, then the harness default registry origin. When the origin in effect is not the default, registry commands MUST print it before making network requests.

#### Scenario: No override
- **WHEN** the user runs `ejm-cli package search QUERY` with no flag or env override
- **THEN** the CLI queries the harness default registry origin

#### Scenario: Environment override
- **WHEN** `EJM_CLI_REGISTRY_URL` is `http://127.0.0.1:8787` and no flag is given
- **THEN** the CLI prints that origin and queries it

#### Scenario: Flag beats environment
- **WHEN** `EJM_CLI_REGISTRY_URL` and `--registry` are both set
- **THEN** the CLI uses the `--registry` value

### Requirement: Registry overrides are validated before use
The CLI MUST reject a registry override that fails the harness registry origin rules. That includes insecure non-loopback HTTP, a path, credentials, a query or fragment, another scheme, and the retired `meetgerry.ai` origin. The rejection MUST happen before any network request or session lookup, exit non-zero, and name `--registry` / `EJM_CLI_REGISTRY_URL`. No flag or env var MAY disable these rules.

#### Scenario: Insecure remote registry
- **WHEN** the user passes `--registry http://registry.example.com`
- **THEN** the CLI exits non-zero with an HTTPS-required error and sends nothing

#### Scenario: Retired registry
- **WHEN** `EJM_CLI_REGISTRY_URL` is `https://harness.meetgerry.ai`
- **THEN** the CLI exits non-zero, says the meetgerry registry is retired, and names the current default

#### Scenario: Path-based registry
- **WHEN** the user passes `--registry https://ejmlabs.com/registry`
- **THEN** the CLI exits non-zero because a registry must be a plain origin

### Requirement: Registry sessions are scoped to the origin in effect
`package login` MUST save the session for the origin in effect. `whoami`, `publisher`, `publish` and `logout` MUST use only the session for that origin, and MUST NOT send a session to any other origin. `logout` MUST remove the stored session for that origin.

#### Scenario: Session from another origin
- **WHEN** the user is signed in to the default registry and runs `package whoami --registry http://127.0.0.1:8787`
- **THEN** the CLI reports that there is no session for that origin and sends no token

#### Scenario: Logout removes the vault session
- **WHEN** the user runs `package logout` while signed in to the origin in effect
- **THEN** the CLI asks that origin to revoke the session, removes its local record even if no legacy session file exists, and a later `whoami` reports no session

### Requirement: Retired-origin sessions can be discarded
When a saved session for the retired `meetgerry.ai` origin exists, the CLI MUST NOT send it anywhere. A blocked `package login` MUST explain how to discard the session. The CLI MUST offer a command that discards it locally without any network request.

#### Scenario: Login blocked by a retired session
- **WHEN** a legacy session for `https://harness.meetgerry.ai` exists and the user runs `package login`
- **THEN** the CLI makes no request with the old token and prints the command that discards it

#### Scenario: Discard the retired session
- **WHEN** the user runs the discard command
- **THEN** the retired session is removed locally with no network request, and `package login` to the default registry can proceed

### Requirement: Doctor reports the registry origin
`ejm-cli doctor` MUST show the registry origin in effect and whether it came from the default, `EJM_CLI_REGISTRY_URL`, or `--registry`. An invalid override MUST be shown as rejected.

#### Scenario: Doctor with an environment override
- **WHEN** `EJM_CLI_REGISTRY_URL` is `http://127.0.0.1:8787` and the user runs `doctor`
- **THEN** the output shows that registry origin and that it came from `EJM_CLI_REGISTRY_URL`

#### Scenario: Doctor with a retired override
- **WHEN** `EJM_CLI_REGISTRY_URL` is `https://harness.meetgerry.ai` and the user runs `doctor`
- **THEN** the output marks the registry override as rejected because the meetgerry registry is retired
