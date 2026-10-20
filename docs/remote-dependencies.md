# Native remote dependencies

Prepared source snapshot: **2026-10-19 UTC**. The table records exact direct
versions requested by this candidate's manifest. Its `Cargo.lock` is the exact
resolved native lock from source tree
`3c5a8385c1bf7689e6efb83d3f272f1efa3f60c9`, with SHA-256
`5143432fc0201bedeef680d18ae1c4681e2215fddd6e2fb45cd4cbf0149f7d1b`.
The pin selects actual protected harness merge
`42e4f9cf33bd3525aec344cfdffabc251271f782`, whose 358-file tree
`826e02310f1097e13806ca5ca1bf5cc00d71e966` supplies the completion API.
The actual protected CLI parent is `c8e2239392f71df82cbef6166b1827c8e5f5f1c5`,
with the same D49 tree as approved PR #9. Preserve the lock and run this final
pair's exact-source locked gate and hosted checks. The earlier [local pass](native-remote-publication.md#historical-local-pass)
is historical. No new build or runtime result is claimed here.

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
| [tokio-tungstenite](https://docs.rs/tokio-tungstenite/0.30.0/tokio_tungstenite/) | 0.30.0 | MIT | Tokio WebSockets with rustls and WebPKI roots, using the existing async runtime. Default features are disabled. |
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
`connect` and `rustls-tls-webpki-roots` features. Native policy bounds the origin,
headers, redirects, response body and transport frames. OAuth2 owns device
polling and response parsing. Its built-in HTTP adapter would select another
Reqwest series, so the CLI supplies the documented HTTP bridge for its pinned
Reqwest version. There is no custom TLS verifier or OAuth polling protocol.

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
system library. The earlier local pass covers macOS compilation only; Linux
and Windows compilation and real native-store operation remain unproved.
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
[test-data notice](../crates/ejm-cli/src/remote/testdata/README.md). The fixture
checks untrusted-certificate rejection and a narrowly trusted control without
provider calls or the native credential store. Neither test has been run for
this candidate.

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
platform checks on the resulting source. No Worker, npm, browser or WASM
package is introduced by this native layer.
