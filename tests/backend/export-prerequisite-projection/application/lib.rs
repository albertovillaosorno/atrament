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

use std::fmt::Debug;
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

fn required<T, E>(result: Result<T, E>, context: &str) -> Option<T>
where
    E: Debug,
{
    assert!(result.is_ok(), "{context}: {:?}", result.as_ref().err());
    result.ok()
}

fn revision_fixture() -> Option<AcceptedRevision> {
    let identities = IdentityAllocator::new();
    let id = required(identities.allocate_revision(), "revision id")?;
    let notebook_id = required(identities.allocate_accepted(), "notebook id")?;
    Some(AcceptedRevision {
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
    })
}

const fn diagnostics(completeness: Completeness) -> DiagnosticSet {
    DiagnosticSet {
        completeness,
        diagnostics: vec![],
    }
}

#[test]
fn independently_selected_export_path_and_real_layout_gate_remain_pending() {
    let Some(current) = revision_fixture() else {
        return;
    };
    let complete = diagnostics(Completeness::Complete);
    let Some(bound) = required(
        RevisionLayoutDiagnostics::bind(current.id, &complete),
        "empty complete evidence binds to requested revision",
    ) else {
        return;
    };
    let Some(layout) = required(
        preflight_layout_for_export(&current, current.id, bound),
        "current revision matches layout evidence",
    ) else {
        return;
    };
    assert!(matches!(layout, ExportLayoutPreflightResult::Ready { .. }));
    let path = PathBuf::from("./explicit-output.atrament");
    let Some(intent) = required(
        classify_explicit_file_path_intent(
            ExplicitFilePurpose::ExportNotebook,
            ExplicitFilePathSource::CallerSelection,
            path.clone(),
            ExplicitOverwriteIntent::PreserveExisting,
        ),
        "trusted selection is a valid file path",
    ) else {
        return;
    };
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
    let Some(current) = revision_fixture() else {
        return;
    };
    let incomplete = diagnostics(Completeness::Incomplete);
    let Some(bound) = required(
        RevisionLayoutDiagnostics::bind(current.id, &incomplete),
        "incomplete evidence still has a revision binding",
    ) else {
        return;
    };
    let Some(layout) = required(
        preflight_layout_for_export(&current, current.id, bound),
        "current revision matches layout evidence",
    ) else {
        return;
    };
    let Some(intent) = required(
        classify_explicit_file_path_intent(
            ExplicitFilePurpose::ExportProfile,
            ExplicitFilePathSource::AdmittedHostPolicy,
            PathBuf::from("profile.atrament"),
            ExplicitOverwriteIntent::ReplaceExisting,
        ),
        "policy independently selected profile export",
    ) else {
        return;
    };
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
    let Some(current) = revision_fixture() else {
        return;
    };
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
                let Some(intent) = required(
                    classify_explicit_file_path_intent(
                        purpose,
                        selected_source,
                        PathBuf::from("chosen-output.atrament"),
                        ExplicitOverwriteIntent::PreserveExisting,
                    ),
                    "path source independently admitted",
                ) else {
                    return;
                };
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
