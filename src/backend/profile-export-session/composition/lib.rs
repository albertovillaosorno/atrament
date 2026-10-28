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
//   - Process-lifetime profile Export attempt, file outcome and receipt state.
// - Must-Not:
//   - Authenticate caller gestures, select paths or profile revisions, persist
//     session records, encode handwriting sections or retry writes.
// - Allows:
//   - Inputs: Host-authenticated intent, revision and sealed ZIP.
//   - Outputs: One synced Export receipt or typed refusal/recovery outcome.
//   - Side effects: Exactly one attempted file commit for a fresh retry key;
//     read-only target inspection for a previously synced retry key.
// - Split-When:
//   - The trusted runtime admits host selection and accepted profile state.
// - Merge-When:
//   - Full Export command ownership incorporates this session journal.
// - Summary:
//   - Saved synced receipts survive same-session retries only with exact bytes.
// - Description:
//   - Conflict, cancellation and uncertain durability never silently replay.
// - Usage:
//   - Invoke only after independent host, revision and artifact admission.
// - Defaults:
//   - Drop destroys all attempt and receipt state; no auto persistence.
//

//! Session-scoped profile Export effects and retained, recoverable receipts.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use atrament_explicit_file_path_intent::{
    ExplicitFilePathIntent, ExplicitFilePathSource, ExplicitOverwriteIntent,
};
use atrament_handwriting_profile_container::Sha256Digest;
use atrament_profile_export_assembly::ProfileExportArtifact;
use atrament_profile_export_retry_binding::{
    ProfileExportRetryBindings, ProfileRetryBindingDisposition,
    ProfileRetryBindingError,
};
use atrament_profile_export_retry_recovery::{
    ProfileExportPriorEffect, ProfileExportRetryRecoveryDecision,
    assess_profile_export_retry_recovery,
};
use atrament_profile_export_target_check::check_profile_export_target;
use atrament_profile_file_commit::{
    ProfileFileCommitDisposition, ProfileFileCommitError,
    ProfileFileCommitEvidence, commit_profile_file_cancellable,
};
#[cfg(test)]
use atrament_profile_manifest_json_writer as _;
#[cfg(test)]
use atrament_profile_stored_zip_writer as _;
use atrament_semantic_notebook::RevisionIdentity;

/// A receipt for an actually successful, directory-synced file commit.
///
/// This is retained only for the owning disposable session. It describes the
/// successful effect, not a guarantee that an external file still exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileExportReceipt {
    /// SHA-256 of the exact complete ZIP committed.
    archive_sha256: Sha256Digest,
    /// Exact output length for the original committed artifact.
    byte_length: usize,
    /// Path selected by an independently authenticated host operation.
    path: PathBuf,
    /// Accepted revision bound to the original attempt.
    revision: RevisionIdentity,
    /// SHA-256 of exact canonical manifest JSON.
    root_sha256: Sha256Digest,
    /// Host-classified source (not, by itself, evidence of selection).
    source: ExplicitFilePathSource,
}

impl ProfileExportReceipt {
    /// SHA-256 of the original synchronized complete ZIP.
    #[must_use]
    pub const fn archive_sha256(&self) -> Sha256Digest {
        self.archive_sha256
    }

    /// Exact byte count returned by the successful file transport.
    #[must_use]
    pub const fn byte_length(&self) -> usize {
        self.byte_length
    }

    /// Exact selected destination originally committed.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Accepted session revision recorded for the original commit.
    #[must_use]
    pub const fn revision(&self) -> RevisionIdentity {
        self.revision
    }

    /// SHA-256 of the original canonical manifest JSON.
    #[must_use]
    pub const fn root_sha256(&self) -> Sha256Digest {
        self.root_sha256
    }

    /// Classified origin supplied by the authenticated host caller.
    #[must_use]
    pub const fn source(&self) -> ExplicitFilePathSource {
        self.source
    }
}

/// Proven result of a fresh attempt or read-only same-key recovery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProfileExportSessionOutcome {
    /// File transport returned an exact, synced commit for this request.
    Committed(ProfileExportReceipt),
    /// Saved receipt rechecked against the current exact target bytes.
    Recovered(ProfileExportReceipt),
}

/// Refusal without retrying a previous file effect.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileExportSessionError {
    /// Invalid key, conflicting revision, path, overwrite or artifact identity.
    Binding(ProfileRetryBindingError),
    /// First attempt refused or lost commit durability evidence.
    Commit(ProfileFileCommitError),
    /// Session journal and binding map disagree; never guess an outcome.
    JournalInconsistent,
    /// A same-key retry cannot recover the earlier synced receipt.
    Retry(ProfileExportRetryRecoveryDecision),
}

#[derive(Clone, Debug)]
struct ProfileExportAttempt {
    effect: ProfileExportPriorEffect,
    receipt: Option<ProfileExportReceipt>,
}

/// Bounded, disposable record of exactly one attempted file effect per key.
///
/// `&mut self` serializes admission within a single session owner. All call
/// sites MUST independently establish genuine selected-path provenance,
/// accepted revision and sealed artifact origin; a classified path and a
/// `RevisionIdentity` argument alone do not authenticate either. A new key
/// admits one file commit; an existing key never calls the writer again.
/// Unknown post-rename durability is retained as uncertain, not repaired by
/// target equality. Only the original successful writer result creates a
/// receipt, and replay requires an independent read-only exact-byte check.
#[derive(Debug)]
pub struct ProfileExportSession {
    attempts: BTreeMap<String, ProfileExportAttempt>,
    bindings: ProfileExportRetryBindings,
}

impl ProfileExportSession {
    /// Attempt a selected Export once, or recover its saved synced receipt.
    ///
    /// Never call this entrypoint with a path or revision supplied only by
    /// clipboard, model, notebook, diagnostic or unverified session text.
    /// The trusted runtime must authenticate caller-selected path/overwrite
    /// provenance and current accepted revision **before** calling it. No
    /// implicit Export, caller authentication or session import is performed.
    /// Subsequent same-key requests inspect the selected file but NEVER write
    /// again, even if the original attempt failed before commit.
    ///
    /// # Errors
    ///
    /// Refuses conflicting binding, first-attempt file failure, uncertain
    /// durability, external drift, unsupported target checks and missing
    /// original effect evidence rather than returning an invented receipt.
    pub fn export(
        &mut self,
        retry_key: &str,
        revision: RevisionIdentity,
        intent: &ExplicitFilePathIntent,
        artifact: &ProfileExportArtifact,
        maximum_bytes: usize,
    ) -> Result<ProfileExportSessionOutcome, ProfileExportSessionError> {
        self.export_cancellable(
            retry_key,
            revision,
            intent,
            artifact,
            maximum_bytes,
            || false,
        )
    }

    /// Export with cooperative cancellation checked only before first commit.
    ///
    /// A repeated key cannot retry a cancelled, failed or unwound attempt.
    /// Its cancellation callback is not invoked on a same-key retry.
    /// A new authorized attempt needs a distinct key.
    ///
    /// # Errors
    ///
    /// Returns typed binding, file-commit or retry refusal without conflating
    /// precommit cancellation and unknown post-rename durability.
    pub fn export_cancellable<Cancel>(
        &mut self,
        retry_key: &str,
        revision: RevisionIdentity,
        intent: &ExplicitFilePathIntent,
        artifact: &ProfileExportArtifact,
        maximum_bytes: usize,
        is_cancelled: Cancel,
    ) -> Result<ProfileExportSessionOutcome, ProfileExportSessionError>
    where
        Cancel: FnMut() -> bool,
    {
        let binding = self
            .bindings
            .bind(retry_key, revision, intent, artifact)
            .map_err(ProfileExportSessionError::Binding)?;
        match binding {
            ProfileRetryBindingDisposition::NewBinding => {
                let overwrite = match intent.overwrite() {
                    ExplicitOverwriteIntent::PreserveExisting => {
                        ProfileFileCommitDisposition::PreserveExisting
                    }
                    ExplicitOverwriteIntent::ReplaceExisting => {
                        ProfileFileCommitDisposition::ReplaceExisting
                    }
                };
                self.reserve_attempt(retry_key)?;
                let result = commit_profile_file_cancellable(
                    intent.path(),
                    artifact.bytes(),
                    maximum_bytes,
                    overwrite,
                    is_cancelled,
                );
                let (effect, receipt) = match result {
                    Ok(ProfileFileCommitEvidence { byte_length }) => {
                        let receipt = ProfileExportReceipt {
                            archive_sha256: artifact.archive_sha256(),
                            byte_length,
                            path: intent.path().to_path_buf(),
                            revision,
                            root_sha256: artifact.root_sha256(),
                            source: intent.source(),
                        };
                        let evidence =
                            ProfileFileCommitEvidence { byte_length };
                        (
                            ProfileExportPriorEffect::SyncedCommit(evidence),
                            Some(receipt),
                        )
                    }
                    Err(
                        ProfileFileCommitError::CommittedDurabilityUnknown(_),
                    ) => (ProfileExportPriorEffect::DurabilityUnknown, None),
                    Err(_) => (ProfileExportPriorEffect::KnownNoCommit, None),
                };
                let attempt = self
                    .attempts
                    .get_mut(retry_key)
                    .ok_or(ProfileExportSessionError::JournalInconsistent)?;
                attempt.effect = effect;
                attempt.receipt.clone_from(&receipt);
                match result {
                    Ok(_) => receipt
                        .map(ProfileExportSessionOutcome::Committed)
                        .ok_or(ProfileExportSessionError::JournalInconsistent),
                    Err(error) => Err(ProfileExportSessionError::Commit(error)),
                }
            }
            ProfileRetryBindingDisposition::SameBinding => {
                let attempt = self
                    .attempts
                    .get(retry_key)
                    .ok_or(ProfileExportSessionError::JournalInconsistent)?;
                let target = if matches!(
                    attempt.effect,
                    ProfileExportPriorEffect::SyncedCommit(_)
                ) {
                    Some(check_profile_export_target(
                        intent,
                        artifact,
                        maximum_bytes,
                    ))
                } else {
                    None
                };
                let decision = assess_profile_export_retry_recovery(
                    Ok(binding),
                    attempt.effect,
                    target,
                    artifact,
                );
                if matches!(
                    decision,
                    ProfileExportRetryRecoveryDecision::PriorCommitBytesVerified
                ) {
                    return attempt
                        .receipt
                        .clone()
                        .map(ProfileExportSessionOutcome::Recovered)
                        .ok_or(ProfileExportSessionError::JournalInconsistent);
                }
                Err(ProfileExportSessionError::Retry(decision))
            }
        }
    }
    /// Whether the session has made no profile Export attempt.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.attempts.is_empty()
    }

    /// Count retained attempts, including refused and uncertain file effects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.attempts.len()
    }

    /// Create an empty in-memory journal with an exact upper bound on keys.
    #[must_use]
    pub const fn new(maximum_attempts: usize) -> Self {
        Self {
            attempts: BTreeMap::new(),
            bindings: ProfileExportRetryBindings::new(maximum_attempts),
        }
    }

    // Reserve the outcome slot BEFORE calling any host callback or I/O.
    // A callback panic leaves this attempt unknown, never retryable by key.
    fn reserve_attempt(
        &mut self,
        retry_key: &str,
    ) -> Result<(), ProfileExportSessionError> {
        if self.attempts.contains_key(retry_key) {
            return Err(ProfileExportSessionError::JournalInconsistent);
        }
        let _previous = self.attempts.insert(
            String::from(retry_key),
            ProfileExportAttempt {
                effect: ProfileExportPriorEffect::NotRecorded,
                receipt: None,
            },
        );
        Ok(())
    }
}
