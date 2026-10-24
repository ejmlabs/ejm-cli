//! Invitations are local secrets, not relay credentials or service requests.

use std::fmt;

use anyhow::{Result, anyhow, ensure};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use super::identity::RelayOrigin;

pub const PROTOCOL_VERSION: u16 = 1;
pub const ENROLLMENT_LIFETIME_MS: u64 = 5 * 60 * 1000;
pub const SHARE_LIFETIME_MS: u64 = 8 * 60 * 60 * 1000;
const INVITATION_PREFIX: &str = "ejm-remote:v1:";
const MAX_INVITATION_BYTES: usize = 2048;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShareRole {
    #[default]
    View,
    Control,
}

pub struct Invitation {
    origin: RelayOrigin,
    routing_id: String,
    host_public_key: [u8; 32],
    psk: Zeroizing<[u8; 32]>,
    role: ShareRole,
    issued_at_ms: u64,
    enrollment_expires_at_ms: u64,
    share_expires_at_ms: u64,
}

impl fmt::Debug for Invitation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Invitation")
            .field("origin", &self.origin)
            .field("role", &self.role)
            .field("enrollment_expires_at_ms", &self.enrollment_expires_at_ms)
            .field("share_expires_at_ms", &self.share_expires_at_ms)
            .finish_non_exhaustive()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InvitationText {
    version: u16,
    origin: String,
    routing_id: String,
    host_public_key: String,
    psk: String,
    role: ShareRole,
    issued_at_ms: u64,
    enrollment_expires_at_ms: u64,
    share_expires_at_ms: u64,
}

impl Drop for InvitationText {
    fn drop(&mut self) {
        self.psk.zeroize();
    }
}

impl Invitation {
    pub fn new(
        origin: RelayOrigin,
        host_public_key: [u8; 32],
        role: ShareRole,
        now_ms: u64,
    ) -> Result<Self> {
        let mut psk = Zeroizing::new([0_u8; 32]);
        let mut routing_id = [0_u8; 32];
        getrandom::fill(psk.as_mut())
            .map_err(|_| anyhow!("operating system random source is unavailable"))?;
        getrandom::fill(&mut routing_id)
            .map_err(|_| anyhow!("operating system random source is unavailable"))?;
        Ok(Self {
            origin,
            routing_id: URL_SAFE_NO_PAD.encode(routing_id),
            host_public_key,
            psk,
            role,
            issued_at_ms: now_ms,
            enrollment_expires_at_ms: now_ms
                .checked_add(ENROLLMENT_LIFETIME_MS)
                .ok_or_else(|| anyhow!("invitation time is out of range"))?,
            share_expires_at_ms: now_ms
                .checked_add(SHARE_LIFETIME_MS)
                .ok_or_else(|| anyhow!("sharing time is out of range"))?,
        })
    }

    /// The caller must display this only in its protected alternate screen.
    /// There is deliberately no Display implementation or serializable Invitation.
    pub fn protected_text(&self) -> Result<Zeroizing<String>> {
        let raw = InvitationText {
            version: PROTOCOL_VERSION,
            origin: self.origin.as_str().into(),
            routing_id: self.routing_id.clone(),
            host_public_key: URL_SAFE_NO_PAD.encode(self.host_public_key),
            psk: URL_SAFE_NO_PAD.encode(self.psk.as_ref()),
            role: self.role,
            issued_at_ms: self.issued_at_ms,
            enrollment_expires_at_ms: self.enrollment_expires_at_ms,
            share_expires_at_ms: self.share_expires_at_ms,
        };
        let json = Zeroizing::new(
            serde_json::to_vec(&raw).map_err(|_| anyhow!("cannot encode local invitation"))?,
        );
        let encoded = Zeroizing::new(URL_SAFE_NO_PAD.encode(json.as_slice()));
        Ok(Zeroizing::new(format!(
            "{INVITATION_PREFIX}{}",
            encoded.as_str()
        )))
    }

    /// The host enforces first enrollment; a pinned controller may restart during the share.
    pub fn decode(input: &str, configured: &RelayOrigin, now_ms: u64) -> Result<Self> {
        ensure!(input.len() <= MAX_INVITATION_BYTES, "invalid invitation");
        let encoded = input
            .trim()
            .strip_prefix(INVITATION_PREFIX)
            .ok_or_else(|| anyhow!("unsupported invitation version or invalid invitation"))?;
        let json = Zeroizing::new(
            URL_SAFE_NO_PAD
                .decode(encoded)
                .map_err(|_| anyhow!("invalid invitation"))?,
        );
        let raw: InvitationText =
            serde_json::from_slice(&json).map_err(|_| anyhow!("invalid invitation"))?;
        ensure!(
            raw.version == PROTOCOL_VERSION,
            "unsupported invitation version"
        );
        let origin = parse_invitation_origin(&raw.origin)?;
        ensure!(
            &origin == configured,
            "invitation belongs to a different relay origin"
        );
        ensure!(
            decode_key(&raw.routing_id).is_ok()
                && raw.enrollment_expires_at_ms.checked_sub(raw.issued_at_ms)
                    == Some(ENROLLMENT_LIFETIME_MS)
                && raw.share_expires_at_ms.checked_sub(raw.issued_at_ms) == Some(SHARE_LIFETIME_MS),
            "invalid invitation bounds"
        );
        let invitation = Self {
            origin,
            routing_id: raw.routing_id.clone(),
            host_public_key: decode_key(&raw.host_public_key)?,
            psk: Zeroizing::new(decode_key(&raw.psk)?),
            role: raw.role,
            issued_at_ms: raw.issued_at_ms,
            enrollment_expires_at_ms: raw.enrollment_expires_at_ms,
            share_expires_at_ms: raw.share_expires_at_ms,
        };
        invitation.validate_share(now_ms)?;
        Ok(invitation)
    }

    pub fn origin(&self) -> &RelayOrigin {
        &self.origin
    }

    pub fn routing_id(&self) -> &str {
        &self.routing_id
    }

    pub fn host_public_key(&self) -> [u8; 32] {
        self.host_public_key
    }

    pub fn role(&self) -> ShareRole {
        self.role
    }

    pub fn enrollment_expires_at_ms(&self) -> u64 {
        self.enrollment_expires_at_ms
    }

    pub fn share_expires_at_ms(&self) -> u64 {
        self.share_expires_at_ms
    }

    pub fn validate_initial(&self, now_ms: u64) -> Result<()> {
        self.validate_share(now_ms)?;
        ensure!(
            now_ms < self.enrollment_expires_at_ms,
            "invitation has expired"
        );
        Ok(())
    }

    pub fn validate_share(&self, now_ms: u64) -> Result<()> {
        ensure!(
            now_ms.saturating_add(60_000) >= self.issued_at_ms && now_ms < self.share_expires_at_ms,
            "sharing session has expired or the local clock is too far before its creation"
        );
        Ok(())
    }

    pub(super) fn psk(&self) -> &[u8; 32] {
        &self.psk
    }

    pub(super) fn prologue(&self) -> Vec<u8> {
        // These values are locally known by both peers; none are a relay handshake field.
        let role = match self.role {
            ShareRole::View => "view",
            ShareRole::Control => "control",
        };
        format!(
            "ejm-cli/remote/{PROTOCOL_VERSION}\n{}\n{}\n{}\n{role}\n{}\n{}\ninitiator=controller\nresponder=host",
            self.origin.as_str(),
            self.routing_id,
            URL_SAFE_NO_PAD.encode(self.host_public_key),
            self.enrollment_expires_at_ms,
            self.share_expires_at_ms,
        )
        .into_bytes()
    }
}

fn parse_invitation_origin(input: &str) -> Result<RelayOrigin> {
    let origin = RelayOrigin::parse(input);
    #[cfg(test)]
    let origin = origin.or_else(|_| RelayOrigin::loopback_for_test(input));
    origin.map_err(|_| anyhow!("invalid invitation origin"))
}

fn decode_key(input: &str) -> Result<[u8; 32]> {
    ensure!(input.len() == 43, "invalid invitation key");
    let bytes = Zeroizing::new(
        URL_SAFE_NO_PAD
            .decode(input)
            .map_err(|_| anyhow!("invalid invitation key"))?,
    );
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("invalid invitation key"))
}

/// Pins one locally confirmed controller for the life of this in-memory share.
pub struct Enrollment {
    invitation: Invitation,
    controller: Option<[u8; 32]>,
    revoked: bool,
}

impl Enrollment {
    pub fn new(invitation: Invitation) -> Self {
        Self {
            invitation,
            controller: None,
            revoked: false,
        }
    }

    pub fn invitation(&self) -> &Invitation {
        &self.invitation
    }

    pub fn is_enrolled(&self) -> bool {
        self.controller.is_some()
    }

    pub fn controller_key(&self) -> Option<[u8; 32]> {
        self.controller
    }

    pub fn revoke(&mut self) {
        self.revoked = true;
        self.invitation.psk.zeroize();
    }

    pub fn validate_handshake(&self, now_ms: u64) -> Result<()> {
        ensure!(!self.revoked, "sharing has stopped");
        if self.controller.is_some() {
            self.invitation.validate_share(now_ms)
        } else {
            self.invitation.validate_initial(now_ms)
        }
    }

    pub(super) fn authorize(
        &mut self,
        controller: [u8; 32],
        locally_confirmed: bool,
        now_ms: u64,
    ) -> Result<()> {
        self.validate_handshake(now_ms)?;
        if let Some(pinned) = self.controller {
            ensure!(
                pinned == controller,
                "this invitation has already enrolled another controller"
            );
        } else {
            ensure!(
                locally_confirmed,
                "the host must locally confirm the controller fingerprint"
            );
            self.controller = Some(controller);
        }
        Ok(())
    }
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "test fixtures should fail at the assertion site"
)]
mod tests {
    use super::*;

    fn invitation() -> Invitation {
        Invitation::new(
            RelayOrigin::parse("https://relay.example").unwrap(),
            [7; 32],
            ShareRole::View,
            1000,
        )
        .unwrap()
    }

    #[test]
    fn invitation_roundtrip_binds_origin_and_has_redacted_debug() {
        let invite = invitation();
        let text = invite.protected_text().unwrap();
        let restored = Invitation::decode(&text, invite.origin(), 1001).unwrap();
        assert_eq!(restored.routing_id(), invite.routing_id());
        assert_eq!(restored.psk(), invite.psk());
        assert_eq!(restored.role(), ShareRole::View);
        assert!(!format!("{invite:?}").contains(&URL_SAFE_NO_PAD.encode(invite.psk())));
        assert!(
            Invitation::decode(
                &text,
                &RelayOrigin::parse("https://other.example").unwrap(),
                1001
            )
            .is_err()
        );
        assert!(Invitation::decode(&text, invite.origin(), invite.share_expires_at_ms()).is_err());
        assert!(
            Invitation::decode(&text.replacen(":v1:", ":v0:", 1), invite.origin(), 1001).is_err()
        );
    }

    #[test]
    fn clock_tolerance_does_not_extend_host_enrollment_or_share_expiry() {
        let invite = Invitation::new(
            RelayOrigin::parse("https://relay.example").unwrap(),
            [7; 32],
            ShareRole::View,
            100_000,
        )
        .unwrap();
        let text = invite.protected_text().unwrap();
        assert!(Invitation::decode(&text, invite.origin(), 99_500).is_ok());
        assert!(Invitation::decode(&text, invite.origin(), 39_999).is_err());
        assert!(
            invite
                .validate_initial(invite.enrollment_expires_at_ms())
                .is_err()
        );
        assert!(invite.validate_share(invite.share_expires_at_ms()).is_err());
    }

    #[test]
    fn first_enrollment_requires_local_confirmation_and_cannot_change_peer() {
        let mut enrollment = Enrollment::new(invitation());
        let text = enrollment.invitation().protected_text().unwrap();
        let after_invite = enrollment.invitation().enrollment_expires_at_ms() + 1;
        let parsed =
            Invitation::decode(&text, enrollment.invitation().origin(), after_invite).unwrap();
        let mut late = Enrollment::new(parsed);
        assert!(late.validate_handshake(after_invite).is_err());
        assert!(late.authorize([1; 32], true, after_invite).is_err());
        assert!(enrollment.authorize([1; 32], false, 1001).is_err());
        assert!(!enrollment.is_enrolled());
        enrollment.authorize([1; 32], true, 1001).unwrap();
        enrollment.authorize([1; 32], false, after_invite).unwrap();
        assert!(enrollment.authorize([2; 32], true, after_invite).is_err());
        let share_end = enrollment.invitation().share_expires_at_ms();
        assert!(enrollment.authorize([1; 32], false, share_end).is_err());
        enrollment.revoke();
        assert!(enrollment.authorize([1; 32], true, 1002).is_err());
        assert_eq!(enrollment.invitation().psk(), &[0; 32]);
    }

    #[test]
    fn random_invitations_have_distinct_opaque_ids_and_256_bit_secrets() {
        let one = invitation();
        let two = invitation();
        assert_ne!(one.psk(), two.psk());
        assert_ne!(one.routing_id(), two.routing_id());
        assert_eq!(decode_key(one.routing_id()).unwrap().len(), 32);
        assert!(
            Invitation::decode(&"x".repeat(MAX_INVITATION_BYTES + 1), one.origin(), 1001).is_err()
        );
    }
}
