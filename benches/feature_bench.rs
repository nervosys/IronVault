//! Benchmarks for v1.3–v1.5 feature modules: tags, signing, scanning, diff,
//! lineage, access control, validation, policies, plugins, profiles, webhooks.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use ironvault::access_control::{AclGuard, Role};
use ironvault::diff::ModelDiffer;
use ironvault::license_scan::LicenseScanner;
use ironvault::lineage_graph::{DerivationKind, LineageEdge, LineageGraph};
use ironvault::plugins::{PluginManifest, PluginRegistry};
use ironvault::policies::{PolicyStore, RetentionPolicy};
use ironvault::profiles::{Profile, ProfileStore};
use ironvault::scanning::PickleScanner;
use ironvault::signing::ModelSigner;
use ironvault::tags::{SearchQuery, TagStore};
use ironvault::validation::ValidationStore;
use ironvault::webhooks::{WebhookStore, WebhookTarget};
use std::collections::BTreeMap;
use std::hint::black_box;
use tempfile::tempdir;

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ── Tags & Search ────────────────────────────────────────────────────────────

fn bench_tag_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("tags");

    group.bench_function("add_10_tags", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let store = TagStore::new(tmp.path()).unwrap();
                let tags: Vec<String> = (0..10).map(|i| format!("tag-{i}")).collect();
                (tmp, store, tags)
            },
            |(_tmp, mut store, tags)| {
                store.add_tags("model", &tags).unwrap();
            },
        );
    });

    group.bench_function("search_100_models", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let mut store = TagStore::new(tmp.path()).unwrap();
                let names: Vec<String> = (0..100).map(|i| format!("model-{i}")).collect();
                for (i, name) in names.iter().enumerate() {
                    store
                        .add_tags(name, &[format!("cat-{}", i % 5), "common".into()])
                        .unwrap();
                }
                let query = SearchQuery {
                    tags: vec!["common".into()],
                    annotations: vec![],
                    name_pattern: None,
                };
                (tmp, store, names, query)
            },
            |(_tmp, store, names, query)| {
                black_box(store.search(&query, &names));
            },
        );
    });

    group.bench_function("annotations_set_get", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let store = TagStore::new(tmp.path()).unwrap();
                (tmp, store)
            },
            |(_tmp, mut store)| {
                store.set_annotation("m", "framework", "pytorch").unwrap();
                black_box(store.get_annotations("m"));
            },
        );
    });

    group.finish();
}

// ── Access Control ───────────────────────────────────────────────────────────

fn bench_acl(c: &mut Criterion) {
    let mut group = c.benchmark_group("acl");

    group.bench_function("grant_and_require", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let guard = AclGuard::new(tmp.path()).unwrap();
                (tmp, guard)
            },
            |(_tmp, mut guard)| {
                guard.grant("user", Role::Writer).unwrap();
                guard.require("user", Role::Reader).unwrap();
            },
        );
    });

    group.bench_function("resolve_50_principals", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let mut guard = AclGuard::new(tmp.path()).unwrap();
                for i in 0..50 {
                    guard.grant(&format!("user-{i}"), Role::Writer).unwrap();
                }
                (tmp, guard)
            },
            |(_tmp, guard)| {
                for i in 0..50 {
                    black_box(guard.resolve(&format!("user-{i}")));
                }
            },
        );
    });

    group.finish();
}

// ── Lineage Graph ────────────────────────────────────────────────────────────

fn bench_lineage(c: &mut Criterion) {
    let mut group = c.benchmark_group("lineage");

    group.bench_function("build_chain_20", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let graph = LineageGraph::new(tmp.path()).unwrap();
                (tmp, graph)
            },
            |(_tmp, mut graph)| {
                for i in 1..=20 {
                    graph
                        .add_edge(LineageEdge {
                            parents: vec![format!("model-{}", i - 1)],
                            child: format!("model-{i}"),
                            kind: DerivationKind::FineTune,
                            notes: BTreeMap::new(),
                            created_at: now_iso(),
                        })
                        .unwrap();
                }
            },
        );
    });

    group.bench_function("ancestors_depth_20", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let mut graph = LineageGraph::new(tmp.path()).unwrap();
                for i in 1..=20 {
                    graph
                        .add_edge(LineageEdge {
                            parents: vec![format!("model-{}", i - 1)],
                            child: format!("model-{i}"),
                            kind: DerivationKind::FineTune,
                            notes: BTreeMap::new(),
                            created_at: now_iso(),
                        })
                        .unwrap();
                }
                (tmp, graph)
            },
            |(_tmp, graph)| {
                black_box(graph.ancestors("model-20"));
            },
        );
    });

    group.finish();
}

// ── Plugins ──────────────────────────────────────────────────────────────────

fn bench_plugins(c: &mut Criterion) {
    let mut group = c.benchmark_group("plugins");

    group.bench_function("install_20_list", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let registry = PluginRegistry::new(tmp.path()).unwrap();
                (tmp, registry)
            },
            |(_tmp, mut reg)| {
                for i in 0..20 {
                    reg.install(PluginManifest {
                        id: format!("plugin-{i}"),
                        name: format!("Plugin {i}"),
                        version: "1.0.0".into(),
                        description: "Bench plugin".into(),
                        author: None,
                        min_aim_version: None,
                        capabilities: vec![],
                        entry_point: None,
                    })
                    .unwrap();
                }
                black_box(reg.list());
            },
        );
    });

    group.finish();
}

// ── Profiles ─────────────────────────────────────────────────────────────────

fn bench_profiles(c: &mut Criterion) {
    let mut group = c.benchmark_group("profiles");

    group.bench_function("set_activate_deactivate", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let store = ProfileStore::new(tmp.path()).unwrap();
                (tmp, store)
            },
            |(_tmp, mut store)| {
                store
                    .set(Profile {
                        name: "bench".into(),
                        description: None,
                        overrides: BTreeMap::from([("key".into(), "val".into())]),
                        created_at: now_iso(),
                    })
                    .unwrap();
                store.activate("bench").unwrap();
                black_box(store.active());
                store.deactivate().unwrap();
            },
        );
    });

    group.finish();
}

// ── Policies ─────────────────────────────────────────────────────────────────

fn bench_policies(c: &mut Criterion) {
    let mut group = c.benchmark_group("policies");

    group.bench_function("set_get_50", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let store = PolicyStore::new(tmp.path()).unwrap();
                (tmp, store)
            },
            |(_tmp, mut store)| {
                for i in 0..50 {
                    store
                        .set(
                            &format!("model-{i}"),
                            RetentionPolicy {
                                max_versions: 10,
                                max_age_days: 90,
                                keep_minimum: 2,
                            },
                        )
                        .unwrap();
                }
                for i in 0..50 {
                    black_box(store.get(&format!("model-{i}")));
                }
            },
        );
    });

    group.finish();
}

// ── Validation ───────────────────────────────────────────────────────────────

fn bench_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation");

    for size in [1024, 10 * 1024, 100 * 1024] {
        group.bench_with_input(BenchmarkId::new("create_probe", size), &size, |b, &size| {
            b.iter_with_setup(
                || {
                    let tmp = tempdir().unwrap();
                    let store = ValidationStore::new(tmp.path()).unwrap();
                    let model_file = tmp.path().join("model.bin");
                    std::fs::write(&model_file, vec![0xABu8; size]).unwrap();
                    (tmp, store, model_file)
                },
                |(_, store, model_file)| {
                    store.create_integrity_probe("m", &model_file).unwrap();
                },
            );
        });
    }

    group.bench_function("validate_100kb", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let store = ValidationStore::new(tmp.path()).unwrap();
                let model_file = tmp.path().join("model.bin");
                std::fs::write(&model_file, vec![0xABu8; 100 * 1024]).unwrap();
                store.create_integrity_probe("m", &model_file).unwrap();
                (tmp, store, model_file)
            },
            |(_, store, model_file)| {
                black_box(store.validate("m", &model_file).unwrap());
            },
        );
    });

    group.finish();
}

// ── Webhooks ─────────────────────────────────────────────────────────────────

fn bench_webhooks(c: &mut Criterion) {
    let mut group = c.benchmark_group("webhooks");

    group.bench_function("add_and_filter_20", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let store = WebhookStore::new(tmp.path()).unwrap();
                (tmp, store)
            },
            |(_tmp, mut store)| {
                for i in 0..20 {
                    store
                        .add(WebhookTarget {
                            id: format!("wh-{i}"),
                            url: format!("https://example.com/{i}"),
                            secret: None,
                            events: vec!["model.stored".into()],
                            enabled: true,
                        })
                        .unwrap();
                }
                black_box(store.targets_for_event("model.stored"));
            },
        );
    });

    group.finish();
}

// ── Signing ──────────────────────────────────────────────────────────────────

fn bench_signing(c: &mut Criterion) {
    let mut group = c.benchmark_group("signing");

    for size in [1024, 10 * 1024, 100 * 1024] {
        group.bench_with_input(BenchmarkId::new("sign", size), &size, |b, &size| {
            b.iter_with_setup(
                || {
                    let tmp = tempdir().unwrap();
                    let keypair = ModelSigner::generate_keypair(None).unwrap();
                    let model_file = tmp.path().join("model.bin");
                    std::fs::write(&model_file, vec![0xABu8; size]).unwrap();
                    (tmp, keypair, model_file)
                },
                |(_tmp, keypair, model_file)| {
                    black_box(
                        ModelSigner::sign(&keypair, &model_file, std::collections::HashMap::new())
                            .unwrap(),
                    );
                },
            );
        });
    }

    group.bench_function("verify_100kb", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let keypair = ModelSigner::generate_keypair(None).unwrap();
                let model_file = tmp.path().join("model.bin");
                std::fs::write(&model_file, vec![0xABu8; 100 * 1024]).unwrap();
                let sig =
                    ModelSigner::sign(&keypair, &model_file, std::collections::HashMap::new())
                        .unwrap();
                (tmp, keypair, model_file, sig)
            },
            |(_tmp, keypair, model_file, sig)| {
                black_box(
                    ModelSigner::verify(&sig, &model_file, Some(&keypair.secret_seed)).unwrap(),
                );
            },
        );
    });

    group.finish();
}

// ── Scanning ─────────────────────────────────────────────────────────────────

fn bench_scanning(c: &mut Criterion) {
    let mut group = c.benchmark_group("scanning");

    for size in [1024, 10 * 1024, 100 * 1024] {
        group.bench_with_input(BenchmarkId::new("scan_bytes", size), &size, |b, &size| {
            let data = vec![0u8; size];
            b.iter(|| {
                black_box(PickleScanner::scan_bytes(&data, "bench.pkl"));
            });
        });
    }

    group.finish();
}

// ── Diff ─────────────────────────────────────────────────────────────────────

fn bench_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff");

    for size in [1024, 10 * 1024, 100 * 1024] {
        group.bench_with_input(BenchmarkId::new("diff_files", size), &size, |b, &size| {
            b.iter_with_setup(
                || {
                    let tmp = tempdir().unwrap();
                    let a = tmp.path().join("a.bin");
                    let b_file = tmp.path().join("b.bin");
                    std::fs::write(&a, vec![0xABu8; size]).unwrap();
                    std::fs::write(&b_file, vec![0xCDu8; size]).unwrap();
                    (tmp, a, b_file)
                },
                |(_tmp, a, b_file)| {
                    black_box(ModelDiffer::diff_files(&a, &b_file, "a", "b").unwrap());
                },
            );
        });
    }

    group.finish();
}

// ── License Scanning ─────────────────────────────────────────────────────────

fn bench_license_scan(c: &mut Criterion) {
    c.bench_function("license_scan_dir", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                std::fs::write(
                    tmp.path().join("LICENSE"),
                    "MIT License\n\nPermission is hereby granted...",
                )
                .unwrap();
                tmp
            },
            |tmp| {
                black_box(LicenseScanner::scan_directory(tmp.path()).unwrap());
            },
        );
    });
}

criterion_group!(
    benches,
    bench_tag_operations,
    bench_acl,
    bench_lineage,
    bench_plugins,
    bench_profiles,
    bench_policies,
    bench_validation,
    bench_webhooks,
    bench_signing,
    bench_scanning,
    bench_diff,
    bench_license_scan,
);
criterion_main!(benches);
