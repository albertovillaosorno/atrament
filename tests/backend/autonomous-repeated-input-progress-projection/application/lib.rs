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
//   - Exhaustive repeated-input state comparison evidence.
// - Must-Not:
//   - Parse diagnostics, infer admission, retry, mutate, stop, or run a loop.
// - Allows:
//   - Inputs: Equality/mismatch of three frozen axes plus new-admission fact.
//   - Outputs: Exact non-progress-evidence-or-none assertion for all 16 states.
//   - Side effects: None.
// - Split-When:
//   - Real diagnostic fingerprint integration needs independent fixtures.
// - Merge-When:
//   - Coordinator tests fully subsume this finite state-comparison oracle.
// - Summary:
//   - Proves only exact repeated inputs without admission count as
//     non-progress.
// - Description:
//   - A change to any authoritative axis prevents repeated-input evidence.
// - Usage:
//   - Cross all same/different axes and admission states independently.
// - Defaults:
//   - No evidence here grants continuation or selects a terminal outcome.
//
use atrament_autonomous_progress_evidence::AutonomousProgressEvidenceClass;
use atrament_autonomous_repeated_input_progress_projection::{
    AutonomousIterationInputs, autonomous_repeated_input_progress_evidence,
};

#[test]
fn all_16_repeated_input_states_match_frozen_non_progress_rule() {
    let previous = AutonomousIterationInputs {
        blocking_evidence: 1_u8,
        intent: 1_u16,
        revision: 1_u32,
    };
    let mut combinations = 0_usize;
    let mut repeated = 0_usize;
    for same_blocking_evidence in [false, true] {
        for same_intent in [false, true] {
            for same_revision in [false, true] {
                for new_admission in [false, true] {
                    let current = AutonomousIterationInputs {
                        blocking_evidence: if same_blocking_evidence {
                            1
                        } else {
                            2
                        },
                        intent: if same_intent { 1 } else { 2 },
                        revision: if same_revision { 1 } else { 2 },
                    };
                    let expected = if same_blocking_evidence
                        && same_intent
                        && same_revision
                        && !new_admission
                    {
                        Some(
                            AutonomousProgressEvidenceClass::
                                RepeatedSameInputsWithoutNewAdmission,
                        )
                    } else {
                        None
                    };
                    assert_eq!(
                        autonomous_repeated_input_progress_evidence(
                            &previous,
                            &current,
                            new_admission,
                        ),
                        expected
                    );
                    combinations += 1;
                    if expected.is_some() {
                        repeated += 1;
                    }
                }
            }
        }
    }
    assert_eq!(combinations, 16);
    assert_eq!(repeated, 1);
}
