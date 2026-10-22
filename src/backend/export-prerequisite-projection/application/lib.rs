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
//   - Read-only composition of Export intent, selected path purpose, and
//     revision-bound layout preflight outcomes.
// - Must-Not:
//   - Claim complete Export readiness, create files, choose output formats,
//     admit callers or filesystem paths, serialize, or commit effects.
// - Allows:
//   - Inputs: Trusted-source classifications, one classified path, and an
//     already-evaluated Export layout preflight result.
//   - Outputs: Exact prerequisite failure or a revision-bound pending state.
//   - Side effects: None.
// - Split-When:
//   - Executable full Export admission requires distinct file authority.
// - Merge-When:
//   - Complete Export preflight application subsumes these independent gates.
// - Summary:
//   - Separates passing prerequisite checks from permission to write files.
// - Description:
//   - Never treats layout readiness alone as file or Export authorization.
// - Usage:
//   - Call only after trusted source and revision/layout evidence are bound.
// - Defaults:
//   - Missing Export source, path purpose, or complete layout evidence blocks.
//

//! Read-only Export prerequisite projection without file-effect authority.

use atrament_autonomous_output_intent::{
    AutonomousOutputClass, AutonomousOutputIntentAdmission,
    AutonomousOutputIntentSource, autonomous_output_intent_admission,
};
use atrament_explicit_file_path_intent::ExplicitFilePathIntent;
use atrament_export_layout_preflight::ExportLayoutPreflightResult;
use atrament_semantic_notebook::RevisionIdentity;

/// One exact reason why an Export cannot advance to later admission gates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportPrerequisiteDisposition {
    /// Layout evidence contains a blocking diagnostic.
    BlockedLayout,
    /// Layout evidence has not been established as complete.
    IncompleteLayout,
    /// Caller goal or independent host policy did not authorize Export intent.
    NoExportIntent,
    /// Goal, path, and layout prerequisites passed but other checks remain.
    RequiresArtifactAndFileAdmission {
        /// Revision bound by the original layout preflight.
        revision: RevisionIdentity,
    },
    /// The selected file path belongs to Import, not Export.
    WrongPathPurpose,
}

/// Project which Export prerequisites have passed, without granting I/O.
///
/// This projection never grants persistent file authority. Even when it yields
/// `RequiresArtifactAndFileAdmission`, future adapters must independently
/// prove the caller-selected path source, exact revision, artifact bytes,
/// format, capacity, path/overwrite policy, file race protections, and retry
/// identity before any file effect. Layout evidence must already be evaluated
/// by the revision-binding Export layout preflight application.
#[must_use]
pub const fn project_export_prerequisites(
    source: AutonomousOutputIntentSource,
    path: &ExplicitFilePathIntent,
    layout: &ExportLayoutPreflightResult,
) -> ExportPrerequisiteDisposition {
    if matches!(
        autonomous_output_intent_admission(
            AutonomousOutputClass::Export, source,
        ),
        AutonomousOutputIntentAdmission::NotAdmitted
    ) {
        return ExportPrerequisiteDisposition::NoExportIntent;
    }
    if !path.purpose().is_export() {
        return ExportPrerequisiteDisposition::WrongPathPurpose;
    }
    match layout {
        ExportLayoutPreflightResult::Blocked { .. } => {
            ExportPrerequisiteDisposition::BlockedLayout
        },
        ExportLayoutPreflightResult::Incomplete { .. } => {
            ExportPrerequisiteDisposition::IncompleteLayout
        },
        ExportLayoutPreflightResult::Ready { revision, .. } => {
            ExportPrerequisiteDisposition::RequiresArtifactAndFileAdmission {
                revision: *revision,
            }
        },
    }
}
