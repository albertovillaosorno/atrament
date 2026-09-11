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
//   - Exhaustive evidence for frozen partial semantic-history automation
//     advice.
// - Must-Not:
//   - Traverse history, retry, schedule loops, or execute cancellation.
// - Allows:
//   - Inputs: All six frozen semantic history result classes.
//   - Outputs: Exact guidance-or-unclassified assertions for every result.
//   - Side effects: None.
// - Split-When:
//   - Stateful autonomous history fixtures require independent evidence.
// - Merge-When:
//   - Executable loop parity tests fully subsume this structural
//     classification.
// - Summary:
//   - Pins four explicit branches and two deliberately unclassified cases.
// - Description:
//   - Uses an independent exhaustive oracle over the history result vocabulary.
// - Usage:
//   - Compare every result with the frozen autonomous history next step.
// - Defaults:
//   - Cancellation and known-no-commit failure stay unclassified.
//
use atrament_autonomous_history_guidance::{
    AutonomousHistoryGuidance, autonomous_history_guidance,
};
use atrament_semantic_history_result::SemanticHistoryResultClass;

const RESULTS: [SemanticHistoryResultClass; 6] = [
    SemanticHistoryResultClass::CancelledBeforeCommit,
    SemanticHistoryResultClass::HistoryBoundary,
    SemanticHistoryResultClass::IdempotentReplay,
    SemanticHistoryResultClass::KnownNoCommitFailure,
    SemanticHistoryResultClass::StaleCurrentRevision,
    SemanticHistoryResultClass::Traversed,
];

fn expected(
    result: SemanticHistoryResultClass,
) -> Option<AutonomousHistoryGuidance> {
    match result {
        SemanticHistoryResultClass::HistoryBoundary => {
            Some(AutonomousHistoryGuidance::StopAtHistoryBoundary)
        },
        SemanticHistoryResultClass::IdempotentReplay => {
            Some(AutonomousHistoryGuidance::RecoveredPriorCompletion)
        },
        SemanticHistoryResultClass::StaleCurrentRevision => {
            Some(AutonomousHistoryGuidance::FreshInspection)
        },
        SemanticHistoryResultClass::Traversed => {
            Some(AutonomousHistoryGuidance::ContinueFromReportedRevision)
        },
        SemanticHistoryResultClass::CancelledBeforeCommit
        | SemanticHistoryResultClass::KnownNoCommitFailure => None,
    }
}

#[test]
fn all_six_history_results_match_frozen_partial_guidance() {
    let mut classified = 0_usize;
    for result in RESULTS {
        let expected = expected(result);
        assert_eq!(autonomous_history_guidance(result), expected);
        if expected.is_some() {
            classified += 1;
        }
    }
    assert_eq!(classified, 4);
}
