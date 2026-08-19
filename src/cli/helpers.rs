//! CLI helper utilities.

use ironvault::{kms, Result, Vault, VaultBuilder, VaultConfig};
use std::io::{self, BufRead, IsTerminal, Write};
use zeroize::Zeroize;

/// Environment variable holding the vault passphrase for unattended use.
///
/// The value is either the passphrase itself or a KMS URI
/// (`env://`, `file://`, `aws-sm://`, `azure-kv://`, `vault://`) — see
/// [`ironvault::kms`].
pub const PASSPHRASE_ENV: &str = "IRONVAULT_PASSPHRASE";

/// Obtain the vault passphrase, in descending order of precedence:
///
/// 1. `$IRONVAULT_PASSPHRASE` — a literal value or a KMS URI to resolve.
/// 2. A line piped on stdin, when stdin is not a terminal.
/// 3. An interactive masked prompt.
///
/// Steps 1 and 2 make every passphrase-gated command usable from CI and from
/// agents; step 3 preserves the interactive behaviour for humans.
///
/// Every intermediate buffer that holds the plaintext is zeroized before it is
/// dropped. Each of the three paths used to leave a copy behind: `kms::resolve`
/// returns an owned `String`, `read_line` fills one, and `trim_end_matches` +
/// `to_vec` copies out of it — so the secret was freed to the allocator intact
/// and could resurface in a later allocation or a core dump (MITRE ATT&CK
/// T1552, credentials in process memory). The returned buffer is the caller's
/// responsibility; it is consumed by `derive_key`, which zeroizes it.
pub fn prompt_passphrase(prompt: &str) -> Result<Vec<u8>> {
    // `var_secret` already trims, rejects an empty value, and zeroizes its own
    // intermediate, so the plaintext lives only in the `Zeroizing` binding.
    if let Some(value) = ironvault::env::var_secret(PASSPHRASE_ENV) {
        let mut resolved = kms::resolve(&value)?;
        let bytes = resolved.as_bytes().to_vec();
        resolved.zeroize();
        return Ok(bytes);
    }

    let stdin = io::stdin();
    if !stdin.is_terminal() {
        let mut line = String::new();
        // A closed/empty stdin is not a passphrase — fall through to the prompt
        // rather than silently unlocking with "".
        let read = stdin.lock().read_line(&mut line);
        match read {
            Ok(n) if n > 0 => {
                let trimmed = line.trim_end_matches(['\n', '\r']);
                if !trimmed.is_empty() {
                    let bytes = trimmed.as_bytes().to_vec();
                    line.zeroize();
                    return Ok(bytes);
                }
                line.zeroize();
            }
            Ok(_) => line.zeroize(),
            Err(err) => {
                line.zeroize();
                return Err(err.into());
            }
        }

        // Nothing on stdin, and no terminal to ask. Falling through to the
        // prompt here used to hang forever rather than fail: `rpassword` opens
        // the console device directly, so on Windows it waits on a keyboard
        // that no one is sitting at. A CI job or cron entry would stall until
        // its timeout with no output, which reads as a slow build rather than
        // a misconfiguration. Fail with something actionable instead.
        return Err(ironvault::VaultError::ConfigError(format!(
            "No passphrase available and no terminal to prompt on. Set ${PASSPHRASE_ENV} (a literal value or a KMS URI) or pipe the passphrase on stdin."
        )));
    }

    print!("{}", prompt);
    io::stdout().flush()?;

    let passphrase = rpassword::read_password()?;
    if passphrase.is_empty() {
        // A closed or non-interactive stdin reads as "" here. Deriving a key
        // from an empty passphrase would silently unlock the vault with no
        // secret at all, so refuse it.
        return Err(ironvault::VaultError::InvalidInput(format!(
            "No passphrase provided. Set ${PASSPHRASE_ENV} (a literal value or a \
             KMS URI), pipe it on stdin, or run interactively."
        )));
    }
    // `String::into_bytes` hands over the same allocation rather than copying,
    // so there is no second buffer to clear on this path.
    Ok(passphrase.into_bytes())
}

/// Build a [`Vault`] using the standard builder, optionally enabling SQLite
/// version storage when `use_sqlite` is true.
///
/// All CLI handlers should use this instead of `Vault::new()` directly.
pub fn build_vault(config: VaultConfig, use_sqlite: bool) -> Result<Vault> {
    let mut builder = VaultBuilder::new().config(config);

    if use_sqlite {
        #[cfg(feature = "sqlite")]
        {
            builder = builder.sqlite_versions();
        }
        #[cfg(not(feature = "sqlite"))]
        {
            return Err(ironvault::VaultError::ConfigError(
                "SQLite version backend requires the `sqlite` feature".to_string(),
            ));
        }
    }

    builder.build()
}
