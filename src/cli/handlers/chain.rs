//! Blockchain audit trail command handlers.
//!
//! Every subcommand reads the chain; none of them write to it. Entries arrive
//! only through `AuditLogger::log`, so the trail cannot be appended to from the
//! command line -- which is the point of having it.

use ironvault::audit::AuditEventType;
use ironvault::blockchain::{AuditProof, BlockchainAudit};
use ironvault::{Result, VaultConfig, VaultError};

use crate::cli::args::ChainCommands;

/// Parse a CLI event-type string into an [`AuditEventType`].
///
/// The variants serialize as `SCREAMING_SNAKE_CASE`, so serde is the single
/// source of truth for the accepted spellings -- a hand-written match here
/// would drift the moment a variant is added.
fn parse_event_type(raw: &str) -> Result<AuditEventType> {
    let upper = raw.to_uppercase();
    serde_json::from_value(serde_json::Value::String(upper)).map_err(|_| {
        VaultError::ConfigError(format!(
            "unknown event type '{raw}' -- expected one of: MODEL_STORED, \
             MODEL_RETRIEVED, MODEL_DELETED, AUTH_SUCCESS, AUTH_FAILURE, \
             SECURITY_VIOLATION, VAULT_CREATED, CONFIG_CHANGED"
        ))
    })
}

/// Error returned when the chain was never turned on.
///
/// Distinguished from an empty chain on purpose: reporting height 0 for a
/// vault that was never recording would read as "nothing happened".
fn chain_disabled() -> VaultError {
    VaultError::ConfigError(
        "blockchain audit trail is not enabled for this vault -- set \
         security.blockchain_audit = true in config.toml (it also requires \
         security.audit_log = true). Entries are recorded from the moment it \
         is enabled; history written before then is not in the chain."
            .to_string(),
    )
}

pub fn handle_chain(command: ChainCommands, config: VaultConfig) -> Result<()> {
    // Opens the chain directly rather than through `Vault`. Going through the
    // vault would log a `VaultOpened` entry, so inspecting the trail would
    // append to it -- `iv chain verify` on a cron would grow the chain by a
    // block per run, and `verify` would report a different height than the
    // `status` printed a moment earlier. Reading evidence must not alter it.
    // It also means these commands need no passphrase.
    if !config.security.blockchain_audit {
        return Err(chain_disabled());
    }

    let chain_dir = config.get_audit_chain_dir();
    if !chain_dir.exists() {
        return Err(VaultError::ConfigError(format!(
            "blockchain audit trail is enabled but nothing has been recorded yet \
             ({} does not exist). It is created on the first audited operation.",
            chain_dir.display()
        )));
    }

    // `BlockchainAudit::new` would create a genesis block against an empty
    // directory; the check above keeps this to a pure open of an existing chain.
    let chain = BlockchainAudit::new(&chain_dir, config.security.blockchain_block_size)?;

    match command {
        ChainCommands::Status => status(&chain),
        ChainCommands::Verify => verify(&chain),
        ChainCommands::Proof {
            block,
            entry,
            output,
        } => proof(&chain, block, entry, output),
        ChainCommands::VerifyProof { proof: path } => verify_proof(&path),
        ChainCommands::Search {
            model,
            event,
            limit,
        } => search(&chain, model, event, limit),
    }
}

fn status(chain: &BlockchainAudit) -> Result<()> {
    println!("Blockchain audit trail");
    println!("  Height:      {} block(s)", chain.height());
    println!("  Block size:  {} entry/entries", chain.block_size());

    if let Some(latest) = chain.latest() {
        println!("  Latest:      #{} {}", latest.index, latest.hash);
        println!("  Entries:     {} in latest block", latest.entries.len());
    }

    let pending = chain.pending_count();
    println!("  Pending:     {pending} entry/entries");
    if pending > 0 {
        println!(
            "\n⚠️  {pending} entry/entries are in memory and not yet on disk. They are\n   \
             not covered by `iv chain verify` and would be lost if this process\n   \
             died now. They are written when the block fills (block size {}) or\n   \
             on clean exit.",
            chain.block_size()
        );
    }

    Ok(())
}

fn verify(chain: &BlockchainAudit) -> Result<()> {
    let result = chain.verify_chain();

    println!(
        "Verified {} of {} block(s)",
        result.blocks_verified, result.blocks_total
    );

    if result.valid {
        println!("✅ Chain intact — hash links, Merkle roots, and block hashes all check out");
        return Ok(());
    }

    println!("❌ Chain verification FAILED");
    for issue in &result.issues {
        println!("   - {issue}");
    }

    // A tampered audit trail is a security event, not a report. Exit non-zero
    // so a cron job or CI step treating this as a check actually fails.
    Err(VaultError::IntegrityError(format!(
        "audit chain verification failed with {} issue(s)",
        result.issues.len()
    )))
}

fn proof(
    chain: &BlockchainAudit,
    block: u64,
    entry: usize,
    output: Option<std::path::PathBuf>,
) -> Result<()> {
    let proof = chain.generate_proof(block, entry)?;
    let json = serde_json::to_string_pretty(&proof)?;

    match output {
        Some(path) => {
            std::fs::write(&path, json)?;
            println!("Proof written to {}", path.display());
        }
        None => println!("{json}"),
    }

    Ok(())
}

fn verify_proof(path: &std::path::Path) -> Result<()> {
    let contents = std::fs::read_to_string(path)?;
    let proof: AuditProof = serde_json::from_str(&contents)?;
    let result = BlockchainAudit::verify_proof(&proof);

    if result.valid {
        println!("✅ Proof valid");
        println!("   Entry is in block #{}", proof.block_index);
        println!("   Chain of custody reaches genesis {}", proof.genesis_hash);
        println!(
            "\nNote: this checks the proof's own consistency. It does not prove the\n\
             genesis hash belongs to your vault — compare it against\n`iv chain status` on the vault you trust."
        );
        return Ok(());
    }

    println!("❌ Proof INVALID");
    for issue in &result.issues {
        println!("   - {issue}");
    }

    Err(VaultError::IntegrityError(format!(
        "proof verification failed with {} issue(s)",
        result.issues.len()
    )))
}

fn search(
    chain: &BlockchainAudit,
    model: Option<String>,
    event: Option<String>,
    limit: usize,
) -> Result<()> {
    let event_type = event.as_deref().map(parse_event_type).transpose()?;

    let results = chain.search(model.as_deref(), event_type, None, None, limit)?;

    if results.is_empty() {
        println!("No matching entries.");
        return Ok(());
    }

    println!("{} matching entr(ies):\n", results.len());
    for (block_idx, entry_idx, entry) in &results {
        let audit = &entry.audit;
        println!(
            "  block {block_idx} entry {entry_idx}  {}  {:?}{}",
            audit.timestamp.to_rfc3339(),
            audit.event_type,
            if audit.success { "" } else { "  (FAILED)" }
        );
        println!("    {}", audit.description);
        println!("    proof: iv chain proof --block {block_idx} --entry {entry_idx}");
    }

    Ok(())
}
