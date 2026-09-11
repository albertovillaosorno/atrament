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
//   - Progress projection for one exact application-capability admission
//     change.
// - Must-Not:
//   - Run capability discovery, infer goal requirements, decide terminal stops,
//     mutate state, refresh command context, or run an autonomous loop.
// - Allows:
//   - Inputs: Previous and current already-qualified application admissions.
//   - Outputs: New-admitted-authority evidence for exact same-capability gain.
//   - Side effects: None.
// - Split-When:
//   - Capability discovery history gains executable coordinator authority.
// - Merge-When:
//   - One coordinator directly owns all stateful autonomous progress evidence.
// - Summary:
//   - Recognizes only unsupported-to-admitted transition of one same
//     capability.
// - Description:
//   - Prevents capability identity drift from being counted as new authority.
// - Usage:
//   - Compare two read-only admission facts after explicit discovery refresh.
// - Defaults:
//   - All transitions other than exact unsupported-to-admitted return no
//     evidence.
//

//! Progress evidence for exact application-capability admission transitions.

use atrament_autonomous_progress_evidence::AutonomousProgressEvidenceClass;
use atrament_semantic_notebook_port::SemanticCommandApplicationAdmission;

/// Project one exact application-capability gain into autonomous progress.
///
/// The admission facts must already come from owning discovery boundaries. This
/// function does not establish that a capability is required by the goal or
/// that
/// an absent capability is terminally unavailable.
#[must_use]
pub fn autonomous_application_capability_progress_evidence(
    previous: SemanticCommandApplicationAdmission,
    current: SemanticCommandApplicationAdmission,
) -> Option<AutonomousProgressEvidenceClass> {
    match (previous, current) {
        (
            SemanticCommandApplicationAdmission::Unsupported { requested },
            SemanticCommandApplicationAdmission::Admitted { capability },
        ) if requested == capability => {
            Some(AutonomousProgressEvidenceClass::NewAdmittedAuthorityOrContext)
        },
        _ => None,
    }
}
