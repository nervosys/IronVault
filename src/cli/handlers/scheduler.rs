//! CLI handler for backup scheduling (iv backup).

use ironvault::scheduler::{BackupFrequency, BackupManager, BackupSchedule};
use ironvault::VaultConfig;
use ironvault::{Result, VaultError};

use crate::cli::args::BackupCommands;

pub fn handle_backup(command: BackupCommands, config: VaultConfig) -> Result<()> {
    let mut mgr = BackupManager::new(&config.dirs.vault_dir)?;

    match command {
        BackupCommands::Set {
            name,
            frequency,
            max_backups,
            output_dir,
        } => {
            let freq: BackupFrequency = frequency.parse().map_err(|e: VaultError| e)?;
            let schedule = BackupSchedule {
                name: name.clone(),
                frequency: freq,
                max_backups,
                output_dir,
                enabled: true,
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            mgr.set_schedule(schedule)?;
            println!("Backup schedule '{}' set ({})", name, freq);
        }
        BackupCommands::Remove { name } => {
            if !mgr.remove_schedule(&name)? {
                return Err(VaultError::NotFound(format!("backup schedule '{name}'")));
            }
            println!("Schedule '{}' removed", name);
        }
        BackupCommands::List => {
            let schedules = mgr.list_schedules();
            if schedules.is_empty() {
                println!("No backup schedules.");
            } else {
                for s in schedules {
                    let status = if s.enabled { "enabled" } else { "disabled" };
                    println!(
                        "  {} — {} (max {}, {}) → {}",
                        s.name,
                        s.frequency,
                        s.max_backups,
                        status,
                        s.output_dir.display()
                    );
                }
            }
        }
        BackupCommands::History { schedule } => {
            let history = mgr.get_history(schedule.as_deref());
            if history.is_empty() {
                println!("No backup history.");
            } else {
                for r in &history {
                    println!(
                        "  [{}] {} — {} bytes (schedule: {})",
                        r.timestamp,
                        r.path.display(),
                        r.size_bytes,
                        r.schedule_name
                    );
                }
            }
        }
    }

    Ok(())
}
