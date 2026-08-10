//! Terminal UI — interactive model browser for the vault.
//!
//! Provides a simple text-based dashboard that displays vault models, versions,
//! storage stats, and tags in a formatted layout.  No external TUI framework
//! is required — output is plain ANSI text suitable for any terminal.

use std::path::Path;

use crate::error::Result;
use crate::tags::TagStore;
use crate::version::VersionControl;

// ── Public API ───────────────────────────────────────────────────────────────

/// Render a terminal-based vault dashboard to stdout.
pub fn browse(vault_path: &Path) -> Result<String> {
    let vc = VersionControl::new(vault_path)?;
    let tags = TagStore::new(vault_path).ok();

    let models = vc.list_models_owned();
    let mut out = String::new();

    // Header
    out.push('\n');
    out.push_str("  ╔══════════════════════════════════════════════════════════╗\n");
    out.push_str("  ║              IronVault — Dashboard                 ║\n");
    out.push_str("  ╚══════════════════════════════════════════════════════════╝\n");
    out.push('\n');

    // Summary
    let total_versions: usize = models.iter().map(|m| vc.list_versions(m).len()).sum();
    let total_size: u64 = models
        .iter()
        .flat_map(|m| vc.list_versions(m))
        .map(|v| v.size_bytes)
        .sum();

    out.push_str(&format!(
        "  Models: {}    Versions: {}    Total size: {}\n\n",
        models.len(),
        total_versions,
        format_bytes(total_size),
    ));

    if models.is_empty() {
        out.push_str("  (vault is empty)\n");
        return Ok(out);
    }

    // Table header
    out.push_str(&format!(
        "  {:<24} {:>5} {:<14} {:>12} {}\n",
        "Model", "Vers", "Format", "Size", "Tags"
    ));
    out.push_str(&format!("  {}\n", "─".repeat(72)));

    for model in &models {
        let versions = vc.list_versions(model);
        let latest = versions.iter().max_by_key(|v| v.version);
        let ver_count = versions.len();

        let (format_str, size) = if let Some(v) = latest {
            (v.format.clone(), v.size_bytes)
        } else {
            ("?".into(), 0)
        };

        let tag_str = if let Some(ref ts) = tags {
            let t = ts.get_tags(model);
            if t.is_empty() {
                String::new()
            } else {
                t.into_iter().collect::<Vec<_>>().join(", ")
            }
        } else {
            String::new()
        };

        out.push_str(&format!(
            "  {:<24} {:>5} {:<14} {:>12} {}\n",
            truncate(model, 24),
            ver_count,
            truncate(&format_str, 14),
            format_bytes(size),
            truncate(&tag_str, 20),
        ));
    }

    out.push('\n');
    Ok(out)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1_048_576), "1.0 MB");
        assert_eq!(format_bytes(1_073_741_824), "1.0 GB");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 6), "hello…");
    }

    #[test]
    fn test_browse_empty_vault() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("data")).unwrap();
        let output = browse(dir.path()).unwrap();
        assert!(output.contains("vault is empty"));
    }
}
