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

use atrament_physical_page_profile::{Length, Rect};

/// Policy controlling whether adjacent measured fragments prefer one page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowUnitPolicy {
    /// Paginate each measured fragment independently in source order.
    Independent,
    /// Keep all fragments on one page when one remaining page can contain them.
    KeepTogetherWhenPossible,
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

/// One page-bound vertical placement of an unchanged measured fragment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlacedFragment<Identity, PageIdentity> {
    /// Unchanged measured fragment height.
    pub height: Length,
    /// Stable semantic owner preserved from measurement.
    pub owner: Identity,
    /// Stable semantic page receiving this fragment.
    pub page: PageIdentity,
    /// Exact absolute physical top coordinate in oriented sheet space.
    pub top: Length,
    /// Unchanged measured fragment width.
    pub width: Length,
}

/// Complete vertical flow pagination result in semantic reading order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginationPlan<Identity, PageIdentity> {
    /// Ordered placements; one output exists for every measured fragment.
    pub placements: Vec<PlacedFragment<Identity, PageIdentity>>,
}

/// Typed failure to paginate already-measured semantic flow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaginationError<Identity, PageIdentity> {
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
}

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

/// Paginate immutable measured fragments over the supplied page sequence.
///
/// `KeepTogetherWhenPossible` moves a complete unit to the earliest remaining
/// fresh page that can contain it when the current remainder cannot. If no
/// single remaining page can contain the whole unit, the unit falls back to
/// fragment-boundary pagination without splitting any fragment.
///
/// # Errors
///
/// Returns a typed failure for invalid writable regions, an indivisible
/// fragment that cannot fit any remaining page, or exhaustion after a fragment
/// would fit only on a fresh additional page.
pub fn paginate<Identity, PageIdentity>(
    pages: &[PageRegion<PageIdentity>],
    units: &[MeasuredFlowUnit<Identity>],
) -> Result<
    PaginationPlan<Identity, PageIdentity>,
    PaginationError<Identity, PageIdentity>,
>
where
    Identity: Copy,
    PageIdentity: Copy,
{
    validate_pages(pages)?;
    let mut cursor = Cursor {
        page_index: 0,
        used_height: Length::ZERO,
    };
    let mut placements = Vec::new();
    for unit in units {
        if unit.fragments.is_empty() {
            continue;
        }
        match unit.policy {
            FlowUnitPolicy::Independent => {
                place_independent(
                    pages,
                    &unit.fragments,
                    &mut cursor,
                    &mut placements,
                )?;
            },
            FlowUnitPolicy::KeepTogetherWhenPossible => {
                place_keep_together_when_possible(
                    pages,
                    &unit.fragments,
                    &mut cursor,
                    &mut placements,
                )?;
            },
        }
    }
    Ok(PaginationPlan { placements })
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
    cursor: &mut Cursor,
    placements: &mut Vec<PlacedFragment<Identity, PageIdentity>>,
) -> Result<(), PaginationError<Identity, PageIdentity>>
where
    Identity: Copy,
    PageIdentity: Copy,
{
    for fragment in fragments {
        place_one(pages, *fragment, cursor, placements)?;
    }
    Ok(())
}

fn place_keep_together_when_possible<Identity, PageIdentity>(
    pages: &[PageRegion<PageIdentity>],
    fragments: &[MeasuredFragment<Identity>],
    cursor: &mut Cursor,
    placements: &mut Vec<PlacedFragment<Identity, PageIdentity>>,
) -> Result<(), PaginationError<Identity, PageIdentity>>
where
    Identity: Copy,
    PageIdentity: Copy,
{
    let Some(extent) = complete_group_extent(fragments) else {
        return place_independent(pages, fragments, cursor, placements);
    };
    if let Some(current) = pages.get(cursor.page_index) {
        let remaining = remaining_height(current.writable, cursor.used_height);
        if group_fits_region(extent, current.writable, remaining) {
            for fragment in fragments {
                place_on_current(*fragment, *current, cursor, placements)?;
            }
            return Ok(());
        }
    }

    let Some(start_index) = cursor.page_index.checked_add(1) else {
        return place_independent(pages, fragments, cursor, placements);
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
        for fragment in fragments {
            place_on_current(*fragment, *page, cursor, placements)?;
        }
        return Ok(());
    }

    place_independent(pages, fragments, cursor, placements)
}

fn place_on_current<Identity, PageIdentity>(
    fragment: MeasuredFragment<Identity>,
    page: PageRegion<PageIdentity>,
    cursor: &mut Cursor,
    placements: &mut Vec<PlacedFragment<Identity, PageIdentity>>,
) -> Result<(), PaginationError<Identity, PageIdentity>>
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
        top: Length::from_micrometres(top_micrometres),
        width: fragment.width,
    });
    cursor.used_height = Length::from_micrometres(next_used_height);
    Ok(())
}

fn place_one<Identity, PageIdentity>(
    pages: &[PageRegion<PageIdentity>],
    fragment: MeasuredFragment<Identity>,
    cursor: &mut Cursor,
    placements: &mut Vec<PlacedFragment<Identity, PageIdentity>>,
) -> Result<(), PaginationError<Identity, PageIdentity>>
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
                return place_on_current(fragment, *page, cursor, placements);
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
) -> Result<(), PaginationError<Identity, PageIdentity>>
where
    PageIdentity: Copy,
{
    for page in pages {
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
