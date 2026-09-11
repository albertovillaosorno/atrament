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
//   - Exhaustive evidence for autonomous output-intent source semantics.
// - Must-Not:
//   - Execute output, grant capabilities, choose paths, or authorize hardware.
// - Allows:
//   - Inputs: Every three-source by three-output combination.
//   - Outputs: Exact nine-case intent-source admission assertions.
//   - Side effects: None.
// - Split-When:
//   - Stateful host-policy or output-chain fixtures require separate evidence.
// - Merge-When:
//   - Executable autonomous output tests subsume this source classification.
// - Summary:
//   - Pins caller/host/content authority without treating prose as permission.
// - Description:
//   - Proves host policy is Export-only and semantic content admits no output.
// - Usage:
//   - Compare all finite source/output pairs with the frozen intent contract.
// - Defaults:
//   - Untrusted semantic content never establishes output intent.
//
use atrament_autonomous_output_intent::{
    AutonomousOutputClass, AutonomousOutputIntentAdmission,
    AutonomousOutputIntentSource, autonomous_output_intent_admission,
};

const OUTPUTS: [AutonomousOutputClass; 3] = [
    AutonomousOutputClass::Export,
    AutonomousOutputClass::Plan,
    AutonomousOutputClass::Render,
];

const SOURCES: [AutonomousOutputIntentSource; 3] = [
    AutonomousOutputIntentSource::CallerExplicitGoal,
    AutonomousOutputIntentSource::IndependentAdmittedHostPolicy,
    AutonomousOutputIntentSource::UntrustedSemanticContent,
];

fn expected(
    output: AutonomousOutputClass,
    source: AutonomousOutputIntentSource,
) -> AutonomousOutputIntentAdmission {
    match source {
        AutonomousOutputIntentSource::CallerExplicitGoal => {
            AutonomousOutputIntentAdmission::Admitted
        },
        AutonomousOutputIntentSource::IndependentAdmittedHostPolicy
            if output == AutonomousOutputClass::Export =>
        {
            AutonomousOutputIntentAdmission::Admitted
        },
        AutonomousOutputIntentSource::IndependentAdmittedHostPolicy
        | AutonomousOutputIntentSource::UntrustedSemanticContent => {
            AutonomousOutputIntentAdmission::NotAdmitted
        },
    }
}

#[test]
fn all_nine_source_output_pairs_match_frozen_intent_authority() {
    let mut cases = 0_usize;
    for source in SOURCES {
        for output in OUTPUTS {
            assert_eq!(
                autonomous_output_intent_admission(output, source),
                expected(output, source),
                "source={source:?} output={output:?}",
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 9);
}
