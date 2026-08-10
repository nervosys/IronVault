//! Integration tests for the REST API server.
//!
//! These tests start an in-process axum server and exercise the endpoints
//! through a real HTTP client.

#![cfg(feature = "api")]

use ironvault::api::server::{create_router, AppState, RateLimiter};
use ironvault::api::ApiConfig;
use ironvault::config::{DirectoryPaths, VaultConfig};
use ironvault::vault::Vault;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Method, Request, StatusCode};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt; // for `oneshot`

/// Helper: create a test AppState backed by a temporary vault.
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

    // `ApiConfig` is `#[non_exhaustive]`: start from `default()` and override.
    let mut api_config = ApiConfig::default();
    api_config.port = 0;
    api_config.jwt_secret = "test-secret-for-integration-tests".into();
    api_config.cors_permissive = true;

    Arc::new(AppState {
        vault: RwLock::new(vault),
        config: api_config,
        auth_rate_limiter: RateLimiter::new(5, std::time::Duration::from_secs(60)),
        // Federation stays off here: these tests cover the JWT-authenticated
        // API, and the federation routes are not registered when disabled.
        vault_config,
        federation: None,
    })
}

/// Helper: create the router with mock ConnectInfo for oneshot testing.
fn test_router(state: Arc<AppState>) -> axum::Router {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    create_router(state).layer(axum::Extension(ConnectInfo(addr)))
}

/// Helper: authenticate and return a JWT token.
async fn get_token(state: &Arc<AppState>) -> String {
    // First unlock the vault
    {
        let mut vault = state.vault.write().await;
        vault
            .unlock(b"integration-test-passphrase".to_vec())
            .unwrap();
    }

    ironvault::api::auth::create_token(
        &state.config.jwt_secret,
        state.config.token_expiry_secs,
    )
    .unwrap()
}

// ── Health ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_health_endpoint() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let app = test_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

// ── Auth ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_auth_token_success() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let app = test_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/auth/token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "passphrase": "my-vault-password"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["token"].is_string());
    assert!(json["expires_in"].as_u64().unwrap() > 0);
}

// ── Models (unauthorized) ────────────────────────────────────────────────────

#[tokio::test]
async fn test_models_unauthorized() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let app = test_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── Models (authorized, empty vault) ─────────────────────────────────────────

#[tokio::test]
async fn test_list_models_empty() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = get_token(&state).await;
    let app = test_router(state);

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

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json, serde_json::json!([]));
}

// ── Stats ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_stats_endpoint() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = get_token(&state).await;
    let app = test_router(state);

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

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["model_count"], 0);
}

// ── Conversions ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_conversions_endpoint() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let app = test_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/conversions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(json.len() >= 10);
}

// ── Convert ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_convert_endpoint() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = get_token(&state).await;
    let app = test_router(state);

    // Valid format names but invalid binary data — conversion should fail gracefully
    let data = b"raw tensor data for api test 0123456789";
    let payload = serde_json::json!({
        "data_base64": B64.encode(data),
        "source_format": "pytorch",
        "target_format": "safetensors",
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/convert")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Endpoint returns 500 because the raw bytes are not a valid PyTorch archive
    assert!(
        resp.status() == StatusCode::INTERNAL_SERVER_ERROR
            || resp.status() == StatusCode::BAD_REQUEST,
        "Expected 400 or 500 for invalid model data, got {}",
        resp.status()
    );
}

/// A conversion needing external tooling must not return plan JSON dressed up as
/// model bytes — a client decoding `data_base64` into `model.onnx` would get a
/// corrupt file. The response says `converted: false` and carries a `plan`.
#[tokio::test]
async fn test_convert_endpoint_reports_plan_instead_of_fake_data() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = get_token(&state).await;
    let app = test_router(state);

    let payload = serde_json::json!({
        "data_base64": B64.encode(b"pytorch model bytes"),
        "source_format": "pytorch",
        "target_format": "onnx",
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/convert")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        json["converted"], false,
        "PyTorch→ONNX needs external tooling"
    );
    assert!(
        json.get("data_base64").is_none() || json["data_base64"].is_null(),
        "no model bytes may be returned when nothing was converted: {json}"
    );
    assert_eq!(json["plan"]["converter"], "pytorch_to_onnx");
    assert_eq!(json["output_size"], 0);
}

// ── OpenAPI ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_openapi_endpoint() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let app = test_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["openapi"], "3.1.0");
    assert_eq!(json["info"]["title"], "IronVault API");
}

// ── Dashboard ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_dashboard_endpoint() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let app = test_router(state);

    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("IronVault"));
    assert!(html.contains("<!DOCTYPE html>"));
}

// ── Audit ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_audit_endpoint_empty() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = get_token(&state).await;
    let app = test_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

// ── Invalid token ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_invalid_token_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let app = test_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/models")
                .header(header::AUTHORIZATION, "Bearer invalid.token.here")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── Model not found ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_nonexistent_model() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = get_token(&state).await;
    let app = test_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/models/nonexistent/versions")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Model Cards ──────────────────────────────────────────────────────────────

/// Helper: store a dummy model directly through the vault so card endpoints work.
async fn store_test_model(state: &Arc<AppState>, name: &str) {
    let mut vault = state.vault.write().await;
    let data = b"test model data for card tests".to_vec();
    let mut metadata = ironvault::formats::ModelMetadata::new(
        name.to_string(),
        ironvault::formats::ModelFormat::Safetensors,
    );
    metadata = metadata.with_description("A test model".to_string());
    vault.store_model(name, data, metadata, None).unwrap();
}

#[tokio::test]
async fn test_get_model_card() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = get_token(&state).await;
    store_test_model(&state, "card-test").await;
    let app = test_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/models/card-test/card")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_model_card_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = get_token(&state).await;
    let app = test_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/models/no-such-model/card")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_model_card() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = get_token(&state).await;
    store_test_model(&state, "card-create-test").await;
    let app = test_router(state);

    let card_json = serde_json::json!({
        "model_details": {
            "name": "card-create-test",
            "version": "v1",
            "description": "Test model",
            "model_type": "transformer",
            "architecture": "GPT-2",
            "size": "124M",
            "framework": "pytorch",
            "format": "safetensors",
            "developers": ["Test Team"]
        },
        "intended_use": {
            "primary_uses": ["text generation"],
            "primary_users": ["researchers"],
            "out_of_scope_uses": ["production without review"]
        },
        "metadata": {},
        "created_at": "2025-01-01T00:00:00Z",
        "updated_at": "2025-01-01T00:00:00Z"
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/models/card-create-test/card")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&card_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["model_details"]["name"], "card-create-test");
}

// ── Compliance ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_compliance_endpoint() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = get_token(&state).await;
    let app = test_router(state);

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

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // Compliance response should have standard check fields
    assert!(json.get("fips_140_3").is_some());
    assert!(json.get("cmmc_level").is_some());
}

// ── RAG ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_rag_search_empty() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = get_token(&state).await;
    let app = test_router(state);

    let payload = serde_json::json!({ "query": "transformer architecture" });

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/rag/search")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["query"], "transformer architecture");
    assert!(json["results"].is_array());
}

#[tokio::test]
async fn test_rag_search_validation() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = get_token(&state).await;
    let app = test_router(state);

    // Empty query should be rejected
    let payload = serde_json::json!({ "query": "" });

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/rag/search")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_rag_add_document() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = get_token(&state).await;
    let app = test_router(state);

    let payload = serde_json::json!({
        "content": "Attention is all you need. The transformer architecture...",
        "metadata": {
            "source": "paper",
            "year": "2017"
        }
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/rag/documents")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["id"].is_string());
    assert_eq!(json["content_length"], 58);
}

#[tokio::test]
async fn test_rag_add_document_validation() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);
    let token = get_token(&state).await;
    let app = test_router(state);

    // Empty content should be rejected
    let payload = serde_json::json!({ "content": "" });

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/rag/documents")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ── RBAC Audit Filtering ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_audit_rbac_admin_sees_all() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);

    // Unlock vault first
    {
        let mut vault = state.vault.write().await;
        vault
            .unlock(b"integration-test-passphrase".to_vec())
            .unwrap();
    }

    // Write a security event to the audit log
    let vault = state.vault.read().await;
    let audit_path = vault.get_config().get_audit_log_path();
    drop(vault);

    if let Some(parent) = audit_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let security_entry = serde_json::json!({
        "timestamp": "2025-01-01T00:00:00Z",
        "event_type": "SECURITY_VIOLATION",
        "description": "Suspicious activity detected",
        "success": false
    });
    let normal_entry = serde_json::json!({
        "timestamp": "2025-01-01T00:00:01Z",
        "event_type": "MODEL_STORED",
        "description": "Model stored",
        "success": true
    });
    let mut log_content = serde_json::to_string(&security_entry).unwrap();
    log_content.push('\n');
    log_content.push_str(&serde_json::to_string(&normal_entry).unwrap());
    log_content.push('\n');
    std::fs::write(&audit_path, &log_content).unwrap();

    // Admin token (default) should see both entries
    let admin_token = ironvault::api::auth::create_token(
        &state.config.jwt_secret,
        state.config.token_expiry_secs,
    )
    .unwrap();

    let app = test_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit")
                .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.len(), 2, "Admin should see all entries");
}

#[tokio::test]
async fn test_audit_rbac_operator_filtered() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_state(&dir);

    // Unlock vault first
    {
        let mut vault = state.vault.write().await;
        vault
            .unlock(b"integration-test-passphrase".to_vec())
            .unwrap();
    }

    // Write audit entries
    let vault = state.vault.read().await;
    let audit_path = vault.get_config().get_audit_log_path();
    drop(vault);

    if let Some(parent) = audit_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let security_entry = serde_json::json!({
        "timestamp": "2025-01-01T00:00:00Z",
        "event_type": "SECURITY_VIOLATION",
        "description": "Suspicious activity",
        "success": false
    });
    let normal_entry = serde_json::json!({
        "timestamp": "2025-01-01T00:00:01Z",
        "event_type": "MODEL_STORED",
        "description": "Model stored",
        "success": true
    });
    let mut log_content = serde_json::to_string(&security_entry).unwrap();
    log_content.push('\n');
    log_content.push_str(&serde_json::to_string(&normal_entry).unwrap());
    log_content.push('\n');
    std::fs::write(&audit_path, &log_content).unwrap();

    // Operator token should NOT see security events
    let operator_token = ironvault::api::auth::create_token_with_role(
        &state.config.jwt_secret,
        state.config.token_expiry_secs,
        ironvault::api::auth::Role::Operator,
    )
    .unwrap();

    let app = test_router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit")
                .header(header::AUTHORIZATION, format!("Bearer {}", operator_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        json.len(),
        1,
        "Operator should only see non-security entries"
    );
    assert_eq!(json[0]["event_type"], "MODEL_STORED");
}
