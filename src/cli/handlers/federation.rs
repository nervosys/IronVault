//! Federation command handlers — the client side of peer sync.

use ironvault::federation::{FederationManager, PeerConfig, SyncManifest};
use ironvault::federation_transport as transport;
use ironvault::formats::{ModelFormat, ModelMetadata};
use ironvault::{Result, VaultConfig, VaultError};

use crate::cli::args::FederationCommands;
use crate::cli::helpers::{build_vault, prompt_passphrase};

/// Build the manager from config, or explain why federation is unavailable.
fn manager(config: &VaultConfig) -> Result<FederationManager> {
    if !config.federation.enabled {
        return Err(VaultError::ConfigError(
            "federation is not enabled -- set federation.enabled = true in config.yaml \
             and add at least one peer. Enabling it also exposes \
             /api/v1/federation/* on `iv serve`."
                .to_string(),
        ));
    }

    let manager_config = transport::to_manager_config(&config.federation)?;
    let state_dir = config.dirs.data_dir.join("federation");
    FederationManager::new(manager_config, state_dir)
}

/// Look up a configured peer by node ID.
fn find_peer(mgr: &FederationManager, peer_id: &str) -> Result<PeerConfig> {
    mgr.peers()
        .iter()
        .find(|p| p.node_id == peer_id)
        .map(|p| PeerConfig {
            node_id: p.node_id.clone(),
            name: p.name.clone(),
            endpoint: p.endpoint.clone(),
            api_key: p.api_key.clone(),
            enabled: p.enabled,
        })
        .ok_or_else(|| {
            VaultError::ConfigError(format!(
                "no peer with node id '{peer_id}' -- see `iv federation status`"
            ))
        })
}

/// Collect this vault's models into the shape `generate_manifest` wants.
fn local_models(
    vault: &ironvault::vault::Vault,
) -> Vec<(String, Vec<ironvault::version::ModelVersion>)> {
    vault
        .list_models()
        .into_iter()
        .map(|name| {
            let versions = vault.list_versions(&name).into_iter().cloned().collect();
            (name, versions)
        })
        .collect()
}

pub fn handle_federation(
    command: FederationCommands,
    config: VaultConfig,
    use_sqlite: bool,
) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().map_err(VaultError::IoError)?;

    match command {
        FederationCommands::Status => rt.block_on(status(&config)),
        FederationCommands::Manifest => rt.block_on(manifest(config, use_sqlite)),
        FederationCommands::Plan { peer } => rt.block_on(sync(config, use_sqlite, &peer, true)),
        FederationCommands::Sync { peer, dry_run } => {
            rt.block_on(sync(config, use_sqlite, &peer, dry_run))
        }
    }
}

async fn status(config: &VaultConfig) -> Result<()> {
    let mgr = manager(config)?;
    let status = mgr.status().await;

    println!("Federation");
    println!("  Node:        {} ({})", status.node_name, status.node_id);
    println!("  Peers:       {}", status.peer_count);
    println!("  Models seen: {}", status.model_count);
    match status.last_sync {
        Some(ts) => println!("  Last sync:   {}", ts.to_rfc3339()),
        None => println!("  Last sync:   never"),
    }
    println!(
        "  Transfers:   {}",
        if config.federation.seal_transfers {
            "sealed (AIMVSEAL)"
        } else {
            "UNENCRYPTED — protected only by the transport"
        }
    );

    if !mgr.peers().is_empty() {
        println!("\nPeers:");
        for peer in mgr.peers() {
            println!(
                "  {} {}  {}  [{}]{}",
                if peer.enabled { "●" } else { "○" },
                peer.node_id,
                peer.endpoint,
                if peer.enabled { "enabled" } else { "disabled" },
                if peer.api_key.is_none() {
                    "  (no key — requests will be rejected)"
                } else {
                    ""
                },
            );
        }
    }

    let history = mgr.get_history(Some(5)).await;
    if !history.is_empty() {
        println!("\nRecent syncs:");
        for result in history {
            println!(
                "  {}  peer={}  ↓{} ↑{}  {}",
                result.timestamp.to_rfc3339(),
                result.peer_id,
                result.versions_downloaded,
                result.versions_uploaded,
                // `SyncResult` has no success flag; an empty error list is the
                // only thing that means the sync completed cleanly.
                if result.errors.is_empty() {
                    "ok"
                } else {
                    "FAILED"
                },
            );
        }
    }

    Ok(())
}

async fn manifest(config: VaultConfig, use_sqlite: bool) -> Result<()> {
    let mgr = manager(&config)?;
    let mut vault = build_vault(config, use_sqlite)?;
    vault.unlock(prompt_passphrase("Enter vault passphrase: ")?)?;

    let manifest = mgr.generate_manifest(local_models(&vault)).await;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}

async fn sync(config: VaultConfig, use_sqlite: bool, peer_id: &str, dry_run: bool) -> Result<()> {
    let mgr = manager(&config)?;
    let peer = find_peer(&mgr, peer_id)?;

    if !peer.enabled {
        return Err(VaultError::ConfigError(format!(
            "peer '{peer_id}' is disabled in config"
        )));
    }

    // Fail before touching the network if sealing is on without a passphrase --
    // otherwise the first transfer fails halfway through a sync.
    if config.federation.seal_transfers {
        transport::seal_passphrase()?;
    }

    let mut vault = build_vault(config.clone(), use_sqlite)?;
    vault.unlock(prompt_passphrase("Enter vault passphrase: ")?)?;

    let local = mgr.generate_manifest(local_models(&vault)).await;

    if dry_run {
        return plan(&mgr, &peer, &local).await;
    }

    // `sync_with_peer` drives the transfers and calls back for vault access.
    // Both closures are the only place plaintext model bytes exist in this
    // process, and both go through the transport helpers so a sealed transfer
    // cannot be bypassed by a call site.
    let federation_settings = config.federation.clone();
    let vault_ref = std::cell::RefCell::new(vault);

    let download_fn = |model: &str, checkpoint_id: &str| -> Result<Vec<u8>> {
        let vault = vault_ref.borrow();
        // Matched on the federation identity: a peer asks for the id the
        // version was created with, which is not this vault's local id when
        // the model itself arrived from elsewhere.
        let version = vault
            .list_versions(model)
            .into_iter()
            .find(|v| transport::federation_checkpoint_id(v) == checkpoint_id)
            .map(|v| v.version)
            .ok_or_else(|| {
                VaultError::ConfigError(format!("local version {model}/{checkpoint_id} vanished"))
            })?;
        let data = vault.get_model(model, Some(version))?;
        transport::seal_for_transit(&federation_settings, &data)
    };

    let upload_fn = |model: &str, checkpoint_id: &str, data: &[u8]| -> Result<()> {
        let plaintext = transport::open_from_transit(&federation_settings, data.to_vec())?;
        let mut vault = vault_ref.borrow_mut();

        if vault
            .list_versions(model)
            .iter()
            .any(|v| transport::federation_checkpoint_id(v) == checkpoint_id)
        {
            return Ok(());
        }

        let mut metadata =
            ModelMetadata::new(model.to_string(), ModelFormat::from_stored("unknown"))
                .with_description(format!("Received from federation peer ({checkpoint_id})"));
        // Carry the origin id so this copy keeps one identity across the
        // federation; without it the next sync re-downloads it.
        metadata.custom_fields = transport::origin_metadata(checkpoint_id);
        vault.store_model(model, plaintext, metadata, None)?;
        Ok(())
    };

    let result = mgr
        .sync_with_peer(&peer, &local, download_fn, upload_fn)
        .await?;

    println!("Sync with {} ({})", peer.name, peer.node_id);
    println!("  Downloaded: {}", result.versions_downloaded);
    println!("  Uploaded:   {}", result.versions_uploaded);
    println!("  Conflicts:  {}", result.conflicts.len());
    println!("  Duration:   {} ms", result.duration_ms);

    for conflict in &result.conflicts {
        println!(
            "  ⚠️  conflict on {} — local {} vs remote {} (from {}) — {:?}",
            conflict.model,
            conflict.local_version,
            conflict.remote_version,
            conflict.remote_node,
            conflict.resolution
        );
    }

    if !result.errors.is_empty() {
        println!("\nErrors:");
        for err in &result.errors {
            println!("  - {err}");
        }
    }

    if result.errors.is_empty() {
        Ok(())
    } else {
        Err(VaultError::IoError(std::io::Error::other(format!(
            "sync completed with {} error(s)",
            result.errors.len()
        ))))
    }
}

/// Report what a sync would move, without moving anything.
async fn plan(mgr: &FederationManager, peer: &PeerConfig, local: &SyncManifest) -> Result<()> {
    let remote = mgr.fetch_peer_manifest(peer).await?;
    let delta = mgr.compute_delta(local, &remote);

    println!("Sync plan for {} ({})", peer.name, peer.node_id);
    println!("  Would download: {}", delta.to_download.len());
    for item in &delta.to_download {
        println!(
            "    ↓ {}/{}  {} bytes",
            item.model, item.checkpoint_id, item.size_bytes
        );
    }
    println!("  Would upload:   {}", delta.to_upload.len());
    for item in &delta.to_upload {
        println!(
            "    ↑ {}/{}  {} bytes",
            item.model, item.checkpoint_id, item.size_bytes
        );
    }
    println!("  Conflicts:      {}", delta.conflicts.len());
    for conflict in &delta.conflicts {
        println!(
            "    ⚠️  {} — local {} vs remote {}",
            conflict.model, conflict.local_version, conflict.remote_version
        );
    }

    Ok(())
}
