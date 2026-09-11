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
//   - Exhaustive evidence for derived/output result completion projection.
// - Must-Not:
//   - Execute output, choose paths, retry, infer intent, or construct receipts.
// - Allows:
//   - Inputs: All three operations crossed with all 11 frozen result classes.
//   - Outputs: Exact complete/incomplete/invalid assertion for all 33 pairs.
//   - Side effects: None.
// - Split-When:
//   - Executable output integration requires independent evidence.
// - Merge-When:
//   - Coordinator tests fully subsume this finite projection oracle.
// - Summary:
//   - Pins which applicable output results satisfy requested output completion.
// - Description:
//   - Preserves invalid operation/result pairs as unclassified.
// - Usage:
//   - Compare the entire operation/result cross-product with an exact oracle.
// - Defaults:
//   - Only successful projection/export/replay results count complete.
//
use atrament_autonomous_completion_admission::
    AutonomousRequestedOutputItemCompletion as OutputItemCompletion;
use atrament_autonomous_output_completion_projection::
    autonomous_requested_output_completion;
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
) -> Option<OutputItemCompletion> {
    if !applies(operation, result) {
        return None;
    }
    match result {
        DerivedOutputResultClass::CompletedProjection
        | DerivedOutputResultClass::Exported
        | DerivedOutputResultClass::IdempotentExportReplay => {
            Some(OutputItemCompletion::Complete)
        },
        DerivedOutputResultClass::CancelledBeforeResultOrEffect
        | DerivedOutputResultClass::CapabilityOrValidationRejection
        | DerivedOutputResultClass::ExportOverwriteConflict
        | DerivedOutputResultClass::ExportPathRejection
        | DerivedOutputResultClass::ExportRetryConflict
        | DerivedOutputResultClass::ExternalTargetDriftConflict
        | DerivedOutputResultClass::InternalFailureKnownNoEffect
        | DerivedOutputResultClass::StaleOrUnavailableRevision => {
            Some(OutputItemCompletion::Incomplete)
        },
    }
}

#[test]
fn all_33_output_result_pairs_match_completion_projection() {
    let mut combinations = 0_usize;
    let mut complete = 0_usize;
    for operation in OPERATIONS {
        for result in RESULTS {
            let expected = expected(operation, result);
            assert_eq!(
                autonomous_requested_output_completion(operation, result),
                expected
            );
            combinations += 1;
            if expected == Some(OutputItemCompletion::Complete) {
                complete += 1;
            }
        }
    }
    assert_eq!(combinations, 33);
    assert_eq!(complete, 4);
}
