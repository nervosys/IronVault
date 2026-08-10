//! Agent pipeline — end-to-end automated workflow over the Rust API.
//!
//! Demonstrates how an automated agent (or a CI job) would chain vault
//! capabilities to safely ingest, classify, and audit a model with no
//! human in the loop:
//!
//!     scan → store → tag → annotate → search → sign → verify
//!
//! Every step uses only the public crate API, so this is also a smoke
//! test of the surface that `.well-known/openapi.yaml` and
//! `.well-known/mcp-manifest.json` expose to remote agents.
//!
//! Run with:  `cargo run --example agent_pipeline`

use std::collections::HashMap;

use ironvault::formats::{ModelFormat, ModelMetadata};
use ironvault::scanning::PickleScanner;
use ironvault::signing::ModelSigner;
use ironvault::tags::{SearchQuery, TagStore};
use ironvault::{VaultBuilder, VaultConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== IronVault — Automated Agent Pipeline ===\n");

    // ── 0. Synthetic checkpoint on disk (stand-in for a real download) ────
    let tmp = tempfile::tempdir()?;
    let checkpoint = tmp.path().join("checkpoint.safetensors");
    let bytes: Vec<u8> = (0..8_192u32).map(|i| (i % 251) as u8).collect();
    std::fs::write(&checkpoint, &bytes)?;
    println!("0. Synthetic checkpoint written: {} bytes", bytes.len());

    // ── 1. Safety scan ────────────────────────────────────────────────────
    // Before storing anything, an agent must verify the artifact isn't a
    // pickle-bomb. ScanReport returns structured findings the agent can
    // reason about; here we gate on the recommendation field.
    println!("\n1. Safety scan (pickle opcodes / suspicious patterns)");
    let report = PickleScanner::scan(&checkpoint)?;
    println!(
        "   ✓ findings: {} · recommendation: {}",
        report.findings.len(),
        first_sentence(&report.recommendation),
    );
    let critical = report.summary.get("CRIT").copied().unwrap_or(0);
    if critical > 0 {
        eprintln!("   ✗ CRITICAL findings — agent should abort ingestion");
        return Err("scan failed".into());
    }

    // ── 2. Open / unlock a vault ──────────────────────────────────────────
    // Use an isolated vault dir for the demo so it never collides with the
    // user's real vault.
    let vault_dir = tmp.path().join("vault");
    std::fs::create_dir_all(&vault_dir)?;
    let mut config = VaultConfig::new()?;
    config.dirs.vault_dir = vault_dir.clone();
    config.dirs.data_dir = vault_dir.clone();
    config.dirs.log_dir = vault_dir.join("logs");
    std::fs::create_dir_all(&config.dirs.log_dir)?;

    let mut vault = VaultBuilder::new().config(config).build()?;
    // In CI an agent would source this from KMS / GitHub secrets, never inline.
    let passphrase = std::env::var("IRONVAULT_PASSPHRASE")
        .unwrap_or_else(|_| "demo-passphrase-not-for-production".to_string());
    vault.unlock(passphrase.into_bytes())?;
    println!(
        "\n2. Vault unlocked at {:?}",
        vault.get_config().dirs.vault_dir
    );

    // ── 3. Store the model with metadata ──────────────────────────────────
    let model_name = "agent-pipeline-demo";
    let metadata = ModelMetadata::new(model_name.to_string(), ModelFormat::Safetensors)
        .with_description("Stored by agent_pipeline example".to_string())
        .with_framework("synthetic".to_string())
        .with_task("smoke-test".to_string());

    let version = vault.store_model(model_name, bytes.clone(), metadata, None)?;
    println!(
        "\n3. Stored model: v{} · {} bytes → {} bytes ({}% compressed) · sha256={}…",
        version.version,
        version.size_bytes,
        version.compressed_size_bytes,
        ((1.0 - version.compressed_size_bytes as f64 / version.size_bytes as f64) * 100.0) as u32,
        &version.checksum_sha256[..12],
    );

    // ── 4. Tag and annotate ───────────────────────────────────────────────
    let mut tags = TagStore::new(&vault.get_config().dirs.vault_dir)?;
    tags.add_tags(
        model_name,
        &["production".to_string(), "smoke-test".to_string()],
    )?;
    tags.set_annotation(model_name, "owner", "agent@ci")?;
    tags.set_annotation(model_name, "scanned", "true")?;
    println!(
        "\n4. Tagged & annotated · tags={:?} · annotations={:?}",
        tags.get_tags(model_name),
        tags.get_annotations(model_name),
    );

    // ── 5. Tag-driven search (what `iv search --tag production` does) ────
    let query = SearchQuery {
        tags: vec!["production".to_string()],
        ..Default::default()
    };
    let hits = tags.search(&query, &vault.list_models());
    println!(
        "\n5. Search `tag=production` → {} match(es): {:?}",
        hits.len(),
        hits.iter().map(|h| &h.model).collect::<Vec<_>>(),
    );

    // ── 6. Provenance — sign the artifact, then verify ────────────────────
    let keypair = ModelSigner::generate_keypair(Some("agent@ci"))?;
    let mut sig_meta = HashMap::new();
    sig_meta.insert("model".to_string(), model_name.to_string());
    sig_meta.insert("version".to_string(), version.version.to_string());
    let signature = ModelSigner::sign(&keypair, &checkpoint, sig_meta)?;
    println!(
        "\n6. Signed by {} · sha256={}…",
        signature.signer.as_deref().unwrap_or("anonymous"),
        &signature.file_sha256[..12],
    );

    let verification = ModelSigner::verify(&signature, &checkpoint, Some(&keypair.secret_seed))?;
    println!(
        "   ✓ verify · valid={} · file_hash_match={} · signature_match={}",
        verification.valid, verification.file_hash_match, verification.signature_match,
    );
    if !verification.valid {
        return Err("signature verification failed".into());
    }

    // ── 7. Final audit summary an agent would emit ───────────────────────
    println!("\n─── Audit envelope (agent-emitted) ─────────────────────────────");
    let envelope = serde_json::json!({
        "model":      model_name,
        "version":    version.version,
        "sha256":     version.checksum_sha256,
        "scan_ok":    critical == 0,
        "tags":       tags.get_tags(model_name),
        "signed_by":  signature.signer,
        "verified":   verification.valid,
    });
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    println!("────────────────────────────────────────────────────────────────");

    Ok(())
}

fn first_sentence(s: &str) -> String {
    let trimmed: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    match trimmed.find('.') {
        Some(i) => trimmed[..=i].to_string(),
        None => trimmed,
    }
}
