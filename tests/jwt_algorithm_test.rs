//! Pins the JWT signing algorithm to HMAC.
//!
//! This is not a style preference — it is the load-bearing premise behind the
//! RUSTSEC-2023-0071 suppression in `.cargo/audit.toml` and `deny.toml`. The
//! `rsa` crate is linked into every `--features api` build (jsonwebtoken's
//! `rust_crypto` provider cannot be taken without it) and carries an
//! unfixed timing sidechannel in RSA decryption. That advisory is safe to
//! suppress only for as long as no RSA key is ever constructed here.
//!
//! If someone introduces RS256/PS256/RS512/etc., these tests fail, and the
//! suppression must be revisited rather than silently carried forward.

#![cfg(feature = "api")]

use base64::Engine;
use ironvault::api::auth::{create_token, verify_token};

/// Decode the `alg` field out of a token's JOSE header without trusting the
/// signature — we only care what algorithm was chosen.
fn header_alg(token: &str) -> String {
    let header_b64 = token
        .split('.')
        .next()
        .expect("a JWT always has at least one segment");
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(header_b64)
        .expect("JOSE header is base64url");
    let header: serde_json::Value = serde_json::from_slice(&raw).expect("JOSE header is JSON");
    header["alg"]
        .as_str()
        .expect("JOSE header carries an `alg`")
        .to_string()
}

#[test]
fn issued_tokens_are_hmac_signed() {
    let token = create_token("a-secret-that-is-long-enough", 60).expect("token issues");
    assert_eq!(
        header_alg(&token),
        "HS256",
        "JWTs must stay HMAC-signed; an RSA algorithm here would make the \
         RUSTSEC-2023-0071 suppression unsound (see .cargo/audit.toml)"
    );
}

#[test]
fn verification_rejects_a_swapped_algorithm() {
    // An attacker-supplied header claiming a different algorithm must not be
    // honoured — `Validation::default()` accepts HS256 only.
    let token = create_token("a-secret-that-is-long-enough", 60).expect("token issues");
    let mut parts = token.split('.');
    let (_, payload, sig) = (
        parts.next().unwrap(),
        parts.next().unwrap(),
        parts.next().unwrap(),
    );

    let forged_header =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#);
    let forged = format!("{forged_header}.{payload}.{sig}");

    assert!(
        verify_token(&forged, "a-secret-that-is-long-enough").is_err(),
        "a token whose header advertises RS256 must be rejected, not routed \
         into the RSA code path"
    );
}
