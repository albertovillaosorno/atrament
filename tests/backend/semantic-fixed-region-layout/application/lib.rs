// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - LicenseRef-Proprietary
// Confidential:
//   - true
// License-File:
//   - LICENSE
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

//! Fixed-region semantic layout regression tests.

use atrament_diagnostic::{
    BlockingDisposition, Completeness, DiagnosticCode, Evidence, LocationKind,
    LocationRole, Operation, OperationContextKind, PhysicalBoundaryEdge,
    PhysicalLengthQuantity, Remediation, Severity,
};
use atrament_fixed_region_bounds::{
    BoundaryEdge, BoundsError, check_bounds,
};
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
    AcceptanceOutcome, CANDIDATE_BLOCK_NESTING_LIMIT, PageProfileEditOutcome,
    SemanticNotebookSession,
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
                    paper_profile: profile_one,
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
                    paper_profile: profile_two,
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

fn matches_within(
    result: Result<FixedRegionLayoutResult, FixedRegionLayoutError>,
    expected: AcceptedFixedPlacement,
) -> bool {
    result.is_ok_and(|checked| {
        checked.is_within_bounds()
            && checked.object() == expected.object
            && checked.page() == expected.page
            && checked.rectangle() == expected.rectangle
            && checked.revision() == expected.revision
    })
}

fn rect(x: u64, y: u64, width: u64, height: u64) -> Rect {
    Rect {
        height: Length::from_micrometres(height),
        width: Length::from_micrometres(width),
        x: Length::from_micrometres(x),
        y: Length::from_micrometres(y),
    }
}

fn next_fixed_layout_value(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *seed >> 32
}

const fn diagnostic_boundary(edge: BoundaryEdge) -> PhysicalBoundaryEdge {
    match edge {
        BoundaryEdge::Bottom => PhysicalBoundaryEdge::Bottom,
        BoundaryEdge::Left => PhysicalBoundaryEdge::Left,
        BoundaryEdge::Right => PhysicalBoundaryEdge::Right,
        BoundaryEdge::Top => PhysicalBoundaryEdge::Top,
    }
}


fn boundary_mask(
    violations: &[atrament_fixed_region_bounds::BoundaryViolation],
) -> usize {
    violations.iter().fold(0usize, |mask, violation| {
        mask | match violation.edge {
            BoundaryEdge::Bottom => 1,
            BoundaryEdge::Left => 2,
            BoundaryEdge::Right => 4,
            BoundaryEdge::Top => 8,
        }
    })
}

#[test]
fn accepted_fixed_layout_diagnostics_match_bounds_oracle() {
    const CASES: usize = 20_000;
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
    let writable = geometry(20_000)
        .writable_region()
        .expect("fixture writable region");
    let accepted = session.current().expect("accepted revision");
    let mut seed = 0x5eed_f1ed_2026_u64;
    let mut seen_boundary_masks = [false; 16];

    for case in 0..CASES {
        let rectangle = rect(
            next_fixed_layout_value(&mut seed) % 230_000,
            next_fixed_layout_value(&mut seed) % 310_000,
            next_fixed_layout_value(&mut seed) % 210_000,
            next_fixed_layout_value(&mut seed) % 290_000,
        );
        let expected =
            check_bounds(writable, rectangle).expect("small generated bounds");
        seen_boundary_masks[boundary_mask(&expected.violations)] = true;
        let actual = validate_fixed_placement(
            accepted,
            placement(revision, page, object, rectangle),
        )
        .expect("generated placement keeps accepted authority");
        assert_eq!(actual.revision(), revision, "generated case {case}");
        assert_eq!(actual.rectangle(), rectangle, "generated case {case}");
        assert_eq!(actual.writable(), writable, "generated case {case}");
        assert_eq!(actual.object(), object, "generated case {case}");
        assert_eq!(actual.page(), page, "generated case {case}");
        assert_eq!(
            actual.is_within_bounds(),
            expected.is_within_bounds(),
            "bounds result mismatch in generated case {case}",
        );
        assert_eq!(actual.report(), &expected, "generated case {case}");
        if expected.is_within_bounds() {
            assert!(actual.diagnostics().is_none());
            continue;
        }
        let diagnostics = actual.diagnostics()
            .expect("overflow case must return diagnostics");
        let report = actual.report();
        assert_eq!(diagnostics.completeness, Completeness::Complete);
        assert_eq!(diagnostics.diagnostics.len(), report.violations.len());
        for (diagnostic, violation) in
            diagnostics.diagnostics.iter().zip(&report.violations)
        {
            assert_eq!(
                diagnostic.code,
                DiagnosticCode::LayoutFixedRegionOverflow,
            );
            assert_eq!(
                diagnostic.disposition,
                BlockingDisposition::Blocking,
            );
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
            assert_eq!(
                diagnostic.locations[1].kind,
                LocationKind::Structure,
            );
            assert_eq!(diagnostic.locations[1].role, LocationRole::Related);
            assert_eq!(
                diagnostic.remediations,
                [Remediation::ChangeConstraint],
            );
            assert_eq!(diagnostic.evidence, [
                Evidence::PhysicalBoundary {
                    edge: diagnostic_boundary(violation.edge),
                },
                Evidence::PhysicalLength {
                    micrometres: i128::from(
                        violation.amount.micrometres(),
                    ),
                    quantity: PhysicalLengthQuantity::Overflow,
                },
            ]);
        }
    }
    assert!(seen_boundary_masks.into_iter().all(|seen| seen));
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
    assert!(!result.is_within_bounds(), "placement must overflow");
    let diagnostics = result.diagnostics().expect("overflow diagnostics");
    let report = result.report();
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
    let checked = placement(
        revision, page, object, rect(35_000, 20_000, 165_000, 267_000),
    );
    assert!(matches_within(
        validate_fixed_placement(&before, checked),
        checked,
    ));
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn zero_size_fixed_rectangles_retain_exact_position_and_bounds()
-> Result<(), &'static str> {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("valid zero-size fixture was refused");
    };
    let page = accepted_for(&mapping, fixture.page_one);
    let object = accepted_for(&mapping, fixture.block);
    let accepted = session.current().ok_or("accepted revision missing")?;
    let inside = rect(35_000, 20_000, 0, 0);
    let within = validate_fixed_placement(
        accepted,
        placement(revision, page, object, inside),
    )
    .map_err(|_| "zero-size rectangle was rejected")?;
    assert_eq!(within.rectangle(), inside);
    assert_eq!(
        within.writable(),
        geometry(20_000).writable_region().expect("fixture writable region"),
    );
    assert!(matches_within(
        Ok(within),
        placement(revision, page, object, inside),
    ));

    let outside = rect(0, 0, 0, 0);
    let overflow = validate_fixed_placement(
        accepted,
        placement(revision, page, object, outside),
    )
    .map_err(|_| "zero-size overflow placement was rejected")?;
    assert_eq!(overflow.rectangle(), outside);
    assert_eq!(
        overflow.writable(),
        geometry(20_000).writable_region().expect("fixture writable region"),
    );
    if overflow.is_within_bounds() {
        return Err("out-of-bounds zero-size rectangle did not overflow");
    }
    let report = overflow.report();
    assert_eq!(report.violations.len(), 2);
    assert_eq!(report.violations[0].edge, BoundaryEdge::Left);
    assert_eq!(report.violations[1].edge, BoundaryEdge::Top);
    Ok(())
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
    assert!(!result.is_within_bounds(), "placement must overflow");
    let diagnostics = result.diagnostics().expect("overflow diagnostics");
    let report = result.report();
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
    assert!(!result.is_within_bounds(), "new margin must overflow left");
    let report = result.report();
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
    assert_eq!(
        validate_fixed_placement(
            session.current().expect("accepted revision"),
            placement(
                revision,
                page_one,
                page_one,
                rect(35_000, 20_000, 1, 1),
            ),
        ),
        Err(FixedRegionLayoutError::ObjectNotOnPage {
            object: page_one,
            page: page_one,
        }),
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
    let checked = placement(
        revision, page, nested, rect(35_000, 20_000, 1, 1),
    );
    assert!(matches_within(
        validate_fixed_placement(
            session.current().expect("accepted revision"), checked,
        ),
        checked,
    ));
}

#[test]
fn maximum_depth_block_ownership_is_iterative() {
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    let target = ids.allocate_candidate().expect("deep target");
    let mut block = Block {
        content: BlockContent::Rule,
        extensions: vec![],
        id: target,
        provenance: None,
        style: None,
    };
    for _ in 0..CANDIDATE_BLOCK_NESTING_LIMIT.saturating_sub(1) {
        block = Block {
            content: BlockContent::Callout(vec![block]),
            extensions: vec![],
            id: ids.allocate_candidate().expect("deep wrapper"),
            provenance: None,
            style: None,
        };
    }
    fixture.notebook.pages[0].flows[0].blocks = vec![block];
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("maximum-depth candidate must be accepted");
    };
    let page = accepted_for(&mapping, fixture.page_one);
    let object = accepted_for(&mapping, target);
    let checked = placement(
        revision, page, object, rect(35_000, 20_000, 1, 1),
    );
    assert!(matches_within(
        validate_fixed_placement(
            session.current().expect("accepted revision"), checked,
        ),
        checked,
    ));
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
    assert!(!result.is_within_bounds(), "far-right placement must overflow");
    let diagnostics = result.diagnostics().expect("overflow diagnostics");
    let report = result.report();
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

fn verify_duplicate_fixed_region_authority_refusal() -> Result<(), String> {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("valid fixed-region fixture was refused".into());
    };
    let page = accepted_for(&mapping, fixture.page_one);
    let object = accepted_for(&mapping, fixture.block);
    let profile = accepted_for(&mapping, fixture.profile_one);
    let checked = placement(revision, page, object, rect(35_000, 20_000, 1, 1));
    let accepted = session
        .current()
        .ok_or("accepted revision missing")?
        .clone();
    if !matches_within(validate_fixed_placement(&accepted, checked), checked)
    {
        return Err("valid fixed region was refused".into());
    }

    let mut repeated_page = accepted.clone();
    repeated_page
        .notebook
        .pages
        .get_mut(1)
        .ok_or("second page missing")?
        .id = page;
    if validate_fixed_placement(&repeated_page, checked)
        != Err(FixedRegionLayoutError::DuplicatePageIdentity { page })
    {
        return Err("repeated accepted page identity was not refused".into());
    }
    for conflicting_geometry in [false, true] {
        let mut repeated_profile = accepted.clone();
        let second = repeated_profile
            .notebook
            .page_profiles
            .get_mut(1)
            .ok_or("second page profile missing")?;
        second.id = profile;
        if conflicting_geometry {
            second.geometry = geometry(40_000);
        }
        if validate_fixed_placement(&repeated_profile, checked)
            != Err(FixedRegionLayoutError::DuplicatePageProfileIdentity {
                profile,
            })
        {
            return Err(format!(
                "profile alias conflicting={conflicting_geometry} accepted"
            ));
        }
    }
    if session.current() != Some(&accepted) {
        return Err("fixed-region checks mutated accepted state".into());
    }
    Ok(())
}

#[test]
fn fixed_regions_refuse_ambiguous_accepted_page_and_profile_owners() {
    assert_eq!(verify_duplicate_fixed_region_authority_refusal(), Ok(()));
}

fn verify_cross_kind_fixed_authority_refusal() -> Result<(), String> {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("valid candidate was refused".into());
    };
    let page = accepted_for(&mapping, fixture.page_one);
    let profile = accepted_for(&mapping, fixture.profile_one);
    let object = accepted_for(&mapping, fixture.block);
    let checked = placement(revision, page, object, rect(35_000, 20_000, 1, 1));
    let accepted = session
        .current()
        .ok_or("accepted revision missing")?
        .clone();
    if !matches_within(validate_fixed_placement(&accepted, checked), checked)
    {
        return Err("valid fixed placement was refused".into());
    }
    for case in 0u8..4 {
        let mut malformed = accepted.clone();
        let expected = match case {
            0 => {
                malformed.notebook.id = page;
                FixedRegionLayoutError::DuplicatePageIdentity { page }
            },
            1 => {
                malformed
                    .notebook
                    .pages
                    .get_mut(1)
                    .and_then(|owning_page| owning_page.flows.first_mut())
                    .ok_or("second flow missing")?
                    .id = page;
                FixedRegionLayoutError::DuplicatePageIdentity { page }
            },
            2 => {
                malformed.notebook.id = profile;
                FixedRegionLayoutError::DuplicatePageProfileIdentity { profile }
            },
            3 => {
                malformed
                    .notebook
                    .pages
                    .get_mut(1)
                    .and_then(|owning_page| owning_page.flows.first_mut())
                    .ok_or("second flow missing")?
                    .id = profile;
                FixedRegionLayoutError::DuplicatePageProfileIdentity { profile }
            },
            _ => return Err("unexpected fixed authority case".into()),
        };
        if validate_fixed_placement(&malformed, checked) != Err(expected) {
            return Err(format!(
                "fixed authority alias {case} was not refused"
            ));
        }
    }
    if session.current() != Some(&accepted) {
        return Err("fixed checks mutated accepted state".into());
    }
    Ok(())
}

#[test]
fn fixed_placement_rejects_cross_kind_page_and_profile_aliases() {
    assert_eq!(verify_cross_kind_fixed_authority_refusal(), Ok(()));
}

fn verify_duplicate_fixed_object_refusal() -> Result<(), String> {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("valid fixed-region fixture was refused".into());
    };
    let page = accepted_for(&mapping, fixture.page_one);
    let object = accepted_for(&mapping, fixture.block);
    let checked = placement(revision, page, object, rect(35_000, 20_000, 1, 1));
    let accepted = session
        .current()
        .ok_or("accepted revision missing")?
        .clone();
    if !matches_within(validate_fixed_placement(&accepted, checked), checked)
    {
        return Err("valid fixed block was refused".into());
    }
    for case in 0u8..3 {
        let mut malformed = accepted.clone();
        match case {
            0 => malformed.notebook.pages
                .get_mut(1)
                .and_then(|owning_page| owning_page.flows.first_mut())
                .and_then(|flow| flow.blocks.first_mut())
                .ok_or("second-page block missing")?
                .id = object,
            1 => malformed.notebook.pages
                .get_mut(0)
                .and_then(|owning_page| owning_page.flows.first_mut())
                .and_then(|flow| flow.blocks.get_mut(1))
                .ok_or("second block missing")?
                .id = object,
            2 => {
                let container = malformed.notebook.pages
                    .get_mut(0)
                    .and_then(|owning_page| owning_page.flows.first_mut())
                    .and_then(|flow| flow.blocks.get_mut(1))
                    .ok_or("callout missing")?;
                let BlockContent::Callout(children) = &mut container.content
                else {
                    return Err("callout content missing".into());
                };
                children.first_mut().ok_or("nested block missing")?.id =
                    object;
            },
            _ => return Err("unexpected duplicate case".into()),
        }
        if validate_fixed_placement(&malformed, checked)
            != Err(FixedRegionLayoutError::DuplicateObjectIdentity { object })
        {
            return Err(format!("duplicate fixed object case {case} accepted"));
        }
        if case == 0 {
            let later_page = accepted_for(&mapping, fixture.page_two);
            let later = placement(
                revision,
                later_page,
                object,
                rect(35_000, 20_000, 1, 1),
            );
            if validate_fixed_placement(&malformed, later)
                != Err(FixedRegionLayoutError::DuplicateObjectIdentity {
                    object,
                })
            {
                return Err("second-page duplicate was not refused".into());
            }
        }
        if session.current() != Some(&accepted) {
            return Err("fixed check changed accepted state".into());
        }
    }
    Ok(())
}

#[test]
fn fixed_region_refuses_ambiguous_nested_or_cross_page_block_owner() {
    assert_eq!(verify_duplicate_fixed_object_refusal(), Ok(()));
}

fn verify_fixed_object_structural_alias_refusal() -> Result<(), String> {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("valid fixed-region fixture was refused".into());
    };
    let page = accepted_for(&mapping, fixture.page_one);
    let object = accepted_for(&mapping, fixture.block);
    let checked = placement(revision, page, object, rect(35_000, 20_000, 1, 1));
    let accepted = session.current().ok_or("revision missing")?.clone();
    for case in 0u8..4 {
        let mut malformed = accepted.clone();
        match case {
            0 => malformed
                .notebook
                .pages
                .first_mut()
                .ok_or("first page missing")?
                .flows
                .push(Flow {
                    blocks: vec![],
                    id: object,
                }),
            1 => {
                let container = malformed
                    .notebook
                    .pages
                    .first_mut()
                    .and_then(|owning_page| owning_page.flows.first_mut())
                    .and_then(|flow| flow.blocks.get_mut(1))
                    .ok_or("callout missing")?;
                let BlockContent::Callout(children) = &mut container.content
                else {
                    return Err("callout missing".into());
                };
                children.first_mut().ok_or("nested block missing")?.content =
                    BlockContent::Paragraph(vec![
                        atrament_semantic_notebook::InlineSpan {
                            id: object,
                            provenance: None,
                            style: None,
                            text: "aliased caption".into(),
                        },
                    ]);
            },
            2 => malformed.notebook.id = object,
            3 => {
                malformed
                    .notebook
                    .page_profiles
                    .get_mut(1)
                    .ok_or("second profile missing")?
                    .id = object
            },
            _ => return Err("unexpected alias case".into()),
        }
        if validate_fixed_placement(&malformed, checked)
            != Err(FixedRegionLayoutError::DuplicateObjectIdentity { object })
        {
            return Err(format!("structural alias {case} not refused"));
        }
    }
    if session.current() != Some(&accepted) {
        return Err("fixed-region validation mutated accepted revision".into());
    }
    Ok(())
}

#[test]
fn fixed_region_rejects_selected_block_alias_of_other_semantic_owner() {
    assert_eq!(verify_fixed_object_structural_alias_refusal(), Ok(()));
}
