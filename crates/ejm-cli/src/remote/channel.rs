//! Ordered authenticated encryption through Snow's standard Noise transport state.

use anyhow::{Result, anyhow, ensure};
use snow::{Builder, HandshakeState, TransportState};
use zeroize::Zeroizing;

use super::{
    identity::{EndpointIdentity, fingerprint},
    invitation::{Enrollment, Invitation},
};

pub const NOISE_PATTERN: &str = "Noise_XXpsk3_25519_ChaChaPoly_SHA256";
pub const MAX_CIPHERTEXT_BYTES: usize = 16 * 1024;
pub const MAX_PLAINTEXT_BYTES: usize = MAX_CIPHERTEXT_BYTES - 16;
const MAX_HANDSHAKE_BYTES: usize = 256;

pub struct NoiseHandshake {
    state: HandshakeState,
    expected_host: Option<[u8; 32]>,
    prologue: Vec<u8>,
    expires_at_ms: u64,
    failed: bool,
}

impl NoiseHandshake {
    pub fn host(identity: &EndpointIdentity, invite: &Invitation, now_ms: u64) -> Result<Self> {
        invite.validate_share(now_ms)?;
        ensure!(
            identity.public_key() == invite.host_public_key(),
            "invitation host identity does not match"
        );
        Self::build(identity, invite, false)
    }

    pub fn controller(
        identity: &EndpointIdentity,
        invite: &Invitation,
        now_ms: u64,
    ) -> Result<Self> {
        invite.validate_share(now_ms)?;
        Self::build(identity, invite, true)
    }

    fn build(identity: &EndpointIdentity, invite: &Invitation, initiator: bool) -> Result<Self> {
        let prologue = invite.prologue();
        let builder = Builder::new(NOISE_PATTERN.parse()?)
            .local_private_key(identity.private_key())?
            .prologue(&prologue)?
            .psk(3, invite.psk())?;
        let state = if initiator {
            builder.build_initiator()?
        } else {
            builder.build_responder()?
        };
        Ok(Self {
            state,
            expected_host: initiator.then(|| invite.host_public_key()),
            prologue,
            expires_at_ms: invite.share_expires_at_ms(),
            failed: false,
        })
    }

    pub fn write_message(&mut self) -> Result<Vec<u8>> {
        ensure!(!self.failed, "encrypted handshake is closed");
        let mut output = vec![0; MAX_HANDSHAKE_BYTES];
        if let Ok(length) = self.state.write_message(&[], &mut output) {
            output.truncate(length);
            Ok(output)
        } else {
            self.failed = true;
            Err(anyhow!("encrypted handshake failed"))
        }
    }

    pub fn read_message(&mut self, message: &[u8]) -> Result<()> {
        ensure!(!self.failed, "encrypted handshake is closed");
        if message.len() > MAX_HANDSHAKE_BYTES {
            self.failed = true;
            return Err(anyhow!("encrypted handshake frame is too large"));
        }
        let mut payload = Zeroizing::new([0; MAX_HANDSHAKE_BYTES]);
        if !matches!(self.state.read_message(message, payload.as_mut()), Ok(0)) {
            self.failed = true;
            return Err(anyhow!("encrypted handshake failed"));
        }
        // Pin the host before writing the controller's static identity in message three.
        if let Some(expected) = self.expected_host
            && let Some(remote) = self.state.get_remote_static()
            && remote != expected
        {
            self.failed = true;
            return Err(anyhow!(
                "encrypted handshake host identity does not match invitation"
            ));
        }
        Ok(())
    }

    pub fn is_finished(&self) -> bool {
        !self.failed && self.state.is_handshake_finished()
    }

    pub fn finish_controller(self, invite: &Invitation, now_ms: u64) -> Result<EncryptedChannel> {
        ensure!(self.is_finished(), "encrypted handshake is incomplete");
        invite.validate_share(now_ms)?;
        ensure!(
            self.prologue == invite.prologue(),
            "invitation changed during handshake"
        );
        let remote = self.remote_key()?;
        ensure!(
            self.expected_host == Some(remote),
            "host identity does not match invitation"
        );
        self.into_channel(remote)
    }

    pub fn finish_host(self) -> Result<UnconfirmedChannel> {
        ensure!(self.is_finished(), "encrypted handshake is incomplete");
        ensure!(
            self.expected_host.is_none(),
            "controller cannot authorize itself as host"
        );
        let remote = self.remote_key()?;
        let prologue = self.prologue.clone();
        Ok(UnconfirmedChannel {
            channel: self.into_channel(remote)?,
            prologue,
        })
    }

    fn remote_key(&self) -> Result<[u8; 32]> {
        self.state
            .get_remote_static()
            .and_then(|key| key.try_into().ok())
            .ok_or_else(|| anyhow!("encrypted handshake has no authenticated peer identity"))
    }

    fn into_channel(self, remote: [u8; 32]) -> Result<EncryptedChannel> {
        Ok(EncryptedChannel {
            state: self.state.into_transport_mode()?,
            expires_at_ms: self.expires_at_ms,
            remote,
            failed: false,
        })
    }
}

/// This type cannot read or write application data before local host admission.
pub struct UnconfirmedChannel {
    channel: EncryptedChannel,
    prologue: Vec<u8>,
}

impl UnconfirmedChannel {
    pub fn remote_public_key(&self) -> [u8; 32] {
        self.channel.remote
    }

    pub fn fingerprint(&self) -> String {
        fingerprint(&self.channel.remote)
    }

    /// `locally_confirmed` must come from the host's local UI, never a wire message.
    pub fn authorize(
        self,
        enrollment: &mut Enrollment,
        locally_confirmed: bool,
        now_ms: u64,
    ) -> Result<EncryptedChannel> {
        ensure!(
            self.prologue == enrollment.invitation().prologue(),
            "sharing invitation changed"
        );
        enrollment.authorize(self.channel.remote, locally_confirmed, now_ms)?;
        Ok(self.channel)
    }
}

pub struct EncryptedChannel {
    state: TransportState,
    expires_at_ms: u64,
    remote: [u8; 32],
    failed: bool,
}

impl EncryptedChannel {
    pub fn encrypt(&mut self, plaintext: &[u8], now_ms: u64) -> Result<Vec<u8>> {
        self.check_live(now_ms)?;
        if plaintext.len() > MAX_PLAINTEXT_BYTES {
            self.failed = true;
            return Err(anyhow!("application frame is too large"));
        }
        let mut ciphertext = vec![0; plaintext.len() + 16];
        if let Ok(length) = self.state.write_message(plaintext, &mut ciphertext) {
            ciphertext.truncate(length);
            Ok(ciphertext)
        } else {
            self.failed = true;
            Err(anyhow!(
                "application encryption failed; reconnect with a fresh handshake"
            ))
        }
    }

    pub fn decrypt(&mut self, ciphertext: &[u8], now_ms: u64) -> Result<Zeroizing<Vec<u8>>> {
        self.check_live(now_ms)?;
        if !(16..=MAX_CIPHERTEXT_BYTES).contains(&ciphertext.len()) {
            self.failed = true;
            return Err(anyhow!("invalid encrypted application frame"));
        }
        let mut plaintext = Zeroizing::new(vec![0; ciphertext.len() - 16]);
        if let Ok(length) = self
            .state
            .read_message(ciphertext, plaintext.as_mut_slice())
        {
            plaintext.truncate(length);
            Ok(plaintext)
        } else {
            self.failed = true;
            Err(anyhow!(
                "application authentication failed; reconnect with a fresh handshake"
            ))
        }
    }

    fn check_live(&mut self, now_ms: u64) -> Result<()> {
        if now_ms >= self.expires_at_ms {
            self.failed = true;
        }
        ensure!(!self.failed, "encrypted channel has stopped or expired");
        Ok(())
    }
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "test fixtures should fail at the assertion site"
)]
mod tests {
    use super::super::{
        identity::RelayOrigin,
        invitation::{ENROLLMENT_LIFETIME_MS, ShareRole},
    };
    use super::*;

    #[derive(serde::Deserialize)]
    struct ProtocolVector {
        protocol_name: String,
        init_prologue: String,
        init_psks: Vec<String>,
        init_static: String,
        init_ephemeral: String,
        resp_prologue: String,
        resp_psks: Vec<String>,
        resp_static: String,
        resp_ephemeral: String,
        handshake_hash: String,
        messages: Vec<VectorMessage>,
    }

    #[derive(serde::Deserialize)]
    struct VectorMessage {
        payload: String,
        ciphertext: String,
    }

    fn unhex(input: &str) -> Vec<u8> {
        let (pairs, remainder) = input.as_bytes().as_chunks::<2>();
        assert_eq!(remainder, [] as [u8; 0]);
        pairs
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    #[test]
    fn noise_pattern_matches_upstream_cacophony_protocol_vector() {
        // Snow 0.10.0 tests/vectors/cacophony.txt, selected standard protocol vector.
        // Fixed ephemeral keys are confined to this known-answer fixture.
        let vector: ProtocolVector =
            serde_json::from_str(include_str!("channel_vector.json")).unwrap();
        assert_eq!(vector.protocol_name, NOISE_PATTERN);
        let init_static = unhex(&vector.init_static);
        let init_ephemeral = unhex(&vector.init_ephemeral);
        let init_prologue = unhex(&vector.init_prologue);
        let init_psk: [u8; 32] = unhex(&vector.init_psks[0]).try_into().unwrap();
        let resp_static = unhex(&vector.resp_static);
        let resp_ephemeral = unhex(&vector.resp_ephemeral);
        let resp_prologue = unhex(&vector.resp_prologue);
        let resp_psk: [u8; 32] = unhex(&vector.resp_psks[0]).try_into().unwrap();
        let mut initiator = Builder::new(NOISE_PATTERN.parse().unwrap())
            .local_private_key(&init_static)
            .unwrap()
            .fixed_ephemeral_key_for_testing_only(&init_ephemeral)
            .prologue(&init_prologue)
            .unwrap()
            .psk(3, &init_psk)
            .unwrap()
            .build_initiator()
            .unwrap();
        let mut responder = Builder::new(NOISE_PATTERN.parse().unwrap())
            .local_private_key(&resp_static)
            .unwrap()
            .fixed_ephemeral_key_for_testing_only(&resp_ephemeral)
            .prologue(&resp_prologue)
            .unwrap()
            .psk(3, &resp_psk)
            .unwrap()
            .build_responder()
            .unwrap();
        let mut write = [0_u8; 1024];
        let mut read = [0_u8; 1024];
        for (index, message) in vector.messages[..3].iter().enumerate() {
            let (sender, receiver) = if index % 2 == 0 {
                (&mut initiator, &mut responder)
            } else {
                (&mut responder, &mut initiator)
            };
            let plaintext = unhex(&message.payload);
            let length = sender.write_message(&plaintext, &mut write).unwrap();
            assert_eq!(&write[..length], unhex(&message.ciphertext));
            let length = receiver.read_message(&write[..length], &mut read).unwrap();
            assert_eq!(&read[..length], plaintext);
        }
        assert_eq!(
            initiator.get_handshake_hash(),
            unhex(&vector.handshake_hash)
        );
        assert_eq!(
            responder.get_handshake_hash(),
            unhex(&vector.handshake_hash)
        );
        let mut initiator = initiator.into_transport_mode().unwrap();
        let mut responder = responder.into_transport_mode().unwrap();
        for (index, message) in vector.messages[3..].iter().enumerate() {
            let (sender, receiver) = if index % 2 == 0 {
                (&mut responder, &mut initiator)
            } else {
                (&mut initiator, &mut responder)
            };
            let plaintext = unhex(&message.payload);
            let length = sender.write_message(&plaintext, &mut write).unwrap();
            assert_eq!(&write[..length], unhex(&message.ciphertext));
            let length = receiver.read_message(&write[..length], &mut read).unwrap();
            assert_eq!(&read[..length], plaintext);
        }
    }

    fn invite(host: &EndpointIdentity) -> Invitation {
        Invitation::new(
            RelayOrigin::parse("https://relay.example").unwrap(),
            host.public_key(),
            ShareRole::Control,
            1000,
        )
        .unwrap()
    }

    fn handshake(
        host: &EndpointIdentity,
        controller: &EndpointIdentity,
        invitation: &Invitation,
        now: u64,
    ) -> (NoiseHandshake, NoiseHandshake) {
        let mut host = NoiseHandshake::host(host, invitation, now).unwrap();
        let mut controller = NoiseHandshake::controller(controller, invitation, now).unwrap();
        host.read_message(&controller.write_message().unwrap())
            .unwrap();
        controller
            .read_message(&host.write_message().unwrap())
            .unwrap();
        host.read_message(&controller.write_message().unwrap())
            .unwrap();
        assert!(host.is_finished());
        assert!(controller.is_finished());
        (host, controller)
    }

    #[test]
    fn first_connection_requires_local_confirmation_then_encrypts_both_directions() {
        let host = EndpointIdentity::memory_only().unwrap();
        let controller = EndpointIdentity::memory_only().unwrap();
        let invitation = invite(&host);
        let (host_handshake, controller_handshake) =
            handshake(&host, &controller, &invitation, 1001);
        let mut sender = controller_handshake
            .finish_controller(&invitation, 1001)
            .unwrap();
        let mut enrollment = Enrollment::new(invitation);
        let unconfirmed = host_handshake.finish_host().unwrap();
        assert_eq!(unconfirmed.remote_public_key(), controller.public_key());
        let mut receiver = unconfirmed.authorize(&mut enrollment, true, 1001).unwrap();
        let ciphertext = sender.encrypt(b"sentinel-private-command", 1002).unwrap();
        assert!(!ciphertext.windows(8).any(|part| part == b"sentinel"));
        assert_eq!(
            &**receiver.decrypt(&ciphertext, 1002).unwrap(),
            b"sentinel-private-command"
        );
        let answer = receiver.encrypt(b"private response", 1002).unwrap();
        assert_eq!(
            &**sender.decrypt(&answer, 1002).unwrap(),
            b"private response"
        );
    }

    #[test]
    fn missing_local_confirmation_and_controller_substitution_release_no_data() {
        let host = EndpointIdentity::memory_only().unwrap();
        let first = EndpointIdentity::memory_only().unwrap();
        let other = EndpointIdentity::memory_only().unwrap();
        let mut enrollment = Enrollment::new(invite(&host));
        let (h, _) = handshake(&host, &first, enrollment.invitation(), 1001);
        assert!(
            h.finish_host()
                .unwrap()
                .authorize(&mut enrollment, false, 1001)
                .is_err()
        );
        let (h, _) = handshake(&host, &first, enrollment.invitation(), 1001);
        let _channel = h
            .finish_host()
            .unwrap()
            .authorize(&mut enrollment, true, 1001)
            .unwrap();
        let (h, _) = handshake(&host, &other, enrollment.invitation(), 1002);
        assert!(
            h.finish_host()
                .unwrap()
                .authorize(&mut enrollment, true, 1002)
                .is_err()
        );
    }

    #[test]
    fn tamper_and_ciphertext_replay_poison_the_channel() {
        for tamper in [false, true] {
            let host = EndpointIdentity::memory_only().unwrap();
            let controller = EndpointIdentity::memory_only().unwrap();
            let invitation = invite(&host);
            let (h, c) = handshake(&host, &controller, &invitation, 1001);
            let mut sender = c.finish_controller(&invitation, 1001).unwrap();
            let mut enrollment = Enrollment::new(invitation);
            let mut receiver = h
                .finish_host()
                .unwrap()
                .authorize(&mut enrollment, true, 1001)
                .unwrap();
            let mut frame = sender.encrypt(b"do once", 1001).unwrap();
            if tamper {
                frame[0] ^= 1;
            } else {
                receiver.decrypt(&frame, 1001).unwrap();
            }
            assert!(receiver.decrypt(&frame, 1001).is_err());
            assert!(
                receiver
                    .decrypt(&sender.encrypt(b"later", 1002).unwrap(), 1002)
                    .is_err()
            );
        }
    }

    #[test]
    fn reconnect_has_fresh_ephemeral_keys_and_old_ciphertext_is_rejected() {
        let host = EndpointIdentity::memory_only().unwrap();
        let controller = EndpointIdentity::memory_only().unwrap();
        let invitation = invite(&host);
        let (h, c) = handshake(&host, &controller, &invitation, 1001);
        let mut old_sender = c.finish_controller(&invitation, 1001).unwrap();
        let mut enrollment = Enrollment::new(invitation);
        let _old_receiver = h
            .finish_host()
            .unwrap()
            .authorize(&mut enrollment, true, 1001)
            .unwrap();
        let old_ciphertext = old_sender.encrypt(b"old mutation", 1001).unwrap();
        let now = 1001 + ENROLLMENT_LIFETIME_MS;
        let (h, c) = handshake(&host, &controller, enrollment.invitation(), now);
        let mut new_sender = c.finish_controller(enrollment.invitation(), now).unwrap();
        let mut new_receiver = h
            .finish_host()
            .unwrap()
            .authorize(&mut enrollment, false, now)
            .unwrap();
        assert_ne!(
            new_sender.encrypt(b"old mutation", now).unwrap(),
            old_ciphertext
        );
        assert!(new_receiver.decrypt(&old_ciphertext, now).is_err());
    }

    #[test]
    fn wrong_psk_role_or_host_key_cannot_complete_authenticated_handshake() {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        for field in ["psk", "role", "host_public_key", "version"] {
            let host_id = EndpointIdentity::memory_only().unwrap();
            let controller_id = EndpointIdentity::memory_only().unwrap();
            let original = invite(&host_id);
            let text = original.protected_text().unwrap();
            let mut raw: serde_json::Value = serde_json::from_slice(
                &URL_SAFE_NO_PAD
                    .decode(text.strip_prefix("ejm-remote:v1:").unwrap())
                    .unwrap(),
            )
            .unwrap();
            raw[field] = match field {
                "role" => serde_json::json!("view"),
                "version" => serde_json::json!(0),
                _ => serde_json::json!(URL_SAFE_NO_PAD.encode([99; 32])),
            };
            let changed_text = format!(
                "ejm-remote:v1:{}",
                URL_SAFE_NO_PAD.encode(serde_json::to_vec(&raw).unwrap())
            );
            let changed = Invitation::decode(&changed_text, original.origin(), 1001);
            if field == "version" {
                assert!(changed.is_err());
                continue;
            }
            let changed = changed.unwrap();
            let mut h = NoiseHandshake::host(&host_id, &original, 1001).unwrap();
            let mut c = NoiseHandshake::controller(&controller_id, &changed, 1001).unwrap();
            let first = h.read_message(&c.write_message().unwrap());
            if field != "psk" {
                // PSK patterns authenticate the invitation prologue in message one.
                assert!(first.is_err());
                continue;
            }
            first.unwrap();
            c.read_message(&h.write_message().unwrap()).unwrap();
            assert!(h.read_message(&c.write_message().unwrap()).is_err());
        }
    }

    #[test]
    fn expiry_and_frame_bounds_stop_application_traffic() {
        let host = EndpointIdentity::memory_only().unwrap();
        let controller = EndpointIdentity::memory_only().unwrap();
        let invitation = invite(&host);
        let expiry = invitation.share_expires_at_ms();
        let (_, c) = handshake(&host, &controller, &invitation, 1001);
        let mut channel = c.finish_controller(&invitation, 1001).unwrap();
        assert!(channel.encrypt(b"late", expiry).is_err());
        let (_, c) = handshake(&host, &controller, &invitation, 1001);
        let mut channel = c.finish_controller(&invitation, 1001).unwrap();
        assert!(
            channel
                .encrypt(&vec![0; MAX_PLAINTEXT_BYTES + 1], 1001)
                .is_err()
        );
    }
}
