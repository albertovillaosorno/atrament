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
//   - Projection from caller-requested derived/output results to directly
//     implied autonomous progress evidence.
// - Must-Not:
//   - Execute output, infer caller intent from content, treat host policy as
//     goal progress, rewrite replay as new output, retry, schedule work, or
//     mutate.
// - Allows:
//   - Inputs: One output operation, result class, and explicit intent source.
//   - Outputs: Progress/recovery evidence only when frozen rules imply it.
//   - Side effects: None.
// - Split-When:
//   - Stateful output aggregation or retry proof gains authority here.
// - Merge-When:
//   - One autonomous coordinator directly owns requested-output evidence.
// - Summary:
//   - Counts only caller-requested successful output as new goal progress.
// - Description:
//   - Keeps recovered Export completion distinct from newly completed output.
// - Usage:
//   - Project completed output results before progress/stop composition.
// - Defaults:
//   - Host-policy, untrusted, failed, and invalid outputs yield no evidence.
//

//! Requested-output result projection into autonomous progress evidence.

use atrament_autonomous_output_intent::AutonomousOutputIntentSource;
use atrament_autonomous_progress_evidence::
    AutonomousProgressEvidenceClass as ProgressEvidence;
use atrament_derived_output_result::{
    DerivedOutputOperation, DerivedOutputResultClass,
    derived_output_result_applies_to,
};

/// Project directly implied progress for output in the caller's admitted goal.
#[must_use]
pub const fn autonomous_requested_output_progress_evidence(
    operation: DerivedOutputOperation,
    result: DerivedOutputResultClass,
    source: AutonomousOutputIntentSource,
) -> Option<ProgressEvidence> {
    if !matches!(source, AutonomousOutputIntentSource::CallerExplicitGoal)
        || !derived_output_result_applies_to(operation, result)
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
