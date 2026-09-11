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
//   - Frozen application operation classes and their lifecycle effect boundary.
// - Must-Not:
//   - Schedule work, assign operation IDs, report progress, cancel execution,
//     persist jobs, recover retries, mutate state, or map adapter protocols.
// - Allows:
//   - Inputs: One frozen first-release application operation class.
//   - Outputs: Its exact authoritative completion/effect boundary class.
//   - Side effects: None.
// - Split-When:
//   - Progress or cancellation admission gains executable lifecycle authority.
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
        },
        ApplicationOperationClass::Export => {
            ApplicationOperationEffectBoundary::FileCommit
        },
        ApplicationOperationClass::HistoryTraversal => {
            ApplicationOperationEffectBoundary::HistoryTraversalCommit
        },
        ApplicationOperationClass::Plan
        | ApplicationOperationClass::Render
        | ApplicationOperationClass::Validate => {
            ApplicationOperationEffectBoundary::ReadOnlyCompletion
        },
    }
}
