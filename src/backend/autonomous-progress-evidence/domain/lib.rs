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
//   - Transport-neutral frozen autonomous progress/non-progress evidence
//     classes.
// - Must-Not:
//   - Infer evidence from prose, inspect state, run loops, choose budgets,
//     schedule retries, mutate state, or broaden output/physical authority.
// - Allows:
//   - Inputs: One already-qualified frozen progress-evidence class.
//   - Outputs: Whether that class is authoritative progress or non-progress.
//   - Side effects: None.
// - Split-When:
//   - Stateful comparison of revisions/diagnostics gains application authority.
// - Merge-When:
//   - One autonomous-loop coordinator directly owns all progress evidence.
// - Summary:
//   - Keeps authoritative progress distinct from retry/no-op/diagnostic churn.
// - Description:
//   - Classifies only evidence conditions frozen by the autonomous-loop
//     contract.
// - Usage:
//   - Apply after owning services prove the evidence condition named by a
//     class.
// - Defaults:
//   - This vocabulary is not exhaustive for future progress evidence classes.
//

//! Frozen progress/non-progress semantics for already-qualified loop evidence.

/// Authoritative evidence condition recognized by the autonomous-loop contract.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AutonomousProgressEvidenceClass {
    /// Apply produced one new accepted revision.
    AcceptedApplyRevisionChange,
    /// Admitted history traversal produced one new accepted revision.
    AcceptedHistoryRevisionChange,
    /// A previously blocking diagnostic resolved after accepted change.
    BlockingDiagnosticResolved,
    /// Explicitly requested Render, Export, or Plan completed successfully.
    ExplicitRequestedOutputCompleted,
    /// Same-retry recovery returned a prior completion without a new edit.
    IdempotentReplayRecovery,
    /// New admitted evidence, capability, or command context became available.
    NewAdmittedAuthorityOrContext,
    /// The same bounded intent produced No-op again without new evidence.
    RepeatedNoOpSameIntent,
    /// Revision, blocking evidence, and intent repeated without new admitted
    /// input.
    RepeatedSameInputsWithoutNewAdmission,
    /// The same blocking diagnostic repeated for unchanged authoritative
    /// inputs.
    StableBlockingDiagnosticRepeated,
}

/// Progress disposition of one already-qualified evidence condition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutonomousProgressDisposition {
    /// Evidence does not represent new autonomous progress.
    NonProgress,
    /// Evidence represents authoritative progress for the bounded goal.
    Progress,
}

/// Classify one frozen, already-qualified progress-evidence condition.
///
/// This function never decides whether raw state satisfies the condition. The
/// owning revision, diagnostic, capability, context, and output boundaries must
/// establish that evidence before classification.
#[must_use]
pub const fn autonomous_progress_disposition(
    evidence: AutonomousProgressEvidenceClass,
) -> AutonomousProgressDisposition {
    match evidence {
        AutonomousProgressEvidenceClass::AcceptedApplyRevisionChange
        | AutonomousProgressEvidenceClass::AcceptedHistoryRevisionChange
        | AutonomousProgressEvidenceClass::BlockingDiagnosticResolved
        | AutonomousProgressEvidenceClass::ExplicitRequestedOutputCompleted
        | AutonomousProgressEvidenceClass::NewAdmittedAuthorityOrContext => {
            AutonomousProgressDisposition::Progress
        },
        AutonomousProgressEvidenceClass::IdempotentReplayRecovery
        | AutonomousProgressEvidenceClass::RepeatedNoOpSameIntent
        | AutonomousProgressEvidenceClass::RepeatedSameInputsWithoutNewAdmission
        | AutonomousProgressEvidenceClass::StableBlockingDiagnosticRepeated => {
            AutonomousProgressDisposition::NonProgress
        },
    }
}
