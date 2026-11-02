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
//   - Operating-system browser launch for the backend-selected localhost URL.
// - Must-Not:
//   - Rewrite the launch URL, persist state, or weaken runtime admission.
// - Allows:
//   - Inputs: One backend-selected localhost URL with a session fragment.
//   - Outputs: Typed launch success or recovery-safe launch failure.
//   - Side effects: Starting the platform browser opener process.
// - Split-When:
//   - A supported platform needs materially different launch lifecycle control.
// - Merge-When:
//   - Browser launch no longer requires operating-system process authority.
// - Summary:
//   - Opens the local Atrament workspace through the operating system.
// - Description:
//   - Keeps browser process spawning outside the inbound HTTP adapter.
// - Usage:
//   - Call launch after the loopback listener owns its canonical origin.
// - Defaults:
//   - Uses the platform opener and refuses Linux launch without a GUI session.
//   - Bounds opener completion to five seconds before failing startup closed.
//   - On Linux, kills the opener group on unsuccessful exit or failure.
//   - Currently exposes the fragment in opener argv (known P1 defect).
//

//! Operating-system browser launch for the disposable Atrament runtime.

#[cfg(target_os = "linux")]
use nix::sys::signal::{killpg, Signal};
#[cfg(target_os = "linux")]
use nix::unistd::Pid;
#[cfg(target_os = "linux")]
use std::env;
use std::error::Error;
use std::ffi::OsStr;
#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt as _;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};
use std::{fmt, io, thread};

const DEFAULT_LAUNCH_TIMEOUT: Duration = Duration::from_secs(5);
const LAUNCH_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// A browser launch failure safe to present without session-private state.
pub enum LaunchError {
    /// The current Linux process has no graphical-session environment marker.
    GraphicalSessionUnavailable,
    /// The started opener process could not be observed or terminated safely.
    Lifecycle(io::Error),
    /// The platform opener could not be started.
    Spawn(io::Error),
    /// The platform opener did not finish within the bounded launch window.
    TimedOut,
    /// The platform opener returned a non-success status.
    Unsuccessful(ExitStatus),
    /// This build target has no admitted browser opener.
    UnsupportedPlatform,
}

// The raw I/O errors are retained for trusted typed inspection, but debug
// formatting must be as private as the user-facing startup error message.
impl fmt::Debug for LaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GraphicalSessionUnavailable => {
                f.write_str("LaunchError::GraphicalSessionUnavailable")
            },
            Self::Spawn(_) => f.write_str("LaunchError::Spawn"),
            Self::Lifecycle(_) => f.write_str("LaunchError::Lifecycle"),
            Self::TimedOut => f.write_str("LaunchError::TimedOut"),
            Self::Unsuccessful(_) => f.write_str("LaunchError::Unsuccessful"),
            Self::UnsupportedPlatform => {
                f.write_str("LaunchError::UnsupportedPlatform")
            },
        }
    }
}

impl fmt::Display for LaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GraphicalSessionUnavailable => f.write_str(concat!(
                "no graphical session is available for automatic ",
                "browser launch",
            )),
            Self::Spawn(_) => f.write_str("browser opener could not start"),
            Self::Lifecycle(_) => f.write_str("browser opener process failed"),
            Self::TimedOut => {
                f.write_str("browser opener exceeded launch deadline")
            },
            Self::Unsuccessful(status) => {
                write!(f, "browser opener returned {status}")
            },
            Self::UnsupportedPlatform => {
                f.write_str("this platform has no configured browser opener")
            },
        }
    }
}

#[allow(
    clippy::allow_attributes,
    clippy::missing_trait_methods,
    reason = "Error has stable defaults plus a nightly-only provide hook",
)]
impl Error for LaunchError {
    fn cause(&self) -> Option<&dyn Error> {
        // Keep syscall details in the typed variant for internal callers,
        // never in an externally traversable diagnostic error chain.
        None
    }

    fn description(&self) -> &str {
        match self {
            Self::GraphicalSessionUnavailable => {
                "graphical session unavailable"
            },
            Self::Spawn(_) => "browser opener could not start",
            Self::Lifecycle(_) => "browser opener process failed",
            Self::TimedOut => "browser opener timed out",
            Self::Unsuccessful(_) => "browser opener failed",
            Self::UnsupportedPlatform => "unsupported browser platform",
        }
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

/// Open the backend-selected localhost URL with the platform browser opener.
///
/// Production supplies a session credential in the URL fragment. The current
/// argv handoff exposes that credential to unrelated same-user processes; null
/// standard streams and bounded launch time do not fix this known P1 defect.
///
/// # Errors
///
/// Returns a typed error when the platform is unsupported, no graphical Linux
/// session is available, or the selected opener cannot complete successfully.
pub fn launch(url: &str) -> Result<(), LaunchError> {
    launch_with_timeout(platform_opener()?, url, DEFAULT_LAUNCH_TIMEOUT)
}

#[cfg(target_os = "linux")]
pub(crate) fn graphical_session_marker_is_usable(
    value: Option<&OsStr>,
) -> bool {
    value.is_some_and(|marker| !marker.is_empty())
}

#[cfg(target_os = "linux")]
fn has_linux_graphical_session() -> bool {
    let display = env::var_os("DISPLAY");
    let wayland_display = env::var_os("WAYLAND_DISPLAY");
    graphical_session_marker_is_usable(display.as_deref())
        || graphical_session_marker_is_usable(wayland_display.as_deref())
}

#[cfg(target_os = "linux")]
fn platform_opener() -> Result<&'static OsStr, LaunchError> {
    if !has_linux_graphical_session() {
        return Err(LaunchError::GraphicalSessionUnavailable);
    }
    Ok(OsStr::new("xdg-open"))
}

#[cfg(target_os = "macos")]
fn platform_opener() -> Result<&'static OsStr, LaunchError> {
    Ok(OsStr::new("open"))
}

#[cfg(target_os = "windows")]
fn platform_opener() -> Result<&'static OsStr, LaunchError> {
    Ok(OsStr::new("explorer"))
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "windows",
)))]
fn platform_opener() -> Result<&'static OsStr, LaunchError> {
    Err(LaunchError::UnsupportedPlatform)
}

/// Open the supplied launch URL with an explicit opener program.
///
/// This surface exists so platform-launch behavior can be regression-tested
/// without starting a real browser.
///
/// # Errors
///
/// Returns a typed error when the opener cannot start, times out, or exits
/// unsuccessfully.
pub fn launch_with(program: &OsStr, url: &str) -> Result<(), LaunchError> {
    launch_with_timeout(program, url, DEFAULT_LAUNCH_TIMEOUT)
}

// An exit observed after the deadline cannot authorize a live session, even
// if the opener happened to return success just before it was reaped.
pub(crate) fn launch_status(
    status: ExitStatus,
    elapsed: Duration,
    timeout: Duration,
) -> Result<(), LaunchError> {
    if elapsed >= timeout {
        Err(LaunchError::TimedOut)
    } else if status.success() {
        Ok(())
    } else {
        Err(LaunchError::Unsuccessful(status))
    }
}

// Keep launcher helpers in a separate group so a failed opener cannot
// strand its own descendants with authority it inherited at launch. A helper
// that deliberately starts a new session is outside this process-group bound.
#[cfg(target_os = "linux")]
fn terminate_opener_group(child: &Child) {
    if let Ok(pid) = i32::try_from(child.id()) {
        let _signal = killpg(Pid::from_raw(pid), Signal::SIGKILL);
    }
}

fn terminate_after_launch_error(child: &mut Child) {
    #[cfg(target_os = "linux")]
    terminate_opener_group(child);
    if child.kill().is_ok() {
        drop(child.wait());
    } else {
        drop(child.try_wait());
    }
}

pub(crate) fn launch_with_timeout(
    program: &OsStr,
    url: &str,
    timeout: Duration,
) -> Result<(), LaunchError> {
    // Admission precedes spawning: an exhausted budget cannot own a child.
    if timeout.is_zero() {
        return Err(LaunchError::TimedOut);
    }
    // The credential-bearing production URL is currently visible in argv.
    // Do not interpret null stdio as confidential process handoff.
    let mut command = Command::new(program);
    let _configured = command
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(target_os = "linux")]
    let _grouped = command.process_group(0);
    let mut child = command.spawn().map_err(LaunchError::Spawn)?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let elapsed = started.elapsed();
                // An opener can fail or complete after its deadline while a
                // same-group helper still holds the inherited URL. Neither
                // state authorizes a ready session or a surviving helper.
                #[cfg(target_os = "linux")]
                if elapsed >= timeout || !status.success() {
                    terminate_opener_group(&child);
                }
                return launch_status(status, elapsed, timeout);
            },
            Ok(None) => {},
            Err(error) => {
                terminate_after_launch_error(&mut child);
                return Err(LaunchError::Lifecycle(error));
            },
        }
        let elapsed = started.elapsed();
        if elapsed >= timeout {
            #[cfg(target_os = "linux")]
            terminate_opener_group(&child);
            match child.kill() {
                Ok(()) => {
                    let _status =
                        child.wait().map_err(LaunchError::Lifecycle)?;
                    return Err(LaunchError::TimedOut);
                },
                Err(kill_error) => match child.try_wait() {
                    // The deadline already expired; a successful exit in
                    // the kill race must not convert timeout to success.
                    Ok(Some(_status)) => return Err(LaunchError::TimedOut),
                    Ok(None) | Err(_) => {
                        return Err(LaunchError::Lifecycle(kill_error));
                    },
                },
            }
        }
        let remaining = timeout.saturating_sub(elapsed);
        thread::sleep(LAUNCH_POLL_INTERVAL.min(remaining));
    }
}
