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

//! Process-local media cleanup lifecycle regression tests.

use atrament_media_job_lifecycle::{
    MediaJobCleanupStatus, MediaJobLifecycleError, MediaJobOutcome,
    OwnedWaveformIntermediate, TerminalMediaJob, WaveformCleanupState,
    media_job_cleanup_status,
};

type Intermediate = OwnedWaveformIntermediate<&'static str, &'static str>;
type Job = TerminalMediaJob<&'static str, Intermediate>;

#[derive(Clone, Copy, Debug)]
enum FixtureCleanupKind {
    NoIntermediate,
    Pending,
    RetryRequired,
}

const fn owned_intermediate() -> Intermediate {
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

#[test]
fn all_terminal_cleanup_states_match_ownership_oracle() {
    let outcomes = [
        MediaJobOutcome::Cancelled,
        MediaJobOutcome::Failed,
        MediaJobOutcome::Succeeded,
    ];
    let mut cases = 0u8;
    let mut saw_cleanup_required = false;
    let mut saw_retry_required = false;
    let mut saw_settled = false;
    let mut saw_foreign = false;
    for outcome in outcomes {
        for cleanup_kind in [
            FixtureCleanupKind::NoIntermediate,
            FixtureCleanupKind::Pending,
            FixtureCleanupKind::RetryRequired,
        ] {
            for owner_matches in [false, true] {
                let intermediate = OwnedWaveformIntermediate {
                    intermediate_identity: "waveform-oracle",
                    job_identity: if owner_matches {
                        "job-oracle"
                    } else {
                        "job-foreign"
                    },
                };
                let cleanup = match cleanup_kind {
                    FixtureCleanupKind::NoIntermediate => {
                        WaveformCleanupState::NoIntermediate
                    },
                    FixtureCleanupKind::Pending => {
                        WaveformCleanupState::Pending(intermediate)
                    },
                    FixtureCleanupKind::RetryRequired => {
                        WaveformCleanupState::RetryRequired(intermediate)
                    },
                };
                let job = Job {
                    cleanup,
                    job_identity: "job-oracle",
                    outcome,
                };
                let expected = match (cleanup_kind, owner_matches) {
                    (FixtureCleanupKind::NoIntermediate, _) => {
                        saw_settled = true;
                        Ok(MediaJobCleanupStatus::Settled)
                    },
                    (FixtureCleanupKind::Pending, true) => {
                        saw_cleanup_required = true;
                        Ok(MediaJobCleanupStatus::CleanupRequired)
                    },
                    (FixtureCleanupKind::RetryRequired, true) => {
                        saw_retry_required = true;
                        Ok(MediaJobCleanupStatus::CleanupRetryRequired)
                    },
                    (
                        FixtureCleanupKind::Pending
                        | FixtureCleanupKind::RetryRequired,
                        false,
                    ) => {
                        saw_foreign = true;
                        Err(MediaJobLifecycleError::ForeignIntermediate {
                            intermediate_job_identity: "job-foreign",
                        })
                    },
                };
                assert_eq!(
                    media_job_cleanup_status(&job),
                    expected,
                    "outcome {outcome:?}, cleanup {cleanup_kind:?}, \
                     owner match {owner_matches}",
                );
                cases = cases.saturating_add(1);
            }
        }
    }
    assert_eq!(cases, 18);
    assert!(saw_cleanup_required);
    assert!(saw_retry_required);
    assert!(saw_settled);
    assert!(saw_foreign);
}

#[test]
fn noncopy_job_and_waveform_identities_preserve_foreign_ownership() {
    let current = String::from("job-current");
    let foreign = String::from("job-foreign");
    let opaque_waveform = vec![0u8, 255, 1, 128];
    let mut cases = 0u8;
    for outcome in [
        MediaJobOutcome::Cancelled,
        MediaJobOutcome::Failed,
        MediaJobOutcome::Succeeded,
    ] {
        for retry in [false, true] {
            for owned in [false, true] {
                let intermediate = OwnedWaveformIntermediate {
                    intermediate_identity: opaque_waveform.clone(),
                    job_identity: if owned {
                        current.clone()
                    } else {
                        foreign.clone()
                    },
                };
                let cleanup = if retry {
                    WaveformCleanupState::RetryRequired(intermediate)
                } else {
                    WaveformCleanupState::Pending(intermediate)
                };
                let job = TerminalMediaJob {
                    cleanup,
                    job_identity: current.clone(),
                    outcome,
                };
                let snapshot = job.clone();
                let expected = if !owned {
                    Err(MediaJobLifecycleError::ForeignIntermediate {
                        intermediate_job_identity: foreign.clone(),
                    })
                } else if retry {
                    Ok(MediaJobCleanupStatus::CleanupRetryRequired)
                } else {
                    Ok(MediaJobCleanupStatus::CleanupRequired)
                };
                assert_eq!(media_job_cleanup_status(&job), expected);
                assert_eq!(job, snapshot);
                cases = cases.saturating_add(1);
            }
        }
    }
    assert_eq!(cases, 12);
}
