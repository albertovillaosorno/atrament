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
//   - Projection from stale/context-drift results plus qualified refreshed
//     completion evidence to successful autonomous completion.
// - Must-Not:
//   - Refresh state, silently rebase, infer goal satisfaction, acquire command
//     context, execute output, retry, mutate state, or run an agent loop.
// - Allows:
//   - Inputs: One semantic result plus refreshed semantic/output completion.
//   - Outputs: Completed only for stale/context-drift results with admitted
//     completion evidence.
//   - Side effects: None.
// - Split-When:
//   - Stateful refresh or satisfaction detection gains authority here.
// - Merge-When:
//   - One autonomous coordinator directly owns the refreshed-state fixture.
// - Summary:
//   - Lets refreshed satisfaction stop a stale historical edit safely.
// - Description:
//   - Keeps refresh-before-completion distinct from silent rebasing.
// - Usage:
//   - Apply only after fresh inspection/context qualification is complete.
// - Defaults:
//   - Other results or incomplete refreshed goals yield no terminal outcome.
//

//! Refreshed stale/context-drift projection into autonomous completion.

use atrament_autonomous_completion_admission::{
    AutonomousRequestedOutputCompletion, AutonomousSemanticGoalCompletion,
    autonomous_completion_terminal_outcome,
};
use atrament_autonomous_goal_outcome::AutonomousGoalTerminalClass;
use atrament_semantic_notebook_port::SemanticCommandResultClass;

/// Project refreshed satisfaction after stale/context drift into completion.
#[must_use]
pub const fn autonomous_refreshed_state_completion_terminal_outcome(
    result: SemanticCommandResultClass,
    semantic: AutonomousSemanticGoalCompletion,
    output: AutonomousRequestedOutputCompletion,
) -> Option<AutonomousGoalTerminalClass> {
    match result {
        SemanticCommandResultClass::CommandContextMismatch
        | SemanticCommandResultClass::StaleBase => {
            autonomous_completion_terminal_outcome(semantic, output)
        },
        SemanticCommandResultClass::Applied
        | SemanticCommandResultClass::CancelledBeforeCommit
        | SemanticCommandResultClass::DependencyGraphRejection
        | SemanticCommandResultClass::IdempotentReplay
        | SemanticCommandResultClass::InternalFailureKnownNoCommit
        | SemanticCommandResultClass::NoOp
        | SemanticCommandResultClass::ResourceLimitRejection
        | SemanticCommandResultClass::RetryConflict
        | SemanticCommandResultClass::SemanticValidationRejection
        | SemanticCommandResultClass::SuccessfulValidation
        | SemanticCommandResultClass::UnrepresentableOrUnresolved
        | SemanticCommandResultClass::UnsupportedProtocolOrCapability
        | SemanticCommandResultClass::WritableScopeViolation => None,
    }
}
