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
//   - Projection from frozen derived/output result classes into requested
//     output completion state.
// - Must-Not:
//   - Execute Render/Plan/Export, choose paths, construct receipts, retry,
//     reinterpret invalid result/operation pairs, or infer output intent.
// - Allows:
//   - Inputs: One derived/output operation and one completed core result class.
//   - Outputs: Complete/incomplete state only when the result applies.
//   - Side effects: None.
// - Split-When:
//   - One output operation gains independent completion policy.
// - Merge-When:
//   - One autonomous coordinator directly owns requested-output aggregation.
// - Summary:
//   - Prevents rejected or invalid output results from satisfying completion.
// - Description:
//   - Maps only applicable frozen output results to completion state.
// - Usage:
//   - Qualify each requested output result before aggregate goal completion.
// - Defaults:
//   - Invalid operation/result pairs yield no completion state.
//

//! Frozen derived/output result projection into requested-output completion.

use atrament_autonomous_completion_admission::
    AutonomousRequestedOutputItemCompletion as OutputItemCompletion;
use atrament_derived_output_result::{
    DerivedOutputOperation, DerivedOutputResultClass,
    derived_output_result_applies_to,
};

/// Project one applicable derived/output result into completion state.
#[must_use]
pub const fn autonomous_requested_output_completion(
    operation: DerivedOutputOperation,
    result: DerivedOutputResultClass,
) -> Option<OutputItemCompletion> {
    if !derived_output_result_applies_to(operation, result) {
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
