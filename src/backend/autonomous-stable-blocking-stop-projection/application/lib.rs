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
//   - Projection from qualified stable-blocking non-progress to terminal stop.
// - Must-Not:
//   - Compare diagnostics, infer repetition, stop on unrelated non-progress,
//     decide goal satisfaction, retry, mutate state, or execute a loop.
// - Allows:
//   - Inputs: One already-qualified autonomous progress-evidence class.
//   - Outputs: Stable-blocking terminal stop only when explicitly established.
//   - Side effects: None.
// - Split-When:
//   - Stateful diagnostic comparison gains executable authority here.
// - Merge-When:
//   - One autonomous coordinator owns progress detection plus stop
//     construction.
// - Summary:
//   - Connects repeated stable blocking evidence to its frozen terminal class.
// - Description:
//   - Leaves all other progress/non-progress classes nonterminal on this axis.
// - Usage:
//   - Apply after the diagnostic owner proves the repeated blocking condition.
// - Defaults:
//   - Unrelated evidence produces no terminal outcome.
//

//! Stable-blocking progress-evidence projection into terminal stop semantics.

use atrament_autonomous_goal_outcome::AutonomousGoalTerminalClass;
use atrament_autonomous_progress_evidence::
    AutonomousProgressEvidenceClass as ProgressEvidence;

/// Return the frozen terminal outcome for qualified stable-blocking evidence.
#[must_use]
pub const fn autonomous_stable_blocking_terminal_outcome(
    evidence: ProgressEvidence,
) -> Option<AutonomousGoalTerminalClass> {
    match evidence {
        ProgressEvidence::StableBlockingDiagnosticRepeated => {
            Some(AutonomousGoalTerminalClass::StoppedStableBlockingFailure)
        },
        ProgressEvidence::AcceptedApplyRevisionChange
        | ProgressEvidence::AcceptedHistoryRevisionChange
        | ProgressEvidence::BlockingDiagnosticResolved
        | ProgressEvidence::ExplicitRequestedOutputCompleted
        | ProgressEvidence::IdempotentReplayRecovery
        | ProgressEvidence::NewAdmittedAuthorityOrContext
        | ProgressEvidence::RepeatedNoOpSameIntent
        | ProgressEvidence::RepeatedSameInputsWithoutNewAdmission => None,
    }
}
