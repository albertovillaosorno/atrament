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
//   - Exact, read-only comparison of one selected Export target to a sealed
//     in-memory profile artifact for later same-retry recovery checks.
// - Must-Not:
//   - Claim caller authorization, bind a session revision or retry identity,
//     issue an Export receipt, write a file, or infer a previous file commit.
// - Allows:
//   - Inputs: Source-classified ExportProfile path, verified artifact, cap.
//   - Outputs: Exact-byte match, drift, missing target or typed refusal.
//   - Side effects: Opens, reads, and closes a selected regular file only.
// - Split-When:
//   - Application retry state can independently attest operation identity.
// - Merge-When:
//   - Full Export session retry recovery owns this verification boundary.
// - Summary:
//   - Safe target-byte checks without rewriting external changes.
// - Description:
//   - Symlink-free opened bytes are bounded, then the selected name is
//     reopened securely to detect observed pathname and metadata rebinding.
// - Usage:
//   - Use only with previously recorded artifact identity and retry state.
// - Defaults:
//   - A matching file is evidence, never permission to return Exported.
//

//! Read-only selected Export target comparison; no commit or retry authority.

use std::fmt;
use std::fs::{File, Metadata};
use std::io;
use std::path::Path;

// The integration fixture alone exercises these dependencies.
#[cfg(test)]
use atrament_profile_file_commit as _;
#[cfg(test)]
use atrament_profile_manifest_json_writer as _;
#[cfg(test)]
use atrament_profile_stored_zip_writer as _;

use atrament_explicit_file_path_intent::{
    ExplicitFilePathIntent, ExplicitFilePurpose,
};
use atrament_handwriting_profile_container::Sha256Digest;
use atrament_profile_export_assembly::ProfileExportArtifact;
use atrament_profile_file_reader::{
    ProfileFileReadError, read_bounded_profile_file,
};
use atrament_profile_selected_file_open::{
    ProfileSelectedFileOpenError, open_selected_profile_file,
    open_selected_profile_file_from_cwd,
};

/// Observed target bytes at this instant, not a prior Export receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileExportTargetStatus {
    /// File was read completely but is not the requested artifact.
    Drifted,
    /// Selected target holds exact bytes with its rechecked file identity.
    Matches {
        /// SHA-256 supplied by the already-verified artifact assembly.
        archive_sha256: Sha256Digest,
        /// Exact number of matched bytes.
        byte_length: usize,
        /// Metadata-change time (seconds, nanoseconds) of the reopened target.
        file_changed: (i64, i64),
        /// Linux device of the rechecked, securely opened target file.
        file_device: u64,
        /// Linux inode of the rechecked, securely opened target file.
        file_inode: u64,
        /// Modification time (seconds, nanoseconds) of the reopened target.
        file_modified: (i64, i64),
    },
    /// No file currently exists at the selected path.
    Missing,
}

/// Refusal without claiming a match or rewriting the target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileExportTargetCheckError {
    /// Expected artifact itself exceeds caller resource ceiling.
    ArtifactTooLarge,
    /// Secure selected file opening failed (including unsafe symlinks).
    Open(ProfileSelectedFileOpenError),
    /// The opened file could not be read as one bounded complete source.
    Read(ProfileFileReadError),
    /// A notebook or Import path cannot stand in for a profile Export target.
    WrongPathPurpose,
}

/// One opened, path-bound, read-only comparison in progress.
///
/// This opaque probe owns the first securely opened descriptor. It is neither
/// authorization nor proof of a prior Export, and cannot be constructed from
/// arbitrary file handles. Verification consumes the probe and reopens the
/// exact selected name before reporting a match. Relative selections retain
/// their initial CWD descriptor for both secure opens.
#[must_use]
pub struct ProfileExportTargetProbe<'selection> {
    artifact: &'selection ProfileExportArtifact,
    maximum_bytes: usize,
    opened: File,
    original_metadata: Metadata,
    selected_cwd: Option<File>,
    selected_path: &'selection Path,
}

impl fmt::Debug for ProfileExportTargetProbe<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProfileExportTargetProbe")
            .finish_non_exhaustive()
    }
}

impl ProfileExportTargetProbe<'_> {
    /// Consume this opened probe and compare complete source bytes to the ZIP.
    ///
    /// A byte match is not enough: the path must still open without following
    /// symlinks and resolve to the original file identity and metadata.
    /// A concurrent mutation after the final observation remains possible.
    ///
    /// # Errors
    ///
    /// Refuses unreadable or changing sources and unsafe path rebinding.
    pub fn verify(
        self,
    ) -> Result<ProfileExportTargetStatus, ProfileExportTargetCheckError> {
        let source =
            match read_bounded_profile_file(self.opened, self.maximum_bytes) {
                Ok(source) => source,
                // The complete artifact is already within the cap, so a larger
                // source is necessarily different from the expected artifact.
                Err(ProfileFileReadError::FileTooLarge) => {
                    return Ok(ProfileExportTargetStatus::Drifted);
                }
                Err(error) => {
                    return Err(ProfileExportTargetCheckError::Read(error));
                }
            };
        if source.bytes() != self.artifact.bytes() {
            return Ok(ProfileExportTargetStatus::Drifted);
        }
        let reopened = self.selected_cwd.as_ref().map_or_else(
            || open_selected_profile_file(self.selected_path),
            |cwd| open_selected_profile_file_from_cwd(self.selected_path, cwd),
        );
        let current = match reopened {
            Ok(current) => current,
            Err(ProfileSelectedFileOpenError::FileIo(
                io::ErrorKind::NotFound,
            )) => {
                return Ok(ProfileExportTargetStatus::Missing);
            }
            Err(error) => {
                return Err(ProfileExportTargetCheckError::Open(error));
            }
        };
        let current_metadata = current.metadata().map_err(|error| {
            ProfileExportTargetCheckError::Read(ProfileFileReadError::FileIo(
                error.kind(),
            ))
        })?;
        if !same_selected_file_metadata(
            &self.original_metadata,
            &current_metadata,
        ) {
            return Ok(ProfileExportTargetStatus::Drifted);
        }
        #[cfg(unix)]
        let (file_device, file_inode, file_modified, file_changed) = {
            use std::os::unix::fs::MetadataExt as _;
            (
                current_metadata.dev(),
                current_metadata.ino(),
                (current_metadata.mtime(), current_metadata.mtime_nsec()),
                (current_metadata.ctime(), current_metadata.ctime_nsec()),
            )
        };
        #[cfg(not(unix))]
        let (file_device, file_inode, file_modified, file_changed) =
            (0, 0, (0, 0), (0, 0));
        Ok(ProfileExportTargetStatus::Matches {
            archive_sha256: self.artifact.archive_sha256(),
            byte_length: self.artifact.bytes().len(),
            file_device,
            file_inode,
            file_modified,
            file_changed,
        })
    }
}

/// Securely open an Export target for one subsequent read-only verification.
///
/// A missing file returns `None`, while a present file yields a sealed probe
/// bound to the supplied artifact and selection. Splitting open and verify
/// allows a caller to retain the descriptor while the external path changes;
/// such rebinding must not produce `Matches` from stale opened bytes.
/// The caller still must authenticate the selection and any prior receipt.
///
/// # Errors
///
/// Refuses wrong-purpose or over-budget artifacts before I/O, unsupported or
/// unsafe selected paths and failed initial descriptor inspection.
pub fn open_profile_export_target_probe<'selection>(
    intent: &'selection ExplicitFilePathIntent,
    artifact: &'selection ProfileExportArtifact,
    maximum_bytes: usize,
) -> Result<
    Option<ProfileExportTargetProbe<'selection>>,
    ProfileExportTargetCheckError,
> {
    if intent.purpose() != ExplicitFilePurpose::ExportProfile {
        return Err(ProfileExportTargetCheckError::WrongPathPurpose);
    }
    if artifact.bytes().len() > maximum_bytes {
        return Err(ProfileExportTargetCheckError::ArtifactTooLarge);
    }
    // A relative selection must retain the original process CWD. Two
    // independently secure opens in different CWDs are not the same selected
    // name even when both happen to lead to hardlinks of one inode.
    let selected_cwd = (!intent.path().is_absolute())
        .then(|| File::open("."))
        .transpose()
        .map_err(|error| {
            ProfileExportTargetCheckError::Open(
                ProfileSelectedFileOpenError::FileIo(error.kind()),
            )
        })?;
    let opened_result = selected_cwd.as_ref().map_or_else(
        || open_selected_profile_file(intent.path()),
        |cwd| open_selected_profile_file_from_cwd(intent.path(), cwd),
    );
    let opened = match opened_result {
        Ok(opened) => opened,
        Err(ProfileSelectedFileOpenError::FileIo(io::ErrorKind::NotFound)) => {
            return Ok(None);
        }
        Err(error) => return Err(ProfileExportTargetCheckError::Open(error)),
    };
    let original_metadata = opened.metadata().map_err(|error| {
        ProfileExportTargetCheckError::Read(ProfileFileReadError::FileIo(
            error.kind(),
        ))
    })?;
    Ok(Some(ProfileExportTargetProbe {
        artifact,
        maximum_bytes,
        opened,
        original_metadata,
        selected_cwd,
        selected_path: intent.path(),
    }))
}

// The file reader checked its own descriptor before returning bytes; this
// second observation checks that the *selected pathname* still resolves to the
// same file and that its observed metadata was not rewritten after opening.
// Neither observation locks an external pathname or proves a disk snapshot.
fn same_selected_file_metadata(before: &Metadata, after: &Metadata) -> bool {
    if !before.is_file() || !after.is_file() || before.len() != after.len() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;

        before.dev() == after.dev()
            && before.ino() == after.ino()
            && before.mtime() == after.mtime()
            && before.mtime_nsec() == after.mtime_nsec()
            && before.ctime() == after.ctime()
            && before.ctime_nsec() == after.ctime_nsec()
    }
    #[cfg(not(unix))]
    {
        before.modified().is_ok() && before.modified() == after.modified()
    }
}

/// Inspect whether a selected file still represents exactly one sealed ZIP.
///
/// The selected path must already have trusted host origin. No path string
/// or source label authenticates that selection on its own. Open uses the
/// secure no-symlink platform policy, and reading checks exact length/EOF.
/// An identical-byte replacement *before opening* can still match because no
/// prior inode is retained here. A replacement after opening and before the
/// final pathname check refuses as drift or unsafe rebinding. This does not
/// prove a historic commit, stable
/// snapshot across concurrent writes, session revision or retry identity.
/// A higher-level application must bind those independently before recovering
/// any previous receipt. This function never writes or returns Exported.
///
/// # Errors
///
/// Refuses non-ExportProfile intents before I/O, oversized expected artifacts,
/// unsupported or unsafe paths, and untrustworthy concurrent read outcomes.
pub fn check_profile_export_target(
    intent: &ExplicitFilePathIntent,
    artifact: &ProfileExportArtifact,
    maximum_bytes: usize,
) -> Result<ProfileExportTargetStatus, ProfileExportTargetCheckError> {
    open_profile_export_target_probe(intent, artifact, maximum_bytes)?.map_or(
        Ok(ProfileExportTargetStatus::Missing),
        ProfileExportTargetProbe::verify,
    )
}
