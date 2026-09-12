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
//   - Projection from already-qualified required-capability availability to the
//     frozen unavailable-capability terminal outcome.
// - Must-Not:
//   - Discover capabilities, decide goal requirements, negotiate compatibility,
//     retry, mutate state, or infer terminal unavailability from one rejection.
// - Allows:
//   - Inputs: Owner-qualified availability for one capability required by the
//     admitted bounded goal.
//   - Outputs: Unavailable-capability terminal stop only for qualified
//     unavailability.
//   - Side effects: None.
// - Split-When:
//   - Goal-requirement or capability-discovery comparison gains authority here.
// - Merge-When:
//   - One autonomous coordinator directly owns required-capability stop logic.
// - Summary:
//   - Projects qualified required-capability unavailability without guessing
//     it.
// - Description:
//   - Keeps ordinary unsupported/recoverable capability results nonterminal
//     until an owning boundary establishes required unavailability.
// - Usage:
//   - Apply after goal and capability owners qualify one required capability.
// - Defaults:
//   - An admitted required capability produces no terminal outcome on this
//     axis.
//

//! Required-capability availability projection into autonomous terminal state.

use atrament_autonomous_goal_outcome::AutonomousGoalTerminalClass;

/// Already-qualified availability of one capability required by the bounded
/// goal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutonomousRequiredCapabilityAvailability {
    /// The required capability is admitted for the bounded workflow.
    Admitted,
    /// The owning boundary established that the required capability is
    /// unavailable.
    Unavailable,
}

/// Project qualified required-capability availability into its terminal stop.
///
/// This function does not decide whether a capability is required and does not
/// treat one unsupported protocol/capability result as terminal unavailability.
/// Those facts must be established by their owning goal/discovery boundaries.
#[must_use]
pub const fn autonomous_unavailable_capability_terminal_outcome(
    availability: AutonomousRequiredCapabilityAvailability,
) -> Option<AutonomousGoalTerminalClass> {
    match availability {
        AutonomousRequiredCapabilityAvailability::Admitted => None,
        AutonomousRequiredCapabilityAvailability::Unavailable => {
            Some(AutonomousGoalTerminalClass::StoppedUnavailableCapability)
        }
    }
}
