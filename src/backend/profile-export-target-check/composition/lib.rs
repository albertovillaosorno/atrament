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
//   - The opened descriptor is symlink-free and all reads are size bounded.
// - Usage:
//   - Use only with previously recorded artifact identity and retry state.
// - Defaults:
//   - A matching file is evidence, never permission to return Exported.
//

//! Read-only selected Export target comparison; no commit or retry authority.

use std::io;

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
};

/// Observed target bytes at this instant, not a prior Export receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileExportTargetStatus {
    /// File was read completely but is not the requested artifact.
    Drifted,
    /// Selected target holds the exact complete artifact bytes.
    Matches {
        /// SHA-256 supplied by the already-verified artifact assembly.
        archive_sha256: Sha256Digest,
        /// Exact number of matched bytes.
        byte_length: usize,
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

/// Inspect whether a selected file still represents exactly one sealed ZIP.
///
/// The selected path must already have trusted host origin. No path string
/// or source label authenticates that selection on its own. Open uses the
/// secure no-symlink platform policy, and reading checks exact length/EOF.
/// Equal byte sequences count as matches even if an external writer replaced
/// the file with identical bytes; this cannot prove a historic commit, stable
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
    if intent.purpose() != ExplicitFilePurpose::ExportProfile {
        return Err(ProfileExportTargetCheckError::WrongPathPurpose);
    }
    if artifact.bytes().len() > maximum_bytes {
        return Err(ProfileExportTargetCheckError::ArtifactTooLarge);
    }
    let opened = match open_selected_profile_file(intent.path()) {
        Ok(opened) => opened,
        Err(ProfileSelectedFileOpenError::FileIo(io::ErrorKind::NotFound)) => {
            return Ok(ProfileExportTargetStatus::Missing);
        }
        Err(error) => return Err(ProfileExportTargetCheckError::Open(error)),
    };
    let source = match read_bounded_profile_file(opened, maximum_bytes) {
        Ok(source) => source,
        // The expected complete artifact is within the cap. Any source
        // exceeding that cap is therefore provably a different file.
        Err(ProfileFileReadError::FileTooLarge) => {
            return Ok(ProfileExportTargetStatus::Drifted);
        }
        Err(error) => return Err(ProfileExportTargetCheckError::Read(error)),
    };
    if source.bytes() != artifact.bytes() {
        return Ok(ProfileExportTargetStatus::Drifted);
    }
    Ok(ProfileExportTargetStatus::Matches {
        archive_sha256: artifact.archive_sha256(),
        byte_length: artifact.bytes().len(),
    })
}
