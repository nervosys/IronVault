//! Introspect CLI command handler.
//!
//! Outputs a complete machine-readable CLI schema for agent discovery.
//! Supports JSON, YAML, and JSON-LD output formats.

use ironvault::Result;
use serde_json::{json, Value};

/// Handle the introspect command — output full CLI schema for agents.
pub fn handle_introspect(format: String, compact: bool) -> Result<()> {
    let schema = build_cli_schema(compact);

    match format.to_lowercase().as_str() {
        "json" => {
            let output = if compact {
                serde_json::to_string(&schema)
                    .map_err(|e| ironvault::VaultError::SerializationError(e.to_string()))?
            } else {
                serde_json::to_string_pretty(&schema)
                    .map_err(|e| ironvault::VaultError::SerializationError(e.to_string()))?
            };
            println!("{output}");
        }
        "yaml" | "yml" => {
            let output = serde_yaml_ng::to_string(&schema)
                .map_err(|e| ironvault::VaultError::SerializationError(e.to_string()))?;
            println!("{output}");
        }
        "jsonld" | "json-ld" => {
            let mut ld = serde_json::Map::new();
            ld.insert(
                "@context".to_string(),
                json!({
                    // Must match the `iv` term in `.well-known/ontology.jsonld`.
                    // It previously pointed at a different host, so the CLI's
                    // JSON-LD and the published ontology minted distinct IRIs
                    // for the same terms — consumers that joined them saw two
                    // unrelated vocabularies.
                    "iv": "https://nervosys.com/ontology/iv#",
                    "schema": "https://schema.org/",
                    "rdfs": "http://www.w3.org/2000/01/rdf-schema#"
                }),
            );
            ld.insert("@type".to_string(), json!("iv:CLISchema"));
            ld.insert("@id".to_string(), json!("iv://cli/introspect"));
            if let Value::Object(map) = schema {
                for (k, v) in map {
                    ld.insert(format!("iv:{k}"), v);
                }
            }
            let output = serde_json::to_string_pretty(&Value::Object(ld))
                .map_err(|e| ironvault::VaultError::SerializationError(e.to_string()))?;
            println!("{output}");
        }
        other => {
            return Err(ironvault::VaultError::InvalidInput(format!(
                "Unknown format {other:?}. Supported: json, yaml, jsonld"
            )))
        }
    }

    Ok(())
}

fn build_cli_schema(compact: bool) -> Value {
    let mut schema = json!({
        "binary": "iv",
        "version": env!("CARGO_PKG_VERSION"),
        "install": "cargo install ironvault",
        "globalFlags": [
            flag("--vault", "-v", "string", false, null_val(), desc(compact, "Vault name (uses default if not specified)")),
            flag("--config", "-c", "path", false, null_val(), desc(compact, "Config file path")),
            flag("--sqlite-versions", "", "bool", false, null_val(), desc(compact, "Use SQLite for version storage")),
            flag("--no-telemetry", "", "bool", false, null_val(), desc(compact, "Disable telemetry for this session")),
        ],
        "commands": build_commands(compact),
    });

    if !compact {
        schema.as_object_mut().unwrap().insert(
            "interfaces".to_string(),
            json!(["cli", "mcp", "rest", "graphql", "rust-lib", "python"]),
        );
        schema.as_object_mut().unwrap().insert(
            "discoveryFiles".to_string(),
            json!({
                "ontology": ".well-known/ontology.jsonld",
                "agents": ".well-known/agents.json",
                "mcp": ".well-known/mcp-manifest.json",
                "openapi": ".well-known/openapi.yaml",
                "plugin": ".well-known/ai-plugin.json"
            }),
        );
    }

    schema
}

fn build_commands(compact: bool) -> Value {
    json!([
        cmd(
            "init",
            "vault",
            compact,
            "Initialize a new encrypted vault",
            vec![arg(
                "--name",
                "-n",
                "string",
                false,
                Some(json!("default")),
                desc(compact, "Vault name")
            )],
            ex(compact, &["iv init", "iv init --name production"])
        ),
        cmd(
            "store",
            "vault",
            compact,
            "Store a model in the vault with encryption",
            vec![
                arg(
                    "name",
                    "",
                    "string",
                    true,
                    None,
                    desc(compact, "Model name")
                ),
                arg(
                    "path",
                    "",
                    "path",
                    true,
                    None,
                    desc(compact, "Path to model file")
                ),
                arg(
                    "--format",
                    "-f",
                    "string",
                    false,
                    None,
                    desc(compact, "Model format (auto-detected if omitted)")
                ),
                arg(
                    "--description",
                    "-d",
                    "string",
                    false,
                    None,
                    desc(compact, "Description")
                ),
                arg(
                    "--framework",
                    "",
                    "string",
                    false,
                    None,
                    desc(compact, "Framework (pytorch, tensorflow, jax)")
                ),
                arg(
                    "--task",
                    "",
                    "string",
                    false,
                    None,
                    desc(compact, "ML task (text-generation, etc.)")
                ),
            ],
            ex(compact, &["iv store my-llm ./model.safetensors"])
        ),
        cmd(
            "get",
            "vault",
            compact,
            "Retrieve and decrypt a model from the vault",
            vec![
                arg(
                    "name",
                    "",
                    "string",
                    true,
                    None,
                    desc(compact, "Model name")
                ),
                arg(
                    "output",
                    "",
                    "path",
                    true,
                    None,
                    desc(compact, "Output file path")
                ),
                arg(
                    "--version",
                    "-v",
                    "u32",
                    false,
                    None,
                    desc(compact, "Version number")
                ),
            ],
            ex(compact, &["iv get my-llm ./output.safetensors"])
        ),
        cmd(
            "list",
            "vault",
            compact,
            "List all models in the vault",
            vec![],
            ex(compact, &["iv list"])
        ),
        cmd(
            "versions",
            "vault",
            compact,
            "Show versions of a model",
            vec![arg(
                "name",
                "",
                "string",
                true,
                None,
                desc(compact, "Model name")
            ),],
            ex(compact, &["iv versions my-llm"])
        ),
        cmd(
            "lineage",
            "vault",
            compact,
            "Show version ancestry tree",
            vec![
                arg(
                    "name",
                    "",
                    "string",
                    true,
                    None,
                    desc(compact, "Model name")
                ),
                arg(
                    "version",
                    "",
                    "u32",
                    true,
                    None,
                    desc(compact, "Version number")
                ),
            ],
            ex(compact, &["iv lineage my-llm 5"])
        ),
        cmd(
            "delete",
            "vault",
            compact,
            "Delete a model version",
            vec![
                arg(
                    "name",
                    "",
                    "string",
                    true,
                    None,
                    desc(compact, "Model name")
                ),
                arg(
                    "version",
                    "",
                    "u32",
                    true,
                    None,
                    desc(compact, "Version number")
                ),
                arg(
                    "--force",
                    "-f",
                    "bool",
                    false,
                    None,
                    desc(compact, "Skip confirmation")
                ),
            ],
            ex(
                compact,
                &["iv delete my-llm 2", "iv delete my-llm 2 --force"]
            )
        ),
        cmd(
            "stats",
            "vault",
            compact,
            "Show vault storage statistics",
            vec![],
            ex(compact, &["iv stats"])
        ),
        cmd(
            "compliance",
            "security",
            compact,
            "Run FIPS/CMMC/MITRE compliance checks",
            vec![],
            ex(compact, &["iv compliance"])
        ),
        cmd(
            "change-passphrase",
            "security",
            compact,
            "Change vault encryption passphrase",
            vec![],
            ex(compact, &["iv change-passphrase"])
        ),
        cmd(
            "archive",
            "utility",
            compact,
            "Archive models to TAR or ZIP",
            vec![
                arg(
                    "models",
                    "",
                    "string[]",
                    true,
                    None,
                    desc(compact, "Model names")
                ),
                arg(
                    "output",
                    "",
                    "path",
                    true,
                    None,
                    desc(compact, "Output archive path")
                ),
                arg(
                    "--format",
                    "-f",
                    "string",
                    false,
                    Some(json!("tar")),
                    desc(compact, "Archive format (tar, zip)")
                ),
                arg(
                    "--versions",
                    "-v",
                    "u32[]",
                    false,
                    None,
                    desc(compact, "Version numbers")
                ),
            ],
            ex(compact, &["iv archive my-llm ./models.tar"])
        ),
        cmd(
            "extract",
            "utility",
            compact,
            "Extract models from archive",
            vec![
                arg(
                    "archive",
                    "",
                    "path",
                    true,
                    None,
                    desc(compact, "Archive file path")
                ),
                arg(
                    "--output",
                    "-o",
                    "path",
                    false,
                    Some(json!(".")),
                    desc(compact, "Output directory")
                ),
            ],
            ex(compact, &["iv extract ./models.tar"])
        ),
        cmd(
            "analyze",
            "utility",
            compact,
            "Analyze model compression efficiency",
            vec![
                arg(
                    "name",
                    "",
                    "string",
                    true,
                    None,
                    desc(compact, "Model name")
                ),
                arg(
                    "--version",
                    "-v",
                    "u32",
                    false,
                    None,
                    desc(compact, "Version number")
                ),
            ],
            ex(compact, &["iv analyze my-llm"])
        ),
        cmd(
            "deduplicate",
            "utility",
            compact,
            "Find duplicate models in vault",
            vec![arg(
                "--detailed",
                "-d",
                "bool",
                false,
                None,
                desc(compact, "Show similarity scores")
            ),],
            ex(compact, &["iv deduplicate"])
        ),
        cmd(
            "export",
            "utility",
            compact,
            "Export model with metadata",
            vec![
                arg(
                    "name",
                    "",
                    "string",
                    true,
                    None,
                    desc(compact, "Model name")
                ),
                arg(
                    "output",
                    "",
                    "path",
                    true,
                    None,
                    desc(compact, "Output directory")
                ),
                arg(
                    "--version",
                    "-v",
                    "u32",
                    false,
                    None,
                    desc(compact, "Version number")
                ),
            ],
            ex(compact, &["iv export my-llm ./exported/"])
        ),
        cmd(
            "cache",
            "utility",
            compact,
            "Show cache statistics",
            vec![],
            ex(compact, &["iv cache"])
        ),
        cmd(
            "convert",
            "conversion",
            compact,
            "Convert model between formats",
            vec![
                arg(
                    "name",
                    "",
                    "string",
                    true,
                    None,
                    desc(compact, "Model name in vault")
                ),
                arg(
                    "--to-format",
                    "-t",
                    "string",
                    true,
                    None,
                    desc(compact, "Target format")
                ),
                arg(
                    "--output",
                    "-o",
                    "path",
                    false,
                    None,
                    desc(compact, "Output file path")
                ),
                arg(
                    "--version",
                    "-v",
                    "u32",
                    false,
                    None,
                    desc(compact, "Source version")
                ),
                arg(
                    "--quantization",
                    "-q",
                    "string",
                    false,
                    None,
                    desc(compact, "Quantization (q4_0, q4_k_m, q5_k_m, q8_0)")
                ),
                arg(
                    "--opset",
                    "",
                    "u32",
                    false,
                    Some(json!(17)),
                    desc(compact, "ONNX opset version")
                ),
                arg(
                    "--validate",
                    "",
                    "bool",
                    false,
                    None,
                    desc(compact, "Validate output")
                ),
                arg(
                    "--plan-only",
                    "",
                    "bool",
                    false,
                    None,
                    desc(compact, "Show plan only")
                ),
            ],
            ex(compact, &["iv convert my-llm -t gguf -q q4_k_m --validate"])
        ),
        cmd(
            "list-conversions",
            "conversion",
            compact,
            "List supported format conversion paths",
            vec![],
            ex(compact, &["iv list-conversions"])
        ),
        cmd(
            "serve",
            "api",
            compact,
            "Start REST/GraphQL API server",
            vec![
                arg(
                    "--host",
                    "",
                    "string",
                    false,
                    Some(json!("127.0.0.1")),
                    desc(compact, "Host address")
                ),
                arg(
                    "--port",
                    "-p",
                    "u16",
                    false,
                    Some(json!(8080)),
                    desc(compact, "Port")
                ),
                arg(
                    "--jwt-secret",
                    "",
                    "string",
                    true,
                    None,
                    desc(compact, "JWT secret")
                ),
                arg(
                    "--token-expiry",
                    "",
                    "u64",
                    false,
                    Some(json!(3600)),
                    desc(compact, "Token expiry (seconds)")
                ),
                arg(
                    "--cors-permissive",
                    "",
                    "bool",
                    false,
                    None,
                    desc(compact, "Allow CORS from any origin")
                ),
                arg(
                    "--no-dashboard",
                    "",
                    "bool",
                    false,
                    None,
                    desc(compact, "Disable web dashboard")
                ),
            ],
            ex(compact, &["iv serve --jwt-secret mysecret"])
        ),
        cmd_sub(
            "cloud",
            "cloud",
            compact,
            "Cloud storage operations",
            vec![
                subcmd(
                    "push",
                    compact,
                    "Push model to cloud",
                    vec![
                        arg(
                            "model",
                            "",
                            "string",
                            true,
                            None,
                            desc(compact, "Model name")
                        ),
                        arg(
                            "--version",
                            "-v",
                            "u32",
                            false,
                            None,
                            desc(compact, "Version number")
                        ),
                        arg(
                            "--provider",
                            "-p",
                            "string",
                            true,
                            None,
                            desc(compact, "Provider (s3, azure, gcs)")
                        ),
                        arg(
                            "--bucket",
                            "-b",
                            "string",
                            true,
                            None,
                            desc(compact, "Bucket name")
                        ),
                    ]
                ),
                subcmd(
                    "pull",
                    compact,
                    "Pull model from cloud",
                    vec![
                        arg(
                            "model",
                            "",
                            "string",
                            true,
                            None,
                            desc(compact, "Model name")
                        ),
                        arg(
                            "--provider",
                            "-p",
                            "string",
                            true,
                            None,
                            desc(compact, "Provider")
                        ),
                        arg(
                            "--bucket",
                            "-b",
                            "string",
                            true,
                            None,
                            desc(compact, "Bucket")
                        ),
                        arg(
                            "--remote-path",
                            "-k",
                            "string",
                            true,
                            None,
                            desc(compact, "Remote path")
                        ),
                    ]
                ),
                subcmd(
                    "list",
                    compact,
                    "List cloud models",
                    vec![
                        arg(
                            "--provider",
                            "-p",
                            "string",
                            true,
                            None,
                            desc(compact, "Provider")
                        ),
                        arg(
                            "--bucket",
                            "-b",
                            "string",
                            true,
                            None,
                            desc(compact, "Bucket")
                        ),
                    ]
                ),
                subcmd("config", compact, "Show cloud configuration", vec![]),
            ]
        ),
        cmd_sub(
            "card",
            "model-card",
            compact,
            "Model card operations",
            vec![
                subcmd(
                    "create",
                    compact,
                    "Create a model card",
                    vec![
                        arg(
                            "name",
                            "",
                            "string",
                            true,
                            None,
                            desc(compact, "Model name")
                        ),
                        arg(
                            "--version",
                            "-v",
                            "string",
                            true,
                            None,
                            desc(compact, "Version")
                        ),
                        arg(
                            "--description",
                            "-d",
                            "string",
                            true,
                            None,
                            desc(compact, "Description")
                        ),
                        arg(
                            "--model-type",
                            "-t",
                            "string",
                            true,
                            None,
                            desc(compact, "Model type")
                        ),
                        arg(
                            "--architecture",
                            "-a",
                            "string",
                            true,
                            None,
                            desc(compact, "Architecture")
                        ),
                        arg(
                            "--output",
                            "-o",
                            "path",
                            true,
                            None,
                            desc(compact, "Output file")
                        ),
                    ]
                ),
                subcmd(
                    "show",
                    compact,
                    "Show a model card",
                    vec![
                        arg(
                            "path",
                            "",
                            "path",
                            true,
                            None,
                            desc(compact, "Card file path")
                        ),
                        arg(
                            "--format",
                            "-f",
                            "string",
                            false,
                            Some(json!("markdown")),
                            desc(compact, "Output format")
                        ),
                    ]
                ),
                subcmd(
                    "validate",
                    compact,
                    "Validate a model card",
                    vec![arg(
                        "path",
                        "",
                        "path",
                        true,
                        None,
                        desc(compact, "Card file path")
                    ),]
                ),
                subcmd(
                    "template",
                    compact,
                    "Generate template card",
                    vec![
                        arg(
                            "--template-type",
                            "-t",
                            "string",
                            false,
                            Some(json!("basic")),
                            desc(compact, "Template type")
                        ),
                        arg(
                            "--output",
                            "-o",
                            "path",
                            true,
                            None,
                            desc(compact, "Output file")
                        ),
                    ]
                ),
            ]
        ),
        cmd_sub(
            "database",
            "rag",
            compact,
            "RAG knowledge base operations",
            vec![
                subcmd(
                    "init",
                    compact,
                    "Initialize database",
                    vec![
                        arg(
                            "--path",
                            "-p",
                            "path",
                            true,
                            None,
                            desc(compact, "Database path")
                        ),
                        arg(
                            "--db-type",
                            "-t",
                            "string",
                            false,
                            Some(json!("sqlite")),
                            desc(compact, "Database type")
                        ),
                    ]
                ),
                subcmd(
                    "store",
                    compact,
                    "Store document",
                    vec![
                        arg(
                            "--path",
                            "-p",
                            "path",
                            true,
                            None,
                            desc(compact, "Database path")
                        ),
                        arg(
                            "--input",
                            "-i",
                            "path",
                            true,
                            None,
                            desc(compact, "Document file")
                        ),
                    ]
                ),
                subcmd(
                    "search",
                    compact,
                    "Search documents",
                    vec![
                        arg(
                            "--path",
                            "-p",
                            "path",
                            true,
                            None,
                            desc(compact, "Database path")
                        ),
                        arg(
                            "query",
                            "",
                            "string",
                            true,
                            None,
                            desc(compact, "Search query")
                        ),
                        arg(
                            "--limit",
                            "-n",
                            "usize",
                            false,
                            Some(json!(10)),
                            desc(compact, "Max results")
                        ),
                    ]
                ),
                subcmd(
                    "list",
                    compact,
                    "List documents",
                    vec![arg(
                        "--path",
                        "-p",
                        "path",
                        true,
                        None,
                        desc(compact, "Database path")
                    ),]
                ),
                subcmd(
                    "delete",
                    compact,
                    "Delete document",
                    vec![
                        arg(
                            "--path",
                            "-p",
                            "path",
                            true,
                            None,
                            desc(compact, "Database path")
                        ),
                        arg("id", "", "string", true, None, desc(compact, "Document ID")),
                    ]
                ),
                subcmd(
                    "stats",
                    compact,
                    "Show database statistics",
                    vec![arg(
                        "--path",
                        "-p",
                        "path",
                        true,
                        None,
                        desc(compact, "Database path")
                    ),]
                ),
            ]
        ),
        cmd_sub(
            "telemetry",
            "config",
            compact,
            "Telemetry settings",
            vec![
                subcmd("status", compact, "Show telemetry status", vec![]),
                subcmd("enable", compact, "Enable telemetry", vec![]),
                subcmd("disable", compact, "Disable telemetry", vec![]),
                subcmd("reset", compact, "Reset device ID", vec![]),
            ]
        ),
        cmd(
            "introspect",
            "agent",
            compact,
            "Output CLI schema for agent discovery",
            vec![
                arg(
                    "--format",
                    "-f",
                    "string",
                    false,
                    Some(json!("json")),
                    desc(compact, "Output format (json, yaml, jsonld)")
                ),
                arg(
                    "--compact",
                    "",
                    "bool",
                    false,
                    None,
                    desc(compact, "Minimize output")
                ),
            ],
            ex(
                compact,
                &[
                    "iv introspect",
                    "iv introspect --format yaml",
                    "iv introspect --compact"
                ]
            )
        ),
    ])
}

// Helper builders

fn null_val() -> Option<Value> {
    None
}

fn desc(compact: bool, d: &str) -> Option<String> {
    if compact {
        None
    } else {
        Some(d.to_string())
    }
}

fn ex(compact: bool, examples: &[&str]) -> Option<Vec<String>> {
    if compact {
        None
    } else {
        Some(examples.iter().map(|s| s.to_string()).collect())
    }
}

fn flag(
    name: &str,
    short: &str,
    typ: &str,
    required: bool,
    default: Option<Value>,
    description: Option<String>,
) -> Value {
    let mut f = json!({"name": name, "type": typ, "required": required});
    if !short.is_empty() {
        f["short"] = json!(short);
    }
    if let Some(d) = default {
        f["default"] = d;
    }
    if let Some(d) = description {
        f["description"] = json!(d);
    }
    f
}

fn arg(
    name: &str,
    short: &str,
    typ: &str,
    required: bool,
    default: Option<Value>,
    description: Option<String>,
) -> Value {
    let mut a = json!({"name": name, "type": typ, "required": required});
    if !short.is_empty() {
        a["short"] = json!(short);
    }
    if let Some(d) = default {
        a["default"] = d;
    }
    if let Some(d) = description {
        a["description"] = json!(d);
    }
    if !name.starts_with('-') {
        a["positional"] = json!(true);
    }
    a
}

fn cmd(
    name: &str,
    category: &str,
    compact: bool,
    description: &str,
    args: Vec<Value>,
    examples: Option<Vec<String>>,
) -> Value {
    let mut c = json!({"name": name, "category": category, "args": args});
    if !compact {
        c["description"] = json!(description);
    }
    if let Some(ex) = examples {
        c["examples"] = json!(ex);
    }
    c
}

fn cmd_sub(
    name: &str,
    category: &str,
    compact: bool,
    description: &str,
    subcommands: Vec<Value>,
) -> Value {
    let mut c = json!({"name": name, "category": category, "subcommands": subcommands});
    if !compact {
        c["description"] = json!(description);
    }
    c
}

fn subcmd(name: &str, compact: bool, description: &str, args: Vec<Value>) -> Value {
    let mut c = json!({"name": name, "args": args});
    if !compact {
        c["description"] = json!(description);
    }
    c
}
