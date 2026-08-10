//! Telemetry CLI command handlers.

use crate::cli::args::TelemetryCommands;
use ironvault::{telemetry, Result, VaultConfig};
use std::fs;

/// Handle telemetry commands
pub fn handle_telemetry(command: TelemetryCommands, mut config: VaultConfig) -> Result<()> {
    match command {
        TelemetryCommands::Status => show_status(&config),
        TelemetryCommands::Enable => enable_telemetry(&mut config),
        TelemetryCommands::Disable => disable_telemetry(&mut config),
        TelemetryCommands::Reset => reset_device_id(&mut config),
    }
}

fn show_status(config: &VaultConfig) -> Result<()> {
    println!("Telemetry Status");
    println!("================");
    println!();

    let enabled = config.telemetry.enabled && !is_env_disabled();
    println!(
        "Status:    {}",
        if enabled {
            "\x1b[32mEnabled\x1b[0m"
        } else {
            "\x1b[33mDisabled\x1b[0m"
        }
    );
    println!("Device ID: {}", &config.telemetry.device_id[..8]);

    if !config.telemetry.enabled {
        println!("\nTelemetry is disabled in config.");
    }
    if is_env_disabled() {
        println!("\nTelemetry is disabled via environment variable.");
    }

    println!("\nData collected (when enabled):");
    println!("  • Commands run (name only, no arguments)");
    println!("  • Feature usage");
    println!("  • Error types (no sensitive data)");
    println!("  • OS, architecture, version");
    println!("  • Model format and size bucket (small/medium/large)");

    println!("\nData NOT collected:");
    println!("  • Model contents or file data");
    println!("  • Passphrases or encryption keys");
    println!("  • File paths or model names");
    println!("  • Personal information");

    println!("\nTo opt out:");
    println!("  iv telemetry disable");
    println!("  # Or set IRONVAULT_TELEMETRY_ENABLED=false");
    println!("  # Or set IRONVAULT_TELEMETRY_DISABLED=1");
    println!("  # Or set DO_NOT_TRACK=1");

    Ok(())
}

fn enable_telemetry(config: &mut VaultConfig) -> Result<()> {
    config.telemetry.enabled = true;
    config.save()?;

    telemetry::init_default(Some(&config.dirs.config_dir))?;

    println!("✓ Telemetry enabled");
    println!("  Thank you for helping improve IronVault!");

    Ok(())
}

fn disable_telemetry(config: &mut VaultConfig) -> Result<()> {
    config.telemetry.enabled = false;
    config.save()?;

    telemetry::disable();

    // Also clear any pending events
    let queue_dir = config
        .dirs
        .cache_dir
        .parent()
        .map(|p| p.join("telemetry"))
        .unwrap_or_else(|| config.dirs.cache_dir.join("telemetry"));

    if queue_dir.exists() {
        let _ = fs::remove_dir_all(&queue_dir);
    }

    println!("✓ Telemetry disabled");
    println!("  No data will be collected or sent.");
    println!("  Any pending data has been deleted.");

    Ok(())
}

fn reset_device_id(config: &mut VaultConfig) -> Result<()> {
    let old_id = config.telemetry.device_id[..8].to_string();
    config.telemetry.device_id = uuid::Uuid::new_v4().to_string();
    config.save()?;

    println!("✓ Device ID reset");
    println!("  Old ID: {}...", old_id);
    println!("  New ID: {}...", &config.telemetry.device_id[..8]);

    Ok(())
}

fn is_env_disabled() -> bool {
    ironvault::env::var("IRONVAULT_TELEMETRY_ENABLED")
        .ok_or(())
        .map(|v| v.to_lowercase() == "false" || v == "0")
        .unwrap_or(false)
        || ironvault::env::var("IRONVAULT_TELEMETRY_DISABLED")
            .ok_or(())
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false)
        || std::env::var("DO_NOT_TRACK")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false)
}
