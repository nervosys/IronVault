//! REST API server for IronVault
//!
//! Provides a network-accessible interface for vault management with:
//! - JWT-based authentication
//! - RESTful model/version CRUD
//! - Format conversion endpoints
//! - Audit log access
//! - OpenAPI specification
//! - Embedded web dashboard
//!
//! Enable with the `api` feature flag.

pub mod auth;
pub mod dashboard;
pub mod error;
pub mod federation_routes;
#[cfg(feature = "graphql")]
pub mod graphql;
pub mod openapi;
pub mod routes;
pub mod server;

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// API server configuration.
///
/// Construct with [`ApiConfig::default`] and assign the fields you need.
/// `#[non_exhaustive]` is deliberate: adding `revocation_store` broke every
/// downstream struct literal, and security settings will keep being added.
/// With this attribute, a future addition is a minor release rather than a
/// major one, and no caller silently ends up with a field they never
/// considered.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ApiConfig {
    /// Host address to bind to (default: "127.0.0.1").
    pub host: String,
    /// Port to listen on (default: 8080).
    pub port: u16,
    /// JWT secret key for token signing. Should be a strong random secret.
    pub jwt_secret: String,
    /// JWT token expiry in seconds (default: 3600 = 1 hour).
    pub token_expiry_secs: u64,
    /// Enable CORS for all origins (default: false).
    pub cors_permissive: bool,
    /// Maximum request body size in bytes (default: 512 MiB).
    pub max_body_size: usize,
    /// Enable the embedded web dashboard (default: true).
    pub enable_dashboard: bool,
    /// File the JWT revocation list is persisted to (default: none).
    ///
    /// Without it, revocation is process-local: restarting the server
    /// re-admits every revoked token that has not yet expired, so a logout is
    /// only honoured until the next deploy. Point this at durable storage —
    /// on Kubernetes, a volume that outlives the pod — to make revocations
    /// stick. See [`auth::configure_revocation_store`], which also documents
    /// why this does not extend across replicas.
    #[serde(default)]
    pub revocation_store: Option<std::path::PathBuf>,

    /// PEM certificate chain for in-process TLS. Required to bind a
    /// non-loopback address; see [`tls_key`](Self::tls_key).
    #[serde(default)]
    pub tls_cert: Option<std::path::PathBuf>,

    /// PEM private key matching [`tls_cert`](Self::tls_cert).
    ///
    /// The server refuses to bind anything but loopback without both of these.
    /// `POST /api/v1/auth/token` carries the vault *passphrase*, not merely a
    /// credential for a session — anyone who can read that request derives the
    /// vault key permanently, including against a copy of the vault taken
    /// later, and leaves no audit trail doing it. A revoked token expires; a
    /// disclosed passphrase does not.
    ///
    /// Loopback is exempt because the traffic never reaches a wire. If TLS is
    /// terminated by a reverse proxy, run the proxy on the same host and let
    /// this bind `127.0.0.1` — that is the configuration this exemption is for.
    /// A proxy on a *different* host means the hop to it is a real network hop
    /// and needs its own TLS, which is why there is no "trust me" flag here.
    #[serde(default)]
    pub tls_key: Option<std::path::PathBuf>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8080,
            jwt_secret: String::new(), // Must be set before serving
            token_expiry_secs: 3600,
            cors_permissive: false,
            max_body_size: 512 * 1024 * 1024,
            enable_dashboard: true,
            revocation_store: None,
            tls_cert: None,
            tls_key: None,
        }
    }
}

impl Drop for ApiConfig {
    fn drop(&mut self) {
        self.jwt_secret.zeroize();
    }
}
