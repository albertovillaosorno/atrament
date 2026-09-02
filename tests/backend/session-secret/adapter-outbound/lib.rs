// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE-MIT
//
// Boundary-Contract:
// - Owns:
//   - Regression evidence for disposable browser session secret generation.
// - Must-Not:
//   - Persist, print, or expose generated session credential values.
// - Allows:
//   - Inputs: Operating-system cryptographic randomness through the adapter.
//   - Outputs: Assertions over representation length and redacted diagnostics.
//   - Side effects: Reading the operating-system random source during tests.
// - Split-When:
//   - Credential comparison requires independent request-admission fixtures.
// - Merge-When:
//   - Session-secret generation no longer has an independent boundary.
// - Summary:
//   - Verifies the 256-bit browser credential representation and redaction.
// - Description:
//   - Exercises generation without writing the resulting credential anywhere.
// - Usage:
//   - Compile this root harness against the session-secret adapter.
// - Defaults:
//   - Requires 64 lowercase hexadecimal transport characters.
//
use session_secret::SessionSecret;

#[allow(dead_code)]
#[path = "../../../../src/backend/session-secret/adapter-outbound/lib.rs"]
mod session_secret;

#[test]
fn generated_secret_has_the_frozen_transport_shape() {
    let secret = SessionSecret::generate()
        .expect("operating-system randomness is available");
    let encoded = secret.encoded();
    assert_eq!(encoded.len(), 64);
    assert!(encoded.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert!(encoded.bytes().all(|byte| !byte.is_ascii_uppercase()));
}

#[test]
fn debug_output_redacts_the_session_secret() {
    let secret = SessionSecret::generate()
        .expect("operating-system randomness is available");
    let debug = format!("{secret:?}");
    assert_eq!(debug, "SessionSecret([REDACTED])");
    assert!(!debug.contains(secret.encoded()));
}
