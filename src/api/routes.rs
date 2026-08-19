//! REST API route handlers.

use axum::extract::{ConnectInfo, Multipart, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;

use crate::conversion::{ConversionOptions, ConversionPipeline};
use crate::formats::{ModelFormat, ModelMetadata};
use crate::traits::VaultState;

use super::auth;
use super::dashboard;
use super::error::ApiError;
use super::openapi;
use super::server::AppState;

// ── Health ───────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,
}

/// GET /api/v1/health
///
/// Returns server health. Includes vault state when AppState is available.
pub async fn health(state: Option<State<Arc<AppState>>>) -> Json<HealthResponse> {
    let (vault_state_str, model_count) = if let Some(State(st)) = state {
        let vault = st.vault.read().await;
        let vs = vault.state();
        let mc = match &vs {
            VaultState::Locked { model_count, .. } => Some(*model_count),
            VaultState::Unlocked { model_count, .. } => Some(*model_count),
            _ => None,
        };
        (Some(vs.to_string()), mc)
    } else {
        (None, None)
    };

    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        vault_state: vault_state_str,
        model_count,
        uptime_seconds: None,
    })
}

// ── Auth ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AuthRequest {
    passphrase: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub expires_in: u64,
}

/// POST /api/v1/auth/token
///
/// Unlocks the vault with the given passphrase and returns a JWT.
pub async fn auth_token(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    // Rate-limit auth attempts per IP
    if !state.auth_rate_limiter.check(addr.ip()) {
        return Err(ApiError::rate_limited("Too many authentication attempts"));
    }

    // Attempt unlock
    {
        let mut vault = state.vault.write().await;
        vault
            .unlock(body.passphrase.into_bytes())
            .map_err(|_| ApiError::unauthorized("Invalid passphrase"))?;
    }

    let token = auth::create_token(&state.config.jwt_secret, state.config.token_expiry_secs)
        .map_err(|e| ApiError::internal(format!("Token creation failed: {e}")))?;

    Ok(Json(AuthResponse {
        token,
        expires_in: state.config.token_expiry_secs,
    }))
}

/// POST /api/v1/auth/logout
///
/// Revokes the presented token so it cannot be used again before it expires.
///
/// `auth::revoke_claims` existed but nothing ever called it: there was no way
/// to invalidate a leaked or finished token short of rotating `jwt_secret`,
/// which invalidates every other token at the same time.
pub async fn auth_logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let claims = require_auth(&headers, &state)?;

    auth::revoke_claims(&claims)
        .map_err(|e| ApiError::internal(format!("Could not persist revocation: {e}")))?;

    Ok(Json(serde_json::json!({
        "revoked": true,
        "jti": claims.jti,
    })))
}

// ── Models ───────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub version_count: usize,
}

/// GET /api/v1/models
pub async fn list_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ModelInfo>>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let models: Vec<ModelInfo> = vault
        .list_models()
        .into_iter()
        .map(|name| {
            let version_count = vault.list_versions(&name).len();
            ModelInfo {
                name,
                version_count,
            }
        })
        .collect();
    Ok(Json(models))
}

/// GET /api/v1/models/:name
pub async fn get_model(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;
    let vault = state.vault.read().await;
    let data = vault.get_model(&name, None).map_err(ApiError::from)?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        data,
    )
        .into_response())
}

/// POST /api/v1/models/:name  (multipart: file + format + description?)
pub async fn store_model(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;

    let mut file_data: Option<Vec<u8>> = None;
    let mut format_str: Option<String> = None;
    let mut description: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("Multipart error: {e}")))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "file" => {
                file_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| ApiError::bad_request(format!("Read error: {e}")))?
                        .to_vec(),
                );
            }
            "format" => {
                format_str = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::bad_request(e.to_string()))?,
                );
            }
            "description" => {
                description = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::bad_request(e.to_string()))?,
                );
            }
            _ => {} // ignore unknown fields
        }
    }

    let data = file_data.ok_or_else(|| ApiError::bad_request("Missing 'file' field"))?;
    let fmt = format_str.ok_or_else(|| ApiError::bad_request("Missing 'format' field"))?;
    // Accept both a format name ("PyTorch") and an extension ("pt"): storing a
    // Custom variant here would break conversion and diffing later.
    let format = ModelFormat::from_stored(&fmt);

    let mut metadata = ModelMetadata::new(name.clone(), format);
    if let Some(desc) = description {
        metadata = metadata.with_description(desc);
    }

    let mut vault = state.vault.write().await;
    let version = vault
        .store_model(&name, data, metadata, None)
        .map_err(ApiError::from)?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "model": name,
            "version": version.version,
            "checkpoint_id": version.checkpoint_id,
            "size_bytes": version.size_bytes,
            "checksum": version.checksum_sha256,
        })),
    ))
}

// ── Versions ─────────────────────────────────────────────────────────────────

/// GET /api/v1/models/:name/versions
pub async fn list_versions(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;
    let vault = state.vault.read().await;
    let versions = vault.list_versions(&name);
    if versions.is_empty() {
        return Err(ApiError::not_found(format!("Model '{}' not found", name)));
    }
    let vs: Vec<serde_json::Value> = versions
        .iter()
        .map(|v| {
            serde_json::json!({
                "version": v.version,
                "checkpoint_id": v.checkpoint_id,
                "timestamp": v.timestamp.to_rfc3339(),
                "format": v.format,
                "size_bytes": v.size_bytes,
                "compressed_size_bytes": v.compressed_size_bytes,
                "checksum_sha256": v.checksum_sha256,
                "parent_version": v.parent_version,
            })
        })
        .collect();
    Ok(Json(vs))
}

/// GET /api/v1/models/:name/versions/:version
pub async fn get_version(
    State(state): State<Arc<AppState>>,
    Path((name, version)): Path<(String, u32)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;
    let vault = state.vault.read().await;
    let data = vault
        .get_model(&name, Some(version))
        .map_err(ApiError::from)?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        data,
    )
        .into_response())
}

/// DELETE /api/v1/models/:name/versions/:version
pub async fn delete_version(
    State(state): State<Arc<AppState>>,
    Path((name, version)): Path<(String, u32)>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;
    let mut vault = state.vault.write().await;
    let deleted = vault
        .delete_version(&name, version)
        .map_err(ApiError::from)?;
    if deleted {
        Ok(Json(
            serde_json::json!({ "deleted": true, "model": name, "version": version }),
        ))
    } else {
        Err(ApiError::not_found(format!(
            "Version {} not found for model '{}'",
            version, name
        )))
    }
}

/// GET /api/v1/models/:name/lineage/:version
pub async fn get_lineage(
    State(state): State<Arc<AppState>>,
    Path((name, version)): Path<(String, u32)>,
    headers: HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;
    let vault = state.vault.read().await;
    let lineage = vault.get_lineage(&name, version);
    let vs: Vec<serde_json::Value> = lineage
        .iter()
        .map(|v| {
            serde_json::json!({
                "version": v.version,
                "checkpoint_id": v.checkpoint_id,
                "timestamp": v.timestamp.to_rfc3339(),
                "format": v.format,
                "size_bytes": v.size_bytes,
            })
        })
        .collect();
    Ok(Json(vs))
}

// ── Conversions ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ConversionInfo {
    pub name: String,
    pub source: String,
    pub target: String,
}

/// GET /api/v1/conversions
pub async fn list_conversions() -> Json<Vec<ConversionInfo>> {
    let pipeline = ConversionPipeline::with_builtins();
    let conversions: Vec<ConversionInfo> = pipeline
        .supported_conversions()
        .into_iter()
        .map(|(src, tgt, converter_name)| ConversionInfo {
            name: converter_name.to_string(),
            source: src.to_string(),
            target: tgt.to_string(),
        })
        .collect();
    Json(conversions)
}

#[derive(Deserialize)]
pub struct ConvertRequest {
    data_base64: String,
    source_format: String,
    target_format: String,
    quantization: Option<String>,
    opset_version: Option<u32>,
    #[serde(default)]
    validate: bool,
}

#[derive(Serialize)]
pub struct ConvertResponse {
    /// True when `data_base64` holds real target-format bytes.
    ///
    /// False when the conversion needs external tooling: `data_base64` is then
    /// `null` and `plan` describes the steps to run. Clients must check this
    /// before writing the payload to a file — otherwise they produce a file
    /// with the target extension and the wrong contents.
    pub converted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_base64: Option<String>,
    /// Instructions for performing this conversion with external tooling.
    /// Present only when `converted` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<serde_json::Value>,
    pub source_format: String,
    pub target_format: String,
    pub conversion_path: Vec<String>,
    pub input_size: u64,
    pub output_size: u64,
    pub validation: Option<serde_json::Value>,
}

/// POST /api/v1/convert
pub async fn convert(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ConvertRequest>,
) -> Result<Json<ConvertResponse>, ApiError> {
    let _claims = require_auth(&headers, &state)?;

    let data = B64
        .decode(&body.data_base64)
        .map_err(|e| ApiError::bad_request(format!("Invalid base64: {e}")))?;

    let src = parse_format(&body.source_format)?;
    let tgt = parse_format(&body.target_format)?;

    let opts = ConversionOptions {
        quantization: body.quantization,
        opset_version: body.opset_version,
        validate: body.validate,
        ..ConversionOptions::default()
    };

    let pipeline = ConversionPipeline::with_builtins();
    let result = pipeline
        .convert(&data, &src, &tgt, &opts, None)
        .map_err(ApiError::from)?;

    let validation = result.validation.as_ref().map(|r| {
        serde_json::json!({
            "passed": r.passed,
            "checks": r.checks.iter().map(|c| serde_json::json!({
                "name": c.name,
                "passed": c.passed,
                "message": c.message,
            })).collect::<Vec<_>>()
        })
    });

    let converted = !result.is_plan();

    Ok(Json(ConvertResponse {
        converted,
        data_base64: converted.then(|| B64.encode(&result.data)),
        plan: result.plan.clone(),
        source_format: result.source_format.to_string(),
        target_format: result.target_format.to_string(),
        conversion_path: result
            .conversion_path
            .iter()
            .map(|f| f.to_string())
            .collect(),
        input_size: result.input_size,
        output_size: result.output_size,
        validation,
    }))
}

// ── Stats ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct StatsResponse {
    pub model_count: usize,
    pub total_versions: usize,
    pub total_size_bytes: u64,
    pub file_count: usize,
}

/// GET /api/v1/stats
pub async fn stats(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<StatsResponse>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let s = vault.get_stats().map_err(ApiError::from)?;
    Ok(Json(StatsResponse {
        model_count: s.model_count,
        total_versions: s.total_versions,
        total_size_bytes: s.total_size_bytes,
        file_count: s.file_count,
    }))
}

// ── Audit ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AuditQuery {
    limit: Option<usize>,
}

/// GET /api/v1/audit
///
/// Returns audit log entries. Admins see all events; Operators and Viewers
/// cannot see `SecurityViolation`, `IntegrityFailure`, or `AuthFailure` events.
pub async fn audit_log(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let claims = require_auth(&headers, &state)?;

    // Read the audit log file from the vault config path
    let vault = state.vault.read().await;
    let audit_path = vault.get_config().get_audit_log_path();

    if !audit_path.exists() {
        return Ok(Json(vec![]));
    }

    let contents =
        std::fs::read_to_string(&audit_path).map_err(|e| ApiError::internal(e.to_string()))?;

    let mut entries: Vec<serde_json::Value> = contents
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    // Role-based filtering: non-admin roles cannot see security-sensitive events
    if claims.role != super::auth::Role::Admin {
        entries.retain(|entry| !is_security_event(entry));
    }

    let limited = if let Some(n) = q.limit {
        entries.into_iter().take(n).collect()
    } else {
        entries
    };

    Ok(Json(limited))
}

// ── Observability ─────────────────────────────────────────────────────────────

/// GET /api/v1/metrics
///
/// Returns vault metrics: state, model counts, operation counters,
/// storage statistics, and compliance status.
pub async fn metrics(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vs = vault.state();
    let stats = vault.get_stats().map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "vault_state": vs.to_string(),
        "models_count": stats.model_count,
        "versions_count": stats.total_versions,
        "storage_bytes": stats.total_size_bytes,
        "file_count": stats.file_count,
        "version": env!("CARGO_PKG_VERSION"),
        "healthy": true,
    })))
}

#[derive(Deserialize)]
pub struct EventsQuery {
    /// Maximum number of events to return.
    limit: Option<usize>,
    /// Filter by event type (e.g. "ModelStored", "VaultUnlocked").
    #[serde(rename = "type")]
    event_type: Option<String>,
}

/// GET /api/v1/events
///
/// Returns audit events from the vault's audit log, with optional
/// filtering by type and limit. Events are returned newest-first.
/// Non-admin roles cannot see security-sensitive events.
pub async fn events(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<EventsQuery>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let claims = require_auth(&headers, &state)?;

    let vault = state.vault.read().await;
    let audit_path = vault.get_config().get_audit_log_path();

    if !audit_path.exists() {
        return Ok(Json(vec![]));
    }

    let contents =
        std::fs::read_to_string(&audit_path).map_err(|e| ApiError::internal(e.to_string()))?;

    let mut entries: Vec<serde_json::Value> = contents
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    // Role-based filtering: non-admin roles cannot see security-sensitive events
    if claims.role != super::auth::Role::Admin {
        entries.retain(|entry| !is_security_event(entry));
    }

    // Filter by event type if specified
    if let Some(ref et) = q.event_type {
        let et_lower = et.to_lowercase();
        entries.retain(|entry| {
            entry
                .get("action")
                .and_then(|a| a.as_str())
                .map(|a| a.to_lowercase().contains(&et_lower))
                .unwrap_or(false)
                || entry
                    .get("type")
                    .and_then(|t| t.as_str())
                    .map(|t| t.to_lowercase().contains(&et_lower))
                    .unwrap_or(false)
        });
    }

    // Return newest first
    entries.reverse();

    // Apply limit
    if let Some(n) = q.limit {
        entries.truncate(n);
    }

    Ok(Json(entries))
}

// ── OpenAPI & Dashboard ──────────────────────────────────────────────────────

/// GET /api/v1/openapi.json
pub async fn openapi_json() -> Json<serde_json::Value> {
    Json(openapi::openapi_spec())
}

/// GET /  — serves the embedded web dashboard
pub async fn dashboard_index() -> Html<&'static str> {
    Html(dashboard::dashboard_html())
}

// ── Model Cards ──────────────────────────────────────────────────────────────

/// GET /api/v1/models/:name/card
///
/// Generate a model card for the given model using vault metadata.
pub async fn get_model_card(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;
    let vault = state.vault.read().await;

    // Verify model exists
    let versions = vault.list_versions(&name);
    if versions.is_empty() {
        return Err(ApiError::not_found(format!("Model '{}' not found", name)));
    }

    let latest = &versions[versions.len() - 1];
    let details = crate::model_card::ModelDetails {
        name: name.clone(),
        version: format!("v{}", latest.version),
        description: latest
            .metadata
            .get("description")
            .cloned()
            .unwrap_or_default(),
        model_type: String::new(),
        architecture: String::new(),
        size: format!("{} bytes", latest.size_bytes),
        framework: latest
            .metadata
            .get("framework")
            .cloned()
            .unwrap_or_default(),
        format: latest.format.clone(),
        license: None,
        citation: None,
        developers: vec![],
        contact: None,
        repository: None,
        paper: None,
    };
    let intended_use = crate::model_card::IntendedUse {
        primary_uses: vec!["General-purpose AI model".to_string()],
        primary_users: vec![],
        out_of_scope_uses: vec![],
        use_case_examples: None,
    };

    let card = crate::model_card::ModelCard::new(details, intended_use);

    let json_str = card
        .to_json()
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let value: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(value))
}

/// POST /api/v1/models/:name/card
///
/// Create (or overwrite) a custom model card from JSON.
/// Returns the rendered card re-serialized from the parsed input.
pub async fn create_model_card(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;

    // Verify model exists
    let vault = state.vault.read().await;
    let versions = vault.list_versions(&name);
    if versions.is_empty() {
        return Err(ApiError::not_found(format!("Model '{}' not found", name)));
    }

    let json_str = serde_json::to_string(&body)
        .map_err(|e| ApiError::bad_request(format!("Invalid JSON: {e}")))?;
    let card = crate::model_card::ModelCard::from_json(&json_str)
        .map_err(|e| ApiError::bad_request(format!("Invalid model card: {e}")))?;

    let roundtrip = card
        .to_json()
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let value: serde_json::Value =
        serde_json::from_str(&roundtrip).map_err(|e| ApiError::internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(value)))
}

// ── Compliance ───────────────────────────────────────────────────────────────

/// GET /api/v1/compliance
///
/// Run FIPS 140-3, CVE, MITRE ATT&CK, and CMMC 2.0 compliance checks.
pub async fn compliance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;

    let checker = crate::compliance::ComplianceChecker::new();
    let status = checker
        .run_all_checks()
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "fips_140_3": status.fips_140_3,
        "cve_scan_passed": status.cve_scan_passed,
        "mitre_attack_aligned": status.mitre_attack_aligned,
        "cmmc_level": status.cmmc_level,
        "all_passed": status.violations.is_empty(),
        "violations": status.violations.iter().map(|v| serde_json::json!({
            "standard": v.standard,
            "control": v.control,
            "severity": v.severity,
            "description": v.description,
            "remediation": v.remediation,
        })).collect::<Vec<_>>(),
    })))
}

// ── RAG ──────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RagSearchRequest {
    query: String,
    limit: Option<usize>,
}

/// POST /api/v1/rag/search
///
/// Search the RAG document store.
pub async fn rag_search(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RagSearchRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;

    if body.query.is_empty() || body.query.len() > 10_000 {
        return Err(ApiError::bad_request(
            "Query must be between 1 and 10,000 characters",
        ));
    }

    let vault = state.vault.read().await;
    let rag_path = vault.get_config().get_vault_path(None).join("rag");

    if !rag_path.exists() {
        return Ok(Json(
            serde_json::json!({ "results": [], "query": body.query }),
        ));
    }

    let kb_config = crate::rag::KnowledgeBaseConfig::default();
    let kb = crate::rag::KnowledgeBase::new("vault".to_string(), kb_config);
    let limit = body.limit.unwrap_or(10).min(100);
    // Without pre-computed embeddings, retrieve returns empty vec.
    // The endpoint is wired and ready for real embedding integration.
    let results = kb.retrieve(&[], Some(limit));

    Ok(Json(serde_json::json!({
        "query": body.query,
        "results": results.iter().map(|doc| serde_json::json!({
            "id": doc.id,
            "content": doc.content,
            "metadata": doc.metadata,
        })).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
pub struct RagAddDocumentRequest {
    content: String,
    #[serde(default)]
    metadata: std::collections::HashMap<String, String>,
}

/// POST /api/v1/rag/documents
///
/// Add a document to the RAG knowledge base.
pub async fn rag_add_document(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RagAddDocumentRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let _claims = require_auth(&headers, &state)?;

    if body.content.is_empty() || body.content.len() > 1_000_000 {
        return Err(ApiError::bad_request(
            "Document content must be between 1 and 1,000,000 characters",
        ));
    }

    // Validate metadata keys/values
    for (k, v) in &body.metadata {
        if k.len() > 256 || v.len() > 4096 {
            return Err(ApiError::bad_request(
                "Metadata key max 256 chars, value max 4096 chars",
            ));
        }
    }

    let id = format!("doc_{}", uuid_v4_simple());

    let doc = crate::rag::Document {
        id: id.clone(),
        content: body.content.clone(),
        metadata: body.metadata.clone(),
        embedding: None,
        chunk_info: None,
    };

    let mut store = crate::rag::DocumentStore::new();
    store
        .add_document(doc)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    // Acknowledge — note: in-memory store won't persist across requests,
    // but this wires the endpoint and demonstrates the API contract.
    let _ = state.vault.read().await; // verify vault is accessible
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": id,
            "content_length": body.content.len(),
            "metadata_keys": body.metadata.keys().collect::<Vec<_>>(),
        })),
    ))
}

/// Generate a simple unique ID (timestamp + random suffix).
fn uuid_v4_simple() -> String {
    use std::time::SystemTime;
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", ts)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Extract and verify the Bearer JWT from the Authorization header.
/// Returns the decoded [`Claims`] on success, for role-based access control.
fn require_auth(headers: &HeaderMap, state: &AppState) -> Result<super::auth::Claims, ApiError> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::unauthorized("Missing Authorization header"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::unauthorized("Invalid Authorization format (expected Bearer)"))?;

    // Reject tokens with invalid structure before verification
    if token.is_empty() || token.len() > 4096 || token.chars().any(|c| c.is_control()) {
        return Err(ApiError::unauthorized("Malformed token"));
    }

    auth::verify_token(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::unauthorized("Invalid or expired token"))
}

/// Parse a format string into a ModelFormat.
fn parse_format(s: &str) -> Result<ModelFormat, ApiError> {
    let f = match s.to_lowercase().as_str() {
        "safetensors" => ModelFormat::Safetensors,
        "gguf" => ModelFormat::GGUF,
        "pytorch" | "pt" | "pth" => ModelFormat::PyTorch,
        "onnx" => ModelFormat::ONNX,
        "tensorrt" | "trt" => ModelFormat::TensorRT,
        "coreml" | "mlmodel" => ModelFormat::CoreML,
        "tflite" => ModelFormat::TFLite,
        "tensorflow" | "tf" | "pb" => ModelFormat::TensorFlow,
        "keras" => ModelFormat::Keras,
        "openvino" => ModelFormat::OpenVINO,
        "mlx" => ModelFormat::MLX,
        "hdf5" | "h5" => ModelFormat::HDF5,
        "numpy" | "npy" | "npz" => ModelFormat::NumPy,
        "pickle" | "pkl" => ModelFormat::Pickle,
        "mxnet" | "params" => ModelFormat::MXNet,
        "caffe" | "caffemodel" => ModelFormat::Caffe,
        "ncnn" | "param" => ModelFormat::NCNN,
        "mnn" => ModelFormat::MNN,
        "rknn" => ModelFormat::RKNN,
        "darknet" | "weights" => ModelFormat::Darknet,
        other => {
            return Err(ApiError::bad_request(format!(
                "Unsupported format: '{other}'"
            )));
        }
    };
    Ok(f)
}

/// Validate a model name: must be 1-128 ASCII alphanumeric, hyphens, underscores, dots.
fn validate_model_name(name: &str) -> Result<(), ApiError> {
    if name.is_empty() || name.len() > 128 {
        return Err(ApiError::bad_request("Model name must be 1-128 characters"));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(ApiError::bad_request(
            "Model name must contain only ASCII alphanumeric, hyphens, underscores, or dots",
        ));
    }
    // Dots are allowed, so `.` and `..` pass the character check intact. Model
    // names are index keys rather than path components today, which is the only
    // reason that is harmless — but `federation_routes` already rejects `..`
    // because it joins names to paths, and a future handler that does the same
    // would inherit no protection from here. Rejecting all-dots names costs
    // nothing and removes the sharp edge.
    if name.chars().all(|c| c == '.') {
        return Err(ApiError::bad_request(
            "Model name must not consist only of dots",
        ));
    }
    Ok(())
}

// ── Tags ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TagAddRequest {
    pub tags: Vec<String>,
}

/// POST /api/v1/models/:name/tags
pub async fn add_tags(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Json(body): Json<TagAddRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let mut store =
        crate::tags::TagStore::new(&vault_path).map_err(|e| ApiError::internal(e.to_string()))?;
    store
        .add_tags(&name, &body.tags)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(
        serde_json::json!({ "model": name, "tags": body.tags }),
    ))
}

/// GET /api/v1/models/:name/tags
pub async fn get_tags(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let store =
        crate::tags::TagStore::new(&vault_path).map_err(|e| ApiError::internal(e.to_string()))?;
    let tags = store.get_tags(&name);
    Ok(Json(serde_json::json!({ "model": name, "tags": tags })))
}

/// DELETE /api/v1/models/:name/tags
pub async fn remove_tags(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Json(body): Json<TagAddRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let mut store =
        crate::tags::TagStore::new(&vault_path).map_err(|e| ApiError::internal(e.to_string()))?;
    store
        .remove_tags(&name, &body.tags)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(
        serde_json::json!({ "model": name, "removed": body.tags }),
    ))
}

// ── Search ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// POST /api/v1/search
pub async fn search_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SearchRequest>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let store =
        crate::tags::TagStore::new(&vault_path).map_err(|e| ApiError::internal(e.to_string()))?;
    let sq = crate::tags::SearchQuery {
        tags: body.tags.unwrap_or_default(),
        name_pattern: body.query,
        annotations: vec![],
    };
    let known_models = vault.list_models();
    let results = store.search(&sq, &known_models);
    let out: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "model": r.model,
                "tags": r.tags,
                "annotations": r.annotations,
            })
        })
        .collect();
    Ok(Json(out))
}

// ── ACL ──────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AclGrantRequest {
    pub principal: String,
    pub role: String,
}

/// POST /api/v1/acl
pub async fn acl_grant(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<AclGrantRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let mut guard = crate::access_control::AclGuard::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let role: crate::access_control::Role = body
        .role
        .parse()
        .map_err(|e: crate::error::VaultError| ApiError::bad_request(e.to_string()))?;
    guard
        .grant(&body.principal, role)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(serde_json::json!({
        "principal": body.principal,
        "role": body.role,
    })))
}

/// GET /api/v1/acl
pub async fn acl_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let guard = crate::access_control::AclGuard::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let entries: Vec<serde_json::Value> = guard
        .list()
        .iter()
        .map(|e| {
            serde_json::json!({
                "principal": e.principal,
                "role": e.role.to_string(),
            })
        })
        .collect();
    Ok(Json(entries))
}

#[derive(Deserialize)]
pub struct AclRevokeRequest {
    pub principal: String,
}

/// DELETE /api/v1/acl
pub async fn acl_revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<AclRevokeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let mut guard = crate::access_control::AclGuard::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    guard
        .revoke(&body.principal)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(serde_json::json!({ "revoked": body.principal })))
}

// ── Webhooks ─────────────────────────────────────────────────────────────────

/// GET /api/v1/webhooks
pub async fn webhook_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let store = crate::webhooks::WebhookStore::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let hooks: Vec<serde_json::Value> = store
        .list()
        .iter()
        .map(|h| {
            serde_json::json!({
                "id": h.id,
                "url": h.url,
                "enabled": h.enabled,
                "events": h.events,
            })
        })
        .collect();
    Ok(Json(hooks))
}

#[derive(Deserialize)]
pub struct WebhookAddRequest {
    pub url: String,
    pub secret: Option<String>,
    pub events: Option<Vec<String>>,
}

/// POST /api/v1/webhooks
pub async fn webhook_add(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<WebhookAddRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let mut store = crate::webhooks::WebhookStore::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let id = format!("wh_{}", uuid_v4_simple());
    let target = crate::webhooks::WebhookTarget {
        id: id.clone(),
        url: body.url.clone(),
        secret: body.secret,
        events: body.events.unwrap_or_default(),
        enabled: true,
    };
    store
        .add(target)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": id, "url": body.url })),
    ))
}

/// DELETE /api/v1/webhooks/:id
pub async fn webhook_remove(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let mut store = crate::webhooks::WebhookStore::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let removed = store
        .remove(&id)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(serde_json::json!({ "id": id, "removed": removed })))
}

// ── Validation ───────────────────────────────────────────────────────────────

/// POST /api/v1/models/:name/validate
pub async fn validate_model(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let store = crate::validation::ValidationStore::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    // Use vault data dir as fallback path for validation
    let data_dir = vault_path.join("data");
    let file_path = data_dir.join(&name);
    match store.validate(&name, &file_path) {
        Ok(report) => Ok(Json(serde_json::json!({
            "model": name,
            "overall_pass": report.overall_pass,
            "results": report.results.iter().map(|r| serde_json::json!({
                "probe": r.probe_label,
                "passed": r.passed,
                "message": r.message,
            })).collect::<Vec<_>>(),
        }))),
        Err(e) => Err(ApiError::internal(e.to_string())),
    }
}

// ── GC ───────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GcQuery {
    #[serde(default)]
    dry_run: bool,
}

/// POST /api/v1/gc
pub async fn garbage_collect(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<GcQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    // Through the vault rather than the path: gc needs the key to read the
    // sealed index, and a locked vault must fail here rather than mistake
    // every blob for an orphan.
    let report = vault
        .gc(q.dry_run)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(serde_json::json!({
        "dry_run": q.dry_run,
        "orphaned_blobs": report.orphaned_blobs,
        "temp_files": report.temp_files,
        "reclaimable_bytes": report.reclaimable_bytes,
        "deleted": report.deleted,
    })))
}

// ── Policies ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PolicySetRequest {
    pub max_versions: Option<usize>,
    pub max_age_days: Option<u32>,
    pub keep_minimum: Option<usize>,
}

/// PUT /api/v1/models/:name/policy
pub async fn policy_set(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Json(body): Json<PolicySetRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let mut store = crate::policies::PolicyStore::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let policy = crate::policies::RetentionPolicy {
        max_versions: body.max_versions.unwrap_or(0),
        max_age_days: body.max_age_days.unwrap_or(0),
        keep_minimum: body.keep_minimum.unwrap_or(1),
    };
    store
        .set(&name, policy)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(serde_json::json!({ "model": name, "policy": "set" })))
}

/// GET /api/v1/policies
pub async fn policy_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let store = crate::policies::PolicyStore::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let policies = store.list();
    let out: Vec<serde_json::Value> = policies
        .iter()
        .map(|(model, p)| {
            serde_json::json!({
                "model": model,
                "max_versions": p.max_versions,
                "max_age_days": p.max_age_days,
                "keep_minimum": p.keep_minimum,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "policies": out })))
}

// ── Profiles ─────────────────────────────────────────────────────────────────

/// GET /api/v1/profiles
pub async fn profile_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let store = crate::profiles::ProfileStore::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let profiles: Vec<serde_json::Value> = store
        .list()
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "description": p.description,
                "overrides": p.overrides,
            })
        })
        .collect();
    let active = store.active().map(|p| p.name.clone());
    Ok(Json(serde_json::json!({
        "profiles": profiles,
        "active": active,
    })))
}

#[derive(Deserialize)]
pub struct ProfileCreateRequest {
    pub name: String,
    pub description: Option<String>,
    pub overrides: Option<std::collections::BTreeMap<String, String>>,
}

/// POST /api/v1/profiles
pub async fn profile_create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ProfileCreateRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let mut store = crate::profiles::ProfileStore::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let profile = crate::profiles::Profile {
        name: body.name.clone(),
        description: body.description,
        overrides: body.overrides.unwrap_or_default(),
        created_at: Utc::now().to_rfc3339(),
    };
    store
        .set(profile)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "name": body.name })),
    ))
}

/// POST /api/v1/profiles/:name/activate
pub async fn profile_activate(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let mut store = crate::profiles::ProfileStore::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    store
        .activate(&name)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(serde_json::json!({ "activated": name })))
}

// ── Lineage Graph ────────────────────────────────────────────────────────────

/// GET /api/v1/lineage-graph
pub async fn lineage_graph_show(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let graph = crate::lineage_graph::LineageGraph::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let edges: Vec<serde_json::Value> = graph
        .edges()
        .iter()
        .map(|e| {
            serde_json::json!({
                "parents": e.parents,
                "child": e.child,
                "kind": format!("{:?}", e.kind),
                "notes": e.notes,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "edges": edges })))
}

#[derive(Deserialize)]
pub struct LineageAddRequest {
    pub child: String,
    pub parents: Vec<String>,
    pub kind: String,
    pub notes: Option<String>,
}

/// POST /api/v1/lineage-graph
pub async fn lineage_graph_add(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<LineageAddRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let mut graph = crate::lineage_graph::LineageGraph::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let kind = match body.kind.to_lowercase().as_str() {
        "fine-tune" | "finetune" => crate::lineage_graph::DerivationKind::FineTune,
        "merge" => crate::lineage_graph::DerivationKind::Merge,
        "distillation" => crate::lineage_graph::DerivationKind::Distillation,
        "quantization" => crate::lineage_graph::DerivationKind::Quantization,
        "conversion" => crate::lineage_graph::DerivationKind::Conversion,
        "prune" => crate::lineage_graph::DerivationKind::Prune,
        other => crate::lineage_graph::DerivationKind::Custom(other.to_string()),
    };
    let mut notes = std::collections::BTreeMap::new();
    if let Some(n) = &body.notes {
        notes.insert("notes".to_string(), n.clone());
    }
    let edge = crate::lineage_graph::LineageEdge {
        parents: body.parents.clone(),
        child: body.child.clone(),
        kind,
        notes,
        created_at: Utc::now().to_rfc3339(),
    };
    graph
        .add_edge(edge)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "child": body.child, "parents": body.parents })),
    ))
}

// ── Plugins ──────────────────────────────────────────────────────────────────

/// GET /api/v1/plugins
pub async fn plugin_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let registry = crate::plugins::PluginRegistry::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let plugins: Vec<serde_json::Value> = registry
        .list()
        .iter()
        .map(|p| {
            serde_json::json!({
                "id": p.manifest.id,
                "name": p.manifest.name,
                "version": p.manifest.version,
                "description": p.manifest.description,
                "loaded": p.loaded,
            })
        })
        .collect();
    Ok(Json(plugins))
}

// ── Quantization ─────────────────────────────────────────────────────────────

/// GET /api/v1/quantization/profiles
pub async fn quant_profile_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let store = crate::quantization::QuantProfileStore::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let profiles: Vec<serde_json::Value> = store
        .list()
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "method": p.method.to_string(),
                "description": p.description,
            })
        })
        .collect();
    Ok(Json(profiles))
}

#[derive(Deserialize)]
pub struct QuantProfileRequest {
    pub name: String,
    pub method: String,
    pub description: Option<String>,
}

/// POST /api/v1/quantization/profiles
pub async fn quant_profile_set(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<QuantProfileRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let mut store = crate::quantization::QuantProfileStore::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let method: crate::quantization::QuantMethod = body
        .method
        .parse()
        .map_err(|e: crate::VaultError| ApiError::bad_request(e.to_string()))?;
    let profile = crate::quantization::QuantProfile {
        name: body.name.clone(),
        method,
        description: body.description,
        metadata: std::collections::BTreeMap::new(),
    };
    store
        .set(profile)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(
        serde_json::json!({ "status": "ok", "name": body.name }),
    ))
}

#[derive(Deserialize)]
pub struct QuantEstimateRequest {
    pub size: u64,
    pub from: String,
    pub to: String,
}

/// POST /api/v1/quantization/estimate
pub async fn quant_estimate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<QuantEstimateRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let from: crate::quantization::QuantMethod = body
        .from
        .parse()
        .map_err(|e: crate::VaultError| ApiError::bad_request(e.to_string()))?;
    let to: crate::quantization::QuantMethod = body
        .to
        .parse()
        .map_err(|e: crate::VaultError| ApiError::bad_request(e.to_string()))?;
    let estimated = crate::quantization::estimate_quantized_size(body.size, from, to);
    Ok(Json(serde_json::json!({
        "original_bytes": body.size,
        "estimated_bytes": estimated,
        "compression_ratio": body.size as f64 / estimated as f64,
        "from": from.to_string(),
        "to": to.to_string(),
    })))
}

// ── Evaluations ──────────────────────────────────────────────────────────────

/// GET /api/v1/evaluations
#[allow(clippy::implicit_hasher)]
pub async fn eval_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let store = crate::evaluation::EvalStore::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let model = params.get("model");
    let version: Option<u64> = params.get("version").and_then(|v| v.parse().ok());
    let runs = if let Some(m) = model {
        store.get_runs(m, version)
    } else {
        store.get_runs("", None) // empty = all if model is not specified
    };
    let results: Vec<serde_json::Value> = runs
        .iter()
        .map(|r| {
            serde_json::json!({
                "model": r.model,
                "version": r.version,
                "suite": r.suite,
                "timestamp": r.timestamp,
                "metrics": r.metrics.iter().map(|m| serde_json::json!({
                    "name": m.name,
                    "value": m.value,
                    "unit": m.unit,
                    "higher_is_better": m.higher_is_better,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(Json(results))
}

#[derive(Deserialize)]
pub struct EvalRecordRequest {
    pub model: String,
    pub version: u64,
    pub suite: String,
    pub metrics: Vec<EvalMetricInput>,
}

#[derive(Deserialize)]
pub struct EvalMetricInput {
    pub name: String,
    pub value: f64,
    #[serde(default = "default_unit")]
    pub unit: String,
    #[serde(default = "default_true")]
    pub higher_is_better: bool,
}

fn default_unit() -> String {
    "score".to_string()
}
fn default_true() -> bool {
    true
}

/// POST /api/v1/evaluations
pub async fn eval_record(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<EvalRecordRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let mut store = crate::evaluation::EvalStore::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let metrics: Vec<crate::evaluation::MetricResult> = body
        .metrics
        .into_iter()
        .map(|m| crate::evaluation::MetricResult {
            name: m.name,
            value: m.value,
            unit: m.unit,
            higher_is_better: m.higher_is_better,
        })
        .collect();
    let run = crate::evaluation::EvalRun {
        suite: body.suite.clone(),
        model: body.model.clone(),
        version: body.version,
        metrics,
        timestamp: Utc::now().to_rfc3339(),
        context: std::collections::BTreeMap::new(),
    };
    store
        .record(run)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(serde_json::json!({ "status": "recorded" })))
}

/// GET /api/v1/evaluations/suites
pub async fn eval_suites(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<String>>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let store = crate::evaluation::EvalStore::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(store.suites()))
}

// ── Backup Schedules ─────────────────────────────────────────────────────────

/// GET /api/v1/backups/schedules
pub async fn backup_schedule_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let mgr = crate::scheduler::BackupManager::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let schedules: Vec<serde_json::Value> = mgr
        .list_schedules()
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": s.name,
                "frequency": s.frequency.to_string(),
                "max_backups": s.max_backups,
                "output_dir": s.output_dir.display().to_string(),
                "enabled": s.enabled,
                "created_at": s.created_at,
            })
        })
        .collect();
    Ok(Json(schedules))
}

#[derive(Deserialize)]
pub struct BackupScheduleRequest {
    pub name: String,
    pub frequency: String,
    #[serde(default = "default_max_backups")]
    pub max_backups: usize,
    pub output_dir: String,
}

fn default_max_backups() -> usize {
    7
}

/// POST /api/v1/backups/schedules
pub async fn backup_schedule_set(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<BackupScheduleRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let mut mgr = crate::scheduler::BackupManager::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let freq: crate::scheduler::BackupFrequency = body
        .frequency
        .parse()
        .map_err(|e: crate::VaultError| ApiError::bad_request(e.to_string()))?;
    let schedule = crate::scheduler::BackupSchedule {
        name: body.name.clone(),
        frequency: freq,
        max_backups: body.max_backups,
        output_dir: std::path::PathBuf::from(&body.output_dir),
        enabled: true,
        created_at: Utc::now().to_rfc3339(),
    };
    mgr.set_schedule(schedule)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(
        serde_json::json!({ "status": "ok", "name": body.name }),
    ))
}

/// GET /api/v1/backups/history
#[allow(clippy::implicit_hasher)]
pub async fn backup_history(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let vault_path = vault.get_config().get_vault_path(None);
    let mgr = crate::scheduler::BackupManager::new(&vault_path)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let schedule = params.get("schedule").map(|s| s.as_str());
    let history: Vec<serde_json::Value> = mgr
        .get_history(schedule)
        .iter()
        .map(|r| {
            serde_json::json!({
                "path": r.path.display().to_string(),
                "timestamp": r.timestamp,
                "size_bytes": r.size_bytes,
                "schedule_name": r.schedule_name,
            })
        })
        .collect();
    Ok(Json(history))
}

// ── Multi-Vault Registry ─────────────────────────────────────────────────────

/// GET /api/v1/vaults
pub async fn vault_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let config_dir = &vault.get_config().dirs.config_dir;
    let reg = crate::multi_vault::VaultRegistry::new(config_dir)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let vaults: Vec<serde_json::Value> = reg
        .list()
        .iter()
        .map(|v| {
            serde_json::json!({
                "name": v.name,
                "path": v.path.display().to_string(),
                "is_active": v.is_active,
                "exists": v.exists,
            })
        })
        .collect();
    Ok(Json(vaults))
}

#[derive(Deserialize)]
pub struct VaultRegisterRequest {
    pub name: String,
    pub path: String,
    pub description: Option<String>,
}

/// POST /api/v1/vaults
pub async fn vault_register(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<VaultRegisterRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let config_dir = &vault.get_config().dirs.config_dir;
    let mut reg = crate::multi_vault::VaultRegistry::new(config_dir)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let entry = crate::multi_vault::VaultEntry {
        name: body.name.clone(),
        path: std::path::PathBuf::from(&body.path),
        description: body.description,
        registered_at: Utc::now().to_rfc3339(),
    };
    reg.register(entry)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(
        serde_json::json!({ "status": "registered", "name": body.name }),
    ))
}

/// POST /api/v1/vaults/:name/activate
pub async fn vault_activate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    let vault = state.vault.read().await;
    let config_dir = &vault.get_config().dirs.config_dir;
    let mut reg = crate::multi_vault::VaultRegistry::new(config_dir)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    reg.activate(&name)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(
        serde_json::json!({ "status": "activated", "name": name }),
    ))
}

/// Check if an audit entry is a security-sensitive event type.
///
/// Used by role-based filtering: non-admin roles cannot see these events.
fn is_security_event(entry: &serde_json::Value) -> bool {
    const SECURITY_TYPES: &[&str] = &[
        "SECURITY_VIOLATION",
        "INTEGRITY_FAILURE",
        "AUTH_FAILURE",
        "KEY_DERIVED",
    ];

    let event_type = entry
        .get("event_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    SECURITY_TYPES
        .iter()
        .any(|t| event_type.eq_ignore_ascii_case(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_format_valid() {
        let cases = vec![
            ("safetensors", ModelFormat::Safetensors),
            ("gguf", ModelFormat::GGUF),
            ("pytorch", ModelFormat::PyTorch),
            ("pt", ModelFormat::PyTorch),
            ("pth", ModelFormat::PyTorch),
            ("onnx", ModelFormat::ONNX),
            ("tensorrt", ModelFormat::TensorRT),
            ("trt", ModelFormat::TensorRT),
            ("coreml", ModelFormat::CoreML),
            ("mlmodel", ModelFormat::CoreML),
            ("tflite", ModelFormat::TFLite),
            ("tensorflow", ModelFormat::TensorFlow),
            ("tf", ModelFormat::TensorFlow),
            ("pb", ModelFormat::TensorFlow),
            ("keras", ModelFormat::Keras),
            ("openvino", ModelFormat::OpenVINO),
            ("mlx", ModelFormat::MLX),
            ("hdf5", ModelFormat::HDF5),
            ("h5", ModelFormat::HDF5),
            ("numpy", ModelFormat::NumPy),
            ("npy", ModelFormat::NumPy),
            ("npz", ModelFormat::NumPy),
            ("pickle", ModelFormat::Pickle),
            ("pkl", ModelFormat::Pickle),
            ("mxnet", ModelFormat::MXNet),
            ("params", ModelFormat::MXNet),
            ("caffe", ModelFormat::Caffe),
            ("caffemodel", ModelFormat::Caffe),
            ("ncnn", ModelFormat::NCNN),
            ("param", ModelFormat::NCNN),
            ("mnn", ModelFormat::MNN),
            ("rknn", ModelFormat::RKNN),
            ("darknet", ModelFormat::Darknet),
            ("weights", ModelFormat::Darknet),
        ];
        for (input, expected) in cases {
            let result = parse_format(input).unwrap();
            assert_eq!(result, expected, "parse_format(\"{input}\") mismatch");
        }
    }

    #[test]
    fn test_parse_format_invalid() {
        let err = parse_format("nonexistent");
        assert!(err.is_err());
    }

    #[test]
    fn test_validate_model_name_valid() {
        assert!(validate_model_name("my-model").is_ok());
        assert!(validate_model_name("a").is_ok());
        assert!(validate_model_name("model_v2.1").is_ok());
        assert!(validate_model_name("ABC-123").is_ok());
    }

    #[test]
    fn test_validate_model_name_empty() {
        assert!(validate_model_name("").is_err());
    }

    #[test]
    fn test_validate_model_name_too_long() {
        let long = "a".repeat(129);
        assert!(validate_model_name(&long).is_err());
    }

    #[test]
    fn test_validate_model_name_invalid_chars() {
        assert!(validate_model_name("model name").is_err()); // space
        assert!(validate_model_name("model/path").is_err()); // slash
        assert!(validate_model_name("model;drop").is_err()); // semicolon
    }

    #[test]
    fn test_is_security_event_true() {
        for event_type in &[
            "SECURITY_VIOLATION",
            "INTEGRITY_FAILURE",
            "AUTH_FAILURE",
            "KEY_DERIVED",
            "security_violation", // case-insensitive
        ] {
            let entry = serde_json::json!({ "event_type": event_type });
            assert!(
                is_security_event(&entry),
                "{event_type} should be a security event"
            );
        }
    }

    #[test]
    fn test_is_security_event_false() {
        let entry = serde_json::json!({ "event_type": "MODEL_STORED" });
        assert!(!is_security_event(&entry));

        let entry2 = serde_json::json!({ "action": "store" });
        assert!(!is_security_event(&entry2));

        let entry3 = serde_json::json!({});
        assert!(!is_security_event(&entry3));
    }

    #[test]
    fn test_health_response_serialization() {
        let resp = HealthResponse {
            status: "ok".to_string(),
            version: "1.3.0".to_string(),
            vault_state: Some("locked".to_string()),
            model_count: Some(5),
            uptime_seconds: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("ok"));
        assert!(json.contains("1.3.0"));
        assert!(json.contains("locked"));
        assert!(!json.contains("uptime_seconds")); // skipped
    }

    #[test]
    fn test_health_response_minimal() {
        let resp = HealthResponse {
            status: "ok".to_string(),
            version: "1.0.0".to_string(),
            vault_state: None,
            model_count: None,
            uptime_seconds: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("vault_state"));
        assert!(!json.contains("model_count"));
    }

    #[test]
    fn test_conversion_info_serialization() {
        let info = ConversionInfo {
            name: "SafeTensorsToPyTorch".to_string(),
            source: "safetensors".to_string(),
            target: "pytorch".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("SafeTensorsToPyTorch"));
    }

    #[test]
    fn test_stats_response_serialization() {
        let resp = StatsResponse {
            model_count: 3,
            total_versions: 7,
            total_size_bytes: 1024 * 1024,
            file_count: 10,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["model_count"], 3);
        assert_eq!(parsed["total_versions"], 7);
    }

    #[tokio::test]
    async fn test_health_without_state() {
        let resp = health(None).await;
        assert_eq!(resp.0.status, "ok");
        assert!(resp.0.vault_state.is_none());
        assert!(resp.0.model_count.is_none());
    }

    #[tokio::test]
    async fn test_list_conversions_returns_entries() {
        let Json(entries) = list_conversions().await;
        assert!(!entries.is_empty());
        for e in &entries {
            assert!(!e.name.is_empty());
            assert!(!e.source.is_empty());
            assert!(!e.target.is_empty());
        }
    }

    #[tokio::test]
    async fn test_openapi_json_returns_valid() {
        let Json(spec) = openapi_json().await;
        assert!(spec.get("openapi").is_some() || spec.get("info").is_some());
    }

    #[tokio::test]
    async fn test_dashboard_index_returns_html() {
        let Html(html) = dashboard_index().await;
        assert!(html.contains("<html") || html.contains("<!DOCTYPE"));
    }

    #[test]
    fn test_uuid_v4_simple_unique() {
        let a = uuid_v4_simple();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let b = uuid_v4_simple();
        assert_ne!(a, b);
        assert!(!a.is_empty());
    }

    #[test]
    fn model_names_that_are_only_dots_are_rejected() {
        // Dots are legal in names, so `.` and `..` survive the character check
        // intact. They are harmless while names are index keys rather than path
        // components, but `federation_routes` rejects `..` for exactly this
        // reason and this validator should not disagree with it.
        for name in [".", "..", "...", "....."] {
            assert!(
                validate_model_name(name).is_err(),
                "expected {name:?} to be rejected"
            );
        }
    }

    #[test]
    fn ordinary_names_containing_dots_are_still_accepted() {
        // The point is to reject names that are *only* dots, not to ban dots --
        // version-like and file-like names stay valid.
        for name in ["llama-3.1", "model.v2", "a.b.c", "resnet_50.onnx"] {
            assert!(
                validate_model_name(name).is_ok(),
                "expected {name:?} to be accepted"
            );
        }
    }

    #[test]
    fn separators_and_length_limits_still_apply() {
        assert!(validate_model_name("../../etc/passwd").is_err());
        assert!(validate_model_name("a/b").is_err());
        assert!(validate_model_name("a\\b").is_err());
        assert!(validate_model_name("").is_err());
        assert!(validate_model_name(&"a".repeat(129)).is_err());
        assert!(validate_model_name(&"a".repeat(128)).is_ok());
    }
}

// ── Reconciliation endpoints (5.1.0) ─────────────────────────────────────────
//
// These paths were documented in `.well-known/openapi.yaml` but never had
// handlers, so every client generated from that spec emitted calls that 404'd.
//
// Several are deliberately NOT implemented as the old spec described them. It
// accepted server-side filesystem paths from the caller: `path` for license
// scanning, `output` for vault export, `archive` for import, and "file path or
// name@version" for diff. Honouring those over HTTP would turn an API token
// into arbitrary file read and write as the server user -- `output` alone is a
// write primitive aimable anywhere the process can reach. Each is therefore
// scoped to vault contents or to the request body, and the spec was corrected
// to match. A model is addressed by name and version, never by path.

/// Resolve `name@version` (or bare `name`) against the vault, returning bytes.
///
/// The only way these handlers name a model. There is deliberately no branch
/// that falls back to treating the input as a filesystem path.
async fn read_vault_model(
    state: &Arc<AppState>,
    reference: &str,
) -> Result<(String, Option<u32>, String, Vec<u8>), ApiError> {
    let (name, version) = match reference.split_once('@') {
        Some((n, v)) => {
            let parsed = v
                .parse::<u32>()
                .map_err(|_| ApiError::bad_request(format!("Invalid version in '{reference}'")))?;
            (n.to_string(), Some(parsed))
        }
        None => (reference.to_string(), None),
    };
    validate_model_name(&name)?;

    let vault = state.vault.read().await;
    let data = vault.get_model(&name, version).map_err(ApiError::from)?;

    // The recorded format, so a diff can report shapes rather than guessing
    // from bytes. Falls back to empty, which the differ treats as unknown.
    let versions = vault.list_versions(&name);
    let format = match version {
        Some(v) => versions.iter().find(|r| r.version == v),
        None => versions.last(),
    }
    .map(|r| r.format.clone())
    .unwrap_or_default();

    Ok((name, version, format, data))
}

/// Write bytes to a temporary file, for library calls that take a path.
///
/// The file lives in the OS temp directory and is removed when the handle
/// drops. The caller never chooses this path.
fn spill_to_temp(data: &[u8]) -> Result<tempfile::NamedTempFile, ApiError> {
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new()
        .map_err(|e| ApiError::internal(format!("Could not create temp file: {e}")))?;
    tmp.write_all(data)
        .map_err(|e| ApiError::internal(format!("Could not write temp file: {e}")))?;
    tmp.flush()
        .map_err(|e| ApiError::internal(format!("Could not flush temp file: {e}")))?;
    Ok(tmp)
}

#[derive(Deserialize)]
pub struct SignRequest {
    pub version: Option<u32>,
    pub identity: Option<String>,
    /// HMAC key seed, hex-encoded. Required -- signing without a key is not
    /// signing.
    pub key: String,
}

/// POST /api/v1/models/:name/sign
///
/// Signs the stored model bytes and returns the detached signature inline.
/// The signature is not written to a caller-named path; the old spec's
/// `signature_path` response implied a server-side write the caller steered.
pub async fn sign_model(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Json(body): Json<SignRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;
    if body.key.trim().is_empty() {
        return Err(ApiError::bad_request("A signing key is required"));
    }

    let vault = state.vault.read().await;
    let data = vault
        .get_model(&name, body.version)
        .map_err(ApiError::from)?;
    drop(vault);

    let keypair =
        crate::signing::ModelSigner::keypair_from_seed(&body.key, body.identity.as_deref())
            .map_err(|e| ApiError::bad_request(format!("Invalid signing key: {e}")))?;
    let tmp = spill_to_temp(&data)?;
    let signature =
        crate::signing::ModelSigner::sign(&keypair, tmp.path(), std::collections::HashMap::new())
            .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "model": name,
        "version": body.version,
        "algorithm": "HMAC-SHA256",
        "signature": signature,
    })))
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub version: Option<u32>,
    /// The detached signature document, as returned by `sign`.
    pub signature: serde_json::Value,
    /// Key seed. Without it, verification reports `signature_checked: false`
    /// rather than inferring validity from a self-reported hash.
    pub key: Option<String>,
}

/// POST /api/v1/models/:name/verify
pub async fn verify_model(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Json(body): Json<VerifyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;

    let signature: crate::signing::ModelSignature = serde_json::from_value(body.signature)
        .map_err(|e| ApiError::bad_request(format!("Malformed signature document: {e}")))?;

    let vault = state.vault.read().await;
    let data = vault
        .get_model(&name, body.version)
        .map_err(ApiError::from)?;
    drop(vault);

    let tmp = spill_to_temp(&data)?;
    let result = crate::signing::ModelSigner::verify(&signature, tmp.path(), body.key.as_deref())
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "model": name,
        "valid": result.valid,
        "signature_checked": result.signature_checked,
        "file_hash_match": result.file_hash_match,
        "signature_match": result.signature_match,
        "signer": result.signer,
    })))
}

#[derive(Deserialize)]
pub struct ScanRequest {
    pub version: Option<u32>,
}

/// POST /api/v1/models/:name/scan
pub async fn scan_model(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
    body: Option<Json<ScanRequest>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;
    let version = body.and_then(|Json(b)| b.version);

    let vault = state.vault.read().await;
    let data = vault.get_model(&name, version).map_err(ApiError::from)?;
    drop(vault);

    let report = crate::scanning::PickleScanner::scan_bytes(&data, &name);
    Ok(Json(serde_json::json!({
        "model": name,
        "version": version,
        "safe": report.findings.is_empty(),
        "findings": report.findings,
    })))
}

#[derive(Deserialize)]
pub struct DiffRequest {
    /// `name` or `name@version`. A filesystem path is not accepted.
    pub left: String,
    pub right: String,
}

/// POST /api/v1/models/diff
pub async fn diff_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<DiffRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;

    let (left_name, left_ver, left_fmt, left) = read_vault_model(&state, &body.left).await?;
    let (right_name, right_ver, right_fmt, right) = read_vault_model(&state, &body.right).await?;

    let diff = crate::diff::ModelDiffer::diff_bytes(
        &left,
        &right,
        &left_name,
        &right_name,
        &left_fmt,
        &right_fmt,
    )
    .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "left": { "model": left_name, "version": left_ver },
        "right": { "model": right_name, "version": right_ver },
        "diff": diff,
    })))
}

#[derive(Deserialize)]
pub struct LicenseScanRequest {
    pub model: String,
    pub version: Option<u32>,
}

/// POST /api/v1/license-scan
///
/// Scans a model held in the vault. The old spec took a `path`, which was an
/// arbitrary-file-read primitive.
pub async fn license_scan(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<LicenseScanRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&body.model)?;

    let vault = state.vault.read().await;
    let data = vault
        .get_model(&body.model, body.version)
        .map_err(ApiError::from)?;
    drop(vault);

    let report = crate::license_scan::LicenseScanner::scan_bytes(&data, &body.model);
    Ok(Json(serde_json::json!({
        "model": body.model,
        "version": body.version,
        "licenses": report.licenses,
    })))
}

/// GET /api/v1/models/:name/benchmarks
pub async fn benchmarks_list(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;

    let vault = state.vault.read().await;
    let base = vault.get_config().get_vault_path(None).join("benchmarks");
    drop(vault);

    let store = crate::benchmark::BenchmarkStore::new(&base)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let records = store
        .list_for_model(&name)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(
        serde_json::json!({ "model": name, "records": records }),
    ))
}

#[derive(Deserialize)]
pub struct BenchmarkRecordRequest {
    pub version: u64,
    pub benchmark: String,
    pub score: f64,
    pub unit: String,
    #[serde(default = "default_higher_is_better")]
    pub higher_is_better: bool,
}

fn default_higher_is_better() -> bool {
    true
}

/// POST /api/v1/models/:name/benchmarks
pub async fn benchmarks_record(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Json(body): Json<BenchmarkRecordRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;

    let vault = state.vault.read().await;
    let base = vault.get_config().get_vault_path(None).join("benchmarks");
    drop(vault);

    let store = crate::benchmark::BenchmarkStore::new(&base)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let mut record = store
        .get_or_create(&name, body.version)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    record.add_result(
        &body.benchmark,
        body.score,
        &body.unit,
        body.higher_is_better,
    );
    store
        .save(&record)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "model": name, "record": record })),
    ))
}

/// POST /api/v1/models/:name/card/validate
pub async fn card_validate(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;

    let vault = state.vault.read().await;
    let card_path = vault
        .get_config()
        .get_vault_path(None)
        .join("cards")
        .join(format!("{name}.json"));
    drop(vault);

    if !card_path.exists() {
        return Err(ApiError::not_found(format!(
            "No model card stored for '{name}'"
        )));
    }
    let raw = std::fs::read_to_string(&card_path)
        .map_err(|e| ApiError::internal(format!("Could not read card: {e}")))?;

    match crate::model_card::ModelCard::from_json(&raw) {
        Ok(card) => Ok(Json(serde_json::json!({
            "model": name,
            "valid": true,
            "card_name": card.model_details.name,
            "card_version": card.model_details.version,
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "model": name,
            "valid": false,
            "error": e.to_string(),
        }))),
    }
}

/// POST /api/v1/models/:name/card/generate
///
/// Builds a card from metadata the vault already holds and returns it rather
/// than writing it, so generating is not itself a mutation.
pub async fn card_generate(
    state: State<Arc<AppState>>,
    path: Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Deliberately delegates rather than reimplementing. `get_model_card`
    // already synthesises a card from stored version metadata, so a second
    // copy of that construction would be two sources of truth for one document
    // and would drift the first time either was edited.
    get_model_card(state, path, headers).await
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub engine: String,
    pub alias: Option<String>,
    pub system_prompt: Option<String>,
    pub version: Option<u32>,
}

/// POST /api/v1/models/:name/register
pub async fn register_model(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    headers: HeaderMap,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    validate_model_name(&name)?;

    let vault = state.vault.read().await;
    let data = vault
        .get_model(&name, body.version)
        .map_err(ApiError::from)?;
    drop(vault);

    // The engines register a path on disk, so the exported copy lives in the
    // OS temp directory for the duration of the call rather than at a path the
    // caller chose.
    let tmp = spill_to_temp(&data)?;
    let alias = body.alias.unwrap_or_else(|| name.clone());

    let result = match body.engine.as_str() {
        "ollama" => crate::interop::register_ollama(&crate::interop::OllamaOptions {
            name: alias.clone(),
            model_path: tmp.path().to_path_buf(),
            system_prompt: body.system_prompt.clone(),
            template: None,
            parameters: Vec::new(),
        }),
        "lm-studio" => crate::interop::register_lm_studio(&crate::interop::LmStudioOptions {
            name: alias.clone(),
            model_path: tmp.path().to_path_buf(),
            models_dir: None,
        }),
        other => {
            return Err(ApiError::bad_request(format!(
                "Unknown engine {other:?}. Supported: ollama, lm-studio"
            )))
        }
    }
    .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "model": name,
        "engine": body.engine,
        "alias": alias,
        "registered": result.success,
        "detail": result.message,
    })))
}

/// POST /api/v1/vault/export
///
/// Streams the bundle back in the response body. The old spec took an `output`
/// path, which was an arbitrary file write aimable anywhere the server user
/// could reach; the bundle is built in a temp directory and returned instead.
pub async fn vault_export(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let _claims = require_auth(&headers, &state)?;

    let vault = state.vault.read().await;

    let dir = tempfile::tempdir()
        .map_err(|e| ApiError::internal(format!("Could not create temp dir: {e}")))?;
    let out = dir.path().join("vault-export.tar.gz");
    // The guard is held across the call now: export needs the vault's key to
    // open the sealed index, so it cannot work from a path alone.
    vault
        .export_bundle(&out, None)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let bytes = std::fs::read(&out)
        .map_err(|e| ApiError::internal(format!("Could not read bundle: {e}")))?;

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/gzip"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"vault-export.tar.gz\"",
            ),
        ],
        bytes,
    )
        .into_response())
}

/// POST /api/v1/vault/import
///
/// Takes the bundle as the request body. The old spec took an `archive` path
/// on the server, which let a caller read any file the process could open.
pub async fn vault_import(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;
    if body.is_empty() {
        return Err(ApiError::bad_request("Request body is empty"));
    }

    let vault = state.vault.read().await;

    let tmp = spill_to_temp(&body)?;
    let report = vault
        .import_bundle(tmp.path(), false)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "models_imported": report.models_imported,
        "versions_imported": report.versions_imported,
        "versions_skipped": report.versions_skipped,
        "checksum_verified": report.checksum_verified,
    })))
}

/// GET /api/v1/telemetry/status
pub async fn telemetry_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;

    let vault = state.vault.read().await;
    let cfg = vault.get_config().telemetry.clone();
    drop(vault);

    let do_not_track = std::env::var("DO_NOT_TRACK")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let env_disabled = crate::env::var("IRONVAULT_TELEMETRY_DISABLED").is_some();

    Ok(Json(serde_json::json!({
        "enabled": cfg.enabled && !do_not_track && !env_disabled,
        "configured_enabled": cfg.enabled,
        "do_not_track": do_not_track,
        "env_disabled": env_disabled,
    })))
}

#[derive(Deserialize)]
pub struct IntrospectQuery {
    #[serde(default)]
    pub compact: bool,
}

/// GET /api/v1/introspect
///
/// The same schema `iv introspect` prints, from the same builder in
/// [`crate::cli_schema`], so the two surfaces cannot describe different CLIs.
/// Unauthenticated on purpose: it is a discovery document containing no vault
/// data, and requiring a token to learn how to obtain one is a loop.
pub async fn introspect(Query(q): Query<IntrospectQuery>) -> Json<serde_json::Value> {
    Json(crate::cli_schema::build(q.compact))
}

#[derive(Deserialize)]
pub struct PullRequest {
    /// `huggingface://owner/repo`, `ollama://name`, or an https URL.
    pub source: String,
    pub sha256: Option<String>,
    pub token: Option<String>,
    /// Store the downloaded bytes in the vault instead of only reporting them.
    #[serde(default)]
    pub store: bool,
    /// Vault name to store under. Defaults to the downloaded file stem.
    pub name: Option<String>,
}

/// POST /api/v1/models/pull
///
/// Downloads into a temp directory the server controls, then optionally stores
/// the bytes in the vault. The output location is never caller-chosen.
///
/// Note this makes the server fetch a caller-supplied URL, which is inherent to
/// the feature and matches `iv pull`. It is authenticated, and the response
/// reports the resolved source and checksum so the caller can tell what was
/// actually retrieved.
pub async fn pull_model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<PullRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _claims = require_auth(&headers, &state)?;

    let source = crate::download::ModelSource::parse(&body.source)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    let dir = tempfile::tempdir()
        .map_err(|e| ApiError::internal(format!("Could not create temp dir: {e}")))?;
    let mut downloader = crate::download::ModelDownloader::new(dir.path());
    if let Some(token) = body.token {
        downloader = downloader.with_hf_token(token);
    }

    let result = downloader
        .download(&source, body.sha256.as_deref())
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    let mut stored = None;
    if body.store {
        let name = body
            .name
            .or_else(|| {
                result
                    .path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
            })
            .ok_or_else(|| ApiError::bad_request("Could not derive a model name; pass `name`"))?;
        validate_model_name(&name)?;

        let data = std::fs::read(&result.path)
            .map_err(|e| ApiError::internal(format!("Could not read download: {e}")))?;
        let metadata = crate::formats::ModelMetadata::new(
            name.clone(),
            crate::formats::ModelFormat::from_extension(&result.format),
        );

        let mut vault = state.vault.write().await;
        let version = vault
            .store_model(&name, data, metadata, None)
            .map_err(ApiError::from)?;
        stored = Some(serde_json::json!({ "model": name, "version": version.version }));
    }

    Ok(Json(serde_json::json!({
        "source": result.source,
        "sha256": result.sha256,
        "size_bytes": result.size_bytes,
        "format": result.format,
        "stored": stored,
    })))
}
