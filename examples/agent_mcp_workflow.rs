//! Agent MCP workflow — register vault-backed MCP tools and run an agent loop.
//!
//! Demonstrates the in-process integration pattern: rather than shelling out
//! to the CLI, the host process exposes vault capabilities as MCP tools that
//! an LLM/agent can call directly. Every tool has a JSON Schema input and
//! returns a `ToolResult { success, data, error, metadata }`.
//!
//! The three tools below are illustrative, not the manifest's surface. They
//! are named `vault.store` / `vault.list` / `vault.search`, while
//! `.well-known/mcp-manifest.json` declares `vault_init`, `model_convert` and
//! 84 others — so an agent onboarded against the manifest will not find these
//! by name, and this file used to claim the opposite. `register_builtin_tools`
//! is the only shipped tool set; it covers the four RAG tools. Everything else
//! the manifest declares is a surface a host process is expected to register
//! itself, using exactly the pattern shown here.
//!
//! Run with:  `cargo run --example agent_mcp_workflow`

use ironvault::rag::{MCPServer, MCPTool, ToolContext, ToolResult};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== IronVault — Agent MCP Workflow ===\n");

    // ── 1. Build an MCP server with vault-flavored tools ──────────────────
    // For the demo the "vault" is a thread-local in-memory map keyed by
    // model name. In production each closure would call into a real
    // `Vault` instance instead.
    thread_local! {
        static STORE: RefCell<BTreeMap<String, ModelRecord>> = const { RefCell::new(BTreeMap::new()) };
    }

    #[derive(Clone)]
    struct ModelRecord {
        size_bytes: u64,
        tags: Vec<String>,
        sha256: String,
    }

    let mut server = MCPServer::new();

    // Tool: vault.store — register a model with the vault.
    server.register_tool(
        MCPTool::new(
            "vault.store".to_string(),
            "Store a model in the vault. Returns the version and checksum.".to_string(),
        )
        .add_parameter("name", "string", "Model name", true)
        .add_parameter("size_bytes", "number", "Model size in bytes", true)
        .add_parameter("tags", "array", "Tags to attach", false),
        |params, _ctx| {
            let name = require_str(&params, "name")?;
            let size = params
                .get("size_bytes")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let tags: Vec<String> = params
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|s| s.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            // Pretend SHA-256 — in a real tool, hash the bytes.
            let sha = format!("{:064x}", name.bytes().map(|b| b as u128).sum::<u128>());

            STORE.with(|s| {
                s.borrow_mut().insert(
                    name.to_string(),
                    ModelRecord {
                        size_bytes: size,
                        tags: tags.clone(),
                        sha256: sha.clone(),
                    },
                );
            });

            Ok(ToolResult::success(json!({
                "name": name,
                "version": 1,
                "sha256": sha,
                "size_bytes": size,
                "tags": tags,
            })))
        },
    )?;

    // Tool: vault.list — enumerate models.
    server.register_tool(
        MCPTool::new(
            "vault.list".to_string(),
            "List all models in the vault.".to_string(),
        ),
        |_params, _ctx| {
            let models: Vec<Value> = STORE.with(|s| {
                s.borrow()
                    .iter()
                    .map(|(name, rec)| {
                        json!({
                            "name": name,
                            "size_bytes": rec.size_bytes,
                            "sha256": rec.sha256,
                            "tags": rec.tags,
                        })
                    })
                    .collect()
            });
            Ok(ToolResult::success(
                json!({ "models": models, "count": models.len() }),
            ))
        },
    )?;

    // Tool: vault.search — filter models by tag.
    server.register_tool(
        MCPTool::new(
            "vault.search".to_string(),
            "Search models by tag. Returns matching model names.".to_string(),
        )
        .add_parameter("tag", "string", "Tag to filter on", true),
        |params, _ctx| {
            let tag = require_str(&params, "tag")?.to_string();
            let hits: Vec<String> = STORE.with(|s| {
                s.borrow()
                    .iter()
                    .filter(|(_, r)| r.tags.iter().any(|t| t == &tag))
                    .map(|(n, _)| n.clone())
                    .collect()
            });
            Ok(ToolResult::success(json!({ "tag": tag, "matches": hits })))
        },
    )?;

    println!(
        "1. Registered {} MCP tools on the server:",
        server.list_tools().len()
    );
    for t in server.list_tools() {
        println!("   · {:<14} — {}", t.name, t.description);
    }
    println!();

    // ── 2. Simulate an agent loop ─────────────────────────────────────────
    // An LLM agent given the user goal "ingest my two checkpoints and find
    // the production-ready ones" would emit this sequence of tool calls.
    let ctx = ToolContext::new();

    let plan: Vec<(&str, Value)> = vec![
        (
            "vault.store",
            json!({
                "name": "llama-3-8b-finetuned",
                "size_bytes": 16_384_000_000_u64,
                "tags": ["production", "llm", "fp16"],
            }),
        ),
        (
            "vault.store",
            json!({
                "name": "llama-3-8b-experimental",
                "size_bytes": 16_384_000_000_u64,
                "tags": ["experimental", "llm", "fp16"],
            }),
        ),
        ("vault.list", json!({})),
        ("vault.search", json!({ "tag": "production" })),
    ];

    println!("2. Agent executes a {}-step plan:\n", plan.len());
    for (i, (tool, params)) in plan.iter().enumerate() {
        println!("   [{}] → {}({})", i + 1, tool, compact(params));
        let result = server.execute_tool(tool, params.clone(), &ctx)?;
        if result.success {
            println!("       ✓ {}", compact(&result.data));
        } else {
            println!(
                "       ✗ {}",
                result.error.as_deref().unwrap_or("unknown error")
            );
        }
    }
    println!();

    // ── 3. Pattern summary ────────────────────────────────────────────────
    println!("─── In-process MCP agent pattern ──────────────────────────────");
    println!("  server.register_tool(spec, |params, ctx| {{ ... }})          ");
    println!("  loop:                                                        ");
    println!("    call = llm.decide(server.list_tools(), conversation)       ");
    println!("    res  = server.execute_tool(call.name, call.params, ctx)   ");
    println!("    conversation.push(ToolMessage(res))                        ");
    println!("───────────────────────────────────────────────────────────────");

    Ok(())
}

fn require_str<'a>(v: &'a Value, key: &str) -> Result<&'a str, ironvault::VaultError> {
    v.get(key).and_then(|x| x.as_str()).ok_or_else(|| {
        ironvault::VaultError::InvalidInput(format!("missing or non-string param: {}", key))
    })
}

fn compact(v: &Value) -> String {
    let s = serde_json::to_string(v).unwrap_or_default();
    if s.len() > 88 {
        format!("{}…", &s[..87])
    } else {
        s
    }
}
