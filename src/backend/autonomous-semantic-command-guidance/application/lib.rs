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
//   - Frozen autonomous next-step guidance for semantic command result classes.
// - Must-Not:
//   - Run an agent loop, choose budgets, retry automatically, widen authority,
//     normalize requests, schedule work, execute outputs, or control hardware.
// - Allows:
//   - Inputs: One completed semantic command result class.
//   - Outputs: One frozen guidance class when the contracts define it exactly.
//   - Side effects: None.
// - Split-When:
//   - Executable autonomous workflow orchestration gains application authority.
// - Merge-When:
//   - A final automation coordinator directly owns all frozen result guidance.
// - Summary:
//   - Prevents blind retry without inventing policy for under-specified cases.
// - Description:
//   - Classifies only result classes with explicit frozen automation guidance.
// - Usage:
//   - Branch on typed application results before considering another mutation.
// - Defaults:
//   - Unclassified results require owning workflow policy, not guessed action.
//

//! Frozen partial guidance for autonomous semantic-command handling.

use atrament_semantic_notebook_port::SemanticCommandResultClass;

/// Frozen automation guidance shared by local CLI/MCP semantic edit loops.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutonomousSemanticCommandGuidance {
    /// Negotiate or obtain a compatible protocol/application capability.
    CompatibilityNegotiation,
    /// Correct command content/dependencies or obtain a different context.
    CorrectRequestOrContext,
    /// Correct retry-identity bookkeeping or stop this attempted workflow.
    CorrectRetryBookkeepingOrStop,
    /// Continue from the revision reported by the completed operation.
    ContinueFromReportedRevision,
    /// Inspect current authority and obtain fresh command context as required.
    FreshInspectionOrContext,
    /// Use an admitted smaller or intentionally alternate workflow.
    SmallerOrAlternateWorkflow,
    /// Stop unresolved or ask the owning caller for an explicitly broader flow.
    StopOrRequestBroaderWorkflow,
}

/// Return frozen automation guidance when the contracts define one exact
/// branch.
///
/// Successful validation, cancellation before commit, and internal known-no-
/// commit failure intentionally return `None`: their owning workflow must
/// decide
/// what to do next from goal, diagnostics, cancellation intent, and host
/// policy.
#[must_use]
pub const fn autonomous_semantic_command_guidance(
    result: SemanticCommandResultClass,
) -> Option<AutonomousSemanticCommandGuidance> {
    match result {
        SemanticCommandResultClass::Applied
        | SemanticCommandResultClass::IdempotentReplay
        | SemanticCommandResultClass::NoOp => {
            Some(
                AutonomousSemanticCommandGuidance::ContinueFromReportedRevision,
            )
        },
        SemanticCommandResultClass::CommandContextMismatch
        | SemanticCommandResultClass::StaleBase => {
            Some(AutonomousSemanticCommandGuidance::FreshInspectionOrContext)
        },
        SemanticCommandResultClass::DependencyGraphRejection
        | SemanticCommandResultClass::SemanticValidationRejection
        | SemanticCommandResultClass::WritableScopeViolation => {
            Some(AutonomousSemanticCommandGuidance::CorrectRequestOrContext)
        },
        SemanticCommandResultClass::ResourceLimitRejection => {
            Some(AutonomousSemanticCommandGuidance::SmallerOrAlternateWorkflow)
        },
        SemanticCommandResultClass::RetryConflict => Some(
            AutonomousSemanticCommandGuidance::CorrectRetryBookkeepingOrStop,
        ),
        SemanticCommandResultClass::UnrepresentableOrUnresolved => Some(
            AutonomousSemanticCommandGuidance::StopOrRequestBroaderWorkflow,
        ),
        SemanticCommandResultClass::UnsupportedProtocolOrCapability => {
            Some(AutonomousSemanticCommandGuidance::CompatibilityNegotiation)
        },
        SemanticCommandResultClass::CancelledBeforeCommit
        | SemanticCommandResultClass::InternalFailureKnownNoCommit
        | SemanticCommandResultClass::SuccessfulValidation => None,
    }
}
