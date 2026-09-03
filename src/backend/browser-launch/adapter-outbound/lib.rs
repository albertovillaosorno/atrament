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
//

//! Operating-system browser launch for the disposable Atrament runtime.

#[cfg(target_os = "linux")]
use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::process::{Command, ExitStatus};
use std::{fmt, io};

/// A browser launch failure safe to present without session-private state.
#[derive(Debug)]
pub enum LaunchError {
    /// The current Linux process has no graphical-session environment marker.
    GraphicalSessionUnavailable,
    /// The platform opener could not be started.
    Spawn(io::Error),
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
            Self::Spawn(error) => Some(error),
            Self::GraphicalSessionUnavailable
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
            Self::Unsuccessful(_) => "browser opener failed",
            Self::UnsupportedPlatform => "unsupported browser platform",
        }
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Spawn(error) => Some(error),
            Self::GraphicalSessionUnavailable
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
    launch_with(platform_opener()?, origin)
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
/// Returns a typed error when the opener cannot start or exits unsuccessfully.
pub fn launch_with(program: &OsStr, origin: &str) -> Result<(), LaunchError> {
    let status = Command::new(program)
        .arg(origin)
        .status()
        .map_err(LaunchError::Spawn)?;
    if status.success() {
        Ok(())
    } else {
        Err(LaunchError::Unsuccessful(status))
    }
}
