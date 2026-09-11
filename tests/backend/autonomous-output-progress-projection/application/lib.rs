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
//   - Exhaustive evidence for caller-requested output progress projection.
// - Must-Not:
//   - Execute output, infer caller intent, retry, mutate state, or run loops.
// - Allows:
//   - Inputs: All operation, result, and intent-source combinations.
//   - Outputs: Exact progress/recovery-evidence-or-none assertion for 99 cases.
//   - Side effects: None.
// - Split-When:
//   - Stateful requested-output aggregation needs independent evidence.
// - Merge-When:
//   - Coordinator tests fully subsume this finite projection oracle.
// - Summary:
//   - Proves only caller-requested successful output counts as new progress.
// - Description:
//   - Keeps recovered Export completion as replay recovery non-progress.
// - Usage:
//   - Compare all 99 combinations with an independent expected mapping.
// - Defaults:
//   - Host-policy, untrusted, failed, and invalid outputs stay unclassified.
//
use atrament_autonomous_output_intent::AutonomousOutputIntentSource;
use atrament_autonomous_output_progress_projection::
    autonomous_requested_output_progress_evidence;
use atrament_autonomous_progress_evidence::
    AutonomousProgressEvidenceClass as ProgressEvidence;
use atrament_derived_output_result::{
    DerivedOutputOperation, DerivedOutputResultClass,
};

const OPERATIONS: [DerivedOutputOperation; 3] = [
    DerivedOutputOperation::Export,
    DerivedOutputOperation::Plan,
    DerivedOutputOperation::Render,
];
const RESULTS: [DerivedOutputResultClass; 11] = [
    DerivedOutputResultClass::CancelledBeforeResultOrEffect,
    DerivedOutputResultClass::CapabilityOrValidationRejection,
    DerivedOutputResultClass::CompletedProjection,
    DerivedOutputResultClass::ExportOverwriteConflict,
    DerivedOutputResultClass::ExportPathRejection,
    DerivedOutputResultClass::ExportRetryConflict,
    DerivedOutputResultClass::Exported,
    DerivedOutputResultClass::ExternalTargetDriftConflict,
    DerivedOutputResultClass::IdempotentExportReplay,
    DerivedOutputResultClass::InternalFailureKnownNoEffect,
    DerivedOutputResultClass::StaleOrUnavailableRevision,
];
const SOURCES: [AutonomousOutputIntentSource; 3] = [
    AutonomousOutputIntentSource::CallerExplicitGoal,
    AutonomousOutputIntentSource::IndependentAdmittedHostPolicy,
    AutonomousOutputIntentSource::UntrustedSemanticContent,
];

fn applies(
    operation: DerivedOutputOperation,
    result: DerivedOutputResultClass,
) -> bool {
    match result {
        DerivedOutputResultClass::CancelledBeforeResultOrEffect
        | DerivedOutputResultClass::CapabilityOrValidationRejection
        | DerivedOutputResultClass::InternalFailureKnownNoEffect
        | DerivedOutputResultClass::StaleOrUnavailableRevision => true,
        DerivedOutputResultClass::CompletedProjection => matches!(
            operation,
            DerivedOutputOperation::Plan | DerivedOutputOperation::Render
        ),
        DerivedOutputResultClass::Exported
        | DerivedOutputResultClass::ExportOverwriteConflict
        | DerivedOutputResultClass::ExportPathRejection
        | DerivedOutputResultClass::ExportRetryConflict
        | DerivedOutputResultClass::ExternalTargetDriftConflict
        | DerivedOutputResultClass::IdempotentExportReplay => {
            operation == DerivedOutputOperation::Export
        },
    }
}

fn expected(
    operation: DerivedOutputOperation,
    result: DerivedOutputResultClass,
    source: AutonomousOutputIntentSource,
) -> Option<ProgressEvidence> {
    if source != AutonomousOutputIntentSource::CallerExplicitGoal
        || !applies(operation, result)
    {
        return None;
    }
    match result {
        DerivedOutputResultClass::CompletedProjection
        | DerivedOutputResultClass::Exported => {
            Some(ProgressEvidence::ExplicitRequestedOutputCompleted)
        },
        DerivedOutputResultClass::IdempotentExportReplay => {
            Some(ProgressEvidence::IdempotentReplayRecovery)
        },
        DerivedOutputResultClass::CancelledBeforeResultOrEffect
        | DerivedOutputResultClass::CapabilityOrValidationRejection
        | DerivedOutputResultClass::ExportOverwriteConflict
        | DerivedOutputResultClass::ExportPathRejection
        | DerivedOutputResultClass::ExportRetryConflict
        | DerivedOutputResultClass::ExternalTargetDriftConflict
        | DerivedOutputResultClass::InternalFailureKnownNoEffect
        | DerivedOutputResultClass::StaleOrUnavailableRevision => None,
    }
}

#[test]
fn all_99_output_result_intent_combinations_match_progress_projection() {
    let mut combinations = 0_usize;
    let mut progress = 0_usize;
    let mut recovery = 0_usize;
    for operation in OPERATIONS {
        for result in RESULTS {
            for source in SOURCES {
                let expected = expected(operation, result, source);
                assert_eq!(
                    autonomous_requested_output_progress_evidence(
                        operation, result, source
                    ),
                    expected
                );
                combinations += 1;
                if expected
                    == Some(ProgressEvidence::ExplicitRequestedOutputCompleted)
                {
                    progress += 1;
                }
                if expected == Some(
                    ProgressEvidence::IdempotentReplayRecovery,
                ) {
                    recovery += 1;
                }
            }
        }
    }
    assert_eq!(combinations, 99);
    assert_eq!(progress, 3);
    assert_eq!(recovery, 1);
}
