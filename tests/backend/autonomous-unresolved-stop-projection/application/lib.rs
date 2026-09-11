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
//   - Exhaustive semantic-result by unresolved-owner-resolution evidence.
// - Must-Not:
//   - Broaden workflows, infer caller choice, retry, mutate, or run an agent.
// - Allows:
//   - Inputs: All 15 semantic result classes crossed with both owner choices.
//   - Outputs: Exact unresolved-terminal-or-none assertion for all 30 states.
//   - Side effects: None.
// - Split-When:
//   - Stateful escalation negotiation needs independent executable fixtures.
// - Merge-When:
//   - Coordinator tests fully subsume this finite owner-resolution oracle.
// - Summary:
//   - Proves only explicit unresolved stop creates an unresolved terminal
//     result.
// - Description:
//   - Broader-workflow request never fabricates completion or terminal stop.
// - Usage:
//   - Compare every result/owner-choice pair with an independent exact oracle.
// - Defaults:
//   - Unrelated results remain outside this unresolved-specific projection.
//
use atrament_autonomous_goal_outcome::AutonomousGoalTerminalClass;
use atrament_autonomous_unresolved_stop_projection::{
    AutonomousUnresolvedResolution, autonomous_unresolved_terminal_outcome,
};
use atrament_semantic_notebook_port::SemanticCommandResultClass;

const RESOLUTIONS: [AutonomousUnresolvedResolution; 2] = [
    AutonomousUnresolvedResolution::RequestBroaderWorkflow,
    AutonomousUnresolvedResolution::StopUnresolved,
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

fn expected(
    result: SemanticCommandResultClass,
    resolution: AutonomousUnresolvedResolution,
) -> Option<AutonomousGoalTerminalClass> {
    if result == SemanticCommandResultClass::UnrepresentableOrUnresolved
        && resolution == AutonomousUnresolvedResolution::StopUnresolved
    {
        Some(AutonomousGoalTerminalClass::StoppedUnresolvedEvidence)
    } else {
        None
    }
}

#[test]
fn all_30_result_resolution_states_require_explicit_unresolved_stop() {
    let mut combinations = 0_usize;
    let mut terminal_stops = 0_usize;
    for result in RESULTS {
        for resolution in RESOLUTIONS {
            let expected = expected(result, resolution);
            assert_eq!(
                autonomous_unresolved_terminal_outcome(result, resolution),
                expected
            );
            combinations += 1;
            if expected.is_some() {
                terminal_stops += 1;
            }
        }
    }
    assert_eq!(combinations, 30);
    assert_eq!(terminal_stops, 1);
}
