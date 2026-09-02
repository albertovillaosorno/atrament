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
//   - Read-only Export layout preflight for one exact accepted revision.
// - Must-Not:
//   - Write files, choose paths, infer overwrite intent, or report Exported.
//   - Treat layout preflight readiness as complete Export authorization.
// - Allows:
//   - Inputs: Current accepted revision and revision-bound layout diagnostics.
//   - Outputs: Ready, blocked, incomplete, or typed binding failure.
//   - Side effects: Process-local diagnostic cloning only.
// - Split-When:
//   - Full Export validation or file effects become independently implemented.
// - Merge-When:
//   - Complete Export application preflight subsumes this layout-only gate.
// - Summary:
//   - Prevents Export readiness while layout evidence blocks or is incomplete.
// - Description:
//   - Applies frozen pre-write diagnostic rules without persistent side
//     effects.
// - Usage:
//   - Run after layout diagnostics are complete for the requested revision.
// - Defaults:
//   - Only complete, non-blocking layout evidence returns layout readiness.
//

//! Read-only layout diagnostic gate for later explicit Export operations.

use atrament_diagnostic::{
    BlockingDisposition, Completeness, DiagnosticSet, Operation,
    OperationContextKind,
};
use atrament_semantic_notebook::{AcceptedRevision, RevisionIdentity};

/// Layout diagnostics explicitly bound to the accepted revision that produced
/// them.
#[derive(Clone, Copy, Debug)]
pub struct RevisionLayoutDiagnostics<'diagnostics> {
    diagnostics: &'diagnostics DiagnosticSet,
    revision: RevisionIdentity,
}

impl<'diagnostics> RevisionLayoutDiagnostics<'diagnostics> {
    /// Bind one layout diagnostic set to the accepted revision that produced
    /// it.
    ///
    /// Empty diagnostic sets have no instance context to inspect, so the typed
    /// revision argument supplies their binding. Every non-empty set must carry
    /// one matching accepted-revision context in each layout diagnostic.
    ///
    /// # Errors
    ///
    /// Returns a typed failure when a diagnostic belongs to another operation,
    /// omits accepted-revision context, or names a different accepted revision.
    pub fn bind(
        revision: RevisionIdentity,
        diagnostics: &'diagnostics DiagnosticSet,
    ) -> Result<Self, ExportLayoutPreflightError> {
        let expected = format!("{revision:?}");
        for diagnostic in &diagnostics.diagnostics {
            if diagnostic.operation.operation != Operation::Layout {
                return Err(ExportLayoutPreflightError::NonLayoutDiagnostic);
            }
            let mut revision_contexts =
                diagnostic.operation.contexts.iter().filter(|context| {
                    context.kind == OperationContextKind::AcceptedRevision
                });
            let Some(context) = revision_contexts.next() else {
                return Err(
                    ExportLayoutPreflightError::DiagnosticContextMissing,
                );
            };
            if revision_contexts.next().is_some()
                || context.identity != expected
            {
                return Err(
                    ExportLayoutPreflightError::DiagnosticContextMismatch,
                );
            }
        }
        Ok(Self { diagnostics, revision })
    }
}

/// Layout-only preflight result for a later explicit Export request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExportLayoutPreflightResult {
    /// Complete layout evidence contains at least one blocking diagnostic.
    Blocked {
        /// Complete blocking/advisory layout evidence for this preflight.
        diagnostics: DiagnosticSet,
        /// Exact accepted revision that remains blocked.
        revision: RevisionIdentity,
    },
    /// Layout evidence is explicitly incomplete and cannot authorize readiness.
    Incomplete {
        /// Explicitly incomplete layout evidence preserved for the caller.
        diagnostics: DiagnosticSet,
        /// Exact accepted revision whose layout evidence is incomplete.
        revision: RevisionIdentity,
    },
    /// Layout evidence is complete and contains no blocking diagnostic.
    Ready {
        /// Complete advisory-or-empty layout evidence preserved for the
        /// caller.
        diagnostics: DiagnosticSet,
        /// Exact accepted revision whose layout gate is ready.
        revision: RevisionIdentity,
    },
}

/// Typed failure before layout diagnostics can be used for Export preflight.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportLayoutPreflightError {
    /// One layout diagnostic carries a different accepted-revision context.
    DiagnosticContextMismatch,
    /// One layout diagnostic omits its accepted-revision operation context.
    DiagnosticContextMissing,
    /// Supplied layout evidence belongs to another accepted revision.
    LayoutRevisionMismatch {
        /// Accepted revision explicitly requested for later Export.
        requested: RevisionIdentity,
        /// Revision whose layout diagnostics were supplied.
        supplied: RevisionIdentity,
    },
    /// One supplied diagnostic was not produced by the layout capability.
    NonLayoutDiagnostic,
    /// Requested revision is not the current accepted revision in this service.
    RequestedRevisionMismatch {
        /// Current accepted revision available to the application.
        current: RevisionIdentity,
        /// Accepted revision explicitly requested for later Export.
        requested: RevisionIdentity,
    },
}

/// Evaluate only the layout-diagnostic prerequisite for explicit Export.
///
/// This does not validate semantic, source, asset, output-format, path,
/// overwrite, retry, or file-commit requirements. A `Ready` result therefore
/// means only that the supplied complete layout evidence does not block the
/// later Export operation.
///
/// # Errors
///
/// Returns a typed binding failure for a stale requested revision, layout
/// evidence from another revision, or diagnostic evidence from another
/// application capability.
pub fn preflight_layout_for_export(
    current: &AcceptedRevision,
    requested: RevisionIdentity,
    layout: RevisionLayoutDiagnostics<'_>,
) -> Result<ExportLayoutPreflightResult, ExportLayoutPreflightError> {
    if requested != current.id {
        return Err(ExportLayoutPreflightError::RequestedRevisionMismatch {
            current: current.id,
            requested,
        });
    }
    if layout.revision != requested {
        return Err(ExportLayoutPreflightError::LayoutRevisionMismatch {
            requested,
            supplied: layout.revision,
        });
    }
    if layout.diagnostics.completeness == Completeness::Incomplete {
        return Ok(ExportLayoutPreflightResult::Incomplete {
            diagnostics: layout.diagnostics.clone(),
            revision: requested,
        });
    }
    if layout.diagnostics.diagnostics.iter().any(|diagnostic| {
        diagnostic.disposition == BlockingDisposition::Blocking
    }) {
        return Ok(ExportLayoutPreflightResult::Blocked {
            diagnostics: layout.diagnostics.clone(),
            revision: requested,
        });
    }
    Ok(ExportLayoutPreflightResult::Ready {
        diagnostics: layout.diagnostics.clone(),
        revision: requested,
    })
}
