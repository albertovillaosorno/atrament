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
//   - Exhaustive evidence for autonomous restart state ownership/bootstrap.
// - Must-Not:
//   - Persist state, authenticate, reconnect, recover retry results, or
//     Inspect.
// - Allows:
//   - Inputs: Every frozen state class and fresh bootstrap step.
//   - Outputs: Exact disposal/ownership and bootstrap-order assertions.
//   - Side effects: None.
// - Split-When:
//   - Executable restart fixtures require independent session integration
//     tests.
// - Merge-When:
//   - Session-coordinator tests fully subsume this static restart evidence.
// - Summary:
//   - Pins three invalidations, one external owner, and two fresh-session
//     steps.
// - Description:
//   - Proves restart cannot reuse command, retry, or admission state.
// - Usage:
//   - Compare every state class and the exact fresh bootstrap sequence.
// - Defaults:
//   - Fresh sessions begin from capability discovery and Inspect.
//
use atrament_autonomous_session_boundary::{
    AUTONOMOUS_FRESH_SESSION_STEPS, AutonomousFreshSessionStep,
    AutonomousSessionEndDisposition, AutonomousSessionStateClass,
    autonomous_session_end_disposition,
};

const STATES: [AutonomousSessionStateClass; 4] = [
    AutonomousSessionStateClass::CommandContext,
    AutonomousSessionStateClass::ExternalCallerWorkflowState,
    AutonomousSessionStateClass::RetryRecoveryState,
    AutonomousSessionStateClass::SessionAdmission,
];

fn expected(
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

#[test]
fn all_four_state_classes_preserve_restart_ownership() {
    let mut invalidated = 0_usize;
    let mut external = 0_usize;
    for state in STATES {
        let expected = expected(state);
        assert_eq!(autonomous_session_end_disposition(state), expected);
        match expected {
            AutonomousSessionEndDisposition::InvalidatedWithSession => {
                invalidated += 1;
            },
            AutonomousSessionEndDisposition::OutsideAtramentSessionOwnership =>
            {
                external += 1;
            },
        }
    }
    assert_eq!(invalidated, 3);
    assert_eq!(external, 1);
}

#[test]
fn fresh_session_restarts_from_discovery_then_inspect() {
    assert_eq!(
        AUTONOMOUS_FRESH_SESSION_STEPS,
        [
            AutonomousFreshSessionStep::CapabilityDiscovery,
            AutonomousFreshSessionStep::Inspect,
        ],
    );
}
