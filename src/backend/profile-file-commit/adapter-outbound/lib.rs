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
//   - One all-or-failed, same-directory profile byte file-commit primitive.
// - Must-Not:
//   - Grant Export authority, choose a target, derive revision bytes, retry,
//     autosave, decode a profile, or expose owned temporary names.
// - Allows:
//   - Inputs: Trusted host-selected target, complete artifact bytes, byte cap,
//     and independently admitted overwrite disposition.
//   - Outputs: Exact committed byte count or typed I/O/commit uncertainty.
//   - Side effects: One explicit file commit; owned sibling temp cleanup.
// - Split-When:
//   - Revision/authorization and same-retry recovery are composed upstream.
// - Merge-When:
//   - A complete Export adapter subsumes this narrow file transport.
// - Summary:
//   - Secure directory-relative staging and atomic Linux file replacement.
// - Description:
//   - Precommit failures preserve targets; postcommit sync failure is unknown.
// - Usage:
//   - Call only after authenticated path, revision and artifact admission.
// - Defaults:
//   - No symlink parent traversal, no silent overwrite, no portable fallback.
//

//! Low-level profile file-commit transport, never itself Export admission.

use std::io;
use std::path::{Component, Path};

#[cfg(target_os = "linux")]
use std::fs::File;

#[cfg(target_os = "linux")]
use nix::errno::Errno;
#[cfg(target_os = "linux")]
use nix::unistd::{UnlinkatFlags, unlinkat};

/// Caller-supplied policy for an already admitted Export target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileFileCommitDisposition {
    /// Never replace any existing target, including a link.
    PreserveExisting,
    /// Replace the exact directory entry only when explicitly authorized.
    ReplaceExisting,
}

/// File effect outcome; only `Ok` proves a committed, synced target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileFileCommitError {
    /// Complete artifact exceeds caller byte ceiling.
    ArtifactTooLarge,
    /// Rename succeeded, but directory sync did not prove durability.
    CommittedDurabilityUnknown(io::ErrorKind),
    /// Initial staging, commit I/O or cleanup failed.
    FileIo(io::ErrorKind),
    /// Existing replacement target is a link or non-regular file.
    InvalidExistingTarget,
    /// Missing file name or forbidden parent traversal.
    InvalidTarget,
    /// Random staging name generation failed.
    RandomUnavailable,
    /// Existing target conflicts with preserve-existing intent.
    TargetExists,
    /// Secure operation is unavailable on this platform.
    UnsupportedPlatform,
}

/// A successful exact, synced file commit (not an application Export receipt).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileFileCommitEvidence {
    /// Number of complete artifact bytes committed to the selected target.
    pub byte_length: usize,
}

#[cfg(target_os = "linux")]
struct OwnedTemp<'directory> {
    committed: bool,
    dir: &'directory File,
    name: String,
}

#[cfg(target_os = "linux")]
#[expect(
    clippy::missing_trait_methods,
    reason = "Drop has only its cleanup method"
)]
impl Drop for OwnedTemp<'_> {
    fn drop(&mut self) {
        if !self.committed {
            let _result: Result<(), Errno> = unlinkat(
                self.dir,
                self.name.as_str(),
                UnlinkatFlags::NoRemoveDir,
            );
        }
    }
}

/// Persist one complete, already validated artifact at an authorized target.
///
/// This adapter cannot establish caller selection, accepted revision, profile
/// semantics or retry identity. The owning host must establish those **before**
/// calling this function. Linux uses a symlink-free, descriptor-bound parent,
/// a random exclusive 0600 sibling file, `sync_all`, and atomic `renameat2`.
/// The directory is synced after rename. A failure after rename is explicitly
/// ambiguous, never advertised as a known no-effect. An unexpected target
/// change can race a replacement: rename changes the directory entry, never
/// follows the target link. Non-Linux platforms refuse without a weak fallback.
///
/// # Errors
///
/// Refuses invalid path, bytes, conflict or unavailable I/O without reporting
/// Export success. If directory sync fails after rename, the file may already
/// have been committed; the caller must recover rather than blindly retry.
pub fn commit_profile_file(
    target: &Path,
    artifact: &[u8],
    maximum_bytes: usize,
    disposition: ProfileFileCommitDisposition,
) -> Result<ProfileFileCommitEvidence, ProfileFileCommitError> {
    if artifact.len() > maximum_bytes {
        return Err(ProfileFileCommitError::ArtifactTooLarge);
    }
    if target.file_name().is_none()
        || target.as_os_str().as_encoded_bytes().ends_with(b"/")
        || target.as_os_str().as_encoded_bytes().ends_with(b"/.")
        || target.as_os_str().as_encoded_bytes().ends_with(b"/..")
        || target
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(ProfileFileCommitError::InvalidTarget);
    }
    commit_platform(target, artifact, disposition)?;
    Ok(ProfileFileCommitEvidence {
        byte_length: artifact.len(),
    })
}

#[cfg(not(target_os = "linux"))]
fn commit_platform(
    _target: &Path,
    _artifact: &[u8],
    _disposition: ProfileFileCommitDisposition,
) -> Result<(), ProfileFileCommitError> {
    Err(ProfileFileCommitError::UnsupportedPlatform)
}

#[cfg(target_os = "linux")]
#[expect(
    clippy::too_many_lines,
    reason = "single atomic file lifecycle with unified error paths"
)]
fn commit_platform(
    target: &Path,
    artifact: &[u8],
    disposition: ProfileFileCommitDisposition,
) -> Result<(), ProfileFileCommitError> {
    use std::io::Write as _;

    use nix::fcntl::{
        AtFlags, OFlag, OpenHow, RenameFlags, ResolveFlag, openat, openat2,
        renameat2,
    };
    use nix::sys::stat::{Mode, SFlag, fstatat};

    // The only filesystem root for the operation is the exact selected
    // parent, secured before creating anything. This protects all ancestor
    // components against symlinks and anchors a rename-resistant directory.
    let (anchor, raw_parent) = if target.is_absolute() {
        (
            "/",
            target
                .parent()
                .and_then(|parent| parent.strip_prefix("/").ok()),
        )
    } else {
        (".", target.parent())
    };
    let directory_path = match raw_parent {
        Some(value) if !value.as_os_str().is_empty() => value,
        _ => Path::new("."),
    };
    let root = File::open(anchor)
        .map_err(|error| ProfileFileCommitError::FileIo(error.kind()))?;
    let how = OpenHow::new()
        .flags(OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC)
        .resolve(
            ResolveFlag::RESOLVE_BENEATH | ResolveFlag::RESOLVE_NO_SYMLINKS,
        );
    let directory =
        File::from(openat2(&root, directory_path, how).map_err(|error| {
            if error == Errno::ENOSYS || error == Errno::EINVAL {
                ProfileFileCommitError::UnsupportedPlatform
            } else {
                ProfileFileCommitError::FileIo(io::Error::from(error).kind())
            }
        })?);
    let name = target
        .file_name()
        .ok_or(ProfileFileCommitError::InvalidTarget)?;

    // For explicit replacement, reject an already observed link or special
    // file without dereferencing. renameat2 never follows a final symlink;
    // a concurrent replacement of that directory entry cannot redirect bytes.
    if disposition == ProfileFileCommitDisposition::ReplaceExisting {
        match fstatat(&directory, name, AtFlags::AT_SYMLINK_NOFOLLOW) {
            Ok(metadata)
                if SFlag::from_bits_truncate(metadata.st_mode)
                    & SFlag::S_IFMT
                    != SFlag::S_IFREG =>
            {
                return Err(ProfileFileCommitError::InvalidExistingTarget);
            }
            Err(Errno::ENOENT) | Ok(_) => {}
            Err(error) => {
                return Err(ProfileFileCommitError::FileIo(
                    io::Error::from(error).kind(),
                ));
            }
        }
    }

    let mut staging = None;
    for _attempt in 0..6u8 {
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce)
            .map_err(|_error| ProfileFileCommitError::RandomUnavailable)?;
        let temporary_name =
            format!(".atrament-export-{:032x}.tmp", u128::from_be_bytes(nonce));
        match openat(
            &directory,
            temporary_name.as_str(),
            OFlag::O_WRONLY
                | OFlag::O_CREAT
                | OFlag::O_EXCL
                | OFlag::O_NOFOLLOW
                | OFlag::O_CLOEXEC,
            Mode::S_IRUSR | Mode::S_IWUSR,
        ) {
            Ok(fd) => {
                staging = Some((
                    File::from(fd),
                    OwnedTemp {
                        dir: &directory,
                        name: temporary_name,
                        committed: false,
                    },
                ));
                break;
            }
            Err(Errno::EEXIST) => {}
            Err(error) => {
                return Err(ProfileFileCommitError::FileIo(
                    io::Error::from(error).kind(),
                ));
            }
        }
    }
    let (mut output, mut temp) =
        staging.ok_or(ProfileFileCommitError::RandomUnavailable)?;
    output
        .write_all(artifact)
        .and_then(|()| output.sync_all())
        .map_err(|error| ProfileFileCommitError::FileIo(error.kind()))?;
    drop(output);

    let flags = match disposition {
        ProfileFileCommitDisposition::PreserveExisting => {
            RenameFlags::RENAME_NOREPLACE
        }
        ProfileFileCommitDisposition::ReplaceExisting => RenameFlags::empty(),
    };
    match renameat2(&directory, temp.name.as_str(), &directory, name, flags) {
        Ok(()) => {
            temp.committed = true;
        }
        Err(Errno::EEXIST) => return Err(ProfileFileCommitError::TargetExists),
        Err(error) => {
            return Err(ProfileFileCommitError::FileIo(
                io::Error::from(error).kind(),
            ));
        }
    }
    // The target already changed. Never return an ordinary no-effect error.
    directory.sync_all().map_err(|error| {
        ProfileFileCommitError::CommittedDurabilityUnknown(error.kind())
    })?;
    Ok(())
}
