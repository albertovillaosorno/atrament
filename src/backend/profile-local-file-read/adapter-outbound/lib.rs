// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE
//
// Boundary-Contract:
// - Owns:
//   - Read-only local profile directory and selected-file transport.
// - Must-Not:
//   - Choose filenames, authorize profile files, edit JSON, or write files.
// - Allows:
//   - Inputs: Host-selected directory, validated selected path and byte limit.
//   - Outputs: Canonical directory path, bounded exact bytes or typed refusal.
//   - Side effects: Read-only filesystem observations and opened-file reads.
// - Split-When:
//   - Additional platforms require independent secure-file open policies.
// - Merge-When:
//   - A native local profile transport subsumes directory and read effects.
// - Summary:
//   - Keeps local file reads out of the authenticated runtime composition root.
// - Description:
//   - Reuses symlink-refusing source opening and bounded descriptor reads.
// - Usage:
//   - Call only after the host admits its profile directory and basename.
// - Defaults:
//   - No silent symlink traversal or insecure platform fallback.
//

//! Read-only local profile source adapter for caller-admitted localhost paths.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use atrament_profile_file_reader::{
    ProfileFileReadError, read_bounded_profile_file,
};
use atrament_profile_selected_file_open::{
    ProfileSelectedFileOpenError, open_selected_profile_file,
};

/// Refusal before any selected bytes are returned to the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalProfileFileReadError {
    /// Source I/O, metadata retrieval or memory reservation failed.
    FileIo,
    /// Source path, file type, size or observed contents were inadmissible.
    InvalidInput,
    /// Source does not exist.
    NotFound,
    /// Platform cannot guarantee link-safe selected-source opening.
    Unavailable,
}

/// Resolve only an existing, non-symlink directory selected by the local host.
///
/// Parent-directory symlinks are allowed because the host explicitly selects
/// this directory. The selected leaf itself must be a real directory. On
/// Unix, the resolved path and a final selected-leaf observation must still
/// match the initial directory inode, even if another process replaced the
/// leaf temporarily during canonicalization. The result is still a pathname,
/// not a directory lock or a guarantee against later filesystem mutation.
///
/// # Errors
/// Refuses missing, linked, non-directory or observably changed selections.
pub fn canonical_local_profile_directory(path: &Path) -> io::Result<PathBuf> {
    canonical_local_profile_directory_with(
        path,
        |selected| fs::canonicalize(selected),
    )
}

pub(crate) fn canonical_local_profile_directory_with<Resolve>(
    path: &Path,
    resolve: Resolve,
) -> io::Result<PathBuf>
where
    Resolve: FnOnce(&Path) -> io::Result<PathBuf>,
{
    let before = fs::symlink_metadata(path)?;
    if !before.file_type().is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "profile directory must be a real existing directory",
        ));
    }
    let canonical = resolve(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;

        // Canonicalization can traverse a replaced selected leaf. Checking
        // only its final spelling would silently accept the substitute.
        let resolved = fs::metadata(&canonical)?;
        let after = fs::symlink_metadata(path)?;
        if !resolved.is_dir()
            || !after.file_type().is_dir()
            || resolved.dev() != before.dev()
            || resolved.ino() != before.ino()
            || after.dev() != before.dev()
            || after.ino() != before.ino()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "selected profile directory changed while resolving",
            ));
        }
    }
    Ok(canonical)
}

/// Preserve the distinction between caller-invalid sources and host failures.
/// The localhost HTTP transport uses these categories for its error status.
#[must_use]
pub(crate) const fn classify_bounded_profile_read_error(
    error: ProfileFileReadError,
) -> LocalProfileFileReadError {
    match error {
        ProfileFileReadError::AllocationFailed
        | ProfileFileReadError::FileIo(_) => LocalProfileFileReadError::FileIo,
        ProfileFileReadError::FileLengthChanged
        | ProfileFileReadError::FileMetadataChanged
        | ProfileFileReadError::FileTooLarge
        | ProfileFileReadError::NotRegularFile => {
            LocalProfileFileReadError::InvalidInput
        }
    }
}

/// Open a host-validated profile path and read its exact bounded bytes.
///
/// Selected-file opening refuses symlink traversal on Linux and has no weaker
/// fallback on unsupported platforms. Descriptor reading verifies regular-file
/// type, byte ceiling, exact EOF and observed metadata consistency. This does
/// not authenticate the caller or atomically freeze a concurrently edited file.
///
/// # Errors
/// Refuses unsafe, missing, unsupported or inconsistent sources without bytes.
pub fn read_local_profile_bytes(
    path: &Path,
    maximum_bytes: usize,
) -> Result<Vec<u8>, LocalProfileFileReadError> {
    let file = open_selected_profile_file(path).map_err(|error| match error {
        ProfileSelectedFileOpenError::FileIo(io::ErrorKind::NotFound) => {
            LocalProfileFileReadError::NotFound
        }
        ProfileSelectedFileOpenError::InvalidSelectedPath
        | ProfileSelectedFileOpenError::NotRegularFile
        | ProfileSelectedFileOpenError::ParentTraversal
        | ProfileSelectedFileOpenError::SymlinkForbidden => {
            LocalProfileFileReadError::InvalidInput
        }
        ProfileSelectedFileOpenError::UnsupportedOpenPolicy => {
            LocalProfileFileReadError::Unavailable
        }
        ProfileSelectedFileOpenError::FileIo(_)
        | ProfileSelectedFileOpenError::WorkingDirectoryChanged => {
            LocalProfileFileReadError::FileIo
        }
    })?;
    let bytes = read_bounded_profile_file(file, maximum_bytes)
        .map_err(classify_bounded_profile_read_error)?;
    Ok(bytes.bytes().to_vec())
}
