use std::{
    cell::{Cell, RefCell},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
};

use ejm_harness_auth::VaultKeyStore;

use super::*;

#[derive(Default)]
struct FixtureLegacy {
    token: RefCell<Option<SecretString>>,
    identity: RefCell<Option<Zeroizing<Vec<u8>>>>,
    reads: Cell<usize>,
    deletes: Cell<usize>,
    deny_delete: bool,
}

impl FixtureLegacy {
    fn populated() -> Self {
        Self {
            token: RefCell::new(Some(SecretString::new("synthetic-relay-private-token"))),
            identity: RefCell::new(Some(Zeroizing::new(vec![0x35; 32]))),
            ..Self::default()
        }
    }
}

impl Legacy for FixtureLegacy {
    fn token(&self) -> Result<Option<SecretString>> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.token.borrow().clone())
    }

    fn identity(&self) -> Result<Option<Zeroizing<Vec<u8>>>> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.identity.borrow().clone())
    }

    fn delete_token(&self) -> Result<()> {
        self.deletes.set(self.deletes.get() + 1);
        ensure!(!self.deny_delete, "synthetic cleanup denial");
        self.token.replace(None);
        Ok(())
    }

    fn delete_identity(&self) -> Result<()> {
        self.deletes.set(self.deletes.get() + 1);
        ensure!(!self.deny_delete, "synthetic cleanup denial");
        self.identity.replace(None);
        Ok(())
    }
}

fn origin() -> RelayOrigin {
    RelayOrigin::parse("https://synthetic-relay.example").unwrap()
}

fn metadata() -> CachedAccount {
    CachedAccount {
        origin: origin().as_str().into(),
        account_id: "synthetic-account".into(),
        device_id: "synthetic-device".into(),
    }
}

fn assert_account(vault: &SecretVault, expected: &CachedAccount, token: &str) {
    let cancel = CancellationToken::new();
    let origin = RelayOrigin::parse(&expected.origin).unwrap();
    let (actual, saved) = load_account_from(vault, &origin, &cancel).unwrap();
    assert!(actual == *expected);
    assert_eq!(saved.expose_secret(), token);
    assert!(cached_account(vault, &cancel).unwrap().as_ref() == Some(expected));
}

#[test]
fn old_writer_exclusion_is_required_before_any_store_access() {
    let cancel = CancellationToken::new();
    let vault = SecretVault::memory();
    let legacy = FixtureLegacy::populated();
    assert!(
        migrate_into(
            &vault,
            &origin(),
            Some(&metadata()),
            &legacy,
            false,
            &cancel
        )
        .is_err()
    );
    assert_eq!(legacy.reads.get(), 0);
    assert_eq!(legacy.deletes.get(), 0);
    let tx = vault.lock_blocking(&cancel).unwrap();
    assert!(tx.get(&id(&origin(), TOKEN).unwrap()).is_none());
    assert!(!tx.migrated(&id(&origin(), TOKEN).unwrap()));
}

#[test]
fn migration_preserves_account_device_and_fingerprint_with_isolated_authority() {
    let cancel = CancellationToken::new();
    let vault = SecretVault::memory();
    let legacy = FixtureLegacy::populated();
    let fingerprint = EndpointIdentity::from_saved(&[0x35; 32])
        .unwrap()
        .fingerprint();
    assert!(migrate_into(&vault, &origin(), Some(&metadata()), &legacy, true, &cancel).unwrap());
    assert_eq!(legacy.deletes.get(), 2);
    let tx = vault.lock_blocking(&cancel).unwrap();
    let value = tx.get(&id(&origin(), TOKEN).unwrap()).unwrap();
    assert_eq!(
        account_token(&value, &metadata()).unwrap().expose_secret(),
        "synthetic-relay-private-token"
    );
    let key = tx.get(&id(&origin(), IDENTITY).unwrap()).unwrap();
    assert_eq!(
        EndpointIdentity::from_saved(key.expose_secret().as_bytes())
            .unwrap()
            .fingerprint(),
        fingerprint
    );
    for key in [
        id(&RelayOrigin::parse("https://other.example").unwrap(), TOKEN).unwrap(),
        id(&origin(), "other-purpose").unwrap(),
        SecretId::new("ejm-cli-registry", TOKEN, origin().as_str()).unwrap(),
        SecretId::new("provider-auth", TOKEN, origin().as_str()).unwrap(),
    ] {
        assert!(tx.get(&key).is_none());
        assert!(!tx.migrated(&key));
    }
    let mut other_device = metadata();
    other_device.device_id = "different-device".into();
    assert!(account_token(&value, &other_device).is_err());
    assert!(account_token(&SecretString::new("malformed-private-value"), &metadata()).is_err());
    drop(tx);
    assert_account(&vault, &metadata(), "synthetic-relay-private-token");
    let mut tx = vault.lock_blocking(&cancel).unwrap();
    tx.delete(&current_id().unwrap());
    tx.commit().unwrap();
    drop(tx);
    let reads = legacy.reads.get();
    for _ in 0..2 {
        assert!(migrate_into(&vault, &origin(), None, &legacy, true, &cancel).unwrap());
        assert_account(&vault, &metadata(), "synthetic-relay-private-token");
    }
    assert_eq!(legacy.reads.get(), reads);
}

#[test]
fn identity_conflict_leaves_both_legacy_entries_and_no_partial_token_commit() {
    let cancel = CancellationToken::new();
    let vault = SecretVault::memory();
    let identity_id = id(&origin(), IDENTITY).unwrap();
    let existing = EndpointIdentity::from_saved(&[0x70; 32]).unwrap();
    let mut tx = vault.lock_blocking(&cancel).unwrap();
    tx.set(identity_id.clone(), existing.saved_value().unwrap())
        .unwrap();
    tx.commit().unwrap();
    drop(tx);
    let legacy = FixtureLegacy::populated();
    let error =
        migrate_into(&vault, &origin(), Some(&metadata()), &legacy, true, &cancel).unwrap_err();
    assert!(error.to_string().contains("conflicting relay endpoint"));
    assert_eq!(legacy.deletes.get(), 0);
    let tx = vault.lock_blocking(&cancel).unwrap();
    assert!(tx.get(&id(&origin(), TOKEN).unwrap()).is_none());
    assert!(!tx.migrated(&id(&origin(), TOKEN).unwrap()));
    let saved = tx.get(&identity_id).unwrap();
    assert_eq!(
        EndpointIdentity::from_saved(saved.expose_secret().as_bytes())
            .unwrap()
            .public_key(),
        existing.public_key()
    );
}

#[test]
fn token_conflict_or_missing_metadata_retains_legacy_credentials() {
    let cancel = CancellationToken::new();
    let vault = SecretVault::memory();
    let legacy = FixtureLegacy::populated();
    assert!(migrate_into(&vault, &origin(), None, &legacy, true, &cancel).is_err());
    assert_eq!(legacy.deletes.get(), 0);
    let mut tx = vault.lock_blocking(&cancel).unwrap();
    tx.set(
        id(&origin(), TOKEN).unwrap(),
        account_value(&metadata(), &SecretString::new("other-private-token")).unwrap(),
    )
    .unwrap();
    tx.commit().unwrap();
    drop(tx);
    assert!(migrate_into(&vault, &origin(), Some(&metadata()), &legacy, true, &cancel).is_err());
    assert_eq!(legacy.deletes.get(), 0);
}

#[test]
fn failed_cleanup_and_logout_cannot_reimport_stale_tokens() {
    let cancel = CancellationToken::new();
    let vault = SecretVault::memory();
    let legacy = FixtureLegacy {
        deny_delete: true,
        ..FixtureLegacy::populated()
    };
    assert!(!migrate_into(&vault, &origin(), Some(&metadata()), &legacy, true, &cancel).unwrap());
    let token_id = id(&origin(), TOKEN).unwrap();
    let mut tx = vault.lock_blocking(&cancel).unwrap();
    assert!(tx.migrated(&token_id));
    tx.delete(&token_id);
    tx.commit().unwrap();
    drop(tx);
    let reads = legacy.reads.get();
    assert!(!migrate_into(&vault, &origin(), None, &legacy, true, &cancel).unwrap());
    assert_eq!(legacy.reads.get(), reads);
    let tx = vault.lock_blocking(&cancel).unwrap();
    assert!(tx.get(&token_id).is_none());
    assert!(tx.migrated(&token_id));
    assert!(tx.get(&id(&origin(), IDENTITY).unwrap()).is_some());
}

#[derive(Default)]
struct FixtureKeys {
    key: Mutex<Option<SecretString>>,
    pause: Mutex<Option<(mpsc::Sender<()>, mpsc::Receiver<()>)>>,
    deny_save: AtomicBool,
    saves: AtomicUsize,
}

impl VaultKeyStore for FixtureKeys {
    fn get(&self, _vault_id: &str) -> Result<Option<SecretString>> {
        if let Some((entered, resume)) = self.pause.lock().unwrap().take() {
            entered.send(()).unwrap();
            resume
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
        }
        Ok(self.key.lock().unwrap().clone())
    }

    fn set(&self, _vault_id: &str, key: &SecretString) -> Result<()> {
        self.saves.fetch_add(1, Ordering::SeqCst);
        ensure!(
            !self.deny_save.load(Ordering::SeqCst),
            "synthetic native key denial"
        );
        self.key.lock().unwrap().replace(key.clone());
        Ok(())
    }
}

#[cfg(unix)]
#[test]
fn account_saves_leave_legacy_links_untouched_and_survive_reopen() {
    for symlink in [false, true] {
        let state = tempfile::tempdir().unwrap();
        let root = state.path().join("state");
        std::fs::create_dir(&root).unwrap();
        let outside = state.path().join("outside");
        let cache = root.join("relay-account.json");
        std::fs::write(&outside, b"untouched sentinel").unwrap();
        if symlink {
            std::os::unix::fs::symlink(&outside, &cache).unwrap();
        } else {
            std::fs::hard_link(&outside, &cache).unwrap();
        }
        let keys = Arc::new(FixtureKeys::default());
        let vault = SecretVault::open_with_key_store(root.join("auth"), keys.clone()).unwrap();
        let cancel = CancellationToken::new();
        save_account_into(
            &vault,
            &metadata(),
            &SecretString::new("saved-token"),
            &cancel,
        )
        .unwrap();
        drop(vault);
        let reopened = SecretVault::open_with_key_store(root.join("auth"), keys).unwrap();
        assert_account(&reopened, &metadata(), "saved-token");
        assert_eq!(std::fs::read(outside).unwrap(), b"untouched sentinel");
        assert_eq!(std::fs::read(cache).unwrap(), b"untouched sentinel");
    }
}

#[test]
fn concurrent_account_replacement_is_coherent_and_stale_logout_is_harmless() {
    let state = tempfile::tempdir().unwrap();
    let keys = Arc::new(FixtureKeys::default());
    let path = state.path().join("auth");
    let vault = SecretVault::open_with_key_store(&path, keys.clone()).unwrap();
    let cancel = CancellationToken::new();
    save_account_into(
        &vault,
        &metadata(),
        &SecretString::new("old-token"),
        &cancel,
    )
    .unwrap();
    let replacement = CachedAccount {
        device_id: "replacement-device".into(),
        ..metadata()
    };
    let (entered, ready) = mpsc::channel();
    let (resume, release) = mpsc::channel();
    *keys.pause.lock().unwrap() = Some((entered, release));
    let first_vault = vault.clone();
    let first_account = replacement.clone();
    let first = std::thread::spawn(move || {
        save_account_into(
            &first_vault,
            &first_account,
            &SecretString::new("first-token"),
            &CancellationToken::new(),
        )
        .unwrap();
    });
    ready
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    let (started, waiting) = mpsc::channel();
    let second_vault = vault.clone();
    let second = std::thread::spawn(move || {
        started.send(()).unwrap();
        save_account_into(
            &second_vault,
            &metadata(),
            &SecretString::new("latest-token"),
            &CancellationToken::new(),
        )
        .unwrap();
    });
    waiting
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    resume.send(()).unwrap();
    first.join().unwrap();
    second.join().unwrap();
    drop(vault);
    let reopened = SecretVault::open_with_key_store(path, keys).unwrap();
    assert_account(&reopened, &metadata(), "latest-token");
    for (stale, token) in [
        (metadata(), "old-token"),
        (replacement.clone(), "latest-token"),
        (replacement, "first-token"),
    ] {
        remove_account_from(&reopened, &stale, &SecretString::new(token), &cancel).unwrap();
        assert_account(&reopened, &metadata(), "latest-token");
    }
    remove_account_from(
        &reopened,
        &metadata(),
        &SecretString::new("latest-token"),
        &cancel,
    )
    .unwrap();
    assert!(cached_account(&reopened, &cancel).unwrap().is_none());
    assert!(load_account_from(&reopened, &origin(), &cancel).is_err());
}

#[test]
fn account_records_are_scoped_to_the_exact_origin() {
    let vault = SecretVault::memory();
    let cancel = CancellationToken::new();
    let token = SecretString::new("isolated-token");
    save_account_into(&vault, &metadata(), &token, &cancel).unwrap();
    let other = RelayOrigin::parse("https://other-relay.example").unwrap();
    assert!(load_account_from(&vault, &other, &cancel).is_err());
    let other_account = CachedAccount {
        origin: other.as_str().into(),
        ..metadata()
    };
    save_account_into(
        &vault,
        &other_account,
        &SecretString::new("other-token"),
        &cancel,
    )
    .unwrap();
    let (saved, saved_token) = load_account_from(&vault, &origin(), &cancel).unwrap();
    assert!(saved == metadata());
    assert_eq!(saved_token.expose_secret(), token.expose_secret());
    remove_account_from(&vault, &metadata(), &token, &cancel).unwrap();
    assert!(load_account_from(&vault, &origin(), &cancel).is_err());
    assert_account(&vault, &other_account, "other-token");
}

#[test]
fn persisted_upgrade_reloads_before_cleanup_and_missing_key_never_regenerates() {
    let cancel = CancellationToken::new();
    let state = tempfile::tempdir().unwrap();
    let root = state.path().join("state ü with spaces");
    let keys = Arc::new(FixtureKeys::default());
    let vault = SecretVault::open_with_key_store(&root, keys.clone()).unwrap();
    let legacy = FixtureLegacy::populated();
    assert!(migrate_into(&vault, &origin(), Some(&metadata()), &legacy, true, &cancel).unwrap());
    drop(vault);
    let vault = SecretVault::open_with_key_store(&root, keys.clone()).unwrap();
    let tx = vault.lock_blocking(&cancel).unwrap();
    let value = tx.get(&id(&origin(), TOKEN).unwrap()).unwrap();
    assert_eq!(
        account_token(&value, &metadata()).unwrap().expose_secret(),
        "synthetic-relay-private-token"
    );
    drop(tx);
    keys.key.lock().unwrap().take();
    let saves = keys.saves.load(Ordering::SeqCst);
    let retry = FixtureLegacy::populated();
    let error =
        migrate_into(&vault, &origin(), Some(&metadata()), &retry, true, &cancel).unwrap_err();
    assert!(error.to_string().contains("do not replace a missing key"));
    assert!(!error.to_string().contains("synthetic-relay-private-token"));
    assert_eq!(keys.saves.load(Ordering::SeqCst), saves);
    assert_eq!(retry.deletes.get(), 0);
    assert_eq!(retry.reads.get(), 0);
}

#[test]
fn fresh_preparation_persists_identity_and_is_idempotent() {
    let cancel = CancellationToken::new();
    let state = tempfile::tempdir().unwrap();
    let root = state.path().join("auth");
    let keys = Arc::new(FixtureKeys::default());
    let vault = SecretVault::open_with_key_store(&root, keys.clone()).unwrap();
    assert!(
        load_identity(&vault, &origin(), &cancel)
            .unwrap_err()
            .to_string()
            .contains("remote migrate-credentials")
    );
    let legacy = FixtureLegacy::default();
    assert!(migrate_into(&vault, &origin(), None, &legacy, true, &cancel).unwrap());
    let fingerprint = load_identity(&vault, &origin(), &cancel)
        .unwrap()
        .fingerprint();
    drop(vault);

    let reopened = SecretVault::open_with_key_store(&root, keys).unwrap();
    assert!(migrate_into(&reopened, &origin(), None, &legacy, true, &cancel).unwrap());
    assert_eq!(
        load_identity(&reopened, &origin(), &cancel)
            .unwrap()
            .fingerprint(),
        fingerprint
    );
    let tx = reopened.lock_blocking(&cancel).unwrap();
    assert!(tx.migrated(&id(&origin(), IDENTITY).unwrap()));
    assert!(tx.migrated(&id(&origin(), TOKEN).unwrap()));
    assert!(tx.get(&id(&origin(), TOKEN).unwrap()).is_none());
}

#[test]
fn preparation_preserves_existing_identity_without_legacy_entry() {
    let cancel = CancellationToken::new();
    let vault = SecretVault::memory();
    let identity_id = id(&origin(), IDENTITY).unwrap();
    let existing = EndpointIdentity::from_saved(&[0x70; 32]).unwrap();
    let mut tx = vault.lock_blocking(&cancel).unwrap();
    tx.set(identity_id.clone(), existing.saved_value().unwrap())
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert!(
        migrate_into(
            &vault,
            &origin(),
            None,
            &FixtureLegacy::default(),
            true,
            &cancel
        )
        .unwrap()
    );
    assert_eq!(
        load_identity(&vault, &origin(), &cancel)
            .unwrap()
            .public_key(),
        existing.public_key()
    );
    let tx = vault.lock_blocking(&cancel).unwrap();
    assert!(tx.migrated(&identity_id));
}

#[test]
fn missing_marked_identity_requires_recovery_without_legacy_access() {
    let cancel = CancellationToken::new();
    let vault = SecretVault::memory();
    let identity_id = id(&origin(), IDENTITY).unwrap();
    let mut tx = vault.lock_blocking(&cancel).unwrap();
    tx.mark_migrated(identity_id.clone()).unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert!(
        load_identity(&vault, &origin(), &cancel)
            .unwrap_err()
            .to_string()
            .contains("restore the original vault")
    );
    let legacy = FixtureLegacy::populated();
    let error =
        migrate_into(&vault, &origin(), Some(&metadata()), &legacy, true, &cancel).unwrap_err();
    assert!(error.to_string().contains("restore the original vault"));
    assert_eq!(legacy.reads.get(), 0);
    assert_eq!(legacy.deletes.get(), 0);
    let tx = vault.lock_blocking(&cancel).unwrap();
    assert!(tx.migrated(&identity_id));
    assert!(tx.get(&identity_id).is_none());
    assert!(tx.get(&id(&origin(), TOKEN).unwrap()).is_none());
    assert!(!tx.migrated(&id(&origin(), TOKEN).unwrap()));
}

#[test]
fn denied_key_creation_keeps_legacy_credentials_recoverable() {
    let cancel = CancellationToken::new();
    let state = tempfile::tempdir().unwrap();
    let keys = Arc::new(FixtureKeys {
        deny_save: AtomicBool::new(true),
        ..FixtureKeys::default()
    });
    let vault = SecretVault::open_with_key_store(state.path().join("auth"), keys).unwrap();
    let legacy = FixtureLegacy::populated();
    assert!(migrate_into(&vault, &origin(), Some(&metadata()), &legacy, true, &cancel).is_err());
    assert_eq!(legacy.deletes.get(), 0);
    assert_eq!(legacy.reads.get(), 0);
}

#[tokio::test]
async fn cancelled_or_dropped_callers_release_blocking_vault_waiters() {
    for dropped in [false, true] {
        let cancel = CancellationToken::new();
        let vault = SecretVault::memory();
        let held = vault.lock(cancel.clone()).await.unwrap();
        let (entered, ready) = tokio::sync::oneshot::channel();
        let (finished, exited) = tokio::sync::oneshot::channel();
        let parent = cancel.clone();
        let task = tokio::spawn(async move {
            blocking(&parent, move |cancel| {
                let _ = entered.send(());
                let result = load_identity(&vault, &origin(), cancel);
                let _ = finished.send(cancel.is_cancelled() && result.is_err());
                result
            })
            .await
        });
        let deadline = std::time::Duration::from_secs(1);
        tokio::time::timeout(deadline, ready)
            .await
            .unwrap()
            .unwrap();
        let caller_ended = if dropped {
            task.abort();
            matches!(tokio::time::timeout(deadline, task).await, Ok(Err(error)) if error.is_cancelled())
        } else {
            cancel.cancel();
            matches!(tokio::time::timeout(deadline, task).await, Ok(Ok(Err(_))))
        };
        let waiter_ended = tokio::time::timeout(deadline, exited).await;
        drop(held);
        assert!(
            caller_ended && matches!(waiter_ended, Ok(Ok(true))),
            "vault waiter survived caller end (dropped={dropped})"
        );
    }
}
