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
//   - Exhaustive source, path-purpose, and layout-state Export gate fixtures.
// - Must-Not:
//   - Read or write output files, fabricate renderer bytes, or grant I/O.
// - Allows:
//   - Inputs: Read-only Export layout results and trusted path classifications.
//   - Outputs: Exact pending or blocked prerequisite outcomes.
//   - Side effects: Process-local allocations only.
// - Split-When:
//   - Artifact or file-commit validation requires independent integration.
// - Merge-When:
//   - Executable Export end-to-end tests subsume prerequisite combinations.
// - Summary:
//   - Proves passing file intent and layout never means a file was exported.
// - Description:
//   - Pins rejection precedence and revision-bound pending outcomes.
// - Usage:
//   - Run Cargo's dedicated export-prerequisite application integration test.
// - Defaults:
//   - No file side effect can be authorized by this projection.
//

//! Read-only Export prerequisite projection regression fixtures.

use std::path::PathBuf;

use atrament_autonomous_output_intent::AutonomousOutputIntentSource;
use atrament_diagnostic::{Completeness, DiagnosticSet};
use atrament_explicit_file_path_intent::{
    ExplicitFilePathSource, ExplicitFilePurpose, ExplicitOverwriteIntent,
    classify_explicit_file_path_intent,
};
use atrament_export_layout_preflight::{
    ExportLayoutPreflightResult, RevisionLayoutDiagnostics,
    preflight_layout_for_export,
};
use atrament_export_prerequisite_projection::{
    ExportPrerequisiteDisposition, project_export_prerequisites,
};
use atrament_semantic_notebook::{
    AcceptedRevision, IdentityAllocator, Notebook,
};

const PURPOSES: [ExplicitFilePurpose; 4] = [
    ExplicitFilePurpose::ExportNotebook,
    ExplicitFilePurpose::ExportProfile,
    ExplicitFilePurpose::ImportNotebook,
    ExplicitFilePurpose::ImportProfile,
];

const GOAL_SOURCES: [AutonomousOutputIntentSource; 3] = [
    AutonomousOutputIntentSource::CallerExplicitGoal,
    AutonomousOutputIntentSource::IndependentAdmittedHostPolicy,
    AutonomousOutputIntentSource::UntrustedSemanticContent,
];

const PATH_SOURCES: [ExplicitFilePathSource; 2] = [
    ExplicitFilePathSource::AdmittedHostPolicy,
    ExplicitFilePathSource::CallerSelection,
];

fn revision_fixture() -> AcceptedRevision {
    let identities = IdentityAllocator::new();
    let id = identities.allocate_revision().expect("revision id");
    let notebook_id = identities.allocate_accepted().expect("notebook id");
    AcceptedRevision {
        id,
        notebook: Notebook {
            assets: vec![],
            constraints: vec![],
            extensions: vec![],
            id: notebook_id,
            output_profiles: vec![],
            page_profiles: vec![],
            pages: vec![],
            provenance: vec![],
            styles: vec![],
        },
    }
}

fn diagnostics(completeness: Completeness) -> DiagnosticSet {
    DiagnosticSet {
        completeness,
        diagnostics: vec![],
    }
}

#[test]
fn independently_selected_export_path_and_real_layout_gate_remain_pending() {
    let current = revision_fixture();
    let complete = diagnostics(Completeness::Complete);
    let bound = RevisionLayoutDiagnostics::bind(current.id, &complete)
        .expect("empty complete evidence binds to requested revision");
    let layout = preflight_layout_for_export(&current, current.id, bound)
        .expect("current revision matches layout evidence");
    assert!(matches!(layout, ExportLayoutPreflightResult::Ready { .. }));
    let path = PathBuf::from("./explicit-output.atrament");
    let intent = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ExportNotebook,
        ExplicitFilePathSource::CallerSelection,
        path.clone(),
        ExplicitOverwriteIntent::PreserveExisting,
    )
    .expect("trusted selection is a valid file path");
    assert_eq!(
        project_export_prerequisites(
            AutonomousOutputIntentSource::CallerExplicitGoal,
            &intent,
            &layout,
        ),
        ExportPrerequisiteDisposition::RequiresArtifactAndFileAdmission {
            revision: current.id,
        },
    );
    assert_eq!(intent.path(), path.as_path());
}

#[test]
fn incomplete_real_layout_preflight_cannot_advance_export() {
    let current = revision_fixture();
    let incomplete = diagnostics(Completeness::Incomplete);
    let bound = RevisionLayoutDiagnostics::bind(current.id, &incomplete)
        .expect("incomplete evidence still has a revision binding");
    let layout = preflight_layout_for_export(&current, current.id, bound)
        .expect("current revision matches layout evidence");
    let intent = classify_explicit_file_path_intent(
        ExplicitFilePurpose::ExportProfile,
        ExplicitFilePathSource::AdmittedHostPolicy,
        PathBuf::from("profile.atrament"),
        ExplicitOverwriteIntent::ReplaceExisting,
    )
    .expect("policy independently selected profile export");
    assert_eq!(
        project_export_prerequisites(
            AutonomousOutputIntentSource::IndependentAdmittedHostPolicy,
            &intent,
            &layout,
        ),
        ExportPrerequisiteDisposition::IncompleteLayout,
    );
}

#[test]
fn all_source_purpose_and_layout_combinations_preserve_precedence() {
    let current = revision_fixture();
    let complete = diagnostics(Completeness::Complete);
    let incomplete = diagnostics(Completeness::Incomplete);
    let scenarios = [
        ExportLayoutPreflightResult::Ready {
            diagnostics: complete.clone(),
            revision: current.id,
        },
        ExportLayoutPreflightResult::Blocked {
            diagnostics: complete,
            revision: current.id,
        },
        ExportLayoutPreflightResult::Incomplete {
            diagnostics: incomplete,
            revision: current.id,
        },
    ];
    let mut cases = 0usize;
    for goal in GOAL_SOURCES {
        for purpose in PURPOSES {
            for selected_source in PATH_SOURCES {
                let intent = classify_explicit_file_path_intent(
                    purpose,
                    selected_source,
                    PathBuf::from("chosen-output.atrament"),
                    ExplicitOverwriteIntent::PreserveExisting,
                )
                .expect("path source independently admitted");
                for (scenario, layout) in scenarios.iter().enumerate() {
                    let expected = if matches!(
                        goal,
                        AutonomousOutputIntentSource::UntrustedSemanticContent
                    ) {
                        ExportPrerequisiteDisposition::NoExportIntent
                    } else if !purpose.is_export() {
                        ExportPrerequisiteDisposition::WrongPathPurpose
                    } else {
                        match scenario {
                            0 => ExportPrerequisiteDisposition::
                                RequiresArtifactAndFileAdmission {
                                revision: current.id,
                            },
                            1 => ExportPrerequisiteDisposition::BlockedLayout,
                            _ => {
                                ExportPrerequisiteDisposition::IncompleteLayout
                            },
                        }
                    };
                    assert_eq!(
                        project_export_prerequisites(goal, &intent, layout),
                        expected,
                        "goal={goal:?} file={purpose:?} case={scenario}",
                    );
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 72);
}
