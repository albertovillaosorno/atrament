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
//   - Compile directly against the media-job session application service.
// - Defaults:
//   - A fresh service contains no jobs or waveform intermediates.
//
use atrament_media_job_lifecycle::{MediaJobCleanupStatus, MediaJobOutcome};

#[allow(dead_code)]
#[path = "../../../../src/backend/media-job-session/application/lib.rs"]
mod application;

use application::{MediaJobSessionError, MediaJobSessionService};

#[test]
fn every_terminal_outcome_requires_registered_waveform_cleanup() {
    for outcome in [
        MediaJobOutcome::Cancelled,
        MediaJobOutcome::Failed,
        MediaJobOutcome::Succeeded,
    ] {
        let mut session = MediaJobSessionService::new();
        let job = session.begin_job().expect("job identity");
        let intermediate = session
            .register_waveform_intermediate(job)
            .expect("waveform identity");
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
    let job = session.begin_job().expect("job identity");
    let intermediate = session
        .register_waveform_intermediate(job)
        .expect("waveform identity");
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
    let first_job = session.begin_job().expect("first job identity");
    let first_intermediate = session
        .register_waveform_intermediate(first_job)
        .expect("first waveform identity");
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

    let second_job = session.begin_job().expect("second job identity");
    let second_intermediate = session
        .register_waveform_intermediate(second_job)
        .expect("second waveform identity");
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
    let job = session.begin_job().expect("job identity");
    assert_eq!(
        session.cleanup_status(job),
        Err(MediaJobSessionError::NotTerminal),
    );
    assert_eq!(
        session.finish_job(job, MediaJobOutcome::Succeeded),
        Ok(MediaJobCleanupStatus::Settled),
    );

    let another_job = session.begin_job().expect("second job identity");
    let another_intermediate = session
        .register_waveform_intermediate(another_job)
        .expect("second waveform identity");
    assert_eq!(
        session.record_cleanup_failure(job, another_intermediate),
        Err(MediaJobSessionError::WaveformNotRegistered),
    );
}

#[test]
fn dropping_service_leaves_a_fresh_service_with_no_job_authority() {
    let old_job = {
        let mut session = MediaJobSessionService::new();
        let job = session.begin_job().expect("job identity");
        let _intermediate = session
            .register_waveform_intermediate(job)
            .expect("waveform identity");
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
