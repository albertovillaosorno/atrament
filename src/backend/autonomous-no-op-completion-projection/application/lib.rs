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
//   - Projection from semantic-command No-op plus qualified completion evidence
//     to successful autonomous completion.
// - Must-Not:
//   - Infer goal satisfaction, reinterpret other result classes as No-op,
//     execute output, construct receipts, retry, mutate state, or run a loop.
// - Allows:
//   - Inputs: One semantic result plus qualified semantic/output completion.
//   - Outputs: Completed terminal outcome only for admitted No-op completion.
//   - Side effects: None.
// - Split-When:
//   - Stateful No-op equivalence or completion proof gains authority here.
// - Merge-When:
//   - One autonomous coordinator owns this exact satisfied-intent fixture.
// - Summary:
//   - Turns a qualified satisfied No-op into successful completion exactly
//     once.
// - Description:
//   - Keeps No-op non-progress distinct from successful goal completion proof.
// - Usage:
//   - Apply after the owning boundary qualifies accepted-state satisfaction.
// - Defaults:
//   - Non-No-op results and incomplete goals yield no terminal completion.
//

//! Satisfied No-op projection into frozen autonomous completion semantics.

use atrament_autonomous_completion_admission::{
    AutonomousRequestedOutputCompletion, AutonomousSemanticGoalCompletion,
    autonomous_completion_terminal_outcome,
};
use atrament_autonomous_goal_outcome::AutonomousGoalTerminalClass;
use atrament_semantic_notebook_port::SemanticCommandResultClass;

/// Project only a qualified satisfied No-op into successful completion.
#[must_use]
pub const fn autonomous_no_op_completion_terminal_outcome(
    result: SemanticCommandResultClass,
    semantic: AutonomousSemanticGoalCompletion,
    output: AutonomousRequestedOutputCompletion,
) -> Option<AutonomousGoalTerminalClass> {
    match result {
        SemanticCommandResultClass::NoOp => {
            autonomous_completion_terminal_outcome(semantic, output)
        },
        SemanticCommandResultClass::Applied
        | SemanticCommandResultClass::CancelledBeforeCommit
        | SemanticCommandResultClass::CommandContextMismatch
        | SemanticCommandResultClass::DependencyGraphRejection
        | SemanticCommandResultClass::IdempotentReplay
        | SemanticCommandResultClass::InternalFailureKnownNoCommit
        | SemanticCommandResultClass::ResourceLimitRejection
        | SemanticCommandResultClass::RetryConflict
        | SemanticCommandResultClass::SemanticValidationRejection
        | SemanticCommandResultClass::StaleBase
        | SemanticCommandResultClass::SuccessfulValidation
        | SemanticCommandResultClass::UnrepresentableOrUnresolved
        | SemanticCommandResultClass::UnsupportedProtocolOrCapability
        | SemanticCommandResultClass::WritableScopeViolation => None,
    }
}
