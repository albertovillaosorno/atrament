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
//   - Exhaustive evidence for frozen partial derived/output automation advice.
// - Must-Not:
//   - Render, plan, export, mutate, retry, schedule work, or choose file
//     intent.
// - Allows:
//   - Inputs: All eleven frozen derived/output result classes.
//   - Outputs: Exact guidance-or-unclassified assertions for every result.
//   - Side effects: None.
// - Split-When:
//   - Stateful autonomous output fixtures require independent evidence.
// - Merge-When:
//   - Executable output-loop parity fully subsumes this classification.
// - Summary:
//   - Pins five explicit remediations and six deliberately unclassified cases.
// - Description:
//   - Uses an independent exhaustive oracle over the output result vocabulary.
// - Usage:
//   - Compare every result with its explicitly frozen autonomous remediation.
// - Defaults:
//   - Results with goal- or diagnostic-dependent next actions remain open.
//
use atrament_autonomous_derived_output_guidance::{
    AutonomousDerivedOutputGuidance, autonomous_derived_output_guidance,
};
use atrament_derived_output_result::DerivedOutputResultClass;

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

fn expected(
    result: DerivedOutputResultClass,
) -> Option<AutonomousDerivedOutputGuidance> {
    match result {
        DerivedOutputResultClass::ExportOverwriteConflict => Some(
            AutonomousDerivedOutputGuidance::
                ChooseExplicitTargetOrOverwriteIntent,
        ),
        DerivedOutputResultClass::ExportRetryConflict => Some(
            AutonomousDerivedOutputGuidance::CorrectRetryOrIssueDistinctExport,
        ),
        DerivedOutputResultClass::ExternalTargetDriftConflict => Some(
            AutonomousDerivedOutputGuidance::
                NewExplicitExportAfterExternalDrift,
        ),
        DerivedOutputResultClass::IdempotentExportReplay => Some(
            AutonomousDerivedOutputGuidance::RecoveredPriorExportCompletion,
        ),
        DerivedOutputResultClass::StaleOrUnavailableRevision => Some(
            AutonomousDerivedOutputGuidance::FreshInspectionOrDeliberateRequest,
        ),
        DerivedOutputResultClass::CancelledBeforeResultOrEffect
        | DerivedOutputResultClass::CapabilityOrValidationRejection
        | DerivedOutputResultClass::CompletedProjection
        | DerivedOutputResultClass::ExportPathRejection
        | DerivedOutputResultClass::Exported
        | DerivedOutputResultClass::InternalFailureKnownNoEffect => None,
    }
}

#[test]
fn all_11_output_results_match_frozen_partial_guidance() {
    let mut classified = 0_usize;
    for result in RESULTS {
        let expected = expected(result);
        assert_eq!(autonomous_derived_output_guidance(result), expected);
        if expected.is_some() {
            classified += 1;
        }
    }
    assert_eq!(classified, 5);
}
