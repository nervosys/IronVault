# `.well-known/` — Agent Discovery Manifests

This directory is the canonical machine-readable surface for **IronVault**. Every agent, LLM client, IDE assistant, or automation pipeline should look here first.

## Read in this order

| #   | File                                     | When to read it                                                                         |
| --- | ---------------------------------------- | --------------------------------------------------------------------------------------- |
| 1   | [`agents.json`](agents.json)             | Always first — capability catalog, taxonomy, interface inventory, version               |
| 2   | [`ai-plugin.json`](ai-plugin.json)       | OpenAI-style plugin manifest; cross-links to every other file in this directory         |
| 3   | [`mcp-manifest.json`](mcp-manifest.json) | If you can speak MCP — 86 tools with full JSON Schema inputs, 7 resources, 4 prompts    |
| 4   | [`openapi.yaml`](openapi.yaml)           | If you prefer REST — OpenAPI 3.1 with 53 endpoints across 20 tag groups                 |
| 5   | [`ontology.jsonld`](ontology.jsonld)     | If you need a semantic model — JSON-LD classes for every concept, relation, and feature |

For prose context that complements these manifests, see [`../AGENTS.md`](../AGENTS.md).

## Versioning

All manifests are versioned alongside the crate (`ironvault`). The current schema version is **1.6.0**. Breaking changes to any manifest will bump the **major** version. Additions (new tools, endpoints, capability blocks) bump the minor version.

## Live introspection

When `iv` is installed, the most authoritative source for the **CLI surface** is the binary itself:

```bash
iv introspect --format json      # full schema
iv introspect --format jsonld    # linked to this ontology
iv introspect --format yaml      # human-readable
iv introspect --compact          # omit descriptions / examples
```

This output is generated from the same code that implements the CLI, so it can never drift from reality.

## Three-surface parity

Every one of the 29 features listed in [`agents.json`](agents.json) is reachable from **all three** of:

1. A CLI subcommand (`iv …`)
2. A REST endpoint (in [`openapi.yaml`](openapi.yaml))
3. An MCP tool (in [`mcp-manifest.json`](mcp-manifest.json))

If you find a gap, please open an issue — parity is a project invariant.
