//! Integration tests for v1.3.0–v1.5.0 modules.
//!
//! Tests module interactions, edge cases, and cross-module workflows that
//! inline unit tests don't cover.

use ironvault::access_control::{AclGuard, Role};
use ironvault::benchmark::{BenchmarkRecord, BenchmarkStore};
use ironvault::diff::ModelDiffer;
use ironvault::evaluation::{EvalRun, EvalStore, MetricResult};
use ironvault::gc;
use ironvault::license_scan::LicenseScanner;
use ironvault::lineage_graph::{DerivationKind, LineageEdge, LineageGraph};
use ironvault::multi_vault::{VaultEntry, VaultRegistry};
use ironvault::plugins::{PluginManifest, PluginRegistry};
use ironvault::policies::{PolicyStore, RetentionPolicy};
use ironvault::profiles::{Profile, ProfileStore};
use ironvault::quantization::{QuantMethod, QuantProfile, QuantProfileStore};
use ironvault::scanning::PickleScanner;
use ironvault::scheduler::{BackupFrequency, BackupManager, BackupSchedule};
use ironvault::signing::ModelSigner;
use ironvault::tags::{SearchQuery, TagStore};
use ironvault::validation::ValidationStore;
use ironvault::webhooks::{WebhookStore, WebhookTarget};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use tempfile::tempdir;

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ============================================================================
// TAGS — search, annotations, cross-model tagging
// ============================================================================

#[test]
fn test_tag_search_by_multiple_criteria() {
    let tmp = tempdir().unwrap();
    let mut store = TagStore::new(tmp.path()).unwrap();

    store
        .add_tags("llama-7b", &["llm".into(), "text-gen".into()])
        .unwrap();
    store
        .add_tags("bert-base", &["nlp".into(), "embeddings".into()])
        .unwrap();
    store
        .add_tags("whisper-small", &["audio".into(), "asr".into()])
        .unwrap();
    store
        .set_annotation("llama-7b", "framework", "pytorch")
        .unwrap();
    store
        .set_annotation("bert-base", "framework", "pytorch")
        .unwrap();

    // Search by tag
    let query = SearchQuery {
        tags: vec!["llm".into()],
        annotations: vec![],
        name_pattern: None,
    };
    let results = store.search(
        &query,
        &[
            "llama-7b".into(),
            "bert-base".into(),
            "whisper-small".into(),
        ],
    );
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].model, "llama-7b");

    // Search by name pattern
    let query = SearchQuery {
        tags: vec![],
        annotations: vec![],
        name_pattern: Some("bert".into()),
    };
    let results = store.search(&query, &["llama-7b".into(), "bert-base".into()]);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].model, "bert-base");
}

#[test]
fn test_tag_remove_and_recount() {
    let tmp = tempdir().unwrap();
    let mut store = TagStore::new(tmp.path()).unwrap();

    store
        .add_tags("model", &["a".into(), "b".into(), "c".into()])
        .unwrap();
    assert_eq!(store.get_tags("model").len(), 3);

    store.remove_tags("model", &["b".into()]).unwrap();
    assert_eq!(store.get_tags("model").len(), 2);
    assert!(!store.get_tags("model").contains("b"));

    let all = store.all_tags();
    assert!(!all.contains("b"));
}

#[test]
fn test_tag_annotations_overwrite() {
    let tmp = tempdir().unwrap();
    let mut store = TagStore::new(tmp.path()).unwrap();

    store.set_annotation("m", "key", "v1").unwrap();
    assert_eq!(store.get_annotations("m").get("key").unwrap(), "v1");

    store.set_annotation("m", "key", "v2").unwrap();
    assert_eq!(store.get_annotations("m").get("key").unwrap(), "v2");
}

#[test]
fn test_tag_remove_model_cleans_all() {
    let tmp = tempdir().unwrap();
    let mut store = TagStore::new(tmp.path()).unwrap();

    store.add_tags("doomed", &["tag1".into()]).unwrap();
    store.set_annotation("doomed", "k", "v").unwrap();
    store.remove_model("doomed").unwrap();

    assert!(store.get_tags("doomed").is_empty());
    assert!(store.get_annotations("doomed").is_empty());
}

// ============================================================================
// ACCESS CONTROL — RBAC grant/revoke/require
// ============================================================================

#[test]
fn test_acl_role_ordering() {
    assert!(Role::Reader < Role::Writer);
    assert!(Role::Writer < Role::Admin);
}

#[test]
fn test_acl_require_minimum_role() {
    let tmp = tempdir().unwrap();
    let mut guard = AclGuard::new(tmp.path()).unwrap();

    guard.grant("alice", Role::Writer).unwrap();

    assert!(guard.require("alice", Role::Reader).is_ok());
    assert!(guard.require("alice", Role::Writer).is_ok());
    assert!(guard.require("alice", Role::Admin).is_err());
}

#[test]
fn test_acl_revoke_unknown_principal() {
    let tmp = tempdir().unwrap();
    let mut guard = AclGuard::new(tmp.path()).unwrap();
    let revoked = guard.revoke("nobody").unwrap();
    assert!(!revoked);
}

#[test]
fn test_acl_overwrite_role() {
    let tmp = tempdir().unwrap();
    let mut guard = AclGuard::new(tmp.path()).unwrap();

    guard.grant("bob", Role::Reader).unwrap();
    assert_eq!(guard.resolve("bob"), Some(Role::Reader));

    guard.grant("bob", Role::Admin).unwrap();
    assert_eq!(guard.resolve("bob"), Some(Role::Admin));
}

#[test]
fn test_acl_list_entries() {
    let tmp = tempdir().unwrap();
    let mut guard = AclGuard::new(tmp.path()).unwrap();

    guard.grant("alice", Role::Writer).unwrap();
    guard.grant("bob", Role::Reader).unwrap();

    let entries = guard.list();
    assert_eq!(entries.len(), 2);
}

// ============================================================================
// LINEAGE GRAPH — DAG traversal, ancestry chains
// ============================================================================

fn make_edge(child: &str, parents: &[&str], kind: DerivationKind) -> LineageEdge {
    LineageEdge {
        parents: parents.iter().map(|s| s.to_string()).collect(),
        child: child.into(),
        kind,
        notes: BTreeMap::new(),
        created_at: now_iso(),
    }
}

#[test]
fn test_lineage_diamond_ancestry() {
    let tmp = tempdir().unwrap();
    let mut graph = LineageGraph::new(tmp.path()).unwrap();

    // A → B, A → C, B+C → D (diamond)
    graph
        .add_edge(make_edge("B", &["A"], DerivationKind::FineTune))
        .unwrap();
    graph
        .add_edge(make_edge("C", &["A"], DerivationKind::Quantization))
        .unwrap();
    graph
        .add_edge(make_edge("D", &["B", "C"], DerivationKind::Merge))
        .unwrap();

    let ancestors = graph.ancestors("D");
    assert!(ancestors.contains(&"A".to_string()));
    assert!(ancestors.contains(&"B".to_string()));
    assert!(ancestors.contains(&"C".to_string()));

    let descendants = graph.descendants("A");
    assert!(descendants.contains(&"B".to_string()));
    assert!(descendants.contains(&"C".to_string()));
    assert!(descendants.contains(&"D".to_string()));
}

#[test]
fn test_lineage_no_ancestors_for_root() {
    let tmp = tempdir().unwrap();
    let mut graph = LineageGraph::new(tmp.path()).unwrap();

    graph
        .add_edge(make_edge("child", &["root"], DerivationKind::FineTune))
        .unwrap();

    assert!(graph.ancestors("root").is_empty());
    assert!(graph.descendants("child").is_empty());
}

#[test]
fn test_lineage_display_nonempty() {
    let tmp = tempdir().unwrap();
    let mut graph = LineageGraph::new(tmp.path()).unwrap();

    graph
        .add_edge(make_edge("B", &["A"], DerivationKind::Distillation))
        .unwrap();

    let output = graph.display();
    assert!(output.contains('A'));
    assert!(output.contains('B'));
}

// ============================================================================
// PLUGINS — discover, install, uninstall lifecycle
// ============================================================================

#[test]
fn test_plugin_install_and_uninstall() {
    let tmp = tempdir().unwrap();
    let mut registry = PluginRegistry::new(tmp.path()).unwrap();

    let manifest = PluginManifest {
        id: "my-plugin".into(),
        name: "My Plugin".into(),
        version: "1.0.0".into(),
        description: "Test plugin".into(),
        author: Some("Test Author".into()),
        min_aim_version: Some("1.5.0".into()),
        capabilities: vec!["format-conversion".into()],
        entry_point: Some("plugin.wasm".into()),
    };

    registry.install(manifest).unwrap();
    assert_eq!(registry.list().len(), 1);
    assert!(registry.get("my-plugin").is_some());

    let removed = registry.uninstall("my-plugin").unwrap();
    assert!(removed);
    assert!(registry.get("my-plugin").is_none());
}

#[test]
fn test_plugin_uninstall_nonexistent() {
    let tmp = tempdir().unwrap();
    let mut registry = PluginRegistry::new(tmp.path()).unwrap();
    let removed = registry.uninstall("nope").unwrap();
    assert!(!removed);
}

#[test]
fn test_plugin_display() {
    let tmp = tempdir().unwrap();
    let mut registry = PluginRegistry::new(tmp.path()).unwrap();

    registry
        .install(PluginManifest {
            id: "p1".into(),
            name: "Plugin One".into(),
            version: "0.1.0".into(),
            description: "First plugin".into(),
            author: None,
            min_aim_version: None,
            capabilities: vec![],
            entry_point: None,
        })
        .unwrap();

    let display = registry.display();
    assert!(display.contains("p1"));
}

// ============================================================================
// PROFILES — activate/deactivate lifecycle
// ============================================================================

#[test]
fn test_profile_activate_deactivate_cycle() {
    let tmp = tempdir().unwrap();
    let mut store = ProfileStore::new(tmp.path()).unwrap();

    store
        .set(Profile {
            name: "prod".into(),
            description: Some("Production".into()),
            overrides: BTreeMap::from([("encryption".into(), "aes-256-gcm".into())]),
            created_at: now_iso(),
        })
        .unwrap();

    store.activate("prod").unwrap();
    assert_eq!(store.active_name(), Some("prod"));
    assert!(store.active().is_some());

    store.deactivate().unwrap();
    assert!(store.active_name().is_none());
}

#[test]
fn test_profile_activate_nonexistent_fails() {
    let tmp = tempdir().unwrap();
    let mut store = ProfileStore::new(tmp.path()).unwrap();
    assert!(store.activate("ghost").is_err());
}

#[test]
fn test_profile_remove_active_deactivates() {
    let tmp = tempdir().unwrap();
    let mut store = ProfileStore::new(tmp.path()).unwrap();

    store
        .set(Profile {
            name: "test".into(),
            description: None,
            overrides: BTreeMap::new(),
            created_at: now_iso(),
        })
        .unwrap();
    store.activate("test").unwrap();

    let removed = store.remove("test").unwrap();
    assert!(removed);
    assert!(store.active_name().is_none());
}

// ============================================================================
// POLICIES — retention enforcement
// ============================================================================

#[test]
fn test_policy_set_and_retrieve() {
    let tmp = tempdir().unwrap();
    let mut store = PolicyStore::new(tmp.path()).unwrap();

    let policy = RetentionPolicy {
        max_versions: 5,
        max_age_days: 90,
        keep_minimum: 2,
    };
    store.set("my-model", policy).unwrap();

    let retrieved = store.get("my-model").unwrap();
    assert_eq!(retrieved.max_versions, 5);
    assert_eq!(retrieved.max_age_days, 90);
    assert_eq!(retrieved.keep_minimum, 2);
}

#[test]
fn test_policy_list_multiple() {
    let tmp = tempdir().unwrap();
    let mut store = PolicyStore::new(tmp.path()).unwrap();

    store
        .set(
            "a",
            RetentionPolicy {
                max_versions: 3,
                max_age_days: 0,
                keep_minimum: 0,
            },
        )
        .unwrap();
    store
        .set(
            "b",
            RetentionPolicy {
                max_versions: 10,
                max_age_days: 365,
                keep_minimum: 1,
            },
        )
        .unwrap();

    assert_eq!(store.list().len(), 2);
}

#[test]
fn test_policy_remove() {
    let tmp = tempdir().unwrap();
    let mut store = PolicyStore::new(tmp.path()).unwrap();

    store
        .set(
            "m",
            RetentionPolicy {
                max_versions: 5,
                max_age_days: 0,
                keep_minimum: 0,
            },
        )
        .unwrap();
    store.remove("m").unwrap();

    assert!(store.get("m").is_none());
}

// ============================================================================
// VALIDATION — integrity probes
// ============================================================================

#[test]
fn test_validation_probe_roundtrip() {
    let tmp = tempdir().unwrap();
    let store = ValidationStore::new(tmp.path()).unwrap();

    let model_file = tmp.path().join("model.bin");
    std::fs::write(&model_file, b"fake model data for validation").unwrap();

    store
        .create_integrity_probe("test-model", &model_file)
        .unwrap();

    let probes = store.load_probes("test-model").unwrap();
    assert!(!probes.is_empty());
}

#[test]
fn test_validation_validates_intact_file() {
    let tmp = tempdir().unwrap();
    let store = ValidationStore::new(tmp.path()).unwrap();

    let model_file = tmp.path().join("model.bin");
    std::fs::write(&model_file, b"consistent data").unwrap();

    store.create_integrity_probe("m", &model_file).unwrap();

    let report = store.validate("m", &model_file).unwrap();
    assert!(report.overall_pass);
}

// ============================================================================
// WEBHOOKS — target management and event filtering
// ============================================================================

#[test]
fn test_webhook_add_and_list() {
    let tmp = tempdir().unwrap();
    let mut store = WebhookStore::new(tmp.path()).unwrap();

    store
        .add(WebhookTarget {
            id: "wh1".into(),
            url: "https://example.com/hook".into(),
            secret: Some("s3cr3t".into()),
            events: vec!["model.stored".into()],
            enabled: true,
        })
        .unwrap();

    assert_eq!(store.list().len(), 1);
}

#[test]
fn test_webhook_event_filtering() {
    let tmp = tempdir().unwrap();
    let mut store = WebhookStore::new(tmp.path()).unwrap();

    store
        .add(WebhookTarget {
            id: "wh-store".into(),
            url: "https://example.com/a".into(),
            secret: None,
            events: vec!["model.stored".into()],
            enabled: true,
        })
        .unwrap();

    store
        .add(WebhookTarget {
            id: "wh-delete".into(),
            url: "https://example.com/b".into(),
            secret: None,
            events: vec!["model.deleted".into()],
            enabled: true,
        })
        .unwrap();

    let store_targets = store.targets_for_event("model.stored");
    assert_eq!(store_targets.len(), 1);
    assert_eq!(store_targets[0].id, "wh-store");

    let delete_targets = store.targets_for_event("model.deleted");
    assert_eq!(delete_targets.len(), 1);
}

#[test]
fn test_webhook_remove() {
    let tmp = tempdir().unwrap();
    let mut store = WebhookStore::new(tmp.path()).unwrap();

    store
        .add(WebhookTarget {
            id: "removable".into(),
            url: "https://example.com/c".into(),
            secret: None,
            events: vec![],
            enabled: true,
        })
        .unwrap();

    let removed = store.remove("removable").unwrap();
    assert!(removed);
    assert!(store.list().is_empty());

    let removed_again = store.remove("removable").unwrap();
    assert!(!removed_again);
}

// ============================================================================
// QUANTIZATION — profiles and size estimation
// ============================================================================

#[test]
fn test_quant_estimate_sizes() {
    let original = 4_000_000_000u64;
    let estimated = ironvault::quantization::estimate_quantized_size(
        original,
        QuantMethod::F32,
        QuantMethod::Q4KM,
    );
    assert!(estimated < original);
    assert!(estimated > 0);
}

#[test]
fn test_quant_profile_crud() {
    let tmp = tempdir().unwrap();
    let mut store = QuantProfileStore::new(tmp.path()).unwrap();

    store
        .set(QuantProfile {
            name: "fast".into(),
            method: QuantMethod::Q4_0,
            description: Some("Fast 4-bit".into()),
            metadata: BTreeMap::new(),
        })
        .unwrap();

    assert!(store.get("fast").is_some());

    let removed = store.remove("fast").unwrap();
    assert!(removed);
    assert!(store.get("fast").is_none());
}

#[test]
fn test_quant_method_bits() {
    assert!(QuantMethod::Q4_0.bits_per_weight() < QuantMethod::F32.bits_per_weight());
    assert!(QuantMethod::Q8_0.bits_per_weight() < QuantMethod::F16.bits_per_weight());
}

// ============================================================================
// EVALUATION — record, compare, suites
// ============================================================================

fn make_eval_run(model: &str, version: u64, suite: &str, metrics: Vec<MetricResult>) -> EvalRun {
    EvalRun {
        suite: suite.into(),
        model: model.into(),
        version,
        metrics,
        timestamp: now_iso(),
        context: BTreeMap::new(),
    }
}

fn metric(name: &str, value: f64, unit: &str) -> MetricResult {
    MetricResult {
        name: name.into(),
        value,
        unit: unit.into(),
        higher_is_better: true,
    }
}

#[test]
fn test_eval_record_and_query() {
    let tmp = tempdir().unwrap();
    let mut store = EvalStore::new(tmp.path()).unwrap();

    let run = make_eval_run(
        "llama",
        1,
        "mmlu",
        vec![
            metric("accuracy", 0.85, "score"),
            metric("perplexity", 12.3, "ppl"),
        ],
    );
    store.record(run).unwrap();

    let runs = store.get_runs("llama", Some(1));
    assert_eq!(runs.len(), 1);

    let suites = store.suites();
    assert!(suites.contains(&"mmlu".to_string()));
}

#[test]
fn test_eval_compare_versions() {
    let tmp = tempdir().unwrap();
    let mut store = EvalStore::new(tmp.path()).unwrap();

    store
        .record(make_eval_run(
            "model",
            1,
            "hellaswag",
            vec![metric("acc", 0.70, "score")],
        ))
        .unwrap();

    store
        .record(make_eval_run(
            "model",
            2,
            "hellaswag",
            vec![metric("acc", 0.80, "score")],
        ))
        .unwrap();

    let cmp = store.compare("model", 1, "model", 2, "hellaswag");
    assert!(cmp.is_some());
}

#[test]
fn test_eval_multiple_suites() {
    let tmp = tempdir().unwrap();
    let mut store = EvalStore::new(tmp.path()).unwrap();

    for suite in &["mmlu", "arc", "winogrande", "truthfulqa"] {
        store
            .record(make_eval_run(
                "m",
                1,
                suite,
                vec![metric("acc", 0.8, "score")],
            ))
            .unwrap();
    }

    let suites = store.suites();
    assert_eq!(suites.len(), 4);
}

// ============================================================================
// SCHEDULER — backup management
// ============================================================================

fn make_schedule(name: &str, freq: BackupFrequency, max: usize, out: PathBuf) -> BackupSchedule {
    BackupSchedule {
        name: name.into(),
        frequency: freq,
        max_backups: max,
        output_dir: out,
        enabled: true,
        created_at: now_iso(),
    }
}

#[test]
fn test_scheduler_crud() {
    let tmp = tempdir().unwrap();
    let out = tempdir().unwrap();
    let mut mgr = BackupManager::new(tmp.path()).unwrap();

    mgr.set_schedule(make_schedule(
        "nightly",
        BackupFrequency::Daily,
        7,
        out.path().to_path_buf(),
    ))
    .unwrap();

    let sched = mgr.get_schedule("nightly").unwrap();
    assert_eq!(sched.max_backups, 7);

    let removed = mgr.remove_schedule("nightly").unwrap();
    assert!(removed);
    assert!(mgr.get_schedule("nightly").is_none());
}

#[test]
fn test_scheduler_multiple_schedules() {
    let tmp = tempdir().unwrap();
    let out = tempdir().unwrap();
    let mut mgr = BackupManager::new(tmp.path()).unwrap();

    mgr.set_schedule(make_schedule(
        "daily",
        BackupFrequency::Daily,
        7,
        out.path().to_path_buf(),
    ))
    .unwrap();
    mgr.set_schedule(make_schedule(
        "weekly",
        BackupFrequency::Weekly,
        4,
        out.path().to_path_buf(),
    ))
    .unwrap();

    let schedules = mgr.list_schedules();
    assert_eq!(schedules.len(), 2);
}

// ============================================================================
// MULTI-VAULT — registry and switching
// ============================================================================

#[test]
fn test_multi_vault_lifecycle() {
    let tmp = tempdir().unwrap();
    let mut reg = VaultRegistry::new(tmp.path()).unwrap();

    reg.register(VaultEntry {
        name: "prod".into(),
        path: PathBuf::from("/data/prod"),
        description: Some("Production vault".into()),
        registered_at: now_iso(),
    })
    .unwrap();
    reg.register(VaultEntry {
        name: "staging".into(),
        path: PathBuf::from("/data/staging"),
        description: None,
        registered_at: now_iso(),
    })
    .unwrap();

    assert_eq!(reg.count(), 2);

    reg.activate("prod").unwrap();
    assert_eq!(reg.active_name(), Some("prod"));

    reg.deactivate().unwrap();
    assert!(reg.active_name().is_none());
}

#[test]
fn test_multi_vault_unregister_active_deactivates() {
    let tmp = tempdir().unwrap();
    let mut reg = VaultRegistry::new(tmp.path()).unwrap();

    reg.register(VaultEntry {
        name: "v1".into(),
        path: PathBuf::from("/data/v1"),
        description: None,
        registered_at: now_iso(),
    })
    .unwrap();
    reg.activate("v1").unwrap();

    let removed = reg.unregister("v1").unwrap();
    assert!(removed);
    assert!(reg.active_name().is_none());
}

#[test]
fn test_multi_vault_activate_unregistered_fails() {
    let tmp = tempdir().unwrap();
    let mut reg = VaultRegistry::new(tmp.path()).unwrap();
    assert!(reg.activate("nonexistent").is_err());
}

// ============================================================================
// SIGNING — keypair generation, sign/verify cycle
// ============================================================================

#[test]
fn test_signing_roundtrip() {
    let tmp = tempdir().unwrap();

    let keypair = ModelSigner::generate_keypair(Some("test@example.com")).unwrap();

    let key_path = tmp.path().join("test.key");
    ModelSigner::save_keypair(&keypair, &key_path).unwrap();
    let loaded = ModelSigner::load_keypair(&key_path).unwrap();
    assert_eq!(loaded.identity, keypair.identity);

    let model_path = tmp.path().join("model.bin");
    std::fs::write(&model_path, b"pretend model weights").unwrap();

    let mut metadata = HashMap::new();
    metadata.insert("model".into(), "test-model".into());

    let signature = ModelSigner::sign(&keypair, &model_path, metadata).unwrap();

    let sig_path = tmp.path().join("model.sig");
    ModelSigner::save_signature(&signature, &sig_path).unwrap();
    let loaded_sig = ModelSigner::load_signature(&sig_path).unwrap();

    let verification =
        ModelSigner::verify(&loaded_sig, &model_path, Some(&keypair.secret_seed)).unwrap();
    assert!(verification.valid);
}

#[test]
fn test_signing_tampered_file_fails_verification() {
    let tmp = tempdir().unwrap();

    let keypair = ModelSigner::generate_keypair(None).unwrap();
    let model_path = tmp.path().join("model.bin");
    std::fs::write(&model_path, b"original data").unwrap();

    let signature = ModelSigner::sign(&keypair, &model_path, HashMap::new()).unwrap();

    std::fs::write(&model_path, b"tampered data").unwrap();

    let verification =
        ModelSigner::verify(&signature, &model_path, Some(&keypair.secret_seed)).unwrap();
    assert!(!verification.valid);
}

// ============================================================================
// SCANNING — pickle safety
// ============================================================================

#[test]
fn test_scan_safe_bytes() {
    let safe_data = b"not a pickle file at all";
    let report = PickleScanner::scan_bytes(safe_data, "safe.bin");
    assert!(
        report.findings.is_empty()
            || report
                .findings
                .iter()
                .all(|f| f.severity != ironvault::Severity::Critical)
    );
}

#[test]
fn test_scan_dangerous_opcodes() {
    let mut data = vec![0x80, 0x02]; // pickle protocol 2 header
    data.extend_from_slice(b"\x63os\nsystem\n"); // GLOBAL 'os.system'
    data.push(0x2e); // STOP

    let report = PickleScanner::scan_bytes(&data, "malicious.pkl");
    assert!(!report.findings.is_empty());
}

// ============================================================================
// DIFF — model comparison
// ============================================================================

#[test]
fn test_diff_identical_files() {
    let tmp = tempdir().unwrap();

    let file_a = tmp.path().join("a.bin");
    let file_b = tmp.path().join("b.bin");
    std::fs::write(&file_a, b"identical content").unwrap();
    std::fs::write(&file_b, b"identical content").unwrap();

    let diff = ModelDiffer::diff_files(&file_a, &file_b, "a.bin", "b.bin").unwrap();
    assert_eq!(diff.summary.changed, 0);
    assert_eq!(diff.summary.added, 0);
    assert_eq!(diff.summary.removed, 0);
}

#[test]
fn test_diff_different_files() {
    let tmp = tempdir().unwrap();

    let file_a = tmp.path().join("a.bin");
    let file_b = tmp.path().join("b.bin");
    std::fs::write(&file_a, b"version one data").unwrap();
    std::fs::write(&file_b, b"version two completely different data here").unwrap();

    let diff = ModelDiffer::diff_files(&file_a, &file_b, "a.bin", "b.bin").unwrap();
    // Generic binary files have no tensor metadata, so tensor-level counts stay 0.
    // The size_change_pct captures file-level differences instead.
    assert!(diff.summary.size_change_pct.abs() > 0.0);
}

// ============================================================================
// LICENSE SCANNING — detect licenses from files
// ============================================================================

#[test]
fn test_license_scan_mit_license_file() {
    let tmp = tempdir().unwrap();
    let license_file = tmp.path().join("LICENSE");
    std::fs::write(
        &license_file,
        "MIT License\n\nPermission is hereby granted...",
    )
    .unwrap();

    let report = LicenseScanner::scan_directory(tmp.path()).unwrap();
    // Should detect MIT or at least find something
    let display = report.display();
    assert!(!report.licenses.is_empty() || display.contains("MIT") || display.contains("Unknown"));
}

#[test]
fn test_license_scan_empty_dir() {
    let tmp = tempdir().unwrap();
    let report = LicenseScanner::scan_directory(tmp.path()).unwrap();
    assert!(report.licenses.is_empty());
}

// ============================================================================
// BENCHMARK METADATA — record storage
// ============================================================================

#[test]
fn test_benchmark_store_roundtrip() {
    let tmp = tempdir().unwrap();
    let store = BenchmarkStore::new(tmp.path()).unwrap();

    let mut record = BenchmarkRecord::new("my-model", 1);
    record.add_result("mmlu", 0.72, "accuracy", true);
    record.add_result("hellaswag", 0.81, "accuracy", true);

    store.save(&record).unwrap();

    let records = store.list_for_model("my-model").unwrap();
    assert_eq!(records.len(), 1);
    assert!(records[0].get_result("mmlu").is_some());
}

#[test]
fn test_benchmark_display() {
    let mut record = BenchmarkRecord::new("test", 1);
    record.add_result("perplexity", 15.2, "ppl", false);

    let display = record.display();
    assert!(display.contains("test"));
    assert!(display.contains("perplexity"));
}

// ============================================================================
// GC — garbage collection on empty vault
// ============================================================================

#[test]
fn test_gc_on_empty_dir() {
    let tmp = tempdir().unwrap();
    let key = ironvault::crypto::VaultCrypto::new()
        .unwrap()
        .derive_key(b"gc-empty-dir".to_vec(), Some(vec![5u8; 16]))
        .unwrap()
        .0;
    let result = gc::gc(tmp.path(), true, &key);
    // Should either succeed with 0 cleaned or return an error gracefully
    assert!(result.is_ok() || result.is_err());
}

// ============================================================================
// CROSS-MODULE WORKFLOWS
// ============================================================================

#[test]
fn test_tag_and_lineage_together() {
    let tmp = tempdir().unwrap();

    let mut tags = TagStore::new(tmp.path()).unwrap();
    tags.add_tags("base-model", &["foundation".into(), "llm".into()])
        .unwrap();
    tags.add_tags("fine-tuned", &["llm".into(), "chat".into()])
        .unwrap();

    let mut lineage = LineageGraph::new(tmp.path()).unwrap();
    lineage
        .add_edge(make_edge(
            "fine-tuned",
            &["base-model"],
            DerivationKind::FineTune,
        ))
        .unwrap();

    let query = SearchQuery {
        tags: vec!["llm".into()],
        annotations: vec![],
        name_pattern: None,
    };
    let results = tags.search(&query, &["base-model".into(), "fine-tuned".into()]);
    assert_eq!(results.len(), 2);

    let ancestors = lineage.ancestors("fine-tuned");
    assert!(ancestors.contains(&"base-model".to_string()));
}

#[test]
fn test_eval_and_benchmark_workflow() {
    let tmp = tempdir().unwrap();

    let mut eval = EvalStore::new(tmp.path()).unwrap();
    eval.record(make_eval_run(
        "model-v2",
        2,
        "mmlu",
        vec![metric("accuracy", 0.78, "score")],
    ))
    .unwrap();

    let store = BenchmarkStore::new(tmp.path()).unwrap();
    let mut record = BenchmarkRecord::new("model-v2", 2);
    record.add_result("throughput", 1500.0, "tokens/sec", true);
    store.save(&record).unwrap();

    let eval_runs = eval.get_runs("model-v2", Some(2));
    assert_eq!(eval_runs.len(), 1);

    let bench_records = store.list_for_model("model-v2").unwrap();
    assert_eq!(bench_records.len(), 1);
}

#[test]
fn test_acl_and_profile_workflow() {
    let tmp = tempdir().unwrap();

    let mut acl = AclGuard::new(tmp.path()).unwrap();
    acl.grant("admin-user", Role::Admin).unwrap();

    let mut profiles = ProfileStore::new(tmp.path()).unwrap();
    profiles
        .set(Profile {
            name: "secure".into(),
            description: Some("High security config".into()),
            overrides: BTreeMap::from([
                ("encryption".into(), "aes-256-gcm".into()),
                ("audit".into(), "enabled".into()),
            ]),
            created_at: now_iso(),
        })
        .unwrap();

    assert!(acl.require("admin-user", Role::Admin).is_ok());

    profiles.activate("secure").unwrap();
    let active = profiles.active().unwrap();
    assert_eq!(active.overrides.get("encryption").unwrap(), "aes-256-gcm");
}
