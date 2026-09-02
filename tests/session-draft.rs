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
//   - Regression evidence for disposable pre-acceptance session draft state.
// - Must-Not:
//   - Exercise HTTP admission or imply draft text is accepted notebook state.
// - Allows:
//   - Inputs: Empty, ordinary, replacement, and over-limit draft field
//     fixtures.
//   - Outputs: Assertions over independent in-memory fields and atomic
//     rejection.
//   - Side effects: Process-local test allocations only.
// - Split-When:
//   - Draft application behavior gains independently testable capabilities.
// - Merge-When:
//   - Pre-acceptance draft state is subsumed by another application fixture.
// - Summary:
//   - Verifies bounded process-local task, source, and raw-response draft
//     state.
// - Description:
//   - Proves replacements are whole-field and over-limit input is non-mutating.
// - Usage:
//   - Compile with the draft inbound-port crate available as an external crate.
// - Defaults:
//   - Starts from an empty disposable session draft.
//
use atrament_diagnostic::{
    BlockingDisposition, Completeness, DiagnosticCode, Evidence, EvidenceUnit,
    LocationKind, LocationRole, Operation, Remediation, Severity,
};
use atrament_session_draft_port::{DraftField, DraftMutation, SessionDraft};

#[allow(dead_code)]
#[path = "../src/backend/session-draft/application/lib.rs"]
mod draft;

fn assert_resource_limit(
    result: DraftMutation,
    expected_identity: &str,
    observed: u64,
) {
    let DraftMutation::ResourceLimit { diagnostics } = result else {
        panic!("over-limit draft replacement must remain ResourceLimit");
    };
    assert_eq!(diagnostics.completeness, Completeness::Complete);
    let [diagnostic] = diagnostics.diagnostics.as_slice() else {
        panic!("resource limit must return one diagnostic");
    };
    assert_eq!(diagnostic.code, DiagnosticCode::SessionDraftResourceLimit);
    assert_eq!(diagnostic.disposition, BlockingDisposition::Blocking);
    assert_eq!(
        diagnostic.operation.operation,
        Operation::SessionDraftReplace
    );
    assert_eq!(diagnostic.remediations, [Remediation::ReduceInput]);
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(matches!(
        diagnostic.evidence.as_slice(),
        [Evidence::LimitExceeded {
            maximum: 1_048_576,
            observed: actual,
            unit: EvidenceUnit::Bytes,
        }] if *actual == observed
    ));
    assert!(matches!(
        diagnostic.locations.as_slice(),
        [location]
            if location.identity == expected_identity
                && location.kind == LocationKind::Field
                && location.role == LocationRole::Primary
                && location.relationship.is_none()
    ));
}

#[test]
fn draft_fields_start_empty_and_replace_independently() {
    let mut service = draft::SessionDraftService::default();
    for field in [DraftField::Candidate, DraftField::Source, DraftField::Task] {
        assert_eq!(service.value(field), "");
    }

    assert_eq!(
        service.replace(DraftField::Task, String::from("format these notes")),
        DraftMutation::Applied,
    );
    assert_eq!(
        service.replace(DraftField::Source, String::from("x² + y²")),
        DraftMutation::Applied,
    );
    assert_eq!(
        service.replace(DraftField::Candidate, String::from("untrusted model")),
        DraftMutation::Applied,
    );
    assert_eq!(service.value(DraftField::Task), "format these notes");
    assert_eq!(service.value(DraftField::Source), "x² + y²");
    assert_eq!(service.value(DraftField::Candidate), "untrusted model");
}

#[test]
fn over_limit_replacement_does_not_change_current_value() {
    let mut service = draft::SessionDraftService::default();
    assert_eq!(
        service.replace(DraftField::Source, String::from("accepted draft")),
        DraftMutation::Applied,
    );
    let over_limit = "a".repeat(draft::MAX_DRAFT_FIELD_BYTES + 1);
    let result = service.replace(DraftField::Source, over_limit);
    assert_resource_limit(result, "session-draft:source", 1_048_577);
    assert_eq!(service.value(DraftField::Source), "accepted draft");
}

#[test]
fn replacement_uses_utf8_byte_limit_without_truncation() {
    let mut service = draft::SessionDraftService::default();
    let exact = "á".repeat(draft::MAX_DRAFT_FIELD_BYTES / 2);
    assert_eq!(exact.len(), draft::MAX_DRAFT_FIELD_BYTES);
    assert_eq!(
        service.replace(DraftField::Candidate, exact),
        DraftMutation::Applied,
    );
    let over_limit = "á".repeat((draft::MAX_DRAFT_FIELD_BYTES / 2) + 1);
    let result = service.replace(DraftField::Candidate, over_limit);
    assert_resource_limit(result, "session-draft:candidate", 1_048_578);
    assert_eq!(
        service.value(DraftField::Candidate).len(),
        draft::MAX_DRAFT_FIELD_BYTES,
    );
}

#[test]
fn debug_output_never_exposes_private_draft_text() {
    let mut service = draft::SessionDraftService::default();
    for (field, value) in [
        (DraftField::Task, "private task"),
        (DraftField::Source, "private source"),
        (DraftField::Candidate, "private response"),
    ] {
        assert_eq!(
            service.replace(field, String::from(value)),
            DraftMutation::Applied,
        );
    }
    let debug = format!("{service:?}");
    assert!(debug.contains("SessionDraftService"));
    for private in ["private task", "private source", "private response"] {
        assert!(!debug.contains(private));
    }
}
