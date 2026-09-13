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

use atrament_application_operation_lifecycle::{
    ApplicationCancellationDisposition, ApplicationCancellationObservation,
    ApplicationOperationClass, application_cancellation_resolution,
};

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
    /// Existing target conflicts with explicit overwrite disposition.
    ExportOverwriteConflict,
    /// Explicit Export target violates the owning path boundary.
    ExportPathRejection,
    /// Export retry identity was reused with different normalized intent.
    ExportRetryConflict,
    /// One explicit Export crossed its admitted file-commit boundary.
    Exported,
    /// Same-retry recovery observed externally changed target state.
    ExternalTargetDriftConflict,
    /// Same normalized Export retry recovered prior committed output.
    IdempotentExportReplay,
    /// Failure is proven to have produced no projection or file effect.
    InternalFailureKnownNoEffect,
    /// Requested accepted revision is stale or unavailable to the operation.
    StaleOrUnavailableRevision,
}

/// Effect disposition guaranteed by one completed derived/output result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DerivedOutputEffectDisposition {
    /// The current Export crossed its file-commit boundary exactly once.
    CommittedFileThisCall,
    /// The current call completed one read-only Render or Plan projection.
    CompletedReadOnlyProjection,
    /// The current call is known to have created no new projection or file
    /// effect.
    KnownNoNewEffect,
    /// Same-retry recovery proved one earlier Export file commit.
    RecoveredPriorFileCommit,
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

/// Classify only the effect guaranteed by one completed result class.
///
/// Idempotent Export replay reports recovery of a prior file commit, not
/// another
/// commit by the current call. Conflict and rejection classes imply no new
/// projection or file effect from the current call.
#[must_use]
pub const fn derived_output_effect_disposition(
    result: DerivedOutputResultClass,
) -> DerivedOutputEffectDisposition {
    match result {
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
    }
}

/// Project the effect disposition only when the result belongs to the named
/// operation.
///
/// Returning `None` prevents an adapter from treating an Export-only result as
/// a Render or Plan result, or `CompletedProjection` as an Export outcome. This
/// remains vocabulary validation only and performs no output operation.
#[must_use]
pub const fn derived_output_effect_disposition_for_operation(
    operation: DerivedOutputOperation,
    result: DerivedOutputResultClass,
) -> Option<DerivedOutputEffectDisposition> {
    if derived_output_result_applies_to(operation, result) {
        Some(derived_output_effect_disposition(result))
    } else {
        None
    }
}


const fn lifecycle_operation(
    operation: DerivedOutputOperation,
) -> ApplicationOperationClass {
    match operation {
        DerivedOutputOperation::Export => ApplicationOperationClass::Export,
        DerivedOutputOperation::Plan => ApplicationOperationClass::Plan,
        DerivedOutputOperation::Render => ApplicationOperationClass::Render,
    }
}

/// Project one already-qualified cancellation observation into this taxonomy.
///
/// A request alone has no result. Cancellation proven before the owning
/// boundary maps to `CancelledBeforeResultOrEffect`; a crossed read-only
/// completion remains `CompletedProjection`, and crossed Export file commit
/// remains `Exported`. This does not admit or signal cancellation, execute an
/// output operation, or recover a lost receipt.
#[must_use]
pub const fn classify_derived_output_cancellation_result(
    operation: DerivedOutputOperation,
    observation: ApplicationCancellationObservation,
) -> Option<DerivedOutputResultClass> {
    match application_cancellation_resolution(
        lifecycle_operation(operation),
        observation,
    ) {
        None => None,
        Some(resolution) => match resolution.disposition {
            ApplicationCancellationDisposition::CancelledBeforeEffect => {
                Some(DerivedOutputResultClass::CancelledBeforeResultOrEffect)
            },
            ApplicationCancellationDisposition::EffectRemainsAuthoritative => {
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
        },
    }
}
