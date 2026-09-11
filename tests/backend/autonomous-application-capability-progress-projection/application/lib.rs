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
//   - Exhaustive application-capability admission-transition progress evidence.
// - Must-Not:
//   - Discover capabilities, infer requirements, stop goals, mutate, or retry.
// - Allows:
//   - Inputs: All four capabilities crossed across both admission states twice.
//   - Outputs: Exact progress-evidence-or-none assertion for all 64
//     transitions.
//   - Side effects: None.
// - Split-When:
//   - Stateful discovery fixtures need independent application integration.
// - Merge-When:
//   - Coordinator tests fully subsume this finite transition oracle.
// - Summary:
//   - Proves only exact same-capability unsupported-to-admitted is progress.
// - Description:
//   - Identity drift and every other admission transition remain unclassified.
// - Usage:
//   - Compare every prior/current capability admission pair with exact oracle.
// - Defaults:
//   - No transition implies terminal unavailability or goal completion.
//
use atrament_autonomous_application_capability_progress_projection::
    autonomous_application_capability_progress_evidence;
use atrament_autonomous_progress_evidence::AutonomousProgressEvidenceClass;
use atrament_semantic_notebook_port::{
    CommandApplicationCapability, SemanticCommandApplicationAdmission,
};

const CAPABILITIES: [CommandApplicationCapability; 4] = [
    CommandApplicationCapability::Apply,
    CommandApplicationCapability::CommandContext,
    CommandApplicationCapability::SelectiveRebatching,
    CommandApplicationCapability::Validate,
];

fn admission(
    capability: CommandApplicationCapability,
    admitted: bool,
) -> SemanticCommandApplicationAdmission {
    if admitted {
        SemanticCommandApplicationAdmission::Admitted { capability }
    } else {
        SemanticCommandApplicationAdmission::Unsupported {
            requested: capability,
        }
    }
}

fn expected(
    previous_capability: CommandApplicationCapability,
    previous_admitted: bool,
    current_capability: CommandApplicationCapability,
    current_admitted: bool,
) -> Option<AutonomousProgressEvidenceClass> {
    if !previous_admitted
        && current_admitted
        && previous_capability == current_capability
    {
        Some(AutonomousProgressEvidenceClass::NewAdmittedAuthorityOrContext)
    } else {
        None
    }
}

#[test]
fn all_64_application_admission_transitions_match_exact_progress_rule() {
    let mut combinations = 0_usize;
    let mut progress = 0_usize;
    for previous_capability in CAPABILITIES {
        for previous_admitted in [false, true] {
            for current_capability in CAPABILITIES {
                for current_admitted in [false, true] {
                    let expected = expected(
                        previous_capability,
                        previous_admitted,
                        current_capability,
                        current_admitted,
                    );
                    assert_eq!(
                        autonomous_application_capability_progress_evidence(
                            admission(previous_capability, previous_admitted),
                            admission(current_capability, current_admitted),
                        ),
                        expected
                    );
                    combinations += 1;
                    if expected.is_some() {
                        progress += 1;
                    }
                }
            }
        }
    }
    assert_eq!(combinations, 64);
    assert_eq!(progress, 4);
}
