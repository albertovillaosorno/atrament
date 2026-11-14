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
//   - Deterministic page advancement for already-measured flow fragments.
// - Must-Not:
//   - Shape text, choose line breaks, mutate semantic state, place fixed
//     objects, choose horizontal alignment, or render pixels.
// - Allows:
//   - Inputs: Measured semantic fragments and exact writable page regions.
//   - Outputs: Ordered page identities and exact vertical physical positions.
//   - Side effects: Process-local result allocation only.
// - Split-When:
//   - Column balancing or fragment measurement becomes independently complex.
// - Merge-When:
//   - Flow pagination stops existing independently from measured layout.
// - Summary:
//   - Paginates measured semantic fragments without inventing glyph metrics.
// - Description:
//   - Preserves fragment order and keep-together intent across physical pages.
// - Usage:
//   - Measure upstream, supply page regions, then paginate immutable fragments.
// - Defaults:
//   - A fragment is indivisible; groups keep together only when one page fits.
//

//! Deterministic pagination for already-measured semantic flow fragments.

use std::collections::BTreeSet;

use atrament_physical_page_profile::{Length, Rect};

/// Policy controlling whether adjacent measured fragments prefer one page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowUnitPolicy {
    /// Paginate each measured fragment independently in source order.
    Independent,
    /// Keep an opening pair on one page if splitting strands the first row;
    /// also balance a lone final row when physically possible.
    KeepTogetherAndBalanceEndsWhenPossible,
    /// Keep a full unit together when possible; otherwise avoid a lone
    /// final fragment on a fresh page when at least two rows remain behind.
    KeepTogetherAndBalanceTailWhenPossible,
    /// Keep all fragments on one page when one remaining page can contain them.
    KeepTogetherWhenPossible,
    /// Prefer the complete unit with its next visible measured fragment.
    /// Consecutive keep-with-next units form a chain.
    KeepWithNextWhenPossible,
    /// Begin this nonempty unit on a fresh physical page when earlier rows
    /// have consumed the current page. Unlike keep preferences, this is a
    /// mandatory boundary that independent recovery must preserve.
    StartOnNewPage,
}

/// One already-measured indivisible semantic flow fragment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeasuredFragment<Identity> {
    /// Exact vertical physical advance owned by upstream measurement.
    pub height: Length,
    /// Stable semantic owner whose measured geometry this fragment represents.
    pub owner: Identity,
    /// Exact maximum horizontal physical extent from upstream measurement.
    pub width: Length,
}

/// One ordered unit of already-measured semantic fragments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeasuredFlowUnit<Identity> {
    /// Measured fragments in semantic reading order.
    pub fragments: Vec<MeasuredFragment<Identity>>,
    /// Page-break preference for this unit.
    pub policy: FlowUnitPolicy,
}

/// One available page and its exact writable physical region.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageRegion<PageIdentity> {
    /// Stable semantic page identity.
    pub page: PageIdentity,
    /// Exact writable rectangle derived from the page profile.
    pub writable: Rect,
}

/// Exact source location in the caller's measured flow units.
///
/// Empty units keep their input index; identical block owners in separate
/// fragments remain distinct across page breaks and keep-together moves.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FragmentSource {
    /// Original zero-based fragment index within that unit.
    pub fragment: usize,
    /// Original zero-based unit index.
    pub unit: usize,
}

/// One page-bound vertical placement of an unchanged measured fragment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlacedFragment<Identity, PageIdentity> {
    /// Unchanged measured fragment height.
    pub height: Length,
    /// Stable semantic owner preserved from measurement.
    pub owner: Identity,
    /// Stable semantic page receiving this fragment.
    pub page: PageIdentity,
    /// Source location of this unchanged measured fragment.
    pub source: FragmentSource,
    /// Exact absolute physical top coordinate in oriented sheet space.
    pub top: Length,
    /// Unchanged measured fragment width.
    pub width: Length,
    /// Exact page writable rectangle, not this fragment's measured extent.
    /// Its origin is physical page geometry, not text alignment.
    pub writable: Rect,
}

/// Complete vertical flow pagination result in semantic reading order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginationPlan<Identity, PageIdentity> {
    /// Ordered page prefix through the last placed fragment, including
    /// skipped pages with no ink from this flow. Does not include trailing
    /// unused pages or pages preceding the supplied region sequence.
    pub covered_pages: Vec<PageRegion<PageIdentity>>,
    /// Ordered placements; one output exists for every measured fragment.
    pub placements: Vec<PlacedFragment<Identity, PageIdentity>>,
}

/// Typed failure to paginate already-measured semantic flow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaginationError<Identity, PageIdentity> {
    /// Two page regions share one semantic identity, aliasing placements.
    DuplicatePageIdentity {
        /// Repeated semantic page identity.
        page: PageIdentity,
    },
    /// No remaining page can contain this indivisible measured fragment.
    FragmentDoesNotFitAnyPage {
        /// Stable semantic owner of the unplaceable fragment.
        owner: Identity,
    },
    /// One supplied writable page rectangle is empty or arithmetically invalid.
    InvalidPageRegion {
        /// Stable semantic page whose writable region is invalid.
        page: PageIdentity,
    },
    /// A fragment could fit a fresh page, but no later page remains available.
    NoPageAvailable {
        /// Stable semantic owner waiting for another page.
        owner: Identity,
    },
    /// A measured fragment has no vertical advance and would overlap ink.
    ZeroHeightFragment {
        /// Stable semantic owner of the non-advancing fragment.
        owner: Identity,
    },
}

/// Result of paginating a complete measured flow.
pub type PaginationResult<Identity, PageIdentity> = Result<
    PaginationPlan<Identity, PageIdentity>,
    PaginationError<Identity, PageIdentity>,
>;

type PlacementResult<Identity, PageIdentity> =
    Result<(), PaginationError<Identity, PageIdentity>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Cursor {
    page_index: usize,
    used_height: Length,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GroupExtent {
    height: Length,
    width: Length,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum PaginationMode {
    IndependentFallback,
    Preferred,
    TailOnly,
}

/// Paginate immutable measured fragments over the supplied page sequence.
///
/// `StartOnNewPage` is a mandatory boundary before a nonempty unit when the
/// current sheet already contains placed flow. An unused initial or skipped
/// page is not discarded, and empty units cannot request a blank page.
/// Unlike advisory grouping, explicit breaks survive fallback replay and
/// refuse with `NoPageAvailable` if no later sheet remains.
///
/// `KeepTogetherWhenPossible` moves a complete unit to the earliest remaining
/// fresh page that can contain it when the current remainder cannot. If no
/// single remaining page can contain the whole unit, the unit falls back to
/// fragment-boundary pagination without splitting any fragment.
///
/// `KeepTogetherAndBalanceTailWhenPossible` also moves the penultimate
/// measured row to the last page of a split group when exactly one final row
/// starts that page, at least three rows from that unit ended on the prior
/// page, and both final rows fit the last page's exact writable geometry.
/// Otherwise the original fragment placement is preserved.
///
/// `KeepTogetherAndBalanceEndsWhenPossible` also tries moving the first
/// fragment to the second fragment's fresh sheet if those first two rows fit
/// there and the remaining measured unit still paginates successfully. A
/// preference that would exhaust the remaining pages is discarded without
/// changing the original successful placement or its cursor. If a later unit
/// then fails, the complete flow is replayed once without opening preferences;
/// a valid tail-only layout wins over such speculative page exhaustion.
/// If a keep-with-next or keep-together preference still exhausts pages, one
/// final independent-fragment replay can rescue the otherwise valid flow.
/// These replays never replace a successful preferred plan or its identities.
///
/// When every supplied unit has no measured fragments, pagination returns an
/// empty plan without consulting page authority because no placement exists.
///
/// # Errors
///
/// Returns a typed failure for invalid writable regions, duplicate page
/// identities, zero-height fragments, an indivisible fragment that cannot fit
/// any remaining page, or exhaustion when no later page remains available.
pub fn paginate<Identity, PageIdentity>(
    pages: &[PageRegion<PageIdentity>],
    units: &[MeasuredFlowUnit<Identity>],
) -> PaginationResult<Identity, PageIdentity>
where
    Identity: Copy,
    PageIdentity: Copy + Ord,
{
    let proposed = paginate_once(pages, units, PaginationMode::Preferred);
    if !matches!(&proposed,
        Err(PaginationError::NoPageAvailable { .. }
            | PaginationError::FragmentDoesNotFitAnyPage { .. }))
    {
        return proposed;
    }
    if units.iter().any(|unit| {
        !unit.fragments.is_empty()
            && unit.policy
                == FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible
    })
        && let Ok(fallback) = paginate_once(
            pages, units, PaginationMode::TailOnly,
        )
    {
        return Ok(fallback);
    }
    if units.iter().any(|unit| !unit.fragments.is_empty()
        && !matches!(unit.policy,
            FlowUnitPolicy::Independent | FlowUnitPolicy::StartOnNewPage))
        && let Ok(fallback) = paginate_once(
            pages, units, PaginationMode::IndependentFallback,
        )
    {
        return Ok(fallback);
    }
    proposed
}

/// Replay bounded advisory preferences when later fragments run out of pages.
/// Independent fallback retains the same checked page and fragment authority.
fn paginate_once<Identity, PageIdentity>(
    pages: &[PageRegion<PageIdentity>],
    units: &[MeasuredFlowUnit<Identity>],
    mode: PaginationMode,
) -> PaginationResult<Identity, PageIdentity>
where
    Identity: Copy,
    PageIdentity: Copy + Ord,
{
    if units.iter().all(|unit| unit.fragments.is_empty()) {
        return Ok(PaginationPlan {
            covered_pages: Vec::new(),
            placements: Vec::new(),
        });
    }
    validate_pages(pages)?;
    for unit in units {
        for fragment in &unit.fragments {
            if fragment.height == Length::ZERO {
                return Err(PaginationError::ZeroHeightFragment {
                    owner: fragment.owner,
                });
            }
        }
    }
    let keep_reservations = if mode == PaginationMode::IndependentFallback {
        Vec::new()
    } else {
        keep_with_next_reservations(units)
    };
    let mut cursor = Cursor {
        page_index: 0,
        used_height: Length::ZERO,
    };
    let mut placements = Vec::new();
    for (unit_index, unit) in units.iter().enumerate() {
        if unit.fragments.is_empty() {
            continue;
        }
        if mode == PaginationMode::IndependentFallback
            && unit.policy != FlowUnitPolicy::StartOnNewPage
        {
            place_independent(
                pages, &unit.fragments, unit_index,
                &mut cursor, &mut placements,
            )?;
            continue;
        }
        match unit.policy {
            FlowUnitPolicy::StartOnNewPage => place_on_fresh_page(
                pages, &unit.fragments, unit_index,
                &mut cursor, &mut placements,
            )?,
            FlowUnitPolicy::Independent => {
                place_independent(
                    pages,
                    &unit.fragments,
                    unit_index,
                    &mut cursor,
                    &mut placements,
                )?;
            },
            FlowUnitPolicy::KeepTogetherWhenPossible => {
                place_keep_together_when_possible(
                    pages,
                    &unit.fragments,
                    unit_index,
                    &mut cursor,
                    &mut placements,
                )?;
            },
            FlowUnitPolicy::KeepWithNextWhenPossible => {
                if let Some(Some(extent)) = keep_reservations.get(unit_index) {
                    prefer_group_page(pages, *extent, &mut cursor);
                }
                place_independent(
                    pages, &unit.fragments, unit_index,
                    &mut cursor, &mut placements,
                )?;
            },
            FlowUnitPolicy::KeepTogetherAndBalanceTailWhenPossible
            | FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible => {
                let first_placement = placements.len();
                place_keep_together_when_possible(
                    pages, &unit.fragments, unit_index,
                    &mut cursor, &mut placements,
                )?;
                if mode == PaginationMode::Preferred && unit.policy
                    == FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible
                {
                    balance_lone_opening_fragment(
                        pages, &unit.fragments, unit_index, first_placement,
                        &mut cursor, &mut placements,
                    );
                }
                balance_lone_trailing_fragment(
                    pages, first_placement, &mut cursor, &mut placements,
                );
            },
        }
    }
    Ok(PaginationPlan {
        covered_pages: pages.iter()
            .take(cursor.page_index.saturating_add(1))
            .copied()
            .collect(),
        placements,
    })
}

const fn advance_cursor(cursor: &mut Cursor) -> bool {
    let Some(next) = cursor.page_index.checked_add(1) else {
        return false;
    };
    cursor.page_index = next;
    cursor.used_height = Length::ZERO;
    true
}

fn complete_group_extent<Identity>(
    fragments: &[MeasuredFragment<Identity>],
) -> Option<GroupExtent> {
    let mut height = 0u64;
    let mut width = Length::ZERO;
    for fragment in fragments {
        height = height.checked_add(fragment.height.micrometres())?;
        width = width.max(fragment.width);
    }
    Some(GroupExtent {
        height: Length::from_micrometres(height),
        width,
    })
}

/// Precompute the complete keep-with-next preference for each measured unit.
/// Reverse traversal keeps long heading chains linear in source size, rather
/// than rescanning all following units for each heading. Empty units are inert.
fn keep_with_next_reservations<Identity>(
    units: &[MeasuredFlowUnit<Identity>],
) -> Vec<Option<GroupExtent>> {
    let mut reversed = Vec::with_capacity(units.len());
    let mut following: Option<GroupExtent> = None;
    for unit in units.iter().rev() {
        if unit.fragments.is_empty() {
            reversed.push(None);
            continue;
        }
        let reserved = match unit.policy {
            FlowUnitPolicy::KeepWithNextWhenPossible => {
                complete_group_extent(&unit.fragments)
                    .zip(following)
                    .and_then(|(own, next)| {
                        own.height.micrometres()
                            .checked_add(next.height.micrometres())
                            .map(|height| GroupExtent {
                                height: Length::from_micrometres(height),
                                width: own.width.max(next.width),
                            })
                    })
            },
            FlowUnitPolicy::KeepTogetherWhenPossible
            | FlowUnitPolicy::KeepTogetherAndBalanceTailWhenPossible
            | FlowUnitPolicy::KeepTogetherAndBalanceEndsWhenPossible => {
                complete_group_extent(&unit.fragments)
            },
            FlowUnitPolicy::StartOnNewPage => None,
            FlowUnitPolicy::Independent => unit.fragments.first()
                .map(|fragment| GroupExtent {
                    height: fragment.height,
                    width: fragment.width,
                }),
        };
        let requested =
            if unit.policy == FlowUnitPolicy::KeepWithNextWhenPossible {
                reserved
            } else {
                None
            };
        reversed.push(requested);
        following = reserved;
    }
    reversed.reverse();
    reversed
}

/// Find the earliest later full fit, never spending pages on an
/// impossible preference. All caller fragments retain their own indices.
fn prefer_group_page<PageIdentity>(
    pages: &[PageRegion<PageIdentity>],
    extent: GroupExtent, cursor: &mut Cursor,
) {
    if pages.get(cursor.page_index).is_some_and(|page| {
        group_fits_region(extent, page.writable,
            remaining_height(page.writable, cursor.used_height))
    }) { return; }
    if let Some((index, _)) = cursor.page_index.checked_add(1)
        .and_then(|from| pages.iter().enumerate().skip(from)
            .find(|(_, page)| group_fits_region(
                extent, page.writable, page.writable.height))) {
        cursor.page_index = index;
        cursor.used_height = Length::ZERO;
    }
}

fn fragment_fits_region<Identity>(
    fragment: &MeasuredFragment<Identity>,
    region: Rect,
) -> bool {
    fragment.width <= region.width && fragment.height <= region.height
}

fn group_fits_region(
    extent: GroupExtent,
    region: Rect,
    available_height: Length,
) -> bool {
    extent.height <= available_height && extent.width <= region.width
}

fn place_independent<Identity, PageIdentity>(
    pages: &[PageRegion<PageIdentity>],
    fragments: &[MeasuredFragment<Identity>],
    unit_index: usize,
    cursor: &mut Cursor,
    placements: &mut Vec<PlacedFragment<Identity, PageIdentity>>,
) -> PlacementResult<Identity, PageIdentity>
where
    Identity: Copy,
    PageIdentity: Copy,
{
    for (fragment_index, fragment) in fragments.iter().enumerate() {
        let source = FragmentSource {
            unit: unit_index,
            fragment: fragment_index,
        };
        place_one(pages, *fragment, source, cursor, placements)?;
    }
    Ok(())
}

/// A requested section boundary cannot be removed by a later replay of
/// advisory grouping. Consume another sheet only if current rows exist.
fn place_on_fresh_page<Identity, PageIdentity>(
    pages: &[PageRegion<PageIdentity>],
    fragments: &[MeasuredFragment<Identity>],
    unit_index: usize,
    cursor: &mut Cursor,
    placements: &mut Vec<PlacedFragment<Identity, PageIdentity>>,
) -> PlacementResult<Identity, PageIdentity>
where
    Identity: Copy,
    PageIdentity: Copy,
{
    let Some(first) = fragments.first() else {
        return Ok(());
    };
    if cursor.used_height != Length::ZERO
        && (!advance_cursor(cursor)
            || pages.get(cursor.page_index).is_none())
    {
        return Err(PaginationError::NoPageAvailable {
            owner: first.owner,
        });
    }
    place_independent(pages, fragments, unit_index, cursor, placements)
}

fn place_keep_together_when_possible<Identity, PageIdentity>(
    pages: &[PageRegion<PageIdentity>],
    fragments: &[MeasuredFragment<Identity>],
    unit_index: usize,
    cursor: &mut Cursor,
    placements: &mut Vec<PlacedFragment<Identity, PageIdentity>>,
) -> PlacementResult<Identity, PageIdentity>
where
    Identity: Copy,
    PageIdentity: Copy,
{
    let Some(extent) = complete_group_extent(fragments) else {
        return place_independent(
            pages, fragments, unit_index, cursor, placements,
        );
    };
    if let Some(current) = pages.get(cursor.page_index) {
        let remaining = remaining_height(current.writable, cursor.used_height);
        if group_fits_region(extent, current.writable, remaining) {
            for (fragment_index, fragment) in fragments.iter().enumerate() {
                let source = FragmentSource {
                    unit: unit_index,
                    fragment: fragment_index,
                };
                place_on_current(
                    *fragment, source, *current, cursor, placements,
                )?;
            }
            return Ok(());
        }
    }

    let Some(start_index) = cursor.page_index.checked_add(1) else {
        return place_independent(
            pages, fragments, unit_index, cursor, placements,
        );
    };
    if let Some((page_index, page)) = pages
        .iter()
        .enumerate()
        .skip(start_index)
        .find(|(_, page)| {
            group_fits_region(extent, page.writable, page.writable.height)
        })
    {
        cursor.page_index = page_index;
        cursor.used_height = Length::ZERO;
        for (fragment_index, fragment) in fragments.iter().enumerate() {
            let source = FragmentSource {
                unit: unit_index,
                fragment: fragment_index,
            };
            place_on_current(*fragment, source, *page, cursor, placements)?;
        }
        return Ok(());
    }

    place_independent(pages, fragments, unit_index, cursor, placements)
}

/// A too-long group can put just its first row at the foot of a page.
/// Re-attempt the complete group from the next row's actual fresh page only
/// if its first pair fits there. The initial placement has already succeeded,
/// so a failed preference cannot turn an accepted layout into an error.
/// The trial is isolated until all unchanged rows have been placed.
fn balance_lone_opening_fragment<Identity, PageIdentity>(
    pages: &[PageRegion<PageIdentity>],
    fragments: &[MeasuredFragment<Identity>],
    unit_index: usize,
    first_placement: usize,
    cursor: &mut Cursor,
    placements: &mut Vec<PlacedFragment<Identity, PageIdentity>>,
) where
    Identity: Copy,
    PageIdentity: Copy + Eq,
{
    let Some(rows) = placements.get(first_placement..) else {
        return;
    };
    let Some([first, second, ..]) = rows.get(..) else {
        return;
    };
    if rows.len() < 3 || first.page == second.page {
        return;
    }
    let Some((next_index, next_page)) = pages.iter().enumerate()
        .find(|(_, page)| page.page == second.page) else {
        return;
    };
    let Some(pair) = fragments.get(..2).and_then(complete_group_extent) else {
        return;
    };
    if !group_fits_region(pair, next_page.writable,
        next_page.writable.height) {
        return;
    }
    let mut trial_cursor = Cursor {
        page_index: next_index,
        used_height: Length::ZERO,
    };
    let mut trial = Vec::with_capacity(fragments.len());
    if place_independent(
        pages, fragments, unit_index, &mut trial_cursor, &mut trial,
    ).is_err() {
        return;
    }
    placements.truncate(first_placement);
    placements.extend(trial);
    *cursor = trial_cursor;
}

/// When a too-long group leaves just its final row on a fresh page, move
/// its penultimate row forward if the new sheet can hold both. Require at
/// least three rows of this very unit on the preceding page: the move must
/// never strand a single opening row or reach back into another block.
/// The immutable row measurements and their source indices do not change.
fn balance_lone_trailing_fragment<Identity, PageIdentity>(
    pages: &[PageRegion<PageIdentity>],
    first_placement: usize,
    cursor: &mut Cursor,
    placements: &mut [PlacedFragment<Identity, PageIdentity>],
) where
    Identity: Copy,
    PageIdentity: Copy + Eq,
{
    let Some(placed) = placements.get(first_placement..) else {
        return;
    };
    if placed.len() < 4 {
        return;
    }
    let Some(tail_start) = placed.len().checked_sub(4) else {
        return;
    };
    let Some([older, previous, penultimate, final_row]) =
        placed.get(tail_start..) else {
        return;
    };
    let Some(current) = pages.get(cursor.page_index) else {
        return;
    };
    if older.page != penultimate.page
        || previous.page != penultimate.page
        || penultimate.page == final_row.page
        || final_row.page != current.page
        || final_row.top != current.writable.y
        || cursor.used_height != final_row.height
        || penultimate.width > current.writable.width
    {
        return;
    }
    let Some(pair_height) = penultimate.height.micrometres()
        .checked_add(final_row.height.micrometres()) else {
        return;
    };
    if pair_height > current.writable.height.micrometres() {
        return;
    }
    let Some(last_top) = current.writable.y.micrometres()
        .checked_add(penultimate.height.micrometres()) else {
        return;
    };
    let Some(pair_start) = placements.len().checked_sub(2) else {
        return;
    };
    let Some([moved_row, trailing_row]) =
        placements.get_mut(pair_start..) else {
        return;
    };
    moved_row.page = current.page;
    moved_row.top = current.writable.y;
    moved_row.writable = current.writable;
    trailing_row.top = Length::from_micrometres(last_top);
    cursor.used_height = Length::from_micrometres(pair_height);
}

fn place_on_current<Identity, PageIdentity>(
    fragment: MeasuredFragment<Identity>,
    source: FragmentSource,
    page: PageRegion<PageIdentity>,
    cursor: &mut Cursor,
    placements: &mut Vec<PlacedFragment<Identity, PageIdentity>>,
) -> PlacementResult<Identity, PageIdentity>
where
    Identity: Copy,
    PageIdentity: Copy,
{
    let Some(top_micrometres) = page
        .writable
        .y
        .micrometres()
        .checked_add(cursor.used_height.micrometres())
    else {
        return Err(PaginationError::InvalidPageRegion { page: page.page });
    };
    let Some(next_used_height) = cursor
        .used_height
        .micrometres()
        .checked_add(fragment.height.micrometres())
    else {
        return Err(PaginationError::InvalidPageRegion { page: page.page });
    };
    placements.push(PlacedFragment {
        height: fragment.height,
        owner: fragment.owner,
        page: page.page,
        source,
        top: Length::from_micrometres(top_micrometres),
        width: fragment.width,
        writable: page.writable,
    });
    cursor.used_height = Length::from_micrometres(next_used_height);
    Ok(())
}

fn place_one<Identity, PageIdentity>(
    pages: &[PageRegion<PageIdentity>],
    fragment: MeasuredFragment<Identity>,
    source: FragmentSource,
    cursor: &mut Cursor,
    placements: &mut Vec<PlacedFragment<Identity, PageIdentity>>,
) -> PlacementResult<Identity, PageIdentity>
where
    Identity: Copy,
    PageIdentity: Copy,
{
    let mut fits_fresh_page = false;
    loop {
        let Some(page) = pages.get(cursor.page_index) else {
            return if fits_fresh_page {
                Err(PaginationError::NoPageAvailable {
                    owner: fragment.owner,
                })
            } else {
                Err(PaginationError::FragmentDoesNotFitAnyPage {
                    owner: fragment.owner,
                })
            };
        };
        if fragment_fits_region(&fragment, page.writable) {
            fits_fresh_page = true;
            let remaining =
                remaining_height(page.writable, cursor.used_height);
            if fragment.height <= remaining {
                return place_on_current(
                    fragment, source, *page, cursor, placements,
                );
            }
        }
        if !advance_cursor(cursor) {
            return if fits_fresh_page {
                Err(PaginationError::NoPageAvailable {
                    owner: fragment.owner,
                })
            } else {
                Err(PaginationError::FragmentDoesNotFitAnyPage {
                    owner: fragment.owner,
                })
            };
        }
    }
}

const fn remaining_height(region: Rect, used_height: Length) -> Length {
    let remaining = region
        .height
        .micrometres()
        .saturating_sub(used_height.micrometres());
    Length::from_micrometres(remaining)
}

fn validate_pages<Identity, PageIdentity>(
    pages: &[PageRegion<PageIdentity>],
) -> PlacementResult<Identity, PageIdentity>
where
    PageIdentity: Copy + Ord,
{
    let mut seen = BTreeSet::new();
    for page in pages {
        if !seen.insert(page.page) {
            return Err(PaginationError::DuplicatePageIdentity {
                page: page.page,
            });
        }
        let region = page.writable;
        let horizontal_end = region
            .x
            .micrometres()
            .checked_add(region.width.micrometres());
        let vertical_end = region
            .y
            .micrometres()
            .checked_add(region.height.micrometres());
        if region.width == Length::ZERO
            || region.height == Length::ZERO
            || horizontal_end.is_none()
            || vertical_end.is_none()
        {
            return Err(PaginationError::InvalidPageRegion { page: page.page });
        }
    }
    Ok(())
}
