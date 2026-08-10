//! Integration tests for the endpoints added in 5.1.0.
//!
//! These paths were documented in `.well-known/openapi.yaml` from 1.x onward
//! and never implemented, so every client generated from the published spec
//! called them and got a 404.
//!
//! Two things are under test. The obvious one is that each endpoint now exists
//! and answers. The one that matters more is that they do **not** honour the
//! shapes the old spec described: it accepted server-side filesystem paths
//! (`path`, `output`, `archive`, and "file path or name@version"), which over
//! HTTP would have handed any token holder arbitrary file read and write as the
//! server user. Those assertions are the reason this file exists.

#![cfg(feature = "api")]

use ironvault::api::server::{create_router, AppState, RateLimiter};
use ironvault::api::ApiConfig;
use ironvault::config::{DirectoryPaths, VaultConfig};
use ironvault::vault::Vault;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Method, Request, StatusCode};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

fn test_state(dir: &tempfile::TempDir) -> Arc<AppState> {
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

    let mut api_config = ApiConfig::default();
    api_config.port = 0;
    api_config.jwt_secret = "test-secret-for-reconciliation-tests".into();
    api_config.cors_permissive = true;

    Arc::new(AppState {
        vault: RwLock::new(vault),
        config: api_config,
        auth_rate_limiter: RateLimiter::new(5, std::time::Duration::from_secs(60)),
        vault_config,
        federation: None,
    })
}

fn test_router(state: Arc<AppState>) -> axum::Router {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    create_router(state).layer(axum::Extension(ConnectInfo(addr)))
}

/// Unlock the vault, store a model, and return a usable token.
async fn setup(state: &Arc<AppState>, model: &str, bytes: &[u8]) -> String {
    {
        let mut vault = state.vault.write().await;
        vault
            .unlock(b"reconciliation-test-passphrase".to_vec())
            .unwrap();
        let metadata = ironvault::formats::ModelMetadata::new(
            model.to_string(),
            ironvault::formats::ModelFormat::Safetensors,
        );
        vault
            .store_model(model, bytes.to_vec(), metadata, None)
            .unwrap();
    }
    ironvault::api::auth::create_token(&state.config.jwt_secret, state.config.token_expiry_secs)
        .unwrap()
}

async fn post_json(
    app: axum::Router,
    uri: &str,
    token: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let raw = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json = serde_json::from_slice(&raw).unwrap_or(serde_json::Value::Null);
    (status, json)
}

// ── The endpoints exist at all ───────────────────────────────────────────────

#[tokio::test]
async fn introspect_serves_the_same_schema_as_the_cli() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let app = test_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/introspect")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK, "introspect must not 404");
    let raw = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&raw).unwrap();

    // Same builder as `iv introspect`, so the two surfaces cannot disagree.
    assert_eq!(json, ironvault::cli_schema::build(false));
    assert_eq!(json["binary"], "iv");
    assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn telemetry_status_reports_disabled_state() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = setup(&state, "m", b"bytes").await;
    let app = test_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/telemetry/status")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let raw = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    assert!(json["enabled"].is_boolean());
    assert!(json["do_not_track"].is_boolean());
}

#[tokio::test]
async fn sign_then_verify_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = setup(&state, "signme", b"model-weights").await;

    let (status, signed) = post_json(
        test_router(state.clone()),
        "/api/v1/models/signme/sign",
        &token,
        serde_json::json!({ "key": "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "sign returned {signed}");
    assert_eq!(signed["algorithm"], "HMAC-SHA256");

    // The signature comes back inline; there is no server-side path to fetch.
    assert!(signed["signature"].is_object());
    assert!(signed.get("signature_path").is_none());

    let (status, verified) = post_json(
        test_router(state),
        "/api/v1/models/signme/verify",
        &token,
        serde_json::json!({
            "signature": signed["signature"],
            "key": "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "verify returned {verified}");
    assert_eq!(verified["valid"], true);
    assert_eq!(verified["signature_checked"], true);
}

#[tokio::test]
async fn verify_rejects_a_signature_from_a_different_key() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = setup(&state, "signme", b"model-weights").await;

    let (_, signed) = post_json(
        test_router(state.clone()),
        "/api/v1/models/signme/sign",
        &token,
        serde_json::json!({ "key": "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff" }),
    )
    .await;

    let (status, verified) = post_json(
        test_router(state),
        "/api/v1/models/signme/verify",
        &token,
        serde_json::json!({
            "signature": signed["signature"],
            "key": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        verified["valid"], false,
        "a signature must not verify under a different key"
    );
}

#[tokio::test]
async fn scan_reports_on_stored_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = setup(&state, "scanme", b"harmless bytes").await;

    let (status, json) = post_json(
        test_router(state),
        "/api/v1/models/scanme/scan",
        &token,
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "scan returned {json}");
    assert_eq!(json["model"], "scanme");
    assert!(json["safe"].is_boolean());
}

#[tokio::test]
async fn benchmarks_record_then_list() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = setup(&state, "benchme", b"bytes").await;

    let (status, _) = post_json(
        test_router(state.clone()),
        "/api/v1/models/benchme/benchmarks",
        &token,
        serde_json::json!({
            "version": 1, "benchmark": "mmlu", "score": 0.71, "unit": "accuracy"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let resp = test_router(state)
        .oneshot(
            Request::builder()
                .uri("/api/v1/models/benchme/benchmarks")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let raw = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(json["model"], "benchme");
    assert!(
        !json["records"].as_array().unwrap().is_empty(),
        "the recorded benchmark should come back"
    );
}

#[tokio::test]
async fn vault_export_streams_the_bundle_in_the_response() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = setup(&state, "exportme", b"bytes").await;

    let resp = test_router(state)
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/vault/export")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/gzip",
        "the bundle must come back in the body, not be written server-side"
    );
    let raw = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(!raw.is_empty(), "bundle body was empty");
    assert_eq!(&raw[..2], &[0x1f, 0x8b], "body is not gzip");
}

// ── The security properties ──────────────────────────────────────────────────

#[tokio::test]
async fn license_scan_does_not_accept_a_filesystem_path() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = setup(&state, "licensed", b"MIT License\n\nPermission is hereby").await;

    // The pre-5.1.0 spec shape. It must not be honoured: `path` is not a field
    // this endpoint has, so the request is rejected rather than reading a file.
    let (status, _) = post_json(
        test_router(state.clone()),
        "/api/v1/license-scan",
        &token,
        serde_json::json!({ "path": "/etc/passwd" }),
    )
    .await;
    assert!(
        status.is_client_error(),
        "a `path` body must be rejected, got {status}"
    );

    // The supported shape names a model in the vault.
    let (status, json) = post_json(
        test_router(state),
        "/api/v1/license-scan",
        &token,
        serde_json::json!({ "model": "licensed" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "license-scan returned {json}");
    assert_eq!(json["model"], "licensed");
}

#[tokio::test]
async fn diff_does_not_accept_a_filesystem_path() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = setup(&state, "left", b"aaaa").await;
    {
        let mut vault = state.vault.write().await;
        let metadata = ironvault::formats::ModelMetadata::new(
            "right".to_string(),
            ironvault::formats::ModelFormat::Safetensors,
        );
        vault
            .store_model("right", b"bbbb".to_vec(), metadata, None)
            .unwrap();
    }

    // A path must not resolve. `validate_model_name` rejects the separators,
    // so this never reaches the filesystem.
    let (status, _) = post_json(
        test_router(state.clone()),
        "/api/v1/models/diff",
        &token,
        serde_json::json!({ "left": "/etc/passwd", "right": "/etc/hosts" }),
    )
    .await;
    assert!(
        status.is_client_error(),
        "a filesystem path must be rejected, got {status}"
    );

    let (status, json) = post_json(
        test_router(state),
        "/api/v1/models/diff",
        &token,
        serde_json::json!({ "left": "left", "right": "right" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "diff returned {json}");
}

#[tokio::test]
async fn vault_import_reads_the_body_not_a_server_path() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = setup(&state, "m", b"bytes").await;

    // The old spec's shape, sent as a JSON body. It is treated as opaque
    // bytes — never as a path to open — so it fails as a malformed bundle
    // rather than reading the named file.
    let (status, json) = post_json(
        test_router(state.clone()),
        "/api/v1/vault/import",
        &token,
        serde_json::json!({ "archive": "/etc/passwd" }),
    )
    .await;
    assert!(
        status.is_client_error(),
        "a JSON body naming a path must fail as a bad bundle, got {status}: {json}"
    );

    // An empty body is refused rather than treated as an empty archive.
    let resp = test_router(state)
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/vault/import")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_new_endpoints_require_authentication() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let _ = setup(&state, "m", b"bytes").await;

    // `/introspect` is deliberately open: it is a discovery document with no
    // vault data, and needing a token to learn how to get a token is a loop.
    //
    // Each body below is *valid* for its endpoint on purpose. axum runs
    // extractors before the handler body, so an unparseable body yields 422
    // before `require_auth` is ever reached — which would make this test pass
    // for the wrong reason and prove nothing about authentication.
    let protected: [(&str, Method, serde_json::Value); 13] = [
        (
            "/api/v1/telemetry/status",
            Method::GET,
            serde_json::Value::Null,
        ),
        (
            "/api/v1/license-scan",
            Method::POST,
            serde_json::json!({"model": "m"}),
        ),
        (
            "/api/v1/models/diff",
            Method::POST,
            serde_json::json!({"left": "m", "right": "m"}),
        ),
        (
            "/api/v1/models/pull",
            Method::POST,
            serde_json::json!({"source": "https://example.invalid/model.bin"}),
        ),
        (
            "/api/v1/models/m/sign",
            Method::POST,
            serde_json::json!({"key": "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"}),
        ),
        (
            "/api/v1/models/m/verify",
            Method::POST,
            serde_json::json!({"signature": {}}),
        ),
        ("/api/v1/models/m/scan", Method::POST, serde_json::json!({})),
        (
            "/api/v1/models/m/register",
            Method::POST,
            serde_json::json!({"engine": "ollama"}),
        ),
        (
            "/api/v1/models/m/benchmarks",
            Method::GET,
            serde_json::Value::Null,
        ),
        (
            "/api/v1/models/m/card/validate",
            Method::POST,
            serde_json::json!({}),
        ),
        (
            "/api/v1/models/m/card/generate",
            Method::POST,
            serde_json::json!({}),
        ),
        ("/api/v1/vault/export", Method::POST, serde_json::json!({})),
        ("/api/v1/vault/import", Method::POST, serde_json::json!({})),
    ];

    for (uri, method, body) in protected {
        let payload = if body.is_null() {
            Body::empty()
        } else {
            Body::from(serde_json::to_vec(&body).unwrap())
        };
        let resp = test_router(state.clone())
            .oneshot(
                Request::builder()
                    .method(method.clone())
                    .uri(uri)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(payload)
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {uri} answered without a token"
        );
    }
}
