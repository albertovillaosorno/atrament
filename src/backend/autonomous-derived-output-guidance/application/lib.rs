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
//   - Frozen autonomous remediation guidance for derived/output result classes.
// - Must-Not:
//   - Render, plan, export, choose paths, overwrite files, retry automatically,
//     mutate semantic state, schedule work, or infer transport outcomes.
// - Allows:
//   - Inputs: One completed derived/output result class.
//   - Outputs: One frozen guidance class when the contracts define it exactly.
//   - Side effects: None.
// - Split-When:
//   - Executable output recovery gains independent application authority.
// - Merge-When:
//   - A final automation coordinator directly owns output-result remediation.
// - Summary:
//   - Captures explicit recovery branches without inventing output policy.
// - Description:
//   - Distinguishes stale, retry, overwrite, drift, and replay recovery
//     actions.
// - Usage:
//   - Interpret typed output outcomes before considering another output
//     request.
// - Defaults:
//   - Results without one frozen remediation branch remain unclassified.
//

//! Frozen partial autonomous guidance for derived/output outcomes.

use atrament_derived_output_result::DerivedOutputResultClass;

/// Frozen autonomous remediation class for derived/output outcomes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutonomousDerivedOutputGuidance {
    /// Choose an explicit different target or explicit overwrite disposition.
    ChooseExplicitTargetOrOverwriteIntent,
    /// Correct retry bookkeeping or intentionally issue a distinct Export.
    CorrectRetryOrIssueDistinctExport,
    /// Inspect current state and deliberately form a new projection request.
    FreshInspectionOrDeliberateRequest,
    /// Issue a new explicit Export action instead of overwriting external
    /// drift.
    NewExplicitExportAfterExternalDrift,
    /// Prior Export commit was recovered; do not rewrite it as fake progress.
    RecoveredPriorExportCompletion,
}

/// Return remediation guidance only where the frozen contract defines it.
///
/// Successful completion, cancellation, path/validation rejection, and internal
/// known-no-effect failure remain unclassified because their next action
/// depends
/// on caller goal, diagnostics, explicit authority, or host policy.
#[must_use]
pub const fn autonomous_derived_output_guidance(
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
