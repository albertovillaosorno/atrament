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
//   - Exhaustive evidence for semantic-history progress projection.
// - Must-Not:
//   - Traverse history, infer repeated boundaries, retry, compare state, or run
//     loops.
// - Allows:
//   - Inputs: All six frozen semantic-history result classes.
//   - Outputs: Exact directly-implied progress-evidence-or-none assertion.
//   - Side effects: None.
// - Split-When:
//   - Stateful history progress fixtures require independent evidence.
// - Merge-When:
//   - Coordinator tests fully subsume this finite projection oracle.
// - Summary:
//   - Pins Traversed as revision progress and replay as recovery non-progress.
// - Description:
//   - Leaves every history result requiring more context unclassified.
// - Usage:
//   - Compare all six history results with an independent expected mapping.
// - Defaults:
//   - Only Traversed and Idempotent replay directly imply progress evidence.
//
use atrament_autonomous_history_progress_projection::
    autonomous_history_result_progress_evidence;
use atrament_autonomous_progress_evidence::AutonomousProgressEvidenceClass;
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
) -> Option<AutonomousProgressEvidenceClass> {
    match result {
        SemanticHistoryResultClass::IdempotentReplay => {
            Some(AutonomousProgressEvidenceClass::IdempotentReplayRecovery)
        },
        SemanticHistoryResultClass::Traversed => {
            Some(AutonomousProgressEvidenceClass::AcceptedHistoryRevisionChange)
        },
        SemanticHistoryResultClass::CancelledBeforeCommit
        | SemanticHistoryResultClass::HistoryBoundary
        | SemanticHistoryResultClass::KnownNoCommitFailure
        | SemanticHistoryResultClass::StaleCurrentRevision => None,
    }
}

#[test]
fn all_six_history_results_match_direct_progress_projection() {
    let mut classified = 0_usize;
    for result in RESULTS {
        let expected = expected(result);
        assert_eq!(
            autonomous_history_result_progress_evidence(result),
            expected
        );
        if expected.is_some() {
            classified += 1;
        }
    }
    assert_eq!(classified, 2);
}
