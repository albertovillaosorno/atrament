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
//   - Run this Cargo harness against the public secret adapter.
// - Defaults:
//   - Requires 64 lowercase hexadecimal transport characters.
//

//! Public session-secret regression tests.

use getrandom as _;
use session_secret::SessionSecret;

use atrament_session_secret as session_secret;

#[test]
fn generated_secret_has_the_frozen_transport_shape() {
    let generated = SessionSecret::generate();
    assert!(generated.is_ok(), "OS randomness must be available");
    let Ok(secret) = generated else {
        return;
    };
    let encoded = secret.encoded();
    assert_eq!(encoded.len(), 64);
    assert!(encoded.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert!(encoded.bytes().all(|byte| !byte.is_ascii_uppercase()));
}

#[test]
fn independent_generations_do_not_reuse_the_same_credential() {
    let generated_first = SessionSecret::generate();
    assert!(generated_first.is_ok(), "first OS random secret");
    let Ok(first) = generated_first else {
        return;
    };
    let generated_second = SessionSecret::generate();
    assert!(generated_second.is_ok(), "second OS random secret");
    let Ok(second) = generated_second else {
        return;
    };
    assert_ne!(first.encoded(), second.encoded());
}

#[test]
fn debug_output_redacts_the_session_secret() {
    let generated = SessionSecret::generate();
    assert!(generated.is_ok(), "OS randomness must be available");
    let Ok(secret) = generated else {
        return;
    };
    let debug = format!("{secret:?}");
    assert_eq!(debug, "SessionSecret([REDACTED])");
    assert!(!debug.contains(secret.encoded()));
}
