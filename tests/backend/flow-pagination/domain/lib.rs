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
//   - Regression evidence for pagination of already-measured semantic flow.
// - Must-Not:
//   - Invent text metrics, line breaks, horizontal alignment, or renderer
//     state.
// - Allows:
//   - Inputs: Exact measured fragments and writable physical page rectangles.
//   - Outputs: Assertions over stable owners, pages, and vertical placement.
//   - Side effects: None beyond process-local test allocation.
// - Split-When:
//   - Measured text wrapping gains an independent executable fixture family.
// - Merge-When:
//   - Flow pagination is fully subsumed by a measured layout-plan harness.
// - Summary:
//   - Verifies deterministic measured-fragment page advancement.
// - Description:
//   - Covers keep-together behavior, overflow, exhaustion, and exact positions.
// - Usage:
//   - Compile directly against flow-pagination and physical-page-profile.
// - Defaults:
//   - Test owners and pages are opaque integer stand-ins for stable identities.
//
use atrament_flow_pagination::{
    FlowUnitPolicy, MeasuredFlowUnit, MeasuredFragment, PageRegion,
    PaginationError, PlacedFragment, paginate,
};
use atrament_physical_page_profile::{Length, Rect};

fn fragment(owner: u64, width: u64, height: u64) -> MeasuredFragment<u64> {
    MeasuredFragment {
        height: Length::from_micrometres(height),
        owner,
        width: Length::from_micrometres(width),
    }
}

fn page(page: u64, y: u64, width: u64, height: u64) -> PageRegion<u64> {
    PageRegion {
        page,
        writable: Rect {
            height: Length::from_micrometres(height),
            width: Length::from_micrometres(width),
            x: Length::from_micrometres(10),
            y: Length::from_micrometres(y),
        },
    }
}

fn unit(
    policy: FlowUnitPolicy,
    fragments: Vec<MeasuredFragment<u64>>,
) -> MeasuredFlowUnit<u64> {
    MeasuredFlowUnit { fragments, policy }
}

fn reference_independent(
    pages: &[PageRegion<u64>],
    fragments: &[MeasuredFragment<u64>],
) -> Result<Vec<PlacedFragment<u64, u64>>, PaginationError<u64, u64>> {
    let mut page_index = 0usize;
    let mut used_height = 0u64;
    let mut placements = Vec::new();
    for fragment in fragments {
        let can_fit_fresh = pages.iter().skip(page_index).any(|page| {
            fragment.width <= page.writable.width
                && fragment.height <= page.writable.height
        });
        if !can_fit_fresh {
            return Err(PaginationError::FragmentDoesNotFitAnyPage {
                owner: fragment.owner,
            });
        }
        loop {
            let Some(current) = pages.get(page_index) else {
                return Err(PaginationError::NoPageAvailable {
                    owner: fragment.owner,
                });
            };
            let fresh_fit = fragment.width <= current.writable.width
                && fragment.height <= current.writable.height;
            let remaining = current
                .writable
                .height
                .micrometres()
                .saturating_sub(used_height);
            if fresh_fit && fragment.height.micrometres() <= remaining {
                let top = current
                    .writable
                    .y
                    .micrometres()
                    .checked_add(used_height)
                    .expect("small oracle coordinates cannot overflow");
                placements.push(PlacedFragment {
                    height: fragment.height,
                    owner: fragment.owner,
                    page: current.page,
                    top: Length::from_micrometres(top),
                    width: fragment.width,
                });
                used_height += fragment.height.micrometres();
                break;
            }
            page_index += 1;
            used_height = 0;
        }
    }
    Ok(placements)
}

#[test]
fn independent_pagination_matches_reference_oracle() {
    let mut cases = 0usize;
    for first_width in 1..=2 {
        for first_height in 1..=2 {
            for second_width in 1..=2 {
                for second_height in 1..=2 {
                    let pages = [
                        page(1, 10, first_width, first_height),
                        page(2, 20, second_width, second_height),
                    ];
                    for first_fragment_width in 1..=3 {
                        for first_fragment_height in 1..=3 {
                            for second_fragment_width in 1..=3 {
                                for second_fragment_height in 1..=3 {
                                    let fragments = vec![
                                        fragment(
                                            11,
                                            first_fragment_width,
                                            first_fragment_height,
                                        ),
                                        fragment(
                                            12,
                                            second_fragment_width,
                                            second_fragment_height,
                                        ),
                                    ];
                                    let units = [unit(
                                        FlowUnitPolicy::Independent,
                                        fragments.clone(),
                                    )];
                                    let actual = paginate(&pages, &units)
                                        .map(|plan| plan.placements);
                                    let expected = reference_independent(
                                        &pages,
                                        &fragments,
                                    );
                                    assert_eq!(actual, expected);
                                    cases += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 1_296);
}

fn reference_keep_together_fresh(
    pages: &[PageRegion<u64>],
    fragments: &[MeasuredFragment<u64>],
) -> Result<Vec<PlacedFragment<u64, u64>>, PaginationError<u64, u64>> {
    let total_height = fragments
        .iter()
        .map(|fragment| fragment.height.micrometres())
        .sum::<u64>();
    let maximum_width = fragments
        .iter()
        .map(|fragment| fragment.width)
        .max()
        .unwrap_or(Length::ZERO);
    let whole_page = pages.iter().find(|page| {
        total_height <= page.writable.height.micrometres()
            && maximum_width <= page.writable.width
    });
    let Some(page) = whole_page else {
        return reference_independent(pages, fragments);
    };
    let mut top = page.writable.y.micrometres();
    let mut placements = Vec::with_capacity(fragments.len());
    for fragment in fragments {
        placements.push(PlacedFragment {
            height: fragment.height,
            owner: fragment.owner,
            page: page.page,
            top: Length::from_micrometres(top),
            width: fragment.width,
        });
        top += fragment.height.micrometres();
    }
    Ok(placements)
}

#[test]
fn keep_together_pagination_matches_fresh_page_reference_oracle() {
    let mut cases = 0usize;
    for first_width in 1..=2 {
        for first_height in 1..=2 {
            for second_width in 1..=2 {
                for second_height in 1..=2 {
                    for third_width in 1..=2 {
                        for third_height in 1..=2 {
                            let pages = [
                                page(1, 10, first_width, first_height),
                                page(2, 20, second_width, second_height),
                                page(3, 30, third_width, third_height),
                            ];
                            for first_fragment_width in 1..=3 {
                                for first_fragment_height in 1..=3 {
                                    for second_fragment_width in 1..=3 {
                                        for second_fragment_height in 1..=3 {
                                            let fragments = vec![
                                                fragment(
                                                    11,
                                                    first_fragment_width,
                                                    first_fragment_height,
                                                ),
                                                fragment(
                                                    12,
                                                    second_fragment_width,
                                                    second_fragment_height,
                                                ),
                                            ];
                                            let units = [unit(
                                                FlowUnitPolicy::
                                                    KeepTogetherWhenPossible,
                                                fragments.clone(),
                                            )];
                                            let actual = paginate(
                                                &pages,
                                                &units,
                                            )
                                            .map(|plan| plan.placements);
                                            let expected =
                                                reference_keep_together_fresh(
                                                    &pages,
                                                    &fragments,
                                                );
                                            assert_eq!(actual, expected);
                                            cases += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 5_184);
}

#[test]
fn empty_flow_needs_no_page_authority() {
    let plan = paginate::<u64, u64>(&[], &[]).expect("empty flow");
    assert!(plan.placements.is_empty());
}

#[test]
fn independent_fragments_fill_then_advance_in_semantic_order() {
    let pages = [page(11, 1_000, 100, 100), page(12, 2_000, 100, 100)];
    let units = [unit(FlowUnitPolicy::Independent, vec![
        fragment(1, 50, 60),
        fragment(2, 80, 40),
        fragment(3, 70, 20),
    ])];

    let plan = paginate(&pages, &units).expect("measured flow");
    assert_eq!(plan.placements, vec![
        PlacedFragment {
            height: Length::from_micrometres(60),
            owner: 1,
            page: 11,
            top: Length::from_micrometres(1_000),
            width: Length::from_micrometres(50),
        },
        PlacedFragment {
            height: Length::from_micrometres(40),
            owner: 2,
            page: 11,
            top: Length::from_micrometres(1_060),
            width: Length::from_micrometres(80),
        },
        PlacedFragment {
            height: Length::from_micrometres(20),
            owner: 3,
            page: 12,
            top: Length::from_micrometres(2_000),
            width: Length::from_micrometres(70),
        },
    ]);
}

#[test]
fn exact_bottom_fit_does_not_create_spurious_page_break() {
    let pages = [page(1, 5_000, 100, 100), page(2, 8_000, 100, 100)];
    let units = [unit(FlowUnitPolicy::Independent, vec![
        fragment(1, 100, 75),
        fragment(2, 100, 25),
    ])];

    let plan = paginate(&pages, &units).expect("exact page fill");
    assert_eq!(plan.placements.len(), 2);
    assert_eq!(plan.placements[1].page, 1);
    assert_eq!(plan.placements[1].top, Length::from_micrometres(5_075));
}

#[test]
fn keep_together_group_moves_whole_when_fresh_page_fits() {
    let pages = [page(1, 0, 100, 100), page(2, 1_000, 100, 100)];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 50, 70)]),
        unit(FlowUnitPolicy::KeepTogetherWhenPossible, vec![
            fragment(2, 80, 20),
            fragment(3, 90, 30),
        ]),
    ];

    let plan = paginate(&pages, &units).expect("keep together");
    assert_eq!(plan.placements[0].page, 1);
    assert_eq!(plan.placements[1].page, 2);
    assert_eq!(plan.placements[1].top, Length::from_micrometres(1_000));
    assert_eq!(plan.placements[2].page, 2);
    assert_eq!(plan.placements[2].top, Length::from_micrometres(1_020));
}

#[test]
fn keep_together_group_stays_on_current_page_when_remainder_fits() {
    let pages = [page(1, 400, 100, 100), page(2, 800, 100, 100)];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 50, 30)]),
        unit(FlowUnitPolicy::KeepTogetherWhenPossible, vec![
            fragment(2, 50, 20),
            fragment(3, 50, 40),
        ]),
    ];

    let plan = paginate(&pages, &units).expect("current remainder");
    assert!(plan.placements.iter().all(|placement| placement.page == 1));
    assert_eq!(plan.placements[1].top, Length::from_micrometres(430));
    assert_eq!(plan.placements[2].top, Length::from_micrometres(450));
}

#[test]
fn oversized_keep_group_splits_only_at_fragment_boundaries() {
    let pages = [
        page(1, 0, 100, 100),
        page(2, 1_000, 100, 100),
        page(3, 2_000, 100, 100),
    ];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 50, 70)]),
        unit(FlowUnitPolicy::KeepTogetherWhenPossible, vec![
            fragment(2, 90, 60),
            fragment(3, 90, 60),
        ]),
    ];

    let plan = paginate(&pages, &units).expect("fragment-boundary fallback");
    assert_eq!(plan.placements[1].page, 2);
    assert_eq!(plan.placements[1].top, Length::from_micrometres(1_000));
    assert_eq!(plan.placements[2].page, 3);
    assert_eq!(plan.placements[2].top, Length::from_micrometres(2_000));
}

#[test]
fn keep_group_skips_narrow_page_for_earliest_whole_fit() {
    let pages = [
        page(1, 0, 100, 100),
        page(2, 1_000, 40, 100),
        page(3, 2_000, 100, 100),
    ];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 50, 80)]),
        unit(FlowUnitPolicy::KeepTogetherWhenPossible, vec![
            fragment(2, 60, 10),
            fragment(3, 80, 20),
        ]),
    ];

    let plan = paginate(&pages, &units).expect("heterogeneous pages");
    assert_eq!(plan.placements[1].page, 3);
    assert_eq!(plan.placements[2].page, 3);
}

#[test]
fn fresh_page_exhaustion_is_distinct_from_unplaceable_measurement() {
    let pages = [page(1, 0, 100, 100)];
    let units = [unit(FlowUnitPolicy::Independent, vec![
        fragment(1, 80, 70),
        fragment(2, 80, 50),
    ])];

    assert_eq!(
        paginate(&pages, &units),
        Err(PaginationError::NoPageAvailable { owner: 2 })
    );
}

#[test]
fn fresh_fit_exhaustion_stays_distinct_when_later_pages_are_too_narrow() {
    let pages = [page(1, 0, 100, 100), page(2, 1_000, 50, 100)];
    let units = [unit(FlowUnitPolicy::Independent, vec![
        fragment(1, 80, 80),
        fragment(2, 80, 30),
    ])];

    assert_eq!(
        paginate(&pages, &units),
        Err(PaginationError::NoPageAvailable { owner: 2 }),
    );
}

#[test]
fn fragment_larger_than_every_remaining_page_is_typed_failure() {
    let pages = [page(1, 0, 100, 100), page(2, 1_000, 120, 90)];
    let units = [unit(FlowUnitPolicy::Independent, vec![fragment(
        7, 121, 80,
    )])];

    assert_eq!(
        paginate(&pages, &units),
        Err(PaginationError::FragmentDoesNotFitAnyPage { owner: 7 },)
    );
}

#[test]
fn invalid_page_region_rejects_before_any_placement() {
    let pages = [PageRegion {
        page: 9_u64,
        writable: Rect {
            height: Length::from_micrometres(100),
            width: Length::from_micrometres(2),
            x: Length::from_micrometres(u64::MAX),
            y: Length::ZERO,
        },
    }];
    let units = [unit(FlowUnitPolicy::Independent, vec![fragment(1, 1, 1)])];

    assert_eq!(
        paginate(&pages, &units),
        Err(PaginationError::InvalidPageRegion { page: 9 },)
    );
}

#[test]
fn empty_unit_is_semantically_inert() {
    let pages = [page(1, 0, 100, 100)];
    let units = [
        unit(FlowUnitPolicy::KeepTogetherWhenPossible, vec![]),
        unit(FlowUnitPolicy::Independent, vec![fragment(4, 10, 10)]),
    ];

    let plan = paginate(&pages, &units).expect("empty unit");
    assert_eq!(plan.placements.len(), 1);
    assert_eq!(plan.placements[0].owner, 4);
    assert_eq!(plan.placements[0].top, Length::ZERO);
}

#[test]
fn overflowing_group_sum_falls_back_to_safe_fragment_pagination() {
    let pages = [page(1, 0, 1, u64::MAX), page(2, 0, 1, u64::MAX)];
    let units = [unit(FlowUnitPolicy::KeepTogetherWhenPossible, vec![
        fragment(1, 1, u64::MAX),
        fragment(2, 1, 1),
    ])];

    let plan = paginate(&pages, &units).expect("overflow-safe split");
    assert_eq!(plan.placements.len(), 2);
    assert_eq!(plan.placements[0].page, 1);
    assert_eq!(plan.placements[1].page, 2);
}
