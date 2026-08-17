//! Pins `.well-known/mcp-manifest.json` to what the crate actually registers.
//!
//! The REST spec got [`openapi_drift_test`] in 5.1.0 after it drifted to 14
//! documented paths with no handler. The third surface never got the
//! equivalent guard, and drifted further: the manifest declares 86 tools, the
//! crate registers 4, and the README advertised the 86 as shipped until 7.2.
//!
//! The gap is not itself a bug — MCP here is a library surface, and the
//! manifest is the schema a host process registers against. What was a bug is
//! that nothing held the documentation to the real number, so the two could
//! diverge in silence. That is what this test fixes.
//!
//! It deliberately does *not* demand that every declared tool be implemented.
//! It demands that the counts stay pinned, so growing either side without
//! saying so fails the build.

use std::collections::BTreeSet;

fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Tools `MCPServer::register_builtin_tools` installs.
///
/// Hard-coded rather than derived: the point is that changing the set has to
/// be a deliberate edit here, visible in review, not an invisible consequence
/// of editing `src/rag/mcp.rs`.
const BUILTIN_TOOLS: &[&str] = &[
    "search_documents",
    "add_document",
    "chunk_text",
    "execute_rule",
];

/// What the manifest declares. A surface definition, not an inventory of
/// shipped code — see the module docs.
const DECLARED_TOOL_COUNT: usize = 86;

fn manifest_tool_names() -> BTreeSet<String> {
    let raw = std::fs::read_to_string(manifest_dir().join(".well-known/mcp-manifest.json"))
        .expect(".well-known/mcp-manifest.json is readable");
    let doc: serde_json::Value =
        serde_json::from_str(&raw).expect(".well-known/mcp-manifest.json is valid JSON");

    doc["tools"]
        .as_array()
        .expect("mcp-manifest.json has a `tools` array")
        .iter()
        .map(|t| {
            t["name"]
                .as_str()
                .expect("every tool has a string `name`")
                .to_string()
        })
        .collect()
}

#[test]
fn the_manifest_declares_the_documented_number_of_tools() {
    let names = manifest_tool_names();

    assert_eq!(
        names.len(),
        DECLARED_TOOL_COUNT,
        "mcp-manifest.json declares {} tools, but this test, the README and \
         AGENTS.md all say {}. Update every one of them together -- the last \
         time these drifted, the README advertised 86 shipped tools against 4 \
         that existed.",
        names.len(),
        DECLARED_TOOL_COUNT,
    );
}

#[test]
fn every_builtin_tool_is_declared_in_the_manifest() {
    let declared = manifest_tool_names();

    for tool in BUILTIN_TOOLS {
        assert!(
            declared.contains(*tool),
            "`{tool}` is registered by MCPServer::register_builtin_tools but is \
             not in mcp-manifest.json. A tool an agent can actually call, that \
             the discovery document omits, is the drift that matters most: \
             callers never learn it exists."
        );
    }
}

#[test]
fn the_builtin_tool_set_has_not_changed_silently() {
    // `register_builtin_tools` is the only shipped registration site. If it
    // grows, the counts in README.md ("4 built-in, 86 declared"), AGENTS.md
    // and the MCP tools table need to grow with it.
    let src = std::fs::read_to_string(manifest_dir().join("src/rag/mcp.rs"))
        .expect("src/rag/mcp.rs is readable");

    let body = src
        .split_once("pub fn register_builtin_tools")
        .expect("register_builtin_tools still exists")
        .1;

    let registered = body.matches("self.register_tool(").count();

    assert_eq!(
        registered,
        BUILTIN_TOOLS.len(),
        "register_builtin_tools now installs {} tools, not {}. Update \
         BUILTIN_TOOLS here, the `MCP Tools` table in AGENTS.md, and the \
         `4 built-in` counts in README.md.",
        registered,
        BUILTIN_TOOLS.len(),
    );
}

/// `agents.json` is the machine-readable one, so overstating there is worse
/// than overstating in prose: an agent parses `builtin_tools` and believes it.
/// It listed 54 -- a third number, agreeing with neither the manifest's 86 nor
/// the 4 that exist -- and described an "MCP server" the crate does not ship.
#[test]
fn agents_json_lists_only_the_tools_that_are_really_built_in() {
    let raw = std::fs::read_to_string(manifest_dir().join(".well-known/agents.json"))
        .expect(".well-known/agents.json is readable");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("agents.json is valid JSON");

    let mcp = doc["agent_interfaces"]
        .as_array()
        .expect("agent_interfaces is an array")
        .iter()
        .find(|i| i["type"] == "mcp")
        .expect("an mcp interface entry exists");

    let listed: BTreeSet<String> = mcp["builtin_tools"]
        .as_array()
        .expect("builtin_tools is an array")
        .iter()
        .map(|v| v.as_str().expect("tool names are strings").to_string())
        .collect();

    let expected: BTreeSet<String> = BUILTIN_TOOLS.iter().map(|s| (*s).to_string()).collect();

    assert_eq!(
        listed, expected,
        "agents.json `builtin_tools` must list exactly the tools \
         register_builtin_tools installs. The declared surface belongs in \
         `declared_tools`, which points at the manifest."
    );

    assert_eq!(
        mcp["declared_tools"]["count"].as_u64(),
        Some(DECLARED_TOOL_COUNT as u64),
        "agents.json `declared_tools.count` must match the manifest."
    );
}

#[test]
fn the_readme_and_agents_md_do_not_advertise_declared_tools_as_shipped() {
    // The exact sentence this guards against: "86 MCP tools", which read as an
    // inventory of working tools rather than a schema to register against.
    for rel in ["README.md", "AGENTS.md"] {
        let text = std::fs::read_to_string(manifest_dir().join(rel))
            .unwrap_or_else(|e| panic!("{rel} is readable: {e}"));

        assert!(
            !text.contains("86 MCP tools"),
            "{rel} says \"86 MCP tools\". Only {} ship; the other {} are \
             definitions a host process registers itself. Say \"86-tool \
             surface definition\" or \"4 built-in, 86 declared\".",
            BUILTIN_TOOLS.len(),
            DECLARED_TOOL_COUNT - BUILTIN_TOOLS.len(),
        );
    }
}

/// The CLI's JSON-LD `@context` and the published ontology must mint the same
/// IRIs, or a consumer joining `iv introspect --format jsonld` with
/// `.well-known/ontology.jsonld` sees two unrelated vocabularies.
///
/// They diverged exactly that way once before. The fix carried a comment
/// saying the two "must match" and nothing that made it so, which is why they
/// could then be reconciled onto a domain the project does not own without
/// anything noticing. This is the part that makes it so.
#[test]
fn the_cli_and_the_ontology_mint_the_same_vocabulary_iri() {
    let ontology_raw = std::fs::read_to_string(manifest_dir().join(".well-known/ontology.jsonld"))
        .expect(".well-known/ontology.jsonld is readable");
    let ontology: serde_json::Value =
        serde_json::from_str(&ontology_raw).expect("ontology.jsonld is valid JSON");

    let published = ontology["@context"]["iv"]
        .as_str()
        .expect("ontology.jsonld binds an `iv` prefix");

    let introspect = std::fs::read_to_string(manifest_dir().join("src/cli/handlers/introspect.rs"))
        .expect("src/cli/handlers/introspect.rs is readable");

    assert!(
        introspect.contains(&format!("\"iv\": \"{published}\"")),
        "introspect.rs does not bind `iv` to {published}, the IRI \
         ontology.jsonld publishes. Joining the CLI's JSON-LD with the \
         published ontology would yield two unrelated vocabularies."
    );

    assert!(
        published.starts_with("https://nervosys.ai/"),
        "the vocabulary is minted under {published}. It belongs on a domain \
         the project controls: nervosys.com is a parked Afternic listing with \
         a null MX, so anyone could buy it and serve these IRIs."
    );
}
