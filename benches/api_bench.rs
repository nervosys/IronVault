//! API endpoint benchmarks — measures request/response latency for key endpoints.

use ironvault::api::server::{create_router, AppState, RateLimiter};
use ironvault::api::ApiConfig;
use ironvault::config::{DirectoryPaths, VaultConfig};
use ironvault::vault::Vault;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Request, StatusCode};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

fn bench_state(dir: &tempfile::TempDir) -> Arc<AppState> {
    let dirs = DirectoryPaths {
        config_dir: dir.path().join("config"),
        data_dir: dir.path().join("data"),
        cache_dir: dir.path().join("cache"),
        vault_dir: dir.path().join("data/vaults/default"),
        log_dir: dir.path().join("data/logs"),
        backends_dir: dir.path().join("config/backends"),
        utilities_dir: dir.path().join("config/utilities"),
        databases_dir: dir.path().join("config/databases"),
    };
    let config = VaultConfig::with_dirs(dirs).unwrap();
    let vault_config = config.clone();
    let vault = Vault::new(Some(config)).unwrap();

    // `ApiConfig` is `#[non_exhaustive]`: start from `default()` and override.
    let mut api_config = ApiConfig::default();
    api_config.port = 0;
    api_config.jwt_secret = "bench-secret-key-for-benchmarks-only".into();
    api_config.cors_permissive = true;
    api_config.enable_dashboard = false;

    Arc::new(AppState {
        vault: RwLock::new(vault),
        config: api_config,
        auth_rate_limiter: RateLimiter::new(100, std::time::Duration::from_secs(60)),
        vault_config,
        federation: None,
    })
}

fn bench_router(state: Arc<AppState>) -> axum::Router {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    create_router(state).layer(axum::Extension(ConnectInfo(addr)))
}

fn bench_health(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("api_health", |b| {
        b.iter(|| {
            rt.block_on(async {
                let dir = tempfile::tempdir().unwrap();
                let state = bench_state(&dir);
                let app = bench_router(state);

                let resp = app
                    .oneshot(
                        Request::builder()
                            .uri("/api/v1/health")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();

                assert_eq!(black_box(resp.status()), StatusCode::OK);
            });
        });
    });
}

fn bench_auth_token(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("api_auth_token", |b| {
        b.iter(|| {
            rt.block_on(async {
                let dir = tempfile::tempdir().unwrap();
                let state = bench_state(&dir);

                // Unlock vault first
                {
                    let mut vault = state.vault.write().await;
                    vault
                        .unlock(b"bench-passphrase-with-entropy".to_vec())
                        .unwrap();
                }

                let app = bench_router(state);
                let resp = app
                    .oneshot(
                        Request::builder()
                            .method("POST")
                            .uri("/api/v1/auth/token")
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Body::from(
                                r#"{"passphrase":"bench-passphrase-with-entropy"}"#,
                            ))
                            .unwrap(),
                    )
                    .await
                    .unwrap();

                assert_eq!(black_box(resp.status()), StatusCode::OK);
            });
        });
    });
}

fn bench_list_models(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("api_list_models", |b| {
        b.iter(|| {
            rt.block_on(async {
                let dir = tempfile::tempdir().unwrap();
                let state = bench_state(&dir);

                let mut vault = state.vault.write().await;
                vault
                    .unlock(b"bench-passphrase-with-entropy".to_vec())
                    .unwrap();
                drop(vault);

                let token = ironvault::api::auth::create_token(
                    &state.config.jwt_secret,
                    state.config.token_expiry_secs,
                )
                .unwrap();

                let app = bench_router(state);
                let resp = app
                    .oneshot(
                        Request::builder()
                            .uri("/api/v1/models")
                            .header(header::AUTHORIZATION, format!("Bearer {}", token))
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();

                assert_eq!(black_box(resp.status()), StatusCode::OK);
            });
        });
    });
}

fn bench_compliance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("api_compliance", |b| {
        b.iter(|| {
            rt.block_on(async {
                let dir = tempfile::tempdir().unwrap();
                let state = bench_state(&dir);

                let mut vault = state.vault.write().await;
                vault
                    .unlock(b"bench-passphrase-with-entropy".to_vec())
                    .unwrap();
                drop(vault);

                let token = ironvault::api::auth::create_token(
                    &state.config.jwt_secret,
                    state.config.token_expiry_secs,
                )
                .unwrap();

                let app = bench_router(state);
                let resp = app
                    .oneshot(
                        Request::builder()
                            .uri("/api/v1/compliance")
                            .header(header::AUTHORIZATION, format!("Bearer {}", token))
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();

                assert_eq!(black_box(resp.status()), StatusCode::OK);
            });
        });
    });
}

fn bench_store_model(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("api_store_model_10kb", |b| {
        b.iter(|| {
            rt.block_on(async {
                let dir = tempfile::tempdir().unwrap();
                let state = bench_state(&dir);

                let mut vault = state.vault.write().await;
                vault
                    .unlock(b"bench-passphrase-with-entropy".to_vec())
                    .unwrap();
                drop(vault);

                let token = ironvault::api::auth::create_token(
                    &state.config.jwt_secret,
                    state.config.token_expiry_secs,
                )
                .unwrap();

                // Build multipart body with a 10 KB payload
                let boundary = "----BenchBoundary";
                let payload = vec![0xABu8; 10 * 1024];
                let body_bytes = format!(
                    "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"bench.safetensors\"\r\nContent-Type: application/octet-stream\r\n\r\n",
                )
                .into_bytes()
                .into_iter()
                .chain(payload)
                .chain(format!("\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"format\"\r\n\r\nsafetensors\r\n--{boundary}--\r\n").into_bytes())
                .collect::<Vec<u8>>();

                let app = bench_router(state);
                let resp = app
                    .oneshot(
                        Request::builder()
                            .method("POST")
                            .uri("/api/v1/models/bench-model")
                            .header(
                                header::CONTENT_TYPE,
                                format!("multipart/form-data; boundary={boundary}"),
                            )
                            .header(header::AUTHORIZATION, format!("Bearer {}", token))
                            .body(Body::from(body_bytes))
                            .unwrap(),
                    )
                    .await
                    .unwrap();

                assert_eq!(black_box(resp.status()), StatusCode::CREATED);
            });
        });
    });
}

fn bench_stats(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("api_stats", |b| {
        b.iter(|| {
            rt.block_on(async {
                let dir = tempfile::tempdir().unwrap();
                let state = bench_state(&dir);

                let mut vault = state.vault.write().await;
                vault
                    .unlock(b"bench-passphrase-with-entropy".to_vec())
                    .unwrap();
                drop(vault);

                let token = ironvault::api::auth::create_token(
                    &state.config.jwt_secret,
                    state.config.token_expiry_secs,
                )
                .unwrap();

                let app = bench_router(state);
                let resp = app
                    .oneshot(
                        Request::builder()
                            .uri("/api/v1/stats")
                            .header(header::AUTHORIZATION, format!("Bearer {}", token))
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();

                assert_eq!(black_box(resp.status()), StatusCode::OK);
            });
        });
    });
}

fn bench_list_conversions(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("api_list_conversions", |b| {
        b.iter(|| {
            rt.block_on(async {
                let dir = tempfile::tempdir().unwrap();
                let state = bench_state(&dir);
                let app = bench_router(state);

                let resp = app
                    .oneshot(
                        Request::builder()
                            .uri("/api/v1/conversions")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();

                assert_eq!(black_box(resp.status()), StatusCode::OK);
            });
        });
    });
}

fn bench_openapi_json(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("api_openapi_json", |b| {
        b.iter(|| {
            rt.block_on(async {
                let dir = tempfile::tempdir().unwrap();
                let state = bench_state(&dir);
                let app = bench_router(state);

                let resp = app
                    .oneshot(
                        Request::builder()
                            .uri("/api/v1/openapi.json")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();

                assert_eq!(black_box(resp.status()), StatusCode::OK);
            });
        });
    });
}

fn bench_metrics(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("api_metrics", |b| {
        b.iter(|| {
            rt.block_on(async {
                let dir = tempfile::tempdir().unwrap();
                let state = bench_state(&dir);

                let mut vault = state.vault.write().await;
                vault
                    .unlock(b"bench-passphrase-with-entropy".to_vec())
                    .unwrap();
                drop(vault);

                let token = ironvault::api::auth::create_token(
                    &state.config.jwt_secret,
                    state.config.token_expiry_secs,
                )
                .unwrap();

                let app = bench_router(state);
                let resp = app
                    .oneshot(
                        Request::builder()
                            .uri("/api/v1/metrics")
                            .header(header::AUTHORIZATION, format!("Bearer {}", token))
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();

                assert_eq!(black_box(resp.status()), StatusCode::OK);
            });
        });
    });
}

criterion_group!(
    benches,
    bench_health,
    bench_auth_token,
    bench_list_models,
    bench_compliance,
    bench_store_model,
    bench_stats,
    bench_list_conversions,
    bench_openapi_json,
    bench_metrics,
);
criterion_main!(benches);
