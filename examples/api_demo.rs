//! Example: REST API usage with IronVault
//!
//! Demonstrates how to start the API server and interact with it
//! programmatically using `reqwest`.
//!
//! # Running
//!
//! ```bash
//! cargo run --example api_demo --features "full,graphql"
//! ```

#[cfg(feature = "api")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ironvault::api::server::{create_router, AppState, RateLimiter};
    use ironvault::api::ApiConfig;
    use ironvault::config::VaultConfig;
    use ironvault::vault::Vault;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    println!("=== IronVault REST API Demo ===\n");

    // ── 1. Set up vault and API config ───────────────────────────────────
    println!("1. Setting up vault and API configuration...");
    let config = VaultConfig::new()?;
    let vault_config = config.clone();
    let vault = Vault::new(Some(config))?;

    // `ApiConfig` is `#[non_exhaustive]`: start from `default()` and override.
    let mut api_config = ApiConfig::default();
    api_config.port = 0; // OS picks a free port
    api_config.jwt_secret = "demo-secret-change-in-production".into();
    api_config.cors_permissive = true;

    let state = Arc::new(AppState {
        vault: RwLock::new(vault),
        config: api_config,
        auth_rate_limiter: RateLimiter::new(10, std::time::Duration::from_secs(60)),
        vault_config,
        federation: None,
    });

    // ── 2. Bind and start HTTP server ────────────────────────────────────
    println!("2. Starting API server...");
    let router = create_router(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let base = format!("http://{addr}/api/v1");
    println!("   ✓ Listening on {addr}\n");

    tokio::spawn(async move {
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });

    let client = reqwest::Client::new();

    // ── 3. Health check ──────────────────────────────────────────────────
    println!("3. Health check...");
    let resp: serde_json::Value = client
        .get(format!("{base}/health"))
        .send()
        .await?
        .json()
        .await?;
    println!("   ✓ Status: {}\n", resp["status"]);

    // ── 4. Authenticate and get JWT token ────────────────────────────────
    println!("4. Authenticating...");
    let resp: serde_json::Value = client
        .post(format!("{base}/auth/token"))
        .json(&serde_json::json!({ "passphrase": "demo-passphrase" }))
        .send()
        .await?
        .json()
        .await?;
    let token = resp["token"].as_str().expect("no token in response");
    println!(
        "   ✓ JWT token received (expires in {}s)\n",
        resp["expires_in"]
    );

    // ── 5. List models (empty vault) ─────────────────────────────────────
    println!("5. Listing models...");
    let resp: serde_json::Value = client
        .get(format!("{base}/models"))
        .bearer_auth(token)
        .send()
        .await?
        .json()
        .await?;
    println!("   ✓ Models: {resp}\n");

    // ── 6. Vault statistics ──────────────────────────────────────────────
    println!("6. Getting vault stats...");
    let resp: serde_json::Value = client
        .get(format!("{base}/stats"))
        .bearer_auth(token)
        .send()
        .await?
        .json()
        .await?;
    println!("   ✓ Model count: {}", resp["model_count"]);
    println!("   ✓ Total size:  {} bytes\n", resp["total_size"]);

    // ── 7. List conversion paths ─────────────────────────────────────────
    println!("7. Listing format conversions...");
    let resp: Vec<serde_json::Value> = client
        .get(format!("{base}/conversions"))
        .send()
        .await?
        .json()
        .await?;
    println!("   ✓ {} conversion paths available\n", resp.len());

    // ── 8. Compliance check ──────────────────────────────────────────────
    println!("8. Running compliance check...");
    let resp: serde_json::Value = client
        .get(format!("{base}/compliance"))
        .bearer_auth(token)
        .send()
        .await?
        .json()
        .await?;
    println!("   ✓ FIPS 140-3:   {}", resp["fips_140_3"]);
    println!("   ✓ CMMC 2.0:     {}", resp["cmmc_level_2"]);
    println!("   ✓ MITRE ATT&CK: {}\n", resp["mitre_attack"]);

    // ── 9. OpenAPI spec ──────────────────────────────────────────────────
    println!("9. Fetching OpenAPI spec...");
    let resp: serde_json::Value = client
        .get(format!("{base}/openapi.json"))
        .send()
        .await?
        .json()
        .await?;
    println!("   ✓ API version: {}", resp["info"]["version"]);
    println!(
        "   ✓ {} endpoints documented\n",
        resp["paths"].as_object().map(|p| p.len()).unwrap_or(0)
    );

    println!("=== Demo complete! ===");
    Ok(())
}

#[cfg(not(feature = "api"))]
fn main() {
    eprintln!("This example requires the `api` feature:");
    eprintln!("  cargo run --example api_demo --features api");
}
