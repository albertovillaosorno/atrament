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
    /// The process working directory moved during relative source opening.
    WorkingDirectoryChanged,
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
/// not a filesystem snapshot or content validation. For relative paths it also
/// refuses a working-directory change observed while opening the descriptor;
/// a transient change away and back remains an inherent process-wide race.
/// Nameless, NUL-containing and directory-marked paths refuse before opening
/// an anchor. Use the bounded reader followed by ZIP, schema and member SHA
/// verification.
///
/// Other platforms and Linux kernels without `openat2` refuse rather than
/// using a weaker lstat/open race-prone fallback.
///
/// # Errors
///
/// Returns typed path, platform, link, type and I/O refusals.
pub fn open_selected_profile_file(
    path: &Path,
) -> Result<File, ProfileSelectedFileOpenError> {
    validate_selected_path(path)?;
    open_selected_without_links(path)
}

/// Open a relative selection using a previously captured process CWD.
///
/// This is a file-transport primitive, not caller authentication. A trusted
/// host may retain the opened directory at selection time and pass it here so
/// later changes to process CWD refuse instead of selecting a different file.
/// The supplied descriptor must be a directory opened from the actual CWD;
/// both before and after the selected open, Linux compares its device and
/// inode with the process CWD. This is a best-effort observation, not an
/// atomic guard against later `chdir` or file replacement.
///
/// # Errors
///
/// Refuses absolute or invalid names, stale CWD anchors, links, nonregular
/// sources and unsupported secure-open policies.
pub fn open_selected_profile_file_from_cwd(
    path: &Path,
    selected_cwd: &File,
) -> Result<File, ProfileSelectedFileOpenError> {
    validate_selected_path(path)?;
    if path.is_absolute() {
        return Err(ProfileSelectedFileOpenError::InvalidSelectedPath);
    }
    open_relative_from_cwd(path, selected_cwd)
}

fn validate_selected_path(
    path: &Path,
) -> Result<(), ProfileSelectedFileOpenError> {
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
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn open_relative_from_cwd(
    _path: &Path,
    _selected_cwd: &File,
) -> Result<File, ProfileSelectedFileOpenError> {
    Err(ProfileSelectedFileOpenError::UnsupportedOpenPolicy)
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
fn open_selected_without_links(
    path: &Path,
) -> Result<File, ProfileSelectedFileOpenError> {
    let (anchor, relative) = if path.is_absolute() {
        (
            "/",
            path.strip_prefix("/").map_err(|_error| {
                ProfileSelectedFileOpenError::ParentTraversal
            })?,
        )
    } else {
        (".", path)
    };
    let directory = File::open(anchor)
        .map_err(|error| ProfileSelectedFileOpenError::FileIo(error.kind()))?;
    open_from_anchor(relative, &directory, !path.is_absolute())
}

#[cfg(target_os = "linux")]
fn open_relative_from_cwd(
    path: &Path,
    selected_cwd: &File,
) -> Result<File, ProfileSelectedFileOpenError> {
    open_from_anchor(path, selected_cwd, true)
}

#[cfg(target_os = "linux")]
fn check_cwd_anchor(
    selected_cwd: &File,
) -> Result<(), ProfileSelectedFileOpenError> {
    use std::os::unix::fs::MetadataExt as _;

    let current = File::open(".")
        .map_err(|error| ProfileSelectedFileOpenError::FileIo(error.kind()))?;
    let expected = selected_cwd
        .metadata()
        .map_err(|error| ProfileSelectedFileOpenError::FileIo(error.kind()))?;
    let observed = current
        .metadata()
        .map_err(|error| ProfileSelectedFileOpenError::FileIo(error.kind()))?;
    if expected.dev() != observed.dev() || expected.ino() != observed.ino() {
        return Err(ProfileSelectedFileOpenError::WorkingDirectoryChanged);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn open_from_anchor(
    relative: &Path,
    directory: &File,
    check_relative_cwd: bool,
) -> Result<File, ProfileSelectedFileOpenError> {
    use nix::fcntl::{OFlag, OpenHow, ResolveFlag, openat2};

    if check_relative_cwd {
        check_cwd_anchor(directory)?;
    }
    let how = OpenHow::new()
        .flags(
            OFlag::O_RDONLY
                | OFlag::O_CLOEXEC
                | OFlag::O_NOFOLLOW
                | OFlag::O_NONBLOCK,
        )
        .resolve(
            ResolveFlag::RESOLVE_BENEATH | ResolveFlag::RESOLVE_NO_SYMLINKS,
        );
    let descriptor = openat2(directory, relative, how)
        .map_err(classify_linux_open_error)?;
    let file = File::from(descriptor);
    let metadata = file
        .metadata()
        .map_err(|error| ProfileSelectedFileOpenError::FileIo(error.kind()))?;
    if !metadata.is_file() {
        return Err(ProfileSelectedFileOpenError::NotRegularFile);
    }
    if check_relative_cwd {
        // An anchored directory may no longer be the selected relative CWD.
        check_cwd_anchor(directory)?;
    }
    Ok(file)
}

#[cfg(not(target_os = "linux"))]
fn open_selected_without_links(
    _path: &Path,
) -> Result<File, ProfileSelectedFileOpenError> {
    Err(ProfileSelectedFileOpenError::UnsupportedOpenPolicy)
}
