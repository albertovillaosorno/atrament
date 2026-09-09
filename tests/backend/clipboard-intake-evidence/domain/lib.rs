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
//   - Regression evidence for typed rich-clipboard source retention.
// - Must-Not:
//   - Access clipboard APIs, decode images, parse MIME, mutate notebooks, or
//     invent metadata-retention policy.
// - Allows:
//   - Inputs: Deterministic content, provenance, issue, and unresolved
//     fixtures.
//   - Outputs: Assertions over content kinds and exact retained evidence.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Clipboard adapters or semantic candidate construction gain fixtures.
// - Merge-When:
//   - Intake evidence moves into a broader ingestion-domain harness.
// - Summary:
//   - Proves rich clipboard loss and unresolved content remain explicit.
// - Description:
//   - Covers all admitted kinds, provenance, issue ordering, and unresolved
//     fragments.
// - Usage:
//   - Compile directly against the clipboard-intake-evidence domain.
// - Defaults:
//   - No empty issue list is interpreted as proof of adapter correctness.
//
use atrament_clipboard_intake_evidence::{
    ClipboardContentKind, ClipboardIntakeEvidence, ClipboardRetentionIssue,
    ClipboardRetentionIssueKind,
};

#[test]
fn first_release_clipboard_content_kinds_are_explicit() {
    let kinds = [
        ClipboardContentKind::Formula,
        ClipboardContentKind::JpegImage,
        ClipboardContentKind::PngImage,
        ClipboardContentKind::StructuredTable,
        ClipboardContentKind::Text,
        ClipboardContentKind::WebpImage,
    ];
    assert_eq!(kinds.len(), 6);
}

#[test]
fn source_provenance_remains_separate_from_decoded_content() {
    let evidence = ClipboardIntakeEvidence::<
        _, _, &'static str, &'static str,
    > {
        content: "x^2 + y^2",
        kind: ClipboardContentKind::Formula,
        provenance: "clipboard-paste-17",
        retention_issues: vec![],
        unresolved_fragments: vec![],
    };
    assert_eq!(evidence.content, "x^2 + y^2");
    assert_eq!(evidence.provenance, "clipboard-paste-17");
}

#[test]
fn retention_issues_preserve_exact_detail_and_order() {
    let evidence = ClipboardIntakeEvidence::<_, _, _, &'static str> {
        content: "table-fragment",
        kind: ClipboardContentKind::StructuredTable,
        provenance: "clipboard-table-4",
        retention_issues: vec![
            ClipboardRetentionIssue {
                detail: "merged-cell-border-style",
                kind: ClipboardRetentionIssueKind::Structure,
            },
            ClipboardRetentionIssue {
                detail: "source-app-column-width-hint",
                kind: ClipboardRetentionIssueKind::Metadata,
            },
        ],
        unresolved_fragments: vec![],
    };
    assert_eq!(evidence.retention_issues.len(), 2);
    assert_eq!(
        evidence.retention_issues[0].detail,
        "merged-cell-border-style",
    );
    assert_eq!(
        evidence.retention_issues[1].kind,
        ClipboardRetentionIssueKind::Metadata,
    );
}

#[test]
fn unresolved_fragments_remain_explicit_instead_of_being_discarded() {
    let evidence = ClipboardIntakeEvidence::<_, _, &'static str, _> {
        content: "decoded-text",
        kind: ClipboardContentKind::Text,
        provenance: "clipboard-text-8",
        retention_issues: vec![],
        unresolved_fragments: vec!["unmapped-inline-object", "unknown-field"],
    };
    assert_eq!(
        evidence.unresolved_fragments,
        ["unmapped-inline-object", "unknown-field"],
    );
}
