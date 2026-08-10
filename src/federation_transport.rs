//! Shared plumbing for the federation client and server halves.
//!
//! Both ends need the same three answers — which keys are accepted, whether a
//! transfer is sealed, and with what passphrase — so they live here rather
//! than being decided twice and drifting apart.

use zeroize::Zeroizing;

use crate::config::{FederationPeerSettings, FederationSettings};
use crate::error::{Result, VaultError};

/// Environment variable holding the shared sealing passphrase.
pub const SEAL_PASSPHRASE_ENV: &str = "IRONVAULT_FEDERATION_PASSPHRASE";

/// Header carrying the peer's shared key.
pub const API_KEY_HEADER: &str = "X-API-Key";

/// Environment variable holding the vault passphrase, mirroring the CLI's.
pub const VAULT_PASSPHRASE_ENV: &str = "IRONVAULT_PASSPHRASE";

/// Version-metadata key recording the checkpoint id a model arrived with.
///
/// Federation identifies versions by checkpoint id, but `add_version` mints a
/// fresh one from the model name, local version number, and current time. A
/// received copy therefore gets an id the sender has never heard of, so the
/// next sync sees the sender's version as still missing and transfers it
/// again — every run duplicating the model on both nodes, forever.
///
/// Storing the origin id here and advertising *it* in the manifest gives a
/// version one identity across the federation, which is what makes sync
/// converge. Kept in metadata rather than overwriting `checkpoint_id` so the
/// local id stays locally unique and neither version backend has to change.
pub const ORIGIN_CHECKPOINT_KEY: &str = "federation_origin_checkpoint_id";

/// The federation-wide identity of a version: its origin id if it came from a
/// peer, otherwise its own.
#[must_use]
pub fn federation_checkpoint_id(version: &crate::version::ModelVersion) -> String {
    version
        .metadata
        .get(ORIGIN_CHECKPOINT_KEY)
        .cloned()
        .unwrap_or_else(|| version.checkpoint_id.clone())
}

/// Metadata marking a model as received from a peer under `origin_id`.
#[must_use]
pub fn origin_metadata(origin_id: &str) -> std::collections::HashMap<String, String> {
    let mut fields = std::collections::HashMap::new();
    fields.insert(ORIGIN_CHECKPOINT_KEY.to_string(), origin_id.to_string());
    fields
}

/// The vault passphrase for unattended startup, if one is available.
///
/// A federating server has to serve model bytes to peers that hold only the
/// shared federation key, never the vault passphrase. Without this the vault
/// stays locked until a human POSTs to `/auth/token`, so every peer request
/// fails until someone logs in by hand — which defeats the point of a daemon.
///
/// Accepts a literal or a KMS URI, same as the CLI. Returns `None` when unset,
/// leaving the vault locked rather than guessing.
pub fn startup_passphrase() -> Result<Option<Zeroizing<String>>> {
    let Some(value) = crate::env::var_secret(VAULT_PASSPHRASE_ENV) else {
        return Ok(None);
    };
    crate::kms::resolve(&value).map(Some)
}

/// Resolve a peer's configured key to its literal value.
///
/// Accepts a KMS URI (`env://`, `file://`, `aws-sm://`, `azure-kv://`,
/// `vault://`) or a literal. Returning `Zeroizing` so the secret is wiped when
/// the caller drops it.
pub fn resolve_peer_key(peer: &FederationPeerSettings) -> Result<Option<Zeroizing<String>>> {
    match &peer.api_key {
        Some(raw) => Ok(Some(crate::kms::resolve(raw)?)),
        None => Ok(None),
    }
}

/// Every key this node accepts on inbound federation requests.
///
/// Drawn from the peer list: the same shared secret authenticates both
/// directions, so configuring a peer to call out also allows it to call in.
/// Disabled peers are excluded — turning a peer off must close the door in
/// both directions, not just stop us dialling it.
pub fn accepted_keys(settings: &FederationSettings) -> Result<Vec<Zeroizing<String>>> {
    let mut keys = Vec::new();
    for peer in settings.peers.iter().filter(|p| p.enabled) {
        if let Some(key) = resolve_peer_key(peer)? {
            if !key.is_empty() {
                keys.push(key);
            }
        }
    }
    Ok(keys)
}

/// Build the runtime [`FederationConfig`](crate::federation::FederationConfig)
/// from on-disk settings, resolving each peer's key through the KMS layer.
///
/// Peers with unresolvable keys are an error rather than a skip: silently
/// dropping a peer would look like a quiet sync that simply found nothing.
pub fn to_manager_config(
    settings: &FederationSettings,
) -> Result<crate::federation::FederationConfig> {
    let mut peers = Vec::with_capacity(settings.peers.len());
    for peer in &settings.peers {
        let key = resolve_peer_key(peer)?;
        peers.push(crate::federation::PeerConfig {
            node_id: peer.node_id.clone(),
            name: peer.name.clone(),
            endpoint: peer.endpoint.trim_end_matches('/').to_string(),
            api_key: key.map(|k| k.to_string()),
            enabled: peer.enabled,
        });
    }

    let defaults = crate::federation::FederationConfig::default();

    // An unstable node_id breaks convergence: vector clocks are keyed by it, so
    // a fresh id each restart makes this node look like a brand-new peer and the
    // clock comparison stops meaning anything. Warn loudly rather than
    // pretending a random id is equivalent.
    let node_id = if settings.node_id.trim().is_empty() {
        eprintln!(
            "warning: federation.node_id is unset — using a random id for this process. \
             Vector clocks are keyed by node id, so sync history will not carry across \
             restarts. Set federation.node_id to a stable value."
        );
        defaults.node_id
    } else {
        settings.node_id.clone()
    };

    let node_name = if settings.node_name.trim().is_empty() {
        defaults.node_name
    } else {
        settings.node_name.clone()
    };

    Ok(crate::federation::FederationConfig {
        node_id,
        node_name,
        peers,
        ..crate::federation::FederationConfig::default()
    })
}

/// Constant-time comparison of two secrets.
///
/// A byte-by-byte `==` returns as soon as it finds a difference, and that
/// timing difference is measurable across a network given enough samples —
/// enough to recover a key one byte at a time. Compares length first (that
/// much is not secret) then every byte regardless of mismatches.
#[must_use]
pub fn secret_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Whether `presented` matches any accepted key.
#[must_use]
pub fn key_is_accepted(presented: &str, accepted: &[Zeroizing<String>]) -> bool {
    // Deliberately does not short-circuit on the first match: `any` would
    // return early and leak, through timing, roughly which peer matched.
    let mut matched = false;
    for key in accepted {
        matched |= secret_eq(presented.as_bytes(), key.as_bytes());
    }
    matched
}

/// The passphrase used to seal transfers, from the environment.
///
/// Never read from the config file. It is the one secret both nodes must share
/// to exchange models, and a config file is the wrong place for it.
pub fn seal_passphrase() -> Result<Zeroizing<String>> {
    // Set-but-empty and unset are the same failure to an operator, and the
    // remedy is identical, so they share one message.
    let value = crate::env::var_secret(SEAL_PASSPHRASE_ENV).ok_or_else(|| {
        VaultError::ConfigError(format!(
            "federation.seal_transfers is on but ${SEAL_PASSPHRASE_ENV} is not set \
             (or is empty). Both peers must share the same value. Set it, or set \
             federation.seal_transfers = false to send models unencrypted \
             (only defensible on a network you fully control)."
        ))
    })?;

    Ok(value)
}

/// Seal model bytes for transit when configured to.
pub fn seal_for_transit(settings: &FederationSettings, plaintext: &[u8]) -> Result<Vec<u8>> {
    if !settings.seal_transfers {
        return Ok(plaintext.to_vec());
    }
    let passphrase = seal_passphrase()?;
    crate::cloud_envelope::seal(plaintext, passphrase.as_bytes().to_vec())
}

/// Reverse [`seal_for_transit`].
///
/// Keys off the envelope magic rather than the local `seal_transfers` setting:
/// the sender decides whether an object is sealed, and the two nodes can
/// disagree. An unsealed arrival while sealing is on is reported rather than
/// stored — silently accepting it would let a peer downgrade the transfer.
pub fn open_from_transit(settings: &FederationSettings, received: Vec<u8>) -> Result<Vec<u8>> {
    let sealed = crate::cloud_envelope::is_sealed(&received);

    if sealed {
        let passphrase = seal_passphrase()?;
        return crate::cloud_envelope::open(&received, passphrase.as_bytes().to_vec());
    }

    if settings.seal_transfers {
        return Err(VaultError::ConfigError(
            "peer sent an unsealed model while federation.seal_transfers is on. \
             Either the peer has sealing disabled or the transfer was downgraded; \
             refusing it rather than storing an unverified plaintext model."
                .to_string(),
        ));
    }

    Ok(received)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(key: Option<&str>, enabled: bool) -> FederationPeerSettings {
        FederationPeerSettings {
            node_id: "peer-1".into(),
            name: "peer".into(),
            endpoint: "https://peer.example.com".into(),
            api_key: key.map(str::to_string),
            enabled,
        }
    }

    fn version_with(checkpoint_id: &str, origin: Option<&str>) -> crate::version::ModelVersion {
        let mut metadata = std::collections::HashMap::new();
        if let Some(origin) = origin {
            metadata.insert(ORIGIN_CHECKPOINT_KEY.to_string(), origin.to_string());
        }
        crate::version::ModelVersion {
            version: 1,
            checkpoint_id: checkpoint_id.to_string(),
            timestamp: chrono::Utc::now(),
            parent_version: None,
            format: "safetensors".into(),
            size_bytes: 1,
            compressed_size_bytes: 1,
            checksum_sha256: "abc".into(),
            metadata,
            file_path: "f".into(),
        }
    }

    /// A synced copy must keep the identity it arrived with.
    ///
    /// Regression: `add_version` mints a fresh checkpoint id, so a received
    /// model advertised an id its sender had never seen. Each sync then found
    /// the sender's version "missing" and transferred it again — the vaults
    /// duplicated the model on every run and never converged.
    #[test]
    fn a_received_version_advertises_its_origin_id() {
        let local_only = version_with("local-abc", None);
        assert_eq!(federation_checkpoint_id(&local_only), "local-abc");

        // Stored locally as "local-xyz", but it came from a peer as "origin-1".
        let received = version_with("local-xyz", Some("origin-1"));
        assert_eq!(
            federation_checkpoint_id(&received),
            "origin-1",
            "a received version must advertise the id it arrived with, not the \
             one this vault minted for it"
        );

        // The round trip the bug broke: peer sends origin-1, we store it, and
        // the peer must then recognise it as already present.
        let fields = origin_metadata("origin-1");
        assert_eq!(fields.get(ORIGIN_CHECKPOINT_KEY).unwrap(), "origin-1");
    }

    #[test]
    fn secret_eq_matches_only_identical_values() {
        assert!(secret_eq(b"abc", b"abc"));
        assert!(!secret_eq(b"abc", b"abd"));
        assert!(!secret_eq(b"abc", b"ab"));
        assert!(secret_eq(b"", b""));
    }

    #[test]
    fn disabled_peers_do_not_grant_access() {
        // A disabled peer must not keep a working inbound key: turning a peer
        // off is expected to close the connection in both directions.
        let settings = FederationSettings {
            peers: vec![peer(Some("literal-key"), false)],
            ..Default::default()
        };
        let keys = accepted_keys(&settings).unwrap();
        assert!(keys.is_empty());
        assert!(!key_is_accepted("literal-key", &keys));
    }

    #[test]
    fn enabled_peer_key_is_accepted() {
        let settings = FederationSettings {
            peers: vec![peer(Some("literal-key"), true)],
            ..Default::default()
        };
        let keys = accepted_keys(&settings).unwrap();
        assert!(key_is_accepted("literal-key", &keys));
        assert!(!key_is_accepted("wrong-key", &keys));
        // An empty presented key must never pass, even against a peer with no
        // key configured elsewhere in the list.
        assert!(!key_is_accepted("", &keys));
    }

    #[test]
    fn peer_without_a_key_contributes_nothing() {
        let settings = FederationSettings {
            peers: vec![peer(None, true)],
            ..Default::default()
        };
        assert!(accepted_keys(&settings).unwrap().is_empty());
    }

    #[test]
    fn sealing_defaults_to_on() {
        // Guards the hand-written Default: a derived one would leave this
        // false and ship models in the clear.
        assert!(FederationSettings::default().seal_transfers);
    }

    #[test]
    fn unsealed_arrival_is_refused_when_sealing_is_on() {
        let settings = FederationSettings::default();
        assert!(settings.seal_transfers);

        let err = open_from_transit(&settings, b"raw model bytes".to_vec()).unwrap_err();
        assert!(
            err.to_string().contains("unsealed"),
            "expected a downgrade complaint, got: {err}"
        );
    }

    #[test]
    fn unsealed_arrival_passes_through_when_sealing_is_off() {
        let settings = FederationSettings {
            seal_transfers: false,
            ..Default::default()
        };
        let data = open_from_transit(&settings, b"raw model bytes".to_vec()).unwrap();
        assert_eq!(data, b"raw model bytes");
    }

    #[test]
    fn seal_roundtrip_hides_the_plaintext() {
        std::env::set_var(SEAL_PASSPHRASE_ENV, "shared-federation-passphrase");
        let settings = FederationSettings::default();

        let plaintext = b"weights-that-should-not-appear-on-the-wire";
        let sealed = seal_for_transit(&settings, plaintext).unwrap();

        assert!(crate::cloud_envelope::is_sealed(&sealed));
        assert!(
            !sealed
                .windows(plaintext.len())
                .any(|w| w == plaintext.as_slice()),
            "plaintext must not survive in the sealed bytes"
        );

        let opened = open_from_transit(&settings, sealed).unwrap();
        assert_eq!(opened, plaintext);

        std::env::remove_var(SEAL_PASSPHRASE_ENV);
    }
}
