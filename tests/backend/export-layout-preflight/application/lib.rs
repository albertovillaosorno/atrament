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
//   - Regression evidence for the read-only Export layout preflight gate.
// - Must-Not:
//   - Create files, choose paths, or claim full Export readiness or success.
// - Allows:
//   - Inputs: Accepted revisions and revision-bound layout diagnostic sets.
//   - Outputs: Assertions over ready, blocking, incomplete, and binding
//     results.
//   - Side effects: Process-local candidate acceptance and test allocation.
// - Split-When:
//   - Complete Export preflight or file adapters receive acceptance fixtures.
// - Merge-When:
//   - Full Export application validation subsumes this layout-only gate.
// - Summary:
//   - Proves blocking or incomplete layout evidence prevents Export readiness.
// - Description:
//   - Integrates the exact fixed-region overflow diagnostic with Export
//     preflight.
// - Usage:
//   - Produce layout diagnostics, bind them to a revision, then preflight them.
// - Defaults:
//   - No path, overwrite, retry, format, render, or file-commit input exists.
//

//! Export preflight admission regression tests.

use atrament_diagnostic::{
    BlockingDisposition, Completeness, Diagnostic, DiagnosticCode,
    DiagnosticSet, Evidence, EvidenceUnit, Operation, OperationBinding,
    Remediation, Severity,
};
use atrament_export_layout_preflight::{
    ExportLayoutPreflightError, ExportLayoutPreflightResult,
    RevisionLayoutDiagnostics, preflight_layout_for_export,
};
use atrament_physical_page_profile::{
    BindingEdge, BorderShape, Length, Orientation,
    PageProfile as PhysicalPageProfile, PaperMarkAppearance, PaperMarkJoin,
    PaperMarkLayer, PaperPattern, Rect, SheetSize,
};
use atrament_semantic_fixed_region_layout::{
    AcceptedFixedPlacement, FixedRegionLayoutResult, validate_fixed_placement,
};
use atrament_semantic_notebook::{
    AcceptedIdentity, Block, BlockContent, CandidateIdentity, Flow,
    IdentityAllocator, Notebook, Page, PaperProfile,
};
use atrament_semantic_notebook_port::{
    AcceptanceOutcome, PageProfileEditOutcome, SemanticNotebookSession as _,
};
use atrament_semantic_notebook_session::SemanticNotebookSessionService;

macro_rules! admitted {
    ($result:expr) => {{
        let result = $result;
        assert!(
            result.is_ok(),
            "expected admitted layout result: {:?}",
            result.as_ref().err(),
        );
        let Ok(value) = result else { return };
        value
    }};
}

macro_rules! present {
    ($value:expr) => {{
        let value = $value;
        assert!(value.is_some(), "expected fixture value");
        let Some(value) = value else { return };
        value
    }};
}

struct AcceptedFixture {
    block: AcceptedIdentity,
    page: AcceptedIdentity,
    profile: AcceptedIdentity,
    revision: atrament_semantic_notebook::RevisionIdentity,
}

const fn geometry(outer_margin: u64) -> PhysicalPageProfile {
    PhysicalPageProfile {
        binding_edge: BindingEdge::Left,
        border_shape: BorderShape::Rectangle,
        corner_roundness: Length::ZERO,
        orientation: Orientation::Portrait,
        outer_margin: Length::from_micrometres(outer_margin),
        paper_mark_appearance: PaperMarkAppearance {
            join: PaperMarkJoin::Sharp,
            maximum_ruler_error: Length::ZERO,
        },
        paper_mark_layer: PaperMarkLayer::BelowInk,
        paper_pattern: PaperPattern::Blank,
        printable_region: Rect {
            height: Length::from_micrometres(277_000),
            width: Length::from_micrometres(190_000),
            x: Length::from_micrometres(10_000),
            y: Length::from_micrometres(10_000),
        },
        sheet: SheetSize {
            height: Length::from_micrometres(297_000),
            width: Length::from_micrometres(210_000),
        },
        top_clearance: Length::from_micrometres(10_000),
        writing_inset: Length::from_micrometres(5_000),
    }
}

fn accepted_fixture(
    session: &mut SemanticNotebookSessionService,
) -> Option<AcceptedFixture> {
    let ids = IdentityAllocator::new();
    let notebook = ids.allocate_candidate().ok()?;
    let profile = ids.allocate_candidate().ok()?;
    let page = ids.allocate_candidate().ok()?;
    let flow = ids.allocate_candidate().ok()?;
    let block = ids.allocate_candidate().ok()?;
    let candidate = Notebook {
        assets: vec![],
        constraints: vec![],
        extensions: vec![],
        id: notebook,
        output_profiles: vec![],
        page_profiles: vec![PaperProfile {
            geometry: geometry(20_000),
            id: profile,
        }],
        pages: vec![Page {
            flows: vec![Flow {
                blocks: vec![Block {
                    content: BlockContent::Rule,
                    extensions: vec![],
                    id: block,
                    provenance: None,
                    style: None,
                }],
                id: flow,
            }],
            id: page,
            paper_profile: profile,
        }],
        provenance: vec![],
        styles: vec![],
    };
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        return None;
    };
    let mapped = |identity: CandidateIdentity| {
        mapping
            .iter()
            .find(|entry| entry.candidate == identity)
            .map(|entry| entry.accepted)
    };
    Some(AcceptedFixture {
        block: mapped(block)?,
        page: mapped(page)?,
        profile: mapped(profile)?,
        revision,
    })
}

const fn complete_empty() -> DiagnosticSet {
    DiagnosticSet {
        completeness: Completeness::Complete,
        diagnostics: vec![],
    }
}

#[test]
fn real_six_mm_fixed_overflow_blocks_export_layout_preflight() {
    let mut session = SemanticNotebookSessionService::default();
    let fixture = present!(accepted_fixture(&mut session));
    let layout = admitted!(validate_fixed_placement(
        present!(session.current()),
        AcceptedFixedPlacement {
            object: fixture.block,
            page: fixture.page,
            rectangle: Rect {
                height: Length::from_micrometres(23_000),
                width: Length::from_micrometres(50_000),
                x: Length::from_micrometres(35_000),
                y: Length::from_micrometres(270_000),
            },
            revision: fixture.revision,
        },
    ));
    assert!(matches!(layout, FixedRegionLayoutResult::Overflow { .. }));
    let FixedRegionLayoutResult::Overflow { diagnostics, .. } = layout else {
        return;
    };
    let result = admitted!(preflight_layout_for_export(
        present!(session.current()),
        fixture.revision,
        admitted!(RevisionLayoutDiagnostics::bind(
            fixture.revision,
            &diagnostics,
        )),
    ));
    assert_eq!(
        result,
        ExportLayoutPreflightResult::Blocked {
            diagnostics: diagnostics.clone(),
            revision: fixture.revision,
        },
    );
    let blocked = diagnostics;
    assert_eq!(blocked.diagnostics.len(), 1);
    assert_eq!(
        present!(blocked.diagnostics.first()).code,
        DiagnosticCode::LayoutFixedRegionOverflow,
    );
    assert_eq!(
        present!(blocked.diagnostics.first()).evidence.get(1),
        Some(&Evidence::PhysicalLength {
            micrometres: 6_000,
            quantity: atrament_diagnostic::PhysicalLengthQuantity::Overflow,
        }),
    );
}

#[test]
fn complete_empty_layout_evidence_is_ready_but_does_not_mutate_revision() {
    let mut session = SemanticNotebookSessionService::default();
    let fixture = present!(accepted_fixture(&mut session));
    let before = present!(session.current()).clone();
    let diagnostics = complete_empty();
    assert_eq!(
        preflight_layout_for_export(
            &before,
            fixture.revision,
            admitted!(RevisionLayoutDiagnostics::bind(
                fixture.revision,
                &diagnostics,
            )),
        ),
        Ok(ExportLayoutPreflightResult::Ready {
            diagnostics,
            revision: fixture.revision,
        }),
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn incomplete_layout_evidence_never_reports_ready() {
    let mut session = SemanticNotebookSessionService::default();
    let fixture = present!(accepted_fixture(&mut session));
    let diagnostics = DiagnosticSet {
        completeness: Completeness::Incomplete,
        diagnostics: vec![],
    };
    assert_eq!(
        preflight_layout_for_export(
            present!(session.current()),
            fixture.revision,
            admitted!(RevisionLayoutDiagnostics::bind(
                fixture.revision,
                &diagnostics,
            )),
        ),
        Ok(ExportLayoutPreflightResult::Incomplete {
            diagnostics,
            revision: fixture.revision,
        }),
    );
}

#[test]
fn stale_requested_revision_rejects_before_layout_evidence() {
    let mut session = SemanticNotebookSessionService::default();
    let fixture = present!(accepted_fixture(&mut session));
    let edit = session.replace_page_profile(
        fixture.revision,
        fixture.profile,
        geometry(40_000),
    );
    assert!(matches!(edit, PageProfileEditOutcome::Applied { .. }));
    let PageProfileEditOutcome::Applied {
        revision: current,
        ..
    } = edit
    else {
        return;
    };
    let diagnostics = complete_empty();
    assert_eq!(
        preflight_layout_for_export(
            present!(session.current()),
            fixture.revision,
            admitted!(RevisionLayoutDiagnostics::bind(
                fixture.revision,
                &diagnostics,
            )),
        ),
        Err(ExportLayoutPreflightError::RequestedRevisionMismatch {
            current,
            requested: fixture.revision,
        }),
    );
}

#[test]
fn layout_evidence_from_another_revision_cannot_be_reused() {
    let mut session = SemanticNotebookSessionService::default();
    let fixture = present!(accepted_fixture(&mut session));
    let edit = session.replace_page_profile(
        fixture.revision,
        fixture.profile,
        geometry(40_000),
    );
    assert!(matches!(edit, PageProfileEditOutcome::Applied { .. }));
    let PageProfileEditOutcome::Applied {
        revision: current,
        ..
    } = edit
    else {
        return;
    };
    let diagnostics = complete_empty();
    assert_eq!(
        preflight_layout_for_export(
            present!(session.current()),
            current,
            admitted!(RevisionLayoutDiagnostics::bind(
                fixture.revision,
                &diagnostics,
            )),
        ),
        Err(ExportLayoutPreflightError::LayoutRevisionMismatch {
            requested: current,
            supplied: fixture.revision,
        }),
    );
}

#[test]
fn non_layout_code_cannot_be_relabelled_as_layout() {
    let mut session = SemanticNotebookSessionService::default();
    let fixture = present!(accepted_fixture(&mut session));
    let mut diagnostics =
        present!(real_overflow_diagnostics(&session, &fixture));
    let diagnostic = present!(diagnostics.diagnostics.first_mut());
    diagnostic.code = DiagnosticCode::SessionDraftResourceLimit;

    assert_eq!(
        present!(diagnostics.diagnostics.first()).operation.operation,
        Operation::Layout,
    );
    assert!(matches!(
        RevisionLayoutDiagnostics::bind(fixture.revision, &diagnostics),
        Err(ExportLayoutPreflightError::NonLayoutDiagnostic),
    ));
}

#[test]
fn non_layout_diagnostic_cannot_be_smuggled_into_layout_preflight() {
    let mut session = SemanticNotebookSessionService::default();
    let fixture = present!(accepted_fixture(&mut session));
    let diagnostics = DiagnosticSet {
        completeness: Completeness::Complete,
        diagnostics: vec![Diagnostic {
            code: DiagnosticCode::SessionDraftResourceLimit,
            disposition: BlockingDisposition::Blocking,
            evidence: vec![Evidence::LimitExceeded {
                maximum: 1,
                observed: 2,
                unit: EvidenceUnit::Bytes,
            }],
            locations: vec![],
            operation: OperationBinding {
                contexts: vec![],
                operation: Operation::SessionDraftReplace,
            },
            remediations: vec![Remediation::ReduceInput],
            severity: Severity::Error,
        }],
    };
    assert!(matches!(
        RevisionLayoutDiagnostics::bind(fixture.revision, &diagnostics),
        Err(ExportLayoutPreflightError::NonLayoutDiagnostic),
    ));
}

fn real_overflow_diagnostics(
    session: &SemanticNotebookSessionService,
    fixture: &AcceptedFixture,
) -> Option<DiagnosticSet> {
    let layout = validate_fixed_placement(
        session.current()?,
        AcceptedFixedPlacement {
            object: fixture.block,
            page: fixture.page,
            rectangle: Rect {
                height: Length::from_micrometres(23_000),
                width: Length::from_micrometres(50_000),
                x: Length::from_micrometres(35_000),
                y: Length::from_micrometres(270_000),
            },
            revision: fixture.revision,
        },
    )
    .ok()?;
    let FixedRegionLayoutResult::Overflow { diagnostics, .. } = layout else {
        return None;
    };
    Some(diagnostics)
}

#[test]
fn blocking_overflow_cannot_be_downgraded_to_advisory() {
    let mut session = SemanticNotebookSessionService::default();
    let fixture = present!(accepted_fixture(&mut session));
    let mut diagnostics =
        present!(real_overflow_diagnostics(&session, &fixture));
    let diagnostic = present!(diagnostics.diagnostics.first_mut());
    diagnostic.disposition = BlockingDisposition::Advisory;

    assert!(matches!(
        RevisionLayoutDiagnostics::bind(fixture.revision, &diagnostics),
        Err(ExportLayoutPreflightError::NonLayoutDiagnostic),
    ));
}

#[test]
fn incomplete_blocking_layout_evidence_remains_incomplete() {
    let mut session = SemanticNotebookSessionService::default();
    let fixture = present!(accepted_fixture(&mut session));
    let mut diagnostics =
        present!(real_overflow_diagnostics(&session, &fixture));
    diagnostics.completeness = Completeness::Incomplete;
    let expected = diagnostics.clone();

    assert_eq!(
        preflight_layout_for_export(
            present!(session.current()),
            fixture.revision,
            admitted!(RevisionLayoutDiagnostics::bind(
                fixture.revision,
                &diagnostics,
            )),
        ),
        Ok(ExportLayoutPreflightResult::Incomplete {
            diagnostics: expected,
            revision: fixture.revision,
        }),
    );
}

#[test]
fn diagnostic_revision_context_cannot_be_relabelled_as_current() {
    let mut session = SemanticNotebookSessionService::default();
    let fixture = present!(accepted_fixture(&mut session));
    let mut diagnostics =
        present!(real_overflow_diagnostics(&session, &fixture));
    let diagnostic = present!(diagnostics.diagnostics.first_mut());
    let context = present!(diagnostic.operation.contexts.first_mut());
    context.identity = String::from("RevisionIdentity(999999)");

    assert!(matches!(
        RevisionLayoutDiagnostics::bind(fixture.revision, &diagnostics),
        Err(ExportLayoutPreflightError::DiagnosticContextMismatch),
    ));
}

#[test]
fn layout_diagnostic_requires_exactly_one_revision_context() {
    let mut session = SemanticNotebookSessionService::default();
    let fixture = present!(accepted_fixture(&mut session));
    let mut missing = present!(real_overflow_diagnostics(&session, &fixture));
    present!(missing.diagnostics.first_mut()).operation.contexts.clear();
    assert!(matches!(
        RevisionLayoutDiagnostics::bind(fixture.revision, &missing),
        Err(ExportLayoutPreflightError::DiagnosticContextMissing),
    ));

    let mut duplicated =
        present!(real_overflow_diagnostics(&session, &fixture));
    let original_diagnostic = present!(duplicated.diagnostics.first());
    let repeated =
        present!(original_diagnostic.operation.contexts.first()).clone();
    let mutable_diagnostic = present!(duplicated.diagnostics.first_mut());
    mutable_diagnostic.operation.contexts.push(repeated);
    assert!(matches!(
        RevisionLayoutDiagnostics::bind(fixture.revision, &duplicated),
        Err(ExportLayoutPreflightError::DiagnosticContextMismatch),
    ));
}
