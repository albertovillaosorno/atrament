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

//! Autonomous output-intent regression tests.

use atrament_autonomous_output_intent::{
    AutonomousOutputClass, AutonomousOutputIntentAdmission,
    AutonomousOutputIntentSource, autonomous_export_path_intent_admission,
    autonomous_output_intent_admission,
};

use std::path::PathBuf;

use atrament_explicit_file_path_intent::{
    ExplicitFilePathSource, ExplicitFilePurpose, ExplicitOverwriteIntent,
    classify_explicit_file_path_intent,
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
    let mut cases = 0usize;
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

#[test]
fn autonomous_export_requires_independent_goal_and_selected_path_intent() {
    let purposes = [
        ExplicitFilePurpose::ExportNotebook,
        ExplicitFilePurpose::ExportProfile,
        ExplicitFilePurpose::ImportNotebook,
        ExplicitFilePurpose::ImportProfile,
    ];
    let origins = [
        ExplicitFilePathSource::CallerSelection,
        ExplicitFilePathSource::AdmittedHostPolicy,
    ];
    let mut cases = 0usize;
    for goal_source in SOURCES {
        for purpose in purposes {
            for origin in origins {
                let actual = classify_explicit_file_path_intent(
                    purpose,
                    origin,
                    PathBuf::from("one-explicit-file.atrament"),
                    ExplicitOverwriteIntent::PreserveExisting,
                )
                .map(|intent| {
                    autonomous_export_path_intent_admission(
                        goal_source, &intent,
                    )
                });
                let expected = if purpose.is_export()
                    && !matches!(
                        goal_source,
                        AutonomousOutputIntentSource::UntrustedSemanticContent
                    )
                {
                    AutonomousOutputIntentAdmission::Admitted
                } else {
                    AutonomousOutputIntentAdmission::NotAdmitted
                };
                assert_eq!(
                    actual,
                    Ok(expected),
                    "goal={goal_source:?} file={purpose:?} origin={origin:?}",
                );
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 24);
}
