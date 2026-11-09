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
//   - Regression evidence for accepted-revision measured-flow pagination.
// - Must-Not:
//   - Invent measurements, mutate accepted state, or supply arbitrary page
//     rectangles in place of accepted page-profile authority.
// - Allows:
//   - Inputs: Accepted semantic revisions and explicit measured fragment sizes.
//   - Outputs: Assertions over revision binding and derived page placement.
//   - Side effects: Process-local candidate acceptance and test allocation.
// - Split-When:
//   - A real handwriting measurement authority receives separate fixtures.
// - Merge-When:
//   - Semantic pagination becomes part of a complete measured-layout harness.
// - Summary:
//   - Verifies pagination derives physical regions from accepted page profiles.
// - Description:
//   - Covers stale measurement, profile edits, spills, and defensive failures.
// - Usage:
//   - Accept candidates, bind measurements to revisions, and paginate directly.
// - Defaults:
//   - The fixture uses A4-like portrait physical geometry in micrometres.
//

//! Semantic pagination regression tests.

use std::collections::BTreeMap;

use atrament_flow_pagination::{
    FlowUnitPolicy, MeasuredFlowUnit, MeasuredFragment, PaginationError,
};
use atrament_physical_page_profile::{
    BindingEdge, BorderShape, Length, Orientation,
    PageProfile as PhysicalPageProfile, PageProfileError, PaperMarkAppearance,
    PaperMarkJoin, PaperMarkLayer, PaperPattern, Rect, SheetSize,
};
use atrament_semantic_fixed_region_layout::{
    AcceptedFixedPlacement, validate_fixed_placement,
};
use atrament_semantic_flow_pagination::{
    FixedPairOverlap, FixedRowBandOverlap, FlowFixedRegionError,
    RevisionFlowMeasurement,
    SemanticPaginationError, check_fixed_regions_for_flow,
    first_fixed_pair_overlap_for_flow, fixed_pair_overlaps_for_flow,
    fixed_row_band_overlaps_for_flow, paginate_revision,
};
use atrament_semantic_notebook::{
    AcceptedIdentity, Asset, Block, BlockContent, CandidateIdentity,
    Constraint, ConstraintKind, Figure, Flow, Formula, FormulaMode,
    IdentityAllocator, InlineSpan, List, ListItem, Notebook, OutputProfile,
    Page, PaperProfile, Provenance, ProvenanceKind, Style, Table, TableCell,
    TableCellSpan, TableRow, TableRowRole, UnresolvedBlock, UnresolvedReason,
};
use atrament_semantic_notebook_port::{
    AcceptanceOutcome, PageProfileEditOutcome, SemanticNotebookSession,
};
use atrament_semantic_notebook_session::SemanticNotebookSessionService;

struct CandidateFixture {
    block: CandidateIdentity,
    flow: CandidateIdentity,
    notebook: Notebook<CandidateIdentity>,
    page_one: CandidateIdentity,
    page_two: CandidateIdentity,
    profile_one: CandidateIdentity,
    profile_two: CandidateIdentity,
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
            maximum_ruler_error: Length::from_micrometres(200),
        },
        paper_mark_layer: PaperMarkLayer::BelowInk,
        paper_pattern: PaperPattern::Squared {
            spacing: Length::from_micrometres(5_000),
        },
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
    let profile_one = ids.allocate_candidate().expect("first profile");
    let profile_two = ids.allocate_candidate().expect("second profile");
    let page_one = ids.allocate_candidate().expect("first page");
    let page_two = ids.allocate_candidate().expect("second page");
    let flow_id = ids.allocate_candidate().expect("flow");
    let block = ids.allocate_candidate().expect("block");
    CandidateFixture {
        block,
        flow: flow_id,
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
                        blocks: vec![Block {
                            content: BlockContent::Rule,
                            extensions: vec![],
                            id: block,
                            provenance: None,
                            style: None,
                        }],
                        id: flow_id,
                    }],
                    id: page_one,
                    paper_profile: profile_one,
                },
                Page {
                    flows: vec![],
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
        profile_two,
    }
}

fn measurement(
    revision: atrament_semantic_notebook::RevisionIdentity,
    flow: AcceptedIdentity,
    owner: AcceptedIdentity,
    fragments: &[(u64, u64)],
) -> RevisionFlowMeasurement {
    RevisionFlowMeasurement {
        flow,
        revision,
        units: vec![MeasuredFlowUnit {
            fragments: fragments
                .iter()
                .map(|(width, height)| MeasuredFragment {
                    height: Length::from_micrometres(*height),
                    owner,
                    width: Length::from_micrometres(*width),
                })
                .collect(),
            policy: FlowUnitPolicy::Independent,
        }],
    }
}

#[test]
fn empty_accepted_flow_needs_no_page_profile_authority() {
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    fixture.notebook.pages[0].flows[0].blocks.clear();
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("empty-flow candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let measured = RevisionFlowMeasurement {
        flow,
        revision,
        units: vec![MeasuredFlowUnit {
            fragments: Vec::new(),
            policy: FlowUnitPolicy::KeepTogetherWhenPossible,
        }],
    };
    let mut defensive = session.current().expect("accepted revision").clone();
    defensive.page_profiles_forget_for_test();

    let plan = paginate_revision(&defensive, &measured)
        .expect("empty flow must not require page geometry");
    assert_eq!(plan.revision(), revision);
    assert_eq!(plan.flow(), flow);
    assert!(plan.placements().is_empty());
}

#[test]
fn measured_inline_block_families_preserve_semantic_order_and_identity() {
    type InlineBlockConstructor = fn(
        Vec<InlineSpan<CandidateIdentity>>,
    ) -> BlockContent<CandidateIdentity>;
    let cases: &[InlineBlockConstructor] = &[
        BlockContent::Citation,
        BlockContent::Date,
        BlockContent::Definition,
        BlockContent::Footnote,
        BlockContent::Heading,
        BlockContent::Label,
        BlockContent::MarginNote,
        BlockContent::Paragraph,
        BlockContent::Quotation,
        BlockContent::SourceNote,
    ];
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    let mut candidate_blocks = Vec::with_capacity(cases.len());
    let mut blocks = Vec::with_capacity(cases.len());
    for (index, constructor) in cases.iter().enumerate() {
        let block = if index == 0 {
            fixture.block
        } else {
            ids.allocate_candidate().expect("inline-family block")
        };
        let span = ids.allocate_candidate().expect("inline-family span");
        candidate_blocks.push(block);
        blocks.push(Block {
            content: constructor(vec![InlineSpan {
                id: span,
                provenance: None,
                style: None,
                text: format!("measured inline family {index}"),
            }]),
            extensions: vec![],
            id: block,
            provenance: None,
            style: None,
        });
    }
    fixture.notebook.pages[0].flows[0].blocks = blocks;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("inline-family candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let page = accepted_for(&mapping, fixture.page_one);
    let expected = candidate_blocks
        .iter()
        .map(|block| accepted_for(&mapping, *block))
        .collect::<Vec<_>>();
    let measured = RevisionFlowMeasurement {
        flow,
        revision,
        units: vec![MeasuredFlowUnit {
            fragments: expected
                .iter()
                .map(|owner| MeasuredFragment {
                    height: Length::from_micrometres(1_000),
                    owner: *owner,
                    width: Length::from_micrometres(80_000),
                })
                .collect(),
            policy: FlowUnitPolicy::Independent,
        }],
    };

    let plan = paginate_revision(
        session.current().expect("accepted inline-family revision"),
        &measured,
    )
    .expect("already-measured inline families must paginate");
    assert_eq!(plan.revision(), revision);
    assert_eq!(plan.flow(), flow);
    assert_eq!(
        plan.placements()
            .iter()
            .map(|placement| placement.owner)
            .collect::<Vec<_>>(),
        expected,
    );
    assert!(plan
        .placements()
        .iter()
        .all(|placement| placement.page == page));
}

#[test]
fn measured_list_block_keeps_containing_block_as_pagination_owner() {
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    let list = ids.allocate_candidate().expect("list");
    let item = ids.allocate_candidate().expect("list item");
    let child = ids.allocate_candidate().expect("list child block");
    let span = ids.allocate_candidate().expect("list child span");
    fixture.notebook.pages[0].flows[0].blocks[0].content =
        BlockContent::List(List {
            id: list,
            items: vec![ListItem {
                blocks: vec![Block {
                    content: BlockContent::Paragraph(vec![InlineSpan {
                        id: span,
                        provenance: None,
                        style: None,
                        text: String::from("already measured list item"),
                    }]),
                    extensions: vec![],
                    id: child,
                    provenance: None,
                    style: None,
                }],
                id: item,
            }],
            ordered: true,
        });
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("list candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let page = accepted_for(&mapping, fixture.page_one);
    let owner = accepted_for(&mapping, fixture.block);
    let accepted_list = accepted_for(&mapping, list);
    let accepted_item = accepted_for(&mapping, item);
    let accepted_child = accepted_for(&mapping, child);
    let accepted = session.current().expect("accepted list revision");
    let BlockContent::List(current_list) =
        &accepted.notebook.pages[0].flows[0].blocks[0].content
    else {
        panic!("accepted top-level block must remain a list");
    };
    assert_eq!(current_list.id, accepted_list);
    assert_eq!(current_list.items[0].id, accepted_item);
    assert_eq!(current_list.items[0].blocks[0].id, accepted_child);

    let measured = measurement(
        revision,
        flow,
        owner,
        &[(80_000, 6_000), (90_000, 7_000)],
    );
    let plan = paginate_revision(accepted, &measured)
        .expect("already-measured list block must paginate");
    assert_eq!(plan.placements().len(), 2);
    assert!(plan
        .placements()
        .iter()
        .all(|placement| placement.owner == owner && placement.page == page));
}

#[test]
fn measured_structured_block_families_preserve_top_level_owner_order() {
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    let callout_child = ids.allocate_candidate().expect("callout child");
    let figure_block = ids.allocate_candidate().expect("figure block");
    let figure = ids.allocate_candidate().expect("figure");
    let freeform_block = ids.allocate_candidate().expect("freeform block");
    let freeform_child = ids.allocate_candidate().expect("freeform child");
    let math_block = ids.allocate_candidate().expect("mathematics block");
    let formula = ids.allocate_candidate().expect("formula");
    let rule_block = ids.allocate_candidate().expect("rule block");
    let table_block = ids.allocate_candidate().expect("table block");
    let table = ids.allocate_candidate().expect("table");
    let row = ids.allocate_candidate().expect("table row");
    let cell = ids.allocate_candidate().expect("table cell");
    let unresolved = ids.allocate_candidate().expect("unresolved block");

    fixture.notebook.pages[0].flows[0].blocks = vec![
        Block {
            content: BlockContent::Callout(vec![Block {
                content: BlockContent::Rule,
                extensions: vec![],
                id: callout_child,
                provenance: None,
                style: None,
            }]),
            extensions: vec![],
            id: fixture.block,
            provenance: None,
            style: None,
        },
        Block {
            content: BlockContent::Figure(Figure {
                asset: None,
                caption: vec![],
                id: figure,
            }),
            extensions: vec![],
            id: figure_block,
            provenance: None,
            style: None,
        },
        Block {
            content: BlockContent::Freeform(vec![Block {
                content: BlockContent::Rule,
                extensions: vec![],
                id: freeform_child,
                provenance: None,
                style: None,
            }]),
            extensions: vec![],
            id: freeform_block,
            provenance: None,
            style: None,
        },
        Block {
            content: BlockContent::Mathematics(Formula {
                id: formula,
                mode: FormulaMode::Display,
                source: String::from("x"),
            }),
            extensions: vec![],
            id: math_block,
            provenance: None,
            style: None,
        },
        Block {
            content: BlockContent::Rule,
            extensions: vec![],
            id: rule_block,
            provenance: None,
            style: None,
        },
        Block {
            content: BlockContent::Table(Table {
                id: table,
                rows: vec![TableRow {
                    cells: vec![TableCell {
                        blocks: vec![],
                        id: cell,
                        span: TableCellSpan::SINGLE,
                    }],
                    id: row,
                    role: TableRowRole::Body,
                }],
            }),
            extensions: vec![],
            id: table_block,
            provenance: None,
            style: None,
        },
        Block {
            content: BlockContent::Unresolved(UnresolvedBlock {
                extensions: vec![],
                reason: UnresolvedReason::Unsupported,
                source: String::from("unsupported measured source"),
            }),
            extensions: vec![],
            id: unresolved,
            provenance: None,
            style: None,
        },
    ];
    let candidates = [
        fixture.block,
        figure_block,
        freeform_block,
        math_block,
        rule_block,
        table_block,
        unresolved,
    ];
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("structured-family candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let page = accepted_for(&mapping, fixture.page_one);
    let expected = candidates
        .iter()
        .map(|candidate| accepted_for(&mapping, *candidate))
        .collect::<Vec<_>>();
    let before = session
        .current()
        .expect("accepted structured-family revision")
        .clone();
    let measured = RevisionFlowMeasurement {
        flow,
        revision,
        units: vec![MeasuredFlowUnit {
            fragments: expected
                .iter()
                .map(|owner| MeasuredFragment {
                    height: Length::from_micrometres(1_000),
                    owner: *owner,
                    width: Length::from_micrometres(80_000),
                })
                .collect(),
            policy: FlowUnitPolicy::Independent,
        }],
    };

    let plan = paginate_revision(
        session.current().expect("accepted revision remains current"),
        &measured,
    )
    .expect("already-measured structured families must paginate");
    assert_eq!(
        plan.placements()
            .iter()
            .map(|placement| placement.owner)
            .collect::<Vec<_>>(),
        expected,
    );
    assert!(plan
        .placements()
        .iter()
        .all(|placement| placement.page == page));
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn accepted_page_profile_derives_exact_writable_top_and_page_identity() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let page_one = accepted_for(&mapping, fixture.page_one);
    let measured = measurement(revision, flow, owner, &[(165_000, 1_000)]);
    let accepted = session.current().expect("accepted revision");

    let plan =
        paginate_revision(accepted, &measured).expect("semantic pagination");
    assert_eq!(plan.placements().len(), 1);
    assert_eq!(plan.placements()[0].owner, owner);
    assert_eq!(plan.placements()[0].page, page_one);
    assert_eq!(plan.placements()[0].top, Length::from_micrometres(20_000));
    assert_eq!(
        plan.placements()[0].writable.x,
        Length::from_micrometres(35_000)
    );
    assert_eq!(plan.placements()[0].width, Length::from_micrometres(165_000));
}

#[test]
fn defensive_duplicate_page_profile_identity_refuses_ambiguous_geometry() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let outcome = session.accept(fixture.notebook);
    assert!(matches!(outcome, AcceptanceOutcome::Accepted { .. }));
    let AcceptanceOutcome::Accepted { mapping, revision } = outcome else {
        return;
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let profile = accepted_for(&mapping, fixture.profile_one);
    let measured = measurement(revision, flow, owner, &[(165_000, 1_000)]);
    assert!(session.current().is_some());
    let Some(accepted) = session.current().cloned() else {
        return;
    };
    assert!(paginate_revision(&accepted, &measured).is_ok());

    // A directly malformed revision must not pick the first geometry for a
    // repeated profile ID, even if the two geometries happen to be equal.
    for duplicate_geometry in [geometry(20_000), geometry(40_000)] {
        let mut malformed = accepted.clone();
        malformed.notebook.page_profiles.push(PaperProfile {
            geometry: duplicate_geometry,
            id: profile,
        });
        assert_eq!(
            paginate_revision(&malformed, &measured),
            Err(SemanticPaginationError::DuplicatePageProfileIdentity {
                profile,
            }),
        );
        assert_eq!(session.current(), Some(&accepted));
    }
}

#[test]
fn many_pages_can_share_one_cached_page_profile() {
    const EXTRA_PAGES: usize = 10_000;

    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    for _ in 0..EXTRA_PAGES {
        let page = ids.allocate_candidate().expect("shared-profile page");
        fixture.notebook.pages.push(Page {
            flows: Vec::new(),
            id: page,
            paper_profile: fixture.profile_one,
        });
    }
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("shared-profile candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let page_one = accepted_for(&mapping, fixture.page_one);
    let measured = measurement(revision, flow, owner, &[(1, 1)]);

    let plan = paginate_revision(
        session.current().expect("accepted revision"),
        &measured,
    )
    .expect("shared page profile must preserve pagination");
    assert_eq!(plan.placements().len(), 1);
    assert_eq!(plan.placements()[0].page, page_one);
}

#[test]
fn large_page_profile_index_preserves_pagination() {
    const EXTRA_PAGES: usize = 10_000;

    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    for _ in 0..EXTRA_PAGES {
        let profile = ids.allocate_candidate().expect("large profile");
        let page = ids.allocate_candidate().expect("large page");
        fixture.notebook.page_profiles.push(PaperProfile {
            geometry: geometry(20_000),
            id: profile,
        });
        fixture.notebook.pages.push(Page {
            flows: Vec::new(),
            id: page,
            paper_profile: profile,
        });
    }
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("large-page candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let page_one = accepted_for(&mapping, fixture.page_one);
    let measured = measurement(revision, flow, owner, &[(1, 1)]);

    let plan = paginate_revision(
        session.current().expect("accepted revision"),
        &measured,
    )
    .expect("large profile index must preserve pagination");
    assert_eq!(plan.placements().len(), 1);
    assert_eq!(plan.placements()[0].page, page_one);
}

#[test]
fn measured_flow_spills_to_next_accepted_page_in_notebook_order() {
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    fixture.notebook.page_profiles[1].geometry = geometry(40_000);
    fixture.notebook.page_profiles[1].geometry.top_clearance =
        Length::from_micrometres(30_000);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let first_page = accepted_for(&mapping, fixture.page_one);
    let second_page = accepted_for(&mapping, fixture.page_two);
    let measured = measurement(revision, flow, owner, &[
        (100_000, 260_000),
        (100_000, 20_000),
    ]);

    let plan = paginate_revision(
        session.current().expect("accepted revision"),
        &measured,
    )
    .expect("two-page flow");
    assert_eq!(plan.placements()[0].page, first_page);
    assert_eq!(plan.placements()[1].page, second_page);
    assert_eq!(plan.placements()[1].top, Length::from_micrometres(40_000));
    assert_eq!(
        plan.placements().first().map(|placed| placed.writable),
        Some(Rect {
            height: Length::from_micrometres(267_000),
            width: Length::from_micrometres(165_000),
            x: Length::from_micrometres(35_000),
            y: Length::from_micrometres(20_000),
        }),
    );
    assert_eq!(
        plan.placements().get(1).map(|placed| placed.writable),
        Some(Rect {
            height: Length::from_micrometres(247_000),
            width: Length::from_micrometres(145_000),
            x: Length::from_micrometres(55_000),
            y: Length::from_micrometres(40_000),
        }),
    );
}

#[test]
fn stale_measurement_is_rejected_before_new_revision_geometry_is_used() {
    let ids = IdentityAllocator::new();
    let first = candidate_fixture(&ids);
    let second = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted {
        mapping,
        revision: first_revision,
    } = session.accept(first.notebook)
    else {
        panic!("first candidate must be accepted");
    };
    let old_flow = accepted_for(&mapping, first.flow);
    let old_owner = accepted_for(&mapping, first.block);
    let AcceptanceOutcome::Accepted { revision: current, .. } =
        session.accept(second.notebook)
    else {
        panic!("second candidate must be accepted");
    };
    let measured = measurement(first_revision, old_flow, old_owner, &[(1, 1)]);

    assert_eq!(
        paginate_revision(
            session.current().expect("current revision"),
            &measured
        ),
        Err(SemanticPaginationError::MeasurementRevisionMismatch {
            accepted: current,
            measured: first_revision,
        }),
    );
}

#[test]
fn accepted_page_profile_edit_changes_fit_only_with_fresh_measurement() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision: base } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let profile_one = accepted_for(&mapping, fixture.profile_one);
    let profile_two = accepted_for(&mapping, fixture.profile_two);
    let second_page = accepted_for(&mapping, fixture.page_two);
    let before = measurement(base, flow, owner, &[(160_000, 1_000)]);
    assert!(
        paginate_revision(session.current().expect("base revision"), &before,)
            .is_ok()
    );

    let PageProfileEditOutcome::Applied { revision, .. } =
        session.replace_page_profile(base, profile_one, geometry(40_000))
    else {
        panic!("profile edit must apply");
    };
    assert_eq!(
        paginate_revision(session.current().expect("edited revision"), &before),
        Err(SemanticPaginationError::MeasurementRevisionMismatch {
            accepted: revision,
            measured: base,
        }),
    );

    let fresh = measurement(revision, flow, owner, &[(160_000, 1_000)]);
    let shifted =
        paginate_revision(session.current().expect("edited revision"), &fresh)
            .expect("later accepted page still fits");
    assert_eq!(shifted.placements()[0].page, second_page);

    let PageProfileEditOutcome::Applied {
        revision: narrowed_revision,
        ..
    } = session.replace_page_profile(revision, profile_two, geometry(40_000))
    else {
        panic!("second profile edit must apply");
    };
    let narrowed =
        measurement(narrowed_revision, flow, owner, &[(160_000, 1_000)]);
    assert_eq!(
        paginate_revision(
            session.current().expect("both profiles narrowed"),
            &narrowed,
        ),
        Err(SemanticPaginationError::Pagination(
            PaginationError::FragmentDoesNotFitAnyPage { owner },
        )),
    );
}

#[test]
fn zero_height_measurement_refuses_without_mutating_accepted_revision() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let outcome = session.accept(fixture.notebook);
    assert!(matches!(outcome, AcceptanceOutcome::Accepted { .. }));
    let AcceptanceOutcome::Accepted { mapping, revision } = outcome else {
        return;
    };
    assert!(session.current().is_some());
    let Some(original) = session.current().cloned() else {
        return;
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);

    for width in [0, 100_000] {
        let invalid = measurement(revision, flow, owner, &[(width, 0)]);
        assert_eq!(
            paginate_revision(&original, &invalid),
            Err(SemanticPaginationError::Pagination(
                PaginationError::ZeroHeightFragment { owner },
            )),
        );
    }
    let allowed_gap = measurement(revision, flow, owner, &[(0, 1_000)]);
    assert_eq!(
        paginate_revision(&original, &allowed_gap)
            .map(|plan| plan.placements().len()),
        Ok(1),
    );
    assert_eq!(session.current(), Some(&original));
}

#[test]
fn pagination_is_read_only_for_accepted_revision() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let measured = measurement(revision, flow, owner, &[(1, 1)]);
    let before = session.current().expect("accepted revision").clone();

    let _plan = paginate_revision(&before, &measured).expect("read-only plan");
    assert_eq!(session.current(), Some(&before));
}

#[test]
fn defensive_duplicate_flow_identity_cannot_select_an_arbitrary_owner() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let outcome = session.accept(fixture.notebook);
    assert!(matches!(outcome, AcceptanceOutcome::Accepted { .. }));
    let AcceptanceOutcome::Accepted { mapping, revision } = outcome else {
        return;
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let measured = measurement(revision, flow, owner, &[(1, 1)]);
    assert!(session.current().is_some());
    let Some(accepted) = session.current().cloned() else {
        return;
    };
    assert!(paginate_revision(&accepted, &measured).is_ok());

    // Normal candidate admission prevents global identity collisions. A
    // malformed typed accepted revision must not pick the first matching flow.
    for page_index in [0, 1] {
        let mut malformed = accepted.clone();
        assert!(malformed.notebook.pages.len() > page_index);
        let Some(page) = malformed.notebook.pages.get_mut(page_index) else {
            return;
        };
        page.flows.push(Flow { blocks: vec![], id: flow });
        assert_eq!(
            paginate_revision(&malformed, &measured),
            Err(SemanticPaginationError::DuplicateFlowIdentity { flow }),
        );
        assert_eq!(session.current(), Some(&accepted));
    }
}

fn verify_cross_kind_flow_identity_refusal() -> Result<(), String> {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("valid candidate was refused".into());
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let measured = measurement(revision, flow, owner, &[(1, 1)]);
    let accepted = session
        .current()
        .ok_or("accepted revision missing")?
        .clone();
    if paginate_revision(&accepted, &measured).is_err() {
        return Err("valid flow was refused".into());
    }
    for case in 0u8..4 {
        let mut malformed = accepted.clone();
        match case {
            0 => malformed.notebook.id = flow,
            1 => {
                malformed
                    .notebook
                    .pages
                    .get_mut(1)
                    .ok_or("second page missing")?
                    .id = flow;
            },
            2 => {
                malformed
                    .notebook
                    .page_profiles
                    .get_mut(1)
                    .ok_or("second profile missing")?
                    .id = flow;
            },
            3 => {
                malformed
                    .notebook
                    .pages
                    .first_mut()
                    .and_then(|page| page.flows.first_mut())
                    .and_then(|selected| selected.blocks.first_mut())
                    .ok_or("first block missing")?
                    .id = flow;
            },
            _ => return Err("unexpected flow case".into()),
        }
        if paginate_revision(&malformed, &measured)
            != Err(SemanticPaginationError::DuplicateFlowIdentity { flow })
        {
            return Err(format!("flow owner alias {case} was not refused"));
        }
    }
    if session.current() != Some(&accepted) {
        return Err("flow checks mutated accepted state".into());
    }
    Ok(())
}

#[test]
fn selected_flow_cannot_alias_another_kind_of_semantic_owner() {
    assert_eq!(verify_cross_kind_flow_identity_refusal(), Ok(()));
}

fn verify_cross_kind_page_authority_refusal() -> Result<(), String> {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("valid candidate was refused".into());
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let page = accepted_for(&mapping, fixture.page_one);
    let profile = accepted_for(&mapping, fixture.profile_one);
    let measured = measurement(revision, flow, owner, &[(1, 1)]);
    let accepted = session
        .current()
        .ok_or("accepted revision missing")?
        .clone();
    if paginate_revision(&accepted, &measured).is_err() {
        return Err("valid page authority was refused".into());
    }
    for case in 0u8..4 {
        let mut malformed = accepted.clone();
        let expected = match case {
            0 => {
                malformed.notebook.id = page;
                SemanticPaginationError::Pagination(
                    PaginationError::DuplicatePageIdentity { page },
                )
            },
            1 => {
                malformed.notebook.assets.push(Asset {
                    id: page,
                    media_type: "image/png".into(),
                });
                SemanticPaginationError::Pagination(
                    PaginationError::DuplicatePageIdentity { page },
                )
            },
            2 => {
                malformed.notebook.id = profile;
                SemanticPaginationError::DuplicatePageProfileIdentity {
                    profile,
                }
            },
            3 => {
                malformed
                    .notebook
                    .pages
                    .get_mut(1)
                    .ok_or("second page missing")?
                    .flows
                    .push(Flow {
                        blocks: vec![],
                        id: profile,
                    });
                SemanticPaginationError::DuplicatePageProfileIdentity {
                    profile,
                }
            },
            _ => return Err("unexpected page case".into()),
        };
        if paginate_revision(&malformed, &measured) != Err(expected) {
            return Err(format!("page authority alias {case} was not refused"));
        }
    }
    if session.current() != Some(&accepted) {
        return Err("page checks mutated accepted state".into());
    }
    Ok(())
}

#[test]
fn selected_page_and_profile_cannot_alias_other_semantic_owners() {
    assert_eq!(verify_cross_kind_page_authority_refusal(), Ok(()));
}

#[test]
fn defensive_duplicate_block_identity_cannot_alias_measured_owner_runs() {
    let ids = IdentityAllocator::new();
    let (mut fixture, second_block) = candidate_two_block_fixture(&ids);
    let third_candidate = ids.allocate_candidate();
    assert!(third_candidate.is_ok());
    let Ok(third_block) = third_candidate else {
        return;
    };
    let maybe_first_flow = fixture
        .notebook
        .pages
        .get_mut(0)
        .and_then(|page| page.flows.get_mut(0));
    assert!(maybe_first_flow.is_some());
    let Some(first_flow) = maybe_first_flow else {
        return;
    };
    first_flow.blocks.push(Block {
        content: BlockContent::Rule,
        extensions: vec![],
        id: third_block,
        provenance: None,
        style: None,
    });
    let mut session = SemanticNotebookSessionService::default();
    let outcome = session.accept(fixture.notebook);
    assert!(matches!(outcome, AcceptanceOutcome::Accepted { .. }));
    let AcceptanceOutcome::Accepted { mapping, revision } = outcome else {
        return;
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let first = accepted_for(&mapping, fixture.block);
    let second = accepted_for(&mapping, second_block);
    let third = accepted_for(&mapping, third_block);
    assert!(session.current().is_some());
    let Some(accepted) = session.current().cloned() else {
        return;
    };

    for (alias_index, owners) in
        [(1, [first, first, third]), (2, [first, second, first])]
    {
        let mut malformed = accepted.clone();
        let maybe_block = malformed
            .notebook
            .pages
            .get_mut(0)
            .and_then(|page| page.flows.get_mut(0))
            .and_then(|selected_flow| {
                selected_flow.blocks.get_mut(alias_index)
            });
        assert!(maybe_block.is_some());
        let Some(block) = maybe_block else {
            return;
        };
        block.id = first;
        let measured = RevisionFlowMeasurement {
            flow,
            revision,
            units: vec![MeasuredFlowUnit {
                fragments: owners
                    .into_iter()
                    .map(|owner| MeasuredFragment {
                        height: Length::from_micrometres(1),
                        owner,
                        width: Length::from_micrometres(1),
                    })
                    .collect(),
                policy: FlowUnitPolicy::Independent,
            }],
        };
        assert_eq!(
            paginate_revision(&malformed, &measured),
            Err(SemanticPaginationError::DuplicateBlockIdentity {
                flow,
                block: first,
            }),
        );
        assert_eq!(session.current(), Some(&accepted));
    }
}

#[test]
fn other_flow_cannot_reuse_selected_top_level_block_identity() {
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    let candidate_other_flow = ids.allocate_candidate();
    let candidate_other_block = ids.allocate_candidate();
    assert!(candidate_other_flow.is_ok() && candidate_other_block.is_ok());
    let (Ok(other_flow), Ok(other_block)) =
        (candidate_other_flow, candidate_other_block)
    else {
        return;
    };
    let Some(page_two) = fixture.notebook.pages.get_mut(1) else {
        return;
    };
    page_two.flows.push(Flow {
        blocks: vec![Block {
            content: BlockContent::Rule,
            extensions: vec![],
            id: other_block,
            provenance: None,
            style: None,
        }],
        id: other_flow,
    });
    let mut session = SemanticNotebookSessionService::default();
    let outcome = session.accept(fixture.notebook);
    assert!(matches!(outcome, AcceptanceOutcome::Accepted { .. }));
    let AcceptanceOutcome::Accepted { mapping, revision } = outcome else {
        return;
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let measured = measurement(revision, flow, owner, &[(1, 1)]);
    assert!(session.current().is_some());
    let Some(accepted) = session.current().cloned() else {
        return;
    };
    assert!(paginate_revision(&accepted, &measured).is_ok());

    // The selected flow's measured owner must not also name an unmeasured
    // block owned by a different accepted flow.
    let mut malformed = accepted.clone();
    let maybe_other = malformed
        .notebook
        .pages
        .get_mut(1)
        .and_then(|page| page.flows.first_mut())
        .and_then(|sibling_flow| sibling_flow.blocks.first_mut());
    assert!(maybe_other.is_some());
    let Some(other) = maybe_other else {
        return;
    };
    other.id = owner;
    assert_eq!(
        paginate_revision(&malformed, &measured),
        Err(SemanticPaginationError::DuplicateBlockIdentity {
            flow,
            block: owner,
        }),
    );
    assert_eq!(session.current(), Some(&accepted));
}

fn candidate_with_nested_sibling(
    ids: &IdentityAllocator,
) -> Result<CandidateFixture, String> {
    let mut fixture = candidate_fixture(ids);
    let new_id = || {
        ids.allocate_candidate()
            .map_err(|_error| String::from("nested identity exhausted"))
    };
    let child = |id| Block {
        content: BlockContent::Rule,
        extensions: vec![],
        id,
        provenance: None,
        style: None,
    };
    let containers = vec![
        BlockContent::Callout(vec![child(new_id()?)]),
        BlockContent::Freeform(vec![child(new_id()?)]),
        BlockContent::List(List {
            id: new_id()?,
            items: vec![ListItem {
                blocks: vec![child(new_id()?)],
                id: new_id()?,
            }],
            ordered: false,
        }),
        BlockContent::Table(Table {
            id: new_id()?,
            rows: vec![TableRow {
                cells: vec![TableCell {
                    blocks: vec![child(new_id()?)],
                    id: new_id()?,
                    span: TableCellSpan::SINGLE,
                }],
                id: new_id()?,
                role: TableRowRole::Body,
            }],
        }),
    ];
    let blocks = containers
        .into_iter()
        .map(|content| {
            Ok(Block {
                content,
                extensions: vec![],
                id: new_id()?,
                provenance: None,
                style: None,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    fixture
        .notebook
        .pages
        .get_mut(1)
        .ok_or("missing sibling page")?
        .flows
        .push(Flow { blocks, id: new_id()? });
    Ok(fixture)
}

fn nested_child(
    content: &mut BlockContent<AcceptedIdentity>,
) -> Option<&mut Block<AcceptedIdentity>> {
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            blocks.first_mut()
        },
        BlockContent::List(list) => list
            .items
            .first_mut()
            .and_then(|item| item.blocks.first_mut()),
        BlockContent::Table(table) => table
            .rows
            .first_mut()
            .and_then(|row| row.cells.first_mut())
            .and_then(|cell| cell.blocks.first_mut()),
        BlockContent::Citation(_)
        | BlockContent::Date(_)
        | BlockContent::Definition(_)
        | BlockContent::Figure(_)
        | BlockContent::Footnote(_)
        | BlockContent::Heading(_)
        | BlockContent::Label(_)
        | BlockContent::MarginNote(_)
        | BlockContent::Mathematics(_)
        | BlockContent::Paragraph(_)
        | BlockContent::Quotation(_)
        | BlockContent::Rule
        | BlockContent::SourceNote(_)
        | BlockContent::Unresolved(_) => None,
    }
}

fn verify_nested_block_identity_refusal() -> Result<(), String> {
    let ids = IdentityAllocator::new();
    let fixture = candidate_with_nested_sibling(&ids)?;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("distinct nested identities must be accepted".into());
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let measured = measurement(revision, flow, owner, &[(1, 1)]);
    let accepted = session
        .current()
        .ok_or("missing accepted revision")?
        .clone();
    if paginate_revision(&accepted, &measured).is_err() {
        return Err("valid nested blocks unexpectedly refused".into());
    }

    for container_index in 0..4 {
        let mut malformed = accepted.clone();
        let container = malformed
            .notebook
            .pages
            .get_mut(1)
            .and_then(|page| page.flows.first_mut())
            .and_then(|sibling| sibling.blocks.get_mut(container_index))
            .ok_or("nested container missing")?;
        let nested = nested_child(&mut container.content)
            .ok_or("nested child block missing")?;
        nested.id = owner;
        if paginate_revision(&malformed, &measured)
            != Err(SemanticPaginationError::DuplicateBlockIdentity {
                flow,
                block: owner,
            })
        {
            return Err(format!(
                "nested block family {container_index} aliased measured owner"
            ));
        }
        if session.current() != Some(&accepted) {
            return Err("nested refusal changed accepted session".into());
        }
    }

    // A selected top-level owner must also be distinct from its own child.
    let mut malformed = accepted.clone();
    let selected = malformed
        .notebook
        .pages
        .first_mut()
        .and_then(|page| page.flows.first_mut())
        .and_then(|selected_flow| selected_flow.blocks.first_mut())
        .ok_or("selected block missing")?;
    selected.content = BlockContent::Callout(vec![Block {
        content: BlockContent::Rule,
        extensions: vec![],
        id: owner,
        provenance: None,
        style: None,
    }]);
    if paginate_revision(&malformed, &measured)
        != Err(SemanticPaginationError::DuplicateBlockIdentity {
            flow,
            block: owner,
        })
    {
        return Err("selected owner's nested child aliased its identity".into());
    }
    if session.current() != Some(&accepted) {
        return Err("selected-child refusal changed accepted session".into());
    }
    Ok(())
}

fn verify_nested_structural_owner_refusal() -> Result<(), String> {
    let ids = IdentityAllocator::new();
    let fixture = candidate_with_nested_sibling(&ids)?;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("distinct nested owner identities must be accepted".into());
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let measured = measurement(revision, flow, owner, &[(1, 1)]);
    let accepted = session
        .current()
        .ok_or("missing accepted revision")?
        .clone();
    if paginate_revision(&accepted, &measured).is_err() {
        return Err("valid nested structural owners were refused".into());
    }

    // A measured top-level block ID cannot also name a list, item, table,
    // row or cell, even when that structural owner is in another flow.
    for kind in 0usize..5 {
        let mut malformed = accepted.clone();
        let container_index = if kind < 2 { 2 } else { 3 };
        let container = malformed
            .notebook
            .pages
            .get_mut(1)
            .and_then(|page| page.flows.first_mut())
            .and_then(|sibling| sibling.blocks.get_mut(container_index))
            .ok_or("nested container missing")?;
        let changed = match (&mut container.content, kind) {
            (BlockContent::List(list), 0) => {
                list.id = owner;
                true
            },
            (BlockContent::List(list), 1) => {
                if let Some(item) = list.items.first_mut() {
                    item.id = owner;
                    true
                } else {
                    false
                }
            },
            (BlockContent::Table(table), 2) => {
                table.id = owner;
                true
            },
            (BlockContent::Table(table), 3) => {
                if let Some(row) = table.rows.first_mut() {
                    row.id = owner;
                    true
                } else {
                    false
                }
            },
            (BlockContent::Table(table), 4) => {
                if let Some(cell) = table
                    .rows
                    .first_mut()
                    .and_then(|row| row.cells.first_mut())
                {
                    cell.id = owner;
                    true
                } else {
                    false
                }
            },
            _ => false,
        };
        if !changed {
            return Err(format!("nested structural owner {kind} missing"));
        }
        if paginate_revision(&malformed, &measured)
            != Err(SemanticPaginationError::DuplicateBlockIdentity {
                flow,
                block: owner,
            })
        {
            return Err(format!(
                "nested structural owner {kind} aliased measured block"
            ));
        }
        if session.current() != Some(&accepted) {
            return Err("structural-owner refusal changed session".into());
        }
    }
    verify_selected_structural_owner_refusal(
        &accepted, &measured, flow, owner,
    )?;
    if session.current() != Some(&accepted) {
        return Err("selected structural refusal changed session".into());
    }
    Ok(())
}

fn verify_selected_structural_owner_refusal(
    accepted: &atrament_semantic_notebook::AcceptedRevision,
    measured: &RevisionFlowMeasurement,
    flow: AcceptedIdentity,
    owner: AcceptedIdentity,
) -> Result<(), String> {
    // The selected measured block must not also be the structural owner
    // of its own content, whether a list or a table.
    for (kind, index) in [2usize, 3].into_iter().enumerate() {
        let mut malformed = accepted.clone();
        let content = malformed
            .notebook
            .pages
            .get(1)
            .and_then(|page| page.flows.first())
            .and_then(|sibling| sibling.blocks.get(index))
            .ok_or("source container missing")?
            .content
            .clone();
        let selected = malformed
            .notebook
            .pages
            .first_mut()
            .and_then(|page| page.flows.first_mut())
            .and_then(|selected_flow| selected_flow.blocks.first_mut())
            .ok_or("selected owner missing")?;
        selected.content = content;
        if let BlockContent::List(list) = &mut selected.content {
            list.id = owner;
        } else if let BlockContent::Table(table) = &mut selected.content {
            table.id = owner;
        } else {
            return Err("selected container family missing".into());
        }
        if paginate_revision(&malformed, measured)
            != Err(SemanticPaginationError::DuplicateBlockIdentity {
                flow,
                block: owner,
            })
        {
            return Err(format!(
                "selected structural owner {kind} aliased its parent block"
            ));
        }
    }
    Ok(())
}

#[test]
fn nested_structural_owners_cannot_alias_a_measured_block() {
    assert_eq!(verify_nested_structural_owner_refusal(), Ok(()));
}

fn verify_leaf_semantic_owner_refusal() -> Result<(), String> {
    let ids = IdentityAllocator::new();
    let fixture = candidate_with_nested_sibling(&ids)?;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("valid nested fixture was refused".into());
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let measured = measurement(revision, flow, owner, &[(1, 1)]);
    let accepted = session
        .current()
        .ok_or("accepted revision missing")?
        .clone();
    if paginate_revision(&accepted, &measured).is_err() {
        return Err("valid nested fixture did not paginate".into());
    }
    // Reuse a child ID that becomes unowned when replacing its callout.
    let Some(BlockContent::Callout(children)) = accepted
        .notebook
        .pages
        .get(1)
        .and_then(|page| page.flows.first())
        .and_then(|sibling| sibling.blocks.first())
        .map(|block| &block.content)
    else {
        return Err("source callout missing".into());
    };
    let spare = children.first().ok_or("child identity missing")?.id;
    let span = InlineSpan {
        id: owner,
        provenance: None,
        style: None,
        text: String::from("x"),
    };
    let cases = [
        BlockContent::Citation(vec![span.clone()]),
        BlockContent::Date(vec![span.clone()]),
        BlockContent::Definition(vec![span.clone()]),
        BlockContent::Footnote(vec![span.clone()]),
        BlockContent::Heading(vec![span.clone()]),
        BlockContent::Label(vec![span.clone()]),
        BlockContent::MarginNote(vec![span.clone()]),
        BlockContent::Paragraph(vec![span.clone()]),
        BlockContent::Quotation(vec![span.clone()]),
        BlockContent::SourceNote(vec![span.clone()]),
        BlockContent::Figure(Figure {
            asset: None,
            caption: vec![],
            id: owner,
        }),
        BlockContent::Figure(Figure {
            asset: None,
            caption: vec![span],
            id: spare,
        }),
        BlockContent::Mathematics(Formula {
            id: owner,
            mode: FormulaMode::Display,
            source: String::from("x"),
        }),
    ];
    for (case, content) in cases.into_iter().enumerate() {
        let mut malformed = accepted.clone();
        let container = malformed
            .notebook
            .pages
            .get_mut(1)
            .and_then(|page| page.flows.first_mut())
            .and_then(|sibling| sibling.blocks.first_mut())
            .ok_or("source container missing")?;
        container.content = content;
        if paginate_revision(&malformed, &measured)
            != Err(SemanticPaginationError::DuplicateBlockIdentity {
                flow,
                block: owner,
            })
        {
            return Err(format!("leaf owner {case} was not refused"));
        }
        if session.current() != Some(&accepted) {
            return Err("leaf owner refusal mutated accepted state".into());
        }
    }
    verify_selected_leaf_owner_refusal(&accepted, &measured, flow, owner)?;
    Ok(())
}

fn verify_selected_leaf_owner_refusal(
    accepted: &atrament_semantic_notebook::AcceptedRevision,
    measured: &RevisionFlowMeasurement,
    flow: AcceptedIdentity,
    owner: AcceptedIdentity,
) -> Result<(), String> {
    let cases = [
        BlockContent::Paragraph(vec![InlineSpan {
            id: owner,
            provenance: None,
            style: None,
            text: String::from("x"),
        }]),
        BlockContent::Figure(Figure {
            asset: None,
            caption: vec![],
            id: owner,
        }),
        BlockContent::Mathematics(Formula {
            id: owner,
            mode: FormulaMode::Display,
            source: String::from("x"),
        }),
    ];
    for (case, content) in cases.into_iter().enumerate() {
        let mut malformed = accepted.clone();
        let selected = malformed
            .notebook
            .pages
            .first_mut()
            .and_then(|page| page.flows.first_mut())
            .and_then(|selected_flow| selected_flow.blocks.first_mut())
            .ok_or("selected block missing")?;
        selected.content = content;
        if paginate_revision(&malformed, measured)
            != Err(SemanticPaginationError::DuplicateBlockIdentity {
                flow,
                block: owner,
            })
        {
            return Err(format!("selected leaf owner {case} was not refused"));
        }
    }
    Ok(())
}

#[test]
fn leaf_semantic_owners_cannot_alias_a_measured_block() {
    assert_eq!(verify_leaf_semantic_owner_refusal(), Ok(()));
}

fn verify_notebook_level_owner_aliases() -> Result<(), String> {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("valid notebook fixture was refused".into());
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let measured = measurement(revision, flow, owner, &[(1, 1)]);
    let accepted = session
        .current()
        .ok_or("accepted revision missing")?
        .clone();
    if paginate_revision(&accepted, &measured).is_err() {
        return Err("valid notebook fixture did not paginate".into());
    }
    // These are semantic owners, not references such as `paper_profile`
    // or `Constraint.target`. Normal candidate admission prevents aliases;
    // mutate a cloned accepted value to exercise the typed layout boundary.
    for case in 0u8..9 {
        let mut malformed = accepted.clone();
        match case {
            0 => malformed.notebook.id = owner,
            1 => malformed
                .notebook
                .pages
                .get_mut(0)
                .ok_or("first page missing")?
                .id = owner,
            2 => malformed
                .notebook
                .pages
                .get_mut(1)
                .ok_or("second page missing")?
                .flows
                .push(Flow {
                    id: owner,
                    blocks: vec![],
                }),
            3 => {
                malformed
                    .notebook
                    .page_profiles
                    .get_mut(0)
                    .ok_or("first profile missing")?
                    .id = owner;
                malformed
                    .notebook
                    .pages
                    .get_mut(0)
                    .ok_or("first page missing")?
                    .paper_profile = owner;
            },
            4 => malformed.notebook.assets.push(Asset {
                id: owner,
                media_type: String::from("image/png"),
            }),
            5 => malformed.notebook.constraints.push(Constraint {
                id: owner,
                kind: ConstraintKind::Layout,
                target: owner,
            }),
            6 => malformed.notebook.output_profiles.push(OutputProfile {
                id: owner,
                name: String::from("print"),
            }),
            7 => malformed.notebook.provenance.push(Provenance {
                id: owner,
                kind: ProvenanceKind::Supplied,
                reference: None,
            }),
            8 => malformed.notebook.styles.push(Style {
                id: owner,
                name: String::from("body"),
            }),
            _ => return Err("unsupported owner case".into()),
        }
        if paginate_revision(&malformed, &measured)
            != Err(SemanticPaginationError::DuplicateBlockIdentity {
                flow,
                block: owner,
            })
        {
            return Err(format!("notebook owner family {case} was not refused"));
        }
        if session.current() != Some(&accepted) {
            return Err("owner refusal mutated accepted session".into());
        }
    }
    Ok(())
}

#[test]
fn notebook_level_owners_cannot_alias_a_measured_block() {
    assert_eq!(verify_notebook_level_owner_aliases(), Ok(()));
}

#[test]
fn nested_blocks_cannot_alias_a_selected_measured_owner() {
    assert_eq!(verify_nested_block_identity_refusal(), Ok(()));
}

#[test]
fn defensive_duplicate_accepted_page_identity_cannot_alias_placements() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let outcome = session.accept(fixture.notebook);
    assert!(matches!(outcome, AcceptanceOutcome::Accepted { .. }));
    let AcceptanceOutcome::Accepted { mapping, revision } = outcome else {
        return;
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let page = accepted_for(&mapping, fixture.page_one);
    let measured = measurement(revision, flow, owner, &[(1, 1)]);
    assert!(session.current().is_some());
    let Some(accepted) = session.current().cloned() else {
        return;
    };
    assert!(paginate_revision(&accepted, &measured).is_ok());

    // Emulate a malformed typed revision; normal candidate admission rejects
    // duplicate global identities before this downstream boundary.
    let mut corrupted = accepted.clone();
    assert!(corrupted.notebook.pages.len() >= 2);
    let Some(second_page) = corrupted.notebook.pages.get_mut(1) else {
        return;
    };
    second_page.id = page;
    assert_eq!(
        paginate_revision(&corrupted, &measured),
        Err(SemanticPaginationError::Pagination(
            PaginationError::DuplicatePageIdentity { page },
        )),
    );
    assert_eq!(session.current(), Some(&accepted));
}

#[test]
fn flow_on_later_page_rejects_identity_aliasing_an_earlier_page() {
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    assert_eq!(fixture.notebook.pages.len(), 2);
    let [first, second] = fixture.notebook.pages.as_mut_slice() else {
        return;
    };
    second.flows.append(&mut first.flows);
    let mut session = SemanticNotebookSessionService::default();
    let outcome = session.accept(fixture.notebook);
    assert!(matches!(outcome, AcceptanceOutcome::Accepted { .. }));
    let AcceptanceOutcome::Accepted { mapping, revision } = outcome else {
        return;
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let second_page = accepted_for(&mapping, fixture.page_two);
    let measured = measurement(revision, flow, owner, &[(1, 1)]);
    assert!(session.current().is_some());
    let Some(accepted) = session.current().cloned() else {
        return;
    };
    let result = paginate_revision(&accepted, &measured);
    assert!(result.is_ok());
    let Ok(plan) = result else {
        return;
    };
    assert_eq!(
        plan.placements().first().map(|place| place.page),
        Some(second_page),
    );

    // The selected flow starts on page two. A page identity collision with
    // the earlier, excluded page must still refuse globally.
    let mut corrupted = accepted.clone();
    let Some(first_page) = corrupted.notebook.pages.first_mut() else {
        return;
    };
    first_page.id = second_page;
    assert_eq!(
        paginate_revision(&corrupted, &measured),
        Err(SemanticPaginationError::Pagination(
            PaginationError::DuplicatePageIdentity { page: second_page },
        )),
    );
    assert_eq!(session.current(), Some(&accepted));
}

#[test]
fn defensive_missing_profile_never_falls_back_to_arbitrary_geometry() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let first_page = accepted_for(&mapping, fixture.page_one);
    let profile = accepted_for(&mapping, fixture.profile_one);
    let measured = measurement(revision, flow, owner, &[(1, 1)]);
    let mut corrupted = session.current().expect("accepted revision").clone();
    corrupted.page_profiles_forget_for_test();

    assert_eq!(
        paginate_revision(&corrupted, &measured),
        Err(SemanticPaginationError::MissingPageProfile {
            page: first_page,
            profile,
        }),
    );
}

#[test]
fn defensive_invalid_profile_returns_typed_physical_failure() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let first_page = accepted_for(&mapping, fixture.page_one);
    let measured = measurement(revision, flow, owner, &[(1, 1)]);
    let mut corrupted = session.current().expect("accepted revision").clone();
    corrupted.notebook.page_profiles[0].geometry.sheet.width = Length::ZERO;

    assert_eq!(
        paginate_revision(&corrupted, &measured),
        Err(SemanticPaginationError::InvalidPageProfile {
            page: first_page,
            reason: PageProfileError::SheetDimensionIsZero,
        }),
    );
}

trait CorruptAcceptedRevisionForTest {
    fn page_profiles_forget_for_test(&mut self);
}

impl CorruptAcceptedRevisionForTest
    for atrament_semantic_notebook::AcceptedRevision
{
    fn page_profiles_forget_for_test(&mut self) {
        self.notebook.page_profiles.clear();
    }
}

#[test]
fn current_revision_tag_does_not_admit_stale_block_owner() {
    let ids = IdentityAllocator::new();
    let first = candidate_fixture(&ids);
    let second = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted {
        mapping: first_mapping, ..
    } = session.accept(first.notebook)
    else {
        panic!("first candidate must be accepted");
    };
    let stale_owner = accepted_for(&first_mapping, first.block);
    let AcceptanceOutcome::Accepted {
        mapping: second_mapping,
        revision: current,
    } = session.accept(second.notebook)
    else {
        panic!("second candidate must be accepted");
    };
    let current_flow = accepted_for(&second_mapping, second.flow);
    let forged_current =
        measurement(current, current_flow, stale_owner, &[(1, 1)]);

    assert_eq!(
        paginate_revision(
            session.current().expect("current revision"),
            &forged_current,
        ),
        Err(SemanticPaginationError::MeasuredBlockNotInFlow {
            flow: current_flow,
            owner: stale_owner,
        }),
    );
}

#[test]
fn non_block_semantic_identity_cannot_masquerade_as_measured_flow_owner() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let page_owner = accepted_for(&mapping, fixture.page_one);
    let invalid = measurement(revision, flow, page_owner, &[(1, 1)]);

    assert_eq!(
        paginate_revision(
            session.current().expect("accepted revision"),
            &invalid
        ),
        Err(SemanticPaginationError::MeasuredBlockNotInFlow {
            flow,
            owner: page_owner,
        }),
    );
}

fn candidate_two_block_fixture(
    ids: &IdentityAllocator,
) -> (CandidateFixture, CandidateIdentity) {
    let mut fixture = candidate_fixture(ids);
    let second_block = ids.allocate_candidate().expect("second block");
    fixture.notebook.pages[0].flows[0].blocks.push(Block {
        content: BlockContent::Rule,
        extensions: vec![],
        id: second_block,
        provenance: None,
        style: None,
    });
    (fixture, second_block)
}

fn expected_owner_sequence_outcome(
    flow: AcceptedIdentity,
    blocks: &[AcceptedIdentity],
    owners: &[AcceptedIdentity],
    outsider: AcceptedIdentity,
) -> Result<(), SemanticPaginationError> {
    let mut previous = None;
    let mut run_index = 0usize;
    for owner in owners {
        if previous == Some(*owner) {
            continue;
        }
        previous = Some(*owner);
        let Some(expected) = blocks.get(run_index) else {
            return if blocks.contains(owner) {
                Err(SemanticPaginationError::MeasurementBlockSequenceMismatch {
                    flow,
                })
            } else {
                Err(SemanticPaginationError::MeasuredBlockNotInFlow {
                    flow,
                    owner: *owner,
                })
            };
        };
        if *owner != *expected {
            return if *owner == outsider {
                Err(SemanticPaginationError::MeasuredBlockNotInFlow {
                    flow,
                    owner: *owner,
                })
            } else {
                Err(SemanticPaginationError::MeasurementBlockSequenceMismatch {
                    flow,
                })
            };
        }
        run_index += 1;
    }
    if let Some(missing) = blocks.get(run_index) {
        return Err(SemanticPaginationError::MeasurementIncomplete {
            flow,
            missing: *missing,
        });
    }
    Ok(())
}

#[test]
fn streaming_owner_admission_matches_reference_oracle() {
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    let second = ids.allocate_candidate().expect("second block");
    let third = ids.allocate_candidate().expect("third block");
    fixture.notebook.pages[0].flows[0].blocks.extend([
        Block {
            content: BlockContent::Rule,
            extensions: vec![],
            id: second,
            provenance: None,
            style: None,
        },
        Block {
            content: BlockContent::Rule,
            extensions: vec![],
            id: third,
            provenance: None,
            style: None,
        },
    ]);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("three-block candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let blocks = [
        accepted_for(&mapping, fixture.block),
        accepted_for(&mapping, second),
        accepted_for(&mapping, third),
    ];
    let outsider = accepted_for(&mapping, fixture.page_one);
    let alphabet = [blocks[0], blocks[1], blocks[2], outsider];
    let accepted = session.current().expect("accepted revision");
    let mut cases = 0usize;
    let mut saw_success = false;
    let mut saw_incomplete = false;
    let mut saw_sequence_mismatch = false;
    let mut saw_foreign_owner = false;

    for length in 0..=4usize {
        let combinations = 4usize.pow(length as u32);
        for encoded in 0..combinations {
            let mut value = encoded;
            let mut owners = Vec::with_capacity(length);
            for _ in 0..length {
                owners.push(alphabet[value % alphabet.len()]);
                value /= alphabet.len();
            }
            let measured = RevisionFlowMeasurement {
                flow,
                revision,
                units: vec![MeasuredFlowUnit {
                    fragments: owners
                        .iter()
                        .map(|owner| MeasuredFragment {
                            height: Length::from_micrometres(1),
                            owner: *owner,
                            width: Length::from_micrometres(1),
                        })
                        .collect(),
                    policy: FlowUnitPolicy::Independent,
                }],
            };
            let expected = expected_owner_sequence_outcome(
                flow,
                &blocks,
                &owners,
                outsider,
            );
            let actual = paginate_revision(accepted, &measured);
            match expected {
                Ok(()) => {
                    saw_success = true;
                    assert!(actual.is_ok());
                },
                Err(reason) => {
                    match reason {
                        SemanticPaginationError::MeasurementIncomplete {
                            ..
                        } => {
                            saw_incomplete = true;
                        },
                        SemanticPaginationError::
                            MeasurementBlockSequenceMismatch { .. } => {
                                saw_sequence_mismatch = true;
                            },
                        SemanticPaginationError::MeasuredBlockNotInFlow {
                            ..
                        } => {
                            saw_foreign_owner = true;
                        },
                        _ => panic!("unexpected exhaustive owner outcome"),
                    }
                    assert_eq!(actual, Err(reason));
                },
            }
            cases += 1;
        }
    }
    assert_eq!(cases, 341);
    assert!(saw_success);
    assert!(saw_incomplete);
    assert!(saw_sequence_mismatch);
    assert!(saw_foreign_owner);
}

fn next_owner_oracle_value(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *seed >> 32
}

#[test]
fn multi_unit_owner_admission_matches_reference_oracle() {
    const CASES: usize = 20_000;
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    let second = ids.allocate_candidate().expect("second block");
    let third = ids.allocate_candidate().expect("third block");
    fixture.notebook.pages[0].flows[0].blocks.extend([
        Block {
            content: BlockContent::Rule,
            extensions: vec![],
            id: second,
            provenance: None,
            style: None,
        },
        Block {
            content: BlockContent::Rule,
            extensions: vec![],
            id: third,
            provenance: None,
            style: None,
        },
    ]);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("three-block candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let blocks = [
        accepted_for(&mapping, fixture.block),
        accepted_for(&mapping, second),
        accepted_for(&mapping, third),
    ];
    let outsider = accepted_for(&mapping, fixture.page_one);
    let alphabet = [blocks[0], blocks[1], blocks[2], outsider];
    let accepted = session.current().expect("accepted revision");
    let mut seed = 0x5eed_0a11_2026_u64;
    let mut seen_owner_counts = [false; 9];
    let mut seen_owner_identities = [false; 4];
    let mut seen_group_sizes = [false; 3];
    let mut seen_policies = [false; 2];
    let mut saw_empty_unit = false;
    let mut saw_success = false;
    let mut saw_incomplete = false;
    let mut saw_sequence_mismatch = false;
    let mut saw_foreign_owner = false;

    for case in 0..CASES {
        let owner_count = (next_owner_oracle_value(&mut seed) % 9) as usize;
        seen_owner_counts[owner_count] = true;
        let owners = (0..owner_count)
            .map(|_| {
                let generated = next_owner_oracle_value(&mut seed) as usize;
                let index = generated % alphabet.len();
                seen_owner_identities[index] = true;
                alphabet[index]
            })
            .collect::<Vec<_>>();
        let mut units = Vec::new();
        let mut offset = 0usize;
        while offset < owners.len() {
            if next_owner_oracle_value(&mut seed).is_multiple_of(5) {
                saw_empty_unit = true;
                units.push(MeasuredFlowUnit {
                    fragments: vec![],
                    policy: FlowUnitPolicy::Independent,
                });
            }
            let remaining = owners.len() - offset;
            let take = usize::min(
                remaining,
                (next_owner_oracle_value(&mut seed) % 3 + 1) as usize,
            );
            seen_group_sizes[take - 1] = true;
            let fragments = owners[offset..offset + take]
                .iter()
                .map(|owner| MeasuredFragment {
                    height: Length::from_micrometres(1),
                    owner: *owner,
                    width: Length::from_micrometres(1),
                })
                .collect();
            let policy_index =
                (next_owner_oracle_value(&mut seed) & 1) as usize;
            seen_policies[policy_index] = true;
            let policy = if policy_index == 0 {
                FlowUnitPolicy::Independent
            } else {
                FlowUnitPolicy::KeepTogetherWhenPossible
            };
            units.push(MeasuredFlowUnit { fragments, policy });
            offset += take;
        }
        if owners.is_empty()
            && next_owner_oracle_value(&mut seed) & 1 == 0
        {
            units.push(MeasuredFlowUnit {
                fragments: vec![],
                policy: FlowUnitPolicy::KeepTogetherWhenPossible,
            });
        }
        let measured = RevisionFlowMeasurement { flow, revision, units };
        let expected = expected_owner_sequence_outcome(
            flow,
            &blocks,
            &owners,
            outsider,
        );
        let actual = paginate_revision(accepted, &measured);
        match expected {
            Ok(()) => {
                saw_success = true;
                assert!(actual.is_ok(), "generated case {case}");
            },
            Err(reason) => {
                match reason {
                    SemanticPaginationError::MeasurementIncomplete { .. } => {
                        saw_incomplete = true;
                    },
                    SemanticPaginationError::MeasurementBlockSequenceMismatch {
                        ..
                    } => saw_sequence_mismatch = true,
                    SemanticPaginationError::MeasuredBlockNotInFlow { .. } => {
                        saw_foreign_owner = true;
                    },
                    _ => panic!("unexpected reference outcome in case {case}"),
                }
                assert_eq!(actual, Err(reason), "generated case {case}");
            },
        }
    }
    assert!(seen_owner_counts.into_iter().all(|seen| seen));
    assert!(seen_owner_identities.into_iter().all(|seen| seen));
    assert!(seen_group_sizes.into_iter().all(|seen| seen));
    assert!(seen_policies.into_iter().all(|seen| seen));
    assert!(saw_empty_unit);
    assert!(saw_success);
    assert!(saw_incomplete);
    assert!(saw_sequence_mismatch);
    assert!(saw_foreign_owner);
}

#[test]
fn large_complete_measurement_streams_in_semantic_block_order() {
    const BLOCKS: usize = 10_000;

    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    let mut candidates = Vec::with_capacity(BLOCKS);
    candidates.push(fixture.block);
    for _ in 1..BLOCKS {
        let block = ids.allocate_candidate().expect("large-flow block");
        fixture.notebook.pages[0].flows[0].blocks.push(Block {
            content: BlockContent::Rule,
            extensions: vec![],
            id: block,
            provenance: None,
            style: None,
        });
        candidates.push(block);
    }
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("large-flow candidate must be accepted");
    };
    let accepted = mapping
        .iter()
        .map(|entry| (entry.candidate, entry.accepted))
        .collect::<BTreeMap<_, _>>();
    let flow = *accepted.get(&fixture.flow).expect("accepted flow");
    let fragments = candidates
        .iter()
        .map(|candidate| MeasuredFragment {
            height: Length::from_micrometres(1),
            owner: *accepted.get(candidate).expect("accepted block"),
            width: Length::from_micrometres(1),
        })
        .collect::<Vec<_>>();
    let first = fragments.first().expect("first measured block").owner;
    let last = fragments.last().expect("last measured block").owner;
    let measured = RevisionFlowMeasurement {
        flow,
        revision,
        units: vec![MeasuredFlowUnit {
            fragments,
            policy: FlowUnitPolicy::Independent,
        }],
    };

    let plan = paginate_revision(
        session.current().expect("accepted revision"),
        &measured,
    )
    .expect("large complete measurement must paginate");
    assert_eq!(plan.placements().len(), BLOCKS);
    assert_eq!(plan.placements().first().map(|item| item.owner), Some(first));
    assert_eq!(plan.placements().last().map(|item| item.owner), Some(last));
    assert_eq!(
        plan.placements().first().map(|item| (
            item.source.unit, item.source.fragment,
        )),
        Some((0, 0)),
    );
    assert_eq!(
        plan.placements().last().map(|item| (
            item.source.unit, item.source.fragment,
        )),
        Some((0, BLOCKS - 1)),
    );
}

#[test]
fn incomplete_measurement_cannot_silently_drop_flow_block() {
    let ids = IdentityAllocator::new();
    let (fixture, second_block) = candidate_two_block_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let first = accepted_for(&mapping, fixture.block);
    let missing = accepted_for(&mapping, second_block);
    let incomplete = measurement(revision, flow, first, &[(1, 1)]);

    assert_eq!(
        paginate_revision(
            session.current().expect("accepted revision"),
            &incomplete,
        ),
        Err(SemanticPaginationError::MeasurementIncomplete { flow, missing }),
    );
}

#[test]
fn reordered_measurement_cannot_change_semantic_flow_order() {
    let ids = IdentityAllocator::new();
    let (fixture, second_block) = candidate_two_block_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let first = accepted_for(&mapping, fixture.block);
    let second = accepted_for(&mapping, second_block);
    let reordered = RevisionFlowMeasurement {
        flow,
        revision,
        units: vec![MeasuredFlowUnit {
            fragments: vec![
                MeasuredFragment {
                    height: Length::from_micrometres(1),
                    owner: second,
                    width: Length::from_micrometres(1),
                },
                MeasuredFragment {
                    height: Length::from_micrometres(1),
                    owner: first,
                    width: Length::from_micrometres(1),
                },
            ],
            policy: FlowUnitPolicy::Independent,
        }],
    };

    assert_eq!(
        paginate_revision(
            session.current().expect("accepted revision"),
            &reordered,
        ),
        Err(SemanticPaginationError::MeasurementBlockSequenceMismatch { flow }),
    );
}

#[test]
fn repeated_owner_across_measurement_units_remains_one_owner_run() {
    let ids = IdentityAllocator::new();
    let (fixture, second_block) = candidate_two_block_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let first = accepted_for(&mapping, fixture.block);
    let second = accepted_for(&mapping, second_block);
    let measured = RevisionFlowMeasurement {
        flow,
        revision,
        units: vec![
            MeasuredFlowUnit {
                fragments: vec![MeasuredFragment {
                    height: Length::from_micrometres(1),
                    owner: first,
                    width: Length::from_micrometres(1),
                }],
                policy: FlowUnitPolicy::Independent,
            },
            MeasuredFlowUnit {
                fragments: vec![
                    MeasuredFragment {
                        height: Length::from_micrometres(2),
                        owner: first,
                        width: Length::from_micrometres(2),
                    },
                    MeasuredFragment {
                        height: Length::from_micrometres(3),
                        owner: second,
                        width: Length::from_micrometres(3),
                    },
                ],
                policy: FlowUnitPolicy::Independent,
            },
        ],
    };

    let plan = paginate_revision(
        session.current().expect("accepted revision"),
        &measured,
    )
    .expect("cross-unit owner run must remain complete");
    assert_eq!(
        plan.placements()
            .iter()
            .map(|placed| (placed.source.unit, placed.source.fragment))
            .collect::<Vec<_>>(),
        [(0, 0), (1, 0), (1, 1)],
    );
    assert_eq!(plan.placements().len(), 3);
    assert_eq!(plan.placements()[0].owner, first);
    assert_eq!(plan.placements()[1].owner, first);
    assert_eq!(plan.placements()[2].owner, second);
}

#[test]
fn nonconsecutive_owner_recurrence_remains_sequence_mismatch() {
    let ids = IdentityAllocator::new();
    let (fixture, second_block) = candidate_two_block_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let first = accepted_for(&mapping, fixture.block);
    let second = accepted_for(&mapping, second_block);
    let measured = RevisionFlowMeasurement {
        flow,
        revision,
        units: vec![MeasuredFlowUnit {
            fragments: vec![
                MeasuredFragment {
                    height: Length::from_micrometres(1),
                    owner: first,
                    width: Length::from_micrometres(1),
                },
                MeasuredFragment {
                    height: Length::from_micrometres(1),
                    owner: second,
                    width: Length::from_micrometres(1),
                },
                MeasuredFragment {
                    height: Length::from_micrometres(1),
                    owner: first,
                    width: Length::from_micrometres(1),
                },
            ],
            policy: FlowUnitPolicy::Independent,
        }],
    };

    assert_eq!(
        paginate_revision(
            session.current().expect("accepted revision"),
            &measured,
        ),
        Err(SemanticPaginationError::MeasurementBlockSequenceMismatch { flow }),
    );
}

#[test]
fn repeated_fragments_for_one_block_remain_one_semantic_owner_run() {
    let ids = IdentityAllocator::new();
    let (fixture, second_block) = candidate_two_block_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let first = accepted_for(&mapping, fixture.block);
    let second = accepted_for(&mapping, second_block);
    let measured = RevisionFlowMeasurement {
        flow,
        revision,
        units: vec![MeasuredFlowUnit {
            fragments: vec![
                MeasuredFragment {
                    height: Length::from_micrometres(1),
                    owner: first,
                    width: Length::from_micrometres(1),
                },
                MeasuredFragment {
                    height: Length::from_micrometres(2),
                    owner: first,
                    width: Length::from_micrometres(2),
                },
                MeasuredFragment {
                    height: Length::from_micrometres(3),
                    owner: second,
                    width: Length::from_micrometres(3),
                },
            ],
            policy: FlowUnitPolicy::Independent,
        }],
    };

    let plan = paginate_revision(
        session.current().expect("accepted revision"),
        &measured,
    )
    .expect("complete ordered measurement");
    assert_eq!(plan.placements().len(), 3);
    assert_eq!(plan.placements()[0].owner, first);
    assert_eq!(plan.placements()[1].owner, first);
    assert_eq!(plan.placements()[2].owner, second);
}

#[test]
fn unknown_flow_identity_is_typed_before_page_geometry() {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let owner = accepted_for(&mapping, fixture.block);
    let not_flow = accepted_for(&mapping, fixture.page_one);
    let invalid = measurement(revision, not_flow, owner, &[(1, 1)]);

    assert_eq!(
        paginate_revision(
            session.current().expect("accepted revision"),
            &invalid
        ),
        Err(SemanticPaginationError::FlowNotFound { flow: not_flow }),
    );
}

#[test]
fn keep_group_skip_never_backfills_earlier_accepted_page() {
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    fixture.notebook.page_profiles[1].geometry = geometry(155_000);
    let profile_three = ids.allocate_candidate().expect("third profile");
    let page_three = ids.allocate_candidate().expect("third page");
    fixture.notebook.page_profiles.push(PaperProfile {
        geometry: geometry(20_000),
        id: profile_three,
    });
    fixture.notebook.pages.push(Page {
        flows: Vec::new(),
        id: page_three,
        paper_profile: profile_three,
    });

    let second = ids.allocate_candidate().expect("second block");
    let third = ids.allocate_candidate().expect("third block");
    let fourth = ids.allocate_candidate().expect("fourth block");
    for block in [second, third, fourth] {
        fixture.notebook.pages[0].flows[0].blocks.push(Block {
            content: BlockContent::Rule,
            extensions: vec![],
            id: block,
            provenance: None,
            style: None,
        });
    }

    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("three-page candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owners = [
        accepted_for(&mapping, fixture.block),
        accepted_for(&mapping, second),
        accepted_for(&mapping, third),
        accepted_for(&mapping, fourth),
    ];
    let first_page = accepted_for(&mapping, fixture.page_one);
    let third_page = accepted_for(&mapping, page_three);
    let measured = RevisionFlowMeasurement {
        flow,
        revision,
        units: vec![
            MeasuredFlowUnit {
                fragments: vec![MeasuredFragment {
                    height: Length::from_micrometres(250_000),
                    owner: owners[0],
                    width: Length::from_micrometres(50_000),
                }],
                policy: FlowUnitPolicy::Independent,
            },
            MeasuredFlowUnit {
                fragments: owners[1..3]
                    .iter()
                    .map(|owner| MeasuredFragment {
                        height: Length::from_micrometres(40_000),
                        owner: *owner,
                        width: Length::from_micrometres(100_000),
                    })
                    .collect(),
                policy: FlowUnitPolicy::KeepTogetherWhenPossible,
            },
            MeasuredFlowUnit {
                fragments: vec![MeasuredFragment {
                    height: Length::from_micrometres(10_000),
                    owner: owners[3],
                    width: Length::from_micrometres(20_000),
                }],
                policy: FlowUnitPolicy::Independent,
            },
        ],
    };

    let plan = paginate_revision(
        session.current().expect("accepted revision"),
        &measured,
    )
    .expect("accepted page order must remain monotone");
    assert_eq!(
        plan.placements()
            .iter()
            .map(|placement| placement.page)
            .collect::<Vec<_>>(),
        [first_page, third_page, third_page, third_page],
    );
    assert_eq!(
        plan.placements()[3].top,
        Length::from_micrometres(100_000),
    );
}

#[test]
fn flow_owned_by_second_page_never_backfills_first_page() {
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    let moved_flows = std::mem::take(&mut fixture.notebook.pages[0].flows);
    fixture.notebook.pages[1].flows = moved_flows;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        panic!("candidate must be accepted");
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let second_page = accepted_for(&mapping, fixture.page_two);
    let measured = measurement(revision, flow, owner, &[(1, 1)]);

    let plan = paginate_revision(
        session.current().expect("accepted revision"),
        &measured,
    )
    .expect("second-page flow");
    assert_eq!(plan.placements().len(), 1);
    assert_eq!(plan.placements()[0].page, second_page);
    // The selected flow starts here, not on the preceding notebook page.
    assert_eq!(plan.covered_pages().iter().map(|p| p.page)
        .collect::<Vec<_>>(), [second_page]);
}

fn add_unmeasured_flow(
    fixture: &mut CandidateFixture,
    page_index: usize,
    flow: CandidateIdentity,
    block: CandidateIdentity,
) -> Result<(), String> {
    let page = fixture.notebook.pages.get_mut(page_index)
        .ok_or_else(|| format!("missing fixture page {page_index}"))?;
    page.flows.push(Flow {
        blocks: vec![Block {
            content: BlockContent::Rule,
            extensions: vec![],
            id: block,
            provenance: None,
            style: None,
        }],
        id: flow,
    });
    Ok(())
}

#[test]
fn later_page_flow_content_cannot_be_overlaid_by_measured_spill()
-> Result<(), String> {
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    let other_flow = ids.allocate_candidate()
        .map_err(|_error| "other flow allocation failed")?;
    let other_block = ids.allocate_candidate()
        .map_err(|_error| "other block allocation failed")?;
    add_unmeasured_flow(&mut fixture, 1, other_flow, other_block)?;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("distinct flows must be valid accepted content".into());
    };
    let selected = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let occupied_page = accepted_for(&mapping, fixture.page_two);
    let occupied_flow = accepted_for(&mapping, other_flow);
    let accepted = session.current()
        .ok_or("accepted revision missing")?.clone();

    let short = measurement(revision, selected, owner, &[(1, 1)]);
    if paginate_revision(&accepted, &short)
        .map_err(|error| format!("short flow refused: {error:?}"))?
        .placements().len() != 1 {
        return Err("short flow was not on its first page".into());
    }
    let spilling = measurement(revision, selected, owner, &[
        (100_000, 260_000),
        (100_000, 20_000),
    ]);
    if paginate_revision(&accepted, &spilling)
        != Err(SemanticPaginationError::UnmeasuredFlowOnPage {
            page: occupied_page,
            flow: occupied_flow,
        }) {
        return Err("spill over unmeasured flow was not refused".into());
    }

    let mut cleared = accepted.clone();
    cleared.notebook.pages.get_mut(1)
        .and_then(|page| page.flows.first_mut())
        .ok_or("empty peer flow fixture missing")?
        .blocks.clear();
    let plan = paginate_revision(&cleared, &spilling)
        .map_err(|error| format!("empty peer refused: {error:?}"))?;
    if plan.placements().get(1).map(|placed| placed.page)
        != Some(occupied_page) {
        return Err("empty peer did not allow page-two spill".into());
    }
    if session.current() != Some(&accepted) {
        return Err("pagination mutated accepted session".into());
    }
    Ok(())
}

#[test]
fn sibling_flow_content_cannot_share_an_unmeasured_writable_region()
-> Result<(), String> {
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    let other_flow = ids.allocate_candidate()
        .map_err(|_error| "sibling flow allocation failed")?;
    let other_block = ids.allocate_candidate()
        .map_err(|_error| "sibling block allocation failed")?;
    add_unmeasured_flow(&mut fixture, 0, other_flow, other_block)?;
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("multiple distinct flows are valid semantic source".into());
    };
    let selected = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let occupied_page = accepted_for(&mapping, fixture.page_one);
    let occupied_flow = accepted_for(&mapping, other_flow);
    let accepted = session.current()
        .ok_or("accepted revision missing")?.clone();
    let measured = measurement(revision, selected, owner, &[(1, 1)]);
    if paginate_revision(&accepted, &measured)
        != Err(SemanticPaginationError::UnmeasuredFlowOnPage {
            page: occupied_page,
            flow: occupied_flow,
        }) {
        return Err("sibling flow page alias was not refused".into());
    }
    if session.current() != Some(&accepted) {
        return Err("pagination mutated accepted session".into());
    }
    Ok(())
}

#[test]
fn occupied_narrow_page_skipped_by_flow_remains_untouched()
-> Result<(), String> {
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    let other_flow = ids.allocate_candidate()
        .map_err(|_error| "unmeasured flow allocation failed")?;
    let other_block = ids.allocate_candidate()
        .map_err(|_error| "unmeasured block allocation failed")?;
    let third_candidate = ids.allocate_candidate()
        .map_err(|_error| "later empty page allocation failed")?;
    fixture.notebook.page_profiles.get_mut(1)
        .ok_or("second paper profile missing")?
        .geometry = geometry(40_000);
    add_unmeasured_flow(&mut fixture, 1, other_flow, other_block)?;
    fixture.notebook.pages.push(Page {
        flows: vec![],
        id: third_candidate,
        paper_profile: fixture.profile_one,
    });
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("an occupied narrow page is valid source authority".into());
    };
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let first = accepted_for(&mapping, fixture.page_one);
    let skipped = accepted_for(&mapping, fixture.page_two);
    let third_page = accepted_for(&mapping, third_candidate);
    let accepted = session.current()
        .ok_or("accepted revision missing")?.clone();

    for policy in [
        FlowUnitPolicy::Independent,
        FlowUnitPolicy::KeepTogetherWhenPossible,
    ] {
        let measurement = RevisionFlowMeasurement {
            flow,
            revision,
            units: vec![
                MeasuredFlowUnit {
                    fragments: vec![MeasuredFragment {
                        owner,
                        width: Length::from_micrometres(100_000),
                        height: Length::from_micrometres(260_000),
                    }],
                    policy: FlowUnitPolicy::Independent,
                },
                MeasuredFlowUnit {
                    fragments: vec![MeasuredFragment {
                        owner,
                        width: Length::from_micrometres(160_000),
                        height: Length::from_micrometres(20_000),
                    }],
                    policy,
                },
            ],
        };
        let plan = paginate_revision(&accepted, &measurement)
            .map_err(|error| format!("skipped page refused: {error:?}"))?;
        if plan.covered_pages().iter().map(|p| p.page)
            .collect::<Vec<_>>() != [first, skipped, third_page] {
            return Err(format!("covered page sequence mismatch: {policy:?}"));
        }
        for covered in plan.covered_pages() {
            let page = accepted.notebook.pages.iter()
                .find(|page| page.id == covered.page)
                .ok_or("covered page not accepted")?;
            let profile = accepted.notebook.page_profiles.iter()
                .find(|profile| profile.id == page.paper_profile)
                .ok_or("covered page profile not accepted")?;
            let expected = profile.geometry.validated()
                .map_err(|reason| format!(
                    "invalid covered profile: {reason:?}"
                ))?
                .writable_region();
            if covered.writable != expected {
                return Err(format!(
                    "covered page geometry mismatch: {policy:?}"
                ));
            }
        }
        if plan.placements().len() != 2
            || plan.placements().first().map(|placed| placed.page)
                != Some(first)
            || plan.placements().get(1).map(|placed| placed.page)
                != Some(third_page)
            || plan.placements().iter().any(|placed| placed.page == skipped)
        {
            return Err(format!("skipped-page placement mismatch: {policy:?}"));
        }
    }
    if session.current() != Some(&accepted) {
        return Err("pagination mutated accepted session".into());
    }
    Ok(())
}

#[test]
fn fixed_regions_inspect_measured_vertical_bands_without_guessing_alignment()
-> Result<(), String> {
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("fixture refused".into());
    };
    let accepted = session.current().ok_or("missing revision")?;
    let flow = accepted_for(&mapping, fixture.flow);
    let block = accepted_for(&mapping, fixture.block);
    let page = accepted_for(&mapping, fixture.page_one);
    let later_page = accepted_for(&mapping, fixture.page_two);
    let measured = measurement(revision, flow, block, &[
        (20_000, 20_000),
        (20_000, 20_000),
        (20_000, 260_000),
    ]);
    let plan = paginate_revision(accepted, &measured)
        .map_err(|error| format!("pagination refused: {error:?}"))?;
    let first = plan.placements().first().ok_or("first row missing")?;
    let third = plan.placements().last().ok_or("last row missing")?;
    if third.page != later_page || first.page != page {
        return Err("expected overflow onto second page".into());
    }
    let fixed = |page, x, y, width, height| {
        validate_fixed_placement(accepted, AcceptedFixedPlacement {
            object: block,
            page,
            rectangle: Rect {
                x: Length::from_micrometres(x),
                y: Length::from_micrometres(y),
                width: Length::from_micrometres(width),
                height: Length::from_micrometres(height),
            },
            revision,
        })
        .map_err(|error| format!("fixed admission failed: {error:?}"))
    };
    let writable = first.writable;
    let top = first.top.micrometres();
    let x = writable.x.micrometres();
    let right = x + writable.width.micrometres();
    let adjacent = fixed(page, x, top + 40_000, 5_000, 10_000)?;
    let before = fixed(page, x, top, 0, 10_000)?;
    let empty = fixed(page, x, top, 10_000, 0)?;
    let far_right = fixed(page, right - 1_000, top + 1, 1_000, 1_000)?;
    assert!(
        check_fixed_regions_for_flow(&plan, &[adjacent, before, empty]).is_ok()
    );
    assert_eq!(
        check_fixed_regions_for_flow(&plan, &[far_right]),
        Err(FlowFixedRegionError::VerticalBandOverlap {
            page,
            object: block,
            fragment_owner: block,
            source: first.source,
            overlap_top: Length::from_micrometres(top + 1),
            overlap_height: Length::from_micrometres(1_000),
        }),
    );
    let beyond = fixed(
        page,
        x,
        writable.y.micrometres(),
        writable.width.micrometres() + 1,
        1_000,
    )?;
    assert_eq!(
        check_fixed_regions_for_flow(&plan, &[beyond]),
        Err(FlowFixedRegionError::FixedRegionOverflow { page, object: block }),
    );
    // A fixed result from a later session revision cannot be substituted.
    let foreign_revision = (0..3)
        .map(|_| {
            ids.allocate_revision()
                .map_err(|error| format!("id: {error:?}"))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .find(|candidate| *candidate != revision)
        .ok_or("expected a fresh revision")?;
    let mut revised = accepted.clone();
    revised.id = foreign_revision;
    let stale = validate_fixed_placement(&revised, AcceptedFixedPlacement {
        object: block,
        page,
        rectangle: Rect {
            x: Length::from_micrometres(x),
            y: Length::from_micrometres(top + 40_000),
            width: Length::from_micrometres(1_000),
            height: Length::from_micrometres(1_000),
        },
        revision: foreign_revision,
    })
    .map_err(|error| format!("revised bounds: {error:?}"))?;
    assert_eq!(
        check_fixed_regions_for_flow(&plan, &[stale]),
        Err(FlowFixedRegionError::RevisionMismatch {
            expected: revision,
            fixed: foreign_revision,
            object: block,
        }),
    );
    // A defensive revision with an unchanged identity but changed page
    // geometry cannot smuggle conflicting writable bounds into this plan.
    let mut changed_geometry = accepted.clone();
    changed_geometry.notebook.page_profiles[0]
        .geometry
        .printable_region
        .width = Length::from_micrometres(185_000);
    let mismatched =
        validate_fixed_placement(&changed_geometry, AcceptedFixedPlacement {
            object: block,
            page,
            rectangle: Rect {
                x: Length::from_micrometres(x),
                y: Length::from_micrometres(top + 40_000),
                width: Length::from_micrometres(1_000),
                height: Length::from_micrometres(1_000),
            },
            revision,
        })
        .map_err(|error| format!("alternate bounds: {error:?}"))?;
    assert_eq!(
        check_fixed_regions_for_flow(&plan, &[mismatched]),
        Err(FlowFixedRegionError::WritableMismatch { page, object: block }),
    );
    assert_eq!(session.current(), Some(accepted));
    Ok(())
}

#[test]
fn fixed_regions_on_uncovered_pages_do_not_collide_with_flow_rows()
-> Result<(), String> {
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    let fixed_flow = ids
        .allocate_candidate()
        .map_err(|error| format!("fixed flow: {error:?}"))?;
    let fixed_block = ids
        .allocate_candidate()
        .map_err(|error| format!("fixed block: {error:?}"))?;
    fixture.notebook.pages[1].flows.push(Flow {
        id: fixed_flow,
        blocks: vec![Block {
            id: fixed_block,
            content: BlockContent::Rule,
            extensions: vec![],
            provenance: None,
            style: None,
        }],
    });
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("fixture with independent page content refused".into());
    };
    let accepted = session.current().ok_or("missing revision")?;
    let flow = accepted_for(&mapping, fixture.flow);
    let block = accepted_for(&mapping, fixture.block);
    let page = accepted_for(&mapping, fixture.page_two);
    let object = accepted_for(&mapping, fixed_block);
    let measured = measurement(revision, flow, block, &[(20_000, 20_000)]);
    let plan = paginate_revision(accepted, &measured)
        .map_err(|error| format!("pagination: {error:?}"))?;
    assert_eq!(plan.covered_pages().len(), 1);
    let region = validate_fixed_placement(accepted, AcceptedFixedPlacement {
        page,
        object,
        revision,
        rectangle: Rect {
            x: Length::from_micrometres(40_000),
            y: Length::from_micrometres(30_000),
            width: Length::from_micrometres(10_000),
            height: Length::from_micrometres(10_000),
        },
    })
    .map_err(|error| format!("fixed bounds: {error:?}"))?;
    assert_eq!(check_fixed_regions_for_flow(&plan, &[region]), Ok(()));
    assert!(check_fixed_regions_for_flow(&plan, &[]).is_ok());
    Ok(())
}

#[test]
fn fixed_flow_clearance_matches_independent_physical_interval_oracle()
-> Result<(), String> {
    use atrament_flow_pagination::FragmentSource;

    const CASES: usize = 20_000;
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("oracle fixture refused".into());
    };
    let accepted = session.current().ok_or("missing accepted revision")?;
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let page = accepted_for(&mapping, fixture.page_one);
    let measured = measurement(revision, flow, owner, &[
        (15_000, 10_000),
        (15_000, 15_000),
    ]);
    let plan = paginate_revision(accepted, &measured)
        .map_err(|error| format!("reference flow: {error:?}"))?;
    let writable = plan
        .covered_pages()
        .first()
        .ok_or("no first page writable region")?
        .writable;
    let width = writable.width.micrometres();
    let height = writable.height.micrometres();
    let x = writable.x.micrometres();
    let y = writable.y.micrometres();
    let mut seed = 0x26d7_4a65_9021_5f31;
    let mut counts = [0usize; 4]; // clear, first-row, second-row, overflow
    for case in 0..CASES {
        let r1 = next_owner_oracle_value(&mut seed);
        let r2 = next_owner_oracle_value(&mut seed);
        let r3 = next_owner_oracle_value(&mut seed);
        let r4 = next_owner_oracle_value(&mut seed);
        let offset_x = if case % 7 == 0 {
            width - 500
        } else {
            r1 % 50_000
        };
        let offset_y = if case % 11 == 0 {
            height - 500
        } else {
            r2 % 60_000
        };
        let extent_x = if case % 13 == 0 {
            0
        } else {
            r3 % 10_000
        };
        let extent_y = if case % 17 == 0 {
            0
        } else {
            r4 % 25_000
        };
        let checked =
            validate_fixed_placement(accepted, AcceptedFixedPlacement {
                object: owner,
                page,
                revision,
                rectangle: Rect {
                    x: Length::from_micrometres(x + offset_x),
                    y: Length::from_micrometres(y + offset_y),
                    width: Length::from_micrometres(extent_x),
                    height: Length::from_micrometres(extent_y),
                },
            })
            .map_err(|error| format!("case {case} bounds: {error:?}"))?;
        // Independent oracle: the first occupied row is [0, 10_000),
        // the second is [10_000, 25_000), relative to accepted writable Y.
        // Horizontal placement is *unknown*, so any nonempty width may
        // intersect those full-width row bands.
        let overflow =
            offset_x + extent_x > width || offset_y + extent_y > height;
        let first = extent_x > 0 && extent_y > 0 && offset_y < 10_000;
        let second = extent_x > 0
            && extent_y > 0
            && offset_y < 25_000
            && offset_y + extent_y > 10_000;
        let (class, expected) = if overflow {
            (
                3,
                Err(FlowFixedRegionError::FixedRegionOverflow {
                    page,
                    object: owner,
                }),
            )
        } else if first {
            (
                1,
                Err(FlowFixedRegionError::VerticalBandOverlap {
                    page,
                    object: owner,
                    fragment_owner: owner,
                    source: FragmentSource { unit: 0, fragment: 0 },
                    overlap_top: Length::from_micrometres(
                        y + offset_y,
                    ),
                    overlap_height: Length::from_micrometres(
                        (offset_y + extent_y).min(10_000)
                            - offset_y,
                    ),
                }),
            )
        } else if second {
            (
                2,
                Err(FlowFixedRegionError::VerticalBandOverlap {
                    page,
                    object: owner,
                    fragment_owner: owner,
                    source: FragmentSource { unit: 0, fragment: 1 },
                    overlap_top: Length::from_micrometres(
                        y + offset_y.max(10_000),
                    ),
                    overlap_height: Length::from_micrometres(
                        (offset_y + extent_y).min(25_000)
                            - offset_y.max(10_000),
                    ),
                }),
            )
        } else {
            (0, Ok(()))
        };
        counts[class] += 1;
        let actual = check_fixed_regions_for_flow(&plan, &[checked]);
        if actual != expected {
            return Err(format!(
                "case {case}: {actual:?} != {expected:?}"
            ));
        }
    }
    if counts.iter().any(|count| *count < 600) {
        return Err(format!("insufficient oracle coverage: {counts:?}"));
    }

    // Failure order is the order of submitted fixed evidence; an earlier
    // overflowing rectangle must not be hidden by a later row collision.
    let rectangle = |x, y, width, height| Rect {
        x: Length::from_micrometres(x),
        y: Length::from_micrometres(y),
        width: Length::from_micrometres(width),
        height: Length::from_micrometres(height),
    };
    let checked = |area| {
        validate_fixed_placement(accepted, AcceptedFixedPlacement {
            object: owner,
            page,
            rectangle: area,
            revision,
        })
        .map_err(|error| format!("order fixture: {error:?}"))
    };
    let overflow = checked(rectangle(x, y, width + 1, 1))?;
    let collide = checked(rectangle(x, y + 1, 1, 1))?;
    let first_refusal =
        FlowFixedRegionError::FixedRegionOverflow { page, object: owner };
    let second_refusal = FlowFixedRegionError::VerticalBandOverlap {
        page,
        object: owner,
        fragment_owner: owner,
        source: FragmentSource { unit: 0, fragment: 0 },
        overlap_top: Length::from_micrometres(y + 1),
        overlap_height: Length::from_micrometres(1),
    };
    if check_fixed_regions_for_flow(&plan, &[overflow.clone(), collide.clone()])
        != Err(first_refusal)
        || check_fixed_regions_for_flow(&plan, &[collide, overflow])
            != Err(second_refusal)
    {
        return Err("fixed-region refusal order was not preserved".into());
    }
    Ok(())
}

#[test]
fn indexed_fixed_clearance_matches_linear_scan_across_many_measured_rows()
-> Result<(), String> {
    const CASES: usize = 4_096;
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("indexed fixture refused".into());
    };
    let accepted = session.current().ok_or("missing accepted revision")?;
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let page = accepted_for(&mapping, fixture.page_one);
    // Distinct physical bands plus a zero-width spacer with positive height.
    // The spacer reserves a row even though it has no horizontal ink extent.
    let mut sizes = vec![(15_000, 1_000); 180];
    sizes[71].0 = 0;
    sizes[129].1 = 2_000;
    let measured = measurement(revision, flow, owner, &sizes);
    let plan = paginate_revision(accepted, &measured)
        .map_err(|error| format!("indexed pagination: {error:?}"))?;
    if plan.placements().len() != sizes.len()
        || plan.placements().iter().any(|row| row.page != page)
    {
        return Err("expected all measured rows on first page".into());
    }
    let writable = plan.covered_pages()[0].writable;
    let x = writable.x.micrometres();
    let y = writable.y.micrometres();
    let width = writable.width.micrometres();
    let height = writable.height.micrometres();
    let mut seed = 0xa83b_25d1_047e_901c;
    let mut counts = [0usize; 3]; // clear, row overlap, overflow
    let mut empty_regions = Vec::new();
    for case in 0..CASES {
        let a = next_owner_oracle_value(&mut seed);
        let b = next_owner_oracle_value(&mut seed);
        let c = next_owner_oracle_value(&mut seed);
        let d = next_owner_oracle_value(&mut seed);
        let offset_x = if case % 7 == 0 {
            width - 100
        } else {
            a % width
        };
        let offset_y = if case % 9 == 0 {
            height - 100
        } else {
            b % height
        };
        let extent_x = if case % 11 == 0 {
            0
        } else {
            c % 14_000
        };
        let extent_y = if case % 13 == 0 {
            0
        } else {
            d % 24_000
        };
        let region =
            validate_fixed_placement(accepted, AcceptedFixedPlacement {
                object: owner,
                page,
                revision,
                rectangle: Rect {
                    x: Length::from_micrometres(x + offset_x),
                    y: Length::from_micrometres(y + offset_y),
                    width: Length::from_micrometres(extent_x),
                    height: Length::from_micrometres(extent_y),
                },
            })
            .map_err(|error| format!("case {case}: {error:?}"))?;
        // Independent linear reference: inspect each original physical band
        // in order, without using the indexed lookup's search logic.
        let overflow =
            offset_x + extent_x > width || offset_y + extent_y > height;
        let overlap = if extent_x == 0 || extent_y == 0 {
            None
        } else {
            plan.placements().iter().find(|row| {
                let top = row.top.micrometres();
                row.page == page
                    && y + offset_y < top + row.height.micrometres()
                    && top < y + offset_y + extent_y
            })
        };
        let (class, expected) = if overflow {
            (
                2,
                Err(FlowFixedRegionError::FixedRegionOverflow {
                    page,
                    object: owner,
                }),
            )
        } else if let Some(row) = overlap {
            (
                1,
                Err(FlowFixedRegionError::VerticalBandOverlap {
                    page,
                    object: owner,
                    fragment_owner: row.owner,
                    source: row.source,
                    overlap_top: Length::from_micrometres(
                        (y + offset_y).max(row.top.micrometres()),
                    ),
                    overlap_height: Length::from_micrometres(
                        (y + offset_y + extent_y)
                            .min(
                                row.top.micrometres()
                                    + row.height.micrometres(),
                            )
                            - (y + offset_y).max(row.top.micrometres()),
                    ),
                }),
            )
        } else {
            (0, Ok(()))
        };
        counts[class] += 1;
        if check_fixed_regions_for_flow(&plan, &[region.clone()]) != expected {
            return Err(format!(
                "linear mismatch at case {case}: {expected:?}"
            ));
        }
        // The full-band oracle checks every row, not just the first hit.
        // Its bounds are computed with u128 physical interval arithmetic.
        let mut expected_bands = Vec::new();
        if !overflow && extent_x > 0 && extent_y > 0 {
            for row in plan.placements() {
                let start = u128::from(y + offset_y);
                let end = start + u128::from(extent_y);
                let row_start = u128::from(row.top.micrometres());
                let row_end = row_start + u128::from(row.height.micrometres());
                let shared_start = start.max(row_start);
                let shared_end = end.min(row_end);
                if row.page == page && shared_start < shared_end {
                    expected_bands.push((
                        revision, flow, page, 0, owner,
                        row.owner, row.source,
                        Length::from_micrometres(
                            u64::try_from(shared_start)
                                .map_err(|error| format!("top: {error}"))?,
                        ),
                        Length::from_micrometres(
                            u64::try_from(shared_end - shared_start)
                                .map_err(|error| format!("height: {error}"))?,
                        ),
                    ));
                }
            }
        }
        let trace = |band: FixedRowBandOverlap| {
            (
                band.revision(), band.flow(), band.page(),
                band.fixed_index(), band.object(),
                band.fragment_owner(), band.source(),
                band.overlap_top(), band.overlap_height(),
            )
        };
        let actual_bands = fixed_row_band_overlaps_for_flow(
            &plan, &[region.clone()], sizes.len(),
        ).map(|bands| bands.into_iter().map(trace).collect::<Vec<_>>());
        if overflow {
            assert_eq!(
                actual_bands,
                Err(FlowFixedRegionError::FixedRegionOverflow {
                    page, object: owner,
                }),
            );
        } else if actual_bands != Ok(expected_bands.clone()) {
            return Err(format!(
                "all row-band oracle case {case}: {actual_bands:?}"
            ));
        } else {
            assert_eq!(
                fixed_row_band_overlaps_for_flow(
                    &plan, &[region.clone()], expected_bands.len(),
                ).map(|bands| bands.len()),
                Ok(expected_bands.len()),
            );
            if !expected_bands.is_empty() {
                let limit = expected_bands.len() - 1;
                assert_eq!(
                    fixed_row_band_overlaps_for_flow(
                        &plan, &[region.clone()], limit,
                    ),
                    Err(FlowFixedRegionError::FixedRowLimitExceeded { limit }),
                );
            }
        }
        if region.is_within_bounds() && (extent_x == 0 || extent_y == 0) {
            empty_regions.push(region);
        }
    }
    if counts.iter().any(|count| *count < 100) || empty_regions.len() < 100 {
        return Err(format!("insufficient interval classes: {counts:?}"));
    }
    // Inspect a large ordered set of cleared fixed placements in one call.
    // The final row collision must retain its original source fragment.
    let first = &plan.placements()[0];
    let collision =
        validate_fixed_placement(accepted, AcceptedFixedPlacement {
            object: owner,
            page,
            revision,
            rectangle: Rect {
                x: Length::from_micrometres(x),
                y: first.top,
                width: Length::from_micrometres(1_000),
                height: Length::from_micrometres(1_000),
            },
        })
        .map_err(|error| format!("final collision: {error:?}"))?;
    empty_regions.push(collision);
    assert_eq!(
        check_fixed_regions_for_flow(&plan, &empty_regions),
        Err(FlowFixedRegionError::VerticalBandOverlap {
            page,
            object: owner,
            fragment_owner: first.owner,
            source: first.source,
            overlap_top: first.top,
            overlap_height: Length::from_micrometres(1_000),
        }),
    );
    Ok(())
}

#[test]
fn indexed_fixed_clearance_respects_covered_but_unplaced_pages()
-> Result<(), String> {
    use atrament_semantic_flow_pagination::{
        RowHorizontalPosition, fixed_row_footprint_overlaps_for_flow,
    };
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    let other_flow = ids
        .allocate_candidate()
        .map_err(|error| format!("fixed flow: {error:?}"))?;
    let other_block = ids
        .allocate_candidate()
        .map_err(|error| format!("fixed block: {error:?}"))?;
    let last_page = ids
        .allocate_candidate()
        .map_err(|error| format!("third page: {error:?}"))?;
    fixture.notebook.page_profiles[1].geometry = geometry(40_000);
    add_unmeasured_flow(&mut fixture, 1, other_flow, other_block)?;
    fixture.notebook.pages.push(Page {
        flows: vec![],
        id: last_page,
        paper_profile: fixture.profile_one,
    });
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("three-page fixed fixture refused".into());
    };
    let accepted = session.current().ok_or("missing revision")?;
    let selected = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let fixed_owner = accepted_for(&mapping, other_block);
    let first = accepted_for(&mapping, fixture.page_one);
    let skipped = accepted_for(&mapping, fixture.page_two);
    let third = accepted_for(&mapping, last_page);
    let measured = RevisionFlowMeasurement {
        revision,
        flow: selected,
        units: vec![
            MeasuredFlowUnit {
                fragments: vec![MeasuredFragment {
                    owner,
                    width: Length::from_micrometres(100_000),
                    height: Length::from_micrometres(260_000),
                }],
                policy: FlowUnitPolicy::Independent,
            },
            MeasuredFlowUnit {
                fragments: vec![MeasuredFragment {
                    owner,
                    width: Length::from_micrometres(160_000),
                    height: Length::from_micrometres(20_000),
                }],
                policy: FlowUnitPolicy::Independent,
            },
        ],
    };
    let plan = paginate_revision(accepted, &measured)
        .map_err(|error| format!("skipped layout refused: {error:?}"))?;
    assert_eq!(
        plan.covered_pages()
            .iter()
            .map(|page| page.page)
            .collect::<Vec<_>>(),
        [first, skipped, third],
    );
    assert_eq!(
        plan.placements()
            .iter()
            .map(|row| row.page)
            .collect::<Vec<_>>(),
        [first, third]
    );
    let writable = plan.covered_pages()[1].writable;
    let make = |revision: &_, page, object, rectangle| {
        validate_fixed_placement(revision, AcceptedFixedPlacement {
            object,
            page,
            revision: revision.id,
            rectangle,
        })
        .map_err(|error| format!("checked placement: {error:?}"))
    };
    let rectangle = Rect {
        x: writable.x,
        y: writable.y,
        width: Length::from_micrometres(1_000),
        height: Length::from_micrometres(1_000),
    };
    let on_skipped = make(accepted, skipped, fixed_owner, rectangle)?;
    assert_eq!(
        check_fixed_regions_for_flow(&plan, &[on_skipped.clone()]),
        Ok(())
    );

    // The skipped page is nevertheless covered and owns an exact writable
    // rectangle; a same-revision defensive mismatch must not be overlooked.
    let mut conflicting = accepted.clone();
    conflicting.notebook.page_profiles[1]
        .geometry
        .printable_region
        .width = Length::from_micrometres(185_000);
    let altered = make(&conflicting, skipped, fixed_owner, rectangle)?;
    assert_eq!(
        check_fixed_regions_for_flow(&plan, &[altered.clone()]),
        Err(FlowFixedRegionError::WritableMismatch {
            page: skipped,
            object: fixed_owner,
        }),
    );

    let first_writable = plan.covered_pages()[0].writable;
    let first_collision = make(accepted, first, owner, Rect {
        x: first_writable.x,
        y: first_writable.y,
        width: Length::from_micrometres(1_000),
        height: Length::from_micrometres(1_000),
    })?;
    // The complete diagnostic follows fixed-input order rather than
    // physical top order, while skipping a covered page without flow rows.
    let second_collision = make(accepted, first, owner, Rect {
        x: first_writable.x,
        y: Length::from_micrometres(
            first_writable.y.micrometres() + 1_000,
        ),
        width: Length::from_micrometres(1_000),
        height: Length::from_micrometres(1_000),
    })?;
    // Explicit X geometry contains only actual measured rows, not a phantom
    // row on the covered-but-unplaced page. Source identity remains intact
    // when a later row jumps from the first to the third physical page.
    let positions = plan.placements().iter().map(|row| RowHorizontalPosition {
        flow: selected,
        left: row.writable.x,
        page: row.page,
        revision,
        source: row.source,
    }).collect::<Vec<_>>();
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(
            &plan, std::slice::from_ref(&on_skipped), &positions, 0,
        ),
        Ok(vec![]),
    );
    let physical = fixed_row_footprint_overlaps_for_flow(
        &plan,
        &[
            on_skipped.clone(), second_collision.clone(),
            first_collision.clone(),
        ],
        &positions, 2,
    ).map_err(|error| format!("three-page footprints: {error:?}"))?;
    assert_eq!(
        physical.iter().map(|overlap| (
            overlap.revision(), overlap.flow(), overlap.fixed_index(),
            overlap.page(), overlap.fragment_owner(), overlap.source(),
            overlap.intersection(),
        )).collect::<Vec<_>>(),
        [
            (revision, selected, 1, first, owner,
             plan.placements()[0].source, second_collision.rectangle()),
            (revision, selected, 2, first, owner,
             plan.placements()[0].source, first_collision.rectangle()),
        ],
    );
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(
            &plan, &[first_collision.clone(), altered.clone()],
            &positions, 0,
        ),
        Err(FlowFixedRegionError::WritableMismatch {
            page: skipped, object: fixed_owner,
        }),
    );
    let mut wrong_page = positions.clone();
    wrong_page.get_mut(1).ok_or("third-page position missing")?
        .page = skipped;
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(
            &plan, std::slice::from_ref(&on_skipped), &wrong_page, 0,
        ),
        Err(FlowFixedRegionError::RowPositionMismatch {
            index: 1,
            page: third,
            fragment_owner: owner,
            source: plan.placements()[1].source,
        }),
    );
    assert_eq!(
        fixed_row_band_overlaps_for_flow(&plan, &[on_skipped.clone()], 0),
        Ok(vec![]),
    );
    let all = fixed_row_band_overlaps_for_flow(
        &plan,
        &[
            on_skipped.clone(), second_collision.clone(),
            first_collision.clone(),
        ],
        2,
    ).map_err(|error| format!("cross-page diagnostics: {error:?}"))?;
    assert_eq!(
        all.iter().map(|band| (
            band.revision(), band.flow(), band.fixed_index(),
            band.page(), band.fragment_owner(), band.source(),
            band.overlap_top(), band.overlap_height(),
        )).collect::<Vec<_>>(),
        [
            (
                revision, selected, 1, first, owner,
                plan.placements()[0].source,
                Length::from_micrometres(
                    first_writable.y.micrometres() + 1_000,
                ),
                Length::from_micrometres(1_000),
            ),
            (
                revision, selected, 2, first, owner,
                plan.placements()[0].source, first_writable.y,
                Length::from_micrometres(1_000),
            ),
        ],
    );
    assert_eq!(
        fixed_row_band_overlaps_for_flow(
            &plan,
            &[on_skipped.clone(), second_collision, first_collision.clone()],
            1,
        ),
        Err(FlowFixedRegionError::FixedRowLimitExceeded { limit: 1 }),
    );
    // The all-bands query validates the skipped page before an earlier
    // physical conflict can be returned, even with a zero output cap.
    assert_eq!(
        fixed_row_band_overlaps_for_flow(
            &plan, &[first_collision.clone(), altered.clone()], 0,
        ),
        Err(FlowFixedRegionError::WritableMismatch {
            page: skipped,
            object: fixed_owner,
        }),
    );
    assert_eq!(
        check_fixed_regions_for_flow(&plan, &[
            on_skipped,
            first_collision.clone(),
        ]),
        Err(FlowFixedRegionError::VerticalBandOverlap {
            page: first,
            object: owner,
            fragment_owner: owner,
            source: atrament_flow_pagination::FragmentSource {
                unit: 0,
                fragment: 0,
            },
            overlap_top: first_writable.y,
            overlap_height: Length::from_micrometres(1_000),
        }),
    );
    assert_eq!(
        check_fixed_regions_for_flow(&plan, &[altered, first_collision]),
        Err(FlowFixedRegionError::WritableMismatch {
            page: skipped,
            object: fixed_owner,
        }),
    );
    assert_eq!(session.current(), Some(accepted));
    Ok(())
}

#[test]
fn indexed_fixed_clearance_retains_owner_and_unit_fragment_provenance()
-> Result<(), String> {
    use atrament_flow_pagination::FragmentSource;

    let ids = IdentityAllocator::new();
    let (fixture, second_candidate) = candidate_two_block_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("two-block fixture refused".into());
    };
    let accepted = session.current().ok_or("missing revision")?;
    let flow = accepted_for(&mapping, fixture.flow);
    let first = accepted_for(&mapping, fixture.block);
    let second = accepted_for(&mapping, second_candidate);
    let page = accepted_for(&mapping, fixture.page_one);
    let fragment = |owner, width, height| MeasuredFragment {
        owner,
        width: Length::from_micrometres(width),
        height: Length::from_micrometres(height),
    };
    let measured = RevisionFlowMeasurement {
        flow,
        revision,
        units: vec![
            MeasuredFlowUnit {
                fragments: vec![fragment(first, 10_000, 20_000)],
                policy: FlowUnitPolicy::Independent,
            },
            MeasuredFlowUnit {
                fragments: vec![
                    fragment(second, 10_000, 10_000),
                    fragment(second, 0, 15_000),
                ],
                policy: FlowUnitPolicy::KeepTogetherWhenPossible,
            },
            MeasuredFlowUnit {
                fragments: vec![fragment(second, 10_000, 5_000)],
                policy: FlowUnitPolicy::Independent,
            },
        ],
    };
    let plan = paginate_revision(accepted, &measured)
        .map_err(|error| format!("mixed-unit layout: {error:?}"))?;
    let expected = [
        (first, FragmentSource { unit: 0, fragment: 0 }),
        (second, FragmentSource { unit: 1, fragment: 0 }),
        (second, FragmentSource { unit: 1, fragment: 1 }),
        (second, FragmentSource { unit: 2, fragment: 0 }),
    ];
    assert_eq!(plan.placements().len(), expected.len());
    assert_eq!(
        plan.placements()
            .iter()
            .map(|row| (row.owner, row.source))
            .collect::<Vec<_>>(),
        expected
    );
    let rows = plan.placements();
    let writable = rows[0].writable;
    let checked = |top: u64, height: u64| {
        validate_fixed_placement(accepted, AcceptedFixedPlacement {
            page,
            object: second,
            revision,
            rectangle: Rect {
                x: writable.x,
                y: Length::from_micrometres(top),
                width: Length::from_micrometres(1_000),
                height: Length::from_micrometres(height),
            },
        })
        .map_err(|error| format!("fixed placement: {error:?}"))
    };
    for (row, (owner, source)) in rows.iter().zip(expected) {
        assert_eq!(
            check_fixed_regions_for_flow(&plan, &[checked(
                row.top.micrometres(),
                1
            )?,]),
            Err(FlowFixedRegionError::VerticalBandOverlap {
                page,
                object: second,
                fragment_owner: owner,
                source,
                overlap_top: row.top,
                overlap_height: Length::from_micrometres(1),
            }),
        );
    }
    // The first crossing row wins even across keep-together fragments;
    // touching the end of the final row does not collide.
    let crossing = rows[2]
        .top
        .micrometres()
        .checked_sub(1)
        .ok_or("preceding row underflow")?;
    assert_eq!(
        check_fixed_regions_for_flow(&plan, &[checked(crossing, 2)?]),
        Err(FlowFixedRegionError::VerticalBandOverlap {
            page,
            object: second,
            fragment_owner: second,
            source: FragmentSource { unit: 1, fragment: 0 },
            overlap_top: Length::from_micrometres(crossing),
            overlap_height: Length::from_micrometres(1),
        }),
    );
    let two_rows = checked(crossing, 2)?;
    let all = fixed_row_band_overlaps_for_flow(&plan, &[two_rows], 2)
        .map_err(|error| format!("crossing query: {error:?}"))?;
    assert_eq!(all.len(), 2);
    assert_eq!(
        all.iter()
            .map(|band| (
                band.revision(), band.flow(), band.fixed_index(),
                band.fragment_owner(), band.source(),
                band.overlap_top(), band.overlap_height(),
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                revision, flow, 0, second,
                FragmentSource { unit: 1, fragment: 0 },
                Length::from_micrometres(crossing),
                Length::from_micrometres(1),
            ),
            (
                revision, flow, 0, second,
                FragmentSource { unit: 1, fragment: 1 },
                Length::from_micrometres(crossing + 1),
                Length::from_micrometres(1),
            ),
        ],
    );
    assert_eq!(
        fixed_row_band_overlaps_for_flow(&plan, &[checked(crossing, 2)?], 1),
        Err(FlowFixedRegionError::FixedRowLimitExceeded { limit: 1 }),
    );
    let final_end = rows[3].top.micrometres() + rows[3].height.micrometres();
    assert_eq!(
        check_fixed_regions_for_flow(&plan, &[checked(final_end, 1)?]),
        Ok(()),
    );
    assert_eq!(session.current(), Some(accepted));
    Ok(())
}

#[test]
fn checked_fixed_pair_inspection_preserves_layering_and_source_order()
-> Result<(), String> {
    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    let fixed_flow = ids
        .allocate_candidate()
        .map_err(|error| format!("id: {error:?}"))?;
    let owners = (0..3)
        .map(|_| {
            ids.allocate_candidate()
                .map_err(|error| format!("id: {error:?}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    fixture.notebook.pages[1].flows.push(Flow {
        id: fixed_flow,
        blocks: owners
            .iter()
            .map(|&id| Block {
                id,
                content: BlockContent::Rule,
                extensions: vec![],
                provenance: None,
                style: None,
            })
            .collect(),
    });
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("fixed-pair fixture refused".into());
    };
    let accepted = session.current().ok_or("missing revision")?;
    let page_one = accepted_for(&mapping, fixture.page_one);
    let page_two = accepted_for(&mapping, fixture.page_two);
    let block = accepted_for(&mapping, fixture.block);
    let objects = owners
        .into_iter()
        .map(|owner| accepted_for(&mapping, owner))
        .collect::<Vec<_>>();
    let flow = accepted_for(&mapping, fixture.flow);
    let measured = measurement(revision, flow, block, &[(10_000, 10_000)]);
    let plan = paginate_revision(accepted, &measured)
        .map_err(|error| format!("pagination: {error:?}"))?;
    // Page two has independent fixed objects but no measured flow rows.
    if plan.placements().len() != 1 || plan.covered_pages().len() != 1 {
        return Err("unexpected physical row placement".into());
    }
    let checked = |page, object, x, y, width, height| {
        validate_fixed_placement(accepted, AcceptedFixedPlacement {
            object,
            page,
            revision,
            rectangle: Rect {
                x: Length::from_micrometres(x),
                y: Length::from_micrometres(y),
                width: Length::from_micrometres(width),
                height: Length::from_micrometres(height),
            },
        })
        .map_err(|error| format!("fixed admission: {error:?}"))
    };
    let trace = |pair: FixedPairOverlap| {
        (
            pair.revision(), pair.flow(), pair.page(),
            pair.earlier_index(), pair.earlier_object(),
            pair.later_index(), pair.later_object(), pair.intersection(),
        )
    };
    let a = checked(page_two, objects[0], 40_000, 50_000, 10_000, 10_000)?;
    let touch = checked(page_two, objects[1], 50_000, 50_000, 10_000, 10_000)?;
    let c = checked(page_two, objects[2], 49_999, 59_999, 5_000, 5_000)?;
    let zero = checked(page_two, objects[1], 40_000, 50_000, 0, 10_000)?;
    let other_page = checked(page_one, block, 40_000, 50_000, 10_000, 10_000)?;
    assert_eq!(
        first_fixed_pair_overlap_for_flow(&plan, &[
            a.clone(),
            touch.clone(),
            zero.clone(),
            other_page,
        ]),
        Ok(None),
    );
    assert_eq!(
        first_fixed_pair_overlap_for_flow(&plan, &[
            a.clone(),
            touch.clone(),
            c.clone(),
            zero,
        ]).map(|result| result.map(trace)),
        Ok(Some((
            revision, flow, page_two, 0, objects[0], 2, objects[2],
            Rect {
                x: Length::from_micrometres(49_999),
                y: Length::from_micrometres(59_999),
                width: Length::from_micrometres(1),
                height: Length::from_micrometres(1),
            },
        ))),
    );
    // A complete bounded observation preserves both overlaps at the corner,
    // including the pair after the existing first-only short circuit.
    let corners = [a.clone(), touch.clone(), c.clone()];
    assert_eq!(
        fixed_pair_overlaps_for_flow(&plan, &corners, 2)
            .map(|pairs| pairs.into_iter().map(trace).collect::<Vec<_>>()),
        Ok(vec![
            (
                revision, flow, page_two, 0, objects[0], 2, objects[2],
                Rect {
                    x: Length::from_micrometres(49_999),
                    y: Length::from_micrometres(59_999),
                    width: Length::from_micrometres(1),
                    height: Length::from_micrometres(1),
                },
            ),
            (
                revision, flow, page_two, 1, objects[1], 2, objects[2],
                Rect {
                    x: Length::from_micrometres(50_000),
                    y: Length::from_micrometres(59_999),
                    width: Length::from_micrometres(4_999),
                    height: Length::from_micrometres(1),
                },
            ),
        ]),
    );
    assert_eq!(
        fixed_pair_overlaps_for_flow(&plan, &corners, 1),
        Err(FlowFixedRegionError::FixedPairLimitExceeded { limit: 1 }),
    );
    assert_eq!(
        fixed_pair_overlaps_for_flow(&plan, &[a.clone(), touch.clone()], 0),
        Ok(vec![]),
    );
    // A geometric pair is not a blocking collision without overlap policy.
    assert_eq!(
        check_fixed_regions_for_flow(&plan, &[a.clone(), c.clone()]),
        Ok(())
    );
    assert_eq!(
        first_fixed_pair_overlap_for_flow(&plan, &[touch, c.clone(), a.clone()])
            .map(|result| result.map(trace)),
        Ok(Some((
            revision, flow, page_two, 0, objects[1], 1, objects[2],
            Rect {
                x: Length::from_micrometres(50_000),
                y: Length::from_micrometres(59_999),
                width: Length::from_micrometres(4_999),
                height: Length::from_micrometres(1),
            },
        ))),
    );
    // Invalid evidence takes priority over a pair found earlier in the batch.
    let invalid = checked(page_two, objects[1], 0, 0, 10_000, 10_000)?;
    assert_eq!(
        first_fixed_pair_overlap_for_flow(
            &plan, &[a.clone(), a.clone(), invalid.clone()],
        ),
        Err(FlowFixedRegionError::FixedRegionOverflow {
            page: page_two,
            object: objects[1],
        }),
    );
    assert_eq!(
        fixed_pair_overlaps_for_flow(
            &plan, &[a.clone(), c.clone(), invalid], 0
        ),
        Err(FlowFixedRegionError::FixedRegionOverflow {
            page: page_two,
            object: objects[1],
        }),
    );
    let row_top = plan.placements()[0].top.micrometres();
    let collision =
        checked(page_one, block, 40_000, row_top + 1, 1_000, 1_000)?;
    let overlapping =
        checked(page_two, objects[0], 40_000, 50_000, 10_000, 10_000)?;
    assert!(matches!(
        first_fixed_pair_overlap_for_flow(&plan, &[overlapping, collision]),
        Err(FlowFixedRegionError::VerticalBandOverlap { .. }),
    ));
    // Compare 1,024 independently generated multi-page rectangle batches
    // against a u128 physical-intersection oracle, including empty boxes.
    let mut seed = 0x179a_2026_1080_4495_u64;
    let mut overlap_cases = 0;
    let mut clear_cases = 0;
    for case in 0..1_024 {
        let mut regions = Vec::new();
        let mut geometry = Vec::new();
        for index in 0..8 {
            let mut draw = || {
                seed = seed
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                seed >> 32
            };
            let is_first_page = draw() % 2 == 0;
            let (physical_page, owner) = if is_first_page {
                (page_one, block)
            } else {
                (page_two, objects[index % objects.len()])
            };
            let x = 40_000 + draw() % 100_000;
            let y = 80_000 + draw() % 140_000;
            let width = if case % 11 == 0 {
                0
            } else {
                draw() % 20_000
            };
            let height = if case % 13 == 0 {
                0
            } else {
                draw() % 20_000
            };
            regions.push(checked(physical_page, owner, x, y, width, height)?);
            geometry.push((physical_page, owner, x, y, width, height));
        }
        let mut expected = Vec::new();
        for later_index in 0..geometry.len() {
            let (page, later_object, x, y, w, h) = geometry[later_index];
            for earlier_index in 0..later_index {
                let (earlier_page, earlier_object, ex, ey, ew, eh) =
                    geometry[earlier_index];
                let common_width = (u128::from(x) + u128::from(w))
                    .min(u128::from(ex) + u128::from(ew))
                    .saturating_sub(u128::from(x.max(ex)));
                let common_height = (u128::from(y) + u128::from(h))
                    .min(u128::from(ey) + u128::from(eh))
                    .saturating_sub(u128::from(y.max(ey)));
                if page == earlier_page
                    && w > 0
                    && h > 0
                    && ew > 0
                    && eh > 0
                    && common_width > 0
                    && common_height > 0
                {
                    expected.push((
                        revision, flow, page,
                        earlier_index, earlier_object,
                        later_index, later_object,
                        Rect {
                            x: Length::from_micrometres(x.max(ex)),
                            y: Length::from_micrometres(y.max(ey)),
                            width: Length::from_micrometres(
                                u64::try_from(common_width)
                                    .map_err(|error| {
                                        format!("oracle width: {error}")
                                    })?,
                            ),
                            height: Length::from_micrometres(
                                u64::try_from(common_height)
                                    .map_err(|error| {
                                        format!("oracle height: {error}")
                                    })?,
                            ),
                        },
                    ));
                }
            }
        }
        let actual = first_fixed_pair_overlap_for_flow(&plan, &regions)
            .map_err(|error| format!("case {case} refused: {error:?}"))?
            .map(trace);
        if actual != expected.first().copied() {
            return Err(format!(
                "first pair oracle case {case}: {actual:?} != {expected:?}"
            ));
        }
        let all = fixed_pair_overlaps_for_flow(&plan, &regions, 28)
            .map_err(|error| format!("all pairs case {case}: {error:?}"))?
            .into_iter().map(trace).collect::<Vec<_>>();
        if all != expected {
            return Err(format!(
                "all pairs oracle case {case}: {all:?} != {expected:?}"
            ));
        }
        assert_eq!(
            fixed_pair_overlaps_for_flow(&plan, &regions, expected.len())
                .map(|pairs| pairs.len()),
            Ok(expected.len()),
        );
        if !expected.is_empty() {
            let limit = expected.len() - 1;
            assert_eq!(
                fixed_pair_overlaps_for_flow(&plan, &regions, limit),
                Err(FlowFixedRegionError::FixedPairLimitExceeded { limit }),
            );
        }
        if actual.is_some() {
            overlap_cases += 1;
        } else {
            clear_cases += 1;
        }
    }
    if overlap_cases < 100 || clear_cases < 100 {
        return Err(format!(
            "pair coverage: yes={overlap_cases}, no={clear_cases}"
        ));
    }
    assert_eq!(session.current(), Some(accepted));
    Ok(())
}

#[test]
fn explicitly_positioned_row_footprints_match_independent_physical_oracle()
-> Result<(), String> {
    use atrament_flow_pagination::FragmentSource;
    use atrament_semantic_flow_pagination::{
        FixedRowFootprintOverlap, RowHorizontalPosition,
        first_fixed_row_footprint_overlap_for_flow,
        fixed_row_footprint_overlaps_for_flow,
    };

    const CASES: usize = 2_048;
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("positioned-row fixture refused".into());
    };
    let accepted = session.current().ok_or("missing accepted revision")?;
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let page = accepted_for(&mapping, fixture.page_one);
    let measured = measurement(revision, flow, owner, &[
        (15_000, 10_000), (0, 10_000), (20_000, 10_000),
    ]);
    let plan = paginate_revision(accepted, &measured)
        .map_err(|error| format!("positioned rows: {error:?}"))?;
    let region = plan.covered_pages().first()
        .ok_or("missing covered page")?.writable;
    let x = region.x.micrometres();
    let y = region.y.micrometres();
    let width = region.width.micrometres();
    let positions_at = |offsets: [u64; 3]| {
        plan.placements().iter().zip(offsets)
            .map(|(row, offset)| RowHorizontalPosition {
                flow,
                revision,
                page: row.page,
                source: row.source,
                left: Length::from_micrometres(x + offset),
            }).collect::<Vec<_>>()
    };
    // An empty physical row may sit on the last writable micrometre.
    // Its zero-width footprint must never claim horizontal ink.
    let positions = positions_at([5_000, width, 25_000]);
    let trace = |overlap: FixedRowFootprintOverlap| {
        (
            overlap.revision(), overlap.flow(), overlap.page(),
            overlap.fixed_index(), overlap.object(), overlap.fragment_owner(),
            overlap.source(), overlap.intersection(),
        )
    };
    let checked = |rectangle| {
        validate_fixed_placement(accepted, AcceptedFixedPlacement {
            object: owner, page, revision, rectangle,
        }).map_err(|error| format!("checked fixed box: {error:?}"))
    };
    let mut seed = 0x28bc_5a13_191d_d73f;
    let mut clear = 0;
    let mut intersects = 0;
    let mut overflows = 0;
    for case in 0..CASES {
        let a = next_owner_oracle_value(&mut seed);
        let b = next_owner_oracle_value(&mut seed);
        let c = next_owner_oracle_value(&mut seed);
        let d = next_owner_oracle_value(&mut seed);
        let dx = if case % 13 == 0 { width - 500 } else { a % width };
        let dy = if case % 11 == 0 { 10_000 } else { b % 35_000 };
        let w = if case % 7 == 0 { 0 } else { c % 30_000 };
        let h = if case % 9 == 0 { 0 } else { d % 25_000 };
        let fixed = checked(Rect {
            x: Length::from_micrometres(x + dx),
            y: Length::from_micrometres(y + dy),
            width: Length::from_micrometres(w),
            height: Length::from_micrometres(h),
        })?;
        let row_zero_x = match case % 31 {
            0 => 0,
            1 => width - 15_000,
            _ => next_owner_oracle_value(&mut seed) % (width - 15_000 + 1),
        };
        let row_two_x = match case % 29 {
            0 => 0,
            1 => width - 20_000,
            _ => next_owner_oracle_value(&mut seed) % (width - 20_000 + 1),
        };
        let case_positions = positions_at([row_zero_x, width, row_two_x]);
        let actual = fixed_row_footprint_overlaps_for_flow(
            &plan, std::slice::from_ref(&fixed), &case_positions, 3,
        ).map(|all| all.into_iter().map(trace).collect::<Vec<_>>());
        let first = first_fixed_row_footprint_overlap_for_flow(
            &plan, std::slice::from_ref(&fixed), &case_positions,
        ).map(|item| item.map(trace));
        // Every row type, including the zero-width middle row, refuses an
        // origin one micrometre beyond its admitted rightmost position.
        // Invalid fixed geometry must refuse first, even with a zero cap.
        let bad_index = case % 3;
        let bad_offset = match bad_index {
            0 => width - 14_999,
            1 => width + 1,
            _ => width - 19_999,
        };
        let mut bad_positions = case_positions.clone();
        bad_positions.get_mut(bad_index)
            .ok_or("oracle missing measured row")?
            .left = Length::from_micrometres(x + bad_offset);
        let bad_result = fixed_row_footprint_overlaps_for_flow(
            &plan, std::slice::from_ref(&fixed), &bad_positions, 0,
        );
        let expected_bad = if dx + w > width {
            FlowFixedRegionError::FixedRegionOverflow {
                page, object: owner,
            }
        } else {
            FlowFixedRegionError::RowFootprintOverflow {
                page, fragment_owner: owner,
                source: plan.placements()[bad_index].source,
            }
        };
        if bad_result != Err(expected_bad) {
            return Err(format!(
                "case {case} invalid X: {bad_result:?} != {expected_bad:?}"
            ));
        }
        assert_eq!(
            first_fixed_row_footprint_overlap_for_flow(
                &plan, std::slice::from_ref(&fixed), &bad_positions,
            ),
            Err(expected_bad),
            "first-hit case {case} must reject an invalid complete batch",
        );
        if dx + w > width {
            overflows += 1;
            if actual != Err(FlowFixedRegionError::FixedRegionOverflow {
                page, object: owner,
            }) {
                return Err(format!("case {case} overflow: {actual:?}"));
            }
            assert_eq!(
                first,
                Err(FlowFixedRegionError::FixedRegionOverflow {
                    page, object: owner,
                }),
            );
            continue;
        }
        // This reference uses independent u128 rectangle arithmetic and
        // literal measured widths/top offsets, not the row-index visitor.
        let mut expected = Vec::new();
        for (row_index, (rx, ry, rw)) in [
            (row_zero_x, 0u64, 15_000u64),
            (width, 10_000, 0),
            (row_two_x, 20_000, 20_000),
        ].into_iter().enumerate() {
            let left = u128::from(dx).max(u128::from(rx));
            let right = u128::from(dx + w).min(u128::from(rx + rw));
            let top = u128::from(dy).max(u128::from(ry));
            let bottom = u128::from(dy + h).min(u128::from(ry + 10_000));
            if left < right && top < bottom {
                expected.push((
                    revision, flow, page, 0, owner, owner,
                    FragmentSource { unit: 0, fragment: row_index },
                    Rect {
                        x: Length::from_micrometres(x +
                            u64::try_from(left).map_err(|e| e.to_string())?),
                        y: Length::from_micrometres(y +
                            u64::try_from(top).map_err(|e| e.to_string())?),
                        width: Length::from_micrometres(
                            u64::try_from(right - left)
                                .map_err(|e| e.to_string())?,
                        ),
                        height: Length::from_micrometres(
                            u64::try_from(bottom - top)
                                .map_err(|e| e.to_string())?,
                        ),
                    },
                ));
            }
        }
        if expected.is_empty() { clear += 1; } else { intersects += 1; }
        if actual != Ok(expected.clone()) {
            return Err(format!(
                "case {case} footprint: {actual:?} != {expected:?}"
            ));
        }
        assert_eq!(
            first, Ok(expected.first().copied()),
            "case {case}: first intersecting row must match linear oracle",
        );
        let cap = expected.len().saturating_sub(1);
        if !expected.is_empty()
            && fixed_row_footprint_overlaps_for_flow(
                &plan, &[fixed], &case_positions, cap,
            ) != Err(FlowFixedRegionError::FixedRowFootprintLimitExceeded {
                limit: cap,
            })
        {
            return Err(format!("case {case} truncated instead of refusing"));
        }
    }
    if clear < 100 || intersects < 100 || overflows < 100 {
        return Err(format!(
            "incomplete footprint classes: {clear}/{intersects}/{overflows}"
        ));
    }

    let positioned = checked(Rect {
        x: Length::from_micrometres(x + 10_000),
        y: Length::from_micrometres(y + 5_000),
        width: Length::from_micrometres(20_000),
        height: Length::from_micrometres(30_000),
    })?;
    // Reordering two same-page proposals does not change their identities.
    // The exact source sequence still has to match the sealed plan.
    let mut reordered = positions.clone();
    reordered.swap(0, 2);
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(
            &plan, &[], &reordered, 0,
        ),
        Err(FlowFixedRegionError::RowPositionMismatch {
            index: 0, page, fragment_owner: owner,
            source: plan.placements()[0].source,
        }),
    );
    // A late invalid row refuses the entire batch before even a zero-cap
    // report can return its earlier, already detected intersection.
    let mut late_overflow = positions.clone();
    late_overflow.get_mut(2).ok_or("last row position missing")?
        .left = Length::from_micrometres(x + width - 19_999);
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(
            &plan, std::slice::from_ref(&positioned), &late_overflow, 0,
        ),
        Err(FlowFixedRegionError::RowFootprintOverflow {
            page, fragment_owner: owner,
            source: plan.placements()[2].source,
        }),
    );
    assert_eq!(
        first_fixed_row_footprint_overlap_for_flow(
            &plan, std::slice::from_ref(&positioned), &late_overflow,
        ),
        Err(FlowFixedRegionError::RowFootprintOverflow {
            page, fragment_owner: owner,
            source: plan.placements()[2].source,
        }),
    );
    let report = fixed_row_footprint_overlaps_for_flow(
        &plan, &[positioned.clone(), positioned.clone()], &positions, 4,
    ).map_err(|error| format!("ordered footprint report: {error:?}"))?;
    assert_eq!(
        first_fixed_row_footprint_overlap_for_flow(
            &plan, &[positioned.clone(), positioned], &positions,
        ),
        Ok(report.first().copied()),
    );
    assert_eq!(
        report.iter().map(|overlap| (
            overlap.fixed_index(), overlap.source().fragment,
        )).collect::<Vec<_>>(),
        vec![(0, 0), (0, 2), (1, 0), (1, 2)],
    );
    Ok(())
}

#[test]
fn indexed_positioned_footprints_match_linear_oracle_across_pages()
-> Result<(), String> {
    use atrament_semantic_flow_pagination::{
        FixedRowFootprintOverlap, RowHorizontalPosition,
        first_fixed_row_footprint_overlap_for_flow,
        fixed_row_footprint_overlaps_for_flow,
    };

    const CASES: usize = 2_048;
    let ids = IdentityAllocator::new();
    let fixture = candidate_fixture(&ids);
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("multi-page footprint fixture refused".into());
    };
    let accepted = session.current().ok_or("missing accepted revision")?;
    let flow = accepted_for(&mapping, fixture.flow);
    let owner = accepted_for(&mapping, fixture.block);
    let mut sizes = vec![(15_000, 1_000); 420];
    for index in [71, 300] {
        sizes.get_mut(index).ok_or("zero-width row index missing")?.0 = 0;
    }
    let plan = paginate_revision(
        accepted, &measurement(revision, flow, owner, &sizes),
    ).map_err(|error| format!("many positioned rows: {error:?}"))?;
    let [origin_region, spill_region] = plan.covered_pages() else {
        return Err("expected exactly two covered pages".into());
    };
    if plan.placements().len() != sizes.len()
        || !plan.placements().iter()
            .any(|row| row.page == spill_region.page)
    {
        return Err("expected measured rows to span both pages".into());
    }
    let positions = plan.placements().iter().enumerate()
        .map(|(index, row)| {
            let writable_width = row.writable.width.micrometres();
            let free = writable_width - row.width.micrometres();
            let offset = if row.width == Length::ZERO {
                free
            } else {
                u64::try_from(index).map_err(|error| error.to_string())?
                    * 7_919 % (free + 1)
            };
            Ok(RowHorizontalPosition {
                flow, revision, page: row.page, source: row.source,
                left: Length::from_micrometres(
                    row.writable.x.micrometres() + offset,
                ),
            })
        }).collect::<Result<Vec<_>, String>>()?;
    let trace = |overlap: FixedRowFootprintOverlap| {
        (
            overlap.revision(), overlap.flow(), overlap.page(),
            overlap.fixed_index(), overlap.object(), overlap.fragment_owner(),
            overlap.source(), overlap.intersection(),
        )
    };
    let mut seed = 0x431a_203e_f576_7701;
    let mut classes = [0usize; 3]; // clear, intersects, fixed overflow
    for case in 0..CASES {
        // Only the origin page owns this checked block. The independent
        // spill page has measured rows but no fixed owner to forge.
        let region = *origin_region;
        let writable = region.writable;
        let physical_left = writable.x.micrometres();
        let physical_top = writable.y.micrometres();
        let page_width = writable.width.micrometres();
        let page_height = writable.height.micrometres();
        let random_x = next_owner_oracle_value(&mut seed);
        let random_y = next_owner_oracle_value(&mut seed);
        let random_width = next_owner_oracle_value(&mut seed);
        let random_height = next_owner_oracle_value(&mut seed);
        let offset_x = if case % 17 == 0 {
            page_width - 100
        } else {
            random_x % page_width
        };
        let offset_y = if case % 19 == 0 {
            page_height - 100
        } else {
            random_y % page_height
        };
        let box_width = if case % 11 == 0 {
            0
        } else {
            random_width % 14_000
        };
        let box_height = if case % 13 == 0 {
            0
        } else {
            random_height % 24_000
        };
        let fixed = validate_fixed_placement(accepted, AcceptedFixedPlacement {
            object: owner,
            page: region.page,
            revision,
            rectangle: Rect {
                x: Length::from_micrometres(physical_left + offset_x),
                y: Length::from_micrometres(physical_top + offset_y),
                width: Length::from_micrometres(box_width),
                height: Length::from_micrometres(box_height),
            },
        }).map_err(|error| format!("case {case}: {error:?}"))?;
        let actual = fixed_row_footprint_overlaps_for_flow(
            &plan, std::slice::from_ref(&fixed), &positions, sizes.len(),
        ).map(|all| all.into_iter().map(trace).collect::<Vec<_>>());
        let first = first_fixed_row_footprint_overlap_for_flow(
            &plan, std::slice::from_ref(&fixed), &positions,
        ).map(|result| result.map(trace));
        if offset_x + box_width > page_width
            || offset_y + box_height > page_height
        {
            classes[2] += 1;
            assert_eq!(
                actual,
                Err(FlowFixedRegionError::FixedRegionOverflow {
                    page: region.page, object: owner,
                }),
            );
            assert_eq!(
                first,
                Err(FlowFixedRegionError::FixedRegionOverflow {
                    page: region.page, object: owner,
                }),
            );
            continue;
        }
        // Linear physical oracle deliberately traverses every placed row,
        // including the other page; it never uses the indexed row visitor.
        let mut expected = Vec::new();
        for (row, position) in plan.placements().iter().zip(&positions) {
            let left = u128::from(physical_left + offset_x)
                .max(u128::from(position.left.micrometres()));
            let right = u128::from(physical_left + offset_x + box_width).min(
                u128::from(position.left.micrometres())
                    + u128::from(row.width.micrometres()),
            );
            let top = u128::from(physical_top + offset_y)
                .max(u128::from(row.top.micrometres()));
            let bottom = u128::from(physical_top + offset_y + box_height).min(
                u128::from(row.top.micrometres())
                    + u128::from(row.height.micrometres()),
            );
            if row.page != region.page || left >= right || top >= bottom {
                continue;
            }
            expected.push((
                revision, flow, region.page, 0, owner, row.owner, row.source,
                Rect {
                    x: Length::from_micrometres(
                        u64::try_from(left).map_err(|e| e.to_string())?,
                    ),
                    y: Length::from_micrometres(
                        u64::try_from(top).map_err(|e| e.to_string())?,
                    ),
                    width: Length::from_micrometres(
                        u64::try_from(right - left).map_err(|e| e.to_string())?,
                    ),
                    height: Length::from_micrometres(
                        u64::try_from(bottom - top).map_err(|e| e.to_string())?,
                    ),
                },
            ));
        }
        classes[usize::from(!expected.is_empty())] += 1;
        if actual != Ok(expected.clone()) {
            return Err(format!(
                "case {case} indexed footprints: {actual:?} != {expected:?}"
            ));
        }
        assert_eq!(
            first, Ok(expected.first().copied()),
            "case {case}: first intersection must ignore other pages",
        );
        if !expected.is_empty() {
            let limit = expected.len() - 1;
            assert_eq!(
                fixed_row_footprint_overlaps_for_flow(
                    &plan, &[fixed], &positions, limit,
                ),
                Err(FlowFixedRegionError::FixedRowFootprintLimitExceeded {
                    limit,
                }),
            );
        }
    }
    if classes.iter().any(|count| *count < 100) {
        return Err(format!("incomplete multi-page footprints: {classes:?}"));
    }
    assert_eq!(session.current(), Some(accepted));
    Ok(())
}

#[test]
fn positioned_row_footprints_refuse_stale_missing_and_unbounded_proposals()
-> Result<(), String> {
    use atrament_semantic_flow_pagination::{
        RowHorizontalPosition, first_fixed_row_footprint_overlap_for_flow,
        fixed_row_footprint_overlaps_for_flow,
    };

    let ids = IdentityAllocator::new();
    let mut fixture = candidate_fixture(&ids);
    let other_flow_candidate = ids.allocate_candidate()
        .map_err(|error| format!("other flow identity: {error:?}"))?;
    fixture.notebook.pages.get_mut(1).ok_or("second page missing")?
        .flows.push(Flow {
            blocks: vec![],
            id: other_flow_candidate,
        });
    let mut session = SemanticNotebookSessionService::default();
    let AcceptanceOutcome::Accepted { mapping, revision } =
        session.accept(fixture.notebook)
    else {
        return Err("positioned-row refusal fixture refused".into());
    };
    let accepted = session.current().ok_or("missing accepted revision")?;
    let flow = accepted_for(&mapping, fixture.flow);
    let other_flow = accepted_for(&mapping, other_flow_candidate);
    let owner = accepted_for(&mapping, fixture.block);
    let page = accepted_for(&mapping, fixture.page_one);
    let plan = paginate_revision(
        accepted, &measurement(revision, flow, owner, &[(20_000, 10_000)]),
    ).map_err(|error| format!("reference measured row: {error:?}"))?;
    let row = *plan.placements().first().ok_or("missing placed row")?;
    let x = row.writable.x.micrometres();
    let y = row.top.micrometres();
    let w = row.writable.width.micrometres();
    let check = |rectangle| {
        validate_fixed_placement(accepted, AcceptedFixedPlacement {
            object: owner, page, revision, rectangle,
        }).map_err(|error| format!("checked fixed box: {error:?}"))
    };
    let area = |dx, dy, width, height| Rect {
        x: Length::from_micrometres(x + dx),
        y: Length::from_micrometres(y + dy),
        width: Length::from_micrometres(width),
        height: Length::from_micrometres(height),
    };
    let near_far_edge = check(area(w - 1_000, 0, 1_000, 1_000))?;
    let overlaps = check(area(15_000, 5_000, 10_000, 2_000))?;
    let invalid = check(area(w - 500, 0, 1_000, 1_000))?;
    let position = RowHorizontalPosition {
        flow, revision, page, source: row.source,
        left: Length::from_micrometres(x),
    };
    assert!(check_fixed_regions_for_flow(&plan, &[near_far_edge.clone()])
        .is_err());
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(
            &plan, std::slice::from_ref(&near_far_edge), &[position], 0,
        ),
        Ok(vec![]),
    );
    // Touching the row's horizontal edge is not an intersection; placing
    // the row's entire width flush with the writable right edge is valid.
    let touching = check(area(20_000, 0, 1_000, 1_000))?;
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(
            &plan, &[touching], &[position], 0,
        ),
        Ok(vec![]),
    );
    let flush_right = RowHorizontalPosition {
        left: Length::from_micrometres(x + w - 20_000),
        ..position
    };
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(
            &plan, &[near_far_edge.clone()], &[flush_right], 1,
        ).map(|all| all.first().map(|overlap| overlap.intersection())),
        Ok(Some(near_far_edge.rectangle())),
    );
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(
            &plan, std::slice::from_ref(&overlaps), &[position], 1,
        ).map(|report| report.first().map(|item| item.intersection())),
        Ok(Some(area(15_000, 5_000, 5_000, 2_000))),
    );
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(
            &plan, std::slice::from_ref(&overlaps), &[position], 0,
        ),
        Err(FlowFixedRegionError::FixedRowFootprintLimitExceeded { limit: 0 }),
    );
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(&plan, &[], &[], 0),
        Err(FlowFixedRegionError::RowPositionCountMismatch {
            expected: 1, supplied: 0,
        }),
    );
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(&plan, &[], &[
            RowHorizontalPosition { flow: other_flow, ..position }
        ], 0),
        Err(FlowFixedRegionError::RowPositionMismatch {
            index: 0, page, fragment_owner: owner, source: row.source,
        }),
    );
    let wrong = RowHorizontalPosition {
        page: accepted_for(&mapping, fixture.page_two), ..position
    };
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(&plan, &[], &[wrong], 0),
        Err(FlowFixedRegionError::RowPositionMismatch {
            index: 0, page, fragment_owner: owner, source: row.source,
        }),
    );
    let other_revision = loop {
        let id = ids.allocate_revision()
            .map_err(|error| format!("revision: {error:?}"))?;
        if id != revision { break id; }
    };
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(&plan, &[], &[
            RowHorizontalPosition { revision: other_revision, ..position }
        ], 0),
        Err(FlowFixedRegionError::RowPositionMismatch {
            index: 0, page, fragment_owner: owner, source: row.source,
        }),
    );
    let before_writable = RowHorizontalPosition {
        left: Length::ZERO, ..position
    };
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(
            &plan, &[], &[before_writable], 0,
        ),
        Err(FlowFixedRegionError::RowFootprintOverflow {
            page, fragment_owner: owner, source: row.source,
        }),
    );
    let outside = RowHorizontalPosition {
        left: Length::from_micrometres(x + w - 10_000), ..position
    };
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(&plan, &[], &[outside], 0),
        Err(FlowFixedRegionError::RowFootprintOverflow {
            page, fragment_owner: owner, source: row.source,
        }),
    );
    // Even after a valid first hit, a later invalid checked fixed proposal
    // must refuse the entire result instead of returning the earlier hit.
    assert_eq!(
        first_fixed_row_footprint_overlap_for_flow(
            &plan, &[overlaps.clone(), invalid.clone()], &[position],
        ),
        Err(FlowFixedRegionError::FixedRegionOverflow {
            page, object: owner,
        }),
    );
    // All fixed geometry wins even when the cap is zero or row X is invalid.
    assert_eq!(
        fixed_row_footprint_overlaps_for_flow(
            &plan, &[overlaps, invalid], &[outside], 0,
        ),
        Err(FlowFixedRegionError::FixedRegionOverflow {
            page, object: owner,
        }),
    );
    assert_eq!(session.current(), Some(accepted));
    Ok(())
}
