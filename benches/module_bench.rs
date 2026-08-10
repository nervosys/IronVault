//! Benchmarks for v1.5.0 modules — quantization, evaluation, scheduler, multi-vault.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ironvault::{
    BackupFrequency, BackupManager, BackupSchedule, EvalRun, EvalStore, MetricResult, QuantMethod,
    QuantProfile, QuantProfileStore, VaultEntry, VaultRegistry,
};
use std::collections::BTreeMap;
use tempfile::tempdir;

// ── Quantization profile store ───────────────────────────────────────────────

fn make_profile(name: &str) -> QuantProfile {
    QuantProfile {
        name: name.to_string(),
        method: QuantMethod::Q4KM,
        description: Some("bench".into()),
        metadata: BTreeMap::new(),
    }
}

fn bench_quant_profile_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("quant_profile_store");

    group.bench_function("set_profile", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let store = QuantProfileStore::new(tmp.path()).unwrap();
                (tmp, store)
            },
            |(_tmp, mut store)| {
                store.set(black_box(make_profile("test"))).unwrap();
            },
        );
    });

    group.bench_function("list_profiles", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let mut store = QuantProfileStore::new(tmp.path()).unwrap();
                for i in 0..10 {
                    store.set(make_profile(&format!("profile-{i}"))).unwrap();
                }
                (tmp, store)
            },
            |(_tmp, store)| {
                black_box(store.list());
            },
        );
    });

    group.bench_function("estimate_size", |b| {
        b.iter(|| {
            black_box(ironvault::quantization::estimate_quantized_size(
                1_000_000_000,
                QuantMethod::F32,
                QuantMethod::Q4KM,
            ))
        });
    });

    group.finish();
}

// ── Evaluation store ─────────────────────────────────────────────────────────

fn make_metric(name: &str, value: f64) -> MetricResult {
    MetricResult {
        name: name.into(),
        value,
        unit: "score".into(),
        higher_is_better: true,
    }
}

fn make_run(model: &str, version: u64, suite: &str, metrics: Vec<MetricResult>) -> EvalRun {
    EvalRun {
        suite: suite.into(),
        model: model.into(),
        version,
        metrics,
        timestamp: "2025-01-01T00:00:00Z".into(),
        context: BTreeMap::new(),
    }
}

fn bench_eval_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_store");

    group.bench_function("record_run", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let store = EvalStore::new(tmp.path()).unwrap();
                (tmp, store)
            },
            |(_tmp, mut store)| {
                let metrics = vec![make_metric("accuracy", 0.85), make_metric("f1", 0.82)];
                store
                    .record(black_box(make_run("model", 1, "mmlu", metrics)))
                    .unwrap();
            },
        );
    });

    group.bench_function("get_runs", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let mut store = EvalStore::new(tmp.path()).unwrap();
                for i in 0..20 {
                    let metrics = vec![make_metric("accuracy", 0.8 + (i as f64) * 0.005)];
                    store
                        .record(make_run("model", i % 5, "mmlu", metrics))
                        .unwrap();
                }
                (tmp, store)
            },
            |(_tmp, store)| {
                black_box(store.get_runs("model", None));
            },
        );
    });

    group.bench_function("get_suite_runs", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let mut store = EvalStore::new(tmp.path()).unwrap();
                for suite in &["mmlu", "hellaswag", "arc", "winogrande", "truthfulqa"] {
                    let metrics = vec![make_metric("accuracy", 0.85)];
                    store.record(make_run("model", 1, suite, metrics)).unwrap();
                }
                (tmp, store)
            },
            |(_tmp, store)| {
                black_box(store.get_suite_runs("model", "mmlu"));
            },
        );
    });

    group.finish();
}

// ── Backup manager ───────────────────────────────────────────────────────────

fn make_schedule(name: &str, out: std::path::PathBuf) -> BackupSchedule {
    BackupSchedule {
        name: name.into(),
        frequency: BackupFrequency::Daily,
        max_backups: 7,
        output_dir: out,
        enabled: true,
        created_at: "2025-01-01T00:00:00Z".into(),
    }
}

fn bench_backup_manager(c: &mut Criterion) {
    let mut group = c.benchmark_group("backup_manager");

    group.bench_function("set_schedule", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let out = tempdir().unwrap();
                let mgr = BackupManager::new(tmp.path()).unwrap();
                (tmp, out, mgr)
            },
            |(_tmp, out, mut mgr)| {
                mgr.set_schedule(make_schedule("nightly", out.path().to_path_buf()))
                    .unwrap();
            },
        );
    });

    group.bench_function("list_schedules", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let out = tempdir().unwrap();
                let mut mgr = BackupManager::new(tmp.path()).unwrap();
                for i in 0..5 {
                    mgr.set_schedule(make_schedule(
                        &format!("sched-{i}"),
                        out.path().to_path_buf(),
                    ))
                    .unwrap();
                }
                (tmp, out, mgr)
            },
            |(_tmp, _out, mgr)| {
                black_box(mgr.list_schedules());
            },
        );
    });

    group.finish();
}

// ── Vault registry ───────────────────────────────────────────────────────────

fn make_entry(name: &str) -> VaultEntry {
    VaultEntry {
        name: name.into(),
        path: format!("/data/{name}").into(),
        description: Some("bench".into()),
        registered_at: "2025-01-01T00:00:00Z".into(),
    }
}

fn bench_vault_registry(c: &mut Criterion) {
    let mut group = c.benchmark_group("vault_registry");

    group.bench_function("register", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let reg = VaultRegistry::new(tmp.path()).unwrap();
                (tmp, reg)
            },
            |(_tmp, mut reg)| {
                reg.register(black_box(make_entry("vault1"))).unwrap();
            },
        );
    });

    group.bench_function("list_10_vaults", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let mut reg = VaultRegistry::new(tmp.path()).unwrap();
                for i in 0..10 {
                    reg.register(make_entry(&format!("vault-{i}"))).unwrap();
                }
                (tmp, reg)
            },
            |(_tmp, reg)| {
                black_box(reg.list());
            },
        );
    });

    group.bench_function("activate_deactivate", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempdir().unwrap();
                let mut reg = VaultRegistry::new(tmp.path()).unwrap();
                reg.register(make_entry("vault1")).unwrap();
                (tmp, reg)
            },
            |(_tmp, mut reg)| {
                reg.activate(black_box("vault1")).unwrap();
                reg.deactivate().unwrap();
            },
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_quant_profile_store,
    bench_eval_store,
    bench_backup_manager,
    bench_vault_registry,
);
criterion_main!(benches);
