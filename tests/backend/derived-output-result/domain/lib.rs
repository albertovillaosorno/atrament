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
//   - Regression evidence for frozen derived/output result applicability.
// - Must-Not:
//   - Execute Render, Plan, Export, filesystem effects, retry, or transport.
// - Allows:
//   - Inputs: All 33 operation/result-class combinations.
//   - Outputs: Exhaustive applicability assertions and exact class inventory.
//   - Side effects: None.
// - Split-When:
//   - Executable operation-result fixtures require independent application
//     tests.
// - Merge-When:
//   - Shared result execution tests fully subsume taxonomy applicability.
// - Summary:
//   - Proves the 11 frozen result classes belong only to admitted operations.
// - Description:
//   - Keeps Export-only file outcomes away from read-only Render and Plan.
// - Usage:
//   - Compare every operation/result pair against an independent oracle.
// - Defaults:
//   - Unknown transport outcome is absent because it is caller state.
//
use atrament_application_operation_lifecycle::{
    ApplicationCancellationObservation,
};
use atrament_derived_output_result::{
    DerivedOutputEffectDisposition, DerivedOutputOperation,
    DerivedOutputResultClass, classify_derived_output_cancellation_result,
    derived_output_effect_disposition,
    derived_output_effect_disposition_for_operation,
    derived_output_result_applies_to,
};

const ALL_OPERATIONS: [DerivedOutputOperation; 3] = [
    DerivedOutputOperation::Export,
    DerivedOutputOperation::Plan,
    DerivedOutputOperation::Render,
];

const ALL_RESULT_CLASSES: [DerivedOutputResultClass; 11] = [
    DerivedOutputResultClass::CancelledBeforeResultOrEffect,
    DerivedOutputResultClass::CapabilityOrValidationRejection,
    DerivedOutputResultClass::CompletedProjection,
    DerivedOutputResultClass::Exported,
    DerivedOutputResultClass::ExportOverwriteConflict,
    DerivedOutputResultClass::ExportPathRejection,
    DerivedOutputResultClass::ExportRetryConflict,
    DerivedOutputResultClass::ExternalTargetDriftConflict,
    DerivedOutputResultClass::IdempotentExportReplay,
    DerivedOutputResultClass::InternalFailureKnownNoEffect,
    DerivedOutputResultClass::StaleOrUnavailableRevision,
];

fn reference_applies(
    operation: DerivedOutputOperation,
    result: DerivedOutputResultClass,
) -> bool {
    let common = [
        DerivedOutputResultClass::CancelledBeforeResultOrEffect,
        DerivedOutputResultClass::CapabilityOrValidationRejection,
        DerivedOutputResultClass::InternalFailureKnownNoEffect,
        DerivedOutputResultClass::StaleOrUnavailableRevision,
    ];
    if common.contains(&result) {
        return true;
    }
    let operation_specific: &[DerivedOutputResultClass] = match operation {
        DerivedOutputOperation::Export => &[
            DerivedOutputResultClass::Exported,
            DerivedOutputResultClass::ExportOverwriteConflict,
            DerivedOutputResultClass::ExportPathRejection,
            DerivedOutputResultClass::ExportRetryConflict,
            DerivedOutputResultClass::ExternalTargetDriftConflict,
            DerivedOutputResultClass::IdempotentExportReplay,
        ],
        DerivedOutputOperation::Plan | DerivedOutputOperation::Render => {
            &[DerivedOutputResultClass::CompletedProjection]
        },
    };
    operation_specific.contains(&result)
}

#[test]
fn taxonomy_has_exactly_11_core_result_classes() {
    assert_eq!(ALL_RESULT_CLASSES.len(), 11);
}

#[test]
fn all_33_operation_result_pairs_match_independent_oracle() {
    let mut cases = 0_usize;
    for operation in ALL_OPERATIONS {
        for result in ALL_RESULT_CLASSES {
            assert_eq!(
                derived_output_result_applies_to(operation, result),
                reference_applies(operation, result),
                "operation={operation:?} result={result:?}",
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 33);
}

#[test]
fn all_11_result_classes_have_exact_effect_disposition() {
    for result in ALL_RESULT_CLASSES {
        let expected = match result {
            DerivedOutputResultClass::CompletedProjection => {
                DerivedOutputEffectDisposition::CompletedReadOnlyProjection
            },
            DerivedOutputResultClass::Exported => {
                DerivedOutputEffectDisposition::CommittedFileThisCall
            },
            DerivedOutputResultClass::IdempotentExportReplay => {
                DerivedOutputEffectDisposition::RecoveredPriorFileCommit
            },
            DerivedOutputResultClass::CancelledBeforeResultOrEffect
            | DerivedOutputResultClass::CapabilityOrValidationRejection
            | DerivedOutputResultClass::ExportOverwriteConflict
            | DerivedOutputResultClass::ExportPathRejection
            | DerivedOutputResultClass::ExportRetryConflict
            | DerivedOutputResultClass::ExternalTargetDriftConflict
            | DerivedOutputResultClass::InternalFailureKnownNoEffect
            | DerivedOutputResultClass::StaleOrUnavailableRevision => {
                DerivedOutputEffectDisposition::KnownNoNewEffect
            },
        };
        assert_eq!(derived_output_effect_disposition(result), expected);
    }
}

#[test]
fn all_33_operation_result_pairs_project_effect_only_when_applicable() {
    let mut cases = 0_usize;
    for operation in ALL_OPERATIONS {
        for result in ALL_RESULT_CLASSES {
            let expected = reference_applies(operation, result)
                .then(|| derived_output_effect_disposition(result));
            assert_eq!(
                derived_output_effect_disposition_for_operation(
                    operation,
                    result,
                ),
                expected,
                "operation={operation:?} result={result:?}",
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 33);
}


#[test]
fn all_nine_cancellation_observations_preserve_output_effect_boundaries() {
    const OBSERVATIONS: [ApplicationCancellationObservation; 3] = [
        ApplicationCancellationObservation::EffectBoundaryCrossed,
        ApplicationCancellationObservation::RequestOnly,
        ApplicationCancellationObservation::TookEffectBeforeBoundary,
    ];
    let mut cases = 0_usize;
    for operation in ALL_OPERATIONS {
        for observation in OBSERVATIONS {
            let expected = match observation {
                ApplicationCancellationObservation::RequestOnly => None,
                ApplicationCancellationObservation::
                    TookEffectBeforeBoundary => {
                    Some(
                        DerivedOutputResultClass::
                            CancelledBeforeResultOrEffect,
                    )
                },
                ApplicationCancellationObservation::EffectBoundaryCrossed => {
                    match operation {
                        DerivedOutputOperation::Export => {
                            Some(DerivedOutputResultClass::Exported)
                        },
                        DerivedOutputOperation::Plan
                        | DerivedOutputOperation::Render => {
                            Some(DerivedOutputResultClass::CompletedProjection)
                        },
                    }
                },
            };
            assert_eq!(
                classify_derived_output_cancellation_result(
                    operation,
                    observation,
                ),
                expected,
                "operation={operation:?} observation={observation:?}",
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 9);
}
