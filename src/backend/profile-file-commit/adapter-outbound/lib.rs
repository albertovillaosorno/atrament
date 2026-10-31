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
use std::ffi::OsStr;
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
    /// A committed file may no longer resolve through its selected parent.
    CommittedPathUnverified,
    /// Initial staging, commit I/O or cleanup failed.
    FileIo(io::ErrorKind),
    /// Replacement target is observed as a link or non-regular file.
    InvalidExistingTarget,
    /// Missing file name or forbidden parent traversal.
    InvalidTarget,
    /// Random staging name generation failed.
    RandomUnavailable,
    /// The selected parent or relative CWD changed before any file effect.
    SelectedParentChanged,
    /// The observed replacement entry changed during precommit staging.
    SelectedTargetChanged,
    /// The anonymous stage changed after file sync but before publication.
    StagedFileChanged,
    /// Existing target conflicts with preserve-existing intent.
    TargetExists,
    /// Secure operation is unavailable on this platform.
    UnsupportedPlatform,
}

impl ProfileFileCommitError {
    /// Whether this refusal may follow a permanent directory-entry effect.
    ///
    /// Caller retry journals must retain uncertainty for these outcomes,
    /// never record an ordinary no-commit refusal. Exhaustive matching makes
    /// future transport error kinds choose their effect classification.
    #[must_use]
    pub const fn may_have_committed(self) -> bool {
        match self {
            Self::CommittedDurabilityUnknown(_)
            | Self::CommittedPathUnverified => true,
            Self::ArtifactTooLarge
            | Self::CancelledBeforeCommit
            | Self::FileIo(_)
            | Self::InvalidExistingTarget
            | Self::InvalidTarget
            | Self::RandomUnavailable
            | Self::SelectedParentChanged
            | Self::SelectedTargetChanged
            | Self::StagedFileChanged
            | Self::TargetExists
            | Self::UnsupportedPlatform => false,
        }
    }
}

/// A successful exact, synced file commit (not an application Export receipt).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileFileCommitEvidence {
    /// Number of complete artifact bytes committed to the selected target.
    pub byte_length: usize,
    /// Metadata-change time (seconds, nanoseconds) after the final link.
    pub file_changed: (i64, i64),
    /// Linux device identity of the staged inode linked at the selected name.
    pub file_device: u64,
    /// Linux inode identity of the staged file, retained for retry recovery.
    pub file_inode: u64,
    /// Modification time (seconds, nanoseconds) after the final file commit.
    pub file_modified: (i64, i64),
}

#[cfg(target_os = "linux")]
struct OwnedTemp<'directory> {
    committed: bool,
    dir: &'directory File,
    name: String,
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReplaceTargetObservation {
    changed: (i64, i64),
    device: u64,
    inode: u64,
    length: i64,
    modified: (i64, i64),
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
/// before the first permanent directory entry is created. The selected parent
/// is reopened before linking and after directory sync; device/inode drift
/// before the link refuses without effect, while post-link drift is uncertain.
/// After syncing, the selected leaf must still be the committed regular inode
/// with matching size and timestamps; a vanished, replaced or linked leaf
/// yields an uncertain committed-path result, never ordinary precommit refusal.
/// For relative targets, the process CWD must still match its original anchor.
/// Replacement also rechecks the selected entry without following links
/// after staging, refusing late symlinks or non-regular targets and changes
/// to the initial target's existence, identity or timestamps before linking.
/// The anonymous stage's identity, size and times must remain unchanged
/// since its completed sync; drift observed before linking is a proven
/// precommit refusal with no selected-target effect.
/// Another actor can change the entry after this final observation; the
/// recheck is not an atomic compare-and-replace guarantee.
/// Linux stages bytes through `O_TMPFILE`, so cancellation or process death
/// during writing cannot
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
/// Pathname checks reduce the rename/replacement window but cannot lock an
/// external pathname: a concurrent actor can still move the directory after
/// either observation, and a failed postcommit observation does not undo a
/// preceding file effect. Only Linux has the secure directory-relative
/// implementation. Other platforms refuse without a weak fallback.
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
        || target.as_os_str().as_encoded_bytes().contains(&0)
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
    commit_platform(target, artifact, disposition, &mut is_cancelled)
}

#[cfg(not(target_os = "linux"))]
fn commit_platform(
    _target: &Path,
    _artifact: &[u8],
    _disposition: ProfileFileCommitDisposition,
    _is_cancelled: &mut dyn FnMut() -> bool,
) -> Result<ProfileFileCommitEvidence, ProfileFileCommitError> {
    Err(ProfileFileCommitError::UnsupportedPlatform)
}

#[cfg(target_os = "linux")]
fn same_opened_directory_identity(first: &File, second: &File) -> bool {
    use std::os::unix::fs::MetadataExt as _;

    let Ok(original) = first.metadata() else {
        return false;
    };
    let Ok(current) = second.metadata() else {
        return false;
    };
    original.dev() == current.dev() && original.ino() == current.ino()
}

#[cfg(target_os = "linux")]
fn selected_parent_is_same_directory(
    root: &File,
    parent_path: &Path,
    selected_directory: &File,
    relative_target: bool,
) -> bool {
    use nix::fcntl::{OFlag, OpenHow, ResolveFlag, openat2};

    // A relative target's external pathname is also bound to process CWD.
    // Reopening only beneath the original descriptor would mistake a
    // post-selection chdir for a successful write at the selected pathname.
    if relative_target {
        let Ok(current_cwd) = File::open(".") else {
            return false;
        };
        if !same_opened_directory_identity(root, &current_cwd) {
            return false;
        }
    }
    let how = OpenHow::new()
        .flags(OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC)
        .resolve(
            ResolveFlag::RESOLVE_BENEATH | ResolveFlag::RESOLVE_NO_SYMLINKS,
        );
    let Ok(reopened) = openat2(root, parent_path, how) else {
        return false;
    };
    same_opened_directory_identity(selected_directory, &File::from(reopened))
}

#[cfg(target_os = "linux")]
fn observe_replace_target(
    directory: &File,
    name: &OsStr,
) -> Result<Option<ReplaceTargetObservation>, ProfileFileCommitError> {
    use nix::fcntl::AtFlags;
    use nix::sys::stat::{SFlag, fstatat};

    match fstatat(directory, name, AtFlags::AT_SYMLINK_NOFOLLOW) {
        Ok(metadata)
            if SFlag::from_bits_truncate(metadata.st_mode) & SFlag::S_IFMT
                != SFlag::S_IFREG =>
        {
            Err(ProfileFileCommitError::InvalidExistingTarget)
        }
        Ok(metadata) => Ok(Some(ReplaceTargetObservation {
            changed: (metadata.st_ctime, metadata.st_ctime_nsec),
            device: metadata.st_dev,
            inode: metadata.st_ino,
            length: metadata.st_size,
            modified: (metadata.st_mtime, metadata.st_mtime_nsec),
        })),
        Err(Errno::ENOENT) => Ok(None),
        Err(error) => Err(ProfileFileCommitError::FileIo(
            io::Error::from(error).kind(),
        )),
    }
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
) -> Result<ProfileFileCommitEvidence, ProfileFileCommitError> {
    use std::fs::Permissions;
    use std::io::Write as _;
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};

    use nix::fcntl::{
        AtFlags, OFlag, OpenHow, RenameFlags, ResolveFlag, openat, openat2,
        renameat2,
    };
    use nix::sys::stat::Mode;

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

    // Bind explicit replacement to the selected entry's observed state.
    // Missing and existing files are distinct; no link is dereferenced.
    let replace_target_before = if disposition
        == ProfileFileCommitDisposition::ReplaceExisting
    {
        Some(observe_replace_target(&directory, name)?)
    } else {
        None
    };

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
    // O_TMPFILE creation is still subject to the process umask. Reassert
    // private, owner-readable 0600 on the unlinked descriptor so restrictive
    // host umasks cannot turn a successful Export into an unreadable file.
    // This happens before any bytes are written or published by a link.
    output
        .set_permissions(Permissions::from_mode(0o600))
        .map_err(|error| {
            ProfileFileCommitError::FileIo(error.kind())
        })?;
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
    // Observe the private staged inode before any link or rename. This is
    // the committed file identity even if a later caller changes the path.
    let staged_metadata = output
        .metadata()
        .map_err(|error| ProfileFileCommitError::FileIo(error.kind()))?;
    // Even an anonymous inode can be linked by a same-user actor with
    // access to its descriptor. A preexisting name breaks the intended
    // private-stage boundary regardless of matching bytes and timestamps.
    if staged_metadata.nlink() != 0 {
        return Err(ProfileFileCommitError::StagedFileChanged);
    }
    let file_identity = (staged_metadata.dev(), staged_metadata.ino());
    if is_cancelled() {
        return Err(ProfileFileCommitError::CancelledBeforeCommit);
    }
    // The selected directory may have been renamed/replaced during staging.
    // Do not link the anonymous inode to a detached, now-unselected parent.
    if !selected_parent_is_same_directory(
        &root,
        directory_path,
        &directory,
        !target.is_absolute(),
    ) {
        return Err(ProfileFileCommitError::SelectedParentChanged);
    }

    // Staging and the final cancellation callback may observe another
    // writer's inserted, removed or modified entry. Refuse that change
    // before publishing even the temporary replacement name. This is a
    // late observation, never an atomic pathname lock.
    if let Some(before) = replace_target_before
        && before != observe_replace_target(&directory, name)?
    {
        return Err(ProfileFileCommitError::SelectedTargetChanged);
    }

    // The final cancellation callback or another process could have opened
    // our still-unnamed stage via a process descriptor. Verify it remains the
    // exact synced inode before making its first visible directory entry.
    let stage_before_link = output
        .metadata()
        .map_err(|error| ProfileFileCommitError::FileIo(error.kind()))?;
    if stage_before_link.nlink() != 0
        || stage_before_link.dev() != staged_metadata.dev()
        || stage_before_link.ino() != staged_metadata.ino()
        || stage_before_link.len() != staged_metadata.len()
        || stage_before_link.mtime() != staged_metadata.mtime()
        || stage_before_link.mtime_nsec() != staged_metadata.mtime_nsec()
        || stage_before_link.ctime() != staged_metadata.ctime()
        || stage_before_link.ctime_nsec() != staged_metadata.ctime_nsec()
    {
        return Err(ProfileFileCommitError::StagedFileChanged);
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
    // Keep the descriptor open through the final metadata observation.
    // The target already changed. Never return an ordinary no-effect error.
    directory.sync_all().map_err(|error| {
        ProfileFileCommitError::CommittedDurabilityUnknown(error.kind())
    })?;
    // Path resolution is external mutable state. Recheck after the file
    // effect: if its parent moved since the precommit check, never issue a
    // success receipt for an output no longer at the selected pathname.
    if !selected_parent_is_same_directory(
        &root,
        directory_path,
        &directory,
        !target.is_absolute(),
    ) {
        return Err(ProfileFileCommitError::CommittedPathUnverified);
    }
    // Link and rename can update ctime, so report metadata only after the
    // final synced directory-entry effect, never the initial staged values.
    let committed_metadata = output
        .metadata()
        .map_err(|_error| ProfileFileCommitError::CommittedPathUnverified)?;
    if committed_metadata.nlink() != 1
        || committed_metadata.dev() != file_identity.0
        || committed_metadata.ino() != file_identity.1
        || committed_metadata.len() != staged_metadata.len()
        || committed_metadata.mtime() != staged_metadata.mtime()
        || committed_metadata.mtime_nsec() != staged_metadata.mtime_nsec()
    {
        // Link/rename changes ctime, not length or mtime. A new mtime after
        // stage sync is evidence of a concurrent write, not original bytes.
        return Err(ProfileFileCommitError::CommittedPathUnverified);
    }
    // Rechecking the parent alone cannot prove the selected leaf still
    // points to the inode we synced: a concurrent actor may unlink, rename
    // or substitute the selected entry after the commit boundary. Inspect
    // the leaf without following links and compare all recorded identity,
    // length and timestamp fields. This is one final observation, not a lock.
    let expected_length = i64::try_from(artifact.len())
        .map_err(|_error| ProfileFileCommitError::CommittedPathUnverified)?;
    let expected_target = ReplaceTargetObservation {
        changed: (
            committed_metadata.ctime(),
            committed_metadata.ctime_nsec(),
        ),
        device: committed_metadata.dev(),
        inode: committed_metadata.ino(),
        length: expected_length,
        modified: (
            committed_metadata.mtime(),
            committed_metadata.mtime_nsec(),
        ),
    };
    if observe_replace_target(&directory, name).ok().flatten()
        != Some(expected_target)
    {
        return Err(ProfileFileCommitError::CommittedPathUnverified);
    }
    Ok(ProfileFileCommitEvidence {
        byte_length: artifact.len(),
        file_device: file_identity.0,
        file_inode: file_identity.1,
        file_modified: (
            committed_metadata.mtime(),
            committed_metadata.mtime_nsec(),
        ),
        file_changed: (
            committed_metadata.ctime(),
            committed_metadata.ctime_nsec(),
        ),
    })
}
