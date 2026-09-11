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
//   - Exhaustive evidence for frozen autonomous completion admission.
// - Must-Not:
//   - Infer goal satisfaction, execute output, construct receipts, or run
//     loops.
// - Allows:
//   - Inputs: All semantic-satisfaction and output-completion combinations.
//   - Outputs: Exact terminal-completion-or-none assertion for every pair.
//   - Side effects: None.
// - Split-When:
//   - Stateful completion proof requires independent fixtures.
// - Merge-When:
//   - Executable coordinator tests fully subsume this admission oracle.
// - Summary:
//   - Pins the semantic-and-output conjunction required for completion.
// - Description:
//   - Crosses two semantic states with three requested-output states.
// - Usage:
//   - Compare all six combinations with an independent expected mapping.
// - Defaults:
//   - Only satisfied semantic state with complete or absent output completes.
//
use atrament_autonomous_completion_admission::{
    AutonomousRequestedOutputCompletion,
    AutonomousRequestedOutputItemCompletion, AutonomousSemanticGoalCompletion,
    autonomous_completion_terminal_outcome,
    autonomous_requested_outputs_completion,
};
use atrament_autonomous_goal_outcome::AutonomousGoalTerminalClass;

const OUTPUTS: [AutonomousRequestedOutputCompletion; 3] = [
    AutonomousRequestedOutputCompletion::Complete,
    AutonomousRequestedOutputCompletion::Incomplete,
    AutonomousRequestedOutputCompletion::NotRequested,
];
const SEMANTICS: [AutonomousSemanticGoalCompletion; 2] = [
    AutonomousSemanticGoalCompletion::Satisfied,
    AutonomousSemanticGoalCompletion::Unsatisfied,
];

fn expected(
    semantic: AutonomousSemanticGoalCompletion,
    output: AutonomousRequestedOutputCompletion,
) -> Option<AutonomousGoalTerminalClass> {
    if semantic == AutonomousSemanticGoalCompletion::Satisfied
        && output != AutonomousRequestedOutputCompletion::Incomplete
    {
        Some(AutonomousGoalTerminalClass::Completed)
    } else {
        None
    }
}

#[test]
fn all_six_completion_combinations_match_frozen_conjunction() {
    let mut combinations = 0_usize;
    let mut completions = 0_usize;
    for semantic in SEMANTICS {
        for output in OUTPUTS {
            let expected = expected(semantic, output);
            assert_eq!(
                autonomous_completion_terminal_outcome(semantic, output),
                expected
            );
            combinations += 1;
            if expected.is_some() {
                completions += 1;
            }
        }
    }
    assert_eq!(combinations, 6);
    assert_eq!(completions, 2);
}

#[test]
fn requested_output_aggregation_requires_every_requested_item_complete() {
    use AutonomousRequestedOutputItemCompletion::{Complete, Incomplete};

    assert_eq!(
        autonomous_requested_outputs_completion(&[]),
        AutonomousRequestedOutputCompletion::NotRequested
    );

    let mut cases = 1_usize;
    for first in [Complete, Incomplete] {
        for second in [Complete, Incomplete] {
            for third in [Complete, Incomplete] {
                for fourth in [Complete, Incomplete] {
                    let outputs = [first, second, third, fourth];
                    let expected = if outputs.contains(&Incomplete) {
                        AutonomousRequestedOutputCompletion::Incomplete
                    } else {
                        AutonomousRequestedOutputCompletion::Complete
                    };
                    assert_eq!(
                        autonomous_requested_outputs_completion(&outputs),
                        expected
                    );
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 17);
}
