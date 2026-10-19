//! Explicit platform stores are used only for controlled legacy relay import.

use keyring_core::{Entry, Result};

#[cfg(target_os = "macos")]
use apple_native_keyring_store::keychain::Store;
#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
))]
use dbus_secret_service_keyring_store::Store;
#[cfg(target_os = "windows")]
use windows_native_keyring_store::Store;

#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    )
))]
pub(super) fn entry(service: &str, user: &str) -> Result<Entry> {
    use keyring_core::api::CredentialStoreApi as _;

    Store::new()?.build(service, user, None)
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "windows",
    all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    )
)))]
pub(super) fn entry(_service: &str, _user: &str) -> Result<Entry> {
    Err(keyring_core::Error::NoDefaultStore)
}
