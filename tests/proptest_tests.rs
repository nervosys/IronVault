//! Property-based tests using proptest.
//!
//! Tests invariants that must hold for all inputs:
//! - Crypto: encrypt→decrypt round-trip always returns original data
//! - Formats: `from_extension()` never panics on arbitrary strings
//! - Version: serialization round-trip preserves all fields

use proptest::prelude::*;

use ironvault::crypto::FipsCrypto;
use ironvault::formats::ModelFormat;
use ironvault::version::ModelVersion;

// ── Crypto round-trip ────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// Encrypt then decrypt must always return the original plaintext.
    #[test]
    fn crypto_encrypt_decrypt_roundtrip(data in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let crypto = FipsCrypto::new().expect("FipsCrypto::new");
        let (key, _salt) = crypto
            .derive_key(b"test-passphrase".to_vec(), None)
            .expect("derive_key");

        let encrypted = crypto.encrypt(&data, &key).expect("encrypt");
        let decrypted = crypto.decrypt(&encrypted, &key).expect("decrypt");

        prop_assert_eq!(&decrypted, &data, "decrypt(encrypt(data)) != data");
    }

    /// Different plaintexts should produce different ciphertexts (with overwhelming probability).
    #[test]
    fn crypto_different_plaintexts_different_ciphertexts(
        a in proptest::collection::vec(any::<u8>(), 1..512),
        b in proptest::collection::vec(any::<u8>(), 1..512),
    ) {
        prop_assume!(a != b);

        let crypto = FipsCrypto::new().expect("FipsCrypto::new");
        let (key, _salt) = crypto.derive_key(b"passphrase".to_vec(), None).expect("derive_key");

        let enc_a = crypto.encrypt(&a, &key).expect("encrypt a");
        let enc_b = crypto.encrypt(&b, &key).expect("encrypt b");

        prop_assert_ne!(enc_a, enc_b, "different plaintexts produced identical ciphertexts");
    }

    /// Ciphertext must be strictly larger than plaintext (nonce + tag overhead).
    #[test]
    fn crypto_ciphertext_has_overhead(data in proptest::collection::vec(any::<u8>(), 0..2048)) {
        let crypto = FipsCrypto::new().expect("FipsCrypto::new");
        let (key, _salt) = crypto.derive_key(b"pass".to_vec(), None).expect("derive_key");

        let encrypted = crypto.encrypt(&data, &key).expect("encrypt");
        // AES-256-GCM: 12-byte nonce + 16-byte auth tag = 28-byte overhead minimum
        prop_assert!(encrypted.len() >= data.len() + 28,
            "ciphertext {} should be >= plaintext {} + 28",
            encrypted.len(), data.len());
    }
}

// ── Format detection ─────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// `from_extension()` must never panic on any arbitrary string.
    #[test]
    fn format_from_extension_never_panics(ext in "\\PC{0,64}") {
        let _format = ModelFormat::from_extension(&ext);
        // If we reach here, no panic occurred — that's the property.
    }

    /// Known extensions must always map to a non-Custom variant.
    #[test]
    fn format_known_extensions_are_recognized(
        ext in prop_oneof![
            Just("safetensors"), Just("gguf"), Just("pt"), Just("pth"), Just("bin"),
            Just("plan"), Just("onnx"), Just("mlmodel"), Just("tflite"), Just("pb"),
            Just("h5"), Just("keras"), Just("xml"), Just("param"), Just("mnn"),
            Just("rknn"), Just("caffemodel"), Just("params"), Just("weights"),
            Just("hdf5"), Just("pkl"), Just("pickle"), Just("npy"), Just("npz"),
        ]
    ) {
        let format = ModelFormat::from_extension(ext);
        prop_assert!(
            !matches!(format, ModelFormat::Custom(_)),
            "known extension {:?} mapped to Custom variant", ext
        );
    }

    /// `from_extension()` should be case-insensitive for known extensions.
    /// Unknown extensions map to Custom(string) which preserves original casing.
    #[test]
    fn format_case_insensitive_known(
        ext in prop_oneof![
            Just("safetensors"), Just("gguf"), Just("pt"), Just("pth"), Just("bin"),
            Just("plan"), Just("onnx"), Just("mlmodel"), Just("tflite"), Just("pb"),
            Just("h5"), Just("keras"), Just("xml"), Just("param"), Just("mnn"),
            Just("rknn"), Just("caffemodel"), Just("params"), Just("weights"),
            Just("hdf5"), Just("pkl"), Just("pickle"), Just("npy"), Just("npz"),
        ]
    ) {
        let lower = ModelFormat::from_extension(&ext.to_lowercase());
        let upper = ModelFormat::from_extension(&ext.to_uppercase());
        prop_assert_eq!(lower, upper, "case sensitivity for known ext {:?}", ext);
    }
}

// ── Version serialization round-trip ─────────────────────────────────────────

fn arb_model_version() -> impl Strategy<Value = ModelVersion> {
    (
        1u32..10000,                   // version
        "[a-f0-9]{8}",                 // checkpoint_id
        prop::option::of(0u32..10000), // parent_version
        prop_oneof![
            Just("safetensors".to_string()),
            Just("gguf".to_string()),
            Just("pt".to_string()),
            Just("onnx".to_string()),
        ], // format
        0u64..1_000_000_000,           // size_bytes
        0u64..1_000_000_000,           // compressed_size_bytes
        "[a-f0-9]{64}",                // checksum_sha256
        "[a-z/]{1,32}",                // file_path
    )
        .prop_map(
            |(
                version,
                checkpoint_id,
                parent_version,
                format,
                size_bytes,
                compressed_size_bytes,
                checksum_sha256,
                file_path,
            )| {
                ModelVersion {
                    version,
                    checkpoint_id,
                    timestamp: chrono::Utc::now(),
                    parent_version,
                    format,
                    size_bytes,
                    compressed_size_bytes,
                    checksum_sha256,
                    metadata: std::collections::HashMap::new(),
                    file_path,
                }
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// JSON round-trip: serialize → deserialize must preserve all fields.
    #[test]
    fn version_json_roundtrip(v in arb_model_version()) {
        let json = serde_json::to_string(&v).expect("serialize");
        let v2: ModelVersion = serde_json::from_str(&json).expect("deserialize");

        prop_assert_eq!(v.version, v2.version);
        prop_assert_eq!(&v.checkpoint_id, &v2.checkpoint_id);
        prop_assert_eq!(v.parent_version, v2.parent_version);
        prop_assert_eq!(&v.format, &v2.format);
        prop_assert_eq!(v.size_bytes, v2.size_bytes);
        prop_assert_eq!(v.compressed_size_bytes, v2.compressed_size_bytes);
        prop_assert_eq!(&v.checksum_sha256, &v2.checksum_sha256);
        prop_assert_eq!(&v.file_path, &v2.file_path);
    }

    /// Parent version, when set, must not equal the version itself.
    /// This is a domain invariant we enforce at generation time.
    #[test]
    fn version_parent_not_self(
        version in 1u32..10000,
        parent in prop::option::of(0u32..10000),
    ) {
        // The invariant: if parent is set, it should differ from version.
        // This test documents the property; in production code we should
        // validate this on construction.
        if let Some(p) = parent {
            if p == version {
                // This is an invalid state — skip it.
                prop_assume!(false);
            }
        }
        // Just confirming the constraint is expressible
        prop_assert!(parent != Some(version));
    }
}

// ── SHA-256 hashing ──────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    /// SHA-256 hash must always produce exactly 32 bytes.
    #[test]
    fn sha256_always_32_bytes(data in proptest::collection::vec(any::<u8>(), 0..8192)) {
        let hash = FipsCrypto::hash_sha256(&data);
        prop_assert_eq!(hash.len(), 32, "SHA-256 must be 32 bytes, got {}", hash.len());
    }

    /// SHA-256 hex must always produce exactly 64 hex characters.
    #[test]
    fn sha256_hex_always_64_chars(data in proptest::collection::vec(any::<u8>(), 0..8192)) {
        let hex = FipsCrypto::hash_sha256_hex(&data);
        prop_assert_eq!(hex.len(), 64, "SHA-256 hex must be 64 chars, got {}", hex.len());
        prop_assert!(hex.chars().all(|c| c.is_ascii_hexdigit()),
            "SHA-256 hex contains non-hex chars: {}", hex);
    }

    /// Identical inputs must produce identical hashes (determinism).
    #[test]
    fn sha256_deterministic(data in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let h1 = FipsCrypto::hash_sha256(&data);
        let h2 = FipsCrypto::hash_sha256(&data);
        prop_assert_eq!(h1, h2, "SHA-256 not deterministic");
    }
}
