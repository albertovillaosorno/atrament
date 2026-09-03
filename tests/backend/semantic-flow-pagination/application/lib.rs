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
use std::collections::BTreeMap;

use atrament_flow_pagination::{
    FlowUnitPolicy, MeasuredFlowUnit, MeasuredFragment, PaginationError,
};
use atrament_physical_page_profile::{
    BindingEdge, BorderShape, Length, Orientation,
    PageProfile as PhysicalPageProfile, PageProfileError, PaperMarkAppearance,
    PaperMarkJoin, PaperMarkLayer, PaperPattern, Rect, SheetSize,
};
use atrament_semantic_flow_pagination::{
    RevisionFlowMeasurement, SemanticPaginationError, paginate_revision,
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
                    page_profile: profile_one,
                },
                Page {
                    flows: vec![],
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
    assert!(plan.placements.is_empty());
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
    assert_eq!(plan.placements.len(), 1);
    assert_eq!(plan.placements[0].owner, owner);
    assert_eq!(plan.placements[0].page, page_one);
    assert_eq!(plan.placements[0].top, Length::from_micrometres(20_000));
    assert_eq!(plan.placements[0].width, Length::from_micrometres(165_000));
}

#[test]
fn measured_flow_spills_to_next_accepted_page_in_notebook_order() {
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
    assert_eq!(plan.placements[0].page, first_page);
    assert_eq!(plan.placements[1].page, second_page);
    assert_eq!(plan.placements[1].top, Length::from_micrometres(20_000));
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
    assert_eq!(shifted.placements[0].page, second_page);

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
    assert_eq!(plan.placements.len(), BLOCKS);
    assert_eq!(plan.placements.first().map(|item| item.owner), Some(first));
    assert_eq!(plan.placements.last().map(|item| item.owner), Some(last));
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
    assert_eq!(plan.placements.len(), 3);
    assert_eq!(plan.placements[0].owner, first);
    assert_eq!(plan.placements[1].owner, first);
    assert_eq!(plan.placements[2].owner, second);
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
    assert_eq!(plan.placements.len(), 3);
    assert_eq!(plan.placements[0].owner, first);
    assert_eq!(plan.placements[1].owner, first);
    assert_eq!(plan.placements[2].owner, second);
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
    assert_eq!(plan.placements.len(), 1);
    assert_eq!(plan.placements[0].page, second_page);
}
