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
//   - Open one exactly selected profile path without resolving symlinks.
// - Must-Not:
//   - Grant path-selection authority, decode profiles, or accept session state.
// - Allows:
//   - Inputs: Path passed by an independently trusted Import caller.
//   - Outputs: Owned read-only descriptor or typed refusal.
//   - Side effects: Open and close the selected source; no writes.
// - Split-When:
//   - Other platforms require independently verified secure-open policies.
// - Merge-When:
//   - A complete host Import adapter owns selection and open/read lifecycle.
// - Summary:
//   - Linux openat2 binds path resolution to the returned open handle.
// - Description:
//   - No fallback to racy lstat/readlink/File::open checks.
// - Usage:
//   - Caller must authenticate ImportProfile selection before invoking opener.
// - Defaults:
//   - Reject unsupported platforms or kernels; reject path parent traversal.
//

//! Fail-closed profile source opening without selection authority.

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn unsupported_secure_open_errors_never_trigger_a_weak_fallback() {
        for error in [Errno::ENOSYS, Errno::EINVAL, Errno::E2BIG] {
            assert_eq!(
                classify_linux_open_error(error),
                ProfileSelectedFileOpenError::UnsupportedOpenPolicy,
            );
        }
        assert_eq!(
            classify_linux_open_error(Errno::ELOOP),
            ProfileSelectedFileOpenError::SymlinkForbidden,
        );
        assert_eq!(
            classify_linux_open_error(Errno::EXDEV),
            ProfileSelectedFileOpenError::ParentTraversal,
        );
        assert_eq!(
            classify_linux_open_error(Errno::ENOENT),
            ProfileSelectedFileOpenError::FileIo(io::ErrorKind::NotFound),
        );
    }
}

#[cfg(target_os = "linux")]
use nix::errno::Errno;
use std::fs::File;
use std::io;
use std::path::{Component, Path};

/// Refusal before the selected source can become a decoder input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileSelectedFileOpenError {
    /// The selected file could not be opened or inspected.
    FileIo(io::ErrorKind),
    /// A nameless, directory-marked or NUL-containing path was supplied.
    InvalidSelectedPath,
    /// The selected target was not an ordinary file.
    NotRegularFile,
    /// No parent-directory escape is allowed in a selected path.
    ParentTraversal,
    /// At least one path component resolves through a symbolic link.
    SymlinkForbidden,
    /// Secure path resolution is unavailable; never fall back to `File::open`.
    UnsupportedOpenPolicy,
}

/// Open an explicitly selected regular profile file without following links.
///
/// The trusted host must independently admit `ImportProfile` path intent
/// from a real selection or host policy **before** calling this low-level
/// opener. A raw path alone is not that evidence. On Linux, `openat2` resolves
/// the exact relative path beneath an opened cwd or filesystem-root anchor,
/// refusing symlinks at **every** component (including procfs magic links),
/// parent traversal, and files that are not regular. A rename or concurrent
/// file rewrite after opening may change the directory entry or file bytes:
/// this method binds the pathname lookup to its returned descriptor, but is
/// not a filesystem snapshot or content validation. Nameless, NUL-containing
/// and directory-marked paths refuse before opening an anchor. Use the bounded reader
/// followed by full ZIP, schema and member SHA verification.
///
/// Other platforms and Linux kernels without `openat2` refuse rather than
/// using a weaker lstat/open race-prone fallback.
///
/// # Errors
///
/// Returns typed path, platform, link, type and I/O refusals.
pub fn open_selected_profile_file(path: &Path) -> Result<File, ProfileSelectedFileOpenError> {
    if path
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(ProfileSelectedFileOpenError::ParentTraversal);
    }
    let selected = path.as_os_str().as_encoded_bytes();
    if path.file_name().is_none()
        || selected.contains(&0)
        || selected.ends_with(b"/")
        || selected.ends_with(b"/.")
    {
        return Err(ProfileSelectedFileOpenError::InvalidSelectedPath);
    }
    open_selected_without_links(path)
}

#[cfg(target_os = "linux")]
fn classify_linux_open_error(error: Errno) -> ProfileSelectedFileOpenError {
    if error == Errno::ELOOP {
        ProfileSelectedFileOpenError::SymlinkForbidden
    } else if error == Errno::EXDEV {
        ProfileSelectedFileOpenError::ParentTraversal
    } else if matches!(error, Errno::ENOSYS | Errno::EINVAL | Errno::E2BIG) {
        // Unsupported kernels or resolve flags must not trigger a weak open.
        ProfileSelectedFileOpenError::UnsupportedOpenPolicy
    } else {
        ProfileSelectedFileOpenError::FileIo(io::Error::from(error).kind())
    }
}

#[cfg(target_os = "linux")]
fn open_selected_without_links(path: &Path) -> Result<File, ProfileSelectedFileOpenError> {
    use nix::fcntl::{OFlag, OpenHow, ResolveFlag, openat2};

    let (anchor, relative) = if path.is_absolute() {
        (
            "/",
            path.strip_prefix("/")
                .map_err(|_error| ProfileSelectedFileOpenError::ParentTraversal)?,
        )
    } else {
        (".", path)
    };
    let directory =
        File::open(anchor).map_err(|error| ProfileSelectedFileOpenError::FileIo(error.kind()))?;
    let how = OpenHow::new()
        .flags(OFlag::O_RDONLY | OFlag::O_CLOEXEC | OFlag::O_NOFOLLOW | OFlag::O_NONBLOCK)
        .resolve(ResolveFlag::RESOLVE_BENEATH | ResolveFlag::RESOLVE_NO_SYMLINKS);
    let descriptor = openat2(&directory, relative, how).map_err(classify_linux_open_error)?;
    let file = File::from(descriptor);
    let metadata = file
        .metadata()
        .map_err(|error| ProfileSelectedFileOpenError::FileIo(error.kind()))?;
    if !metadata.is_file() {
        return Err(ProfileSelectedFileOpenError::NotRegularFile);
    }
    Ok(file)
}

#[cfg(not(target_os = "linux"))]
fn open_selected_without_links(_path: &Path) -> Result<File, ProfileSelectedFileOpenError> {
    Err(ProfileSelectedFileOpenError::UnsupportedOpenPolicy)
}
