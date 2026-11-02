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
//   - Inputs: Deterministic opener names and synthetic localhost launch URLs.
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
//   - Uses standard Unix commands whose inherited output would expose the
//     synthetic launch credential.
//

//! Fail-closed browser opener lifecycle regression tests.

use std::ffi::OsStr;
#[cfg(unix)]
use std::time::{Duration, Instant};

#[allow(
    dead_code,
    unreachable_pub,
    reason = "test-only module includes the public opener implementation",
)]
#[path = "../../../../src/backend/browser-launch/adapter-outbound/lib.rs"]
mod browser_launch;
use atrament_browser_launch as _;

const ORIGIN: &str = "http://127.0.0.1:43123";
const PRIVATE_LAUNCH_URL: &str = concat!(
    "http://127.0.0.1:43123/#session=",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
);

// Launch success is admitted only while the budget is still live. A late
// successful exit cannot turn an expired launch into a ready session.
#[cfg(unix)]
#[test]
fn observed_launch_status_respects_the_deadline_even_on_success() {
    use std::os::unix::process::ExitStatusExt as _;
    use std::process::ExitStatus;

    let budget = Duration::from_millis(50);
    let success = ExitStatus::from_raw(0);
    let failure = ExitStatus::from_raw(7 << 8);
    assert!(browser_launch::launch_status(
        success, Duration::from_millis(49), budget,
    ).is_ok());
    assert!(matches!(
        browser_launch::launch_status(
            success, Duration::from_millis(50), budget,
        ),
        Err(browser_launch::LaunchError::TimedOut),
    ));
    assert!(matches!(
        browser_launch::launch_status(
            success, Duration::from_millis(51), budget,
        ),
        Err(browser_launch::LaunchError::TimedOut),
    ));
    assert!(matches!(
        browser_launch::launch_status(
            failure, Duration::from_millis(49), budget,
        ),
        Err(browser_launch::LaunchError::Unsuccessful(_)),
    ));
    assert!(matches!(
        browser_launch::launch_status(
            failure, Duration::from_millis(51), budget,
        ),
        Err(browser_launch::LaunchError::TimedOut),
    ));
}

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

#[cfg(unix)]
#[test]
fn zero_launch_budget_rejects_before_spawning_any_opener() {
    let error = browser_launch::launch_with_timeout(
        OsStr::new("atrament-browser-opener-does-not-exist"),
        PRIVATE_LAUNCH_URL,
        Duration::ZERO,
    )
    .expect_err("no process may spawn without a positive launch budget");
    assert!(matches!(error, browser_launch::LaunchError::TimedOut));
    assert!(!error.to_string().contains(PRIVATE_LAUNCH_URL));
}

#[cfg(unix)]
#[test]
fn credential_echoing_stdout_opener_is_silenced_and_killed_at_deadline() {
    let started = Instant::now();
    let error = browser_launch::launch_with_timeout(
        OsStr::new("yes"),
        PRIVATE_LAUNCH_URL,
        Duration::from_millis(80),
    )
    .expect_err("yes must exceed the bounded launch deadline");
    let elapsed = started.elapsed();
    assert!(matches!(error, browser_launch::LaunchError::TimedOut));
    assert!(elapsed >= Duration::from_millis(60));
    assert!(elapsed < Duration::from_secs(1));
    assert!(!error.to_string().contains(PRIVATE_LAUNCH_URL));
}

#[cfg(unix)]
#[test]
fn credential_echoing_stderr_opener_is_silenced_and_typed() {
    let error =
        browser_launch::launch_with(OsStr::new("cat"), PRIVATE_LAUNCH_URL)
            .expect_err("cat must reject the synthetic URL as a file path");
    assert!(matches!(
        error,
        browser_launch::LaunchError::Unsuccessful(_)
    ));
    assert!(!error.to_string().contains(PRIVATE_LAUNCH_URL));
}

// A failed opener must not leave a child capable of using its inherited
// credential. This includes both timeout and a nonzero opener exit.
#[cfg(target_os = "linux")]
#[expect(
    clippy::expect_used,
    clippy::missing_trait_methods,
    clippy::create_dir,
    reason = "test-only helper directory and fail-fast cleanup",
)]
fn assert_linux_failed_opener_reaps_descendant(
    trailer: &str,
    timeout: Duration,
) -> browser_launch::LaunchError {
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::PathBuf;
    use std::process;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);
    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("remove only test-owned helper");
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../.temp");
    fs::create_dir_all(&root).expect("project-local scratch exists");
    let scratch = Scratch(root.join(format!(
        "opener-descendant-{}-{}",
        process::id(),
        FIXTURE_ID.fetch_add(1, Ordering::Relaxed),
    )));
    fs::create_dir(&scratch.0).expect("unique helper fixture");
    let marker = scratch.0.join("descendant-continued");
    let started = scratch.0.join("descendant-started");
    let opener = scratch.0.join("opener.sh");
    fs::write(
        &opener,
        format!(
            concat!(
                "#!/bin/sh\n",
                "(sleep 0.4; printf continued > '{}') &\n",
                "printf started > '{}'\n",
                "{}\n",
            ),
            marker.display(),
            started.display(),
            trailer,
        ),
    )
    .expect("write test opener");
    fs::set_permissions(&opener, fs::Permissions::from_mode(0o700))
        .expect("make test opener executable");
    let error = browser_launch::launch_with_timeout(
        opener.as_os_str(),
        PRIVATE_LAUNCH_URL,
        timeout,
    )
    .expect_err("test opener must fail");
    assert!(started.exists(), "opener must have spawned its child");
    assert!(!error.to_string().contains(PRIVATE_LAUNCH_URL));
    thread::sleep(Duration::from_millis(700));
    assert!(
        !marker.exists(),
        "a failed opener must not leave its child executing",
    );
    error
}

#[cfg(target_os = "linux")]
#[test]
fn timed_out_linux_opener_does_not_leave_a_running_descendant() {
    let error = assert_linux_failed_opener_reaps_descendant(
        "wait",
        Duration::from_millis(80),
    );
    assert!(matches!(error, browser_launch::LaunchError::TimedOut));
}

#[cfg(target_os = "linux")]
#[test]
fn unsuccessful_linux_opener_does_not_leave_a_running_descendant() {
    let error = assert_linux_failed_opener_reaps_descendant(
        "exit 7",
        Duration::from_secs(1),
    );
    assert!(matches!(
        error,
        browser_launch::LaunchError::Unsuccessful(_)
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn linux_graphical_session_markers_must_be_nonempty() {
    assert!(!browser_launch::graphical_session_marker_is_usable(None));
    assert!(!browser_launch::graphical_session_marker_is_usable(Some(
        OsStr::new(""),
    )));
    for marker in [":0", "wayland-0"] {
        assert!(browser_launch::graphical_session_marker_is_usable(Some(
            OsStr::new(marker),
        )));
    }
}
