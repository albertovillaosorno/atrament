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
//   - Regression evidence for accepted fixed-region overflow validation.
// - Must-Not:
//   - Invent anchor, alignment, collision, minimum-size, or repair semantics.
// - Allows:
//   - Inputs: Accepted revisions plus explicit solver-derived rectangles.
//   - Outputs: Assertions over bounds results and typed layout diagnostics.
//   - Side effects: Process-local candidate acceptance and test allocations.
// - Split-When:
//   - Full placement solving or collision diagnostics receive separate
//     fixtures.
// - Merge-When:
//   - Fixed geometry validation becomes part of complete layout acceptance
//     tests.
// - Summary:
//   - Proves overflow binds to accepted revision, page, block, and paper
//     profile.
// - Description:
//   - Covers exact 6 mm evidence, ownership, stale geometry, and profile edits.
// - Usage:
//   - Accept a candidate, provide derived geometry, then validate it directly.
// - Defaults:
//   - Uses deterministic A4-like portrait geometry in canonical micrometres.
//
use atrament_diagnostic::{
    BlockingDisposition, Completeness, DiagnosticCode, Evidence, LocationKind,
    LocationRole, Operation, OperationContextKind, PhysicalBoundaryEdge,
    PhysicalLengthQuantity, Remediation, Severity,
};
use atrament_fixed_region_bounds::{BoundaryEdge, BoundsError};
use atrament_physical_page_profile::{
    BindingEdge, BorderShape, Length, Orientation,
    PageProfile as PhysicalPageProfile, PageProfileError, PaperMarkAppearance,
    PaperMarkJoin, PaperMarkLayer, PaperPattern, Rect, SheetSize,
};
use atrament_semantic_fixed_region_layout::{
    AcceptedFixedPlacement, FixedRegionLayoutError, FixedRegionLayoutResult,
    validate_fixed_placement,
};
use atrament_semantic_notebook::{
    AcceptedIdentity, Block, BlockContent, CandidateIdentity, Flow,
    IdentityAllocator, Notebook, Page, PaperProfile,
};
use atrament_semantic_notebook_port::{
    AcceptanceOutcome, PageProfileEditOutcome, SemanticNotebookSession,
};
use atrament_semantic_notebook_session::SemanticNotebookSessionService;

struct CandidateFixture {
    block: CandidateIdentity,
    nested: CandidateIdentity,
    notebook: Notebook<CandidateIdentity>,
    page_one: CandidateIdentity,
    page_two: CandidateIdentity,
    profile_one: CandidateIdentity,
}

fn accepted_for(
    mapping: &[atrament_semantic_notebook_port::IdentityMapping],
    candidate: CandidateIdentity,
) -> AcceptedIdentity {
    mapping
        .iter()
        .find(|entry| entry.candidate == candidate)
        .expect("candidate identity is mapped")
        .accepted
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

fn candidate_fixture(ids: &IdentityAllocator) -> CandidateFixture {
    let notebook_id = ids.allocate_candidate().expect("notebook");
    let profile_one = ids.allocate_candidate().expect("profile one");
    let profile_two = ids.allocate_candidate().expect("profile two");
    let page_one = ids.allocate_candidate().expect("page one");
    let page_two = ids.allocate_candidate().expect("page two");
    let flow_one = ids.allocate_candidate().expect("flow one");
    let flow_two = ids.allocate_candidate().expect("flow two");
    let block = ids.allocate_candidate().expect("block");
    let nested = ids.allocate_candidate().expect("nested block");
    let other = ids.allocate_candidate().expect("other block");
    CandidateFixture {
        block,
        nested,
        notebook: Notebook {
            assets: vec![],
            constraints: vec![],
            extensions: vec![],
            id: notebook_id,
            output_profiles: vec![],
            page_profiles: vec![
                PaperProfile {
                    geometry: geometry(20_000),
                    id: profile_one,
                },
                PaperProfile {
                    geometry: geometry(20_000),
                    id: profile_two,
                },
            ],
            pages: vec![
                Page {
                    flows: vec![Flow {
                        blocks: vec![
                            Block {
                                content: BlockContent::Rule,
                                extensions: vec![],
                                id: block,
                                provenance: None,
                                style: None,
                            },
                            Block {
                                content: BlockContent::Callout(vec![Block {
                                    content: BlockContent::Rule,
                                    extensions: vec![],
                                    id: nested,
                                    provenance: None,
                                    style: None,
                                }]),
                                extensions: vec![],
                                id: ids.allocate_candidate().expect("callout"),
                                provenance: None,
                                style: None,
                            },
                        ],
                        id: flow_one,
                    }],
                    id: page_one,
                    page_profile: profile_one,
                },
                Page {
                    flows: vec![Flow {
                        blocks: vec![Block {
                            content: BlockContent::Rule,
                            extensions: vec![],
                            id: other,
                            provenance: None,
                            style: None,
                        }],
                        id: flow_two,
                    }],
                    id: page_two,
                    page_profile: profile_two,
                },
            ],
            provenance: vec![],
            styles: vec![],
        },
        page_one,
        page_two,
        profile_one,
    }
}

fn placement(
    revision: atrament_semantic_notebook::RevisionIdentity,
    page: AcceptedIdentity,
    object: AcceptedIdentity,
    rectangle: Rect,
) -> AcceptedFixedPlacement {
    AcceptedFixedPlacement {
        object,
        page,
        rectangle,
        revision,
    }
}

fn rect(x: u64, y: u64, width: u64, height: u64) -> Rect {
    Rect {
        height: Length::from_micrometres(height),
        width: Length::from_micrometres(width),
        x: Length::from_micrometres(x),
        y: Length::from_micrometres(y),
    }
}

#[test]
fn first_journey_bottom_overflow_produces_exact_blocking_six_mm_diagnostic() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let page = accepted_for(&mapping, fixture.page_one);
    let object = accepted_for(&mapping, fixture.block);
    let result = validate_fixed_placement(
        session.current().expect("accepted revision"),
        placement(
            revision,
            page,
            object,
            rect(35_000, 270_000, 50_000, 23_000),
        ),
    )
    .expect("valid derived placement");
    let FixedRegionLayoutResult::Overflow { diagnostics, report, .. } = result
    else {
        panic!("placement must overflow");
    };
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].edge, BoundaryEdge::Bottom);
    assert_eq!(report.violations[0].amount.micrometres(), 6_000);
    assert_eq!(diagnostics.completeness, Completeness::Complete);
    assert_eq!(diagnostics.diagnostics.len(), 1);
    let diagnostic = &diagnostics.diagnostics[0];
    assert_eq!(diagnostic.code, DiagnosticCode::LayoutFixedRegionOverflow);
    assert_eq!(diagnostic.disposition, BlockingDisposition::Blocking);
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.operation.operation, Operation::Layout);
    assert_eq!(diagnostic.operation.contexts.len(), 1);
    assert_eq!(
        diagnostic.operation.contexts[0].kind,
        OperationContextKind::AcceptedRevision,
    );
    assert_eq!(diagnostic.locations.len(), 2);
    assert_eq!(diagnostic.locations[0].kind, LocationKind::Object);
    assert_eq!(diagnostic.locations[0].role, LocationRole::Primary);
    assert_eq!(diagnostic.locations[1].kind, LocationKind::Structure);
    assert_eq!(diagnostic.locations[1].role, LocationRole::Related);
    assert_ne!(
        diagnostic.locations[0].identity,
        diagnostic.locations[1].identity
    );
    assert_eq!(diagnostic.remediations, vec![Remediation::ChangeConstraint]);
    assert_eq!(diagnostic.evidence, vec![
        Evidence::PhysicalBoundary {
            edge: PhysicalBoundaryEdge::Bottom,
        },
        Evidence::PhysicalLength {
            micrometres: 6_000,
            quantity: PhysicalLengthQuantity::Overflow,
        },
    ],);
}

#[test]
fn exact_fit_is_read_only_and_produces_no_diagnostic_result() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let page = accepted_for(&mapping, fixture.page_one);
    let object = accepted_for(&mapping, fixture.block);
    let before = session.current().expect("accepted revision").clone();
    assert_eq!(
        validate_fixed_placement(
            &before,
            placement(
                revision,
                page,
                object,
                rect(35_000, 20_000, 165_000, 267_000)
            ),
        ),
        Ok(FixedRegionLayoutResult::WithinBounds { object, page }),
    );
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn multi_edge_overflow_produces_one_complete_diagnostic_per_edge() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let page = accepted_for(&mapping, fixture.page_one);
    let object = accepted_for(&mapping, fixture.block);
    let result = validate_fixed_placement(
        session.current().expect("accepted revision"),
        placement(
            revision,
            page,
            object,
            rect(30_000, 15_000, 175_000, 277_000),
        ),
    )
    .expect("valid bounds comparison");
    let FixedRegionLayoutResult::Overflow { diagnostics, report, .. } = result
    else {
        panic!("placement must overflow");
    };
    let edges = report
        .violations
        .iter()
        .map(|violation| violation.edge)
        .collect::<Vec<_>>();
    assert_eq!(edges, vec![
        BoundaryEdge::Bottom,
        BoundaryEdge::Left,
        BoundaryEdge::Right,
        BoundaryEdge::Top
    ],);
    assert_eq!(diagnostics.diagnostics.len(), 4);
    assert!(
        diagnostics
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code
                == DiagnosticCode::LayoutFixedRegionOverflow)
    );
}

#[test]
fn stale_placement_is_rejected_before_new_page_geometry_is_used() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let page = accepted_for(&mapping, fixture.page_one);
    let object = accepted_for(&mapping, fixture.block);
    let profile = accepted_for(&mapping, fixture.profile_one);
    let stale =
        placement(revision, page, object, rect(35_000, 20_000, 10_000, 10_000));
    let PageProfileEditOutcome::Applied { revision: current, .. } =
        session.replace_page_profile(revision, profile, geometry(40_000))
    else {
        panic!("profile edit must apply");
    };
    assert_eq!(
        validate_fixed_placement(
            session.current().expect("edited revision"),
            stale
        ),
        Err(FixedRegionLayoutError::RevisionMismatch {
            accepted: current,
            placement: revision,
        }),
    );
    let fresh = AcceptedFixedPlacement {
        revision: current,
        ..stale
    };
    let result = validate_fixed_placement(
        session.current().expect("edited revision"),
        fresh,
    )
    .expect("fresh placement checks new geometry");
    let FixedRegionLayoutResult::Overflow { report, .. } = result else {
        panic!("new margin must make the old x coordinate overflow left");
    };
    assert_eq!(report.violations[0].edge, BoundaryEdge::Left);
    assert_eq!(report.violations[0].amount.micrometres(), 20_000);
}

#[test]
fn object_must_belong_to_the_named_accepted_page() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let page_one = accepted_for(&mapping, fixture.page_one);
    let page_two = accepted_for(&mapping, fixture.page_two);
    let object = accepted_for(&mapping, fixture.block);
    assert_eq!(
        validate_fixed_placement(
            session.current().expect("accepted revision"),
            placement(revision, page_two, object, rect(35_000, 20_000, 1, 1)),
        ),
        Err(FixedRegionLayoutError::ObjectNotOnPage { object, page: page_two }),
    );
    let fake_page = object;
    assert_eq!(
        validate_fixed_placement(
            session.current().expect("accepted revision"),
            placement(revision, fake_page, object, rect(35_000, 20_000, 1, 1)),
        ),
        Err(FixedRegionLayoutError::PageNotFound { page: fake_page }),
    );
    assert_ne!(page_one, page_two);
}

#[test]
fn nested_block_is_still_owned_by_its_accepted_page() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let page = accepted_for(&mapping, fixture.page_one);
    let nested = accepted_for(&mapping, fixture.nested);
    assert_eq!(
        validate_fixed_placement(
            session.current().expect("accepted revision"),
            placement(revision, page, nested, rect(35_000, 20_000, 1, 1)),
        ),
        Ok(FixedRegionLayoutResult::WithinBounds { object: nested, page }),
    );
}

#[test]
fn very_large_overflow_amount_is_lossless_in_diagnostic_evidence() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let page = accepted_for(&mapping, fixture.page_one);
    let object = accepted_for(&mapping, fixture.block);
    let x = u64::MAX - 1;
    let result = validate_fixed_placement(
        session.current().expect("accepted revision"),
        placement(revision, page, object, rect(x, 20_000, 1, 1)),
    )
    .expect("maximum right coordinate is representable");
    let FixedRegionLayoutResult::Overflow { diagnostics, report, .. } = result
    else {
        panic!("far-right placement must overflow");
    };
    let amount = report.violations[0].amount.micrometres();
    assert!(amount > i64::MAX as u64);
    assert_eq!(
        diagnostics.diagnostics[0].evidence[1],
        Evidence::PhysicalLength {
            micrometres: i128::from(amount),
            quantity: PhysicalLengthQuantity::Overflow,
        },
    );
}

#[test]
fn placement_coordinate_overflow_is_typed_before_diagnostics() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let page = accepted_for(&mapping, fixture.page_one);
    let object = accepted_for(&mapping, fixture.block);
    assert_eq!(
        validate_fixed_placement(
            session.current().expect("accepted revision"),
            placement(revision, page, object, rect(u64::MAX, 20_000, 1, 1)),
        ),
        Err(FixedRegionLayoutError::Bounds(
            BoundsError::ObjectCoordinateOverflow,
        )),
    );
}

trait CorruptAcceptedRevisionForTest {
    fn forget_profiles(&mut self);
    fn invalidate_first_profile(&mut self);
}

impl CorruptAcceptedRevisionForTest
    for atrament_semantic_notebook::AcceptedRevision
{
    fn forget_profiles(&mut self) {
        self.notebook.page_profiles.clear();
    }

    fn invalidate_first_profile(&mut self) {
        self.notebook.page_profiles[0].geometry.sheet.width = Length::ZERO;
    }
}

#[test]
fn missing_or_invalid_profile_never_falls_back_to_arbitrary_bounds() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let page = accepted_for(&mapping, fixture.page_one);
    let object = accepted_for(&mapping, fixture.block);
    let profile = accepted_for(&mapping, fixture.profile_one);
    let checked = placement(revision, page, object, rect(35_000, 20_000, 1, 1));
    let mut missing = session.current().expect("accepted revision").clone();
    missing.forget_profiles();
    assert_eq!(
        validate_fixed_placement(&missing, checked),
        Err(FixedRegionLayoutError::MissingPageProfile { page, profile }),
    );
    let mut invalid = session.current().expect("accepted revision").clone();
    invalid.invalidate_first_profile();
    assert_eq!(
        validate_fixed_placement(&invalid, checked),
        Err(FixedRegionLayoutError::InvalidPageProfile {
            page,
            reason: PageProfileError::SheetDimensionIsZero,
        }),
    );
}
