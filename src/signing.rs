//! Ed25519 model signing and provenance verification.
//!
//! Provides cryptographic signatures on model files for supply-chain
//! security.  Keys are generated locally and stored in the vault's config
//! directory.  Signatures are detached `.sig` JSON files containing the
//! signature, public key, timestamp, and optional metadata.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Result, VaultError};

// ── Signature envelope ───────────────────────────────────────────────────────

/// Detached signature for a model file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSignature {
    /// Ed25519 signature (hex-encoded)
    pub signature: String,
    /// Public key of the signer (hex-encoded)
    pub public_key: String,
    /// SHA-256 of the signed file (hex-encoded)
    pub file_sha256: String,
    /// Signer identity (optional display name / email)
    pub signer: Option<String>,
    /// ISO-8601 timestamp of signing
    pub signed_at: String,
    /// Signature format version
    pub version: u32,
    /// Additional metadata (model name, version, etc.)
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

// ── Key pair ─────────────────────────────────────────────────────────────────

/// An Ed25519 signing key pair.
///
/// Keys are stored as hex-encoded strings.  The secret key is 64 bytes
/// (seed + public key) as returned by the Ed25519 expand step.
#[derive(Clone, Serialize, Deserialize)]
pub struct SigningKeyPair {
    /// Hex-encoded 32-byte secret seed
    pub secret_seed: String,
    /// Hex-encoded 32-byte public key
    pub public_key: String,
    /// Human-readable identity
    pub identity: Option<String>,
    /// When the key was created
    pub created_at: String,
}

/// Redacts the secret seed.
///
/// The derived `Debug` printed the seed verbatim, so any `{:?}` of a keypair —
/// a `tracing` call, an `unwrap` panic, an error report — would have put
/// signing key material into a log. `Serialize` is deliberately left intact:
/// writing the seed is the whole point of `save_keypair`.
impl std::fmt::Debug for SigningKeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SigningKeyPair")
            .field("secret_seed", &"<redacted>")
            .field("public_key", &self.public_key)
            .field("identity", &self.identity)
            .field("created_at", &self.created_at)
            .finish()
    }
}

// ── Signer ───────────────────────────────────────────────────────────────────

/// Signs model files using Ed25519 (via the `ed25519-dalek` crate when
/// available, or a HKDF-based HMAC fallback using SHA-256).
///
/// Since this crate forbids `unsafe_code` and we want zero additional
/// heavyweight dependencies, we implement signing as HMAC-SHA256 over
/// the file hash keyed by a 32-byte secret.  This provides:
///   - Tamper detection (integrity)
///   - Signer authentication (only secret-holder can produce the tag)
///
/// For full non-repudiation upgrade to Ed25519 by enabling a future
/// `ed25519` feature flag.
pub struct ModelSigner;

/// Signature format written by [`ModelSigner::sign`].
///
/// Version 1 computed `SHA-256(seed || file_hash)` while calling itself
/// HMAC-SHA256. That is not HMAC: it is a bare hash of a concatenation, the
/// construction HMAC exists to replace. Version 2 is RFC 2104 HMAC-SHA256.
/// Version 1 signatures still verify, against the version-1 construction, so
/// existing `.sig` files keep working.
const SIGNATURE_VERSION: u32 = 2;

/// RFC 2104 HMAC-SHA256.
fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;

    // Keys longer than the block size are hashed down first.
    let mut padded = [0u8; BLOCK];
    if key.len() > BLOCK {
        padded[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        padded[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= padded[i];
        opad[i] ^= padded[i];
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);

    padded.fill(0);
    ipad.fill(0);
    opad.fill(0);

    outer.finalize().into()
}

/// The pre-version-2 tag: `SHA-256(seed || file_hash)`. Kept only so old
/// signatures remain verifiable.
fn legacy_tag(seed: &[u8], file_hash: &[u8]) -> [u8; 32] {
    let mut input = Vec::with_capacity(seed.len() + file_hash.len());
    input.extend_from_slice(seed);
    input.extend_from_slice(file_hash);
    Sha256::digest(&input).into()
}

/// Compare two tags without leaking where they first differ.
///
/// A byte-wise `==` on the hex strings returns as soon as it finds a mismatch,
/// which lets an attacker who can time repeated verifications recover a valid
/// tag one byte at a time.
fn tags_equal(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

impl ModelSigner {
    /// Generate a new signing key pair.
    ///
    /// Uses the OS CSPRNG to create a random 32-byte seed, then derives
    /// the "public key" as SHA-256(seed) for identification.
    pub fn generate_keypair(identity: Option<&str>) -> Result<SigningKeyPair> {
        use rand_core::{OsRng, RngCore};

        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);

        let public = Sha256::digest(seed);

        let kp = SigningKeyPair {
            secret_seed: hex::encode(seed),
            public_key: hex::encode(public),
            identity: identity.map(String::from),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        // Zeroize seed from stack
        seed.fill(0);

        Ok(kp)
    }

    /// Save a key pair to a JSON file with restrictive permissions.
    ///
    /// This file holds the secret seed — the only thing standing between an
    /// attacker and the ability to forge signatures for this identity. It is
    /// created with restrictive permissions *before* the seed is written, and
    /// tightened again afterwards for Windows, where ACLs can only be set on
    /// an existing file.
    pub fn save_keypair(keypair: &SigningKeyPair, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(keypair)?;

        // Previously `fs::write` + a `#[cfg(unix)]` chmod. That left the seed
        // world-readable for the window between creation and the chmod on
        // Unix, and on Windows left it with fully inherited ACLs — granting
        // BUILTIN\Administrators access the vault's own files do not.
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        crate::permissions::set_create_mode(&mut options);

        let mut file = options.open(path)?;
        crate::permissions::restrict_file(path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;

        Ok(())
    }

    /// Rebuild a key pair from a hex-encoded 32-byte secret seed.
    ///
    /// Lets a signing key live in a secret manager as bare seed material rather
    /// than as a full keypair document — the public key is re-derived, so the
    /// result is identical to the keypair the seed was generated with.
    pub fn keypair_from_seed(seed_hex: &str, identity: Option<&str>) -> Result<SigningKeyPair> {
        let seed_hex = seed_hex.trim();
        let seed = hex::decode(seed_hex)
            .map_err(|e| VaultError::InvalidInput(format!("Signing seed is not valid hex: {e}")))?;
        if seed.len() != 32 {
            return Err(VaultError::InvalidInput(format!(
                "Signing seed must be 32 bytes ({} hex chars), got {}",
                64,
                seed.len()
            )));
        }

        Ok(SigningKeyPair {
            secret_seed: hex::encode(&seed),
            public_key: hex::encode(Sha256::digest(&seed)),
            identity: identity.map(String::from),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Parse a key pair from either a JSON keypair document or a bare
    /// hex-encoded seed — the two shapes a secret manager might hold.
    pub fn parse_keypair(data: &str, identity: Option<&str>) -> Result<SigningKeyPair> {
        let trimmed = data.trim();
        if trimmed.starts_with('{') {
            return serde_json::from_str(trimmed).map_err(Into::into);
        }
        Self::keypair_from_seed(trimmed, identity)
    }

    /// Load a key pair from a JSON file.
    pub fn load_keypair(path: &Path) -> Result<SigningKeyPair> {
        let data = fs::read_to_string(path)?;
        let kp: SigningKeyPair = serde_json::from_str(&data)?;
        Ok(kp)
    }

    /// Sign a model file, producing a detached [`ModelSignature`].
    pub fn sign(
        keypair: &SigningKeyPair,
        file_path: &Path,
        metadata: HashMap<String, String>,
    ) -> Result<ModelSignature> {
        // Read file and compute SHA-256
        let data = fs::read(file_path)?;
        let file_hash = Sha256::digest(&data);
        let file_sha256 = hex::encode(file_hash);

        // Decode secret seed
        let seed_bytes = hex::decode(&keypair.secret_seed)
            .map_err(|e| VaultError::CryptoError(format!("Invalid secret seed: {}", e)))?;

        let signature_bytes = hmac_sha256(&seed_bytes, &file_hash);

        Ok(ModelSignature {
            signature: hex::encode(signature_bytes),
            public_key: keypair.public_key.clone(),
            file_sha256,
            signer: keypair.identity.clone(),
            signed_at: chrono::Utc::now().to_rfc3339(),
            version: SIGNATURE_VERSION,
            metadata,
        })
    }

    /// Verify a detached signature against a model file.
    ///
    /// A verification only passes when the keyed tag was actually checked.
    /// Without a secret seed this reports `valid: false` and
    /// `signature_checked: false`, because the only thing comparable without
    /// the key is the file hash — and that hash is read out of the signature
    /// file, which is exactly what an attacker substituting a tampered model
    /// would rewrite. Reporting such a pair as valid is worse than reporting
    /// nothing.
    pub fn verify(
        signature: &ModelSignature,
        file_path: &Path,
        secret_seed: Option<&str>,
    ) -> Result<SignatureVerification> {
        let data = fs::read(file_path)?;
        let file_hash = Sha256::digest(&data);
        let file_sha256 = hex::encode(file_hash);

        // Check file hash matches what was signed
        if file_sha256 != signature.file_sha256 {
            return Ok(SignatureVerification {
                valid: false,
                file_hash_match: false,
                signature_match: false,
                signature_checked: secret_seed.is_some(),
                signer: signature.signer.clone(),
                signed_at: signature.signed_at.clone(),
                reason: Some("File SHA-256 does not match signed hash".to_string()),
            });
        }

        let Some(seed_hex) = secret_seed else {
            return Ok(SignatureVerification {
                valid: false,
                file_hash_match: true,
                signature_match: false,
                signature_checked: false,
                signer: signature.signer.clone(),
                signed_at: signature.signed_at.clone(),
                reason: Some(
                    "No verification key was supplied, so authenticity was not checked. \
                     The file hash matched, but that hash is stored in the signature file \
                     itself and proves nothing about who produced it. Pass --key with the \
                     signing key (a file path or a KMS URI) to verify."
                        .to_string(),
                ),
            });
        };

        let seed_bytes = hex::decode(seed_hex)
            .map_err(|e| VaultError::CryptoError(format!("Invalid seed: {}", e)))?;

        let expected = match signature.version {
            0 | 1 => legacy_tag(&seed_bytes, &file_hash),
            _ => hmac_sha256(&seed_bytes, &file_hash),
        };
        let signature_match = tags_equal(&hex::encode(expected), &signature.signature);

        Ok(SignatureVerification {
            valid: signature_match,
            file_hash_match: true,
            signature_match,
            signature_checked: true,
            signer: signature.signer.clone(),
            signed_at: signature.signed_at.clone(),
            reason: if signature_match {
                None
            } else {
                Some("HMAC signature does not match".to_string())
            },
        })
    }

    /// Save a signature to a `.sig` JSON file alongside the model.
    pub fn save_signature(signature: &ModelSignature, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(signature)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load a signature from a `.sig` JSON file.
    pub fn load_signature(path: &Path) -> Result<ModelSignature> {
        let data = fs::read_to_string(path)?;
        let sig: ModelSignature = serde_json::from_str(&data)?;
        Ok(sig)
    }
}

/// Result of a signature verification.
///
/// The four flags are an independent report of what was checked and what
/// passed, not a state machine — a caller may care that the file hash matched
/// even when the tag was never checked. Collapsing them into an enum would
/// discard exactly the distinction the unkeyed-verify fix exists to preserve.
#[derive(Debug, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct SignatureVerification {
    /// Overall validity
    pub valid: bool,
    /// Whether the file hash matches
    pub file_hash_match: bool,
    /// Whether the cryptographic signature matches
    pub signature_match: bool,
    /// Whether the keyed tag was checked at all. False when no key was
    /// supplied — in which case `valid` is false regardless of the file hash.
    pub signature_checked: bool,
    /// Signer identity
    pub signer: Option<String>,
    /// When the file was signed
    pub signed_at: String,
    /// Reason for failure (if any)
    pub reason: Option<String>,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_generate_keypair() {
        let kp = ModelSigner::generate_keypair(Some("test-user")).unwrap();
        assert_eq!(kp.secret_seed.len(), 64); // 32 bytes hex
        assert_eq!(kp.public_key.len(), 64);
        assert_eq!(kp.identity.as_deref(), Some("test-user"));
    }

    #[test]
    fn test_sign_and_verify() {
        let kp = ModelSigner::generate_keypair(Some("tester")).unwrap();

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"fake model data for testing").unwrap();

        let sig = ModelSigner::sign(&kp, file.path(), HashMap::new()).unwrap();
        assert!(!sig.signature.is_empty());
        assert_eq!(sig.public_key, kp.public_key);

        // Verify with secret seed
        let result = ModelSigner::verify(&sig, file.path(), Some(&kp.secret_seed)).unwrap();
        assert!(result.valid);
        assert!(result.file_hash_match);
        assert!(result.signature_match);

        // Without the secret, nothing about authenticity can be established.
        let result2 = ModelSigner::verify(&sig, file.path(), None).unwrap();
        assert!(!result2.valid, "no key means no verification");
        assert!(!result2.signature_checked);
        assert!(result2.file_hash_match);
    }

    /// The signature file carries both the tag *and* the file hash it was made
    /// over. Verifying without a key compared the file against that stored
    /// hash and reported `valid: true` — so anyone could ship a tampered model
    /// with a regenerated `.sig` and have `iv verify` call it good. The whole
    /// point of the command is to detect exactly this.
    #[test]
    fn test_forged_signature_without_key_is_not_reported_valid() {
        let kp = ModelSigner::generate_keypair(Some("honest-signer")).unwrap();

        let mut genuine = NamedTempFile::new().unwrap();
        genuine.write_all(b"the real model").unwrap();
        let real_sig = ModelSigner::sign(&kp, genuine.path(), HashMap::new()).unwrap();

        // Attacker swaps the payload and rewrites the hash in the .sig, keeping
        // the original signer name and tag.
        let mut tampered = NamedTempFile::new().unwrap();
        tampered.write_all(b"malicious payload").unwrap();
        let tampered_hash = hex::encode(Sha256::digest(b"malicious payload"));

        let forged = ModelSignature {
            file_sha256: tampered_hash,
            ..real_sig.clone()
        };

        // No key: must refuse to bless it.
        let unkeyed = ModelSigner::verify(&forged, tampered.path(), None).unwrap();
        assert!(
            !unkeyed.valid,
            "a forged signature must never verify, least of all with no key"
        );
        assert!(
            unkeyed.file_hash_match,
            "the forged hash does match the file"
        );
        assert!(!unkeyed.signature_checked);

        // With the key: the tag is over the original hash, so it fails.
        let keyed = ModelSigner::verify(&forged, tampered.path(), Some(&kp.secret_seed)).unwrap();
        assert!(!keyed.valid);
        assert!(!keyed.signature_match);
    }

    #[test]
    fn test_wrong_key_does_not_verify() {
        let signer = ModelSigner::generate_keypair(None).unwrap();
        let attacker = ModelSigner::generate_keypair(None).unwrap();

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"model bytes").unwrap();
        let sig = ModelSigner::sign(&signer, file.path(), HashMap::new()).unwrap();

        let result = ModelSigner::verify(&sig, file.path(), Some(&attacker.secret_seed)).unwrap();
        assert!(!result.valid);
        assert!(result.signature_checked);
        assert!(!result.signature_match);
    }

    /// RFC 2104 test vector: key = 20 × 0x0b, data = "Hi There".
    #[test]
    fn test_hmac_sha256_matches_rfc4231_vector() {
        let key = [0x0bu8; 20];
        let tag = hmac_sha256(&key, b"Hi There");
        assert_eq!(
            hex::encode(tag),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    /// A key longer than the 64-byte block must be hashed down, per RFC 2104.
    /// RFC 4231 case 4: 131 × 0xaa, data "Test Using Larger Than Block-Size Key -
    /// Hash Key First".
    #[test]
    fn test_hmac_sha256_handles_oversized_keys() {
        let key = [0xaau8; 131];
        let tag = hmac_sha256(
            &key,
            b"Test Using Larger Than Block-Size Key - Hash Key First",
        );
        assert_eq!(
            hex::encode(tag),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    /// The v1 construction was `SHA-256(seed || hash)`, not HMAC. Signatures
    /// written before the fix must still verify, or upgrading silently
    /// invalidates every existing `.sig`.
    #[test]
    fn test_version_1_signatures_still_verify() {
        let kp = ModelSigner::generate_keypair(None).unwrap();

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"legacy model").unwrap();
        let file_hash = Sha256::digest(b"legacy model");

        let seed = hex::decode(&kp.secret_seed).unwrap();
        let legacy = ModelSignature {
            signature: hex::encode(legacy_tag(&seed, &file_hash)),
            public_key: kp.public_key.clone(),
            file_sha256: hex::encode(file_hash),
            signer: None,
            signed_at: "2026-01-01T00:00:00Z".to_string(),
            version: 1,
            metadata: HashMap::new(),
        };

        let result = ModelSigner::verify(&legacy, file.path(), Some(&kp.secret_seed)).unwrap();
        assert!(result.valid, "existing v1 signatures must keep verifying");

        // And new signatures are written as v2.
        let fresh = ModelSigner::sign(&kp, file.path(), HashMap::new()).unwrap();
        assert_eq!(fresh.version, 2);
        assert_ne!(
            fresh.signature, legacy.signature,
            "v2 must not reproduce the v1 tag"
        );
    }

    #[test]
    fn test_tag_comparison_rejects_length_and_content_mismatch() {
        assert!(tags_equal("abcd", "abcd"));
        assert!(!tags_equal("abcd", "abce"));
        assert!(!tags_equal("abcd", "abc"));
        assert!(!tags_equal("", "a"));
        assert!(tags_equal("", ""));
    }

    #[test]
    fn test_verify_tampered_file() {
        let kp = ModelSigner::generate_keypair(None).unwrap();

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"original data").unwrap();

        let sig = ModelSigner::sign(&kp, file.path(), HashMap::new()).unwrap();

        // Tamper with the file
        file.as_file().set_len(0).unwrap();
        file.write_all(b"tampered data").unwrap();

        let result = ModelSigner::verify(&sig, file.path(), Some(&kp.secret_seed)).unwrap();
        assert!(!result.valid);
        assert!(!result.file_hash_match);
    }

    #[test]
    fn test_save_load_keypair() {
        let kp = ModelSigner::generate_keypair(Some("test")).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.key");

        ModelSigner::save_keypair(&kp, &path).unwrap();
        let loaded = ModelSigner::load_keypair(&path).unwrap();
        assert_eq!(loaded.secret_seed, kp.secret_seed);
        assert_eq!(loaded.public_key, kp.public_key);
    }

    #[test]
    fn test_save_load_signature() {
        let kp = ModelSigner::generate_keypair(None).unwrap();
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"model bytes").unwrap();

        let sig = ModelSigner::sign(&kp, file.path(), HashMap::new()).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let sig_path = dir.path().join("model.sig");
        ModelSigner::save_signature(&sig, &sig_path).unwrap();

        let loaded = ModelSigner::load_signature(&sig_path).unwrap();
        assert_eq!(loaded.signature, sig.signature);
        assert_eq!(loaded.file_sha256, sig.file_sha256);
    }

    #[test]
    fn test_keypair_from_seed_roundtrip() {
        let original = ModelSigner::generate_keypair(Some("alice")).unwrap();

        // Rebuilding from the seed alone must re-derive the same public key,
        // so signatures made either way verify against each other.
        let rebuilt = ModelSigner::keypair_from_seed(&original.secret_seed, Some("alice")).unwrap();
        assert_eq!(rebuilt.secret_seed, original.secret_seed);
        assert_eq!(rebuilt.public_key, original.public_key);
    }

    #[test]
    fn test_keypair_from_seed_rejects_bad_input() {
        assert!(ModelSigner::keypair_from_seed("not-hex", None).is_err());
        // 16 bytes instead of 32
        assert!(ModelSigner::keypair_from_seed(&"ab".repeat(16), None).is_err());
    }

    #[test]
    fn test_parse_keypair_accepts_both_shapes() {
        let kp = ModelSigner::generate_keypair(Some("bob")).unwrap();

        let as_json = serde_json::to_string(&kp).unwrap();
        let from_json = ModelSigner::parse_keypair(&as_json, None).unwrap();
        assert_eq!(from_json.public_key, kp.public_key);
        assert_eq!(from_json.identity.as_deref(), Some("bob"));

        // A secret manager may hold just the seed; identity then comes from the caller.
        let from_seed = ModelSigner::parse_keypair(&kp.secret_seed, Some("bob")).unwrap();
        assert_eq!(from_seed.public_key, kp.public_key);
        assert_eq!(from_seed.identity.as_deref(), Some("bob"));
    }

    #[test]
    fn test_seed_sourced_key_produces_verifiable_signature() {
        let kp = ModelSigner::generate_keypair(None).unwrap();
        let rebuilt = ModelSigner::keypair_from_seed(&kp.secret_seed, None).unwrap();

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"payload signed with a KMS-sourced key")
            .unwrap();

        let sig = ModelSigner::sign(&rebuilt, file.path(), HashMap::new()).unwrap();
        let result = ModelSigner::verify(&sig, file.path(), Some(&kp.secret_seed)).unwrap();
        assert!(
            result.valid,
            "seed-sourced key must verify against the original"
        );
    }
    /// The secret seed must never reach a log through `{:?}`.
    #[test]
    fn test_keypair_debug_redacts_the_secret_seed() {
        let kp = ModelSigner::generate_keypair(Some("ML Team")).unwrap();
        let rendered = format!("{kp:?}");

        assert!(
            !rendered.contains(&kp.secret_seed),
            "Debug output leaked the secret seed: {rendered}"
        );
        assert!(rendered.contains("<redacted>"), "got: {rendered}");
        // The non-secret fields stay useful for diagnostics.
        assert!(rendered.contains(&kp.public_key), "got: {rendered}");
        assert!(rendered.contains("ML Team"), "got: {rendered}");
    }

    /// The key file holds the only thing that can forge signatures for this
    /// identity, so it must not be readable by other accounts.
    #[test]
    fn test_saved_keypair_is_not_world_readable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("signing_key.json");
        let kp = ModelSigner::generate_keypair(None).unwrap();
        ModelSigner::save_keypair(&kp, &path).unwrap();

        // The seed really is in there — otherwise this test proves nothing.
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains(&kp.secret_seed));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(
                mode & 0o077,
                0,
                "group/other bits set on the signing key: {:o}",
                mode
            );
        }

        #[cfg(windows)]
        {
            // `icacls` reports inherited ACEs with "(I)". The key must not
            // simply inherit whatever the parent directory grants — that is
            // how BUILTIN\Administrators ended up with FullControl.
            let out = std::process::Command::new("icacls")
                .arg(&path)
                .output()
                .expect("icacls should be available on Windows");
            let text = String::from_utf8_lossy(&out.stdout);
            assert!(
                !text.contains("(I)"),
                "signing key still has inherited ACLs: {text}"
            );
        }
    }
}
