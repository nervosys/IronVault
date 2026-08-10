//! Client-side encryption for objects leaving the machine.
//!
//! `iv cloud push` used to upload what [`crate::Vault::get_model`] returns,
//! which is plaintext — the vault decrypts on read. The object in the bucket
//! was therefore the bare model, and confidentiality depended entirely on the
//! bucket's own server-side encryption and access policy.
//!
//! This module seals the payload before it leaves the process, so a bucket can
//! be treated as untrusted storage.
//!
//! # Why not just upload the vault's on-disk file
//!
//! The vault's stored blob is encrypted with a key derived from the vault's
//! own salt, which lives in the vault. Uploading it verbatim would produce an
//! object that only *that* vault directory can open — useless for the case
//! cloud storage exists to serve, where a colleague or a CI runner pulls into
//! a different vault. The envelope below carries its own salt, so anyone with
//! the passphrase can open it.
//!
//! # Format
//!
//! ```text
//! offset  size  field
//! 0       8     magic "IRONSEAL" (4.x wrote "AIMVSEAL"; both are read)
//! 8       1     format version (currently 1)
//! 9       1     KDF id (1 = Argon2id, the vault's parameters)
//! 10      2     salt length, big-endian u16
//! 12      N     salt
//! 12+N    ..    nonce || ciphertext || GCM tag
//! ```
//!
//! The header is not itself authenticated. It does not need to be: every field
//! in it feeds key derivation, so altering any of them yields a different key
//! and the GCM tag check fails. Tampering produces an error, never wrong
//! plaintext.

use crate::crypto::{FipsCrypto, SALT_SIZE};
use crate::error::{Result, VaultError};

/// Identifies a sealed object. Chosen to be unambiguous in the first bytes of
/// a file so [`is_sealed`] can distinguish a sealed upload from one made by an
/// older version, which wrote plaintext.
///
/// Written by [`seal`] from 5.0 on.
pub const MAGIC: &[u8; 8] = b"IRONSEAL";

/// The 4.x magic, still accepted on read.
///
/// Renaming the constant does not rename the bytes already written: every
/// object sealed before 5.0 — in an S3 bucket, in an Azure container, in a
/// federation transfer — begins with these eight. [`is_sealed`] and [`open`]
/// accept both spellings so those objects keep opening; only [`seal`] is
/// one-way. Nothing re-writes an existing object, so a bucket may hold a mix
/// indefinitely and both are readable.
pub const LEGACY_MAGIC: &[u8; 8] = b"AIMVSEAL";

/// Envelope format version.
pub const FORMAT_VERSION: u8 = 1;

/// Argon2id with the vault's parameters.
const KDF_ARGON2ID: u8 = 1;

/// Bytes before the salt: magic, version, KDF id, salt length.
const HEADER_PREFIX_LEN: usize = 8 + 1 + 1 + 2;

/// True if `data` starts with either envelope magic.
///
/// Used on the download path so objects pushed by a version that uploaded
/// plaintext still open, rather than failing with a confusing crypto error.
/// Accepts [`LEGACY_MAGIC`] as well as [`MAGIC`]: an object sealed before 5.0
/// is still a sealed object, and reporting it as plaintext would hand the
/// caller ciphertext and call it a model.
#[must_use]
pub fn is_sealed(data: &[u8]) -> bool {
    data.len() >= MAGIC.len()
        && (&data[..MAGIC.len()] == MAGIC || &data[..LEGACY_MAGIC.len()] == LEGACY_MAGIC)
}

/// Encrypt `plaintext` under `passphrase`, returning a self-contained object.
///
/// A fresh salt is drawn per call, so pushing the same model twice produces
/// two different ciphertexts and reveals nothing by comparison.
pub fn seal(plaintext: &[u8], passphrase: Vec<u8>) -> Result<Vec<u8>> {
    let crypto = FipsCrypto::new()?;
    let (key, salt) = crypto.derive_key(passphrase, None)?;
    let body = crypto.encrypt(plaintext, &key)?;

    let salt_len = u16::try_from(salt.len())
        .map_err(|_| VaultError::CryptoError("Salt too long to encode".to_string()))?;

    let mut out = Vec::with_capacity(HEADER_PREFIX_LEN + salt.len() + body.len());
    out.extend_from_slice(MAGIC);
    out.push(FORMAT_VERSION);
    out.push(KDF_ARGON2ID);
    out.extend_from_slice(&salt_len.to_be_bytes());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&body);

    Ok(out)
}

/// Decrypt an object produced by [`seal`].
///
/// # Errors
///
/// [`VaultError::AuthenticationFailed`] if the passphrase is wrong or the
/// object was altered in transit or at rest — GCM does not distinguish the
/// two, and neither should the message.
pub fn open(sealed: &[u8], passphrase: Vec<u8>) -> Result<Vec<u8>> {
    if !is_sealed(sealed) {
        return Err(VaultError::CryptoError(
            "Not an IRONSEAL object: missing magic. An object uploaded by \
             a version before 4.3.0 is stored as plaintext and needs no key."
                .to_string(),
        ));
    }
    if sealed.len() < HEADER_PREFIX_LEN {
        return Err(VaultError::CryptoError(
            "Sealed object truncated: shorter than its header".to_string(),
        ));
    }

    let version = sealed[8];
    if version != FORMAT_VERSION {
        return Err(VaultError::CryptoError(format!(
            "Unsupported envelope version {version}: this build understands \
             version {FORMAT_VERSION}. Upgrade `iv` to read it."
        )));
    }

    let kdf = sealed[9];
    if kdf != KDF_ARGON2ID {
        return Err(VaultError::CryptoError(format!(
            "Unsupported KDF id {kdf} in sealed object"
        )));
    }

    let salt_len = usize::from(u16::from_be_bytes([sealed[10], sealed[11]]));
    let body_start = HEADER_PREFIX_LEN
        .checked_add(salt_len)
        .ok_or_else(|| VaultError::CryptoError("Salt length overflows".to_string()))?;
    if sealed.len() <= body_start {
        return Err(VaultError::CryptoError(
            "Sealed object truncated: no ciphertext after the salt".to_string(),
        ));
    }

    let salt = sealed[HEADER_PREFIX_LEN..body_start].to_vec();
    let crypto = FipsCrypto::new()?;
    let (key, _) = crypto.derive_key(passphrase, Some(salt))?;
    crypto.decrypt(&sealed[body_start..], &key)
}

/// Salt length written by [`seal`], exposed for tests and diagnostics.
#[must_use]
pub const fn salt_len() -> usize {
    SALT_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pass() -> Vec<u8> {
        b"correct horse battery staple".to_vec()
    }

    #[test]
    fn test_roundtrip() {
        let plaintext = b"model weights, or near enough".to_vec();
        let sealed = seal(&plaintext, pass()).unwrap();
        assert_eq!(open(&sealed, pass()).unwrap(), plaintext);
    }

    /// The point of the whole module: the payload must not be recoverable by
    /// reading the object.
    #[test]
    fn test_plaintext_does_not_appear_in_the_sealed_object() {
        let needle = b"SECRET-TENSOR-DATA-DO-NOT-LEAK";
        let sealed = seal(needle, pass()).unwrap();
        assert!(
            !sealed.windows(needle.len()).any(|w| w == needle),
            "plaintext survived into the sealed object"
        );
    }

    #[test]
    fn test_wrong_passphrase_is_rejected() {
        let sealed = seal(b"payload", pass()).unwrap();
        let err = open(&sealed, b"wrong passphrase".to_vec()).unwrap_err();
        assert!(
            matches!(err, VaultError::AuthenticationFailed),
            "expected AuthenticationFailed, got {err:?}"
        );
    }

    /// A flipped bit anywhere in the ciphertext must fail the GCM tag rather
    /// than yield corrupted plaintext.
    #[test]
    fn test_tampered_ciphertext_is_rejected() {
        let mut sealed = seal(b"payload that is long enough to poke at", pass()).unwrap();
        let last = sealed.len() - 1;
        sealed[last] ^= 0x01;
        assert!(matches!(
            open(&sealed, pass()).unwrap_err(),
            VaultError::AuthenticationFailed
        ));
    }

    /// The salt is unauthenticated by design; altering it must still fail,
    /// because it changes the derived key.
    #[test]
    fn test_tampered_salt_is_rejected() {
        let mut sealed = seal(b"payload", pass()).unwrap();
        sealed[HEADER_PREFIX_LEN] ^= 0xff;
        assert!(matches!(
            open(&sealed, pass()).unwrap_err(),
            VaultError::AuthenticationFailed
        ));
    }

    /// Two seals of identical input must differ, or an observer could tell
    /// that the same model was pushed twice.
    #[test]
    fn test_each_seal_uses_a_fresh_salt_and_nonce() {
        let a = seal(b"same input", pass()).unwrap();
        let b = seal(b"same input", pass()).unwrap();
        assert_ne!(a, b, "two seals of the same payload were identical");
        assert_eq!(open(&a, pass()).unwrap(), open(&b, pass()).unwrap());
    }

    /// Portability is the reason the salt travels with the object: a peer who
    /// knows only the passphrase, with no access to the originating vault,
    /// must be able to open it.
    #[test]
    fn test_openable_without_the_originating_vault() {
        let sealed = seal(b"shared model", pass()).unwrap();
        // No vault, no config, no salt from anywhere else -- just the passphrase.
        assert_eq!(open(&sealed, pass()).unwrap(), b"shared model");
    }

    /// The 5.0 rename changed the magic. Every object already in a bucket
    /// still carries the 4.x one, so reading must accept both — otherwise the
    /// rename silently orphans them, and `is_sealed` reporting `false` would
    /// hand the caller raw ciphertext as if it were a model.
    #[test]
    fn test_objects_sealed_before_the_rename_still_open() {
        let sealed = seal(b"sealed by 4.x", pass()).unwrap();

        // Rewrite the header to exactly what 4.6.x wrote. Only the magic
        // differed; everything after it is byte-identical.
        let mut legacy = sealed.clone();
        legacy[..LEGACY_MAGIC.len()].copy_from_slice(LEGACY_MAGIC);

        assert!(
            is_sealed(&legacy),
            "a 4.x object must not read as plaintext"
        );
        assert_eq!(
            open(&legacy, pass()).unwrap(),
            b"sealed by 4.x",
            "a 4.x sealed object must still decrypt under 5.0"
        );
    }

    /// New objects carry the new magic — the compatibility above is read-only.
    #[test]
    fn test_seal_writes_the_current_magic() {
        let sealed = seal(b"payload", pass()).unwrap();
        assert_eq!(&sealed[..MAGIC.len()], MAGIC);
        assert_ne!(&sealed[..LEGACY_MAGIC.len()], LEGACY_MAGIC);
    }

    #[test]
    fn test_is_sealed_discriminates_legacy_plaintext() {
        assert!(is_sealed(&seal(b"x", pass()).unwrap()));
        assert!(!is_sealed(b"raw plaintext model bytes"));
        assert!(!is_sealed(b""));
        assert!(!is_sealed(b"AIMV"));
    }

    #[test]
    fn test_legacy_plaintext_gets_an_actionable_error() {
        let err = open(b"raw plaintext model bytes", pass()).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("IRONSEAL"), "unhelpful message: {msg}");
    }

    #[test]
    fn test_truncated_object_is_rejected_without_panicking() {
        let sealed = seal(b"payload", pass()).unwrap();
        for cut in [1, 8, 9, 11, HEADER_PREFIX_LEN, HEADER_PREFIX_LEN + 4] {
            let truncated = &sealed[..cut.min(sealed.len())];
            assert!(
                open(truncated, pass()).is_err(),
                "accepted {cut}-byte object"
            );
        }
        // Header intact, body absent.
        let no_body = &sealed[..HEADER_PREFIX_LEN + salt_len()];
        assert!(open(no_body, pass()).is_err());
    }

    #[test]
    fn test_unknown_version_is_refused() {
        let mut sealed = seal(b"payload", pass()).unwrap();
        sealed[8] = 99;
        let msg = open(&sealed, pass()).unwrap_err().to_string();
        assert!(msg.contains("version"), "unhelpful message: {msg}");
    }

    #[test]
    fn test_empty_payload_roundtrips() {
        let sealed = seal(b"", pass()).unwrap();
        assert!(open(&sealed, pass()).unwrap().is_empty());
    }
}
