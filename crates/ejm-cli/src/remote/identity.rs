//! Relay origins and native endpoint keys. Private keys use ASCII only for OS
//! secret-store records; they have no CLI argument, display or export format.

use std::fmt::{self, Write as _};

use anyhow::{Result, anyhow, ensure};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ejm_harness_auth::SecretString;
use sha2::{Digest, Sha256};
use snow::{
    params::DHChoice,
    resolvers::{CryptoResolver, DefaultResolver},
};
use tokio_util::sync::CancellationToken;
use url::Url;
use zeroize::Zeroizing;

const IDENTITY_STORE_PREFIX: &[u8] = b"ejm-cli-noise-v1:";
// An unpadded Base64 encoding of a 32-byte key has 43 bytes.
const IDENTITY_STORE_LENGTH: usize = IDENTITY_STORE_PREFIX.len() + 43;

/// The configured HTTPS origin is also the account and key-store namespace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayOrigin(Url);

impl RelayOrigin {
    pub fn parse(input: &str) -> Result<Self> {
        ensure!(input.len() <= 2048, "relay origin is too long");
        let url = Url::parse(input).map_err(|_| anyhow!("invalid relay origin"))?;
        ensure!(
            url.scheme() == "https"
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
                && url.path() == "/"
                && url.query().is_none()
                && url.fragment().is_none(),
            "relay must be an HTTPS origin without credentials, path, query or fragment"
        );
        Ok(Self(url))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[cfg(test)]
    pub fn loopback_for_test(input: &str) -> Result<Self> {
        let url = Url::parse(input).map_err(|_| anyhow!("invalid test relay origin"))?;
        let loopback = match url.host() {
            Some(url::Host::Ipv4(address)) => address.is_loopback(),
            Some(url::Host::Ipv6(address)) => address.is_loopback(),
            _ => false,
        };
        ensure!(
            loopback
                && url.scheme() == "http"
                && url.username().is_empty()
                && url.password().is_none()
                && url.path() == "/"
                && url.query().is_none()
                && url.fragment().is_none(),
            "test relay must be a plain HTTP loopback origin"
        );
        Ok(Self(url))
    }

    /// A route may not replace the configured authority, even through a redirect-like URL.
    pub fn url(&self, path: &str) -> Result<Url> {
        ensure!(
            path.starts_with('/') && !path.starts_with("//") && !path.contains('\\'),
            "relay route must be an absolute path on the configured origin"
        );
        let url = self
            .0
            .join(path)
            .map_err(|_| anyhow!("invalid relay route"))?;
        ensure!(
            url.origin() == self.0.origin()
                && url.username().is_empty()
                && url.password().is_none()
                && url.fragment().is_none(),
            "relay route must stay on the configured origin"
        );
        Ok(url)
    }

    pub fn namespace(&self, purpose: &str) -> String {
        let digest = Sha256::digest(self.as_str().as_bytes());
        format!("ejm-cli.relay.v1.{}.{}", hex(&digest), purpose)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityStorage {
    SecretStore,
    MemoryOnly,
}

/// Snow owns the DH implementation; this type controls where its static secret lives.
pub struct EndpointIdentity {
    private: Zeroizing<[u8; 32]>,
    public: [u8; 32],
    storage: IdentityStorage,
}

impl fmt::Debug for EndpointIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EndpointIdentity")
            .field("fingerprint", &self.fingerprint())
            .field("storage", &self.storage)
            .finish_non_exhaustive()
    }
}

impl EndpointIdentity {
    pub fn memory_only() -> Result<Self> {
        let mut private = Zeroizing::new([0_u8; 32]);
        getrandom::fill(private.as_mut())
            .map_err(|_| anyhow!("operating system random source is unavailable"))?;
        Self::from_private(private, IdentityStorage::MemoryOnly)
    }

    pub async fn load_or_create(
        origin: &RelayOrigin,
        state: &std::path::Path,
        storage: IdentityStorage,
        cancel: &CancellationToken,
    ) -> Result<Self> {
        if storage == IdentityStorage::MemoryOnly {
            return Self::memory_only();
        }
        let state = state.to_path_buf();
        let origin = origin.clone();
        super::vault::blocking(cancel, move |cancel| {
            super::vault::identity(&state, &origin, cancel)
        })
        .await
    }

    pub(super) fn new_persistent() -> Result<Self> {
        let mut identity = Self::memory_only()?;
        identity.storage = IdentityStorage::SecretStore;
        Ok(identity)
    }

    pub(super) fn from_saved(value: &[u8]) -> Result<Self> {
        Self::from_private(decode_saved_identity(value)?, IdentityStorage::SecretStore)
    }

    pub(super) fn saved_value(&self) -> Result<SecretString> {
        let encoded = encode_saved_identity(&self.private)?;
        Ok(SecretString::new(std::str::from_utf8(encoded.as_ref())?))
    }

    fn from_private(private: Zeroizing<[u8; 32]>, storage: IdentityStorage) -> Result<Self> {
        let mut dh = DefaultResolver
            .resolve_dh(&DHChoice::Curve25519)
            .ok_or_else(|| anyhow!("Noise Curve25519 implementation is unavailable"))?;
        dh.set(private.as_ref());
        let public = dh
            .pubkey()
            .try_into()
            .map_err(|_| anyhow!("Noise returned an invalid public key"))?;
        Ok(Self {
            private,
            public,
            storage,
        })
    }

    pub fn public_key(&self) -> [u8; 32] {
        self.public
    }

    pub fn fingerprint(&self) -> String {
        fingerprint(&self.public)
    }

    pub fn storage(&self) -> IdentityStorage {
        self.storage
    }

    pub(super) fn private_key(&self) -> &[u8] {
        self.private.as_ref()
    }
}

#[cfg(test)]
fn secret_store_error() -> anyhow::Error {
    anyhow!(
        "OS secret store is unavailable; explicitly choose memory-only mode for this foreground session"
    )
}

pub fn fingerprint(public_key: &[u8; 32]) -> String {
    Sha256::digest(public_key)
        .chunks(2)
        .map(hex)
        .collect::<Vec<_>>()
        .join(" ")
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        // Writing to a String cannot fail.
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
fn saved_identity(secret: keyring_core::Result<Vec<u8>>) -> Result<Option<EndpointIdentity>> {
    match secret {
        Ok(secret) => {
            let secret = Zeroizing::new(secret);
            let private = decode_saved_identity(secret.as_slice())?;
            EndpointIdentity::from_private(private, IdentityStorage::SecretStore).map(Some)
        }
        Err(keyring_core::Error::NoEntry) => Ok(None),
        Err(_) => Err(secret_store_error()),
    }
}

fn encode_saved_identity(private: &[u8; 32]) -> Result<Zeroizing<[u8; IDENTITY_STORE_LENGTH]>> {
    let mut encoded = Zeroizing::new([0; IDENTITY_STORE_LENGTH]);
    let (prefix, payload) = encoded.split_at_mut(IDENTITY_STORE_PREFIX.len());
    prefix.copy_from_slice(IDENTITY_STORE_PREFIX);
    URL_SAFE_NO_PAD
        .encode_slice(private, payload)
        .map_err(|_| anyhow!("could not encode relay identity for OS secret store"))?;
    Ok(encoded)
}

fn decode_saved_identity(secret: &[u8]) -> Result<Zeroizing<[u8; 32]>> {
    let mut private = Zeroizing::new([0; 32]);
    // Previous versions stored exactly 32 raw bytes. Preserve that identity.
    if secret.len() == private.len() {
        private.copy_from_slice(secret);
        return Ok(private);
    }
    ensure!(
        secret.len() == IDENTITY_STORE_LENGTH,
        invalid_saved_identity()
    );
    let payload = secret
        .strip_prefix(IDENTITY_STORE_PREFIX)
        .ok_or_else(invalid_saved_identity)?;
    let decoded = URL_SAFE_NO_PAD
        .decode_slice(payload, private.as_mut())
        .map_err(|_| invalid_saved_identity())?;
    ensure!(decoded == private.len(), invalid_saved_identity());
    Ok(private)
}

fn invalid_saved_identity() -> anyhow::Error {
    anyhow!("saved relay identity is invalid; remove it explicitly")
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "test fixtures should fail at the assertion site"
)]
mod tests {
    use super::*;

    #[test]
    fn origin_and_routes_never_change_credential_authority() {
        let origin = RelayOrigin::parse("https://relay.example").unwrap();
        assert_eq!(origin.as_str(), "https://relay.example/");
        assert_eq!(
            origin.url("/api/device").unwrap().host_str(),
            Some("relay.example")
        );
        for rejected in [
            "http://relay.example",
            "https://user:secret@relay.example",
            "https://relay.example/api",
            "https://relay.example/?token=secret",
            "https://relay.example/#secret",
        ] {
            assert!(RelayOrigin::parse(rejected).is_err());
        }
        for rejected in [
            "https://other.example/api",
            "//other.example/api",
            "/\\other.example",
            "/api#secret",
        ] {
            assert!(origin.url(rejected).is_err());
        }
        assert_ne!(
            origin.namespace("identity"),
            RelayOrigin::parse("https://other.example")
                .unwrap()
                .namespace("identity")
        );
        assert_ne!(origin.namespace("identity"), origin.namespace("account"));
    }

    #[tokio::test]
    async fn memory_only_keys_are_fresh_and_debug_has_no_private_material() {
        let directory = tempfile::tempdir().unwrap();
        let state = directory.path().join("must-not-exist");
        let origin = RelayOrigin::parse("https://synthetic-relay.example").unwrap();
        let storage = IdentityStorage::MemoryOnly;
        let cancel = CancellationToken::new();
        let first = EndpointIdentity::load_or_create(&origin, &state, storage, &cancel)
            .await
            .unwrap();
        let second = EndpointIdentity::load_or_create(&origin, &state, storage, &cancel)
            .await
            .unwrap();
        assert!(
            !state.exists(),
            "memory-only identity touched persistent state"
        );
        assert_ne!(first.public_key(), second.public_key());
        assert_eq!(first.storage(), IdentityStorage::MemoryOnly);
        assert!(!format!("{first:?}").contains(&hex(first.private_key())));
        let restored = EndpointIdentity::from_private(
            Zeroizing::new(*first.private),
            IdentityStorage::MemoryOnly,
        )
        .unwrap();
        assert_eq!(first.public_key(), restored.public_key());
    }

    #[test]
    fn unavailable_or_malformed_secret_store_never_becomes_a_saved_identity() {
        let unavailable = saved_identity(Err(keyring_core::Error::NoDefaultStore));
        assert!(unavailable.is_err());
        assert!(
            unavailable
                .err()
                .unwrap()
                .to_string()
                .contains("explicitly choose memory-only")
        );
        assert!(saved_identity(Ok(vec![3; 31])).is_err());
        assert!(
            saved_identity(Err(keyring_core::Error::NoEntry))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn utf8_secret_store_record_and_legacy_raw_key_preserve_identity() {
        let original =
            EndpointIdentity::from_private(Zeroizing::new([0xff; 32]), IdentityStorage::MemoryOnly)
                .unwrap();
        let encoded = encode_saved_identity(&original.private).unwrap();
        assert!(std::str::from_utf8(encoded.as_ref()).is_ok());
        for stored in [encoded.to_vec(), original.private_key().to_vec()] {
            let restored = saved_identity(Ok(stored)).unwrap().unwrap();
            assert_eq!(original.public_key(), restored.public_key());
            assert_eq!(restored.storage(), IdentityStorage::SecretStore);
        }
    }

    #[test]
    fn malformed_or_unknown_secret_store_records_fail_without_exposing_material() {
        let encoded = encode_saved_identity(&[0xff; 32]).unwrap();
        let mut truncated = encoded.to_vec();
        truncated.pop();
        let mut padded = encoded.to_vec();
        padded.push(b'=');
        let mut unknown_version = encoded.to_vec();
        unknown_version[IDENTITY_STORE_PREFIX.len() - 2] = b'2';
        let mut invalid_alphabet = encoded.to_vec();
        invalid_alphabet[IDENTITY_STORE_PREFIX.len()] = b'!';
        let mut invalid_trailing_bits = encoded.to_vec();
        invalid_trailing_bits[IDENTITY_STORE_LENGTH - 1] = b'9';
        for stored in [
            truncated,
            padded,
            unknown_version,
            invalid_alphabet,
            invalid_trailing_bits,
        ] {
            let error = saved_identity(Ok(stored)).err().unwrap();
            assert_eq!(
                error.to_string(),
                "saved relay identity is invalid; remove it explicitly"
            );
        }
    }

    #[test]
    fn curve25519_public_key_matches_rfc_7748_alice_vector() {
        let private = [
            0x77, 0x07, 0x6d, 0x0a, 0x73, 0x18, 0xa5, 0x7d, 0x3c, 0x16, 0xc1, 0x72, 0x51, 0xb2,
            0x66, 0x45, 0xdf, 0x4c, 0x2f, 0x87, 0xeb, 0xc0, 0x99, 0x2a, 0xb1, 0x77, 0xfb, 0xa5,
            0x1d, 0xb9, 0x2c, 0x2a,
        ];
        let identity =
            EndpointIdentity::from_private(Zeroizing::new(private), IdentityStorage::MemoryOnly)
                .unwrap();
        assert_eq!(
            hex(&identity.public_key()),
            "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"
        );
    }
}
