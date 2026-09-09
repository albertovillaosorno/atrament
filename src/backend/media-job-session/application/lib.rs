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
//   - Active-process media-job identities, one bounded waveform intermediate
//     identity per job, and terminal cleanup state.
// - Must-Not:
//   - Create/delete files, choose temporary paths, decode media, invoke
//     transcription engines, schedule retries, persist recovery state, or own
//     notebook semantics.
// - Allows:
//   - Inputs: Terminal job outcomes plus adapter-reported cleanup success or
//     failure.
//   - Outputs: Opaque process-local identities and typed cleanup status.
//   - Side effects: Process-local mutation only.
// - Split-When:
//   - Temporary-storage or transcription adapters gain executable authority.
// - Merge-When:
//   - One broader session application owner composes this lifecycle service.
// - Summary:
//   - Owns ephemeral media cleanup bookkeeping for one disposable session.
// - Description:
//   - Prevents terminal media jobs from being reported settled while their
//     registered waveform intermediate still requires cleanup.
// - Usage:
//   - Compose once per active process and drop with the disposable session.
// - Defaults:
//   - Starts with no jobs or registered waveform intermediates.
//

//! Process-local application ownership for media-job cleanup bookkeeping.

use std::collections::BTreeMap;
use std::num::NonZeroU64;

pub use atrament_media_job_lifecycle::{
    MediaJobCleanupStatus, MediaJobOutcome,
};
use atrament_media_job_lifecycle::{
    MediaJobLifecycleError, OwnedWaveformIntermediate, TerminalMediaJob,
    WaveformCleanupState, media_job_cleanup_status,
};

/// Exhaustion of one process-local media lifecycle identity sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaJobIdentityExhausted {
    /// Media-job identity sequence exhausted.
    Job,
    /// Waveform-intermediate identity sequence exhausted.
    WaveformIntermediate,
}

/// Opaque active-process identity for one media ingestion/transcription job.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MediaJobIdentity(NonZeroU64);

/// Opaque active-process identity for one owned waveform intermediate.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct WaveformIntermediateIdentity(NonZeroU64);

type OwnedIntermediate = OwnedWaveformIntermediate<
    WaveformIntermediateIdentity,
    MediaJobIdentity,
>;

#[derive(Clone, Debug, Eq, PartialEq)]
struct MediaJobRecord {
    cleanup: WaveformCleanupState<OwnedIntermediate>,
    outcome: Option<MediaJobOutcome>,
}

/// Typed media lifecycle application failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MediaJobSessionError {
    /// A terminal job cannot be completed a second time or change outcome.
    AlreadyTerminal {
        /// Existing terminal outcome retained without mutation.
        outcome: MediaJobOutcome,
    },
    /// Process-local identity allocation exhausted before registration.
    IdentityExhausted(MediaJobIdentityExhausted),
    /// Internal cleanup evidence violates the media-job ownership invariant.
    Lifecycle(MediaJobLifecycleError<MediaJobIdentity>),
    /// Cleanup may only be recorded after the job reaches a terminal outcome.
    NotTerminal,
    /// Caller named a job that does not exist in this active process.
    UnknownJob,
    /// Caller named a different intermediate than the one owned by this job.
    WaveformMismatch {
        /// Intermediate identity currently owned by the job.
        expected: WaveformIntermediateIdentity,
    },
    /// This job has no registered waveform intermediate to clean.
    WaveformNotRegistered,
    /// A job already owns its one bounded waveform intermediate.
    WaveformRegistered {
        /// Existing intermediate retained without replacement.
        intermediate: WaveformIntermediateIdentity,
    },
}

/// In-memory media-job lifecycle authority for one disposable session.
#[derive(Debug)]
pub struct MediaJobSessionService {
    jobs: BTreeMap<MediaJobIdentity, MediaJobRecord>,
    next_intermediate: Option<NonZeroU64>,
    next_job: Option<NonZeroU64>,
}

impl Default for MediaJobSessionService {
    fn default() -> Self {
        Self::new()
    }
}

impl MediaJobSessionService {
    /// Begin one active-process media job without creating an intermediate.
    ///
    /// # Errors
    ///
    /// Returns [`MediaJobIdentityExhausted::Job`] after all process-local job
    /// identities have been consumed.
    pub fn begin_job(
        &mut self,
    ) -> Result<MediaJobIdentity, MediaJobIdentityExhausted> {
        let identity = MediaJobIdentity(allocate_next(
            &mut self.next_job,
            MediaJobIdentityExhausted::Job,
        )?);
        let prior = self.jobs.insert(
            identity,
            MediaJobRecord {
                cleanup: WaveformCleanupState::NoIntermediate,
                outcome: None,
            },
        );
        debug_assert!(
            prior.is_none(),
            "fresh media job identity must not collide",
        );
        Ok(identity)
    }

    /// Inspect cleanup status for one terminal job without mutation.
    ///
    /// # Errors
    ///
    /// Returns a typed failure for unknown or still-active jobs, or when
    /// internal cleanup ownership evidence is inconsistent.
    pub fn cleanup_status(
        &self,
        job: MediaJobIdentity,
    ) -> Result<MediaJobCleanupStatus, MediaJobSessionError> {
        let record = self
            .jobs
            .get(&job)
            .ok_or(MediaJobSessionError::UnknownJob)?;
        terminal_cleanup_status(job, record)
    }

    /// Mark an active job terminal while preserving cleanup requirements.
    ///
    /// # Errors
    ///
    /// Returns a typed failure for an unknown job, a repeated terminal
    /// transition, or inconsistent internal cleanup ownership evidence.
    pub fn finish_job(
        &mut self,
        job: MediaJobIdentity,
        outcome: MediaJobOutcome,
    ) -> Result<MediaJobCleanupStatus, MediaJobSessionError> {
        let record = self
            .jobs
            .get_mut(&job)
            .ok_or(MediaJobSessionError::UnknownJob)?;
        if let Some(existing) = record.outcome {
            return Err(MediaJobSessionError::AlreadyTerminal {
                outcome: existing,
            });
        }
        record.outcome = Some(outcome);
        terminal_cleanup_status(job, record)
    }

    /// Whether this active-process owner has no registered media jobs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    /// Construct an empty service for one fresh active process.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            jobs: BTreeMap::new(),
            next_intermediate: Some(NonZeroU64::MIN),
            next_job: Some(NonZeroU64::MIN),
        }
    }

    /// Record a failed cleanup attempt while retaining retry-required evidence.
    ///
    /// # Errors
    ///
    /// Returns a typed failure when the job is unknown or active, no waveform
    /// is
    /// registered, or the caller names a different intermediate.
    pub fn record_cleanup_failure(
        &mut self,
        job: MediaJobIdentity,
        intermediate: WaveformIntermediateIdentity,
    ) -> Result<MediaJobCleanupStatus, MediaJobSessionError> {
        let record = self
            .jobs
            .get_mut(&job)
            .ok_or(MediaJobSessionError::UnknownJob)?;
        ensure_terminal(record)?;
        let owned = registered_intermediate(record, intermediate)?.clone();
        record.cleanup = WaveformCleanupState::RetryRequired(owned);
        terminal_cleanup_status(job, record)
    }

    /// Record successful cleanup after the owned intermediate is gone.
    ///
    /// # Errors
    ///
    /// Returns a typed failure when the job is unknown or active, no waveform
    /// is
    /// registered, or the caller names a different intermediate.
    pub fn record_cleanup_success(
        &mut self,
        job: MediaJobIdentity,
        intermediate: WaveformIntermediateIdentity,
    ) -> Result<MediaJobCleanupStatus, MediaJobSessionError> {
        let record = self
            .jobs
            .get_mut(&job)
            .ok_or(MediaJobSessionError::UnknownJob)?;
        ensure_terminal(record)?;
        let _owned = registered_intermediate(record, intermediate)?;
        record.cleanup = WaveformCleanupState::NoIntermediate;
        terminal_cleanup_status(job, record)
    }

    /// Register the job's one bounded waveform intermediate in process memory.
    ///
    /// This records ownership only. It does not create a file or choose a path.
    ///
    /// # Errors
    ///
    /// Returns a typed failure for an unknown or terminal job, when an
    /// intermediate is already registered, or when the process-local
    /// intermediate identity sequence is exhausted.
    pub fn register_waveform_intermediate(
        &mut self,
        job: MediaJobIdentity,
    ) -> Result<WaveformIntermediateIdentity, MediaJobSessionError> {
        let record = self
            .jobs
            .get_mut(&job)
            .ok_or(MediaJobSessionError::UnknownJob)?;
        if let Some(outcome) = record.outcome {
            return Err(MediaJobSessionError::AlreadyTerminal { outcome });
        }
        if let Some(existing) = cleanup_intermediate(&record.cleanup) {
            return Err(MediaJobSessionError::WaveformRegistered {
                intermediate: existing.intermediate_identity,
            });
        }
        let intermediate = WaveformIntermediateIdentity(
            allocate_next(
                &mut self.next_intermediate,
                MediaJobIdentityExhausted::WaveformIntermediate,
            )
            .map_err(MediaJobSessionError::IdentityExhausted)?,
        );
        record.cleanup =
            WaveformCleanupState::Pending(OwnedWaveformIntermediate {
                intermediate_identity: intermediate,
                job_identity: job,
            });
        Ok(intermediate)
    }

}

fn allocate_next(
    sequence: &mut Option<NonZeroU64>,
    exhausted: MediaJobIdentityExhausted,
) -> Result<NonZeroU64, MediaJobIdentityExhausted> {
    let Some(current) = *sequence else {
        return Err(exhausted);
    };
    *sequence = current.get().checked_add(1).and_then(NonZeroU64::new);
    Ok(current)
}

const fn cleanup_intermediate(
    cleanup: &WaveformCleanupState<OwnedIntermediate>,
) -> Option<&OwnedIntermediate> {
    match cleanup {
        WaveformCleanupState::NoIntermediate => None,
        WaveformCleanupState::Pending(intermediate)
        | WaveformCleanupState::RetryRequired(intermediate) => {
            Some(intermediate)
        },
    }
}

const fn ensure_terminal(
    record: &MediaJobRecord,
) -> Result<(), MediaJobSessionError> {
    if record.outcome.is_some() {
        return Ok(());
    }
    Err(MediaJobSessionError::NotTerminal)
}

fn registered_intermediate(
    record: &MediaJobRecord,
    requested: WaveformIntermediateIdentity,
) -> Result<&OwnedIntermediate, MediaJobSessionError> {
    let Some(owned) = cleanup_intermediate(&record.cleanup) else {
        return Err(MediaJobSessionError::WaveformNotRegistered);
    };
    if owned.intermediate_identity == requested {
        return Ok(owned);
    }
    Err(MediaJobSessionError::WaveformMismatch {
        expected: owned.intermediate_identity,
    })
}

fn terminal_cleanup_status(
    job: MediaJobIdentity,
    record: &MediaJobRecord,
) -> Result<MediaJobCleanupStatus, MediaJobSessionError> {
    let Some(outcome) = record.outcome else {
        return Err(MediaJobSessionError::NotTerminal);
    };
    let terminal = TerminalMediaJob {
        cleanup: record.cleanup.clone(),
        job_identity: job,
        outcome,
    };
    media_job_cleanup_status(&terminal).map_err(MediaJobSessionError::Lifecycle)
}
