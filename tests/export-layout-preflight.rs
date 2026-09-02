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
    AcceptanceOutcome, PageProfileEditOutcome, SemanticNotebookSession,
};
use atrament_semantic_notebook_session::SemanticNotebookSessionService;

struct AcceptedFixture {
    block: AcceptedIdentity,
    page: AcceptedIdentity,
    profile: AcceptedIdentity,
    revision: atrament_semantic_notebook::RevisionIdentity,
}

fn geometry(outer_margin: u64) -> PhysicalPageProfile {
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
) -> AcceptedFixture {
    let ids = IdentityAllocator::new();
    let notebook = ids.allocate_candidate().expect("notebook");
    let profile = ids.allocate_candidate().expect("profile");
    let page = ids.allocate_candidate().expect("page");
    let flow = ids.allocate_candidate().expect("flow");
    let block = ids.allocate_candidate().expect("block");
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
            page_profile: profile,
        }],
        provenance: vec![],
        styles: vec![],
    };
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(candidate)
    else {
        panic!("candidate must be accepted");
    };
    let mapped = |candidate: CandidateIdentity| {
        mapping
            .iter()
            .find(|entry| entry.candidate == candidate)
            .expect("candidate identity mapped")
            .accepted
    };
    AcceptedFixture {
        block: mapped(block),
        page: mapped(page),
        profile: mapped(profile),
        revision,
    }
}

fn complete_empty() -> DiagnosticSet {
    DiagnosticSet {
        completeness: Completeness::Complete,
        diagnostics: vec![],
    }
}

#[test]
fn real_six_mm_fixed_overflow_blocks_export_layout_preflight() {
    let mut session = SemanticNotebookSessionService::default();
    let fixture = accepted_fixture(&mut session);
    let layout = validate_fixed_placement(
        session.current().expect("accepted revision"),
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
    .expect("fixed layout validation");
    let FixedRegionLayoutResult::Overflow { diagnostics, .. } = layout else {
        panic!("fixture must overflow");
    };
    let result = preflight_layout_for_export(
        session.current().expect("accepted revision"),
        fixture.revision,
        RevisionLayoutDiagnostics::bind(fixture.revision, &diagnostics)
            .expect("layout diagnostic binding"),
    )
    .expect("revision binding is valid");
    let ExportLayoutPreflightResult::Blocked {
        diagnostics: blocked,
        revision,
    } = result
    else {
        panic!("blocking layout diagnostic must block preflight");
    };
    assert_eq!(revision, fixture.revision);
    assert_eq!(blocked, diagnostics);
    assert_eq!(blocked.diagnostics.len(), 1);
    assert_eq!(
        blocked.diagnostics[0].code,
        DiagnosticCode::LayoutFixedRegionOverflow,
    );
    assert_eq!(
        blocked.diagnostics[0].evidence[1],
        Evidence::PhysicalLength {
            micrometres: 6_000,
            quantity: atrament_diagnostic::PhysicalLengthQuantity::Overflow,
        },
    );
}

#[test]
fn complete_empty_layout_evidence_is_ready_but_does_not_mutate_revision() {
    let mut session = SemanticNotebookSessionService::default();
    let fixture = accepted_fixture(&mut session);
    let before = session.current().expect("accepted revision").clone();
    let diagnostics = complete_empty();
    assert_eq!(
        preflight_layout_for_export(
            &before,
            fixture.revision,
            RevisionLayoutDiagnostics::bind(fixture.revision, &diagnostics)
                .expect("layout diagnostic binding"),
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
    let fixture = accepted_fixture(&mut session);
    let diagnostics = DiagnosticSet {
        completeness: Completeness::Incomplete,
        diagnostics: vec![],
    };
    assert_eq!(
        preflight_layout_for_export(
            session.current().expect("accepted revision"),
            fixture.revision,
            RevisionLayoutDiagnostics::bind(fixture.revision, &diagnostics)
                .expect("layout diagnostic binding"),
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
    let fixture = accepted_fixture(&mut session);
    let PageProfileEditOutcome::Applied { revision: current, .. } = session
        .replace_page_profile(
            fixture.revision,
            fixture.profile,
            geometry(40_000),
        )
    else {
        panic!("profile edit must apply");
    };
    let diagnostics = complete_empty();
    assert_eq!(
        preflight_layout_for_export(
            session.current().expect("edited revision"),
            fixture.revision,
            RevisionLayoutDiagnostics::bind(fixture.revision, &diagnostics)
                .expect("layout diagnostic binding"),
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
    let fixture = accepted_fixture(&mut session);
    let PageProfileEditOutcome::Applied { revision: current, .. } = session
        .replace_page_profile(
            fixture.revision,
            fixture.profile,
            geometry(40_000),
        )
    else {
        panic!("profile edit must apply");
    };
    let diagnostics = complete_empty();
    assert_eq!(
        preflight_layout_for_export(
            session.current().expect("edited revision"),
            current,
            RevisionLayoutDiagnostics::bind(fixture.revision, &diagnostics)
                .expect("layout diagnostic binding"),
        ),
        Err(ExportLayoutPreflightError::LayoutRevisionMismatch {
            requested: current,
            supplied: fixture.revision,
        }),
    );
}

#[test]
fn non_layout_diagnostic_cannot_be_smuggled_into_layout_preflight() {
    let mut session = SemanticNotebookSessionService::default();
    let fixture = accepted_fixture(&mut session);
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
) -> DiagnosticSet {
    let layout = validate_fixed_placement(
        session.current().expect("accepted revision"),
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
    .expect("fixed layout validation");
    let FixedRegionLayoutResult::Overflow { diagnostics, .. } = layout else {
        panic!("fixture must overflow");
    };
    diagnostics
}

#[test]
fn diagnostic_revision_context_cannot_be_relabelled_as_current() {
    let mut session = SemanticNotebookSessionService::default();
    let fixture = accepted_fixture(&mut session);
    let mut diagnostics = real_overflow_diagnostics(&session, &fixture);
    diagnostics.diagnostics[0].operation.contexts[0].identity =
        String::from("RevisionIdentity(999999)");

    assert!(matches!(
        RevisionLayoutDiagnostics::bind(fixture.revision, &diagnostics),
        Err(ExportLayoutPreflightError::DiagnosticContextMismatch),
    ));
}

#[test]
fn layout_diagnostic_requires_exactly_one_revision_context() {
    let mut session = SemanticNotebookSessionService::default();
    let fixture = accepted_fixture(&mut session);
    let mut missing = real_overflow_diagnostics(&session, &fixture);
    missing.diagnostics[0].operation.contexts.clear();
    assert!(matches!(
        RevisionLayoutDiagnostics::bind(fixture.revision, &missing),
        Err(ExportLayoutPreflightError::DiagnosticContextMissing),
    ));

    let mut duplicated = real_overflow_diagnostics(&session, &fixture);
    let repeated = duplicated.diagnostics[0].operation.contexts[0].clone();
    duplicated.diagnostics[0].operation.contexts.push(repeated);
    assert!(matches!(
        RevisionLayoutDiagnostics::bind(fixture.revision, &duplicated),
        Err(ExportLayoutPreflightError::DiagnosticContextMismatch),
    ));
}
