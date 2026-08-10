#![no_main]
use libfuzzer_sys::fuzz_target;

use ironvault::crypto::FipsCrypto;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let crypto = match FipsCrypto::new() {
        Ok(c) => c,
        Err(_) => return,
    };

    // Use a fixed passphrase for deterministic key derivation
    let passphrase = b"fuzz-test-passphrase".to_vec();
    let (key, _salt) = match crypto.derive_key(passphrase, None) {
        Ok(k) => k,
        Err(_) => return,
    };

    // Encrypt the fuzzed data
    let encrypted = match crypto.encrypt(data, &key) {
        Ok(e) => e,
        Err(_) => return,
    };

    // Decrypt must roundtrip exactly
    let decrypted = crypto
        .decrypt(&encrypted, &key)
        .expect("decrypt must succeed on valid ciphertext");

    assert_eq!(
        data,
        &decrypted[..],
        "roundtrip mismatch: plaintext changed after encrypt/decrypt"
    );
});
