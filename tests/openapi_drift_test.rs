//! Pins `.well-known/openapi.yaml` to the routes the server actually registers.
//!
//! This exists because the two drifted badly and silently. Before 5.1.0 the
//! spec documented 53 paths against 44 registered routes, and the sets were not
//! nested: 14 documented paths had no handler at all, so any client generated
//! from the published spec emitted calls that 404'd. For a project whose
//! premise is machine-readable discovery, a discovery document that describes
//! endpoints which do not exist is worse than having none.
//!
//! Reconciling it once was the easy part. This test is the part that keeps it
//! reconciled: adding a route without documenting it, or documenting a path
//! without implementing it, now fails the build.
//!
//! It reads the router source rather than introspecting a live `Router` because
//! axum exposes no way to enumerate registered paths.

#![cfg(feature = "api")]

use std::collections::BTreeSet;

/// Routes deliberately absent from the REST spec.
///
/// These are not `/api/v1` REST resources: `/` serves the embedded HTML
/// dashboard and `/graphql` is a separate protocol with its own schema, which
/// OpenAPI cannot usefully describe. Anything else missing is drift.
const NOT_REST_RESOURCES: &[&str] = &["/", "/graphql"];

fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Extract every path passed to `.route(...)` in the router source.
///
/// Normalises axum's `:param` to OpenAPI's `{param}` so the two are comparable.
fn registered_routes() -> BTreeSet<String> {
    let src = std::fs::read_to_string(manifest_dir().join("src/api/server.rs"))
        .expect("router source is readable");

    let mut out = BTreeSet::new();
    // `.route(` and its path may be on separate lines, so scan the whole text
    // for the literal that follows each occurrence.
    for (idx, _) in src.match_indices(".route(") {
        let rest = &src[idx + ".route(".len()..];
        let Some(open) = rest.find('"') else { continue };
        // Only accept a quote that precedes the first `,` or `)`, so a
        // `.route(` with no literal path does not swallow an unrelated string.
        let boundary = rest.find([',', ')']).unwrap_or(rest.len());
        if open > boundary {
            continue;
        }
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else {
            continue;
        };
        let path = &after[..close];

        let normalised = path
            .split('/')
            .map(|seg| {
                seg.strip_prefix(':')
                    .map_or_else(|| seg.to_string(), |p| format!("{{{p}}}"))
            })
            .collect::<Vec<_>>()
            .join("/");
        out.insert(normalised);
    }

    assert!(
        out.len() > 20,
        "route extraction found only {} paths — the parser is broken, not the router",
        out.len()
    );
    out
}

/// Extract every path documented in the OpenAPI spec, minus the `/api/v1` prefix.
fn documented_paths() -> BTreeSet<String> {
    let spec = std::fs::read_to_string(manifest_dir().join(".well-known/openapi.yaml"))
        .expect("openapi.yaml is readable");

    let mut out = BTreeSet::new();
    let mut in_paths = false;
    for line in spec.lines() {
        if line.starts_with("paths:") {
            in_paths = true;
            continue;
        }
        // A new top-level key ends the paths block.
        if in_paths && !line.starts_with(' ') && !line.trim().is_empty() {
            break;
        }
        if !in_paths {
            continue;
        }
        // Path items sit at exactly two spaces of indentation: `  /foo:`.
        let Some(rest) = line.strip_prefix("  ") else {
            continue;
        };
        if rest.starts_with(' ') || !rest.starts_with('/') {
            continue;
        }
        let Some(path) = rest.strip_suffix(':') else {
            continue;
        };
        out.insert(
            path.strip_prefix("/api/v1")
                .unwrap_or(path)
                .trim()
                .to_string(),
        );
    }

    assert!(
        out.len() > 20,
        "spec extraction found only {} paths — the parser is broken, not the spec",
        out.len()
    );
    out
}

#[test]
fn every_registered_route_is_documented() {
    let registered = registered_routes();
    let documented = documented_paths();
    let exempt: BTreeSet<String> = NOT_REST_RESOURCES
        .iter()
        .map(|s| (*s).to_string())
        .collect();

    let undocumented: Vec<&String> = registered
        .iter()
        .filter(|p| !documented.contains(*p) && !exempt.contains(*p))
        .collect();

    assert!(
        undocumented.is_empty(),
        "these routes are registered but absent from .well-known/openapi.yaml, so \
         agents cannot discover them:\n  {}\n\nDocument them, or add them to \
         NOT_REST_RESOURCES with a reason.",
        undocumented
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

#[test]
fn every_documented_path_has_a_handler() {
    let registered = registered_routes();
    let documented = documented_paths();

    let phantom: Vec<&String> = documented
        .iter()
        .filter(|p| !registered.contains(*p))
        .collect();

    assert!(
        phantom.is_empty(),
        "these paths are documented in .well-known/openapi.yaml but no handler is \
         registered, so a generated client will call them and get 404:\n  {}\n\n\
         Implement them, or remove them from the spec.",
        phantom
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

/// The spec must be valid YAML with the structure OpenAPI requires.
///
/// The other tests in this file scan lines, so a syntax error that made the
/// document unparseable would slip past them while breaking every real client.
#[test]
fn the_spec_is_parseable_yaml() {
    let raw = std::fs::read_to_string(manifest_dir().join(".well-known/openapi.yaml"))
        .expect("openapi.yaml is readable");

    let doc: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&raw).expect("openapi.yaml must be valid YAML");

    let map = doc.as_mapping().expect("top level is a mapping");
    for key in ["openapi", "info", "paths", "components", "tags"] {
        assert!(
            map.contains_key(serde_yaml_ng::Value::String(key.into())),
            "openapi.yaml is missing the top-level `{key}` key"
        );
    }

    // Cross-check the line scanner against the parser: if they disagree, one
    // of them is wrong and every other assertion here is untrustworthy.
    let parsed_paths = map
        .get(serde_yaml_ng::Value::String("paths".into()))
        .and_then(|p| p.as_mapping())
        .expect("paths is a mapping")
        .len();
    assert_eq!(
        parsed_paths,
        documented_paths().len(),
        "the YAML parser and the line scanner disagree about how many paths exist"
    );
}

/// Every tag an operation uses must be declared, and every declared tag used.
///
/// Undeclared tags render as bare strings with no description in generated
/// docs, and 12 of them had accumulated by 5.0 — including `Cards` and
/// `Model Cards` as two names for one group.
#[test]
fn tags_are_declared_and_used() {
    let spec = std::fs::read_to_string(manifest_dir().join(".well-known/openapi.yaml"))
        .expect("openapi.yaml is readable");

    let mut used = BTreeSet::new();
    let mut declared = BTreeSet::new();
    // `- name:` also introduces a *parameter*, so declarations are only
    // collected inside the top-level `tags:` block.
    let mut in_tag_block = false;
    for line in spec.lines() {
        if line.starts_with("tags:") {
            in_tag_block = true;
            continue;
        }
        if in_tag_block && !line.starts_with(' ') && !line.trim().is_empty() {
            in_tag_block = false;
        }

        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("tags: [") {
            if let Some(inner) = rest.strip_suffix(']') {
                for tag in inner.split(',') {
                    used.insert(tag.trim().to_string());
                }
            }
        } else if in_tag_block {
            if let Some(name) = line.strip_prefix("  - name: ") {
                declared.insert(name.trim().to_string());
            }
        }
    }

    assert!(
        declared.len() > 10,
        "tag extraction found only {} declarations — the parser is broken",
        declared.len()
    );

    let undeclared: Vec<_> = used.difference(&declared).cloned().collect();
    assert!(
        undeclared.is_empty(),
        "tags used by operations but never declared: {undeclared:?}"
    );

    let unused: Vec<_> = declared.difference(&used).cloned().collect();
    assert!(
        unused.is_empty(),
        "tags declared but used by no operation: {unused:?}"
    );
}

/// The spec's advertised version must track the crate's.
#[test]
fn spec_version_matches_the_crate() {
    let spec = std::fs::read_to_string(manifest_dir().join(".well-known/openapi.yaml"))
        .expect("openapi.yaml is readable");
    let declared = spec
        .lines()
        .find_map(|l| l.trim_start().strip_prefix("version: "))
        .expect("openapi.yaml declares an info.version")
        .trim()
        .to_string();
    assert_eq!(
        declared,
        env!("CARGO_PKG_VERSION"),
        "openapi.yaml version drifted from Cargo.toml"
    );
}

/// No published manifest may advertise a pre-5.0 environment variable name.
///
/// The 5.0 rename collapsed `aimodelvault_*` and `AIM_*` into `IRONVAULT_*`,
/// but missed both `.well-known/agents.json` and `.well-known/ontology.jsonld`
/// — the two documents an agent actually reads to learn how to configure this
/// thing. `src/env.rs` still accepts the old names, so nothing broke loudly;
/// agents were simply being handed names scheduled for removal in 6.0.
#[test]
fn manifests_do_not_advertise_pre_5_0_environment_names() {
    let manifests = [
        ".well-known/agents.json",
        ".well-known/ontology.jsonld",
        ".well-known/mcp-manifest.json",
        ".well-known/openapi.yaml",
        ".well-known/ai-plugin.json",
    ];

    for rel in manifests {
        let raw = std::fs::read_to_string(manifest_dir().join(rel))
            .unwrap_or_else(|e| panic!("{rel} is readable: {e}"));

        for (n, line) in raw.lines().enumerate() {
            assert!(
                !line.contains("aimodelvault_"),
                "{rel}:{} advertises the 4.x `aimodelvault_*` prefix:\n  {}",
                n + 1,
                line.trim()
            );
            // `AIM_` only as a variable prefix — plain prose about the old
            // binary is fine.
            assert!(
                !line.contains("AIM_"),
                "{rel}:{} advertises the 4.x `AIM_*` prefix:\n  {}",
                n + 1,
                line.trim()
            );
        }
    }
}

/// The release workflow must build and test the binaries with `--features api`.
///
/// Through 5.1.0 it did neither. Every published binary was a default-feature
/// build, so `iv serve` answered "unrecognized subcommand" on all five targets
/// while the README offered the prebuilt binary a few lines above the REST API
/// documentation — and every endpoint in this file was unreachable to anyone
/// who had not compiled from source.
///
/// The test step matters just as much: without the feature, `cargo test` skips
/// every `#![cfg(feature = "api")]` suite, so the release gate silently ran
/// none of the API, auth, or drift tests it appeared to run.
#[test]
fn the_release_workflow_builds_the_api_feature() {
    let workflow = std::fs::read_to_string(manifest_dir().join(".github/workflows/release.yml"))
        .expect("release workflow is readable");

    let build = workflow
        .lines()
        .find(|l| l.contains("cargo build --release"))
        .expect("release workflow has a cargo build step");
    assert!(
        build.contains("--features api"),
        "release binaries would ship without `iv serve`:\n  {}",
        build.trim()
    );

    let test = workflow
        .lines()
        .find(|l| l.contains("cargo test --release"))
        .expect("release workflow has a cargo test step");
    assert!(
        test.contains("--features api"),
        "the release gate would skip every api-gated test suite:\n  {}",
        test.trim()
    );
}

/// The exemption list must not become a dumping ground for undocumented routes.
#[test]
fn the_exemption_list_stays_small_and_real() {
    let registered = registered_routes();
    for exempt in NOT_REST_RESOURCES {
        assert!(
            registered.contains(*exempt),
            "'{exempt}' is exempted from the spec but is not a registered route — \
             remove the stale exemption"
        );
    }
    assert!(
        NOT_REST_RESOURCES.len() <= 2,
        "exemptions grew to {}; each one is an endpoint agents cannot discover",
        NOT_REST_RESOURCES.len()
    );
}

/// The served spec and the checked-in spec must both declare the crate version.
///
/// `openapi.rs` hard-coded `"1.3.0"` from 1.x until 6.1, so `/api/v1/openapi.json`
/// told every client generated from it that it was talking to a 1.3.0 server
/// while the crate went to 6.0. The path-drift tests above never looked at
/// `info.version`, which is exactly how it survived five major releases.
#[test]
fn both_specs_declare_the_crate_version() {
    let crate_version = env!("CARGO_PKG_VERSION");

    let served = ironvault::api::openapi::openapi_spec();
    let served_version = served
        .get("info")
        .and_then(|i| i.get("version"))
        .and_then(|v| v.as_str())
        .expect("served spec has info.version");
    assert_eq!(
        served_version, crate_version,
        "the spec served at /api/v1/openapi.json declares {served_version} but the crate is          {crate_version}; `info.version` in src/api/openapi.rs must come from CARGO_PKG_VERSION"
    );

    let raw = std::fs::read_to_string(manifest_dir().join(".well-known/openapi.yaml"))
        .expect("openapi.yaml is readable");
    let doc: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&raw).expect("openapi.yaml must be valid YAML");
    let file_version = doc
        .get("info")
        .and_then(|i| i.get("version"))
        .and_then(|v| v.as_str())
        .expect("openapi.yaml has info.version");
    assert_eq!(
        file_version, crate_version,
        ".well-known/openapi.yaml declares {file_version} but the crate is {crate_version};          bump it with the release"
    );
}
