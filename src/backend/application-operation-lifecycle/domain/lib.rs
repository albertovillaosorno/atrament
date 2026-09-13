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
//   - Frozen application operation classes, effect boundaries, cancellation
//     meaning, and completion authority for already-qualified observations.
// - Must-Not:
//   - Schedule work, assign operation IDs, report progress, cancel execution,
//     persist jobs, recover retries, mutate state, or map adapter protocols.
// - Allows:
//   - Inputs: One frozen operation class plus qualified cancellation or
//     completion observations.
//   - Outputs: Exact effect, cancellation, and completion meaning when defined.
//   - Side effects: None.
// - Split-When:
//   - Progress, cancellation admission, or execution scheduling gains lifecycle
//     authority.
// - Merge-When:
//   - One application coordinator directly owns every operation lifecycle.
// - Summary:
//   - Pins effect boundaries without inventing asynchronous infrastructure.
// - Description:
//   - Keeps read-only completion distinct from semantic, history, and file
//     commit.
// - Usage:
//   - Classify the owning effect boundary before projecting lifecycle metadata.
// - Defaults:
//   - Progress or transport termination never proves boundary completion.
//

//! Frozen effect-boundary vocabulary shared by application operation
//! lifecycles.

/// First-release operation class governed by the shared lifecycle contract.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ApplicationOperationClass {
    /// Atomic semantic command Apply.
    Apply,
    /// Caller-authorized persistent Export.
    Export,
    /// Accepted semantic Undo or Redo traversal.
    HistoryTraversal,
    /// Read-only device-neutral Plan compilation.
    Plan,
    /// Read-only Render projection.
    Render,
    /// Read-only semantic command Validate.
    Validate,
}

/// Authoritative boundary after which an operation's effect is established.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationOperationEffectBoundary {
    /// Atomic accepted semantic revision commit.
    AcceptedSemanticCommit,
    /// Explicit persistent Export file commit.
    FileCommit,
    /// Atomic accepted history traversal commit.
    HistoryTraversalCommit,
    /// Complete read-only result admission with no persistent effect.
    ReadOnlyCompletion,
}

/// Already-qualified observation relevant to cancellation resolution.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ApplicationCancellationObservation {
    /// The owning operation proved its authoritative effect boundary was
    /// crossed.
    EffectBoundaryCrossed,
    /// A cancellation request exists, but no final boundary fact is
    /// established.
    RequestOnly,
    /// The owning operation proved cancellation took effect before its
    /// boundary.
    TookEffectBeforeBoundary,
}

/// Lifecycle meaning established by a qualified cancellation observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationCancellationDisposition {
    /// Cancellation took effect before the owning boundary, so no effect was
    /// established by that operation.
    CancelledBeforeEffect,
    /// The authoritative boundary was already crossed and cannot be relabeled
    /// as cancelled or rolled back by the cancellation request.
    EffectRemainsAuthoritative,
}

/// Observation that may or may not establish operation completion.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ApplicationOperationCompletionObservation {
    /// A cancellation request exists without a final operation result.
    CancellationRequest,
    /// The owning capability returned its final typed result or receipt.
    FinalTypedResultOrReceipt,
    /// One observational progress update was emitted.
    ProgressObservation,
    /// The owning mutating capability resolved a prior unknown outcome.
    SameRetryRecovery,
    /// Transport ended without a final application result.
    TransportTermination,
}

/// Completion meaning established by one lifecycle observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationOperationCompletionDisposition {
    /// A final typed result or receipt establishes application completion.
    CompleteByFinalResult,
    /// Same-retry recovery establishes completion of a prior mutating attempt.
    CompleteByRecoveredMutatingOutcome,
    /// This observation does not establish application completion.
    NotEstablished,
}

/// Qualified cancellation resolution retaining the operation's owning boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApplicationCancellationResolution {
    /// Boundary whose status makes the cancellation outcome authoritative.
    pub boundary: ApplicationOperationEffectBoundary,
    /// Cancellation meaning established by the qualified observation.
    pub disposition: ApplicationCancellationDisposition,
}

/// Return the authoritative effect boundary for one operation class.
///
/// This does not say whether the boundary was crossed, whether cancellation is
/// admitted, or how progress, transport loss, retry, or receipts are projected.
#[must_use]
pub const fn application_operation_effect_boundary(
    operation: ApplicationOperationClass,
) -> ApplicationOperationEffectBoundary {
    match operation {
        ApplicationOperationClass::Apply => {
            ApplicationOperationEffectBoundary::AcceptedSemanticCommit
        }
        ApplicationOperationClass::Export => {
            ApplicationOperationEffectBoundary::FileCommit
        }
        ApplicationOperationClass::HistoryTraversal => {
            ApplicationOperationEffectBoundary::HistoryTraversalCommit
        }
        ApplicationOperationClass::Plan
        | ApplicationOperationClass::Render
        | ApplicationOperationClass::Validate => {
            ApplicationOperationEffectBoundary::ReadOnlyCompletion
        }
    }
}

/// Resolve cancellation meaning only from an already-qualified boundary fact.
///
/// A request alone returns `None`: it is not proof that cancellation took
/// effect, that a commit was avoided, or that completed work was rolled back.
/// This function does not admit cancellation, observe execution, or create a
/// result class.
#[must_use]
pub const fn application_cancellation_resolution(
    operation: ApplicationOperationClass,
    observation: ApplicationCancellationObservation,
) -> Option<ApplicationCancellationResolution> {
    let boundary = application_operation_effect_boundary(operation);
    match observation {
        ApplicationCancellationObservation::RequestOnly => None,
        ApplicationCancellationObservation::TookEffectBeforeBoundary => {
            Some(ApplicationCancellationResolution {
                boundary,
                disposition:
                    ApplicationCancellationDisposition::CancelledBeforeEffect,
            })
        },
        ApplicationCancellationObservation::EffectBoundaryCrossed => {
            Some(ApplicationCancellationResolution {
                boundary,
                disposition: ApplicationCancellationDisposition::
                    EffectRemainsAuthoritative,
            })
        },
    }
}

/// Classify whether an observation establishes application-level completion.
///
/// Progress, transport termination, and cancellation requests remain
/// non-authoritative. Same-retry recovery is admitted here only for the three
/// operations with mutating effect boundaries: Apply, Export, and history
/// traversal. Read-only operations use a new final result when repeated after
/// transport loss rather than a mutating-outcome recovery classification.
#[must_use]
pub const fn application_operation_completion_disposition(
    operation: ApplicationOperationClass,
    observation: ApplicationOperationCompletionObservation,
) -> Option<ApplicationOperationCompletionDisposition> {
    match observation {
        ApplicationOperationCompletionObservation::CancellationRequest
        | ApplicationOperationCompletionObservation::ProgressObservation
        | ApplicationOperationCompletionObservation::TransportTermination => {
            Some(ApplicationOperationCompletionDisposition::NotEstablished)
        },
        ApplicationOperationCompletionObservation::
            FinalTypedResultOrReceipt => Some(
            ApplicationOperationCompletionDisposition::CompleteByFinalResult,
        ),
        ApplicationOperationCompletionObservation::SameRetryRecovery => {
            match operation {
                ApplicationOperationClass::Apply
                | ApplicationOperationClass::Export
                | ApplicationOperationClass::HistoryTraversal => Some(
                    ApplicationOperationCompletionDisposition::
                        CompleteByRecoveredMutatingOutcome,
                ),
                ApplicationOperationClass::Plan
                | ApplicationOperationClass::Render
                | ApplicationOperationClass::Validate => None,
            }
        },
    }
}
