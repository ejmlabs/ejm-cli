# Spec Delta

## Purpose

Let ejm-cli users rely on the ejmlabs registry by default, optionally point package commands at a safe alternative registry, and never send a registry session to an origin it was not issued for.

## ADDED Requirements

### Requirement: Registry origin selection
The CLI MUST choose the registry origin for `package` commands in this order: `--registry`, then `EJM_CLI_REGISTRY_URL`, then `https://registry.ejmlabs.com`. When the origin in effect is not the default, registry commands MUST print it before making network requests.

#### Scenario: No override
- **WHEN** the user runs `ejm-cli package search QUERY` with no flag or env override
- **THEN** the CLI queries `https://registry.ejmlabs.com`

#### Scenario: Environment override
- **WHEN** `EJM_CLI_REGISTRY_URL` is `http://127.0.0.1:8787` and no flag is given
- **THEN** the CLI prints that origin and queries it

#### Scenario: Flag beats environment
- **WHEN** `EJM_CLI_REGISTRY_URL` and `--registry` are both set
- **THEN** the CLI uses the `--registry` value

### Requirement: Registry overrides are validated before use
The CLI MUST reject a registry override that is not a plain origin, uses a scheme other than https, or uses http for a host other than loopback. Loopback is `localhost`, `127.0.0.0/8`, or `::1`. The CLI MUST reject `0.0.0.0`, LAN hosts, and any host that is `meetgerry.ai` or a subdomain of it. Rejection MUST happen before any network request or session lookup, exit non-zero, and name `--registry` or `EJM_CLI_REGISTRY_URL`. No flag or env var disables these rules.

#### Scenario: Insecure remote registry
- **WHEN** the user passes `--registry http://registry.example.com`
- **THEN** the CLI exits non-zero with an HTTPS-required error and sends nothing

#### Scenario: Retired registry
- **WHEN** `EJM_CLI_REGISTRY_URL` is `https://harness.meetgerry.ai` or `https://other.meetgerry.ai`
- **THEN** the CLI exits non-zero, says the meetgerry registry is retired, and names `https://registry.ejmlabs.com`

#### Scenario: Path-based registry
- **WHEN** the user passes `--registry https://ejmlabs.com/registry`
- **THEN** the CLI exits non-zero because a registry must be a plain origin

### Requirement: Registry sessions are scoped to the origin in effect
`package login` MUST save the session for the origin in effect. `whoami`, `publisher`, `publish` and `logout` MUST use only the session for that origin, and MUST NOT send a session to any other origin. `logout` MUST remove the stored session for that origin.

#### Scenario: Session from another origin
- **WHEN** the user is signed in to the default registry and runs `package whoami --registry http://127.0.0.1:8787`
- **THEN** the CLI reports that there is no session for that origin and sends no token

#### Scenario: Logout removes the stored session
- **WHEN** the user runs `package logout` while signed in to the origin in effect
- **THEN** the CLI asks that origin to revoke the session, removes the local session, and does not fail when no legacy session file remains

### Requirement: Retired-origin sessions are never sent
The CLI MUST NOT send or import a session for the retired `meetgerry.ai` origin. It MUST NOT offer a discard command: nothing shipped against that origin.

#### Scenario: Retired registry override
- **WHEN** the user runs a package command with `EJM_CLI_REGISTRY_URL` set to `https://harness.meetgerry.ai`
- **THEN** the CLI rejects the origin before any session load or request and names the default registry

### Requirement: Doctor reports the registry origin
`ejm-cli doctor` MUST show the registry origin in effect and whether it came from the default, `EJM_CLI_REGISTRY_URL`, or `--registry`. An invalid override MUST be shown as rejected. Both registry lines MUST use one helper that prints a parsed URL without userinfo, query, or fragment and strips control characters. An unparseable value MUST be `(invalid registry URL)`, never the raw input.

#### Scenario: Doctor with an environment override
- **WHEN** `EJM_CLI_REGISTRY_URL` is `http://127.0.0.1:8787` and the user runs `doctor`
- **THEN** the output shows that registry origin and that it came from `EJM_CLI_REGISTRY_URL`

#### Scenario: Doctor with a retired override
- **WHEN** `EJM_CLI_REGISTRY_URL` is `https://harness.meetgerry.ai` and the user runs `doctor`
- **THEN** the output marks the registry override as rejected because the meetgerry registry is retired

### Requirement: HTTP loopback registry access goes direct

An accepted HTTP loopback registry origin MUST be reached without any proxy from `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY` or their lowercase forms, so a registry bearer never leaves the loopback host. HTTPS origins MUST keep proxy support.

#### Scenario: Doctor with a proxy set

- **WHEN** doctor resolves an HTTP loopback registry while a proxy variable is set
- **THEN** it accepts the choice, prints the origin and its source without the proxy value, and opens no connection
