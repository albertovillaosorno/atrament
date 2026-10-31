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
//   - Regression evidence for process-local media-job cleanup ownership.
// - Must-Not:
//   - Touch files, choose temporary paths, decode media, invoke engines,
//     schedule retries, or persist recovery state.
// - Allows:
//   - Inputs: In-memory job outcomes and cleanup attempt results.
//   - Outputs: Assertions over job/intermediate ownership and cleanup status.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Temporary-storage or transcription adapters gain fixtures.
// - Merge-When:
//   - Full session-process lifecycle evidence supersedes this service proof.
// - Summary:
//   - Proves media cleanup bookkeeping remains explicit and ephemeral.
// - Description:
//   - Covers terminal outcomes, retry-required cleanup, ownership mismatch,
//     repeated completion, and fresh-service isolation.
// - Usage:
//   - Run this Cargo harness against the public media-job service.
// - Defaults:
//   - A fresh service contains no jobs or waveform intermediates.
//

//! Public media-job-session regression tests.

use atrament_media_job_lifecycle::{MediaJobCleanupStatus, MediaJobOutcome};

use atrament_media_job_session as application;

use application::{MediaJobSessionError, MediaJobSessionService};

macro_rules! admitted {
    ($result:expr) => {{
        let result = $result;
        assert!(
            result.is_ok(),
            "fixture admission failed: {:?}",
            result.as_ref().err(),
        );
        let Ok(value) = result else {
            return;
        };
        value
    }};
}

#[test]
fn every_terminal_outcome_requires_registered_waveform_cleanup() {
    for outcome in [
        MediaJobOutcome::Cancelled,
        MediaJobOutcome::Failed,
        MediaJobOutcome::Succeeded,
    ] {
        let mut session = MediaJobSessionService::new();
        let job = admitted!(session.begin_job());
        let intermediate =
            admitted!(session.register_waveform_intermediate(job));
        assert_eq!(
            session.finish_job(job, outcome),
            Ok(MediaJobCleanupStatus::CleanupRequired),
        );
        assert_eq!(
            session.cleanup_status(job),
            Ok(MediaJobCleanupStatus::CleanupRequired),
        );
        assert_eq!(
            session.record_cleanup_success(job, intermediate),
            Ok(MediaJobCleanupStatus::Settled),
        );
        assert_eq!(
            session.cleanup_status(job),
            Ok(MediaJobCleanupStatus::Settled),
        );
    }
}

#[test]
fn failed_cleanup_remains_retry_required_until_success_is_recorded() {
    let mut session = MediaJobSessionService::new();
    let job = admitted!(session.begin_job());
    let intermediate = admitted!(session.register_waveform_intermediate(job));
    assert_eq!(
        session.finish_job(job, MediaJobOutcome::Failed),
        Ok(MediaJobCleanupStatus::CleanupRequired),
    );
    assert_eq!(
        session.record_cleanup_failure(job, intermediate),
        Ok(MediaJobCleanupStatus::CleanupRetryRequired),
    );
    assert_eq!(
        session.cleanup_status(job),
        Ok(MediaJobCleanupStatus::CleanupRetryRequired),
    );
    assert_eq!(
        session.record_cleanup_failure(job, intermediate),
        Ok(MediaJobCleanupStatus::CleanupRetryRequired),
    );
    assert_eq!(
        session.record_cleanup_success(job, intermediate),
        Ok(MediaJobCleanupStatus::Settled),
    );
}

#[test]
fn active_and_terminal_transitions_preserve_single_intermediate_ownership() {
    let mut session = MediaJobSessionService::new();
    let first_job = admitted!(session.begin_job());
    let first_intermediate =
        admitted!(session.register_waveform_intermediate(first_job));
    assert_eq!(
        session.register_waveform_intermediate(first_job),
        Err(MediaJobSessionError::WaveformRegistered {
            intermediate: first_intermediate,
        }),
    );
    assert_eq!(
        session.record_cleanup_success(first_job, first_intermediate),
        Err(MediaJobSessionError::NotTerminal),
    );

    let second_job = admitted!(session.begin_job());
    let second_intermediate =
        admitted!(session.register_waveform_intermediate(second_job));
    assert_eq!(
        session.finish_job(first_job, MediaJobOutcome::Succeeded),
        Ok(MediaJobCleanupStatus::CleanupRequired),
    );
    assert_eq!(
        session.record_cleanup_success(first_job, second_intermediate),
        Err(MediaJobSessionError::WaveformMismatch {
            expected: first_intermediate,
        }),
    );
    assert_eq!(
        session.finish_job(first_job, MediaJobOutcome::Failed),
        Err(MediaJobSessionError::AlreadyTerminal {
            outcome: MediaJobOutcome::Succeeded,
        }),
    );
    assert_eq!(
        session.register_waveform_intermediate(first_job),
        Err(MediaJobSessionError::AlreadyTerminal {
            outcome: MediaJobOutcome::Succeeded,
        }),
    );
}

#[test]
fn jobs_without_waveforms_settle_immediately_and_reject_cleanup_invention() {
    let mut session = MediaJobSessionService::new();
    let job = admitted!(session.begin_job());
    assert_eq!(
        session.cleanup_status(job),
        Err(MediaJobSessionError::NotTerminal),
    );
    assert_eq!(
        session.finish_job(job, MediaJobOutcome::Succeeded),
        Ok(MediaJobCleanupStatus::Settled),
    );

    let another_job = admitted!(session.begin_job());
    let another_intermediate =
        admitted!(session.register_waveform_intermediate(another_job));
    assert_eq!(
        session.record_cleanup_failure(job, another_intermediate),
        Err(MediaJobSessionError::WaveformNotRegistered),
    );
}

#[test]
fn dropping_service_leaves_a_fresh_service_with_no_job_authority() {
    let old_job = {
        let mut session = MediaJobSessionService::new();
        let job = admitted!(session.begin_job());
        let _intermediate =
            admitted!(session.register_waveform_intermediate(job));
        assert!(!session.is_empty(), "populated service must own job");
        job
    };

    let fresh = MediaJobSessionService::new();
    assert!(fresh.is_empty(), "fresh service must own no jobs");
    assert_eq!(
        fresh.cleanup_status(old_job),
        Err(MediaJobSessionError::UnknownJob),
    );
}

#[test]
fn settled_job_rejects_late_cleanup_events_without_recreating_work() {
    let mut session = MediaJobSessionService::new();
    let job = admitted!(session.begin_job());
    let intermediate = admitted!(session.register_waveform_intermediate(job));
    assert_eq!(
        session.finish_job(job, MediaJobOutcome::Succeeded),
        Ok(MediaJobCleanupStatus::CleanupRequired),
    );
    assert_eq!(
        session.record_cleanup_success(job, intermediate),
        Ok(MediaJobCleanupStatus::Settled),
    );

    for late in [
        session.record_cleanup_failure(job, intermediate),
        session.record_cleanup_success(job, intermediate),
    ] {
        assert_eq!(late, Err(MediaJobSessionError::WaveformNotRegistered));
        assert_eq!(
            session.cleanup_status(job),
            Ok(MediaJobCleanupStatus::Settled),
        );
    }
}

#[test]
fn many_interleaved_cleanup_jobs_never_accept_another_jobs_waveform() {
    let mut session = MediaJobSessionService::new();
    let mut jobs = Vec::new();
    for index in 0..128usize {
        let job = admitted!(session.begin_job());
        let waveform = if index.rem_euclid(4) == 0 {
            None
        } else {
            Some(admitted!(session.register_waveform_intermediate(job)))
        };
        let outcome = match index.rem_euclid(3) {
            0 => MediaJobOutcome::Cancelled,
            1 => MediaJobOutcome::Failed,
            _ => MediaJobOutcome::Succeeded,
        };
        let expected = if waveform.is_some() {
            MediaJobCleanupStatus::CleanupRequired
        } else {
            MediaJobCleanupStatus::Settled
        };
        assert_eq!(session.finish_job(job, outcome), Ok(expected));
        jobs.push((job, waveform));
    }

    for (index, (job, waveform)) in jobs.iter().copied().enumerate() {
        let expected = match (index.rem_euclid(4), waveform) {
            (0, None) | (3, Some(_)) => Some(MediaJobCleanupStatus::Settled),
            (1, Some(_)) => Some(MediaJobCleanupStatus::CleanupRequired),
            (2, Some(_)) => Some(MediaJobCleanupStatus::CleanupRetryRequired),
            _ => None,
        };
        assert!(expected.is_some(), "generated job shape changed");
        let Some(expected_status) = expected else {
            return;
        };
        if let Some(owned) = waveform {
            let observed = MediaJobSessionError::WaveformMismatch {
                expected: owned,
            };
            if index.rem_euclid(4) == 2 {
                assert_eq!(
                    session.record_cleanup_failure(job, owned),
                    Ok(MediaJobCleanupStatus::CleanupRetryRequired),
                );
            }
            let mut foreign_cases = 0u32;
            for (other_job, other_waveform) in jobs.iter().copied() {
                if other_job == job {
                    continue;
                }
                if let Some(foreign_waveform) = other_waveform {
                    assert_eq!(
                        session.record_cleanup_success(job, foreign_waveform),
                        Err(observed.clone()),
                    );
                    foreign_cases = foreign_cases.saturating_add(1);
                }
            }
            assert_eq!(foreign_cases, 95);
            let current_status = if index.rem_euclid(4) == 2 {
                MediaJobCleanupStatus::CleanupRetryRequired
            } else {
                MediaJobCleanupStatus::CleanupRequired
            };
            assert_eq!(session.cleanup_status(job), Ok(current_status));
            if index.rem_euclid(4) == 3 {
                assert_eq!(
                    session.record_cleanup_success(job, owned),
                    Ok(MediaJobCleanupStatus::Settled),
                );
            }
        }
        assert_eq!(session.cleanup_status(job), Ok(expected_status));
    }
    assert!(!session.is_empty());
    assert_eq!(jobs.len(), 128);
}

#[test]
fn all_terminal_session_cleanup_paths_match_state_machine() {
    let outcomes = [
        MediaJobOutcome::Cancelled,
        MediaJobOutcome::Failed,
        MediaJobOutcome::Succeeded,
    ];
    let mut cases = 0u8;
    for outcome in outcomes {
        for path in 0u8..4 {
            let mut session = MediaJobSessionService::new();
            let job = admitted!(session.begin_job());
            if path == 0 {
                assert_eq!(
                    session.finish_job(job, outcome),
                    Ok(MediaJobCleanupStatus::Settled),
                );
                assert_eq!(
                    session.cleanup_status(job),
                    Ok(MediaJobCleanupStatus::Settled),
                );
                cases = cases.saturating_add(1);
                continue;
            }
            let intermediate =
                admitted!(session.register_waveform_intermediate(job));
            assert_eq!(
                session.finish_job(job, outcome),
                Ok(MediaJobCleanupStatus::CleanupRequired),
            );
            assert_eq!(
                session.cleanup_status(job),
                Ok(MediaJobCleanupStatus::CleanupRequired),
            );
            if path >= 2 {
                assert_eq!(
                    session.record_cleanup_failure(job, intermediate),
                    Ok(MediaJobCleanupStatus::CleanupRetryRequired),
                );
                assert_eq!(
                    session.cleanup_status(job),
                    Ok(MediaJobCleanupStatus::CleanupRetryRequired),
                );
            }
            if path == 3 {
                assert_eq!(
                    session.record_cleanup_success(job, intermediate),
                    Ok(MediaJobCleanupStatus::Settled),
                );
                assert_eq!(
                    session.cleanup_status(job),
                    Ok(MediaJobCleanupStatus::Settled),
                );
            }
            cases = cases.saturating_add(1);
        }
    }
    assert_eq!(cases, 12);
}
