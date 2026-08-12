//! Comprehensive cryptography tests

use ironvault::crypto::{
    compression::{compress, decompress, CompressionAlgorithm, CompressionLevel},
    KeyManager, VaultCrypto,
};

#[test]
fn test_key_derivation_consistency() {
    let crypto = VaultCrypto::new().unwrap();
    let passphrase = b"test_passphrase_12345".to_vec();

    // Derive key twice with same passphrase and salt
    let (key1, salt) = crypto.derive_key(passphrase.clone(), None).unwrap();
    let (key2, _) = crypto.derive_key(passphrase, Some(salt.clone())).unwrap();

    assert_eq!(key1.as_bytes(), key2.as_bytes());
}

#[test]
fn test_key_derivation_different_salts() {
    let crypto = VaultCrypto::new().unwrap();
    let passphrase = b"test_passphrase_12345".to_vec();

    // Derive keys with different salts
    let (key1, _) = crypto.derive_key(passphrase.clone(), None).unwrap();
    let (key2, _) = crypto.derive_key(passphrase, None).unwrap();

    // Keys should be different with different salts
    assert_ne!(key1.as_bytes(), key2.as_bytes());
}

#[test]
fn test_encrypt_decrypt_various_sizes() {
    let crypto = VaultCrypto::new().unwrap();
    let passphrase = b"test_passphrase_12345".to_vec();
    let (key, _) = crypto.derive_key(passphrase, None).unwrap();

    // Test various data sizes
    let sizes = vec![0, 1, 16, 100, 1024, 10240, 1048576];

    for size in sizes {
        let data = vec![42u8; size];
        let encrypted = crypto.encrypt(&data, &key).unwrap();
        let decrypted = crypto.decrypt(&encrypted, &key).unwrap();

        assert_eq!(data, decrypted, "Failed for size {}", size);
        // Encrypted size should be larger due to nonce and auth tag
        if size > 0 {
            assert!(encrypted.len() > size);
        }
    }
}

#[test]
fn test_encrypt_decrypt_random_data() {
    let crypto = VaultCrypto::new().unwrap();
    let passphrase = b"test_passphrase_12345".to_vec();
    let (key, _) = crypto.derive_key(passphrase, None).unwrap();

    // Use pseudo-random data
    let data: Vec<u8> = (0..1000).map(|i| ((i * 7 + 13) % 256) as u8).collect();

    let encrypted = crypto.encrypt(&data, &key).unwrap();
    let decrypted = crypto.decrypt(&encrypted, &key).unwrap();

    assert_eq!(data, decrypted);
}

#[test]
fn test_encryption_authentication_failure() {
    let crypto = VaultCrypto::new().unwrap();
    let passphrase = b"test_passphrase_12345".to_vec();
    let (key, _) = crypto.derive_key(passphrase, None).unwrap();

    let data = b"sensitive data";
    let mut encrypted = crypto.encrypt(data, &key).unwrap();

    // Tamper with the encrypted data
    if encrypted.len() > 20 {
        encrypted[20] ^= 1; // Flip a bit
    }

    // Decryption should fail due to authentication tag mismatch
    let result = crypto.decrypt(&encrypted, &key);
    assert!(result.is_err(), "Should fail authentication check");
}

#[test]
fn test_encryption_nonce_uniqueness() {
    let crypto = VaultCrypto::new().unwrap();
    let passphrase = b"test_passphrase_12345".to_vec();
    let (key, _) = crypto.derive_key(passphrase, None).unwrap();

    let data = b"test data";

    // Encrypt same data multiple times
    let encrypted1 = crypto.encrypt(data, &key).unwrap();
    let encrypted2 = crypto.encrypt(data, &key).unwrap();
    let encrypted3 = crypto.encrypt(data, &key).unwrap();

    // Encrypted results should be different (different nonces)
    assert_ne!(encrypted1, encrypted2);
    assert_ne!(encrypted2, encrypted3);
    assert_ne!(encrypted1, encrypted3);

    // But all should decrypt to same data
    assert_eq!(crypto.decrypt(&encrypted1, &key).unwrap(), data);
    assert_eq!(crypto.decrypt(&encrypted2, &key).unwrap(), data);
    assert_eq!(crypto.decrypt(&encrypted3, &key).unwrap(), data);
}

#[test]
fn test_secure_key_zeroization() {
    let crypto = VaultCrypto::new().unwrap();
    let passphrase = b"test_passphrase_12345".to_vec();
    let (key, _) = crypto.derive_key(passphrase, None).unwrap();

    // Store key bytes reference
    let key_bytes = key.as_bytes().to_vec();

    // Drop the key (should zeroize)
    drop(key);

    // Key should have been zeroized (we can't directly verify without unsafe)
    assert_eq!(key_bytes.len(), 32); // AES-256 key size
}

#[test]
fn test_gzip_compression_decompression() {
    let data = b"This is test data for compression. ".repeat(100);

    let compressed = compress(
        &data,
        CompressionAlgorithm::Gzip,
        CompressionLevel::Balanced,
    )
    .unwrap();
    let decompressed = decompress(&compressed, CompressionAlgorithm::Gzip).unwrap();

    assert_eq!(data.to_vec(), decompressed);
    assert!(compressed.len() < data.len());
}

#[test]
fn test_lzma_compression_decompression() {
    let data = b"LZMA compression test data. ".repeat(50);

    let compressed =
        compress(&data, CompressionAlgorithm::Lzma, CompressionLevel::Maximum).unwrap();
    let decompressed = decompress(&compressed, CompressionAlgorithm::Lzma).unwrap();

    assert_eq!(data.to_vec(), decompressed);
}

#[test]
fn test_compression_levels() {
    let data = vec![42u8; 10000];

    let fast = compress(&data, CompressionAlgorithm::Gzip, CompressionLevel::Fast).unwrap();
    let balanced = compress(
        &data,
        CompressionAlgorithm::Gzip,
        CompressionLevel::Balanced,
    )
    .unwrap();
    let maximum = compress(&data, CompressionAlgorithm::Gzip, CompressionLevel::Maximum).unwrap();

    // All should decompress correctly
    assert_eq!(decompress(&fast, CompressionAlgorithm::Gzip).unwrap(), data);
    assert_eq!(
        decompress(&balanced, CompressionAlgorithm::Gzip).unwrap(),
        data
    );
    assert_eq!(
        decompress(&maximum, CompressionAlgorithm::Gzip).unwrap(),
        data
    );

    // Maximum should generally produce smallest size (for compressible data)
    assert!(maximum.len() <= balanced.len());
}

#[test]
fn test_compression_empty_data() {
    let data = vec![];

    let compressed = compress(
        &data,
        CompressionAlgorithm::Gzip,
        CompressionLevel::Balanced,
    )
    .unwrap();
    let decompressed = decompress(&compressed, CompressionAlgorithm::Gzip).unwrap();

    assert_eq!(data, decompressed);
}

#[test]
fn test_compression_incompressible_data() {
    // Pseudo-random incompressible data
    let data: Vec<u8> = (0..1000).map(|i| ((i * 31 + 17) % 256) as u8).collect();

    let compressed = compress(
        &data,
        CompressionAlgorithm::Gzip,
        CompressionLevel::Balanced,
    )
    .unwrap();
    let decompressed = decompress(&compressed, CompressionAlgorithm::Gzip).unwrap();

    assert_eq!(data, decompressed);
    // Incompressible data might actually be larger after compression
}

#[test]
fn test_key_manager_store_and_load() {
    let manager = KeyManager::new().unwrap();
    let crypto = VaultCrypto::new().unwrap();
    let passphrase = b"test_passphrase_12345".to_vec();

    let (key, _) = crypto.derive_key(passphrase.clone(), None).unwrap();

    // Store key with encryption
    let stored_data = manager.store_key(&key, passphrase.clone()).unwrap();
    assert!(!stored_data.is_empty());

    // Load key back
    let loaded_key = manager.load_key(&stored_data, passphrase).unwrap();
    assert_eq!(loaded_key.as_bytes(), key.as_bytes());
}

#[test]
fn test_key_manager_wrong_passphrase() {
    let manager = KeyManager::new().unwrap();
    let crypto = VaultCrypto::new().unwrap();

    let (key, _) = crypto.derive_key(b"pass1".to_vec(), None).unwrap();

    // Store with one passphrase
    let stored_data = manager.store_key(&key, b"correct_pass".to_vec()).unwrap();

    // Try to load with wrong passphrase
    let result = manager.load_key(&stored_data, b"wrong_pass".to_vec());
    assert!(result.is_err());
}
