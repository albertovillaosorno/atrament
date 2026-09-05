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
//   - Regression evidence for process-startup browser-launch failure mapping.
// - Must-Not:
//   - Launch a browser, bind a listener, or expose a generated session secret.
// - Allows:
//   - Inputs: Typed browser-launch failures.
//   - Outputs: Assertions over fail-closed, secret-free recovery errors.
//   - Side effects: Process-local allocations only.
// - Split-When:
//   - Startup gains independently testable lifecycle states.
// - Merge-When:
//   - Runtime-bootstrap composition moves behind a library boundary.
// - Summary:
//   - Verifies failed automatic browser launch cannot advertise manual access.
// - Description:
//   - Pins the recovery error independently from platform browser processes.
// - Usage:
//   - Compile with the runtime-bootstrap composition source as a test module.
// - Defaults:
//   - Recovery requires fixing browser launch and restarting Atrament.
//
use atrament_browser_launch::LaunchError;

#[allow(dead_code)]
#[path = "../../../../src/backend/runtime-bootstrap/composition/main.rs"]
mod bootstrap;

fn assert_recovery_error_is_secret_free(launch_error: &LaunchError) {
    let error = bootstrap::browser_launch_failure(launch_error);
    let message = error.to_string();
    assert!(message.contains("restart Atrament"));
    assert!(message.contains("credential is intentionally not published"));
    assert!(!message.contains("http://"));
    assert!(!message.contains("https://"));
    assert!(!message.contains("#session="));
    assert!(!message.contains("Open "));
}

#[test]
fn browser_launch_failure_requires_restart_without_publishing_access() {
    assert_recovery_error_is_secret_free(
        &LaunchError::GraphicalSessionUnavailable,
    );
}

#[test]
fn browser_launch_timeout_requires_restart_without_publishing_access() {
    assert_recovery_error_is_secret_free(&LaunchError::TimedOut);
}
