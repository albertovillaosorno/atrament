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
//   - Process-only binding of one profile Export retry key to normalized
//     revision, exact selected target, intent and sealed artifact identity.
// - Must-Not:
//   - Authenticate path selection, authorize Export, write a file, retain a
//     receipt, infer completion or authorize replay from key equivalence.
// - Allows:
//   - Inputs: Caller retry key, selected path classification, accepted revision
//     identity and sealed profile ZIP evidence.
//   - Outputs: New/existing identical binding or bounded typed refusal.
//   - Side effects: Bounded process-lifetime memory only.
// - Split-When:
//   - Session-owned retry records need a separate lifecycle and receipt store.
// - Merge-When:
//   - Complete Export application owns retry admission and verified recovery.
// - Summary:
//   - Rejects identity reuse with different Export inputs before file effects.
// - Description:
//   - Paths and selected-origin classes remain byte-exact and never normalized.
// - Usage:
//   - Caller must admit Export and revision before inserting this binding.
// - Defaults:
//   - Same binding proves no prior effect; capacity is always caller-bounded.
//

//! Pure, bounded retry identity binding; no Export effect or receipt authority.

use std::collections::BTreeMap;
use std::path::PathBuf;

// Complete ZIP fixture dependencies are only needed by integration tests.
#[cfg(test)]
use atrament_profile_manifest_json_writer as _;
#[cfg(test)]
use atrament_profile_stored_zip_writer as _;

use atrament_explicit_file_path_intent::{
    ExplicitFilePathIntent, ExplicitFilePathSource, ExplicitFilePurpose,
    ExplicitOverwriteIntent,
};
use atrament_handwriting_profile_container::Sha256Digest;
use atrament_profile_export_assembly::ProfileExportArtifact;
use atrament_semantic_notebook::RevisionIdentity;

/// Maximum byte length of a caller-owned retry identity.
pub const MAXIMUM_PROFILE_RETRY_KEY_BYTES: usize = 128;
/// Maximum selected pathname byte length retained in a session retry record.
pub const MAXIMUM_PROFILE_RETRY_PATH_BYTES: usize = 4096;

/// Read-only classification; neither variant permits automatic Export replay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileRetryBindingDisposition {
    /// This retry key was first bound to the supplied exact inputs.
    NewBinding,
    /// The same retry key already names the same exact inputs.
    SameBinding,
}

/// Typed refusal before a retry identity can be used by an Export executor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileRetryBindingError {
    /// Another normalized request already owns this retry identity.
    ConflictingRequest,
    /// Empty, NUL-containing or unreasonably long retry identities refuse.
    InvalidRetryKey,
    /// This session's independently configured in-memory record cap was hit.
    JournalFull,
    /// The selected path exceeds the bounded retry identity memory policy.
    SelectedPathTooLong,
    /// The classified path is not for handwriting-profile Export.
    WrongPathPurpose,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NormalizedProfileExport {
    archive_sha256: Sha256Digest,
    byte_length: usize,
    overwrite: ExplicitOverwriteIntent,
    path: PathBuf,
    revision: RevisionIdentity,
    root_sha256: Sha256Digest,
    source: ExplicitFilePathSource,
}

/// Process-local, bounded identity map for one disposable Export session.
///
/// Drop the instance on session destruction. No record is persisted and no
/// state in this map can prove a file commit or authorize a retry response.
#[derive(Debug)]
pub struct ProfileExportRetryBindings {
    maximum_records: usize,
    records: BTreeMap<String, NormalizedProfileExport>,
}

impl ProfileExportRetryBindings {
    /// Bind one retry key before attempting a separately admitted file effect.
    ///
    /// The host must first authenticate the explicit caller selection or
    /// independently authorized host policy and validate the accepted revision
    /// and artifact origin. This operation only preserves the request identity
    /// within a disposable process; it does not classify a later file effect
    /// as committed, pending, safe to retry, recoverable, or cancelled.
    ///
    /// Both canonical ZIP and manifest digests, byte count, exact path bytes,
    /// selection source, revision and overwrite intent must all agree for a
    /// second use of the key to return `SameBinding`. This does not prove that
    /// a prior execution did anything: the executor still needs session-owned
    /// outcome evidence and an independent target drift check.
    ///
    /// # Errors
    ///
    /// Refuses wrong purposes, invalid or oversized keys or paths, reuse with
    /// changed inputs, and exhaustion without replacing a prior binding.
    pub fn bind(
        &mut self,
        retry_key: &str,
        revision: RevisionIdentity,
        intent: &ExplicitFilePathIntent,
        artifact: &ProfileExportArtifact,
    ) -> Result<ProfileRetryBindingDisposition, ProfileRetryBindingError> {
        if intent.purpose() != ExplicitFilePurpose::ExportProfile {
            return Err(ProfileRetryBindingError::WrongPathPurpose);
        }
        if retry_key.is_empty()
            || retry_key.len() > MAXIMUM_PROFILE_RETRY_KEY_BYTES
            || retry_key.as_bytes().contains(&0)
        {
            return Err(ProfileRetryBindingError::InvalidRetryKey);
        }
        if intent.path().as_os_str().as_encoded_bytes().len()
            > MAXIMUM_PROFILE_RETRY_PATH_BYTES
        {
            return Err(ProfileRetryBindingError::SelectedPathTooLong);
        }
        let normalized = NormalizedProfileExport {
            archive_sha256: artifact.archive_sha256(),
            byte_length: artifact.bytes().len(),
            overwrite: intent.overwrite(),
            path: intent.path().to_path_buf(),
            revision,
            root_sha256: artifact.root_sha256(),
            source: intent.source(),
        };
        if let Some(existing) = self.records.get(retry_key) {
            return if *existing == normalized {
                Ok(ProfileRetryBindingDisposition::SameBinding)
            } else {
                Err(ProfileRetryBindingError::ConflictingRequest)
            };
        }
        if self.records.len() >= self.maximum_records {
            return Err(ProfileRetryBindingError::JournalFull);
        }
        let _previous =
            self.records.insert(String::from(retry_key), normalized);
        Ok(ProfileRetryBindingDisposition::NewBinding)
    }

    /// Whether this map retains no retry identities.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Number of retained identities, without receipt or operation evidence.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Construct an empty map with a strict maximum number of retry keys.
    #[must_use]
    pub const fn new(maximum_records: usize) -> Self {
        Self {
            maximum_records,
            records: BTreeMap::new(),
        }
    }
}
