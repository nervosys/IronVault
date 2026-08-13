//! `Vault::unlock` must reject a wrong passphrase.
//!
//! Until 6.2.1 it accepted any passphrase: deriving a key always succeeds —
//! Argon2 stretches whatever it is given — and nothing checked the result, so
//! `unlock` returned `Ok` with a garbage key. Failure surfaced only when an
//! AEAD tag mismatched on the first read, which meant every operation that did
//! not touch ciphertext succeeded for an attacker:
//!
//! - `iv list` and `iv stats` printed the model inventory and exited 0
//! - `POST /api/v1/auth/token` minted an admin JWT, which then read `/models`,
//!   `/audit`, `/acl`, `/policies` and `/stats`
//!
//! Model *contents* stayed protected, because those are AEAD-sealed. The
//! metadata around them was not.

use ironvault::config::VaultConfig;
use ironvault::error::VaultError;
use ironvault::formats::{ModelFormat, ModelMetadata};
use ironvault::vault::Vault;
use tempfile::TempDir;

const RIGHT: &[u8] = b"the-owners-real-passphrase-9931";
const WRONG: &[u8] = b"an-attackers-guess";

fn vault_in(dir: &TempDir) -> Vault {
    let mut config = VaultConfig::default();
    config.dirs.config_dir = dir.path().join("config");
    config.dirs.data_dir = dir.path().join("data");
    config.dirs.cache_dir = dir.path().join("cache");
    config.dirs.vault_dir = dir.path().join("data/vaults");
    config.dirs.log_dir = dir.path().join("data/logs");
    config.dirs.backends_dir = dir.path().join("config/backends");
    config.dirs.utilities_dir = dir.path().join("config/utilities");
    config.dirs.databases_dir = dir.path().join("config/databases");
    Vault::new(Some(config)).expect("vault constructs")
}

fn metadata() -> ModelMetadata {
    ModelMetadata::new("m".to_string(), ModelFormat::Safetensors)
}

#[test]
fn a_wrong_passphrase_is_rejected_on_a_vault_holding_data() {
    let dir = TempDir::new().unwrap();

    let mut vault = vault_in(&dir);
    vault.unlock(RIGHT.to_vec()).expect("first unlock");
    vault
        .store_model("m", b"payload".to_vec(), metadata(), None)
        .expect("store");

    let mut attacker = vault_in(&dir);
    let err = attacker
        .unlock(WRONG.to_vec())
        .expect_err("a wrong passphrase must not unlock");
    assert!(
        matches!(err, VaultError::AuthenticationFailed),
        "expected AuthenticationFailed, got {err:?}"
    );
}

#[test]
fn a_wrong_passphrase_leaves_the_vault_locked_and_contents_unreadable() {
    // The bug was that `unlock` returned Ok, so callers proceeded as though
    // authenticated. What must hold now: the error propagates, no key is
    // installed, and nothing sealed can be opened.
    //
    // Note this is about the *key*, not the version index. `versions.json` is
    // stored in the clear, so anything holding the vault directory can read
    // model names and sizes from it regardless of any passphrase — see
    // `SECURITY.md`. Encrypting the index is a format change, not a fix to
    // this.
    let dir = TempDir::new().unwrap();

    let mut vault = vault_in(&dir);
    vault.unlock(RIGHT.to_vec()).unwrap();
    vault
        .store_model("secret-model", b"payload".to_vec(), metadata(), None)
        .unwrap();

    let mut attacker = vault_in(&dir);
    assert!(attacker.unlock(WRONG.to_vec()).is_err());
    assert!(
        attacker.get_model("secret-model", None).is_err(),
        "a failed unlock must leave contents unreadable"
    );
}

#[test]
fn the_right_passphrase_still_works_after_a_wrong_attempt() {
    let dir = TempDir::new().unwrap();

    let mut vault = vault_in(&dir);
    vault.unlock(RIGHT.to_vec()).unwrap();
    vault
        .store_model("m", b"payload".to_vec(), metadata(), None)
        .unwrap();

    let mut attacker = vault_in(&dir);
    let _ = attacker.unlock(WRONG.to_vec());

    // A failed attempt must not have written a key check for the wrong key,
    // which would lock the owner out of their own vault.
    let mut owner = vault_in(&dir);
    owner.unlock(RIGHT.to_vec()).expect("owner still unlocks");
    assert_eq!(owner.get_model("m", None).unwrap(), b"payload".to_vec());
}

#[test]
fn an_empty_vault_accepts_the_passphrase_that_creates_it() {
    // Any passphrase is legitimately correct for a vault with nothing in it;
    // the key check is created here and enforced from the next unlock on.
    let dir = TempDir::new().unwrap();

    let mut vault = vault_in(&dir);
    vault
        .unlock(RIGHT.to_vec())
        .expect("first unlock of a new vault");

    let mut second = vault_in(&dir);
    assert!(
        second.unlock(WRONG.to_vec()).is_err(),
        "once created, the vault must reject a different passphrase"
    );

    let mut owner = vault_in(&dir);
    owner.unlock(RIGHT.to_vec()).expect("owner reopens");
}

#[test]
fn changing_the_passphrase_leaves_the_vault_openable_with_the_new_one() {
    // The key check must be re-sealed under the new key. Re-encrypting every
    // blob and then rejecting the new passphrase at the next unlock would be a
    // passphrase change that locks the owner out of their own vault.
    let dir = TempDir::new().unwrap();
    const NEW: &[u8] = b"a-brand-new-passphrase-5567";

    let mut vault = vault_in(&dir);
    vault.unlock(RIGHT.to_vec()).unwrap();
    vault
        .store_model("m", b"payload".to_vec(), metadata(), None)
        .unwrap();
    vault
        .change_passphrase(NEW.to_vec())
        .expect("passphrase changes");

    let mut reopened = vault_in(&dir);
    reopened
        .unlock(NEW.to_vec())
        .expect("new passphrase opens the vault");
    assert_eq!(reopened.get_model("m", None).unwrap(), b"payload".to_vec());

    let mut stale = vault_in(&dir);
    assert!(
        stale.unlock(RIGHT.to_vec()).is_err(),
        "the old passphrase must stop working"
    );
}
