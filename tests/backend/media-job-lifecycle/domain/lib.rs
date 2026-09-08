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
//   - Regression evidence for terminal media-job cleanup and ownership.
// - Must-Not:
//   - Touch files, choose temp paths, decode media, call engines, schedule
//     retry,
//     or infer filesystem behavior.
// - Allows:
//   - Inputs: Deterministic terminal-job/intermediate identity fixtures.
//   - Outputs: Assertions over cleanup status and foreign ownership rejection.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Temporary-storage adapters or retry scheduling gain fixtures.
// - Merge-When:
//   - Cleanup evidence moves into an ingestion application harness.
// - Summary:
//   - Proves terminal outcomes cannot hide retained waveform intermediates.
// - Description:
//   - Covers success/cancel/failure, cleanup retry, and job-bound ownership.
// - Usage:
//   - Compile directly against the media-job-lifecycle domain.
// - Defaults:
//   - No retry count or storage behavior is invented.
//
use atrament_media_job_lifecycle::{
    MediaJobCleanupStatus, MediaJobLifecycleError, MediaJobOutcome,
    OwnedWaveformIntermediate, TerminalMediaJob, WaveformCleanupState,
    media_job_cleanup_status,
};

type Intermediate = OwnedWaveformIntermediate<&'static str, &'static str>;
type Job = TerminalMediaJob<&'static str, Intermediate>;

fn owned_intermediate() -> Intermediate {
    OwnedWaveformIntermediate {
        intermediate_identity: "waveform-7",
        job_identity: "job-4",
    }
}

#[test]
fn every_terminal_outcome_requires_cleanup_while_intermediate_remains() {
    for outcome in [
        MediaJobOutcome::Cancelled,
        MediaJobOutcome::Failed,
        MediaJobOutcome::Succeeded,
    ] {
        let job = Job {
            cleanup: WaveformCleanupState::Pending(owned_intermediate()),
            job_identity: "job-4",
            outcome,
        };
        assert_eq!(
            media_job_cleanup_status(&job),
            Ok(MediaJobCleanupStatus::CleanupRequired),
        );
    }
}

#[test]
fn cleanup_failure_remains_explicitly_retry_required() {
    let job = Job {
        cleanup: WaveformCleanupState::RetryRequired(owned_intermediate()),
        job_identity: "job-4",
        outcome: MediaJobOutcome::Failed,
    };
    assert_eq!(
        media_job_cleanup_status(&job),
        Ok(MediaJobCleanupStatus::CleanupRetryRequired),
    );
}

#[test]
fn terminal_job_is_settled_only_after_no_intermediate_remains() {
    let job = Job {
        cleanup: WaveformCleanupState::NoIntermediate,
        job_identity: "job-4",
        outcome: MediaJobOutcome::Succeeded,
    };
    assert_eq!(
        media_job_cleanup_status(&job),
        Ok(MediaJobCleanupStatus::Settled),
    );
}

#[test]
fn foreign_intermediate_rejects_instead_of_being_cleaned_by_wrong_job() {
    let job = Job {
        cleanup: WaveformCleanupState::Pending(OwnedWaveformIntermediate {
            intermediate_identity: "waveform-other",
            job_identity: "job-other",
        }),
        job_identity: "job-4",
        outcome: MediaJobOutcome::Cancelled,
    };
    assert_eq!(
        media_job_cleanup_status(&job),
        Err(MediaJobLifecycleError::ForeignIntermediate {
            intermediate_job_identity: "job-other",
        }),
    );
}
