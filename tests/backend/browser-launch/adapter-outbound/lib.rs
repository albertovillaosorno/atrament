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
//   - Regression evidence for operating-system browser launch behavior.
// - Must-Not:
//   - Start a real browser or alter session admission.
// - Allows:
//   - Inputs: Deterministic command names and one canonical loopback origin.
//   - Outputs: Assertions over typed launch outcomes.
//   - Side effects: Short-lived no-op test process execution.
// - Split-When:
//   - Platform-specific launch fixtures require independent harnesses.
// - Merge-When:
//   - Browser launch no longer has an independent outbound boundary.
// - Summary:
//   - Verifies browser opener success and recovery-safe failures.
// - Description:
//   - Exercises explicit opener execution without launching a browser.
// - Usage:
//   - Compile this root test harness against the browser launch adapter.
// - Defaults:
//   - Uses standard no-op process commands on Unix test hosts.
//
use std::ffi::OsStr;

#[allow(dead_code)]
#[path = "../../../../src/backend/browser-launch/adapter-outbound/lib.rs"]
mod browser_launch;

const ORIGIN: &str = "http://127.0.0.1:43123";

#[cfg(unix)]
#[test]
fn explicit_successful_opener_is_accepted() {
    let result = browser_launch::launch_with(OsStr::new("true"), ORIGIN);
    assert!(result.is_ok());
}

#[cfg(unix)]
#[test]
fn explicit_unsuccessful_opener_is_typed() {
    let error = browser_launch::launch_with(OsStr::new("false"), ORIGIN)
        .expect_err("false command fails");
    assert!(matches!(
        error,
        browser_launch::LaunchError::Unsuccessful(_)
    ));
}

#[test]
fn missing_opener_is_typed_without_exposing_the_origin() {
    let error = browser_launch::launch_with(
        OsStr::new("atrament-browser-opener-does-not-exist"),
        ORIGIN,
    )
    .expect_err("missing opener fails");
    assert!(matches!(error, browser_launch::LaunchError::Spawn(_)));
    assert!(!error.to_string().contains(ORIGIN));
}
