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
//   - Exhaustive evidence for the five frozen autonomous terminal classes.
// - Must-Not:
//   - Decide goal satisfaction, execute work, retry, or construct receipts.
// - Allows:
//   - Inputs: Every frozen terminal class.
//   - Outputs: Exact completion/stopped disposition for every class.
//   - Side effects: None.
// - Split-When:
//   - Stateful completion/stop fixtures require separate evidence.
// - Merge-When:
//   - Executable autonomous coordinator tests subsume this static taxonomy.
// - Summary:
//   - Pins one successful completion and four non-completion stop classes.
// - Description:
//   - Proves no frozen stop reason is mislabeled as goal completion.
// - Usage:
//   - Compare all five terminal classes with an independent exact oracle.
// - Defaults:
//   - No nonterminal state is represented by this terminal-only domain.
//
use atrament_autonomous_goal_outcome::{
    AutonomousGoalTerminalClass, AutonomousGoalTerminalDisposition,
    autonomous_goal_terminal_disposition,
};

const TERMINALS: [AutonomousGoalTerminalClass; 5] = [
    AutonomousGoalTerminalClass::Completed,
    AutonomousGoalTerminalClass::StoppedExhaustedBudget,
    AutonomousGoalTerminalClass::StoppedStableBlockingFailure,
    AutonomousGoalTerminalClass::StoppedUnavailableCapability,
    AutonomousGoalTerminalClass::StoppedUnresolvedEvidence,
];

fn expected(
    terminal: AutonomousGoalTerminalClass,
) -> AutonomousGoalTerminalDisposition {
    match terminal {
        AutonomousGoalTerminalClass::Completed => {
            AutonomousGoalTerminalDisposition::SuccessfulCompletion
        },
        AutonomousGoalTerminalClass::StoppedExhaustedBudget
        | AutonomousGoalTerminalClass::StoppedStableBlockingFailure
        | AutonomousGoalTerminalClass::StoppedUnavailableCapability
        | AutonomousGoalTerminalClass::StoppedUnresolvedEvidence => {
            AutonomousGoalTerminalDisposition::StoppedWithoutCompletion
        },
    }
}

#[test]
fn all_five_terminal_classes_preserve_completion_distinction() {
    let mut completed = 0_usize;
    let mut stopped = 0_usize;
    for terminal in TERMINALS {
        let expected = expected(terminal);
        assert_eq!(autonomous_goal_terminal_disposition(terminal), expected);
        match expected {
            AutonomousGoalTerminalDisposition::StoppedWithoutCompletion => {
                stopped += 1;
            },
            AutonomousGoalTerminalDisposition::SuccessfulCompletion => {
                completed += 1;
            },
        }
    }
    assert_eq!(completed, 1);
    assert_eq!(stopped, 4);
}
