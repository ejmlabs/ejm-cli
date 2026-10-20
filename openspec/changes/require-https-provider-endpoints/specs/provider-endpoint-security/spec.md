# Spec Delta

## Purpose

Make sure the CLI never uses a provider API key with a base URL that would send it in cleartext, and gives users a clear error that names the offending flag or environment variable.

## ADDED Requirements

### Requirement: CLI base URL requires HTTPS except for loopback
The CLI MUST accept a provider base URL from `--base-url` or `EJM_CLI_BASE_URL` only if it uses `https`, or uses `http` with a loopback host. Loopback hosts are `localhost` (exact, ASCII case-insensitive), an IPv4 address in `127.0.0.0/8`, or `::1`. This is the same rule the harness enforces. Every other value MUST be rejected with a non-zero exit, and no flag or environment variable MAY disable the rule.

#### Scenario: HTTPS base URL from the flag
- **WHEN** the user runs the CLI with `--base-url https://api.deepseek.com/v1`
- **THEN** the CLI uses that endpoint

#### Scenario: HTTPS base URL from the environment
- **WHEN** `EJM_CLI_BASE_URL` is `https://api.deepseek.com/v1` and no flag is given
- **THEN** the CLI uses that endpoint

#### Scenario: HTTP remote base URL from the flag
- **WHEN** the user passes `--base-url http://api.example.com/v1`
- **THEN** the CLI exits non-zero with an error that names `--base-url` / `EJM_CLI_BASE_URL` and says HTTPS is required except for loopback hosts

#### Scenario: HTTP remote base URL from the environment
- **WHEN** `EJM_CLI_BASE_URL` is `http://api.example.com/v1`
- **THEN** the CLI exits non-zero with the same error

#### Scenario: HTTP loopback base URL
- **WHEN** the base URL is `http://localhost:11434/v1`, `http://127.0.0.1:1234/v1` or `http://[::1]:1234/v1`
- **THEN** the CLI uses that endpoint

#### Scenario: Lookalike or non-loopback hosts
- **WHEN** the base URL is `http://localhost.evil.com/v1`, `http://127.0.0.1.nip.io/v1` or `http://192.168.1.10:11434/v1`
- **THEN** the CLI exits non-zero with the HTTPS-required error

#### Scenario: Other schemes
- **WHEN** the base URL uses `ftp`, `file`, `ws` or no scheme
- **THEN** the CLI exits non-zero with an invalid base URL error

### Requirement: Base URL is validated before credentials are used
The CLI MUST validate the base URL before it reads or reports on the provider API key, and before it opens any network connection. The error output MUST NOT include the API key.

#### Scenario: Insecure URL with a key present
- **WHEN** `EJM_CLI_API_KEY` is set and the base URL is `http://api.example.com/v1`
- **THEN** the CLI exits with the base URL error, makes no network request, and does not print the key

#### Scenario: Insecure URL with no key present
- **WHEN** no API key is configured and the base URL is `http://api.example.com/v1`
- **THEN** the CLI reports the base URL error rather than the missing-key error

### Requirement: Doctor reports a disallowed endpoint
The `doctor` command MUST show when the configured provider base URL is not permitted, instead of printing it as a usable endpoint.

#### Scenario: Doctor with an insecure endpoint
- **WHEN** the user runs `doctor` with `EJM_CLI_BASE_URL` set to `http://api.example.com/v1`
- **THEN** the endpoint line marks the URL as rejected and explains that HTTPS is required except for loopback hosts

### Requirement: Default and fixed endpoints are unchanged
With no base URL given, the CLI MUST keep using `https://api.openai.com/v1` for the `openai` provider and `https://api.anthropic.com/v1` for the `anthropic` provider. The `responses` and `chatgpt` providers MUST keep rejecting any base URL override.

#### Scenario: No override
- **WHEN** the user selects `--provider anthropic` without a base URL
- **THEN** the CLI uses `https://api.anthropic.com/v1`

#### Scenario: Override on a fixed-endpoint provider
- **WHEN** the user selects `--provider chatgpt` with `--base-url https://other.test/v1`
- **THEN** the CLI exits non-zero, saying the provider uses a fixed official endpoint
