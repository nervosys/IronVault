//! Environment variable names, and the 4.x compatibility shim behind them.
//!
//! Before the IronVault rename this project read two unrelated prefixes:
//! `aimodelvault_*` (the old package name, lowercased) and `AIM_*` (the old
//! `aim` binary name, now `iv`). Both are gone in 5.0; everything is
//! `IRONVAULT_*`.
//!
//! A rename that silently stops reading a deployment's configuration is a bad
//! rename. Rather than break every existing `EnvironmentFile` and CI job on
//! upgrade, every read goes through [`var`], which falls back to the old
//! spellings and warns once per variable. The fallback is scheduled for
//! removal in 6.0.
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

/// Read an `IRONVAULT_*` variable, falling back to its 4.x name.
///
/// Empty values count as unset. If only the old name is set, its value is used
/// and a one-time deprecation notice goes to stderr.
pub fn var(name: &str) -> Option<String> {
    if let Some(value) = raw(name) {
        return Some(value);
    }

    for legacy in legacy_names(name) {
        if let Some(value) = raw(&legacy) {
            warn_legacy(&legacy, name);
            return Some(value);
        }
    }

    None
}

/// Every `IRONVAULT_*` variable this project defines.
///
/// Only used by [`migrate_legacy`]. Test-only knobs are omitted deliberately —
/// they never existed in a 4.x deployment, so they have nothing to migrate.
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

/// Copy any 4.x variable into its 5.0 name, once, at process start.
///
/// [`var`] handles the fallback for reads that go through this module, but
/// several variables are consumed by clap's `#[arg(env = "...")]`, which reads
/// the process environment directly and cannot be intercepted. Rather than
/// leave those as the one family that silently stops working on upgrade, the
/// binary normalises the environment before parsing arguments.
///
/// Only fills in names that are unset, so an explicit new-style value always
/// wins. Call once, before argument parsing.
pub fn migrate_legacy() {
    for name in KNOWN {
        if raw(name).is_some() {
            continue;
        }
        for legacy in legacy_names(name) {
            if let Some(value) = raw(&legacy) {
                warn_legacy(&legacy, name);
                std::env::set_var(name, value);
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
    let mut candidates = vec![name.to_string()];
    candidates.extend(legacy_names(name));

    for (i, candidate) in candidates.iter().enumerate() {
        let Ok(mut value) = std::env::var(candidate) else {
            continue;
        };
        let trimmed = Zeroizing::new(value.trim().to_string());
        value.zeroize();
        if trimmed.is_empty() {
            continue;
        }
        if i > 0 {
            warn_legacy(candidate, name);
        }
        return Some(trimmed);
    }

    None
}

/// Whether a variable is set to a non-empty value, under either spelling.
///
/// Used for variables that act as flags, where the value is irrelevant.
pub fn is_set(name: &str) -> bool {
    var(name).is_some()
}

/// Print a deprecation notice for `legacy`, at most once per process.
fn warn_legacy(legacy: &str, current: &str) {
    let mut seen = warned()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if seen.insert(legacy.to_string()) {
        // Name only. Some of these hold passphrases.
        eprintln!(
            "warning: ${legacy} is the 4.x name and will stop being read in 6.0. \
             Rename it to ${current}."
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
    fn falls_back_to_the_package_prefix() {
        std::env::set_var("aimodelvault_T_PKG", "value");
        assert_eq!(var("IRONVAULT_T_PKG").as_deref(), Some("value"));
        std::env::remove_var("aimodelvault_T_PKG");
    }

    #[test]
    fn falls_back_to_the_binary_prefix() {
        std::env::set_var("AIM_T_BIN", "value");
        assert_eq!(var("IRONVAULT_T_BIN").as_deref(), Some("value"));
        std::env::remove_var("AIM_T_BIN");
    }

    #[test]
    fn empty_and_whitespace_are_unset() {
        std::env::set_var("IRONVAULT_T_EMPTY", "   ");
        assert_eq!(var("IRONVAULT_T_EMPTY"), None);
        std::env::remove_var("IRONVAULT_T_EMPTY");
    }

    #[test]
    fn an_empty_current_name_still_falls_back() {
        // An operator who blanks the new variable has not thereby unset the
        // old one; the old one is still the only configuration present.
        std::env::set_var("IRONVAULT_T_BLANK", "");
        std::env::set_var("aimodelvault_T_BLANK", "old");
        assert_eq!(var("IRONVAULT_T_BLANK").as_deref(), Some("old"));
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
    fn secrets_resolve_through_the_same_fallback() {
        std::env::set_var("aimodelvault_T_SECRET", "  hunter2  ");
        let got = var_secret("IRONVAULT_T_SECRET").expect("legacy secret resolves");
        assert_eq!(&*got, "hunter2");
        std::env::remove_var("aimodelvault_T_SECRET");
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
