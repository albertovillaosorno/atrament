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
//   - Transport-neutral frozen Render, Plan, and Export result vocabulary.
// - Must-Not:
//   - Render, plan, export, touch files, choose paths or overwrite intent,
//     normalize receipts, persist retry state, or infer transport outcomes.
// - Allows:
//   - Inputs: One derived/output operation and one frozen result class.
//   - Outputs: Exact operation applicability for that result class.
//   - Side effects: None.
// - Split-When:
//   - One operation gains independent executable result interpretation policy.
// - Merge-When:
//   - One shared application-result owner subsumes derived/output vocabulary.
// - Summary:
//   - Freezes machine-readable derived/output result semantics without effects.
// - Description:
//   - Keeps projection success distinct from Export commit and conflicts.
// - Usage:
//   - Validate result vocabulary before adapters project operation outcomes.
// - Defaults:
//   - Unknown transport outcome remains caller state, never a core result.
//

//! Frozen transport-neutral result vocabulary for Render, Plan, and Export.

/// Application operation governed by the derived/output result taxonomy.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DerivedOutputOperation {
    /// Caller-authorized persistent Export.
    Export,
    /// Read-only device-neutral Plan compilation.
    Plan,
    /// Read-only Render projection.
    Render,
}

/// Core result class returned by completed derived/output operations.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DerivedOutputResultClass {
    /// Admitted cancellation completed before result or file effect.
    CancelledBeforeResultOrEffect,
    /// Required capability or owning validation rejected the operation.
    CapabilityOrValidationRejection,
    /// Render or Plan produced one complete read-only projection.
    CompletedProjection,
    /// One explicit Export crossed its admitted file-commit boundary.
    Exported,
    /// Existing target conflicts with explicit overwrite disposition.
    ExportOverwriteConflict,
    /// Explicit Export target violates the owning path boundary.
    ExportPathRejection,
    /// Export retry identity was reused with different normalized intent.
    ExportRetryConflict,
    /// Same-retry recovery observed externally changed target state.
    ExternalTargetDriftConflict,
    /// Same normalized Export retry recovered prior committed output.
    IdempotentExportReplay,
    /// Failure is proven to have produced no projection or file effect.
    InternalFailureKnownNoEffect,
    /// Requested accepted revision is stale or unavailable to the operation.
    StaleOrUnavailableRevision,
}

/// Check whether one frozen result class belongs to an operation.
///
/// This is vocabulary applicability only. It does not determine which result an
/// execution should return, choose rejection precedence, or perform any effect.
#[must_use]
pub const fn derived_output_result_applies_to(
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
            matches!(operation, DerivedOutputOperation::Export)
        },
    }
}
