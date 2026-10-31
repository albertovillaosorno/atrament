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
//   - Conservative read-only adjudication of a same-key profile Export retry.
// - Must-Not:
//   - Record a commit, authenticate a caller, read or write a file, or issue
//     a receipt from retry-key or target equality alone.
// - Allows:
//   - Inputs: Prior session-owned effect evidence, verified request binding,
//     sealed artifact identity and independently checked target observation.
//   - Outputs: Distinct new, conflict, match, drift and uncertain decisions.
//   - Side effects: None.
// - Split-When:
//   - Trusted session orchestration owns file effects and retained receipts.
// - Merge-When:
//   - A full Export application composes these observations atomically.
// - Summary:
//   - Only known synced prior commits with exact current bytes can recover.
// - Description:
//   - Unknown durability and missing outcomes never masquerade as completion.
// - Usage:
//   - Trust the session-owned effect record and read the target independently.
// - Defaults:
//   - Positive classification is eligibility for receipt recovery only.
//

//! Pure same-retry evidence classification, never an Export receipt.

// Fixture-only manifest and ZIP encoders are used by integration tests.
#[cfg(test)]
use atrament_explicit_file_path_intent as _;
#[cfg(test)]
use atrament_handwriting_profile_container as _;
#[cfg(test)]
use atrament_profile_manifest_json_writer as _;
#[cfg(test)]
use atrament_profile_stored_zip_writer as _;
#[cfg(test)]
use atrament_semantic_notebook as _;

use atrament_profile_export_assembly::ProfileExportArtifact;
use atrament_profile_export_retry_binding::{
    ProfileRetryBindingDisposition, ProfileRetryBindingError,
};
use atrament_profile_export_target_check::{
    ProfileExportTargetCheckError, ProfileExportTargetStatus,
};
use atrament_profile_file_commit::ProfileFileCommitEvidence;

/// Session-owned evidence, never inferred from target content alone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileExportPriorEffect {
    /// The adapter crossed rename but did not prove directory durability.
    DurabilityUnknown,
    /// The adapter proved the original attempt crossed no file boundary.
    KnownNoCommit,
    /// No trusted outcome observation has been saved for the bound retry key.
    NotRecorded,
    /// The adapter returned success after syncing the selected directory.
    SyncedCommit(ProfileFileCommitEvidence),
}
/// A conservative decision; none of these variants writes or issues a receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileExportRetryRecoveryDecision {
    /// Repeated request conflicts with the key's first normalized request.
    BindingRefused(ProfileRetryBindingError),
    /// Previous complete output is not at the selected target anymore.
    ExternalTargetDrift,
    /// New bindings are initial requests, not evidence of earlier effects.
    FreshBinding,
    /// Caller-supplied commit or target facts disagree with the sealed ZIP.
    InconsistentEvidence,
    /// A prior synced commit still has exact bytes and matching file identity.
    PriorCommitBytesVerified,
    /// Renamed output might not have been durably committed.
    PriorDurabilityUnknown,
    /// An identical request is known to have performed no file commit.
    PriorKnownNoCommit,
    /// An identical key is present but has no trusted original outcome.
    PriorOutcomeNotRecorded,
    /// Target could not be inspected with the platform safety guarantees.
    TargetInspectionFailed(ProfileExportTargetCheckError),
    /// A synced prior commit needs a fresh read-only target observation.
    TargetInspectionRequired,
}
/// Classify a repeated profile Export without modifying the target.
///
/// Caller authority, current revision and accepted artifact origin must have
/// been established independently. The session must retain an authentic
/// result from its earlier file-commit call; the target checker must inspect
/// the selected path after that result. Byte equality alone cannot recover a
/// receipt after another file replaces the original committed inode. A
/// previous uncertain durability result cannot be upgraded to success merely
/// by reading the target.
///
/// `PriorCommitBytesVerified` permits a trusted Export application to consult
/// its saved receipt; it is not itself proof of that receipt or a durable
/// snapshot of the target against concurrent external changes.
#[must_use]
pub fn assess_profile_export_retry_recovery(
    binding: Result<ProfileRetryBindingDisposition, ProfileRetryBindingError>,
    prior_effect: ProfileExportPriorEffect,
    target: Option<
        Result<ProfileExportTargetStatus, ProfileExportTargetCheckError>,
    >,
    artifact: &ProfileExportArtifact,
) -> ProfileExportRetryRecoveryDecision {
    match binding {
        Err(reason) => {
            return ProfileExportRetryRecoveryDecision::BindingRefused(reason);
        }
        Ok(ProfileRetryBindingDisposition::NewBinding) => {
            return ProfileExportRetryRecoveryDecision::FreshBinding;
        }
        Ok(ProfileRetryBindingDisposition::SameBinding) => {}
    }
    let commit = match prior_effect {
        ProfileExportPriorEffect::NotRecorded => {
            return ProfileExportRetryRecoveryDecision::PriorOutcomeNotRecorded;
        }
        ProfileExportPriorEffect::KnownNoCommit => {
            return ProfileExportRetryRecoveryDecision::PriorKnownNoCommit;
        }
        ProfileExportPriorEffect::DurabilityUnknown => {
            return ProfileExportRetryRecoveryDecision::PriorDurabilityUnknown;
        }
        ProfileExportPriorEffect::SyncedCommit(commit) => commit,
    };
    if commit.byte_length != artifact.bytes().len() {
        return ProfileExportRetryRecoveryDecision::InconsistentEvidence;
    }
    match target {
        None => ProfileExportRetryRecoveryDecision::TargetInspectionRequired,
        Some(Err(reason)) => {
            ProfileExportRetryRecoveryDecision::TargetInspectionFailed(reason)
        }
        Some(Ok(
            ProfileExportTargetStatus::Missing
            | ProfileExportTargetStatus::Drifted
        )) => {
            ProfileExportRetryRecoveryDecision::ExternalTargetDrift
        }
        Some(Ok(ProfileExportTargetStatus::Matches {
            archive_sha256,
            byte_length,
            file_device,
            file_inode,
        })) => {
            if archive_sha256 != artifact.archive_sha256()
                || byte_length != artifact.bytes().len()
            {
                ProfileExportRetryRecoveryDecision::InconsistentEvidence
            } else if file_device != commit.file_device
                || file_inode != commit.file_inode
            {
                ProfileExportRetryRecoveryDecision::ExternalTargetDrift
            } else {
                ProfileExportRetryRecoveryDecision::PriorCommitBytesVerified
            }
        }
    }
}
