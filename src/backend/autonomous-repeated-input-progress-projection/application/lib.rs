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
//   - Stateful comparison for repeated authoritative autonomous-loop inputs.
// - Must-Not:
//   - Parse diagnostics, compare localized prose, infer admissions, retry,
//     mutate state, choose terminal outcomes, or run an autonomous loop.
// - Allows:
//   - Inputs: Two already-qualified input snapshots and new-admission fact.
//   - Outputs: Frozen repeated-same-inputs non-progress evidence or no
//     evidence.
//   - Side effects: None.
// - Split-When:
//   - Diagnostic fingerprint construction gains application authority here.
// - Merge-When:
//   - One coordinator directly owns all stateful autonomous progress evidence.
// - Summary:
//   - Recognizes unchanged revision, blocker, and intent without new admission.
// - Description:
//   - Uses owner-supplied equality-stable fingerprints instead of message text.
// - Usage:
//   - Compare consecutive bounded iterations after owning services qualify
//     data.
// - Defaults:
//   - Any changed axis or new admission prevents this non-progress evidence.
//

//! Stateful repeated-input evidence for bounded autonomous iterations.

use atrament_autonomous_progress_evidence::AutonomousProgressEvidenceClass;

/// Equality-stable authoritative inputs for one bounded autonomous iteration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutonomousIterationInputs<BlockingEvidence, Intent, Revision> {
    /// Owner-qualified blocking evidence fingerprint for this iteration.
    pub blocking_evidence: BlockingEvidence,
    /// Owner-qualified bounded intent identity or fingerprint.
    pub intent: Intent,
    /// Accepted revision observed by this iteration.
    pub revision: Revision,
}

/// Project exact repeated authoritative inputs into frozen non-progress
/// evidence.
///
/// `new_admission` must already be qualified by the owning evidence,
/// capability,
/// or command-context boundary. This function does not infer admissions or
/// construct blocking-evidence fingerprints.
#[must_use]
pub fn autonomous_repeated_input_progress_evidence<
    BlockingEvidence: PartialEq,
    Intent: PartialEq,
    Revision: PartialEq,
>(
    previous: &AutonomousIterationInputs<BlockingEvidence, Intent, Revision>,
    current: &AutonomousIterationInputs<BlockingEvidence, Intent, Revision>,
    new_admission: bool,
) -> Option<AutonomousProgressEvidenceClass> {
    if !new_admission && previous == current {
        Some(
            AutonomousProgressEvidenceClass::
                RepeatedSameInputsWithoutNewAdmission,
        )
    } else {
        None
    }
}
