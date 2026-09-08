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
//   - Terminal media-job cleanup state and intermediate ownership invariants.
// - Must-Not:
//   - Create/delete files, choose temporary paths, decode media, invoke
//     engines,
//     schedule retries, choose retry limits, or persist recovery state.
// - Allows:
//   - Inputs: Terminal job outcome plus opaque job/intermediate identities.
//   - Outputs: Settled, cleanup-required, retry-required, or ownership error.
//   - Side effects: None.
// - Split-When:
//   - Temporary-storage adapters or retry scheduling gain executable authority.
// - Merge-When:
//   - Cleanup state becomes inseparable from one ingestion application service.
// - Summary:
//   - Prevents terminal jobs from hiding retained waveform intermediates.
// - Description:
//   - Applies the same cleanup requirement after success, cancellation, and
//     failure.
// - Usage:
//   - Gate terminal ingestion receipts before reporting cleanup as complete.
// - Defaults:
//   - Retry policy and filesystem behavior remain caller/application owned.
//

//! Terminal media-job cleanup state independent of filesystem implementation.

/// Accepted terminal outcomes for one media ingestion/transcription job.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MediaJobOutcome {
    /// User or application cancellation reached a terminal job state.
    Cancelled,
    /// A handled decoder/transcription/application failure reached terminal
    /// state.
    Failed,
    /// Media processing completed successfully.
    Succeeded,
}

/// Opaque temporary waveform intermediate bound to its owning media job.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedWaveformIntermediate<IntermediateIdentity, JobIdentity> {
    /// Opaque intermediate identity; not a filesystem path contract.
    pub intermediate_identity: IntermediateIdentity,
    /// Job identity that exclusively owns this intermediate.
    pub job_identity: JobIdentity,
}

/// Cleanup evidence associated with a terminal media job.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WaveformCleanupState<Intermediate> {
    /// No waveform intermediate remains owned by the terminal job.
    NoIntermediate,
    /// An owned intermediate still requires cleanup.
    Pending(Intermediate),
    /// A prior cleanup attempt failed and must be retried explicitly.
    RetryRequired(Intermediate),
}

/// One terminal media job before cleanup settlement is projected.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalMediaJob<JobIdentity, Intermediate> {
    /// Cleanup evidence for the job's owned intermediate, if any.
    pub cleanup: WaveformCleanupState<Intermediate>,
    /// Unique ingestion/transcription job identity.
    pub job_identity: JobIdentity,
    /// Success, cancellation, or handled failure.
    pub outcome: MediaJobOutcome,
}

/// Cleanup status that may be reported for one terminal media job.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaJobCleanupStatus {
    /// Cleanup has not completed because an owned intermediate remains.
    CleanupRequired,
    /// Cleanup failed previously and explicit retry remains required.
    CleanupRetryRequired,
    /// No intermediate remains and the terminal job is fully settled.
    Settled,
}

/// Why terminal cleanup evidence is not owned by the job being settled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MediaJobLifecycleError<JobIdentity> {
    /// The retained waveform intermediate belongs to another job identity.
    ForeignIntermediate {
        /// Job identity recorded on the foreign intermediate.
        intermediate_job_identity: JobIdentity,
    },
}

/// Project terminal cleanup status without performing filesystem cleanup.
///
/// # Errors
///
/// Returns `ForeignIntermediate` when retained cleanup evidence belongs to a
/// different job identity.
pub fn media_job_cleanup_status<JobIdentity, IntermediateIdentity>(
    job: &TerminalMediaJob<
        JobIdentity,
        OwnedWaveformIntermediate<IntermediateIdentity, JobIdentity>,
    >,
) -> Result<MediaJobCleanupStatus, MediaJobLifecycleError<JobIdentity>>
where
    JobIdentity: Clone + PartialEq,
{
    match &job.cleanup {
        WaveformCleanupState::NoIntermediate => {
            Ok(MediaJobCleanupStatus::Settled)
        }
        WaveformCleanupState::Pending(intermediate) => {
            validate_intermediate_owner(&job.job_identity, intermediate)?;
            Ok(MediaJobCleanupStatus::CleanupRequired)
        }
        WaveformCleanupState::RetryRequired(intermediate) => {
            validate_intermediate_owner(&job.job_identity, intermediate)?;
            Ok(MediaJobCleanupStatus::CleanupRetryRequired)
        }
    }
}

fn validate_intermediate_owner<JobIdentity, IntermediateIdentity>(
    job_identity: &JobIdentity,
    intermediate: &OwnedWaveformIntermediate<IntermediateIdentity, JobIdentity>,
) -> Result<(), MediaJobLifecycleError<JobIdentity>>
where
    JobIdentity: Clone + PartialEq,
{
    if intermediate.job_identity == *job_identity {
        return Ok(());
    }
    Err(MediaJobLifecycleError::ForeignIntermediate {
        intermediate_job_identity: intermediate.job_identity.clone(),
    })
}
