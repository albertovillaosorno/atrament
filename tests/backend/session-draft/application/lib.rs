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
//   - Run this Cargo harness against the public draft crate.
// - Defaults:
//   - Starts from an empty disposable session draft.
//

//! Public session-draft regression tests.

use atrament_diagnostic::{
    BlockingDisposition, Completeness, Diagnostic, DiagnosticCode,
    DiagnosticSet, Evidence, EvidenceUnit, LocationKind, LocationRole,
    Operation, OperationBinding, Remediation, SemanticLocation, Severity,
};
use atrament_session_draft_port::{DraftField, DraftMutation, SessionDraft as _};

use atrament_session_draft as draft;

fn assert_resource_limit(
    result: &DraftMutation,
    expected_identity: &str,
    observed: u64,
) {
    assert_eq!(
        result,
        &DraftMutation::ResourceLimit {
            diagnostics: DiagnosticSet {
                completeness: Completeness::Complete,
                diagnostics: vec![Diagnostic {
                    code: DiagnosticCode::SessionDraftResourceLimit,
                    disposition: BlockingDisposition::Blocking,
                    evidence: vec![Evidence::LimitExceeded {
                        maximum: 1_048_576,
                        observed,
                        unit: EvidenceUnit::Bytes,
                    }],
                    locations: vec![SemanticLocation {
                        identity: String::from(expected_identity),
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
            },
        },
        "resource-limit rejection must return one exact typed diagnostic",
    );
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
        service.replace(DraftField::Source, String::from("x\u{b2} + y\u{b2}")),
        DraftMutation::Applied,
    );
    assert_eq!(
        service.replace(DraftField::Candidate, String::from("untrusted model")),
        DraftMutation::Applied,
    );
    assert_eq!(service.value(DraftField::Task), "format these notes");
    assert_eq!(service.value(DraftField::Source), "x\u{b2} + y\u{b2}");
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
    assert_resource_limit(&result, "session-draft:source", 1_048_577);
    assert_eq!(service.value(DraftField::Source), "accepted draft");
}

#[test]
fn every_field_enforces_utf8_byte_limit_without_truncation() {
    // Three-byte text needs an ASCII suffix to reach the exact byte cap.
    let codepoints = [
        ("a", 1_048_576, "", 1_048_577, 1_048_577),
        ("\u{e1}", 524_288, "", 524_289, 1_048_578),
        ("\u{20ac}", 349_525, "a", 349_526, 1_048_578),
        ("\u{1f4dd}", 262_144, "", 262_145, 1_048_580),
    ];
    for (field, identity) in [
        (DraftField::Candidate, "session-draft:candidate"),
        (DraftField::Source, "session-draft:source"),
        (DraftField::Task, "session-draft:task"),
    ] {
        let other = if field == DraftField::Task {
            DraftField::Source
        } else {
            DraftField::Task
        };
        for (codepoint, count, suffix, over_count, observed) in codepoints {
            let mut service = draft::SessionDraftService::default();
            assert_eq!(
                service.replace(other, String::from("preserve")),
                DraftMutation::Applied,
            );
            let mut exact = codepoint.repeat(count);
            exact.push_str(suffix);
            assert_eq!(exact.len(), draft::MAX_DRAFT_FIELD_BYTES);
            assert_eq!(
                service.replace(field, exact.clone()),
                DraftMutation::Applied,
            );
            let over_limit = codepoint.repeat(over_count);
            assert_eq!(
                u64::try_from(over_limit.len()),
                Ok(observed),
            );
            let result = service.replace(field, over_limit);
            assert_resource_limit(&result, identity, observed);
            assert_eq!(service.value(field), exact);
            assert_eq!(service.value(other), "preserve");
        }
    }
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
