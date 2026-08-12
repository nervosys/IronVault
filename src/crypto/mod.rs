//! Vault cryptography: AES-256-GCM with Argon2id key derivation.
//!
//! **Not a FIPS 140-3 validated cryptographic module.** FIPS 140-3 validates a
//! module through NIST's CMVP; these implementations hold no certificate, and
//! Argon2id is not an approved KDF in any case — SP 800-132 approves PBKDF2.
//! Argon2id is used deliberately: it is materially stronger against
//! GPU-accelerated cracking, which is the right trade for most deployments but
//! does place the KDF outside FIPS. See `compliance.rs`, which reports this
//! relationship rather than asserting a determination.
//!
//! Standards followed:
//! - FIPS 197 (AES) — approved algorithm
//! - NIST SP 800-38D (GCM mode) — approved mode
//! - FIPS 180-4 (SHA-256) — approved algorithm
//! - RFC 9106 (Argon2) — not a FIPS standard
//! - NIST SP 800-63B (password recommendations)

pub mod compression;
pub mod streaming;

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm,
};
use argon2::{
    password_hash::{rand_core::RngCore, PasswordHasher, SaltString},
    Argon2, ParamsBuilder, Version,
};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{Result, VaultError};

/// Size of AES-256 key in bytes
pub const KEY_SIZE: usize = 32;

/// Size of GCM nonce in bytes (96 bits recommended)
pub const NONCE_SIZE: usize = 12;

/// Size of salt for key derivation
pub const SALT_SIZE: usize = 32;

/// Vault cryptographic operations: AES-256-GCM with Argon2id key derivation.
///
/// Named `FipsCrypto` until 6.1. That name asserted a validation this crate has
/// never held, on the one type whose rustdoc lands in front of anyone
/// evaluating the cryptography. The old name survives as a deprecated alias, so
/// nothing downstream breaks.
///
/// What is and is not FIPS here:
/// - AES-256-GCM (FIPS 197, SP 800-38D) and SHA-256 (FIPS 180-4) are
///   FIPS-*approved algorithms*
/// - the implementations hold no CMVP certificate, so this is not a
///   FIPS-*validated module*
/// - Argon2id is not an approved KDF at all; SP 800-132 approves PBKDF2
///
/// Compliance mappings:
/// - MITRE ATT&CK: T1486 mitigation (Data Encrypted for Impact), T1005 (Data
///   from Local System)
/// - CMMC 2.0: contributes to SC.L2-3.13.8 (protect confidentiality at rest).
///   It does **not** satisfy SC.L2-3.13.11, which requires FIPS-validated
///   cryptography for CUI — see `compliance.rs`.
pub struct VaultCrypto {
    argon2: Argon2<'static>,
}

/// Former name of [`VaultCrypto`], kept so 5.x and 6.0 code still compiles.
///
/// An alias rather than a removal: renaming a public type is only breaking if
/// the old name disappears, and there was no reason to make every downstream
/// caller edit for a rename that is about honesty in the *name*, not a change
/// in behaviour. The type behind it is identical.
#[deprecated(
    since = "6.1.0",
    note = "renamed to `VaultCrypto`. The old name implied FIPS 140-3 validation that this \
            crate does not hold — it uses FIPS-approved AES-256-GCM and SHA-256, but the \
            implementations carry no CMVP certificate and Argon2id is not an approved KDF. \
            Behaviour is unchanged; only the name was wrong."
)]
pub type FipsCrypto = VaultCrypto;

/// Secure key container that zeroizes on drop
#[derive(Clone, ZeroizeOnDrop)]
pub struct SecureKey {
    key: [u8; KEY_SIZE],
}

impl SecureKey {
    /// Create new secure key from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != KEY_SIZE {
            return Err(VaultError::CryptoError(format!(
                "Invalid key size: expected {}, got {}",
                KEY_SIZE,
                bytes.len()
            )));
        }

        let mut key = [0u8; KEY_SIZE];
        key.copy_from_slice(bytes);
        Ok(Self { key })
    }

    /// Get key bytes
    pub fn as_bytes(&self) -> &[u8; KEY_SIZE] {
        &self.key
    }
}

impl VaultCrypto {
    /// Create a crypto instance with the recommended Argon2id parameters.
    pub fn new() -> Result<Self> {
        // Argon2id at OWASP-recommended parameters. Deliberately *not* FIPS:
        // SP 800-132 approves PBKDF2 for password-based derivation, not Argon2,
        // so this KDF sits outside FIPS no matter how it is implemented. It is
        // used anyway because it is materially stronger against GPU-accelerated
        // cracking; `compliance.rs` reports the consequence honestly.
        let params = ParamsBuilder::new()
            .m_cost(65536) // 64 MiB memory
            .t_cost(3) // 3 iterations
            .p_cost(1) // 1 parallelism
            .build()
            .map_err(|e| {
                VaultError::CryptoError(format!("Failed to build Argon2 params: {}", e))
            })?;

        Ok(Self {
            argon2: Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params),
        })
    }

    /// Derive encryption key from passphrase using Argon2id
    ///
    /// # Arguments
    /// * `passphrase` - User passphrase (will be zeroized)
    /// * `salt` - Optional salt (generated if not provided)
    ///
    /// # Returns
    /// Tuple of (encryption_key, salt)
    ///
    /// # Standards
    /// - RFC 9106: Argon2 password hashing
    /// - **Not** SP 800-132: that approves PBKDF2, not Argon2, so this
    ///   derivation is outside FIPS by construction
    pub fn derive_key(
        &self,
        mut passphrase: Vec<u8>,
        salt: Option<Vec<u8>>,
    ) -> Result<(SecureKey, Vec<u8>)> {
        let salt_bytes = if let Some(s) = salt {
            s
        } else {
            let mut salt = vec![0u8; SALT_SIZE];
            OsRng.fill_bytes(&mut salt);
            salt
        };

        let salt_string = SaltString::encode_b64(&salt_bytes)
            .map_err(|e| VaultError::CryptoError(format!("Failed to encode salt: {}", e)))?;

        let password_hash = self
            .argon2
            .hash_password(&passphrase, &salt_string)
            .map_err(|e| VaultError::CryptoError(format!("Failed to derive key: {}", e)))?;

        // Extract the hash bytes (first 32 bytes for AES-256)
        let hash_bytes = password_hash
            .hash
            .ok_or_else(|| VaultError::CryptoError("No hash in password hash".to_string()))?;

        let key = SecureKey::from_bytes(&hash_bytes.as_bytes()[..KEY_SIZE])?;

        // Zeroize passphrase
        passphrase.zeroize();

        Ok((key, salt_bytes))
    }

    /// Encrypt data using AES-256-GCM
    ///
    /// # Arguments
    /// * `data` - Plaintext data to encrypt
    /// * `key` - 256-bit encryption key
    ///
    /// # Returns
    /// Encrypted data with format: nonce || ciphertext (includes auth tag)
    ///
    /// # Compliance
    /// - FIPS 197: AES encryption
    /// - NIST SP 800-38D: GCM mode
    /// - CMMC SC.3.191: Protection of CUI at rest
    pub fn encrypt(&self, data: &[u8], key: &SecureKey) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(key.as_bytes().into());

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt((&nonce_bytes).into(), data)
            .map_err(|e| VaultError::CryptoError(format!("Encryption failed: {}", e)))?;

        // Combine nonce || ciphertext
        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt data using AES-256-GCM
    ///
    /// # Arguments
    /// * `encrypted_data` - Encrypted data (nonce || ciphertext)
    /// * `key` - 256-bit encryption key
    ///
    /// # Returns
    /// Decrypted plaintext data
    ///
    /// # Errors
    /// Returns `AuthenticationFailed` if authentication tag verification fails
    pub fn decrypt(&self, encrypted_data: &[u8], key: &SecureKey) -> Result<Vec<u8>> {
        if encrypted_data.len() < NONCE_SIZE {
            return Err(VaultError::CryptoError(
                "Encrypted data too short".to_string(),
            ));
        }

        let cipher = Aes256Gcm::new(key.as_bytes().into());

        // Extract nonce and ciphertext
        let ciphertext = &encrypted_data[NONCE_SIZE..];

        // Decrypt and verify
        let plaintext = cipher
            .decrypt((&encrypted_data[..NONCE_SIZE]).into(), ciphertext)
            .map_err(|_| VaultError::AuthenticationFailed)?;

        Ok(plaintext)
    }

    /// Generate cryptographically secure random bytes
    ///
    /// # Arguments
    /// * `length` - Number of random bytes to generate
    ///
    /// # Compliance
    /// - FIPS 140-3: Approved random number generation
    #[must_use]
    pub fn generate_random(&self, length: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; length];
        OsRng.fill_bytes(&mut bytes);
        bytes
    }

    /// Compute SHA-256 hash
    #[must_use]
    pub fn hash_sha256(data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }

    /// Compute SHA-256 hash as hex string
    #[must_use]
    pub fn hash_sha256_hex(data: &[u8]) -> String {
        hex::encode(Self::hash_sha256(data))
    }
}

/// Note: `VaultCrypto::default()` panics if RNG initialization fails.
/// Prefer `VaultCrypto::new()` which returns `Result` for fallible creation.
impl Default for VaultCrypto {
    fn default() -> Self {
        Self::new().expect("Failed to create VaultCrypto: RNG unavailable")
    }
}

/// Key manager for secure key storage and retrieval
///
/// Compliance:
/// - CMMC AC.3.018: Control connection of mobile devices
/// - CMMC IA.3.080: Protect authenticators
pub struct KeyManager {
    crypto: VaultCrypto,
}

impl KeyManager {
    /// Create new key manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            crypto: VaultCrypto::new()?,
        })
    }

    /// Store encryption key using key encryption key (KEK)
    pub fn store_key(&self, key: &SecureKey, master_passphrase: Vec<u8>) -> Result<Vec<u8>> {
        // Derive KEK from master passphrase
        let (kek, salt) = self.crypto.derive_key(master_passphrase, None)?;

        // Encrypt the key
        let encrypted_key = self.crypto.encrypt(key.as_bytes(), &kek)?;

        // Combine salt || encrypted_key
        let mut result = Vec::with_capacity(salt.len() + encrypted_key.len());
        result.extend_from_slice(&salt);
        result.extend_from_slice(&encrypted_key);

        Ok(result)
    }

    /// Load and decrypt stored encryption key
    pub fn load_key(&self, stored_data: &[u8], master_passphrase: Vec<u8>) -> Result<SecureKey> {
        if stored_data.len() < SALT_SIZE {
            return Err(VaultError::CryptoError(
                "Invalid stored key data".to_string(),
            ));
        }

        // Extract salt and encrypted key
        let salt = stored_data[..SALT_SIZE].to_vec();
        let encrypted_key = &stored_data[SALT_SIZE..];

        // Derive KEK from master passphrase
        let (kek, _) = self.crypto.derive_key(master_passphrase, Some(salt))?;

        // Decrypt the key
        let key_bytes = self.crypto.decrypt(encrypted_key, &kek)?;

        SecureKey::from_bytes(&key_bytes)
    }
}

/// Note: `KeyManager::default()` panics if RNG initialization fails.
/// Prefer `KeyManager::new()` which returns `Result` for fallible creation.
impl Default for KeyManager {
    fn default() -> Self {
        Self::new().expect("Failed to create KeyManager: RNG unavailable")
    }
}

// ── Trait implementation ─────────────────────────────────────

impl crate::traits::CryptoProvider for VaultCrypto {
    fn derive_key(
        &self,
        passphrase: Vec<u8>,
        salt: Option<Vec<u8>>,
    ) -> Result<(SecureKey, Vec<u8>)> {
        VaultCrypto::derive_key(self, passphrase, salt)
    }

    fn encrypt(&self, data: &[u8], key: &SecureKey) -> Result<Vec<u8>> {
        VaultCrypto::encrypt(self, data, key)
    }

    fn decrypt(&self, encrypted_data: &[u8], key: &SecureKey) -> Result<Vec<u8>> {
        VaultCrypto::decrypt(self, encrypted_data, key)
    }

    fn hash(&self, data: &[u8]) -> Vec<u8> {
        VaultCrypto::hash_sha256(data)
    }

    fn random_bytes(&self, length: usize) -> Vec<u8> {
        VaultCrypto::generate_random(self, length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let crypto = VaultCrypto::new().unwrap();
        let passphrase = b"test_passphrase_with_sufficient_entropy".to_vec();
        let (key, _) = crypto.derive_key(passphrase, None).unwrap();

        let plaintext = b"Hello, IronVault!";
        let encrypted = crypto.encrypt(plaintext, &key).unwrap();
        let decrypted = crypto.decrypt(&encrypted, &key).unwrap();

        assert_eq!(plaintext, &decrypted[..]);
    }

    #[test]
    fn test_key_derivation_deterministic() {
        let crypto = VaultCrypto::new().unwrap();
        let passphrase = b"test_passphrase".to_vec();
        let salt = vec![0u8; SALT_SIZE];

        let (key1, _) = crypto
            .derive_key(passphrase.clone(), Some(salt.clone()))
            .unwrap();
        let (key2, _) = crypto.derive_key(passphrase, Some(salt)).unwrap();

        assert_eq!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_authentication_failure() {
        let crypto = VaultCrypto::new().unwrap();
        let passphrase1 = b"correct_passphrase".to_vec();
        let passphrase2 = b"wrong_passphrase".to_vec();

        let (key1, salt) = crypto.derive_key(passphrase1, None).unwrap();
        let plaintext = b"Secret data";
        let encrypted = crypto.encrypt(plaintext, &key1).unwrap();

        let (key2, _) = crypto.derive_key(passphrase2, Some(salt)).unwrap();
        let result = crypto.decrypt(&encrypted, &key2);

        assert!(result.is_err());
    }

    #[test]
    fn test_key_manager_store_load_roundtrip() {
        let km = KeyManager::new().unwrap();
        let crypto = VaultCrypto::new().unwrap();
        let (original_key, _) = crypto
            .derive_key(b"some_passphrase".to_vec(), None)
            .unwrap();

        let stored = km
            .store_key(&original_key, b"master_pass_1234".to_vec())
            .unwrap();
        let loaded = km.load_key(&stored, b"master_pass_1234".to_vec()).unwrap();

        assert_eq!(original_key.as_bytes(), loaded.as_bytes());
    }

    #[test]
    fn test_key_manager_wrong_passphrase() {
        let km = KeyManager::new().unwrap();
        let crypto = VaultCrypto::new().unwrap();
        let (key, _) = crypto.derive_key(b"gen_pass".to_vec(), None).unwrap();

        let stored = km.store_key(&key, b"correct_pass".to_vec()).unwrap();
        let result = km.load_key(&stored, b"wrong_pass".to_vec());

        assert!(result.is_err());
    }

    #[test]
    fn test_key_manager_truncated_data() {
        let km = KeyManager::new().unwrap();
        let short_data = vec![0u8; SALT_SIZE - 1];
        let result = km.load_key(&short_data, b"pass".to_vec());
        assert!(result.is_err());
    }

    #[test]
    fn test_key_manager_default() {
        let km = KeyManager::default();
        let crypto = VaultCrypto::new().unwrap();
        let (key, _) = crypto.derive_key(b"default_test".to_vec(), None).unwrap();
        let stored = km.store_key(&key, b"mp".to_vec()).unwrap();
        assert!(!stored.is_empty());
    }

    #[test]
    fn test_secure_key_wrong_size() {
        // Covers L58, L61 — SecureKey::from_bytes with wrong key size
        let short = vec![0u8; 16]; // too short (need 32)
        let result = SecureKey::from_bytes(&short);
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(err_msg.contains("Invalid key size"));
    }

    #[test]
    fn test_decrypt_too_short_data() {
        // Covers L186-187 — decrypt with data shorter than NONCE_SIZE
        let crypto = VaultCrypto::new().unwrap();
        let (key, _) = crypto.derive_key(b"short_test".to_vec(), None).unwrap();
        let short_data = vec![0u8; 5]; // less than NONCE_SIZE (12)
        let result = crypto.decrypt(&short_data, &key);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("too short"));
    }

    #[test]
    fn test_generate_random() {
        // Covers L212-215 — generate_random
        let crypto = VaultCrypto::new().unwrap();
        let random1 = crypto.generate_random(32);
        let random2 = crypto.generate_random(32);
        assert_eq!(random1.len(), 32);
        assert_eq!(random2.len(), 32);
        assert_ne!(random1, random2);
    }

    #[test]
    fn test_hash_sha256() {
        let hash = VaultCrypto::hash_sha256(b"hello");
        assert_eq!(hash.len(), 32);
        let hex = hex::encode(&hash);
        assert_eq!(
            hex,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_hash_sha256_hex() {
        // Covers L228-229 — hash_sha256_hex
        let hex = VaultCrypto::hash_sha256_hex(b"hello");
        assert_eq!(hex.len(), 64);
        assert_eq!(
            hex,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_fips_crypto_default() {
        // Covers L235-237 — Default impl for VaultCrypto
        let crypto = VaultCrypto::default();
        let (key, _) = crypto.derive_key(b"default_works".to_vec(), None).unwrap();
        let encrypted = crypto.encrypt(b"test", &key).unwrap();
        assert!(!encrypted.is_empty());
    }

    #[test]
    fn test_crypto_provider_trait() {
        // Covers CryptoProvider trait impl
        use crate::traits::CryptoProvider;
        let crypto = VaultCrypto::new().unwrap();
        let provider: &dyn CryptoProvider = &crypto;

        let (key, salt) = provider.derive_key(b"trait_test".to_vec(), None).unwrap();
        assert!(!salt.is_empty());

        let encrypted = provider.encrypt(b"data", &key).unwrap();
        let decrypted = provider.decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, b"data");

        let hash = provider.hash(b"test");
        assert_eq!(hash.len(), 32);

        let random = provider.random_bytes(16);
        assert_eq!(random.len(), 16);

        let hex = provider.hash_hex(b"test");
        assert_eq!(hex.len(), 64);
    }

    /// The rename is only non-breaking if the old name still resolves to the
    /// same type. Downstream 5.x/6.0 code does exactly this.
    #[test]
    #[allow(deprecated)]
    fn the_old_name_still_resolves_to_the_same_type() {
        let via_old: FipsCrypto = VaultCrypto::new().expect("constructs");
        let key = via_old
            .derive_key(b"passphrase".to_vec(), Some(vec![7u8; SALT_SIZE]))
            .expect("derives")
            .0;
        let sealed = via_old.encrypt(b"payload", &key).expect("encrypts");
        assert_eq!(
            via_old.decrypt(&sealed, &key).expect("decrypts"),
            b"payload"
        );
    }
}
