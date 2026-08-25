//! The canonical IronVault CLI schema, as data.
//!
//! Lives in the library rather than the binary so that every surface serves
//! the *same* schema: `iv introspect` renders it, and `GET /api/v1/introspect`
//! returns it. When it lived in the binary the REST endpoint could not exist,
//! which is part of why the OpenAPI spec documented a path with no handler.

use serde_json::{json, Value};

/// Build the complete CLI schema.
///
/// `compact` drops descriptions and examples for a smaller payload.
pub fn build(compact: bool) -> Value {
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
    let mut commands = json!([
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
                arg(
                    "--from-dir",
                    "",
                    "path",
                    false,
                    None,
                    desc(
                        compact,
                        "HuggingFace checkpoint directory; required for the native \
                         safetensors → GGUF path. The vault is not opened."
                    )
                ),
            ],
            ex(
                compact,
                &[
                    "iv convert my-llm -t gguf -q q4_k_m --validate",
                    "iv convert tinyllama --from-dir ./TinyLlama-1.1B -t gguf",
                ]
            )
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
        // -- Model acquisition and inspection ------------------------------
        cmd(
            "pull",
            "vault",
            compact,
            "Download a model from HuggingFace, Ollama, or an HTTPS URL",
            vec![
                arg("source", "", "string", true, None, desc(compact, "Model source URI (hf://org/model, ollama://model:tag, or https://...)")),
                arg("--output", "-o", "path", false, None, desc(compact, "Output file path")),
                arg("--sha256", "", "string", false, None, desc(compact, "Expected SHA-256 hash for verification")),
                arg("--token", "", "string", false, None, desc(compact, "HuggingFace auth token (for private repos)")),
                arg("--store", "", "bool", false, None, desc(compact, "Store directly into the vault after download")),
                arg("--name", "", "string", false, None, desc(compact, "Model name when using --store")),
            ],
            ex(compact, &["iv pull hf://meta-llama/Llama-3-8B", "iv pull ollama://llama3:8b --store --name llama3"])
        ),
        cmd(
            "browse",
            "vault",
            compact,
            "Browse vault models in a formatted table",
            vec![],
            ex(compact, &["iv browse"])
        ),
        cmd(
            "search",
            "vault",
            compact,
            "Search models by name or tags",
            vec![
                arg("query", "", "string", false, Some(json!("")), desc(compact, "Search query text")),
                arg("--tag", "-t", "string[]", false, None, desc(compact, "Filter by tags")),
                arg("--format", "-f", "string", false, None, desc(compact, "Output format (text, json)")),
            ],
            ex(compact, &["iv search llama", "iv search --tag production"])
        ),
        cmd(
            "diff",
            "utility",
            compact,
            "Compare two model versions or files",
            vec![
                arg("left", "", "string", true, None, desc(compact, "Left model (name@version or file path)")),
                arg("right", "", "string", true, None, desc(compact, "Right model (name@version or file path)")),
                arg("--format", "-f", "string", false, Some(json!("text")), desc(compact, "Output format (text, json)")),
            ],
            ex(compact, &["iv diff mymodel@1 mymodel@2"])
        ),

        // -- Vault maintenance ---------------------------------------------
        cmd(
            "gc",
            "vault",
            compact,
            "Garbage-collect orphaned blobs and temp files",
            vec![arg("--dry-run", "", "bool", false, None, desc(compact, "Show what would be cleaned without deleting"))],
            ex(compact, &["iv gc --dry-run", "iv gc"])
        ),
        cmd(
            "vault-export",
            "vault",
            compact,
            "Export the whole vault as a portable archive",
            vec![arg("output", "", "path", true, None, desc(compact, "Output archive path (.tar.gz)"))],
            ex(compact, &["iv vault-export backup.tar.gz"])
        ),
        cmd(
            "vault-import",
            "vault",
            compact,
            "Import a vault from a portable archive",
            vec![
                arg("archive", "", "path", true, None, desc(compact, "Archive path")),
                arg("--target", "-t", "path", false, None, desc(compact, "Target directory (defaults to current vault)")),
            ],
            ex(compact, &["iv vault-import backup.tar.gz"])
        ),

        // -- Provenance and safety -----------------------------------------
        cmd(
            "sign",
            "security",
            compact,
            "Sign a model for provenance verification",
            vec![
                arg("name", "", "string", true, None, desc(compact, "Model name in vault (or path to file with --file)")),
                arg("--version", "-v", "u32", false, None, desc(compact, "Version number (latest if not specified)")),
                arg("--key", "-k", "string", false, None, desc(compact, "Signing key pair: a file path, or a KMS URI such as azure-kv://my-vault/hmac-key")),
                arg("--identity", "-i", "string", false, None, desc(compact, "Signer identity (e.g. email or name)")),
                arg("--file", "", "path", false, None, desc(compact, "Sign a file on disk instead of a vault model")),
            ],
            ex(compact, &["iv sign mymodel --identity alice@example.com"])
        ),
        cmd(
            "verify",
            "security",
            compact,
            "Verify a model's signature",
            vec![
                arg("name", "", "string", true, None, desc(compact, "Model name in vault (or path to file with --file)")),
                arg("--version", "-v", "u32", false, None, desc(compact, "Version number (latest if not specified)")),
                arg("--signature", "-s", "path", true, None, desc(compact, "Path to signature file")),
                arg("--key", "-k", "string", false, None, desc(compact, "Verification key: a file path or a KMS URI. Omit to check the file hash only")),
                arg("--file", "", "path", false, None, desc(compact, "Verify a file on disk instead of a vault model")),
            ],
            ex(compact, &["iv verify mymodel --signature mymodel.sig"])
        ),
        cmd(
            "scan",
            "security",
            compact,
            "Scan a model for pickle deserialization vulnerabilities",
            vec![
                arg("name", "", "string", false, None, desc(compact, "Model name in vault")),
                arg("--file", "", "path", false, None, desc(compact, "Scan a file on disk instead of a vault model")),
                arg("--version", "-v", "u32", false, None, desc(compact, "Version number (latest if not specified)")),
                arg("--format", "-f", "string", false, Some(json!("text")), desc(compact, "Output format (text, json)")),
            ],
            ex(compact, &["iv scan mymodel", "iv scan --file suspect.pt"])
        ),
        cmd(
            "validate",
            "security",
            compact,
            "Validate a stored model's integrity",
            vec![
                arg("name", "", "string", true, None, desc(compact, "Model name")),
                arg("--version", "-v", "u32", false, None, desc(compact, "Version (latest if not specified)")),
            ],
            ex(compact, &["iv validate mymodel"])
        ),
        cmd(
            "license-scan",
            "security",
            compact,
            "Scan model files or directories for license information",
            vec![
                arg("path", "", "path", true, None, desc(compact, "Path to scan (file or directory)")),
                arg("--format", "-f", "string", false, Some(json!("text")), desc(compact, "Output format (text, json)")),
            ],
            ex(compact, &["iv license-scan ./models"])
        ),
        cmd(
            "register",
            "utility",
            compact,
            "Register a vault model with an inference engine (Ollama, LM Studio)",
            vec![
                arg("name", "", "string", true, None, desc(compact, "Model name in vault")),
                arg("--engine", "-e", "string", true, None, desc(compact, "Inference engine (ollama, lm-studio)")),
                arg("--version", "-v", "u32", false, None, desc(compact, "Version number (latest if not specified)")),
                arg("--alias", "", "string", false, None, desc(compact, "Name to register the model as (defaults to vault model name)")),
                arg("--system-prompt", "", "string", false, None, desc(compact, "System prompt (Ollama only)")),
            ],
            ex(compact, &["iv register mymodel --engine ollama"])
        ),

        // -- Command groups -------------------------------------------------
        cmd_sub(
            "chain",
            "security",
            compact,
            "Blockchain audit trail (requires security.blockchain_audit = true)",
            vec![
                subcmd("status", compact, "Show chain height, latest block hash, and pending entry count", vec![]),
                subcmd("verify", compact, "Verify the whole chain: hash links, Merkle roots, and block hashes", vec![]),
                subcmd("proof", compact, "Emit a Merkle inclusion proof for one entry", vec![
                    arg("--block", "-b", "u64", true, None, desc(compact, "Block index")),
                    arg("--entry", "-e", "usize", true, None, desc(compact, "Entry index within that block")),
                    arg("--output", "-o", "path", false, None, desc(compact, "Write the proof JSON here instead of stdout")),
                ]),
                subcmd("verify-proof", compact, "Verify a proof produced by `iv chain proof`", vec![
                    arg("proof", "", "path", true, None, desc(compact, "Path to the proof JSON")),
                ]),
                subcmd("search", compact, "Search recorded entries", vec![
                    arg("--model", "-m", "string", false, None, desc(compact, "Filter by model name")),
                    arg("--event", "-e", "string", false, None, desc(compact, "Filter by event type")),
                    arg("--limit", "-l", "usize", false, Some(json!(50)), desc(compact, "Maximum results")),
                ]),
            ]
        ),
        cmd_sub(
            "federation",
            "cloud",
            compact,
            "Sync models with peer nodes",
            vec![
                subcmd("status", compact, "Show this node's identity, peers, and sync history", vec![]),
                subcmd("plan", compact, "Show what a sync with a peer would transfer, without transferring it", vec![
                    arg("peer", "", "string", true, None, desc(compact, "Peer node ID")),
                ]),
                subcmd("sync", compact, "Sync with a peer: download what is missing here, upload what is missing there", vec![
                    arg("peer", "", "string", true, None, desc(compact, "Peer node ID")),
                    arg("--dry-run", "", "bool", false, None, desc(compact, "Report what would transfer, then stop")),
                ]),
                subcmd("manifest", compact, "Print this node's sync manifest (what peers would see)", vec![]),
            ]
        ),
        cmd_sub(
            "tag",
            "vault",
            compact,
            "Manage model tags",
            vec![
                subcmd("add", compact, "Add tags to a model", vec![
                    arg("model", "", "string", true, None, desc(compact, "Model name")),
                    arg("tags", "", "string[]", true, None, desc(compact, "Tags to add")),
                ]),
                subcmd("remove", compact, "Remove tags from a model", vec![
                    arg("model", "", "string", true, None, desc(compact, "Model name")),
                    arg("tags", "", "string[]", true, None, desc(compact, "Tags to remove")),
                ]),
                subcmd("list", compact, "List tags for a model", vec![
                    arg("model", "", "string", true, None, desc(compact, "Model name")),
                ]),
            ]
        ),
        cmd_sub(
            "acl",
            "security",
            compact,
            "Manage access control lists",
            vec![
                subcmd("grant", compact, "Grant a role to an identity", vec![
                    arg("identity", "", "string", true, None, desc(compact, "Identity (e.g. username or email)")),
                    arg("role", "", "string", true, None, desc(compact, "Role (reader, writer, admin)")),
                ]),
                subcmd("revoke", compact, "Revoke access for an identity", vec![
                    arg("identity", "", "string", true, None, desc(compact, "Identity")),
                ]),
                subcmd("list", compact, "List all ACL entries", vec![]),
            ]
        ),
        cmd_sub(
            "policy",
            "config",
            compact,
            "Manage retention policies",
            vec![
                subcmd("set", compact, "Set a retention policy for a model", vec![
                    arg("model", "", "string", true, None, desc(compact, "Model name")),
                    arg("--max-versions", "", "usize", false, None, desc(compact, "Maximum number of versions to keep")),
                    arg("--max-age-days", "", "u64", false, None, desc(compact, "Maximum age in days")),
                    arg("--keep-minimum", "", "usize", false, None, desc(compact, "Minimum versions to always keep")),
                ]),
                subcmd("remove", compact, "Remove a retention policy", vec![
                    arg("model", "", "string", true, None, desc(compact, "Model name")),
                ]),
                subcmd("show", compact, "Show the policy for a model", vec![
                    arg("model", "", "string", true, None, desc(compact, "Model name")),
                ]),
                subcmd("apply", compact, "Apply retention policies (delete old versions)", vec![
                    arg("--model", "-m", "string", false, None, desc(compact, "Apply only for a specific model")),
                    arg("--dry-run", "", "bool", false, None, desc(compact, "Show what would be deleted without deleting")),
                ]),
            ]
        ),
        cmd_sub(
            "webhook",
            "config",
            compact,
            "Manage webhook targets",
            vec![
                subcmd("add", compact, "Register a webhook target", vec![
                    arg("id", "", "string", true, None, desc(compact, "Webhook ID")),
                    arg("url", "", "string", true, None, desc(compact, "Target URL")),
                    arg("--secret", "-s", "string", false, None, desc(compact, "Shared secret for HMAC signing")),
                    arg("--events", "-e", "string[]", false, None, desc(compact, "Events to subscribe to (empty = all)")),
                ]),
                subcmd("remove", compact, "Remove a webhook", vec![
                    arg("id", "", "string", true, None, desc(compact, "Webhook ID")),
                ]),
                subcmd("list", compact, "List all webhooks", vec![]),
            ]
        ),
        cmd_sub(
            "lineage-graph",
            "vault",
            compact,
            "Cross-model lineage graph",
            vec![
                subcmd("add", compact, "Add a derivation edge", vec![
                    arg("parent", "", "string", true, None, desc(compact, "Parent model (name or name@version)")),
                    arg("child", "", "string", true, None, desc(compact, "Child model (name or name@version)")),
                    arg("--kind", "-k", "string", false, Some(json!("finetune")), desc(compact, "Derivation kind (finetune, quantize, merge, distill, convert)")),
                ]),
                subcmd("show", compact, "Show lineage for a model", vec![
                    arg("model", "", "string", true, None, desc(compact, "Model name")),
                    arg("--format", "-f", "string", false, Some(json!("text")), desc(compact, "Output format (text, json)")),
                ]),
            ]
        ),
        cmd_sub(
            "plugin",
            "config",
            compact,
            "Manage plugins",
            vec![
                subcmd("list", compact, "List installed plugins", vec![]),
                subcmd("install", compact, "Install a plugin from a directory", vec![
                    arg("path", "", "path", true, None, desc(compact, "Plugin directory path")),
                ]),
                subcmd("uninstall", compact, "Uninstall a plugin by ID", vec![
                    arg("id", "", "string", true, None, desc(compact, "Plugin ID")),
                ]),
                subcmd("discover", compact, "Discover plugins in the plugin directory", vec![]),
            ]
        ),
        cmd_sub(
            "profile",
            "config",
            compact,
            "Manage configuration profiles",
            vec![
                subcmd("create", compact, "Create a new profile", vec![
                    arg("name", "", "string", true, None, desc(compact, "Profile name")),
                    arg("--set", "-s", "string[]", false, None, desc(compact, "Key=value overrides")),
                ]),
                subcmd("activate", compact, "Activate a profile", vec![
                    arg("name", "", "string", true, None, desc(compact, "Profile name")),
                ]),
                subcmd("deactivate", compact, "Deactivate the current profile", vec![]),
                subcmd("list", compact, "List all profiles", vec![]),
                subcmd("show", compact, "Show a profile's details", vec![
                    arg("name", "", "string", true, None, desc(compact, "Profile name")),
                ]),
                subcmd("delete", compact, "Delete a profile", vec![
                    arg("name", "", "string", true, None, desc(compact, "Profile name")),
                ]),
            ]
        ),
        cmd_sub(
            "vaults",
            "config",
            compact,
            "Manage multiple vault registrations",
            vec![
                subcmd("register", compact, "Register a vault", vec![
                    arg("name", "", "string", true, None, desc(compact, "Vault name/alias")),
                    arg("path", "", "path", true, None, desc(compact, "Path to vault directory")),
                    arg("--description", "-d", "string", false, None, desc(compact, "Description")),
                ]),
                subcmd("unregister", compact, "Unregister a vault", vec![
                    arg("name", "", "string", true, None, desc(compact, "Vault name")),
                ]),
                subcmd("activate", compact, "Set the active vault", vec![
                    arg("name", "", "string", true, None, desc(compact, "Vault name")),
                ]),
                subcmd("deactivate", compact, "Clear the active vault", vec![]),
                subcmd("list", compact, "List registered vaults", vec![]),
            ]
        ),
        cmd_sub(
            "backup",
            "vault",
            compact,
            "Manage vault backup schedules",
            vec![
                subcmd("set", compact, "Create or update a backup schedule", vec![
                    arg("name", "", "string", true, None, desc(compact, "Schedule name")),
                    arg("--frequency", "-f", "string", true, None, desc(compact, "Frequency (hourly, daily, weekly, monthly)")),
                    arg("--max-backups", "-m", "usize", false, Some(json!(7)), desc(compact, "Maximum backups to retain")),
                    arg("--output-dir", "-o", "path", true, None, desc(compact, "Output directory for backup archives")),
                ]),
                subcmd("remove", compact, "Remove a backup schedule", vec![
                    arg("name", "", "string", true, None, desc(compact, "Schedule name")),
                ]),
                subcmd("list", compact, "List backup schedules", vec![]),
                subcmd("history", compact, "Show backup history", vec![
                    arg("--schedule", "-s", "string", false, None, desc(compact, "Filter by schedule name")),
                ]),
            ]
        ),
        cmd_sub(
            "benchmark",
            "utility",
            compact,
            "Record or view benchmark results for a model",
            vec![
                subcmd("add", compact, "Record a benchmark result for a model version", vec![
                    arg("name", "", "string", true, None, desc(compact, "Model name")),
                    arg("--version", "-v", "u32", true, None, desc(compact, "Version number")),
                    arg("--benchmark", "-b", "string", true, None, desc(compact, "Benchmark name (e.g. MMLU, HumanEval, perplexity)")),
                    arg("--score", "-s", "f64", true, None, desc(compact, "Score value")),
                    arg("--unit", "-u", "string", false, Some(json!("score")), desc(compact, "Unit (e.g. accuracy, ppl, pass@1, ms, tokens/s)")),
                    arg("--higher-is-better", "", "bool", false, None, desc(compact, "Higher is better")),
                    arg("--hardware", "", "string", false, None, desc(compact, "Hardware description")),
                    arg("--dataset", "", "string", false, None, desc(compact, "Dataset or split")),
                ]),
                subcmd("show", compact, "Show benchmark results for a model", vec![
                    arg("name", "", "string", true, None, desc(compact, "Model name")),
                    arg("--version", "-v", "u32", false, None, desc(compact, "Version number (shows all versions if not specified)")),
                    arg("--format", "-f", "string", false, Some(json!("text")), desc(compact, "Output format (text, json)")),
                ]),
            ]
        ),
        cmd_sub(
            "eval",
            "utility",
            compact,
            "Record and compare model evaluations",
            vec![
                subcmd("record", compact, "Record an evaluation run", vec![
                    arg("name", "", "string", true, None, desc(compact, "Model name")),
                    arg("--version", "-v", "u64", true, None, desc(compact, "Model version")),
                    arg("--suite", "-s", "string", true, None, desc(compact, "Evaluation suite name (e.g. mmlu, hellaswag)")),
                    arg("--metric", "-m", "string[]", true, None, desc(compact, "Metric in name=value format (repeatable)")),
                    arg("--unit", "-u", "string", false, Some(json!("score")), desc(compact, "Unit for all metrics")),
                    arg("--higher-is-better", "", "bool", false, Some(json!(true)), desc(compact, "Higher is better")),
                ]),
                subcmd("list", compact, "List evaluation runs for a model", vec![
                    arg("name", "", "string", true, None, desc(compact, "Model name")),
                    arg("--version", "-v", "u64", false, None, desc(compact, "Filter by version")),
                    arg("--format", "-f", "string", false, Some(json!("text")), desc(compact, "Output format (text, json)")),
                ]),
                subcmd("compare", compact, "Compare two model versions on a suite", vec![
                    arg("a", "", "string", true, None, desc(compact, "First model (name@version)")),
                    arg("b", "", "string", true, None, desc(compact, "Second model (name@version)")),
                    arg("--suite", "-s", "string", true, None, desc(compact, "Evaluation suite")),
                    arg("--format", "-f", "string", false, Some(json!("text")), desc(compact, "Output format (text, json)")),
                ]),
                subcmd("suites", compact, "List all known evaluation suites", vec![]),
            ]
        ),
        cmd_sub(
            "quantize",
            "conversion",
            compact,
            "Manage quantization profiles and estimate sizes",
            vec![
                subcmd("set", compact, "Create or update a quantization profile", vec![
                    arg("name", "", "string", true, None, desc(compact, "Profile name")),
                    arg("--method", "-m", "string", true, None, desc(compact, "Quantization method (q4_0, q4_k_m, q5_k_m, q8_0, f16, f32)")),
                    arg("--description", "-d", "string", false, None, desc(compact, "Description")),
                ]),
                subcmd("remove", compact, "Remove a quantization profile", vec![
                    arg("name", "", "string", true, None, desc(compact, "Profile name")),
                ]),
                subcmd("list", compact, "List quantization profiles", vec![]),
                subcmd("estimate", compact, "Estimate output size for a quantization method", vec![
                    arg("--size", "-s", "u64", true, None, desc(compact, "Original file size in bytes")),
                    arg("--from", "", "string", false, Some(json!("f32")), desc(compact, "Source precision")),
                    arg("--to", "-t", "string", true, None, desc(compact, "Target quantization method")),
                ]),
            ]
        ),
    ]);

    // `serve` exists only in an `api` build -- `Commands::Serve` in
    // `cli/args.rs` is `#[cfg(feature = "api")]`. Listing it unconditionally
    // made `iv introspect` advertise a subcommand the running binary would
    // reject as unrecognised, which is the same failure as documenting a REST
    // path with no handler: an agent trusts the discovery document and the
    // call fails. The schema now describes the binary it was compiled into.
    if !cfg!(feature = "api") {
        if let Some(list) = commands.as_array_mut() {
            list.retain(|c| c.get("name").and_then(Value::as_str) != Some("serve"));
        }
    }

    commands
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
    c["effects"] = effects_for(name);
    if !compact {
        c["description"] = json!(description);
    }
    if let Some(ex) = examples {
        c["examples"] = json!(ex);
    }
    c
}

/// What a command does to the vault, for an agent deciding what it may run.
///
/// Three fields, because two would hide the one that matters. `mutates` says
/// the vault's contents or configuration change. `network` says the command
/// listens or reaches out. `decrypts` says the command produces plaintext
/// model bytes outside the vault -- and that is the field a mutation flag
/// cannot express, because `get` and `export` change nothing and are exactly
/// the operations that disclose. An agent told only "read-only" about `get`
/// has been told the least useful true thing about it.
///
/// Where the answer depends on internals, `decrypts` is set the conservative
/// way. `analyze` and `deduplicate` compare model content; if a future
/// implementation compares ciphertext instead, this should be relaxed
/// deliberately rather than left optimistic by default. Under-declaring here
/// is the failure that matters.
fn effects_for(name: &str) -> Value {
    // (mutates, decrypts, network)
    let (mutates, decrypts, network) = match name {
        "init" => (true, false, false),
        "store" => (true, false, false),
        "delete" => (true, false, false),
        "change-passphrase" => (true, false, false),
        "extract" => (true, false, false),
        "get" => (false, true, false),
        "export" => (false, true, false),
        "archive" => (false, true, false),
        "convert" => (false, true, false),
        "analyze" => (false, true, false),
        "deduplicate" => (false, true, false),
        "serve" => (false, true, true),
        "list" | "versions" | "lineage" | "stats" | "cache" | "compliance" | "list-conversions"
        | "introspect" => (false, false, false),

        // Command groups, classified by the most permissive thing any of
        // their subcommands does -- a group is only as safe as its widest
        // member, and an agent reads the group before it reads the list.
        //
        // `cloud` pushes and pulls models, so it both discloses and leaves
        // the machine. `card` writes model cards. `database` initialises and
        // resets storage. `telemetry` toggles reporting and sends it.
        "cloud" => (true, true, true),
        "card" => (true, false, false),
        "database" => (true, false, false),
        "telemetry" => (true, false, true),
        // The 27 commands the schema had never declared. Each is classified
        // from what its handler actually reaches, not from its name:
        // `decrypts` is true wherever a command reads model content, because
        // the field exists to stop an agent treating a disclosing read as a
        // harmless one.
        "gc" => (true, false, false),
        "vault-import" => (true, true, false),
        "pull" => (true, false, true),
        "sign" => (true, true, true),
        "browse" | "search" | "license-scan" => (false, false, false),
        "diff" | "scan" | "validate" => (false, true, false),
        "verify" | "register" => (false, true, true),
        // `vault-export` copies blobs that are already sealed and re-seals the
        // index, so it writes no plaintext model bytes. It is still declared
        // as decrypting: it needs the key, and it puts the entire vault --
        // every model, and the inventory naming them -- into one portable
        // file. An agent gating on `decrypts` should stop before doing that,
        // and this is the same conservative call the file already makes for
        // `analyze` and `deduplicate`.
        "vault-export" => (false, true, false),
        // Groups, classified by their widest subcommand as above.
        // `federation` moves models between nodes, so it matches `cloud`.
        "federation" => (true, true, true),
        "acl" | "backup" | "benchmark" | "chain" | "eval" | "lineage-graph" | "plugin"
        | "policy" | "profile" | "quantize" | "tag" | "vaults" | "webhook" => (true, false, false),

        // Unreachable in a correct build: `every_command_declares_effects`
        // fails if a command reaches this arm. Declared as the most
        // restrictive combination so that an unclassified command is never
        // mistaken for a safe one, and flagged with `declared: false` so the
        // test can tell it apart from a command that genuinely does all
        // three -- `cloud` does, which is why sniffing the values alone was
        // the wrong check.
        _ => {
            return json!({
                "mutates": true,
                "decrypts": true,
                "network": true,
                "declared": false,
            })
        }
    };
    json!({
        "mutates": mutates,
        "decrypts": decrypts,
        "network": network,
        "declared": true,
    })
}

fn cmd_sub(
    name: &str,
    category: &str,
    compact: bool,
    description: &str,
    subcommands: Vec<Value>,
) -> Value {
    let mut c = json!({"name": name, "category": category, "subcommands": subcommands});
    c["effects"] = effects_for(name);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn command_names(schema: &Value) -> Vec<String> {
        schema["commands"]
            .as_array()
            .expect("commands is an array")
            .iter()
            .filter_map(|c| c.get("name").and_then(Value::as_str))
            .map(str::to_string)
            .collect()
    }

    /// The schema must describe the binary it was compiled into.
    ///
    /// `serve` is `#[cfg(feature = "api")]` in `cli/args.rs`. Advertising it
    /// from a build that cannot run it is the CLI equivalent of documenting a
    /// REST path with no handler: an agent reads the discovery document, issues
    /// the command, and gets "unrecognized subcommand".
    #[test]
    fn serve_is_advertised_only_when_it_exists() {
        let names = command_names(&build(false));
        assert_eq!(
            names.iter().any(|n| n == "serve"),
            cfg!(feature = "api"),
            "`serve` presence in the schema must track the `api` feature; \
             schema listed: {names:?}"
        );
    }

    /// Commands that are not feature-gated must always be present, so the
    /// filter above cannot quietly remove more than it should.
    #[test]
    fn ungated_commands_are_always_present() {
        let names = command_names(&build(false));
        for expected in ["init", "store", "get", "list", "introspect"] {
            assert!(
                names.iter().any(|n| n == expected),
                "`{expected}` is not feature-gated but is missing from the schema"
            );
        }
    }

    /// Compact mode drops prose, not commands.
    #[test]
    fn compact_keeps_every_command() {
        assert_eq!(
            command_names(&build(true)).len(),
            command_names(&build(false)).len()
        );
    }

    #[test]
    fn the_schema_names_the_current_binary_and_version() {
        let schema = build(false);
        assert_eq!(schema["binary"], "iv");
        assert_eq!(schema["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(schema["install"], "cargo install ironvault");
    }

    /// Every command declares its effects, and none falls through to the
    /// catch-all.
    ///
    /// The catch-all in `effects_for` returns the most restrictive
    /// combination, so a command added without a classification is refused
    /// rather than waved through -- but that is a safety net, not the
    /// intended path. This test is what makes adding a command force the
    /// question "does this decrypt anything?" to be answered.
    #[test]
    fn every_command_declares_effects() {
        let schema = build(false);
        let commands = schema["commands"].as_array().expect("commands is an array");
        assert!(!commands.is_empty(), "schema declares no commands");

        let mut unclassified = Vec::new();
        for c in commands {
            let name = c["name"].as_str().unwrap_or("<unnamed>");
            let effects = &c["effects"];
            assert!(
                effects.is_object(),
                "command `{name}` has no effects object"
            );
            if effects["declared"] != json!(true) {
                unclassified.push(name.to_string());
            }
        }
        assert!(
            unclassified.is_empty(),
            "these commands hit the unclassified catch-all in effects_for: {unclassified:?}"
        );
    }

    /// The commands that take plaintext out of the vault are declared as
    /// doing so.
    ///
    /// Pinned by name because this is the security-relevant claim in the
    /// whole schema: an agent gating only on `mutates` would treat `get` and
    /// `export` as harmless reads. If a future change makes one of these stop
    /// decrypting, this test should be edited deliberately.
    #[test]
    fn disclosing_commands_declare_decryption() {
        let schema = build(false);
        let commands = schema["commands"].as_array().unwrap();
        for name in ["get", "export", "convert"] {
            let c = commands
                .iter()
                .find(|c| c["name"] == json!(name))
                .unwrap_or_else(|| panic!("`{name}` is missing from the schema"));
            assert_eq!(
                c["effects"]["decrypts"],
                json!(true),
                "`{name}` produces plaintext but does not declare it"
            );
        }
    }
}
