//! Integration tests for VaultBuilder, EventBus, and Metrics wiring.
//!
//! These tests validate that Architecture v2 components (VaultBuilder,
//! EventBus subscribers, MetricsSubscriber, AuditLogSubscriber, streaming)
//! work correctly when integrated through the live vault pipeline.

use ironvault::formats::{ModelFormat, ModelMetadata};
use ironvault::{EventBus, EventSubscriber, VaultBuilder, VaultConfig, VaultEvent};
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

// ──────────────────────────────────────────────────────────────
// VaultBuilder Construction Tests
// ──────────────────────────────────────────────────────────────

#[test]
fn test_vault_builder_default_backend() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let vault = VaultBuilder::new().config(config).build().unwrap();
    assert_eq!(vault.version_backend_name(), "json");
}

#[test]
fn test_vault_builder_with_config() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();
    config.vault.default_vault = "custom-vault".to_string();

    let vault = VaultBuilder::new().config(config).build().unwrap();
    assert_eq!(vault.version_backend_name(), "json");
}

#[test]
fn test_vault_builder_sqlite_backend() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let vault = VaultBuilder::new()
        .config(config)
        .sqlite_versions()
        .build()
        .unwrap();
    assert_eq!(vault.version_backend_name(), "sqlite");
}

#[test]
fn test_vault_builder_default_has_metrics() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let vault = VaultBuilder::new().config(config).build().unwrap();

    // Default builder wires MetricsSubscriber, so metrics() should return Some
    let snapshot = vault.metrics();
    assert!(snapshot.is_some());
    let m = snapshot.unwrap();
    assert_eq!(m.models_stored_total, 0);
    assert_eq!(m.models_retrieved_total, 0);
    assert_eq!(m.models_deleted_total, 0);
    assert_eq!(m.bytes_stored_total, 0);
    assert_eq!(m.errors_total, 0);
    assert!(!m.vault_unlocked);
}

#[test]
fn test_vault_new_has_no_metrics() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let vault = ironvault::Vault::new(Some(config)).unwrap();
    // Vault::new() doesn't wire MetricsSubscriber, so metrics is None
    assert!(vault.metrics().is_none());
}

#[test]
fn test_vault_builder_no_default_subscribers() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let vault = VaultBuilder::new()
        .config(config)
        .no_default_subscribers()
        .build()
        .unwrap();

    // No default subscribers means no MetricsSubscriber, so metrics is None
    assert!(vault.metrics().is_none());
    // And the event bus has 0 subscribers
    assert_eq!(vault.event_bus().subscriber_count(), 0);
}

#[test]
fn test_vault_builder_default_subscriber_count() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();
    config.security.audit_log = true;

    let vault = VaultBuilder::new().config(config).build().unwrap();

    // With audit_log enabled: AuditLogSubscriber + MetricsSubscriber = 2
    assert_eq!(vault.event_bus().subscriber_count(), 2);
}

#[test]
fn test_vault_builder_default_subscriber_count_no_audit() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();
    config.security.audit_log = false;

    let vault = VaultBuilder::new().config(config).build().unwrap();

    // Without audit_log: only MetricsSubscriber = 1
    assert_eq!(vault.event_bus().subscriber_count(), 1);
}

// ──────────────────────────────────────────────────────────────
// Custom Subscriber Tests
// ──────────────────────────────────────────────────────────────

/// A test subscriber that records all events it receives.
struct RecordingSubscriber {
    events: Arc<Mutex<Vec<String>>>,
}

impl RecordingSubscriber {
    fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                events: events.clone(),
            },
            events,
        )
    }
}

impl EventSubscriber for RecordingSubscriber {
    fn on_event(&self, event: &VaultEvent) -> ironvault::Result<()> {
        let name = match event {
            VaultEvent::VaultCreated { .. } => "VaultCreated",
            VaultEvent::VaultUnlocked { .. } => "VaultUnlocked",
            VaultEvent::VaultLocked { .. } => "VaultLocked",
            VaultEvent::ModelStored { .. } => "ModelStored",
            VaultEvent::ModelRetrieved { .. } => "ModelRetrieved",
            VaultEvent::ModelDeleted { .. } => "ModelDeleted",
            VaultEvent::PassphraseChanged { .. } => "PassphraseChanged",
            VaultEvent::IntegrityFailed { .. } => "IntegrityFailed",
            VaultEvent::ComplianceChecked { .. } => "ComplianceChecked",
        };
        self.events.lock().unwrap().push(name.to_string());
        Ok(())
    }

    fn name(&self) -> &str {
        "RecordingSubscriber"
    }
}

#[test]
fn test_vault_builder_custom_subscriber() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let (subscriber, _events) = RecordingSubscriber::new();

    let vault = VaultBuilder::new()
        .config(config)
        .no_default_subscribers()
        .subscriber(Box::new(subscriber))
        .build()
        .unwrap();

    assert_eq!(vault.event_bus().subscriber_count(), 1);
}

#[test]
fn test_vault_builder_custom_plus_defaults() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();
    config.security.audit_log = true;

    let (subscriber, _events) = RecordingSubscriber::new();

    let vault = VaultBuilder::new()
        .config(config)
        .subscriber(Box::new(subscriber))
        .build()
        .unwrap();

    // AuditLogSubscriber + MetricsSubscriber + RecordingSubscriber = 3
    assert_eq!(vault.event_bus().subscriber_count(), 3);
}

// ──────────────────────────────────────────────────────────────
// EventBus Unit Tests (standalone, no vault needed)
// ──────────────────────────────────────────────────────────────

#[test]
fn test_event_bus_dispatch() {
    let mut bus = EventBus::new();
    let (subscriber, events) = RecordingSubscriber::new();
    bus.subscribe(Box::new(subscriber));

    let event = VaultEvent::VaultCreated {
        vault: "test-vault".to_string(),
        timestamp: chrono::Utc::now(),
    };
    bus.emit(&event);

    let recorded = events.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0], "VaultCreated");
}

#[test]
fn test_event_bus_multiple_subscribers() {
    let mut bus = EventBus::new();
    let (sub1, events1) = RecordingSubscriber::new();
    let (sub2, events2) = RecordingSubscriber::new();
    bus.subscribe(Box::new(sub1));
    bus.subscribe(Box::new(sub2));

    let event = VaultEvent::ModelStored {
        vault: "test-vault".to_string(),
        model: "llama".to_string(),
        version: 1,
        format: "safetensors".to_string(),
        size: 1024,
        checksum: "abc123".to_string(),
        timestamp: chrono::Utc::now(),
    };
    bus.emit(&event);

    assert_eq!(events1.lock().unwrap().len(), 1);
    assert_eq!(events2.lock().unwrap().len(), 1);
    assert_eq!(events1.lock().unwrap()[0], "ModelStored");
    assert_eq!(events2.lock().unwrap()[0], "ModelStored");
}

#[test]
fn test_event_bus_subscriber_count() {
    let mut bus = EventBus::new();
    assert_eq!(bus.subscriber_count(), 0);

    let (sub1, _) = RecordingSubscriber::new();
    bus.subscribe(Box::new(sub1));
    assert_eq!(bus.subscriber_count(), 1);

    let (sub2, _) = RecordingSubscriber::new();
    bus.subscribe(Box::new(sub2));
    assert_eq!(bus.subscriber_count(), 2);
}

// ──────────────────────────────────────────────────────────────
// Metrics Integration Tests
// ──────────────────────────────────────────────────────────────

#[test]
fn test_metrics_updated_on_store() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let mut vault = VaultBuilder::new().config(config).build().unwrap();
    vault.unlock(b"test_pass_12345678".to_vec()).unwrap();

    let data = vec![42u8; 100];
    let metadata = ModelMetadata::new("metrics-model".to_string(), ModelFormat::Safetensors);
    vault
        .store_model("metrics-model", data, metadata, None)
        .unwrap();

    let m = vault.metrics().expect("metrics should be Some");
    assert_eq!(m.models_stored_total, 1);
    assert!(m.bytes_stored_total > 0);
}

#[test]
fn test_metrics_updated_on_retrieve() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let mut vault = VaultBuilder::new().config(config).build().unwrap();
    vault.unlock(b"test_pass_12345678".to_vec()).unwrap();

    let data = vec![99u8; 50];
    let metadata = ModelMetadata::new("ret-model".to_string(), ModelFormat::ONNX);
    vault
        .store_model("ret-model", data.clone(), metadata, None)
        .unwrap();

    let _ = vault.get_model("ret-model", None).unwrap();

    let m = vault.metrics().expect("metrics should be Some");
    assert_eq!(m.models_stored_total, 1);
    assert_eq!(m.models_retrieved_total, 1);
}

#[test]
fn test_metrics_updated_on_delete() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let mut vault = VaultBuilder::new().config(config).build().unwrap();
    vault.unlock(b"test_pass_12345678".to_vec()).unwrap();

    let data = vec![7u8; 30];
    let metadata = ModelMetadata::new("del-model".to_string(), ModelFormat::PyTorch);
    vault
        .store_model("del-model", data, metadata, None)
        .unwrap();

    vault.delete_version("del-model", 1).unwrap();

    let m = vault.metrics().expect("metrics should be Some");
    assert_eq!(m.models_stored_total, 1);
    assert_eq!(m.models_deleted_total, 1);
}

#[test]
fn test_metrics_multiple_operations() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let mut vault = VaultBuilder::new().config(config).build().unwrap();
    vault.unlock(b"test_pass_12345678".to_vec()).unwrap();

    // Store 3 models
    for i in 0..3 {
        let name = format!("model-{}", i);
        let data = vec![i as u8; 100 + i * 50];
        let metadata = ModelMetadata::new(name.clone(), ModelFormat::Safetensors);
        vault.store_model(&name, data, metadata, None).unwrap();
    }

    // Retrieve 2 of them
    vault.get_model("model-0", None).unwrap();
    vault.get_model("model-1", None).unwrap();

    // Delete 1
    vault.delete_version("model-2", 1).unwrap();

    let m = vault.metrics().expect("metrics should be Some");
    assert_eq!(m.models_stored_total, 3);
    assert_eq!(m.models_retrieved_total, 2);
    assert_eq!(m.models_deleted_total, 1);
    assert!(m.bytes_stored_total > 0);
}

// ──────────────────────────────────────────────────────────────
// Event Emission Integration Tests
// ──────────────────────────────────────────────────────────────

#[test]
fn test_events_emitted_on_store() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let (subscriber, events) = RecordingSubscriber::new();

    let mut vault = VaultBuilder::new()
        .config(config)
        .no_default_subscribers()
        .subscriber(Box::new(subscriber))
        .build()
        .unwrap();

    vault.unlock(b"test_pass_12345678".to_vec()).unwrap();

    let data = vec![1u8; 64];
    let metadata = ModelMetadata::new("event-model".to_string(), ModelFormat::GGUF);
    vault
        .store_model("event-model", data, metadata, None)
        .unwrap();

    let recorded = events.lock().unwrap();
    assert!(
        recorded.contains(&"ModelStored".to_string()),
        "Expected ModelStored event, got: {:?}",
        recorded
    );
}

#[test]
fn test_events_emitted_on_retrieve() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let (subscriber, events) = RecordingSubscriber::new();

    let mut vault = VaultBuilder::new()
        .config(config)
        .no_default_subscribers()
        .subscriber(Box::new(subscriber))
        .build()
        .unwrap();

    vault.unlock(b"test_pass_12345678".to_vec()).unwrap();

    let data = vec![2u8; 32];
    let metadata = ModelMetadata::new("ev-ret".to_string(), ModelFormat::PyTorch);
    vault
        .store_model("ev-ret", data.clone(), metadata, None)
        .unwrap();
    let _ = vault.get_model("ev-ret", None).unwrap();

    let recorded = events.lock().unwrap();
    assert!(
        recorded.contains(&"ModelStored".to_string()),
        "Expected ModelStored event"
    );
    assert!(
        recorded.contains(&"ModelRetrieved".to_string()),
        "Expected ModelRetrieved event"
    );
}

#[test]
fn test_events_emitted_on_delete() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let (subscriber, events) = RecordingSubscriber::new();

    let mut vault = VaultBuilder::new()
        .config(config)
        .no_default_subscribers()
        .subscriber(Box::new(subscriber))
        .build()
        .unwrap();

    vault.unlock(b"test_pass_12345678".to_vec()).unwrap();

    let data = vec![3u8; 16];
    let metadata = ModelMetadata::new("ev-del".to_string(), ModelFormat::ONNX);
    vault.store_model("ev-del", data, metadata, None).unwrap();
    vault.delete_version("ev-del", 1).unwrap();

    let recorded = events.lock().unwrap();
    assert!(
        recorded.contains(&"ModelStored".to_string()),
        "Expected ModelStored event"
    );
    assert!(
        recorded.contains(&"ModelDeleted".to_string()),
        "Expected ModelDeleted event"
    );
}

// ──────────────────────────────────────────────────────────────
// Streaming API Tests
// ──────────────────────────────────────────────────────────────

#[test]
fn test_store_model_streamed() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let mut vault = VaultBuilder::new().config(config).build().unwrap();
    vault.unlock(b"test_pass_12345678".to_vec()).unwrap();

    // Stream model data in chunks
    let chunks: Vec<Vec<u8>> = vec![vec![10u8; 100], vec![20u8; 100], vec![30u8; 100]];
    let metadata = ModelMetadata::new("stream-model".to_string(), ModelFormat::Safetensors);

    let version = vault
        .store_model_streamed("stream-model", chunks, metadata, None)
        .unwrap();
    assert_eq!(version.version, 1);

    // Verify round-trip
    let retrieved = vault.get_model("stream-model", None).unwrap();
    let expected: Vec<u8> = [vec![10u8; 100], vec![20u8; 100], vec![30u8; 100]].concat();
    assert_eq!(retrieved, expected);
}

#[test]
fn test_get_model_chunked() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let mut vault = VaultBuilder::new().config(config).build().unwrap();
    vault.unlock(b"test_pass_12345678".to_vec()).unwrap();

    let data = vec![55u8; 500];
    let metadata = ModelMetadata::new("chunk-model".to_string(), ModelFormat::ONNX);
    vault
        .store_model("chunk-model", data.clone(), metadata, None)
        .unwrap();

    // Retrieve in 128-byte chunks
    let chunks: Vec<Vec<u8>> = vault
        .get_model_chunked("chunk-model", None, 128)
        .unwrap()
        .collect();

    let reassembled: Vec<u8> = chunks.into_iter().flatten().collect();
    assert_eq!(reassembled, data);
}

// ──────────────────────────────────────────────────────────────
// SQLite + JSON Backend Parity Tests
// ──────────────────────────────────────────────────────────────

#[test]
fn test_sqlite_backend_store_retrieve() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let mut vault = VaultBuilder::new()
        .config(config)
        .sqlite_versions()
        .build()
        .unwrap();

    vault.unlock(b"test_pass_12345678".to_vec()).unwrap();

    let data = vec![88u8; 200];
    let metadata = ModelMetadata::new("sqlite-model".to_string(), ModelFormat::Safetensors);
    let ver = vault
        .store_model("sqlite-model", data.clone(), metadata, None)
        .unwrap();
    assert_eq!(ver.version, 1);

    let retrieved = vault.get_model("sqlite-model", None).unwrap();
    assert_eq!(retrieved, data);
}

#[test]
fn test_sqlite_backend_versioning() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let mut vault = VaultBuilder::new()
        .config(config)
        .sqlite_versions()
        .build()
        .unwrap();

    vault.unlock(b"test_pass_12345678".to_vec()).unwrap();

    // Store two versions
    let meta1 = ModelMetadata::new("sv-model".to_string(), ModelFormat::PyTorch);
    let v1 = vault
        .store_model("sv-model", vec![1u8; 50], meta1, None)
        .unwrap();
    assert_eq!(v1.version, 1);

    let meta2 = ModelMetadata::new("sv-model".to_string(), ModelFormat::PyTorch);
    let v2 = vault
        .store_model("sv-model", vec![2u8; 50], meta2, None)
        .unwrap();
    assert_eq!(v2.version, 2);

    // Retrieve specific versions
    let r1 = vault.get_model("sv-model", Some(1)).unwrap();
    assert_eq!(r1, vec![1u8; 50]);

    let r2 = vault.get_model("sv-model", Some(2)).unwrap();
    assert_eq!(r2, vec![2u8; 50]);
}

#[test]
fn test_sqlite_backend_list_versions() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let mut vault = VaultBuilder::new()
        .config(config)
        .sqlite_versions()
        .build()
        .unwrap();

    vault.unlock(b"test_pass_12345678".to_vec()).unwrap();

    let meta = ModelMetadata::new("sv-list".to_string(), ModelFormat::ONNX);
    vault
        .store_model("sv-list", vec![1u8; 10], meta, None)
        .unwrap();
    let meta2 = ModelMetadata::new("sv-list".to_string(), ModelFormat::ONNX);
    vault
        .store_model("sv-list", vec![2u8; 10], meta2, None)
        .unwrap();

    let versions = vault.list_versions("sv-list");
    assert_eq!(versions.len(), 2);
}

#[test]
fn test_sqlite_backend_delete() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let mut vault = VaultBuilder::new()
        .config(config)
        .sqlite_versions()
        .build()
        .unwrap();

    vault.unlock(b"test_pass_12345678".to_vec()).unwrap();

    let meta = ModelMetadata::new("sv-del".to_string(), ModelFormat::GGUF);
    vault
        .store_model("sv-del", vec![1u8; 10], meta, None)
        .unwrap();

    vault.delete_version("sv-del", 1).unwrap();

    let versions = vault.list_versions("sv-del");
    assert_eq!(versions.len(), 0);
}

// ──────────────────────────────────────────────────────────────
// IvUri Tests
// ──────────────────────────────────────────────────────────────

#[test]
fn test_iv_uri_parse_roundtrip() {
    use ironvault::IvUri;

    let uri = IvUri::parse("iv://my-vault/llama-3@2").unwrap();
    assert_eq!(uri.vault, Some("my-vault".to_string()));
    assert_eq!(uri.model, Some("llama-3".to_string()));
    assert_eq!(uri.version, Some(2));
}

#[test]
fn test_iv_uri_no_version() {
    use ironvault::IvUri;

    let uri = IvUri::parse("iv://default/my-model").unwrap();
    assert_eq!(uri.vault, Some("default".to_string()));
    assert_eq!(uri.model, Some("my-model".to_string()));
    assert_eq!(uri.version, None);
}

#[test]
fn test_iv_uri_invalid() {
    use ironvault::IvUri;

    assert!(IvUri::parse("http://wrong").is_err());
    assert!(IvUri::parse("").is_err());
    // iv:// with no path segments is valid (root URI with all None fields)
    // assert!(IvUri::parse("iv://").is_err());
}

// ──────────────────────────────────────────────────────────────
// Audit Log Integration Tests
// ──────────────────────────────────────────────────────────────

#[test]
fn test_audit_log_written_via_subscriber() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();
    config.security.audit_log = true;

    let mut vault = VaultBuilder::new().config(config.clone()).build().unwrap();
    vault.unlock(b"test_pass_12345678".to_vec()).unwrap();

    let data = vec![5u8; 64];
    let metadata = ModelMetadata::new("audit-model".to_string(), ModelFormat::Safetensors);
    vault
        .store_model("audit-model", data, metadata, None)
        .unwrap();

    // The audit log file should exist and contain entries
    let log_path = config.get_audit_log_path();
    assert!(
        log_path.exists(),
        "Audit log should exist at {:?}",
        log_path
    );

    let contents = std::fs::read_to_string(&log_path).unwrap();
    assert!(
        contents.contains("audit-model"),
        "Audit log should mention the model name"
    );
}
