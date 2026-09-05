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
//   - Operating-system browser launch for the canonical localhost origin.
// - Must-Not:
//   - Rewrite the origin, persist launch state, or weaken runtime admission.
// - Allows:
//   - Inputs: One backend-selected canonical localhost HTTP origin.
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
//

//! Operating-system browser launch for the disposable Atrament runtime.

#[cfg(target_os = "linux")]
use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};
use std::{fmt, io, thread};

const DEFAULT_LAUNCH_TIMEOUT: Duration = Duration::from_secs(5);
const LAUNCH_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// A browser launch failure safe to present without session-private state.
#[derive(Debug)]
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

impl fmt::Display for LaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GraphicalSessionUnavailable => f.write_str(concat!(
                "no graphical session is available for automatic ",
                "browser launch",
            )),
            Self::Spawn(error) => {
                write!(f, "browser opener could not start: {error}")
            },
            Self::Lifecycle(error) => {
                write!(f, "browser opener process failed: {error}")
            },
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
        match self {
            Self::Lifecycle(error) | Self::Spawn(error) => Some(error),
            Self::GraphicalSessionUnavailable
            | Self::TimedOut
            | Self::Unsuccessful(_)
            | Self::UnsupportedPlatform => None,
        }
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
        match self {
            Self::Lifecycle(error) | Self::Spawn(error) => Some(error),
            Self::GraphicalSessionUnavailable
            | Self::TimedOut
            | Self::Unsuccessful(_)
            | Self::UnsupportedPlatform => None,
        }
    }
}

/// Open the canonical Atrament origin with the current platform browser opener.
///
/// # Errors
///
/// Returns a typed error when the platform is unsupported, no graphical Linux
/// session is available, or the selected opener cannot complete successfully.
pub fn launch(origin: &str) -> Result<(), LaunchError> {
    launch_with_timeout(platform_opener()?, origin, DEFAULT_LAUNCH_TIMEOUT)
}

#[cfg(target_os = "linux")]
fn has_linux_graphical_session() -> bool {
    env::var_os("DISPLAY").is_some() || env::var_os("WAYLAND_DISPLAY").is_some()
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

/// Open `origin` with an explicit opener program.
///
/// This surface exists so platform-launch behavior can be regression-tested
/// without starting a real browser.
///
/// # Errors
///
/// Returns a typed error when the opener cannot start, times out, or exits
/// unsuccessfully.
pub fn launch_with(program: &OsStr, origin: &str) -> Result<(), LaunchError> {
    launch_with_timeout(program, origin, DEFAULT_LAUNCH_TIMEOUT)
}

fn launch_status(status: ExitStatus) -> Result<(), LaunchError> {
    if status.success() {
        Ok(())
    } else {
        Err(LaunchError::Unsuccessful(status))
    }
}

fn terminate_after_launch_error(child: &mut Child) {
    if child.kill().is_ok() {
        drop(child.wait());
    } else {
        drop(child.try_wait());
    }
}

pub(crate) fn launch_with_timeout(
    program: &OsStr,
    origin: &str,
    timeout: Duration,
) -> Result<(), LaunchError> {
    let mut child = Command::new(program)
        .arg(origin)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(LaunchError::Spawn)?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return launch_status(status),
            Ok(None) => {},
            Err(error) => {
                terminate_after_launch_error(&mut child);
                return Err(LaunchError::Lifecycle(error));
            },
        }
        let elapsed = started.elapsed();
        if elapsed >= timeout {
            match child.kill() {
                Ok(()) => {
                    let _status =
                        child.wait().map_err(LaunchError::Lifecycle)?;
                    return Err(LaunchError::TimedOut);
                },
                Err(kill_error) => match child.try_wait() {
                    Ok(Some(status)) => return launch_status(status),
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
