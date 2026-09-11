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
//   - Exhaustive evidence for satisfied No-op completion projection.
// - Must-Not:
//   - Infer satisfaction, execute output, retry, mutate state, or run a loop.
// - Allows:
//   - Inputs: All result, semantic, and requested-output state combinations.
//   - Outputs: Exact completion-or-none assertion for all 90 combinations.
//   - Side effects: None.
// - Split-When:
//   - Stateful satisfied-intent fixtures gain independent execution evidence.
// - Merge-When:
//   - Coordinator tests fully subsume this finite projection oracle.
// - Summary:
//   - Proves only qualified No-op can complete through this boundary.
// - Description:
//   - Crosses 15 semantic results, two semantic states, and three output
//     states.
// - Usage:
//   - Compare every cross-product entry with an independent expected mapping.
// - Defaults:
//   - Every non-No-op result stays nonterminal on this projection.
//
use atrament_autonomous_completion_admission::{
    AutonomousRequestedOutputCompletion, AutonomousSemanticGoalCompletion,
};
use atrament_autonomous_goal_outcome::AutonomousGoalTerminalClass;
use atrament_autonomous_no_op_completion_projection::
    autonomous_no_op_completion_terminal_outcome;
use atrament_semantic_notebook_port::SemanticCommandResultClass;

const OUTPUTS: [AutonomousRequestedOutputCompletion; 3] = [
    AutonomousRequestedOutputCompletion::Complete,
    AutonomousRequestedOutputCompletion::Incomplete,
    AutonomousRequestedOutputCompletion::NotRequested,
];
const RESULTS: [SemanticCommandResultClass; 15] = [
    SemanticCommandResultClass::Applied,
    SemanticCommandResultClass::CancelledBeforeCommit,
    SemanticCommandResultClass::CommandContextMismatch,
    SemanticCommandResultClass::DependencyGraphRejection,
    SemanticCommandResultClass::IdempotentReplay,
    SemanticCommandResultClass::InternalFailureKnownNoCommit,
    SemanticCommandResultClass::NoOp,
    SemanticCommandResultClass::ResourceLimitRejection,
    SemanticCommandResultClass::RetryConflict,
    SemanticCommandResultClass::SemanticValidationRejection,
    SemanticCommandResultClass::StaleBase,
    SemanticCommandResultClass::SuccessfulValidation,
    SemanticCommandResultClass::UnrepresentableOrUnresolved,
    SemanticCommandResultClass::UnsupportedProtocolOrCapability,
    SemanticCommandResultClass::WritableScopeViolation,
];
const SEMANTICS: [AutonomousSemanticGoalCompletion; 2] = [
    AutonomousSemanticGoalCompletion::Satisfied,
    AutonomousSemanticGoalCompletion::Unsatisfied,
];

fn expected(
    result: SemanticCommandResultClass,
    semantic: AutonomousSemanticGoalCompletion,
    output: AutonomousRequestedOutputCompletion,
) -> Option<AutonomousGoalTerminalClass> {
    if result == SemanticCommandResultClass::NoOp
        && semantic == AutonomousSemanticGoalCompletion::Satisfied
        && output != AutonomousRequestedOutputCompletion::Incomplete
    {
        Some(AutonomousGoalTerminalClass::Completed)
    } else {
        None
    }
}

#[test]
fn all_90_no_op_completion_combinations_match_frozen_fixture() {
    let mut combinations = 0_usize;
    let mut completions = 0_usize;
    for result in RESULTS {
        for semantic in SEMANTICS {
            for output in OUTPUTS {
                let expected = expected(result, semantic, output);
                assert_eq!(
                    autonomous_no_op_completion_terminal_outcome(
                        result, semantic, output
                    ),
                    expected
                );
                combinations += 1;
                if expected.is_some() {
                    completions += 1;
                }
            }
        }
    }
    assert_eq!(combinations, 90);
    assert_eq!(completions, 2);
}
