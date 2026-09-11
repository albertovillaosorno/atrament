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
//   - Generic autonomous semantic/output steps versus physical-device
//     authority.
// - Must-Not:
//   - Connect, home, arm, start, pause, resume, cancel, safe-stop, inspect
//     device
//     state, accumulate permissions, or replace a physical-device contract.
// - Allows:
//   - Inputs: Any sequence of successful generic workflow classes and one
//     physical action class.
//   - Outputs: Exact denial of physical authority from generic workflow
//     history.
//   - Side effects: None.
// - Split-When:
//   - A separate admitted physical-device contract gains executable authority.
// - Merge-When:
//   - One physical admission owner directly proves this generic denial.
// - Summary:
//   - Prevents semantic/output success from accumulating hardware permission.
// - Description:
//   - Keeps generic automation device-neutral through Plan.
// - Usage:
//   - Check that generic workflow history never substitutes for physical admit.
// - Defaults:
//   - Empty or nonempty generic workflow history grants no physical authority.
//

//! Fail-closed physical boundary for generic autonomous semantic/output work.

/// Generic workflow class named by the autonomous physical-boundary contract.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AutonomousGenericWorkflowClass {
    /// Successful atomic semantic Apply.
    Apply,
    /// Successful explicit persistent Export.
    Export,
    /// Successful device-neutral Plan.
    Plan,
    /// Successful read-only Render.
    Render,
}

/// Physical action that remains behind a separate admitted device contract.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AutonomousPhysicalActionClass {
    /// Arm a physical device.
    Arm,
    /// Cancel a physical-device operation.
    Cancel,
    /// Connect to a physical device.
    Connect,
    /// Home a physical device.
    Home,
    /// Pause a physical-device operation.
    Pause,
    /// Resume a physical-device operation.
    Resume,
    /// Execute physical safe-stop behavior.
    SafeStop,
    /// Start a physical-device operation.
    Start,
}

/// Physical authority implied by generic autonomous workflow history.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutonomousPhysicalAuthority {
    /// Generic semantic/output history does not authorize this physical action.
    NotAuthorized,
}

/// Report physical authority implied by any generic autonomous workflow
/// history.
///
/// Successful Apply, Render, Export, or Plan entries never accumulate device
/// authority. A separate admitted physical-device contract and operator
/// boundary
/// are required before any physical action can become authorized.
#[must_use]
pub const fn autonomous_generic_workflow_physical_authority(
    _history: &[AutonomousGenericWorkflowClass],
    _action: AutonomousPhysicalActionClass,
) -> AutonomousPhysicalAuthority {
    AutonomousPhysicalAuthority::NotAuthorized
}
