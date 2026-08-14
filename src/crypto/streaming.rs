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
//!   - magic: b"IRNV" (4 bytes; 4.x wrote b"AIMV", still read)
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
//!
//! That budget describes *writing*. Until 7.1 reading gave it all back:
//! [`decrypt_chunked`] takes the whole ciphertext as a slice and returns the
//! whole plaintext as a `Vec`, so a reader had to hold both. [`ChunkDecryptReader`]
//! is the streaming counterpart — see its docs for why an inference engine
//! needs one.

use std::io::Read;

use sha2::{Digest, Sha256};

use crate::crypto::{SecureKey, VaultCrypto, NONCE_SIZE};
use crate::error::{Result, VaultError};

/// Per-chunk framing overhead: a 12-byte nonce and a 16-byte GCM tag.
const CHUNK_OVERHEAD: usize = NONCE_SIZE + 16;

/// Size of the trailing stream MAC.
const STREAM_MAC_SIZE: usize = 32;

/// Default chunk size: 4 MiB (tuned for SSD page alignment).
pub const DEFAULT_CHUNK_SIZE: usize = 4 * 1024 * 1024;

/// Stream header magic bytes, written from 5.0 on.
pub const STREAM_MAGIC: &[u8; 4] = b"IRNV";

/// The 4.x stream magic, still accepted on read.
///
/// This is the first four bytes of every chunked model already encrypted on
/// disk — including inside vaults that will never be rewritten, since nothing
/// re-encrypts a stored model. [`StreamHeader::from_bytes`] and
/// [`is_chunked_format`] accept both; only [`StreamHeader::to_bytes`] is
/// one-way.
pub const LEGACY_STREAM_MAGIC: &[u8; 4] = b"AIMV";

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

        // Both spellings: a model encrypted before the 5.0 rename carries the
        // legacy magic and must still decrypt.
        if &buf[0..4] != STREAM_MAGIC && &buf[0..4] != LEGACY_STREAM_MAGIC {
            return Err(VaultError::CryptoError(
                "Invalid stream magic bytes — not an IRNV chunked file".to_string(),
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
    crypto: &VaultCrypto,
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
pub fn decrypt_chunked(crypto: &VaultCrypto, encrypted: &[u8], key: &SecureKey) -> Result<Vec<u8>> {
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

/// Decrypt a chunked stream incrementally, holding one chunk at a time.
///
/// [`decrypt_chunked`] is slice-in, `Vec`-out: it needs the entire ciphertext
/// resident *and* allocates the entire plaintext. For a 17 GB model that is
/// ~34 GB of peak residency to read something that will immediately be written
/// somewhere else. This reader holds one chunk (4 MiB by default) regardless of
/// model size.
///
/// It exists for inference engines. IronWorks maps a model as one flat byte
/// range, page-locks parts of it for host-to-device transfer, and bakes those
/// host pointers into captured CUDA graphs — so it needs the plaintext to land
/// in one contiguous buffer it controls, and it cannot afford a second full-size
/// copy on the way there. Composed with a decompressor this reader writes
/// straight into that destination.
///
/// # Authentication, and when it happens
///
/// Every chunk is independently AES-256-GCM authenticated, so tampering *within*
/// a chunk fails that chunk's `decrypt` immediately, before its bytes are handed
/// out. The stream MAC — which is what detects reordering, truncation, or
/// extension — can only be checked once the last chunk has been read, exactly as
/// in [`decrypt_chunked`].
///
/// **This means a caller that stops reading early has verified nothing about the
/// stream as a whole.** Read to EOF: the final [`Read::read`] returning `Ok(0)`
/// is what runs the MAC check, and it is the only thing that can report
/// truncation. [`crate::Vault::read_model_into`] does this for you.
pub struct ChunkDecryptReader<'a, R: Read> {
    crypto: &'a VaultCrypto,
    key: &'a SecureKey,
    inner: R,
    header: StreamHeader,
    mac_hasher: Sha256,
    /// Plaintext of the chunk currently being served.
    plain: Vec<u8>,
    /// How much of `plain` has already been handed out.
    plain_pos: usize,
    /// Scratch holding one encrypted chunk.
    enc: Vec<u8>,
    /// Plaintext bytes emitted so far.
    emitted: u64,
    /// Chunks decrypted so far.
    chunks_done: u64,
    /// Set once the stream MAC has been checked.
    finished: bool,
}

impl<'a, R: Read> ChunkDecryptReader<'a, R> {
    /// Read and validate the stream header, leaving `reader` at the first chunk.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::CryptoError`] if the header is short, carries
    /// unknown magic, or declares an unsupported version.
    pub fn new(crypto: &'a VaultCrypto, key: &'a SecureKey, mut reader: R) -> Result<Self> {
        let mut header_buf = [0u8; HEADER_SIZE];
        reader.read_exact(&mut header_buf)?;
        let header = StreamHeader::from_bytes(&header_buf)?;

        Ok(Self {
            crypto,
            key,
            inner: reader,
            header,
            mac_hasher: Sha256::new(),
            plain: Vec::new(),
            plain_pos: 0,
            enc: Vec::new(),
            emitted: 0,
            chunks_done: 0,
            finished: false,
        })
    }

    /// Total plaintext this stream will yield, from the header.
    ///
    /// Note this is the size of what was *encrypted*, which is the compressed
    /// size when the model was stored with compression.
    pub fn plaintext_len(&self) -> u64 {
        self.header.original_size
    }

    /// Decrypt the next chunk into `self.plain`, or finalize the stream.
    fn advance(&mut self) -> Result<()> {
        if self.emitted >= self.header.original_size {
            return self.finalize();
        }

        let remaining = (self.header.original_size - self.emitted) as usize;
        let plain_len = remaining.min(self.header.chunk_size as usize);
        let enc_len = plain_len + CHUNK_OVERHEAD;

        self.enc.resize(enc_len, 0);
        self.inner.read_exact(&mut self.enc)?;

        // The GCM tag is the last 16 bytes; the stream MAC is taken over the
        // tags in order, so it must be accumulated whether or not the caller
        // ever reaches the end.
        self.mac_hasher.update(&self.enc[enc_len - 16..]);

        self.plain = self.crypto.decrypt(&self.enc, self.key)?;
        self.plain_pos = 0;
        self.emitted += self.plain.len() as u64;
        self.chunks_done += 1;
        Ok(())
    }

    /// Read the trailing MAC and verify the stream as a whole.
    fn finalize(&mut self) -> Result<()> {
        if self.finished {
            return Ok(());
        }

        let mut stored = [0u8; STREAM_MAC_SIZE];
        self.inner.read_exact(&mut stored)?;

        let mut hasher = std::mem::replace(&mut self.mac_hasher, Sha256::new());
        hasher.update(self.chunks_done.to_le_bytes());
        let computed = hasher.finalize();

        if computed[..] != stored[..] {
            return Err(VaultError::IntegrityError(
                "Stream MAC verification failed — data may have been truncated, reordered, or tampered with".to_string(),
            ));
        }

        if self.chunks_done != self.header.total_chunks {
            return Err(VaultError::IntegrityError(format!(
                "Expected {} chunks, got {}",
                self.header.total_chunks, self.chunks_done
            )));
        }

        self.finished = true;
        Ok(())
    }
}

impl<R: Read> Read for ChunkDecryptReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        loop {
            if self.plain_pos < self.plain.len() {
                let n = (self.plain.len() - self.plain_pos).min(buf.len());
                buf[..n].copy_from_slice(&self.plain[self.plain_pos..self.plain_pos + n]);
                self.plain_pos += n;
                return Ok(n);
            }

            if self.finished {
                return Ok(0);
            }

            // Either decrypts the next chunk or, once the declared plaintext is
            // exhausted, checks the stream MAC and sets `finished`.
            self.advance()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        }
    }
}

/// Check if data starts with the AIMV chunked stream header.
pub fn is_chunked_format(data: &[u8]) -> bool {
    data.len() >= HEADER_SIZE && (&data[0..4] == STREAM_MAGIC || &data[0..4] == LEGACY_STREAM_MAGIC)
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

    /// The 5.0 rename changed the stream magic. Models encrypted before it are
    /// sitting in vaults that nothing will ever rewrite, so reading must accept
    /// the old spelling or those models become permanently undecryptable.
    #[test]
    fn test_models_encrypted_before_the_rename_still_decrypt() {
        let crypto = VaultCrypto::new().unwrap();
        let passphrase = b"test_passphrase_with_sufficient_entropy".to_vec();
        let (key, _) = crypto.derive_key(passphrase, None).unwrap();

        let plaintext = b"weights encrypted by 4.x".repeat(200);
        let mut encrypted = encrypt_chunked(&crypto, &plaintext, &key, 1024).unwrap();

        // Downgrade the header to exactly what 4.6.x wrote. Only the four
        // magic bytes differed; the rest of the format is unchanged.
        encrypted[0..4].copy_from_slice(LEGACY_STREAM_MAGIC);

        assert!(
            is_chunked_format(&encrypted),
            "a 4.x chunked model must still be recognised as chunked"
        );
        assert_eq!(
            decrypt_chunked(&crypto, &encrypted, &key).unwrap(),
            plaintext,
            "a 4.x chunked model must still decrypt under 5.0"
        );
    }

    /// The compatibility above is read-only: new writes use the new magic.
    #[test]
    fn test_encrypt_writes_the_current_magic() {
        let crypto = VaultCrypto::new().unwrap();
        let (key, _) = crypto
            .derive_key(b"test_passphrase_with_sufficient_entropy".to_vec(), None)
            .unwrap();
        let encrypted = encrypt_chunked(&crypto, b"payload", &key, 1024).unwrap();
        assert_eq!(&encrypted[0..4], STREAM_MAGIC);
        assert_ne!(&encrypted[0..4], LEGACY_STREAM_MAGIC);
    }

    #[test]
    fn test_encrypt_decrypt_chunked_small() {
        let crypto = VaultCrypto::new().unwrap();
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
        let crypto = VaultCrypto::new().unwrap();
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
        let crypto = VaultCrypto::new().unwrap();
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
        let crypto = VaultCrypto::new().unwrap();
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
        let crypto = VaultCrypto::new().unwrap();
        let passphrase = b"test_passphrase_empty".to_vec();
        let (key, _) = crypto.derive_key(passphrase, None).unwrap();

        let data = b"";
        let encrypted = encrypt_chunked(&crypto, data, &key, 32).unwrap();
        let decrypted = decrypt_chunked(&crypto, &encrypted, &key).unwrap();
        assert_eq!(data.as_slice(), &decrypted[..]);
    }

    #[test]
    fn test_truncated_chunk_rejected() {
        let crypto = VaultCrypto::new().unwrap();
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
        let crypto = VaultCrypto::new().unwrap();
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

    /// Helper: crypto + key for the streaming-reader tests.
    fn crypto_and_key() -> (VaultCrypto, SecureKey) {
        let crypto = VaultCrypto::new().unwrap();
        let (key, _) = crypto
            .derive_key(b"streaming_reader_test_passphrase".to_vec(), None)
            .unwrap();
        (crypto, key)
    }

    /// The streaming reader must be byte-for-byte interchangeable with the
    /// buffered path. If it ever is not, every model read through it is
    /// silently corrupt in a way that still looks like weights.
    #[test]
    fn the_streaming_reader_matches_decrypt_chunked_exactly() {
        let (crypto, key) = crypto_and_key();

        // Deliberately not a multiple of the chunk size, so the final short
        // chunk is exercised.
        let data: Vec<u8> = (0..(64 * 1024 + 777)).map(|i| (i % 251) as u8).collect();
        let encrypted = encrypt_chunked(&crypto, &data, &key, 4096).unwrap();

        let buffered = decrypt_chunked(&crypto, &encrypted, &key).unwrap();

        let mut streamed = Vec::new();
        ChunkDecryptReader::new(&crypto, &key, std::io::Cursor::new(&encrypted))
            .unwrap()
            .read_to_end(&mut streamed)
            .unwrap();

        assert_eq!(streamed, buffered);
        assert_eq!(streamed, data);
    }

    #[test]
    fn the_streaming_reader_reports_the_plaintext_length_before_reading() {
        let (crypto, key) = crypto_and_key();
        let data = vec![9u8; 5000];
        let encrypted = encrypt_chunked(&crypto, &data, &key, 1024).unwrap();

        let reader = ChunkDecryptReader::new(&crypto, &key, std::io::Cursor::new(&encrypted));
        assert_eq!(reader.unwrap().plaintext_len(), 5000);
    }

    /// Truncation is only detectable at the stream MAC, which is only reached by
    /// reading to EOF. This pins that it IS reached.
    #[test]
    fn a_truncated_stream_fails_at_the_end_of_the_read() {
        let (crypto, key) = crypto_and_key();
        let data = vec![0xCCu8; 8192];
        let mut encrypted = encrypt_chunked(&crypto, &data, &key, 1024).unwrap();

        // Drop the final chunk, keeping a well-formed-looking MAC on the end.
        let mac = encrypted[encrypted.len() - 32..].to_vec();
        encrypted.truncate(encrypted.len() - 32 - (1024 + NONCE_SIZE + 16));
        encrypted.extend_from_slice(&mac);

        let mut out = Vec::new();
        let err = ChunkDecryptReader::new(&crypto, &key, std::io::Cursor::new(&encrypted))
            .unwrap()
            .read_to_end(&mut out)
            .expect_err("a truncated stream must not read cleanly");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    /// Tampering inside a chunk fails that chunk's GCM tag, which happens
    /// *before* those bytes are handed out — not at the end like the stream MAC.
    #[test]
    fn a_tampered_chunk_fails_before_its_bytes_are_emitted() {
        let (crypto, key) = crypto_and_key();
        let data = vec![0x5Au8; 8192];
        let mut encrypted = encrypt_chunked(&crypto, &data, &key, 1024).unwrap();

        // Corrupt a byte inside the FIRST chunk's ciphertext.
        encrypted[HEADER_SIZE + NONCE_SIZE + 4] ^= 0xFF;

        let mut reader =
            ChunkDecryptReader::new(&crypto, &key, std::io::Cursor::new(&encrypted)).unwrap();
        let mut buf = [0u8; 64];
        let err = reader
            .read(&mut buf)
            .expect_err("a tampered chunk must fail on the read that would emit it");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn an_empty_model_streams_and_still_checks_its_mac() {
        let (crypto, key) = crypto_and_key();
        let encrypted = encrypt_chunked(&crypto, b"", &key, 1024).unwrap();

        let mut out = Vec::new();
        ChunkDecryptReader::new(&crypto, &key, std::io::Cursor::new(&encrypted))
            .unwrap()
            .read_to_end(&mut out)
            .unwrap();
        assert!(out.is_empty());
    }

    /// A tiny destination buffer must not change the result: the reader has to
    /// serve a chunk across many `read` calls.
    #[test]
    fn reading_in_small_pieces_yields_the_same_bytes() {
        let (crypto, key) = crypto_and_key();
        let data: Vec<u8> = (0..9000).map(|i| (i % 97) as u8).collect();
        let encrypted = encrypt_chunked(&crypto, &data, &key, 2048).unwrap();

        let mut reader =
            ChunkDecryptReader::new(&crypto, &key, std::io::Cursor::new(&encrypted)).unwrap();
        let mut out = Vec::new();
        let mut buf = [0u8; 7]; // pathologically small, and not a chunk divisor
        loop {
            let n = reader.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n]);
        }
        assert_eq!(out, data);
    }

    #[test]
    fn test_decrypt_chunked_too_short() {
        // Covers L166-167 — encrypted data too short for chunked format
        let crypto = VaultCrypto::new().unwrap();
        let (key, _) = crypto.derive_key(b"short_test".to_vec(), None).unwrap();
        let short = vec![0u8; 10]; // way too short
        let result = decrypt_chunked(&crypto, &short, &key);
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(err_msg.contains("too short"));
    }
}
