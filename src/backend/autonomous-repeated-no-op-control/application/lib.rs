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
//   - Local mutation-loop stop evidence for repeated No-op on the same bounded
//     intent.
// - Must-Not:
//   - Decide goal completion, construct terminal outcomes, infer intent
//     identity
//     or evidence relevance, retry, mutate state, schedule work, or grant
//     continuation.
// - Allows:
//   - Inputs: Qualified consecutive command results, bounded-intent identities,
//     an explicit new-relevant-evidence fact, or qualified progress evidence.
//   - Outputs: Repeated-No-op progress evidence and its local mutation-stop
//     requirement.
//   - Side effects: None.
// - Split-When:
//   - Intent-fingerprint or relevant-evidence qualification gains authority.
// - Merge-When:
//   - One coordinator directly owns qualified mutation-loop stop decisions.
// - Summary:
//   - Stops repeated equivalent No-op mutation without inventing goal outcome.
// - Description:
//   - False means only that this axis did not establish a stop decision.
// - Usage:
//   - Apply after an owner proves repeated No-op for the same bounded intent.
// - Defaults:
//   - Other evidence classes do not establish this specific mutation stop.
//

//! Local stop control for repeated No-op mutation evidence.

use atrament_autonomous_progress_evidence::AutonomousProgressEvidenceClass;
use atrament_semantic_notebook_port::SemanticCommandResultClass;

/// One qualified semantic-command result tied to a bounded intent identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutonomousNoOpObservation<Intent> {
    /// Owner-qualified bounded intent identity or equality-stable fingerprint.
    pub intent: Intent,
    /// Completed semantic-command result for this bounded attempt.
    pub result: SemanticCommandResultClass,
}

/// Detect frozen repeated-No-op non-progress from consecutive observations.
///
/// `new_relevant_evidence` is supplied by the owner of evidence qualification.
/// This function does not decide whether newly observed state or data is
/// relevant to the bounded intent.
#[must_use]
pub fn autonomous_repeated_no_op_progress_evidence<Intent>(
    previous: &AutonomousNoOpObservation<Intent>,
    current: &AutonomousNoOpObservation<Intent>,
    new_relevant_evidence: bool,
) -> Option<AutonomousProgressEvidenceClass>
where
    Intent: PartialEq,
{
    if !new_relevant_evidence
        && previous.intent == current.intent
        && previous.result == SemanticCommandResultClass::NoOp
        && current.result == SemanticCommandResultClass::NoOp
    {
        Some(AutonomousProgressEvidenceClass::RepeatedNoOpSameIntent)
    } else {
        None
    }
}

/// Return whether repeated-No-op evidence requires stopping equivalent
/// mutation.
///
/// `false` does not authorize another iteration. Other stop, completion,
/// budget,
/// capability, and workflow conditions remain independent.
#[must_use]
pub const fn autonomous_repeated_no_op_requires_equivalent_mutation_stop(
    evidence: AutonomousProgressEvidenceClass,
) -> bool {
    matches!(
        evidence,
        AutonomousProgressEvidenceClass::RepeatedNoOpSameIntent
    )
}
