//! Thin browser bindings for the same maintained Noise engine as the native client.
//!
//! Product invitation validation lives here. Snow owns key agreement, the Noise
//! state machine, authenticated encryption and nonce sequencing.

use anyhow::{Result, anyhow, ensure};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use snow::{Builder, HandshakeState, TransportState};
use url::Url;
use wasm_bindgen::prelude::*;
use zeroize::{Zeroize, Zeroizing};

const PATTERN: &str = "Noise_XXpsk3_25519_ChaChaPoly_SHA256";
const FRAME_LIMIT: usize = 16 * 1024;
const HANDSHAKE_LIMIT: usize = 256;
const ENROLLMENT_MS: u64 = 300_000;
const SHARE_MS: u64 = 28_800_000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InvitationText {
    version: u16,
    origin: String,
    routing_id: String,
    host_public_key: String,
    psk: String,
    role: Role,
    issued_at_ms: u64,
    enrollment_expires_at_ms: u64,
    share_expires_at_ms: u64,
}

impl Drop for InvitationText {
    fn drop(&mut self) {
        self.psk.zeroize();
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Role {
    View,
    Control,
}

struct Invitation {
    origin: Url,
    routing_id: String,
    host: [u8; 32],
    psk: Zeroizing<[u8; 32]>,
    role: Role,
    issued: u64,
    enrollment_expires: u64,
    share_expires: u64,
}

impl Invitation {
    fn decode(text: &str, configured: &str, now: u64) -> Result<Self> {
        ensure!(text.len() <= 2048, "invalid invitation");
        let encoded = text
            .trim()
            .strip_prefix("ejm-remote:v1:")
            .ok_or_else(|| anyhow!("unsupported or invalid invitation"))?;
        let json = Zeroizing::new(
            URL_SAFE_NO_PAD
                .decode(encoded)
                .map_err(|_| anyhow!("invalid invitation"))?,
        );
        let raw: InvitationText =
            serde_json::from_slice(&json).map_err(|_| anyhow!("invalid invitation"))?;
        let origin = parse_origin(&raw.origin)?;
        ensure!(
            origin == parse_origin(configured)?,
            "invitation belongs to another relay"
        );
        ensure!(
            raw.version == 1
                && key(&raw.routing_id).is_ok()
                && raw.enrollment_expires_at_ms.checked_sub(raw.issued_at_ms)
                    == Some(ENROLLMENT_MS)
                && raw.share_expires_at_ms.checked_sub(raw.issued_at_ms) == Some(SHARE_MS),
            "invalid invitation bounds"
        );
        let invite = Self {
            origin,
            routing_id: raw.routing_id.clone(),
            host: key(&raw.host_public_key)?,
            psk: Zeroizing::new(key(&raw.psk)?),
            role: raw.role,
            issued: raw.issued_at_ms,
            enrollment_expires: raw.enrollment_expires_at_ms,
            share_expires: raw.share_expires_at_ms,
        };
        invite.live(now)?;
        ensure!(now < invite.enrollment_expires, "invitation has expired");
        Ok(invite)
    }

    fn live(&self, now: u64) -> Result<()> {
        ensure!(
            now >= self.issued && now < self.share_expires,
            "sharing has expired or the local clock is before its creation"
        );
        Ok(())
    }

    fn prologue(&self) -> Vec<u8> {
        let role = match self.role {
            Role::View => "view",
            Role::Control => "control",
        };
        format!(
            "ejm-cli/remote/1\n{}\n{}\n{}\n{role}\n{}\n{}\ninitiator=controller\nresponder=host",
            self.origin,
            self.routing_id,
            URL_SAFE_NO_PAD.encode(self.host),
            self.enrollment_expires,
            self.share_expires,
        )
        .into_bytes()
    }
}

fn parse_origin(text: &str) -> Result<Url> {
    let origin = Url::parse(text).map_err(|_| anyhow!("invalid relay origin"))?;
    let scheme_ok = origin.scheme() == "https"
        || (cfg!(all(feature = "test-loopback", debug_assertions))
            && origin.scheme() == "http"
            && matches!(origin.host_str(), Some("127.0.0.1" | "[::1]")));
    ensure!(
        scheme_ok
            && origin.host_str().is_some()
            && origin.username().is_empty()
            && origin.password().is_none()
            && origin.path() == "/"
            && origin.query().is_none()
            && origin.fragment().is_none(),
        "relay origin must be an HTTPS authority"
    );
    Ok(origin)
}

fn key(text: &str) -> Result<[u8; 32]> {
    ensure!(text.len() == 43, "invalid invitation key");
    let decoded = Zeroizing::new(
        URL_SAFE_NO_PAD
            .decode(text)
            .map_err(|_| anyhow!("invalid invitation key"))?,
    );
    decoded
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("invalid invitation key"))
}

fn fingerprint(public: &[u8]) -> String {
    Sha256::digest(public)
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| format!("{:02x}{:02x}", pair[0], pair[1]))
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Metadata<'a> {
    origin: &'a str,
    routing_id: &'a str,
    role: Role,
    host_fingerprint: String,
    controller_fingerprint: String,
    share_expires_at_ms: u64,
}

enum Channel {
    Idle,
    Handshake(Box<HandshakeState>),
    Transport(Box<TransportState>),
    Failed,
}

struct Core {
    invitation: Invitation,
    private: Zeroizing<Vec<u8>>,
    public: Vec<u8>,
    channel: Channel,
}

impl Core {
    fn new(text: &str, origin: &str, now: u64) -> Result<Self> {
        let invitation = Invitation::decode(text, origin, now)?;
        let identity = Builder::new(PATTERN.parse()?).generate_keypair()?;
        Ok(Self {
            invitation,
            private: Zeroizing::new(identity.private),
            public: identity.public,
            channel: Channel::Idle,
        })
    }

    fn metadata(&self) -> Result<String> {
        Ok(serde_json::to_string(&Metadata {
            origin: self.invitation.origin.as_str(),
            routing_id: &self.invitation.routing_id,
            role: self.invitation.role,
            host_fingerprint: fingerprint(&self.invitation.host),
            controller_fingerprint: fingerprint(&self.public),
            share_expires_at_ms: self.invitation.share_expires,
        })?)
    }

    fn begin(&mut self, now: u64) -> Result<Vec<u8>> {
        self.channel = Channel::Failed;
        self.invitation.live(now)?;
        let prologue = self.invitation.prologue();
        let mut handshake = Builder::new(PATTERN.parse()?)
            .local_private_key(&self.private)?
            .prologue(&prologue)?
            .psk(3, &self.invitation.psk)?
            .build_initiator()?;
        let mut output = vec![0; HANDSHAKE_LIMIT];
        let length = handshake.write_message(&[], &mut output)?;
        output.truncate(length);
        self.channel = Channel::Handshake(Box::new(handshake));
        Ok(output)
    }

    fn accept(&mut self, input: &[u8], now: u64) -> Result<Vec<u8>> {
        let previous = std::mem::replace(&mut self.channel, Channel::Failed);
        self.invitation.live(now)?;
        ensure!(input.len() <= HANDSHAKE_LIMIT, "invalid handshake frame");
        let Channel::Handshake(mut handshake) = previous else {
            return Err(anyhow!("handshake is not awaiting the host"));
        };
        let mut payload = Zeroizing::new([0; HANDSHAKE_LIMIT]);
        let length = handshake
            .read_message(input, payload.as_mut())
            .map_err(|_| anyhow!("host authentication failed"))?;
        ensure!(length == 0, "unexpected handshake payload");
        // Do not release the controller static identity until the host pin matches.
        ensure!(
            handshake.get_remote_static() == Some(self.invitation.host.as_slice()),
            "host identity does not match invitation"
        );
        let mut output = vec![0; HANDSHAKE_LIMIT];
        let length = handshake.write_message(&[], &mut output)?;
        output.truncate(length);
        ensure!(handshake.is_handshake_finished(), "handshake is incomplete");
        self.channel = Channel::Transport(Box::new(handshake.into_transport_mode()?));
        Ok(output)
    }

    fn encrypt(&mut self, input: &[u8], now: u64) -> Result<Vec<u8>> {
        let previous = std::mem::replace(&mut self.channel, Channel::Failed);
        self.invitation.live(now)?;
        ensure!(
            input.len() <= FRAME_LIMIT - 16,
            "application frame is too large"
        );
        let Channel::Transport(mut transport) = previous else {
            return Err(anyhow!("encrypted channel is not ready"));
        };
        let mut output = vec![0; input.len() + 16];
        let length = transport
            .write_message(input, &mut output)
            .map_err(|_| anyhow!("application encryption failed"))?;
        output.truncate(length);
        self.channel = Channel::Transport(transport);
        Ok(output)
    }

    fn decrypt(&mut self, input: &[u8], now: u64) -> Result<Vec<u8>> {
        let previous = std::mem::replace(&mut self.channel, Channel::Failed);
        self.invitation.live(now)?;
        ensure!(
            (16..=FRAME_LIMIT).contains(&input.len()),
            "invalid encrypted frame"
        );
        let Channel::Transport(mut transport) = previous else {
            return Err(anyhow!("encrypted channel is not ready"));
        };
        let mut output = Zeroizing::new(vec![0; input.len() - 16]);
        let length = transport
            .read_message(input, output.as_mut())
            .map_err(|_| anyhow!("application authentication failed; reconnect"))?;
        output.truncate(length);
        self.channel = Channel::Transport(transport);
        Ok(output.to_vec())
    }
}

/// A tab-local identity and invitation. Free this object on explicit disconnect.
#[wasm_bindgen]
pub struct BrowserPeer {
    core: Core,
}

#[wasm_bindgen]
impl BrowserPeer {
    /// Parse a fresh invitation against the page's configured HTTPS origin.
    ///
    /// # Errors
    /// Rejects malformed, expired, non-HTTPS or cross-origin invitations.
    #[wasm_bindgen(constructor)]
    pub fn new(text: &str, origin: &str, now_ms: u64) -> std::result::Result<Self, JsError> {
        Core::new(text, origin, now_ms)
            .map(|core| Self { core })
            .map_err(|_| {
                JsError::new("Invitation is invalid, expired, or belongs to another relay.")
            })
    }

    /// Return public pairing details. This never includes the invitation PSK.
    ///
    /// # Errors
    /// Returns an error if the public metadata cannot be encoded.
    pub fn metadata(&self) -> std::result::Result<String, JsError> {
        self.core
            .metadata()
            .map_err(|_| JsError::new("Cannot read public pairing details."))
    }

    /// Start a fresh Noise handshake while retaining the controller identity.
    ///
    /// # Errors
    /// Rejects expired shares or failure to initialize a fresh channel.
    pub fn begin(&mut self, now_ms: u64) -> std::result::Result<Vec<u8>, JsError> {
        self.core
            .begin(now_ms)
            .map_err(|_| JsError::new("Share expired or encryption could not start."))
    }

    /// Authenticate the host pin before writing handshake message three.
    ///
    /// # Errors
    /// Any invalid message, host substitution or expired share closes the channel.
    #[wasm_bindgen(js_name = acceptHandshake)]
    pub fn accept_handshake(
        &mut self,
        input: &[u8],
        now_ms: u64,
    ) -> std::result::Result<Vec<u8>, JsError> {
        self.core.accept(input, now_ms).map_err(|_| {
            JsError::new("Host authentication failed. Check the invitation and reconnect.")
        })
    }

    /// Encrypt one bounded product frame with Snow's ordered nonce.
    ///
    /// # Errors
    /// A missing, expired or failed channel and oversized frames are rejected.
    pub fn encrypt(&mut self, input: &[u8], now_ms: u64) -> std::result::Result<Vec<u8>, JsError> {
        self.core.encrypt(input, now_ms).map_err(|_| {
            JsError::new("Encrypted send failed. Reconnect to recover the action outcome.")
        })
    }

    /// Authenticate one bounded frame. A failure permanently closes this channel.
    ///
    /// # Errors
    /// Rejects invalid sizes, tampering, replay, expired or unavailable channels.
    pub fn decrypt(&mut self, input: &[u8], now_ms: u64) -> std::result::Result<Vec<u8>, JsError> {
        self.core.decrypt(input, now_ms).map_err(|_| {
            JsError::new("Encrypted frame authentication failed. Reconnect with a fresh channel.")
        })
    }
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "test fixtures should fail at the assertion site"
)]
mod tests;
