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
//   - Decide goal completion, construct terminal outcomes, infer No-op
//     repetition, retry, mutate state, schedule work, or grant continuation.
// - Allows:
//   - Inputs: One already-qualified autonomous progress-evidence class.
//   - Outputs: Whether this axis requires stopping equivalent mutation retry.
//   - Side effects: None.
// - Split-When:
//   - Stateful repeated-No-op detection gains executable authority here.
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
