//! The OpenAPI specification served at `/api/v1/openapi.json`.
//!
//! This is `.well-known/openapi.yaml`, embedded at compile time and converted
//! to JSON — not a second copy of it.
//!
//! It used to be a hand-written `serde_json::json!` literal, and the two drifted
//! exactly as far as you would expect: the served spec described 18 paths while
//! the checked-in one described 56, and its `info.version` still said `1.3.0`
//! after the crate reached 6.x. Anyone generating a client from the live
//! discovery endpoint — which is the endpoint this project points agents at —
//! got a third of the API and a five-major-version-old label.
//!
//! `tests/openapi_drift_test.rs` pins the YAML to the router in both directions,
//! so serving the YAML makes the router, the file, and the endpoint agree by
//! construction rather than by anyone remembering to update a literal.

use std::sync::OnceLock;

/// The canonical spec, embedded so the binary cannot be separated from it.
const SPEC_YAML: &str = include_str!("../../.well-known/openapi.yaml");

/// The OpenAPI spec for the vault API, as JSON.
///
/// Parsed once and cached: the document is a few hundred lines and every call
/// serves the same bytes.
pub fn openapi_spec() -> serde_json::Value {
    static SPEC: OnceLock<serde_json::Value> = OnceLock::new();
    SPEC.get_or_init(|| {
        // `include_str!` means a malformed spec fails the build, not a request,
        // and `openapi_drift_test` parses the same file. Reaching this panic
        // requires shipping a binary whose embedded spec never parsed in CI.
        serde_yaml_ng::from_str(SPEC_YAML)
            .expect("embedded .well-known/openapi.yaml must be valid YAML")
    })
    .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_served_spec_is_the_canonical_spec() {
        let spec = openapi_spec();
        let paths = spec
            .get("paths")
            .and_then(|p| p.as_object())
            .expect("spec has paths");

        // The hand-written literal this replaced had 18. The canonical file has
        // every documented endpoint, and the drift test ties that to the router.
        assert!(
            paths.len() >= 50,
            "served spec declares only {} paths; it should be the whole canonical spec",
            paths.len()
        );

        assert_eq!(
            spec.get("info")
                .and_then(|i| i.get("version"))
                .and_then(|v| v.as_str()),
            Some(env!("CARGO_PKG_VERSION")),
            "served spec must declare the crate version"
        );
    }

    #[test]
    fn the_spec_parses_and_is_cached() {
        // Second call comes from the OnceLock; both must be identical.
        assert_eq!(openapi_spec(), openapi_spec());
    }
}
