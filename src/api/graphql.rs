//! GraphQL API for IronVault
//!
//! Provides a GraphQL interface alongside the REST API with:
//! - Queries: models, versions, lineage, stats
//! - Mutations: store, delete models
//!
//! Enable with the `graphql` feature flag.

use async_graphql::{
    Context, EmptySubscription, InputObject, Object, Result as GqlResult, Schema, SimpleObject, ID,
};
use axum::extract::State;
use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use super::auth;
use super::server::AppState;
use crate::formats::ModelFormat;

/// GraphQL schema type alias
pub type VaultSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

/// Build the GraphQL schema
pub fn build_schema(state: Arc<AppState>) -> VaultSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(state)
        .finish()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Types
// ═══════════════════════════════════════════════════════════════════════════════

/// Model information
#[derive(SimpleObject, Clone)]
pub struct Model {
    /// Model name
    pub name: String,
    /// Number of versions
    pub version_count: i32,
    /// Latest version number
    pub latest_version: Option<i32>,
    /// Total size across all versions (bytes)
    pub total_size: i64,
}

/// Model version details
#[derive(SimpleObject, Clone)]
pub struct ModelVersion {
    /// Version number
    pub version: i32,
    /// Unique checkpoint ID
    pub checkpoint_id: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Model format
    pub format: String,
    /// Size in bytes
    pub size_bytes: i64,
    /// SHA-256 checksum
    pub checksum: String,
    /// Parent version (for lineage)
    pub parent_version: Option<i32>,
}

/// Version lineage entry
#[derive(SimpleObject, Clone)]
pub struct LineageEntry {
    /// Version number
    pub version: i32,
    /// Checkpoint ID
    pub checkpoint_id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Parent checkpoint ID
    pub parent_id: Option<String>,
}

/// Vault statistics
#[derive(SimpleObject, Clone)]
pub struct VaultStats {
    /// Total number of models
    pub model_count: i32,
    /// Total number of versions
    pub version_count: i32,
    /// Total storage size (bytes)
    pub total_size: i64,
    /// Vault status (locked/unlocked)
    pub status: String,
}

/// Audit log entry
#[derive(SimpleObject, Clone)]
pub struct AuditEntry {
    /// Entry ID
    pub id: ID,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: String,
    /// Description
    pub description: String,
    /// Associated model name
    pub model_name: Option<String>,
    /// Success status
    pub success: bool,
}

/// Conversion capability
#[derive(SimpleObject, Clone)]
pub struct ConversionPath {
    /// Source format
    pub source: String,
    /// Target format
    pub target: String,
    /// Whether direct conversion is available
    pub direct: bool,
    /// Intermediate formats for multi-step conversion
    pub via: Vec<String>,
}

/// Store model input
#[derive(InputObject)]
pub struct StoreModelInput {
    /// Model name
    pub name: String,
    /// Model format (e.g., "safetensors", "onnx")
    pub format: String,
    /// Base64-encoded model data
    pub data_base64: String,
    /// Optional description
    pub description: Option<String>,
    /// Optional framework
    pub framework: Option<String>,
}

/// Convert model input
#[derive(InputObject)]
pub struct ConvertModelInput {
    /// Source model name
    pub name: String,
    /// Source version (latest if not specified)
    pub version: Option<i32>,
    /// Target format
    pub target_format: String,
    /// New model name (defaults to "{name}_{format}")
    pub new_name: Option<String>,
}

/// Store result
#[derive(SimpleObject)]
pub struct StoreResult {
    /// Model name
    pub name: String,
    /// Created version number
    pub version: i32,
    /// Checkpoint ID
    pub checkpoint_id: String,
    /// Size in bytes
    pub size_bytes: i64,
    /// Checksum
    pub checksum: String,
}

/// Delete result
#[derive(SimpleObject)]
pub struct DeleteResult {
    /// Whether deletion was successful
    pub success: bool,
    /// Number of versions deleted
    pub versions_deleted: i32,
}

/// Conversion result
#[derive(SimpleObject)]
pub struct ConversionResult {
    /// Whether conversion was successful
    pub success: bool,
    /// New model name
    pub new_name: String,
    /// New version number
    pub version: i32,
    /// Duration in milliseconds
    pub duration_ms: i64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Query Root
// ═══════════════════════════════════════════════════════════════════════════════

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get all models in the vault
    async fn models(&self, ctx: &Context<'_>) -> GqlResult<Vec<Model>> {
        let state = ctx.data::<Arc<AppState>>()?;
        let vault = state.vault.read().await;

        let models = vault
            .list_models()
            .into_iter()
            .map(|name| {
                let versions = vault.list_versions(&name);
                let latest = versions.iter().map(|v| v.version as i32).max();
                let total_size: i64 = versions.iter().map(|v| v.size_bytes as i64).sum();

                Model {
                    name,
                    version_count: versions.len() as i32,
                    latest_version: latest,
                    total_size,
                }
            })
            .collect();

        Ok(models)
    }

    /// Get a specific model by name
    async fn model(&self, ctx: &Context<'_>, name: String) -> GqlResult<Option<Model>> {
        let state = ctx.data::<Arc<AppState>>()?;
        let vault = state.vault.read().await;

        let models = vault.list_models();
        if !models.contains(&name) {
            return Ok(None);
        }

        let versions = vault.list_versions(&name);
        let latest = versions.iter().map(|v| v.version as i32).max();
        let total_size: i64 = versions.iter().map(|v| v.size_bytes as i64).sum();

        Ok(Some(Model {
            name,
            version_count: versions.len() as i32,
            latest_version: latest,
            total_size,
        }))
    }

    /// Get versions of a model
    async fn versions(&self, ctx: &Context<'_>, model: String) -> GqlResult<Vec<ModelVersion>> {
        let state = ctx.data::<Arc<AppState>>()?;
        let vault = state.vault.read().await;

        let versions = vault
            .list_versions(&model)
            .into_iter()
            .map(|v| ModelVersion {
                version: v.version as i32,
                checkpoint_id: v.checkpoint_id.clone(),
                created_at: v.timestamp,
                format: v.format.clone(),
                size_bytes: v.size_bytes as i64,
                checksum: v.checksum_sha256.clone(),
                parent_version: v.parent_version.map(|p| p as i32),
            })
            .collect();

        Ok(versions)
    }

    /// Get version lineage for a model
    async fn lineage(
        &self,
        ctx: &Context<'_>,
        model: String,
        version: i32,
    ) -> GqlResult<Vec<LineageEntry>> {
        let state = ctx.data::<Arc<AppState>>()?;
        let vault = state.vault.read().await;

        let lineage = vault
            .get_lineage(&model, version as u32)
            .into_iter()
            .map(|l| LineageEntry {
                version: l.version as i32,
                checkpoint_id: l.checkpoint_id.clone(),
                timestamp: l.timestamp,
                parent_id: l.parent_version.map(|v| format!("v{}", v)),
            })
            .collect();

        Ok(lineage)
    }

    /// Get vault statistics
    async fn stats(&self, ctx: &Context<'_>) -> GqlResult<VaultStats> {
        let state = ctx.data::<Arc<AppState>>()?;
        let vault = state.vault.read().await;

        let models = vault.list_models();
        let mut total_versions = 0;
        let mut total_size: i64 = 0;

        for model in &models {
            let versions = vault.list_versions(model);
            total_versions += versions.len();
            total_size += versions.iter().map(|v| v.size_bytes as i64).sum::<i64>();
        }

        Ok(VaultStats {
            model_count: models.len() as i32,
            version_count: total_versions as i32,
            total_size,
            status: if vault.is_unlocked() {
                "unlocked".into()
            } else {
                "locked".into()
            },
        })
    }

    /// Get audit log entries (returns empty - use REST API for audit access)
    async fn audit_log(
        &self,
        _ctx: &Context<'_>,
        #[graphql(default = 100)] _limit: i32,
    ) -> GqlResult<Vec<AuditEntry>> {
        // Audit log is internal to vault - use REST API /api/v1/audit for access
        Ok(Vec::new())
    }

    /// Get available format conversions
    async fn conversions(&self, _ctx: &Context<'_>) -> GqlResult<Vec<ConversionPath>> {
        // Return list of known conversions (static list)
        // Actual conversion uses the convert mutation
        let formats = [
            "safetensors",
            "pytorch",
            "onnx",
            "gguf",
            "tflite",
            "coreml",
            "tensorrt",
        ];

        let mut paths = Vec::new();
        for source in &formats {
            for target in &formats {
                if source != target {
                    paths.push(ConversionPath {
                        source: source.to_string(),
                        target: target.to_string(),
                        direct: true,
                        via: vec![],
                    });
                }
            }
        }

        Ok(paths)
    }

    /// Health check
    async fn health(&self) -> GqlResult<String> {
        Ok("ok".into())
    }

    /// API version
    async fn version(&self) -> GqlResult<String> {
        Ok(env!("CARGO_PKG_VERSION").into())
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Mutation Root
// ═══════════════════════════════════════════════════════════════════════════════

pub struct MutationRoot;

/// Verify JWT auth from the HTTP headers passed through GraphQL context.
fn require_gql_auth(ctx: &Context<'_>) -> GqlResult<()> {
    let state = ctx.data::<Arc<AppState>>()?;
    let headers = ctx.data::<HeaderMap>()?;

    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| async_graphql::Error::new("Missing Authorization header"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| async_graphql::Error::new("Invalid Authorization format"))?;

    if token.is_empty() || token.len() > 4096 || token.chars().any(|c| c.is_control()) {
        return Err(async_graphql::Error::new("Malformed token"));
    }

    auth::verify_token(token, &state.config.jwt_secret)
        .map_err(|_| async_graphql::Error::new("Invalid or expired token"))?;

    Ok(())
}

#[Object]
impl MutationRoot {
    /// Store a new model or version
    async fn store_model(
        &self,
        ctx: &Context<'_>,
        input: StoreModelInput,
    ) -> GqlResult<StoreResult> {
        require_gql_auth(ctx)?;
        use base64::{engine::general_purpose::STANDARD as B64, Engine};

        let state = ctx.data::<Arc<AppState>>()?;

        let data = B64
            .decode(&input.data_base64)
            .map_err(|e| async_graphql::Error::new(format!("Invalid base64: {e}")))?;

        // Accept both a format name ("PyTorch") and an extension ("pt"): storing
        // a Custom variant here would break conversion and diffing later.
        let format = ModelFormat::from_stored(&input.format);
        let mut metadata = crate::formats::ModelMetadata::new(input.name.clone(), format);

        if let Some(desc) = input.description {
            metadata = metadata.with_description(desc);
        }
        if let Some(fw) = input.framework {
            metadata = metadata.with_framework(fw);
        }

        let mut vault = state.vault.write().await;
        let version = vault.store_model(&input.name, data, metadata, None)?;

        Ok(StoreResult {
            name: input.name,
            version: version.version as i32,
            checkpoint_id: version.checkpoint_id,
            size_bytes: version.size_bytes as i64,
            checksum: version.checksum_sha256,
        })
    }

    /// Delete a model (all versions)
    async fn delete_model(&self, ctx: &Context<'_>, name: String) -> GqlResult<DeleteResult> {
        require_gql_auth(ctx)?;
        let state = ctx.data::<Arc<AppState>>()?;
        let mut vault = state.vault.write().await;

        let versions: Vec<u32> = vault
            .list_versions(&name)
            .iter()
            .map(|v| v.version)
            .collect();
        let count = versions.len();

        // Delete each version
        for version in versions {
            vault.delete_version(&name, version)?;
        }

        Ok(DeleteResult {
            success: true,
            versions_deleted: count as i32,
        })
    }

    /// Delete a specific version
    async fn delete_version(
        &self,
        ctx: &Context<'_>,
        name: String,
        version: i32,
    ) -> GqlResult<DeleteResult> {
        require_gql_auth(ctx)?;
        let state = ctx.data::<Arc<AppState>>()?;
        let mut vault = state.vault.write().await;

        vault.delete_version(&name, version as u32)?;

        Ok(DeleteResult {
            success: true,
            versions_deleted: 1,
        })
    }

    /// Convert a model to a different format
    /// Note: For full conversion support, use the REST API /api/v1/convert endpoint
    async fn convert_model(
        &self,
        ctx: &Context<'_>,
        _input: ConvertModelInput,
    ) -> GqlResult<ConversionResult> {
        require_gql_auth(ctx)?;
        // Conversion requires external converters and isn't available via GraphQL
        // Use the REST API /api/v1/convert endpoint instead
        Err(async_graphql::Error::new(
            "Model conversion is available via REST API at /api/v1/convert",
        ))
    }

    /// Unlock the vault
    async fn unlock(&self, ctx: &Context<'_>, passphrase: String) -> GqlResult<bool> {
        let state = ctx.data::<Arc<AppState>>()?;
        let mut vault = state.vault.write().await;
        vault.unlock(passphrase.into_bytes())?;
        Ok(true)
    }

    /// Lock the vault
    async fn lock(&self, ctx: &Context<'_>) -> GqlResult<bool> {
        require_gql_auth(ctx)?;
        let state = ctx.data::<Arc<AppState>>()?;
        let mut vault = state.vault.write().await;
        vault.lock();
        Ok(true)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Axum Integration
// ═══════════════════════════════════════════════════════════════════════════════

/// GraphQL handler for Axum — manually bridges axum 0.7 extractors
/// to async-graphql, avoiding axum version conflicts with async-graphql-axum.
pub async fn graphql_handler(
    State(schema): State<VaultSchema>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let request_str = match std::str::from_utf8(&body) {
        Ok(s) => s,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "Invalid UTF-8 in request body").into_response()
        }
    };

    let gql_request: async_graphql::Request = match serde_json::from_str(request_str) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Invalid GraphQL request: {e}"),
            )
                .into_response()
        }
    };

    let gql_request = gql_request.data(headers);
    let response = schema.execute(gql_request).await;
    let json = serde_json::to_string(&response).unwrap_or_default();

    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        json,
    )
        .into_response()
}

/// GraphQL Playground handler
pub async fn graphql_playground() -> impl axum::response::IntoResponse {
    axum::response::Html(async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/graphql"),
    ))
}
