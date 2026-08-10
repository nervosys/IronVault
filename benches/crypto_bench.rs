//! Cryptography benchmarks

use ironvault::crypto::{
    compression::{compress, CompressionAlgorithm, CompressionLevel},
    FipsCrypto,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn bench_encryption(c: &mut Criterion) {
    let crypto = FipsCrypto::new().unwrap();
    let passphrase = b"benchmark_passphrase_12345".to_vec();
    let (key, _) = crypto.derive_key(passphrase, None).unwrap();

    let mut group = c.benchmark_group("encryption");

    for size in &[1024, 1024 * 10, 1024 * 100, 1024 * 1024] {
        let data = vec![0u8; *size];

        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| crypto.encrypt(black_box(data), black_box(&key)).unwrap());
        });
    }

    group.finish();
}

fn bench_decryption(c: &mut Criterion) {
    let crypto = FipsCrypto::new().unwrap();
    let passphrase = b"benchmark_passphrase_12345".to_vec();
    let (key, _) = crypto.derive_key(passphrase, None).unwrap();

    let mut group = c.benchmark_group("decryption");

    for size in &[1024, 1024 * 10, 1024 * 100, 1024 * 1024] {
        let data = vec![0u8; *size];
        let encrypted = crypto.encrypt(&data, &key).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &encrypted,
            |b, encrypted| {
                b.iter(|| {
                    crypto
                        .decrypt(black_box(encrypted), black_box(&key))
                        .unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_key_derivation(c: &mut Criterion) {
    let crypto = FipsCrypto::new().unwrap();

    c.bench_function("key_derivation", |b| {
        b.iter(|| {
            let passphrase = b"benchmark_passphrase_12345".to_vec();
            crypto.derive_key(black_box(passphrase), None).unwrap()
        });
    });
}

fn bench_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");

    for size in &[1024, 1024 * 10, 1024 * 100] {
        let data = vec![42u8; *size]; // Highly compressible

        group.bench_with_input(BenchmarkId::new("gzip", size), &data, |b, data| {
            b.iter(|| {
                compress(
                    black_box(data),
                    CompressionAlgorithm::Gzip,
                    CompressionLevel::Balanced,
                )
                .unwrap()
            });
        });

        group.bench_with_input(BenchmarkId::new("lzma", size), &data, |b, data| {
            b.iter(|| {
                compress(
                    black_box(data),
                    CompressionAlgorithm::Lzma,
                    CompressionLevel::Balanced,
                )
                .unwrap()
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_encryption,
    bench_decryption,
    bench_key_derivation,
    bench_compression
);
criterion_main!(benches);
