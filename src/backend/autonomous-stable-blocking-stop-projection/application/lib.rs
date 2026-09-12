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
//   - Construct diagnostic fingerprints, parse localized prose, stop on
//     unrelated non-progress, decide goal satisfaction, retry, mutate state, or
//     execute a loop.
// - Allows:
//   - Inputs: Consecutive owner-qualified blocking observations plus admission
//     state, or one already-qualified autonomous progress-evidence class.
//   - Outputs: Repeated stable-blocking evidence and its frozen terminal stop.
//   - Side effects: None.
// - Split-When:
//   - Diagnostic-fingerprint construction gains executable authority here.
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
use atrament_autonomous_progress_evidence::AutonomousProgressEvidenceClass;

type ProgressEvidence = AutonomousProgressEvidenceClass;

/// Authoritative inputs tied to one already-qualified blocking diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutonomousStableBlockingObservation<Diagnostic, Intent, Revision> {
    /// Owner-qualified typed blocking-diagnostic fingerprint.
    pub diagnostic: Diagnostic,
    /// Owner-qualified bounded intent identity or equality-stable fingerprint.
    pub intent: Intent,
    /// Accepted revision to which the blocking diagnostic applies.
    pub revision: Revision,
}

/// Detect frozen repeated stable-blocking non-progress across two iterations.
///
/// The diagnostic owner must construct an equality-stable fingerprint from
/// typed code, location, blocking disposition, and evidence. `new_admission`
/// must likewise be qualified by its owning evidence/capability/context
/// boundary.
#[must_use]
pub fn autonomous_stable_blocking_progress_evidence<
    Diagnostic,
    Intent,
    Revision,
>(
    previous: &AutonomousStableBlockingObservation<
        Diagnostic,
        Intent,
        Revision,
    >,
    current: &AutonomousStableBlockingObservation<Diagnostic, Intent, Revision>,
    new_admission: bool,
) -> Option<ProgressEvidence>
where
    Diagnostic: PartialEq,
    Intent: PartialEq,
    Revision: PartialEq,
{
    if !new_admission && previous == current {
        Some(ProgressEvidence::StableBlockingDiagnosticRepeated)
    } else {
        None
    }
}

/// Return the frozen terminal outcome for qualified stable-blocking evidence.
#[must_use]
pub const fn autonomous_stable_blocking_terminal_outcome(
    evidence: ProgressEvidence,
) -> Option<AutonomousGoalTerminalClass> {
    match evidence {
        ProgressEvidence::StableBlockingDiagnosticRepeated => {
            Some(AutonomousGoalTerminalClass::StoppedStableBlockingFailure)
        }
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
