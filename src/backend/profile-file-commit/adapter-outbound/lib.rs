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
//     independently admitted overwrite disposition and optional cancel probe.
//   - Outputs: Exact byte count, precommit cancellation or typed I/O refusal.
//   - Side effects: One explicit file commit; owned sibling temp cleanup.
// - Split-When:
//   - Revision/authorization and same-retry recovery are composed upstream.
// - Merge-When:
//   - A complete Export adapter subsumes this narrow file transport.
// - Summary:
//   - Anonymous Linux staging with late link and atomic file replacement.
// - Description:
//   - Precommit failures preserve targets; postcommit sync failure is unknown.
// - Usage:
//   - Call only after authenticated path, revision and artifact admission.
// - Defaults:
//   - No symlink traversal, no named mid-write stage or weak fallback.
//

//! Low-level profile file-commit transport, never itself Export admission.

use std::io;
use std::path::{Component, Path};

#[cfg(target_os = "linux")]
use std::fs::File;

#[cfg(target_os = "linux")]
use nix::errno::Errno;
#[cfg(target_os = "linux")]
use nix::unistd::{UnlinkatFlags, linkat, unlinkat};

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
    /// Cancellation was observed before the irreversible rename.
    CancelledBeforeCommit,
    /// The target changed but directory sync did not prove durability.
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
/// This compatibility entrypoint uses the same atomic staging behavior as
/// [`commit_profile_file_cancellable`] without a cancellation source.
/// Selection, revision, artifact and overwrite admission remain the caller's
/// responsibility, not this file transport's authority.
///
/// # Errors
///
/// Refuses invalid path, bytes, conflict or unavailable I/O. Directory-sync
/// failure after rename remains explicitly uncertain.
pub fn commit_profile_file(
    target: &Path,
    artifact: &[u8],
    maximum_bytes: usize,
    disposition: ProfileFileCommitDisposition,
) -> Result<ProfileFileCommitEvidence, ProfileFileCommitError> {
    commit_profile_file_cancellable(
        target,
        artifact,
        maximum_bytes,
        disposition,
        || false,
    )
}

/// Persist admitted bytes with cooperative checks before the file commit.
///
/// A caller-supplied predicate is checked after byte/path preflight and before
/// any filesystem effects, before each 64 KiB write, and after file sync just
/// before the first permanent directory entry is created. Linux stages bytes
/// through `O_TMPFILE`, so cancellation or process death during writing cannot
/// leave a named temporary file. Preserve-existing links the completed file
/// directly to the chosen target; replacement links a random sibling for the
/// shortest possible rename window. Crash after that link but before rename
/// can still leave the owned stage name, so it is not a fully crash-atomic
/// replacement cleanup guarantee. The predicate is cancellation observation,
/// never a source of Export authorization.
///
/// A predicate that becomes true **after** the final check cannot revoke a
/// completed commit. A directory-sync failure after linking/rename remains
/// uncertain; the caller must not blindly retry.
/// Only Linux has the secure directory-relative implementation. Other
/// platforms refuse without a weak fallback.
///
/// # Errors
///
/// Reports proven precommit cancellation, typed I/O refusal, and uncertain
/// post-rename durability without suggesting a receipt or retry policy.
pub fn commit_profile_file_cancellable<Cancel>(
    target: &Path,
    artifact: &[u8],
    maximum_bytes: usize,
    disposition: ProfileFileCommitDisposition,
    mut is_cancelled: Cancel,
) -> Result<ProfileFileCommitEvidence, ProfileFileCommitError>
where
    Cancel: FnMut() -> bool,
{
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
    if is_cancelled() {
        return Err(ProfileFileCommitError::CancelledBeforeCommit);
    }
    commit_platform(target, artifact, disposition, &mut is_cancelled)?;
    Ok(ProfileFileCommitEvidence {
        byte_length: artifact.len(),
    })
}

#[cfg(not(target_os = "linux"))]
fn commit_platform(
    _target: &Path,
    _artifact: &[u8],
    _disposition: ProfileFileCommitDisposition,
    _is_cancelled: &mut dyn FnMut() -> bool,
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
    is_cancelled: &mut dyn FnMut() -> bool,
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

    // An unnamed inode cannot survive process death as a directory entry.
    // Refuse a filesystem without anonymous staging instead of falling back
    // to a named file that can leak partial profile bytes after a crash.
    let mut output = File::from(openat(
        &directory,
        ".",
        OFlag::O_WRONLY | OFlag::O_TMPFILE | OFlag::O_CLOEXEC,
        Mode::S_IRUSR | Mode::S_IWUSR,
    ).map_err(|error| {
        if matches!(
            error,
            Errno::ENOSYS | Errno::EINVAL | Errno::EOPNOTSUPP | Errno::EISDIR
        ) {
            ProfileFileCommitError::UnsupportedPlatform
        } else {
            ProfileFileCommitError::FileIo(io::Error::from(error).kind())
        }
    })?);
    for chunk in artifact.chunks(64 * 1024) {
        if is_cancelled() {
            return Err(ProfileFileCommitError::CancelledBeforeCommit);
        }
        output
            .write_all(chunk)
            .map_err(|error| ProfileFileCommitError::FileIo(error.kind()))?;
    }
    output
        .sync_all()
        .map_err(|error| ProfileFileCommitError::FileIo(error.kind()))?;
    if is_cancelled() {
        return Err(ProfileFileCommitError::CancelledBeforeCommit);
    }

    match disposition {
        ProfileFileCommitDisposition::PreserveExisting => {
            // Direct linking is atomic no-overwrite without a temporary name.
            let linked =
                linkat(&output, "", &directory, name, AtFlags::AT_EMPTY_PATH);
            match linked {
                Ok(()) => {}
                Err(Errno::EEXIST) => {
                    return Err(ProfileFileCommitError::TargetExists);
                }
                Err(error) => {
                    return Err(ProfileFileCommitError::FileIo(
                        io::Error::from(error).kind(),
                    ));
                }
            }
        }
        ProfileFileCommitDisposition::ReplaceExisting => {
            // Replacement needs a name for renameat2. Create it only after
            // all bytes are synced and the final cancellation check passes.
            let mut linked = None;
            for _attempt in 0..6u8 {
                let mut nonce = [0u8; 16];
                getrandom::fill(&mut nonce).map_err(|_error| {
                    ProfileFileCommitError::RandomUnavailable
                })?;
                let temporary_name = format!(
                    ".atrament-export-{:032x}.tmp",
                    u128::from_be_bytes(nonce),
                );
                match linkat(
                    &output,
                    "",
                    &directory,
                    temporary_name.as_str(),
                    AtFlags::AT_EMPTY_PATH,
                ) {
                    Ok(()) => {
                        linked = Some(OwnedTemp {
                            committed: false,
                            dir: &directory,
                            name: temporary_name,
                        });
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
            let mut temp =
                linked.ok_or(ProfileFileCommitError::RandomUnavailable)?;
            renameat2(
                &directory,
                temp.name.as_str(),
                &directory,
                name,
                RenameFlags::empty(),
            )
            .map_err(|error| {
                ProfileFileCommitError::FileIo(io::Error::from(error).kind())
            })?;
            temp.committed = true;
        }
    }
    drop(output);
    // The target already changed. Never return an ordinary no-effect error.
    directory.sync_all().map_err(|error| {
        ProfileFileCommitError::CommittedDurabilityUnknown(error.kind())
    })?;
    Ok(())
}
