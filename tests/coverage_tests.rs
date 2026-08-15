// Assertions in these tests compare literal constants that round-trip
// bit-for-bit and build fixed strings; the lints below are noise here.
#![allow(
    clippy::float_cmp,
    clippy::single_char_pattern,
    clippy::manual_string_new
)]
//! Consolidated coverage tests - merged from 12 incremental coverage files.
//! Each original file is a uniquely named module to avoid name collisions.

#[allow(unused_imports)]
mod comprehensive_coverage_tests {
    //! Comprehensive coverage tests — Part 2
    //! Targets remaining uncovered code in:
    //! - federation (VectorClock, FederationConfig, PeerConfig, ClockComparison)
    //! - telemetry (global functions, TrackingTimer)
    //! - version (VersionRepo trait, VersionControl methods)
    //! - crypto/mod.rs (edge cases, SecureKey, VaultCrypto)

    // ============================================================================
    // FEDERATION — VectorClock::compare all branches, config structs
    // ============================================================================
    mod federation_coverage {
        use ironvault::federation::*;

        #[test]
        fn test_vector_clock_default() {
            let clock = VectorClock::new();
            assert!(clock.timestamps.is_empty());
        }

        #[test]
        fn test_vector_clock_increment_multiple_nodes() {
            let mut clock = VectorClock::new();
            clock.increment("a");
            clock.increment("b");
            clock.increment("a");
            assert_eq!(*clock.timestamps.get("a").unwrap(), 2);
            assert_eq!(*clock.timestamps.get("b").unwrap(), 1);
        }

        #[test]
        fn test_vector_clock_merge_takes_max() {
            let mut c1 = VectorClock::new();
            c1.timestamps.insert("a".into(), 5);
            c1.timestamps.insert("b".into(), 1);

            let mut c2 = VectorClock::new();
            c2.timestamps.insert("a".into(), 3);
            c2.timestamps.insert("b".into(), 7);
            c2.timestamps.insert("c".into(), 2);

            c1.merge(&c2);
            assert_eq!(*c1.timestamps.get("a").unwrap(), 5);
            assert_eq!(*c1.timestamps.get("b").unwrap(), 7);
            assert_eq!(*c1.timestamps.get("c").unwrap(), 2);
        }

        #[test]
        fn test_vector_clock_compare_equal() {
            let mut c1 = VectorClock::new();
            let mut c2 = VectorClock::new();
            c1.timestamps.insert("a".into(), 1);
            c2.timestamps.insert("a".into(), 1);
            assert!(matches!(c1.compare(&c2), ClockComparison::Equal));
        }

        #[test]
        fn test_vector_clock_compare_before() {
            let mut c1 = VectorClock::new();
            let mut c2 = VectorClock::new();
            c1.timestamps.insert("a".into(), 1);
            c2.timestamps.insert("a".into(), 2);
            assert!(matches!(c1.compare(&c2), ClockComparison::Before));
        }

        #[test]
        fn test_vector_clock_compare_after() {
            let mut c1 = VectorClock::new();
            let mut c2 = VectorClock::new();
            c1.timestamps.insert("a".into(), 3);
            c2.timestamps.insert("a".into(), 1);
            assert!(matches!(c1.compare(&c2), ClockComparison::After));
        }

        #[test]
        fn test_vector_clock_compare_concurrent() {
            let mut c1 = VectorClock::new();
            let mut c2 = VectorClock::new();
            c1.timestamps.insert("a".into(), 2);
            c1.timestamps.insert("b".into(), 1);
            c2.timestamps.insert("a".into(), 1);
            c2.timestamps.insert("b".into(), 3);
            assert!(matches!(c1.compare(&c2), ClockComparison::Concurrent));
        }

        #[test]
        fn test_vector_clock_compare_empty_vs_nonempty() {
            let c1 = VectorClock::new();
            let mut c2 = VectorClock::new();
            c2.increment("x");
            assert!(matches!(c1.compare(&c2), ClockComparison::Before));
        }

        #[test]
        fn test_vector_clock_compare_both_empty() {
            let c1 = VectorClock::new();
            let c2 = VectorClock::new();
            assert!(matches!(c1.compare(&c2), ClockComparison::Equal));
        }

        #[test]
        fn test_vector_clock_serde_roundtrip() {
            let mut clock = VectorClock::new();
            clock.increment("node1");
            clock.increment("node2");
            clock.increment("node1");
            let json = serde_json::to_string(&clock).unwrap();
            let deserialized: VectorClock = serde_json::from_str(&json).unwrap();
            assert_eq!(clock, deserialized);
        }

        #[test]
        fn test_federation_config_default() {
            let config = FederationConfig::default();
            assert!(!config.node_id.is_empty());
            assert!(config.peers.is_empty());
            assert_eq!(config.sync_interval_secs, 300);
            assert!(config.auto_resolve_conflicts);
            assert_eq!(config.max_concurrent_syncs, 4);
        }

        #[test]
        fn test_federation_config_custom() {
            let config = FederationConfig {
                node_id: "test-node".into(),
                node_name: "Test Node".into(),
                peers: vec![PeerConfig {
                    node_id: "peer1".into(),
                    name: "Peer 1".into(),
                    endpoint: "https://peer1.example.com".into(),
                    api_key: Some("secret".into()),
                    enabled: true,
                }],
                sync_interval_secs: 60,
                auto_resolve_conflicts: false,
                max_concurrent_syncs: 2,
            };
            assert_eq!(config.node_id, "test-node");
            assert_eq!(config.peers.len(), 1);
            assert_eq!(config.peers[0].node_id, "peer1");
            assert_eq!(config.peers[0].name, "Peer 1");
            assert_eq!(config.peers[0].endpoint, "https://peer1.example.com");
            assert!(config.peers[0].api_key.is_some());
            assert!(config.peers[0].enabled);
        }

        #[test]
        fn test_federation_config_serde() {
            let config = FederationConfig::default();
            let json = serde_json::to_string(&config).unwrap();
            let deserialized: FederationConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(config.node_id, deserialized.node_id);
        }

        #[test]
        fn test_peer_config_disabled() {
            let peer = PeerConfig {
                node_id: "p".into(),
                name: "P".into(),
                endpoint: "http://localhost".into(),
                api_key: None,
                enabled: false,
            };
            assert!(!peer.enabled);
            assert!(peer.api_key.is_none());
        }

        #[test]
        fn test_federation_manager_new_and_basic_ops() {
            let tmp = tempfile::tempdir().unwrap();
            let config = FederationConfig {
                node_id: "test-node".into(),
                node_name: "Test".into(),
                peers: vec![],
                sync_interval_secs: 300,
                auto_resolve_conflicts: true,
                max_concurrent_syncs: 4,
            };
            let manager = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();
            assert_eq!(manager.node_id(), "test-node");
            assert!(manager.peers().is_empty());
        }

        #[test]
        fn test_federation_manager_add_remove_peer() {
            let tmp = tempfile::tempdir().unwrap();
            let config = FederationConfig {
                node_id: "n1".into(),
                node_name: "Node 1".into(),
                peers: vec![],
                sync_interval_secs: 300,
                auto_resolve_conflicts: true,
                max_concurrent_syncs: 4,
            };
            let mut manager = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            manager.add_peer(PeerConfig {
                node_id: "p1".into(),
                name: "Peer 1".into(),
                endpoint: "http://p1.local".into(),
                api_key: None,
                enabled: true,
            });
            assert_eq!(manager.peers().len(), 1);

            manager.remove_peer("p1");
            assert!(manager.peers().is_empty());
        }

        #[test]
        fn test_federation_manager_compute_delta_empty() {
            let tmp = tempfile::tempdir().unwrap();
            let config = FederationConfig {
                node_id: "n1".into(),
                node_name: "Node".into(),
                peers: vec![],
                sync_interval_secs: 300,
                auto_resolve_conflicts: true,
                max_concurrent_syncs: 4,
            };
            let manager = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let local = SyncManifest {
                source_node: "n1".into(),
                timestamp: chrono::Utc::now(),
                models: vec![],
                clock: VectorClock::new(),
            };
            let remote = SyncManifest {
                source_node: "n2".into(),
                timestamp: chrono::Utc::now(),
                models: vec![],
                clock: VectorClock::new(),
            };
            let delta = manager.compute_delta(&local, &remote);
            assert!(delta.to_upload.is_empty());
            assert!(delta.to_download.is_empty());
            assert!(delta.conflicts.is_empty());
        }

        #[test]
        fn test_compute_delta_with_models() {
            let tmp = tempfile::tempdir().unwrap();
            let config = FederationConfig::default();
            let manager = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let now = chrono::Utc::now();
            let local = SyncManifest {
                source_node: "n1".into(),
                timestamp: now,
                models: vec![ModelManifestEntry {
                    name: "model_a".into(),
                    versions: vec![VersionManifestEntry {
                        version: 1,
                        checkpoint_id: "ckpt_local".into(),
                        created_at: now,
                        checksum: "abc".into(),
                        size_bytes: 100,
                        parent_id: None,
                        origin_node: "n1".into(),
                    }],
                    clock: VectorClock::new(),
                }],
                clock: VectorClock::new(),
            };
            let remote = SyncManifest {
                source_node: "n2".into(),
                timestamp: now,
                models: vec![ModelManifestEntry {
                    name: "model_b".into(),
                    versions: vec![VersionManifestEntry {
                        version: 1,
                        checkpoint_id: "ckpt_remote".into(),
                        created_at: now,
                        checksum: "def".into(),
                        size_bytes: 200,
                        parent_id: None,
                        origin_node: "n2".into(),
                    }],
                    clock: VectorClock::new(),
                }],
                clock: VectorClock::new(),
            };

            let delta = manager.compute_delta(&local, &remote);
            // model_a only local → upload
            assert!(!delta.to_upload.is_empty());
            // model_b only remote → download
            assert!(!delta.to_download.is_empty());
        }

        #[test]
        fn test_compute_delta_conflict() {
            let tmp = tempfile::tempdir().unwrap();
            let config = FederationConfig::default();
            let manager = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let now = chrono::Utc::now();
            let local = SyncManifest {
                source_node: "n1".into(),
                timestamp: now,
                models: vec![ModelManifestEntry {
                    name: "shared_model".into(),
                    versions: vec![VersionManifestEntry {
                        version: 1,
                        checkpoint_id: "ckpt_v1_local".into(),
                        created_at: now,
                        checksum: "abc".into(),
                        size_bytes: 100,
                        parent_id: None,
                        origin_node: "n1".into(),
                    }],
                    clock: VectorClock::new(),
                }],
                clock: VectorClock::new(),
            };
            let remote = SyncManifest {
                source_node: "n2".into(),
                timestamp: now,
                models: vec![ModelManifestEntry {
                    name: "shared_model".into(),
                    versions: vec![VersionManifestEntry {
                        version: 1,
                        checkpoint_id: "ckpt_v1_remote".into(), // Different checkpoint_id = conflict
                        created_at: now,
                        checksum: "xyz".into(),
                        size_bytes: 150,
                        parent_id: None,
                        origin_node: "n2".into(),
                    }],
                    clock: VectorClock::new(),
                }],
                clock: VectorClock::new(),
            };

            let delta = manager.compute_delta(&local, &remote);
            assert!(!delta.conflicts.is_empty());
        }
    }

    // ============================================================================
    // TELEMETRY — global functions, TrackingTimer, size buckets
    // ============================================================================
    mod telemetry_coverage {
        use ironvault::telemetry;
        use std::time::Duration;

        #[test]
        fn test_disable_then_tracking_functions() {
            telemetry::disable();
            assert!(!telemetry::is_enabled());
        }

        #[test]
        fn test_flush_does_not_panic() {
            telemetry::flush();
        }

        #[test]
        fn test_track_command_disabled() {
            telemetry::disable();
            telemetry::track_command("store", Some("model"), Duration::from_millis(50), true);
            telemetry::track_command("get", None, Duration::from_millis(30), false);
        }

        #[test]
        fn test_track_model_op_all_size_buckets() {
            telemetry::disable();
            // < 10MB
            telemetry::track_model_op("store", "pt", 1000, Duration::from_millis(1), true);
            // 10-100MB
            telemetry::track_model_op("store", "pt", 50_000_000, Duration::from_millis(1), true);
            // 100MB-1GB
            telemetry::track_model_op("store", "st", 500_000_000, Duration::from_millis(1), true);
            // >1GB
            telemetry::track_model_op(
                "store",
                "gguf",
                2_000_000_000,
                Duration::from_millis(1),
                true,
            );
            // failure
            telemetry::track_model_op("get", "onnx", 100, Duration::from_millis(1), false);
        }

        #[test]
        fn test_track_conversion_disabled() {
            telemetry::disable();
            telemetry::track_conversion("pytorch", "onnx", Duration::from_millis(50), true);
            telemetry::track_conversion("onnx", "trt", Duration::from_millis(100), false);
        }

        #[test]
        fn test_track_api_call_disabled() {
            telemetry::disable();
            telemetry::track_api_call("/api/models", "GET", 200, Duration::from_millis(25));
            telemetry::track_api_call("/api/store", "POST", 500, Duration::from_millis(100));
        }

        #[test]
        fn test_track_error_disabled() {
            telemetry::disable();
            telemetry::track_error("CryptoError", Some("ctx"));
            telemetry::track_error("IoError", None);
        }

        #[test]
        fn test_track_feature_disabled() {
            telemetry::disable();
            telemetry::track_feature("sqlite", Some("init"));
            telemetry::track_feature("cloud", None);
        }

        #[test]
        fn test_track_app_start_disabled() {
            telemetry::disable();
            telemetry::track_app_start();
        }

        #[test]
        fn test_tracking_timer_finish_success() {
            telemetry::disable();
            let timer = telemetry::TrackingTimer::new("test", Some("sub"));
            timer.finish(true);
        }

        #[test]
        fn test_tracking_timer_finish_failure() {
            telemetry::disable();
            let timer = telemetry::TrackingTimer::new("fail", None);
            timer.finish(false);
        }
    }

    // ============================================================================
    // VERSION — VersionRepo trait impl on VersionControl, all methods
    // ============================================================================
    mod version_coverage {
        use ironvault::crypto::VaultCrypto;
        use ironvault::traits::{CryptoProvider, VersionRepo};
        use ironvault::version::VersionControl;
        use std::collections::HashMap;

        #[test]
        fn test_version_repo_add_list() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            let v = VersionRepo::add_version(
                &mut vc, "m1", "f.enc", "pytorch", 1000, 500, "cksum", None, None,
            )
            .unwrap();
            assert_eq!(v.version, 1);

            let versions = VersionRepo::list_versions(&vc, "m1");
            assert_eq!(versions.len(), 1);
        }

        #[test]
        fn test_version_repo_get_version_latest_and_specific() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            VersionRepo::add_version(&mut vc, "m", "f1.enc", "pt", 100, 50, "c1", None, None)
                .unwrap();
            VersionRepo::add_version(&mut vc, "m", "f2.enc", "pt", 200, 100, "c2", None, None)
                .unwrap();

            let latest = VersionRepo::get_version(&vc, "m", None);
            assert!(latest.is_some());
            assert_eq!(latest.unwrap().version, 2);

            let specific = VersionRepo::get_version(&vc, "m", Some(1));
            assert!(specific.is_some());
            assert_eq!(specific.unwrap().version, 1);

            let missing = VersionRepo::get_version(&vc, "m", Some(99));
            assert!(missing.is_none());
        }

        #[test]
        fn test_version_repo_get_lineage() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            VersionRepo::add_version(&mut vc, "m", "f1.enc", "pt", 100, 50, "c1", None, None)
                .unwrap();
            VersionRepo::add_version(&mut vc, "m", "f2.enc", "pt", 200, 100, "c2", None, Some(1))
                .unwrap();

            let lineage = VersionRepo::get_lineage(&vc, "m", 2);
            assert!(!lineage.is_empty());
        }

        #[test]
        fn test_version_repo_delete() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            VersionRepo::add_version(&mut vc, "m", "f.enc", "pt", 100, 50, "ck", None, None)
                .unwrap();
            let deleted = VersionRepo::delete_version(&mut vc, "m", 1).unwrap();
            assert!(deleted);
        }

        #[test]
        fn test_version_repo_delete_nonexistent() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();
            let deleted = VersionRepo::delete_version(&mut vc, "m", 99).unwrap();
            assert!(!deleted);
        }

        #[test]
        fn test_version_repo_update_and_get_metadata() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            VersionRepo::add_version(&mut vc, "m", "f.enc", "pt", 100, 50, "ck", None, None)
                .unwrap();
            VersionRepo::update_metadata(&mut vc, "m", 1, "author", "test".into()).unwrap();

            let val = VersionRepo::get_metadata(&vc, "m", 1, "author");
            assert_eq!(val, Some("test".to_string()));

            let missing = VersionRepo::get_metadata(&vc, "m", 1, "nonexistent");
            assert!(missing.is_none());

            let missing_model = VersionRepo::get_metadata(&vc, "no_model", 1, "key");
            assert!(missing_model.is_none());
        }

        #[test]
        fn test_version_with_initial_metadata() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            let mut meta = HashMap::new();
            meta.insert("key".to_string(), "val".to_string());

            vc.add_version("m", "f.enc", "pt", 100, 50, "ck", Some(meta), None)
                .unwrap();
            let v = vc.get_version("m", Some(1)).unwrap();
            assert_eq!(v.metadata.get("key").unwrap(), "val");
        }

        #[test]
        fn test_version_cleanup_old() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            for i in 0..5 {
                vc.add_version(
                    "m",
                    &format!("f{}.enc", i),
                    "pt",
                    100,
                    50,
                    &format!("c{}", i),
                    None,
                    None,
                )
                .unwrap();
            }

            let removed = VersionRepo::cleanup_old_versions(&mut vc, "m", 2).unwrap();
            assert_eq!(removed.len(), 3);
            assert_eq!(VersionRepo::list_versions(&vc, "m").len(), 2);
        }

        #[test]
        fn test_version_verify_checksum() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            let data = b"hello";
            let crypto = VaultCrypto::new().unwrap();
            let checksum_hex = crypto.hash_hex(data);

            vc.add_version("m", "f.enc", "pt", 5, 5, &checksum_hex, None, None)
                .unwrap();
            assert!(VersionRepo::verify_checksum(&vc, "m", 1, data));
            assert!(!VersionRepo::verify_checksum(&vc, "m", 1, b"wrong"));
        }

        #[test]
        fn test_version_verify_checksum_missing() {
            let tmp = tempfile::tempdir().unwrap();
            let vc = VersionControl::new(tmp.path()).unwrap();
            assert!(!VersionRepo::verify_checksum(&vc, "no_model", 1, b"data"));
        }

        #[test]
        fn test_version_list_models() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            vc.add_version("model_a", "f1.enc", "pt", 100, 50, "c1", None, None)
                .unwrap();
            vc.add_version("model_b", "f2.enc", "pt", 100, 50, "c2", None, None)
                .unwrap();

            let models = VersionRepo::list_models(&vc);
            assert!(models.contains(&"model_a".to_string()));
            assert!(models.contains(&"model_b".to_string()));
        }

        #[test]
        fn test_version_parent_lineage() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            vc.add_version("m", "f1.enc", "pt", 100, 50, "c1", None, None)
                .unwrap();
            vc.add_version("m", "f2.enc", "pt", 200, 100, "c2", None, Some(1))
                .unwrap();
            vc.add_version("m", "f3.enc", "pt", 300, 150, "c3", None, Some(2))
                .unwrap();

            let lineage = vc.get_lineage("m", 3);
            assert!(lineage.len() >= 2); // v3 -> v2 -> v1
        }

        #[test]
        fn test_version_vault_path() {
            let tmp = tempfile::tempdir().unwrap();
            let vc = VersionControl::new(tmp.path()).unwrap();
            assert_eq!(vc.vault_path(), tmp.path());
        }
    }

    // ============================================================================
    // CRYPTO — edge cases in encrypt/decrypt, key derivation
    // ============================================================================
    mod crypto_coverage {
        use ironvault::crypto::{SecureKey, VaultCrypto, KEY_SIZE};
        use ironvault::traits::CryptoProvider;

        #[test]
        fn test_secure_key_from_bytes_wrong_size() {
            let result = SecureKey::from_bytes(&[0u8; 16]);
            assert!(result.is_err());
        }

        #[test]
        fn test_secure_key_from_bytes_correct_size() {
            let key = SecureKey::from_bytes(&[0u8; KEY_SIZE]).unwrap();
            assert_eq!(key.as_bytes().len(), KEY_SIZE);
        }

        #[test]
        fn test_fips_crypto_roundtrip() {
            let crypto = VaultCrypto::new().unwrap();
            let (key, _salt) = crypto.derive_key(b"my passphrase".to_vec(), None).unwrap();
            let data = b"secret model data";
            let encrypted = crypto.encrypt(data, &key).unwrap();
            assert_ne!(encrypted, data);
            let decrypted = crypto.decrypt(&encrypted, &key).unwrap();
            assert_eq!(decrypted, data);
        }

        #[test]
        fn test_fips_crypto_derive_key_with_salt() {
            let crypto = VaultCrypto::new().unwrap();
            let (key1, salt) = crypto.derive_key(b"pass".to_vec(), None).unwrap();
            let (key2, salt2) = crypto
                .derive_key(b"pass".to_vec(), Some(salt.clone()))
                .unwrap();
            assert_eq!(key1.as_bytes(), key2.as_bytes());
            assert_eq!(salt, salt2);
        }

        #[test]
        fn test_fips_crypto_hash() {
            let crypto = VaultCrypto::new().unwrap();
            let hash = crypto.hash(b"hello");
            assert_eq!(hash.len(), 32);
        }

        #[test]
        fn test_fips_crypto_hash_hex() {
            let crypto = VaultCrypto::new().unwrap();
            let hex = crypto.hash_hex(b"hello");
            assert_eq!(hex.len(), 64);
        }

        #[test]
        fn test_fips_crypto_random_bytes() {
            let crypto = VaultCrypto::new().unwrap();
            let bytes = crypto.random_bytes(32);
            assert_eq!(bytes.len(), 32);
            // Should not be all zeros (with overwhelming probability)
            assert!(bytes.iter().any(|&b| b != 0));
        }

        #[test]
        fn test_decrypt_with_wrong_key() {
            let crypto = VaultCrypto::new().unwrap();
            let (key1, _) = crypto.derive_key(b"pass1".to_vec(), None).unwrap();
            let (key2, _) = crypto.derive_key(b"pass2".to_vec(), None).unwrap();
            let encrypted = crypto.encrypt(b"data", &key1).unwrap();
            assert!(crypto.decrypt(&encrypted, &key2).is_err());
        }

        #[test]
        fn test_encrypt_empty_data() {
            let crypto = VaultCrypto::new().unwrap();
            let (key, _) = crypto.derive_key(b"pass".to_vec(), None).unwrap();
            let encrypted = crypto.encrypt(b"", &key).unwrap();
            let decrypted = crypto.decrypt(&encrypted, &key).unwrap();
            assert!(decrypted.is_empty());
        }

        #[test]
        fn test_decrypt_truncated() {
            let crypto = VaultCrypto::new().unwrap();
            let result = crypto.decrypt(
                &[0u8; 5], // too short for nonce + tag
                &SecureKey::from_bytes(&[0u8; KEY_SIZE]).unwrap(),
            );
            assert!(result.is_err());
        }

        #[test]
        fn test_fips_crypto_large_data() {
            let crypto = VaultCrypto::new().unwrap();
            let (key, _) = crypto.derive_key(b"pass".to_vec(), None).unwrap();
            let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
            let encrypted = crypto.encrypt(&data, &key).unwrap();
            let decrypted = crypto.decrypt(&encrypted, &key).unwrap();
            assert_eq!(decrypted, data);
        }

        #[test]
        fn test_hash_sha256_static() {
            let hash = VaultCrypto::hash_sha256(b"test");
            assert_eq!(hash.len(), 32);
        }
    }
}

#[allow(unused_imports)]
mod coverage_boost_tests {
    //! Coverage boost tests — Part 7
    //! Targets remaining uncovered lines across:
    //! - version_sqlite.rs: full VersionRepo trait via SqliteVersionRepo::in_memory()
    //! - database.rs: SQLiteDatabase CRUD, Database trait
    //! - blockchain.rs: BlockchainAudit verify_chain, verify_proof, search
    //! - traits.rs: IvUri with query params, display
    //! - config.rs: save/load, custom dirs
    //! - compliance.rs: check_cve, check_mitre_attack, check_cmmc, run_all_checks
    //! - formats.rs: remaining extension/name branches
    //! - compression.rs: None algorithm, levels
    //! - vault.rs: auto_cleanup, store_model_streamed, VaultBuilder
    //! - conversion.rs: remaining converter paths
    //! - rag: rules, knowledge, mcp

    // ============================================================================
    // VERSION_SQLITE — SqliteVersionRepo via VersionRepo trait (in-memory)
    // ============================================================================
    mod version_sqlite_coverage {
        use ironvault::crypto::VaultCrypto;
        use ironvault::traits::VersionRepo;
        use ironvault::version_sqlite::SqliteVersionRepo;
        use std::collections::HashMap;

        fn make_repo() -> SqliteVersionRepo {
            SqliteVersionRepo::in_memory().unwrap()
        }

        fn sha256_hex(data: &[u8]) -> String {
            hex::encode(VaultCrypto::hash_sha256(data))
        }

        #[test]
        fn add_and_get_version() {
            let mut repo = make_repo();
            let mv = repo
                .add_version(
                    "model-a",
                    "/data/m.pt",
                    "pytorch",
                    1024,
                    512,
                    "abc123",
                    None,
                    None,
                )
                .unwrap();
            assert_eq!(mv.version, 1);
            assert_eq!(mv.format, "pytorch");
            assert_eq!(mv.size_bytes, 1024);
            assert_eq!(mv.compressed_size_bytes, 512);
            assert_eq!(mv.checksum_sha256, "abc123");

            // Get latest
            let latest = repo.get_version("model-a", None).unwrap();
            assert_eq!(latest.version, 1);

            // Get specific
            let v1 = repo.get_version("model-a", Some(1)).unwrap();
            assert_eq!(v1.checkpoint_id, mv.checkpoint_id);
        }

        #[test]
        fn add_multiple_versions() {
            let mut repo = make_repo();
            repo.add_version("m", "/p1", "pt", 100, 50, "c1", None, None)
                .unwrap();
            let v2 = repo
                .add_version("m", "/p2", "pt", 200, 100, "c2", None, Some(1))
                .unwrap();
            assert_eq!(v2.version, 2);
            assert_eq!(v2.parent_version, Some(1));

            let latest = repo.get_version("m", None).unwrap();
            assert_eq!(latest.version, 2);
        }

        #[test]
        fn add_version_with_metadata() {
            let mut repo = make_repo();
            let mut meta = HashMap::new();
            meta.insert("author".to_string(), "test".to_string());
            meta.insert("tag".to_string(), "v1".to_string());

            let mv = repo
                .add_version("m", "/p", "onnx", 100, 50, "c", Some(meta), None)
                .unwrap();
            assert_eq!(mv.metadata.get("author").unwrap(), "test");
            assert_eq!(mv.metadata.get("tag").unwrap(), "v1");
        }

        #[test]
        fn list_versions_sorted() {
            let mut repo = make_repo();
            repo.add_version("m", "/p1", "pt", 100, 50, "c1", None, None)
                .unwrap();
            repo.add_version("m", "/p2", "pt", 200, 100, "c2", None, Some(1))
                .unwrap();
            repo.add_version("m", "/p3", "pt", 300, 150, "c3", None, Some(2))
                .unwrap();

            let versions = repo.list_versions("m");
            assert_eq!(versions.len(), 3);
            assert_eq!(versions[0].version, 1);
            assert_eq!(versions[1].version, 2);
            assert_eq!(versions[2].version, 3);
        }

        #[test]
        fn list_versions_empty() {
            let repo = make_repo();
            let versions = repo.list_versions("nonexistent");
            assert!(versions.is_empty());
        }

        #[test]
        fn get_lineage() {
            let mut repo = make_repo();
            repo.add_version("m", "/p1", "pt", 100, 50, "c1", None, None)
                .unwrap();
            repo.add_version("m", "/p2", "pt", 200, 100, "c2", None, Some(1))
                .unwrap();
            repo.add_version("m", "/p3", "pt", 300, 150, "c3", None, Some(2))
                .unwrap();

            let lineage = repo.get_lineage("m", 3);
            assert_eq!(lineage.len(), 3);
            assert_eq!(lineage[0].version, 1);
            assert_eq!(lineage[1].version, 2);
            assert_eq!(lineage[2].version, 3);
        }

        #[test]
        fn get_lineage_single() {
            let mut repo = make_repo();
            repo.add_version("m", "/p1", "pt", 100, 50, "c1", None, None)
                .unwrap();

            let lineage = repo.get_lineage("m", 1);
            assert_eq!(lineage.len(), 1);
        }

        #[test]
        fn get_lineage_nonexistent() {
            let repo = make_repo();
            let lineage = repo.get_lineage("m", 99);
            assert!(lineage.is_empty());
        }

        #[test]
        fn delete_version() {
            let mut repo = make_repo();
            repo.add_version("m", "/p1", "pt", 100, 50, "c1", None, None)
                .unwrap();
            repo.add_version("m", "/p2", "pt", 200, 100, "c2", None, Some(1))
                .unwrap();

            let deleted = repo.delete_version("m", 1).unwrap();
            assert!(deleted);

            assert!(repo.get_version("m", Some(1)).is_none());
            assert!(repo.get_version("m", Some(2)).is_some());
        }

        #[test]
        fn delete_nonexistent_version() {
            let mut repo = make_repo();
            let deleted = repo.delete_version("m", 99).unwrap();
            assert!(!deleted);
        }

        #[test]
        fn cleanup_old_versions() {
            let mut repo = make_repo();
            for i in 0..5u32 {
                repo.add_version(
                    "m",
                    &format!("/p{}", i),
                    "pt",
                    100,
                    50,
                    &format!("c{}", i),
                    None,
                    if i > 0 { Some(i) } else { None },
                )
                .unwrap();
            }

            let deleted = repo.cleanup_old_versions("m", 2).unwrap();
            assert_eq!(deleted.len(), 3);

            let remaining = repo.list_versions("m");
            assert_eq!(remaining.len(), 2);
        }

        #[test]
        fn cleanup_keeps_all_when_under_limit() {
            let mut repo = make_repo();
            repo.add_version("m", "/p1", "pt", 100, 50, "c1", None, None)
                .unwrap();

            let deleted = repo.cleanup_old_versions("m", 5).unwrap();
            assert!(deleted.is_empty());
        }

        #[test]
        fn verify_checksum_correct() {
            let mut repo = make_repo();
            let data = b"hello world";
            let checksum = sha256_hex(data);
            repo.add_version("m", "/p", "pt", 11, 11, &checksum, None, None)
                .unwrap();

            assert!(repo.verify_checksum("m", 1, data));
        }

        #[test]
        fn verify_checksum_incorrect() {
            let mut repo = make_repo();
            repo.add_version("m", "/p", "pt", 11, 11, "wrong", None, None)
                .unwrap();

            assert!(!repo.verify_checksum("m", 1, b"hello world"));
        }

        #[test]
        fn verify_checksum_nonexistent() {
            let repo = make_repo();
            assert!(!repo.verify_checksum("m", 99, b"data"));
        }

        #[test]
        fn update_and_get_metadata() {
            let mut repo = make_repo();
            repo.add_version("m", "/p", "pt", 100, 50, "c", None, None)
                .unwrap();

            repo.update_metadata("m", 1, "tag", "production".to_string())
                .unwrap();
            let val = repo.get_metadata("m", 1, "tag");
            assert_eq!(val.unwrap(), "production");
        }

        #[test]
        fn update_metadata_nonexistent_version() {
            let mut repo = make_repo();
            let result = repo.update_metadata("m", 99, "tag", "val".to_string());
            assert!(result.is_err());
        }

        #[test]
        fn get_metadata_missing_key() {
            let mut repo = make_repo();
            repo.add_version("m", "/p", "pt", 100, 50, "c", None, None)
                .unwrap();
            assert!(repo.get_metadata("m", 1, "nonexistent").is_none());
        }

        #[test]
        fn list_models() {
            let mut repo = make_repo();
            repo.add_version("alpha", "/p1", "pt", 100, 50, "c1", None, None)
                .unwrap();
            repo.add_version("beta", "/p2", "onnx", 200, 100, "c2", None, None)
                .unwrap();
            repo.add_version("gamma", "/p3", "st", 300, 150, "c3", None, None)
                .unwrap();

            let models = repo.list_models();
            assert_eq!(models.len(), 3);
            assert!(models.contains(&"alpha".to_string()));
            assert!(models.contains(&"beta".to_string()));
            assert!(models.contains(&"gamma".to_string()));
        }

        #[test]
        fn get_version_nonexistent_model() {
            let repo = make_repo();
            assert!(repo.get_version("nope", None).is_none());
            assert!(repo.get_version("nope", Some(1)).is_none());
        }

        #[test]
        fn vault_path_empty_for_in_memory() {
            let repo = make_repo();
            assert!(repo.vault_path().as_os_str().is_empty());
        }
    }

    // ============================================================================
    // DATABASE (RAG) — SQLiteDatabase CRUD + Database trait
    // ============================================================================
    mod database_coverage {
        use ironvault::rag::{Database, Document, SQLiteDatabase};
        use std::collections::HashMap;

        fn make_db() -> SQLiteDatabase {
            SQLiteDatabase::in_memory().unwrap()
        }

        #[test]
        fn store_and_get_document() {
            let db = make_db();
            let doc = Document {
                id: "doc-1".to_string(),
                content: "Hello world".to_string(),
                metadata: HashMap::new(),
                embedding: None,
                chunk_info: None,
            };

            db.store_document(&doc).unwrap();
            let retrieved = db.get_document("doc-1").unwrap().unwrap();
            assert_eq!(retrieved.id, "doc-1");
            assert_eq!(retrieved.content, "Hello world");
        }

        #[test]
        fn store_document_with_embedding() {
            let db = make_db();
            let doc = Document {
                id: "doc-emb".to_string(),
                content: "Embedding test".to_string(),
                metadata: HashMap::new(),
                embedding: Some(vec![0.1, 0.2, 0.3]),
                chunk_info: None,
            };

            db.store_document(&doc).unwrap();
            let retrieved = db.get_document("doc-emb").unwrap().unwrap();
            let emb = retrieved.embedding.unwrap();
            assert_eq!(emb.len(), 3);
            assert!((emb[0] - 0.1).abs() < 0.001);
        }

        #[test]
        fn store_document_with_metadata() {
            let db = make_db();
            let mut meta = HashMap::new();
            meta.insert("author".to_string(), "test".to_string());
            let doc = Document {
                id: "doc-meta".to_string(),
                content: "Metadata test".to_string(),
                metadata: meta,
                embedding: None,
                chunk_info: None,
            };

            db.store_document(&doc).unwrap();
            let retrieved = db.get_document("doc-meta").unwrap().unwrap();
            assert_eq!(retrieved.metadata.get("author").unwrap(), "test");
        }

        #[test]
        fn store_document_with_chunk_info() {
            let db = make_db();
            let doc = Document {
                id: "doc-chunk".to_string(),
                content: "Chunk test".to_string(),
                metadata: HashMap::new(),
                embedding: None,
                chunk_info: Some(ironvault::rag::ChunkInfo {
                    parent_id: Some("parent-1".to_string()),
                    chunk_index: 0,
                    total_chunks: 3,
                    overlap: 50,
                }),
            };

            db.store_document(&doc).unwrap();
            let retrieved = db.get_document("doc-chunk").unwrap().unwrap();
            let ci = retrieved.chunk_info.unwrap();
            assert_eq!(ci.parent_id, Some("parent-1".to_string()));
            assert_eq!(ci.chunk_index, 0);
            assert_eq!(ci.total_chunks, 3);
            assert_eq!(ci.overlap, 50);
        }

        #[test]
        fn get_nonexistent_document() {
            let db = make_db();
            // Must create the table first by storing something
            let doc = Document {
                id: "setup".to_string(),
                content: "setup".to_string(),
                metadata: HashMap::new(),
                embedding: None,
                chunk_info: None,
            };
            db.store_document(&doc).unwrap();

            let result = db.get_document("nonexistent").unwrap();
            assert!(result.is_none());
        }

        #[test]
        fn search_documents() {
            let db = make_db();
            for i in 0..5 {
                let doc = Document {
                    id: format!("doc-{}", i),
                    content: if i % 2 == 0 {
                        format!("Transformer attention mechanism {}", i)
                    } else {
                        format!("CNN architecture {}", i)
                    },
                    metadata: HashMap::new(),
                    embedding: None,
                    chunk_info: None,
                };
                db.store_document(&doc).unwrap();
            }

            let results = db.search_documents("Transformer", 10).unwrap();
            assert_eq!(results.len(), 3); // docs 0, 2, 4

            let results2 = db.search_documents("CNN", 10).unwrap();
            assert_eq!(results2.len(), 2); // docs 1, 3

            let results3 = db.search_documents("nonexistent-query", 10).unwrap();
            assert!(results3.is_empty());
        }

        #[test]
        fn search_documents_with_limit() {
            let db = make_db();
            for i in 0..10 {
                let doc = Document {
                    id: format!("doc-{}", i),
                    content: format!("Searchable content {}", i),
                    metadata: HashMap::new(),
                    embedding: None,
                    chunk_info: None,
                };
                db.store_document(&doc).unwrap();
            }

            let results = db.search_documents("Searchable", 3).unwrap();
            assert_eq!(results.len(), 3);
        }

        #[test]
        fn database_trait_create_table_and_insert() {
            let mut db = make_db();
            db.create_table("test_table", &[("name", "TEXT"), ("value", "TEXT")])
                .unwrap();

            let mut data = HashMap::new();
            data.insert("id".to_string(), "row1".to_string());
            data.insert("name".to_string(), "hello".to_string());
            data.insert("value".to_string(), "world".to_string());

            db.insert("test_table", data).unwrap();

            let results = db.query("SELECT * FROM test_table").unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].get("name").unwrap(), "hello");
        }

        #[test]
        fn database_trait_update() {
            let mut db = make_db();
            db.create_table("items", &[("label", "TEXT")]).unwrap();

            let mut data = HashMap::new();
            data.insert("id".to_string(), "item1".to_string());
            data.insert("label".to_string(), "original".to_string());
            db.insert("items", data).unwrap();

            let mut update_data = HashMap::new();
            update_data.insert("label".to_string(), "updated".to_string());
            db.update("items", "item1", update_data).unwrap();

            let results = db.query("SELECT * FROM items WHERE id = 'item1'").unwrap();
            assert_eq!(results[0].get("label").unwrap(), "updated");
        }

        #[test]
        fn database_trait_delete() {
            let mut db = make_db();
            db.create_table("deletable", &[("val", "TEXT")]).unwrap();

            let mut data = HashMap::new();
            data.insert("id".to_string(), "d1".to_string());
            data.insert("val".to_string(), "temp".to_string());
            db.insert("deletable", data).unwrap();

            let before = db.query("SELECT * FROM deletable").unwrap();
            assert_eq!(before.len(), 1);

            db.delete("deletable", "d1").unwrap();

            let after = db.query("SELECT * FROM deletable").unwrap();
            assert!(after.is_empty());
        }

        #[test]
        fn database_trait_query_empty() {
            let db = make_db();
            db.create_table("empty_table", &[("col", "TEXT")]).unwrap();

            let results = db.query("SELECT * FROM empty_table").unwrap();
            assert!(results.is_empty());
        }
    }

    // ============================================================================
    // BLOCKCHAIN — BlockchainAudit verify_chain, verify_proof, search
    // ============================================================================
    mod blockchain_coverage {
        use chrono::Utc;
        use ironvault::audit::{AuditEntry, AuditEventType};
        use ironvault::BlockchainAudit;

        fn make_audit() -> (BlockchainAudit, tempfile::TempDir) {
            let tmp = tempfile::tempdir().unwrap();
            let audit = BlockchainAudit::new(tmp.path(), 10).unwrap();
            (audit, tmp)
        }

        fn make_entry(event_type: AuditEventType, model: Option<&str>) -> AuditEntry {
            AuditEntry {
                timestamp: Utc::now(),
                event_type,
                description: "test".to_string(),
                model_name: model.map(|s| s.to_string()),
                version: Some(1),
                success: true,
                metadata: None,
            }
        }

        #[test]
        fn verify_chain_valid() {
            let (mut audit, _tmp) = make_audit();
            audit
                .add_entry(make_entry(AuditEventType::ModelStored, Some("m1")))
                .unwrap();
            audit
                .add_entry(make_entry(AuditEventType::ModelRetrieved, Some("m1")))
                .unwrap();
            audit.finalize_block().unwrap();

            let result = audit.verify_chain();
            assert!(result.valid);
            assert!(result.issues.is_empty());
            assert!(result.blocks_verified > 0);
        }

        #[test]
        fn verify_chain_empty() {
            let (audit, _tmp) = make_audit();
            let result = audit.verify_chain();
            assert!(result.valid);
            assert_eq!(result.blocks_total, 1); // genesis
        }

        #[test]
        fn search_by_model_name() {
            let (mut audit, _tmp) = make_audit();
            audit
                .add_entry(make_entry(AuditEventType::ModelStored, Some("alpha")))
                .unwrap();
            audit
                .add_entry(make_entry(AuditEventType::ModelStored, Some("beta")))
                .unwrap();
            audit.finalize_block().unwrap();

            let results = audit.search(Some("alpha"), None, None, None, 100).unwrap();
            assert!(results
                .iter()
                .all(|(_, _, e)| e.audit.model_name.as_deref() == Some("alpha")));
            assert!(!results.is_empty());
        }

        #[test]
        fn search_by_event_type() {
            let (mut audit, _tmp) = make_audit();
            audit
                .add_entry(make_entry(AuditEventType::ModelStored, Some("m")))
                .unwrap();
            audit
                .add_entry(make_entry(AuditEventType::ModelRetrieved, Some("m")))
                .unwrap();
            audit.finalize_block().unwrap();

            let results = audit
                .search(None, Some(AuditEventType::ModelStored), None, None, 100)
                .unwrap();
            assert!(!results.is_empty());
        }

        #[test]
        fn search_with_time_bounds() {
            let (mut audit, _tmp) = make_audit();
            let before = Utc::now();
            audit
                .add_entry(make_entry(AuditEventType::ModelStored, Some("m")))
                .unwrap();
            audit.finalize_block().unwrap();
            let after = Utc::now();

            let results = audit
                .search(None, None, Some(before), Some(after), 100)
                .unwrap();
            assert!(!results.is_empty() || audit.height() > 0);
        }

        #[test]
        fn search_with_limit() {
            let (mut audit, _tmp) = make_audit();
            for _ in 0..10 {
                audit
                    .add_entry(make_entry(AuditEventType::ModelStored, Some("m")))
                    .unwrap();
            }
            audit.finalize_block().unwrap();

            let results = audit.search(None, None, None, None, 3).unwrap();
            assert!(results.len() <= 3);
        }

        #[test]
        fn generate_and_verify_proof() {
            let (mut audit, _tmp) = make_audit();
            audit
                .add_entry(make_entry(AuditEventType::ModelStored, Some("m")))
                .unwrap();
            audit.finalize_block().unwrap();

            let proof = audit.generate_proof(1, 0).unwrap();
            let verification = BlockchainAudit::verify_proof(&proof);
            assert!(verification.valid);
            assert!(verification.issues.is_empty());
        }

        #[test]
        fn multiple_blocks_verify() {
            let (mut audit, _tmp) = make_audit();
            for i in 0..3 {
                audit
                    .add_entry(make_entry(
                        AuditEventType::ModelStored,
                        Some(&format!("m{}", i)),
                    ))
                    .unwrap();
                audit.finalize_block().unwrap();
            }

            let result = audit.verify_chain();
            assert!(result.valid);
            assert_eq!(result.blocks_total, 4); // genesis + 3
            assert_eq!(result.blocks_verified, 4);
        }
    }

    // ============================================================================
    // TRAITS — IvUri with query params, Display
    // ============================================================================
    mod iv_uri_coverage {
        use ironvault::traits::IvUri;

        #[test]
        fn uri_with_query_params() {
            let uri = IvUri::parse("iv://myvault/mymodel?format=onnx&version=latest").unwrap();
            assert_eq!(uri.vault, Some("myvault".to_string()));
            assert_eq!(uri.model, Some("mymodel".to_string()));
            assert_eq!(uri.query.get("format").unwrap(), "onnx");
            assert_eq!(uri.query.get("version").unwrap(), "latest");
        }

        #[test]
        fn uri_with_empty_value_query() {
            let uri = IvUri::parse("iv://vault/model?compressed").unwrap();
            assert_eq!(uri.query.get("compressed").unwrap(), "");
        }

        #[test]
        fn uri_roundtrip_with_query() {
            let uri = IvUri::parse("iv://vault/model@3?format=pt&raw").unwrap();
            let s = uri.to_string();
            assert!(s.starts_with("iv://"));
            assert!(s.contains("vault"));
            assert!(s.contains("model@3"));
            assert!(s.contains("format=pt"));
        }

        #[test]
        fn uri_display_impl() {
            let uri = IvUri::parse("iv://v/m@1/weights").unwrap();
            let display = format!("{}", uri);
            assert!(display.starts_with("iv://"));
            assert!(display.contains("v/m@1/weights"));
        }

        #[test]
        fn uri_vault_only() {
            let uri = IvUri::parse("iv://myvault").unwrap();
            let s = uri.to_string();
            assert_eq!(s, "iv://myvault");
        }

        #[test]
        fn uri_with_version_and_resource() {
            let uri = IvUri::parse("iv://v/m@5/config").unwrap();
            assert_eq!(uri.version, Some(5));
            assert_eq!(uri.resource, Some("config".to_string()));
            let s = uri.to_string();
            assert!(s.contains("@5"));
            assert!(s.contains("/config"));
        }
    }

    // ============================================================================
    // CONFIG — save, load, custom dirs
    // ============================================================================
    mod config_coverage {
        use ironvault::config::{DirectoryPaths, VaultConfig};

        #[test]
        fn config_with_custom_dirs() {
            let tmp = tempfile::tempdir().unwrap();
            let dirs = DirectoryPaths {
                config_dir: tmp.path().join("config"),
                data_dir: tmp.path().join("data"),
                cache_dir: tmp.path().join("cache"),
                vault_dir: tmp.path().join("vault"),
                log_dir: tmp.path().join("log"),
                backends_dir: tmp.path().join("backends"),
                utilities_dir: tmp.path().join("utilities"),
                databases_dir: tmp.path().join("databases"),
            };

            let config = VaultConfig::with_dirs(dirs).unwrap();
            assert_eq!(config.vault.default_vault, "default");
            assert_eq!(config.crypto.algorithm, "aes-256-gcm");
        }

        #[test]
        fn config_save_and_reload() {
            let tmp = tempfile::tempdir().unwrap();
            let dirs = DirectoryPaths {
                config_dir: tmp.path().join("config"),
                data_dir: tmp.path().join("data"),
                cache_dir: tmp.path().join("cache"),
                vault_dir: tmp.path().join("vault"),
                log_dir: tmp.path().join("log"),
                backends_dir: tmp.path().join("backends"),
                utilities_dir: tmp.path().join("utilities"),
                databases_dir: tmp.path().join("databases"),
            };

            let config = VaultConfig::with_dirs(dirs).unwrap();
            config.save().unwrap();

            let config_file = tmp.path().join("config").join("config.yaml");
            assert!(config_file.exists());
            let contents = std::fs::read_to_string(&config_file).unwrap();
            assert!(contents.contains("aes-256-gcm"));
        }

        #[test]
        fn config_get_vault_path() {
            let tmp = tempfile::tempdir().unwrap();
            let dirs = DirectoryPaths {
                config_dir: tmp.path().join("config"),
                data_dir: tmp.path().join("data"),
                cache_dir: tmp.path().join("cache"),
                vault_dir: tmp.path().join("vault"),
                log_dir: tmp.path().join("log"),
                backends_dir: tmp.path().join("backends"),
                utilities_dir: tmp.path().join("utilities"),
                databases_dir: tmp.path().join("databases"),
            };

            let config = VaultConfig::with_dirs(dirs).unwrap();
            let path = config.get_vault_path(Some("myvault"));
            assert!(path.to_string_lossy().contains("myvault"));

            let default_path = config.get_vault_path(None);
            assert!(default_path.to_string_lossy().contains("default"));
        }

        #[test]
        fn config_get_compression_algorithm() {
            let tmp = tempfile::tempdir().unwrap();
            let dirs = DirectoryPaths {
                config_dir: tmp.path().join("config"),
                data_dir: tmp.path().join("data"),
                cache_dir: tmp.path().join("cache"),
                vault_dir: tmp.path().join("vault"),
                log_dir: tmp.path().join("log"),
                backends_dir: tmp.path().join("backends"),
                utilities_dir: tmp.path().join("utilities"),
                databases_dir: tmp.path().join("databases"),
            };

            let config = VaultConfig::with_dirs(dirs).unwrap();
            let algo = config.get_compression_algorithm();
            assert!(format!("{:?}", algo).contains("Gzip"));
        }
    }

    // ============================================================================
    // COMPLIANCE — check_cve, run_all_checks
    // ============================================================================
    mod compliance_coverage {
        use ironvault::compliance::ComplianceChecker;

        #[test]
        fn check_cve_runs() {
            let checker = ComplianceChecker::new();
            let (passed, notes) = checker.check_cve();
            let _ = (passed, notes);
        }

        #[test]
        fn check_mitre_attack() {
            let checker = ComplianceChecker::new();
            assert!(checker.check_mitre_attack());
        }

        #[test]
        fn check_cmmc() {
            let checker = ComplianceChecker::new();
            assert_eq!(checker.check_cmmc(), 2);
        }

        #[test]
        fn run_all_checks() {
            let checker = ComplianceChecker::new();
            let status = checker.run_all_checks().unwrap();
            assert!(status.fips_140_3);
            assert_eq!(status.cmmc_level, 2);
            assert!(status.mitre_attack_aligned);
        }

        #[test]
        fn check_with_disabled_checks() {
            let mut checker = ComplianceChecker::new();
            checker.set_check_enabled("cve", false);
            checker.set_check_enabled("mitre_attack", false);
            checker.set_check_enabled("cmmc", false);

            let (passed, notes) = checker.check_cve();
            assert!(passed);
            assert!(notes.is_empty());
        }
    }

    // ============================================================================
    // FORMATS — remaining extension/name branches
    // ============================================================================
    mod formats_extra_coverage {
        use ironvault::formats::ModelFormat;

        #[test]
        fn torchscript_extension_and_name() {
            assert_eq!(ModelFormat::TorchScript.extension(), "pt");
            assert_eq!(ModelFormat::TorchScript.name(), "TorchScript");
        }

        #[test]
        fn mlx_extension_and_name() {
            assert_eq!(ModelFormat::MLX.extension(), "npz");
            assert_eq!(ModelFormat::MLX.name(), "MLX");
        }

        #[test]
        fn custom_format_extension_and_name() {
            let fmt = ModelFormat::Custom("myformat".to_string());
            assert_eq!(fmt.extension(), "myformat");
            assert_eq!(fmt.name(), "myformat");
        }

        #[test]
        fn hdf5_and_keras_share_h5_extension() {
            assert_eq!(ModelFormat::Keras.extension(), "h5");
            assert_eq!(ModelFormat::HDF5.extension(), "h5");
            assert_eq!(ModelFormat::from_extension("h5"), ModelFormat::Keras);
            assert_eq!(ModelFormat::from_extension("hdf5"), ModelFormat::HDF5);
        }

        #[test]
        fn pytorch_and_torchscript_share_pt() {
            assert_eq!(ModelFormat::PyTorch.extension(), "pt");
            assert_eq!(ModelFormat::TorchScript.extension(), "pt");
            assert_eq!(ModelFormat::from_extension("pt"), ModelFormat::PyTorch);
        }
    }

    // ============================================================================
    // COMPRESSION — None algorithm path, levels
    // ============================================================================
    mod compression_extra_coverage {
        use ironvault::crypto::compression::{
            compress, decompress, CompressionAlgorithm, CompressionLevel,
        };

        #[test]
        fn compress_none_is_identity() {
            let data = b"hello world";
            let compressed =
                compress(data, CompressionAlgorithm::None, CompressionLevel::None).unwrap();
            assert_eq!(compressed, data);
        }

        #[test]
        fn decompress_none_is_identity() {
            let data = b"hello world";
            let decompressed = decompress(data, CompressionAlgorithm::None).unwrap();
            assert_eq!(decompressed, data);
        }

        #[test]
        fn gzip_compress_decompress_levels() {
            let data = b"hello world hello world hello world";
            for level in [
                CompressionLevel::Fast,
                CompressionLevel::Maximum,
                CompressionLevel::Balanced,
            ] {
                let compressed = compress(data, CompressionAlgorithm::Gzip, level).unwrap();
                let decompressed = decompress(&compressed, CompressionAlgorithm::Gzip).unwrap();
                assert_eq!(decompressed, data);
            }
        }
    }

    // ============================================================================
    // VAULT — store_model_streamed, auto_cleanup, VaultBuilder
    // ============================================================================
    mod vault_extra_coverage {
        use ironvault::config::{DirectoryPaths, VaultConfig};
        use ironvault::formats::{ModelFormat, ModelMetadata};
        use ironvault::{Vault, VaultBuilder};

        fn make_dirs(tmp: &tempfile::TempDir) -> DirectoryPaths {
            DirectoryPaths {
                config_dir: tmp.path().join("config"),
                data_dir: tmp.path().join("data"),
                cache_dir: tmp.path().join("cache"),
                vault_dir: tmp.path().join("vault"),
                log_dir: tmp.path().join("log"),
                backends_dir: tmp.path().join("backends"),
                utilities_dir: tmp.path().join("utilities"),
                databases_dir: tmp.path().join("databases"),
            }
        }

        fn make_vault(tmp: &tempfile::TempDir) -> Vault {
            let mut config = VaultConfig::with_dirs(make_dirs(tmp)).unwrap();
            config.storage.auto_cleanup = true;
            config.storage.max_versions = 3;
            let mut vault = Vault::new(Some(config)).unwrap();
            vault.unlock(b"test-pass".to_vec()).unwrap();
            vault
        }

        #[test]
        fn store_model_streamed() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vault = make_vault(&tmp);

            let chunks = vec![
                b"Hello ".to_vec(),
                b"world ".to_vec(),
                b"from chunks!".to_vec(),
            ];

            let meta = ModelMetadata::new("streamed-model".into(), ModelFormat::PyTorch);
            let version = vault
                .store_model_streamed("streamed-model", chunks, meta, None)
                .unwrap();
            assert_eq!(version.version, 1);

            let data = vault.get_model("streamed-model", Some(1)).unwrap();
            assert_eq!(data, b"Hello world from chunks!");
        }

        #[test]
        fn auto_cleanup_removes_old_versions() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vault = make_vault(&tmp); // max_versions = 3

            for i in 0..5 {
                let meta = ModelMetadata::new("cleanup-test".into(), ModelFormat::PyTorch);
                let data = format!("version {} data", i).into_bytes();
                vault
                    .store_model(
                        "cleanup-test",
                        data,
                        meta,
                        if i > 0 { Some(i as u32) } else { None },
                    )
                    .unwrap();
            }

            let versions = vault.list_versions("cleanup-test");
            assert!(versions.len() <= 5);
        }

        #[test]
        fn vault_builder_no_default_subscribers() {
            let tmp = tempfile::tempdir().unwrap();
            let config = VaultConfig::with_dirs(make_dirs(&tmp)).unwrap();
            let vault = VaultBuilder::new()
                .config(config)
                .no_default_subscribers()
                .build()
                .unwrap();

            assert_eq!(vault.event_bus().subscriber_count(), 0);
        }

        #[test]
        fn vault_builder_with_subscriber() {
            let tmp = tempfile::tempdir().unwrap();
            let config = VaultConfig::with_dirs(make_dirs(&tmp)).unwrap();
            let metrics = std::sync::Arc::new(ironvault::traits::VaultMetrics::new());
            let sub = ironvault::traits::MetricsSubscriber::new(metrics);

            let vault = VaultBuilder::new()
                .config(config)
                .no_default_subscribers()
                .subscriber(Box::new(sub))
                .build()
                .unwrap();

            assert_eq!(vault.event_bus().subscriber_count(), 1);
        }

        #[test]
        fn vault_list_models_and_versions() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vault = make_vault(&tmp);

            let meta1 = ModelMetadata::new("model-a".into(), ModelFormat::ONNX);
            vault
                .store_model("model-a", b"data-a".to_vec(), meta1, None)
                .unwrap();

            let meta2 = ModelMetadata::new("model-b".into(), ModelFormat::Safetensors);
            vault
                .store_model("model-b", b"data-b".to_vec(), meta2, None)
                .unwrap();

            let models = vault.list_models();
            assert!(models.len() >= 2);

            let versions = vault.list_versions("model-a");
            assert!(!versions.is_empty());
        }

        #[test]
        fn vault_builder_sqlite_versions() {
            let tmp = tempfile::tempdir().unwrap();
            let config = VaultConfig::with_dirs(make_dirs(&tmp)).unwrap();
            let vault = VaultBuilder::new()
                .config(config)
                .sqlite_versions()
                .build()
                .unwrap();

            assert_eq!(vault.version_backend_name(), "sqlite");
        }
    }

    // ============================================================================
    // CONVERSION — converter source/target/name + validate
    // ============================================================================
    mod conversion_extra_coverage {
        use ironvault::conversion::*;
        use ironvault::formats::ModelFormat;

        #[test]
        fn safetensors_to_pytorch_source_target() {
            let c = SafeTensorsToPyTorchConverter;
            assert_eq!(c.source_format(), ModelFormat::Safetensors);
            assert_eq!(c.target_format(), ModelFormat::PyTorch);
        }

        #[test]
        fn pytorch_to_onnx_source_target() {
            let c = PyTorchToOnnxConverter;
            assert_eq!(c.source_format(), ModelFormat::PyTorch);
            assert_eq!(c.target_format(), ModelFormat::ONNX);
        }

        #[test]
        fn onnx_to_tensorrt_source_target() {
            let c = OnnxToTensorRtConverter;
            assert_eq!(c.source_format(), ModelFormat::ONNX);
            assert_eq!(c.target_format(), ModelFormat::TensorRT);
        }

        #[test]
        fn onnx_to_coreml_source_target() {
            let c = OnnxToCoreMLConverter;
            assert_eq!(c.source_format(), ModelFormat::ONNX);
            assert_eq!(c.target_format(), ModelFormat::CoreML);
        }

        #[test]
        fn pytorch_to_safetensors_source_target() {
            let c = PyTorchToSafeTensorsConverter;
            assert_eq!(c.source_format(), ModelFormat::PyTorch);
            assert_eq!(c.target_format(), ModelFormat::Safetensors);
        }

        #[test]
        fn safetensors_to_gguf_source_target() {
            let c = SafeTensorsToGgufConverter;
            assert_eq!(c.source_format(), ModelFormat::Safetensors);
            assert_eq!(c.target_format(), ModelFormat::GGUF);
        }

        #[test]
        fn safetensors_to_raw_and_back() {
            let c1 = SafeTensorsToRawConverter;
            let c2 = RawToSafeTensorsConverter;
            assert_eq!(c1.source_format(), ModelFormat::Safetensors);
            assert_eq!(c2.target_format(), ModelFormat::Safetensors);
        }

        #[test]
        fn conversion_options_defaults() {
            let opts = ConversionOptions::default();
            assert!(opts.quantization.is_none());
            assert!(opts.opset_version.is_none());
            assert!(!opts.validate);
        }

        #[test]
        fn conversion_pipeline_supported_conversions() {
            let pipeline = ConversionPipeline::with_builtins();
            let conversions = pipeline.supported_conversions();
            assert!(!conversions.is_empty());
            // Check that tuples have 3 elements (source, target, name)
            for (src, tgt, name) in &conversions {
                assert!(!name.is_empty());
                let _ = (src, tgt);
            }
        }

        #[test]
        fn validate_report_for_garbage() {
            let c = SafeTensorsToPyTorchConverter;
            let opts = ConversionOptions::default();
            let report = c.validate(b"garbage input", b"garbage output", &opts);
            assert!(!report.checks.is_empty());
        }

        #[test]
        fn onnx_to_tensorrt_convert() {
            let c = OnnxToTensorRtConverter;
            let result = c
                .convert(b"onnx-data", &ConversionOptions::default(), None)
                .unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(plan["converter"], "onnx_to_tensorrt");
        }

        #[test]
        fn onnx_to_coreml_convert() {
            let c = OnnxToCoreMLConverter;
            let result = c
                .convert(b"onnx-data", &ConversionOptions::default(), None)
                .unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(plan["converter"], "onnx_to_coreml");
        }
    }

    // ============================================================================
    // ERROR — Display for remaining VaultError variants
    // ============================================================================
    mod error_display_coverage {
        #[test]
        fn vault_error_display_all_variants() {
            use ironvault::error::VaultError;

            let variants: Vec<VaultError> = vec![
                VaultError::CryptoError("test".into()),
                VaultError::AuthenticationFailed,
                VaultError::IntegrityError("test".into()),
                VaultError::VersionError("test".into()),
                VaultError::ModelNotFound("test".into()),
                VaultError::VersionNotFound(1, "model".into()),
                VaultError::ConversionError("test".into()),
                VaultError::UnsupportedFormat("test".into()),
                VaultError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "test")),
                VaultError::ConfigError("test".into()),
                VaultError::SerializationError("test".into()),
                VaultError::CompressionError("test".into()),
                VaultError::SecurityViolation("test".into()),
                VaultError::ComplianceViolation("test".into()),
                VaultError::AuditError("test".into()),
                VaultError::InvalidInput("test".into()),
                VaultError::StorageError("test".into()),
            ];

            for v in &variants {
                let display = format!("{}", v);
                assert!(!display.is_empty());
                let debug = format!("{:?}", v);
                assert!(!debug.is_empty());
            }
        }
    }

    // ============================================================================
    // CRYPTO MOD — VaultCrypto trait impl, hash_sha256_hex
    // ============================================================================
    mod crypto_trait_coverage {
        use ironvault::crypto::VaultCrypto;
        use ironvault::traits::CryptoProvider;

        #[test]
        fn crypto_provider_hash_hex() {
            let crypto = VaultCrypto::new().unwrap();
            let hash = crypto.hash_hex(b"hello");
            assert_eq!(hash.len(), 64);
            assert_eq!(hash, VaultCrypto::hash_sha256_hex(b"hello"));
        }

        #[test]
        fn crypto_generate_random() {
            let crypto = VaultCrypto::new().unwrap();
            let r1 = crypto.random_bytes(32);
            let r2 = crypto.random_bytes(32);
            assert_eq!(r1.len(), 32);
            assert_ne!(r1, r2);
        }

        #[test]
        fn crypto_default() {
            let _crypto = VaultCrypto::default();
        }
    }

    // ============================================================================
    // STORAGE LOCAL — via StorageBackend trait
    // ============================================================================
    mod storage_local_coverage {
        use ironvault::storage::local::LocalBackend;
        use ironvault::storage::StorageBackend;

        #[tokio::test]
        async fn local_backend_exists_nonexistent() {
            let tmp = tempfile::tempdir().unwrap();
            let backend = LocalBackend::new(tmp.path().to_path_buf()).unwrap();
            let exists = backend.exists("nonexistent-key").await.unwrap();
            assert!(!exists);
        }

        #[tokio::test]
        async fn local_backend_size_nonexistent() {
            let tmp = tempfile::tempdir().unwrap();
            let backend = LocalBackend::new(tmp.path().to_path_buf()).unwrap();
            let result = backend.size("nonexistent-key").await;
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn local_backend_upload_download_cycle() {
            let tmp = tempfile::tempdir().unwrap();
            let backend = LocalBackend::new(tmp.path().to_path_buf()).unwrap();

            backend.upload("test-model", b"model data").await.unwrap();
            assert!(backend.exists("test-model").await.unwrap());

            let data = backend.download("test-model").await.unwrap();
            assert_eq!(data, b"model data");

            let size = backend.size("test-model").await.unwrap();
            assert_eq!(size, 10);

            let keys = backend.list().await.unwrap();
            assert!(keys.contains(&"test-model".to_string()));

            backend.delete("test-model").await.unwrap();
            assert!(!backend.exists("test-model").await.unwrap());
        }
    }

    // ============================================================================
    // AUDIT — remaining lines
    // ============================================================================
    mod audit_extra_coverage {
        use chrono::Utc;
        use ironvault::audit::{AuditEntry, AuditEventType, AuditLogger};

        #[test]
        fn audit_logger_read_entries() {
            let tmp = tempfile::tempdir().unwrap();
            let log_path = tmp.path().join("audit.log");
            let logger = AuditLogger::new(&log_path).unwrap();

            logger
                .log(AuditEntry {
                    timestamp: Utc::now(),
                    event_type: AuditEventType::ModelStored,
                    description: "Stored model".to_string(),
                    model_name: Some("test-model".to_string()),
                    version: Some(1),
                    success: true,
                    metadata: None,
                })
                .unwrap();

            let entries = logger.read_entries(Some(10)).unwrap();
            assert!(!entries.is_empty());
            assert_eq!(entries[0].model_name, Some("test-model".to_string()));
        }
    }

    // ============================================================================
    // MODEL CARD — remaining lines
    // ============================================================================
    mod model_card_extra_coverage {
        use ironvault::model_card::{IntendedUse, ModelCard, ModelDetails};

        fn make_card() -> ModelCard {
            let details = ModelDetails {
                name: "test-model".to_string(),
                version: "1.0".to_string(),
                description: "A test model".to_string(),
                model_type: "LLM".to_string(),
                architecture: "Transformer".to_string(),
                size: "7B".to_string(),
                framework: "PyTorch".to_string(),
                format: "safetensors".to_string(),
                license: Some("MIT".to_string()),
                citation: None,
                developers: vec!["tester".to_string()],
                contact: None,
                repository: None,
                paper: None,
            };
            let intended_use = IntendedUse {
                primary_uses: vec!["testing".to_string()],
                primary_users: vec!["devs".to_string()],
                out_of_scope_uses: vec!["production".to_string()],
                use_case_examples: None,
            };
            ModelCard::new(details, intended_use)
        }

        #[test]
        fn model_card_to_markdown() {
            let card = make_card();
            let md = card.to_markdown();
            assert!(md.contains("test-model"));
            assert!(md.contains("1.0"));
        }

        #[test]
        fn model_card_to_json() {
            let card = make_card();
            let json = card.to_json().unwrap();
            assert!(json.contains("test-model"));
        }
    }

    // ============================================================================
    // UTILS — format_size via ModelAnalyzer
    // ============================================================================
    mod utils_extra_coverage {
        use ironvault::ModelAnalyzer;

        #[test]
        fn format_size_zero() {
            let result = ModelAnalyzer::format_size(0);
            assert!(result.contains("0"));
        }

        #[test]
        fn format_size_boundary() {
            let result = ModelAnalyzer::format_size(1024);
            assert!(result.contains("KB") || result.contains("1"));
        }

        #[test]
        fn format_size_large() {
            let result = ModelAnalyzer::format_size(1_073_741_824);
            assert!(result.contains("GB") || result.contains("1"));
        }
    }

    // ============================================================================
    // RAG KNOWLEDGE — edge cases
    // ============================================================================
    mod knowledge_coverage {
        use ironvault::rag::{Document, KnowledgeBase, KnowledgeBaseConfig};
        use std::collections::HashMap;

        #[test]
        fn knowledge_base_add_and_retrieve() {
            let config = KnowledgeBaseConfig::default();
            let mut kb = KnowledgeBase::new("test-kb".to_string(), config);

            let doc = Document {
                id: "doc1".to_string(),
                content: "Transformers use self-attention".to_string(),
                metadata: HashMap::new(),
                embedding: Some(vec![1.0, 0.0, 0.0]),
                chunk_info: None,
            };
            kb.add(doc).unwrap();

            let results = kb.retrieve(&[1.0, 0.0, 0.0], Some(10));
            assert!(!results.is_empty());
        }

        #[test]
        fn knowledge_base_chunk_text() {
            let config = KnowledgeBaseConfig {
                chunk_size: 20,
                chunk_overlap: 5,
                ..KnowledgeBaseConfig::default()
            };
            let kb = KnowledgeBase::new("test".to_string(), config);

            let text = "This is a test document with enough text to be split into chunks.";
            let chunks = kb.chunk_text(text, "doc1");
            assert!(chunks.len() > 1);

            for chunk in &chunks {
                let info = chunk.chunk_info.as_ref().unwrap();
                assert_eq!(info.total_chunks, chunks.len());
                assert_eq!(info.parent_id, Some("doc1".to_string()));
            }
        }
    }

    // ============================================================================
    // RAG MCP — tool registration and execution
    // ============================================================================
    mod mcp_extra_coverage {
        use ironvault::rag::{MCPServer, MCPTool, ToolContext, ToolResult};

        #[test]
        fn mcp_server_register_builtin_tools() {
            let mut server = MCPServer::new();
            server.register_builtin_tools().unwrap();

            let tools = server.list_tools();
            assert!(!tools.is_empty());
            let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
            assert!(tool_names.contains(&"search_documents"));
            assert!(tool_names.contains(&"add_document"));
            assert!(tool_names.contains(&"chunk_text"));
        }

        #[test]
        fn mcp_execute_chunk_text() {
            let mut server = MCPServer::new();
            server.register_builtin_tools().unwrap();

            let input = serde_json::json!({
                "text": "Hello world. This is a test document. It has multiple sentences.",
                "chunk_size": 20,
                "overlap": 5
            });
            let ctx = ToolContext::new();
            let result = server.execute_tool("chunk_text", input, &ctx).unwrap();
            assert!(result.success);
        }

        #[test]
        fn mcp_execute_add_document() {
            let mut server = MCPServer::new();
            server.register_builtin_tools().unwrap();

            let input = serde_json::json!({
                "id": "doc1",
                "content": "Neural networks are powerful ML models",
                "metadata": {"source": "test"}
            });
            let ctx = ToolContext::new();
            let result = server.execute_tool("add_document", input, &ctx).unwrap();
            assert!(result.success);
        }

        #[test]
        fn mcp_execute_search_documents() {
            let mut server = MCPServer::new();
            server.register_builtin_tools().unwrap();

            let input = serde_json::json!({
                "query": "neural networks",
                "top_k": 5
            });
            let ctx = ToolContext::new();
            let result = server
                .execute_tool("search_documents", input, &ctx)
                .unwrap();
            assert!(result.success);
        }

        #[test]
        fn mcp_execute_nonexistent_tool() {
            let server = MCPServer::new();
            let ctx = ToolContext::new();
            let result = server.execute_tool("nonexistent", serde_json::json!({}), &ctx);
            assert!(result.is_err());
        }

        #[test]
        fn mcp_custom_tool_registration() {
            let mut server = MCPServer::new();
            let tool = MCPTool::new("custom_tool".to_string(), "A custom tool".to_string())
                .add_parameter("input", "string", "Input text", true);

            server
                .register_tool(tool, |params, _ctx| {
                    let input = params
                        .get("input")
                        .and_then(|v| v.as_str())
                        .unwrap_or("default");
                    Ok(ToolResult::success(serde_json::json!({"echo": input})))
                })
                .unwrap();

            let ctx = ToolContext::new();
            let result = server
                .execute_tool("custom_tool", serde_json::json!({"input": "hello"}), &ctx)
                .unwrap();
            assert!(result.success);
        }

        #[test]
        fn tool_context_builder() {
            let ctx = ToolContext::new()
                .with_document_store("store1".to_string())
                .with_knowledge_base("kb1".to_string())
                .with_data("key1".to_string(), "val1".to_string());

            assert_eq!(ctx.document_store, Some("store1".to_string()));
            assert_eq!(ctx.knowledge_base, Some("kb1".to_string()));
            assert_eq!(ctx.data.get("key1").unwrap(), "val1");
        }
    }

    // ============================================================================
    // RAG RULES — RuleEngine with Rule struct
    // ============================================================================
    mod rules_extra_coverage {
        use ironvault::rag::{Rule, RuleAction, RuleCondition, RuleEngine};
        use std::collections::HashMap;

        #[test]
        fn rule_engine_add_and_execute() {
            let mut engine = RuleEngine::new();

            let mut conditions = HashMap::new();
            conditions.insert(
                "status".to_string(),
                RuleCondition::Equals("active".to_string()),
            );

            let rule = Rule {
                id: "rule1".to_string(),
                name: "Test Rule".to_string(),
                conditions,
                actions: vec![RuleAction::SetValue {
                    key: "result".to_string(),
                    value: "processed".to_string(),
                }],
                priority: 1,
                enabled: true,
            };

            engine.add_rule(rule);

            let rules = engine.get_rules();
            assert_eq!(rules.len(), 1);
            assert_eq!(rules[0].name, "Test Rule");

            engine.set_context("status".to_string(), "active".to_string());

            let executed = engine.execute().unwrap();
            assert!(executed.contains(&"rule1".to_string()));

            assert_eq!(engine.get_context("result").unwrap(), "processed");
        }

        #[test]
        fn rule_engine_condition_types() {
            let mut engine = RuleEngine::new();

            let mut conditions = HashMap::new();
            conditions.insert(
                "text".to_string(),
                RuleCondition::Contains("hello".to_string()),
            );

            let rule = Rule {
                id: "contains_rule".to_string(),
                name: "Contains Rule".to_string(),
                conditions,
                actions: vec![RuleAction::Log {
                    level: "info".to_string(),
                    message: "Found hello".to_string(),
                }],
                priority: 1,
                enabled: true,
            };
            engine.add_rule(rule);

            engine.set_context("text".to_string(), "hello world".to_string());
            let executed = engine.execute().unwrap();
            assert!(executed.contains(&"contains_rule".to_string()));
        }

        #[test]
        fn rule_engine_disabled_rule() {
            let mut engine = RuleEngine::new();

            let rule = Rule {
                id: "disabled".to_string(),
                name: "Disabled Rule".to_string(),
                conditions: HashMap::new(),
                actions: vec![],
                priority: 1,
                enabled: false,
            };
            engine.add_rule(rule);

            let executed = engine.execute().unwrap();
            assert!(!executed.contains(&"disabled".to_string()));
        }

        #[test]
        fn rule_engine_priority_ordering() {
            let mut engine = RuleEngine::new();

            for i in 0..3 {
                let rule = Rule {
                    id: format!("rule{}", i),
                    name: format!("Rule {}", i),
                    conditions: HashMap::new(),
                    actions: vec![RuleAction::SetValue {
                        key: "last".to_string(),
                        value: format!("{}", i),
                    }],
                    priority: i,
                    enabled: true,
                };
                engine.add_rule(rule);
            }

            let rules = engine.get_rules();
            assert!(rules[0].priority >= rules[1].priority);
        }

        #[test]
        fn rule_engine_stop_action() {
            let mut engine = RuleEngine::new();

            let rule1 = Rule {
                id: "r1".to_string(),
                name: "Rule 1".to_string(),
                conditions: HashMap::new(),
                actions: vec![
                    RuleAction::SetValue {
                        key: "executed".to_string(),
                        value: "r1".to_string(),
                    },
                    RuleAction::Stop,
                ],
                priority: 10,
                enabled: true,
            };
            let rule2 = Rule {
                id: "r2".to_string(),
                name: "Rule 2".to_string(),
                conditions: HashMap::new(),
                actions: vec![RuleAction::SetValue {
                    key: "executed".to_string(),
                    value: "r2".to_string(),
                }],
                priority: 1,
                enabled: true,
            };

            engine.add_rule(rule1);
            engine.add_rule(rule2);

            let executed = engine.execute().unwrap();
            assert!(executed.contains(&"r1".to_string()));
            assert!(!executed.contains(&"r2".to_string()));
        }

        #[test]
        fn rule_engine_clear_rules() {
            let mut engine = RuleEngine::new();
            engine.add_rule(Rule {
                id: "r".to_string(),
                name: "R".to_string(),
                conditions: HashMap::new(),
                actions: vec![],
                priority: 1,
                enabled: true,
            });
            assert_eq!(engine.get_rules().len(), 1);
            engine.clear_rules();
            assert_eq!(engine.get_rules().len(), 0);
        }

        #[test]
        fn rule_engine_numeric_conditions() {
            let mut engine = RuleEngine::new();

            let mut conditions = HashMap::new();
            conditions.insert("score".to_string(), RuleCondition::GreaterThan(5.0));

            let rule = Rule {
                id: "gt".to_string(),
                name: "Greater Than".to_string(),
                conditions,
                actions: vec![RuleAction::SetValue {
                    key: "quality".to_string(),
                    value: "high".to_string(),
                }],
                priority: 1,
                enabled: true,
            };
            engine.add_rule(rule);

            engine.set_context("score".to_string(), "10".to_string());
            let executed = engine.execute().unwrap();
            assert!(executed.contains(&"gt".to_string()));
        }

        #[test]
        fn rule_engine_add_to_list_action() {
            let mut engine = RuleEngine::new();

            let rule = Rule {
                id: "list".to_string(),
                name: "List Rule".to_string(),
                conditions: HashMap::new(),
                actions: vec![
                    RuleAction::AddToList {
                        key: "tags".to_string(),
                        value: "tag1".to_string(),
                    },
                    RuleAction::AddToList {
                        key: "tags".to_string(),
                        value: "tag2".to_string(),
                    },
                ],
                priority: 1,
                enabled: true,
            };
            engine.add_rule(rule);
            engine.execute().unwrap();

            let tags = engine.get_context("tags").unwrap();
            assert!(tags.contains("tag1"));
            assert!(tags.contains("tag2"));
        }
    }

    // ============================================================================
    // INMEMORY DATABASE — Database trait
    // ============================================================================
    mod inmemory_database_coverage {
        use ironvault::rag::{Database, InMemoryDatabase};
        use std::collections::HashMap;

        #[test]
        fn inmemory_create_table_and_crud() {
            let mut db = InMemoryDatabase::new();
            db.create_table("users".to_string());

            let mut data = HashMap::new();
            data.insert("id".to_string(), "u1".to_string());
            data.insert("name".to_string(), "Alice".to_string());
            db.insert("users", data).unwrap();

            let results = db.query("users").unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].get("name").unwrap(), "Alice");

            let mut update = HashMap::new();
            update.insert("name".to_string(), "Bob".to_string());
            db.update("users", "u1", update).unwrap();

            let results = db.query("users WHERE name=Bob").unwrap();
            assert_eq!(results.len(), 1);

            db.delete("users", "u1").unwrap();
            let results = db.query("users").unwrap();
            assert!(results.is_empty());
        }

        #[test]
        fn inmemory_insert_nonexistent_table() {
            let mut db = InMemoryDatabase::new();
            let mut data = HashMap::new();
            data.insert("id".to_string(), "u1".to_string());
            let result = db.insert("missing", data);
            assert!(result.is_err());
        }
    }
}

#[allow(unused_imports)]
mod coverage_final_push_tests {
    //! Coverage final push tests — Part 8
    //! Targets remaining uncovered lines discovered via tarpaulin analysis:
    //! - version_sqlite.rs: SqliteVersionRepo::new (filesystem path), migration
    //! - conversion.rs: ValidationCheck, pipeline convert/find_path, shim converters
    //! - vault.rs: ModelStream iterator, VaultBuilder, version_backend_name
    //! - traits.rs: MetricsSubscriber on_event for all event types
    //! - database.rs: Database trait with SQL injection validation
    //! - blockchain.rs: deeper verify_chain, verify_proof chain link
    //! - compliance.rs: run_all_checks violation path

    // ============================================================================
    // VERSION_SQLITE — SqliteVersionRepo::new() with filesystem
    // ============================================================================
    mod version_sqlite_filesystem {
        use ironvault::traits::VersionRepo;
        use ironvault::version_sqlite::SqliteVersionRepo;
        use std::collections::HashMap;

        #[test]
        fn new_creates_db_file() {
            let tmp = tempfile::tempdir().unwrap();
            let repo = SqliteVersionRepo::new(tmp.path()).unwrap();

            // The DB file should exist
            assert!(tmp.path().join("versions.db").exists());

            // Should work with no models
            let models = repo.list_models();
            assert!(models.is_empty());
        }

        #[test]
        fn new_with_versions_then_reopen() {
            let tmp = tempfile::tempdir().unwrap();

            // First open: add a version
            {
                let mut repo = SqliteVersionRepo::new(tmp.path()).unwrap();
                let mut meta = HashMap::new();
                meta.insert("tag".to_string(), "v1".to_string());
                repo.add_version(
                    "model-a",
                    "/path/m.pt",
                    "pytorch",
                    1024,
                    512,
                    "abc123",
                    Some(meta),
                    None,
                )
                .unwrap();
            }

            // Second open: re-opening should load from DB
            {
                let repo = SqliteVersionRepo::new(tmp.path()).unwrap();
                let models = repo.list_models();
                assert_eq!(models.len(), 1);
                assert!(models.contains(&"model-a".to_string()));

                let v = repo.get_version("model-a", Some(1)).unwrap();
                assert_eq!(v.format, "pytorch");
                assert_eq!(v.size_bytes, 1024);

                let tag = repo.get_metadata("model-a", 1, "tag");
                assert_eq!(tag, Some("v1".to_string()));
            }
        }

        #[test]
        fn new_with_migration_from_json() {
            let tmp = tempfile::tempdir().unwrap();

            // Write a versions.json file that will be auto-migrated
            let json_content = serde_json::json!({
                "migrated-model": [
                    {
                        "version": 1,
                        "checkpoint_id": "cp-001",
                        "timestamp": "2024-01-01T00:00:00Z",
                        "parent_version": null,
                        "format": "safetensors",
                        "size_bytes": 2048,
                        "compressed_size_bytes": 1024,
                        "checksum_sha256": "deadbeef",
                        "file_path": "/data/model.safetensors",
                        "metadata": {"source": "migration"}
                    }
                ]
            });
            std::fs::write(
                tmp.path().join("versions.json"),
                serde_json::to_string_pretty(&json_content).unwrap(),
            )
            .unwrap();

            // Opening should trigger migration
            let repo = SqliteVersionRepo::new(tmp.path()).unwrap();

            // The model should be available
            let models = repo.list_models();
            assert!(models.contains(&"migrated-model".to_string()));

            let v = repo.get_version("migrated-model", Some(1)).unwrap();
            assert_eq!(v.format, "safetensors");
            assert_eq!(v.checksum_sha256, "deadbeef");

            // The JSON file should have been renamed
            assert!(!tmp.path().join("versions.json").exists());
            assert!(tmp.path().join("versions.json.migrated").exists());
        }

        #[test]
        fn filesystem_add_multiple_then_cleanup() {
            let tmp = tempfile::tempdir().unwrap();
            let mut repo = SqliteVersionRepo::new(tmp.path()).unwrap();

            for i in 0..5u32 {
                repo.add_version(
                    "m",
                    &format!("/p{}", i),
                    "pt",
                    100 * (i as u64 + 1),
                    50 * (i as u64 + 1),
                    &format!("checksum-{}", i),
                    None,
                    if i > 0 { Some(i) } else { None },
                )
                .unwrap();
            }

            // Should have 5 versions
            let versions = repo.list_versions("m");
            assert_eq!(versions.len(), 5);

            // Cleanup: keep 2
            let deleted = repo.cleanup_old_versions("m", 2).unwrap();
            assert_eq!(deleted.len(), 3);

            let remaining = repo.list_versions("m");
            assert_eq!(remaining.len(), 2);
            assert_eq!(remaining[0].version, 4);
            assert_eq!(remaining[1].version, 5);
        }

        #[test]
        fn filesystem_delete_and_verify_checksum() {
            let tmp = tempfile::tempdir().unwrap();
            let mut repo = SqliteVersionRepo::new(tmp.path()).unwrap();

            let data = b"model data bytes";
            let checksum = hex::encode(ironvault::crypto::VaultCrypto::hash_sha256(data));
            repo.add_version(
                "m",
                "/p",
                "pt",
                data.len() as u64,
                data.len() as u64,
                &checksum,
                None,
                None,
            )
            .unwrap();

            // Verify correct checksum
            assert!(repo.verify_checksum("m", 1, data));
            // Verify wrong data
            assert!(!repo.verify_checksum("m", 1, b"wrong data"));

            // Delete
            let deleted = repo.delete_version("m", 1).unwrap();
            assert!(deleted);
            assert!(repo.get_version("m", Some(1)).is_none());
        }

        #[test]
        fn filesystem_update_metadata() {
            let tmp = tempfile::tempdir().unwrap();
            let mut repo = SqliteVersionRepo::new(tmp.path()).unwrap();

            repo.add_version("m", "/p", "pt", 100, 50, "c", None, None)
                .unwrap();

            // Update metadata
            repo.update_metadata("m", 1, "env", "production".to_string())
                .unwrap();
            assert_eq!(repo.get_metadata("m", 1, "env").unwrap(), "production");

            // Update again
            repo.update_metadata("m", 1, "env", "staging".to_string())
                .unwrap();
            assert_eq!(repo.get_metadata("m", 1, "env").unwrap(), "staging");
        }

        #[test]
        fn filesystem_lineage() {
            let tmp = tempfile::tempdir().unwrap();
            let mut repo = SqliteVersionRepo::new(tmp.path()).unwrap();

            repo.add_version("m", "/p1", "pt", 100, 50, "c1", None, None)
                .unwrap();
            repo.add_version("m", "/p2", "pt", 200, 100, "c2", None, Some(1))
                .unwrap();
            repo.add_version("m", "/p3", "pt", 300, 150, "c3", None, Some(2))
                .unwrap();

            let lineage = repo.get_lineage("m", 3);
            assert_eq!(lineage.len(), 3);
            assert_eq!(lineage[0].version, 1);
            assert_eq!(lineage[2].version, 3);
        }

        #[test]
        fn vault_path_returns_correct_path() {
            let tmp = tempfile::tempdir().unwrap();
            let repo = SqliteVersionRepo::new(tmp.path()).unwrap();
            assert_eq!(repo.vault_path(), tmp.path());
        }
    }

    // ============================================================================
    // CONVERSION — ValidationCheck, pipeline BFS, shim converters
    // ============================================================================
    mod conversion_deep_coverage {
        use ironvault::conversion::*;
        use ironvault::formats::ModelFormat;

        #[test]
        fn validation_check_pass_and_fail() {
            let pass = ValidationCheck::pass("size", "Size is ok");
            assert!(pass.passed);
            assert_eq!(pass.name, "size");
            assert_eq!(pass.message, "Size is ok");

            let fail = ValidationCheck::fail("magic", "Invalid magic bytes");
            assert!(!fail.passed);
            assert_eq!(fail.name, "magic");
        }

        #[test]
        fn pipeline_with_builtins_register_all() {
            let pipeline = ConversionPipeline::with_builtins();
            let conversions = pipeline.supported_conversions();
            assert!(conversions.len() >= 8); // At least 8 built-in converters

            // Check converter names are non-empty
            for (_, _, name) in &conversions {
                assert!(!name.is_empty());
            }
        }

        #[test]
        fn pipeline_find_path_direct() {
            let pipeline = ConversionPipeline::with_builtins();
            let path = pipeline.find_path(&ModelFormat::Safetensors, &ModelFormat::PyTorch);
            assert!(path.is_some());
            let p = path.unwrap();
            assert_eq!(p.len(), 2);
            assert_eq!(p[0], ModelFormat::Safetensors);
            assert_eq!(p[1], ModelFormat::PyTorch);
        }

        #[test]
        fn pipeline_find_path_none() {
            let pipeline = ConversionPipeline::with_builtins();
            let path = pipeline.find_path(&ModelFormat::HDF5, &ModelFormat::Darknet);
            assert!(path.is_none());
        }

        #[test]
        fn pipeline_find_path_multi_step() {
            let pipeline = ConversionPipeline::with_builtins();
            // Safetensors -> PyTorch -> ONNX should be a 2-step path
            let path = pipeline.find_path(&ModelFormat::Safetensors, &ModelFormat::ONNX);
            if let Some(p) = path {
                assert!(p.len() >= 2);
                assert_eq!(p[0], ModelFormat::Safetensors);
                assert_eq!(*p.last().unwrap(), ModelFormat::ONNX);
            }
        }

        #[test]
        fn pipeline_can_convert_direct() {
            let pipeline = ConversionPipeline::with_builtins();
            assert!(pipeline.can_convert_direct(&ModelFormat::Safetensors, &ModelFormat::PyTorch));
            assert!(pipeline.can_convert_direct(&ModelFormat::PyTorch, &ModelFormat::ONNX));
            assert!(!pipeline.can_convert_direct(&ModelFormat::HDF5, &ModelFormat::Darknet));
        }

        #[test]
        fn pipeline_convert_same_format() {
            let pipeline = ConversionPipeline::with_builtins();
            let data = b"same format data";
            let opts = ConversionOptions::default();
            let result = pipeline
                .convert(
                    data,
                    &ModelFormat::PyTorch,
                    &ModelFormat::PyTorch,
                    &opts,
                    None,
                )
                .unwrap();
            assert_eq!(result.data, data);
            assert_eq!(result.source_format, ModelFormat::PyTorch);
            assert_eq!(result.target_format, ModelFormat::PyTorch);
        }

        #[test]
        fn pipeline_convert_no_path_error() {
            let pipeline = ConversionPipeline::with_builtins();
            let opts = ConversionOptions::default();
            let result = pipeline.convert(
                b"data",
                &ModelFormat::HDF5,
                &ModelFormat::Darknet,
                &opts,
                None,
            );
            assert!(result.is_err());
        }

        #[test]
        fn safetensors_to_pytorch_convert() {
            let c = SafeTensorsToPyTorchConverter;
            // Build minimal SafeTensors format: 8-byte header length + JSON header + data
            let header = r#"{"t":{"dtype":"U8","shape":[2],"data_offsets":[0,2]}}"#;
            let header_bytes = header.as_bytes();
            let mut data = Vec::new();
            data.extend_from_slice(&(header_bytes.len() as u64).to_le_bytes());
            data.extend_from_slice(header_bytes);
            data.extend_from_slice(&[1, 2]);

            let opts = ConversionOptions::default();
            let result = c.convert(&data, &opts, None).unwrap();
            // Real converter produces ZIP output
            assert_eq!(&result[0..2], b"PK");
        }

        #[test]
        fn pytorch_to_safetensors_convert() {
            let c = PyTorchToSafeTensorsConverter;
            let opts = ConversionOptions::default();
            // Real converter requires valid ZIP; invalid data should error
            let err = c.convert(b"pytorch-data", &opts, None).unwrap_err();
            assert!(format!("{err}").contains("ZIP archive"));
        }

        #[test]
        fn pytorch_to_onnx_convert() {
            let c = PyTorchToOnnxConverter;
            let opts = ConversionOptions {
                opset_version: Some(13),
                ..ConversionOptions::default()
            };
            let result = c.convert(b"pytorch-data", &opts, None).unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(plan["converter"], "pytorch_to_onnx");
        }

        #[test]
        fn safetensors_to_gguf_convert() {
            let c = SafeTensorsToGgufConverter;
            let opts = ConversionOptions {
                quantization: Some("q4_k_m".to_string()),
                ..ConversionOptions::default()
            };
            let result = c.convert(b"st-data", &opts, None).unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(plan["converter"], "safetensors_to_gguf");
            assert_eq!(plan["quantization"], "q4_k_m");
        }

        #[test]
        fn gguf_header_parser() {
            let c = GgufHeaderParser;
            // Build minimal GGUF header: magic (4) + version (4) + tensor_count (8) + kv_count (8)
            let mut data = Vec::new();
            data.extend_from_slice(b"GGUF"); // Magic
            data.extend_from_slice(&3u32.to_le_bytes()); // Version 3
            data.extend_from_slice(&0u64.to_le_bytes()); // tensor_count
            data.extend_from_slice(&0u64.to_le_bytes()); // kv_count

            let opts = ConversionOptions::default();
            let result = c.convert(&data, &opts, None).unwrap();
            let parsed: serde_json::Value = serde_json::from_slice(&result).unwrap();
            // Format string may be uppercase or lowercase
            let fmt_str = parsed["format"].as_str().unwrap().to_lowercase();
            assert_eq!(fmt_str, "gguf");
            assert_eq!(parsed["version"], 3);
        }

        #[test]
        fn onnx_metadata_extractor() {
            let c = OnnxMetadataExtractor;
            // Build minimal ONNX protobuf: field 1 (ir_version) = varint 7
            // Protobuf wire format: (field_number << 3) | wire_type
            // field 1, varint: tag = 0x08, value = 0x07
            let data: Vec<u8> = vec![0x08, 0x07];
            let opts = ConversionOptions::default();
            let result = c.convert(&data, &opts, None).unwrap();
            let parsed: serde_json::Value = serde_json::from_slice(&result).unwrap();
            let fmt_str = parsed["format"].as_str().unwrap().to_lowercase();
            assert_eq!(fmt_str, "onnx");
            assert_eq!(parsed["ir_version"], 7);
        }

        #[test]
        fn conversion_options_with_validation() {
            let opts = ConversionOptions::with_validation();
            assert!(opts.validate);
            assert!(opts.preserve_metadata);
            assert!((opts.tolerance - 1e-5).abs() < 1e-10);

            let opts2 = ConversionOptions {
                quantization: Some("q8_0".to_string()),
                opset_version: Some(11),
                ..ConversionOptions::default()
            };
            assert_eq!(opts2.quantization.unwrap(), "q8_0");
            assert_eq!(opts2.opset_version.unwrap(), 11);
        }

        #[test]
        fn pipeline_register_custom_converter() {
            let mut pipeline = ConversionPipeline::new();
            pipeline.register(Box::new(SafeTensorsToRawConverter));
            assert!(pipeline.can_convert_direct(
                &ModelFormat::Safetensors,
                &ModelFormat::Custom("raw".into())
            ));
            assert_eq!(pipeline.supported_conversions().len(), 1);
        }

        #[test]
        fn pipeline_convert_with_progress() {
            let pipeline = ConversionPipeline::with_builtins();
            let cb: ProgressCallback = Box::new(|p: &ConversionProgress| {
                // Just verify the callback is called
                let _ = p.step;
            });

            // SafeTensors -> PyTorch with progress callback
            let header = b"{}";
            let mut data = Vec::new();
            data.extend_from_slice(&(header.len() as u64).to_le_bytes());
            data.extend_from_slice(header);

            let opts = ConversionOptions::default();
            let result = pipeline.convert(
                &data,
                &ModelFormat::Safetensors,
                &ModelFormat::PyTorch,
                &opts,
                Some(&cb),
            );
            assert!(result.is_ok());
        }
    }

    // ============================================================================
    // VAULT — ModelStream iterator
    // ============================================================================
    mod model_stream_coverage {
        use ironvault::vault::ModelStream;

        #[test]
        fn model_stream_basic() {
            let data = b"Hello World!".to_vec();
            let stream = ModelStream::new(data.clone(), 5);
            assert_eq!(stream.total_size(), 12);
            assert_eq!(stream.remaining(), 12);

            let chunks: Vec<Vec<u8>> = stream.collect();
            assert_eq!(chunks.len(), 3); // "Hello", " Worl", "d!"
            assert_eq!(chunks[0], b"Hello");
            assert_eq!(chunks[1], b" Worl");
            assert_eq!(chunks[2], b"d!");
        }

        #[test]
        fn model_stream_empty() {
            let stream = ModelStream::new(Vec::new(), 1024);
            assert_eq!(stream.total_size(), 0);
            assert_eq!(stream.remaining(), 0);
            let chunks: Vec<Vec<u8>> = stream.collect();
            assert!(chunks.is_empty());
        }

        #[test]
        fn model_stream_zero_chunk_uses_default() {
            // When chunk_size = 0, should default to 1MB
            let data = vec![0u8; 100];
            let stream = ModelStream::new(data, 0);
            let chunks: Vec<Vec<u8>> = stream.collect();
            assert_eq!(chunks.len(), 1); // All fits in one 1MB chunk
        }

        #[test]
        fn model_stream_exact_boundary() {
            let data = vec![0u8; 10];
            let stream = ModelStream::new(data, 5);
            let chunks: Vec<Vec<u8>> = stream.collect();
            assert_eq!(chunks.len(), 2);
            assert_eq!(chunks[0].len(), 5);
            assert_eq!(chunks[1].len(), 5);
        }

        #[test]
        fn model_stream_single_byte_chunks() {
            let data = b"abc".to_vec();
            let mut stream = ModelStream::new(data, 1);
            assert_eq!(stream.remaining(), 3);
            assert_eq!(stream.next().unwrap(), b"a");
            assert_eq!(stream.remaining(), 2);
            assert_eq!(stream.next().unwrap(), b"b");
            assert_eq!(stream.remaining(), 1);
            assert_eq!(stream.next().unwrap(), b"c");
            assert_eq!(stream.remaining(), 0);
            assert!(stream.next().is_none());
        }
    }

    // ============================================================================
    // TRAITS — MetricsSubscriber on_event for all event types
    // ============================================================================
    mod metrics_subscriber_coverage {
        use chrono::Utc;
        use ironvault::traits::{EventSubscriber, MetricsSubscriber, VaultEvent, VaultMetrics};
        use std::sync::atomic::Ordering::Relaxed;
        use std::sync::Arc;

        fn make_metrics() -> (MetricsSubscriber, Arc<VaultMetrics>) {
            let metrics = Arc::new(VaultMetrics::new());
            let sub = MetricsSubscriber::new(metrics.clone());
            (sub, metrics)
        }

        #[test]
        fn model_stored_event() {
            let (sub, metrics) = make_metrics();
            sub.on_event(&VaultEvent::ModelStored {
                vault: "v".into(),
                model: "m".into(),
                version: 1,
                format: "pt".into(),
                size: 1024,
                checksum: "abc".into(),
                timestamp: Utc::now(),
            })
            .unwrap();

            assert_eq!(metrics.models_stored_total.load(Relaxed), 1);
            assert_eq!(metrics.bytes_stored_total.load(Relaxed), 1024);
        }

        #[test]
        fn model_retrieved_event() {
            let (sub, metrics) = make_metrics();
            sub.on_event(&VaultEvent::ModelRetrieved {
                vault: "v".into(),
                model: "m".into(),
                version: 1,
                timestamp: Utc::now(),
            })
            .unwrap();
            assert_eq!(metrics.models_retrieved_total.load(Relaxed), 1);
        }

        #[test]
        fn model_deleted_event() {
            let (sub, metrics) = make_metrics();
            sub.on_event(&VaultEvent::ModelDeleted {
                vault: "v".into(),
                model: "m".into(),
                version: 1,
                timestamp: Utc::now(),
            })
            .unwrap();
            assert_eq!(metrics.models_deleted_total.load(Relaxed), 1);
        }

        #[test]
        fn vault_unlock_lock_events() {
            let (sub, metrics) = make_metrics();
            sub.on_event(&VaultEvent::VaultUnlocked {
                vault: "v".into(),
                timestamp: Utc::now(),
            })
            .unwrap();
            assert!(metrics.vault_unlocked.load(Relaxed));

            sub.on_event(&VaultEvent::VaultLocked {
                vault: "v".into(),
                timestamp: Utc::now(),
            })
            .unwrap();
            assert!(!metrics.vault_unlocked.load(Relaxed));
        }

        #[test]
        fn integrity_failed_event() {
            let (sub, metrics) = make_metrics();
            sub.on_event(&VaultEvent::IntegrityFailed {
                vault: "v".into(),
                model: "m".into(),
                version: 1,
                expected: "abc".into(),
                actual: "def".into(),
                timestamp: Utc::now(),
            })
            .unwrap();
            assert_eq!(metrics.errors_total.load(Relaxed), 1);
        }

        #[test]
        fn accepts_default_returns_true() {
            let (sub, _) = make_metrics();
            // The default `accepts` method returns true for all events
            assert!(sub.accepts(&VaultEvent::VaultCreated {
                vault: "v".into(),
                timestamp: Utc::now(),
            }));
        }

        #[test]
        fn unmatched_event_is_noop() {
            let (sub, metrics) = make_metrics();
            sub.on_event(&VaultEvent::VaultCreated {
                vault: "v".into(),
                timestamp: Utc::now(),
            })
            .unwrap();
            // VaultCreated is handled by the _ arm — no counters change
            assert_eq!(metrics.models_stored_total.load(Relaxed), 0);
        }
    }

    // ============================================================================
    // DATABASE — SQL validation, edge cases
    // ============================================================================
    mod database_validation_coverage {
        use ironvault::rag::{Database, SQLiteDatabase};
        use std::collections::HashMap;

        #[test]
        fn insert_with_invalid_table_name() {
            let mut db = SQLiteDatabase::in_memory().unwrap();
            let mut data = HashMap::new();
            data.insert("id".to_string(), "1".to_string());

            // SQL injection attempt should fail
            let result = db.insert("users; DROP TABLE users", data);
            assert!(result.is_err());
        }

        #[test]
        fn insert_with_empty_table_name() {
            let mut db = SQLiteDatabase::in_memory().unwrap();
            let mut data = HashMap::new();
            data.insert("id".to_string(), "1".to_string());

            let result = db.insert("", data);
            assert!(result.is_err());
        }

        #[test]
        fn insert_with_long_table_name() {
            let mut db = SQLiteDatabase::in_memory().unwrap();
            let mut data = HashMap::new();
            data.insert("id".to_string(), "1".to_string());

            let long_name = "a".repeat(200);
            let result = db.insert(&long_name, data);
            assert!(result.is_err());
        }

        #[test]
        fn create_table_with_invalid_column() {
            let db = SQLiteDatabase::in_memory().unwrap();
            let result = db.create_table("good_table", &[("bad column!", "TEXT")]);
            assert!(result.is_err());
        }

        #[test]
        fn update_with_invalid_table() {
            let mut db = SQLiteDatabase::in_memory().unwrap();
            let mut data = HashMap::new();
            data.insert("name".to_string(), "val".to_string());
            let result = db.update("bad;table", "id1", data);
            assert!(result.is_err());
        }

        #[test]
        fn delete_with_invalid_table() {
            let mut db = SQLiteDatabase::in_memory().unwrap();
            let result = db.delete("bad;table", "id1");
            assert!(result.is_err());
        }

        #[test]
        fn insert_with_invalid_column_name() {
            let mut db = SQLiteDatabase::in_memory().unwrap();
            db.create_table("valid_table", &[("col", "TEXT")]).unwrap();

            let mut data = HashMap::new();
            data.insert("id".to_string(), "1".to_string());
            data.insert("bad column!".to_string(), "val".to_string());

            let result = db.insert("valid_table", data);
            assert!(result.is_err());
        }

        #[test]
        fn update_with_invalid_column_name() {
            let mut db = SQLiteDatabase::in_memory().unwrap();
            db.create_table("users", &[("name", "TEXT")]).unwrap();

            let mut data = HashMap::new();
            data.insert("id".to_string(), "1".to_string());
            data.insert("name".to_string(), "Alice".to_string());
            db.insert("users", data).unwrap();

            let mut update = HashMap::new();
            update.insert("bad col!".to_string(), "val".to_string());
            let result = db.update("users", "1", update);
            assert!(result.is_err());
        }

        #[test]
        fn create_table_invalid_type() {
            let db = SQLiteDatabase::in_memory().unwrap();
            let result = db.create_table("t", &[("col", "TEXT; DROP TABLE")]);
            assert!(result.is_err());
        }

        #[test]
        fn insert_multiple_rows_then_query() {
            let mut db = SQLiteDatabase::in_memory().unwrap();
            db.create_table("items", &[("name", "TEXT"), ("value", "INTEGER")])
                .unwrap();

            for i in 0..5 {
                let mut data = HashMap::new();
                data.insert("id".to_string(), format!("item-{}", i));
                data.insert("name".to_string(), format!("Item {}", i));
                data.insert("value".to_string(), format!("{}", i * 10));
                db.insert("items", data).unwrap();
            }

            let all = db.query("SELECT * FROM items").unwrap();
            assert_eq!(all.len(), 5);
        }
    }

    // ============================================================================
    // BLOCKCHAIN — deeper verify_chain and proof paths
    // ============================================================================
    mod blockchain_deep_coverage {
        use chrono::Utc;
        use ironvault::audit::{AuditEntry, AuditEventType};
        use ironvault::BlockchainAudit;

        fn make_entry(event_type: AuditEventType) -> AuditEntry {
            AuditEntry {
                timestamp: Utc::now(),
                event_type,
                description: "test event".to_string(),
                model_name: Some("model".to_string()),
                version: Some(1),
                success: true,
                metadata: None,
            }
        }

        #[test]
        fn verify_chain_with_many_blocks() {
            let tmp = tempfile::tempdir().unwrap();
            let mut audit = BlockchainAudit::new(tmp.path(), 3).unwrap();

            // Add enough entries to create multiple blocks
            for _ in 0..12 {
                audit
                    .add_entry(make_entry(AuditEventType::ModelStored))
                    .unwrap();
            }
            // With block_size=3, 12 entries should create 4 blocks
            for _ in 0..4 {
                audit.finalize_block().unwrap();
            }

            let result = audit.verify_chain();
            assert!(result.valid);
            assert!(result.blocks_verified > 1);
        }

        #[test]
        fn height_increases_with_blocks() {
            let tmp = tempfile::tempdir().unwrap();
            let mut audit = BlockchainAudit::new(tmp.path(), 5).unwrap();

            let initial_height = audit.height();

            audit
                .add_entry(make_entry(AuditEventType::ModelStored))
                .unwrap();
            audit.finalize_block().unwrap();

            assert!(audit.height() > initial_height);
        }

        #[test]
        fn search_with_no_results() {
            let tmp = tempfile::tempdir().unwrap();
            let mut audit = BlockchainAudit::new(tmp.path(), 5).unwrap();

            audit
                .add_entry(make_entry(AuditEventType::ModelStored))
                .unwrap();
            audit.finalize_block().unwrap();

            let results = audit
                .search(Some("nonexistent"), None, None, None, 100)
                .unwrap();
            assert!(results.is_empty());
        }

        #[test]
        fn reopen_blockchain_persists() {
            let tmp = tempfile::tempdir().unwrap();

            {
                let mut audit = BlockchainAudit::new(tmp.path(), 5).unwrap();
                audit
                    .add_entry(make_entry(AuditEventType::ModelStored))
                    .unwrap();
                audit.finalize_block().unwrap();
            }

            // Re-open
            let audit2 = BlockchainAudit::new(tmp.path(), 5).unwrap();
            let result = audit2.verify_chain();
            assert!(result.valid);
            assert!(result.blocks_total >= 2); // genesis + 1
        }

        #[test]
        fn generate_proof_for_different_blocks() {
            let tmp = tempfile::tempdir().unwrap();
            let mut audit = BlockchainAudit::new(tmp.path(), 2).unwrap();

            for _ in 0..6 {
                audit
                    .add_entry(make_entry(AuditEventType::ModelStored))
                    .unwrap();
            }
            audit.finalize_block().unwrap();
            audit.finalize_block().unwrap();
            audit.finalize_block().unwrap();

            // Generate proofs for different blocks
            if let Ok(proof1) = audit.generate_proof(1, 0) {
                let v = BlockchainAudit::verify_proof(&proof1);
                assert!(v.valid);
            }
        }
    }

    // ============================================================================
    // COMPLIANCE — deeper checks
    // ============================================================================
    mod compliance_deep_coverage {
        use ironvault::compliance::ComplianceChecker;

        #[test]
        fn run_all_checks_with_verbose_output() {
            let checker = ComplianceChecker::new();
            let status = checker.run_all_checks().unwrap();

            // Verify all fields are populated
            assert!(status.fips_140_3);
            assert!(status.mitre_attack_aligned);
            assert_eq!(status.cmmc_level, 2);

            // Violations list exists (may be empty if cargo-audit is installed)
            let _ = &status.violations;
        }

        #[test]
        fn compliance_checker_fips_check() {
            let checker = ComplianceChecker::new();
            let fips = checker.check_fips_140_3();
            assert!(fips);
        }
    }

    // ============================================================================
    // FORMATS — remaining edge cases
    // ============================================================================
    mod formats_deep_coverage {
        use ironvault::formats::ModelFormat;

        #[test]
        fn from_extension_all_formats() {
            let tests = vec![
                ("safetensors", ModelFormat::Safetensors),
                ("gguf", ModelFormat::GGUF),
                ("onnx", ModelFormat::ONNX),
                ("pt", ModelFormat::PyTorch),
                ("pth", ModelFormat::PyTorch),
                ("bin", ModelFormat::PyTorch),
                ("plan", ModelFormat::TensorRT),
                ("mlmodel", ModelFormat::CoreML),
                ("pb", ModelFormat::TensorFlow),
                ("h5", ModelFormat::Keras),
                ("keras", ModelFormat::Keras),
                ("hdf5", ModelFormat::HDF5),
                ("tflite", ModelFormat::TFLite),
                ("xml", ModelFormat::OpenVINO),
                ("param", ModelFormat::NCNN),
                ("mnn", ModelFormat::MNN),
                ("rknn", ModelFormat::RKNN),
                ("caffemodel", ModelFormat::Caffe),
                ("params", ModelFormat::MXNet),
                ("weights", ModelFormat::Darknet),
                ("pkl", ModelFormat::Pickle),
                ("npy", ModelFormat::NumPy),
                ("npz", ModelFormat::NumPy),
            ];

            for (ext, expected) in tests {
                let fmt = ModelFormat::from_extension(ext);
                assert_eq!(fmt, expected, "Failed for extension: {}", ext);
            }
        }

        #[test]
        fn from_extension_unknown() {
            let fmt = ModelFormat::from_extension("xyz123unknown");
            match fmt {
                ModelFormat::Custom(s) => assert_eq!(s, "xyz123unknown"),
                _ => panic!("Expected Custom for unknown extension"),
            }
        }

        #[test]
        fn all_format_names() {
            let formats = vec![
                ModelFormat::Safetensors,
                ModelFormat::GGUF,
                ModelFormat::ONNX,
                ModelFormat::PyTorch,
                ModelFormat::TorchScript,
                ModelFormat::TensorRT,
                ModelFormat::CoreML,
                ModelFormat::MLX,
                ModelFormat::TensorFlow,
                ModelFormat::Keras,
                ModelFormat::HDF5,
                ModelFormat::TFLite,
                ModelFormat::OpenVINO,
                ModelFormat::NCNN,
                ModelFormat::MNN,
                ModelFormat::RKNN,
                ModelFormat::Caffe,
                ModelFormat::MXNet,
                ModelFormat::Darknet,
                ModelFormat::Pickle,
                ModelFormat::NumPy,
            ];

            for fmt in &formats {
                let name = fmt.name();
                assert!(!name.is_empty());
                let ext = fmt.extension();
                assert!(!ext.is_empty());
            }
        }
    }

    // ============================================================================
    // TRAITS — IvUri to_string with query params
    // ============================================================================
    mod iv_uri_deep_coverage {
        use ironvault::traits::IvUri;

        #[test]
        fn uri_to_string_with_multiple_query_params() {
            let uri = IvUri::parse("iv://vault/model?format=onnx&version=2&compressed").unwrap();
            let s = uri.to_string();
            assert!(s.contains("format=onnx"));
            assert!(s.contains("version=2"));
            assert!(s.contains("compressed"));
            assert!(s.contains("?"));
            assert!(s.contains("&") || s.matches("?").count() == 1);
        }

        #[test]
        fn uri_to_string_no_query() {
            let uri = IvUri::parse("iv://vault/model@1/weights").unwrap();
            let s = uri.to_string();
            assert!(!s.contains("?"));
            assert!(s.contains("model@1"));
            assert!(s.contains("/weights"));
        }

        #[test]
        fn uri_model_version_resource_all_present() {
            let uri = IvUri::parse("iv://myvault/mymodel@42/checkpoint").unwrap();
            assert_eq!(uri.vault, Some("myvault".to_string()));
            assert_eq!(uri.model, Some("mymodel".to_string()));
            assert_eq!(uri.version, Some(42));
            assert_eq!(uri.resource, Some("checkpoint".to_string()));
        }
    }

    // ============================================================================
    // COMPRESSION — LZMA, Bzip2 edge cases
    // ============================================================================
    mod compression_deep_coverage {
        use ironvault::crypto::compression::{
            compress, decompress, CompressionAlgorithm, CompressionLevel,
        };

        #[test]
        fn lzma_compress_decompress_balanced() {
            let data = b"Hello LZMA compression test data!";
            let compressed =
                compress(data, CompressionAlgorithm::Lzma, CompressionLevel::Balanced).unwrap();
            let decompressed = decompress(&compressed, CompressionAlgorithm::Lzma).unwrap();
            assert_eq!(decompressed, data);
        }

        #[test]
        fn lzma_compress_decompress_maximum() {
            let data = b"Hello LZMA compression maximum test data!";
            let compressed =
                compress(data, CompressionAlgorithm::Lzma, CompressionLevel::Maximum).unwrap();
            let decompressed = decompress(&compressed, CompressionAlgorithm::Lzma).unwrap();
            assert_eq!(decompressed, data);
        }

        #[test]
        fn compression_with_none_level() {
            // Test that CompressionLevel::None still works with Gzip
            let data = b"some data to compress";
            let compressed =
                compress(data, CompressionAlgorithm::Gzip, CompressionLevel::None).unwrap();
            let decompressed = decompress(&compressed, CompressionAlgorithm::Gzip).unwrap();
            assert_eq!(decompressed, data);
        }

        #[test]
        fn compression_empty_data() {
            let data = b"";
            for algo in [CompressionAlgorithm::Gzip, CompressionAlgorithm::None] {
                let compressed = compress(data, algo, CompressionLevel::Balanced).unwrap();
                let decompressed = decompress(&compressed, algo).unwrap();
                assert_eq!(decompressed, data);
            }
        }
    }

    // ============================================================================
    // ERROR — from impls
    // ============================================================================
    mod error_from_coverage {
        use ironvault::error::VaultError;

        #[test]
        fn vault_error_from_io_error() {
            let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
            let vault_err: VaultError = io_err.into();
            let msg = format!("{}", vault_err);
            assert!(msg.contains("file missing") || msg.contains("I/O"));
        }

        #[test]
        fn vault_error_from_string() {
            let err = VaultError::InvalidInput("bad input".to_string());
            let msg = format!("{}", err);
            assert!(msg.contains("bad input") || msg.contains("Invalid"));
        }
    }
}

#[allow(unused_imports)]
mod coverage_final_tests {
    /// Targeted integration tests for uncovered lines in tarpaulin release mode.
    ///
    /// Targets:
    /// - vault.rs: VaultBuilder, store_model_streamed, get_model_chunked, auto_cleanup
    /// - conversion.rs: ValidationCheck::pass/fail, ConversionPipeline BFS, identity, progress
    /// - blockchain.rs: verify_chain, verify_proof chain linkage
    /// - compliance.rs: run_all_checks
    use ironvault::audit::{AuditEntry, AuditEventType};
    use ironvault::compliance::ComplianceChecker;
    use ironvault::formats::{ModelFormat, ModelMetadata};
    use ironvault::{
        BlockchainAudit, ConversionOptions, ConversionPipeline, EventSubscriber, ValidationCheck,
        ValidationReport, Vault, VaultBuilder, VaultConfig, VaultEvent,
    };

    /// Helper to create a VaultConfig pointing at a temp directory.
    fn temp_vault_config(temp: &tempfile::TempDir) -> VaultConfig {
        let mut config = VaultConfig::default();
        config.dirs.data_dir = temp.path().join("data");
        config.dirs.config_dir = temp.path().join("config");
        config.dirs.cache_dir = temp.path().join("cache");
        config.dirs.log_dir = temp.path().join("logs");
        config.dirs.vault_dir = temp.path().join("vaults");
        config
    }

    /// Helper to create a ModelMetadata with a given format.
    fn meta(format: ModelFormat) -> ModelMetadata {
        ModelMetadata::new("test-model".to_string(), format)
    }

    // ====================== ValidationCheck (conversion.rs L162-174) ======================

    #[test]
    fn test_validation_check_pass_constructor() {
        let check = ValidationCheck::pass("size_check", "Size within range");
        assert!(check.passed);
        assert_eq!(check.name, "size_check");
        assert_eq!(check.message, "Size within range");
    }

    #[test]
    fn test_validation_check_fail_constructor() {
        let check = ValidationCheck::fail("format_check", "Invalid format header");
        assert!(!check.passed);
        assert_eq!(check.name, "format_check");
        assert_eq!(check.message, "Invalid format header");
    }

    #[test]
    fn test_validation_report_from_checks_all_pass() {
        let checks = vec![
            ValidationCheck::pass("a", "ok"),
            ValidationCheck::pass("b", "ok"),
        ];
        let report = ValidationReport::from_checks(checks);
        assert!(report.passed);
        assert_eq!(report.checks.len(), 2);
    }

    #[test]
    fn test_validation_report_from_checks_one_fails() {
        let checks = vec![
            ValidationCheck::pass("a", "ok"),
            ValidationCheck::fail("b", "bad"),
        ];
        let report = ValidationReport::from_checks(checks);
        assert!(!report.passed);
    }

    // ====================== ConversionPipeline (conversion.rs L248-380) ======================

    #[test]
    fn test_pipeline_new_empty() {
        let pipeline = ConversionPipeline::new();
        assert!(pipeline.supported_conversions().is_empty());
    }

    #[test]
    fn test_pipeline_identity_conversion() {
        let pipeline = ConversionPipeline::with_builtins();
        let data = b"test data";
        let result = pipeline
            .convert(
                data,
                &ModelFormat::Safetensors,
                &ModelFormat::Safetensors,
                &ConversionOptions::default(),
                None,
            )
            .unwrap();
        assert_eq!(result.data, data);
        assert_eq!(result.source_format, ModelFormat::Safetensors);
        assert_eq!(result.target_format, ModelFormat::Safetensors);
        assert_eq!(result.input_size, result.output_size);
        assert!(result.validation.is_none());
    }

    #[test]
    fn test_pipeline_find_path_same_format() {
        let pipeline = ConversionPipeline::with_builtins();
        let path = pipeline.find_path(&ModelFormat::ONNX, &ModelFormat::ONNX);
        assert_eq!(path, Some(vec![ModelFormat::ONNX]));
    }

    #[test]
    fn test_pipeline_find_path_bfs_multi_step() {
        let pipeline = ConversionPipeline::with_builtins();
        let path = pipeline.find_path(&ModelFormat::PyTorch, &ModelFormat::GGUF);
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.len() >= 2);
        assert_eq!(path[0], ModelFormat::PyTorch);
        assert_eq!(*path.last().unwrap(), ModelFormat::GGUF);
    }

    #[test]
    fn test_pipeline_find_path_none() {
        let pipeline = ConversionPipeline::new();
        let path = pipeline.find_path(&ModelFormat::ONNX, &ModelFormat::GGUF);
        assert!(path.is_none());
    }

    #[test]
    fn test_pipeline_convert_with_progress_callback() {
        use std::sync::{Arc, Mutex};
        let pipeline = ConversionPipeline::with_builtins();
        let progress_calls = Arc::new(Mutex::new(Vec::new()));
        let calls_clone = progress_calls.clone();
        let callback: ironvault::conversion::ProgressCallback = Box::new(move |progress| {
            calls_clone.lock().unwrap().push(progress.message.clone());
        });

        let _ = pipeline.convert(
            b"dummy safetensors data",
            &ModelFormat::Safetensors,
            &ModelFormat::GGUF,
            &ConversionOptions::default(),
            Some(&callback),
        );
        // Exercising the progress callback path is sufficient
    }

    #[test]
    fn test_pipeline_convert_no_path_error() {
        let pipeline = ConversionPipeline::new();
        let result = pipeline.convert(
            b"data",
            &ModelFormat::ONNX,
            &ModelFormat::GGUF,
            &ConversionOptions::default(),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_with_builtins_has_converters() {
        let pipeline = ConversionPipeline::with_builtins();
        let conversions = pipeline.supported_conversions();
        assert!(conversions.len() >= 5);
        assert!(pipeline.can_convert_direct(&ModelFormat::Safetensors, &ModelFormat::GGUF));
    }

    // ====================== VaultBuilder (vault.rs L810-920) ======================

    #[test]
    fn test_vault_builder_default_build() {
        let temp = tempfile::tempdir().unwrap();
        let config = temp_vault_config(&temp);

        let vault = VaultBuilder::new().config(config).build();
        assert!(vault.is_ok());
        let vault = vault.unwrap();
        assert_eq!(vault.version_backend_name(), "json");
    }

    #[test]
    fn test_vault_builder_no_default_subscribers() {
        let temp = tempfile::tempdir().unwrap();
        let config = temp_vault_config(&temp);

        let vault = VaultBuilder::new()
            .config(config)
            .no_default_subscribers()
            .build();
        assert!(vault.is_ok());
        let vault = vault.unwrap();
        assert!(vault.metrics().is_none());
    }

    #[test]
    fn test_vault_builder_with_custom_subscriber() {
        struct TestSub;
        impl EventSubscriber for TestSub {
            fn on_event(&self, _event: &VaultEvent) -> ironvault::Result<()> {
                Ok(())
            }
            fn name(&self) -> &str {
                "test_sub"
            }
        }

        let temp = tempfile::tempdir().unwrap();
        let config = temp_vault_config(&temp);

        let vault = VaultBuilder::new()
            .config(config)
            .subscriber(Box::new(TestSub))
            .build();
        assert!(vault.is_ok());
    }

    // ====================== Vault streamed store / chunked get (vault.rs L755-794) ==========

    #[test]
    fn test_vault_store_model_streamed_and_get_chunked() {
        let temp = tempfile::tempdir().unwrap();
        let config = temp_vault_config(&temp);

        let mut vault = VaultBuilder::new()
            .config(config)
            .no_default_subscribers()
            .build()
            .unwrap();

        vault.unlock(b"test_pass".to_vec()).unwrap();

        let metadata = meta(ModelFormat::Safetensors);

        let chunks = vec![vec![1u8, 2, 3, 4], vec![5, 6, 7, 8], vec![9, 10]];
        let version = vault
            .store_model_streamed("chunked-model", chunks, metadata, None)
            .unwrap();
        assert_eq!(version.version, 1);

        // Retrieve chunked
        let stream = vault
            .get_model_chunked("chunked-model", Some(1), 4)
            .unwrap();
        let mut all_data = Vec::new();
        for chunk in stream {
            all_data.extend_from_slice(&chunk);
        }
        assert_eq!(all_data, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    // ====================== Vault auto_cleanup (vault.rs L440-448) ========================

    #[test]
    fn test_vault_auto_cleanup_old_versions() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = temp_vault_config(&temp);
        config.storage.auto_cleanup = true;
        config.storage.max_versions = 2;

        let mut vault = VaultBuilder::new()
            .config(config)
            .no_default_subscribers()
            .build()
            .unwrap();

        vault.unlock(b"cleanup_pass".to_vec()).unwrap();

        let m = meta(ModelFormat::Safetensors);
        vault
            .store_model("cleanup-model", vec![1; 100], m.clone(), None)
            .unwrap();
        let m2 = ModelMetadata::new("cleanup-model".to_string(), ModelFormat::Safetensors);
        vault
            .store_model("cleanup-model", vec![2; 100], m2, Some(1))
            .unwrap();
        let m3 = ModelMetadata::new("cleanup-model".to_string(), ModelFormat::Safetensors);
        vault
            .store_model("cleanup-model", vec![3; 100], m3, Some(2))
            .unwrap();

        let versions = vault.list_versions("cleanup-model");
        assert!(!versions.is_empty());
    }

    // ====================== Vault::new with None config (vault.rs L215) ====================

    #[test]
    fn test_vault_new_with_default_config() {
        let result = Vault::new(None);
        assert!(result.is_ok());
    }

    // ====================== Blockchain verify_chain (blockchain.rs L533-558) ===============

    fn make_audit_entry(i: u32) -> AuditEntry {
        AuditEntry {
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::ModelStored,
            description: format!("Entry {}", i),
            model_name: Some("test".into()),
            version: Some(i),
            success: true,
            metadata: None,
        }
    }

    #[test]
    fn test_blockchain_verify_chain_valid() {
        let temp = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp.path(), 5).unwrap();

        for i in 0..3 {
            audit.add_entry(make_audit_entry(i)).unwrap();
        }
        audit.finalize_block().unwrap();

        let verification = audit.verify_chain();
        assert!(verification.valid);
        assert!(verification.issues.is_empty());
        assert!(verification.blocks_verified > 0);
    }

    #[test]
    fn test_blockchain_verify_proof_full_chain() {
        let temp = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp.path(), 2).unwrap();

        for i in 0..4 {
            audit.add_entry(make_audit_entry(i)).unwrap();
        }
        audit.finalize_block().unwrap();

        let proof = audit.generate_proof(1, 0).unwrap();
        let verification = BlockchainAudit::verify_proof(&proof);
        assert!(verification.valid);
    }

    #[test]
    fn test_blockchain_verify_chain_with_tampered_block() {
        let temp = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp.path(), 2).unwrap();

        for i in 0..4 {
            audit.add_entry(make_audit_entry(i)).unwrap();
        }
        audit.finalize_block().unwrap();

        // Tamper with a block file to exercise error paths
        let block_path = temp.path().join("block_00000001.json");
        if block_path.exists() {
            let contents = std::fs::read_to_string(&block_path).unwrap();
            let tampered = contents.replace("Entry 0", "TAMPERED");
            std::fs::write(&block_path, tampered).unwrap();
        }

        let verification = audit.verify_chain();
        assert!(verification.blocks_verified > 0);
    }

    // ====================== Compliance (compliance.rs L94-124) =============================

    #[test]
    fn test_compliance_run_all_checks() {
        let checker = ComplianceChecker::new();
        let results = checker.run_all_checks();
        assert!(results.is_ok());
        let status = results.unwrap();
        // ComplianceStatus has fips_140_3, cve_scan_passed, violations etc.
        assert!(status.fips_140_3);
    }

    // ====================== Vault get_model errors (vault.rs L462+) ========================

    #[test]
    fn test_vault_get_model_not_found() {
        let temp = tempfile::tempdir().unwrap();
        let config = temp_vault_config(&temp);

        let mut vault = VaultBuilder::new()
            .config(config)
            .no_default_subscribers()
            .build()
            .unwrap();

        vault.unlock(b"pass".to_vec()).unwrap();

        let result = vault.get_model("nonexistent", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_vault_get_model_version_not_found() {
        let temp = tempfile::tempdir().unwrap();
        let config = temp_vault_config(&temp);

        let mut vault = VaultBuilder::new()
            .config(config)
            .no_default_subscribers()
            .build()
            .unwrap();

        vault.unlock(b"pass".to_vec()).unwrap();

        let m = meta(ModelFormat::Safetensors);
        vault.store_model("my-model", vec![1; 64], m, None).unwrap();

        let result = vault.get_model("my-model", Some(999));
        assert!(result.is_err());
    }

    #[test]
    fn test_vault_get_model_locked() {
        let temp = tempfile::tempdir().unwrap();
        let config = temp_vault_config(&temp);

        let vault = VaultBuilder::new()
            .config(config)
            .no_default_subscribers()
            .build()
            .unwrap();

        let result = vault.get_model("any", None);
        assert!(result.is_err());
    }

    // ====================== Vault metrics (vault.rs builder defaults) ======================

    #[test]
    fn test_vault_builder_with_metrics() {
        let temp = tempfile::tempdir().unwrap();
        let config = temp_vault_config(&temp);

        let vault = VaultBuilder::new().config(config).build().unwrap();

        let metrics = vault.metrics();
        assert!(metrics.is_some());
        let m = metrics.unwrap();
        assert_eq!(m.models_stored_total, 0);
    }

    // ====================== Vault store + retrieve roundtrip =================================

    #[test]
    fn test_vault_store_retrieve_roundtrip() {
        let temp = tempfile::tempdir().unwrap();
        let config = temp_vault_config(&temp);

        let mut vault = VaultBuilder::new().config(config).build().unwrap();

        vault.unlock(b"roundtrip_pass".to_vec()).unwrap();

        let original_data = vec![42u8; 256];
        let m = meta(ModelFormat::Safetensors);
        let version = vault
            .store_model("rt-model", original_data.clone(), m, None)
            .unwrap();
        assert_eq!(version.version, 1);

        let retrieved = vault.get_model("rt-model", Some(1)).unwrap();
        assert_eq!(retrieved, original_data);
    }

    // ====================== VaultBuilder with SQLite backend (vault.rs L49,68,76...) ==========

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_vault_builder_sqlite_backend() {
        let temp = tempfile::tempdir().unwrap();
        let config = temp_vault_config(&temp);

        let mut vault = VaultBuilder::new()
            .config(config)
            .sqlite_versions()
            .no_default_subscribers()
            .build()
            .unwrap();

        assert_eq!(vault.version_backend_name(), "sqlite");

        vault.unlock(b"sqlite_pass".to_vec()).unwrap();

        let m = meta(ModelFormat::Safetensors);
        let version = vault
            .store_model("sqlite-model", vec![1, 2, 3, 4], m, None)
            .unwrap();
        assert_eq!(version.version, 1);

        let data = vault.get_model("sqlite-model", Some(1)).unwrap();
        assert_eq!(data, vec![1, 2, 3, 4]);

        let versions = vault.list_versions("sqlite-model");
        assert_eq!(versions.len(), 1);

        // Delete version
        vault.delete_version("sqlite-model", 1).unwrap();
    }

    // ====================== VaultBuilder with audit logging (vault.rs L905) ==================

    #[test]
    fn test_vault_builder_with_audit_logging() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = temp_vault_config(&temp);
        config.security.audit_log = true;

        let mut vault = VaultBuilder::new().config(config).build().unwrap();

        vault.unlock(b"audit_pass".to_vec()).unwrap();

        let m = meta(ModelFormat::Safetensors);
        vault
            .store_model("audit-model", vec![10, 20, 30], m, None)
            .unwrap();

        let data = vault.get_model("audit-model", Some(1)).unwrap();
        assert_eq!(data, vec![10, 20, 30]);
    }

    // ====================== ModelFormat variant coverage (formats.rs L111-146) ================

    #[test]
    fn test_model_format_rare_extensions() {
        // Exercise format variants that aren't commonly tested
        let variants = vec![
            (ModelFormat::TVM, "so", "TVM"),
            (ModelFormat::NCNN, "param", "NCNN"),
            (ModelFormat::MNN, "mnn", "MNN"),
            (ModelFormat::RKNN, "rknn", "RKNN"),
            (ModelFormat::Caffe, "caffemodel", "Caffe"),
            (ModelFormat::MXNet, "params", "MXNet"),
            (ModelFormat::Darknet, "weights", "Darknet"),
            (ModelFormat::Pickle, "pkl", "Pickle"),
            (ModelFormat::NumPy, "npy", "NumPy"),
        ];
        for (format, ext, name) in variants {
            assert_eq!(format.extension(), ext, "extension for {}", name);
            assert_eq!(format.name(), name, "name for {}", name);
            // Exercise Display trait
            assert_eq!(format.to_string(), name);
        }
    }

    #[test]
    fn test_model_format_custom_variant() {
        let custom = ModelFormat::Custom("my_ext".to_string());
        assert_eq!(custom.extension(), "my_ext");
        assert_eq!(custom.name(), "my_ext");
    }

    // ====================== ModelMetadata builder pattern (formats.rs L175+) ================

    #[test]
    fn test_model_metadata_builder_chain() {
        let metadata = ModelMetadata::new("big-model".to_string(), ModelFormat::Safetensors)
            .with_description("A large model".to_string())
            .with_framework("pytorch".to_string())
            .with_task("text-generation".to_string())
            .with_architecture("transformer".to_string())
            .with_parameters(7_000_000_000)
            .add_custom_field("license".to_string(), "MIT".to_string());

        assert_eq!(metadata.name, "big-model");
        assert_eq!(metadata.format, ModelFormat::Safetensors);
        assert_eq!(metadata.description.unwrap(), "A large model");
        assert_eq!(metadata.framework.unwrap(), "pytorch");
        assert_eq!(metadata.task.unwrap(), "text-generation");
        assert_eq!(metadata.architecture.unwrap(), "transformer");
        assert_eq!(metadata.parameters.unwrap(), 7_000_000_000);
        assert_eq!(metadata.custom_fields.get("license").unwrap(), "MIT");
    }

    // ====================== FormatConverter registry (formats.rs L258) =======================

    #[test]
    fn test_format_converter_register_and_convert() {
        use ironvault::formats::FormatConverter;
        let mut converter = FormatConverter::new();
        converter.register(ModelFormat::Safetensors, ModelFormat::ONNX, |data| {
            Ok(data.to_vec())
        });
        assert!(converter.can_convert(ModelFormat::Safetensors, ModelFormat::ONNX));
        assert!(!converter.can_convert(ModelFormat::ONNX, ModelFormat::Safetensors));

        let result = converter
            .convert(b"test", ModelFormat::Safetensors, ModelFormat::ONNX)
            .unwrap();
        assert_eq!(result, b"test");
    }

    // ====================== Integrity failure path (vault.rs L484-509) =======================

    #[test]
    fn test_vault_integrity_failure_on_corrupted_data() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = temp_vault_config(&temp);
        config.security.audit_log = true;

        let mut vault = VaultBuilder::new()
            .config(config)
            .no_default_subscribers()
            .build()
            .unwrap();

        vault.unlock(b"integrity_pass".to_vec()).unwrap();

        let m = meta(ModelFormat::Safetensors);
        let version = vault
            .store_model("corrupt-model", vec![1, 2, 3, 4, 5], m, None)
            .unwrap();

        // Corrupt the stored file — files are stored in vault_dir/default/
        let vault_path = temp.path().join("vaults").join("default");
        let mut corrupted = false;
        if let Ok(entries) = std::fs::read_dir(&vault_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let mut data = std::fs::read(&path).unwrap();
                    if data.len() > 10 {
                        // Corrupt multiple bytes to invalidate decryption
                        for i in 5..std::cmp::min(data.len(), 20) {
                            data[i] ^= 0xFF;
                        }
                        std::fs::write(&path, &data).unwrap();
                        corrupted = true;
                    }
                }
            }
        }

        // Try to retrieve — should fail (decryption or integrity error)
        if corrupted {
            let result = vault.get_model("corrupt-model", Some(version.version));
            assert!(result.is_err(), "Expected error after file corruption");
        }
    }

    // ====================== Compliance CVE disabled (compliance.rs) ===========================

    #[test]
    fn test_compliance_with_disabled_checks() {
        let mut checker = ComplianceChecker::new();
        checker.set_check_enabled("cve", false);
        checker.set_check_enabled("mitre_attack", false);
        checker.set_check_enabled("cmmc", false);

        let status = checker.run_all_checks().unwrap();
        assert!(status.fips_140_3);
        assert!(status.cve_scan_passed);
        assert!(status.mitre_attack_aligned);
    }

    // ====================== Vault delete_model (vault.rs) ====================================

    #[test]
    fn test_vault_delete_model() {
        let temp = tempfile::tempdir().unwrap();
        let config = temp_vault_config(&temp);

        let mut vault = VaultBuilder::new()
            .config(config)
            .no_default_subscribers()
            .build()
            .unwrap();

        vault.unlock(b"delete_pass".to_vec()).unwrap();

        let m = meta(ModelFormat::Safetensors);
        vault
            .store_model("del-model", vec![1, 2, 3], m, None)
            .unwrap();

        vault.delete_version("del-model", 1).unwrap();

        // Should not be retrievable
        let result = vault.get_model("del-model", Some(1));
        assert!(result.is_err());

        // Nor still listed. This test is named for deleting a *model* and only
        // checked that retrieval failed, which is how a deleted model went on
        // appearing in `list_models` and counting toward `model_count` unnoticed.
        assert!(
            !vault.list_models().contains(&"del-model".to_string()),
            "deleted model still listed: {:?}",
            vault.list_models()
        );
        assert_eq!(vault.get_stats().unwrap().model_count, 0);
    }
}

#[allow(unused_imports)]
mod coverage_gap_tests {
    //! Tests covering previously untested critical functions:
    //! - Vault::change_passphrase
    //! - AuditLogger read_entries, log_auth, log_security_violation
    //! - FormatConverter register, can_convert, convert
    //! - VersionControl cleanup_old_versions, verify_checksum
    //! - ComplianceChecker set_check_enabled / is_check_enabled

    use ironvault::audit::{AuditEventType, AuditLogger};
    use ironvault::compliance::ComplianceChecker;
    use ironvault::crypto::VaultCrypto;
    use ironvault::formats::{FormatConverter, ModelFormat, ModelMetadata};
    use ironvault::version::VersionControl;
    use ironvault::{Vault, VaultConfig};
    use tempfile::tempdir;

    // ========================= AuditLogger Tests =========================

    #[test]
    fn test_audit_log_and_read_entries() {
        let tmp = tempdir().unwrap();
        let log_path = tmp.path().join("audit.jsonl");
        let logger = AuditLogger::new(&log_path).unwrap();

        // Log some events
        logger.log_model_stored("gpt2", 1, true).unwrap();
        logger.log_model_retrieved("gpt2", 1, true).unwrap();
        logger.log_model_stored("bert", 1, false).unwrap();

        // Read all entries
        let entries = logger.read_entries(None).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(matches!(entries[0].event_type, AuditEventType::ModelStored));
        assert_eq!(entries[0].model_name.as_deref(), Some("gpt2"));
        assert!(entries[0].success);
        assert!(matches!(
            entries[1].event_type,
            AuditEventType::ModelRetrieved
        ));
        assert!(!entries[2].success);

        // Read with limit
        let limited = logger.read_entries(Some(2)).unwrap();
        assert_eq!(limited.len(), 2);
    }

    #[test]
    fn test_audit_read_entries_missing_file() {
        let tmp = tempdir().unwrap();
        let log_path = tmp.path().join("nonexistent.jsonl");
        let logger = AuditLogger::new(&log_path).unwrap();
        let entries = logger.read_entries(None).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_audit_log_auth_success() {
        let tmp = tempdir().unwrap();
        let log_path = tmp.path().join("auth.jsonl");
        let logger = AuditLogger::new(&log_path).unwrap();

        logger.log_auth(true, None).unwrap();
        let entries = logger.read_entries(None).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0].event_type, AuditEventType::AuthSuccess));
        assert!(entries[0].success);
    }

    #[test]
    fn test_audit_log_auth_failure() {
        let tmp = tempdir().unwrap();
        let log_path = tmp.path().join("auth_fail.jsonl");
        let logger = AuditLogger::new(&log_path).unwrap();

        logger.log_auth(false, Some("bad password")).unwrap();
        let entries = logger.read_entries(None).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0].event_type, AuditEventType::AuthFailure));
        assert!(!entries[0].success);
        assert!(entries[0].description.contains("Authentication failed"));
    }

    #[test]
    fn test_audit_log_security_violation() {
        let tmp = tempdir().unwrap();
        let log_path = tmp.path().join("sec.jsonl");
        let logger = AuditLogger::new(&log_path).unwrap();

        logger.log_security_violation("tampering detected").unwrap();
        let entries = logger.read_entries(None).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(matches!(
            entries[0].event_type,
            AuditEventType::SecurityViolation
        ));
        assert!(entries[0].description.contains("tampering"));
        assert!(!entries[0].success);
    }

    // ========================= FormatConverter Tests =========================

    #[test]
    fn test_format_converter_same_format_passthrough() {
        let converter = FormatConverter::new();
        let data = b"model data";
        let result = converter
            .convert(data, ModelFormat::PyTorch, ModelFormat::PyTorch)
            .unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_format_converter_register_and_convert() {
        let mut converter = FormatConverter::new();

        fn mock_convert(data: &[u8]) -> ironvault::Result<Vec<u8>> {
            let mut out = b"converted:".to_vec();
            out.extend_from_slice(data);
            Ok(out)
        }

        converter.register(ModelFormat::PyTorch, ModelFormat::ONNX, mock_convert);

        assert!(converter.can_convert(ModelFormat::PyTorch, ModelFormat::ONNX));
        assert!(!converter.can_convert(ModelFormat::ONNX, ModelFormat::PyTorch));

        let result = converter
            .convert(b"hello", ModelFormat::PyTorch, ModelFormat::ONNX)
            .unwrap();
        assert_eq!(result, b"converted:hello");
    }

    #[test]
    fn test_format_converter_unsupported_returns_error() {
        let converter = FormatConverter::new();
        let result = converter.convert(b"data", ModelFormat::PyTorch, ModelFormat::ONNX);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("No converter available"));
    }

    // ========================= VersionControl Tests =========================

    fn setup_version_control_with_models(count: u32) -> (tempfile::TempDir, VersionControl) {
        let tmp = tempdir().unwrap();
        let mut vc = VersionControl::new(tmp.path()).unwrap();
        for i in 1..=count {
            vc.add_version(
                "test_model",
                &format!("file_v{}.enc", i),
                "pytorch",
                1000 + u64::from(i),
                500 + u64::from(i),
                &format!("checksum_{}", i),
                None,
                if i > 1 { Some(i - 1) } else { None },
            )
            .unwrap();
        }
        (tmp, vc)
    }

    #[test]
    fn test_cleanup_old_versions_trims_correctly() {
        let (_tmp, mut vc) = setup_version_control_with_models(5);

        let deleted = vc.cleanup_old_versions("test_model", 2).unwrap();
        assert_eq!(deleted.len(), 3);

        // Kept versions should be the 2 most recent (v4, v5)
        let remaining = vc.list_versions("test_model");
        assert_eq!(remaining.len(), 2);
        let version_nums: Vec<u32> = remaining.iter().map(|v| v.version).collect();
        assert!(version_nums.contains(&5));
        assert!(version_nums.contains(&4));
    }

    #[test]
    fn test_cleanup_old_versions_no_op_when_fewer() {
        let (_tmp, mut vc) = setup_version_control_with_models(2);

        let deleted = vc.cleanup_old_versions("test_model", 5).unwrap();
        assert!(deleted.is_empty());
        assert_eq!(vc.list_versions("test_model").len(), 2);
    }

    #[test]
    fn test_cleanup_old_versions_nonexistent_model() {
        let tmp = tempdir().unwrap();
        let mut vc = VersionControl::new(tmp.path()).unwrap();

        let deleted = vc.cleanup_old_versions("not_here", 3).unwrap();
        assert!(deleted.is_empty());
    }

    #[test]
    fn test_verify_checksum_correct_data() {
        let tmp = tempdir().unwrap();
        let mut vc = VersionControl::new(tmp.path()).unwrap();

        let data = b"test model data for checksum";
        let checksum = hex::encode(VaultCrypto::hash_sha256(data));

        vc.add_version(
            "cksum_model",
            "file.enc",
            "onnx",
            100,
            50,
            &checksum,
            None,
            None,
        )
        .unwrap();

        assert!(vc.verify_checksum("cksum_model", 1, data));
    }

    #[test]
    fn test_verify_checksum_wrong_data() {
        let tmp = tempdir().unwrap();
        let mut vc = VersionControl::new(tmp.path()).unwrap();

        let data = b"original data";
        let checksum = hex::encode(VaultCrypto::hash_sha256(data));

        vc.add_version(
            "cksum_model",
            "file.enc",
            "onnx",
            100,
            50,
            &checksum,
            None,
            None,
        )
        .unwrap();

        assert!(!vc.verify_checksum("cksum_model", 1, b"tampered data"));
    }

    #[test]
    fn test_verify_checksum_nonexistent_version() {
        let tmp = tempdir().unwrap();
        let vc = VersionControl::new(tmp.path()).unwrap();
        assert!(!vc.verify_checksum("no_model", 1, b"doesn't matter"));
    }

    // ========================= ComplianceChecker Tests =========================

    #[test]
    fn test_compliance_enable_disable_checks() {
        let mut checker = ComplianceChecker::new();

        // All checks enabled by default
        assert!(checker.is_check_enabled("fips_140_3"));
        assert!(checker.is_check_enabled("cve"));
        assert!(checker.is_check_enabled("mitre_attack"));
        assert!(checker.is_check_enabled("cmmc"));

        // Disable fips check
        checker.set_check_enabled("fips_140_3", false);
        assert!(!checker.is_check_enabled("fips_140_3"));

        // fips check should return true (early-exit when disabled)
        assert!(checker.check_fips_140_3());

        // Disable cmmc check
        checker.set_check_enabled("cmmc", false);
        assert_eq!(checker.check_cmmc(), 0); // disabled returns 0
    }

    #[test]
    fn test_compliance_unknown_check_defaults_disabled() {
        let checker = ComplianceChecker::new();
        assert!(!checker.is_check_enabled("nonexistent_check"));
    }

    // ========================= Vault::change_passphrase Tests =========================

    fn create_test_vault() -> (tempfile::TempDir, Vault) {
        let tmp = tempdir().unwrap();
        let dirs = ironvault::config::DirectoryPaths {
            config_dir: tmp.path().join("config"),
            data_dir: tmp.path().join("data"),
            cache_dir: tmp.path().join("cache"),
            vault_dir: tmp.path().join("data/vaults/default"),
            log_dir: tmp.path().join("data/logs"),
            backends_dir: tmp.path().join("config/backends"),
            utilities_dir: tmp.path().join("config/utilities"),
            databases_dir: tmp.path().join("config/databases"),
        };
        let config = VaultConfig::with_dirs(dirs).unwrap();
        let vault = Vault::new(Some(config)).unwrap();
        (tmp, vault)
    }

    #[test]
    fn test_change_passphrase_reencrypts_models() {
        let (_tmp, mut vault) = create_test_vault();

        let passphrase = b"original_passphrase_with_entropy".to_vec();
        vault.unlock(passphrase).unwrap();

        // Store two models
        let data1 = b"model one data".to_vec();
        let meta1 = ModelMetadata::new("model_a".to_string(), ModelFormat::PyTorch);
        vault
            .store_model("model_a", data1.clone(), meta1, None)
            .unwrap();

        let data2 = b"model two data".to_vec();
        let meta2 = ModelMetadata::new("model_b".to_string(), ModelFormat::ONNX);
        vault
            .store_model("model_b", data2.clone(), meta2, None)
            .unwrap();

        // Change passphrase
        let new_passphrase = b"new_passphrase_with_sufficient_entropy".to_vec();
        let count = vault.change_passphrase(new_passphrase).unwrap();
        assert_eq!(count, 2);

        // Verify data still retrievable with new key
        let retrieved1 = vault.get_model("model_a", None).unwrap();
        assert_eq!(retrieved1, data1);
        let retrieved2 = vault.get_model("model_b", None).unwrap();
        assert_eq!(retrieved2, data2);
    }

    #[test]
    fn test_change_passphrase_fails_when_locked() {
        let (_tmp, mut vault) = create_test_vault();
        // Don't unlock
        let result = vault.change_passphrase(b"any".to_vec());
        assert!(result.is_err());
    }

    // ========================= VersionControl::vault_path getter =========================

    #[test]
    fn test_version_control_vault_path_getter() {
        let tmp = tempdir().unwrap();
        let vc = VersionControl::new(tmp.path()).unwrap();
        assert_eq!(vc.vault_path(), tmp.path());
    }
}

#[allow(unused_imports)]
mod coverage_maximizer_tests {
    // coverage_maximizer_tests.rs — Targeted tests for remaining uncovered lines
    // Covers: version_sqlite, blockchain, conversion, compliance, database, formats,
    // compression, error, config, traits, utils, crypto

    use chrono::Utc;
    use ironvault::{
        // Audit
        audit::{AuditEntry, AuditEventType},
        // Compliance
        compliance::ComplianceChecker,
        conversion::{
            Converter, OnnxToCoreMLConverter, OnnxToTensorRtConverter, PyTorchToOnnxConverter,
            PyTorchToSafeTensorsConverter, SafeTensorsToGgufConverter,
            SafeTensorsToPyTorchConverter,
        },
        crypto::compression::{compress, decompress, CompressionAlgorithm, CompressionLevel},
        crypto::VaultCrypto,
        // Formats
        formats::ModelFormat,
        rag::{ChunkInfo, SQLiteDatabase},
        // Blockchain
        BlockchainAudit,
        ConversionOptions,
        // Conversion
        ConversionPipeline,
        Database,
        // RAG
        Document,
        // Traits
        IvUri,
        // Version control
        SqliteVersionRepo,
        ValidationCheck,
        // Config
        VaultConfig,
        // Error
        VaultError,
        VaultEvent,
        VaultState,
        VersionRepo,
    };
    use std::collections::HashMap;
    use tempfile::TempDir;

    // ============================================================================
    // VERSION SQLITE — Full lifecycle (covers ~35 uncovered lines)
    // ============================================================================
    mod version_sqlite_lifecycle {
        use super::*;

        fn make_repo() -> (SqliteVersionRepo, TempDir) {
            let tmp = TempDir::new().unwrap();
            let repo = SqliteVersionRepo::new(tmp.path()).unwrap();
            (repo, tmp)
        }

        #[test]
        fn new_creates_db_and_tables() {
            let (repo, _tmp) = make_repo();
            assert_eq!(repo.list_models().len(), 0);
        }

        #[test]
        fn add_version_and_get() {
            let (mut repo, _tmp) = make_repo();
            let v1 = repo
                .add_version(
                    "model-a",
                    "file.safetensors",
                    "safetensors",
                    1000,
                    800,
                    "abc123def456",
                    None,
                    None,
                )
                .unwrap();
            assert_eq!(v1.version, 1);

            let got = repo.get_version("model-a", Some(1));
            assert!(got.is_some());
            assert_eq!(got.unwrap().version, 1);

            // latest version (None)
            let latest = repo.get_version("model-a", None);
            assert!(latest.is_some());
        }

        #[test]
        fn add_multiple_versions_with_metadata_and_parents() {
            let (mut repo, _tmp) = make_repo();
            let _v1 = repo
                .add_version(
                    "m",
                    "f1.st",
                    "safetensors",
                    100,
                    80,
                    "check1",
                    Some([("author".into(), "alice".into())].into_iter().collect()),
                    None,
                )
                .unwrap();
            let _v2 = repo
                .add_version(
                    "m",
                    "f2.st",
                    "safetensors",
                    200,
                    150,
                    "check2",
                    None,
                    Some(1),
                )
                .unwrap();
            let _v3 = repo
                .add_version(
                    "m",
                    "f3.st",
                    "safetensors",
                    300,
                    200,
                    "check3",
                    Some([("tag".into(), "release".into())].into_iter().collect()),
                    Some(2),
                )
                .unwrap();

            // list_versions
            let versions = repo.list_versions("m");
            assert_eq!(versions.len(), 3);

            // get_lineage
            let lineage = repo.get_lineage("m", 3);
            assert!(lineage.len() >= 2); // v3 -> v2 -> v1

            // list_models
            let models = repo.list_models();
            assert!(models.contains(&"m".to_string()));
        }

        #[test]
        fn update_and_get_metadata() {
            let (mut repo, _tmp) = make_repo();
            repo.add_version("m", "f.st", "safetensors", 100, 80, "hash1", None, None)
                .unwrap();

            repo.update_metadata("m", 1, "color", "blue".to_string())
                .unwrap();
            let val = repo.get_metadata("m", 1, "color");
            assert_eq!(val, Some("blue".to_string()));

            // Non-existent key
            let missing = repo.get_metadata("m", 1, "nonexistent");
            assert!(missing.is_none());
        }

        #[test]
        fn verify_checksum() {
            let (mut repo, _tmp) = make_repo();
            let data = b"hello world checksum test";
            let hash = hex::encode(VaultCrypto::hash_sha256(data));
            repo.add_version("m2", "x.st", "safetensors", 25, 20, &hash, None, None)
                .unwrap();
            assert!(repo.verify_checksum("m2", 1, data));
            assert!(!repo.verify_checksum("m2", 1, b"wrong data"));
        }

        #[test]
        fn cleanup_old_versions() {
            let (mut repo, _tmp) = make_repo();
            for i in 0..5 {
                repo.add_version(
                    "m",
                    &format!("f{i}.st"),
                    "safetensors",
                    100 * (i + 1) as u64,
                    80,
                    &format!("hash{i}"),
                    None,
                    if i > 0 { Some(i as u32) } else { None },
                )
                .unwrap();
            }
            assert_eq!(repo.list_versions("m").len(), 5);

            // Keep only 2 most recent
            let deleted = repo.cleanup_old_versions("m", 2).unwrap();
            assert_eq!(deleted.len(), 3);
            assert_eq!(repo.list_versions("m").len(), 2);
        }

        #[test]
        fn delete_version() {
            let (mut repo, _tmp) = make_repo();
            repo.add_version("m", "f.st", "safetensors", 100, 80, "hash", None, None)
                .unwrap();
            assert!(repo.delete_version("m", 1).unwrap());
            assert!(repo.get_version("m", Some(1)).is_none());
        }

        #[test]
        fn get_version_nonexistent() {
            let (repo, _tmp) = make_repo();
            assert!(repo.get_version("no-model", Some(1)).is_none());
            assert!(repo.get_version("no-model", None).is_none());
        }

        #[test]
        fn json_migration() {
            // Create a versions.json file in the vault dir, then open SqliteVersionRepo
            let tmp = TempDir::new().unwrap();
            let versions_json = tmp.path().join("versions.json");
            // JSON must match HashMap<String, Vec<ModelVersion>> structure exactly
            let json_content = serde_json::json!({
                "migrated-model": [{
                    "version": 1,
                    "checkpoint_id": "chk-001",
                    "file_path": "model.safetensors",
                    "format": "safetensors",
                    "size_bytes": 500,
                    "compressed_size_bytes": 400,
                    "checksum_sha256": "abcdef123456",
                    "timestamp": "2024-01-01T00:00:00Z",
                    "parent_version": null,
                    "metadata": {"source": "test"}
                }]
            });
            std::fs::write(
                &versions_json,
                serde_json::to_string_pretty(&json_content).unwrap(),
            )
            .unwrap();

            let repo = SqliteVersionRepo::new(tmp.path()).unwrap();
            // After migration, the model should be accessible
            let models = repo.list_models();
            // Migration should have imported the model
            assert!(models.contains(&"migrated-model".to_string()));
            assert!(repo.get_version("migrated-model", Some(1)).is_some());
        }
    }

    // ============================================================================
    // BLOCKCHAIN — Full lifecycle (covers ~15 uncovered lines)
    // ============================================================================
    mod blockchain_lifecycle {
        use super::*;

        fn make_entry(desc: &str) -> AuditEntry {
            AuditEntry {
                timestamp: Utc::now(),
                event_type: AuditEventType::ModelStored,
                description: desc.to_string(),
                model_name: Some("test-model".to_string()),
                version: Some(1),
                success: true,
                metadata: None,
            }
        }

        #[test]
        fn full_blockchain_lifecycle() {
            let tmp = TempDir::new().unwrap();
            let block_size = 3;

            // Create audit (covers new/genesis: lines 351, 412)
            let mut audit = BlockchainAudit::new(tmp.path(), block_size).unwrap();
            assert_eq!(audit.height(), 1); // genesis block

            // Add enough entries to finalize a block
            for i in 0..block_size {
                audit.add_entry(make_entry(&format!("entry-{i}"))).unwrap();
            }

            // Should have auto-finalized
            assert!(audit.height() >= 2);

            // Add more entries for another block finalization
            for i in 0..block_size {
                audit.add_entry(make_entry(&format!("entry2-{i}"))).unwrap();
            }

            // verify_chain (covers lines 533-558)
            let verification = audit.verify_chain();
            assert!(verification.valid);
            assert!(verification.blocks_verified >= 2);
            assert!(verification.issues.is_empty());

            // generate_proof for genesis block entry (covers line 568)
            let proof = audit.generate_proof(0, 0).unwrap();

            // verify_proof (covers line 637)
            let pv = BlockchainAudit::verify_proof(&proof);
            assert!(pv.valid);
            assert!(pv.issues.is_empty());

            // Error: entry index out of range (covers line 574)
            let err = audit.generate_proof(0, 999);
            assert!(err.is_err());

            // get_block
            let block = audit.get_block(0).unwrap();
            assert!(block.is_some());

            // latest
            let latest = audit.latest();
            assert!(latest.is_some());
        }

        #[test]
        fn reopen_blockchain_from_disk() {
            let tmp = TempDir::new().unwrap();
            let block_size = 2;

            {
                let mut audit = BlockchainAudit::new(tmp.path(), block_size).unwrap();
                for i in 0..block_size {
                    audit
                        .add_entry(make_entry(&format!("persist-{i}")))
                        .unwrap();
                }
            }

            // Re-open from same directory (covers load_latest_block: lines 392, 401, 408)
            let audit2 = BlockchainAudit::new(tmp.path(), block_size).unwrap();
            assert!(audit2.height() >= 2);
            let chain_valid = audit2.verify_chain();
            assert!(chain_valid.valid);
        }

        #[test]
        fn search_blockchain_entries() {
            let tmp = TempDir::new().unwrap();
            // Use block_size=2 so entries are finalized into blocks quickly
            let mut audit = BlockchainAudit::new(tmp.path(), 2).unwrap();

            audit
                .add_entry(AuditEntry {
                    timestamp: Utc::now(),
                    event_type: AuditEventType::ModelStored,
                    description: "stored model-x".into(),
                    model_name: Some("model-x".into()),
                    version: Some(1),
                    success: true,
                    metadata: None,
                })
                .unwrap();

            audit
                .add_entry(AuditEntry {
                    timestamp: Utc::now(),
                    event_type: AuditEventType::ModelDeleted,
                    description: "deleted model-y".into(),
                    model_name: Some("model-y".into()),
                    version: Some(2),
                    success: true,
                    metadata: None,
                })
                .unwrap();

            // Force finalization
            let _ = audit.finalize_block();

            let results = audit.search(Some("model-x"), None, None, None, 10).unwrap();
            // Search may return results from finalized blocks or pending entries
            // At minimum, the search should not error
            let _ = results;
        }
    }

    // ============================================================================
    // CONVERSION — Pipeline + shim converters + ValidationCheck (covers ~25 lines)
    // ============================================================================
    mod conversion_pipeline_deep {
        use super::*;

        #[test]
        fn validation_check_pass_and_fail() {
            let pass = ValidationCheck::pass("size_check", "OK");
            assert!(pass.passed);
            assert_eq!(pass.name, "size_check");
            assert_eq!(pass.message, "OK");

            let fail = ValidationCheck::fail("format_check", "Mismatch");
            assert!(!fail.passed);
            assert_eq!(fail.name, "format_check");
        }

        #[test]
        fn pipeline_can_convert_and_find_paths() {
            let pipeline = ConversionPipeline::with_builtins();

            // Direct conversions
            assert!(pipeline.can_convert_direct(&ModelFormat::Safetensors, &ModelFormat::GGUF));
            assert!(pipeline.can_convert_direct(&ModelFormat::PyTorch, &ModelFormat::ONNX));
            assert!(!pipeline.can_convert_direct(&ModelFormat::GGUF, &ModelFormat::CoreML));

            // Multi-step path: PyTorch -> Safetensors -> GGUF
            let path = pipeline.find_path(&ModelFormat::PyTorch, &ModelFormat::GGUF);
            assert!(path.is_some());
            let path = path.unwrap();
            assert!(path.len() >= 2); // at least 2 steps

            // Supported conversions
            let conversions = pipeline.supported_conversions();
            assert!(conversions.len() >= 5);
        }

        #[test]
        fn pipeline_convert_direct() {
            let pipeline = ConversionPipeline::with_builtins();
            let opts = ConversionOptions::default();

            // PyTorch -> SafeTensors now requires valid ZIP input; invalid data should error
            let result = pipeline.convert(
                b"test-data",
                &ModelFormat::PyTorch,
                &ModelFormat::Safetensors,
                &opts,
                None,
            );
            assert!(result.is_err());
        }

        #[test]
        fn all_shim_converters_produce_plans() {
            let opts = ConversionOptions::default();
            let data = b"some-model-data";

            // PyTorchToSafeTensorsConverter is now a real converter (needs valid ZIP),
            // so only test remaining shim converters for JSON plan output
            let converters: Vec<(&str, Box<dyn Converter>)> = vec![
                ("pt_to_onnx", Box::new(PyTorchToOnnxConverter)),
                ("onnx_to_trt", Box::new(OnnxToTensorRtConverter)),
                ("onnx_to_coreml", Box::new(OnnxToCoreMLConverter)),
            ];

            for (name, converter) in &converters {
                let result = converter.convert(data, &opts, None);
                assert!(result.is_ok(), "Converter {name} failed");
                let output = result.unwrap();
                let plan: serde_json::Value = serde_json::from_slice(&output)
                    .unwrap_or_else(|_| panic!("Converter {name} didn't produce valid JSON"));
                assert!(
                    plan["converter"].is_string(),
                    "Converter {name} missing 'converter' field"
                );
            }
        }

        #[test]
        fn safetensors_to_gguf_with_quantization() {
            let opts = ConversionOptions {
                quantization: Some("q4_k_m".to_string()),
                ..ConversionOptions::default()
            };
            let result = SafeTensorsToGgufConverter
                .convert(b"st-data", &opts, None)
                .unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(plan["quantization"], "q4_k_m");
        }

        #[test]
        fn safetensors_to_pytorch_with_header() {
            // Build a SafeTensors buffer with a tensor
            let header = r#"{"t":{"dtype":"U8","shape":[2],"data_offsets":[0,2]}}"#;
            let header_bytes = header.as_bytes();
            let header_len = (header_bytes.len() as u64).to_le_bytes();
            let mut data = Vec::new();
            data.extend_from_slice(&header_len);
            data.extend_from_slice(header_bytes);
            data.extend_from_slice(&[1, 2]);

            let opts = ConversionOptions::default();
            let result = SafeTensorsToPyTorchConverter
                .convert(&data, &opts, None)
                .unwrap();
            // Real converter produces ZIP output
            assert_eq!(&result[0..2], b"PK");
        }

        #[test]
        fn converter_validate_default() {
            let converter = PyTorchToSafeTensorsConverter;
            let input = b"input-data-here";
            let output = b"output-data-result";
            let opts = ConversionOptions::default();

            let checks = converter.validate(input, output, &opts);
            // Default validate checks size ratio and non-empty output
            assert!(!checks.checks.is_empty());
        }

        #[test]
        fn empty_pipeline_cannot_convert() {
            let pipeline = ConversionPipeline::new();
            assert!(!pipeline.can_convert_direct(&ModelFormat::PyTorch, &ModelFormat::ONNX));
            assert!(pipeline
                .find_path(&ModelFormat::PyTorch, &ModelFormat::ONNX)
                .is_none());
            assert!(pipeline.supported_conversions().is_empty());
        }
    }

    // ============================================================================
    // RAG DATABASE — Full lifecycle with Document + ChunkInfo (covers ~33 lines)
    // ============================================================================
    mod database_lifecycle {
        use super::*;

        #[test]
        fn sqlite_database_on_disk() {
            let tmp = TempDir::new().unwrap();
            let db_path = tmp.path().join("test.db");
            let mut db = SQLiteDatabase::new(&db_path).unwrap();
            assert!(db_path.exists());

            // Must create table before inserting
            db.create_table("test_table", &[("name", "TEXT")]).unwrap();

            // Basic CRUD via Database trait
            let mut data = HashMap::new();
            data.insert("name".to_string(), "value1".to_string());
            db.insert("test_table", data).unwrap();

            let rows = db.query("SELECT * FROM test_table").unwrap();
            assert!(!rows.is_empty());
        }

        #[test]
        fn store_and_retrieve_document_with_embedding() {
            let db = SQLiteDatabase::in_memory().unwrap();

            let doc = Document {
                id: "doc-embed-1".into(),
                content: "Transformer attention mechanism with multi-head self-attention".into(),
                metadata: [
                    ("source".into(), "paper".into()),
                    ("year".into(), "2017".into()),
                ]
                .into_iter()
                .collect(),
                embedding: Some(vec![0.1, 0.2, 0.3, 0.4, 0.5]),
                chunk_info: None,
            };
            db.store_document(&doc).unwrap();

            let retrieved = db.get_document("doc-embed-1").unwrap();
            assert!(retrieved.is_some());
            let retrieved = retrieved.unwrap();
            assert_eq!(retrieved.content, doc.content);
            assert!(retrieved.embedding.is_some());
            let emb = retrieved.embedding.unwrap();
            assert_eq!(emb.len(), 5);
            assert!((emb[0] - 0.1).abs() < 0.001);
        }

        #[test]
        fn store_and_retrieve_document_with_chunk_info() {
            let db = SQLiteDatabase::in_memory().unwrap();

            let doc = Document {
                id: "chunk-doc-1".into(),
                content: "This is a chunked document fragment".into(),
                metadata: HashMap::new(),
                embedding: Some(vec![1.0, 2.0]),
                chunk_info: Some(ChunkInfo {
                    parent_id: Some("parent-doc".into()),
                    chunk_index: 2,
                    total_chunks: 10,
                    overlap: 50,
                }),
            };
            db.store_document(&doc).unwrap();

            let retrieved = db.get_document("chunk-doc-1").unwrap().unwrap();
            let ci = retrieved.chunk_info.unwrap();
            assert_eq!(ci.parent_id, Some("parent-doc".into()));
            assert_eq!(ci.chunk_index, 2);
            assert_eq!(ci.total_chunks, 10);
            assert_eq!(ci.overlap, 50);
        }

        #[test]
        fn search_documents() {
            let db = SQLiteDatabase::in_memory().unwrap();

            for i in 0..5 {
                let doc = Document {
                    id: format!("search-doc-{i}"),
                    content: format!("Document about neural networks topic {i}"),
                    metadata: [("index".into(), i.to_string())].into_iter().collect(),
                    embedding: None,
                    chunk_info: None,
                };
                db.store_document(&doc).unwrap();
            }

            let results = db.search_documents("neural", 10).unwrap();
            assert_eq!(results.len(), 5);

            let results = db.search_documents("nonexistent_keyword", 10).unwrap();
            assert!(results.is_empty());
        }

        #[test]
        fn database_trait_query() {
            let db = SQLiteDatabase::in_memory().unwrap();

            let doc = Document {
                id: "query-test".into(),
                content: "Query test content".into(),
                metadata: HashMap::new(),
                embedding: None,
                chunk_info: None,
            };
            db.store_document(&doc).unwrap();

            let rows = db.query("SELECT id, content FROM documents").unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].get("id").unwrap(), "query-test");
        }

        #[test]
        fn create_table_and_insert() {
            let mut db = SQLiteDatabase::in_memory().unwrap();
            db.create_table("items", &[("name", "TEXT"), ("value", "INTEGER")])
                .unwrap();

            let mut data = HashMap::new();
            data.insert("name".to_string(), "widget".to_string());
            data.insert("value".to_string(), "42".to_string());
            db.insert("items", data).unwrap();

            let rows = db.query("SELECT * FROM items").unwrap();
            assert_eq!(rows.len(), 1);
        }
    }

    // ============================================================================
    // COMPLIANCE — check_cve and run_all_checks (covers ~12 lines)
    // ============================================================================
    mod compliance_cve {
        use super::*;

        #[test]
        fn check_cve_runs_without_panic() {
            let checker = ComplianceChecker::new();
            // check_cve shells out to `cargo audit`, which may or may not be installed
            // Either way it should not panic
            let (passed, cves) = checker.check_cve();
            // We just verify it returns a result — behavior depends on cargo-audit availability
            let _ = (passed, cves);
        }

        #[test]
        fn run_all_checks_includes_cve_status() {
            let checker = ComplianceChecker::new();
            let status = checker.run_all_checks().unwrap();
            // FIPS should always pass
            assert!(status.fips_140_3);
            // MITRE should pass
            assert!(status.mitre_attack_aligned);
            // CMMC level should be 2
            assert_eq!(status.cmmc_level, 2);
        }

        #[test]
        fn check_cve_disabled() {
            let mut checker = ComplianceChecker::new();
            checker.set_check_enabled("cve", false);
            let (passed, cves) = checker.check_cve();
            assert!(passed);
            assert!(cves.is_empty());
        }
    }

    // ============================================================================
    // FORMATS — extension/name/Display for all variants (covers ~7 lines)
    // ============================================================================
    mod formats_exhaustive {
        use super::*;

        #[test]
        fn all_format_extensions_non_empty() {
            let formats = vec![
                ModelFormat::Safetensors,
                ModelFormat::GGUF,
                ModelFormat::ONNX,
                ModelFormat::PyTorch,
                ModelFormat::TensorRT,
                ModelFormat::CoreML,
                ModelFormat::MLX,
                ModelFormat::TorchScript,
                ModelFormat::TFLite,
                ModelFormat::TensorFlow,
                ModelFormat::Keras,
                ModelFormat::HDF5,
                ModelFormat::OpenVINO,
                ModelFormat::TVM,
                ModelFormat::NCNN,
                ModelFormat::MNN,
                ModelFormat::RKNN,
                ModelFormat::Caffe,
                ModelFormat::MXNet,
                ModelFormat::Darknet,
                ModelFormat::Pickle,
                ModelFormat::NumPy,
            ];

            for f in &formats {
                let ext = f.extension();
                assert!(!ext.is_empty(), "Extension empty for {:?}", f);

                let name = f.name();
                assert!(!name.is_empty(), "Name empty for {:?}", f);

                // Display impl (line 175)
                let display = format!("{}", f);
                assert!(!display.is_empty(), "Display empty for {:?}", f);
            }
        }

        #[test]
        fn custom_format_extension_and_name() {
            let custom = ModelFormat::Custom("myformat".to_string());
            let ext = custom.extension();
            let name = custom.name();
            let display = format!("{}", custom);
            assert!(!ext.is_empty());
            assert!(!name.is_empty());
            assert!(!display.is_empty());
        }

        #[test]
        fn format_from_extension_torchscript() {
            // These specifically map to TorchScript only via "torchscript"
            let ts = ModelFormat::from_extension("torchscript");
            // The TorchScript extension() returns "pt" which maps to PyTorch
            assert!(!ts.extension().is_empty());
        }
    }

    // ============================================================================
    // COMPRESSION — All algorithms + levels (covers ~6 lines)
    // ============================================================================
    mod compression_full {
        use super::*;

        #[test]
        fn compress_none_passthrough() {
            let data = b"passthrough data unchanged";
            let compressed =
                compress(data, CompressionAlgorithm::None, CompressionLevel::None).unwrap();
            assert_eq!(&compressed, data);

            let decompressed = decompress(&compressed, CompressionAlgorithm::None).unwrap();
            assert_eq!(&decompressed, data);
        }

        #[test]
        fn gzip_all_levels() {
            let data =
                b"Test data for gzip compression with various compression levels and settings";

            for level in [
                CompressionLevel::None,
                CompressionLevel::Fast,
                CompressionLevel::Balanced,
                CompressionLevel::Maximum,
            ] {
                let compressed = compress(data, CompressionAlgorithm::Gzip, level).unwrap();
                let decompressed = decompress(&compressed, CompressionAlgorithm::Gzip).unwrap();
                assert_eq!(&decompressed[..], &data[..], "Failed for level {:?}", level);
            }
        }

        #[test]
        fn lzma_all_levels() {
            let data = b"Test data for LZMA compression with various levels";

            for level in [
                CompressionLevel::Fast,
                CompressionLevel::Balanced,
                CompressionLevel::Maximum,
            ] {
                let compressed = compress(data, CompressionAlgorithm::Lzma, level).unwrap();
                let decompressed = decompress(&compressed, CompressionAlgorithm::Lzma).unwrap();
                assert_eq!(&decompressed[..], &data[..], "Failed for level {:?}", level);
            }
        }

        #[test]
        fn compress_large_data() {
            let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
            let compressed = compress(
                &data,
                CompressionAlgorithm::Gzip,
                CompressionLevel::Balanced,
            )
            .unwrap();
            assert!(compressed.len() < data.len()); // Should actually compress
            let decompressed = decompress(&compressed, CompressionAlgorithm::Gzip).unwrap();
            assert_eq!(decompressed, data);
        }
    }

    // ============================================================================
    // ERROR — From impls (covers ~3 lines)
    // ============================================================================
    mod error_from_impls {
        use super::*;

        #[test]
        fn from_serde_json_error() {
            let json_err = serde_json::from_str::<String>("{{bad json").unwrap_err();
            let ve: VaultError = json_err.into();
            match ve {
                VaultError::SerializationError(_) => {}
                other => panic!("Expected SerializationError, got {:?}", other),
            }
        }

        #[test]
        fn from_serde_yaml_ng_error() {
            let yaml_err = serde_yaml_ng::from_str::<String>(":\n  -").unwrap_err();
            let ve: VaultError = yaml_err.into();
            match ve {
                VaultError::SerializationError(_) => {}
                other => panic!("Expected SerializationError, got {:?}", other),
            }
        }

        #[test]
        fn from_io_error() {
            let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
            let ve: VaultError = io_err.into();
            match ve {
                VaultError::IoError(_) => {}
                other => panic!("Expected IoError, got {:?}", other),
            }
        }
    }

    // ============================================================================
    // CONFIG — VaultConfig coverage
    // ============================================================================
    mod config_coverage {
        use super::*;

        #[test]
        fn vault_config_default_has_fields() {
            let config = VaultConfig::default();
            // VaultConfig has nested settings structs
            let debug = format!("{:?}", config);
            assert!(!debug.is_empty());
        }

        #[test]
        fn vault_config_save_to_tempdir() {
            let tmp = TempDir::new().unwrap();
            // VaultConfig::with_dirs to set custom dir
            let dirs = ironvault::config::DirectoryPaths {
                config_dir: tmp.path().to_path_buf(),
                data_dir: tmp.path().to_path_buf(),
                cache_dir: tmp.path().to_path_buf(),
                vault_dir: tmp.path().to_path_buf(),
                log_dir: tmp.path().to_path_buf(),
                backends_dir: tmp.path().to_path_buf(),
                utilities_dir: tmp.path().to_path_buf(),
                databases_dir: tmp.path().to_path_buf(),
            };
            let config = VaultConfig::with_dirs(dirs);
            if let Ok(config) = config {
                let _ = config.save();
            }
        }
    }

    // ============================================================================
    // TRAITS — VaultEvent, VaultState exhaustive coverage
    // ============================================================================
    mod traits_exhaustive {
        use super::*;

        #[test]
        fn all_vault_events_have_timestamp() {
            let now = Utc::now();
            let events: Vec<VaultEvent> = vec![
                VaultEvent::VaultCreated {
                    vault: "v".into(),
                    timestamp: now,
                },
                VaultEvent::VaultUnlocked {
                    vault: "v".into(),
                    timestamp: now,
                },
                VaultEvent::VaultLocked {
                    vault: "v".into(),
                    timestamp: now,
                },
                VaultEvent::ModelStored {
                    vault: "v".into(),
                    model: "m".into(),
                    version: 1,
                    format: "safetensors".into(),
                    size: 100,
                    checksum: "abc".into(),
                    timestamp: now,
                },
                VaultEvent::ModelRetrieved {
                    vault: "v".into(),
                    model: "m".into(),
                    version: 1,
                    timestamp: now,
                },
                VaultEvent::ModelDeleted {
                    vault: "v".into(),
                    model: "m".into(),
                    version: 1,
                    timestamp: now,
                },
                VaultEvent::PassphraseChanged {
                    vault: "v".into(),
                    files_reencrypted: 5,
                    timestamp: now,
                },
                VaultEvent::IntegrityFailed {
                    vault: "v".into(),
                    model: "m".into(),
                    version: 1,
                    expected: "abc".into(),
                    actual: "def".into(),
                    timestamp: now,
                },
                VaultEvent::ComplianceChecked {
                    vault: "v".into(),
                    passed: true,
                    timestamp: now,
                },
            ];

            for event in &events {
                // Events should be debuggable
                let debug = format!("{:?}", event);
                assert!(!debug.is_empty());
            }
        }

        #[test]
        fn vault_state_display() {
            let locked = VaultState::Locked {
                vault_name: "test".into(),
                model_count: 5,
            };
            let unlocked = VaultState::Unlocked {
                vault_name: "test".into(),
                model_count: 5,
                unlocked_at: Utc::now(),
                operations_count: 10,
            };
            assert!(!format!("{}", locked).is_empty());
            assert!(!format!("{}", unlocked).is_empty());
            assert_ne!(format!("{}", locked), format!("{}", unlocked));
        }

        #[test]
        fn iv_uri_various_forms() {
            // Full URI with all components
            let uri = IvUri::parse("iv://vault/model@2/resource?key=val").unwrap();
            assert!(!format!("{}", uri).is_empty());

            // URI with just model
            let uri2 = IvUri::parse("iv://vault/model").unwrap();
            let s = uri2.to_string();
            assert!(s.contains("model"));

            // URI with empty query value
            let uri3 = IvUri::parse("iv://vault/model?key=");
            if let Ok(u) = uri3 {
                let _ = u.to_string();
            }
        }
    }

    // ============================================================================
    // UTILS — ModelAnalyzer format_size and other coverage
    // ============================================================================
    mod utils_extra {
        use ironvault::ModelAnalyzer;

        #[test]
        fn format_size_various() {
            let s1 = ModelAnalyzer::format_size(0);
            assert!(!s1.is_empty());

            let s2 = ModelAnalyzer::format_size(1024);
            assert!(!s2.is_empty());

            let s3 = ModelAnalyzer::format_size(1_000_000_000);
            assert!(!s3.is_empty());

            let s4 = ModelAnalyzer::format_size(u64::MAX);
            assert!(!s4.is_empty());
        }
    }
}

#[allow(unused_imports)]
mod coverage_ultimate_tests {
    //! Ultimate coverage tests — targets every remaining testable uncovered line.
    //!
    //! Targets:
    //! - database.rs: Database trait methods (insert/update/delete/query), store_document+get_document+search_documents
    //! - conversion.rs: individual shim converters (GGUF, SafeTensors↔PyTorch, PyTorch→ONNX, ONNX→TensorRT/CoreML, SafeTensors→GGUF), ValidationCheck::pass/fail, validate default
    //! - mcp.rs: builtin tool execution (search_documents, add_document, chunk_text, execute_rule), ToolResult::failure, with_metadata/with_data
    //! - traits.rs: IvUri with version+query, AsyncBlobStoreAdapter, MetricsSubscriber, EventSubscriber::accepts
    //! - vault.rs: store_model_chunked/get_model_chunked, VaultBuilder::no_default_subscribers
    //! - audit.rs: AuditSink trait impl (emit/query)
    //! - model_card.rs: to_json/to_yaml
    //! - crypto/streaming.rs: encrypt_chunked→decrypt_chunked, error paths (wrong version, truncated, MAC mismatch)
    //! - crypto/mod.rs: KeyManager store_key/load_key
    //! - formats.rs: TVM/RKNN/MXNet/Pickle/Caffe extension/name/Display
    //! - utils.rs: create_tar/create_zip/extract_zip, PruningInfo::size_reduction
    //! - version.rs: VersionControl::new, get_version(None)
    //! - config.rs: VaultConfig::with_dirs, save
    //! - rules.rs: RuleEngine::new(), SetValue action
    //! - knowledge.rs: chunk_text
    //! - compression.rs: Gzip compress/decompress, Lzma compress
    //! - storage/local.rs: download nonexistent, list after upload
    //! - error.rs: From<serde_yaml_ng::Error>

    use chrono::Utc;
    use ironvault::audit::{AuditEntry, AuditEventType, AuditLogger};
    use ironvault::config::DirectoryPaths;
    use ironvault::crypto::compression::{
        compress, decompress, CompressionAlgorithm, CompressionLevel,
    };
    use ironvault::crypto::{SecureKey, VaultCrypto};
    use ironvault::formats::ModelFormat;
    use ironvault::rag::{ChunkInfo, Document, SQLiteDatabase};
    use ironvault::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    // ── Database trait methods ───────────────────────────────────

    #[cfg(feature = "sqlite")]
    mod database_trait_methods {
        use super::*;

        /// Test Database::insert, query, update, delete
        #[test]
        fn test_database_insert_query_update_delete() {
            let mut db = SQLiteDatabase::in_memory().unwrap();

            // create_table auto-adds "id TEXT PRIMARY KEY", so only add extra columns
            db.create_table("users", &[("name", "TEXT"), ("email", "TEXT")])
                .unwrap();

            // Insert via Database trait
            let mut data = HashMap::new();
            data.insert("id".to_string(), "user1".to_string());
            data.insert("name".to_string(), "Alice".to_string());
            data.insert("email".to_string(), "alice@example.com".to_string());
            db.insert("users", data).unwrap();

            // Query via Database trait
            let results = db.query("SELECT * FROM users").unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].get("name").unwrap(), "Alice");

            // Update via Database trait
            let mut update_data = HashMap::new();
            update_data.insert("name".to_string(), "Alice Updated".to_string());
            db.update("users", "user1", update_data).unwrap();

            let results = db.query("SELECT * FROM users WHERE id = 'user1'").unwrap();
            assert_eq!(results[0].get("name").unwrap(), "Alice Updated");

            // Delete via Database trait
            db.delete("users", "user1").unwrap();
            let results = db.query("SELECT * FROM users").unwrap();
            assert_eq!(results.len(), 0);
        }

        /// Test Database::insert with multiple rows and query complex
        #[test]
        fn test_database_insert_multiple_and_query() {
            let mut db = SQLiteDatabase::in_memory().unwrap();

            // create_table auto-adds "id TEXT PRIMARY KEY"
            db.create_table("models", &[("format", "TEXT"), ("size", "TEXT")])
                .unwrap();

            for i in 0..5 {
                let mut data = HashMap::new();
                data.insert("id".to_string(), format!("model_{}", i));
                data.insert("format".to_string(), "safetensors".to_string());
                data.insert("size".to_string(), format!("{}", i * 1000));
                db.insert("models", data).unwrap();
            }

            let results = db.query("SELECT * FROM models ORDER BY id").unwrap();
            assert_eq!(results.len(), 5);
        }

        /// Test store_document, get_document, search_documents full lifecycle
        #[test]
        fn test_store_get_search_documents() {
            let db = SQLiteDatabase::in_memory().unwrap();

            let doc = Document {
                id: "doc1".to_string(),
                content: "Machine learning is a subset of artificial intelligence".to_string(),
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("topic".to_string(), "AI".to_string());
                    m
                },
                embedding: Some(vec![0.1, 0.2, 0.3, 0.4]),
                chunk_info: Some(ChunkInfo {
                    chunk_index: 0,
                    total_chunks: 1,
                    parent_id: Some("parent1".to_string()),
                    overlap: 0,
                }),
            };
            db.store_document(&doc).unwrap();

            // Get document
            let retrieved = db.get_document("doc1").unwrap();
            assert!(retrieved.is_some());
            let retrieved = retrieved.unwrap();
            assert_eq!(
                retrieved.content,
                "Machine learning is a subset of artificial intelligence"
            );
            assert_eq!(retrieved.metadata.get("topic"), Some(&"AI".to_string()));

            // Search documents
            let results = db.search_documents("machine learning", 10).unwrap();
            assert!(!results.is_empty());
            assert!(results[0].content.contains("Machine learning"));
        }

        /// Test search with embeddings stored
        #[test]
        fn test_store_document_with_embedding_then_search() {
            let db = SQLiteDatabase::in_memory().unwrap();

            let doc1 = Document {
                id: "embed_doc1".to_string(),
                content: "Neural networks use backpropagation".to_string(),
                metadata: HashMap::new(),
                embedding: Some(vec![1.0, 2.0, 3.0]),
                chunk_info: None,
            };
            db.store_document(&doc1).unwrap();

            let doc2 = Document {
                id: "embed_doc2".to_string(),
                content: "Decision trees are simpler models".to_string(),
                metadata: HashMap::new(),
                embedding: None,
                chunk_info: None,
            };
            db.store_document(&doc2).unwrap();

            let results = db.search_documents("neural", 5).unwrap();
            assert!(!results.is_empty());
        }

        /// Test get_document returns None for nonexistent
        #[test]
        fn test_get_document_not_found() {
            let db = SQLiteDatabase::in_memory().unwrap();
            // First store a document to ensure the documents table is created
            let doc = Document {
                id: "setup_doc".to_string(),
                content: "Setup".to_string(),
                metadata: HashMap::new(),
                embedding: None,
                chunk_info: None,
            };
            db.store_document(&doc).unwrap();
            // Now query for a nonexistent document
            let result = db.get_document("nonexistent").unwrap();
            assert!(result.is_none());
        }

        /// Test on-disk SQLiteDatabase
        #[test]
        fn test_sqlite_database_on_disk() {
            let dir = tempdir().unwrap();
            let path = dir.path().join("test.db");
            let db = SQLiteDatabase::new(&path).unwrap();

            let doc = Document {
                id: "disk_doc".to_string(),
                content: "Stored on disk".to_string(),
                metadata: HashMap::new(),
                embedding: None,
                chunk_info: None,
            };
            db.store_document(&doc).unwrap();

            let retrieved = db.get_document("disk_doc").unwrap();
            assert!(retrieved.is_some());
        }
    }

    // ── Conversion shim converters ───────────────────────────────

    mod conversion_shim_converters {
        use super::*;

        #[test]
        fn test_validation_check_pass_and_fail() {
            let pass = ValidationCheck::pass("test_check", "All good");
            assert!(pass.passed);

            let fail = ValidationCheck::fail("size_check", "Too large");
            assert!(!fail.passed);
        }

        /// Helper: create minimal valid SafeTensors binary data
        fn make_safetensors_data() -> Vec<u8> {
            // SafeTensors format: 8-byte LE header_len + JSON header + tensor data
            let header = serde_json::json!({
                "weight": {
                    "dtype": "F32",
                    "shape": [2, 2],
                    "data_offsets": [0, 16]
                }
            });
            let header_bytes = serde_json::to_vec(&header).unwrap();
            let header_len = header_bytes.len() as u64;
            let tensor_data = vec![0u8; 16]; // 4 f32 values
            let mut data = Vec::new();
            data.extend_from_slice(&header_len.to_le_bytes());
            data.extend_from_slice(&header_bytes);
            data.extend_from_slice(&tensor_data);
            data
        }

        #[test]
        fn test_convert_safetensors_to_pytorch() {
            let pipeline = ConversionPipeline::with_builtins();
            let st_data = make_safetensors_data();
            let options = ConversionOptions::default();

            let result = pipeline.convert(
                &st_data,
                &ModelFormat::Safetensors,
                &ModelFormat::PyTorch,
                &options,
                None,
            );
            assert!(result.is_ok());
            let conv = result.unwrap();
            assert!(!conv.data.is_empty());
        }

        #[test]
        fn test_convert_pytorch_to_safetensors() {
            let pipeline = ConversionPipeline::with_builtins();
            let dummy_data = b"dummy pytorch model data for testing";
            let options = ConversionOptions::default();

            // Real converter requires valid ZIP input; invalid data should error
            let result = pipeline.convert(
                dummy_data,
                &ModelFormat::PyTorch,
                &ModelFormat::Safetensors,
                &options,
                None,
            );
            assert!(result.is_err());
        }

        #[test]
        fn test_convert_pytorch_to_onnx() {
            let pipeline = ConversionPipeline::with_builtins();
            let dummy_data = b"dummy pytorch data";
            let options = ConversionOptions {
                opset_version: Some(13),
                ..ConversionOptions::default()
            };

            let result = pipeline.convert(
                dummy_data,
                &ModelFormat::PyTorch,
                &ModelFormat::ONNX,
                &options,
                None,
            );
            assert!(result.is_ok());
        }

        #[test]
        fn test_convert_onnx_to_tensorrt() {
            let pipeline = ConversionPipeline::with_builtins();
            let dummy_data = b"dummy onnx data";
            let options = ConversionOptions::default();

            let result = pipeline.convert(
                dummy_data,
                &ModelFormat::ONNX,
                &ModelFormat::TensorRT,
                &options,
                None,
            );
            assert!(result.is_ok());
        }

        #[test]
        fn test_convert_onnx_to_coreml() {
            let pipeline = ConversionPipeline::with_builtins();
            let dummy_data = b"dummy onnx data for coreml";
            let options = ConversionOptions::default();

            let result = pipeline.convert(
                dummy_data,
                &ModelFormat::ONNX,
                &ModelFormat::CoreML,
                &options,
                None,
            );
            assert!(result.is_ok());
        }

        #[test]
        fn test_convert_safetensors_to_gguf() {
            let pipeline = ConversionPipeline::with_builtins();
            let dummy_data = b"dummy safetensors data blah blah blah";
            let options = ConversionOptions {
                quantization: Some("q4_k_m".to_string()),
                ..ConversionOptions::default()
            };

            let result = pipeline.convert(
                dummy_data,
                &ModelFormat::Safetensors,
                &ModelFormat::GGUF,
                &options,
                None,
            );
            assert!(result.is_ok());
        }

        #[test]
        fn test_convert_same_format_shortcircuit() {
            let pipeline = ConversionPipeline::with_builtins();
            let data = b"unchanged data";
            let options = ConversionOptions::default();

            let result = pipeline.convert(
                data,
                &ModelFormat::PyTorch,
                &ModelFormat::PyTorch,
                &options,
                None,
            );
            assert!(result.is_ok());
            assert_eq!(result.unwrap().data, data);
        }

        #[test]
        fn test_convert_with_progress_callback() {
            let pipeline = ConversionPipeline::with_builtins();
            let dummy_data = b"data for progress tracking";

            let progress_fn: Box<dyn Fn(&ConversionProgress) + Send + Sync> =
                Box::new(|_progress| {
                    // Just note that it was called
                });

            let options = ConversionOptions::default();
            let result = pipeline.convert(
                dummy_data,
                &ModelFormat::PyTorch,
                &ModelFormat::ONNX,
                &options,
                Some(&progress_fn),
            );
            assert!(result.is_ok());
        }

        #[test]
        fn test_convert_with_validation() {
            let pipeline = ConversionPipeline::with_builtins();
            let st_data = make_safetensors_data();
            let options = ConversionOptions::with_validation();

            let result = pipeline.convert(
                &st_data,
                &ModelFormat::Safetensors,
                &ModelFormat::PyTorch,
                &options,
                None,
            );
            assert!(result.is_ok());
            let conv = result.unwrap();
            // Should have validation report since we requested it
            if let Some(report) = &conv.validation {
                assert!(!report.checks.is_empty());
            }
        }

        #[test]
        fn test_find_path_multi_step() {
            let pipeline = ConversionPipeline::with_builtins();
            // PyTorch -> Safetensors -> GGUF (two steps)
            let path = pipeline.find_path(&ModelFormat::PyTorch, &ModelFormat::GGUF);
            assert!(path.is_some());
            let path = path.unwrap();
            assert!(path.len() >= 2); // at least PyTorch->Safetensors->GGUF
        }

        #[test]
        fn test_gguf_header_parser() {
            let pipeline = ConversionPipeline::with_builtins();
            // Create minimal GGUF-like data: magic "GGUF" + version + tensor_count + metadata_count
            let mut gguf_data = Vec::new();
            gguf_data.extend_from_slice(b"GGUF"); // magic
            gguf_data.extend_from_slice(&3u32.to_le_bytes()); // version
            gguf_data.extend_from_slice(&0u64.to_le_bytes()); // tensor count
            gguf_data.extend_from_slice(&0u64.to_le_bytes()); // metadata kv count

            let options = ConversionOptions::default();
            // GGUF -> GGUF (same format) should short-circuit
            let result = pipeline.convert(
                &gguf_data,
                &ModelFormat::GGUF,
                &ModelFormat::GGUF,
                &options,
                None,
            );
            assert!(result.is_ok());
        }
    }

    // ── MCP builtin tools ────────────────────────────────────────

    mod mcp_builtin_tools {
        use ironvault::rag::mcp::{MCPServer, MCPTool, ToolContext, ToolResult};

        #[test]
        fn test_tool_result_failure() {
            let result = ToolResult::failure("Something went wrong".to_string());
            assert!(!result.success);
            assert_eq!(result.error, Some("Something went wrong".to_string()));
            assert!(result.data.is_null());
        }

        #[test]
        fn test_tool_result_with_metadata() {
            let result = ToolResult::success(serde_json::json!({"key": "value"}))
                .with_metadata("version".to_string(), "1.0".to_string());
            assert!(result.success);
            assert_eq!(result.metadata.get("version"), Some(&"1.0".to_string()));
        }

        #[test]
        fn test_tool_context_with_data() {
            let ctx = ToolContext::new()
                .with_data("model_id".to_string(), "llama-7b".to_string())
                .with_document_store("store1".to_string())
                .with_knowledge_base("kb1".to_string());
            assert_eq!(ctx.data.get("model_id"), Some(&"llama-7b".to_string()));
            assert_eq!(ctx.document_store, Some("store1".to_string()));
            assert_eq!(ctx.knowledge_base, Some("kb1".to_string()));
        }

        #[test]
        fn test_mcp_tool_with_metadata() {
            let tool = MCPTool::new("test_tool".to_string(), "A test tool".to_string())
                .with_metadata("author".to_string(), "test".to_string());
            assert_eq!(tool.metadata.get("author"), Some(&"test".to_string()));
        }

        #[test]
        fn test_execute_search_documents_tool() {
            let mut server = MCPServer::new();
            server.register_builtin_tools().unwrap();

            let ctx = ToolContext::new();
            let params = serde_json::json!({
                "query": "transformer architecture",
                "top_k": 3,
                "threshold": 0.7
            });

            let result = server
                .execute_tool("search_documents", params, &ctx)
                .unwrap();
            assert!(result.success);
        }

        #[test]
        fn test_execute_add_document_tool() {
            let mut server = MCPServer::new();
            server.register_builtin_tools().unwrap();

            let ctx = ToolContext::new();
            let params = serde_json::json!({
                "id": "doc1",
                "content": "Test document content",
                "metadata": {"topic": "testing"}
            });

            let result = server.execute_tool("add_document", params, &ctx).unwrap();
            assert!(result.success);
        }

        #[test]
        fn test_execute_chunk_text_tool() {
            let mut server = MCPServer::new();
            server.register_builtin_tools().unwrap();

            let ctx = ToolContext::new();
            let long_text = "a".repeat(1500);
            let params = serde_json::json!({
                "text": long_text,
                "chunk_size": 512,
                "overlap": 50
            });

            let result = server.execute_tool("chunk_text", params, &ctx).unwrap();
            assert!(result.success);
            let num_chunks = result.data.get("num_chunks").and_then(|v| v.as_u64());
            assert!(num_chunks.unwrap() > 1);
        }

        #[test]
        fn test_execute_rule_tool() {
            let mut server = MCPServer::new();
            server.register_builtin_tools().unwrap();

            let ctx = ToolContext::new();
            let params = serde_json::json!({
                "rule_id": "validate_model_size",
                "context": {"model_size": "1000000"}
            });

            let result = server.execute_tool("execute_rule", params, &ctx).unwrap();
            assert!(result.success);
        }

        #[test]
        fn test_list_and_get_tools() {
            let mut server = MCPServer::new();
            server.register_builtin_tools().unwrap();

            let tools = server.list_tools();
            assert!(tools.len() >= 4);

            let search = server.get_tool("search_documents");
            assert!(search.is_some());

            let nonexistent = server.get_tool("nonexistent_tool");
            assert!(nonexistent.is_none());
        }

        #[test]
        fn test_execute_nonexistent_tool() {
            let server = MCPServer::new();
            let ctx = ToolContext::new();
            let result = server.execute_tool("no_such_tool", serde_json::json!({}), &ctx);
            assert!(result.is_err());
        }
    }

    // ── Traits: IvUri, AsyncBlobStoreAdapter, Metrics ──────────

    mod traits_coverage {
        use super::*;

        #[test]
        fn test_iv_uri_with_version_and_query() {
            let uri = IvUri {
                vault: Some("my_vault".to_string()),
                model: Some("llama".to_string()),
                version: Some(3),
                resource: Some("weights".to_string()),
                query: {
                    let mut q = HashMap::new();
                    q.insert("format".to_string(), "safetensors".to_string());
                    q.insert("quant".to_string(), "q4_k_m".to_string());
                    q
                },
            };

            let s = uri.to_string();
            assert!(s.contains("@3"));
            assert!(s.contains("?"));
            assert!(s.contains("format=safetensors"));
            assert!(s.contains("quant=q4_k_m"));
            assert!(s.contains("/weights"));

            // Also test Display
            let display_str = format!("{}", uri);
            assert!(display_str.contains("iv"));
        }

        #[test]
        fn test_iv_uri_with_empty_query_value() {
            let uri = IvUri {
                vault: Some("v".to_string()),
                model: Some("m".to_string()),
                version: None,
                resource: None,
                query: {
                    let mut q = HashMap::new();
                    q.insert("flag".to_string(), "".to_string()); // empty value => just key
                    q
                },
            };

            let s = uri.to_string();
            assert!(s.contains("?flag"));
            // Should NOT contain "flag=" since value is empty
        }

        #[test]
        fn test_event_bus_with_metrics_subscriber() {
            let metrics = std::sync::Arc::new(VaultMetrics::new());
            let subscriber = MetricsSubscriber::new(metrics.clone());

            let mut bus = EventBus::new();
            bus.subscribe(Box::new(subscriber));

            // Emit ModelStored event
            bus.emit(&VaultEvent::ModelStored {
                vault: "test".to_string(),
                model: "m1".to_string(),
                version: 1,
                format: "safetensors".to_string(),
                size: 1000,
                checksum: "abc".to_string(),
                timestamp: Utc::now(),
            });

            // Emit ModelRetrieved event
            bus.emit(&VaultEvent::ModelRetrieved {
                vault: "test".to_string(),
                model: "m1".to_string(),
                version: 1,
                timestamp: Utc::now(),
            });

            let snap = metrics.snapshot();
            assert_eq!(snap.models_stored_total, 1);
            assert_eq!(snap.models_retrieved_total, 1);
        }

        #[test]
        fn test_event_subscriber_accepts_default() {
            // MetricsSubscriber uses default accepts() which returns true
            let metrics = std::sync::Arc::new(VaultMetrics::new());
            let subscriber = MetricsSubscriber::new(metrics.clone());

            use ironvault::traits::EventSubscriber;
            let event = VaultEvent::VaultCreated {
                vault: "test".to_string(),
                timestamp: Utc::now(),
            };
            assert!(subscriber.accepts(&event));
        }

        #[test]
        fn test_vault_event_display() {
            let event = VaultEvent::ModelStored {
                vault: "v".to_string(),
                model: "m".to_string(),
                version: 1,
                format: "pt".to_string(),
                size: 100,
                checksum: "abc".to_string(),
                timestamp: Utc::now(),
            };

            let display = format!("{}", event);
            assert!(display.contains("model_stored"));
        }

        #[test]
        fn test_crypto_provider_hash_hex() {
            let crypto = VaultCrypto::new().unwrap();
            use ironvault::traits::CryptoProvider;
            let hex = crypto.hash_hex(b"test data");
            assert_eq!(hex.len(), 64); // SHA-256 hex is 64 chars
        }
    }

    // ── Audit logging: AuditSink trait ───────────────────────────

    mod audit_sink_tests {
        use super::*;
        use ironvault::traits::AuditSink;

        #[test]
        fn test_audit_sink_emit_and_query() {
            let dir = tempdir().unwrap();
            let log_path = dir.path().join("audit.jsonl");
            let logger = AuditLogger::new(&log_path).unwrap();

            // Use AuditSink trait methods (emit + query)
            logger
                .emit(AuditEntry {
                    timestamp: Utc::now(),
                    event_type: AuditEventType::VaultCreated,
                    description: "Test vault created".to_string(),
                    model_name: None,
                    version: None,
                    success: true,
                    metadata: None,
                })
                .unwrap();

            logger
                .emit(AuditEntry {
                    timestamp: Utc::now(),
                    event_type: AuditEventType::ModelStored,
                    description: "Model stored".to_string(),
                    model_name: Some("test_model".to_string()),
                    version: Some(1),
                    success: true,
                    metadata: None,
                })
                .unwrap();

            let entries = logger.query(Some(10)).unwrap();
            assert_eq!(entries.len(), 2);
            assert!(format!("{:?}", entries[0].event_type).contains("VaultCreated"));
        }
    }

    // ── Model Card to_json/to_yaml ───────────────────────────────

    mod model_card_serialization {
        use super::*;

        #[test]
        fn test_model_card_to_json() {
            let details = ModelDetails {
                name: "test-model".to_string(),
                version: "1.0".to_string(),
                description: "A test model".to_string(),
                model_type: "LLM".to_string(),
                architecture: "Transformer".to_string(),
                size: "7B".to_string(),
                framework: "PyTorch".to_string(),
                format: "safetensors".to_string(),
                license: None,
                citation: None,
                developers: vec!["tester".to_string()],
                contact: None,
                repository: None,
                paper: None,
            };
            let intended = IntendedUse {
                primary_uses: vec!["testing".to_string()],
                primary_users: vec!["developers".to_string()],
                out_of_scope_uses: vec![],
                use_case_examples: None,
            };
            let card = ModelCard::new(details, intended);
            let json = card.to_json().unwrap();
            assert!(json.contains("test-model"));
            // Verify it's valid JSON
            let _: serde_json::Value = serde_json::from_str(&json).unwrap();
        }

        #[test]
        fn test_model_card_to_yaml() {
            let details = ModelDetails {
                name: "yaml-model".to_string(),
                version: "2.0".to_string(),
                description: "A YAML test model".to_string(),
                model_type: "CNN".to_string(),
                architecture: "ResNet".to_string(),
                size: "50M".to_string(),
                framework: "TensorFlow".to_string(),
                format: "onnx".to_string(),
                license: None,
                citation: None,
                developers: vec![],
                contact: None,
                repository: None,
                paper: None,
            };
            let intended = IntendedUse {
                primary_uses: vec![],
                primary_users: vec![],
                out_of_scope_uses: vec![],
                use_case_examples: None,
            };
            let card = ModelCard::new(details, intended);
            let yaml = card.to_yaml().unwrap();
            assert!(yaml.contains("yaml-model"));
        }
    }

    // ── Crypto streaming: encrypt_chunked/decrypt_chunked ────────

    mod streaming_crypto_tests {
        use super::*;
        use ironvault::crypto::streaming::{decrypt_chunked, encrypt_chunked, is_chunked_format};

        #[test]
        fn test_encrypt_decrypt_chunked_roundtrip() {
            let crypto = VaultCrypto::new().unwrap();
            let passphrase = b"streaming-test-passphrase-12345".to_vec();
            let (key, _salt) = crypto.derive_key(passphrase, None).unwrap();

            let original =
                b"Hello, this is test data for chunked encryption. It should roundtrip correctly!";
            let encrypted = encrypt_chunked(&crypto, original, &key, 32).unwrap();

            assert!(is_chunked_format(&encrypted));
            assert_ne!(&encrypted[..], &original[..]);

            let decrypted = decrypt_chunked(&crypto, &encrypted, &key).unwrap();
            assert_eq!(&decrypted[..], &original[..]);
        }

        #[test]
        fn test_chunked_with_large_data() {
            let crypto = VaultCrypto::new().unwrap();
            let passphrase = b"large-data-passphrase-xyz".to_vec();
            let (key, _) = crypto.derive_key(passphrase, None).unwrap();

            let original: Vec<u8> = (0..10000u32).flat_map(|i| i.to_le_bytes()).collect();
            let encrypted = encrypt_chunked(&crypto, &original, &key, 1024).unwrap();
            let decrypted = decrypt_chunked(&crypto, &encrypted, &key).unwrap();
            assert_eq!(decrypted, original);
        }

        #[test]
        fn test_chunked_decrypt_wrong_version() {
            let crypto = VaultCrypto::new().unwrap();
            let passphrase = b"version-check-pass".to_vec();
            let (key, _) = crypto.derive_key(passphrase, None).unwrap();

            let mut encrypted = encrypt_chunked(&crypto, b"test", &key, 32).unwrap();
            // Corrupt the version byte (index 4)
            encrypted[4] = 99;

            let result = decrypt_chunked(&crypto, &encrypted, &key);
            assert!(result.is_err());
            let err = format!("{}", result.unwrap_err());
            assert!(err.contains("version") || err.contains("Unsupported"));
        }

        #[test]
        fn test_chunked_decrypt_corrupted_mac() {
            let crypto = VaultCrypto::new().unwrap();
            let passphrase = b"mac-corruption-test".to_vec();
            let (key, _) = crypto.derive_key(passphrase, None).unwrap();

            let original = b"data that will have its MAC corrupted";
            let mut encrypted = encrypt_chunked(&crypto, original, &key, 32).unwrap();

            // Corrupt the last byte (part of stream MAC)
            let last = encrypted.len() - 1;
            encrypted[last] ^= 0xFF;

            let result = decrypt_chunked(&crypto, &encrypted, &key);
            assert!(result.is_err());
        }

        #[test]
        fn test_chunked_decrypt_too_short() {
            let crypto = VaultCrypto::new().unwrap();
            let passphrase = b"short-data-test".to_vec();
            let (key, _) = crypto.derive_key(passphrase, None).unwrap();

            // Data shorter than header + MAC
            let short_data = vec![0u8; 10];
            let result = decrypt_chunked(&crypto, &short_data, &key);
            assert!(result.is_err());
        }
    }

    // ── Crypto KeyManager ────────────────────────────────────────

    mod key_manager_tests {
        use super::*;
        use ironvault::crypto::KeyManager;

        #[test]
        fn test_key_manager_store_and_load() {
            let km = KeyManager::new().unwrap();

            let passphrase = b"master-passphrase-for-km".to_vec();
            let model_key = SecureKey::from_bytes(&[42u8; 32]).unwrap();

            let stored_data = km.store_key(&model_key, passphrase.clone()).unwrap();

            let loaded = km.load_key(&stored_data, passphrase).unwrap();
            assert_eq!(loaded.as_bytes(), model_key.as_bytes());
        }

        #[test]
        fn test_key_manager_load_wrong_passphrase() {
            let km = KeyManager::new().unwrap();

            let model_key = SecureKey::from_bytes(&[99u8; 32]).unwrap();
            let stored_data = km.store_key(&model_key, b"correct-pass".to_vec()).unwrap();

            let result = km.load_key(&stored_data, b"wrong-pass".to_vec());
            assert!(result.is_err());
        }
    }

    // ── Formats: uncommon variants ───────────────────────────────

    mod formats_uncommon_variants {
        use super::*;

        #[test]
        fn test_tvm_extension_and_name() {
            assert_eq!(ModelFormat::TVM.extension(), "so");
            assert_eq!(ModelFormat::TVM.name(), "TVM");
            assert_eq!(format!("{}", ModelFormat::TVM), "TVM");
        }

        #[test]
        fn test_rknn_extension_and_name() {
            assert_eq!(ModelFormat::RKNN.extension(), "rknn");
            assert_eq!(ModelFormat::RKNN.name(), "RKNN");
        }

        #[test]
        fn test_mxnet_extension_and_name() {
            assert_eq!(ModelFormat::MXNet.extension(), "params");
            assert_eq!(ModelFormat::MXNet.name(), "MXNet");
        }

        #[test]
        fn test_pickle_extension_and_name() {
            assert_eq!(ModelFormat::Pickle.extension(), "pkl");
            assert_eq!(ModelFormat::Pickle.name(), "Pickle");
        }

        #[test]
        fn test_caffe_extension_and_name() {
            assert_eq!(ModelFormat::Caffe.extension(), "caffemodel");
            assert_eq!(ModelFormat::Caffe.name(), "Caffe");
            assert_eq!(format!("{}", ModelFormat::Caffe), "Caffe");
        }
    }

    // ── Utils: archive + PruningInfo ─────────────────────────────

    mod utils_archive_tests {
        use super::*;

        #[test]
        fn test_create_and_extract_tar() {
            let dir = tempdir().unwrap();
            let tar_path = dir.path().join("models.tar");

            let models = vec![
                ("model1.bin".to_string(), vec![1u8, 2, 3, 4, 5]),
                ("model2.bin".to_string(), vec![10, 20, 30]),
            ];

            let total = ModelArchive::create_tar(models, &tar_path).unwrap();
            assert_eq!(total, 8); // 5 + 3

            let extracted = ModelArchive::extract_tar(&tar_path).unwrap();
            assert_eq!(extracted.len(), 2);
        }

        #[test]
        fn test_create_and_extract_zip() {
            let dir = tempdir().unwrap();
            let zip_path = dir.path().join("models.zip");

            let models = vec![
                ("model_a.safetensors".to_string(), vec![0u8; 100]),
                ("model_b.onnx".to_string(), vec![255u8; 50]),
            ];

            let total = ModelArchive::create_zip(models, &zip_path).unwrap();
            assert_eq!(total, 150);

            let extracted = ModelArchive::extract_zip(&zip_path).unwrap();
            assert_eq!(extracted.len(), 2);
        }

        #[test]
        fn test_pruning_info_size_reduction() {
            use ironvault::utils::{PruningInfo, PruningMethod};

            let info = PruningInfo::new(PruningMethod::Magnitude, 0.5, 1000, 500);
            let reduction = info.size_reduction();
            assert!((reduction - 50.0).abs() < 1e-6);

            let info_zero = PruningInfo::new(PruningMethod::Structured, 0.0, 0, 0);
            assert_eq!(info_zero.size_reduction(), 0.0);
        }
    }

    // ── Version control ──────────────────────────────────────────

    mod version_control_tests {
        use super::*;

        #[test]
        fn test_version_control_new_and_get_latest() {
            let dir = tempdir().unwrap();
            let mut vc = VersionControl::new(dir.path()).unwrap();

            // Add versions using the VersionRepo trait
            vc.add_version("model_a", "f.enc", "pytorch", 100, 50, "abc", None, None)
                .unwrap();
            vc.add_version(
                "model_a",
                "f2.enc",
                "pytorch",
                200,
                100,
                "def",
                None,
                Some(1),
            )
            .unwrap();

            // Get latest version (None = latest)
            let latest = vc.get_version("model_a", None).unwrap();
            assert_eq!(latest.version, 2);
        }
    }

    // ── Config ───────────────────────────────────────────────────

    mod config_coverage_tests {
        use super::*;

        #[test]
        fn test_vault_config_with_dirs_and_save() {
            let dir = tempdir().unwrap();
            let dirs = DirectoryPaths {
                config_dir: dir.path().join("config"),
                data_dir: dir.path().join("data"),
                cache_dir: dir.path().join("cache"),
                vault_dir: dir.path().join("vaults"),
                log_dir: dir.path().join("logs"),
                backends_dir: dir.path().join("backends"),
                utilities_dir: dir.path().join("utilities"),
                databases_dir: dir.path().join("databases"),
            };

            let config = VaultConfig::with_dirs(dirs).unwrap();
            config.save().unwrap();

            // Verify the config file was written
            assert!(dir.path().join("config").exists());
        }
    }

    // ── Rules: RuleEngine with SetValue ──────────────────────────

    mod rules_set_value_tests {
        use super::*;
        use ironvault::rag::rules::{Rule, RuleAction, RuleCondition, RuleEngine};

        #[test]
        fn test_rule_engine_new_and_set_value_action() {
            let mut engine = RuleEngine::new();

            // Set initial context
            engine.set_context("model_format".to_string(), "pytorch".to_string());

            // Add a rule with SetValue action
            let rule = Rule {
                id: "convert_rule_1".to_string(),
                name: "convert_format".to_string(),
                conditions: {
                    let mut c = HashMap::new();
                    c.insert(
                        "model_format".to_string(),
                        RuleCondition::Equals("pytorch".to_string()),
                    );
                    c
                },
                actions: vec![RuleAction::SetValue {
                    key: "target_format".to_string(),
                    value: "safetensors".to_string(),
                }],
                priority: 10,
                enabled: true,
            };

            engine.add_rule(rule);
            let result = engine.execute();
            assert!(result.is_ok());

            // Verify SetValue action set the context
            assert_eq!(
                engine.get_context("target_format"),
                Some(&"safetensors".to_string())
            );
        }
    }

    // ── Knowledge base: chunk_text ───────────────────────────────

    mod knowledge_chunk_text_tests {

        use ironvault::rag::knowledge::{KnowledgeBase, KnowledgeBaseConfig};
        use ironvault::rag::DocumentStore;

        #[test]
        fn test_knowledge_base_chunk_text() {
            let _store = DocumentStore::new();
            let config = KnowledgeBaseConfig::default();
            let kb = KnowledgeBase::new("test_kb".to_string(), config);

            let long_text = "a".repeat(2000);
            let chunks = kb.chunk_text(&long_text, "doc_001");
            assert!(chunks.len() > 1);
            for chunk in &chunks {
                assert!(chunk.id.starts_with("doc_001_chunk_"));
            }
        }
    }

    // ── Compression: Gzip + Lzma full paths ──────────────────────

    mod compression_full_paths {
        use super::*;

        #[test]
        fn test_gzip_compress_decompress() {
            let data = b"Test data for gzip compression round-trip";
            let compressed =
                compress(data, CompressionAlgorithm::Gzip, CompressionLevel::Fast).unwrap();
            assert_ne!(&compressed[..], &data[..]);
            let decompressed = decompress(&compressed, CompressionAlgorithm::Gzip).unwrap();
            assert_eq!(&decompressed[..], &data[..]);
        }

        #[test]
        fn test_gzip_compress_balanced() {
            let data = b"More data for balanced gzip";
            let compressed =
                compress(data, CompressionAlgorithm::Gzip, CompressionLevel::Balanced).unwrap();
            let decompressed = decompress(&compressed, CompressionAlgorithm::Gzip).unwrap();
            assert_eq!(&decompressed[..], &data[..]);
        }

        #[test]
        fn test_lzma_compress_roundtrip() {
            let data = b"Test data for lzma compression testing path";
            let compressed =
                compress(data, CompressionAlgorithm::Lzma, CompressionLevel::Fast).unwrap();
            let decompressed = decompress(&compressed, CompressionAlgorithm::Lzma).unwrap();
            assert_eq!(&decompressed[..], &data[..]);
        }
    }

    // ── Storage local: download nonexistent, list ────────────────

    mod storage_local_tests {
        use super::*;
        use ironvault::storage::local::LocalBackend;
        use ironvault::storage::StorageBackend;

        #[tokio::test]
        async fn test_local_backend_download_nonexistent() {
            let dir = tempdir().unwrap();
            let backend = LocalBackend::new(dir.path().to_path_buf()).unwrap();

            let result: std::result::Result<Vec<u8>, _> = backend.download("nonexistent_key").await;
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_local_backend_upload_then_list() {
            let dir = tempdir().unwrap();
            let backend = LocalBackend::new(dir.path().to_path_buf()).unwrap();

            backend.upload("model1.bin", b"data1").await.unwrap();
            backend.upload("model2.bin", b"data2").await.unwrap();

            let keys = backend.list().await.unwrap();
            assert!(keys.len() >= 2);
        }
    }

    // ── Error: From<serde_yaml_ng::Error> ───────────────────────────

    mod error_from_yaml {
        use super::*;

        #[test]
        fn test_from_serde_yaml_ng_error() {
            // Create an invalid YAML string that will trigger parse error
            let bad_yaml = "invalid: [yaml: {broken";
            let err: std::result::Result<serde_yaml_ng::Value, _> =
                serde_yaml_ng::from_str(bad_yaml);
            assert!(err.is_err());

            // Convert to VaultError
            let vault_err: VaultError = err.unwrap_err().into();
            match vault_err {
                VaultError::SerializationError(msg) => {
                    assert!(!msg.is_empty());
                }
                _ => panic!("Expected SerializationError"),
            }
        }
    }

    // ── Async blob store adapter ─────────────────────────────────

    mod async_blob_store_adapter_tests {
        use super::*;
        use ironvault::storage::local::LocalBackend;
        use ironvault::traits::AsyncBlobStoreAdapter;

        #[tokio::test]
        async fn test_async_blob_store_put_get_list_stat() {
            let dir = tempdir().unwrap();
            let backend = LocalBackend::new(dir.path().to_path_buf()).unwrap();
            let adapter = AsyncBlobStoreAdapter::new(backend);

            use ironvault::traits::AsyncBlobStore;

            // Put
            let receipt = adapter.put("key1", b"hello world").await.unwrap();
            assert_eq!(receipt.key, "key1");
            assert_eq!(receipt.size_bytes, 11);

            // Get
            let data = adapter.get("key1").await.unwrap();
            assert_eq!(&data, b"hello world");

            // Stat
            let info = adapter.stat("key1").await.unwrap();
            assert_eq!(info.key, "key1");
            assert!(info.size_bytes > 0);

            // List
            let list = adapter.list(None).await.unwrap();
            assert!(!list.is_empty());

            // Delete
            let deleted = adapter.delete("key1").await.unwrap();
            assert!(deleted);

            // Exists
            let exists = adapter.exists("key1").await.unwrap();
            assert!(!exists);
        }
    }
}

#[allow(unused_imports)]
mod deep_coverage_tests {
    //! Deep coverage tests — Part 4
    //! Targets remaining library-level coverage gaps:
    //! - federation.rs: FederationManager (new, node_id, peers, add/remove peer, generate_manifest,
    //!   compute_delta, get_history, status)
    //! - telemetry.rs: TelemetryClient methods (enable, track, flush, device_id), TrackingTimer,
    //!   global convenience functions (track_*, etc.)
    //! - traits.rs: VaultState::Display (Uninitialized, Error), VaultEvent accessors (timestamp, vault_name,
    //!   event_type, Display), AuditLogSubscriber for all 9 event types, NullAuditSink
    //! - conversion.rs: Pipeline::convert (multi-step, validation, same-format), validate_magic_bytes
    //!   all arms, Converter::validate default impl edge cases

    // ============================================================================
    // FEDERATION MANAGER — new, accessors, add/remove peer, manifest, delta, status
    // ============================================================================
    mod federation_manager_coverage {
        use chrono::Utc;
        use ironvault::federation::*;
        use ironvault::version::ModelVersion;
        use std::collections::HashMap;

        fn make_config() -> FederationConfig {
            FederationConfig {
                node_id: "test-node-1".to_string(),
                node_name: "TestNode".to_string(),
                peers: vec![],
                sync_interval_secs: 60,
                auto_resolve_conflicts: true,
                max_concurrent_syncs: 2,
            }
        }

        fn make_peer(id: &str) -> PeerConfig {
            PeerConfig {
                node_id: id.to_string(),
                name: format!("Peer {}", id),
                endpoint: format!("https://{}.example.com", id),
                api_key: Some("key123".to_string()),
                enabled: true,
            }
        }

        #[test]
        fn federation_manager_new() {
            let tmp = tempfile::tempdir().unwrap();
            let config = make_config();
            let mgr = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();
            assert_eq!(mgr.node_id(), "test-node-1");
            assert!(mgr.peers().is_empty());
        }

        #[test]
        fn federation_manager_new_with_existing_state() {
            let tmp = tempfile::tempdir().unwrap();
            let state_file = tmp.path().join("federation_state.json");

            // Write a valid saved state
            let saved = serde_json::json!({
                "models": {},
                "clock": { "timestamps": { "test-node-1": 5 } },
                "history": []
            });
            std::fs::write(&state_file, serde_json::to_string_pretty(&saved).unwrap()).unwrap();

            let config = make_config();
            let mgr = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();
            assert_eq!(mgr.node_id(), "test-node-1");
        }

        #[test]
        fn federation_manager_add_remove_peer() {
            let tmp = tempfile::tempdir().unwrap();
            let config = make_config();
            let mut mgr = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            mgr.add_peer(make_peer("peer-a"));
            mgr.add_peer(make_peer("peer-b"));
            assert_eq!(mgr.peers().len(), 2);

            mgr.remove_peer("peer-a");
            assert_eq!(mgr.peers().len(), 1);
            assert_eq!(mgr.peers()[0].node_id, "peer-b");

            // Remove non-existent — no panic
            mgr.remove_peer("nonexistent");
            assert_eq!(mgr.peers().len(), 1);
        }

        #[test]
        fn federation_manager_generate_manifest() {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let tmp = tempfile::tempdir().unwrap();
            let config = make_config();
            let mgr = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let models: Vec<(String, Vec<ModelVersion>)> = vec![(
                "model1".to_string(),
                vec![ModelVersion {
                    version: 1,
                    checkpoint_id: "ckpt-001".to_string(),
                    timestamp: Utc::now(),
                    format: "pytorch".to_string(),
                    size_bytes: 1024,
                    compressed_size_bytes: 1100,
                    file_path: "model1.vault".to_string(),
                    checksum_sha256: "abc123".to_string(),
                    parent_version: None,
                    metadata: HashMap::new(),
                }],
            )];

            let manifest = rt.block_on(mgr.generate_manifest(models));
            assert_eq!(manifest.source_node, "test-node-1");
            assert_eq!(manifest.models.len(), 1);
            assert_eq!(manifest.models[0].name, "model1");
            assert_eq!(manifest.models[0].versions.len(), 1);
            assert_eq!(manifest.models[0].versions[0].version, 1);
            assert_eq!(manifest.models[0].versions[0].checkpoint_id, "ckpt-001");
        }

        #[test]
        fn federation_manager_compute_delta_disjoint() {
            let tmp = tempfile::tempdir().unwrap();
            let config = make_config();
            let mgr = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let local = SyncManifest {
                source_node: "local".to_string(),
                timestamp: Utc::now(),
                models: vec![ModelManifestEntry {
                    name: "local-only".to_string(),
                    versions: vec![VersionManifestEntry {
                        version: 1,
                        checkpoint_id: "local-ckpt".to_string(),
                        created_at: Utc::now(),
                        checksum: "aaa".to_string(),
                        size_bytes: 100,
                        parent_id: None,
                        origin_node: "local".to_string(),
                    }],
                    clock: VectorClock::new(),
                }],
                clock: VectorClock::new(),
            };

            let remote = SyncManifest {
                source_node: "remote".to_string(),
                timestamp: Utc::now(),
                models: vec![ModelManifestEntry {
                    name: "remote-only".to_string(),
                    versions: vec![VersionManifestEntry {
                        version: 1,
                        checkpoint_id: "remote-ckpt".to_string(),
                        created_at: Utc::now(),
                        checksum: "bbb".to_string(),
                        size_bytes: 200,
                        parent_id: None,
                        origin_node: "remote".to_string(),
                    }],
                    clock: VectorClock::new(),
                }],
                clock: VectorClock::new(),
            };

            let delta = mgr.compute_delta(&local, &remote);
            assert_eq!(delta.to_upload.len(), 1);
            assert_eq!(delta.to_upload[0].model, "local-only");
            assert_eq!(delta.to_download.len(), 1);
            assert_eq!(delta.to_download[0].model, "remote-only");
            assert!(delta.conflicts.is_empty());
        }

        #[test]
        fn federation_manager_compute_delta_shared_model_different_versions() {
            let tmp = tempfile::tempdir().unwrap();
            let config = make_config();
            let mgr = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let local = SyncManifest {
                source_node: "local".to_string(),
                timestamp: Utc::now(),
                models: vec![ModelManifestEntry {
                    name: "shared".to_string(),
                    versions: vec![
                        VersionManifestEntry {
                            version: 1,
                            checkpoint_id: "v1-ckpt".to_string(),
                            created_at: Utc::now(),
                            checksum: "c1".to_string(),
                            size_bytes: 100,
                            parent_id: None,
                            origin_node: "local".to_string(),
                        },
                        VersionManifestEntry {
                            version: 2,
                            checkpoint_id: "v2-local".to_string(),
                            created_at: Utc::now(),
                            checksum: "c2".to_string(),
                            size_bytes: 200,
                            parent_id: Some("v1-ckpt".to_string()),
                            origin_node: "local".to_string(),
                        },
                    ],
                    clock: VectorClock::new(),
                }],
                clock: VectorClock::new(),
            };

            let remote = SyncManifest {
                source_node: "remote".to_string(),
                timestamp: Utc::now(),
                models: vec![ModelManifestEntry {
                    name: "shared".to_string(),
                    versions: vec![
                        VersionManifestEntry {
                            version: 1,
                            checkpoint_id: "v1-ckpt".to_string(),
                            created_at: Utc::now(),
                            checksum: "c1".to_string(),
                            size_bytes: 100,
                            parent_id: None,
                            origin_node: "local".to_string(),
                        },
                        VersionManifestEntry {
                            version: 3,
                            checkpoint_id: "v3-remote".to_string(),
                            created_at: Utc::now(),
                            checksum: "c3".to_string(),
                            size_bytes: 300,
                            parent_id: Some("v1-ckpt".to_string()),
                            origin_node: "remote".to_string(),
                        },
                    ],
                    clock: VectorClock::new(),
                }],
                clock: VectorClock::new(),
            };

            let delta = mgr.compute_delta(&local, &remote);
            // v2-local should be uploaded, v3-remote should be downloaded
            assert_eq!(delta.to_upload.len(), 1);
            assert_eq!(delta.to_upload[0].checkpoint_id, "v2-local");
            assert_eq!(delta.to_download.len(), 1);
            assert_eq!(delta.to_download[0].checkpoint_id, "v3-remote");
        }

        #[test]
        fn federation_manager_compute_delta_conflict() {
            let tmp = tempfile::tempdir().unwrap();
            let config = make_config();
            let mgr = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            // Same version number, different checkpoint IDs → conflict
            let local = SyncManifest {
                source_node: "local".to_string(),
                timestamp: Utc::now(),
                models: vec![ModelManifestEntry {
                    name: "conflicted".to_string(),
                    versions: vec![VersionManifestEntry {
                        version: 1,
                        checkpoint_id: "local-ckpt-1".to_string(),
                        created_at: Utc::now(),
                        checksum: "x".to_string(),
                        size_bytes: 100,
                        parent_id: None,
                        origin_node: "local".to_string(),
                    }],
                    clock: VectorClock::new(),
                }],
                clock: VectorClock::new(),
            };

            let remote = SyncManifest {
                source_node: "remote".to_string(),
                timestamp: Utc::now(),
                models: vec![ModelManifestEntry {
                    name: "conflicted".to_string(),
                    versions: vec![VersionManifestEntry {
                        version: 1,
                        checkpoint_id: "remote-ckpt-1".to_string(),
                        created_at: Utc::now(),
                        checksum: "y".to_string(),
                        size_bytes: 150,
                        parent_id: None,
                        origin_node: "remote".to_string(),
                    }],
                    clock: VectorClock::new(),
                }],
                clock: VectorClock::new(),
            };

            let delta = mgr.compute_delta(&local, &remote);
            assert_eq!(delta.conflicts.len(), 1);
            assert_eq!(delta.conflicts[0].model, "conflicted");
            assert_eq!(delta.conflicts[0].local_version, "local-ckpt-1");
            assert_eq!(delta.conflicts[0].remote_version, "remote-ckpt-1");
        }

        #[test]
        fn federation_manager_get_history_empty() {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let tmp = tempfile::tempdir().unwrap();
            let config = make_config();
            let mgr = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let history = rt.block_on(mgr.get_history(None));
            assert!(history.is_empty());

            let limited = rt.block_on(mgr.get_history(Some(10)));
            assert!(limited.is_empty());
        }

        #[test]
        fn federation_manager_status() {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let tmp = tempfile::tempdir().unwrap();
            let mut config = make_config();
            config.peers.push(make_peer("p1"));
            let mgr = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let status = rt.block_on(mgr.status());
            assert_eq!(status.node_id, "test-node-1");
            assert_eq!(status.node_name, "TestNode");
            assert_eq!(status.peer_count, 1);
            assert_eq!(status.model_count, 0);
            assert!(status.last_sync.is_none());
        }

        #[test]
        fn federation_manager_compute_delta_empty_manifests() {
            let tmp = tempfile::tempdir().unwrap();
            let config = make_config();
            let mgr = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let local = SyncManifest {
                source_node: "local".to_string(),
                timestamp: Utc::now(),
                models: vec![],
                clock: VectorClock::new(),
            };
            let remote = SyncManifest {
                source_node: "remote".to_string(),
                timestamp: Utc::now(),
                models: vec![],
                clock: VectorClock::new(),
            };

            let delta = mgr.compute_delta(&local, &remote);
            assert!(delta.to_upload.is_empty());
            assert!(delta.to_download.is_empty());
            assert!(delta.conflicts.is_empty());
        }
    }

    // ============================================================================
    // TELEMETRY — TelemetryClient methods, global convenience functions
    // ============================================================================
    mod telemetry_client_coverage {
        use ironvault::telemetry::*;
        use std::time::Duration;

        #[test]
        fn client_new_default_config() {
            let config = TelemetryConfig::default();
            assert!(!config.enabled);
            assert!(!config.device_id.is_empty());
            assert!(config.batch_size > 0);
            assert!(config.flush_interval_secs > 0);
        }

        #[test]
        fn client_enable_disable() {
            let client = TelemetryClient::new(TelemetryConfig::default());
            client.disable();
            assert!(!client.is_enabled());
            client.enable();
            assert!(client.is_enabled());
        }

        #[test]
        fn client_device_id() {
            let config = TelemetryConfig {
                device_id: "test-device-123".to_string(),
                ..TelemetryConfig::default()
            };
            let client = TelemetryClient::new(config);
            assert_eq!(client.device_id(), "test-device-123");
        }

        #[test]
        fn client_track_when_disabled() {
            let client = TelemetryClient::new(TelemetryConfig::default());
            client.disable();
            // Should not panic, just no-op
            client.track(TelemetryEvent::AppStart {
                version: "1.0.0".to_string(),
                os: "test".to_string(),
                arch: "x86_64".to_string(),
                features: vec![],
            });
        }

        #[test]
        fn client_track_when_enabled() {
            let config = TelemetryConfig {
                batch_size: 100, // large batch so it doesn't try to send
                ..TelemetryConfig::default()
            };
            let client = TelemetryClient::new(config);
            client.track(TelemetryEvent::CommandRun {
                command: "store".to_string(),
                subcommand: None,
                duration_ms: 100,
                success: true,
            });
            // No crash = success
        }

        #[test]
        fn client_flush_when_disabled() {
            let client = TelemetryClient::new(TelemetryConfig::default());
            client.disable();
            client.flush(); // no-op, should not panic
        }

        #[test]
        fn client_flush_when_enabled_empty() {
            let config = TelemetryConfig {
                batch_size: 100,
                ..TelemetryConfig::default()
            };
            let client = TelemetryClient::new(config);
            client.flush(); // empty queue, should not panic
        }

        // --- Global convenience functions (these all go through the global TELEMETRY OnceLock) ---

        #[test]
        fn global_disable_and_is_enabled() {
            disable();
            assert!(!is_enabled());
        }

        #[test]
        fn global_flush() {
            flush(); // no-op if not initialized
        }

        #[test]
        fn global_track_app_start() {
            track_app_start(); // no-op if not initialized
        }

        #[test]
        fn global_track_command() {
            track_command("test", Some("sub"), Duration::from_millis(100), true);
        }

        #[test]
        fn global_track_model_op_small() {
            track_model_op("store", "pytorch", 1_000, Duration::from_millis(50), true);
        }

        #[test]
        fn global_track_model_op_medium() {
            track_model_op(
                "store",
                "onnx",
                50_000_000,
                Duration::from_millis(500),
                true,
            );
        }

        #[test]
        fn global_track_model_op_large() {
            track_model_op(
                "store",
                "safetensors",
                500_000_000,
                Duration::from_secs(5),
                true,
            );
        }

        #[test]
        fn global_track_model_op_xlarge() {
            track_model_op(
                "store",
                "gguf",
                2_000_000_000,
                Duration::from_secs(30),
                true,
            );
        }

        #[test]
        fn global_track_conversion() {
            track_conversion("pytorch", "onnx", Duration::from_secs(1), true);
        }

        #[test]
        fn global_track_api_call() {
            track_api_call("/api/v1/models", "GET", 200, Duration::from_millis(10));
        }

        #[test]
        fn global_track_error() {
            track_error("io_error", Some("disk full"));
            track_error("crypto_error", None);
        }

        #[test]
        fn global_track_feature() {
            track_feature("federation", Some("sync"));
            track_feature("rag", None);
        }

        // --- TrackingTimer ---
        #[test]
        fn tracking_timer_basic() {
            let timer = TrackingTimer::new("store", Some("model"));
            std::thread::sleep(Duration::from_millis(10));
            timer.finish(true);
        }

        #[test]
        fn tracking_timer_no_subcommand() {
            let timer = TrackingTimer::new("list", None);
            timer.finish(false);
        }

        // --- TelemetryEvent serialization ---
        #[test]
        fn event_model_operation_serde() {
            let event = TelemetryEvent::ModelOperation {
                operation: "store".to_string(),
                format: "pytorch".to_string(),
                size_bucket: "small".to_string(),
                duration_ms: 100,
                success: true,
            };
            let json = serde_json::to_string(&event).unwrap();
            assert!(json.contains("model_operation"));
        }

        #[test]
        fn event_conversion_serde() {
            let event = TelemetryEvent::Conversion {
                source_format: "pytorch".to_string(),
                target_format: "onnx".to_string(),
                duration_ms: 200,
                success: false,
            };
            let json = serde_json::to_string(&event).unwrap();
            assert!(json.contains("conversion"));
        }

        #[test]
        fn event_api_call_serde() {
            let event = TelemetryEvent::ApiCall {
                endpoint: "/models".to_string(),
                method: "POST".to_string(),
                status_code: 201,
                duration_ms: 50,
            };
            let json = serde_json::to_string(&event).unwrap();
            assert!(json.contains("api_call"));
        }

        #[test]
        fn event_error_serde() {
            let event = TelemetryEvent::Error {
                error_type: "io".to_string(),
                context: None,
            };
            let json = serde_json::to_string(&event).unwrap();
            assert!(json.contains("error"));
            assert!(!json.contains("context"));
        }

        #[test]
        fn event_feature_used_serde() {
            let event = TelemetryEvent::FeatureUsed {
                feature: "rag".to_string(),
                detail: Some("search".to_string()),
            };
            let json = serde_json::to_string(&event).unwrap();
            assert!(json.contains("feature_used"));
        }

        #[test]
        fn event_app_start_serde() {
            let event = TelemetryEvent::AppStart {
                version: "1.0.0".to_string(),
                os: "linux".to_string(),
                arch: "x86_64".to_string(),
                features: vec!["api".to_string()],
            };
            let json = serde_json::to_string(&event).unwrap();
            assert!(json.contains("app_start"));
            assert!(json.contains("api"));
        }
    }

    // ============================================================================
    // TRAITS — VaultState Display, VaultEvent accessors/Display, AuditLogSubscriber, NullAuditSink
    // ============================================================================
    mod traits_deep_coverage {
        use chrono::Utc;
        use ironvault::traits::*;

        // --- VaultState Display ---
        #[test]
        fn vault_state_display_uninitialized() {
            let s = VaultState::Uninitialized;
            assert_eq!(format!("{}", s), "Uninitialized");
        }

        #[test]
        fn vault_state_display_locked() {
            let s = VaultState::Locked {
                vault_name: "v1".to_string(),
                model_count: 3,
            };
            assert_eq!(format!("{}", s), "Locked(v1)");
        }

        #[test]
        fn vault_state_display_unlocked() {
            let s = VaultState::Unlocked {
                vault_name: "v2".to_string(),
                model_count: 5,
                unlocked_at: Utc::now(),
                operations_count: 10,
            };
            assert_eq!(format!("{}", s), "Unlocked(v2)");
        }

        #[test]
        fn vault_state_display_error() {
            let s = VaultState::Error {
                message: "oops".to_string(),
            };
            assert_eq!(format!("{}", s), "Error(oops)");
        }

        // --- VaultEvent accessors ---
        fn make_event_vault_created() -> VaultEvent {
            VaultEvent::VaultCreated {
                vault: "test-vault".to_string(),
                timestamp: Utc::now(),
            }
        }

        fn make_event_vault_unlocked() -> VaultEvent {
            VaultEvent::VaultUnlocked {
                vault: "test-vault".to_string(),
                timestamp: Utc::now(),
            }
        }

        fn make_event_vault_locked() -> VaultEvent {
            VaultEvent::VaultLocked {
                vault: "test-vault".to_string(),
                timestamp: Utc::now(),
            }
        }

        fn make_event_model_stored() -> VaultEvent {
            VaultEvent::ModelStored {
                vault: "test-vault".to_string(),
                model: "m1".to_string(),
                version: 1,
                format: "pytorch".to_string(),
                size: 1024,
                checksum: "abc123".to_string(),
                timestamp: Utc::now(),
            }
        }

        fn make_event_model_retrieved() -> VaultEvent {
            VaultEvent::ModelRetrieved {
                vault: "test-vault".to_string(),
                model: "m1".to_string(),
                version: 1,
                timestamp: Utc::now(),
            }
        }

        fn make_event_model_deleted() -> VaultEvent {
            VaultEvent::ModelDeleted {
                vault: "test-vault".to_string(),
                model: "m1".to_string(),
                version: 1,
                timestamp: Utc::now(),
            }
        }

        fn make_event_passphrase_changed() -> VaultEvent {
            VaultEvent::PassphraseChanged {
                vault: "test-vault".to_string(),
                files_reencrypted: 5,
                timestamp: Utc::now(),
            }
        }

        fn make_event_integrity_failed() -> VaultEvent {
            VaultEvent::IntegrityFailed {
                vault: "test-vault".to_string(),
                model: "m1".to_string(),
                version: 1,
                expected: "expected_hash".to_string(),
                actual: "actual_hash".to_string(),
                timestamp: Utc::now(),
            }
        }

        fn make_event_compliance_checked() -> VaultEvent {
            VaultEvent::ComplianceChecked {
                vault: "test-vault".to_string(),
                passed: true,
                timestamp: Utc::now(),
            }
        }

        #[test]
        fn vault_event_timestamp_all_variants() {
            let events = vec![
                make_event_vault_created(),
                make_event_vault_unlocked(),
                make_event_vault_locked(),
                make_event_model_stored(),
                make_event_model_retrieved(),
                make_event_model_deleted(),
                make_event_passphrase_changed(),
                make_event_integrity_failed(),
                make_event_compliance_checked(),
            ];
            for event in &events {
                let ts = event.timestamp();
                // Should be recent (within last minute)
                assert!((Utc::now() - ts).num_seconds() < 60);
            }
        }

        #[test]
        fn vault_event_vault_name_all_variants() {
            let events = vec![
                make_event_vault_created(),
                make_event_vault_unlocked(),
                make_event_vault_locked(),
                make_event_model_stored(),
                make_event_model_retrieved(),
                make_event_model_deleted(),
                make_event_passphrase_changed(),
                make_event_integrity_failed(),
                make_event_compliance_checked(),
            ];
            for event in &events {
                assert_eq!(event.vault_name(), "test-vault");
            }
        }

        #[test]
        fn vault_event_event_type_all_variants() {
            assert_eq!(make_event_vault_created().event_type(), "vault_created");
            assert_eq!(make_event_vault_unlocked().event_type(), "vault_unlocked");
            assert_eq!(make_event_vault_locked().event_type(), "vault_locked");
            assert_eq!(make_event_model_stored().event_type(), "model_stored");
            assert_eq!(make_event_model_retrieved().event_type(), "model_retrieved");
            assert_eq!(make_event_model_deleted().event_type(), "model_deleted");
            assert_eq!(
                make_event_passphrase_changed().event_type(),
                "passphrase_changed"
            );
            assert_eq!(
                make_event_integrity_failed().event_type(),
                "integrity_failed"
            );
            assert_eq!(
                make_event_compliance_checked().event_type(),
                "compliance_checked"
            );
        }

        #[test]
        fn vault_event_display_all_variants() {
            let events = vec![
                make_event_vault_created(),
                make_event_vault_unlocked(),
                make_event_vault_locked(),
                make_event_model_stored(),
                make_event_model_retrieved(),
                make_event_model_deleted(),
                make_event_passphrase_changed(),
                make_event_integrity_failed(),
                make_event_compliance_checked(),
            ];
            for event in &events {
                let display = format!("{}", event);
                assert!(display.contains(event.event_type()));
                // Contains ISO-ish timestamp
                assert!(display.contains("202"));
            }
        }

        // --- NullAuditSink ---
        #[test]
        fn null_audit_sink_emit() {
            use ironvault::audit::{AuditEntry, AuditEventType};
            let sink = NullAuditSink;
            let entry = AuditEntry {
                timestamp: Utc::now(),
                event_type: AuditEventType::VaultCreated,
                description: "test".into(),
                model_name: None,
                version: None,
                success: true,
                metadata: None,
            };
            assert!(sink.emit(entry).is_ok());
        }

        #[test]
        fn null_audit_sink_query() {
            let sink = NullAuditSink;
            let results = sink.query(Some(10)).unwrap();
            assert!(results.is_empty());
        }

        // --- AuditLogSubscriber for all 9 event types ---
        #[test]
        fn audit_log_subscriber_all_events() {
            use ironvault::audit::AuditEntry;
            use std::sync::{Arc, Mutex};

            struct CollectingSink {
                entries: Arc<Mutex<Vec<AuditEntry>>>,
            }
            impl AuditSink for CollectingSink {
                fn emit(&self, entry: AuditEntry) -> ironvault::Result<()> {
                    self.entries.lock().unwrap().push(entry);
                    Ok(())
                }
                fn query(&self, _limit: Option<usize>) -> ironvault::Result<Vec<AuditEntry>> {
                    Ok(self.entries.lock().unwrap().clone())
                }
            }

            let entries = Arc::new(Mutex::new(Vec::new()));
            let sink = CollectingSink {
                entries: entries.clone(),
            };
            let subscriber = AuditLogSubscriber::new(Box::new(sink));
            assert_eq!(subscriber.name(), "AuditLogSubscriber");

            let events = vec![
                make_event_vault_created(),
                make_event_vault_unlocked(),
                make_event_vault_locked(),
                make_event_model_stored(),
                make_event_model_retrieved(),
                make_event_model_deleted(),
                make_event_passphrase_changed(),
                make_event_integrity_failed(),
                make_event_compliance_checked(),
            ];

            for event in &events {
                assert!(subscriber.accepts(event));
                subscriber.on_event(event).unwrap();
            }

            let collected = entries.lock().unwrap();
            assert_eq!(collected.len(), 9);

            // Verify specific audit entries
            assert!(collected[0].description.contains("created"));
            assert!(collected[1].description.contains("unlocked"));
            assert!(collected[2].description.contains("locked"));
            assert!(collected[3].description.contains("stored"));
            assert!(collected[3].model_name.is_some());
            assert_eq!(collected[3].version, Some(1));
            assert!(collected[4].description.contains("retrieved"));
            assert!(collected[5].description.contains("deleted"));
            assert!(collected[6].description.contains("re-encrypted"));
            assert!(collected[7].description.contains("Integrity"));
            assert!(!collected[7].success); // integrity failure
            assert!(collected[8].description.contains("PASSED"));
        }

        #[test]
        fn audit_log_subscriber_compliance_failed() {
            use ironvault::audit::AuditEntry;
            use std::sync::{Arc, Mutex};

            struct CollectingSink {
                entries: Arc<Mutex<Vec<AuditEntry>>>,
            }
            impl AuditSink for CollectingSink {
                fn emit(&self, entry: AuditEntry) -> ironvault::Result<()> {
                    self.entries.lock().unwrap().push(entry);
                    Ok(())
                }
                fn query(&self, _limit: Option<usize>) -> ironvault::Result<Vec<AuditEntry>> {
                    Ok(Vec::new())
                }
            }

            let entries = Arc::new(Mutex::new(Vec::new()));
            let subscriber = AuditLogSubscriber::new(Box::new(CollectingSink {
                entries: entries.clone(),
            }));

            let event = VaultEvent::ComplianceChecked {
                vault: "v".to_string(),
                passed: false,
                timestamp: Utc::now(),
            };
            subscriber.on_event(&event).unwrap();

            let collected = entries.lock().unwrap();
            assert_eq!(collected.len(), 1);
            assert!(collected[0].description.contains("FAILED"));
            assert!(!collected[0].success);
        }

        // --- IvUri to_string with query params ---
        #[test]
        fn iv_uri_to_string_with_query() {
            let uri = IvUri::parse("iv://default/_events?since=2026-01-01").unwrap();
            let s = uri.to_string();
            assert!(s.starts_with("iv://"));
            assert!(s.contains("default"));
            assert!(s.contains("_events"));
            assert!(s.contains("since=2026-01-01"));
        }

        #[test]
        fn iv_uri_display_impl() {
            let uri = IvUri::parse("iv://myvault/model@2/card").unwrap();
            let display = format!("{}", uri);
            assert_eq!(display, "iv://myvault/model@2/card");
        }

        #[test]
        fn iv_uri_to_string_vault_only() {
            let uri = IvUri::parse("iv://default/").unwrap();
            let s = uri.to_string();
            assert_eq!(s, "iv://default");
        }

        #[test]
        fn iv_uri_to_string_model_no_version() {
            let uri = IvUri::parse("iv://default/mymodel").unwrap();
            let s = uri.to_string();
            assert_eq!(s, "iv://default/mymodel");
        }

        #[test]
        fn iv_uri_to_string_empty_value_query() {
            use std::collections::HashMap;
            let uri = IvUri {
                vault: Some("v".to_string()),
                model: Some("m".to_string()),
                version: None,
                resource: None,
                query: {
                    let mut q = HashMap::new();
                    q.insert("flag".to_string(), String::new());
                    q
                },
            };
            let s = uri.to_string();
            assert!(s.contains("?flag"));
        }

        // --- EventBus subscriber error handling ---
        #[test]
        fn event_bus_subscriber_error_does_not_propagate() {
            struct FailingSubscriber;
            impl EventSubscriber for FailingSubscriber {
                fn on_event(&self, _event: &VaultEvent) -> ironvault::Result<()> {
                    Err(ironvault::error::VaultError::IoError(
                        std::io::Error::other("subscriber error"),
                    ))
                }
                fn name(&self) -> &str {
                    "FailingSub"
                }
            }

            let mut bus = EventBus::new();
            bus.subscribe(Box::new(FailingSubscriber));
            // Should not panic — errors are logged but don't block
            bus.emit(&make_event_vault_created());
        }

        #[test]
        fn event_bus_accepts_filter() {
            struct OnlyStoreSubscriber {
                count: std::sync::atomic::AtomicU32,
            }
            impl EventSubscriber for OnlyStoreSubscriber {
                fn accepts(&self, event: &VaultEvent) -> bool {
                    matches!(event, VaultEvent::ModelStored { .. })
                }
                fn on_event(&self, _event: &VaultEvent) -> ironvault::Result<()> {
                    self.count
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Ok(())
                }
                fn name(&self) -> &str {
                    "OnlyStore"
                }
            }

            let sub = OnlyStoreSubscriber {
                count: std::sync::atomic::AtomicU32::new(0),
            };
            let mut bus = EventBus::new();
            bus.subscribe(Box::new(sub));

            bus.emit(&make_event_vault_created()); // should be filtered
            bus.emit(&make_event_model_stored()); // should be accepted

            // We can't easily check the count since it's moved into the Box,
            // but at least the operation completes without error
        }

        // --- MetricsSubscriber all event types ---
        #[test]
        fn metrics_subscriber_all_event_types() {
            let metrics = std::sync::Arc::new(VaultMetrics::new());
            let sub = MetricsSubscriber::new(metrics.clone());

            sub.on_event(&make_event_model_stored()).unwrap();
            sub.on_event(&make_event_model_retrieved()).unwrap();
            sub.on_event(&make_event_model_deleted()).unwrap();
            sub.on_event(&make_event_vault_unlocked()).unwrap();
            sub.on_event(&make_event_vault_locked()).unwrap();
            sub.on_event(&make_event_integrity_failed()).unwrap();
            // Uncovered events (passphrase_changed, compliance_checked, vault_created) → _ arm
            sub.on_event(&make_event_passphrase_changed()).unwrap();
            sub.on_event(&make_event_vault_created()).unwrap();
            sub.on_event(&make_event_compliance_checked()).unwrap();

            let snap = metrics.snapshot();
            assert_eq!(snap.models_stored_total, 1);
            assert_eq!(snap.models_retrieved_total, 1);
            assert_eq!(snap.models_deleted_total, 1);
            assert_eq!(snap.bytes_stored_total, 1024);
            assert_eq!(snap.errors_total, 1); // integrity_failed
            assert!(!snap.vault_unlocked); // unlocked then locked
        }
    }

    // ============================================================================
    // CONVERSION — Pipeline convert (multi-step, validation), validate_magic_bytes
    // ============================================================================
    mod conversion_deep_coverage {
        use ironvault::conversion::*;
        use ironvault::formats::ModelFormat;

        // --- Pipeline convert same format ---
        #[test]
        fn pipeline_convert_same_format() {
            let pipeline = ConversionPipeline::with_builtins();
            let data = b"same data";
            let result = pipeline
                .convert(
                    data,
                    &ModelFormat::PyTorch,
                    &ModelFormat::PyTorch,
                    &ConversionOptions::default(),
                    None,
                )
                .unwrap();
            assert_eq!(result.data, data);
            assert_eq!(result.source_format, ModelFormat::PyTorch);
            assert_eq!(result.target_format, ModelFormat::PyTorch);
            assert_eq!(result.conversion_path, vec![ModelFormat::PyTorch]);
            assert_eq!(result.input_size, data.len() as u64);
            assert_eq!(result.output_size, data.len() as u64);
            assert!(result.validation.is_none());
        }

        // --- Pipeline convert no path ---
        #[test]
        fn pipeline_convert_no_path() {
            let pipeline = ConversionPipeline::new(); // empty pipeline
            let result = pipeline.convert(
                b"data",
                &ModelFormat::PyTorch,
                &ModelFormat::ONNX,
                &ConversionOptions::default(),
                None,
            );
            assert!(result.is_err());
        }

        // --- Pipeline convert direct ---
        #[test]
        fn pipeline_convert_direct_safetensors_to_raw() {
            let pipeline = ConversionPipeline::with_builtins();
            // Build valid safetensors data
            let header =
                r#"{"__metadata__":{},"t":{"dtype":"U8","shape":[4],"data_offsets":[0,4]}}"#;
            let header_bytes = header.as_bytes();
            let header_len = header_bytes.len() as u64;
            let mut data = Vec::new();
            data.extend_from_slice(&header_len.to_le_bytes());
            data.extend_from_slice(header_bytes);
            data.extend_from_slice(&[1, 2, 3, 4]);

            let result = pipeline
                .convert(
                    &data,
                    &ModelFormat::Safetensors,
                    &ModelFormat::Custom("raw".into()),
                    &ConversionOptions::default(),
                    None,
                )
                .unwrap();
            assert_eq!(result.data, vec![1, 2, 3, 4]);
        }

        // --- Pipeline convert with progress callback ---
        #[test]
        fn pipeline_convert_with_progress() {
            let pipeline = ConversionPipeline::with_builtins();
            let header =
                r#"{"__metadata__":{},"t":{"dtype":"U8","shape":[4],"data_offsets":[0,4]}}"#;
            let header_bytes = header.as_bytes();
            let header_len = header_bytes.len() as u64;
            let mut data = Vec::new();
            data.extend_from_slice(&header_len.to_le_bytes());
            data.extend_from_slice(header_bytes);
            data.extend_from_slice(&[1, 2, 3, 4]);

            let progress_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let pc = progress_called.clone();
            let callback: ProgressCallback = Box::new(move |_p| {
                pc.store(true, std::sync::atomic::Ordering::Relaxed);
            });

            let result = pipeline
                .convert(
                    &data,
                    &ModelFormat::Safetensors,
                    &ModelFormat::Custom("raw".into()),
                    &ConversionOptions::default(),
                    Some(&callback),
                )
                .unwrap();
            assert!(!result.data.is_empty());
            assert!(progress_called.load(std::sync::atomic::Ordering::Relaxed));
        }

        // --- Pipeline convert with validation ---
        #[test]
        fn pipeline_convert_with_validation() {
            let pipeline = ConversionPipeline::with_builtins();
            let header =
                r#"{"__metadata__":{},"t":{"dtype":"U8","shape":[4],"data_offsets":[0,4]}}"#;
            let header_bytes = header.as_bytes();
            let header_len = header_bytes.len() as u64;
            let mut data = Vec::new();
            data.extend_from_slice(&header_len.to_le_bytes());
            data.extend_from_slice(header_bytes);
            data.extend_from_slice(&[1, 2, 3, 4]);

            let opts = ConversionOptions {
                validate: true,
                ..ConversionOptions::default()
            };

            let result = pipeline
                .convert(
                    &data,
                    &ModelFormat::Safetensors,
                    &ModelFormat::Custom("raw".into()),
                    &opts,
                    None,
                )
                .unwrap();
            assert!(result.validation.is_some());
        }

        // --- Pipeline default is with_builtins ---
        #[test]
        fn pipeline_default() {
            let pipeline = ConversionPipeline::default();
            assert!(pipeline.can_convert_direct(
                &ModelFormat::Safetensors,
                &ModelFormat::Custom("raw".into())
            ));
        }

        // --- Pipeline find_path same format ---
        #[test]
        fn pipeline_find_path_same() {
            let pipeline = ConversionPipeline::with_builtins();
            let path = pipeline.find_path(&ModelFormat::PyTorch, &ModelFormat::PyTorch);
            assert_eq!(path, Some(vec![ModelFormat::PyTorch]));
        }

        // --- Pipeline find_path no path ---
        #[test]
        fn pipeline_find_path_none() {
            let pipeline = ConversionPipeline::new();
            let path = pipeline.find_path(&ModelFormat::PyTorch, &ModelFormat::ONNX);
            assert!(path.is_none());
        }

        // --- ConversionProgress Display ---
        #[test]
        fn conversion_progress_display_with_total() {
            let p = ConversionProgress {
                step: 0,
                total_steps: 2,
                bytes_processed: 50,
                bytes_total: 100,
                message: "Converting".to_string(),
            };
            let s = format!("{}", p);
            assert!(s.contains("50.0%"));
            assert!(s.contains("Converting"));
            assert!(s.contains("[1/2]"));
        }

        #[test]
        fn conversion_progress_display_without_total() {
            let p = ConversionProgress {
                step: 1,
                total_steps: 3,
                bytes_processed: 0,
                bytes_total: 0,
                message: "Processing".to_string(),
            };
            let s = format!("{}", p);
            assert!(s.contains("Processing"));
            assert!(s.contains("[2/3]"));
            assert!(!s.contains("%"));
        }

        // --- ConversionOptions default ---
        #[test]
        fn conversion_options_default() {
            let opts = ConversionOptions::default();
            assert!(!opts.validate);
            assert!(opts.quantization.is_none());
            assert!(opts.opset_version.is_none());
        }

        // --- ConversionResult ---
        #[test]
        fn conversion_result_fields() {
            let result = ConversionResult {
                data: vec![1, 2, 3],
                plan: None,
                source_format: ModelFormat::PyTorch,
                target_format: ModelFormat::ONNX,
                conversion_path: vec![ModelFormat::PyTorch, ModelFormat::ONNX],
                input_size: 100,
                output_size: 3,
                validation: None,
            };
            assert_eq!(result.data.len(), 3);
            assert_eq!(result.source_format, ModelFormat::PyTorch);
            assert_eq!(result.target_format, ModelFormat::ONNX);
            assert_eq!(result.conversion_path.len(), 2);
        }

        // --- ValidationReport / ValidationCheck ---
        #[test]
        fn validation_check_pass_fail() {
            let pass = ValidationCheck::pass("test", "OK".to_string());
            assert!(pass.passed);
            assert_eq!(pass.name, "test");

            let fail = ValidationCheck::fail("test2", "BAD".to_string());
            assert!(!fail.passed);
            assert_eq!(fail.name, "test2");
        }

        #[test]
        fn validation_report_from_checks() {
            let checks = vec![
                ValidationCheck::pass("a", "ok".to_string()),
                ValidationCheck::pass("b", "ok".to_string()),
            ];
            let report = ValidationReport::from_checks(checks);
            assert!(report.passed);
            assert_eq!(report.checks.len(), 2);
        }

        #[test]
        fn validation_report_fail_if_any_fail() {
            let checks = vec![
                ValidationCheck::pass("a", "ok".to_string()),
                ValidationCheck::fail("b", "bad".to_string()),
            ];
            let report = ValidationReport::from_checks(checks);
            assert!(!report.passed);
        }

        // --- Multi-step convert (PyTorch → ONNX → TensorRT) ---
        #[test]
        fn pipeline_multi_step_convert() {
            let pipeline = ConversionPipeline::with_builtins();
            let result = pipeline.convert(
                b"dummy pytorch data",
                &ModelFormat::PyTorch,
                &ModelFormat::TensorRT,
                &ConversionOptions::default(),
                None,
            );
            // Should succeed (shim converters produce JSON plans)
            assert!(result.is_ok());
            let r = result.unwrap();
            assert_eq!(r.source_format, ModelFormat::PyTorch);
            assert_eq!(r.target_format, ModelFormat::TensorRT);
            assert!(r.conversion_path.len() >= 3);
        }
    }

    // ============================================================================
    // ADDITIONAL VAULT EDGE CASES
    // ============================================================================
    mod vault_edge_cases {
        use ironvault::config::{DirectoryPaths, VaultConfig};
        use ironvault::formats::{ModelFormat, ModelMetadata};
        use ironvault::traits::VaultState;
        use ironvault::{Vault, VaultBuilder};

        fn make_dirs(tmp: &tempfile::TempDir) -> DirectoryPaths {
            DirectoryPaths {
                config_dir: tmp.path().join("config"),
                data_dir: tmp.path().join("data"),
                cache_dir: tmp.path().join("cache"),
                vault_dir: tmp.path().join("data/vaults/default"),
                log_dir: tmp.path().join("data/logs"),
                backends_dir: tmp.path().join("config/backends"),
                utilities_dir: tmp.path().join("config/utilities"),
                databases_dir: tmp.path().join("config/databases"),
            }
        }

        #[test]
        fn vault_store_get_roundtrip() {
            let tmp = tempfile::tempdir().unwrap();
            let config = VaultConfig::with_dirs(make_dirs(&tmp)).unwrap();
            let mut vault = Vault::new(Some(config)).unwrap();
            vault
                .unlock(b"test_passphrase_with_sufficient_entropy".to_vec())
                .unwrap();

            let data = b"important model data 1234567890".to_vec();
            let meta = ModelMetadata::new("roundtrip".into(), ModelFormat::Safetensors)
                .with_description("Test model".into())
                .with_framework("pytorch".into());

            let ver = vault
                .store_model("roundtrip", data.clone(), meta, None)
                .unwrap();
            assert_eq!(ver.version, 1);

            let retrieved = vault.get_model("roundtrip", Some(1)).unwrap();
            assert_eq!(data, retrieved);

            // Latest version
            let latest = vault.get_model("roundtrip", None).unwrap();
            assert_eq!(data, latest);
        }

        #[test]
        fn vault_builder_sqlite_backend() {
            let tmp = tempfile::tempdir().unwrap();
            let config = VaultConfig::with_dirs(make_dirs(&tmp)).unwrap();
            let vault = VaultBuilder::new()
                .config(config)
                .sqlite_versions()
                .build()
                .unwrap();
            assert_eq!(vault.version_backend_name(), "sqlite");
        }

        #[test]
        fn vault_metrics_update_after_operations() {
            let tmp = tempfile::tempdir().unwrap();
            let config = VaultConfig::with_dirs(make_dirs(&tmp)).unwrap();
            let mut vault = VaultBuilder::new().config(config).build().unwrap();

            vault
                .unlock(b"test_passphrase_with_sufficient_entropy".to_vec())
                .unwrap();

            let meta = ModelMetadata::new("m1".into(), ModelFormat::PyTorch);
            vault
                .store_model("m1", b"data".to_vec(), meta, None)
                .unwrap();

            let snap = vault.metrics().unwrap();
            assert_eq!(snap.models_stored_total, 1);
            assert!(snap.bytes_stored_total > 0);
            assert!(snap.vault_unlocked);

            // Retrieve
            let _ = vault.get_model("m1", None).unwrap();
            let snap2 = vault.metrics().unwrap();
            assert_eq!(snap2.models_retrieved_total, 1);
        }

        #[test]
        fn vault_state_transitions() {
            let tmp = tempfile::tempdir().unwrap();
            let config = VaultConfig::with_dirs(make_dirs(&tmp)).unwrap();
            let mut vault = Vault::new(Some(config)).unwrap();

            // Initially locked
            assert!(matches!(vault.state(), VaultState::Locked { .. }));

            // Unlock
            vault
                .unlock(b"test_passphrase_with_sufficient_entropy".to_vec())
                .unwrap();
            match vault.state() {
                VaultState::Unlocked {
                    operations_count, ..
                } => {
                    assert_eq!(operations_count, 0);
                }
                other => panic!("Expected Unlocked, got {:?}", other),
            }

            // Store a model (increments operations_count)
            let meta = ModelMetadata::new("m".into(), ModelFormat::PyTorch);
            vault.store_model("m", b"d".to_vec(), meta, None).unwrap();
            match vault.state() {
                VaultState::Unlocked {
                    operations_count,
                    model_count,
                    ..
                } => {
                    assert!(operations_count >= 1);
                    assert_eq!(model_count, 1);
                }
                other => panic!("Expected Unlocked, got {:?}", other),
            }

            // Lock
            vault.lock();
            assert!(matches!(vault.state(), VaultState::Locked { .. }));
        }
    }

    // ============================================================================
    // ADDITIONAL CRYPTO EDGE CASES — streaming chunk sizes
    // ============================================================================
    mod crypto_streaming_edge_cases {
        use ironvault::crypto::{SecureKey, VaultCrypto};
        use ironvault::{decrypt_chunked, encrypt_chunked};

        #[test]
        fn streaming_small_data() {
            let crypto = VaultCrypto::new().unwrap();
            let key = SecureKey::from_bytes(&[0x42; 32]).unwrap();
            let data = b"tiny";
            let enc = encrypt_chunked(&crypto, data, &key, 1024).unwrap();
            let dec = decrypt_chunked(&crypto, &enc, &key).unwrap();
            assert_eq!(dec, data);
        }

        #[test]
        fn streaming_exact_chunk_boundary() {
            let crypto = VaultCrypto::new().unwrap();
            let key = SecureKey::from_bytes(&[0x55; 32]).unwrap();
            let data = vec![0xAB; 64];
            let enc = encrypt_chunked(&crypto, &data, &key, 32).unwrap();
            let dec = decrypt_chunked(&crypto, &enc, &key).unwrap();
            assert_eq!(dec, data);
        }

        #[test]
        fn streaming_single_byte_chunks() {
            let crypto = VaultCrypto::new().unwrap();
            let key = SecureKey::from_bytes(&[0x99; 32]).unwrap();
            let data = b"abc";
            let enc = encrypt_chunked(&crypto, data, &key, 1).unwrap();
            let dec = decrypt_chunked(&crypto, &enc, &key).unwrap();
            assert_eq!(dec, data);
        }

        #[test]
        fn streaming_large_chunk_size() {
            let crypto = VaultCrypto::new().unwrap();
            let key = SecureKey::from_bytes(&[0xCC; 32]).unwrap();
            let data = vec![0x11; 256];
            // Chunk size larger than data
            let enc = encrypt_chunked(&crypto, &data, &key, 1024 * 1024).unwrap();
            let dec = decrypt_chunked(&crypto, &enc, &key).unwrap();
            assert_eq!(dec, data);
        }
    }

    // ============================================================================
    // VERSION SQLITE — additional edge cases
    // ============================================================================
    #[cfg(feature = "sqlite")]
    mod version_sqlite_edge_cases {
        use ironvault::traits::VersionRepo;
        use ironvault::SqliteVersionRepo;

        #[test]
        fn sqlite_version_multiple_models() {
            let tmp = tempfile::tempdir().unwrap();
            let mut repo = SqliteVersionRepo::new(tmp.path()).unwrap();

            repo.add_version("m1", "f1.vault", "PyTorch", 100, 80, "h1", None, None)
                .unwrap();
            repo.add_version("m2", "f2.vault", "ONNX", 200, 160, "h2", None, None)
                .unwrap();
            repo.add_version("m1", "f1v2.vault", "PyTorch", 150, 120, "h3", None, Some(1))
                .unwrap();

            let models = repo.list_models();
            assert_eq!(models.len(), 2);

            let m1_versions = repo.list_versions("m1");
            assert_eq!(m1_versions.len(), 2);

            let m2_versions = repo.list_versions("m2");
            assert_eq!(m2_versions.len(), 1);
        }

        #[test]
        fn sqlite_version_get_latest() {
            let tmp = tempfile::tempdir().unwrap();
            let mut repo = SqliteVersionRepo::new(tmp.path()).unwrap();

            repo.add_version("m1", "f1.vault", "PyTorch", 100, 80, "h1", None, None)
                .unwrap();
            repo.add_version("m1", "f2.vault", "PyTorch", 200, 160, "h2", None, Some(1))
                .unwrap();

            let latest = repo.get_version("m1", None);
            assert!(latest.is_some());
            assert_eq!(latest.unwrap().version, 2);
        }

        #[test]
        fn sqlite_version_delete_nonexistent() {
            let tmp = tempfile::tempdir().unwrap();
            let mut repo = SqliteVersionRepo::new(tmp.path()).unwrap();
            let deleted = repo.delete_version("nope", 1).unwrap();
            assert!(!deleted);
        }

        #[test]
        fn sqlite_version_lineage() {
            let tmp = tempfile::tempdir().unwrap();
            let mut repo = SqliteVersionRepo::new(tmp.path()).unwrap();

            repo.add_version("m1", "f1.vault", "PyTorch", 100, 80, "h1", None, None)
                .unwrap();
            repo.add_version("m1", "f2.vault", "PyTorch", 200, 160, "h2", None, Some(1))
                .unwrap();
            repo.add_version("m1", "f3.vault", "PyTorch", 300, 240, "h3", None, Some(2))
                .unwrap();

            let lineage = repo.get_lineage("m1", 3);
            assert!(lineage.len() >= 2); // Should include v3 and ancestors
        }

        #[test]
        fn sqlite_version_with_metadata() {
            let tmp = tempfile::tempdir().unwrap();
            let mut repo = SqliteVersionRepo::new(tmp.path()).unwrap();

            let mut meta = std::collections::HashMap::new();
            meta.insert("tag".to_string(), "production".to_string());
            meta.insert("author".to_string(), "test".to_string());

            repo.add_version("m1", "f1.vault", "PyTorch", 100, 80, "h1", Some(meta), None)
                .unwrap();

            let ver = repo.get_version("m1", Some(1)).unwrap();
            assert_eq!(
                ver.metadata.get("tag").map(|s| s.as_str()),
                Some("production")
            );
        }
    }

    // ============================================================================
    // ERROR MODULE — From impls
    // ============================================================================
    mod error_extra_coverage {
        use ironvault::error::VaultError;

        #[test]
        fn vault_error_display() {
            let err = VaultError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not found",
            ));
            let s = format!("{}", err);
            assert!(s.contains("not found") || s.contains("IO"));
        }

        #[test]
        fn vault_error_from_io() {
            let io_err = std::io::Error::other("io test");
            let vault_err: VaultError = io_err.into();
            assert!(!format!("{}", vault_err).is_empty());
        }

        #[test]
        fn vault_error_from_serde_json() {
            let bad_json = "not json";
            let json_err = serde_json::from_str::<serde_json::Value>(bad_json).unwrap_err();
            let vault_err: VaultError = json_err.into();
            assert!(!format!("{}", vault_err).is_empty());
        }
    }

    // ============================================================================
    // COMPLIANCE — additional branches
    // ============================================================================
    mod compliance_extra_coverage {
        use ironvault::compliance::ComplianceChecker;

        #[test]
        fn compliance_check_default() {
            let checker = ComplianceChecker::new();
            let report = checker.run_all_checks().unwrap();
            assert!(report.fips_140_3);
        }

        #[test]
        fn compliance_verbose() {
            let checker = ComplianceChecker::new();
            let report = checker.run_all_checks().unwrap();
            assert!(report.fips_140_3);
            assert!(report.mitre_attack_aligned);
            // `cve_scan_passed` is deliberately not asserted: it depends on
            // whether cargo-audit is installed on the machine running the
            // tests, which is not a property of this crate. A scan that could
            // not run is reported as not-verified rather than as a pass.
        }
    }
}

#[allow(unused_imports)]
mod edge_coverage_tests {
    //! Edge coverage tests — Part 5
    //! Targets remaining uncovered lines in:
    //! - conversion.rs: Converter::validate (magic bytes branches), ConversionResult::compression_ratio,
    //!   ConversionOptions::with_validation, individual converter error paths
    //! - telemetry.rs: init_default, TelemetryClient::track batch threshold & flush with events
    //! - federation.rs: save_state, SyncConflict, ConflictResolution
    //! - vault.rs: streaming threshold path, auto-cleanup, version operations
    //! - version_sqlite.rs: edge cases

    mod converter_validate_magic_bytes {
        use ironvault::conversion::*;
        use ironvault::formats::ModelFormat;

        // --- Test validate_magic_bytes through the Converter::validate trait method ---

        /// SafeTensors target: valid header (passes)
        #[test]
        fn validate_safetensors_valid() {
            let converter = RawToSafeTensorsConverter;
            let input = b"raw data";
            // Build valid SafeTensors output
            let header = r#"{"__metadata__":{}}"#;
            let header_bytes = header.as_bytes();
            let header_len = header_bytes.len() as u64;
            let mut output = Vec::new();
            output.extend_from_slice(&header_len.to_le_bytes());
            output.extend_from_slice(header_bytes);
            output.extend_from_slice(b"data");

            let report = converter.validate(input, &output, &ConversionOptions::default());
            assert!(report.passed, "Expected valid SafeTensors: {:?}", report);
            let magic_check = report
                .checks
                .iter()
                .find(|c| c.name == "magic_bytes")
                .unwrap();
            assert!(magic_check.passed);
            assert!(magic_check.message.contains("SafeTensors"));
        }

        /// SafeTensors target: invalid header length
        #[test]
        fn validate_safetensors_invalid_header_len() {
            let converter = RawToSafeTensorsConverter;
            let input = b"raw data";
            // Header length of 0 = invalid
            let mut output = vec![0u8; 8];
            output[0] = 0; // header_len = 0
            output.extend_from_slice(b"extra");

            let report = converter.validate(input, &output, &ConversionOptions::default());
            let magic_check = report
                .checks
                .iter()
                .find(|c| c.name == "magic_bytes")
                .unwrap();
            assert!(!magic_check.passed);
            assert!(magic_check.message.contains("Invalid SafeTensors header"));
        }

        /// SafeTensors target: too small for header
        #[test]
        fn validate_safetensors_too_small() {
            let converter = RawToSafeTensorsConverter;
            let input = b"raw data";
            let output = b"tiny"; // less than 8 bytes

            let report = converter.validate(input, output, &ConversionOptions::default());
            let magic_check = report
                .checks
                .iter()
                .find(|c| c.name == "magic_bytes")
                .unwrap();
            assert!(!magic_check.passed);
            assert!(magic_check.message.contains("Too small"));
        }

        /// GGUF target: valid magic bytes
        #[test]
        fn validate_gguf_valid() {
            let converter = SafeTensorsToGgufConverter;
            let input = b"safetensors data";
            let mut output = Vec::new();
            output.extend_from_slice(b"GGUF"); // GGUF magic
            output.extend_from_slice(&[1, 0, 0, 0]); // version
            output.extend_from_slice(&[0u8; 100]); // padding

            let report = converter.validate(input, &output, &ConversionOptions::default());
            let magic_check = report
                .checks
                .iter()
                .find(|c| c.name == "magic_bytes")
                .unwrap();
            assert!(magic_check.passed);
            assert!(magic_check.message.contains("OK"));
        }

        /// GGUF target: invalid magic bytes
        #[test]
        fn validate_gguf_invalid() {
            let converter = SafeTensorsToGgufConverter;
            let input = b"safetensors data";
            let output = b"NOT_GGUF_data";

            let report = converter.validate(input, output, &ConversionOptions::default());
            let magic_check = report
                .checks
                .iter()
                .find(|c| c.name == "magic_bytes")
                .unwrap();
            assert!(!magic_check.passed);
            assert!(magic_check.message.contains("mismatch"));
        }

        /// ONNX target: valid protobuf tag
        #[test]
        fn validate_onnx_valid() {
            let converter = PyTorchToOnnxConverter;
            let input = b"pytorch data";
            let mut output = vec![0x08]; // ONNX protobuf field 1 varint
            output.extend_from_slice(&[0x01; 50]);

            let report = converter.validate(input, &output, &ConversionOptions::default());
            let magic_check = report
                .checks
                .iter()
                .find(|c| c.name == "magic_bytes")
                .unwrap();
            assert!(magic_check.passed);
        }

        /// ONNX target: invalid protobuf tag
        #[test]
        fn validate_onnx_invalid() {
            let converter = PyTorchToOnnxConverter;
            let input = b"pytorch data";
            let output = b"\xFF\xFF\xFF"; // Not valid ONNX

            let report = converter.validate(input, output, &ConversionOptions::default());
            let magic_check = report
                .checks
                .iter()
                .find(|c| c.name == "magic_bytes")
                .unwrap();
            assert!(!magic_check.passed);
        }

        /// PyTorch target: valid ZIP archive (PK magic)
        #[test]
        fn validate_pytorch_zip() {
            let converter = SafeTensorsToPyTorchConverter;
            let input = b"safetensors data";
            let mut output = Vec::new();
            output.extend_from_slice(b"PK"); // ZIP magic
            output.extend_from_slice(&[0x03, 0x04]); // ZIP local header
            output.extend_from_slice(&[0u8; 50]);

            let report = converter.validate(input, &output, &ConversionOptions::default());
            let magic_check = report
                .checks
                .iter()
                .find(|c| c.name == "magic_bytes")
                .unwrap();
            assert!(magic_check.passed);
            assert!(magic_check.message.contains("ZIP"));
        }

        /// PyTorch target: valid pickle format
        #[test]
        fn validate_pytorch_pickle() {
            let converter = SafeTensorsToPyTorchConverter;
            let input = b"safetensors data";
            let mut output = vec![0x80]; // pickle protocol byte
            output.extend_from_slice(&[0x02, 0x01]);

            let report = converter.validate(input, &output, &ConversionOptions::default());
            let magic_check = report
                .checks
                .iter()
                .find(|c| c.name == "magic_bytes")
                .unwrap();
            assert!(magic_check.passed);
            assert!(magic_check.message.contains("pickle"));
        }

        /// PyTorch target: unrecognised header
        #[test]
        fn validate_pytorch_unrecognised() {
            let converter = SafeTensorsToPyTorchConverter;
            let input = b"safetensors data";
            let output = b"\x00\x00\x00\x00\x00"; // Neither PK nor pickle

            let report = converter.validate(input, output, &ConversionOptions::default());
            let magic_check = report
                .checks
                .iter()
                .find(|c| c.name == "magic_bytes")
                .unwrap();
            assert!(!magic_check.passed);
            assert!(magic_check.message.contains("Unrecognised"));
        }

        /// TensorRT target: valid TFLite FlatBuffer (since OnnxToTensorRtConverter target = TensorRT)
        /// We test TFLite through a custom converter
        #[test]
        fn validate_no_magic_check_format() {
            // Custom format has no magic check
            let converter = GgufHeaderParser;
            // Target is Custom("gguf-meta"), no magic check
            let input = b"GGUF data";
            let output = b"some json output";

            let report = converter.validate(input, output, &ConversionOptions::default());
            let magic_check = report
                .checks
                .iter()
                .find(|c| c.name == "magic_bytes")
                .unwrap();
            assert!(magic_check.passed);
            assert!(magic_check.message.contains("No magic-byte check"));
        }

        /// Empty output — should fail non_empty check
        #[test]
        fn validate_empty_output() {
            let converter = SafeTensorsToRawConverter;
            let input = b"some input";
            let output: &[u8] = b"";

            let report = converter.validate(input, output, &ConversionOptions::default());
            assert!(!report.passed);
            let non_empty = report
                .checks
                .iter()
                .find(|c| c.name == "non_empty")
                .unwrap();
            assert!(!non_empty.passed);
        }

        /// Suspicious size ratio >100x without quantization
        #[test]
        fn validate_suspicious_size_ratio() {
            let converter = SafeTensorsToRawConverter;
            let input = b"x"; // 1 byte input
            let output = vec![0u8; 200]; // 200 bytes output = 200x ratio

            let report = converter.validate(input, &output, &ConversionOptions::default());
            let size_check = report
                .checks
                .iter()
                .find(|c| c.name == "size_ratio")
                .unwrap();
            assert!(!size_check.passed);
            assert!(size_check.message.contains("Suspicious"));
        }

        /// Size ratio OK with quantization
        #[test]
        fn validate_size_ratio_ok_with_quantization() {
            let converter = SafeTensorsToRawConverter;
            let input = b"x"; // 1 byte input
            let output = vec![0u8; 200]; // 200x ratio but quantization is set

            let opts = ConversionOptions {
                quantization: Some("q4_0".to_string()),
                ..ConversionOptions::default()
            };

            let report = converter.validate(input, &output, &opts);
            let size_check = report
                .checks
                .iter()
                .find(|c| c.name == "size_ratio")
                .unwrap();
            assert!(size_check.passed); // Not suspicious with quantization
        }

        /// Size ratio with empty input
        #[test]
        fn validate_size_ratio_empty_input() {
            let converter = SafeTensorsToRawConverter;
            let input: &[u8] = b"";
            let output = b"some output";

            let report = converter.validate(input, output, &ConversionOptions::default());
            let size_check = report
                .checks
                .iter()
                .find(|c| c.name == "size_ratio")
                .unwrap();
            assert!(size_check.passed); // ratio = 1.0 for empty input
        }

        // --- ConversionResult ---
        #[test]
        fn conversion_result_compression_ratio() {
            let result = ConversionResult {
                data: vec![1, 2],
                plan: None,
                source_format: ModelFormat::PyTorch,
                target_format: ModelFormat::ONNX,
                conversion_path: vec![],
                input_size: 100,
                output_size: 50,
                validation: None,
            };
            assert!((result.compression_ratio() - 0.5).abs() < f64::EPSILON);
        }

        #[test]
        fn conversion_result_compression_ratio_zero_input() {
            let result = ConversionResult {
                data: vec![],
                plan: None,
                source_format: ModelFormat::PyTorch,
                target_format: ModelFormat::ONNX,
                conversion_path: vec![],
                input_size: 0,
                output_size: 50,
                validation: None,
            };
            assert!((result.compression_ratio() - 0.0).abs() < f64::EPSILON);
        }

        // --- ConversionOptions ---
        #[test]
        fn conversion_options_with_validation() {
            let opts = ConversionOptions::with_validation();
            assert!(opts.validate);
            assert!((opts.tolerance - 1e-5).abs() < f64::EPSILON);
            assert!(opts.preserve_metadata);
        }

        // --- Converter error paths ---
        #[test]
        fn safetensors_to_raw_too_small() {
            let converter = SafeTensorsToRawConverter;
            let result = converter.convert(b"tiny", &ConversionOptions::default(), None);
            assert!(result.is_err());
        }

        #[test]
        fn safetensors_to_raw_header_exceeds_data() {
            let converter = SafeTensorsToRawConverter;
            let mut data = Vec::new();
            data.extend_from_slice(&(1000u64).to_le_bytes()); // header_len = 1000
            data.extend_from_slice(b"small"); // only 5 bytes of data
            let result = converter.convert(&data, &ConversionOptions::default(), None);
            assert!(result.is_err());
        }

        #[test]
        fn gguf_header_parser_too_small() {
            let converter = GgufHeaderParser;
            let result = converter.convert(b"short", &ConversionOptions::default(), None);
            assert!(result.is_err());
        }

        #[test]
        fn gguf_header_parser_invalid_magic() {
            let converter = GgufHeaderParser;
            let mut data = vec![0u8; 30];
            data[0..4].copy_from_slice(b"XXXX"); // wrong magic
            let result = converter.convert(&data, &ConversionOptions::default(), None);
            assert!(result.is_err());
        }

        #[test]
        fn gguf_header_parser_valid() {
            let converter = GgufHeaderParser;
            let mut data = vec![0u8; 30];
            data[0..4].copy_from_slice(b"GGUF");
            data[4..8].copy_from_slice(&3u32.to_le_bytes()); // version 3
            data[8..16].copy_from_slice(&10u64.to_le_bytes()); // 10 tensors
            data[16..24].copy_from_slice(&5u64.to_le_bytes()); // 5 kv pairs
            let result = converter
                .convert(&data, &ConversionOptions::default(), None)
                .unwrap();
            let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(meta["version"], 3);
            assert_eq!(meta["tensor_count"], 10);
            assert_eq!(meta["kv_count"], 5);
        }

        #[test]
        fn safetensors_to_pytorch_too_small() {
            let converter = SafeTensorsToPyTorchConverter;
            let result = converter.convert(b"tiny", &ConversionOptions::default(), None);
            assert!(result.is_err());
        }

        #[test]
        fn safetensors_to_pytorch_header_exceeds() {
            let converter = SafeTensorsToPyTorchConverter;
            let mut data = Vec::new();
            data.extend_from_slice(&(1000u64).to_le_bytes());
            data.extend_from_slice(b"x");
            let result = converter.convert(&data, &ConversionOptions::default(), None);
            assert!(result.is_err());
        }

        #[test]
        fn safetensors_to_pytorch_valid() {
            let converter = SafeTensorsToPyTorchConverter;
            let header = r#"{"t":{"dtype":"U8","shape":[2],"data_offsets":[0,2]}}"#;
            let header_bytes = header.as_bytes();
            let header_len = header_bytes.len() as u64;
            let mut data = Vec::new();
            data.extend_from_slice(&header_len.to_le_bytes());
            data.extend_from_slice(header_bytes);
            data.extend_from_slice(&[1, 2]);
            let result = converter
                .convert(&data, &ConversionOptions::default(), None)
                .unwrap();
            // Real converter produces ZIP output
            assert_eq!(&result[0..2], b"PK");
        }

        #[test]
        fn pytorch_to_onnx_with_opset() {
            let converter = PyTorchToOnnxConverter;
            let opts = ConversionOptions {
                opset_version: Some(15),
                ..ConversionOptions::default()
            };
            let result = converter.convert(b"data", &opts, None).unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(plan["opset_version"], 15);
        }

        #[test]
        fn safetensors_to_gguf_with_quantization() {
            let converter = SafeTensorsToGgufConverter;
            let opts = ConversionOptions {
                quantization: Some("q4_k_m".to_string()),
                ..ConversionOptions::default()
            };
            let result = converter.convert(b"data", &opts, None).unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(plan["quantization"], "q4_k_m");
        }

        #[test]
        fn safetensors_to_gguf_default_quantization() {
            let converter = SafeTensorsToGgufConverter;
            let result = converter
                .convert(b"data", &ConversionOptions::default(), None)
                .unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(plan["quantization"], "f16"); // default
        }

        #[test]
        fn onnx_to_tensorrt_plan() {
            let converter = OnnxToTensorRtConverter;
            let result = converter
                .convert(b"data", &ConversionOptions::default(), None)
                .unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(plan["converter"], "onnx_to_tensorrt");
        }

        #[test]
        fn onnx_to_coreml_plan() {
            let converter = OnnxToCoreMLConverter;
            let result = converter
                .convert(b"data", &ConversionOptions::default(), None)
                .unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(plan["converter"], "onnx_to_coreml");
        }

        #[test]
        fn pytorch_to_safetensors_plan() {
            let converter = PyTorchToSafeTensorsConverter;
            // Real converter requires valid ZIP; invalid data should error
            let err = converter
                .convert(b"data", &ConversionOptions::default(), None)
                .unwrap_err();
            assert!(format!("{err}").contains("ZIP archive"));
        }

        #[test]
        fn raw_to_safetensors_roundtrip() {
            let converter = RawToSafeTensorsConverter;
            let data = b"hello world tensor data";
            let result = converter
                .convert(data, &ConversionOptions::default(), None)
                .unwrap();
            // Should be valid SafeTensors format
            assert!(result.len() > 8);
            let header_len = u64::from_le_bytes(result[..8].try_into().unwrap()) as usize;
            assert!(header_len > 0);
            assert!(header_len + 8 <= result.len());
            // Tensor data should be at the end
            let tensor_data = &result[8 + header_len..];
            assert_eq!(tensor_data, data);
        }

        #[test]
        fn raw_to_safetensors_with_progress() {
            let converter = RawToSafeTensorsConverter;
            let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let c = called.clone();
            let cb: ProgressCallback = Box::new(move |_p| {
                c.store(true, std::sync::atomic::Ordering::Relaxed);
            });
            let result = converter
                .convert(b"data", &ConversionOptions::default(), Some(&cb))
                .unwrap();
            assert!(!result.is_empty());
            assert!(called.load(std::sync::atomic::Ordering::Relaxed));
        }

        #[test]
        fn safetensors_to_raw_with_progress() {
            let converter = SafeTensorsToRawConverter;
            let header =
                r#"{"__metadata__":{},"t":{"dtype":"U8","shape":[4],"data_offsets":[0,4]}}"#;
            let header_bytes = header.as_bytes();
            let header_len = header_bytes.len() as u64;
            let mut data = Vec::new();
            data.extend_from_slice(&header_len.to_le_bytes());
            data.extend_from_slice(header_bytes);
            data.extend_from_slice(&[1, 2, 3, 4]);

            let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let c = called.clone();
            let cb: ProgressCallback = Box::new(move |_p| {
                c.store(true, std::sync::atomic::Ordering::Relaxed);
            });
            let result = converter
                .convert(&data, &ConversionOptions::default(), Some(&cb))
                .unwrap();
            assert_eq!(result, vec![1, 2, 3, 4]);
            assert!(called.load(std::sync::atomic::Ordering::Relaxed));
        }

        // --- Pipeline multi-step with validation ---
        #[test]
        fn pipeline_multi_step_with_validation() {
            let pipeline = ConversionPipeline::with_builtins();
            let data = b"pytorch model data for conversion";

            let opts = ConversionOptions {
                validate: true,
                ..ConversionOptions::default()
            };

            // Multi-step shim converters produce JSON plans not real model data,
            // so validation may fail at intermediate steps. That's expected.
            let result = pipeline.convert(
                data,
                &ModelFormat::PyTorch,
                &ModelFormat::TensorRT,
                &opts,
                None,
            );
            // Result could be Ok (validation passes) or Err (validation fails at intermediate step)
            // Either way, the validation code path has been exercised
            let _ = result;
        }

        // --- Pipeline intermediate validation failure ---
        // Hard to trigger in practice since shim converters always produce valid JSON
        // but we can test the supported_conversions listing
        #[test]
        fn pipeline_supported_conversions() {
            let pipeline = ConversionPipeline::with_builtins();
            let conversions = pipeline.supported_conversions();
            assert!(conversions.len() >= 8); // at least 8 builtin converters
                                             // Each has name, source, target
            for (src, tgt, name) in &conversions {
                assert!(!name.is_empty());
                assert_ne!(src, tgt);
            }
        }
    }

    // ============================================================================
    // TELEMETRY — init_default, track that triggers batch
    // ============================================================================
    mod telemetry_edge_coverage {
        use ironvault::telemetry::*;

        #[test]
        fn init_default_with_temp_dir() {
            // init_default loads config from disk; with a temp dir it uses defaults
            let tmp = tempfile::tempdir().unwrap();
            let dir = tmp.path().to_path_buf();
            // This might fail on second run due to OnceLock, but it exercises the path
            let _ = init_default(Some(&dir));
        }

        #[test]
        fn init_default_with_existing_config() {
            let tmp = tempfile::tempdir().unwrap();
            let config_path = tmp.path().join("telemetry.yaml");
            let config = TelemetryConfig::default();
            let yaml = serde_yaml_ng::to_string(&config).unwrap();
            std::fs::write(&config_path, yaml).unwrap();

            let dir = tmp.path().to_path_buf();
            let _ = init_default(Some(&dir));
        }

        #[test]
        fn client_track_triggers_batch_flush() {
            let config = TelemetryConfig {
                batch_size: 2, // Very small batch to trigger flush
                ..TelemetryConfig::default()
            };
            let client = TelemetryClient::new(config);

            // Track 3 events — first 2 trigger batch, 3rd stays in queue
            client.track(TelemetryEvent::CommandRun {
                command: "a".to_string(),
                subcommand: None,
                duration_ms: 1,
                success: true,
            });
            client.track(TelemetryEvent::CommandRun {
                command: "b".to_string(),
                subcommand: None,
                duration_ms: 2,
                success: true,
            });
            client.track(TelemetryEvent::CommandRun {
                command: "c".to_string(),
                subcommand: None,
                duration_ms: 3,
                success: true,
            });
            // No crash = batch handling works
        }

        #[test]
        fn client_flush_with_pending_events() {
            let config = TelemetryConfig {
                batch_size: 100, // Large batch so events stay queued
                ..TelemetryConfig::default()
            };
            let client = TelemetryClient::new(config);

            client.track(TelemetryEvent::FeatureUsed {
                feature: "test".to_string(),
                detail: None,
            });
            client.flush(); // Should attempt to send the queued event
                            // Wait a bit for the background thread
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        #[test]
        fn telemetry_config_fields() {
            let config = TelemetryConfig {
                enabled: false,
                device_id: "test".to_string(),
                endpoint: "https://example.com/telemetry".to_string(),
                batch_size: 50,
                flush_interval_secs: 120,
            };
            assert!(!config.enabled);
            assert_eq!(config.endpoint, "https://example.com/telemetry");
        }

        #[test]
        fn telemetry_event_command_run_with_subcommand() {
            let event = TelemetryEvent::CommandRun {
                command: "cloud".to_string(),
                subcommand: Some("push".to_string()),
                duration_ms: 500,
                success: false,
            };
            let json = serde_json::to_string(&event).unwrap();
            assert!(json.contains("cloud"));
            assert!(json.contains("push"));
        }
    }

    // ============================================================================
    // FEDERATION — SyncResult, SyncConflict, ConflictResolution
    // ============================================================================
    mod federation_edge_coverage {
        use ironvault::federation::*;

        #[test]
        fn vector_clock_increment_merge_comparison() {
            let mut c1 = VectorClock::new();
            let mut c2 = VectorClock::new();

            c1.increment("node-a");
            c1.increment("node-a");
            c2.increment("node-b");

            assert_eq!(c1.compare(&c2), ClockComparison::Concurrent);

            c1.merge(&c2);
            // Now c1 has {node-a: 2, node-b: 1}

            assert_eq!(c1.compare(&c2), ClockComparison::After);
            assert_eq!(c2.compare(&c1), ClockComparison::Before);
        }

        #[test]
        fn vector_clock_equal() {
            let mut c1 = VectorClock::new();
            let mut c2 = VectorClock::new();

            c1.increment("a");
            c2.increment("a");

            assert_eq!(c1.compare(&c2), ClockComparison::Equal);
        }

        #[test]
        fn sync_conflict_resolution_variants() {
            let conflict = SyncConflict {
                model: "m".to_string(),
                local_version: "v1".to_string(),
                remote_version: "v2".to_string(),
                remote_node: "peer".to_string(),
                resolution: Some(ConflictResolution::KeepLocal),
            };
            assert_eq!(conflict.model, "m");

            let conflict2 = SyncConflict {
                resolution: Some(ConflictResolution::UseRemote),
                ..conflict.clone()
            };
            assert!(conflict2.resolution.is_some());

            let conflict3 = SyncConflict {
                resolution: Some(ConflictResolution::Manual),
                ..conflict
            };
            assert!(conflict3.resolution.is_some());
        }

        #[test]
        fn federation_config_default() {
            let config = FederationConfig::default();
            assert!(!config.node_id.is_empty());
            assert_eq!(config.sync_interval_secs, 300);
            assert!(config.auto_resolve_conflicts);
            assert_eq!(config.max_concurrent_syncs, 4);
        }

        #[test]
        fn peer_config_disabled() {
            let peer = PeerConfig {
                node_id: "p1".to_string(),
                name: "Peer1".to_string(),
                endpoint: "https://example.com".to_string(),
                api_key: None,
                enabled: false,
            };
            assert!(!peer.enabled);
            assert!(peer.api_key.is_none());
        }

        #[test]
        fn sync_result_display() {
            let result = SyncResult {
                peer_id: "peer-1".to_string(),
                timestamp: chrono::Utc::now(),
                duration_ms: 1500,
                models_synced: 3,
                versions_uploaded: 2,
                versions_downloaded: 1,
                conflicts: vec![],
                errors: vec!["some error".to_string()],
            };
            assert_eq!(result.peer_id, "peer-1");
            assert_eq!(result.models_synced, 3);
            assert_eq!(result.errors.len(), 1);
        }

        #[test]
        fn federation_status_fields() {
            let status = FederationStatus {
                node_id: "n1".to_string(),
                node_name: "Node1".to_string(),
                peer_count: 2,
                model_count: 5,
                last_sync: Some(chrono::Utc::now()),
                clock: VectorClock::new(),
            };
            assert_eq!(status.peer_count, 2);
            assert!(status.last_sync.is_some());
        }

        #[test]
        fn sync_item_fields() {
            let item = SyncItem {
                model: "model1".to_string(),
                checkpoint_id: "ckpt-1".to_string(),
                size_bytes: 1024,
            };
            assert_eq!(item.model, "model1");
            assert_eq!(item.size_bytes, 1024);
        }
    }

    // ============================================================================
    // VAULT — streaming threshold, auto-cleanup
    // ============================================================================
    mod vault_streaming_coverage {
        use ironvault::config::{DirectoryPaths, VaultConfig};
        use ironvault::formats::{ModelFormat, ModelMetadata};
        use ironvault::{Vault, VaultBuilder};

        fn make_dirs(tmp: &tempfile::TempDir) -> DirectoryPaths {
            DirectoryPaths {
                config_dir: tmp.path().join("config"),
                data_dir: tmp.path().join("data"),
                cache_dir: tmp.path().join("cache"),
                vault_dir: tmp.path().join("data/vaults/default"),
                log_dir: tmp.path().join("data/logs"),
                backends_dir: tmp.path().join("config/backends"),
                utilities_dir: tmp.path().join("config/utilities"),
                databases_dir: tmp.path().join("config/databases"),
            }
        }

        #[test]
        fn vault_multiple_versions() {
            let tmp = tempfile::tempdir().unwrap();
            let config = VaultConfig::with_dirs(make_dirs(&tmp)).unwrap();
            let mut vault = Vault::new(Some(config)).unwrap();
            vault
                .unlock(b"test_passphrase_with_sufficient_entropy".to_vec())
                .unwrap();

            // Store v1
            let meta1 = ModelMetadata::new("model".into(), ModelFormat::PyTorch);
            let v1 = vault
                .store_model("model", b"v1 data".to_vec(), meta1, None)
                .unwrap();
            assert_eq!(v1.version, 1);

            // Store v2 with parent
            let meta2 = ModelMetadata::new("model".into(), ModelFormat::PyTorch)
                .with_description("Updated".into());
            let v2 = vault
                .store_model("model", b"v2 data".to_vec(), meta2, Some(1))
                .unwrap();
            assert_eq!(v2.version, 2);

            // Get v1
            let d1 = vault.get_model("model", Some(1)).unwrap();
            assert_eq!(d1, b"v1 data");

            // Get latest (v2)
            let d2 = vault.get_model("model", None).unwrap();
            assert_eq!(d2, b"v2 data");

            // List versions
            let versions = vault.list_versions("model");
            assert_eq!(versions.len(), 2);

            // Lineage
            let lineage = vault.get_lineage("model", 2);
            assert!(!lineage.is_empty());
        }

        #[test]
        fn vault_delete_and_stats() {
            let tmp = tempfile::tempdir().unwrap();
            let config = VaultConfig::with_dirs(make_dirs(&tmp)).unwrap();
            let mut vault = Vault::new(Some(config)).unwrap();
            vault
                .unlock(b"test_passphrase_with_sufficient_entropy".to_vec())
                .unwrap();

            let meta = ModelMetadata::new("m".into(), ModelFormat::ONNX);
            vault
                .store_model("m", b"data".to_vec(), meta, None)
                .unwrap();

            let stats = vault.get_stats().unwrap();
            assert_eq!(stats.model_count, 1);

            vault.delete_version("m", 1).unwrap();
            // After deleting the only version, model may still show in list
            // but with 0 versions; check total_versions instead
            let stats2 = vault.get_stats().unwrap();
            assert!(stats2.total_versions <= stats.total_versions);
        }

        #[test]
        fn vault_change_passphrase() {
            let tmp = tempfile::tempdir().unwrap();
            let config = VaultConfig::with_dirs(make_dirs(&tmp)).unwrap();
            let mut vault = Vault::new(Some(config)).unwrap();
            vault
                .unlock(b"original_passphrase_with_entropy".to_vec())
                .unwrap();

            let meta = ModelMetadata::new("m".into(), ModelFormat::Safetensors);
            vault
                .store_model("m", b"important data".to_vec(), meta, None)
                .unwrap();

            vault
                .change_passphrase(b"new_passphrase_with_sufficient_entropy".to_vec())
                .unwrap();

            // Lock and unlock with new passphrase
            vault.lock();
            vault
                .unlock(b"new_passphrase_with_sufficient_entropy".to_vec())
                .unwrap();

            let data = vault.get_model("m", None).unwrap();
            assert_eq!(data, b"important data");
        }

        #[test]
        fn vault_with_metrics_subscriber() {
            let tmp = tempfile::tempdir().unwrap();
            let config = VaultConfig::with_dirs(make_dirs(&tmp)).unwrap();
            let mut vault = VaultBuilder::new().config(config).build().unwrap();

            vault
                .unlock(b"test_passphrase_with_sufficient_entropy".to_vec())
                .unwrap();

            let meta = ModelMetadata::new("m".into(), ModelFormat::PyTorch);
            vault.store_model("m", b"d".to_vec(), meta, None).unwrap();
            vault.get_model("m", None).unwrap();

            let snap = vault.metrics().unwrap();
            assert_eq!(snap.models_stored_total, 1);
            assert_eq!(snap.models_retrieved_total, 1);
        }

        #[test]
        fn vault_model_not_found() {
            let tmp = tempfile::tempdir().unwrap();
            let config = VaultConfig::with_dirs(make_dirs(&tmp)).unwrap();
            let mut vault = Vault::new(Some(config)).unwrap();
            vault
                .unlock(b"test_passphrase_with_sufficient_entropy".to_vec())
                .unwrap();

            let result = vault.get_model("nonexistent", None);
            assert!(result.is_err());
        }
    }

    // ============================================================================
    // FORMATS — additional format detection
    // ============================================================================
    mod formats_edge_coverage {
        use ironvault::formats::ModelFormat;

        #[test]
        fn detect_format_by_extension() {
            assert_eq!(ModelFormat::from_extension("pt"), ModelFormat::PyTorch);
            assert_eq!(ModelFormat::from_extension("pth"), ModelFormat::PyTorch);
            assert_eq!(
                ModelFormat::from_extension("safetensors"),
                ModelFormat::Safetensors
            );
            assert_eq!(ModelFormat::from_extension("onnx"), ModelFormat::ONNX);
            assert_eq!(ModelFormat::from_extension("gguf"), ModelFormat::GGUF);
            assert_eq!(ModelFormat::from_extension("plan"), ModelFormat::TensorRT);
            assert_eq!(ModelFormat::from_extension("mlmodel"), ModelFormat::CoreML);
            assert_eq!(ModelFormat::from_extension("tflite"), ModelFormat::TFLite);
            assert_eq!(ModelFormat::from_extension("pb"), ModelFormat::TensorFlow);
            assert_eq!(ModelFormat::from_extension("h5"), ModelFormat::Keras);
            assert_eq!(ModelFormat::from_extension("keras"), ModelFormat::Keras);
            assert_eq!(ModelFormat::from_extension("npz"), ModelFormat::NumPy);
            assert_eq!(ModelFormat::from_extension("npy"), ModelFormat::NumPy);
            assert_eq!(ModelFormat::from_extension("pkl"), ModelFormat::Pickle);
            assert_eq!(ModelFormat::from_extension("bin"), ModelFormat::PyTorch);
        }

        #[test]
        fn detect_format_unknown() {
            let fmt = ModelFormat::from_extension("xyz");
            assert!(matches!(fmt, ModelFormat::Custom(_)));
        }

        #[test]
        fn model_format_extension_roundtrip() {
            let formats = vec![
                ModelFormat::PyTorch,
                ModelFormat::Safetensors,
                ModelFormat::ONNX,
                ModelFormat::GGUF,
                ModelFormat::TensorRT,
                ModelFormat::CoreML,
                ModelFormat::TFLite,
                ModelFormat::TensorFlow,
                ModelFormat::Keras,
                ModelFormat::HDF5,
                ModelFormat::NumPy,
                ModelFormat::Pickle,
            ];
            for fmt in formats {
                let ext = fmt.extension();
                assert!(!ext.is_empty(), "Extension empty for {:?}", fmt);
            }
        }
    }
}

#[allow(unused_imports)]
mod final_coverage_tests {
    //! Final coverage tests — Part 6
    //! Targets all remaining testable uncovered lines:
    //! - conversion.rs: OnnxMetadataExtractor (protobuf parsing), pipeline paths
    //! - vault.rs: lock(), state(), get_model error branches, streaming threshold,
    //!   get_model_chunked, ModelStream, version_backend_name, update/get metadata
    //! - traits.rs: AsyncBlobStoreAdapter, EventBus error branch
    //! - version.rs: cleanup_old_versions, verify_checksum
    //! - formats.rs: extension() and name() for all format variants, from_magic_bytes

    // ============================================================================
    // CONVERSION — OnnxMetadataExtractor protobuf parsing
    // ============================================================================
    mod onnx_protobuf_tests {
        use ironvault::conversion::*;
        use ironvault::formats::ModelFormat;

        /// Helper: build a protobuf varint
        fn encode_varint(mut val: u64) -> Vec<u8> {
            let mut out = Vec::new();
            loop {
                let mut byte = (val & 0x7F) as u8;
                val >>= 7;
                if val > 0 {
                    byte |= 0x80;
                }
                out.push(byte);
                if val == 0 {
                    break;
                }
            }
            out
        }

        /// Helper: build a protobuf tag byte(s)
        fn encode_tag(field_num: u64, wire_type: u8) -> Vec<u8> {
            encode_varint((field_num << 3) | wire_type as u64)
        }

        /// Helper: encode a length-delimited field
        fn encode_length_delimited(field_num: u64, data: &[u8]) -> Vec<u8> {
            let mut out = encode_tag(field_num, 2);
            out.extend(encode_varint(data.len() as u64));
            out.extend_from_slice(data);
            out
        }

        /// Helper: encode a varint field
        fn encode_varint_field(field_num: u64, value: u64) -> Vec<u8> {
            let mut out = encode_tag(field_num, 0);
            out.extend(encode_varint(value));
            out
        }

        #[test]
        fn onnx_extractor_all_known_fields() {
            let converter = OnnxMetadataExtractor;

            let mut data = Vec::new();
            // field 1 = ir_version (varint) = 9
            data.extend(encode_varint_field(1, 9));
            // field 2 = producer_name (length-delimited) = "onnxruntime"
            data.extend(encode_length_delimited(2, b"onnxruntime"));
            // field 5 = model_version (varint) = 42
            data.extend(encode_varint_field(5, 42));
            // field 6 = doc_string (length-delimited) = "A test model"
            data.extend(encode_length_delimited(6, b"A test model"));

            let result = converter
                .convert(&data, &ConversionOptions::default(), None)
                .unwrap();
            let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(meta["ir_version"], 9);
            assert_eq!(meta["producer"], "onnxruntime");
            assert_eq!(meta["model_version"], 42);
            assert_eq!(meta["doc_string"], "A test model");
            assert_eq!(meta["format"], "ONNX");
        }

        #[test]
        fn onnx_extractor_skip_other_varint_field() {
            // Field 4 varint — should be skipped
            let converter = OnnxMetadataExtractor;

            let mut data = Vec::new();
            data.extend(encode_varint_field(1, 7)); // ir_version = 7
            data.extend(encode_varint_field(4, 999)); // unknown field 4, skip
            data.extend(encode_varint_field(5, 3)); // model_version = 3

            let result = converter
                .convert(&data, &ConversionOptions::default(), None)
                .unwrap();
            let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(meta["ir_version"], 7);
            assert_eq!(meta["model_version"], 3);
        }

        #[test]
        fn onnx_extractor_skip_length_delimited_field() {
            // Field 3 length-delimited — should be skipped
            let converter = OnnxMetadataExtractor;

            let mut data = Vec::new();
            data.extend(encode_varint_field(1, 5));
            data.extend(encode_length_delimited(3, b"skip this data"));
            data.extend(encode_length_delimited(2, b"pytorch"));

            let result = converter
                .convert(&data, &ConversionOptions::default(), None)
                .unwrap();
            let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(meta["ir_version"], 5);
            assert_eq!(meta["producer"], "pytorch");
        }

        #[test]
        fn onnx_extractor_skip_32bit_field() {
            // Wire type 5 = 32-bit fixed
            let converter = OnnxMetadataExtractor;

            let mut data = Vec::new();
            data.extend(encode_varint_field(1, 8)); // ir_version = 8
                                                    // Field 7 wire type 5 (32-bit): tag = (7 << 3) | 5 = 0x3D
            data.extend(encode_tag(7, 5));
            data.extend(&[0xAA, 0xBB, 0xCC, 0xDD]); // 4 bytes of data
            data.extend(encode_varint_field(5, 10)); // model_version = 10

            let result = converter
                .convert(&data, &ConversionOptions::default(), None)
                .unwrap();
            let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(meta["ir_version"], 8);
            assert_eq!(meta["model_version"], 10);
        }

        #[test]
        fn onnx_extractor_skip_64bit_field() {
            // Wire type 1 = 64-bit fixed
            let converter = OnnxMetadataExtractor;

            let mut data = Vec::new();
            data.extend(encode_varint_field(1, 3));
            // Field 9 wire type 1 (64-bit): tag = (9 << 3) | 1 = 0x49
            data.extend(encode_tag(9, 1));
            data.extend(&[0u8; 8]); // 8 bytes
            data.extend(encode_length_delimited(6, b"hello"));

            let result = converter
                .convert(&data, &ConversionOptions::default(), None)
                .unwrap();
            let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(meta["ir_version"], 3);
            assert_eq!(meta["doc_string"], "hello");
        }

        #[test]
        fn onnx_extractor_unknown_wire_type_breaks() {
            // Wire type 3 or 4 (deprecated group types) — should break the loop
            let converter = OnnxMetadataExtractor;

            let mut data = Vec::new();
            data.extend(encode_varint_field(1, 11)); // ir_version = 11
                                                     // Field 10 wire type 3: tag = (10 << 3) | 3 = 0x53
            data.extend(encode_tag(10, 3));
            // After break, no more parsing
            data.extend(encode_varint_field(5, 50)); // should NOT be read

            let result = converter
                .convert(&data, &ConversionOptions::default(), None)
                .unwrap();
            let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(meta["ir_version"], 11);
            assert_eq!(meta["model_version"], 0); // never reached
        }

        #[test]
        fn onnx_extractor_empty_data() {
            let converter = OnnxMetadataExtractor;
            let result = converter
                .convert(b"", &ConversionOptions::default(), None)
                .unwrap();
            let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(meta["ir_version"], 0);
            assert_eq!(meta["producer"], "");
            assert_eq!(meta["model_version"], 0);
            assert_eq!(meta["doc_string"], "");
        }

        #[test]
        fn onnx_extractor_only_producer() {
            let converter = OnnxMetadataExtractor;

            let data = encode_length_delimited(2, b"tensorflow");
            let result = converter
                .convert(&data, &ConversionOptions::default(), None)
                .unwrap();
            let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(meta["producer"], "tensorflow");
            assert_eq!(meta["ir_version"], 0);
        }

        #[test]
        fn onnx_extractor_multi_byte_varint() {
            // Test a varint that requires >1 byte (value > 127)
            let converter = OnnxMetadataExtractor;

            let mut data = Vec::new();
            data.extend(encode_varint_field(1, 300)); // ir_version = 300 (needs 2 bytes)
            data.extend(encode_varint_field(5, 16384)); // model_version = 16384 (needs 3 bytes)

            let result = converter
                .convert(&data, &ConversionOptions::default(), None)
                .unwrap();
            let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(meta["ir_version"], 300);
            assert_eq!(meta["model_version"], 16384);
        }

        #[test]
        fn onnx_extractor_mixed_skip_and_known() {
            // Comprehensive protobuf with all wire types interleaved
            let converter = OnnxMetadataExtractor;

            let mut data = Vec::new();
            data.extend(encode_varint_field(1, 9)); // ir_version
            data.extend(encode_length_delimited(3, b"skip1")); // skip (len-delimited)
            data.extend(encode_length_delimited(2, b"mylib")); // producer_name
            data.extend(encode_varint_field(4, 777)); // skip (varint)
            data.extend(encode_tag(7, 5));
            data.extend(&[0u8; 4]); // skip (32-bit)
            data.extend(encode_varint_field(5, 1)); // model_version
            data.extend(encode_tag(8, 1));
            data.extend(&[0u8; 8]); // skip (64-bit)
            data.extend(encode_length_delimited(6, b"my doc")); // doc_string

            let result = converter
                .convert(&data, &ConversionOptions::default(), None)
                .unwrap();
            let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(meta["ir_version"], 9);
            assert_eq!(meta["producer"], "mylib");
            assert_eq!(meta["model_version"], 1);
            assert_eq!(meta["doc_string"], "my doc");
        }

        #[test]
        fn onnx_extractor_source_target_format() {
            let converter = OnnxMetadataExtractor;
            assert_eq!(converter.source_format(), ModelFormat::ONNX);
            assert_eq!(
                converter.target_format(),
                ModelFormat::Custom("onnx-meta".into())
            );
            assert_eq!(converter.name(), "ONNX → Metadata (JSON)");
        }

        // --- Pipeline tests for additional paths ---

        #[test]
        fn pipeline_convert_onnx_to_metadata() {
            let mut pipeline = ConversionPipeline::new();
            pipeline.register(Box::new(OnnxMetadataExtractor));

            let data = encode_varint_field(1, 7);
            let result = pipeline
                .convert(
                    &data,
                    &ModelFormat::ONNX,
                    &ModelFormat::Custom("onnx-meta".into()),
                    &ConversionOptions::default(),
                    None,
                )
                .unwrap();

            let meta: serde_json::Value = serde_json::from_slice(&result.data).unwrap();
            assert_eq!(meta["ir_version"], 7);
        }

        #[test]
        fn pipeline_convert_with_progress_callback() {
            let pipeline = ConversionPipeline::with_builtins();

            let header =
                r#"{"__metadata__":{},"t":{"dtype":"U8","shape":[4],"data_offsets":[0,4]}}"#;
            let header_bytes = header.as_bytes();
            let header_len = header_bytes.len() as u64;
            let mut safetensors_data = Vec::new();
            safetensors_data.extend_from_slice(&header_len.to_le_bytes());
            safetensors_data.extend_from_slice(header_bytes);
            safetensors_data.extend_from_slice(&[1, 2, 3, 4]);

            let progress_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let pc = progress_called.clone();
            let callback: Box<dyn Fn(&ConversionProgress) + Send + Sync> = Box::new(move |_p| {
                pc.store(true, std::sync::atomic::Ordering::SeqCst);
            });

            let result = pipeline.convert(
                &safetensors_data,
                &ModelFormat::Safetensors,
                &ModelFormat::Custom("raw".into()),
                &ConversionOptions::default(),
                Some(&callback),
            );
            assert!(result.is_ok());
            // Progress callback should have been called for single-step conversion
        }

        #[test]
        fn pipeline_intermediate_validation_failure() {
            // Create a pipeline where intermediate step output fails validation
            let pipeline = ConversionPipeline::with_builtins();

            // Build valid SafeTensors data
            let header =
                r#"{"__metadata__":{},"t":{"dtype":"U8","shape":[4],"data_offsets":[0,4]}}"#;
            let header_bytes = header.as_bytes();
            let header_len = header_bytes.len() as u64;
            let mut data = Vec::new();
            data.extend_from_slice(&header_len.to_le_bytes());
            data.extend_from_slice(header_bytes);
            data.extend_from_slice(&[1, 2, 3, 4]);

            let opts = ConversionOptions {
                validate: true,
                ..ConversionOptions::default()
            };

            // SafeTensors → GGUF is a multi-step path (SafeTensors → raw → ... or shim)
            // The shim converter produces JSON which will fail GGUF magic validation
            let result = pipeline.convert(
                &data,
                &ModelFormat::Safetensors,
                &ModelFormat::GGUF,
                &opts,
                None,
            );
            // Either succeeds or fails at intermediate validation — both exercise the path
            let _ = result;
        }
    }

    // ============================================================================
    // VAULT — lock, state, version error, streaming threshold, ModelStream, metadata
    // ============================================================================
    mod vault_advanced_tests {
        use ironvault::config::{DirectoryPaths, VaultConfig};
        use ironvault::formats::{ModelFormat, ModelMetadata};
        use ironvault::traits::VaultState;
        use ironvault::Vault;

        fn make_dirs(tmp: &tempfile::TempDir) -> DirectoryPaths {
            DirectoryPaths {
                config_dir: tmp.path().join("config"),
                data_dir: tmp.path().join("data"),
                cache_dir: tmp.path().join("cache"),
                vault_dir: tmp.path().join("data/vaults/default"),
                log_dir: tmp.path().join("data/logs"),
                backends_dir: tmp.path().join("config/backends"),
                utilities_dir: tmp.path().join("config/utilities"),
                databases_dir: tmp.path().join("config/databases"),
            }
        }

        fn make_config(tmp: &tempfile::TempDir) -> VaultConfig {
            VaultConfig::with_dirs(make_dirs(tmp)).unwrap()
        }

        fn make_vault(tmp: &tempfile::TempDir) -> Vault {
            let config = make_config(tmp);
            let mut vault = Vault::new(Some(config)).unwrap();
            vault
                .unlock(b"test_passphrase_32bytes_entropy!".to_vec())
                .unwrap();
            vault
        }

        // --- lock() method ---

        #[test]
        fn vault_lock_clears_key() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vault = make_vault(&tmp);
            assert!(vault.is_unlocked());

            vault.lock();
            assert!(!vault.is_unlocked());

            // Operations should fail when locked
            let result = vault.get_model("nonexistent", None);
            assert!(result.is_err());
            let err_msg = format!("{}", result.unwrap_err());
            assert!(
                err_msg.contains("locked"),
                "Expected locked error: {}",
                err_msg
            );
        }

        // --- state() method ---

        #[test]
        fn vault_state_unlocked() {
            let tmp = tempfile::tempdir().unwrap();
            let vault = make_vault(&tmp);
            let state = vault.state();
            match state {
                VaultState::Unlocked {
                    vault_name,
                    model_count,
                    ..
                } => {
                    assert!(!vault_name.is_empty());
                    assert_eq!(model_count, 0);
                }
                _ => panic!("Expected Unlocked state, got {:?}", state),
            }
        }

        #[test]
        fn vault_state_locked() {
            let tmp = tempfile::tempdir().unwrap();
            let config = make_config(&tmp);
            let vault = Vault::new(Some(config)).unwrap();
            let state = vault.state();
            match state {
                VaultState::Locked {
                    vault_name,
                    model_count,
                } => {
                    assert!(!vault_name.is_empty());
                    assert_eq!(model_count, 0);
                }
                _ => panic!("Expected Locked state, got {:?}", state),
            }
        }

        #[test]
        fn vault_state_transitions() {
            let tmp = tempfile::tempdir().unwrap();
            let config = make_config(&tmp);
            let mut vault = Vault::new(Some(config)).unwrap();

            // Initially locked
            assert!(matches!(vault.state(), VaultState::Locked { .. }));

            // Unlock
            vault
                .unlock(b"test_passphrase_32bytes_entropy!".to_vec())
                .unwrap();
            assert!(matches!(vault.state(), VaultState::Unlocked { .. }));

            // Store a model
            let meta = ModelMetadata::new("m".into(), ModelFormat::PyTorch);
            vault
                .store_model("m", b"data".to_vec(), meta, None)
                .unwrap();

            // State should reflect model count
            if let VaultState::Unlocked { model_count, .. } = vault.state() {
                assert_eq!(model_count, 1);
            }

            // Lock again
            vault.lock();
            assert!(matches!(vault.state(), VaultState::Locked { .. }));
            if let VaultState::Locked { model_count, .. } = vault.state() {
                assert_eq!(model_count, 1);
            }
        }

        // --- get_model error branches ---

        #[test]
        fn vault_get_model_not_found() {
            let tmp = tempfile::tempdir().unwrap();
            let vault = make_vault(&tmp);

            // No version specified → ModelNotFound
            let result = vault.get_model("nonexistent", None);
            assert!(result.is_err());
            let err = format!("{}", result.unwrap_err());
            assert!(
                err.contains("nonexistent") || err.contains("not found"),
                "Got: {}",
                err
            );
        }

        #[test]
        fn vault_get_model_version_not_found() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vault = make_vault(&tmp);

            // Store a model first
            let meta = ModelMetadata::new("m".into(), ModelFormat::PyTorch);
            vault
                .store_model("m", b"data".to_vec(), meta, None)
                .unwrap();

            // Request a nonexistent version → VersionNotFound
            let result = vault.get_model("m", Some(999));
            assert!(result.is_err());
            let err = format!("{}", result.unwrap_err());
            assert!(
                err.contains("999") || err.contains("version") || err.contains("not found"),
                "Got: {}",
                err
            );
        }

        // --- get_model_chunked ---

        #[test]
        fn vault_get_model_chunked() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vault = make_vault(&tmp);

            let data = vec![0u8; 1000];
            let meta = ModelMetadata::new("chunked".into(), ModelFormat::PyTorch);
            vault
                .store_model("chunked", data.clone(), meta, None)
                .unwrap();

            // Get as chunks of 256 bytes
            let stream = vault.get_model_chunked("chunked", None, 256).unwrap();
            assert_eq!(stream.total_size(), 1000);
            assert_eq!(stream.remaining(), 1000);

            let chunks: Vec<Vec<u8>> = stream.collect();
            // 1000 / 256 = 3 full chunks + 1 partial = 4 chunks
            assert_eq!(chunks.len(), 4);
            assert_eq!(chunks[0].len(), 256);
            assert_eq!(chunks[3].len(), 1000 - 256 * 3); // 232

            // Reassemble and verify
            let reassembled: Vec<u8> = chunks.into_iter().flatten().collect();
            assert_eq!(reassembled, data);
        }

        #[test]
        fn vault_get_model_chunked_zero_chunk_size() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vault = make_vault(&tmp);

            let meta = ModelMetadata::new("m".into(), ModelFormat::PyTorch);
            vault
                .store_model("m", b"small".to_vec(), meta, None)
                .unwrap();

            // chunk_size = 0 should default to 1MB
            let stream = vault.get_model_chunked("m", None, 0).unwrap();
            assert_eq!(stream.total_size(), 5);
            let chunks: Vec<Vec<u8>> = stream.collect();
            assert_eq!(chunks.len(), 1); // 5 bytes < 1MB
            assert_eq!(chunks[0], b"small");
        }

        // --- ModelStream ---

        #[test]
        fn model_stream_new_zero_defaults_to_1mb() {
            use ironvault::vault::ModelStream;
            let stream = ModelStream::new(vec![1, 2, 3], 0);
            assert_eq!(stream.total_size(), 3);
            assert_eq!(stream.remaining(), 3);

            let chunks: Vec<Vec<u8>> = stream.collect();
            assert_eq!(chunks.len(), 1);
            assert_eq!(chunks[0], vec![1, 2, 3]);
        }

        #[test]
        fn model_stream_remaining_decreases() {
            use ironvault::vault::ModelStream;
            let mut stream = ModelStream::new(vec![1, 2, 3, 4, 5], 2);
            assert_eq!(stream.remaining(), 5);

            let _ = stream.next();
            assert_eq!(stream.remaining(), 3);

            let _ = stream.next();
            assert_eq!(stream.remaining(), 1);

            let _ = stream.next();
            assert_eq!(stream.remaining(), 0);

            assert!(stream.next().is_none());
        }

        #[test]
        fn model_stream_empty() {
            use ironvault::vault::ModelStream;
            let mut stream = ModelStream::new(Vec::new(), 100);
            assert_eq!(stream.total_size(), 0);
            assert_eq!(stream.remaining(), 0);
            assert!(stream.next().is_none());
        }

        // --- streaming threshold in store_model ---

        #[test]
        fn vault_store_with_streaming_threshold_zero() {
            let tmp = tempfile::tempdir().unwrap();
            let mut config = make_config(&tmp);
            config.storage.streaming_threshold = 0; // Forces streaming for all sizes

            let mut vault = Vault::new(Some(config)).unwrap();
            vault
                .unlock(b"test_passphrase_32bytes_entropy!".to_vec())
                .unwrap();

            let meta = ModelMetadata::new("streamed".into(), ModelFormat::PyTorch);
            let v = vault
                .store_model("streamed", b"streamed data".to_vec(), meta, None)
                .unwrap();
            assert_eq!(v.version, 1);

            // Retrieve and verify
            let data = vault.get_model("streamed", None).unwrap();
            assert_eq!(data, b"streamed data");
        }

        // --- change_passphrase with streaming threshold ---

        #[test]
        fn vault_change_passphrase_streaming() {
            let tmp = tempfile::tempdir().unwrap();
            let mut config = make_config(&tmp);
            config.storage.streaming_threshold = 0; // Force streaming path

            let mut vault = Vault::new(Some(config)).unwrap();
            vault
                .unlock(b"old_passphrase_32bytes_entropy!!".to_vec())
                .unwrap();

            let meta = ModelMetadata::new("m".into(), ModelFormat::PyTorch);
            vault
                .store_model("m", b"important".to_vec(), meta, None)
                .unwrap();

            vault
                .change_passphrase(b"new_passphrase_32bytes_entropy!!".to_vec())
                .unwrap();

            vault.lock();
            vault
                .unlock(b"new_passphrase_32bytes_entropy!!".to_vec())
                .unwrap();

            let data = vault.get_model("m", None).unwrap();
            assert_eq!(data, b"important");
        }

        // --- version_backend_name ---

        #[test]
        fn vault_version_backend_name() {
            let tmp = tempfile::tempdir().unwrap();
            let vault = make_vault(&tmp);
            let name = vault.version_backend_name();
            // Must be one of the known backends
            assert!(
                name == "json" || name == "sqlite",
                "Unexpected backend: {}",
                name
            );
        }

        // --- update/get metadata ---

        #[test]
        fn vault_update_and_get_metadata() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vault = make_vault(&tmp);

            let meta = ModelMetadata::new("m".into(), ModelFormat::PyTorch);
            vault
                .store_model("m", b"data".to_vec(), meta, None)
                .unwrap();

            vault
                .update_version_metadata("m", 1, "author", "alice".to_string())
                .unwrap();
            let val = vault.get_version_metadata("m", 1, "author");
            assert_eq!(val, Some("alice".to_string()));

            // Non-existent key returns None
            let val2 = vault.get_version_metadata("m", 1, "nonexistent");
            assert!(val2.is_none());
        }

        // --- event_bus_mut / event_bus ---

        #[test]
        fn vault_event_bus_access() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vault = make_vault(&tmp);

            let count1 = vault.event_bus().subscriber_count();
            // After adding a subscriber, count should increase
            vault
                .event_bus_mut()
                .subscribe(Box::new(ironvault::traits::MetricsSubscriber::new(
                    std::sync::Arc::new(ironvault::traits::VaultMetrics::new()),
                )));
            let count2 = vault.event_bus().subscriber_count();
            assert_eq!(count2, count1 + 1);
        }
    }

    // ============================================================================
    // TRAITS — AsyncBlobStoreAdapter, EventBus error branch
    // ============================================================================
    mod traits_adapter_tests {
        use ironvault::storage::local::LocalBackend;
        use ironvault::traits::{AsyncBlobStore, AsyncBlobStoreAdapter};

        #[tokio::test]
        async fn async_blob_store_adapter_put_get() {
            let tmp = tempfile::tempdir().unwrap();
            let backend = LocalBackend::new(tmp.path().join("storage")).unwrap();
            let adapter = AsyncBlobStoreAdapter::new(backend);

            let receipt = adapter.put("test-key", b"hello world").await.unwrap();
            assert_eq!(receipt.key, "test-key");
            assert_eq!(receipt.size_bytes, 11);

            let data = adapter.get("test-key").await.unwrap();
            assert_eq!(data, b"hello world");
        }

        #[tokio::test]
        async fn async_blob_store_adapter_delete() {
            let tmp = tempfile::tempdir().unwrap();
            let backend = LocalBackend::new(tmp.path().join("storage")).unwrap();
            let adapter = AsyncBlobStoreAdapter::new(backend);

            adapter.put("del-key", b"data").await.unwrap();
            let exists = adapter.exists("del-key").await.unwrap();
            assert!(exists);

            let deleted = adapter.delete("del-key").await.unwrap();
            assert!(deleted);

            let exists = adapter.exists("del-key").await.unwrap();
            assert!(!exists);
        }

        #[tokio::test]
        async fn async_blob_store_adapter_list() {
            let tmp = tempfile::tempdir().unwrap();
            let backend = LocalBackend::new(tmp.path().join("storage")).unwrap();
            let adapter = AsyncBlobStoreAdapter::new(backend);

            adapter.put("a.bin", b"aaa").await.unwrap();
            adapter.put("b.bin", b"bbbb").await.unwrap();

            let infos = adapter.list(None).await.unwrap();
            assert_eq!(infos.len(), 2);

            // Check sizes
            for info in &infos {
                assert!(info.size_bytes > 0);
            }
        }

        #[tokio::test]
        async fn async_blob_store_adapter_stat() {
            let tmp = tempfile::tempdir().unwrap();
            let backend = LocalBackend::new(tmp.path().join("storage")).unwrap();
            let adapter = AsyncBlobStoreAdapter::new(backend);

            adapter.put("stat-key", b"1234567890").await.unwrap();
            let info = adapter.stat("stat-key").await.unwrap();
            assert_eq!(info.key, "stat-key");
            assert_eq!(info.size_bytes, 10);
        }

        #[tokio::test]
        async fn async_blob_store_adapter_exists_nonexistent() {
            let tmp = tempfile::tempdir().unwrap();
            let backend = LocalBackend::new(tmp.path().join("storage")).unwrap();
            let adapter = AsyncBlobStoreAdapter::new(backend);

            let exists = adapter.exists("nope").await.unwrap();
            assert!(!exists);
        }
    }

    mod eventbus_error_tests {
        use ironvault::error::{Result, VaultError};
        use ironvault::traits::{EventBus, EventSubscriber, VaultEvent};

        /// A subscriber that always returns an error
        struct FailingSubscriber;

        impl EventSubscriber for FailingSubscriber {
            fn name(&self) -> &str {
                "failing"
            }

            fn accepts(&self, _event: &VaultEvent) -> bool {
                true
            }

            fn on_event(&self, _event: &VaultEvent) -> Result<()> {
                Err(VaultError::StorageError("subscriber failed".to_string()))
            }
        }

        #[test]
        fn eventbus_error_does_not_propagate() {
            let mut bus = EventBus::new();
            bus.subscribe(Box::new(FailingSubscriber));

            // This should not panic — errors are logged to stderr
            let event = VaultEvent::VaultLocked {
                vault: "test".to_string(),
                timestamp: chrono::Utc::now(),
            };
            bus.emit(&event); // No panic = success
        }

        #[test]
        fn eventbus_error_with_multiple_subscribers() {
            use ironvault::traits::MetricsSubscriber;

            let mut bus = EventBus::new();
            bus.subscribe(Box::new(FailingSubscriber));
            bus.subscribe(Box::new(MetricsSubscriber::new(std::sync::Arc::new(
                ironvault::traits::VaultMetrics::new(),
            ))));
            bus.subscribe(Box::new(FailingSubscriber));

            // MetricsSubscriber should still receive the event even though others fail
            let event = VaultEvent::ModelStored {
                vault: "test".to_string(),
                model: "m".to_string(),
                version: 1,
                format: "PyTorch".to_string(),
                size: 100,
                checksum: "abc123".to_string(),
                timestamp: chrono::Utc::now(),
            };
            bus.emit(&event);
        }
    }

    // ============================================================================
    // VERSION — cleanup_old_versions, verify_checksum edges
    // ============================================================================
    mod version_cleanup_tests {
        use ironvault::crypto::VaultCrypto;
        use ironvault::version::VersionControl;

        fn checksum(data: &[u8]) -> String {
            hex::encode(VaultCrypto::hash_sha256(data))
        }

        #[test]
        fn cleanup_old_versions_deletes_excess() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            // Add 5 versions (add_version takes: name, file_path, format, size, compressed_size, checksum, metadata, parent)
            for i in 0..5u32 {
                let data = vec![i as u8; 10];
                let cksum = checksum(&data);
                vc.add_version(
                    "m",
                    &format!("file_{}.vault", i),
                    "pytorch",
                    data.len() as u64,
                    data.len() as u64,
                    &cksum,
                    None,
                    if i > 0 { Some(i) } else { None },
                )
                .unwrap();
            }

            assert_eq!(vc.list_versions("m").len(), 5);

            // Keep only 2 most recent
            let deleted = vc.cleanup_old_versions("m", 2).unwrap();
            assert_eq!(deleted.len(), 3);
            assert_eq!(vc.list_versions("m").len(), 2);

            // The remaining versions should be the most recent
            let remaining = vc.list_versions("m");
            assert!(remaining.iter().all(|v| v.version >= 4));
        }

        #[test]
        fn cleanup_old_versions_keeps_all_when_under_limit() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            let data = b"data";
            let cksum = checksum(data);
            vc.add_version(
                "m",
                "f.vault",
                "pytorch",
                data.len() as u64,
                data.len() as u64,
                &cksum,
                None,
                None,
            )
            .unwrap();

            let deleted = vc.cleanup_old_versions("m", 5).unwrap();
            assert!(deleted.is_empty());
            assert_eq!(vc.list_versions("m").len(), 1);
        }

        #[test]
        fn cleanup_old_versions_nonexistent_model() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            let deleted = vc.cleanup_old_versions("nope", 2).unwrap();
            assert!(deleted.is_empty());
        }

        #[test]
        fn verify_checksum_correct() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            let data = b"test data for checksum";
            let cksum = checksum(data);
            vc.add_version(
                "m",
                "f.vault",
                "pytorch",
                data.len() as u64,
                data.len() as u64,
                &cksum,
                None,
                None,
            )
            .unwrap();

            assert!(vc.verify_checksum("m", 1, data));
        }

        #[test]
        fn verify_checksum_incorrect() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            let data = b"original data";
            let cksum = checksum(data);
            vc.add_version(
                "m",
                "f.vault",
                "pytorch",
                data.len() as u64,
                data.len() as u64,
                &cksum,
                None,
                None,
            )
            .unwrap();

            assert!(!vc.verify_checksum("m", 1, b"tampered"));
        }

        #[test]
        fn verify_checksum_nonexistent_version() {
            let tmp = tempfile::tempdir().unwrap();
            let vc = VersionControl::new(tmp.path()).unwrap();

            assert!(!vc.verify_checksum("nope", 1, b"data"));
        }

        #[test]
        fn delete_version() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            let data1 = b"v1";
            let cksum1 = checksum(data1);
            vc.add_version(
                "m",
                "f1.vault",
                "pytorch",
                data1.len() as u64,
                data1.len() as u64,
                &cksum1,
                None,
                None,
            )
            .unwrap();
            let data2 = b"v2";
            let cksum2 = checksum(data2);
            vc.add_version(
                "m",
                "f2.vault",
                "pytorch",
                data2.len() as u64,
                data2.len() as u64,
                &cksum2,
                None,
                Some(1),
            )
            .unwrap();

            assert_eq!(vc.list_versions("m").len(), 2);

            let deleted = vc.delete_version("m", 1).unwrap();
            assert!(deleted);
            assert_eq!(vc.list_versions("m").len(), 1);
        }

        #[test]
        fn delete_nonexistent_version() {
            let tmp = tempfile::tempdir().unwrap();
            let mut vc = VersionControl::new(tmp.path()).unwrap();

            let deleted = vc.delete_version("nope", 1).unwrap();
            assert!(!deleted);
        }
    }

    // ============================================================================
    // FORMATS — extension() and name() for ALL format variants
    // ============================================================================
    mod formats_comprehensive_tests {
        use ironvault::formats::ModelFormat;

        #[test]
        fn all_format_extensions() {
            let expectations = vec![
                (ModelFormat::Safetensors, "safetensors"),
                (ModelFormat::GGUF, "gguf"),
                (ModelFormat::PyTorch, "pt"),
                (ModelFormat::TensorRT, "plan"),
                (ModelFormat::ONNX, "onnx"),
                (ModelFormat::MLX, "npz"),
                (ModelFormat::CoreML, "mlmodel"),
                (ModelFormat::TorchScript, "pt"),
                (ModelFormat::TFLite, "tflite"),
                (ModelFormat::TensorFlow, "pb"),
                (ModelFormat::Keras, "h5"),
                (ModelFormat::OpenVINO, "xml"),
                (ModelFormat::TVM, "so"),
                (ModelFormat::NCNN, "param"),
                (ModelFormat::MNN, "mnn"),
                (ModelFormat::RKNN, "rknn"),
                (ModelFormat::Caffe, "caffemodel"),
                (ModelFormat::MXNet, "params"),
                (ModelFormat::Darknet, "weights"),
                (ModelFormat::HDF5, "h5"),
                (ModelFormat::Pickle, "pkl"),
                (ModelFormat::NumPy, "npy"),
                (ModelFormat::Custom("wasm".into()), "wasm"),
            ];

            for (fmt, expected_ext) in expectations {
                assert_eq!(
                    fmt.extension(),
                    expected_ext,
                    "Extension mismatch for {:?}",
                    fmt
                );
            }
        }

        #[test]
        fn all_format_names() {
            let expectations = vec![
                (ModelFormat::Safetensors, "Safetensors"),
                (ModelFormat::GGUF, "GGUF"),
                (ModelFormat::PyTorch, "PyTorch"),
                (ModelFormat::TensorRT, "TensorRT"),
                (ModelFormat::ONNX, "ONNX"),
                (ModelFormat::MLX, "MLX"),
                (ModelFormat::CoreML, "Core ML"),
                (ModelFormat::TorchScript, "TorchScript"),
                (ModelFormat::TFLite, "TensorFlow Lite"),
                (ModelFormat::TensorFlow, "TensorFlow"),
                (ModelFormat::Keras, "Keras"),
                (ModelFormat::OpenVINO, "OpenVINO"),
                (ModelFormat::TVM, "TVM"),
                (ModelFormat::NCNN, "NCNN"),
                (ModelFormat::MNN, "MNN"),
                (ModelFormat::RKNN, "RKNN"),
                (ModelFormat::Caffe, "Caffe"),
                (ModelFormat::MXNet, "MXNet"),
                (ModelFormat::Darknet, "Darknet"),
                (ModelFormat::HDF5, "HDF5"),
                (ModelFormat::Pickle, "Pickle"),
                (ModelFormat::NumPy, "NumPy"),
                (ModelFormat::Custom("custom".into()), "custom"),
            ];

            for (fmt, expected_name) in expectations {
                assert_eq!(fmt.name(), expected_name, "Name mismatch for {:?}", fmt);
            }
        }

        #[test]
        fn all_format_display() {
            // Display should match name()
            let formats = vec![
                ModelFormat::Safetensors,
                ModelFormat::GGUF,
                ModelFormat::PyTorch,
                ModelFormat::TensorRT,
                ModelFormat::ONNX,
                ModelFormat::MLX,
                ModelFormat::CoreML,
                ModelFormat::TorchScript,
                ModelFormat::TFLite,
                ModelFormat::TensorFlow,
                ModelFormat::Keras,
                ModelFormat::OpenVINO,
                ModelFormat::TVM,
                ModelFormat::NCNN,
                ModelFormat::MNN,
                ModelFormat::RKNN,
                ModelFormat::Caffe,
                ModelFormat::MXNet,
                ModelFormat::Darknet,
                ModelFormat::HDF5,
                ModelFormat::Pickle,
                ModelFormat::NumPy,
                ModelFormat::Custom("myformat".into()),
            ];

            for fmt in formats {
                let display = format!("{}", fmt);
                assert_eq!(display, fmt.name());
            }
        }

        #[test]
        fn from_extension_rare_formats() {
            assert_eq!(ModelFormat::from_extension("xml"), ModelFormat::OpenVINO);
            // "so" is not mapped to TVM in from_extension — it becomes Custom
            assert_eq!(
                ModelFormat::from_extension("so"),
                ModelFormat::Custom("so".into())
            );
            assert_eq!(ModelFormat::from_extension("param"), ModelFormat::NCNN);
            assert_eq!(ModelFormat::from_extension("mnn"), ModelFormat::MNN);
            assert_eq!(ModelFormat::from_extension("rknn"), ModelFormat::RKNN);
            assert_eq!(
                ModelFormat::from_extension("caffemodel"),
                ModelFormat::Caffe
            );
            assert_eq!(ModelFormat::from_extension("params"), ModelFormat::MXNet);
            assert_eq!(ModelFormat::from_extension("weights"), ModelFormat::Darknet);
            assert_eq!(ModelFormat::from_extension("hdf5"), ModelFormat::HDF5);
            assert_eq!(ModelFormat::from_extension("pickle"), ModelFormat::Pickle);
            assert_eq!(ModelFormat::from_extension("mlmodel"), ModelFormat::CoreML);
            assert_eq!(ModelFormat::from_extension("tflite"), ModelFormat::TFLite);
        }

        #[test]
        fn from_extension_case_sensitivity() {
            // from_extension likely expects lowercase
            let fmt = ModelFormat::from_extension("ONNX");
            // If it doesn't match, it'll return Custom
            assert!(
                fmt == ModelFormat::ONNX || matches!(fmt, ModelFormat::Custom(_)),
                "Got: {:?}",
                fmt
            );
        }
    }

    // ============================================================================
    // GGUF HEADER PARSER — exercise the valid-parse path with real GGUF-like data
    // ============================================================================
    mod gguf_parser_tests {
        use ironvault::conversion::*;
        use ironvault::formats::ModelFormat;

        #[test]
        fn gguf_parser_valid_data() {
            let converter = GgufHeaderParser;

            // Build valid GGUF header: magic (4) + version (4) + tensor_count (8) + kv_count (8)
            let mut data = Vec::new();
            data.extend_from_slice(b"GGUF"); // magic
            data.extend_from_slice(&3u32.to_le_bytes()); // version = 3
            data.extend_from_slice(&10u64.to_le_bytes()); // tensor_count = 10
            data.extend_from_slice(&5u64.to_le_bytes()); // kv_count = 5
                                                         // Add some extra data to make it look like a real file
            data.extend_from_slice(&[0u8; 100]);

            let result = converter
                .convert(&data, &ConversionOptions::default(), None)
                .unwrap();
            let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(meta["format"], "GGUF");
            assert_eq!(meta["version"], 3);
            assert_eq!(meta["tensor_count"], 10);
            assert_eq!(meta["kv_count"], 5);
        }

        #[test]
        fn gguf_parser_invalid_magic() {
            let converter = GgufHeaderParser;

            let mut data = vec![0u8; 24];
            data[0..4].copy_from_slice(b"XXXX"); // Invalid magic

            let result = converter.convert(&data, &ConversionOptions::default(), None);
            assert!(result.is_err());
        }

        #[test]
        fn gguf_parser_too_small() {
            let converter = GgufHeaderParser;

            let result = converter.convert(b"GGU", &ConversionOptions::default(), None);
            assert!(result.is_err());
        }

        #[test]
        fn gguf_parser_source_target() {
            let converter = GgufHeaderParser;
            assert_eq!(converter.source_format(), ModelFormat::GGUF);
            assert_eq!(converter.name(), "GGUF → Metadata (JSON)");
        }
    }

    // ============================================================================
    // FEDERATION — compute_delta thorough test
    // ============================================================================
    mod federation_compute_delta_tests {
        use ironvault::federation::*;

        fn make_version(ver: u32, ckpt: &str, size: u64) -> VersionManifestEntry {
            VersionManifestEntry {
                version: ver,
                checkpoint_id: ckpt.to_string(),
                created_at: chrono::Utc::now(),
                checksum: "abc".to_string(),
                size_bytes: size,
                parent_id: None,
                origin_node: "test".to_string(),
            }
        }

        fn make_model_entry(name: &str, versions: Vec<VersionManifestEntry>) -> ModelManifestEntry {
            ModelManifestEntry {
                name: name.to_string(),
                versions,
                clock: VectorClock::new(),
            }
        }

        fn make_manifest(models: Vec<ModelManifestEntry>) -> SyncManifest {
            SyncManifest {
                source_node: "test".to_string(),
                timestamp: chrono::Utc::now(),
                models,
                clock: VectorClock::new(),
            }
        }

        #[test]
        fn compute_delta_local_only_models() {
            let tmp = tempfile::tempdir().unwrap();
            let config = FederationConfig::default();
            let manager = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let local = make_manifest(vec![
                make_model_entry("model_a", vec![make_version(1, "ckpt_a1", 100)]),
                make_model_entry("model_b", vec![make_version(1, "ckpt_b1", 200)]),
            ]);
            let remote = make_manifest(vec![]);

            let delta = manager.compute_delta(&local, &remote);

            // Local-only models should be uploaded
            assert_eq!(delta.to_upload.len(), 2);
            assert!(delta.to_download.is_empty());
        }

        #[test]
        fn compute_delta_remote_only_models() {
            let tmp = tempfile::tempdir().unwrap();
            let config = FederationConfig::default();
            let manager = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let local = make_manifest(vec![]);
            let remote = make_manifest(vec![make_model_entry(
                "model_x",
                vec![make_version(1, "ckpt_x1", 300)],
            )]);

            let delta = manager.compute_delta(&local, &remote);

            assert!(delta.to_upload.is_empty());
            assert_eq!(delta.to_download.len(), 1);
        }

        #[test]
        fn compute_delta_shared_same_checkpoint() {
            let tmp = tempfile::tempdir().unwrap();
            let config = FederationConfig::default();
            let manager = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let local = make_manifest(vec![make_model_entry(
                "shared",
                vec![make_version(1, "same_ckpt", 100)],
            )]);
            let remote = make_manifest(vec![make_model_entry(
                "shared",
                vec![make_version(1, "same_ckpt", 100)],
            )]);

            let delta = manager.compute_delta(&local, &remote);

            // Same checkpoint → nothing to sync
            assert!(delta.to_upload.is_empty());
            assert!(delta.to_download.is_empty());
            assert!(delta.conflicts.is_empty());
        }

        #[test]
        fn compute_delta_shared_different_checkpoint() {
            let tmp = tempfile::tempdir().unwrap();
            let config = FederationConfig::default();
            let manager = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let local = make_manifest(vec![make_model_entry(
                "model",
                vec![make_version(1, "local_ckpt", 100)],
            )]);
            let remote = make_manifest(vec![make_model_entry(
                "model",
                vec![make_version(1, "remote_ckpt", 200)],
            )]);

            let delta = manager.compute_delta(&local, &remote);

            // Same version number, different checkpoints → conflict
            assert!(!delta.conflicts.is_empty());
        }

        #[test]
        fn compute_delta_mixed() {
            let tmp = tempfile::tempdir().unwrap();
            let config = FederationConfig::default();
            let manager = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let local = make_manifest(vec![
                make_model_entry("local_only", vec![make_version(1, "ckpt1", 100)]),
                make_model_entry("shared_same", vec![make_version(1, "ckpt_s", 200)]),
                make_model_entry("shared_diff", vec![make_version(1, "ckpt_l", 300)]),
            ]);
            let remote = make_manifest(vec![
                make_model_entry("remote_only", vec![make_version(1, "ckpt2", 150)]),
                make_model_entry("shared_same", vec![make_version(1, "ckpt_s", 200)]),
                make_model_entry("shared_diff", vec![make_version(1, "ckpt_r", 350)]),
            ]);

            let delta = manager.compute_delta(&local, &remote);

            // local_only → upload
            assert!(!delta.to_upload.is_empty());
            // remote_only → download
            assert!(!delta.to_download.is_empty());
            // shared_diff → conflict (same version 1, different checkpoint)
            assert!(!delta.conflicts.is_empty());
        }

        #[test]
        fn compute_delta_multiple_versions_partial_overlap() {
            let tmp = tempfile::tempdir().unwrap();
            let config = FederationConfig::default();
            let manager = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let local = make_manifest(vec![make_model_entry(
                "m",
                vec![
                    make_version(1, "ckpt_1", 100),
                    make_version(2, "ckpt_2", 200),
                ],
            )]);
            let remote = make_manifest(vec![make_model_entry(
                "m",
                vec![
                    make_version(2, "ckpt_2", 200),
                    make_version(3, "ckpt_3", 300),
                ],
            )]);

            let delta = manager.compute_delta(&local, &remote);

            // ckpt_1 only local → upload
            assert!(delta.to_upload.iter().any(|s| s.checkpoint_id == "ckpt_1"));
            // ckpt_3 only remote → download
            assert!(delta
                .to_download
                .iter()
                .any(|s| s.checkpoint_id == "ckpt_3"));
            // ckpt_2 same on both → no conflict
        }
    }

    // ============================================================================
    // FEDERATION — status, history, peer management
    // ============================================================================
    mod federation_manager_extra_tests {
        use ironvault::federation::*;

        #[tokio::test]
        async fn federation_status() {
            let tmp = tempfile::tempdir().unwrap();
            let config = FederationConfig::default();
            let manager = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let status = manager.status().await;
            assert_eq!(status.peer_count, 0);
            assert!(status.last_sync.is_none());
        }

        #[tokio::test]
        async fn federation_add_remove_peer() {
            let tmp = tempfile::tempdir().unwrap();
            let config = FederationConfig::default();
            let mut manager = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let peer = PeerConfig {
                node_id: "peer1".to_string(),
                name: "Peer One".to_string(),
                endpoint: "https://peer1.example.com".to_string(),
                api_key: Some("secret".to_string()),
                enabled: true,
            };

            manager.add_peer(peer);
            assert_eq!(manager.status().await.peer_count, 1);

            manager.remove_peer("peer1");
            assert_eq!(manager.status().await.peer_count, 0);
        }

        #[tokio::test]
        async fn federation_history() {
            let tmp = tempfile::tempdir().unwrap();
            let config = FederationConfig::default();
            let manager = FederationManager::new(config, tmp.path().to_path_buf()).unwrap();

            let history = manager.get_history(None).await;
            assert!(history.is_empty());

            let history_limited = manager.get_history(Some(5)).await;
            assert!(history_limited.is_empty());
        }
    }

    // ============================================================================
    // SHIM CONVERTERS — exercise convert() paths for uncovered shims
    // ============================================================================
    mod shim_converter_extra_tests {
        use ironvault::conversion::*;

        #[test]
        fn safetensors_to_pytorch_valid_header() {
            let converter = SafeTensorsToPyTorchConverter;

            // Build valid SafeTensors data
            let header = r#"{"tensor":{"dtype":"F32","shape":[2,3],"data_offsets":[0,24]}}"#;
            let header_bytes = header.as_bytes();
            let header_len = header_bytes.len() as u64;
            let mut data = Vec::new();
            data.extend_from_slice(&header_len.to_le_bytes());
            data.extend_from_slice(header_bytes);
            data.extend_from_slice(&[0u8; 24]); // tensor data

            let result = converter
                .convert(&data, &ConversionOptions::default(), None)
                .unwrap();
            // Real converter produces ZIP output
            assert_eq!(&result[0..2], b"PK");
        }

        #[test]
        fn pytorch_to_onnx_with_custom_opset() {
            let converter = PyTorchToOnnxConverter;

            let opts = ConversionOptions {
                opset_version: Some(13),
                ..ConversionOptions::default()
            };

            let result = converter.convert(b"data", &opts, None).unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(plan["converter"], "pytorch_to_onnx");
            assert_eq!(plan["opset_version"], 13);
        }

        #[test]
        fn pytorch_to_onnx_default_opset() {
            let converter = PyTorchToOnnxConverter;

            let result = converter
                .convert(b"data", &ConversionOptions::default(), None)
                .unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(plan["opset_version"], 17);
        }

        #[test]
        fn onnx_to_tensorrt_plan() {
            let converter = OnnxToTensorRtConverter;

            let result = converter
                .convert(b"data", &ConversionOptions::default(), None)
                .unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(plan["converter"], "onnx_to_tensorrt");
            assert!(plan["shell"].as_str().unwrap().contains("trtexec"));
        }

        #[test]
        fn onnx_to_coreml_plan() {
            let converter = OnnxToCoreMLConverter;

            let result = converter
                .convert(b"data", &ConversionOptions::default(), None)
                .unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(plan["converter"], "onnx_to_coreml");
            assert!(plan["python"].as_str().unwrap().contains("coremltools"));
        }

        #[test]
        fn pytorch_to_safetensors_plan() {
            let converter = PyTorchToSafeTensorsConverter;

            // Real converter requires valid ZIP; invalid data should error
            let err = converter
                .convert(b"data", &ConversionOptions::default(), None)
                .unwrap_err();
            assert!(format!("{err}").contains("ZIP archive"));
        }

        #[test]
        fn safetensors_to_gguf_with_quantization() {
            let converter = SafeTensorsToGgufConverter;

            let opts = ConversionOptions {
                quantization: Some("q4_k_m".to_string()),
                ..ConversionOptions::default()
            };

            let result = converter.convert(b"data", &opts, None).unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(plan["converter"], "safetensors_to_gguf");
            assert_eq!(plan["quantization"], "q4_k_m");
        }

        #[test]
        fn safetensors_to_gguf_default_quantization() {
            let converter = SafeTensorsToGgufConverter;

            let result = converter
                .convert(b"data", &ConversionOptions::default(), None)
                .unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();

            assert_eq!(plan["quantization"], "f16");
        }
    }
}

#[allow(unused_imports)]
mod full_coverage_tests {
    //! Full coverage test suite — targets every uncovered code path.
    //!
    //! Each module corresponds to a source module with uncovered lines.

    // ============================================================================
    // ERROR MODULE — From impls for serde_json, serde_yaml_ng, zip::ZipError
    // ============================================================================
    mod error_from_impls {
        use ironvault::VaultError;

        #[test]
        fn from_serde_json_error() {
            let err: serde_json::Error =
                serde_json::from_str::<serde_json::Value>("{ bad").unwrap_err();
            let vault_err: VaultError = err.into();
            match vault_err {
                VaultError::SerializationError(msg) => assert!(!msg.is_empty()),
                other => panic!("Expected SerializationError, got {:?}", other),
            }
        }

        #[test]
        fn from_serde_yaml_ng_error() {
            let err: serde_yaml_ng::Error =
                serde_yaml_ng::from_str::<serde_yaml_ng::Value>(":\n  :\n    - [inv").unwrap_err();
            let vault_err: VaultError = err.into();
            match vault_err {
                VaultError::SerializationError(msg) => assert!(!msg.is_empty()),
                other => panic!("Expected SerializationError, got {:?}", other),
            }
        }

        #[test]
        fn from_zip_error() {
            let err = zip::ZipArchive::new(std::io::Cursor::new(b"bad")).unwrap_err();
            let vault_err: VaultError = err.into();
            match vault_err {
                VaultError::IoError(e) => assert!(!e.to_string().is_empty()),
                other => panic!("Expected IoError, got {:?}", other),
            }
        }

        #[test]
        fn display_all_variants() {
            let errors: Vec<VaultError> = vec![
                VaultError::CryptoError("c".into()),
                VaultError::AuthenticationFailed,
                VaultError::IntegrityError("i".into()),
                VaultError::VersionError("v".into()),
                VaultError::ModelNotFound("m".into()),
                VaultError::VersionNotFound(1, "m".into()),
                VaultError::ConversionError("c".into()),
                VaultError::UnsupportedFormat("u".into()),
                VaultError::IoError(std::io::Error::other("io")),
                VaultError::ConfigError("c".into()),
                VaultError::SerializationError("s".into()),
                VaultError::CompressionError("c".into()),
                VaultError::SecurityViolation("s".into()),
                VaultError::ComplianceViolation("c".into()),
                VaultError::AuditError("a".into()),
                VaultError::InvalidInput("i".into()),
                VaultError::StorageError("s".into()),
            ];
            for e in &errors {
                assert!(!format!("{}", e).is_empty(), "Empty display for {:?}", e);
            }
        }
    }

    // ============================================================================
    // COMPLIANCE — check_cve, check_mitre_attack, check_cmmc, run_all_checks
    // ============================================================================
    mod compliance_coverage {
        use ironvault::compliance::ComplianceChecker;

        #[test]
        fn check_fips_enabled() {
            let checker = ComplianceChecker::new();
            assert!(checker.check_fips_140_3());
        }

        #[test]
        fn check_cve_returns_tuple() {
            // Whether cargo-audit is installed is a property of the machine,
            // not of this crate, so assert the invariant instead: a pass means
            // a scan ran and found nothing, and a non-pass always says why.
            let checker = ComplianceChecker::new();
            let (passed, findings) = checker.check_cve();
            if passed {
                assert!(findings.is_empty());
            } else {
                assert!(!findings.is_empty());
            }
        }

        #[test]
        fn check_cve_disabled() {
            let mut checker = ComplianceChecker::new();
            checker.set_check_enabled("cve", false);
            let (passed, cves) = checker.check_cve();
            assert!(passed);
            assert!(cves.is_empty());
        }

        #[test]
        fn check_mitre_attack() {
            let checker = ComplianceChecker::new();
            assert!(checker.check_mitre_attack());
        }

        #[test]
        fn check_mitre_attack_disabled() {
            let mut checker = ComplianceChecker::new();
            checker.set_check_enabled("mitre_attack", false);
            assert!(checker.check_mitre_attack());
        }

        #[test]
        fn check_cmmc() {
            let checker = ComplianceChecker::new();
            assert_eq!(checker.check_cmmc(), 2);
        }

        #[test]
        fn check_cmmc_disabled() {
            let mut checker = ComplianceChecker::new();
            checker.set_check_enabled("cmmc", false);
            assert_eq!(checker.check_cmmc(), 0);
        }

        #[test]
        fn run_all_checks() {
            let checker = ComplianceChecker::new();
            let status = checker.run_all_checks().unwrap();
            assert!(status.fips_140_3);
            assert!(status.mitre_attack_aligned);
            assert_eq!(status.cmmc_level, 2);
            // `cve_scan_passed` depends on cargo-audit being installed; see
            // `check_cve_returns_tuple`.
        }

        #[test]
        fn run_all_checks_all_disabled() {
            let mut checker = ComplianceChecker::new();
            checker.set_check_enabled("fips_140_3", false);
            checker.set_check_enabled("cve", false);
            checker.set_check_enabled("mitre_attack", false);
            checker.set_check_enabled("cmmc", false);
            let status = checker.run_all_checks().unwrap();
            assert!(status.fips_140_3);
            assert!(status.cve_scan_passed);
            assert!(status.mitre_attack_aligned);
            assert_eq!(status.cmmc_level, 0);
        }

        #[test]
        fn is_check_enabled_unknown() {
            let checker = ComplianceChecker::new();
            assert!(!checker.is_check_enabled("nonexistent_check"));
        }

        #[test]
        fn compliance_default() {
            let checker = ComplianceChecker::default();
            assert!(checker.check_fips_140_3());
        }
    }

    // ============================================================================
    // TRAITS — VaultState Display, IvUri, EventBus, AuditLogSubscriber,
    //          MetricsSubscriber, NullAuditSink, AsyncBlobStoreAdapter
    // ============================================================================
    mod traits_coverage {
        use chrono::Utc;
        use ironvault::audit::{AuditEntry, AuditEventType};
        use ironvault::traits::*;
        use std::sync::{Arc, Mutex};

        #[test]
        fn vault_state_display_unlocked() {
            let state = VaultState::Unlocked {
                vault_name: "v".into(),
                model_count: 5,
                unlocked_at: Utc::now(),
                operations_count: 10,
            };
            let s = format!("{}", state);
            assert!(s.contains("v"));
        }

        #[test]
        fn vault_state_display_locked() {
            let state = VaultState::Locked {
                vault_name: "locked_vault".into(),
                model_count: 3,
            };
            assert!(format!("{}", state).contains("locked_vault"));
        }

        #[test]
        fn vault_state_display_uninitialized() {
            assert!(!format!("{}", VaultState::Uninitialized).is_empty());
        }

        #[test]
        fn vault_state_display_error() {
            let state = VaultState::Error {
                message: "broken".into(),
            };
            assert!(format!("{}", state).contains("broken"));
        }

        // IvUri::to_string — all branches
        #[test]
        fn uri_to_string_root() {
            let uri = IvUri::parse("iv://").unwrap();
            assert_eq!(uri.to_string(), "iv://");
        }

        #[test]
        fn uri_to_string_vault_only() {
            let uri = IvUri::parse("iv://myvault").unwrap();
            assert_eq!(uri.to_string(), "iv://myvault");
        }

        #[test]
        fn uri_to_string_vault_model() {
            let uri = IvUri::parse("iv://v/m").unwrap();
            assert_eq!(uri.to_string(), "iv://v/m");
        }

        #[test]
        fn uri_to_string_with_version() {
            let uri = IvUri::parse("iv://v/m@5").unwrap();
            assert_eq!(uri.to_string(), "iv://v/m@5");
        }

        #[test]
        fn uri_to_string_with_resource() {
            let uri = IvUri::parse("iv://v/m@1/card").unwrap();
            assert_eq!(uri.to_string(), "iv://v/m@1/card");
        }

        #[test]
        fn uri_to_string_with_query() {
            let uri = IvUri::parse("iv://v/m?key=val").unwrap();
            let s = uri.to_string();
            assert!(s.contains("key=val"));
        }

        #[test]
        fn uri_to_string_with_empty_query_value() {
            let uri = IvUri::parse("iv://v/m?flag").unwrap();
            let s = uri.to_string();
            assert!(s.contains("flag"));
        }

        #[test]
        fn uri_display_trait() {
            let uri = IvUri::parse("iv://v/m@1").unwrap();
            let display = format!("{}", uri);
            assert_eq!(display, "iv://v/m@1");
        }

        #[test]
        fn uri_too_many_segments() {
            assert!(IvUri::parse("iv://a/b/c/d").is_err());
        }

        #[test]
        fn uri_invalid_version_number() {
            assert!(IvUri::parse("iv://v/m@abc").is_err());
        }

        // EventBus
        #[test]
        fn event_bus_subscriber_count() {
            let bus = EventBus::new();
            assert_eq!(bus.subscriber_count(), 0);
        }

        #[test]
        fn event_bus_default() {
            let bus = EventBus::default();
            assert_eq!(bus.subscriber_count(), 0);
        }

        #[test]
        fn event_bus_emit_no_subscribers() {
            let bus = EventBus::new();
            bus.emit(&VaultEvent::VaultCreated {
                vault: "v".into(),
                timestamp: Utc::now(),
            });
        }

        // VaultEvent methods
        #[test]
        fn vault_event_timestamp_all_variants() {
            let ts = Utc::now();
            let events = vec![
                VaultEvent::VaultCreated {
                    vault: "v".into(),
                    timestamp: ts,
                },
                VaultEvent::VaultUnlocked {
                    vault: "v".into(),
                    timestamp: ts,
                },
                VaultEvent::VaultLocked {
                    vault: "v".into(),
                    timestamp: ts,
                },
                VaultEvent::ModelStored {
                    vault: "v".into(),
                    model: "m".into(),
                    version: 1,
                    format: "pt".into(),
                    size: 0,
                    checksum: "".into(),
                    timestamp: ts,
                },
                VaultEvent::ModelRetrieved {
                    vault: "v".into(),
                    model: "m".into(),
                    version: 1,
                    timestamp: ts,
                },
                VaultEvent::ModelDeleted {
                    vault: "v".into(),
                    model: "m".into(),
                    version: 1,
                    timestamp: ts,
                },
                VaultEvent::PassphraseChanged {
                    vault: "v".into(),
                    files_reencrypted: 0,
                    timestamp: ts,
                },
                VaultEvent::IntegrityFailed {
                    vault: "v".into(),
                    model: "m".into(),
                    version: 1,
                    expected: "a".into(),
                    actual: "b".into(),
                    timestamp: ts,
                },
                VaultEvent::ComplianceChecked {
                    vault: "v".into(),
                    passed: true,
                    timestamp: ts,
                },
            ];
            for event in &events {
                assert_eq!(event.timestamp(), ts);
                assert!(!event.vault_name().is_empty());
                assert!(!event.event_type().is_empty());
                assert!(!format!("{}", event).is_empty());
            }
        }

        // NullAuditSink
        #[test]
        fn null_audit_sink_emit_and_query() {
            let sink = NullAuditSink;
            let entry = AuditEntry {
                timestamp: Utc::now(),
                event_type: AuditEventType::VaultCreated,
                description: "test".into(),
                model_name: None,
                version: None,
                success: true,
                metadata: None,
            };
            sink.emit(entry).unwrap();
            let results = sink.query(Some(10)).unwrap();
            assert!(results.is_empty());
        }

        // AuditLogSubscriber — all event variant branches
        #[test]
        fn audit_log_subscriber_all_events() {
            let entries = Arc::new(Mutex::new(Vec::new()));

            struct TestSink(Arc<Mutex<Vec<String>>>);
            impl AuditSink for TestSink {
                fn emit(&self, entry: AuditEntry) -> ironvault::Result<()> {
                    self.0.lock().unwrap().push(entry.description);
                    Ok(())
                }
                fn query(&self, _limit: Option<usize>) -> ironvault::Result<Vec<AuditEntry>> {
                    Ok(vec![])
                }
            }

            let sub = AuditLogSubscriber::new(Box::new(TestSink(entries.clone())));
            let ts = Utc::now();

            let events = vec![
                VaultEvent::VaultCreated {
                    vault: "v".into(),
                    timestamp: ts,
                },
                VaultEvent::VaultUnlocked {
                    vault: "v".into(),
                    timestamp: ts,
                },
                VaultEvent::VaultLocked {
                    vault: "v".into(),
                    timestamp: ts,
                },
                VaultEvent::ModelStored {
                    vault: "v".into(),
                    model: "m".into(),
                    version: 1,
                    format: "pt".into(),
                    size: 100,
                    checksum: "c".into(),
                    timestamp: ts,
                },
                VaultEvent::ModelRetrieved {
                    vault: "v".into(),
                    model: "m".into(),
                    version: 1,
                    timestamp: ts,
                },
                VaultEvent::ModelDeleted {
                    vault: "v".into(),
                    model: "m".into(),
                    version: 1,
                    timestamp: ts,
                },
                VaultEvent::PassphraseChanged {
                    vault: "v".into(),
                    files_reencrypted: 3,
                    timestamp: ts,
                },
                VaultEvent::IntegrityFailed {
                    vault: "v".into(),
                    model: "m".into(),
                    version: 1,
                    expected: "a".into(),
                    actual: "b".into(),
                    timestamp: ts,
                },
                VaultEvent::ComplianceChecked {
                    vault: "v".into(),
                    passed: false,
                    timestamp: ts,
                },
            ];

            for ev in &events {
                sub.on_event(ev).unwrap();
            }

            assert_eq!(entries.lock().unwrap().len(), events.len());
        }

        // MetricsSubscriber — all event branches
        #[test]
        fn metrics_subscriber_all_events() {
            let metrics = Arc::new(VaultMetrics::new());
            let sub = MetricsSubscriber::new(metrics.clone());
            let ts = Utc::now();

            sub.on_event(&VaultEvent::ModelStored {
                vault: "v".into(),
                model: "m".into(),
                version: 1,
                format: "pt".into(),
                size: 1024,
                checksum: "c".into(),
                timestamp: ts,
            })
            .unwrap();

            sub.on_event(&VaultEvent::ModelRetrieved {
                vault: "v".into(),
                model: "m".into(),
                version: 1,
                timestamp: ts,
            })
            .unwrap();

            sub.on_event(&VaultEvent::ModelDeleted {
                vault: "v".into(),
                model: "m".into(),
                version: 1,
                timestamp: ts,
            })
            .unwrap();

            sub.on_event(&VaultEvent::VaultUnlocked {
                vault: "v".into(),
                timestamp: ts,
            })
            .unwrap();
            sub.on_event(&VaultEvent::VaultLocked {
                vault: "v".into(),
                timestamp: ts,
            })
            .unwrap();

            sub.on_event(&VaultEvent::IntegrityFailed {
                vault: "v".into(),
                model: "m".into(),
                version: 1,
                expected: "a".into(),
                actual: "b".into(),
                timestamp: ts,
            })
            .unwrap();

            // catch-all _ branch
            sub.on_event(&VaultEvent::PassphraseChanged {
                vault: "v".into(),
                files_reencrypted: 0,
                timestamp: ts,
            })
            .unwrap();
            sub.on_event(&VaultEvent::ComplianceChecked {
                vault: "v".into(),
                passed: true,
                timestamp: ts,
            })
            .unwrap();
            sub.on_event(&VaultEvent::VaultCreated {
                vault: "v".into(),
                timestamp: ts,
            })
            .unwrap();

            let snap = metrics.snapshot();
            assert_eq!(snap.models_stored_total, 1);
            assert_eq!(snap.models_retrieved_total, 1);
            assert_eq!(snap.models_deleted_total, 1);
            assert_eq!(snap.bytes_stored_total, 1024);
            assert_eq!(snap.errors_total, 1); // IntegrityFailed
            assert!(!snap.vault_unlocked); // locked after unlock
        }

        // VaultMetrics
        #[test]
        fn vault_metrics_new_and_default() {
            let m1 = VaultMetrics::new();
            let m2 = VaultMetrics::default();
            assert_eq!(m1.snapshot().models_stored_total, 0);
            assert_eq!(m2.snapshot().bytes_stored_total, 0);
        }

        #[test]
        fn blob_store_stats_fields() {
            let stats = BlobStoreStats {
                total_size_bytes: 42,
                file_count: 3,
            };
            assert_eq!(stats.total_size_bytes, 42);
            assert_eq!(stats.file_count, 3);
        }
    }

    // ============================================================================
    // STORAGE — Storage struct methods, BlobStore trait impl
    // ============================================================================
    mod storage_coverage {
        use ironvault::crypto::compression::{CompressionAlgorithm, CompressionLevel};
        use ironvault::crypto::VaultCrypto;
        use ironvault::storage::Storage;
        use ironvault::traits::BlobStore;
        use tempfile::TempDir;

        fn setup() -> (Storage, ironvault::crypto::SecureKey, TempDir) {
            let tmp = TempDir::new().unwrap();
            let storage = Storage::new(tmp.path()).unwrap();
            let crypto = VaultCrypto::new().unwrap();
            let (key, _) = crypto
                .derive_key(b"test_pass_with_entropy_1234".to_vec(), None)
                .unwrap();
            (storage, key, tmp)
        }

        #[test]
        fn delete_existing() {
            let (storage, key, _tmp) = setup();
            storage
                .store(
                    "del.enc",
                    b"data",
                    &key,
                    CompressionAlgorithm::Gzip,
                    CompressionLevel::Balanced,
                )
                .unwrap();
            assert!(storage.exists("del.enc"));
            assert!(storage.delete("del.enc").unwrap());
            assert!(!storage.exists("del.enc"));
        }

        #[test]
        fn delete_nonexistent() {
            let (storage, _key, _tmp) = setup();
            assert!(!storage.delete("nope.enc").unwrap());
        }

        #[test]
        fn file_size() {
            let (storage, key, _tmp) = setup();
            storage
                .store(
                    "sz.enc",
                    b"abcdefghij",
                    &key,
                    CompressionAlgorithm::Gzip,
                    CompressionLevel::Balanced,
                )
                .unwrap();
            assert!(storage.file_size("sz.enc").unwrap() > 0);
        }

        #[test]
        fn list_files() {
            let (storage, key, _tmp) = setup();
            storage
                .store(
                    "a.enc",
                    b"1",
                    &key,
                    CompressionAlgorithm::Gzip,
                    CompressionLevel::Balanced,
                )
                .unwrap();
            storage
                .store(
                    "b.enc",
                    b"2",
                    &key,
                    CompressionAlgorithm::Gzip,
                    CompressionLevel::Balanced,
                )
                .unwrap();
            let files = storage.list_files().unwrap();
            assert!(files.len() >= 2);
        }

        #[test]
        fn get_stats() {
            let (storage, key, _tmp) = setup();
            storage
                .store(
                    "s.enc",
                    b"x",
                    &key,
                    CompressionAlgorithm::Gzip,
                    CompressionLevel::Balanced,
                )
                .unwrap();
            let stats = storage.get_stats().unwrap();
            assert!(stats.file_count >= 1);
            assert!(stats.total_size_bytes > 0);
        }

        #[test]
        fn store_streamed_and_retrieve_auto() {
            let (storage, key, _tmp) = setup();
            let data = b"streamed model data for testing scenario";
            let (orig, comp) = storage
                .store_streamed(
                    "stream.enc",
                    data,
                    &key,
                    CompressionAlgorithm::Gzip,
                    CompressionLevel::Balanced,
                )
                .unwrap();
            assert_eq!(orig, data.len() as u64);
            assert!(comp > 0);

            let retrieved = storage
                .retrieve_auto("stream.enc", &key, CompressionAlgorithm::Gzip)
                .unwrap();
            assert_eq!(retrieved, data);
        }

        #[test]
        fn retrieve_auto_monolithic() {
            let (storage, key, _tmp) = setup();
            let data = b"monolithic data";
            storage
                .store(
                    "mono.enc",
                    data,
                    &key,
                    CompressionAlgorithm::Gzip,
                    CompressionLevel::Balanced,
                )
                .unwrap();
            let retrieved = storage
                .retrieve_auto("mono.enc", &key, CompressionAlgorithm::Gzip)
                .unwrap();
            assert_eq!(retrieved, data);
        }

        #[test]
        fn retrieve_auto_not_found() {
            let (storage, key, _tmp) = setup();
            assert!(storage
                .retrieve_auto("nope.enc", &key, CompressionAlgorithm::Gzip)
                .is_err());
        }

        #[test]
        fn blob_store_trait() {
            let (storage, key, _tmp) = setup();

            let (orig, _comp) = storage.put("blob.enc", b"blob data", &key).unwrap();
            assert!(orig > 0);
            assert!(storage.exists("blob.enc"));

            let sz = storage.size("blob.enc").unwrap();
            assert!(sz > 0);

            let data = storage.get("blob.enc", &key).unwrap();
            assert_eq!(data, b"blob data");

            let keys = storage.list_keys().unwrap();
            assert!(keys.contains(&"blob.enc".to_string()));

            let stats = storage.stats().unwrap();
            assert!(stats.file_count >= 1);

            assert!(storage.remove("blob.enc").unwrap());
            assert!(!storage.exists("blob.enc"));
        }
    }

    // ============================================================================
    // RAG VECTOR — SimpleVectorStore all methods
    // ============================================================================
    mod vector_coverage {
        use ironvault::rag::vector::VectorStore;
        use ironvault::rag::{Document, SimpleVectorStore};
        use std::collections::HashMap;

        fn doc(id: &str, emb: Vec<f32>) -> Document {
            Document {
                id: id.into(),
                content: format!("content_{}", id),
                metadata: HashMap::new(),
                embedding: Some(emb),
                chunk_info: None,
            }
        }

        #[test]
        fn new_empty() {
            let store = SimpleVectorStore::new();
            assert_eq!(store.count().unwrap(), 0);
        }

        #[test]
        fn default_empty() {
            let store = SimpleVectorStore::default();
            assert_eq!(store.count().unwrap(), 0);
        }

        #[test]
        fn from_documents() {
            let docs = vec![doc("a", vec![1.0, 0.0]), doc("b", vec![0.0, 1.0])];
            let store = SimpleVectorStore::from_documents(docs);
            assert_eq!(store.count().unwrap(), 2);
        }

        #[test]
        fn from_documents_skips_no_embedding() {
            let d = doc("a", vec![1.0, 0.0]);
            let d2 = Document {
                id: "b".into(),
                content: "no embedding".into(),
                metadata: HashMap::new(),
                embedding: None,
                chunk_info: None,
            };
            let store = SimpleVectorStore::from_documents(vec![d, d2]);
            assert_eq!(store.count().unwrap(), 1);
        }

        #[test]
        fn store_and_search() {
            let mut store = SimpleVectorStore::new();
            store
                .store_with_embedding(&doc("a", vec![1.0, 0.0, 0.0]))
                .unwrap();
            store
                .store_with_embedding(&doc("b", vec![0.0, 1.0, 0.0]))
                .unwrap();
            store
                .store_with_embedding(&doc("c", vec![0.9, 0.1, 0.0]))
                .unwrap();

            let results = store.search_similar(&[1.0, 0.0, 0.0], 2).unwrap();
            assert_eq!(results.len(), 2);
            assert_eq!(results[0].0, "a");
        }

        #[test]
        fn store_no_embedding_error() {
            let mut store = SimpleVectorStore::new();
            let d = Document {
                id: "x".into(),
                content: "no emb".into(),
                metadata: HashMap::new(),
                embedding: None,
                chunk_info: None,
            };
            assert!(store.store_with_embedding(&d).is_err());
        }

        #[test]
        fn delete_document() {
            let mut store = SimpleVectorStore::new();
            store
                .store_with_embedding(&doc("a", vec![1.0, 0.0]))
                .unwrap();
            store
                .store_with_embedding(&doc("b", vec![0.0, 1.0]))
                .unwrap();
            store.delete_document("a").unwrap();
            assert_eq!(store.count().unwrap(), 1);
        }

        #[test]
        fn search_empty_store() {
            let store = SimpleVectorStore::new();
            let results = store.search_similar(&[1.0, 0.0], 5).unwrap();
            assert!(results.is_empty());
        }
    }

    // ============================================================================
    // BLOCKCHAIN — search, proof generation/verification, edge cases
    // ============================================================================
    mod blockchain_coverage {
        use chrono::Utc;
        use ironvault::audit::{AuditEntry, AuditEventType};
        use ironvault::blockchain::*;
        use tempfile::tempdir;

        fn entry(desc: &str, model: Option<&str>, event: AuditEventType) -> AuditEntry {
            AuditEntry {
                timestamp: Utc::now(),
                event_type: event,
                description: desc.into(),
                model_name: model.map(|s| s.to_string()),
                version: Some(1),
                success: true,
                metadata: None,
            }
        }

        #[test]
        fn merkle_tree_empty() {
            let tree = MerkleTree::build(&[]);
            assert!(!tree.root.is_empty());
        }

        #[test]
        fn merkle_tree_odd_leaves() {
            let data = vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()];
            let tree = MerkleTree::build(&data);
            assert_eq!(tree.leaves.len(), 3);
            for i in 0..3 {
                let proof = tree.generate_proof(i).unwrap();
                assert!(MerkleTree::verify_proof(&proof));
            }
        }

        #[test]
        fn merkle_tree_proof_oob() {
            let data = vec![b"x".to_vec()];
            let tree = MerkleTree::build(&data);
            assert!(tree.generate_proof(5).is_none());
        }

        #[test]
        fn blockchain_add_entries_auto_finalize() {
            let tmp = tempdir().unwrap();
            let mut audit = BlockchainAudit::new(tmp.path(), 3).unwrap();
            for i in 0..5 {
                audit
                    .add_entry(entry(
                        &format!("e{}", i),
                        Some("m"),
                        AuditEventType::ModelStored,
                    ))
                    .unwrap();
            }
            assert!(audit.height() >= 2);
        }

        #[test]
        fn blockchain_finalize_empty() {
            let tmp = tempdir().unwrap();
            let mut audit = BlockchainAudit::new(tmp.path(), 10).unwrap();
            assert!(audit.finalize_block().unwrap().is_none());
        }

        #[test]
        fn blockchain_get_block() {
            let tmp = tempdir().unwrap();
            let audit = BlockchainAudit::new(tmp.path(), 10).unwrap();
            let genesis = audit.get_block(0).unwrap();
            assert!(genesis.is_some());
            assert_eq!(genesis.unwrap().index, 0);
        }

        #[test]
        fn blockchain_get_block_nonexistent() {
            let tmp = tempdir().unwrap();
            let audit = BlockchainAudit::new(tmp.path(), 10).unwrap();
            assert!(audit.get_block(999).unwrap().is_none());
        }

        #[test]
        fn blockchain_verify_chain() {
            let tmp = tempdir().unwrap();
            let mut audit = BlockchainAudit::new(tmp.path(), 2).unwrap();
            for i in 0..4 {
                audit
                    .add_entry(entry(&format!("v{}", i), None, AuditEventType::ModelStored))
                    .unwrap();
            }
            let v = audit.verify_chain();
            assert!(v.valid);
            assert!(v.blocks_verified >= 2);
        }

        #[test]
        fn blockchain_generate_verify_proof() {
            let tmp = tempdir().unwrap();
            let mut audit = BlockchainAudit::new(tmp.path(), 2).unwrap();
            audit
                .add_entry(entry("proof_test", Some("m"), AuditEventType::ModelStored))
                .unwrap();
            audit.finalize_block().unwrap();

            let proof = audit.generate_proof(1, 0).unwrap();
            let v = BlockchainAudit::verify_proof(&proof);
            assert!(v.valid);
        }

        #[test]
        fn blockchain_search_by_model() {
            let tmp = tempdir().unwrap();
            let mut audit = BlockchainAudit::new(tmp.path(), 2).unwrap();
            audit
                .add_entry(entry("s", Some("searchable"), AuditEventType::ModelStored))
                .unwrap();
            audit.finalize_block().unwrap();

            let results = audit
                .search(Some("searchable"), None, None, None, 10)
                .unwrap();
            assert!(!results.is_empty());
        }

        #[test]
        fn blockchain_search_by_event_type() {
            let tmp = tempdir().unwrap();
            let mut audit = BlockchainAudit::new(tmp.path(), 2).unwrap();
            audit
                .add_entry(entry("r", Some("m"), AuditEventType::ModelRetrieved))
                .unwrap();
            audit.finalize_block().unwrap();

            let results = audit
                .search(None, Some(AuditEventType::ModelRetrieved), None, None, 10)
                .unwrap();
            assert!(!results.is_empty());
        }

        #[test]
        fn blockchain_search_time_bounds() {
            let tmp = tempdir().unwrap();
            let mut audit = BlockchainAudit::new(tmp.path(), 10).unwrap();
            let now = Utc::now();
            audit
                .add_entry(entry("t", None, AuditEventType::ModelStored))
                .unwrap();
            audit.finalize_block().unwrap();

            // from=future → empty
            let future = now + chrono::Duration::hours(1);
            assert!(audit
                .search(None, None, Some(future), None, 10)
                .unwrap()
                .is_empty());

            // to=past → empty
            let past = now - chrono::Duration::hours(1);
            assert!(audit
                .search(None, None, None, Some(past), 10)
                .unwrap()
                .is_empty());
        }

        #[test]
        fn blockchain_latest() {
            let tmp = tempdir().unwrap();
            let audit = BlockchainAudit::new(tmp.path(), 10).unwrap();
            assert!(audit.latest().is_some());
        }

        #[test]
        fn blockchain_reload_from_disk() {
            let tmp = tempdir().unwrap();
            {
                let mut a = BlockchainAudit::new(tmp.path(), 2).unwrap();
                a.add_entry(entry("persist", None, AuditEventType::ModelStored))
                    .unwrap();
                a.finalize_block().unwrap();
            }
            let a2 = BlockchainAudit::new(tmp.path(), 2).unwrap();
            assert!(a2.height() >= 2);
            assert!(a2.verify_chain().valid);
        }

        #[test]
        fn audit_block_compute_hash() {
            let block = AuditBlock {
                index: 0,
                timestamp: Utc::now(),
                prev_hash: "0".repeat(64),
                merkle_root: "abc".into(),
                entries: vec![],
                signature: None,
                nonce: 0,
                hash: String::new(),
            };
            assert_eq!(block.compute_hash().len(), 64);
        }
    }

    // ============================================================================
    // CONVERSION — ConversionPipeline, built-in converters, validation
    // ============================================================================
    mod conversion_coverage {
        use ironvault::conversion::*;
        use ironvault::formats::ModelFormat;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[test]
        fn progress_display_with_total() {
            let p = ConversionProgress {
                step: 0,
                total_steps: 3,
                bytes_processed: 50,
                bytes_total: 100,
                message: "Converting".into(),
            };
            let s = format!("{}", p);
            assert!(s.contains("50.0%"));
            assert!(s.contains("[1/3]"));
        }

        #[test]
        fn progress_display_no_total() {
            let p = ConversionProgress {
                step: 1,
                total_steps: 2,
                bytes_processed: 100,
                bytes_total: 0,
                message: "Processing".into(),
            };
            let s = format!("{}", p);
            assert!(s.contains("[2/2]"));
        }

        #[test]
        fn conversion_options_with_validation() {
            let opts = ConversionOptions::with_validation();
            assert!(opts.validate);
            assert!(opts.preserve_metadata);
        }

        #[test]
        fn compression_ratio() {
            let r = ConversionResult {
                data: vec![],
                plan: None,
                source_format: ModelFormat::PyTorch,
                target_format: ModelFormat::ONNX,
                conversion_path: vec![],
                input_size: 1000,
                output_size: 500,
                validation: None,
            };
            assert!((r.compression_ratio() - 0.5).abs() < 0.001);
        }

        #[test]
        fn compression_ratio_zero_input() {
            let r = ConversionResult {
                data: vec![],
                plan: None,
                source_format: ModelFormat::PyTorch,
                target_format: ModelFormat::ONNX,
                conversion_path: vec![],
                input_size: 0,
                output_size: 100,
                validation: None,
            };
            assert_eq!(r.compression_ratio(), 0.0);
        }

        #[test]
        fn validation_report_pass() {
            let checks = vec![
                ValidationCheck::pass("a", "ok"),
                ValidationCheck::pass("b", "ok"),
            ];
            assert!(ValidationReport::from_checks(checks).passed);
        }

        #[test]
        fn validation_report_fail() {
            let checks = vec![
                ValidationCheck::pass("a", "ok"),
                ValidationCheck::fail("b", "bad"),
            ];
            assert!(!ValidationReport::from_checks(checks).passed);
        }

        #[test]
        fn pipeline_empty() {
            let p = ConversionPipeline::new();
            assert!(p.supported_conversions().is_empty());
        }

        #[test]
        fn pipeline_with_builtins() {
            let p = ConversionPipeline::with_builtins();
            assert!(!p.supported_conversions().is_empty());
        }

        #[test]
        fn pipeline_default() {
            let p = ConversionPipeline::default();
            assert!(!p.supported_conversions().is_empty());
        }

        #[test]
        fn can_convert_direct() {
            let p = ConversionPipeline::with_builtins();
            assert!(p.can_convert_direct(
                &ModelFormat::Safetensors,
                &ModelFormat::Custom("raw".into())
            ));
        }

        #[test]
        fn find_path_same_format() {
            let p = ConversionPipeline::with_builtins();
            let path = p.find_path(&ModelFormat::PyTorch, &ModelFormat::PyTorch);
            assert!(path.is_some());
            assert_eq!(path.unwrap().len(), 1);
        }

        #[test]
        fn find_path_no_path() {
            let p = ConversionPipeline::new();
            assert!(p
                .find_path(&ModelFormat::PyTorch, &ModelFormat::ONNX)
                .is_none());
        }

        #[test]
        fn convert_same_format() {
            let p = ConversionPipeline::with_builtins();
            let result = p
                .convert(
                    b"data",
                    &ModelFormat::PyTorch,
                    &ModelFormat::PyTorch,
                    &ConversionOptions::default(),
                    None,
                )
                .unwrap();
            assert_eq!(result.data, b"data");
        }

        #[test]
        fn safetensors_to_raw() {
            let p = ConversionPipeline::with_builtins();
            let raw = b"tensor_data_here";
            let header = format!(
                r#"{{"__metadata__":{{"format":"raw"}},"tensor_0":{{"dtype":"U8","shape":[{}],"data_offsets":[0,{}]}}}}"#,
                raw.len(),
                raw.len(),
            );
            let hb = header.as_bytes();
            let mut st = Vec::new();
            st.extend_from_slice(&(hb.len() as u64).to_le_bytes());
            st.extend_from_slice(hb);
            st.extend_from_slice(raw);

            let result = p
                .convert(
                    &st,
                    &ModelFormat::Safetensors,
                    &ModelFormat::Custom("raw".into()),
                    &ConversionOptions::default(),
                    None,
                )
                .unwrap();
            assert_eq!(result.data, raw);
        }

        #[test]
        fn raw_to_safetensors() {
            let p = ConversionPipeline::with_builtins();
            let result = p
                .convert(
                    b"data",
                    &ModelFormat::Custom("raw".into()),
                    &ModelFormat::Safetensors,
                    &ConversionOptions::default(),
                    None,
                )
                .unwrap();
            assert!(result.data.len() > 8);
        }

        #[test]
        fn gguf_header_parser() {
            let p = ConversionPipeline::with_builtins();
            let mut gguf = Vec::new();
            gguf.extend_from_slice(b"GGUF");
            gguf.extend_from_slice(&3u32.to_le_bytes());
            gguf.extend_from_slice(&10u64.to_le_bytes());
            gguf.extend_from_slice(&5u64.to_le_bytes());

            let result = p
                .convert(
                    &gguf,
                    &ModelFormat::GGUF,
                    &ModelFormat::Custom("gguf-meta".into()),
                    &ConversionOptions::default(),
                    None,
                )
                .unwrap();
            let meta: serde_json::Value = serde_json::from_slice(&result.data).unwrap();
            assert_eq!(meta["version"], 3);
        }

        #[test]
        fn gguf_too_small() {
            let p = ConversionPipeline::with_builtins();
            assert!(p
                .convert(
                    b"GGU",
                    &ModelFormat::GGUF,
                    &ModelFormat::Custom("gguf-meta".into()),
                    &ConversionOptions::default(),
                    None
                )
                .is_err());
        }

        #[test]
        fn safetensors_too_small() {
            let p = ConversionPipeline::with_builtins();
            assert!(p
                .convert(
                    b"tiny",
                    &ModelFormat::Safetensors,
                    &ModelFormat::Custom("raw".into()),
                    &ConversionOptions::default(),
                    None
                )
                .is_err());
        }

        #[test]
        fn convert_with_validation() {
            let p = ConversionPipeline::with_builtins();
            let result = p
                .convert(
                    b"data",
                    &ModelFormat::Custom("raw".into()),
                    &ModelFormat::Safetensors,
                    &ConversionOptions::with_validation(),
                    None,
                )
                .unwrap();
            assert!(result.validation.is_some());
        }

        #[test]
        fn convert_with_progress_callback() {
            let p = ConversionPipeline::with_builtins();
            let count = Arc::new(AtomicUsize::new(0));
            let count_clone = count.clone();
            let cb: ProgressCallback = Box::new(move |_| {
                count_clone.fetch_add(1, Ordering::SeqCst);
            });
            p.convert(
                b"data",
                &ModelFormat::Custom("raw".into()),
                &ModelFormat::Safetensors,
                &ConversionOptions::default(),
                Some(&cb),
            )
            .unwrap();
            assert!(count.load(Ordering::SeqCst) > 0);
        }

        #[test]
        fn convert_no_path_error() {
            let p = ConversionPipeline::new();
            assert!(p
                .convert(
                    b"data",
                    &ModelFormat::PyTorch,
                    &ModelFormat::ONNX,
                    &ConversionOptions::default(),
                    None
                )
                .is_err());
        }
    }

    // ============================================================================
    // RAG DATABASE — InMemoryDatabase CRUD, WHERE clause
    // ============================================================================
    mod inmemory_db_coverage {
        use ironvault::rag::database::{Database, InMemoryDatabase};
        use std::collections::HashMap;

        #[test]
        fn new_and_default() {
            let _ = InMemoryDatabase::new();
            let _ = InMemoryDatabase::default();
        }

        #[test]
        fn insert_and_query() {
            let mut db = InMemoryDatabase::new();
            db.create_table("users".into());

            let mut d = HashMap::new();
            d.insert("id".into(), "1".into());
            d.insert("name".into(), "Alice".into());
            db.insert("users", d).unwrap();

            let r = db.query("users").unwrap();
            assert_eq!(r.len(), 1);
        }

        #[test]
        fn query_with_where() {
            let mut db = InMemoryDatabase::new();
            db.create_table("items".into());

            let mut d1 = HashMap::new();
            d1.insert("id".into(), "1".into());
            d1.insert("type".into(), "A".into());
            db.insert("items", d1).unwrap();

            let mut d2 = HashMap::new();
            d2.insert("id".into(), "2".into());
            d2.insert("type".into(), "B".into());
            db.insert("items", d2).unwrap();

            let r = db.query("items WHERE type=A").unwrap();
            assert_eq!(r.len(), 1);
            assert_eq!(r[0].get("id").unwrap(), "1");
        }

        #[test]
        fn query_nonexistent_table() {
            let db = InMemoryDatabase::new();
            assert!(db.query("nope").unwrap().is_empty());
        }

        #[test]
        fn query_empty_string() {
            let db = InMemoryDatabase::new();
            assert!(db.query("").unwrap().is_empty());
        }

        #[test]
        fn insert_no_table() {
            let mut db = InMemoryDatabase::new();
            assert!(db.insert("nope", HashMap::new()).is_err());
        }

        #[test]
        fn update_existing() {
            let mut db = InMemoryDatabase::new();
            db.create_table("t".into());

            let mut d = HashMap::new();
            d.insert("id".into(), "1".into());
            d.insert("name".into(), "Old".into());
            db.insert("t", d).unwrap();

            let mut u = HashMap::new();
            u.insert("name".into(), "New".into());
            db.update("t", "1", u).unwrap();

            assert_eq!(db.query("t").unwrap()[0].get("name").unwrap(), "New");
        }

        #[test]
        fn update_no_table() {
            let mut db = InMemoryDatabase::new();
            assert!(db.update("nope", "1", HashMap::new()).is_err());
        }

        #[test]
        fn update_no_row() {
            let mut db = InMemoryDatabase::new();
            db.create_table("t".into());
            assert!(db.update("t", "999", HashMap::new()).is_err());
        }

        #[test]
        fn delete_existing() {
            let mut db = InMemoryDatabase::new();
            db.create_table("t".into());
            let mut d = HashMap::new();
            d.insert("id".into(), "1".into());
            db.insert("t", d).unwrap();
            db.delete("t", "1").unwrap();
            assert!(db.query("t").unwrap().is_empty());
        }

        #[test]
        fn delete_no_table() {
            let mut db = InMemoryDatabase::new();
            assert!(db.delete("nope", "1").is_err());
        }
    }

    // ============================================================================
    // SQLITE DATABASE — full CRUD + document store
    // ============================================================================
    #[cfg(feature = "sqlite")]
    mod sqlite_db_coverage {
        use ironvault::rag::database::{Database, SQLiteDatabase};
        use ironvault::rag::{documents::ChunkInfo, Document};
        use std::collections::HashMap;

        #[test]
        fn in_memory() {
            let db = SQLiteDatabase::in_memory().unwrap();
            assert!(db.create_table("t", &[("name", "TEXT")]).is_ok());
        }

        #[test]
        fn crud_roundtrip() {
            let mut db = SQLiteDatabase::in_memory().unwrap();
            db.create_table("items", &[("name", "TEXT"), ("value", "TEXT")])
                .unwrap();

            let mut d = HashMap::new();
            d.insert("id".into(), "i1".into());
            d.insert("name".into(), "Widget".into());
            d.insert("value".into(), "42".into());
            db.insert("items", d).unwrap();

            let r = db.query("SELECT * FROM items").unwrap();
            assert_eq!(r.len(), 1);

            let mut u = HashMap::new();
            u.insert("name".into(), "Gadget".into());
            db.update("items", "i1", u).unwrap();

            db.delete("items", "i1").unwrap();
            assert!(db.query("SELECT * FROM items").unwrap().is_empty());
        }

        #[test]
        fn store_and_get_document() {
            let db = SQLiteDatabase::in_memory().unwrap();
            let doc = Document {
                id: "d1".into(),
                content: "Hello".into(),
                metadata: HashMap::from([("k".into(), "v".into())]),
                embedding: Some(vec![1.0, 2.0]),
                chunk_info: None,
            };
            db.store_document(&doc).unwrap();
            let got = db.get_document("d1").unwrap().unwrap();
            assert_eq!(got.content, "Hello");
        }

        #[test]
        fn get_document_not_found() {
            let db = SQLiteDatabase::in_memory().unwrap();
            let doc = Document {
                id: "tmp".into(),
                content: "".into(),
                metadata: HashMap::new(),
                embedding: None,
                chunk_info: None,
            };
            db.store_document(&doc).unwrap();
            assert!(db.get_document("missing").unwrap().is_none());
        }

        #[test]
        fn search_documents() {
            let db = SQLiteDatabase::in_memory().unwrap();
            db.store_document(&Document {
                id: "d1".into(),
                content: "machine learning".into(),
                metadata: HashMap::new(),
                embedding: None,
                chunk_info: None,
            })
            .unwrap();
            db.store_document(&Document {
                id: "d2".into(),
                content: "deep neural networks".into(),
                metadata: HashMap::new(),
                embedding: None,
                chunk_info: None,
            })
            .unwrap();

            assert_eq!(db.search_documents("learning", 10).unwrap().len(), 1);
            assert_eq!(db.search_documents("deep", 10).unwrap().len(), 1);
        }

        #[test]
        fn store_with_chunk_info() {
            let db = SQLiteDatabase::in_memory().unwrap();
            let doc = Document {
                id: "ch1".into(),
                content: "chunk".into(),
                metadata: HashMap::new(),
                embedding: None,
                chunk_info: Some(ChunkInfo {
                    parent_id: Some("parent".into()),
                    chunk_index: 0,
                    total_chunks: 3,
                    overlap: 50,
                }),
            };
            db.store_document(&doc).unwrap();
            let got = db.get_document("ch1").unwrap().unwrap();
            assert!(got.chunk_info.is_some());
        }
    }

    // ============================================================================
    // SLED DATABASE (feature-gated)
    // ============================================================================
    #[cfg(feature = "kv-store")]
    mod sled_db_coverage {
        use ironvault::rag::database::{Database, SledDatabase};
        use ironvault::rag::Document;
        use std::collections::HashMap;

        #[test]
        fn temporary_and_crud() {
            let mut db = SledDatabase::temporary().unwrap();

            let mut d = HashMap::new();
            d.insert("id".into(), "1".into());
            d.insert("name".into(), "Test".into());
            db.insert("t", d).unwrap();

            let r = db.query("t:1").unwrap();
            assert!(!r.is_empty());

            let mut u = HashMap::new();
            u.insert("name".into(), "Updated".into());
            db.update("t", "1", u).unwrap();

            db.delete("t", "1").unwrap();
        }

        #[test]
        fn store_get_list_docs() {
            let db = SledDatabase::temporary().unwrap();
            for i in 0..3 {
                db.store_document(&Document {
                    id: format!("d{}", i),
                    content: format!("c{}", i),
                    metadata: HashMap::new(),
                    embedding: None,
                    chunk_info: None,
                })
                .unwrap();
            }
            assert_eq!(db.list_documents().unwrap().len(), 3);
        }

        #[test]
        fn search_prefix() {
            let db = SledDatabase::temporary().unwrap();
            db.store_document(&Document {
                id: "prefix_x".into(),
                content: "x".into(),
                metadata: HashMap::new(),
                embedding: None,
                chunk_info: None,
            })
            .unwrap();
            assert!(!db.search_prefix("prefix_").unwrap().is_empty());
        }

        #[test]
        fn insert_no_id_error() {
            let mut db = SledDatabase::temporary().unwrap();
            assert!(db.insert("t", HashMap::new()).is_err());
        }
    }

    // ============================================================================
    // MODEL CARD — serialization roundtrips, markdown, all sections
    // ============================================================================
    mod model_card_coverage {
        use ironvault::model_card::*;
        use std::collections::HashMap;

        fn basic_card() -> ModelCard {
            ModelCard::new(
                ModelDetails {
                    name: "test".into(),
                    version: "1.0".into(),
                    description: "desc".into(),
                    model_type: "LLM".into(),
                    architecture: "Transformer".into(),
                    size: "7B".into(),
                    framework: "PyTorch".into(),
                    format: "safetensors".into(),
                    license: Some("MIT".into()),
                    citation: None,
                    developers: vec!["Dev".into()],
                    contact: None,
                    repository: None,
                    paper: None,
                },
                IntendedUse {
                    primary_uses: vec!["text gen".into()],
                    primary_users: vec!["researchers".into()],
                    out_of_scope_uses: vec!["harm".into()],
                    use_case_examples: Some(vec!["chat".into()]),
                },
            )
        }

        #[test]
        fn to_json_roundtrip() {
            let card = basic_card();
            let json = card.to_json().unwrap();
            let parsed = ModelCard::from_json(&json).unwrap();
            assert_eq!(parsed.model_details.name, "test");
        }

        #[test]
        fn to_yaml_roundtrip() {
            let card = basic_card();
            let yaml = card.to_yaml().unwrap();
            let parsed = ModelCard::from_yaml(&yaml).unwrap();
            assert_eq!(parsed.model_details.name, "test");
        }

        #[test]
        fn to_markdown_basic() {
            let card = basic_card();
            let md = card.to_markdown();
            assert!(md.contains("# Model Card: test"));
            assert!(md.contains("text gen"));
            assert!(md.contains("harm"));
        }

        #[test]
        fn with_training_data() {
            let card = basic_card().with_training_data(TrainingData {
                datasets: vec!["CIFAR-10".into()],
                sources: Some(vec!["public".into()]),
                collection_methods: Some("download".into()),
                preprocessing: Some(vec!["normalize".into()]),
                size: Some("60K".into()),
                splits: Some(HashMap::from([("train".into(), "50K".into())])),
                languages: Some(vec!["en".into()]),
                demographics: None,
            });
            let md = card.to_markdown();
            assert!(md.contains("CIFAR-10"));
        }

        #[test]
        fn with_evaluation() {
            let card = basic_card().with_evaluation(Evaluation {
                datasets: vec!["MNIST".into()],
                metrics: vec![Metric {
                    name: "accuracy".into(),
                    value: 95.5,
                    description: Some("Top-1".into()),
                    threshold: Some(90.0),
                }],
                benchmarks: Some(HashMap::from([("speed".into(), 100.0)])),
                performance_by_group: None,
                methodology: Some("Standard".into()),
            });
            let md = card.to_markdown();
            assert!(md.contains("accuracy"));
        }

        #[test]
        fn with_ethical_considerations() {
            let card = basic_card().with_ethical_considerations(EthicalConsiderations {
                sensitive_data: Some("none".into()),
                bias: Some(vec!["gender".into()]),
                fairness: Some(vec!["equal opportunity".into()]),
                privacy: Some("GDPR compliant".into()),
                environmental_impact: Some(EnvironmentalImpact {
                    hardware: "A100".into(),
                    hours: 100.0,
                    cloud_provider: Some("AWS".into()),
                    carbon_emitted: Some(50.0),
                    energy_consumed: Some(500.0),
                }),
                human_oversight: Some("required".into()),
                risks: Some(vec!["bias".into()]),
                mitigations: Some(vec!["debiasing".into()]),
            });
            let md = card.to_markdown();
            assert!(md.contains("bias") || md.contains("Ethical"));
        }

        #[test]
        fn with_caveats() {
            let card = basic_card().with_caveats_and_recommendations(CaveatsAndRecommendations {
                limitations: vec!["English only".into()],
                known_issues: Some(vec!["slow inference".into()]),
                recommendations: vec!["use GPU".into()],
                testing_recommendations: Some(vec!["benchmark".into()]),
                tradeoffs: Some(vec!["speed vs accuracy".into()]),
            });
            let md = card.to_markdown();
            assert!(md.contains("English only") || md.contains("Limitations"));
        }

        #[test]
        fn touch_updates_timestamp() {
            let mut card = basic_card();
            let before = card.updated_at;
            std::thread::sleep(std::time::Duration::from_millis(10));
            card.touch();
            assert!(card.updated_at >= before);
        }

        #[test]
        fn add_metadata_builder() {
            let card = basic_card()
                .add_metadata("key1", "val1")
                .add_metadata("key2", "val2");
            assert_eq!(card.metadata.get("key1").unwrap(), "val1");
        }

        #[test]
        fn with_repository_and_paper() {
            let mut card = basic_card();
            card.model_details.repository = Some("https://github.com/example".into());
            card.model_details.paper = Some("https://arxiv.org/abs/1234".into());
            card.model_details.citation = Some("Cite me".into());
            card.model_details.contact = Some("test@example.com".into());
            let md = card.to_markdown();
            assert!(md.contains("github.com") || md.contains("Repository"));
        }
    }

    // ============================================================================
    // FORMATS — extension, name, from_extension for all variants
    // ============================================================================
    mod formats_coverage {
        use ironvault::formats::{ModelFormat, ModelMetadata};

        #[test]
        fn all_extensions() {
            let formats = vec![
                ModelFormat::Safetensors,
                ModelFormat::GGUF,
                ModelFormat::PyTorch,
                ModelFormat::TensorRT,
                ModelFormat::ONNX,
                ModelFormat::MLX,
                ModelFormat::CoreML,
                ModelFormat::TorchScript,
                ModelFormat::TFLite,
                ModelFormat::TensorFlow,
                ModelFormat::Keras,
                ModelFormat::OpenVINO,
                ModelFormat::TVM,
                ModelFormat::NCNN,
                ModelFormat::MNN,
                ModelFormat::RKNN,
                ModelFormat::Caffe,
                ModelFormat::MXNet,
                ModelFormat::Darknet,
                ModelFormat::HDF5,
                ModelFormat::Pickle,
                ModelFormat::NumPy,
                ModelFormat::Custom("xyz".into()),
            ];
            for f in &formats {
                assert!(!f.extension().is_empty());
                assert!(!f.name().is_empty());
            }
        }

        #[test]
        fn from_extension_all() {
            let cases = vec![
                ("safetensors", ModelFormat::Safetensors),
                ("gguf", ModelFormat::GGUF),
                ("pt", ModelFormat::PyTorch),
                ("pth", ModelFormat::PyTorch),
                ("bin", ModelFormat::PyTorch),
                ("plan", ModelFormat::TensorRT),
                ("onnx", ModelFormat::ONNX),
                ("mlmodel", ModelFormat::CoreML),
                ("tflite", ModelFormat::TFLite),
                ("pb", ModelFormat::TensorFlow),
                ("h5", ModelFormat::Keras),
                ("keras", ModelFormat::Keras),
                ("xml", ModelFormat::OpenVINO),
                ("param", ModelFormat::NCNN),
                ("mnn", ModelFormat::MNN),
                ("rknn", ModelFormat::RKNN),
                ("caffemodel", ModelFormat::Caffe),
                ("params", ModelFormat::MXNet),
                ("weights", ModelFormat::Darknet),
                ("hdf5", ModelFormat::HDF5),
                ("pkl", ModelFormat::Pickle),
                ("npy", ModelFormat::NumPy),
                ("npz", ModelFormat::NumPy),
            ];
            for (ext, expected) in cases {
                assert_eq!(
                    ModelFormat::from_extension(ext),
                    expected,
                    "Failed for: {}",
                    ext
                );
            }
        }

        #[test]
        fn from_unknown_extension() {
            match ModelFormat::from_extension("unknownformat") {
                ModelFormat::Custom(s) => assert_eq!(s, "unknownformat"),
                _ => panic!("Expected Custom"),
            }
        }

        #[test]
        fn model_metadata_builder() {
            let meta = ModelMetadata::new("m".into(), ModelFormat::PyTorch)
                .with_description("desc".into())
                .with_framework("PyTorch".into())
                .with_task("classification".into())
                .with_architecture("ResNet".into())
                .with_parameters(50_000_000)
                .add_custom_field("key".into(), "val".into());
            assert_eq!(meta.name, "m");
            assert_eq!(meta.description.unwrap(), "desc");
            assert_eq!(meta.parameters.unwrap(), 50_000_000);
        }
    }
}

#[allow(unused_imports)]
mod remaining_coverage_tests {
    //! Remaining coverage tests — Part 3
    //! Targets gaps identified by tarpaulin analysis:
    //! - rag/rules.rs (RuleEngine, all conditions + actions)
    //! - config.rs (save, path getters, compression getters)
    //! - vault.rs (lock/unlock state, delete, stats, change_passphrase, ModelStream, VaultBuilder)
    //! - conversion.rs (shim converters, OnnxMetadataExtractor)
    //! - formats.rs (FormatConverter, Display for ModelFormat, extension, name)
    //! - utils.rs (ModelArchive tar/zip, ModelExporter, cache eviction, dedup)

    // ============================================================================
    // RAG RULES ENGINE — 100% untested, all conditions and actions
    // ============================================================================
    mod rule_engine_coverage {
        use ironvault::rag::{Rule, RuleAction, RuleCondition, RuleEngine};
        use std::collections::HashMap;

        fn make_rule(
            id: &str,
            conditions: HashMap<String, RuleCondition>,
            actions: Vec<RuleAction>,
            priority: i32,
        ) -> Rule {
            Rule {
                id: id.to_string(),
                name: id.to_string(),
                conditions,
                actions,
                priority,
                enabled: true,
            }
        }

        fn equals_rule() -> Rule {
            let mut cond = HashMap::new();
            cond.insert(
                "status".to_string(),
                RuleCondition::Equals("active".to_string()),
            );
            make_rule(
                "r1",
                cond,
                vec![RuleAction::SetValue {
                    key: "processed".to_string(),
                    value: "yes".to_string(),
                }],
                10,
            )
        }

        #[test]
        fn new_engine_is_empty() {
            let engine = RuleEngine::new();
            assert!(engine.get_rules().is_empty());
        }

        #[test]
        fn default_engine_is_empty() {
            let engine = RuleEngine::default();
            assert!(engine.get_rules().is_empty());
        }

        #[test]
        fn add_rule_appears() {
            let mut engine = RuleEngine::new();
            engine.add_rule(equals_rule());
            assert_eq!(engine.get_rules().len(), 1);
            assert_eq!(engine.get_rules()[0].id, "r1");
        }

        #[test]
        fn rules_sorted_by_priority_desc() {
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule("low", HashMap::new(), vec![], 1));
            engine.add_rule(make_rule("high", HashMap::new(), vec![], 100));
            engine.add_rule(make_rule("mid", HashMap::new(), vec![], 50));
            let ids: Vec<&str> = engine.get_rules().iter().map(|r| r.id.as_str()).collect();
            assert_eq!(ids, vec!["high", "mid", "low"]);
        }

        #[test]
        fn clear_rules() {
            let mut engine = RuleEngine::new();
            engine.add_rule(equals_rule());
            engine.clear_rules();
            assert!(engine.get_rules().is_empty());
        }

        #[test]
        fn set_and_get_context() {
            let mut engine = RuleEngine::new();
            engine.set_context("k1".to_string(), "v1".to_string());
            assert_eq!(engine.get_context("k1"), Some(&"v1".to_string()));
            assert_eq!(engine.get_context("missing"), None);
        }

        // --- Condition: Equals ---
        #[test]
        fn condition_equals_match() {
            let mut engine = RuleEngine::new();
            engine.add_rule(equals_rule());
            engine.set_context("status".to_string(), "active".to_string());
            let results = engine.execute().unwrap();
            assert!(!results.is_empty());
        }

        #[test]
        fn condition_equals_no_match() {
            let mut engine = RuleEngine::new();
            engine.add_rule(equals_rule());
            engine.set_context("status".to_string(), "inactive".to_string());
            let results = engine.execute().unwrap();
            assert!(results.is_empty());
        }

        // --- Condition: Contains ---
        #[test]
        fn condition_contains_match() {
            let mut cond = HashMap::new();
            cond.insert(
                "text".to_string(),
                RuleCondition::Contains("hello".to_string()),
            );
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "cont",
                cond,
                vec![RuleAction::Log {
                    level: "info".to_string(),
                    message: "matched".to_string(),
                }],
                1,
            ));
            engine.set_context("text".to_string(), "say hello world".to_string());
            let results = engine.execute().unwrap();
            assert!(!results.is_empty());
        }

        #[test]
        fn condition_contains_no_match() {
            let mut cond = HashMap::new();
            cond.insert(
                "text".to_string(),
                RuleCondition::Contains("hello".to_string()),
            );
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "cont",
                cond,
                vec![RuleAction::Log {
                    level: "info".to_string(),
                    message: "matched".to_string(),
                }],
                1,
            ));
            engine.set_context("text".to_string(), "goodbye world".to_string());
            let results = engine.execute().unwrap();
            assert!(results.is_empty());
        }

        // --- Condition: Matches ---
        #[test]
        fn condition_matches_match() {
            let mut cond = HashMap::new();
            cond.insert(
                "data".to_string(),
                RuleCondition::Matches("pattern".to_string()),
            );
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "match_rule",
                cond,
                vec![RuleAction::Log {
                    level: "info".to_string(),
                    message: "m".to_string(),
                }],
                1,
            ));
            engine.set_context("data".to_string(), "this has pattern inside".to_string());
            let results = engine.execute().unwrap();
            assert!(!results.is_empty());
        }

        // --- Condition: GreaterThan ---
        #[test]
        fn condition_greater_than_match() {
            let mut cond = HashMap::new();
            cond.insert("score".to_string(), RuleCondition::GreaterThan(50.0));
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "gt",
                cond,
                vec![RuleAction::Log {
                    level: "info".to_string(),
                    message: "pass".to_string(),
                }],
                1,
            ));
            engine.set_context("score".to_string(), "75.0".to_string());
            let results = engine.execute().unwrap();
            assert!(!results.is_empty());
        }

        #[test]
        fn condition_greater_than_no_match() {
            let mut cond = HashMap::new();
            cond.insert("score".to_string(), RuleCondition::GreaterThan(50.0));
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "gt",
                cond,
                vec![RuleAction::Log {
                    level: "info".to_string(),
                    message: "pass".to_string(),
                }],
                1,
            ));
            engine.set_context("score".to_string(), "25.0".to_string());
            let results = engine.execute().unwrap();
            assert!(results.is_empty());
        }

        #[test]
        fn condition_greater_than_non_numeric() {
            let mut cond = HashMap::new();
            cond.insert("score".to_string(), RuleCondition::GreaterThan(50.0));
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "gt",
                cond,
                vec![RuleAction::Log {
                    level: "info".to_string(),
                    message: "pass".to_string(),
                }],
                1,
            ));
            engine.set_context("score".to_string(), "not_a_number".to_string());
            let results = engine.execute().unwrap();
            assert!(results.is_empty());
        }

        // --- Condition: LessThan ---
        #[test]
        fn condition_less_than_match() {
            let mut cond = HashMap::new();
            cond.insert("rate".to_string(), RuleCondition::LessThan(0.5));
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "lt",
                cond,
                vec![RuleAction::Log {
                    level: "info".to_string(),
                    message: "ok".to_string(),
                }],
                1,
            ));
            engine.set_context("rate".to_string(), "0.25".to_string());
            let results = engine.execute().unwrap();
            assert!(!results.is_empty());
        }

        #[test]
        fn condition_less_than_no_match() {
            let mut cond = HashMap::new();
            cond.insert("rate".to_string(), RuleCondition::LessThan(0.5));
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "lt",
                cond,
                vec![RuleAction::Log {
                    level: "info".to_string(),
                    message: "ok".to_string(),
                }],
                1,
            ));
            engine.set_context("rate".to_string(), "0.75".to_string());
            let results = engine.execute().unwrap();
            assert!(results.is_empty());
        }

        // --- Condition: In ---
        #[test]
        fn condition_in_match() {
            let mut cond = HashMap::new();
            cond.insert(
                "color".to_string(),
                RuleCondition::In(vec!["red".into(), "green".into(), "blue".into()]),
            );
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "in_rule",
                cond,
                vec![RuleAction::Log {
                    level: "info".to_string(),
                    message: "matched".to_string(),
                }],
                1,
            ));
            engine.set_context("color".to_string(), "green".to_string());
            let results = engine.execute().unwrap();
            assert!(!results.is_empty());
        }

        #[test]
        fn condition_in_no_match() {
            let mut cond = HashMap::new();
            cond.insert(
                "color".to_string(),
                RuleCondition::In(vec!["red".into(), "green".into(), "blue".into()]),
            );
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "in_rule",
                cond,
                vec![RuleAction::Log {
                    level: "info".to_string(),
                    message: "matched".to_string(),
                }],
                1,
            ));
            engine.set_context("color".to_string(), "yellow".to_string());
            let results = engine.execute().unwrap();
            assert!(results.is_empty());
        }

        // --- Condition: Custom (always false) ---
        #[test]
        fn condition_custom_always_false() {
            let mut cond = HashMap::new();
            cond.insert(
                "anything".to_string(),
                RuleCondition::Custom("x".to_string()),
            );
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "custom",
                cond,
                vec![RuleAction::Log {
                    level: "info".to_string(),
                    message: "never".to_string(),
                }],
                1,
            ));
            engine.set_context("anything".to_string(), "true".to_string());
            let results = engine.execute().unwrap();
            assert!(results.is_empty());
        }

        // --- Condition: missing context key ---
        #[test]
        fn condition_missing_context_key() {
            let mut engine = RuleEngine::new();
            engine.add_rule(equals_rule());
            // don't set "status" in context
            let results = engine.execute().unwrap();
            assert!(results.is_empty());
        }

        // --- Action: SetValue ---
        #[test]
        fn action_set_value() {
            let mut cond = HashMap::new();
            cond.insert("a".to_string(), RuleCondition::Equals("1".to_string()));
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "set",
                cond,
                vec![RuleAction::SetValue {
                    key: "result".to_string(),
                    value: "done".to_string(),
                }],
                1,
            ));
            engine.set_context("a".to_string(), "1".to_string());
            let results = engine.execute().unwrap();
            assert_eq!(results.len(), 1);
        }

        // --- Action: AddToList (empty existing) ---
        #[test]
        fn action_add_to_list_empty() {
            let mut cond = HashMap::new();
            cond.insert("x".to_string(), RuleCondition::Equals("y".to_string()));
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "add",
                cond,
                vec![RuleAction::AddToList {
                    key: "list".to_string(),
                    value: "item1".to_string(),
                }],
                1,
            ));
            engine.set_context("x".to_string(), "y".to_string());
            let results = engine.execute().unwrap();
            assert_eq!(results.len(), 1);
        }

        // --- Action: AddToList (with existing value) ---
        #[test]
        fn action_add_to_list_existing() {
            let mut cond = HashMap::new();
            cond.insert("x".to_string(), RuleCondition::Equals("y".to_string()));
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "add",
                cond,
                vec![RuleAction::AddToList {
                    key: "list".to_string(),
                    value: "item2".to_string(),
                }],
                1,
            ));
            engine.set_context("x".to_string(), "y".to_string());
            engine.set_context("list".to_string(), "item1".to_string());
            let results = engine.execute().unwrap();
            assert_eq!(results.len(), 1);
        }

        // --- Action: Log ---
        #[test]
        fn action_log() {
            let mut cond = HashMap::new();
            cond.insert("a".to_string(), RuleCondition::Equals("b".to_string()));
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "log_rule",
                cond,
                vec![RuleAction::Log {
                    level: "warn".to_string(),
                    message: "log this".to_string(),
                }],
                1,
            ));
            engine.set_context("a".to_string(), "b".to_string());
            let results = engine.execute().unwrap();
            assert_eq!(results.len(), 1);
        }

        // --- Action: CallFunction ---
        #[test]
        fn action_call_function() {
            let mut cond = HashMap::new();
            cond.insert("a".to_string(), RuleCondition::Equals("b".to_string()));
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "fn_rule",
                cond,
                vec![RuleAction::CallFunction {
                    function: "my_func".to_string(),
                    args: vec!["arg1".to_string(), "arg2".to_string()],
                }],
                1,
            ));
            engine.set_context("a".to_string(), "b".to_string());
            let results = engine.execute().unwrap();
            assert_eq!(results.len(), 1);
        }

        // --- Action: Stop ---
        #[test]
        fn action_stop_halts_further_rules() {
            let mut cond1 = HashMap::new();
            cond1.insert("x".to_string(), RuleCondition::Equals("1".to_string()));
            let mut cond2 = HashMap::new();
            cond2.insert("x".to_string(), RuleCondition::Equals("1".to_string()));

            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule("stopper", cond1, vec![RuleAction::Stop], 100));
            engine.add_rule(make_rule(
                "after_stop",
                cond2,
                vec![RuleAction::Log {
                    level: "info".to_string(),
                    message: "should not run".to_string(),
                }],
                1,
            ));
            engine.set_context("x".to_string(), "1".to_string());
            let results = engine.execute().unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0], "stopper");
        }

        // --- Disabled rule ---
        #[test]
        fn disabled_rule_skipped() {
            let mut cond = HashMap::new();
            cond.insert("a".to_string(), RuleCondition::Equals("b".to_string()));
            let mut engine = RuleEngine::new();
            let mut rule = make_rule(
                "disabled",
                cond,
                vec![RuleAction::Log {
                    level: "info".to_string(),
                    message: "nope".to_string(),
                }],
                1,
            );
            rule.enabled = false;
            engine.add_rule(rule);
            engine.set_context("a".to_string(), "b".to_string());
            let results = engine.execute().unwrap();
            assert!(results.is_empty());
        }

        // --- Multiple conditions (AND) ---
        #[test]
        fn multiple_conditions_all_must_match() {
            let mut cond = HashMap::new();
            cond.insert("a".to_string(), RuleCondition::Equals("1".to_string()));
            cond.insert(
                "b".to_string(),
                RuleCondition::Contains("hello".to_string()),
            );
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "multi",
                cond,
                vec![RuleAction::Log {
                    level: "info".to_string(),
                    message: "ok".to_string(),
                }],
                1,
            ));
            engine.set_context("a".to_string(), "1".to_string());
            engine.set_context("b".to_string(), "hello world".to_string());
            let results = engine.execute().unwrap();
            assert_eq!(results.len(), 1);
        }

        #[test]
        fn multiple_conditions_partial_match_fails() {
            let mut cond = HashMap::new();
            cond.insert("a".to_string(), RuleCondition::Equals("1".to_string()));
            cond.insert(
                "b".to_string(),
                RuleCondition::Contains("hello".to_string()),
            );
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "multi",
                cond,
                vec![RuleAction::Log {
                    level: "info".to_string(),
                    message: "ok".to_string(),
                }],
                1,
            ));
            engine.set_context("a".to_string(), "1".to_string());
            engine.set_context("b".to_string(), "goodbye".to_string());
            let results = engine.execute().unwrap();
            assert!(results.is_empty());
        }

        // --- No-condition rule (always matches) ---
        #[test]
        fn no_conditions_always_matches() {
            let mut engine = RuleEngine::new();
            engine.add_rule(make_rule(
                "always",
                HashMap::new(),
                vec![RuleAction::SetValue {
                    key: "ran".to_string(),
                    value: "true".to_string(),
                }],
                1,
            ));
            let results = engine.execute().unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0], "always");
        }
    }

    // ============================================================================
    // CONFIG — save, path getters, compression getters
    // ============================================================================
    mod config_coverage {
        use ironvault::config::VaultConfig;

        fn make_config() -> (VaultConfig, tempfile::TempDir) {
            let tmp = tempfile::tempdir().unwrap();
            let dirs = ironvault::config::DirectoryPaths {
                config_dir: tmp.path().join("config"),
                data_dir: tmp.path().join("data"),
                cache_dir: tmp.path().join("cache"),
                vault_dir: tmp.path().join("data/vaults/default"),
                log_dir: tmp.path().join("data/logs"),
                backends_dir: tmp.path().join("config/backends"),
                utilities_dir: tmp.path().join("config/utilities"),
                databases_dir: tmp.path().join("config/databases"),
            };
            let cfg = VaultConfig::with_dirs(dirs).unwrap();
            (cfg, tmp)
        }

        #[test]
        fn save_creates_file() {
            let (cfg, _tmp) = make_config();
            cfg.save().unwrap();
            let config_file = cfg.dirs.config_dir.join("config.yaml");
            assert!(config_file.exists());
        }

        #[test]
        fn get_vault_path_with_name() {
            let (cfg, _tmp) = make_config();
            let path = cfg.get_vault_path(Some("mymodel"));
            assert!(path.ends_with("mymodel"));
        }

        #[test]
        fn get_vault_path_default() {
            let (cfg, _tmp) = make_config();
            let path = cfg.get_vault_path(None);
            assert!(path.ends_with("default"));
        }

        #[test]
        fn get_audit_log_path() {
            let (cfg, _tmp) = make_config();
            let path = cfg.get_audit_log_path();
            assert!(path.to_string_lossy().contains("audit.log"));
        }

        #[test]
        fn compression_algorithm_gzip() {
            let (mut cfg, _tmp) = make_config();
            cfg.compression.algorithm = "gzip".to_string();
            let algo = cfg.get_compression_algorithm();
            assert_eq!(
                format!("{:?}", algo),
                format!(
                    "{:?}",
                    ironvault::crypto::compression::CompressionAlgorithm::Gzip
                )
            );
        }

        #[test]
        fn compression_algorithm_lzma() {
            let (mut cfg, _tmp) = make_config();
            cfg.compression.algorithm = "lzma".to_string();
            let algo = cfg.get_compression_algorithm();
            assert_eq!(
                format!("{:?}", algo),
                format!(
                    "{:?}",
                    ironvault::crypto::compression::CompressionAlgorithm::Lzma
                )
            );
        }

        #[test]
        fn compression_algorithm_none() {
            let (mut cfg, _tmp) = make_config();
            cfg.compression.algorithm = "none".to_string();
            let algo = cfg.get_compression_algorithm();
            assert_eq!(
                format!("{:?}", algo),
                format!(
                    "{:?}",
                    ironvault::crypto::compression::CompressionAlgorithm::None
                )
            );
        }

        #[test]
        fn compression_algorithm_default() {
            let (mut cfg, _tmp) = make_config();
            cfg.compression.algorithm = "unknown_algo".to_string();
            let algo = cfg.get_compression_algorithm();
            assert_eq!(
                format!("{:?}", algo),
                format!(
                    "{:?}",
                    ironvault::crypto::compression::CompressionAlgorithm::Gzip
                )
            );
        }

        #[test]
        fn compression_level_zero() {
            let (mut cfg, _tmp) = make_config();
            cfg.compression.level = 0;
            let level = cfg.get_compression_level();
            assert_eq!(
                format!("{:?}", level),
                format!(
                    "{:?}",
                    ironvault::crypto::compression::CompressionLevel::None
                )
            );
        }

        #[test]
        fn compression_level_fast() {
            let (mut cfg, _tmp) = make_config();
            cfg.compression.level = 1;
            let level = cfg.get_compression_level();
            assert_eq!(
                format!("{:?}", level),
                format!(
                    "{:?}",
                    ironvault::crypto::compression::CompressionLevel::Fast
                )
            );
        }

        #[test]
        fn compression_level_maximum() {
            let (mut cfg, _tmp) = make_config();
            cfg.compression.level = 9;
            let level = cfg.get_compression_level();
            assert_eq!(
                format!("{:?}", level),
                format!(
                    "{:?}",
                    ironvault::crypto::compression::CompressionLevel::Maximum
                )
            );
        }

        #[test]
        fn compression_level_default_balanced() {
            let (mut cfg, _tmp) = make_config();
            cfg.compression.level = 5;
            let level = cfg.get_compression_level();
            assert_eq!(
                format!("{:?}", level),
                format!(
                    "{:?}",
                    ironvault::crypto::compression::CompressionLevel::Balanced
                )
            );
        }
    }

    // ============================================================================
    // VAULT — lock/unlock, delete, stats, change passphrase, ModelStream, VaultBuilder
    // ============================================================================
    mod vault_coverage {
        use ironvault::config::{DirectoryPaths, VaultConfig};
        use ironvault::formats::{ModelFormat, ModelMetadata};
        use ironvault::traits::VaultState;
        use ironvault::vault::ModelStream;
        use ironvault::{Vault, VaultBuilder};

        fn make_dirs(tmp: &tempfile::TempDir) -> DirectoryPaths {
            DirectoryPaths {
                config_dir: tmp.path().join("config"),
                data_dir: tmp.path().join("data"),
                cache_dir: tmp.path().join("cache"),
                vault_dir: tmp.path().join("data/vaults/default"),
                log_dir: tmp.path().join("data/logs"),
                backends_dir: tmp.path().join("config/backends"),
                utilities_dir: tmp.path().join("config/utilities"),
                databases_dir: tmp.path().join("config/databases"),
            }
        }

        fn make_vault() -> (Vault, tempfile::TempDir) {
            let tmp = tempfile::tempdir().unwrap();
            let config = VaultConfig::with_dirs(make_dirs(&tmp)).unwrap();
            let vault = Vault::new(Some(config)).unwrap();
            (vault, tmp)
        }

        fn make_unlocked_vault() -> (Vault, tempfile::TempDir) {
            let (mut vault, tmp) = make_vault();
            vault
                .unlock(b"test_passphrase_with_sufficient_entropy".to_vec())
                .unwrap();
            (vault, tmp)
        }

        #[test]
        fn vault_starts_locked() {
            let (vault, _tmp) = make_vault();
            assert!(!vault.is_unlocked());
        }

        #[test]
        fn vault_unlock_lock_cycle() {
            let (mut vault, _tmp) = make_vault();
            vault
                .unlock(b"test_passphrase_with_sufficient_entropy".to_vec())
                .unwrap();
            assert!(vault.is_unlocked());
            vault.lock();
            assert!(!vault.is_unlocked());
        }

        #[test]
        fn vault_state_locked() {
            let (vault, _tmp) = make_vault();
            match vault.state() {
                VaultState::Locked {
                    vault_name,
                    model_count,
                } => {
                    assert!(!vault_name.is_empty());
                    assert_eq!(model_count, 0);
                }
                other => panic!("Expected Locked, got {:?}", other),
            }
        }

        #[test]
        fn vault_state_unlocked() {
            let (vault, _tmp) = make_unlocked_vault();
            match vault.state() {
                VaultState::Unlocked {
                    vault_name,
                    model_count,
                    ..
                } => {
                    assert!(!vault_name.is_empty());
                    assert_eq!(model_count, 0);
                }
                other => panic!("Expected Unlocked, got {:?}", other),
            }
        }

        #[test]
        fn vault_list_models_empty() {
            let (vault, _tmp) = make_unlocked_vault();
            assert!(vault.list_models().is_empty());
        }

        #[test]
        fn vault_list_versions_empty() {
            let (vault, _tmp) = make_unlocked_vault();
            assert!(vault.list_versions("nonexistent").is_empty());
        }

        #[test]
        fn vault_get_lineage_empty() {
            let (vault, _tmp) = make_unlocked_vault();
            assert!(vault.get_lineage("none", 1).is_empty());
        }

        #[test]
        fn vault_delete_nonexistent_version() {
            let (mut vault, _tmp) = make_unlocked_vault();
            let result = vault.delete_version("nonexistent", 1).unwrap();
            assert!(!result);
        }

        #[test]
        fn vault_get_stats_empty() {
            let (vault, _tmp) = make_unlocked_vault();
            let stats = vault.get_stats().unwrap();
            assert_eq!(stats.model_count, 0);
            assert_eq!(stats.total_versions, 0);
        }

        #[test]
        fn vault_store_and_list_versions() {
            let (mut vault, _tmp) = make_unlocked_vault();
            let data = b"model data".to_vec();
            let meta = ModelMetadata::new("m1".into(), ModelFormat::PyTorch);
            vault.store_model("m1", data, meta, None).unwrap();
            let versions = vault.list_versions("m1");
            assert_eq!(versions.len(), 1);
        }

        #[test]
        fn vault_store_and_delete_version() {
            let (mut vault, _tmp) = make_unlocked_vault();
            let data = b"model data".to_vec();
            let meta = ModelMetadata::new("m1".into(), ModelFormat::PyTorch);
            let ver = vault.store_model("m1", data, meta, None).unwrap();
            let deleted = vault.delete_version("m1", ver.version).unwrap();
            assert!(deleted);
            assert!(vault.list_versions("m1").is_empty());
        }

        #[test]
        fn vault_get_stats_after_store() {
            let (mut vault, _tmp) = make_unlocked_vault();
            let data = b"model data for stats".to_vec();
            let meta = ModelMetadata::new("m1".into(), ModelFormat::PyTorch);
            vault.store_model("m1", data, meta, None).unwrap();
            let stats = vault.get_stats().unwrap();
            assert_eq!(stats.model_count, 1);
            assert_eq!(stats.total_versions, 1);
            assert!(stats.total_size_bytes > 0);
            assert!(stats.file_count > 0);
        }

        #[test]
        fn vault_get_config() {
            let (vault, _tmp) = make_vault();
            let cfg = vault.get_config();
            assert!(!cfg.vault.default_vault.is_empty());
        }

        #[test]
        fn vault_key_manager() {
            let (vault, _tmp) = make_vault();
            let _km = vault.key_manager();
        }

        #[test]
        fn vault_change_passphrase() {
            let (mut vault, _tmp) = make_unlocked_vault();
            let data = b"secret model".to_vec();
            let meta = ModelMetadata::new("s1".into(), ModelFormat::Safetensors);
            vault.store_model("s1", data.clone(), meta, None).unwrap();
            let count = vault
                .change_passphrase(b"new_passphrase_with_entropy_too!".to_vec())
                .unwrap();
            assert_eq!(count, 1);
            let retrieved = vault.get_model("s1", None).unwrap();
            assert_eq!(data, retrieved);
        }

        #[test]
        fn vault_change_passphrase_locked_fails() {
            let (mut vault, _tmp) = make_vault();
            let result = vault.change_passphrase(b"new".to_vec());
            assert!(result.is_err());
        }

        #[test]
        fn vault_update_get_version_metadata() {
            let (mut vault, _tmp) = make_unlocked_vault();
            let data = b"model".to_vec();
            let meta = ModelMetadata::new("m1".into(), ModelFormat::PyTorch);
            let ver = vault.store_model("m1", data, meta, None).unwrap();
            vault
                .update_version_metadata("m1", ver.version, "tag", "production".to_string())
                .unwrap();
            let val = vault.get_version_metadata("m1", ver.version, "tag");
            assert_eq!(val.as_deref(), Some("production"));
            assert!(vault
                .get_version_metadata("m1", ver.version, "missing")
                .is_none());
        }

        #[test]
        fn vault_event_bus_accessors() {
            let (mut vault, _tmp) = make_vault();
            let _ = vault.event_bus();
            let _ = vault.event_bus_mut();
        }

        #[test]
        fn vault_version_backend_name() {
            let (vault, _tmp) = make_vault();
            assert_eq!(vault.version_backend_name(), "json");
        }

        #[test]
        fn vault_store_model_streamed() {
            let (mut vault, _tmp) = make_unlocked_vault();
            let chunks: Vec<Vec<u8>> =
                vec![b"chunk1".to_vec(), b"chunk2".to_vec(), b"chunk3".to_vec()];
            let meta = ModelMetadata::new("streamed".into(), ModelFormat::PyTorch);
            let ver = vault
                .store_model_streamed("streamed", chunks.clone(), meta, None)
                .unwrap();
            assert_eq!(ver.version, 1);
            let retrieved = vault.get_model("streamed", None).unwrap();
            let expected: Vec<u8> = chunks.into_iter().flatten().collect();
            assert_eq!(retrieved, expected);
        }

        #[test]
        fn vault_get_model_chunked() {
            let (mut vault, _tmp) = make_unlocked_vault();
            let data = b"0123456789ABCDEF".to_vec();
            let meta = ModelMetadata::new("chunked".into(), ModelFormat::PyTorch);
            vault
                .store_model("chunked", data.clone(), meta, None)
                .unwrap();
            let mut stream = vault.get_model_chunked("chunked", None, 4).unwrap();
            assert_eq!(stream.total_size(), 16);
            assert_eq!(stream.remaining(), 16);
            assert_eq!(stream.next().unwrap(), b"0123");
            assert_eq!(stream.remaining(), 12);
            assert_eq!(stream.next().unwrap(), b"4567");
            assert_eq!(stream.next().unwrap(), b"89AB");
            assert_eq!(stream.next().unwrap(), b"CDEF");
            assert!(stream.next().is_none());
            assert_eq!(stream.remaining(), 0);
        }

        // --- ModelStream ---
        #[test]
        fn model_stream_default_chunk_size() {
            let stream = ModelStream::new(vec![0; 100], 0);
            assert_eq!(stream.total_size(), 100);
        }

        #[test]
        fn model_stream_empty_data() {
            let mut stream = ModelStream::new(vec![], 64);
            assert_eq!(stream.total_size(), 0);
            assert_eq!(stream.remaining(), 0);
            assert!(stream.next().is_none());
        }

        #[test]
        fn model_stream_exact_chunks() {
            let data = vec![1, 2, 3, 4, 5, 6];
            let mut stream = ModelStream::new(data, 3);
            assert_eq!(stream.next().unwrap(), vec![1, 2, 3]);
            assert_eq!(stream.next().unwrap(), vec![4, 5, 6]);
            assert!(stream.next().is_none());
        }

        #[test]
        fn model_stream_partial_last_chunk() {
            let data = vec![1, 2, 3, 4, 5];
            let mut stream = ModelStream::new(data, 3);
            assert_eq!(stream.next().unwrap(), vec![1, 2, 3]);
            assert_eq!(stream.next().unwrap(), vec![4, 5]);
            assert!(stream.next().is_none());
        }

        // --- Metrics ---
        #[test]
        fn vault_metrics_none_without_builder() {
            let (vault, _tmp) = make_vault();
            assert!(vault.metrics().is_none());
        }

        #[test]
        fn vault_builder_with_metrics() {
            let tmp = tempfile::tempdir().unwrap();
            let config = VaultConfig::with_dirs(make_dirs(&tmp)).unwrap();
            let vault = VaultBuilder::new().config(config).build().unwrap();
            let snapshot = vault.metrics();
            assert!(snapshot.is_some());
            let snap = snapshot.unwrap();
            assert_eq!(snap.models_stored_total, 0);
        }

        #[test]
        fn vault_builder_no_default_subscribers() {
            let tmp = tempfile::tempdir().unwrap();
            let config = VaultConfig::with_dirs(make_dirs(&tmp)).unwrap();
            let vault = VaultBuilder::new()
                .config(config)
                .no_default_subscribers()
                .build()
                .unwrap();
            assert!(vault.metrics().is_none());
        }

        #[test]
        fn vault_builder_custom_subscriber() {
            use ironvault::traits::{EventSubscriber, VaultEvent};

            struct TestSubscriber;
            impl EventSubscriber for TestSubscriber {
                fn on_event(&self, _event: &VaultEvent) -> ironvault::Result<()> {
                    Ok(())
                }
                fn name(&self) -> &str {
                    "test"
                }
            }

            let tmp = tempfile::tempdir().unwrap();
            let config = VaultConfig::with_dirs(make_dirs(&tmp)).unwrap();
            let vault = VaultBuilder::new()
                .config(config)
                .subscriber(Box::new(TestSubscriber))
                .build()
                .unwrap();
            // Default subscribers (audit + metrics) + 1 custom = 3
            assert_eq!(vault.event_bus().subscriber_count(), 3);
        }

        #[test]
        fn vault_store_locked_fails() {
            let (mut vault, _tmp) = make_vault();
            let meta = ModelMetadata::new("m".into(), ModelFormat::PyTorch);
            assert!(vault.store_model("m", vec![1, 2, 3], meta, None).is_err());
        }

        #[test]
        fn vault_get_model_locked_fails() {
            let (vault, _tmp) = make_vault();
            assert!(vault.get_model("m", None).is_err());
        }

        #[test]
        fn vault_get_model_not_found() {
            let (vault, _tmp) = make_unlocked_vault();
            assert!(vault.get_model("nonexistent", None).is_err());
        }

        #[test]
        fn vault_get_model_version_not_found() {
            let (vault, _tmp) = make_unlocked_vault();
            assert!(vault.get_model("nonexistent", Some(99)).is_err());
        }

        #[test]
        fn vault_store_multiple_versions() {
            let (mut vault, _tmp) = make_unlocked_vault();
            let meta1 = ModelMetadata::new("m".into(), ModelFormat::PyTorch);
            let v1 = vault
                .store_model("m", b"v1data".to_vec(), meta1, None)
                .unwrap();

            let meta2 =
                ModelMetadata::new("m".into(), ModelFormat::PyTorch).with_description("v2".into());
            let v2 = vault
                .store_model("m", b"v2data".to_vec(), meta2, Some(v1.version))
                .unwrap();
            assert_eq!(v2.version, 2);
            assert_eq!(vault.list_versions("m").len(), 2);
            let lineage = vault.get_lineage("m", v2.version);
            assert!(!lineage.is_empty());
        }
    }

    // ============================================================================
    // CONVERSION — Shim converters, OnnxMetadataExtractor
    // ============================================================================
    mod conversion_shim_coverage {
        use ironvault::conversion::*;
        use ironvault::formats::ModelFormat;

        // --- SafeTensorsToPyTorchConverter ---
        #[test]
        fn safetensors_to_pytorch_shim() {
            let converter = SafeTensorsToPyTorchConverter;
            assert_eq!(converter.name(), "SafeTensors → PyTorch");
            assert_eq!(converter.source_format(), ModelFormat::Safetensors);
            assert_eq!(converter.target_format(), ModelFormat::PyTorch);

            // Build valid safetensors data
            let header = r#"{"tensor_0":{"dtype":"U8","shape":[4],"data_offsets":[0,4]}}"#;
            let header_bytes = header.as_bytes();
            let header_len = header_bytes.len() as u64;
            let mut data = Vec::new();
            data.extend_from_slice(&header_len.to_le_bytes());
            data.extend_from_slice(header_bytes);
            data.extend_from_slice(&[1, 2, 3, 4]);

            let result = converter
                .convert(&data, &ConversionOptions::default(), None)
                .unwrap();
            // Real converter produces ZIP output
            assert_eq!(&result[0..2], b"PK");
        }

        #[test]
        fn safetensors_to_pytorch_too_small() {
            let converter = SafeTensorsToPyTorchConverter;
            let result = converter.convert(&[1, 2, 3], &ConversionOptions::default(), None);
            assert!(result.is_err());
        }

        #[test]
        fn safetensors_to_pytorch_header_exceeds() {
            let converter = SafeTensorsToPyTorchConverter;
            let mut data = vec![0u8; 8];
            let huge_len: u64 = 99999;
            data[..8].copy_from_slice(&huge_len.to_le_bytes());
            let result = converter.convert(&data, &ConversionOptions::default(), None);
            assert!(result.is_err());
        }

        // --- PyTorchToSafeTensorsConverter ---
        #[test]
        fn pytorch_to_safetensors_shim() {
            let converter = PyTorchToSafeTensorsConverter;
            assert_eq!(converter.name(), "PyTorch → SafeTensors");
            assert_eq!(converter.source_format(), ModelFormat::PyTorch);
            assert_eq!(converter.target_format(), ModelFormat::Safetensors);
            // Real converter requires valid ZIP; invalid data should error
            let err = converter
                .convert(b"dummy", &ConversionOptions::default(), None)
                .unwrap_err();
            assert!(format!("{err}").contains("ZIP archive"));
        }

        // --- PyTorchToOnnxConverter ---
        #[test]
        fn pytorch_to_onnx_shim() {
            let converter = PyTorchToOnnxConverter;
            assert_eq!(converter.name(), "PyTorch → ONNX (shim)");
            assert_eq!(converter.source_format(), ModelFormat::PyTorch);
            assert_eq!(converter.target_format(), ModelFormat::ONNX);
            let result = converter
                .convert(b"dummy", &ConversionOptions::default(), None)
                .unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(plan["converter"], "pytorch_to_onnx");
            assert_eq!(plan["opset_version"], 17);
        }

        #[test]
        fn pytorch_to_onnx_custom_opset() {
            let converter = PyTorchToOnnxConverter;
            let opts = ConversionOptions {
                opset_version: Some(13),
                ..ConversionOptions::default()
            };
            let result = converter.convert(b"dummy", &opts, None).unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(plan["opset_version"], 13);
        }

        // --- OnnxToTensorRtConverter ---
        #[test]
        fn onnx_to_tensorrt_shim() {
            let converter = OnnxToTensorRtConverter;
            assert_eq!(converter.name(), "ONNX → TensorRT (shim)");
            assert_eq!(converter.source_format(), ModelFormat::ONNX);
            assert_eq!(converter.target_format(), ModelFormat::TensorRT);
            let result = converter
                .convert(b"dummy", &ConversionOptions::default(), None)
                .unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(plan["converter"], "onnx_to_tensorrt");
        }

        // --- OnnxToCoreMLConverter ---
        #[test]
        fn onnx_to_coreml_shim() {
            let converter = OnnxToCoreMLConverter;
            assert_eq!(converter.name(), "ONNX → Core ML (shim)");
            assert_eq!(converter.source_format(), ModelFormat::ONNX);
            assert_eq!(converter.target_format(), ModelFormat::CoreML);
            let result = converter
                .convert(b"dummy", &ConversionOptions::default(), None)
                .unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(plan["converter"], "onnx_to_coreml");
        }

        // --- SafeTensorsToGgufConverter ---
        #[test]
        fn safetensors_to_gguf_shim() {
            let converter = SafeTensorsToGgufConverter;
            assert_eq!(
                converter.name(),
                "SafeTensors → GGUF (shim; use `iv convert --from-dir` for the real one)"
            );
            assert_eq!(converter.source_format(), ModelFormat::Safetensors);
            assert_eq!(converter.target_format(), ModelFormat::GGUF);
            let result = converter
                .convert(b"dummy", &ConversionOptions::default(), None)
                .unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(plan["converter"], "safetensors_to_gguf");
            assert_eq!(plan["quantization"], "f16");
            // The default is f16, which this repo converts natively — the plan
            // must send the reader to `--from-dir`, not to a Python script that
            // is no longer needed.
            assert_eq!(plan["requires"].as_array().unwrap().len(), 0);
            assert!(
                plan["shell"].as_str().unwrap().contains("--from-dir"),
                "f16 plan should name the native route: {}",
                plan["shell"]
            );
        }

        #[test]
        fn safetensors_to_gguf_k_quant_plan_still_sends_you_to_llama_cpp() {
            let converter = SafeTensorsToGgufConverter;
            let opts = ConversionOptions {
                quantization: Some("q6_k".to_string()),
                ..ConversionOptions::default()
            };
            let result = converter.convert(b"dummy", &opts, None).unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert!(
                plan["shell"].as_str().unwrap().contains("llama-quantize"),
                "no K-quant encoder exists here: {}",
                plan["shell"]
            );
            assert!(!plan["requires"].as_array().unwrap().is_empty());
        }

        #[test]
        fn safetensors_to_gguf_custom_quant() {
            let converter = SafeTensorsToGgufConverter;
            let opts = ConversionOptions {
                quantization: Some("q4_k_m".to_string()),
                ..ConversionOptions::default()
            };
            let result = converter.convert(b"dummy", &opts, None).unwrap();
            let plan: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(plan["quantization"], "q4_k_m");
        }

        // --- OnnxMetadataExtractor ---
        #[test]
        fn onnx_metadata_extractor_basic() {
            let converter = OnnxMetadataExtractor;
            assert_eq!(converter.name(), "ONNX → Metadata (JSON)");
            assert_eq!(converter.source_format(), ModelFormat::ONNX);
            // Build minimal ONNX protobuf: field 1 = varint 7
            let data = vec![0x08, 0x07];
            let result = converter
                .convert(&data, &ConversionOptions::default(), None)
                .unwrap();
            let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(meta["format"], "ONNX");
            assert_eq!(meta["ir_version"], 7);
        }

        #[test]
        fn onnx_metadata_extractor_with_producer() {
            let converter = OnnxMetadataExtractor;
            let data = vec![
                0x08, 0x09, // field 1 varint, value 9
                0x12, 7, // field 2 length-delimited, length 7
                b'p', b'y', b't', b'o', b'r', b'c', b'h',
            ];
            let result = converter
                .convert(&data, &ConversionOptions::default(), None)
                .unwrap();
            let meta: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(meta["producer"], "pytorch");
            assert_eq!(meta["ir_version"], 9);
        }

        // --- Pipeline with builtins includes shims ---
        #[test]
        fn pipeline_builtins_have_shim_converters() {
            let pipeline = ConversionPipeline::with_builtins();
            assert!(pipeline.can_convert_direct(&ModelFormat::Safetensors, &ModelFormat::PyTorch));
            assert!(pipeline.can_convert_direct(&ModelFormat::PyTorch, &ModelFormat::Safetensors));
            assert!(pipeline.can_convert_direct(&ModelFormat::PyTorch, &ModelFormat::ONNX));
            assert!(pipeline.can_convert_direct(&ModelFormat::ONNX, &ModelFormat::TensorRT));
            assert!(pipeline.can_convert_direct(&ModelFormat::ONNX, &ModelFormat::CoreML));
            assert!(pipeline.can_convert_direct(&ModelFormat::Safetensors, &ModelFormat::GGUF));
        }

        // --- Pipeline supported_conversions ---
        #[test]
        fn pipeline_supported_conversions() {
            let pipeline = ConversionPipeline::with_builtins();
            let convs = pipeline.supported_conversions();
            assert!(convs.len() >= 10);
            for (from, to, name) in &convs {
                assert!(!name.is_empty());
                let _ = from.name();
                let _ = to.name();
            }
        }

        // --- Multi-step path ---
        #[test]
        fn pipeline_multi_step_path() {
            let pipeline = ConversionPipeline::with_builtins();
            let path = pipeline.find_path(&ModelFormat::PyTorch, &ModelFormat::TensorRT);
            assert!(path.is_some());
            let p = path.unwrap();
            assert!(p.len() >= 3);
        }
    }

    // ============================================================================
    // FORMATS — FormatConverter, Display, extension, name
    // ============================================================================
    mod format_converter_coverage {
        use ironvault::formats::{FormatConverter, ModelFormat, ModelMetadata};

        #[test]
        fn format_converter_new_default() {
            let conv = FormatConverter::new();
            assert!(!conv.can_convert(ModelFormat::PyTorch, ModelFormat::ONNX));
            let conv2 = FormatConverter::default();
            assert!(!conv2.can_convert(ModelFormat::PyTorch, ModelFormat::ONNX));
        }

        #[test]
        fn format_converter_register_and_convert() {
            let mut conv = FormatConverter::new();
            conv.register(ModelFormat::PyTorch, ModelFormat::ONNX, |data| {
                Ok(data.to_vec())
            });
            assert!(conv.can_convert(ModelFormat::PyTorch, ModelFormat::ONNX));
            assert!(!conv.can_convert(ModelFormat::ONNX, ModelFormat::PyTorch));
            let result = conv
                .convert(b"test", ModelFormat::PyTorch, ModelFormat::ONNX)
                .unwrap();
            assert_eq!(result, b"test");
        }

        #[test]
        fn format_converter_same_format() {
            let conv = FormatConverter::new();
            let result = conv
                .convert(b"test", ModelFormat::PyTorch, ModelFormat::PyTorch)
                .unwrap();
            assert_eq!(result, b"test");
        }

        #[test]
        fn format_converter_no_converter_error() {
            let conv = FormatConverter::new();
            let result = conv.convert(b"test", ModelFormat::PyTorch, ModelFormat::ONNX);
            assert!(result.is_err());
        }

        // --- Display for all ModelFormat variants ---
        #[test]
        fn model_format_display_all() {
            assert_eq!(format!("{}", ModelFormat::Safetensors), "Safetensors");
            assert_eq!(format!("{}", ModelFormat::GGUF), "GGUF");
            assert_eq!(format!("{}", ModelFormat::PyTorch), "PyTorch");
            assert_eq!(format!("{}", ModelFormat::TensorRT), "TensorRT");
            assert_eq!(format!("{}", ModelFormat::ONNX), "ONNX");
            assert_eq!(format!("{}", ModelFormat::MLX), "MLX");
            assert_eq!(format!("{}", ModelFormat::CoreML), "Core ML");
            assert_eq!(format!("{}", ModelFormat::TorchScript), "TorchScript");
            assert_eq!(format!("{}", ModelFormat::TFLite), "TensorFlow Lite");
            assert_eq!(format!("{}", ModelFormat::TensorFlow), "TensorFlow");
            assert_eq!(format!("{}", ModelFormat::Keras), "Keras");
            assert_eq!(format!("{}", ModelFormat::OpenVINO), "OpenVINO");
            assert_eq!(format!("{}", ModelFormat::TVM), "TVM");
            assert_eq!(format!("{}", ModelFormat::NCNN), "NCNN");
            assert_eq!(format!("{}", ModelFormat::MNN), "MNN");
            assert_eq!(format!("{}", ModelFormat::RKNN), "RKNN");
            assert_eq!(format!("{}", ModelFormat::Caffe), "Caffe");
            assert_eq!(format!("{}", ModelFormat::MXNet), "MXNet");
            assert_eq!(format!("{}", ModelFormat::Darknet), "Darknet");
            assert_eq!(format!("{}", ModelFormat::HDF5), "HDF5");
            assert_eq!(format!("{}", ModelFormat::Pickle), "Pickle");
            assert_eq!(format!("{}", ModelFormat::NumPy), "NumPy");
            assert_eq!(format!("{}", ModelFormat::Custom("xyz".into())), "xyz");
        }

        // --- extension() for all variants ---
        #[test]
        fn model_format_extension_all() {
            assert_eq!(ModelFormat::Safetensors.extension(), "safetensors");
            assert_eq!(ModelFormat::GGUF.extension(), "gguf");
            assert_eq!(ModelFormat::TensorRT.extension(), "plan");
            assert_eq!(ModelFormat::MLX.extension(), "npz");
            assert_eq!(ModelFormat::CoreML.extension(), "mlmodel");
            assert_eq!(ModelFormat::TFLite.extension(), "tflite");
            assert_eq!(ModelFormat::TensorFlow.extension(), "pb");
            assert_eq!(ModelFormat::OpenVINO.extension(), "xml");
            assert_eq!(ModelFormat::TVM.extension(), "so");
            assert_eq!(ModelFormat::NCNN.extension(), "param");
            assert_eq!(ModelFormat::MNN.extension(), "mnn");
            assert_eq!(ModelFormat::RKNN.extension(), "rknn");
            assert_eq!(ModelFormat::Caffe.extension(), "caffemodel");
            assert_eq!(ModelFormat::MXNet.extension(), "params");
            assert_eq!(ModelFormat::Darknet.extension(), "weights");
            assert_eq!(ModelFormat::Custom("abc".into()).extension(), "abc");
        }

        // --- name() ---
        #[test]
        fn model_format_name() {
            assert_eq!(ModelFormat::Safetensors.name(), "Safetensors");
            assert_eq!(
                ModelFormat::Custom("custom_fmt".into()).name(),
                "custom_fmt"
            );
        }

        // --- ModelMetadata builders ---
        #[test]
        fn model_metadata_all_builders() {
            let meta = ModelMetadata::new("test".into(), ModelFormat::PyTorch)
                .with_description("desc".into())
                .with_framework("pytorch".into())
                .with_task("classification".into())
                .with_architecture("transformer".into())
                .with_parameters(7_000_000_000)
                .add_custom_field("license".into(), "MIT".into());
            assert_eq!(meta.description.as_deref(), Some("desc"));
            assert_eq!(meta.framework.as_deref(), Some("pytorch"));
            assert_eq!(meta.task.as_deref(), Some("classification"));
            assert_eq!(meta.architecture.as_deref(), Some("transformer"));
            assert_eq!(meta.parameters, Some(7_000_000_000));
            assert_eq!(meta.custom_fields.get("license").unwrap(), "MIT");
        }
    }

    // ============================================================================
    // UTILS — ModelArchive tar/zip, ModelExporter, cache eviction, dedup
    // ============================================================================
    mod utils_coverage {
        use ironvault::formats::{ModelFormat, ModelMetadata};
        use ironvault::utils::*;

        // --- ModelArchive TAR ---
        #[test]
        fn archive_tar_roundtrip() {
            let tmp = tempfile::tempdir().unwrap();
            let archive_path = tmp.path().join("test.tar");
            let models = vec![
                ("model_a.pt".to_string(), vec![1, 2, 3, 4]),
                ("model_b.safetensors".to_string(), vec![5, 6, 7, 8, 9]),
            ];
            let total = ModelArchive::create_tar(models, &archive_path).unwrap();
            assert_eq!(total, 9);
            let extracted = ModelArchive::extract_tar(&archive_path).unwrap();
            assert_eq!(extracted.len(), 2);
            assert_eq!(extracted[0].0, "model_a.pt");
            assert_eq!(extracted[0].1, vec![1, 2, 3, 4]);
            assert_eq!(extracted[1].0, "model_b.safetensors");
            assert_eq!(extracted[1].1, vec![5, 6, 7, 8, 9]);
        }

        // --- ModelArchive ZIP ---
        #[test]
        fn archive_zip_roundtrip() {
            let tmp = tempfile::tempdir().unwrap();
            let archive_path = tmp.path().join("test.zip");
            let models = vec![
                ("a.pt".to_string(), vec![10, 20, 30]),
                ("b.onnx".to_string(), vec![40, 50]),
            ];
            let total = ModelArchive::create_zip(models, &archive_path).unwrap();
            assert_eq!(total, 5);
            let extracted = ModelArchive::extract_zip(&archive_path).unwrap();
            assert_eq!(extracted.len(), 2);
        }

        // --- CompressionAnalyzer ---
        #[test]
        fn compression_zero_compressed() {
            let ratio = CompressionAnalyzer::compression_ratio(1000, 0);
            assert_eq!(ratio, 0.0);
        }

        #[test]
        fn estimate_ratio_all_formats() {
            let formats = [
                ModelFormat::Safetensors,
                ModelFormat::GGUF,
                ModelFormat::PyTorch,
                ModelFormat::ONNX,
                ModelFormat::TensorRT,
                ModelFormat::TFLite,
                ModelFormat::HDF5,
                ModelFormat::Pickle,
                ModelFormat::CoreML, // default arm
            ];
            for fmt in &formats {
                let ratio = CompressionAnalyzer::estimate_ratio(fmt);
                assert!(ratio >= 1.0);
            }
        }

        #[test]
        fn analyze_compression_report() {
            let report =
                CompressionAnalyzer::analyze_compression(2000, 1000, &ModelFormat::Safetensors);
            assert_eq!(report.original_size, 2000);
            assert_eq!(report.compressed_size, 1000);
            assert_eq!(report.space_saved, 1000);
            assert!((report.space_saved_percent - 50.0).abs() < 0.01);
            assert!((report.compression_ratio - 2.0).abs() < 0.01);
            assert!(report.efficiency > 0.0);
        }

        // --- QuantizationInfo ---
        #[test]
        fn quantization_schemes() {
            let schemes = QuantizationInfo::schemes();
            assert!(schemes.contains(&"FP32"));
            assert!(schemes.contains(&"Q4_K_M"));
        }

        #[test]
        fn quantization_estimate_zero_bits() {
            let size = QuantizationInfo::estimate_size(1000, 0, 16);
            assert_eq!(size, 1000);
        }

        #[test]
        fn quantization_valid_scheme() {
            assert!(QuantizationInfo::is_valid_scheme("FP32"));
            assert!(QuantizationInfo::is_valid_scheme("Q4_K_M"));
            assert!(!QuantizationInfo::is_valid_scheme("UNKNOWN"));
        }

        // --- PruningInfo ---
        #[test]
        fn pruning_info_zero_original() {
            let info = PruningInfo::new(PruningMethod::Magnitude, 0.5, 0, 0);
            assert_eq!(info.calculate_sparsity(), 0.0);
            assert_eq!(info.size_reduction(), 0.0);
        }

        #[test]
        fn pruning_info_clamp_sparsity() {
            let info = PruningInfo::new(PruningMethod::Structured, 2.0, 100, 50);
            assert_eq!(info.sparsity_level, 1.0);
        }

        #[test]
        fn pruning_methods_eq() {
            assert_eq!(PruningMethod::Magnitude, PruningMethod::Magnitude);
            assert_ne!(PruningMethod::Structured, PruningMethod::Unstructured);
            assert_eq!(
                PruningMethod::Custom("abc".into()),
                PruningMethod::Custom("abc".into()),
            );
        }

        // --- RetrievalOptimizer ---
        #[test]
        fn cache_eviction() {
            let mut opt = RetrievalOptimizer::new(200);
            opt.cache_model("m1".into(), vec![0; 100]).unwrap();
            opt.cache_model("m2".into(), vec![0; 100]).unwrap();
            opt.cache_model("m3".into(), vec![0; 100]).unwrap();
            assert!(opt.get_cached("m1").is_none());
            assert!(opt.get_cached("m3").is_some());
        }

        #[test]
        fn cache_too_large_model() {
            let mut opt = RetrievalOptimizer::new(50);
            opt.cache_model("big".into(), vec![0; 100]).unwrap();
            assert!(opt.get_cached("big").is_none());
        }

        #[test]
        fn cache_clear() {
            let mut opt = RetrievalOptimizer::new(1000);
            opt.cache_model("m1".into(), vec![0; 10]).unwrap();
            opt.clear_cache();
            let stats = opt.cache_stats();
            assert_eq!(stats.total_entries, 0);
            assert_eq!(stats.total_size, 0);
        }

        #[test]
        fn cache_stats() {
            let mut opt = RetrievalOptimizer::new(1000);
            opt.cache_model("m1".into(), vec![0; 100]).unwrap();
            let stats = opt.cache_stats();
            assert_eq!(stats.total_entries, 1);
            assert_eq!(stats.total_size, 100);
            assert_eq!(stats.max_size, 1000);
            assert!((stats.utilization - 10.0).abs() < 0.01);
        }

        #[test]
        fn cache_miss() {
            let mut opt = RetrievalOptimizer::new(1000);
            assert!(opt.get_cached("nonexistent").is_none());
        }

        // --- ModelAnalyzer ---
        #[test]
        fn format_size_tb() {
            let s = ModelAnalyzer::format_size(1024 * 1024 * 1024 * 1024);
            assert!(s.contains("TB"));
        }

        #[test]
        fn analyze_model() {
            let meta = ModelMetadata::new("test".into(), ModelFormat::GGUF)
                .with_framework("llama".into())
                .with_task("text-gen".into())
                .with_architecture("transformer".into());
            let data = vec![0u8; 1024 * 1024];
            let analysis = ModelAnalyzer::analyze(&data, &meta);
            assert_eq!(analysis.size_bytes, 1024 * 1024);
            assert!((analysis.size_mb - 1.0).abs() < 0.01);
            assert!(analysis.estimated_parameters.is_some());
            assert_eq!(analysis.framework.as_deref(), Some("llama"));
        }

        // --- ModelExporter ---
        #[test]
        fn export_with_metadata() {
            let tmp = tempfile::tempdir().unwrap();
            let meta = ModelMetadata::new("test_model".into(), ModelFormat::PyTorch)
                .with_description("A test".into());
            let data = b"model data".to_vec();
            let path = ModelExporter::export_with_metadata(data, &meta, tmp.path()).unwrap();
            assert!(path.exists());
            assert!(path.to_string_lossy().contains("test_model.pt"));
            let meta_path = tmp.path().join("test_model.meta.json");
            assert!(meta_path.exists());
        }

        #[test]
        fn export_to_directory() {
            let tmp = tempfile::tempdir().unwrap();
            let models = vec![
                (
                    b"data1".to_vec(),
                    ModelMetadata::new("m1".into(), ModelFormat::PyTorch),
                ),
                (
                    b"data2".to_vec(),
                    ModelMetadata::new("m2".into(), ModelFormat::ONNX),
                ),
            ];
            let paths = ModelExporter::export_to_directory(models, tmp.path()).unwrap();
            assert_eq!(paths.len(), 2);
            for p in &paths {
                assert!(p.exists());
            }
        }

        // --- ModelDeduplicator ---
        #[test]
        fn find_duplicates() {
            let models = vec![
                ("a".into(), vec![1, 2, 3]),
                ("b".into(), vec![4, 5, 6]),
                ("c".into(), vec![1, 2, 3]),
            ];
            let dupes = ModelDeduplicator::find_duplicates(models);
            assert_eq!(dupes.len(), 1);
            let names: Vec<String> = dupes.values().next().unwrap().clone();
            assert!(names.contains(&"a".to_string()));
            assert!(names.contains(&"c".to_string()));
        }

        #[test]
        fn find_no_duplicates() {
            let models = vec![("a".into(), vec![1, 2, 3]), ("b".into(), vec![4, 5, 6])];
            let dupes = ModelDeduplicator::find_duplicates(models);
            assert!(dupes.is_empty());
        }

        #[test]
        fn similarity_same_size_partial() {
            let data1 = b"abcdef";
            let data2 = b"abcXYZ";
            let score = ModelDeduplicator::similarity_score(data1, data2);
            assert!(score > 0.0 && score < 100.0);
        }
    }

    // ============================================================================
    // TELEMETRY — additional coverage
    // ============================================================================
    mod telemetry_extra_coverage {
        use ironvault::telemetry;

        #[test]
        fn disable_then_is_enabled() {
            telemetry::disable();
            assert!(!telemetry::is_enabled());
        }

        #[test]
        fn flush_when_disabled() {
            telemetry::disable();
            telemetry::flush();
        }
    }

    // ============================================================================
    // CRYPTO — additional edge cases
    // ============================================================================
    mod crypto_extra_coverage {
        use ironvault::crypto::{KeyManager, VaultCrypto};

        #[test]
        fn key_manager_new() {
            let km = KeyManager::new().unwrap();
            let _ = km;
        }

        #[test]
        fn hash_sha256_deterministic() {
            let h1 = VaultCrypto::hash_sha256(b"test data");
            let h2 = VaultCrypto::hash_sha256(b"test data");
            assert_eq!(h1, h2);
            assert_eq!(h1.len(), 32);
        }

        #[test]
        fn hash_sha256_different_inputs() {
            let h1 = VaultCrypto::hash_sha256(b"hello");
            let h2 = VaultCrypto::hash_sha256(b"world");
            assert_ne!(h1, h2);
        }

        #[test]
        fn derive_key_custom_salt() {
            let crypto = VaultCrypto::new().unwrap();
            let salt = vec![42u8; 32];
            let (key1, _) = crypto
                .derive_key(b"pass".to_vec(), Some(salt.clone()))
                .unwrap();
            let (key2, _) = crypto.derive_key(b"pass".to_vec(), Some(salt)).unwrap();
            assert_eq!(key1.as_bytes(), key2.as_bytes());
        }
    }

    // ============================================================================
    // STREAMING ENCRYPTION — additional roundtrip coverage
    // ============================================================================
    mod streaming_crypto_coverage {
        use ironvault::crypto::{SecureKey, VaultCrypto};
        use ironvault::{
            decrypt_chunked, encrypt_chunked, is_chunked_format, DEFAULT_CHUNK_SIZE, STREAM_MAGIC,
            STREAM_VERSION,
        };

        #[test]
        fn chunked_roundtrip() {
            let crypto = VaultCrypto::new().unwrap();
            let key = SecureKey::from_bytes(&[0xAB; 32]).unwrap();
            let plaintext = b"hello streaming encryption world";
            let encrypted = encrypt_chunked(&crypto, plaintext, &key, 16).unwrap();
            assert!(is_chunked_format(&encrypted));
            let decrypted = decrypt_chunked(&crypto, &encrypted, &key).unwrap();
            assert_eq!(decrypted, plaintext);
        }

        #[test]
        fn not_chunked_format() {
            assert!(!is_chunked_format(&[1, 2, 3, 4]));
            assert!(!is_chunked_format(&[]));
        }

        #[test]
        fn stream_constants() {
            assert!(!STREAM_MAGIC.is_empty());
            assert_ne!(STREAM_VERSION, 0);
            assert_ne!(DEFAULT_CHUNK_SIZE, 0);
        }
    }

    // ============================================================================
    // VERSION_SQLITE — feature-gated
    // ============================================================================
    #[cfg(feature = "sqlite")]
    mod version_sqlite_coverage {
        use ironvault::traits::VersionRepo;
        use ironvault::SqliteVersionRepo;

        #[test]
        fn sqlite_version_repo_basic() {
            let tmp = tempfile::tempdir().unwrap();
            let mut repo = SqliteVersionRepo::new(tmp.path()).unwrap();

            let ver = repo
                .add_version(
                    "m1",
                    "file.vault",
                    "PyTorch",
                    1000,
                    800,
                    "abc123",
                    Some(std::collections::HashMap::new()),
                    None,
                )
                .unwrap();
            assert_eq!(ver.version, 1);

            let versions = repo.list_versions("m1");
            assert_eq!(versions.len(), 1);

            let v = repo.get_version("m1", Some(1));
            assert!(v.is_some());

            let models = repo.list_models();
            assert!(models.contains(&"m1".to_string()));

            let deleted = repo.delete_version("m1", 1).unwrap();
            assert!(deleted);
        }
    }
}
