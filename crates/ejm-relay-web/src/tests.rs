use super::*;

const NOW: u64 = 1000;

fn fixture(role: &str) -> (Core, Vec<u8>, String) {
    let host = Builder::new(PATTERN.parse().unwrap())
        .generate_keypair()
        .unwrap();
    let text = format!(
        "ejm-remote:v1:{}",
        URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&serde_json::json!({
                "version": 1, "origin": "https://relay.example/",
                "routing_id": URL_SAFE_NO_PAD.encode([3; 32]),
                "host_public_key": URL_SAFE_NO_PAD.encode(&host.public),
                "psk": URL_SAFE_NO_PAD.encode([7; 32]), "role": role,
                "issued_at_ms": NOW, "enrollment_expires_at_ms": NOW + ENROLLMENT_MS,
                "share_expires_at_ms": NOW + SHARE_MS,
            }))
            .unwrap()
        )
    );
    (
        Core::new(&text, "https://relay.example", NOW).unwrap(),
        host.private,
        text,
    )
}

fn connect(peer: &mut Core, host_key: &[u8], now: u64) -> TransportState {
    let prologue = peer.invitation.prologue();
    let mut host = Builder::new(PATTERN.parse().unwrap())
        .local_private_key(host_key)
        .unwrap()
        .prologue(&prologue)
        .unwrap()
        .psk(3, &peer.invitation.psk)
        .unwrap()
        .build_responder()
        .unwrap();
    let mut buffer = [0; 256];
    let first = peer.begin(now).unwrap();
    assert_eq!(host.read_message(&first, &mut buffer).unwrap(), 0);
    let len = host.write_message(&[], &mut buffer).unwrap();
    let third = peer.accept(&buffer[..len], now).unwrap();
    assert_eq!(host.read_message(&third, &mut buffer).unwrap(), 0);
    assert_eq!(host.get_remote_static(), Some(peer.public.as_slice()));
    host.into_transport_mode().unwrap()
}

#[test]
fn native_pattern_and_prologue_exchange_ciphertext_both_directions() {
    for role in ["view", "control"] {
        let (mut peer, key, _) = fixture(role);
        let expected = format!(
            "ejm-cli/remote/1\nhttps://relay.example/\n{}\n{}\n{role}\n301000\n28801000\ninitiator=controller\nresponder=host",
            URL_SAFE_NO_PAD.encode([3; 32]),
            URL_SAFE_NO_PAD.encode(peer.invitation.host),
        );
        assert_eq!(peer.invitation.prologue(), expected.as_bytes());
        let mut host = connect(&mut peer, &key, NOW);
        let encrypted = peer.encrypt(b"private-browser-command", NOW).unwrap();
        assert!(!encrypted.windows(7).any(|part| part == b"private"));
        let mut plain = [0; 128];
        let len = host.read_message(&encrypted, &mut plain).unwrap();
        assert_eq!(&plain[..len], b"private-browser-command");
        let mut cipher = [0; 128];
        let len = host
            .write_message(b"private-host-output", &mut cipher)
            .unwrap();
        assert_eq!(
            peer.decrypt(&cipher[..len], NOW).unwrap(),
            b"private-host-output"
        );
        let metadata: serde_json::Value = serde_json::from_str(&peer.metadata().unwrap()).unwrap();
        assert_eq!(metadata["role"], role);
        assert!(metadata.get("psk").is_none());
        assert_eq!(
            metadata["controllerFingerprint"]
                .as_str()
                .unwrap()
                .split(' ')
                .count(),
            16
        );
    }
}

#[test]
fn pins_host_before_releasing_third_handshake_message() {
    let (mut peer, _, _) = fixture("control");
    let other = Builder::new(PATTERN.parse().unwrap())
        .generate_keypair()
        .unwrap();
    let prologue = peer.invitation.prologue();
    let mut host = Builder::new(PATTERN.parse().unwrap())
        .local_private_key(&other.private)
        .unwrap()
        .prologue(&prologue)
        .unwrap()
        .psk(3, &peer.invitation.psk)
        .unwrap()
        .build_responder()
        .unwrap();
    let mut buffer = [0; 256];
    host.read_message(&peer.begin(NOW).unwrap(), &mut buffer)
        .unwrap();
    let len = host.write_message(&[], &mut buffer).unwrap();
    assert!(peer.accept(&buffer[..len], NOW).is_err());
    assert!(peer.encrypt(b"must not send", NOW).is_err());
    assert!(peer.accept(&buffer[..len], NOW).is_err());
}

#[test]
fn reconnect_retains_identity_uses_fresh_handshake_and_rejects_old_ciphertext() {
    let (mut peer, key, text) = fixture("control");
    let identity = peer.public.clone();
    let mut host = connect(&mut peer, &key, NOW);
    let mut cipher = [0; 128];
    let len = host.write_message(b"old output", &mut cipher).unwrap();
    let old = cipher[..len].to_vec();
    let _host = connect(&mut peer, &key, NOW + ENROLLMENT_MS);
    assert_eq!(peer.public, identity);
    assert!(peer.decrypt(&old, NOW + ENROLLMENT_MS).is_err());
    assert!(peer.decrypt(&old, NOW + ENROLLMENT_MS).is_err());
    let reloaded = Core::new(&text, "https://relay.example", NOW).unwrap();
    assert_ne!(reloaded.public, identity);
    assert!(Core::new(&text, "https://relay.example", NOW + ENROLLMENT_MS).is_err());
    let first = peer.begin(NOW + ENROLLMENT_MS).unwrap();
    let second = peer.begin(NOW + ENROLLMENT_MS).unwrap();
    assert_ne!(first, second);
}

#[test]
fn tampering_replay_size_and_expiry_close_transport_until_fresh_handshake() {
    for mode in ["tamper", "replay", "size", "expiry"] {
        let (mut peer, key, _) = fixture("view");
        let mut host = connect(&mut peer, &key, NOW);
        let mut cipher = [0; 128];
        let len = host.write_message(b"private text", &mut cipher).unwrap();
        let mut message = cipher[..len].to_vec();
        match mode {
            "tamper" => message[0] ^= 1,
            "replay" => {
                peer.decrypt(&message, NOW).unwrap();
            }
            "size" => message.resize(FRAME_LIMIT + 1, 0),
            _ => {}
        }
        let now = if mode == "expiry" {
            NOW + SHARE_MS
        } else {
            NOW
        };
        assert!(peer.decrypt(&message, now).is_err());
        assert!(peer.encrypt(b"closed", NOW).is_err());
    }
}

#[test]
fn invitation_rejects_cross_origin_unknown_fields_time_changes_and_invalid_keys() {
    let (_, _, text) = fixture("control");
    for origin in [
        "https://other.example",
        "http://relay.example",
        "https://relay.example/path",
    ] {
        assert!(Core::new(&text, origin, NOW).is_err());
    }
    let raw = URL_SAFE_NO_PAD
        .decode(text.strip_prefix("ejm-remote:v1:").unwrap())
        .unwrap();
    let original: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    for (field, value) in [
        ("version", serde_json::json!(2)),
        (
            "origin",
            serde_json::json!("https://user:pass@relay.example/"),
        ),
        (
            "enrollment_expires_at_ms",
            serde_json::json!(NOW + ENROLLMENT_MS + 1),
        ),
        ("share_expires_at_ms", serde_json::json!(NOW + SHARE_MS + 1)),
        ("routing_id", serde_json::json!("bad")),
        ("psk", serde_json::json!("bad")),
        ("extra", serde_json::json!(true)),
    ] {
        let mut raw = original.clone();
        raw[field] = value;
        let encoded = format!(
            "ejm-remote:v1:{}",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&raw).unwrap())
        );
        assert!(
            Core::new(&encoded, "https://relay.example", NOW).is_err(),
            "field {field}"
        );
    }
    assert!(Core::new(&text, "https://relay.example", NOW - 1).is_err());
    assert!(Core::new(&text.repeat(4), "https://relay.example", NOW).is_err());
}
