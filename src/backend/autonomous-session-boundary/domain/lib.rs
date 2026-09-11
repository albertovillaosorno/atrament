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
//   - Transport-neutral autonomous state ownership across session restart.
// - Must-Not:
//   - Persist Atrament loop state, authenticate callers, reconnect adapters,
//     recover old retry results, recreate command contexts, or execute Inspect.
// - Allows:
//   - Inputs: One frozen session-state ownership class.
//   - Outputs: Session-end disposition and fresh-session bootstrap vocabulary.
//   - Side effects: None.
// - Split-When:
//   - Restart orchestration or safe reattachment gains executable authority.
// - Merge-When:
//   - One session coordinator owns disposal plus fresh-session bootstrap.
// - Summary:
//   - Invalidates Atrament-owned autonomous state across application sessions.
// - Description:
//   - Keeps external caller workflow memory outside Atrament persistence
//     claims.
// - Usage:
//   - Apply at session end and use fresh bootstrap steps after process restart.
// - Defaults:
//   - Prior command/retry/admission state is never reusable in a new session.
//

//! Autonomous state ownership and bootstrap semantics across session restart.

/// State class relevant to one autonomous session boundary.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AutonomousSessionStateClass {
    /// Backend-generated command context owned by the active session.
    CommandContext,
    /// Caller workflow state maintained explicitly outside Atrament.
    ExternalCallerWorkflowState,
    /// Retry-result/recovery state owned by the active session.
    RetryRecoveryState,
    /// Inbound session admission owned by the active application session.
    SessionAdmission,
}

/// Ownership disposition when the active Atrament session ends.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutonomousSessionEndDisposition {
    /// This state is outside Atrament's session-state ownership.
    OutsideAtramentSessionOwnership,
    /// This Atrament-owned state is invalid after session end.
    InvalidatedWithSession,
}

/// Required bootstrap class for a fresh Atrament autonomous session.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AutonomousFreshSessionStep {
    /// Discover release/live capability state again.
    CapabilityDiscovery,
    /// Inspect accepted state again before deriving new command context.
    Inspect,
}

/// Fresh-session bootstrap steps frozen by the autonomous-loop contract.
pub const AUTONOMOUS_FRESH_SESSION_STEPS: [AutonomousFreshSessionStep; 2] = [
    AutonomousFreshSessionStep::CapabilityDiscovery,
    AutonomousFreshSessionStep::Inspect,
];

/// Project one state class into its session-end ownership disposition.
#[must_use]
pub const fn autonomous_session_end_disposition(
    state: AutonomousSessionStateClass,
) -> AutonomousSessionEndDisposition {
    match state {
        AutonomousSessionStateClass::ExternalCallerWorkflowState => {
            AutonomousSessionEndDisposition::OutsideAtramentSessionOwnership
        },
        AutonomousSessionStateClass::CommandContext
        | AutonomousSessionStateClass::RetryRecoveryState
        | AutonomousSessionStateClass::SessionAdmission => {
            AutonomousSessionEndDisposition::InvalidatedWithSession
        },
    }
}
