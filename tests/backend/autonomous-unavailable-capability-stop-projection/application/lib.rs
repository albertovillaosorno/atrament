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
//   - Exhaustive required-capability availability projection evidence.
// - Must-Not:
//   - Discover capabilities, infer goal requirements, negotiate, retry, or
//     mutate state.
// - Allows:
//   - Inputs: Both already-qualified required-capability availability states.
//   - Outputs: Exact unavailable-terminal-or-none assertions.
//   - Side effects: None.
// - Split-When:
//   - Stateful capability discovery or goal qualification gains fixtures here.
// - Merge-When:
//   - Coordinator tests fully subsume this finite projection oracle.
// - Summary:
//   - Proves only qualified required-capability unavailability is terminal.
// - Description:
//   - Admitted capability availability remains nonterminal on this stop axis.
// - Usage:
//   - Exercise both finite states against the frozen terminal vocabulary.
// - Defaults:
//   - No unsupported-result interpretation is performed by this fixture.
//
use atrament_autonomous_goal_outcome::AutonomousGoalTerminalClass;
use atrament_autonomous_unavailable_capability_stop_projection::{
    AutonomousRequiredCapabilityAvailability,
    autonomous_unavailable_capability_terminal_outcome,
};

#[test]
fn only_qualified_required_capability_unavailability_is_terminal() {
    let cases = [
        (AutonomousRequiredCapabilityAvailability::Admitted, None),
        (
            AutonomousRequiredCapabilityAvailability::Unavailable,
            Some(AutonomousGoalTerminalClass::StoppedUnavailableCapability),
        ),
    ];
    for (availability, expected) in cases {
        assert_eq!(
            autonomous_unavailable_capability_terminal_outcome(availability),
            expected,
        );
    }
}
