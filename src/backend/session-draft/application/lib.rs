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
//   - Disposable task, source, and raw model-response text before acceptance.
// - Must-Not:
//   - Persist draft text or treat it as an accepted semantic notebook revision.
// - Allows:
//   - Inputs: Complete bounded field replacements from an admitted caller.
//   - Outputs: Current in-memory field values and typed replacement results.
//   - Side effects: Mutates process-local session draft memory only.
// - Split-When:
//   - Draft preparation needs independently transactional sub-capabilities.
// - Merge-When:
//   - A later session application boundary subsumes pre-acceptance draft state.
// - Summary:
//   - Owns the first mutable application state in a disposable Atrament
//     session.
// - Description:
//   - Retains source-preparation text without creating accepted notebook state.
// - Usage:
//   - Inject one service instance into the localhost runtime for one process.
// - Defaults:
//   - Each field is empty and bounded to one mebibyte of UTF-8 bytes.
//

//! Process-local pre-acceptance draft state for one Atrament session.

use std::fmt;

use atrament_diagnostic::{
    BlockingDisposition, Completeness, Diagnostic, DiagnosticCode,
    DiagnosticSet, Evidence, EvidenceUnit, LocationKind, LocationRole,
    Operation, OperationBinding, Remediation, SemanticLocation, Severity,
};
use atrament_session_draft_port::{DraftField, DraftMutation, SessionDraft};

/// Maximum admitted UTF-8 byte length of one complete draft text field.
pub const MAX_DRAFT_FIELD_BYTES: usize = 1_048_576;

const MAX_DRAFT_FIELD_BYTES_DIAGNOSTIC: u64 = 1_048_576;

/// Mutable pre-acceptance text retained only for the active process lifetime.
#[derive(Default)]
pub struct SessionDraftService {
    candidate: String,
    source: String,
    task: String,
}

impl fmt::Debug for SessionDraftService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionDraftService")
            .finish_non_exhaustive()
    }
}

impl SessionDraft for SessionDraftService {
    fn replace(&mut self, field: DraftField, value: String) -> DraftMutation {
        if value.len() > MAX_DRAFT_FIELD_BYTES {
            return DraftMutation::ResourceLimit {
                diagnostics: resource_limit_diagnostics(field, value.len()),
            };
        }
        match field {
            DraftField::Candidate => self.candidate = value,
            DraftField::Source => self.source = value,
            DraftField::Task => self.task = value,
        }
        DraftMutation::Applied
    }

    fn value(&self, field: DraftField) -> &str {
        match field {
            DraftField::Candidate => &self.candidate,
            DraftField::Source => &self.source,
            DraftField::Task => &self.task,
        }
    }
}

const fn draft_field_identity(field: DraftField) -> &'static str {
    match field {
        DraftField::Candidate => "session-draft:candidate",
        DraftField::Source => "session-draft:source",
        DraftField::Task => "session-draft:task",
    }
}

fn resource_limit_diagnostics(
    field: DraftField,
    observed_bytes: usize,
) -> DiagnosticSet {
    let observed_bytes_u64 = u64::try_from(observed_bytes).unwrap_or(u64::MAX);
    DiagnosticSet {
        completeness: Completeness::Complete,
        diagnostics: vec![Diagnostic {
            code: DiagnosticCode::SessionDraftResourceLimit,
            disposition: BlockingDisposition::Blocking,
            evidence: vec![Evidence::LimitExceeded {
                maximum: MAX_DRAFT_FIELD_BYTES_DIAGNOSTIC,
                observed: observed_bytes_u64,
                unit: EvidenceUnit::Bytes,
            }],
            locations: vec![SemanticLocation {
                identity: String::from(draft_field_identity(field)),
                kind: LocationKind::Field,
                relationship: None,
                role: LocationRole::Primary,
            }],
            operation: OperationBinding {
                contexts: vec![],
                operation: Operation::SessionDraftReplace,
            },
            remediations: vec![Remediation::ReduceInput],
            severity: Severity::Error,
        }],
    }
}
