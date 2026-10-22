# Native remote dependencies

This records the dependency graph accepted through protected native CLI PR #21;
the [acceptance record](native-remote-publication.md) binds its merge, tree and
Linux/macOS checks. The earlier protected docs/UI
prerequisite kept C8's Cargo manifests, lock and working 4a harness pin unchanged.
That PR #20 prerequisite merged as `9e3f8ef11dc083c31b651b9dae15a98e086e3359`;
native PR #21 followed it with the graph below. Following source changes still
require their own normal and hosted gates; live acceptance remains separate.

Original dependency review: **2026-10-19 UTC**. The table records exact direct
versions requested by the native manifest. After the October 20 TLS
advisory correction, the accepted native baseline `Cargo.lock` SHA-256 is
`02ba0f0097754444a40e106d683ce530886ec7c31d30009f0c5cc43750d6118b`.
The earlier `5143432fc0201bedeef680d18ae1c4681e2215fddd6e2fb45cd4cbf0149f7d1b`
lock from tree `3c5a8385c1bf7689e6efb83d3f272f1efa3f60c9` is historical.
The pin selects actual protected harness merge
`42e4f9cf33bd3525aec344cfdffabc251271f782`, whose 358-file tree
`826e02310f1097e13806ca5ca1bf5cc00d71e966` supplies the completion API.
Native preparation began at protected CLI
`c8e2239392f71df82cbef6166b1827c8e5f5f1c5`, with the same D49 tree as approved
PR #9. The accepted inert PR #22 preserved that lock. The following Worker
layer adds only the browser workspace/package and required feature edges through
Cargo; its changed lock must preserve native package identities and patched TLS.
It requires its own exact-source locked gate and hosted checks.
The [corrected d8 local result](native-remote-publication.md) and
[earlier passes](native-remote-publication.md#historical-local-pass) remain
historical, separate from PR #21's acceptance. This native baseline record claims no new
build or runtime result for the following Worker layer.

Existing Tokio, Serde, URL, harness and terminal packages remain shared. The
versioned links below are primary API references retained from the source
review. They are not a fresh upstream maintenance or advisory check. `MIT /
Apache-2.0` means either license may be chosen.

| Package | Version | License | Purpose and selection evidence |
| --- | --- | --- | --- |
| [snow](https://docs.rs/snow/0.10.0/snow/) | 0.10.0 | MIT / Apache-2.0 | Implements the standard Noise handshake and transport state, including authentication and cipher counters. Its published feature API permits selecting the primitives this protocol uses. |
| [keyring-core](https://docs.rs/keyring-core/1.0.1/keyring_core/) | 1.0.1 | MIT / Apache-2.0 | Legacy relay import API. The adapter builds each legacy entry from its explicit platform store instead of changing a process-wide default; new records use the public harness vault. |
| [apple-native-keyring-store](https://docs.rs/apple-native-keyring-store/1.0.2/apple_native_keyring_store/) | 1.0.2 | MIT / Apache-2.0 | Legacy macOS Keychain entry import, with only the `keychain` feature selected. The harness also uses this maintained store to protect the common vault key. |
| [windows-native-keyring-store](https://docs.rs/windows-native-keyring-store/1.1.0/windows_native_keyring_store/) | 1.1.0 | MIT / Apache-2.0 | Legacy Windows Credential Manager entry import. The harness protects the common vault key through the same maintained API. |
| [dbus-secret-service-keyring-store](https://docs.rs/dbus-secret-service-keyring-store/1.0.1/dbus_secret_service_keyring_store/) | 1.0.1 | MIT / Apache-2.0 | Unix Secret Service through dbus-secret-service `4.1.0`. The `crypto-rust` feature encrypts local Secret Service traffic; `vendored` is disabled. |
| [oauth2](https://docs.rs/oauth2/5.0.0/oauth2/) | 5.0.0 | MIT / Apache-2.0 | Owns device authorization polling and protocol responses. Upstream documents its supported HTTP adapter interface and Rust compatibility policy. |
| [reqwest](https://docs.rs/reqwest/0.13.5/reqwest/) | 0.13.5 | MIT / Apache-2.0 | Certificate-verified HTTPS, bounded streamed responses and request timeouts. Default features are disabled; the CLI selects rustls, JSON and streaming. |
| [tokio-tungstenite](https://docs.rs/tokio-tungstenite/0.30.0/tokio_tungstenite/) | 0.30.0 | MIT | Tokio WebSockets with rustls and native root loading, using the existing async runtime. Default features are disabled. |
| [futures-util](https://docs.rs/futures-util/0.3.34/futures_util/) | 0.3.34 | MIT / Apache-2.0 | Sink and stream adapters for that transport; only `sink` and `std` are selected. |
| [getrandom](https://docs.rs/getrandom/0.4.3/getrandom/) | 0.4.3 | MIT / Apache-2.0 | Operating-system random bytes for native invitation and identity material. |
| [zeroize](https://docs.rs/zeroize/1.9.1/zeroize/) | 1.9.1 | MIT / Apache-2.0 | Clears reachable secret buffers. It does not promise erasure of every allocator, library or browser copy. |
| [sha2](https://docs.rs/sha2/0.10.9/sha2/) | 0.10.9 | MIT / Apache-2.0 | SHA-256 fingerprints and credential namespace hashes. |
| [base64](https://docs.rs/base64/0.23.1/base64/) | 0.23.1 | MIT / Apache-2.0 | Strict invitation and credential encoding. |


## Package boundaries

Snow supplies `Noise_XXpsk3_25519_ChaChaPoly_SHA256`. Its selected features are
`std`, `default-resolver`, `use-curve25519`, `use-chacha20poly1305`, `use-sha2`
and `use-getrandom`; default features are disabled. Its `std` feature may add
`ring` to the resolved graph without choosing the ring resolver. Review those
features in the refreshed lock. **Snow's
[upstream README](https://github.com/mcginty/snow) states that it has not received
a formal audit.** Local tests do not change that status.

The standard Noise implementation avoids writing cryptographic primitives or
a handshake. Libsodium key exchange and secretstream would still require a
replacement authenticated ephemeral PSK handshake. A libp2p Noise adapter adds
libp2p transport conventions. Iroh uses a different relay server and protocol.
No comparative package size, CPU or latency benchmark has been run here.

Reqwest owns HTTPS with rustls, JSON, streamed bodies and timeout support;
default features are disabled. Tokio-tungstenite owns WebSockets with the
`connect` and `rustls-tls-native-roots` features. Native policy bounds the origin,
headers, redirects, response body and transport frames. OAuth2 owns device
polling and response parsing. Its built-in HTTP adapter would select another
Reqwest series, so the CLI supplies the documented HTTP bridge for its pinned
Reqwest version. There is no custom TLS verifier or OAuth polling protocol.

WSS reuses `rustls-native-certs 0.8.5` to load OS trust anchors; Rustls still
verifies certificate chains and hostnames. This is root loading, not the full
OS platform verifier. Empty loaded roots fail closed. Local
[`SSL_CERT_FILE` or `SSL_CERT_DIR` overrides](https://docs.rs/rustls-native-certs/0.8.5/rustls_native_certs/fn.load_native_certs.html)
replace the platform roots. The corrected lock updates `rustls-webpki` from
0.103.15 to 0.103.16 and `webpki-root-certs` from 1.0.9 to 1.0.10, removes
both `webpki-roots` versions (0.26.11 and 1.0.9) and reduces packages from 469 to 467
without a new package. The accepted native receipt includes the corrected graph's
normal and advisory gates; following graph changes need fresh checks.

## Persistent secrets and controlled import

New relay records use the public `ejm-harness-auth` encrypted vault at
`state/auth`. Product, purpose and HTTPS-origin identifiers separate them from
provider and registry credentials. The harness owns key access, authenticated
encryption, locking and atomic writes. This slice requires its published public
vault API and exact-run steer/cancel API; see
[the integration contract](native-remote-publication.md#dependency-integration).

Direct keyring-core and native-store dependencies are retained only for the
explicit controlled import command. Each legacy entry uses its own platform
store builder, without replacing a process-wide store. Normal endpoint loading
reads only the vault. Import requires older writers to have stopped, preserves
an existing identity, commits and reloads its destination and markers, then
removes old entries. A fresh endpoint is created only in that preparation step.
A missing marked key requires recovery. See
[credentials and recovery](remote-control.md#credentials-and-recovery).

On Linux, the selected DBus store uses `crypto-rust` without `vendored`.
Builds need the distro's `pkg-config` and libdbus development package, such as
`libdbus-1-dev` on Debian/Ubuntu or `dbus-devel` on Fedora. Runtime persistence
needs libdbus, a session D-Bus and an unlocked Secret Service. The operating
system library has its own security and distribution review:
[D-Bus COPYING](https://dbus.freedesktop.org/doc/COPYING) offers AFL-2.1 or
GPL-2.0-or-later for libdbus; Cargo's Rust license gate does not cover that
system library. The earlier local pass covered macOS compilation only; PR #21
later passed hosted Linux/macOS checks. Windows compilation and native-store
operation beyond the tested fixtures remain unproved.
The store documents a [KDE Wallet UTF-8 restriction](https://docs.rs/dbus-secret-service-keyring-store/1.0.1/dbus_secret_service_keyring_store/#usage-with-kde-wallet);
legacy key records use canonical versioned ASCII Base64 while retaining exact
32-byte legacy read compatibility.

Keyring's umbrella wrapper would select additional Linux backend requirements.
The explicit maintained stores let the importer choose the needed API and
avoid the previously rejected standalone Zbus maintenance chain. Do not restore
that wrapper or add advisory ignores to resolve the graph. Plain credential
files do not meet the vault requirement.

## Test and terminal dependencies

`async-trait = 0.1.92` supports the offline test tool. `tokio-rustls = 0.26.6`
with only `aws_lc_rs` supplies the real local TLS fixture. Its checked-in
self-signed certificate and key are public, test-only material; see the
[test-data notice](../crates/ejm-cli/src/remote/testdata/README.md). The existing
fixture now checks that an untrusted peer receives neither HTTP bearer/OAuth
data nor a WSS ticket, with a narrowly trusted HTTP control. Earlier HTTP-only
coverage passed in the [d8 gate](native-remote-publication.md); PR #21's accepted
native source includes the checked extension. No provider calls or native
credential store are used by that fixture.

The preceding [terminal slice](terminal-input.md) owns Termwiz, Rustix,
Crossterm and WinSafe input adaptation, including input resets before private
screens. This layer calls that API; it does not add another terminal reader.
Retain [terminal notices](terminal-third-party-notices.md) and review all final
resolved license exceptions. `deny.toml` adds only exact duplicate-version
exceptions copied from the reviewed source; their necessity and advisory state
must be checked against the refreshed lock. No advisory ignore is added.

## Build and release requirements

Keep Rust `1.99.1`, rustfmt and Clippy from `rust-toolchain.toml`. Resolve only
the intended native graph against the exact published harness source. Run
`cargo deny --locked --all-features check`, the full [contribution
gate](contributing.md), the native protected-input/PTY scenarios and hosted
platform checks on each following source. The original native layer introduced
no Worker, npm, browser or WASM package. The following
[inert UI/Relay prerequisite](ci-bootstrap.md#current-inert-shared-ui-and-relay-inputs)
accepted install review data without a runnable Relay manifest. The new Worker
source activates that reviewed manifest and adds the browser crate. See
[publication boundaries](relay-publication.md#source-and-dependency-boundaries)
for public UI contracts, source/install binding and the minimal browser lock
change. Full Worker/UI acceptance and deployment remain pending.
