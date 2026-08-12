//! Environment variable names, and the 4.x compatibility shim behind them.
//!
//! Before the IronVault rename this project read two unrelated prefixes:
//! `aimodelvault_*` (the old package name, lowercased) and `AIM_*` (the old
//! `aim` binary name, now `iv`). Both are gone in 5.0; everything is
//! `IRONVAULT_*`.
//!
//! A rename that silently stops reading a deployment's configuration is a bad
//! rename, so 5.x read the old spellings and warned once per variable that the
//! fallback would go in 6.0. **It is gone.** The old names are still detected,
//! and a variable set under one now produces a warning saying it has no effect
//! — the removal announces itself rather than quietly dropping a passphrase.
//!
//! Only the variable *name* is ever printed. Several of these carry
//! passphrases, so the value must never reach a log line.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use zeroize::{Zeroize, Zeroizing};

/// The single prefix for this project's own variables.
pub const PREFIX: &str = "IRONVAULT_";

/// Prefixes accepted for backward compatibility with 4.x, in priority order.
///
/// `aimodelvault_` was the package name; `AIM_` was the binary name. A given
/// variable used exactly one of them, so trying both is unambiguous.
const LEGACY_PREFIXES: [&str; 2] = ["aimodelvault_", "AIM_"];

/// Names already warned about, so a variable read in a loop warns once.
fn warned() -> &'static Mutex<HashSet<String>> {
    static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    WARNED.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Read a raw variable, treating empty and whitespace-only values as unset.
///
/// An exported-but-empty variable is how shells represent "I meant to set this
/// and didn't"; honouring it as a real value produces empty paths and empty
/// passphrases.
fn raw(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// The 4.x spellings of an `IRONVAULT_*` variable.
///
/// Returns nothing for a name that is not ours — `AWS_REGION` and
/// `DO_NOT_TRACK` are third-party or cross-vendor conventions and must be read
/// verbatim.
fn legacy_names(name: &str) -> Vec<String> {
    name.strip_prefix(PREFIX)
        .map(|suffix| {
            LEGACY_PREFIXES
                .iter()
                .map(|p| format!("{p}{suffix}"))
                .collect()
        })
        .unwrap_or_default()
}

/// Read an `IRONVAULT_*` variable.
///
/// Empty values count as unset. The 4.x spellings are no longer read; if one is
/// set and the current name is not, a one-time warning says so.
pub fn var(name: &str) -> Option<String> {
    if let Some(value) = raw(name) {
        return Some(value);
    }

    // 6.0 stopped reading the 4.x spellings, as 5.x warned it would. They are
    // still *detected*, because the alternative is a deployment whose
    // passphrase or JWT secret silently becomes unset on upgrade. A rename that
    // quietly drops configuration is what the 5.x shim existed to prevent, and
    // going silent at removal time would reintroduce it at the worst moment.
    for legacy in legacy_names(name) {
        if raw(&legacy).is_some() {
            warn_ignored(&legacy, name);
        }
    }

    None
}

/// Every `IRONVAULT_*` variable this project defines.
///
/// Only used by [`report_legacy`]. Test-only knobs are omitted deliberately —
/// they never existed in a 4.x deployment, so they have nothing to report.
const KNOWN: [&str; 13] = [
    "IRONVAULT_HOME",
    "IRONVAULT_CONFIG",
    "IRONVAULT_VAULT",
    "IRONVAULT_VAULT_DIR",
    "IRONVAULT_VAULT_PATH",
    "IRONVAULT_PASSPHRASE",
    "IRONVAULT_FEDERATION_PASSPHRASE",
    "IRONVAULT_JWT_SECRET",
    "IRONVAULT_HOST",
    "IRONVAULT_PORT",
    "IRONVAULT_REVOCATION_STORE",
    "IRONVAULT_TELEMETRY_ENABLED",
    "IRONVAULT_TELEMETRY_DISABLED",
];

/// Warn about any 4.x variable that is set and no longer read, at process start.
///
/// Several variables are consumed by clap's `#[arg(env = "...")]`, which reads
/// the process environment directly and cannot be intercepted by [`var`]. This
/// swept them in 5.x by copying the old name into the new one; in 6.0 it
/// reports instead of copying, so an operator who missed the deprecation gets a
/// message naming the variable rather than a silently unset secret.
///
/// Only considers names that are currently unset. Call once, before argument
/// parsing.
pub fn report_legacy() {
    for name in KNOWN {
        if raw(name).is_some() {
            continue;
        }
        for legacy in legacy_names(name) {
            if raw(&legacy).is_some() {
                warn_ignored(&legacy, name);
                break;
            }
        }
    }
}

/// Read a variable whose value is a secret.
///
/// Same resolution as [`var`], but the result is [`Zeroizing`] and the
/// untrimmed intermediate is wiped rather than freed to the allocator intact.
/// This does not make the secret disappear — the process environment block
/// still holds a copy, and only the parent that spawned us can prevent that —
/// but it stops this crate from leaving extra copies behind (MITRE ATT&CK
/// T1552, credentials in process memory).
pub fn var_secret(name: &str) -> Option<Zeroizing<String>> {
    if let Ok(mut value) = std::env::var(name) {
        let trimmed = Zeroizing::new(value.trim().to_string());
        value.zeroize();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }

    // Detected but not read -- see `var`. Only the name is printed.
    for legacy in legacy_names(name) {
        if raw(&legacy).is_some() {
            warn_ignored(&legacy, name);
        }
    }

    None
}

/// Whether a variable is set to a non-empty value, under either spelling.
///
/// Used for variables that act as flags, where the value is irrelevant.
pub fn is_set(name: &str) -> bool {
    var(name).is_some()
}

/// Report that a 4.x variable is set and is no longer read, once per process.
fn warn_ignored(legacy: &str, current: &str) {
    let mut seen = warned()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if seen.insert(legacy.to_string()) {
        // Name only. Some of these hold passphrases.
        eprintln!(
            "warning: ${legacy} is set but is NOT being read. It is the 4.x name, \
             removed in 6.0 as 5.x warned it would be. Rename it to ${current} — \
             until you do, this setting has no effect."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Env vars are process-global, so these tests use distinct names rather
    /// than a shared lock.
    #[test]
    fn prefers_the_current_name() {
        std::env::set_var("IRONVAULT_T_PREFER", "new");
        std::env::set_var("aimodelvault_T_PREFER", "old");
        assert_eq!(var("IRONVAULT_T_PREFER").as_deref(), Some("new"));
        std::env::remove_var("IRONVAULT_T_PREFER");
        std::env::remove_var("aimodelvault_T_PREFER");
    }

    #[test]
    fn the_package_prefix_is_no_longer_read() {
        // 5.x resolved this to "value" and warned. 6.0 does not read it.
        std::env::set_var("aimodelvault_T_PKG", "value");
        assert_eq!(var("IRONVAULT_T_PKG"), None);
        std::env::remove_var("aimodelvault_T_PKG");
    }

    #[test]
    fn the_binary_prefix_is_no_longer_read() {
        std::env::set_var("AIM_T_BIN", "value");
        assert_eq!(var("IRONVAULT_T_BIN"), None);
        std::env::remove_var("AIM_T_BIN");
    }

    #[test]
    fn empty_and_whitespace_are_unset() {
        std::env::set_var("IRONVAULT_T_EMPTY", "   ");
        assert_eq!(var("IRONVAULT_T_EMPTY"), None);
        std::env::remove_var("IRONVAULT_T_EMPTY");
    }

    #[test]
    fn a_blank_current_name_does_not_revive_the_old_one() {
        std::env::set_var("IRONVAULT_T_BLANK", "");
        std::env::set_var("aimodelvault_T_BLANK", "old");
        assert_eq!(var("IRONVAULT_T_BLANK"), None);
        std::env::remove_var("IRONVAULT_T_BLANK");
        std::env::remove_var("aimodelvault_T_BLANK");
    }

    #[test]
    fn third_party_names_are_read_verbatim_with_no_fallback() {
        // `legacy_names` must not invent `aimodelvault_REGION` for `AWS_REGION`.
        assert!(legacy_names("AWS_REGION").is_empty());
        assert!(legacy_names("DO_NOT_TRACK").is_empty());
        assert_eq!(
            legacy_names("IRONVAULT_HOME"),
            vec!["aimodelvault_HOME", "AIM_HOME"]
        );
    }

    #[test]
    fn secrets_do_not_resolve_through_the_old_name_either() {
        // The dangerous case: 5.x read this, so an upgrade must not leave the
        // operator thinking the passphrase is still being picked up.
        std::env::set_var("aimodelvault_T_SECRET", "  hunter2  ");
        assert!(var_secret("IRONVAULT_T_SECRET").is_none());
        std::env::remove_var("aimodelvault_T_SECRET");
    }

    #[test]
    fn the_current_name_still_resolves_for_secrets() {
        std::env::set_var("IRONVAULT_T_SECRET_OK", "  hunter2  ");
        let got = var_secret("IRONVAULT_T_SECRET_OK").expect("current secret resolves");
        assert_eq!(&*got, "hunter2");
        std::env::remove_var("IRONVAULT_T_SECRET_OK");
    }

    #[test]
    fn a_blank_secret_is_unset() {
        std::env::set_var("IRONVAULT_T_BLANK_SECRET", "   ");
        assert!(var_secret("IRONVAULT_T_BLANK_SECRET").is_none());
        std::env::remove_var("IRONVAULT_T_BLANK_SECRET");
    }

    #[test]
    fn values_are_trimmed() {
        std::env::set_var("IRONVAULT_T_TRIM", "  spaced  ");
        assert_eq!(var("IRONVAULT_T_TRIM").as_deref(), Some("spaced"));
        std::env::remove_var("IRONVAULT_T_TRIM");
    }
}
