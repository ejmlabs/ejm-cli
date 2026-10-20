//! Relay persistence through the public harness vault. Native stores are legacy input only.
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use ejm_harness_auth::{SecretId, SecretString, SecretVault};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use zeroize::{Zeroize as _, Zeroizing};

use super::{
    account::CachedAccount,
    identity::{EndpointIdentity, RelayOrigin},
};

const PRODUCT: &str = "ejm-cli-relay";
const TOKEN: &str = "device-session";
const IDENTITY: &str = "endpoint-identity";
const UPGRADE: &str = "persistent relay credentials need controlled preparation or upgrade; stop all older host/controller processes, then run ejm-cli remote migrate-credentials, or explicitly use --memory-only";
const MISSING_IDENTITY: &str =
    "saved relay endpoint key is missing; restore the original vault before pairing again";

pub(super) async fn blocking<T: Send + 'static>(
    cancel: &CancellationToken,
    operation: impl FnOnce(&CancellationToken) -> Result<T> + Send + 'static,
) -> Result<T> {
    let worker_cancel = cancel.child_token();
    let _guard = worker_cancel.clone().drop_guard();
    tokio::task::spawn_blocking(move || operation(&worker_cancel))
        .await
        .context("relay credential operation interrupted")?
}

pub(super) fn open(state: &Path) -> Result<SecretVault> {
    SecretVault::open(state.join("auth")).map_err(|_| unavailable())
}

fn id(origin: &RelayOrigin, purpose: &str) -> Result<SecretId> {
    SecretId::new(PRODUCT, purpose, origin.as_str())
}

fn current_id() -> Result<SecretId> {
    SecretId::new(PRODUCT, "current-account", "current")
}

fn unavailable() -> anyhow::Error {
    anyhow::anyhow!(
        "encrypted relay vault is unavailable, locked or invalid; restore its original native key access and retry, or explicitly choose --memory-only for this foreground session; do not replace a missing key"
    )
}

#[derive(Serialize)]
struct AccountRecord<'a> {
    metadata: &'a CachedAccount,
    token: &'a str,
}

#[derive(Deserialize)]
struct SavedAccount {
    metadata: CachedAccount,
    token: String,
}

impl Drop for SavedAccount {
    fn drop(&mut self) {
        self.token.zeroize();
    }
}

fn account_value(metadata: &CachedAccount, token: &SecretString) -> Result<SecretString> {
    let encoded = Zeroizing::new(serde_json::to_string(&AccountRecord {
        metadata,
        token: token.expose_secret(),
    })?);
    Ok(SecretString::new(encoded.as_str()))
}

pub(super) fn load_account(
    state: &Path,
    origin: &RelayOrigin,
    cancel: &CancellationToken,
) -> Result<(CachedAccount, SecretString)> {
    load_account_from(&open(state)?, origin, cancel)
}

pub(super) fn load_account_from(
    vault: &SecretVault,
    origin: &RelayOrigin,
    cancel: &CancellationToken,
) -> Result<(CachedAccount, SecretString)> {
    let tx = vault.lock_blocking(cancel).map_err(|_| unavailable())?;
    let key = id(origin, TOKEN)?;
    let Some(value) = tx.get(&key) else {
        ensure!(tx.migrated(&key), UPGRADE);
        bail!("no saved relay credential; run ejm-cli remote login");
    };
    let saved = saved_account(&value)?;
    ensure!(
        saved.metadata.origin == origin.as_str(),
        "saved relay account belongs to another origin; sign in to the configured relay"
    );
    Ok((saved.metadata.clone(), SecretString::new(&saved.token)))
}

fn saved_account(value: &SecretString) -> Result<SavedAccount> {
    serde_json::from_str(value.expose_secret()).map_err(|_| {
        anyhow::anyhow!("saved relay credential is invalid; restore the vault or sign in again")
    })
}

fn account_token(value: &SecretString, metadata: &CachedAccount) -> Result<SecretString> {
    let saved = saved_account(value)?;
    ensure!(
        saved.metadata == *metadata,
        "relay metadata does not match the saved credential; sign in again"
    );
    Ok(SecretString::new(&saved.token))
}

pub(super) fn save_account(
    state: &Path,
    metadata: &CachedAccount,
    token: &SecretString,
    cancel: &CancellationToken,
) -> Result<()> {
    save_account_into(&open(state)?, metadata, token, cancel)
}

pub(super) fn save_account_into(
    vault: &SecretVault,
    metadata: &CachedAccount,
    token: &SecretString,
    cancel: &CancellationToken,
) -> Result<()> {
    let origin = RelayOrigin::parse(&metadata.origin)?;
    let mut tx = vault.lock_blocking(cancel).map_err(|_| unavailable())?;
    let key = id(&origin, TOKEN)?;
    tx.set(key.clone(), account_value(metadata, token)?)?;
    tx.set(
        current_id()?,
        SecretString::new(serde_json::to_string(metadata)?),
    )?;
    tx.mark_migrated(key)?;
    tx.commit().map_err(|_| unavailable())
}

pub(super) fn cached_account(
    vault: &SecretVault,
    cancel: &CancellationToken,
) -> Result<Option<CachedAccount>> {
    let tx = vault.lock_blocking(cancel).map_err(|_| unavailable())?;
    let Some(value) = tx.get(&current_id()?) else {
        return Ok(None);
    };
    let metadata: CachedAccount = serde_json::from_str(value.expose_secret())
        .context("saved relay account index is invalid; run ejm-cli remote migrate-credentials")?;
    let origin = RelayOrigin::parse(&metadata.origin)?;
    let value = tx.get(&id(&origin, TOKEN)?).context(
        "saved relay account index has no credential; run ejm-cli remote migrate-credentials",
    )?;
    account_token(&value, &metadata)?;
    Ok(Some(metadata))
}

pub(super) fn remove_account(
    state: &Path,
    metadata: &CachedAccount,
    token: &SecretString,
    cancel: &CancellationToken,
) -> Result<()> {
    remove_account_from(&open(state)?, metadata, token, cancel)
}

pub(super) fn remove_account_from(
    vault: &SecretVault,
    metadata: &CachedAccount,
    token: &SecretString,
    cancel: &CancellationToken,
) -> Result<()> {
    let origin = RelayOrigin::parse(&metadata.origin)?;
    let mut tx = vault.lock_blocking(cancel).map_err(|_| unavailable())?;
    let key = id(&origin, TOKEN)?;
    let Some(value) = tx.get(&key) else {
        return Ok(());
    };
    let saved = saved_account(&value)?;
    if saved.metadata != *metadata || saved.token != token.expose_secret() {
        return Ok(());
    }
    let current = current_id()?;
    if let Some(value) = tx.get(&current) {
        let indexed: CachedAccount = serde_json::from_str(value.expose_secret())
            .context("saved relay account index is invalid; local credential was retained")?;
        if indexed == *metadata {
            tx.delete(&current);
        }
    }
    tx.delete(&key);
    tx.mark_migrated(key)?;
    tx.commit().map_err(|_| unavailable())
}

pub(super) fn identity(
    state: &Path,
    origin: &RelayOrigin,
    cancel: &CancellationToken,
) -> Result<EndpointIdentity> {
    load_identity(&open(state)?, origin, cancel)
}

fn load_identity(
    vault: &SecretVault,
    origin: &RelayOrigin,
    cancel: &CancellationToken,
) -> Result<EndpointIdentity> {
    let tx = vault.lock_blocking(cancel).map_err(|_| unavailable())?;
    let key = id(origin, IDENTITY)?;
    if let Some(value) = tx.get(&key) {
        return EndpointIdentity::from_saved(value.expose_secret().as_bytes());
    }
    ensure!(!tx.migrated(&key), MISSING_IDENTITY);
    bail!(UPGRADE)
}

/// The caller must first confirm that all older writers for this origin stopped.
pub(super) fn migrate(
    state: &Path,
    origin: &RelayOrigin,
    excluded_old_writers: bool,
    cancel: &CancellationToken,
) -> Result<bool> {
    ensure!(excluded_old_writers, UPGRADE);
    let metadata = super::account::legacy_metadata(state)?;
    if let Some(metadata) = &metadata {
        ensure!(
            metadata.origin == origin.as_str(),
            "saved relay account belongs to another origin"
        );
    }
    let vault = open(state)?;
    migrate_into(
        &vault,
        origin,
        metadata.as_ref(),
        &NativeLegacy { origin },
        true,
        cancel,
    )
}

trait Legacy {
    fn token(&self) -> Result<Option<SecretString>>;
    fn identity(&self) -> Result<Option<Zeroizing<Vec<u8>>>>;
    fn delete_token(&self) -> Result<()>;
    fn delete_identity(&self) -> Result<()>;
}

struct NativeLegacy<'a> {
    origin: &'a RelayOrigin,
}

impl NativeLegacy<'_> {
    fn entry(&self, purpose: &str, user: &str) -> Result<keyring_core::Entry> {
        super::secrets::entry(&self.origin.namespace(purpose), user).map_err(|_| unavailable())
    }

    fn read(&self, purpose: &str, user: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        match self.entry(purpose, user)?.get_secret() {
            Ok(value) => Ok(Some(Zeroizing::new(value))),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(_) => Err(unavailable()),
        }
    }

    fn delete(&self, purpose: &str, user: &str) -> Result<()> {
        match self.entry(purpose, user)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(_) => bail!(
                "legacy relay cleanup is incomplete; keep using the vault and retry remote migrate-credentials with older clients stopped"
            ),
        }
    }
}

impl Legacy for NativeLegacy<'_> {
    fn token(&self) -> Result<Option<SecretString>> {
        self.read("relay-account-v1", "device-session")?
            .map(|value| {
                let token =
                    std::str::from_utf8(&value).context("invalid legacy relay credential")?;
                ensure!(
                    !token.is_empty()
                        && token.len() <= 4096
                        && !token.chars().any(char::is_control),
                    "invalid legacy relay credential"
                );
                Ok(SecretString::new(token))
            })
            .transpose()
    }

    fn identity(&self) -> Result<Option<Zeroizing<Vec<u8>>>> {
        self.read("identity", "native-noise-v1")
    }

    fn delete_token(&self) -> Result<()> {
        self.delete("relay-account-v1", "device-session")
    }

    fn delete_identity(&self) -> Result<()> {
        self.delete("identity", "native-noise-v1")
    }
}

fn migrate_into(
    vault: &SecretVault,
    origin: &RelayOrigin,
    metadata: Option<&CachedAccount>,
    legacy: &impl Legacy,
    excluded_old_writers: bool,
    cancel: &CancellationToken,
) -> Result<bool> {
    ensure!(excluded_old_writers, UPGRADE);
    let token_id = id(origin, TOKEN)?;
    let identity_id = id(origin, IDENTITY)?;
    let mut tx = vault.lock_blocking(cancel).map_err(|_| unavailable())?;
    let existing_identity = tx
        .get(&identity_id)
        .map(|value| EndpointIdentity::from_saved(value.expose_secret().as_bytes()))
        .transpose()?;
    ensure!(
        !tx.migrated(&identity_id) || existing_identity.is_some(),
        MISSING_IDENTITY
    );
    if !tx.migrated(&token_id) {
        if let Some(token) = legacy.token()? {
            let metadata = metadata.context("legacy relay token has no account/device metadata; restore relay-account.json before migration")?;
            ensure!(
                metadata.origin == origin.as_str(),
                "legacy relay metadata belongs to another origin"
            );
            let value = account_value(metadata, &token)?;
            if let Some(existing) = tx.get(&token_id) {
                ensure!(
                    existing.expose_secret() == value.expose_secret(),
                    "conflicting relay credentials; migration left both stores intact"
                );
            }
            tx.set(token_id.clone(), value)?;
        }
        tx.mark_migrated(token_id.clone())?;
    }
    if !tx.migrated(&identity_id) {
        let identity = match (existing_identity, legacy.identity()?) {
            (Some(existing), Some(bytes)) => {
                let imported = EndpointIdentity::from_saved(&bytes)?;
                ensure!(
                    existing.public_key() == imported.public_key(),
                    "conflicting relay endpoint identities; migration left both stores intact"
                );
                existing
            }
            (Some(existing), None) => existing,
            (None, Some(bytes)) => EndpointIdentity::from_saved(&bytes)?,
            (None, None) => EndpointIdentity::new_persistent()?,
        };
        tx.set(identity_id.clone(), identity.saved_value()?)?;
        tx.mark_migrated(identity_id.clone())?;
    }
    let expected_token = tx.get(&token_id);
    let current_id = current_id()?;
    if let Some(value) = &expected_token {
        let saved = saved_account(value)?;
        ensure!(
            saved.metadata.origin == origin.as_str(),
            "saved relay account belongs to another origin"
        );
        tx.set(
            current_id.clone(),
            SecretString::new(serde_json::to_string(&saved.metadata)?),
        )?;
    }
    let expected_current = tx.get(&current_id);
    let expected_identity = tx.get(&identity_id);
    tx.commit().map_err(|_| unavailable())?;
    drop(tx);
    // Reload after commit before deleting either recoverable legacy record.
    let verified = vault.lock_blocking(cancel).map_err(|_| unavailable())?;
    for (key, expected) in [
        (&token_id, expected_token),
        (&identity_id, expected_identity),
        (&current_id, expected_current),
    ] {
        ensure!(
            verified.get(key).as_ref().map(SecretString::expose_secret)
                == expected.as_ref().map(SecretString::expose_secret),
            "relay migration could not verify durable reload; legacy entries were retained"
        );
    }
    ensure!(
        verified.migrated(&token_id) && verified.migrated(&identity_id),
        "relay migration markers could not be verified"
    );
    drop(verified);
    let token_clean = legacy.delete_token().is_ok();
    let identity_clean = legacy.delete_identity().is_ok();
    Ok(token_clean && identity_clean)
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "fixture failures belong at the assertion site"
)]
#[path = "vault_tests.rs"]
mod tests;
