//! Axum HTTP server for IronVault.
//!
//! Start with [`serve`] or build a router with [`create_router`].
//!
//! ## TLS / HTTPS
//!
//! This server binds plain HTTP by default. For production deployments,
//! terminate TLS at a reverse proxy (e.g., nginx, Caddy, AWS ALB) or use
//! `axum-server` with `rustls` for direct TLS termination. Never expose
//! the API over plain HTTP on untrusted networks.

use axum::routing::{delete, get, post, put};
use axum::Router;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::config::VaultConfig;
use crate::error::{Result, VaultError};
use crate::vault::Vault;

use super::routes;
use super::ApiConfig;

/// Shared application state.
pub struct AppState {
    /// Thread-safe vault handle.
    pub vault: RwLock<Vault>,
    /// API configuration.
    pub config: ApiConfig,
    /// Per-IP rate limiter for auth endpoints.
    pub auth_rate_limiter: RateLimiter,
    /// Vault configuration, kept for the settings the federation endpoints
    /// consult on every request (accepted peer keys, sealing).
    pub vault_config: VaultConfig,
    /// Federation manifest generator; `None` when federation is disabled.
    pub federation: Option<crate::federation::FederationManager>,
}

/// Simple sliding-window rate limiter keyed by IP address.
pub struct RateLimiter {
    /// Map of IP → (attempt count, window start).
    state: std::sync::Mutex<HashMap<std::net::IpAddr, (u32, Instant)>>,
    /// Maximum attempts per window.
    max_attempts: u32,
    /// Window duration.
    window: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter.
    pub fn new(max_attempts: u32, window: Duration) -> Self {
        Self {
            state: std::sync::Mutex::new(HashMap::new()),
            max_attempts,
            window,
        }
    }

    /// Check if the given IP is allowed. Returns `true` if under the limit.
    pub fn check(&self, ip: std::net::IpAddr) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();

        let entry = state.entry(ip).or_insert((0, now));

        // Reset window if expired
        if now.duration_since(entry.1) >= self.window {
            *entry = (0, now);
        }

        entry.0 += 1;
        entry.0 <= self.max_attempts
    }

    /// Prune expired entries to prevent unbounded memory growth.
    pub fn prune(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        state.retain(|_, (_, start)| now.duration_since(*start) < self.window);
    }
}

/// Build the axum [`Router`] with all API routes.
pub fn create_router(state: Arc<AppState>) -> Router {
    // CORS: only permissive when explicitly configured; default is restrictive
    let cors = if state.config.cors_permissive {
        CorsLayer::permissive()
    } else {
        // Restrictive CORS: no cross-origin requests allowed by default.
        // Configure allowed_origins in ApiConfig for specific trusted domains.
        CorsLayer::new()
    };

    let api = Router::new()
        .route("/health", get(routes::health))
        .route("/auth/token", post(routes::auth_token))
        .route("/auth/logout", post(routes::auth_logout))
        .route("/models", get(routes::list_models))
        .route(
            "/models/:name",
            get(routes::get_model).post(routes::store_model),
        )
        .route(
            "/models/:name/card",
            get(routes::get_model_card).post(routes::create_model_card),
        )
        .route("/models/:name/versions", get(routes::list_versions))
        .route(
            "/models/:name/versions/:version",
            get(routes::get_version).delete(routes::delete_version),
        )
        .route("/models/:name/lineage/:version", get(routes::get_lineage))
        .route("/conversions", get(routes::list_conversions))
        .route("/convert", post(routes::convert))
        .route("/compliance", get(routes::compliance))
        .route("/rag/search", post(routes::rag_search))
        .route("/rag/documents", post(routes::rag_add_document))
        .route("/stats", get(routes::stats))
        .route("/audit", get(routes::audit_log))
        .route("/metrics", get(routes::metrics))
        .route("/events", get(routes::events))
        .route("/openapi.json", get(routes::openapi_json))
        // v1.4.0 endpoints
        .route(
            "/models/:name/tags",
            get(routes::get_tags)
                .post(routes::add_tags)
                .delete(routes::remove_tags),
        )
        .route("/search", post(routes::search_models))
        .route(
            "/acl",
            get(routes::acl_list)
                .post(routes::acl_grant)
                .delete(routes::acl_revoke),
        )
        .route(
            "/webhooks",
            get(routes::webhook_list).post(routes::webhook_add),
        )
        .route("/webhooks/:id", delete(routes::webhook_remove))
        .route("/models/:name/validate", post(routes::validate_model))
        .route("/gc", post(routes::garbage_collect))
        .route("/models/:name/policy", put(routes::policy_set))
        .route("/policies", get(routes::policy_list))
        .route(
            "/profiles",
            get(routes::profile_list).post(routes::profile_create),
        )
        .route("/profiles/:name/activate", post(routes::profile_activate))
        .route(
            "/lineage-graph",
            get(routes::lineage_graph_show).post(routes::lineage_graph_add),
        )
        .route("/plugins", get(routes::plugin_list))
        // v1.5.0 endpoints
        .route(
            "/quantization/profiles",
            get(routes::quant_profile_list).post(routes::quant_profile_set),
        )
        .route("/quantization/estimate", post(routes::quant_estimate))
        .route(
            "/evaluations",
            get(routes::eval_list).post(routes::eval_record),
        )
        .route("/evaluations/suites", get(routes::eval_suites))
        .route(
            "/backups/schedules",
            get(routes::backup_schedule_list).post(routes::backup_schedule_set),
        )
        .route("/backups/history", get(routes::backup_history))
        .route(
            "/vaults",
            get(routes::vault_list).post(routes::vault_register),
        )
        .route("/vaults/:name/activate", post(routes::vault_activate))
        .with_state(state.clone());

    // Registered only when federation is enabled, so a default server does not
    // expose model-serving paths at all — an unauthenticated 404 rather than a
    // 401 that confirms the endpoint exists.
    let federation_routes = if state.vault_config.federation.enabled {
        Router::new()
            .route(
                "/federation/manifest",
                get(super::federation_routes::manifest),
            )
            .route(
                "/federation/models/:name/versions/:checkpoint_id",
                get(super::federation_routes::get_version)
                    .put(super::federation_routes::put_version),
            )
            .with_state(state.clone())
    } else {
        Router::new()
    };

    let api = api.merge(federation_routes);

    let dashboard = if state.config.enable_dashboard {
        Router::new().route("/", get(routes::dashboard_index))
    } else {
        Router::new()
    };

    #[cfg(feature = "graphql")]
    let graphql_routes = {
        use super::graphql;
        let schema = graphql::build_schema(state.clone());
        Router::new()
            .route(
                "/graphql",
                get(graphql::graphql_playground).post(graphql::graphql_handler),
            )
            .with_state(schema)
    };
    #[cfg(not(feature = "graphql"))]
    let graphql_routes = Router::new();

    dashboard
        .merge(graphql_routes)
        .nest("/api/v1", api)
        .layer(axum::middleware::from_fn(track_request))
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(state.config.max_body_size))
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(300),
        ))
        .layer(TraceLayer::new_for_http())
}

/// Record one API call: route template, method, status, and duration.
///
/// The endpoint reported is [`MatchedPath`] — the *template* axum matched,
/// such as `/api/v1/models/{name}` — and never `uri().path()`, which is the
/// resolved path and therefore contains the model name. That distinction is
/// the whole reason this is a middleware rather than something sprinkled
/// through the handlers: `MatchedPath` is only available here, and it is a
/// literal from the router table, so no request data can reach the event.
///
/// A request that matches no route (a 404) has no `MatchedPath`. It is
/// reported as the constant `"<no match>"` rather than the path the client
/// asked for, which is attacker-controlled and frequently a probe string.
async fn track_request(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::extract::MatchedPath;

    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map_or("<no match>", |m| m.as_str())
        .to_string();
    let method = req.method().as_str().to_string();

    let started = Instant::now();
    let response = next.run(req).await;

    crate::telemetry::track_api_call(
        &route,
        &method,
        response.status().as_u16(),
        started.elapsed(),
    );

    response
}

/// Minimum HS256 signing key length.
///
/// RFC 7518 §3.2: an HMAC key "of the same size as the hash output (for
/// instance, 256 bits for HS256) or larger MUST be used".
pub const MIN_JWT_SECRET_BYTES: usize = 32;

/// Reject a JWT signing secret too weak for HS256.
///
/// Tokens are signed with HS256, so anyone holding a single issued token can
/// brute-force a short secret offline and then mint tokens for any subject.
/// Only emptiness was checked before, which accepted `--jwt-secret hunter2`.
pub fn validate_jwt_secret(secret: &str) -> Result<()> {
    if secret.is_empty() {
        return Err(VaultError::ConfigError(
            "JWT secret must not be empty. Set --jwt-secret or IRONVAULT_JWT_SECRET.".into(),
        ));
    }

    if secret.len() < MIN_JWT_SECRET_BYTES {
        return Err(VaultError::ConfigError(format!(
            "JWT secret is {} bytes; HS256 requires at least {MIN_JWT_SECRET_BYTES} \
             (RFC 7518 §3.2). A shorter secret can be recovered offline from a \
             single issued token. Generate one with: openssl rand -base64 48",
            secret.len(),
        )));
    }

    Ok(())
}

/// Start the API server.
///
/// This is a blocking call that runs until the process is terminated.
pub async fn serve(vault_config: VaultConfig, api_config: ApiConfig) -> Result<()> {
    validate_jwt_secret(&api_config.jwt_secret)?;

    // Load persisted revocations before the listener binds, so no request can
    // be served against an empty list. A corrupt or unreadable store aborts
    // startup rather than starting with revocations silently dropped.
    if let Some(path) = &api_config.revocation_store {
        super::auth::configure_revocation_store(path).map_err(VaultError::IoError)?;
    } else {
        eprintln!(
            "warning: no revocation_store configured — revoked tokens will be \
             honoured only until this process restarts"
        );
    }

    // Build the federation manager before the vault takes ownership of the
    // config. Resolving peer keys here means a bad KMS reference aborts
    // startup instead of failing the first sync at 3am.
    let federation = if vault_config.federation.enabled {
        let manager_config =
            crate::federation_transport::to_manager_config(&vault_config.federation)?;
        let state_dir = vault_config.dirs.data_dir.join("federation");
        let peers = manager_config.peers.len();
        let manager = crate::federation::FederationManager::new(manager_config, state_dir)?;
        eprintln!(
            "federation: enabled, serving /api/v1/federation/* to {peers} configured peer(s)"
        );
        if !vault_config.federation.seal_transfers {
            eprintln!(
                "warning: federation.seal_transfers is off — models will be sent to peers \
                 unencrypted, protected only by the transport"
            );
        }
        Some(manager)
    } else {
        None
    };

    let state_config = vault_config.clone();
    let mut vault = Vault::new(Some(vault_config))?;

    // Unlock at startup when federation is on and a passphrase is available.
    //
    // Peers authenticate with the shared federation key and never hold the
    // vault passphrase, so without this every peer request 500s until a human
    // POSTs to /auth/token. Scoped to federation deliberately: a plain `iv
    // serve` keeps the existing behaviour of starting locked.
    if state_config.federation.enabled {
        match crate::federation_transport::startup_passphrase()? {
            Some(passphrase) => {
                vault.unlock(passphrase.as_bytes().to_vec())?;
                eprintln!("federation: vault unlocked at startup to serve peer requests");
            }
            None => eprintln!(
                "warning: federation is enabled but ${} is unset — the vault starts \
                 locked and peer requests will fail until it is unlocked via /auth/token",
                crate::federation_transport::VAULT_PASSPHRASE_ENV
            ),
        }
    }
    let state = Arc::new(AppState {
        vault: RwLock::new(vault),
        config: api_config.clone(),
        auth_rate_limiter: RateLimiter::new(5, Duration::from_secs(60)),
        vault_config: state_config,
        federation,
    });

    let router = create_router(state.clone()).into_make_service_with_connect_info::<SocketAddr>();

    // Spawn periodic cleanup of expired rate-limiter entries
    let limiter = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(300)).await;
            limiter.auth_rate_limiter.prune();
        }
    });

    let addr: SocketAddr = format!("{}:{}", api_config.host, api_config.port)
        .parse()
        .map_err(|e| VaultError::ConfigError(format!("Invalid bind address: {e}")))?;

    println!("IronVault API v{}", env!("CARGO_PKG_VERSION"));
    println!("  Listening on http://{}", addr);
    println!("  Dashboard:   http://{}/", addr);
    println!("  OpenAPI:     http://{}/api/v1/openapi.json", addr);
    println!();

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(VaultError::IoError)?;

    axum::serve(listener, router)
        .await
        .map_err(|e| VaultError::IoError(std::io::Error::other(e.to_string())))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        let ip: std::net::IpAddr = "127.0.0.1".parse().unwrap();
        assert!(limiter.check(ip));
        assert!(limiter.check(ip));
        assert!(limiter.check(ip));
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        let ip: std::net::IpAddr = "127.0.0.1".parse().unwrap();
        assert!(limiter.check(ip));
        assert!(limiter.check(ip));
        assert!(!limiter.check(ip)); // 3rd attempt blocked
    }

    #[test]
    fn test_rate_limiter_separate_ips() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        let ip1: std::net::IpAddr = "10.0.0.1".parse().unwrap();
        let ip2: std::net::IpAddr = "10.0.0.2".parse().unwrap();
        assert!(limiter.check(ip1));
        assert!(limiter.check(ip2));
        assert!(!limiter.check(ip1)); // ip1 blocked
        assert!(!limiter.check(ip2)); // ip2 blocked
    }

    #[test]
    fn test_rate_limiter_window_reset() {
        let limiter = RateLimiter::new(1, Duration::from_millis(1));
        let ip: std::net::IpAddr = "127.0.0.1".parse().unwrap();
        assert!(limiter.check(ip));
        assert!(!limiter.check(ip));
        std::thread::sleep(Duration::from_millis(5));
        assert!(limiter.check(ip)); // window expired, reset
    }

    #[test]
    fn test_rate_limiter_prune_expired() {
        let limiter = RateLimiter::new(5, Duration::from_millis(1));
        let ip: std::net::IpAddr = "127.0.0.1".parse().unwrap();
        limiter.check(ip);
        std::thread::sleep(Duration::from_millis(5));
        limiter.prune();
        // State should be empty after prune
        let state = limiter.state.lock().unwrap();
        assert!(state.is_empty());
    }

    #[test]
    fn test_rate_limiter_prune_keeps_active() {
        let limiter = RateLimiter::new(5, Duration::from_secs(60));
        let ip: std::net::IpAddr = "127.0.0.1".parse().unwrap();
        limiter.check(ip);
        limiter.prune();
        let state = limiter.state.lock().unwrap();
        assert_eq!(state.len(), 1); // still active
    }
    /// A short JWT secret must be refused at startup.
    ///
    /// Tokens are HS256; RFC 7518 §3.2 requires a key at least as large as
    /// the hash output. Only emptiness used to be checked, so
    /// `--jwt-secret hunter2` started a server whose secret could be
    /// recovered offline from one issued token.
    #[test]
    fn test_weak_jwt_secrets_are_rejected() {
        for weak in [
            "",
            "hunter2",
            "secret",
            "0123456789abcdef0123456789abcde", // 31 bytes — one short
        ] {
            let err = validate_jwt_secret(weak).expect_err(&format!("{weak:?} must be refused"));
            assert!(
                matches!(err, VaultError::ConfigError(_)),
                "expected ConfigError for {weak:?}, got {err:?}"
            );
        }
    }

    #[test]
    fn test_sufficient_jwt_secret_is_accepted() {
        let exact = "0123456789abcdef0123456789abcdef";
        assert_eq!(exact.len(), MIN_JWT_SECRET_BYTES);
        assert!(validate_jwt_secret(exact).is_ok());

        let longer = "0123456789abcdef0123456789abcdef0123456789";
        assert!(validate_jwt_secret(longer).is_ok());
    }

    /// The message must tell the operator what to do about it.
    #[test]
    fn test_weak_secret_error_is_actionable() {
        let err = validate_jwt_secret("short").unwrap_err().to_string();
        assert!(err.contains("RFC 7518"), "got: {err}");
        assert!(err.contains("openssl rand"), "got: {err}");
    }
}
