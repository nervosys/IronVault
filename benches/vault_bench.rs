//! Vault operation benchmarks — store, retrieve, format detection, model card serialization

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use ironvault::crypto::FipsCrypto;
use ironvault::formats::{ModelFormat, ModelMetadata};
use ironvault::model_card::{
    Evaluation, IntendedUse, Metric, ModelCard, ModelDetails, TrainingData,
};
use ironvault::{Vault, VaultConfig};
use tempfile::tempdir;

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
    let mut vault = Vault::new(Some(config)).unwrap();
    vault
        .unlock(b"bench_passphrase_with_entropy_01234".to_vec())
        .unwrap();
    (tmp, vault)
}

fn bench_store_and_retrieve(c: &mut Criterion) {
    let mut group = c.benchmark_group("vault_store_retrieve");

    for size in [1024, 10 * 1024, 100 * 1024] {
        let data = vec![0xABu8; size];

        group.bench_with_input(BenchmarkId::new("store", size), &data, |b, data| {
            b.iter_with_setup(
                || {
                    let (tmp, vault) = create_test_vault();
                    (tmp, vault, data.clone())
                },
                |(_tmp, mut vault, data)| {
                    let meta = ModelMetadata::new("bench_model".into(), ModelFormat::PyTorch);
                    vault
                        .store_model("bench_model", black_box(data), meta, None)
                        .unwrap();
                },
            );
        });

        group.bench_with_input(BenchmarkId::new("retrieve", size), &data, |b, data| {
            b.iter_with_setup(
                || {
                    let (tmp, mut vault) = create_test_vault();
                    let meta = ModelMetadata::new("bench_model".into(), ModelFormat::PyTorch);
                    vault
                        .store_model("bench_model", data.clone(), meta, None)
                        .unwrap();
                    (tmp, vault)
                },
                |(_tmp, vault)| {
                    black_box(vault.get_model("bench_model", None).unwrap());
                },
            );
        });
    }

    group.finish();
}

fn bench_format_detection(c: &mut Criterion) {
    let extensions = [
        "safetensors",
        "gguf",
        "pt",
        "onnx",
        "tflite",
        "mlmodel",
        "plan",
        "h5",
        "pkl",
        "npy",
        "unknown_ext",
    ];

    c.bench_function("format_from_extension", |b| {
        b.iter(|| {
            for ext in &extensions {
                black_box(ModelFormat::from_extension(ext));
            }
        });
    });

    c.bench_function("format_name", |b| {
        let formats = [
            ModelFormat::Safetensors,
            ModelFormat::GGUF,
            ModelFormat::PyTorch,
            ModelFormat::ONNX,
            ModelFormat::TFLite,
            ModelFormat::CoreML,
            ModelFormat::Custom("my_format".to_string()),
        ];
        b.iter(|| {
            for fmt in &formats {
                black_box(fmt.name());
            }
        });
    });
}

fn bench_sha256(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha256");

    for size in [1024, 10 * 1024, 100 * 1024, 1024 * 1024] {
        let data = vec![0x42u8; size];
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| black_box(FipsCrypto::hash_sha256(data)));
        });
    }

    group.finish();
}

fn make_full_model_card() -> ModelCard {
    ModelCard::new(
        ModelDetails {
            name: "BenchModel-7B".to_string(),
            version: "1.0.0".to_string(),
            description: "A benchmark model for testing serialization perf".to_string(),
            model_type: "Large Language Model".to_string(),
            architecture: "Transformer (GPT-style)".to_string(),
            size: "7B parameters".to_string(),
            framework: "PyTorch".to_string(),
            format: "safetensors".to_string(),
            license: Some("Apache-2.0".to_string()),
            developers: vec!["Org A".to_string(), "Org B".to_string()],
            repository: Some("https://github.com/example/model".to_string()),
            citation: Some("@article{bench2024, title={BenchModel}}".to_string()),
            contact: None,
            paper: None,
        },
        IntendedUse {
            primary_uses: vec!["Text generation".to_string(), "Code completion".to_string()],
            primary_users: vec!["Developers".to_string(), "Researchers".to_string()],
            out_of_scope_uses: vec!["Medical advice".to_string()],
            use_case_examples: None,
        },
    )
    .with_training_data(TrainingData {
        datasets: vec!["CommonCrawl".to_string(), "Wikipedia".to_string()],
        size: Some("1TB".to_string()),
        preprocessing: Some(vec!["Deduplication".to_string(), "Filtering".to_string()]),
        languages: Some(vec!["English".to_string(), "French".to_string()]),
        sources: None,
        collection_methods: None,
        splits: None,
        demographics: None,
    })
    .with_evaluation(Evaluation {
        datasets: vec!["eval_set".to_string()],
        metrics: vec![
            Metric {
                name: "Accuracy".to_string(),
                value: 0.925,
                description: Some("Top-1 accuracy on eval set".to_string()),
                threshold: None,
            },
            Metric {
                name: "F1".to_string(),
                value: 0.912,
                description: None,
                threshold: None,
            },
        ],
        benchmarks: Some(
            vec![("MMLU".to_string(), 0.72), ("HellaSwag".to_string(), 0.81)]
                .into_iter()
                .collect(),
        ),
        performance_by_group: None,
        methodology: None,
    })
}

fn bench_model_card_serialization(c: &mut Criterion) {
    let card = make_full_model_card();

    c.bench_function("model_card_to_json", |b| {
        b.iter(|| black_box(card.to_json().unwrap()));
    });

    c.bench_function("model_card_to_yaml", |b| {
        b.iter(|| black_box(card.to_yaml().unwrap()));
    });

    c.bench_function("model_card_to_markdown", |b| {
        b.iter(|| black_box(card.to_markdown()));
    });

    let json = card.to_json().unwrap();
    c.bench_function("model_card_from_json", |b| {
        b.iter(|| black_box(ModelCard::from_json(&json).unwrap()));
    });

    let yaml = card.to_yaml().unwrap();
    c.bench_function("model_card_from_yaml", |b| {
        b.iter(|| black_box(ModelCard::from_yaml(&yaml).unwrap()));
    });
}

criterion_group!(
    benches,
    bench_store_and_retrieve,
    bench_format_detection,
    bench_sha256,
    bench_model_card_serialization,
);
criterion_main!(benches);
