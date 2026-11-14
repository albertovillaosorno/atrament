// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE
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

//! Flow pagination regression tests.

use atrament_flow_pagination::{
    FlowUnitPolicy, FragmentSource, MeasuredFlowUnit, MeasuredFragment,
    PageRegion, PaginationError, PlacedFragment, paginate,
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
    for (fragment_index, fragment) in fragments.iter().enumerate() {
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
                    source: FragmentSource {
                        unit: 0,
                        fragment: fragment_index,
                    },
                    writable: current.writable,
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

fn next_case_value(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *seed >> 32
}

// Deliberately independent small-integer reservation oracle: it never calls
// the production group extent or page-selection helpers.
fn reference_next_reservation(
    units: &[MeasuredFlowUnit<u64>], start: usize,
) -> Option<(u64, u64)> {
    let mut total_height = 0u64;
    let mut maximum_width = 0u64;
    for unit in units.iter().skip(start) {
        if unit.fragments.is_empty() { continue; }
        if unit.policy == FlowUnitPolicy::StartOnNewPage {
            return None;
        }
        let count = if unit.policy == FlowUnitPolicy::Independent {
            1
        } else { unit.fragments.len() };
        for fragment in unit.fragments.iter().take(count) {
            total_height = total_height.checked_add(
                fragment.height.micrometres())?;
            maximum_width = maximum_width.max(fragment.width.micrometres());
        }
        if unit.policy != FlowUnitPolicy::KeepWithNextWhenPossible {
            return Some((total_height, maximum_width));
        }
    }
    None
}

fn reference_mixed_units(
    pages: &[PageRegion<u64>],
    units: &[MeasuredFlowUnit<u64>],
) -> Result<Vec<PlacedFragment<u64, u64>>, PaginationError<u64, u64>> {
    let proposed = reference_mixed_once(pages, units, true, true);
    if proposed.is_ok() {
        return proposed;
    }
    if units.iter().any(|unit| unit.policy
        == FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible)
        && let Ok(fallback) = reference_mixed_once(pages, units, false, true)
    {
        return Ok(fallback);
    }
    if units.iter().any(|unit| !matches!(unit.policy,
        FlowUnitPolicy::Independent | FlowUnitPolicy::StartOnNewPage))
        && let Ok(fallback) = reference_mixed_once(pages, units, false, false)
    {
        return Ok(fallback);
    }
    proposed
}

fn reference_mixed_once(
    pages: &[PageRegion<u64>],
    units: &[MeasuredFlowUnit<u64>],
    balance_opening: bool,
    use_preferences: bool,
) -> Result<Vec<PlacedFragment<u64, u64>>, PaginationError<u64, u64>> {
    let mut cursor = ReferenceCursor {
        page_index: 0,
        used_height: 0,
    };
    let mut placements = Vec::new();
    for (unit_index, unit) in units.iter().enumerate() {
        if unit.fragments.is_empty() {
            continue;
        }
        if !use_preferences
            && unit.policy != FlowUnitPolicy::StartOnNewPage
        {
            reference_place_group(
                pages, &unit.fragments, unit_index,
                &mut cursor, &mut placements,
            )?;
            continue;
        }
        match unit.policy {
            FlowUnitPolicy::StartOnNewPage => {
                if cursor.used_height > 0 {
                    cursor.page_index += 1;
                    cursor.used_height = 0;
                    if cursor.page_index >= pages.len() {
                        return Err(PaginationError::NoPageAvailable {
                            owner: unit.fragments[0].owner,
                        });
                    }
                }
                reference_place_group(
                    pages, &unit.fragments, unit_index,
                    &mut cursor, &mut placements,
                )?;
            },
            FlowUnitPolicy::Independent => reference_place_group(
                pages,
                &unit.fragments,
                unit_index,
                &mut cursor,
                &mut placements,
            )?,
            FlowUnitPolicy::KeepTogetherWhenPossible
            | FlowUnitPolicy::KeepTogetherAndBalanceTailWhenPossible
            | FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible => {
                let first_placement = placements.len();
                let total_height = unit
                    .fragments
                    .iter()
                    .map(|fragment| fragment.height.micrometres())
                    .sum::<u64>();
                let maximum_width = unit
                    .fragments
                    .iter()
                    .map(|fragment| fragment.width)
                    .max()
                    .unwrap_or(Length::ZERO);
                let current = pages.get(cursor.page_index);
                let current_fits = current.is_some_and(|page| {
                    let remaining = page
                        .writable
                        .height
                        .micrometres()
                        .saturating_sub(cursor.used_height);
                    total_height <= remaining
                        && maximum_width <= page.writable.width
                });
                if current_fits {
                    reference_place_group(
                        pages,
                        &unit.fragments,
                        unit_index,
                        &mut cursor,
                        &mut placements,
                    )?;
                    continue;
                }
                let later = cursor
                    .page_index
                    .checked_add(1)
                    .and_then(|start| {
                        pages
                            .iter()
                            .enumerate()
                            .skip(start)
                            .find(|(_, page)| {
                                total_height
                                    <= page.writable.height.micrometres()
                                    && maximum_width <= page.writable.width
                            })
                    });
                if let Some((page_index, _page)) = later {
                    cursor.page_index = page_index;
                    cursor.used_height = 0;
                }
                reference_place_group(
                    pages,
                    &unit.fragments,
                    unit_index,
                    &mut cursor,
                    &mut placements,
                )?;
                if balance_opening && unit.policy
                    == FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible
                {
                    reference_balance_opening(
                        pages, &unit.fragments, unit_index, first_placement,
                        &mut cursor, &mut placements,
                    );
                }
                if matches!(unit.policy,
                    FlowUnitPolicy::KeepTogetherAndBalanceTailWhenPossible
                    | FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible)
                {
                    reference_balance_tail(pages, first_placement,
                        &mut cursor, &mut placements);
                }
            },
            FlowUnitPolicy::KeepWithNextWhenPossible => {
                if let Some((height, width)) =
                    reference_next_reservation(units, unit_index)
                {
                    let fits_here = pages.get(cursor.page_index)
                        .is_some_and(|page| {
                            height <= page.writable.height.micrometres()
                                .saturating_sub(cursor.used_height)
                                && width <= page.writable.width.micrometres()
                        });
                    if !fits_here {
                        if let Some((later, _)) = pages.iter().enumerate()
                            .skip(cursor.page_index.saturating_add(1))
                            .find(|(_, page)| {
                                height <= page.writable.height.micrometres()
                                    && width <=
                                        page.writable.width.micrometres()
                            })
                        {
                            cursor.page_index = later;
                            cursor.used_height = 0;
                        }
                    }
                }
                reference_place_group(
                    pages, &unit.fragments, unit_index,
                    &mut cursor, &mut placements,
                )?;
            },
        }
    }
    Ok(placements)
}

// Independent reference for the first-row preference: simulate the full
// remainder on the physical sheet where the next row initially landed. A
// failed speculation must retain the already-valid baseline plan and cursor.
fn reference_balance_opening(
    pages: &[PageRegion<u64>],
    fragments: &[MeasuredFragment<u64>],
    unit_index: usize,
    start: usize,
    cursor: &mut ReferenceCursor,
    placements: &mut Vec<PlacedFragment<u64, u64>>,
) {
    let Some(rows) = placements.get(start..) else { return; };
    let Some([first_row, second_row, ..]) = rows.get(..) else {
        return;
    };
    if rows.len() < 3 || first_row.page == second_row.page {
        return;
    }
    let Some((next_index, next)) = pages.iter().enumerate()
        .find(|(_, page)| page.page == second_row.page) else {
        return;
    };
    let (Some(first), Some(second)) = (fragments.first(), fragments.get(1))
    else { return; };
    let Some(pair_height) = first.height.micrometres()
        .checked_add(second.height.micrometres()) else { return; };
    if pair_height > next.writable.height.micrometres()
        || first.width > next.writable.width
        || second.width > next.writable.width {
        return;
    }
    let mut trial_cursor = ReferenceCursor {
        page_index: next_index,
        used_height: 0,
    };
    let mut trial = Vec::new();
    if reference_place_group(
        pages, fragments, unit_index, &mut trial_cursor, &mut trial,
    ).is_err() { return; }
    placements.truncate(start);
    placements.extend(trial);
    *cursor = trial_cursor;
}

// Separate integer-coordinate reference for a lone last row of a long
// measured unit. Inspect the oracle's actual emitted rows, not a production
// reservation, to decide whether the final physical sheet can hold two.
fn reference_balance_tail(
    pages: &[PageRegion<u64>],
    start: usize,
    cursor: &mut ReferenceCursor,
    placements: &mut [PlacedFragment<u64, u64>],
) {
    let Some(rows) = placements.get(start..) else { return; };
    let Some(tail_start) = rows.len().checked_sub(4) else {
        return;
    };
    let Some([first_old, second_old, to_move, last]) =
        rows.get(tail_start..) else { return; };
    let Some(target) = pages.get(cursor.page_index) else { return; };
    let can_balance = first_old.page == to_move.page
        && second_old.page == to_move.page
        && to_move.page != last.page
        && last.page == target.page
        && last.top == target.writable.y
        && cursor.used_height == last.height.micrometres()
        && to_move.width <= target.writable.width
        && to_move.height.micrometres()
            .checked_add(last.height.micrometres())
            .is_some_and(|height| height <=
                target.writable.height.micrometres());
    if !can_balance {
        return;
    }
    let Some(new_height) = to_move.height.micrometres()
        .checked_add(last.height.micrometres()) else { return; };
    let Some(new_final_top) = target.writable.y.micrometres()
        .checked_add(to_move.height.micrometres()) else { return; };
    let Some(pair_start) = placements.len().checked_sub(2) else {
        return;
    };
    let Some([moved_row, trailing_row]) =
        placements.get_mut(pair_start..) else { return; };
    moved_row.page = target.page;
    moved_row.top = target.writable.y;
    moved_row.writable = target.writable;
    trailing_row.top = Length::from_micrometres(new_final_top);
    cursor.used_height = new_height;
}

// An independent reference derives sheet coverage from *reference*
// placements and caller-supplied physical order, never from the plan under
// test. It includes skipped pages but not unused trailing regions.
fn reference_covered_pages(
    pages: &[PageRegion<u64>],
    placements: &[PlacedFragment<u64, u64>],
) -> Vec<PageRegion<u64>> {
    let Some(last) = placements.last() else {
        return Vec::new();
    };
    let mut covered = Vec::new();
    for page in pages {
        covered.push(*page);
        if page.page == last.page {
            return covered;
        }
    }
    panic!("reference placement outside supplied page authority");
}

// The oracle checks where fragments land. Independently prove that the plan
// neither drops nor duplicates an input row, and leaves its measured payload
// untouched even when keep-together skips a page or falls back to splitting.
fn assert_placement_source_bijection(
    pages: &[PageRegion<u64>],
    units: &[MeasuredFlowUnit<u64>],
    placements: &[PlacedFragment<u64, u64>],
) {
    let expected = units
        .iter()
        .enumerate()
        .flat_map(|(unit_index, unit)| {
            unit.fragments.iter().enumerate().map(
                move |(fragment_index, fragment)| {
                    (
                        unit_index,
                        fragment_index,
                        fragment.owner,
                        fragment.width,
                        fragment.height,
                    )
                },
            )
        })
        .collect::<Vec<_>>();
    let actual = placements
        .iter()
        .map(|placed| {
            (
                placed.source.unit,
                placed.source.fragment,
                placed.owner,
                placed.width,
                placed.height,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected, "source-to-placement bijection");

    for placed in placements {
        let expected_page = pages
            .iter()
            .find(|page| page.page == placed.page)
            .expect("placed page must belong to input");
        assert_eq!(placed.writable, expected_page.writable);
        assert!(placed.top >= placed.writable.y);
        assert!(placed.width <= placed.writable.width);
        let end = placed
            .top
            .micrometres()
            .checked_add(placed.height.micrometres())
            .expect("placed bottom must be finite");
        let writable_bottom = placed
            .writable
            .y
            .micrometres()
            .checked_add(placed.writable.height.micrometres())
            .expect("writable bottom must be finite");
        assert!(end <= writable_bottom, "placed row exceeds writable page");
    }
}

#[test]
fn mixed_policy_pagination_matches_reference_oracle() {
    const CASES: usize = 10_000;
    let mut seed = 0x5eed_f10f_u64;
    let mut seen_page_counts = [false; 4];
    let mut seen_unit_counts = [false; 4];
    let mut seen_policies = [false; 6];
    let mut seen_fragment_counts = [false; 6];
    let mut saw_success = false;
    let mut saw_failure = false;
    for case in 0..CASES {
        let page_count = (next_case_value(&mut seed) % 4 + 1) as usize;
        seen_page_counts[page_count - 1] = true;
        let mut pages = Vec::with_capacity(page_count);
        for index in 0..page_count {
            let width = next_case_value(&mut seed) % 6 + 1;
            let height = next_case_value(&mut seed) % 6 + 1;
            pages.push(page(
                100 + index as u64,
                1_000 * index as u64,
                width,
                height,
            ));
        }

        let unit_count = (next_case_value(&mut seed) % 4 + 1) as usize;
        seen_unit_counts[unit_count - 1] = true;
        let mut units = Vec::with_capacity(unit_count);
        let mut owner = 1u64;
        for _ in 0..unit_count {
            let policy_index = usize::try_from(
                next_case_value(&mut seed).rem_euclid(6),
            ).unwrap_or_default();
            seen_policies[policy_index] = true;
            let policy = match policy_index {
                0 => FlowUnitPolicy::Independent,
                1 => FlowUnitPolicy::KeepTogetherWhenPossible,
                2 => FlowUnitPolicy::KeepWithNextWhenPossible,
                3 => FlowUnitPolicy::KeepTogetherAndBalanceTailWhenPossible,
                4 => FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible,
                _ => FlowUnitPolicy::StartOnNewPage,
            };
            let fragment_count = usize::try_from(
                next_case_value(&mut seed).rem_euclid(6),
            ).unwrap_or_default();
            seen_fragment_counts[fragment_count] = true;
            let mut fragments = Vec::with_capacity(fragment_count);
            for _ in 0..fragment_count {
                fragments.push(fragment(
                    owner,
                    next_case_value(&mut seed) % 7 + 1,
                    next_case_value(&mut seed) % 7 + 1,
                ));
                owner += 1;
            }
            units.push(unit(policy, fragments));
        }

        let actual = paginate(&pages, &units);
        let expected = reference_mixed_units(&pages, &units);
        if let (Ok(plan), Ok(placements)) = (&actual, &expected) {
            assert_placement_source_bijection(&pages, &units, &plan.placements);
            saw_success = true;
            assert_eq!(plan.covered_pages,
                reference_covered_pages(&pages, placements),
                "generated coverage case {case}");
        } else {
            saw_failure = true;
        }
        assert_eq!(actual.map(|plan| plan.placements), expected,
            "generated case {case}");
    }
    assert!(seen_page_counts.into_iter().all(|seen| seen));
    assert!(seen_unit_counts.into_iter().all(|seen| seen));
    assert!(seen_policies.into_iter().all(|seen| seen));
    assert!(seen_fragment_counts.into_iter().all(|seen| seen));
    assert!(saw_success);
    assert!(saw_failure);
}

#[derive(Default)]
struct PaginationOutcomeCoverage {
    saw_no_fit: bool,
    saw_no_page: bool,
    saw_success: bool,
}

impl PaginationOutcomeCoverage {
    fn observe(
        &mut self,
        result: &Result<
            Vec<PlacedFragment<u64, u64>>,
            PaginationError<u64, u64>,
        >,
    ) {
        match result {
            Ok(_) => self.saw_success = true,
            Err(PaginationError::FragmentDoesNotFitAnyPage { .. }) => {
                self.saw_no_fit = true;
            },
            Err(PaginationError::NoPageAvailable { .. }) => {
                self.saw_no_page = true;
            },
            Err(
                PaginationError::InvalidPageRegion { .. }
                | PaginationError::DuplicatePageIdentity { .. }
                | PaginationError::ZeroHeightFragment { .. },
            ) => {
                panic!("Cartesian fixtures only construct valid page regions");
            },
        }
    }

    fn assert_complete(self) {
        assert!(self.saw_success);
        assert!(self.saw_no_fit);
        assert!(self.saw_no_page);
    }
}

#[test]
fn independent_pagination_matches_reference_oracle() {
    let mut cases = 0usize;
    let mut outcomes = PaginationOutcomeCoverage::default();
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
                                    let actual = paginate(&pages, &units);
                                    let expected = reference_independent(
                                        &pages,
                                        &fragments,
                                    );
                                    outcomes.observe(&expected);
                                    if let (Ok(plan), Ok(placements)) =
                                        (&actual, &expected) {
                                        assert_eq!(plan.covered_pages,
                                            reference_covered_pages(
                                                &pages, placements));
                                    }
                                    assert_eq!(actual.map(|p| p.placements),
                                        expected);
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
    outcomes.assert_complete();
}

#[derive(Clone, Copy)]
struct ReferenceCursor {
    page_index: usize,
    used_height: u64,
}

fn reference_place_fragment(
    pages: &[PageRegion<u64>],
    fragment: MeasuredFragment<u64>,
    source: FragmentSource,
    cursor: &mut ReferenceCursor,
    placements: &mut Vec<PlacedFragment<u64, u64>>,
) -> Result<(), PaginationError<u64, u64>> {
    let can_fit_fresh = pages.iter().skip(cursor.page_index).any(|page| {
        fragment.width <= page.writable.width
            && fragment.height <= page.writable.height
    });
    if !can_fit_fresh {
        return Err(PaginationError::FragmentDoesNotFitAnyPage {
            owner: fragment.owner,
        });
    }
    loop {
        let Some(page) = pages.get(cursor.page_index) else {
            return Err(PaginationError::NoPageAvailable {
                owner: fragment.owner,
            });
        };
        let fresh_fit = fragment.width <= page.writable.width
            && fragment.height <= page.writable.height;
        let remaining = page
            .writable
            .height
            .micrometres()
            .saturating_sub(cursor.used_height);
        if fresh_fit && fragment.height.micrometres() <= remaining {
            placements.push(PlacedFragment {
                height: fragment.height,
                owner: fragment.owner,
                page: page.page,
                source,
                writable: page.writable,
                top: Length::from_micrometres(
                    page.writable.y.micrometres() + cursor.used_height,
                ),
                width: fragment.width,
            });
            cursor.used_height += fragment.height.micrometres();
            return Ok(());
        }
        cursor.page_index += 1;
        cursor.used_height = 0;
    }
}

fn reference_place_group(
    pages: &[PageRegion<u64>],
    fragments: &[MeasuredFragment<u64>],
    unit_index: usize,
    cursor: &mut ReferenceCursor,
    placements: &mut Vec<PlacedFragment<u64, u64>>,
) -> Result<(), PaginationError<u64, u64>> {
    for (fragment_index, fragment) in fragments.iter().enumerate() {
        reference_place_fragment(
            pages,
            *fragment,
            FragmentSource {
                unit: unit_index,
                fragment: fragment_index,
            },
            cursor,
            placements,
        )?;
    }
    Ok(())
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
    for (fragment_index, fragment) in fragments.iter().enumerate() {
        placements.push(PlacedFragment {
            height: fragment.height,
            owner: fragment.owner,
            page: page.page,
            source: FragmentSource {
                unit: 0,
                fragment: fragment_index,
            },
            writable: page.writable,
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
    let mut outcomes = PaginationOutcomeCoverage::default();
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
                                            );
                                            let expected =
                                                reference_keep_together_fresh(
                                                    &pages,
                                                    &fragments,
                                                );
                                            outcomes.observe(&expected);
                                            if let (Ok(plan), Ok(placements)) =
                                                (&actual, &expected) {
                                                assert_eq!(plan.covered_pages,
                                                    reference_covered_pages(
                                                        &pages, placements));
                                            }
                                            assert_eq!(
                                                actual.map(|p| p.placements),
                                                expected,
                                            );
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
    outcomes.assert_complete();
}

fn reference_prefix_then_keep(
    pages: &[PageRegion<u64>],
    prefix: MeasuredFragment<u64>,
    fragments: &[MeasuredFragment<u64>],
) -> Result<Vec<PlacedFragment<u64, u64>>, PaginationError<u64, u64>> {
    let mut cursor = ReferenceCursor {
        page_index: 0,
        used_height: 0,
    };
    let mut placements = Vec::new();
    reference_place_fragment(
        pages,
        prefix,
        FragmentSource { unit: 0, fragment: 0 },
        &mut cursor,
        &mut placements,
    )?;
    let total_height = fragments
        .iter()
        .map(|fragment| fragment.height.micrometres())
        .sum::<u64>();
    let maximum_width = fragments
        .iter()
        .map(|fragment| fragment.width)
        .max()
        .unwrap_or(Length::ZERO);
    let current = &pages[cursor.page_index];
    let remaining = current
        .writable
        .height
        .micrometres()
        .saturating_sub(cursor.used_height);
    if total_height <= remaining && maximum_width <= current.writable.width {
        for (fragment_index, fragment) in fragments.iter().enumerate() {
            reference_place_fragment(
                pages,
                *fragment,
                FragmentSource {
                    unit: 1,
                    fragment: fragment_index,
                },
                &mut cursor,
                &mut placements,
            )?;
        }
        return Ok(placements);
    }
    if let Some((page_index, page)) = pages
        .iter()
        .enumerate()
        .skip(cursor.page_index + 1)
        .find(|(_, page)| {
            total_height <= page.writable.height.micrometres()
                && maximum_width <= page.writable.width
        })
    {
        cursor.page_index = page_index;
        cursor.used_height = 0;
        for (fragment_index, fragment) in fragments.iter().enumerate() {
            placements.push(PlacedFragment {
                height: fragment.height,
                owner: fragment.owner,
                page: page.page,
                source: FragmentSource {
                    unit: 1,
                    fragment: fragment_index,
                },
                writable: page.writable,
                top: Length::from_micrometres(
                    page.writable.y.micrometres() + cursor.used_height,
                ),
                width: fragment.width,
            });
            cursor.used_height += fragment.height.micrometres();
        }
        return Ok(placements);
    }
    reference_place_group(pages, fragments, 1, &mut cursor, &mut placements)?;
    Ok(placements)
}

#[test]
fn keep_together_remainder_matches_reference_oracle() {
    let keep_together = FlowUnitPolicy::KeepTogetherWhenPossible;
    let mut cases = 0usize;
    let mut outcomes = PaginationOutcomeCoverage::default();
    for first_width in 1..=2 {
        for first_height in 1..=2 {
            for second_width in 1..=2 {
                for second_height in 1..=2 {
                    let pages = [
                        page(1, 10, first_width, first_height),
                        page(2, 20, second_width, second_height),
                    ];
                    for prefix_width in 1..=2 {
                        for prefix_height in 1..=2 {
                            let prefix = fragment(
                                10,
                                prefix_width,
                                prefix_height,
                            );
                            for first_width in 1..=2 {
                                for first_height in 1..=2 {
                                    for second_width in 1..=2 {
                                        for second_height in 1..=2 {
                                            let group = vec![
                                                fragment(
                                                    11,
                                                    first_width,
                                                    first_height,
                                                ),
                                                fragment(
                                                    12,
                                                    second_width,
                                                    second_height,
                                                ),
                                            ];
                                            let units = [
                                                unit(
                                                    FlowUnitPolicy::Independent,
                                                    vec![prefix],
                                                ),
                                                unit(
                                                    keep_together,
                                                    group.clone(),
                                                ),
                                            ];
                                            let actual = paginate(
                                                &pages,
                                                &units,
                                            )
                                            .map(|plan| plan.placements);
                                            let expected =
                                                reference_prefix_then_keep(
                                                    &pages,
                                                    prefix,
                                                    &group,
                                                );
                                            outcomes.observe(&expected);
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
    assert_eq!(cases, 1_024);
    outcomes.assert_complete();
}

#[test]
fn empty_flow_needs_no_page_authority() {
    let plan = paginate::<u64, u64>(&[], &[]).expect("empty flow");
    assert!(plan.placements.is_empty());
    assert!(plan.covered_pages.is_empty());

    let invalid_pages = [PageRegion {
        page: 7_u64,
        writable: Rect {
            height: Length::ZERO,
            width: Length::ZERO,
            x: Length::from_micrometres(u64::MAX),
            y: Length::from_micrometres(u64::MAX),
        },
    }];
    let empty_units = [unit(
        FlowUnitPolicy::KeepTogetherWhenPossible,
        Vec::new(),
    )];
    let plan = paginate(&invalid_pages, &empty_units)
        .expect("empty measured flow must not inspect unused page authority");
    assert!(plan.placements.is_empty());
    assert!(plan.covered_pages.is_empty());
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
            source: FragmentSource { unit: 0, fragment: 0 },
            owner: 1,
            page: 11,
            writable: Rect {
                height: Length::from_micrometres(100),
                width: Length::from_micrometres(100),
                x: Length::from_micrometres(10),
                y: Length::from_micrometres(1_000),
            },
            top: Length::from_micrometres(1_000),
            width: Length::from_micrometres(50),
        },
        PlacedFragment {
            height: Length::from_micrometres(40),
            source: FragmentSource { unit: 0, fragment: 1 },
            owner: 2,
            page: 11,
            writable: Rect {
                height: Length::from_micrometres(100),
                width: Length::from_micrometres(100),
                x: Length::from_micrometres(10),
                y: Length::from_micrometres(1_000),
            },
            top: Length::from_micrometres(1_060),
            width: Length::from_micrometres(80),
        },
        PlacedFragment {
            height: Length::from_micrometres(20),
            source: FragmentSource { unit: 0, fragment: 2 },
            owner: 3,
            page: 12,
            writable: Rect {
                height: Length::from_micrometres(100),
                width: Length::from_micrometres(100),
                x: Length::from_micrometres(10),
                y: Length::from_micrometres(2_000),
            },
            top: Length::from_micrometres(2_000),
            width: Length::from_micrometres(70),
        },
    ]);
}

#[test]
fn placed_fragments_retain_each_selected_pages_writable_horizontal_bounds() {
    let mut first = page(1, 300, 100, 50);
    first.writable.x = Length::from_micrometres(17);
    let mut second = page(2, 600, 100, 60);
    second.writable.x = Length::from_micrometres(83);
    second.writable.width = Length::from_micrometres(40);
    for policy in [
        FlowUnitPolicy::Independent,
        FlowUnitPolicy::KeepTogetherWhenPossible,
    ] {
        let units = [
            unit(FlowUnitPolicy::Independent, vec![fragment(1, 50, 30)]),
            unit(policy, vec![fragment(2, 30, 15), fragment(3, 30, 15)]),
        ];
        let plan = paginate(&[first, second], &units).expect("page placements");
        assert_eq!(plan.placements.len(), 3);
        assert_eq!(
            plan.placements.first().map(|placed| placed.writable.x),
            Some(Length::from_micrometres(17))
        );
        assert_eq!(
            plan.placements.first().map(|placed| placed.writable.width),
            Some(Length::from_micrometres(100))
        );
        let group_left = if policy == FlowUnitPolicy::Independent {
            [17, 83]
        } else {
            [83, 83]
        };
        let group_page = if policy == FlowUnitPolicy::Independent {
            [1, 2]
        } else {
            [2, 2]
        };
        let group_width = if policy == FlowUnitPolicy::Independent {
            [100, 40]
        } else {
            [40, 40]
        };
        for (position, ((&left, &page), &width)) in group_left
            .iter()
            .zip(&group_page)
            .zip(&group_width)
            .enumerate()
        {
            assert_eq!(
                plan.placements
                    .get(position + 1)
                    .map(|placed| placed.writable.width),
                Some(Length::from_micrometres(width))
            );
            assert_eq!(
                plan.placements
                    .get(position + 1)
                    .map(|placed| placed.writable.x),
                Some(Length::from_micrometres(left))
            );
            assert_eq!(plan.placements[position + 1].page, page);
        }
    }
}

#[test]
fn placed_fragments_retain_page_vertical_bounds_across_layout_policies() {
    let pages = [page(1, 310, 100, 50), page(2, 640, 80, 80)];
    for policy in [
        FlowUnitPolicy::Independent,
        FlowUnitPolicy::KeepTogetherWhenPossible,
    ] {
        let units = [
            unit(FlowUnitPolicy::Independent, vec![fragment(1, 20, 30)]),
            unit(policy, vec![fragment(2, 20, 15), fragment(3, 20, 15)]),
        ];
        let plan = paginate(&pages, &units).expect("valid measured placements");
        assert_eq!(plan.placements.len(), 3);
        let expected_pages = if policy == FlowUnitPolicy::Independent {
            [1, 1, 2]
        } else {
            [1, 2, 2]
        };
        for (placement, expected_page) in
            plan.placements.iter().zip(expected_pages)
        {
            let source = pages
                .iter()
                .find(|page| page.page == expected_page)
                .expect("expected page exists");
            assert_eq!(placement.page, expected_page);
            assert_eq!(placement.writable, source.writable);
            assert!(placement.top >= placement.writable.y);
            assert!(
                placement.top.micrometres() + placement.height.micrometres()
                    <= placement.writable.y.micrometres()
                        + placement.writable.height.micrometres()
            );
        }
        assert_eq!(plan.placements[0].top, Length::from_micrometres(310));
    }
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
    // The sheet skipped by ink remains an ordered part of the physical plan.
    assert_eq!(plan.covered_pages, pages);
}

#[test]
fn covered_pages_stop_at_last_ink_not_unused_trailing_paper() {
    // Page identities need not sort like the physical sheet order.
    let mut pages = [
        page(91, 0, 100, 100),
        page(8, 1_000, 30, 100),
        page(45, 2_000, 100, 100),
        page(3, 3_000, 100, 100),
    ];
    pages[1].writable.x = Length::from_micrometres(55);
    pages[2].writable.x = Length::from_micrometres(75);
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 50, 80)]),
        unit(FlowUnitPolicy::KeepTogetherWhenPossible, vec![
            fragment(2, 60, 20),
            fragment(3, 80, 20),
        ]),
    ];
    let plan = paginate(&pages, &units).expect("covered physical pages");
    assert_eq!(plan.covered_pages, pages[..3]);
    assert_eq!(plan.placements.iter().map(|p| p.page)
        .collect::<Vec<_>>(), [91, 45, 45]);
}

#[test]
fn later_fragment_never_backfills_page_skipped_by_keep_group() {
    let pages = [
        page(1, 0, 100, 100),
        page(2, 1_000, 30, 100),
        page(3, 2_000, 100, 100),
    ];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 50, 80)]),
        unit(FlowUnitPolicy::KeepTogetherWhenPossible, vec![
            fragment(2, 60, 20),
            fragment(3, 80, 20),
        ]),
        unit(FlowUnitPolicy::Independent, vec![fragment(4, 20, 10)]),
    ];

    let plan = paginate(&pages, &units).expect("ordered page advancement");
    assert_eq!(
        plan.placements
            .iter()
            .map(|placement| placement.page)
            .collect::<Vec<_>>(),
        [1, 3, 3, 3],
    );
    assert_eq!(plan.placements[3].top, Length::from_micrometres(2_040));
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
fn duplicate_page_identity_refuses_ambiguous_physical_placements() {
    let pages = [page(9, 0, 100, 100), page(9, 500, 100, 100)];
    for policy in [
        FlowUnitPolicy::Independent,
        FlowUnitPolicy::KeepTogetherWhenPossible,
    ] {
        let units =
            [unit(policy, vec![fragment(1, 20, 60), fragment(2, 20, 60)])];
        assert_eq!(
            paginate(&pages, &units),
            Err(PaginationError::DuplicatePageIdentity { page: 9 }),
        );
    }

    // Without measured content, no page identity is projected into output.
    let empty: [MeasuredFlowUnit<u64>; 0] = [];
    assert_eq!(
        paginate(&pages, &empty).map(|plan| plan.placements),
        Ok(Vec::new()),
    );
}

#[test]
fn zero_height_fragment_cannot_be_emitted_as_a_page_placement() {
    let pages = [page(1, 0, 100, 100), page(2, 500, 100, 100)];
    for policy in [
        FlowUnitPolicy::Independent,
        FlowUnitPolicy::KeepTogetherWhenPossible,
    ] {
        for (owner, heights) in [
            (1, [0, 30, 30]),
            (2, [30, 0, 30]),
            (3, [30, 30, 0]),
        ] {
            let [first, second, third] = heights;
            let units = [unit(policy, vec![
                fragment(1, 20, first),
                fragment(2, 20, second),
                fragment(3, 20, third),
            ])];
            assert_eq!(
                paginate(&pages, &units),
                Err(PaginationError::ZeroHeightFragment { owner }),
            );
        }
    }
}

#[test]
fn zero_width_positive_height_fragment_reserves_vertical_space() {
    let pages = [page(1, 500, 100, 100)];
    let units = [unit(FlowUnitPolicy::Independent, vec![
        fragment(1, 0, 20),
        fragment(2, 10, 30),
    ])];
    assert_eq!(
        paginate(&pages, &units).map(|plan| plan.placements),
        Ok(vec![
            PlacedFragment {
                height: Length::from_micrometres(20),
                source: FragmentSource { unit: 0, fragment: 0 },
                owner: 1,
                page: 1,
                writable: Rect {
                    height: Length::from_micrometres(100),
                    width: Length::from_micrometres(100),
                    x: Length::from_micrometres(10),
                    y: Length::from_micrometres(500),
                },
                top: Length::from_micrometres(500),
                width: Length::ZERO,
            },
            PlacedFragment {
                height: Length::from_micrometres(30),
                source: FragmentSource { unit: 0, fragment: 1 },
                owner: 2,
                page: 1,
                writable: Rect {
                    height: Length::from_micrometres(100),
                    width: Length::from_micrometres(100),
                    x: Length::from_micrometres(10),
                    y: Length::from_micrometres(500),
                },
                top: Length::from_micrometres(520),
                width: Length::from_micrometres(10),
            },
        ])
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

#[test]
fn placement_source_tracks_measured_units_across_empty_units_and_skips() {
    let pages = [
        page(41, 1_000, 30, 30),
        page(42, 2_000, 50, 30),
        page(43, 3_000, 60, 50),
    ];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(9, 30, 30)]),
        unit(FlowUnitPolicy::Independent, vec![]),
        unit(FlowUnitPolicy::KeepTogetherWhenPossible, vec![
            fragment(9, 55, 15),
            fragment(9, 55, 15),
        ]),
    ];
    let plan = paginate(&pages, &units).expect("measured fragments fit");
    assert_eq!(
        plan.placements
            .iter()
            .map(|placed| (
                placed.page,
                placed.source.unit,
                placed.source.fragment,
            ))
            .collect::<Vec<_>>(),
        [(41, 0, 0), (43, 2, 0), (43, 2, 1)],
    );
    assert_eq!(plan.placements[1].owner, plan.placements[0].owner);
}

#[test]
fn keep_with_next_moves_heading_and_first_body_row_together() {
    let pages = [page(1, 500, 100, 100), page(2, 700, 100, 100)];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 50, 75)]),
        unit(FlowUnitPolicy::KeepWithNextWhenPossible,
            vec![fragment(2, 50, 15)]),
        unit(FlowUnitPolicy::Independent,
            vec![fragment(3, 50, 20), fragment(4, 50, 30)]),
    ];
    let plan = paginate(&pages, &units).expect("heading can travel with body");
    assert_eq!(plan.placements.iter().map(|p| p.page).collect::<Vec<_>>(),
        [1, 2, 2, 2]);
    assert_eq!(plan.placements.iter().map(|p| p.top.micrometres())
        .collect::<Vec<_>>(), [500, 700, 715, 735]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
    assert_eq!(plan.covered_pages, pages);
}

#[test]
fn keep_with_next_chain_skips_empty_units_and_narrow_pages() {
    let pages = [
        page(1, 0, 100, 100),
        page(2, 1000, 100, 25),
        page(3, 2000, 100, 120),
    ];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 40, 80)]),
        unit(FlowUnitPolicy::KeepWithNextWhenPossible,
            vec![fragment(2, 40, 10)]),
        unit(FlowUnitPolicy::Independent, vec![]),
        unit(FlowUnitPolicy::KeepWithNextWhenPossible,
            vec![fragment(3, 40, 10)]),
        unit(FlowUnitPolicy::KeepTogetherWhenPossible,
            vec![fragment(4, 40, 15), fragment(5, 40, 15)]),
    ];
    let plan = paginate(&pages, &units).expect("two titles join full body");
    assert_eq!(plan.placements.iter().map(|p| p.page).collect::<Vec<_>>(),
        [1, 3, 3, 3, 3]);
    assert_eq!(plan.placements.iter().map(|p| p.top.micrometres())
        .collect::<Vec<_>>(), [0, 2000, 2010, 2020, 2035]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
    assert_eq!(plan.covered_pages, pages);
}

#[test]
fn keep_with_next_degrades_when_no_common_page_fits() {
    let pages = [page(1, 0, 100, 100), page(2, 1000, 100, 100)];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 50, 80)]),
        unit(FlowUnitPolicy::KeepWithNextWhenPossible,
            vec![fragment(2, 50, 10)]),
        unit(FlowUnitPolicy::Independent, vec![fragment(3, 50, 95)]),
    ];
    let plan = paginate(&pages, &units)
        .expect("unsatisfiable pair is advisory");
    assert_eq!(plan.placements.iter().map(|p| p.page).collect::<Vec<_>>(),
        [1, 1, 2]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
}

#[test]
fn keep_with_next_exact_current_fit_does_not_skip_a_page() {
    let pages = [page(1, 10, 100, 100), page(2, 200, 100, 100)];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 50, 60)]),
        unit(FlowUnitPolicy::KeepWithNextWhenPossible,
            vec![fragment(2, 50, 20)]),
        unit(FlowUnitPolicy::Independent, vec![fragment(3, 50, 20)]),
    ];
    let plan = paginate(&pages, &units).expect("exact bottom fit");
    assert_eq!(plan.placements.iter().map(|p| p.page).collect::<Vec<_>>(),
        [1, 1, 1]);
    assert_eq!(plan.covered_pages, vec![pages[0]]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
}

#[test]
fn keep_with_next_without_successor_is_independent() {
    let pages = [page(1, 0, 100, 30), page(2, 100, 100, 30)];
    let units = [
        unit(FlowUnitPolicy::KeepWithNextWhenPossible,
            vec![fragment(1, 30, 20), fragment(2, 30, 20)]),
        unit(FlowUnitPolicy::KeepTogetherWhenPossible, vec![]),
    ];
    let plan = paginate(&pages, &units).expect("trailing heading is legal");
    assert_eq!(plan.placements.iter().map(|p| p.page).collect::<Vec<_>>(),
        [1, 2]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
}

#[test]
fn keep_with_next_overflow_falls_back_without_wrapping() {
    let pages = [
        page(1, 0, 100, u64::MAX),
        page(2, 0, 100, 100),
    ];
    let units = [
        unit(FlowUnitPolicy::KeepWithNextWhenPossible,
            vec![fragment(1, 50, u64::MAX - 10)]),
        unit(FlowUnitPolicy::Independent, vec![fragment(2, 50, 20)]),
    ];
    let plan = paginate(&pages, &units).expect("no overflowing group sum");
    assert_eq!(plan.placements.iter().map(|p| p.page).collect::<Vec<_>>(),
        [1, 2]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
}

#[test]
fn long_keep_with_next_chain_reserves_one_page_in_source_order() {
    // A large chain is intentionally linear to process. Each unit remains
    // identifiable rather than being folded into a synthetic giant fragment.
    let pages = [page(1, 0, 100, 1_000), page(2, 20_000, 100, 16_385)];
    let mut units = vec![unit(
        FlowUnitPolicy::Independent,
        vec![fragment(1, 20, 999)],
    )];
    for owner in 2..=16_385u64 {
        units.push(unit(
            FlowUnitPolicy::KeepWithNextWhenPossible,
            vec![fragment(owner, 20, 1)],
        ));
    }
    units.push(unit(
        FlowUnitPolicy::Independent,
        vec![fragment(16_386, 20, 1)],
    ));
    let plan = paginate(&pages, &units)
        .expect("full heading chain fits page 2");
    assert_eq!(plan.placements.len(), 16_386);
    assert_eq!(plan.placements.first().map(|placed| placed.page), Some(1));
    assert_eq!(plan.placements.last().map(|placed| placed.page), Some(2));
    assert!(plan.placements.iter().skip(1).all(|placed| placed.page == 2));
    assert_eq!(plan.placements.last().map(|placed| placed.top),
        Some(Length::from_micrometres(36_384)));
    assert_placement_source_bijection(&pages, &units, &plan.placements);
}

#[test]
fn keep_with_next_cannot_exhaust_usable_pages_by_skipping_an_opening() {
    let pages = [
        page(80, 100, 30, 8), page(5, 200, 5, 12),
        page(60, 300, 30, 7),
    ];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 20, 5)]),
        unit(FlowUnitPolicy::KeepWithNextWhenPossible,
            vec![fragment(2, 20, 2)]),
        unit(FlowUnitPolicy::Independent, vec![fragment(3, 20, 4)]),
        unit(FlowUnitPolicy::Independent, vec![fragment(4, 20, 2)]),
    ];
    assert_eq!(reference_mixed_once(&pages, &units, true, true),
        Err(PaginationError::NoPageAvailable { owner: 4 }));
    let plan = paginate(&pages, &units)
        .expect("an advisory heading group must not exhaust the next row");
    assert_eq!(plan.covered_pages, pages);
    assert_eq!(plan.placements.iter().map(|placed| placed.page)
        .collect::<Vec<_>>(), [80, 80, 60, 60]);
    assert_eq!(plan.placements.iter().map(|placed| placed.top.micrometres())
        .collect::<Vec<_>>(), [100, 105, 300, 304]);
    assert_eq!(plan.placements,
        reference_mixed_once(&pages, &units, false, false)
            .expect("independent reference can fit all fragments"));
    assert_placement_source_bijection(&pages, &units, &plan.placements);
}

#[test]
fn keep_together_page_preference_cannot_strand_later_measured_rows() {
    let pages = [page(40, 100, 30, 8), page(17, 300, 30, 7)];
    for policy in [
        FlowUnitPolicy::KeepTogetherWhenPossible,
        FlowUnitPolicy::KeepTogetherAndBalanceTailWhenPossible,
        FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible,
    ] {
        let units = [
            unit(FlowUnitPolicy::Independent, vec![fragment(1, 20, 5)]),
            unit(policy, vec![fragment(2, 20, 2), fragment(3, 20, 2)]),
            unit(FlowUnitPolicy::Independent, vec![fragment(4, 20, 4)]),
        ];
        assert_eq!(reference_mixed_once(&pages, &units, true, true),
            Err(PaginationError::NoPageAvailable { owner: 4 }));
        let plan = paginate(&pages, &units)
            .expect("full-unit preference must yield to a valid split");
        assert_eq!(plan.placements.iter().map(|placed| placed.page)
            .collect::<Vec<_>>(), [40, 40, 17, 17]);
        assert_eq!(plan.placements.iter().map(
            |placed| placed.top.micrometres())
            .collect::<Vec<_>>(), [100, 105, 300, 302]);
        assert_eq!(plan.placements,
            reference_mixed_once(&pages, &units, false, false)
                .expect("independent reference fits"));
        assert_placement_source_bijection(&pages, &units, &plan.placements);

        // An impossible final row must still refuse with its original
        // source owner, even if a fallback can split the earlier group.
        let mut impossible = units.clone();
        impossible[2].fragments[0].height =
            Length::from_micrometres(8);
        assert_eq!(paginate(&pages, &impossible),
            Err(PaginationError::FragmentDoesNotFitAnyPage { owner: 4 }));
    }
}

#[test]
fn exhausted_group_preferences_match_exhaustive_split_reference() {
    let mut cases = 0usize;
    let mut salvaged = [0usize; 2];
    for (policy_index, policy) in [
        FlowUnitPolicy::KeepWithNextWhenPossible,
        FlowUnitPolicy::KeepTogetherWhenPossible,
    ].into_iter().enumerate() {
        for first_page_height in 2..=10 {
            for last_page_height in 2..=10 {
                let pages = [
                    page(19, 100, 20, first_page_height),
                    page(3, 300, 20, last_page_height),
                ];
                for prior_height in 1..=6 {
                    for first_height in 1..=6 {
                        for last_height in 1..=6 {
                            let grouped = if policy
                                == FlowUnitPolicy::KeepWithNextWhenPossible
                            {
                                [
                                    unit(policy,
                                        vec![fragment(2, 20, first_height)]),
                                    unit(FlowUnitPolicy::Independent,
                                        vec![fragment(3, 20, last_height)]),
                                ]
                            } else {
                                [
                                    unit(policy,
                                        vec![fragment(2, 20, first_height),
                                            fragment(3, 20, last_height)]),
                                    unit(FlowUnitPolicy::Independent, vec![]),
                                ]
                            };
                            for final_height in 1..=6 {
                                let units = [
                                    unit(FlowUnitPolicy::Independent,
                                        vec![fragment(1, 20, prior_height)]),
                                    grouped[0].clone(), grouped[1].clone(),
                                    unit(FlowUnitPolicy::Independent,
                                        vec![fragment(4, 20, final_height)]),
                                ];
                                let proposed = reference_mixed_once(
                                    &pages, &units, true, true);
                                let independent = reference_mixed_once(
                                    &pages, &units, false, false);
                                if proposed.is_err() && independent.is_ok() {
                                    salvaged[policy_index] += 1;
                                }
                                let expected = proposed.or_else(|original| {
                                    independent.map_err(|_| original)
                                });
                                let actual = paginate(&pages, &units);
                                assert_eq!(actual.map(|plan| plan.placements),
                                    expected,
                                    "case {cases}, policy {policy:?}");
                                cases += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 209_952);
    assert!(salvaged.into_iter().all(|count| count > 100));
}

#[test]
fn group_preference_replay_preserves_structural_input_refusals() {
    let units = [unit(FlowUnitPolicy::KeepWithNextWhenPossible,
        vec![fragment(2, 10, 2)]),
        unit(FlowUnitPolicy::Independent, vec![fragment(3, 10, 2)])];
    let duplicate = [page(11, 100, 30, 8), page(11, 300, 30, 8)];
    assert_eq!(paginate(&duplicate, &units),
        Err(PaginationError::DuplicatePageIdentity { page: 11 }));
    let invalid = [page(11, u64::MAX, 30, 8)];
    assert_eq!(paginate(&invalid, &units),
        Err(PaginationError::InvalidPageRegion { page: 11 }));
    let mut zero = units.clone();
    zero[1].fragments[0].height = Length::ZERO;
    assert_eq!(paginate(&[page(11, 100, 30, 8)], &zero),
        Err(PaginationError::ZeroHeightFragment { owner: 3 }));
}

#[test]
fn successful_keep_preference_is_not_weakened_by_fallback() {
    let pages = [page(1, 100, 30, 8), page(2, 300, 30, 8)];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 20, 5)]),
        unit(FlowUnitPolicy::KeepTogetherWhenPossible,
            vec![fragment(2, 20, 2), fragment(3, 20, 2)]),
        unit(FlowUnitPolicy::Independent, vec![fragment(4, 20, 2)]),
    ];
    let plan = paginate(&pages, &units).expect("preferred group fits");
    assert_eq!(plan.placements.iter().map(|placed| placed.page)
        .collect::<Vec<_>>(), [1, 2, 2, 2]);
    assert_eq!(plan.placements.iter().map(|placed| placed.top.micrometres())
        .collect::<Vec<_>>(), [100, 300, 302, 304]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
}

#[test]
fn balanced_ends_moves_opening_row_and_then_guards_trailing_row() {
    let pages = [page(19, 100, 100, 60), page(7, 500, 100, 40),
        page(31, 900, 100, 40)];
    let rows = (0..5).map(|index| fragment(index + 1, 40, 10))
        .collect::<Vec<_>>();
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(90, 40, 45)]),
        unit(FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible,
            rows.clone()),
        unit(FlowUnitPolicy::Independent, vec![fragment(91, 40, 10)]),
    ];
    let outcome = paginate(&pages, &units);
    assert!(outcome.is_ok(), "balanced ends: {outcome:?}");
    let Ok(plan) = outcome else { return; };
    assert_eq!(plan.placements.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [19, 7, 7, 7, 31, 31, 31]);
    assert_eq!(plan.placements.iter().map(|row| row.top.micrometres())
        .collect::<Vec<_>>(), [100, 500, 510, 520, 900, 910, 920]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);

    let original = [
        unit(FlowUnitPolicy::Independent, vec![fragment(90, 40, 45)]),
        unit(FlowUnitPolicy::KeepTogetherAndBalanceTailWhenPossible, rows),
        unit(FlowUnitPolicy::Independent, vec![fragment(91, 40, 10)]),
    ];
    let result = paginate(&pages, &original);
    assert!(result.is_ok(), "original policy: {result:?}");
    let Ok(baseline) = result else { return; };
    assert_eq!(baseline.placements.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [19, 19, 7, 7, 7, 7, 31]);
}

#[test]
fn balanced_ends_does_not_turn_fitting_layout_into_exhaustion() {
    let pages = [page(1, 100, 100, 15), page(2, 300, 100, 20)];
    let rows = vec![fragment(1, 30, 10), fragment(2, 30, 10),
        fragment(3, 30, 10)];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(90, 20, 5)]),
        unit(FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible, rows),
    ];
    let result = paginate(&pages, &units);
    assert!(result.is_ok(), "must retain successful baseline: {result:?}");
    let Ok(plan) = result else { return; };
    assert_eq!(plan.placements.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 1, 2, 2]);
    assert_eq!(plan.placements.iter().map(|row| row.top.micrometres())
        .collect::<Vec<_>>(), [100, 105, 300, 310]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
}

#[test]
fn balanced_ends_recovers_when_later_block_needs_remaining_page() {
    let pages = [
        page(8, 100, 100, 60), page(90, 300, 100, 40),
        page(7, 700, 100, 30),
    ];
    let rows = (0..5).map(|n| fragment(n + 1, 30, 10))
        .collect::<Vec<_>>();
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(91, 30, 45)]),
        unit(FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible,
            rows.clone()),
        unit(FlowUnitPolicy::Independent, vec![fragment(92, 30, 30)]),
    ];
    let result = paginate(&pages, &units);
    assert!(result.is_ok(), "later block must retain space: {result:?}");
    let Ok(plan) = result else { return; };
    assert_eq!(plan.placements.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [8, 8, 90, 90, 90, 90, 7]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
    let baseline_units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(91, 30, 45)]),
        unit(FlowUnitPolicy::KeepTogetherAndBalanceTailWhenPossible,
            rows),
        unit(FlowUnitPolicy::Independent, vec![fragment(92, 30, 30)]),
    ];
    let baseline = paginate(&pages, &baseline_units);
    assert_eq!(baseline.map(|plan| plan.placements),
        Ok(plan.placements));
}

#[test]
fn balanced_ends_respects_width_and_skipped_unsorted_page_ids() {
    let pages = [
        page(90, 100, 100, 40), page(2, 300, 5, 50),
        page(87, 500, 100, 30), page(3, 700, 100, 30),
    ];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(99, 20, 30)]),
        unit(FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible,
            vec![fragment(1, 20, 10), fragment(2, 20, 10),
                fragment(3, 20, 10), fragment(4, 20, 10)]),
    ];
    let outcome = paginate(&pages, &units);
    assert!(outcome.is_ok(), "skipped narrow region: {outcome:?}");
    let Ok(plan) = outcome else { return; };
    assert_eq!(plan.covered_pages, pages);
    assert_eq!(plan.placements.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [90, 87, 87, 3, 3]);
    assert_eq!(plan.placements.iter().map(|row| row.top.micrometres())
        .collect::<Vec<_>>(), [100, 500, 510, 700, 710]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);

    let narrow_pair = [page(1, 0, 100, 30), page(2, 100, 10, 20)];
    let narrower_units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(90, 20, 20)]),
        unit(FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible,
            vec![fragment(1, 20, 10), fragment(2, 10, 10),
                fragment(3, 10, 10)]),
    ];
    let outcome = paginate(&narrow_pair, &narrower_units);
    assert!(outcome.is_ok(), "pair too wide: {outcome:?}");
    let Ok(plan) = outcome else { return; };
    assert_eq!(plan.placements.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 1, 2, 2]);
    assert_placement_source_bijection(
        &narrow_pair, &narrower_units, &plan.placements);
}

#[test]
fn balanced_tail_moves_one_long_paragraph_row_without_stranding_its_start() {
    let pages = [page(17, 100, 100, 70), page(3, 500, 100, 70)];
    let rows = vec![
        fragment(1, 40, 20), fragment(2, 40, 20),
        fragment(3, 40, 20), fragment(4, 40, 20),
    ];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(9, 40, 10)]),
        unit(FlowUnitPolicy::KeepTogetherAndBalanceTailWhenPossible,
            rows.clone()),
        unit(FlowUnitPolicy::Independent, vec![fragment(10, 40, 30)]),
    ];
    let outcome1 = paginate(&pages, &units);
    assert!(outcome1.is_ok(), "expected pagination: {outcome1:?}");
    let Ok(plan) = outcome1 else { return; };
    assert_eq!(plan.covered_pages, pages);
    assert_eq!(plan.placements.iter().map(|placed| placed.page)
        .collect::<Vec<_>>(), [17, 17, 17, 3, 3, 3]);
    assert_eq!(plan.placements.iter().map(|placed| placed.top.micrometres())
        .collect::<Vec<_>>(), [100, 110, 130, 500, 520, 540]);
    assert_eq!(plan.placements.iter().map(|placed| placed.owner)
        .collect::<Vec<_>>(), [9, 1, 2, 3, 4, 10]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);

    let baseline_units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(9, 40, 10)]),
        unit(FlowUnitPolicy::KeepTogetherWhenPossible, rows),
        unit(FlowUnitPolicy::Independent, vec![fragment(10, 40, 30)]),
    ];
    let outcome2 = paginate(&pages, &baseline_units);
    assert!(outcome2.is_ok(), "expected pagination: {outcome2:?}");
    let Ok(unbalanced) = outcome2 else { return; };
    assert_eq!(unbalanced.placements.iter().map(|placed| placed.page)
        .collect::<Vec<_>>(), [17, 17, 17, 17, 3, 3]);
}

#[test]
fn balanced_tail_keeps_two_starting_rows_and_honors_later_page_limits() {
    let policy = FlowUnitPolicy::KeepTogetherAndBalanceTailWhenPossible;
    let samples = [
        (page(2, 100, 100, 20), [1, 1, 1, 2]),
        (page(2, 100, 10, 80), [1, 1, 1, 2]),
    ];
    for (last_page, expected) in samples {
        let pages = [page(1, 0, 100, 60), last_page];
        let units = [unit(policy, vec![
            fragment(1, 40, 20), fragment(2, 40, 20),
            fragment(3, 40, 20), fragment(4, 10, 20),
        ])];
        let outcome1 = paginate(&pages, &units);
        assert!(outcome1.is_ok(), "expected pagination: {outcome1:?}");
        let Ok(plan) = outcome1 else { return; };
        assert_eq!(plan.placements.iter().map(|placed| placed.page)
            .collect::<Vec<_>>(), expected);
        assert_placement_source_bijection(&pages, &units, &plan.placements);
    }

    let pages = [page(1, 0, 100, 40), page(2, 100, 100, 60)];
    let units = [unit(policy, vec![
        fragment(1, 40, 20), fragment(2, 40, 20),
        fragment(3, 40, 20),
    ])];
    let outcome2 = paginate(&pages, &units);
    assert!(outcome2.is_ok(), "expected pagination: {outcome2:?}");
    let Ok(plan) = outcome2 else { return; };
    assert_eq!(plan.placements.iter().map(|placed| placed.page)
        .collect::<Vec<_>>(), [2, 2, 2]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
}

#[test]
fn balanced_tail_tracks_skipped_narrow_page_and_unsorted_page_ids() {
    let pages = [
        page(91, 100, 40, 60),
        page(5, 300, 10, 60),
        page(42, 600, 40, 45),
    ];
    let units = [unit(
        FlowUnitPolicy::KeepTogetherAndBalanceTailWhenPossible,
        vec![fragment(1, 25, 20), fragment(2, 25, 20),
            fragment(3, 25, 20), fragment(4, 25, 20)],
    )];
    let outcome1 = paginate(&pages, &units);
    assert!(outcome1.is_ok(), "expected pagination: {outcome1:?}");
    let Ok(plan) = outcome1 else { return; };
    assert_eq!(plan.covered_pages, pages);
    assert_eq!(plan.placements.iter().map(|placed| placed.page)
        .collect::<Vec<_>>(), [91, 91, 42, 42]);
    assert_eq!(plan.placements.iter().map(|placed| placed.top.micrometres())
        .collect::<Vec<_>>(), [100, 120, 600, 620]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
}

#[test]
fn balanced_tail_preserves_complete_groups_and_heading_reservations() {
    let pages = [page(1, 100, 100, 90), page(2, 700, 100, 100)];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 40, 60)]),
        unit(FlowUnitPolicy::KeepWithNextWhenPossible,
            vec![fragment(2, 40, 10)]),
        unit(FlowUnitPolicy::KeepTogetherAndBalanceTailWhenPossible,
            vec![fragment(3, 40, 20), fragment(4, 40, 20),
                fragment(5, 40, 20), fragment(6, 40, 20)]),
    ];
    let outcome1 = paginate(&pages, &units);
    assert!(outcome1.is_ok(), "expected pagination: {outcome1:?}");
    let Ok(plan) = outcome1 else { return; };
    assert_eq!(plan.placements.iter().map(|placed| placed.page)
        .collect::<Vec<_>>(), [1, 2, 2, 2, 2, 2]);
    assert_eq!(plan.placements.iter().map(|placed| placed.top.micrometres())
        .collect::<Vec<_>>(), [100, 700, 710, 730, 750, 770]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
}

#[test]
fn balanced_tail_exhaustive_small_page_reference_and_real_moves() {
    let mut total = 0usize;
    let mut moved = 0usize;
    for first_height in 1..=9 {
        for second_height in 1..=9 {
            let pages = [
                page(11, 50, 20, first_height),
                page(17, 150, 20, second_height),
            ];
            for first_row_height in 1..=3 {
                for second_row_height in 1..=3 {
                    for third_row_height in 1..=3 {
                        for fourth_row_height in 1..=3 {
                            let fragments = vec![
                                fragment(1, 10, first_row_height),
                                fragment(2, 10, second_row_height),
                                fragment(3, 10, third_row_height),
                                fragment(4, 10, fourth_row_height),
                            ];
                            let policy = FlowUnitPolicy::
                                KeepTogetherAndBalanceTailWhenPossible;
                            let units = [unit(policy, fragments.clone())];
                            let actual = paginate(&pages, &units);
                            let expected =
                                reference_mixed_units(&pages, &units);
                            if let (Ok(plan), Ok(expected_rows)) =
                                (&actual, &expected) {
                                assert_placement_source_bijection(
                                    &pages, &units, &plan.placements);
                                assert_eq!(plan.covered_pages,
                                    reference_covered_pages(
                                        &pages, expected_rows));
                                let baseline_units = [unit(
                                    FlowUnitPolicy::KeepTogetherWhenPossible,
                                    fragments,
                                )];
                                if let Ok(baseline) =
                                    paginate(&pages, &baseline_units)
                                    && baseline.placements != plan.placements {
                                    moved += 1;
                                }
                            }
                            assert_eq!(actual.map(|plan| plan.placements),
                                expected, "fixture {total}");
                            total += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(total, 6_561);
    assert!(moved > 50, "oracle must cover balancing outcomes");
}

#[test]
fn balanced_ends_exhaustive_three_page_reference_and_real_moves() {
    use FlowUnitPolicy::{
        KeepTogetherAndBalanceEndsWhenPossible as BalanceEnds,
        KeepTogetherAndBalanceTailWhenPossible as BalanceTail,
    };
    let mut cases = 0usize;
    let mut moved = 0usize;
    let mut preserved = 0usize;
    for first_height in 2..=7 {
        for second_height in 2..=7 {
            for third_height in 2..=7 {
                let pages = [
                    page(31, 100, 20, first_height),
                    page(5, 300, 20, second_height),
                    page(17, 500, 20, third_height),
                ];
                for prefill in 1..=3 {
                    for first in 1..=3 {
                        for second in 1..=3 {
                            for third in 1..=3 {
                                for fourth in 1..=3 {
                                    let rows = vec![
                                        fragment(1, 10, first),
                                        fragment(2, 10, second),
                                        fragment(3, 10, third),
                                        fragment(4, 10, fourth),
                                    ];
                                    let units = [
                                        unit(FlowUnitPolicy::Independent,
                                            vec![fragment(99, 10, prefill)]),
                                        unit(
                                            BalanceEnds,
                                            rows.clone(),
                                        ),
                                    ];
                                    let actual = paginate(&pages, &units);
                                    let oracle =
                                        reference_mixed_units(&pages, &units);
                                    if let (Ok(plan), Ok(expected)) =
                                        (&actual, &oracle) {
                                        assert_placement_source_bijection(
                                            &pages, &units, &plan.placements);
                                        assert_eq!(plan.covered_pages,
                                            reference_covered_pages(
                                                &pages, expected));
                                        let baseline_units = [
                                            unit(
                                                FlowUnitPolicy::Independent,
                                                vec![fragment(
                                                    99, 10, prefill,
                                                )],
                                            ),
                                            unit(
                                                BalanceTail,
                                                rows,
                                            ),
                                        ];
                                        let baseline = paginate(
                                            &pages, &baseline_units);
                                        if let Ok(before) = baseline {
                                            if before.placements
                                                == plan.placements
                                            {
                                                preserved += 1;
                                            } else {
                                                moved += 1;
                                            }
                                        }
                                    }
                                    assert_eq!(
                                        actual.map(|plan| plan.placements),
                                        oracle, "three-page fixture {cases}",
                                    );
                                    cases += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 52_488);
    assert!(moved > 100, "oracle must exercise opening-row moves");
    assert!(preserved > 100, "ineligible pairs retain original layout");
}

#[test]
fn explicit_page_start_respects_used_page_and_preserves_source_and_geometry() {
    let pages = [
        page(17, 80, 10, 4),
        page(3, 280, 7, 3),
        page(25, 480, 10, 5),
    ];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 8, 2)]),
        unit(FlowUnitPolicy::StartOnNewPage, vec![
            fragment(2, 8, 2), fragment(3, 8, 2),
        ]),
        unit(FlowUnitPolicy::Independent, vec![fragment(4, 8, 1)]),
    ];
    let plan = paginate(&pages, &units).expect("explicit page start");
    assert_eq!(plan.placements.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [17, 25, 25, 25]);
    assert_eq!(plan.placements.iter().map(|row| row.top.micrometres())
        .collect::<Vec<_>>(), [80, 480, 482, 484]);
    assert_eq!(plan.covered_pages, pages);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
}

#[test]
fn explicit_page_start_does_not_waste_initial_or_empty_pages() {
    let pages = [page(7, 50, 8, 2), page(9, 150, 8, 2)];
    let units = [
        unit(FlowUnitPolicy::StartOnNewPage, vec![]),
        unit(FlowUnitPolicy::StartOnNewPage, vec![fragment(11, 4, 1)]),
        unit(FlowUnitPolicy::StartOnNewPage, vec![]),
        unit(FlowUnitPolicy::StartOnNewPage, vec![fragment(12, 4, 1)]),
    ];
    let plan = paginate(&pages, &units).expect("nonempty break units");
    assert_eq!(plan.placements.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [7, 9]);
    assert_eq!(plan.placements.iter().map(|row| row.top.micrometres())
        .collect::<Vec<_>>(), [50, 150]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
}

#[test]
fn mandatory_break_blocks_keep_with_next_across_the_boundary() {
    let pages = [
        page(11, 20, 8, 2),
        page(22, 60, 8, 2),
        page(33, 100, 8, 2),
    ];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 3, 1)]),
        unit(FlowUnitPolicy::KeepWithNextWhenPossible,
            vec![fragment(2, 3, 1)]),
        unit(FlowUnitPolicy::StartOnNewPage, vec![fragment(3, 3, 1)]),
    ];
    let plan = paginate(&pages, &units).expect("heading cannot cross break");
    assert_eq!(plan.placements.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [11, 11, 22]);
    assert_eq!(plan.covered_pages, pages[..2]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
}

#[test]
fn explicit_break_refuses_without_an_available_fresh_sheet() {
    let pages = [page(1, 10, 8, 5)];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 4, 1)]),
        unit(FlowUnitPolicy::StartOnNewPage, vec![fragment(2, 4, 1)]),
    ];
    assert_eq!(paginate(&pages, &units),
        Err(PaginationError::NoPageAvailable { owner: 2 }));
    let initial = [unit(FlowUnitPolicy::StartOnNewPage,
        vec![fragment(7, 4, 1)])];
    assert_eq!(paginate(&pages, &initial).unwrap().placements[0].page, 1);
}

#[test]
fn independent_rescue_preserves_mandatory_break_after_advisory_keep_group() {
    let pages = [
        page(71, 100, 10, 3),
        page(5, 300, 10, 3),
        page(92, 500, 10, 4),
    ];
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(1, 3, 1)]),
        unit(FlowUnitPolicy::KeepTogetherWhenPossible,
            vec![fragment(2, 3, 2), fragment(3, 3, 2)]),
        unit(FlowUnitPolicy::StartOnNewPage, vec![fragment(4, 3, 1)]),
    ];
    let plan = paginate(&pages, &units)
        .expect("independent replay must rescue without discarding break");
    assert_eq!(plan.placements.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [71, 71, 5, 92]);
    assert_eq!(plan.placements.iter().map(|row| row.top.micrometres())
        .collect::<Vec<_>>(), [100, 101, 300, 500]);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
    assert_eq!(plan.covered_pages, pages);
}

#[test]
fn explicit_page_start_matches_independent_reference_across_physical_shapes() {
    let mut cases = 0;
    let mut accepted = 0;
    let mut refused = 0;
    for first_height in 1..=3 {
        for second_height in 1..=3 {
            for first_width in 1..=3 {
                for second_width in 1..=3 {
                    let pages = [
                        page(9, 100, first_width, first_height),
                        page(2, 400, second_width, second_height),
                    ];
                    for before in 1..=3 {
                        for after in 1..=3 {
                            for after_width in 1..=3 {
                                let units = [
                                    unit(FlowUnitPolicy::Independent,
                                        vec![fragment(4, 1, before)]),
                                    unit(FlowUnitPolicy::StartOnNewPage,
                                        vec![fragment(5, after_width, after)]),
                                ];
                                let actual = paginate(&pages, &units);
                                let expected = reference_mixed_units(
                                    &pages, &units);
                                if let (Ok(plan), Ok(oracle)) =
                                    (&actual, &expected) {
                                    accepted += 1;
                                    assert_placement_source_bijection(
                                        &pages, &units, &plan.placements);
                                    assert_eq!(plan.covered_pages,
                                        reference_covered_pages(
                                            &pages, oracle));
                                } else {
                                    refused += 1;
                                }
                                assert_eq!(
                                    actual.map(|p| p.placements), expected,
                                    "physical break case {cases}");
                                cases += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 2_187);
    assert!(accepted > 0);
    assert!(refused > 0);
}

#[test]
fn explicit_page_starts_use_all_thirty_two_supplied_sheets_without_reuse() {
    let pages = (0..32).map(|index| page(
        100 - index, index * 100, 10, 5,
    )).collect::<Vec<_>>();
    let mut units = (0..32).map(|index| unit(
        FlowUnitPolicy::StartOnNewPage,
        vec![fragment(index + 1, 4, 1)],
    )).collect::<Vec<_>>();
    let plan = paginate(&pages, &units).expect("32 independent section starts");
    assert_eq!(plan.covered_pages, pages);
    assert_eq!(plan.placements.len(), 32);
    for (index, row) in plan.placements.iter().enumerate() {
        assert_eq!(row.page, pages[index].page);
        assert_eq!(row.top, pages[index].writable.y);
        assert_eq!(row.source, FragmentSource { unit: index, fragment: 0 });
    }
    assert_placement_source_bijection(&pages, &units, &plan.placements);
    units.push(unit(FlowUnitPolicy::StartOnNewPage,
        vec![fragment(33, 4, 1)]));
    assert_eq!(paginate(&pages, &units),
        Err(PaginationError::NoPageAvailable { owner: 33 }));
}

#[test]
fn opening_balance_fallback_cannot_erase_explicit_page_boundary() {
    let pages = [
        page(19, 100, 100, 60),
        page(4, 300, 100, 40),
        page(83, 700, 100, 30),
    ];
    let rows = (0..5).map(|index| fragment(index + 1, 30, 10))
        .collect::<Vec<_>>();
    let units = [
        unit(FlowUnitPolicy::Independent, vec![fragment(90, 30, 45)]),
        unit(FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible,
            rows.clone()),
        unit(FlowUnitPolicy::StartOnNewPage, vec![fragment(91, 30, 1)]),
    ];
    let plan = paginate(&pages, &units)
        .expect("tail-only replay preserves mandatory later break");
    assert_eq!(plan.placements.iter().map(|placed| placed.page)
        .collect::<Vec<_>>(), [19, 19, 4, 4, 4, 4, 83]);
    assert_eq!(plan.placements.iter().map(
        |placed| placed.top.micrometres()).collect::<Vec<_>>(),
        [100, 145, 300, 310, 320, 330, 700]);
    assert_eq!(plan.covered_pages, pages);
    assert_placement_source_bijection(&pages, &units, &plan.placements);
    assert_eq!(reference_mixed_units(&pages, &units),
        Ok(plan.placements));

    let preferred_without_break = [
        unit(FlowUnitPolicy::Independent, vec![fragment(90, 30, 45)]),
        unit(FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible, rows),
        unit(FlowUnitPolicy::Independent, vec![fragment(91, 30, 1)]),
    ];
    let alternate = paginate(&pages, &preferred_without_break)
        .expect("advisory opening balance can put tail on last page");
    assert_eq!(alternate.placements[1].page, 4);
    assert_eq!(alternate.placements[5].page, 83);
    assert_eq!(alternate.placements[6].page, 83);
}
