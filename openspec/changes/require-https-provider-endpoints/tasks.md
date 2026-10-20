# Tasks

## 1. Reject non-loopback HTTP before the API key is read

- [ ] 1.1 Change `Args::api_base_url()` in `crates/ejm-cli/src/args.rs` so it returns `Result<&str>` and validates the effective base URL (`--base-url`, otherwise `EJM_CLI_BASE_URL`, otherwise the existing https default). Parse with the `url` crate already used by the CLI. Do not resolve DNS or open a socket. Accept `https` when a host is present. Accept `http` only when `url::Host` is ASCII case-insensitive `localhost`, an IPv4 address with `is_loopback()` (`127.0.0.0/8`), or an IPv6 address with `is_loopback()` (`::1`). Return `invalid provider base URL. Change --base-url or EJM_CLI_BASE_URL.` when the value does not parse or has no host. Return `provider base URL must use https, or http on a loopback host (localhost, 127.0.0.0/8, or ::1). Change --base-url or EJM_CLI_BASE_URL.` for every other rejection. Do not include the supplied URL, userinfo, query, or fragment in either error. Do not add a length cap, rewrite the URL, or add a bypass flag. Leave harness userinfo, query, and fragment checks where they are.
- [ ] 1.2 In `crates/ejm-cli/src/main.rs` `provider()`, call `api_base_url()?` for OpenAI-compatible and Anthropic providers before `required_api_key()` and before `OpenAiProvider::new` or `AnthropicProvider::new`. Keep the `responses` and `chatgpt` fixed-endpoint check ahead of this rule. Do not call the validating method from `--replay`, `--replay-file`, session listing, or `--package-provider`.
- [ ] 1.3 In `doctor()`, call `api_base_url()?` for OpenAI-compatible and Anthropic providers before any stdout write. On failure, exit through the existing `ejm-cli: ` path with status 1 and do not print the URL. Leave `responses` and `chatgpt` doctor output unchanged.
- [ ] 1.4 Update the `--base-url` help text: https is required, http is limited to a loopback host, and ChatGPT and Responses keep their fixed official endpoint.

## 2. Tests

- [ ] 2.1 Add `Args` unit tests for the accepted defaults and for `https` remote URLs, `http://localhost`, `http://LOCALHOST`, `http://127.0.0.1`, `http://127.0.0.2`, and `http://[::1]` with a port and path. Reject remote `http`, `http://0.0.0.0`, `http://[::]`, `http://[::ffff:127.0.0.1]`, `http://localhost.example`, `file://`, the empty string, and a value that does not parse. For `http://user:secret@example.test/v1`, assert the https sentence and assert the error text contains neither `secret` nor the raw URL. Assert that `http://127.1/v1` follows `is_loopback()` on the host `url` 2 actually parses.
- [ ] 2.2 Add integration tests in `crates/ejm-cli/tests/cli.rs` that do not use the `--replay` helper. Clear `EJM_CLI_API_KEY`, `OPENAI_API_KEY`, and `ANTHROPIC_API_KEY`. For `doctor` and `run`, and for both `openai` and `anthropic`, cover `--base-url http://api.example.test/v1` and `EJM_CLI_BASE_URL` alone. Assert status code 1, stderr containing `ejm-cli: provider base URL must use https`, and stderr that does not contain a planted fixture key, `set EJM_CLI_API_KEY`, or `provider connection failed`. Assert stdout does not contain the URL. Assert `doctor --base-url http://127.0.0.1:9/v1` succeeds and prints that endpoint. Assert `--replay run` with a remote `http` base URL succeeds. Assert `sessions` with `EJM_CLI_BASE_URL=http://api.example.test/v1` succeeds. Assert `models` for `responses` and `chatgpt` with an `http://` base URL still reports the fixed official endpoint and does not report the new https sentence.
- [ ] 2.3 Run `cargo test -p ejm-cli api_base_url` and `cargo test -p ejm-cli --test cli` with at most two Cargo jobs. No paid provider calls and no live provider.

## 3. Docs and changelog

- [ ] 3.1 Update `docs/ejm-cli.md`, `README.md`, and the OpenAI-compatible row in `site/dist/index.html` so they match `--help`: https is required, http is only for a loopback host (`localhost`, `127.0.0.0/8`, or `::1`), a rejected URL exits 1 and is not printed, and the existing https examples stay. Do not deploy the site.
- [ ] 3.2 Add an Unreleased Fixed changelog entry for the CLI behavior. State that `.github/harness-revision` stays on `4a751c6a3f24d7d143140f79ff53517fb93e3531` and that other ejm-harness callers are outside this change.

## 4. Harness pin and full gate

- [ ] 4.1 Do not edit `../ejm-harness`, `.github/harness-revision`, or `Cargo.lock`. The new tests must fail when the CLI check is removed even though this pin's private `endpoint()` still accepts remote `http://`.
- [ ] 4.2 Run `./scripts/check`. In the implementation notes, record that adopting harness change `require-https-provider-endpoints` is a separate pin update after that change reaches protected harness main, and that the CLI early error stays after that update.

## Workflow follow-up

- Implement only after Everett approves this design. `proposal.md` and `specs/` belong to the product manager. Do not add them unless asked.
- Do not open the harness pin adoption from the implementation of this change.
- Sync and archive only after the implementation tasks above are verified.
