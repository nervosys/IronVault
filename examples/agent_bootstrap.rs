//! Agent bootstrap pattern — discover the CLI surface, then drive it.
//!
//! This example shows the canonical agent-first integration pattern:
//!
//!   1. Shell out to `iv introspect --format json` to get the machine-
//!      readable CLI schema (commands, flags, types, examples, exit codes).
//!   2. Parse it. Pick a capability. Build an argv. Invoke. Parse JSON output.
//!
//! The read-only `iv` subcommands listed in README.md's stability contract
//! accept `--format json` and emit structured output suitable for an LLM
//! tool-calling loop. (The grouped reads -- `cloud list`, `database list`,
//! `acl list` -- do not; they are text-only.) Errors come back as
//! `{ "code", "message", "hint" }` on stderr with stable exit codes.
//!
//! The exit-code contract is `VaultError::exit_code` in `src/error.rs`, and
//! is published in README.md, AGENTS.md, docs/CLI.md and
//! `.well-known/agents.json`. It is deliberately not restated here: this file
//! carried a fifth table that disagreed with all of them, which is the exact
//! defect v3.0.0 set out to remove.
//!
//! Run with:  `cargo run --example agent_bootstrap`
//!
//! Unlike the other two agent examples, this one shells out to `iv`, so the
//! binary must be on PATH -- `cargo install --path .` first, or point PATH at
//! `target/release`. Without it the example exits 1 and says so.
//!
//! Requires the `iv` binary on `PATH` (build it first with `cargo build --release`
//! and add `target/release/` to PATH, or just run `cargo install --path .`).

use serde_json::Value;
use std::process::{Command, Output};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== IronVault — Agent Bootstrap Demo ===\n");

    // ── Step 1 — Discover the full CLI surface ────────────────────────────
    println!("1. `iv introspect --format json` — fetch machine-readable schema");
    let schema = aim_json(&["introspect", "--format", "json", "--compact"])?;

    let commands = schema
        .get("commands")
        .and_then(|c| c.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    println!(
        "   ✓ schema version: {}",
        schema.get("version").unwrap_or(&Value::Null)
    );
    println!("   ✓ commands discovered: {}\n", commands);

    // ── Step 2 — Filter for read-only / safe commands ─────────────────────
    println!("2. Filter for safe (read-only / idempotent) commands");
    if let Some(cmds) = schema.get("commands").and_then(|c| c.as_array()) {
        let safe: Vec<&str> = cmds
            .iter()
            .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
            .filter(|n| {
                matches!(
                    *n,
                    "list"
                        | "versions"
                        | "stats"
                        | "compliance"
                        | "search"
                        | "lineage"
                        | "introspect"
                )
            })
            .collect();
        println!("   ✓ safe commands available: {:?}\n", safe);
    }

    // ── Step 3 — Invoke a capability and parse the JSON envelope ──────────
    println!("3. Invoke `iv list --format json` to enumerate models");
    match aim_json(&["list", "--format", "json"]) {
        Ok(models) => {
            let n = models.as_array().map(|a| a.len()).unwrap_or(0);
            println!("   ✓ {} model(s) in active vault", n);
        }
        Err(e) => {
            // The agent should recover gracefully — no vault yet is normal.
            println!("   · no vault yet ({}). Run `iv init` to create one.", e);
        }
    }
    println!();

    // ── Step 4 — Demonstrate error-envelope handling ──────────────────────
    println!("4. Demonstrate stable error envelope on a missing model");
    let out = aim_raw(&[
        "get",
        "nonexistent-model",
        "/tmp/out.bin",
        "--format",
        "json",
    ])?;
    let code = out.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&out.stderr);
    println!("   ✓ exit code: {code} (3 = not-found per stability contract)");
    if let Ok(env) = serde_json::from_str::<Value>(stderr.trim()) {
        println!("   ✓ error envelope: {}", env);
    } else {
        println!(
            "   · stderr (first line): {}",
            stderr.lines().next().unwrap_or("")
        );
    }
    println!();

    // ── Pattern summary ───────────────────────────────────────────────────
    println!("─── Agent integration pattern ─────────────────────────────────");
    println!("  loop:                                                       ");
    println!("    schema  = iv introspect --format json                    ");
    println!("    tool    = llm.pick_tool(schema.commands, user_goal)      ");
    println!("    result  = subprocess.run([\"iv\", tool, ...], json)     ");
    println!("    if exit != 0: parse stderr JSON envelope, retry or back off");
    println!("───────────────────────────────────────────────────────────────");

    Ok(())
}

/// Run `iv ARGS` and parse stdout as JSON.
fn aim_json(args: &[&str]) -> Result<Value, Box<dyn std::error::Error>> {
    let out = aim_raw(args)?;
    if !out.status.success() {
        return Err(format!(
            "iv {} exited with code {}",
            args.join(" "),
            out.status.code().unwrap_or(-1)
        )
        .into());
    }
    let v: Value = serde_json::from_slice(&out.stdout)?;
    Ok(v)
}

/// Run `iv ARGS` and return the raw `Output` (no exit-code check).
fn aim_raw(args: &[&str]) -> Result<Output, Box<dyn std::error::Error>> {
    Command::new("iv").args(args).output().map_err(|e| {
        format!(
            "failed to spawn `iv {}` — is it on PATH? ({})",
            args.join(" "),
            e
        )
        .into()
    })
}
