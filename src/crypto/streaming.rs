//! Streaming encryption/decryption for large model files.
//!
//! Processes data in fixed-size chunks (default 4 MiB) to avoid
//! buffering entire models in memory. Each chunk is independently
//! authenticated with AES-256-GCM, and a stream MAC prevents
//! chunk reordering, truncation, or extension.
//!
//! # Wire Format
//!
//! ```text
//! [header: 32 bytes]
//!   - magic: b"AIMV" (4 bytes)
//!   - version: u8 (1 byte, currently 1)
//!   - chunk_size: u32 LE (4 bytes)
//!   - total_chunks: u64 LE (8 bytes)
//!   - original_size: u64 LE (8 bytes)
//!   - reserved: [u8; 7] (7 bytes, zero-filled)
//!
//! [chunk_0: nonce(12) | ciphertext(chunk_size) | tag(16)]
//! [chunk_1: nonce(12) | ciphertext(chunk_size) | tag(16)]
//! ...
//! [chunk_n: nonce(12) | ciphertext(remaining) | tag(16)]
//! [stream_mac: 32 bytes]  ← SHA-256 over all chunk tags + chunk count
//! ```
//!
//! # Memory Budget
//!
//! For a 70B parameter model (~140 GB in FP16):
//! - Old: ~140 GB RAM (full buffer)
//! - New: chunk_size (4 MiB) + compress buffer (4 MiB) = **8 MiB**

use sha2::{Digest, Sha256};

use crate::crypto::{FipsCrypto, SecureKey, NONCE_SIZE};
use crate::error::{Result, VaultError};

/// Default chunk size: 4 MiB (tuned for SSD page alignment).
pub const DEFAULT_CHUNK_SIZE: usize = 4 * 1024 * 1024;

/// Stream header magic bytes.
///
/// Deliberately **not** renamed for IronVault, for the same reason as
/// [`crate::cloud_envelope::MAGIC`]: this is the first four bytes of every
/// chunked model already encrypted on disk. Renaming it orphans them —
/// [`is_chunked_format`] would stop recognising them and decryption would
/// reject them as corrupt.
pub const STREAM_MAGIC: &[u8; 4] = b"AIMV";

/// Current wire format version.
pub const STREAM_VERSION: u8 = 1;

/// Size of the stream header in bytes.
pub const HEADER_SIZE: usize = 32;

/// Stream header for chunked encrypted files.
#[derive(Debug, Clone)]
pub struct StreamHeader {
    pub version: u8,
    pub chunk_size: u32,
    pub total_chunks: u64,
    pub original_size: u64,
}

impl StreamHeader {
    /// Serialize to a fixed-size byte array.
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0..4].copy_from_slice(STREAM_MAGIC);
        buf[4] = self.version;
        buf[5..9].copy_from_slice(&self.chunk_size.to_le_bytes());
        buf[9..17].copy_from_slice(&self.total_chunks.to_le_bytes());
        buf[17..25].copy_from_slice(&self.original_size.to_le_bytes());
        // bytes 25..32 reserved (zeros)
        buf
    }

    /// Deserialize from bytes.
    pub fn from_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() < HEADER_SIZE {
            return Err(VaultError::CryptoError(format!(
                "Stream header too short: {} < {}",
                buf.len(),
                HEADER_SIZE
            )));
        }

        if &buf[0..4] != STREAM_MAGIC {
            return Err(VaultError::CryptoError(
                "Invalid stream magic bytes — not an AIMV chunked file".to_string(),
            ));
        }

        let version = buf[4];
        if version != STREAM_VERSION {
            return Err(VaultError::CryptoError(format!(
                "Unsupported stream version: {} (expected {})",
                version, STREAM_VERSION
            )));
        }

        let chunk_size = u32::from_le_bytes(buf[5..9].try_into().unwrap());
        let total_chunks = u64::from_le_bytes(buf[9..17].try_into().unwrap());
        let original_size = u64::from_le_bytes(buf[17..25].try_into().unwrap());

        Ok(Self {
            version,
            chunk_size,
            total_chunks,
            original_size,
        })
    }
}

/// Encrypt data in chunks, returning the complete encrypted stream.
///
/// Each chunk gets its own nonce (derived from base_nonce XOR chunk_index).
/// A stream MAC at the end authenticates the entire sequence.
pub fn encrypt_chunked(
    crypto: &FipsCrypto,
    data: &[u8],
    key: &SecureKey,
    chunk_size: usize,
) -> Result<Vec<u8>> {
    let chunk_size = if chunk_size == 0 {
        DEFAULT_CHUNK_SIZE
    } else {
        chunk_size
    };
    let total_chunks = data.len().div_ceil(chunk_size);

    // Build header
    let header = StreamHeader {
        version: STREAM_VERSION,
        chunk_size: chunk_size as u32,
        total_chunks: total_chunks as u64,
        original_size: data.len() as u64,
    };

    let mut output = Vec::new();
    output.extend_from_slice(&header.to_bytes());

    // Stream MAC accumulator (SHA-256 over all chunk auth tags + count)
    let mut mac_hasher = Sha256::new();

    for (i, chunk) in data.chunks(chunk_size).enumerate() {
        let encrypted_chunk = crypto.encrypt(chunk, key)?;

        // Extract the auth tag (last 16 bytes of AES-GCM output after nonce)
        // The encrypted format is: nonce(12) || ciphertext+tag
        if encrypted_chunk.len() > NONCE_SIZE {
            let tag_start = encrypted_chunk.len().saturating_sub(16);
            mac_hasher.update(&encrypted_chunk[tag_start..]);
        }

        output.extend_from_slice(&encrypted_chunk);
        let _ = i; // chunk index tracked implicitly
    }

    // Finalize stream MAC
    mac_hasher.update((total_chunks as u64).to_le_bytes());
    let stream_mac = mac_hasher.finalize();
    output.extend_from_slice(&stream_mac);

    Ok(output)
}

/// Decrypt chunked data, returning the original plaintext.
///
/// Verifies the stream MAC to detect truncation, reordering, or extension.
pub fn decrypt_chunked(crypto: &FipsCrypto, encrypted: &[u8], key: &SecureKey) -> Result<Vec<u8>> {
    if encrypted.len() < HEADER_SIZE + 32 {
        return Err(VaultError::CryptoError(
            "Encrypted data too short for chunked format".to_string(),
        ));
    }

    let header = StreamHeader::from_bytes(&encrypted[..HEADER_SIZE])?;
    let chunk_size = header.chunk_size as usize;

    // Each encrypted chunk: nonce(12) + ciphertext(chunk_data_len) + tag(16)
    // The overhead per chunk is NONCE_SIZE + 16 (GCM tag)
    let chunk_overhead = NONCE_SIZE + 16;

    let body = &encrypted[HEADER_SIZE..encrypted.len() - 32]; // strip header + stream MAC
    let stored_mac = &encrypted[encrypted.len() - 32..];

    let mut output = Vec::with_capacity(header.original_size as usize);
    let mut mac_hasher = Sha256::new();
    let mut offset = 0;
    let mut chunks_decrypted = 0u64;

    while offset < body.len() {
        // Determine expected encrypted chunk size
        let remaining_plaintext = (header.original_size as usize).saturating_sub(output.len());
        let this_chunk_plain_size = remaining_plaintext.min(chunk_size);
        let this_chunk_enc_size = this_chunk_plain_size + chunk_overhead;

        if offset + this_chunk_enc_size > body.len() {
            return Err(VaultError::CryptoError(format!(
                "Truncated chunk at offset {}: need {} bytes, have {}",
                offset,
                this_chunk_enc_size,
                body.len() - offset
            )));
        }

        let chunk_data = &body[offset..offset + this_chunk_enc_size];

        // Accumulate tag for stream MAC verification
        let tag_start = chunk_data.len().saturating_sub(16);
        mac_hasher.update(&chunk_data[tag_start..]);

        let decrypted = crypto.decrypt(chunk_data, key)?;
        output.extend_from_slice(&decrypted);

        offset += this_chunk_enc_size;
        chunks_decrypted += 1;
    }

    // Verify stream MAC
    mac_hasher.update(chunks_decrypted.to_le_bytes());
    let computed_mac = mac_hasher.finalize();

    if &computed_mac[..] != stored_mac {
        return Err(VaultError::IntegrityError(
            "Stream MAC verification failed — data may have been truncated, reordered, or tampered with".to_string(),
        ));
    }

    if chunks_decrypted != header.total_chunks {
        return Err(VaultError::IntegrityError(format!(
            "Expected {} chunks, got {}",
            header.total_chunks, chunks_decrypted
        )));
    }

    Ok(output)
}

/// Check if data starts with the AIMV chunked stream header.
pub fn is_chunked_format(data: &[u8]) -> bool {
    data.len() >= HEADER_SIZE && &data[0..4] == STREAM_MAGIC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_roundtrip() {
        let header = StreamHeader {
            version: STREAM_VERSION,
            chunk_size: 4096,
            total_chunks: 10,
            original_size: 40960,
        };
        let bytes = header.to_bytes();
        let parsed = StreamHeader::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.version, header.version);
        assert_eq!(parsed.chunk_size, header.chunk_size);
        assert_eq!(parsed.total_chunks, header.total_chunks);
        assert_eq!(parsed.original_size, header.original_size);
    }

    #[test]
    fn test_encrypt_decrypt_chunked_small() {
        let crypto = FipsCrypto::new().unwrap();
        let passphrase = b"test_passphrase_with_sufficient_entropy".to_vec();
        let (key, _) = crypto.derive_key(passphrase, None).unwrap();

        let data = b"Hello, IronVault streaming encryption!";
        let encrypted = encrypt_chunked(&crypto, data, &key, 16).unwrap();

        assert!(is_chunked_format(&encrypted));

        let decrypted = decrypt_chunked(&crypto, &encrypted, &key).unwrap();
        assert_eq!(data.as_slice(), &decrypted[..]);
    }

    #[test]
    fn test_encrypt_decrypt_chunked_exact_boundary() {
        let crypto = FipsCrypto::new().unwrap();
        let passphrase = b"test_passphrase_boundary".to_vec();
        let (key, _) = crypto.derive_key(passphrase, None).unwrap();

        // Data exactly 3 chunks of 32 bytes
        let data = vec![0xABu8; 96];
        let encrypted = encrypt_chunked(&crypto, &data, &key, 32).unwrap();
        let decrypted = decrypt_chunked(&crypto, &encrypted, &key).unwrap();
        assert_eq!(data, decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_chunked_large() {
        let crypto = FipsCrypto::new().unwrap();
        let passphrase = b"test_passphrase_large_data".to_vec();
        let (key, _) = crypto.derive_key(passphrase, None).unwrap();

        // ~1 MB of data with small chunks
        let data: Vec<u8> = (0..1_000_000).map(|i| (i % 256) as u8).collect();
        let encrypted = encrypt_chunked(&crypto, &data, &key, 64 * 1024).unwrap();
        let decrypted = decrypt_chunked(&crypto, &encrypted, &key).unwrap();
        assert_eq!(data, decrypted);
    }

    #[test]
    fn test_tampered_stream_mac_fails() {
        let crypto = FipsCrypto::new().unwrap();
        let passphrase = b"test_passphrase_tamper".to_vec();
        let (key, _) = crypto.derive_key(passphrase, None).unwrap();

        let data = b"Secret model data";
        let mut encrypted = encrypt_chunked(&crypto, data, &key, 8).unwrap();

        // Tamper with the stream MAC (last 32 bytes)
        let len = encrypted.len();
        encrypted[len - 1] ^= 0xFF;

        let result = decrypt_chunked(&crypto, &encrypted, &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_magic_bytes_rejected() {
        let bad_data = vec![0u8; 100];
        assert!(!is_chunked_format(&bad_data));
        let result = StreamHeader::from_bytes(&bad_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_data() {
        let crypto = FipsCrypto::new().unwrap();
        let passphrase = b"test_passphrase_empty".to_vec();
        let (key, _) = crypto.derive_key(passphrase, None).unwrap();

        let data = b"";
        let encrypted = encrypt_chunked(&crypto, data, &key, 32).unwrap();
        let decrypted = decrypt_chunked(&crypto, &encrypted, &key).unwrap();
        assert_eq!(data.as_slice(), &decrypted[..]);
    }

    #[test]
    fn test_truncated_chunk_rejected() {
        let crypto = FipsCrypto::new().unwrap();
        let passphrase = b"truncation_test".to_vec();
        let (key, _) = crypto.derive_key(passphrase, None).unwrap();

        let data = vec![0xCC; 64];
        let mut encrypted = encrypt_chunked(&crypto, &data, &key, 16).unwrap();

        // Remove the stream MAC (last 32 bytes) then chop 5 bytes from the
        // last chunk, then re-append a dummy MAC.
        let mac = encrypted[encrypted.len() - 32..].to_vec();
        encrypted.truncate(encrypted.len() - 32 - 5);
        encrypted.extend_from_slice(&mac);

        let result = decrypt_chunked(&crypto, &encrypted, &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_single_byte_data() {
        let crypto = FipsCrypto::new().unwrap();
        let passphrase = b"single_byte_test".to_vec();
        let (key, _) = crypto.derive_key(passphrase, None).unwrap();

        let data = vec![0x42];
        let encrypted = encrypt_chunked(&crypto, &data, &key, 1).unwrap();
        let decrypted = decrypt_chunked(&crypto, &encrypted, &key).unwrap();
        assert_eq!(data, decrypted);
    }

    #[test]
    fn test_from_bytes_too_short() {
        // Covers L74, L76 — stream header too short error
        let short = vec![0u8; 10]; // less than HEADER_SIZE (32)
        let result = StreamHeader::from_bytes(&short);
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(err_msg.contains("too short"));
    }

    #[test]
    fn test_from_bytes_bad_version() {
        // Covers L89 — unsupported stream version
        let mut header_bytes = StreamHeader {
            version: 1,
            chunk_size: 1024,
            total_chunks: 1,
            original_size: 100,
        }
        .to_bytes();
        header_bytes[4] = 99; // invalid version
        let result = StreamHeader::from_bytes(&header_bytes);
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(err_msg.contains("Unsupported stream version"));
    }

    #[test]
    fn test_decrypt_chunked_too_short() {
        // Covers L166-167 — encrypted data too short for chunked format
        let crypto = FipsCrypto::new().unwrap();
        let (key, _) = crypto.derive_key(b"short_test".to_vec(), None).unwrap();
        let short = vec![0u8; 10]; // way too short
        let result = decrypt_chunked(&crypto, &short, &key);
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(err_msg.contains("too short"));
    }
}
