//! CLI integration tests for the `iv` binary.
//!
//! Uses `assert_cmd` to exercise the CLI end-to-end, validating argument parsing,
//! help output, version display, and vault lifecycle commands.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

/// Helper to get a Command for the `iv` binary.
fn iv() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("iv").unwrap()
}

// ──────────────────────────────────────────────────────────────
// Help & Version
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_help() {
    iv().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("secure vault"))
        .stdout(predicate::str::contains("Usage").or(predicate::str::contains("USAGE")));
}

#[test]
fn test_cli_version() {
    iv().arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("1.2.1").or(predicate::str::contains("iv")));
}

#[test]
fn test_cli_no_args_shows_help() {
    // With no subcommand, clap prints help as a convenience — but the command
    // line was still incomplete, so this is invalid input (6), not success.
    // It must not be 2 either: that code means authentication failed.
    iv().assert().code(6);
}

// ──────────────────────────────────────────────────────────────
// Subcommand Help
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_init_help() {
    iv().args(["init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("init").or(predicate::str::contains("vault")));
}

#[test]
fn test_cli_store_help() {
    iv().args(["store", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("store").or(predicate::str::contains("model")));
}

#[test]
fn test_cli_list_help() {
    iv().args(["list", "--help"]).assert().success();
}

#[test]
fn test_cli_convert_help() {
    iv().args(["convert", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("convert"));
}

#[test]
fn test_cli_compliance_help() {
    iv().args(["compliance", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("compliance"));
}

// ──────────────────────────────────────────────────────────────
// Vault Lifecycle (init, store, list, get)
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_init_vault() {
    let dir = tempdir().unwrap();

    iv()
        .args(["init", "--name", "test-vault"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Vault").or(predicate::str::contains("initialized").or(predicate::str::contains("created"))));
}

/// With neither `IRONVAULT_PASSPHRASE` nor anything on stdin, `list` must
/// fail rather than unlock with an empty passphrase — a closed stdin reads as
/// "" from the prompt, which would otherwise derive a key from no secret.
/// The unattended paths are covered by `test_cli_list_empty_vault_non_interactive`.
#[test]
fn test_cli_list_without_passphrase_source() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["list"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .env_remove("IRONVAULT_PASSPHRASE")
        .timeout(std::time::Duration::from_secs(10))
        // Only the exit status is asserted: with no terminal, `rpassword` may
        // either return an empty string or fail outright depending on how the
        // test harness attaches stdin, and the two render different messages.
        // What must hold either way is that the vault does not unlock.
        .assert()
        .failure();
}

/// An explicitly empty `IRONVAULT_PASSPHRASE` must not unlock the vault
/// either — it falls through to the prompt, which now refuses an empty secret.
#[test]
fn test_cli_empty_passphrase_env_is_rejected() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["list"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .env("IRONVAULT_PASSPHRASE", "")
        .timeout(std::time::Duration::from_secs(10))
        .assert()
        .failure();
}

#[test]
fn test_cli_list_conversions() {
    iv().args(["list-conversions"]).assert().success().stdout(
        predicate::str::contains("safetensors")
            .or(predicate::str::contains("Safetensors"))
            .or(predicate::str::contains("GGUF"))
            .or(predicate::str::contains("gguf")),
    );
}

#[test]
fn test_cli_stats_on_vault() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .env("IRONVAULT_PASSPHRASE", "cli-test-passphrase-3318")
        .assert()
        .success();

    // `stats` requires the passphrase from 7.0 on. It reads the unencrypted
    // version index, so it used to answer without one.
    iv().args(["stats"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .env("IRONVAULT_PASSPHRASE", "cli-test-passphrase-3318")
        .assert()
        .success();
}

// ──────────────────────────────────────────────────────────────
// Error Cases
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_unknown_subcommand() {
    iv().arg("nonexistent-command").assert().failure();
}

#[test]
fn test_cli_store_missing_args() {
    iv().args(["store"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

#[test]
fn test_cli_get_missing_args() {
    iv().args(["get"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

// ──────────────────────────────────────────────────────────────
// Feature Flags
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_sqlite_versions_flag_accepted() {
    // The --sqlite-versions flag should be accepted without error
    let dir = tempdir().unwrap();

    iv().args(["--sqlite-versions", "init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();
}

#[test]
fn test_cli_compliance_runs() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["compliance"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();
}

// ──────────────────────────────────────────────────────────────
// Additional Subcommand Help Tests
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_versions_help() {
    iv().args(["versions", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("version").or(predicate::str::contains("Version")));
}

#[test]
fn test_cli_lineage_help() {
    iv().args(["lineage", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("lineage").or(predicate::str::contains("history")));
}

#[test]
fn test_cli_delete_help() {
    iv().args(["delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("delete").or(predicate::str::contains("Delete")));
}

#[test]
fn test_cli_archive_help() {
    iv().args(["archive", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("archive").or(predicate::str::contains("Archive")));
}

#[test]
fn test_cli_extract_help() {
    iv().args(["extract", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("extract").or(predicate::str::contains("Extract")));
}

#[test]
fn test_cli_analyze_help() {
    iv().args(["analyze", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("analyze").or(predicate::str::contains("Analyze")));
}

#[test]
fn test_cli_deduplicate_help() {
    iv().args(["deduplicate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("deduplicate").or(predicate::str::contains("duplicate")));
}

#[test]
fn test_cli_export_help() {
    iv().args(["export", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("export").or(predicate::str::contains("Export")));
}

#[test]
fn test_cli_cache_help() {
    iv().args(["cache", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cache").or(predicate::str::contains("Cache")));
}

#[test]
fn test_cli_change_passphrase_help() {
    iv().args(["change-passphrase", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("passphrase")
                .or(predicate::str::contains("Passphrase"))
                .or(predicate::str::contains("Change")),
        );
}

#[test]
fn test_cli_cloud_help() {
    iv().args(["cloud", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cloud").or(predicate::str::contains("Cloud")));
}

#[test]
fn test_cli_database_help() {
    iv().args(["database", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("database").or(predicate::str::contains("Database")));
}

#[test]
fn test_cli_card_help() {
    iv().args(["card", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("card").or(predicate::str::contains("Card")));
}

#[test]
fn test_cli_telemetry_help() {
    iv().args(["telemetry", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("telemetry").or(predicate::str::contains("Telemetry")));
}

// ──────────────────────────────────────────────────────────────
// Telemetry Subcommands (non-interactive)
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_telemetry_status() {
    iv().args(["telemetry", "status"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Telemetry")
                .or(predicate::str::contains("telemetry"))
                .or(predicate::str::contains("enabled"))
                .or(predicate::str::contains("disabled")),
        );
}

#[test]
fn test_cli_telemetry_disable_then_status() {
    // Disable telemetry, then check status reports disabled
    iv().args(["telemetry", "disable"])
        .env("DO_NOT_TRACK", "1")
        .assert()
        .success();

    iv().args(["telemetry", "status"])
        .env("DO_NOT_TRACK", "1")
        .assert()
        .success();
}

// ──────────────────────────────────────────────────────────────
// Database Subcommands (non-interactive)
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_database_init_and_stats() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    iv().args([
        "database",
        "init",
        "--path",
        db_path.to_str().unwrap(),
        "--db-type",
        "sqlite",
    ])
    .assert()
    .success();

    iv().args(["database", "stats", "--path", db_path.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_cli_database_list_empty() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    iv().args([
        "database",
        "init",
        "--path",
        db_path.to_str().unwrap(),
        "--db-type",
        "sqlite",
    ])
    .assert()
    .success();

    iv().args(["database", "list", "--path", db_path.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_cli_database_store_and_search() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let doc_path = dir.path().join("doc.txt");
    std::fs::write(
        &doc_path,
        "The transformer architecture uses attention mechanisms.",
    )
    .unwrap();

    iv().args([
        "database",
        "init",
        "--path",
        db_path.to_str().unwrap(),
        "--db-type",
        "sqlite",
    ])
    .assert()
    .success();

    iv().args([
        "database",
        "store",
        "--path",
        db_path.to_str().unwrap(),
        "--input",
        doc_path.to_str().unwrap(),
    ])
    .assert()
    .success();

    iv().args([
        "database",
        "search",
        "--path",
        db_path.to_str().unwrap(),
        "transformer",
    ])
    .assert()
    .success();
}

// ──────────────────────────────────────────────────────────────
// Model Card Subcommands (non-interactive)
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_card_template_basic() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("card.json");

    iv().args([
        "card",
        "template",
        "--template-type",
        "basic",
        "--output",
        output.to_str().unwrap(),
    ])
    .assert()
    .success();

    // Verify the template file was created
    assert!(output.exists());
}

#[test]
fn test_cli_card_create_and_validate() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("card.json");

    iv().args([
        "card",
        "create",
        "test-model",
        "--version",
        "1.0",
        "--description",
        "A test model",
        "--model-type",
        "classifier",
        "--architecture",
        "ResNet-50",
        "--output",
        output.to_str().unwrap(),
    ])
    .assert()
    .success();

    assert!(output.exists());

    iv().args(["card", "validate", output.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_cli_card_create_and_show() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("card.yaml");

    iv().args([
        "card",
        "create",
        "my-llm",
        "--version",
        "2.0",
        "--description",
        "A language model",
        "--model-type",
        "LLM",
        "--architecture",
        "Transformer",
        "--output",
        output.to_str().unwrap(),
    ])
    .assert()
    .success();

    iv().args([
        "card",
        "show",
        output.to_str().unwrap(),
        "--format",
        "markdown",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("my-llm").or(predicate::str::contains("language model")));
}

#[test]
fn test_cli_card_convert_json_to_yaml() {
    let dir = tempdir().unwrap();
    let json_out = dir.path().join("card.json");
    let yaml_out = dir.path().join("card.yaml");

    iv().args([
        "card",
        "create",
        "conv-model",
        "--version",
        "1.0",
        "--description",
        "For conversion test",
        "--model-type",
        "classifier",
        "--architecture",
        "CNN",
        "--output",
        json_out.to_str().unwrap(),
    ])
    .assert()
    .success();

    iv().args([
        "card",
        "convert",
        json_out.to_str().unwrap(),
        yaml_out.to_str().unwrap(),
    ])
    .assert()
    .success();

    assert!(yaml_out.exists());
}

// ──────────────────────────────────────────────────────────────
// Additional Error Cases
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_delete_missing_args() {
    iv().args(["delete"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

#[test]
fn test_cli_archive_missing_args() {
    iv().args(["archive"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

#[test]
fn test_cli_versions_missing_args() {
    iv().args(["versions"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

#[test]
fn test_cli_lineage_missing_args() {
    iv().args(["lineage"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

#[test]
fn test_cli_export_missing_args() {
    iv().args(["export"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

// ──────────────────────────────────────────────────────────────
// Vault Lifecycle — Extended
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_cache_on_vault() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["cache"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();
}

#[test]
fn test_cli_list_conversions_contains_formats() {
    // Validate that list-conversions includes expected format names
    iv().args(["list-conversions"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ONNX").or(predicate::str::contains("onnx")))
        .stdout(
            predicate::str::contains("PyTorch")
                .or(predicate::str::contains("pytorch"))
                .or(predicate::str::contains("pt")),
        );
}

#[test]
fn test_cli_init_custom_name() {
    let dir = tempdir().unwrap();

    iv().args(["init", "--name", "my-custom-vault"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("my-custom-vault")
                .or(predicate::str::contains("Vault"))
                .or(predicate::str::contains("initialized")),
        );
}

#[test]
fn test_cli_sqlite_versions_with_stats() {
    let dir = tempdir().unwrap();

    iv().args(["--sqlite-versions", "init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .env("IRONVAULT_PASSPHRASE", "cli-test-passphrase-3318")
        .assert()
        .success();

    // Same 7.0 requirement on the SQLite backend.
    iv().args(["--sqlite-versions", "stats"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .env("IRONVAULT_PASSPHRASE", "cli-test-passphrase-3318")
        .assert()
        .success();
}

#[test]
fn test_cli_sqlite_versions_with_compliance() {
    let dir = tempdir().unwrap();

    iv().args(["--sqlite-versions", "init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["--sqlite-versions", "compliance"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();
}

// ──────────────────────────────────────────────────────────────
// Telemetry Flags
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_no_telemetry_flag_with_init() {
    let dir = tempdir().unwrap();

    iv().args(["--no-telemetry", "init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();
}

#[test]
fn test_cli_do_not_track_env() {
    iv().args(["compliance"])
        .env("DO_NOT_TRACK", "1")
        .assert()
        .success();
}

// ──────────────────────────────────────────────────────────────
// Convert Error Cases
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_convert_missing_args() {
    iv().args(["convert"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

// ──────────────────────────────────────────────────────────────
// Cloud Error Cases
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_cloud_push_missing_args() {
    iv().args(["cloud", "push"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

#[test]
fn test_cli_cloud_pull_missing_args() {
    iv().args(["cloud", "pull"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

#[test]
fn test_cli_cloud_list_missing_args() {
    iv().args(["cloud", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

// ──────────────────────────────────────────────────────────────
// Archive / Extract Error Cases
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_extract_nonexistent_archive() {
    iv().args(["extract", "nonexistent.tar"]).assert().failure();
}

// ──────────────────────────────────────────────────────────────
// Database Error Cases
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_database_search_no_results() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("empty.db");

    iv().args([
        "database",
        "init",
        "--path",
        db_path.to_str().unwrap(),
        "--db-type",
        "sqlite",
    ])
    .assert()
    .success();

    iv().args([
        "database",
        "search",
        "--path",
        db_path.to_str().unwrap(),
        "nonexistent query",
    ])
    .assert()
    .success();
}

#[test]
fn test_cli_database_store_nonexistent_file() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    iv().args([
        "database",
        "init",
        "--path",
        db_path.to_str().unwrap(),
        "--db-type",
        "sqlite",
    ])
    .assert()
    .success();

    iv().args([
        "database",
        "store",
        "--path",
        db_path.to_str().unwrap(),
        "--input",
        "nonexistent_file.txt",
    ])
    .assert()
    .failure();
}

// ──────────────────────────────────────────────────────────────
// Card Error Cases
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_card_validate_nonexistent_file() {
    iv().args(["card", "validate", "no-such-card.json"])
        .assert()
        .failure();
}

#[test]
fn test_cli_card_show_nonexistent_file() {
    iv().args(["card", "show", "no-such-card.json"])
        .assert()
        .failure();
}

// ──────────────────────────────────────────────────────────────
// SQLite Backend — Extended
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_sqlite_versions_with_cache() {
    let dir = tempdir().unwrap();

    iv().args(["--sqlite-versions", "init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["--sqlite-versions", "cache"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();
}

#[test]
fn test_cli_init_twice_same_dir() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();
}

// ──────────────────────────────────────────────────────────────
// v1.4.0 Feature — Help Tests
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_tag_help() {
    iv().args(["tag", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tag").or(predicate::str::contains("Tag")));
}

#[test]
fn test_cli_search_help() {
    iv().args(["search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("search").or(predicate::str::contains("Search")));
}

#[test]
fn test_cli_vault_export_help() {
    iv().args(["vault-export", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("export").or(predicate::str::contains("Export")));
}

#[test]
fn test_cli_vault_import_help() {
    iv().args(["vault-import", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("import").or(predicate::str::contains("Import")));
}

#[test]
fn test_cli_gc_help() {
    iv().args(["gc", "--help"]).assert().success().stdout(
        predicate::str::contains("gc")
            .or(predicate::str::contains("garbage").or(predicate::str::contains("clean"))),
    );
}

#[test]
fn test_cli_browse_help() {
    iv().args(["browse", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("browse").or(predicate::str::contains("Browse")));
}

#[test]
fn test_cli_webhook_help() {
    iv().args(["webhook", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("webhook").or(predicate::str::contains("Webhook")));
}

#[test]
fn test_cli_acl_help() {
    iv().args(["acl", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("acl").or(predicate::str::contains("access")));
}

#[test]
fn test_cli_validate_help() {
    iv().args(["validate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("validate").or(predicate::str::contains("Validate")));
}

#[test]
fn test_cli_policy_help() {
    iv().args(["policy", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("policy").or(predicate::str::contains("Policy")));
}

#[test]
fn test_cli_lineage_graph_help() {
    iv().args(["lineage-graph", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("lineage").or(predicate::str::contains("Lineage")));
}

#[test]
fn test_cli_plugin_help() {
    iv().args(["plugin", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("plugin").or(predicate::str::contains("Plugin")));
}

#[test]
fn test_cli_profile_help() {
    iv().args(["profile", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("profile").or(predicate::str::contains("Profile")));
}

// ──────────────────────────────────────────────────────────────
// v1.4.0 Feature — Functional Tests
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_gc_dry_run_on_vault() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["gc", "--dry-run"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();
}

#[test]
fn test_cli_acl_grant_list_revoke() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["acl", "grant", "alice", "writer"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["acl", "list"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("alice"));

    iv().args(["acl", "revoke", "alice"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();
}

#[test]
fn test_cli_webhook_add_list_remove() {
    let dir = tempdir().unwrap();
    let id = format!("hook-{}", std::process::id());

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["webhook", "add", &id, "https://example.com/hook"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["webhook", "list"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("example.com"));
}

#[test]
fn test_cli_policy_set_show() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["policy", "set", "test-model", "--max-versions", "5"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["policy", "show", "test-model"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("test-model"));
}

#[test]
fn test_cli_profile_create_list_activate() {
    let dir = tempdir().unwrap();

    iv().args(["profile", "create", "dev"])
        .env("IRONVAULT_CONFIG", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["profile", "list"])
        .env("IRONVAULT_CONFIG", dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("dev"));

    iv().args(["profile", "activate", "dev"])
        .env("IRONVAULT_CONFIG", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["profile", "show", "dev"])
        .env("IRONVAULT_CONFIG", dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("dev"));
}

#[test]
fn test_cli_lineage_graph_show_empty() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["lineage-graph", "show", "any-model"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();
}

#[test]
fn test_cli_plugin_list_empty() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["plugin", "list"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();
}

#[test]
fn test_cli_tag_add_list_on_vault() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["tag", "add", "my-model", "llm", "production"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["tag", "list", "my-model"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("llm"));
}

// ──────────────────────────────────────────────────────────────
// v1.4.0 Feature — Error Cases
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_validate_missing_args() {
    iv().args(["validate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

#[test]
fn test_cli_vault_export_missing_args() {
    iv().args(["vault-export"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

#[test]
fn test_cli_vault_import_missing_args() {
    iv().args(["vault-import"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

#[test]
fn test_cli_tag_add_missing_args() {
    iv().args(["tag", "add"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

#[test]
fn test_cli_search_empty_query() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["search"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();
}

// ──────────────────────────────────────────────────────────────
// v1.5.0 Feature — Help Tests
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_quantize_help() {
    iv().args(["quantize", "--help"]).assert().success().stdout(
        predicate::str::contains("quantize")
            .or(predicate::str::contains("Quantize").or(predicate::str::contains("quantization"))),
    );
}

#[test]
fn test_cli_eval_help() {
    iv().args(["eval", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("eval").or(predicate::str::contains("Eval")));
}

#[test]
fn test_cli_backup_help() {
    iv().args(["backup", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("backup").or(predicate::str::contains("Backup")));
}

#[test]
fn test_cli_vaults_help() {
    iv().args(["vaults", "--help"]).assert().success().stdout(
        predicate::str::contains("vaults")
            .or(predicate::str::contains("Vaults").or(predicate::str::contains("vault"))),
    );
}

// ──────────────────────────────────────────────────────────────
// v1.5.0 Feature — Functional Tests
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_quantize_set_list_remove() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args([
        "quantize",
        "set",
        "fast-q4",
        "--method",
        "q4_k_m",
        "--description",
        "Fast 4-bit",
    ])
    .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
    .assert()
    .success();

    iv().args(["quantize", "list"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("fast-q4"));

    iv().args(["quantize", "remove", "fast-q4"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();
}

#[test]
fn test_cli_quantize_estimate() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args([
        "quantize",
        "estimate",
        "--size",
        "1000000000",
        "--to",
        "q4_k_m",
    ])
    .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
    .assert()
    .success();
}

#[test]
fn test_cli_eval_record_list_suites() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args([
        "eval",
        "record",
        "my-model",
        "--version",
        "1",
        "--suite",
        "mmlu",
        "--metric",
        "accuracy=0.85",
        "--unit",
        "score",
    ])
    .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
    .assert()
    .success();

    iv().args(["eval", "list", "my-model"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("mmlu").or(predicate::str::contains("my-model")));

    iv().args(["eval", "suites"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("mmlu"));
}

#[test]
fn test_cli_backup_set_list_remove() {
    let dir = tempdir().unwrap();
    let backup_dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args([
        "backup",
        "set",
        "nightly",
        "--frequency",
        "daily",
        "--max-backups",
        "5",
        "--output-dir",
        backup_dir.path().to_str().unwrap(),
    ])
    .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
    .assert()
    .success();

    iv().args(["backup", "list"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("nightly"));

    iv().args(["backup", "remove", "nightly"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();
}

#[test]
fn test_cli_backup_history_empty() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["backup", "history"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();
}

#[test]
fn test_cli_vaults_register_list_activate_deactivate() {
    let dir = tempdir().unwrap();
    let vault_dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args([
        "vaults",
        "register",
        "prod",
        vault_dir.path().to_str().unwrap(),
        "--description",
        "Production vault",
    ])
    .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
    .assert()
    .success();

    iv().args(["vaults", "list"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("prod"));

    iv().args(["vaults", "activate", "prod"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["vaults", "deactivate"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["vaults", "unregister", "prod"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();
}

// ──────────────────────────────────────────────────────────────
// v1.5.0 Feature — Error Cases
// ──────────────────────────────────────────────────────────────

#[test]
fn test_cli_quantize_set_missing_method() {
    iv().args(["quantize", "set", "test-profile"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

#[test]
fn test_cli_eval_record_missing_args() {
    iv().args(["eval", "record"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

#[test]
fn test_cli_backup_set_missing_args() {
    iv().args(["backup", "set"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

#[test]
fn test_cli_vaults_register_missing_path() {
    iv().args(["vaults", "register", "test-vault"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

#[test]
fn test_cli_eval_compare_missing_suite() {
    iv().args(["eval", "compare", "a@1", "b@1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("error")));
}

// ──────────────────────────────────────────────────────────────
// Non-interactive passphrase (IRONVAULT_PASSPHRASE / stdin / KMS URI)
//
// Before these existed, every passphrase-gated command required a TTY, so the
// vault round-trip below could not be tested from CI at all.
// ──────────────────────────────────────────────────────────────

const TEST_PASS: &str = "cli-integration-passphrase";

/// `init` + `store` + `list` + `get`, driven entirely by the env var.
#[test]
fn test_cli_roundtrip_with_passphrase_env() {
    let dir = tempdir().unwrap();
    let vault_dir = dir.path().to_str().unwrap();

    let model = dir.path().join("model.safetensors");
    std::fs::write(&model, b"fake-safetensors-payload").unwrap();

    iv().args(["init", "--name", "roundtrip"])
        .env("IRONVAULT_HOME", vault_dir)
        .assert()
        .success();

    iv().args(["store", "demo", model.to_str().unwrap()])
        .env("IRONVAULT_HOME", vault_dir)
        .env("IRONVAULT_PASSPHRASE", TEST_PASS)
        .assert()
        .success()
        .stdout(predicate::str::contains("stored successfully"));

    iv().args(["list"])
        .env("IRONVAULT_HOME", vault_dir)
        .env("IRONVAULT_PASSPHRASE", TEST_PASS)
        .assert()
        .success()
        .stdout(predicate::str::contains("demo"));

    let out = dir.path().join("retrieved.safetensors");
    iv().args(["get", "demo", out.to_str().unwrap()])
        .env("IRONVAULT_HOME", vault_dir)
        .env("IRONVAULT_PASSPHRASE", TEST_PASS)
        .assert()
        .success();

    assert_eq!(
        std::fs::read(&out).unwrap(),
        b"fake-safetensors-payload",
        "retrieved model must match what was stored"
    );
}

/// A wrong passphrase must fail rather than silently returning garbage.
#[test]
fn test_cli_wrong_passphrase_fails() {
    let dir = tempdir().unwrap();
    let vault_dir = dir.path().to_str().unwrap();
    let model = dir.path().join("m.bin");
    std::fs::write(&model, b"payload").unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", vault_dir)
        .assert()
        .success();

    iv().args(["store", "m", model.to_str().unwrap()])
        .env("IRONVAULT_HOME", vault_dir)
        .env("IRONVAULT_PASSPHRASE", TEST_PASS)
        .assert()
        .success();

    let out = dir.path().join("out.bin");
    iv().args(["get", "m", out.to_str().unwrap()])
        .env("IRONVAULT_HOME", vault_dir)
        .env("IRONVAULT_PASSPHRASE", "not-the-right-passphrase")
        .assert()
        .failure();
}

/// The env var may hold an `env://` KMS URI pointing at another variable.
#[test]
fn test_cli_passphrase_via_kms_env_uri() {
    let dir = tempdir().unwrap();
    let vault_dir = dir.path().to_str().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", vault_dir)
        .assert()
        .success();

    iv().args(["list"])
        .env("IRONVAULT_HOME", vault_dir)
        .env("IRONVAULT_PASSPHRASE", "env://IRONVAULT_CLI_TEST_SECRET")
        .env("IRONVAULT_CLI_TEST_SECRET", TEST_PASS)
        .assert()
        .success();
}

/// ...or a `file://` URI pointing at a secret file.
#[test]
fn test_cli_passphrase_via_kms_file_uri() {
    let dir = tempdir().unwrap();
    let vault_dir = dir.path().to_str().unwrap();

    let secret = dir.path().join("passphrase.txt");
    std::fs::write(&secret, format!("{TEST_PASS}\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

    iv().args(["init"])
        .env("IRONVAULT_HOME", vault_dir)
        .assert()
        .success();

    iv().args(["list"])
        .env("IRONVAULT_HOME", vault_dir)
        .env(
            "IRONVAULT_PASSPHRASE",
            format!("file://{}", secret.display()),
        )
        .assert()
        .success();
}

/// An unresolvable KMS URI must fail loudly, not fall back to an empty secret.
#[test]
fn test_cli_unresolvable_kms_uri_fails() {
    let dir = tempdir().unwrap();
    let vault_dir = dir.path().to_str().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", vault_dir)
        .assert()
        .success();

    iv().args(["list"])
        .env("IRONVAULT_HOME", vault_dir)
        .env(
            "IRONVAULT_PASSPHRASE",
            "env://IRONVAULT_NOT_SET_ANYWHERE_42",
        )
        .assert()
        .failure();
}

/// With no env var, a piped passphrase on stdin is accepted.
#[test]
fn test_cli_passphrase_via_stdin() {
    let dir = tempdir().unwrap();
    let vault_dir = dir.path().to_str().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", vault_dir)
        .assert()
        .success();

    iv().args(["list"])
        .env("IRONVAULT_HOME", vault_dir)
        .env_remove("IRONVAULT_PASSPHRASE")
        .write_stdin(format!("{TEST_PASS}\n"))
        .assert()
        .success();
}

/// `list` against a freshly initialised vault now runs unattended — this
/// replaces the old test that could only assert "it did not crash".
#[test]
fn test_cli_list_empty_vault_non_interactive() {
    let dir = tempdir().unwrap();
    let vault_dir = dir.path().to_str().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", vault_dir)
        .assert()
        .success();

    iv().args(["list"])
        .env("IRONVAULT_HOME", vault_dir)
        .env("IRONVAULT_PASSPHRASE", TEST_PASS)
        .assert()
        .success();
}

/// `iv sign` / `iv verify` accept a KMS URI for --key, not just a file path.
/// The secret may be a bare hex seed, as a secret manager would store it.
#[test]
fn test_cli_sign_verify_with_kms_key_uri() {
    let dir = tempdir().unwrap();
    let vault_dir = dir.path().to_str().unwrap();

    let model = dir.path().join("model.bin");
    std::fs::write(&model, b"contents to be signed").unwrap();

    // A 32-byte seed, hex-encoded, as `iv sign` would have generated.
    let seed = "a".repeat(64);

    iv().args([
        "sign",
        "ignored",
        "--file",
        model.to_str().unwrap(),
        "--key",
        "env://IRONVAULT_TEST_SIGN_KEY",
    ])
    .env("IRONVAULT_HOME", vault_dir)
    .env("IRONVAULT_TEST_SIGN_KEY", &seed)
    .assert()
    .success()
    .stdout(predicate::str::contains("KMS"));

    let sig = model.with_extension("sig");
    assert!(
        sig.exists(),
        "detached signature should be written next to the file"
    );

    iv().args([
        "verify",
        "ignored",
        "--file",
        model.to_str().unwrap(),
        "--signature",
        sig.to_str().unwrap(),
        "--key",
        "env://IRONVAULT_TEST_SIGN_KEY",
    ])
    .env("IRONVAULT_HOME", vault_dir)
    .env("IRONVAULT_TEST_SIGN_KEY", &seed)
    .assert()
    .success()
    .stdout(predicate::str::contains("Verification PASSED"));
}

/// A malformed KMS-sourced signing key must fail loudly.
#[test]
fn test_cli_sign_rejects_bad_kms_key() {
    let dir = tempdir().unwrap();
    let model = dir.path().join("m.bin");
    std::fs::write(&model, b"data").unwrap();

    iv().args([
        "sign",
        "ignored",
        "--file",
        model.to_str().unwrap(),
        "--key",
        "env://IRONVAULT_TEST_BAD_KEY",
    ])
    .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
    .env("IRONVAULT_TEST_BAD_KEY", "not-a-valid-seed")
    .assert()
    .failure();
}

// ──────────────────────────────────────────────────────────────
// Conversion
// ──────────────────────────────────────────────────────────────

/// Regression: version records store `format.name()` ("PyTorch"), which
/// `from_extension` does not recognise. `iv convert` used to parse the stored
/// format that way, yielding `Custom("pytorch")`, so it could never find a
/// conversion path for ANY vaulted model.
#[test]
fn test_cli_convert_resolves_stored_format() {
    let dir = tempdir().unwrap();
    let vault_dir = dir.path().to_str().unwrap();

    // Minimal ZIP-magic payload so the format is detected as PyTorch.
    let model = dir.path().join("m.pt");
    let mut bytes = vec![0x50, 0x4b, 0x03, 0x04];
    bytes.extend_from_slice(&[0u8; 60]);
    std::fs::write(&model, &bytes).unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", vault_dir)
        .assert()
        .success();

    iv().args(["store", "demo", model.to_str().unwrap()])
        .env("IRONVAULT_HOME", vault_dir)
        .env("IRONVAULT_PASSPHRASE", TEST_PASS)
        .assert()
        .success();

    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();

    iv().args(["convert", "demo", "--to-format", "onnx"])
        .current_dir(&work)
        .env("IRONVAULT_HOME", vault_dir)
        .env("IRONVAULT_PASSPHRASE", TEST_PASS)
        .assert()
        .success()
        .stdout(predicate::str::contains("Source format: PyTorch"))
        .stdout(predicate::str::contains("No conversion path").not());
}

/// A conversion needing external tooling must leave a plan, never a file with
/// the target extension containing JSON.
#[test]
fn test_cli_convert_writes_plan_not_fake_target_file() {
    let dir = tempdir().unwrap();
    let vault_dir = dir.path().to_str().unwrap();

    let model = dir.path().join("m.pt");
    let mut bytes = vec![0x50, 0x4b, 0x03, 0x04];
    bytes.extend_from_slice(&[0u8; 60]);
    std::fs::write(&model, &bytes).unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", vault_dir)
        .assert()
        .success();
    iv().args(["store", "demo", model.to_str().unwrap()])
        .env("IRONVAULT_HOME", vault_dir)
        .env("IRONVAULT_PASSPHRASE", TEST_PASS)
        .assert()
        .success();

    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();

    iv().args(["convert", "demo", "--to-format", "onnx"])
        .current_dir(&work)
        .env("IRONVAULT_HOME", vault_dir)
        .env("IRONVAULT_PASSPHRASE", TEST_PASS)
        .assert()
        .success()
        .stdout(predicate::str::contains("No ONNX file was produced"));

    assert!(
        work.join("demo_converted.plan.json").exists(),
        "the plan should be written alongside the requested output"
    );
    assert!(
        !work.join("demo_converted.onnx").exists(),
        "no .onnx file may be produced when nothing was converted"
    );
}

// ──────────────────────────────────────────────────────────────
// Diff
// ──────────────────────────────────────────────────────────────

/// Build a minimal spec-shaped GGUF v3 file: magic, counts, two metadata KV
/// pairs (a string and an array, so the parser has to step over both), then the
/// tensor infos.
fn write_gguf(path: &std::path::Path, tensors: &[(&str, &[u64], u32)]) {
    fn push_str(out: &mut Vec<u8>, s: &str) {
        out.extend_from_slice(&(s.len() as u64).to_le_bytes());
        out.extend_from_slice(s.as_bytes());
    }

    let mut data = Vec::new();
    data.extend_from_slice(b"GGUF");
    data.extend_from_slice(&3u32.to_le_bytes());
    data.extend_from_slice(&(tensors.len() as u64).to_le_bytes());
    data.extend_from_slice(&2u64.to_le_bytes());

    push_str(&mut data, "general.architecture");
    data.extend_from_slice(&8u32.to_le_bytes()); // STRING
    push_str(&mut data, "llama");

    push_str(&mut data, "tokenizer.ggml.tokens");
    data.extend_from_slice(&9u32.to_le_bytes()); // ARRAY
    data.extend_from_slice(&8u32.to_le_bytes()); // of STRING
    data.extend_from_slice(&2u64.to_le_bytes());
    push_str(&mut data, "a");
    push_str(&mut data, "bb");

    for (name, dims, ggml_type) in tensors {
        push_str(&mut data, name);
        data.extend_from_slice(&(dims.len() as u32).to_le_bytes());
        for d in *dims {
            data.extend_from_slice(&d.to_le_bytes());
        }
        data.extend_from_slice(&ggml_type.to_le_bytes());
        data.extend_from_slice(&0u64.to_le_bytes()); // data offset
    }

    std::fs::write(path, data).unwrap();
}

/// `iv diff` used to fabricate its GGUF tensor map from the header's tensor
/// *count* alone — `tensor_0`, `tensor_1`, … with no shape and dtype
/// `"unknown"`. Two files with equal tensor counts therefore always reported as
/// identical. This pair differs in dtype, in one tensor name, and in one shape,
/// while keeping the count at 3 — exactly the case that used to come back
/// clean.
#[test]
fn test_cli_diff_gguf_reports_real_tensor_changes() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("left.gguf");
    let right = dir.path().join("right.gguf");

    write_gguf(
        &left,
        &[
            ("blk.0.attn_q.weight", &[4096, 4096], 0), // F32
            ("blk.0.attn_k.weight", &[4096, 1024], 0),
            ("output_norm.weight", &[4096], 0),
        ],
    );
    write_gguf(
        &right,
        &[
            ("blk.0.attn_q.weight", &[4096, 4096], 12), // Q4_K
            ("blk.0.attn_k.weight", &[4096, 1024], 12),
            ("blk.0.ffn_gate.weight", &[4096, 11008], 12),
        ],
    );

    iv().args(["diff", left.to_str().unwrap(), right.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("added: 1"))
        .stdout(predicate::str::contains("removed: 1"))
        .stdout(predicate::str::contains("changed: 2"))
        // Real names and dtypes, not `tensor_0` / `unknown`.
        .stdout(predicate::str::contains("blk.0.ffn_gate.weight"))
        .stdout(predicate::str::contains("F32 → Q4_K"))
        .stdout(predicate::str::contains("output_norm.weight"));
}

/// A GGUF file whose header is cut short must not panic or hang the CLI.
#[test]
fn test_cli_diff_truncated_gguf_is_handled() {
    let dir = tempdir().unwrap();
    let full = dir.path().join("full.gguf");
    write_gguf(&full, &[("a", &[2, 2], 0), ("b", &[2, 2], 0)]);

    let bytes = std::fs::read(&full).unwrap();
    let cut = dir.path().join("cut.gguf");
    std::fs::write(&cut, &bytes[..bytes.len() - 12]).unwrap();

    iv().args(["diff", full.to_str().unwrap(), cut.to_str().unwrap()])
        .assert()
        .success();
}

// ──────────────────────────────────────────────────────────────
// Exit codes
//
// README.md, AGENTS.md, docs/CLI.md, .well-known/agents.json and
// .well-known/ontology.jsonld all publish an exit-code table, and agents are
// told to branch on it. These tests assert the binary actually honours it —
// previously `main` returned `Result`, so every failure collapsed to 1.
// ──────────────────────────────────────────────────────────────

/// Exit 6 — invalid input. An unknown `--format` is a caller mistake, not a
/// generic failure, and it must not exit 1.
#[test]
fn test_cli_exit_code_invalid_input() {
    iv().args(["introspect", "--format", "not-a-format"])
        .assert()
        .code(6)
        .stderr(predicate::str::contains("Unknown format"));
}

/// Exit 3 — not found. Asking for a model that does not exist is the single
/// most common branch an agent needs to distinguish.
#[test]
fn test_cli_exit_code_not_found() {
    let dir = tempdir().unwrap();

    iv().args(["init"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .assert()
        .success();

    iv().args(["versions", "no-such-model"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .env("IRONVAULT_PASSPHRASE", "correct horse battery staple")
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .code(3);
}

/// A mistyped subcommand must not exit 2. clap's default usage-error code is
/// 2, which this table assigns to "authentication failed" — so left alone, an
/// agent would read a typo as a wrong passphrase. Usage errors are invalid
/// input (6).
#[test]
fn test_cli_exit_code_usage_error_is_not_confused_with_auth_failure() {
    iv().args(["no-such-subcommand"])
        .assert()
        .code(6)
        .stderr(predicate::str::contains("unrecognized subcommand"));

    iv().args(["versions", "--no-such-flag"]).assert().code(6);

    // A required argument left off, and no subcommand at all, are the same
    // category — clap prints help for both, but the command line was still
    // incomplete.
    iv().args(["versions"]).assert().code(6);
    iv().assert().code(6);
}

/// `--help` and `--version` route through the same error path inside clap, but
/// they are successes, not usage errors.
#[test]
fn test_cli_exit_code_help_and_version_are_success() {
    iv().args(["--help"]).assert().code(0);
    iv().args(["--version"]).assert().code(0);
    iv().args(["versions", "--help"]).assert().code(0);
}

/// Exit 7 — configuration error. A malformed `--config` file is distinct from
/// a missing one and from a general failure.
#[test]
fn test_cli_exit_code_config_error() {
    let dir = tempdir().unwrap();
    let bad = dir.path().join("config.yaml");
    // Tab indentation is invalid YAML.
    std::fs::write(&bad, "dirs:\n\t data_dir: /tmp\n").unwrap();

    iv().args(["--config", bad.to_str().unwrap(), "list"])
        .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .code(7);

    // A `--config` path that does not exist is also a config error, not a
    // generic I/O failure.
    iv().args([
        "--config",
        dir.path().join("absent.yaml").to_str().unwrap(),
        "list",
    ])
    .env("IRONVAULT_HOME", dir.path().to_str().unwrap())
    .timeout(std::time::Duration::from_secs(30))
    .assert()
    .code(7);
}

/// Exit 5 — integrity/verification failure. `iv verify` is what a pipeline
/// gates on, so its failure code is the one that matters most.
#[test]
fn test_cli_exit_code_verify_failure() {
    let dir = tempdir().unwrap();
    let model = dir.path().join("model.bin");
    std::fs::write(&model, b"the real payload").unwrap();
    let key = dir.path().join("signing_key.json");

    iv().args([
        "sign",
        "model",
        "--file",
        model.to_str().unwrap(),
        "--key",
        key.to_str().unwrap(),
    ])
    .assert()
    .success();

    let sig = dir.path().join("model.sig");
    assert!(sig.exists(), "expected a detached signature at {sig:?}");

    // Tamper with the payload, leaving the signature alone.
    std::fs::write(&model, b"the WRONG payload").unwrap();

    iv().args([
        "verify",
        "model",
        "--signature",
        sig.to_str().unwrap(),
        "--file",
        model.to_str().unwrap(),
        "--key",
        key.to_str().unwrap(),
    ])
    .assert()
    .code(5)
    .stdout(predicate::str::contains("FAILED"));
}

/// Verification with no `--key` must also fail, and for the same reason: it
/// checked nothing. This is the case that used to print PASSED and exit 0.
#[test]
fn test_cli_exit_code_verify_without_key_is_not_success() {
    let dir = tempdir().unwrap();
    let model = dir.path().join("model.bin");
    std::fs::write(&model, b"payload").unwrap();
    let key = dir.path().join("signing_key.json");

    iv().args([
        "sign",
        "model",
        "--file",
        model.to_str().unwrap(),
        "--key",
        key.to_str().unwrap(),
    ])
    .assert()
    .success();

    let sig = dir.path().join("model.sig");

    iv().args([
        "verify",
        "model",
        "--signature",
        sig.to_str().unwrap(),
        "--file",
        model.to_str().unwrap(),
    ])
    .assert()
    .code(5)
    .stdout(predicate::str::contains("NOT CHECKED"));
}

/// Success must stay 0 — the mapping must not make ordinary runs look failed.
#[test]
fn test_cli_exit_code_success_is_zero() {
    iv().args(["--version"]).assert().code(0);
    iv().args(["introspect", "--format", "json"])
        .assert()
        .code(0);
}

/// The exit-code table is published in five places. It drifted into four
/// mutually contradictory versions once already, so this test makes the
/// machine-readable manifests answer to the implementation.
#[test]
fn test_published_exit_code_tables_match_the_implementation() {
    use ironvault::{
        EXIT_AUTH, EXIT_COMPLIANCE, EXIT_CONFIG, EXIT_GENERAL, EXIT_INTEGRITY, EXIT_INVALID_INPUT,
        EXIT_NOT_FOUND, EXIT_PERMISSION, EXIT_SUCCESS,
    };

    let expected: Vec<(String, u8)> = vec![
        ("0".to_string(), EXIT_SUCCESS),
        ("1".to_string(), EXIT_GENERAL),
        ("2".to_string(), EXIT_AUTH),
        ("3".to_string(), EXIT_NOT_FOUND),
        ("4".to_string(), EXIT_PERMISSION),
        ("5".to_string(), EXIT_INTEGRITY),
        ("6".to_string(), EXIT_INVALID_INPUT),
        ("7".to_string(), EXIT_CONFIG),
        ("8".to_string(), EXIT_COMPLIANCE),
    ];
    // The keys are the codes, so each must equal the constant it documents.
    for (key, code) in &expected {
        assert_eq!(key, &code.to_string(), "constant drifted from its own key");
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    let agents: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".well-known/agents.json")).unwrap(),
    )
    .unwrap();
    let cli_iface = agents["agent_interfaces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["type"] == "cli")
        .expect("agents.json must describe the cli interface");
    let published = cli_iface["exit_codes"].as_object().unwrap();
    assert_eq!(
        published.len(),
        expected.len(),
        "agents.json publishes {} codes, the implementation defines {}",
        published.len(),
        expected.len()
    );
    for (key, _) in &expected {
        assert!(
            published.contains_key(key),
            "agents.json is missing exit code {key}"
        );
    }

    let ontology: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".well-known/ontology.jsonld")).unwrap(),
    )
    .unwrap();
    let contract = ontology["iv:errors"]["@exitCodeContract"]
        .as_object()
        .expect("ontology.jsonld must publish iv:errors.@exitCodeContract");
    for (key, _) in &expected {
        assert!(
            contract.contains_key(key),
            "ontology.jsonld is missing exit code {key}"
        );
    }

    // Every error type in the taxonomy must claim a code the implementation
    // can actually produce.
    let taxonomy = ontology["iv:errors"]["errorTypes"]
        .as_array()
        .expect("ontology.jsonld must list iv:errors.errorTypes");
    let valid: Vec<u8> = expected.iter().map(|(_, c)| *c).collect();
    for entry in taxonomy {
        let code = u8::try_from(entry["exit_code"].as_u64().unwrap()).unwrap();
        assert!(
            valid.contains(&code),
            "{} claims exit code {code}, which is not in the contract",
            entry["rdfs:label"]
        );
        assert_ne!(
            code, EXIT_SUCCESS,
            "{} is an error and must not claim exit 0",
            entry["rdfs:label"]
        );
    }
}

/// The `.well-known/` manifests publish a version that agents may branch on,
/// and nothing kept it in step with the crate. It had drifted to `1.6.0`
/// (agents.json, mcp-manifest.json) and `1.5.0` (openapi.yaml) while the crate
/// was at 4.2.1 — three major versions of skew on the discovery surface the
/// README tells agents to read first.
///
/// `ontology.jsonld` is deliberately excluded: its `version` is the ontology
/// schema's own version, tracked by `owl:versionInfo`, and is independent of
/// the crate.
#[test]
fn test_well_known_manifests_declare_the_crate_version() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let crate_version = env!("CARGO_PKG_VERSION");

    let agents: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".well-known/agents.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        agents["project"]["version"].as_str(),
        Some(crate_version),
        ".well-known/agents.json project.version drifted from Cargo.toml"
    );

    let mcp: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".well-known/mcp-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        mcp["version"].as_str(),
        Some(crate_version),
        ".well-known/mcp-manifest.json version drifted from Cargo.toml"
    );

    // Parsed as text rather than YAML: the top-level `version:` under `info:`
    // is the only two-space-indented `version:` in the file, and the test would
    // otherwise need a YAML dependency for one field.
    let openapi = std::fs::read_to_string(root.join(".well-known/openapi.yaml")).unwrap();
    let declared = openapi
        .lines()
        .find(|l| l.starts_with("  version:"))
        .map(|l| l.trim_start_matches("  version:").trim().to_string())
        .expect("openapi.yaml has no top-level info.version");
    assert_eq!(
        declared, crate_version,
        ".well-known/openapi.yaml info.version drifted from Cargo.toml"
    );
}

/// `iv cloud` supports S3 and Azure. GCS was removed along with the
/// `cloud-storage` crate, and there is no `gcs` cargo feature — so a manifest
/// advertising it sends an agent down a path that cannot work.
#[test]
fn test_manifests_do_not_advertise_removed_gcs_support() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for name in [
        ".well-known/ai-plugin.json",
        ".well-known/agents.json",
        ".well-known/mcp-manifest.json",
    ] {
        let body = std::fs::read_to_string(root.join(name)).unwrap();
        assert!(
            !body.contains("GCS") && !body.contains("Google Cloud Storage"),
            "{name} advertises GCS, which `iv cloud` cannot do"
        );
    }
}

/// The PyPI distribution is `ironvault`; the crates.io crate is
/// `ironvault`. agents.json told agents to run `pip install
/// ironvault`, which does not resolve. Names that differ by punctuation
/// across two registries are exactly the kind of thing that rots silently.
#[test]
fn test_manifest_python_install_uses_the_real_pypi_name() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let agents: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".well-known/agents.json")).unwrap(),
    )
    .unwrap();

    let pypi_name = {
        let pyproject = std::fs::read_to_string(root.join("pyproject.toml")).unwrap();
        pyproject
            .lines()
            .find(|l| l.trim_start().starts_with("name"))
            .and_then(|l| l.split('=').nth(1))
            .map(|v| v.trim().trim_matches('"').to_string())
            .expect("pyproject.toml has no name")
    };

    let install = agents["agent_interfaces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["type"] == "python_bindings")
        .and_then(|i| i["install"].as_str())
        .expect("no python_bindings interface with an install command");

    assert_eq!(
        install,
        format!("pip install {pypi_name}"),
        "agents.json python install command does not match pyproject.toml name"
    );
}

/// `AZURE_STORAGE_KEY` is rejected by `AzureBackend::new` — the Azure SDK for
/// Rust v1 has no shared-key credential. Advertising it in the discovery
/// surface sends an agent down a path that terminates in a hard error.
#[test]
fn test_manifest_does_not_advertise_unsupported_azure_shared_key() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let agents = std::fs::read_to_string(root.join(".well-known/agents.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&agents).unwrap();

    for iface in parsed["agent_interfaces"].as_array().unwrap() {
        if let Some(vars) = iface["environment_variables"].as_array() {
            for v in vars {
                let name = v["name"].as_str().unwrap_or_default();
                assert_ne!(
                    name, "AZURE_STORAGE_KEY",
                    "agents.json lists AZURE_STORAGE_KEY, which the Azure v1 SDK cannot use"
                );
            }
        }
    }
}

/// Capabilities that are reachable from the CLI must say so.
///
/// `federation` and `blockchain_audit` sat in agents.json with no `commands`
/// field for as long as they were library-only. Once wired, an agent reading
/// the catalog would still have concluded there was no way to invoke them.
#[test]
fn test_wired_capabilities_list_their_commands() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let agents = std::fs::read_to_string(root.join(".well-known/agents.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&agents).unwrap();

    for name in ["federation", "blockchain_audit"] {
        let cap = &parsed["capabilities"][name];
        let commands = cap["commands"].as_array().unwrap_or_else(|| {
            panic!("agents.json capability '{name}' is wired to the CLI but lists no commands")
        });
        assert!(
            !commands.is_empty(),
            "agents.json capability '{name}' has an empty commands list"
        );
    }
}
