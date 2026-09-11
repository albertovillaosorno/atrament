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
//   - Evidence that generic autonomous success never accumulates physical
//     grant.
// - Must-Not:
//   - Execute hardware, admit devices, inspect physical state, or choose
//     policy.
// - Allows:
//   - Inputs: Empty, single, repeated, and complete generic histories crossed
//     with all eight physical action classes.
//   - Outputs: Exact denial assertions for every fixture/action pair.
//   - Side effects: None.
// - Split-When:
//   - Physical-device admission gains independent executable fixtures.
// - Merge-When:
//   - Physical admission tests fully subsume generic no-accumulation evidence.
// - Summary:
//   - Pins fail-closed authority across repeated semantic/output successes.
// - Description:
//   - Proves repeated Plan or mixed successes cannot manufacture device grant.
// - Usage:
//   - Cross representative histories with every frozen physical action class.
// - Defaults:
//   - Every case is not authorized.
//
use atrament_autonomous_physical_boundary::{
    AutonomousGenericWorkflowClass, AutonomousPhysicalActionClass,
    AutonomousPhysicalAuthority, autonomous_generic_workflow_physical_authority,
};

const ACTIONS: [AutonomousPhysicalActionClass; 8] = [
    AutonomousPhysicalActionClass::Arm,
    AutonomousPhysicalActionClass::Cancel,
    AutonomousPhysicalActionClass::Connect,
    AutonomousPhysicalActionClass::Home,
    AutonomousPhysicalActionClass::Pause,
    AutonomousPhysicalActionClass::Resume,
    AutonomousPhysicalActionClass::SafeStop,
    AutonomousPhysicalActionClass::Start,
];

#[test]
fn generic_success_history_never_accumulates_physical_authority() {
    let histories: [&[AutonomousGenericWorkflowClass]; 5] = [
        &[],
        &[AutonomousGenericWorkflowClass::Apply],
        &[AutonomousGenericWorkflowClass::Plan],
        &[
            AutonomousGenericWorkflowClass::Apply,
            AutonomousGenericWorkflowClass::Render,
            AutonomousGenericWorkflowClass::Export,
            AutonomousGenericWorkflowClass::Plan,
        ],
        &[
            AutonomousGenericWorkflowClass::Plan,
            AutonomousGenericWorkflowClass::Plan,
            AutonomousGenericWorkflowClass::Plan,
            AutonomousGenericWorkflowClass::Plan,
        ],
    ];
    let mut cases = 0_usize;
    for history in histories {
        for action in ACTIONS {
            assert_eq!(
                autonomous_generic_workflow_physical_authority(history, action),
                AutonomousPhysicalAuthority::NotAuthorized,
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 40);
}
