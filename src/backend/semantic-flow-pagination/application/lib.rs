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
//   - Accepted-revision and accepted-flow binding for measured pagination.
//   - Derivation of writable page regions from accepted page-profile ownership.
//   - Completeness and order admission for measured top-level flow blocks.
//   - Conservative vertical-band inspection of supplied checked fixed regions.
//   - Optional caller-positioned measured-row footprint intersection reports.
//   - Bounded pair inspection of physically intersecting fixed regions.
// - Must-Not:
//   - Measure text, choose fixed overlap policy, mutate revisions, or render.
// - Allows:
//   - Inputs: One accepted revision, measured units, checked fixed bounds.
//   - Inputs: Explicit revision/source-bound horizontal row positions.
//   - Outputs: Typed pagination plan or typed non-mutation failure.
//   - Outputs: Read-only fixed-vs-flow vertical-band conflict result.
//   - Side effects: Process-local result allocation only.
// - Split-When:
//   - Measurement admission or layout diagnostics become independent services.
// - Merge-When:
//   - Semantic page/profile binding moves into a larger layout application.
// - Summary:
//   - Paginates complete revision-bound measurements for one accepted flow.
// - Description:
//   - Prevents stale, reordered, incomplete, or arbitrary geometry from reflow.
// - Usage:
//   - Bind upstream measurements to a revision and flow, then paginate them.
// - Defaults:
//   - Every selected-flow top-level block owns one or more contiguous
//     fragments.
//

//! Accepted-flow binding for deterministic measured-flow pagination.

use std::collections::{BTreeMap, BTreeSet};

use atrament_flow_pagination::{
    FragmentSource, MeasuredFlowUnit, PageRegion, PaginationError,
    PaginationPlan, PlacedFragment, paginate,
};
use atrament_physical_page_profile::{Length, Rect};
use atrament_semantic_fixed_region_layout::FixedRegionLayoutResult;
use atrament_semantic_notebook::{
    AcceptedIdentity, AcceptedRevision, Block, PhysicalPageProfileError,
    RevisionIdentity, semantic_duplicate_owner_among,
};

/// Complete measured-flow plan bound to its accepted semantic authority.
///
/// Downstream consumers must compare `revision()` with the live accepted state
/// before using the derived physical geometry. The `flow()` identifies which
/// accepted flow was measured, even when no fragments were placed. All fields
/// are private so consumers cannot overwrite that validation evidence.
///
/// ```compile_fail
/// use atrament_semantic_flow_pagination::SemanticPaginationPlan;
/// fn forge_revision(mut plan: SemanticPaginationPlan) {
///     plan.revision = plan.revision();
/// }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticPaginationPlan {
    // Keep authority and geometry constructor-controlled: callers may inspect
    // a validated result, but cannot synthesize or change its provenance.
    covered_pages: Vec<PageRegion<AcceptedIdentity>>,
    flow: AcceptedIdentity,
    placements: Vec<PlacedFragment<AcceptedIdentity, AcceptedIdentity>>,
    revision: RevisionIdentity,
}

impl SemanticPaginationPlan {
    /// Ordered accepted page rectangles, including skipped intermediate pages.
    #[must_use]
    pub fn covered_pages(&self) -> &[PageRegion<AcceptedIdentity>] {
        &self.covered_pages
    }

    /// Exact accepted flow that supplied the measured content.
    #[must_use]
    pub const fn flow(&self) -> AcceptedIdentity {
        self.flow
    }

    /// Ordered physical placements in semantic reading order.
    #[must_use]
    pub fn placements(
        &self,
    ) -> &[PlacedFragment<AcceptedIdentity, AcceptedIdentity>] {
        &self.placements
    }

    /// Accepted revision whose geometry and measurements were validated.
    #[must_use]
    pub const fn revision(&self) -> RevisionIdentity {
        self.revision
    }
}

/// Typed reason supplied checked fixed regions cannot share measured row bands.
///
/// The measured flow does not choose horizontal ink alignment, so a vertical
/// overlap is a *possible* collision even when horizontal objects might not
/// intersect. Callers must not mistake an `Ok` result for complete page
/// collision admission: only explicitly supplied fixed regions are inspected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowFixedRegionError {
    /// A checked physical coordinate unexpectedly cannot be represented.
    CoordinateOverflow {
        /// Page with unrepresentable coordinates.
        page: AcceptedIdentity,
        /// Fixed object being inspected.
        object: AcceptedIdentity,
    },
    /// Complete fixed-pair diagnostics would exceed the caller-supplied cap.
    FixedPairLimitExceeded {
        /// Maximum number of complete intersection records admitted.
        limit: usize,
    },
    /// The submitted fixed rectangle exceeds its accepted writable region.
    FixedRegionOverflow {
        /// Accepted physical page of the fixed object.
        page: AcceptedIdentity,
        /// Fixed semantic owner whose checked bounds overflow.
        object: AcceptedIdentity,
    },
    /// Complete positioned row-footprint diagnostics exceed the result cap.
    FixedRowFootprintLimitExceeded {
        /// Maximum admitted complete physical intersection records.
        limit: usize,
    },
    /// Complete fixed-versus-row diagnostics exceed the caller's cap.
    FixedRowLimitExceeded {
        /// Maximum admitted complete vertical-overlap records.
        limit: usize,
    },
    /// Fixed bounds belong to a different semantic revision than the plan.
    RevisionMismatch {
        /// Selected measured pagination revision.
        expected: RevisionIdentity,
        /// Revision used to validate the fixed object.
        fixed: RevisionIdentity,
        /// Stable fixed semantic owner.
        object: AcceptedIdentity,
    },
    /// One positioned measured-row footprint escapes its writable page.
    RowFootprintOverflow {
        /// Accepted page of the measured row.
        page: AcceptedIdentity,
        /// Stable semantic owner of the measured row.
        fragment_owner: AcceptedIdentity,
        /// Original measured unit and fragment index.
        source: FragmentSource,
    },
    /// Supplied positioned rows are not a one-to-one match for the plan.
    RowPositionCountMismatch {
        /// Number of sealed measured rows in the plan.
        expected: usize,
        /// Number of submitted horizontal row positions.
        supplied: usize,
    },
    /// One horizontal position does not belong to this measured plan row.
    RowPositionMismatch {
        /// Physical output row index in source placement order.
        index: usize,
        /// Expected accepted page for this row.
        page: AcceptedIdentity,
        /// Stable semantic owner of the measured row.
        fragment_owner: AcceptedIdentity,
        /// Original measured unit and fragment index.
        source: FragmentSource,
    },
    /// A fixed rectangle shares a vertical band with a measured flow row.
    VerticalBandOverlap {
        /// Physical page where overlap was observed.
        page: AcceptedIdentity,
        /// Accepted fixed semantic owner.
        object: AcceptedIdentity,
        /// Accepted flow block owning the placed row.
        fragment_owner: AcceptedIdentity,
        /// Exact source row whose vertical band overlaps the fixed region.
        source: FragmentSource,
        /// Top of the positive-height shared vertical interval on this page.
        overlap_top: Length,
        /// Height of the shared interval, without horizontal ink assumptions.
        overlap_height: Length,
    },
    /// Same-page results disagree about the accepted writable rectangle.
    WritableMismatch {
        /// Shared accepted physical page.
        page: AcceptedIdentity,
        /// Fixed object checked against conflicting writable bounds.
        object: AcceptedIdentity,
    },
}

/// Constructor-controlled evidence of a first fixed-pair overlap.
///
/// This is **inspection**, not a collision-policy decision: fixed shapes may
/// deliberately layer. Indices distinguish proposals sharing an object owner;
/// revision and flow bind the result to the exact checked pagination plan.
/// The returned rectangle is the positive-area physical intersection, not
/// a collision policy or a modified/approved placement.
/// Callers cannot forge an apparently validated pair or replace its authority.
///
/// ```compile_fail
/// use atrament_semantic_flow_pagination::FixedPairOverlap;
/// fn replace_revision(mut pair: FixedPairOverlap) {
///     pair.revision = pair.revision();
/// }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedPairOverlap {
    earlier_index: usize,
    earlier_object: AcceptedIdentity,
    flow: AcceptedIdentity,
    intersection: Rect,
    later_index: usize,
    later_object: AcceptedIdentity,
    page: AcceptedIdentity,
    revision: RevisionIdentity,
}

impl FixedPairOverlap {
    /// Original index of the first submitted rectangle in this pair.
    #[must_use]
    pub const fn earlier_index(&self) -> usize {
        self.earlier_index
    }

    /// Accepted semantic owner of the first submitted rectangle.
    #[must_use]
    pub const fn earlier_object(&self) -> AcceptedIdentity {
        self.earlier_object
    }

    /// Accepted selected flow whose measured plan checked these rectangles.
    #[must_use]
    pub const fn flow(&self) -> AcceptedIdentity {
        self.flow
    }

    /// Exact positive-area intersection in the accepted page's coordinates.
    #[must_use]
    pub const fn intersection(&self) -> Rect {
        self.intersection
    }

    /// Original index of the second submitted rectangle in this pair.
    #[must_use]
    pub const fn later_index(&self) -> usize {
        self.later_index
    }

    /// Accepted semantic owner of the second submitted rectangle.
    #[must_use]
    pub const fn later_object(&self) -> AcceptedIdentity {
        self.later_object
    }

    /// Accepted physical page where both rectangles intersect.
    #[must_use]
    pub const fn page(&self) -> AcceptedIdentity {
        self.page
    }

    /// Exact accepted revision that admitted the checked physical plan.
    #[must_use]
    pub const fn revision(&self) -> RevisionIdentity {
        self.revision
    }
}

/// Constructor-controlled physical overlap of one fixed region and flow row.
///
/// This reserves each measured row's full writable horizontal band; it is a
/// possible collision, **not** evidence of actual horizontal ink contact.
/// All provenance comes from one revision and the original supplied region.
///
/// ```compile_fail
/// use atrament_semantic_flow_pagination::FixedRowBandOverlap;
/// fn overwrite_revision(mut overlap: FixedRowBandOverlap) {
///     overlap.revision = overlap.revision();
/// }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedRowBandOverlap {
    fixed_index: usize,
    flow: AcceptedIdentity,
    fragment_owner: AcceptedIdentity,
    object: AcceptedIdentity,
    overlap_height: Length,
    overlap_top: Length,
    page: AcceptedIdentity,
    revision: RevisionIdentity,
    source: FragmentSource,
}

impl FixedRowBandOverlap {
    /// Original index in the supplied checked fixed-region sequence.
    #[must_use]
    pub const fn fixed_index(&self) -> usize {
        self.fixed_index
    }

    /// Selected accepted flow from the sealed measured plan.
    #[must_use]
    pub const fn flow(&self) -> AcceptedIdentity {
        self.flow
    }

    /// Stable semantic owner of the overlapping measured row.
    #[must_use]
    pub const fn fragment_owner(&self) -> AcceptedIdentity {
        self.fragment_owner
    }

    /// Stable semantic owner of the checked fixed region.
    #[must_use]
    pub const fn object(&self) -> AcceptedIdentity {
        self.object
    }

    /// Positive shared vertical interval height in physical micrometres.
    #[must_use]
    pub const fn overlap_height(&self) -> Length {
        self.overlap_height
    }

    /// Top of the shared vertical interval in physical micrometres.
    #[must_use]
    pub const fn overlap_top(&self) -> Length {
        self.overlap_top
    }

    /// Accepted physical page containing the overlapping vertical bands.
    #[must_use]
    pub const fn page(&self) -> AcceptedIdentity {
        self.page
    }

    /// Accepted semantic revision that bound the complete query.
    #[must_use]
    pub const fn revision(&self) -> RevisionIdentity {
        self.revision
    }

    /// Exact source unit and row fragment from measured-flow pagination.
    #[must_use]
    pub const fn source(&self) -> FragmentSource {
        self.source
    }
}

/// Caller-selected physical X position for one exact measured flow row.
///
/// The plan owns the row's accepted revision, flow, page, source and size.
/// This proposal supplies only horizontal placement, **not** measured ink
/// geometry or proof that visible glyphs occupy the resulting rectangle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RowHorizontalPosition {
    /// Accepted selected flow whose measured source row is positioned.
    pub flow: AcceptedIdentity,
    /// Absolute physical X origin for this row in micrometres.
    pub left: Length,
    /// Accepted page receiving this measured row.
    pub page: AcceptedIdentity,
    /// Exact revision that produced the measured pagination plan.
    pub revision: RevisionIdentity,
    /// Original unit and fragment that own this horizontal position.
    pub source: FragmentSource,
}

/// Checked intersection of a fixed rectangle and positioned row footprint.
///
/// This is a **provisional** measured-box intersection: actual glyph stroke
/// extents, horizontal placement intent and fixed layering remain unresolved.
/// Exact revision, flow, page and source cannot be replaced downstream.
///
/// ```compile_fail
/// use atrament_semantic_flow_pagination::FixedRowFootprintOverlap;
/// fn forge_revision(mut overlap: FixedRowFootprintOverlap) {
///     overlap.revision = overlap.revision();
/// }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedRowFootprintOverlap {
    fixed_index: usize,
    flow: AcceptedIdentity,
    fragment_owner: AcceptedIdentity,
    intersection: Rect,
    object: AcceptedIdentity,
    page: AcceptedIdentity,
    revision: RevisionIdentity,
    source: FragmentSource,
}

impl FixedRowFootprintOverlap {
    /// Original fixed-region proposal index.
    #[must_use]
    pub const fn fixed_index(&self) -> usize {
        self.fixed_index
    }

    /// Selected accepted semantic flow.
    #[must_use]
    pub const fn flow(&self) -> AcceptedIdentity {
        self.flow
    }

    /// Stable semantic owner of the measured row.
    #[must_use]
    pub const fn fragment_owner(&self) -> AcceptedIdentity {
        self.fragment_owner
    }

    /// Exact two-dimensional intersection in physical page coordinates.
    #[must_use]
    pub const fn intersection(&self) -> Rect {
        self.intersection
    }

    /// Stable semantic owner of the fixed region.
    #[must_use]
    pub const fn object(&self) -> AcceptedIdentity {
        self.object
    }

    /// Accepted physical page containing the intersecting rectangles.
    #[must_use]
    pub const fn page(&self) -> AcceptedIdentity {
        self.page
    }

    /// Accepted revision that admitted the complete physical query.
    #[must_use]
    pub const fn revision(&self) -> RevisionIdentity {
        self.revision
    }

    /// Original measured unit and fragment index.
    #[must_use]
    pub const fn source(&self) -> FragmentSource {
        self.source
    }
}

/// Already-measured semantic flow bound to one exact accepted revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionFlowMeasurement {
    /// Stable semantic flow whose top-level blocks produced these
    /// measurements.
    pub flow: AcceptedIdentity,
    /// Accepted revision whose semantic content produced these measurements.
    pub revision: RevisionIdentity,
    /// Measured semantic flow units in source reading order.
    pub units: Vec<MeasuredFlowUnit<AcceptedIdentity>>,
}

/// Typed failure to paginate measurements against one accepted revision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticPaginationError {
    /// A selected top-level block identity also names another semantic owner.
    DuplicateBlockIdentity {
        /// Stable selected flow containing the alias.
        flow: AcceptedIdentity,
        /// Repeated top-level block identity.
        block: AcceptedIdentity,
    },
    /// The selected flow identity is reused by another semantic owner, so
    /// selection is ambiguous.
    DuplicateFlowIdentity {
        /// Colliding semantic flow identity.
        flow: AcceptedIdentity,
    },
    /// A page-profile identity is reused by another semantic owner.
    DuplicatePageProfileIdentity {
        /// Repeated physical profile identity.
        profile: AcceptedIdentity,
    },
    /// Measurement names no top-level semantic flow in the accepted revision.
    FlowNotFound {
        /// Stable semantic flow identity that could not be resolved.
        flow: AcceptedIdentity,
    },
    /// An accepted page profile cannot produce valid physical page geometry.
    InvalidPageProfile {
        /// Stable semantic page using the invalid profile.
        page: AcceptedIdentity,
        /// Typed physical profile validation failure.
        reason: PhysicalPageProfileError,
    },
    /// A measured fragment owner is not a top-level block in the selected flow.
    MeasuredBlockNotInFlow {
        /// Stable selected semantic flow.
        flow: AcceptedIdentity,
        /// Supplied semantic owner outside that flow.
        owner: AcceptedIdentity,
    },
    /// Measured block-owner runs do not preserve selected flow block order.
    MeasurementBlockSequenceMismatch {
        /// Stable selected semantic flow whose order was violated.
        flow: AcceptedIdentity,
    },
    /// One selected-flow block has no measured fragment in the complete input.
    MeasurementIncomplete {
        /// Stable selected semantic flow.
        flow: AcceptedIdentity,
        /// First semantic block omitted from the measured owner sequence.
        missing: AcceptedIdentity,
    },
    /// Measurement belongs to another accepted revision and cannot be reused.
    MeasurementRevisionMismatch {
        /// Accepted revision requested for pagination.
        accepted: RevisionIdentity,
        /// Revision whose semantic content produced the supplied measurement.
        measured: RevisionIdentity,
    },
    /// An accepted page references a page profile absent from its notebook.
    MissingPageProfile {
        /// Stable semantic page whose physical profile is missing.
        page: AcceptedIdentity,
        /// Referenced profile identity that could not be resolved.
        profile: AcceptedIdentity,
    },
    /// Pure measured-flow pagination rejected the derived page sequence.
    Pagination(PaginationError<AcceptedIdentity, AcceptedIdentity>),
    /// A page receiving measured ink already owns another unmeasured flow.
    UnmeasuredFlowOnPage {
        /// Physical page whose full writable region would be shared.
        page: AcceptedIdentity,
        /// Another nonempty accepted flow without derived placement here.
        flow: AcceptedIdentity,
    },
}

// Already checked, nonempty physical box indexed for pair inspection.
#[derive(Clone, Copy)]
struct IndexedFixedRectangle {
    bottom_um: u64,
    index: usize,
    left_um: u64,
    object: AcceptedIdentity,
    right_um: u64,
    top_um: u64,
}

#[derive(Clone, Copy, Debug)]
struct FlowScope<'revision> {
    blocks: &'revision [Block<AcceptedIdentity>],
    page_index: usize,
}

type PageRegionsResult =
    Result<Vec<PageRegion<AcceptedIdentity>>, SemanticPaginationError>;

/// Inspect checked fixed placements for possible overlap with measured flow.
///
/// Flow placement reserves each row's full writable horizontal band because
/// the paginator does not choose actual text X coordinates. A fixed region
/// touching but not crossing a row boundary is allowed. Zero-width or
/// zero-height fixed rectangles occupy no ink space. Only placements on
/// covered pages with measured flow rows are compared; other accepted pages
/// do not share ink with this plan.
///
/// This verifies *only the supplied fixed-region evidence*, not whether all
/// semantic objects have been measured or supplied. It does not solve
/// horizontal collisions, relocate objects, or authorize native Export.
///
/// # Errors
///
/// Returns the first stale, overflowing, inconsistent, or potentially
/// overlapping fixed region in caller order.
pub fn check_fixed_regions_for_flow(
    plan: &SemanticPaginationPlan,
    fixed_regions: &[FixedRegionLayoutResult],
) -> Result<(), FlowFixedRegionError> {
    if fixed_regions.is_empty() {
        return Ok(());
    }
    let writable_by_page = writable_index(plan);
    let rows_by_page = measured_row_index(plan);
    for fixed in fixed_regions {
        validate_fixed_region_for_flow(plan, fixed, &writable_by_page)?;
        let mut first = None;
        if let Some(rows) = rows_by_page.get(&fixed.page()) {
            visit_fixed_measured_rows(fixed, rows, |row, top, height| {
                first = Some(FlowFixedRegionError::VerticalBandOverlap {
                    page: fixed.page(),
                    object: fixed.object(),
                    fragment_owner: row.owner,
                    source: row.source,
                    overlap_top: top,
                    overlap_height: height,
                });
                false
            })?;
        }
        if let Some(error) = first {
            return Err(error);
        }
    }
    Ok(())
}

/// Inspect every physical fixed-versus-measured-row vertical interval.
///
/// Unlike the blocking first-failure checker, this validates **all** supplied
/// checked regions against accepted revision, physical bounds and writable
/// envelopes *before* collecting possible row conflicts. Its observations
/// are ordered by fixed input index and physical row order. The caller's
/// maximum limits records: over-limit queries refuse without partial output.
/// A zero limit succeeds only when the supplied regions clear measured rows.
///
/// Results are conservative full-width band overlaps, not horizontal ink
/// intersections or an exhaustive page-wide collision policy. They do not
/// authorize native Export, nor prove omitted regions are clear.
///
/// # Errors
///
/// Returns a typed revision, physical-bounds, writable-envelope, coordinate
/// or result-limit refusal. A later invalid fixed region prevents returning
/// earlier overlap observations.
pub fn fixed_row_band_overlaps_for_flow(
    plan: &SemanticPaginationPlan,
    fixed_regions: &[FixedRegionLayoutResult],
    max_overlaps: usize,
) -> Result<Vec<FixedRowBandOverlap>, FlowFixedRegionError> {
    let writable_by_page = writable_index(plan);
    for fixed in fixed_regions {
        validate_fixed_region_for_flow(plan, fixed, &writable_by_page)?;
    }
    let rows_by_page = measured_row_index(plan);
    let mut overlaps = Vec::new();
    let mut exceeded = false;
    for (fixed_index, fixed) in fixed_regions.iter().enumerate() {
        if let Some(rows) = rows_by_page.get(&fixed.page()) {
            visit_fixed_measured_rows(fixed, rows, |row, top, height| {
                if overlaps.len() == max_overlaps {
                    exceeded = true;
                    return false;
                }
                overlaps.push(FixedRowBandOverlap {
                    fixed_index,
                    flow: plan.flow(),
                    fragment_owner: row.owner,
                    object: fixed.object(),
                    overlap_height: height,
                    overlap_top: top,
                    page: fixed.page(),
                    revision: plan.revision(),
                    source: row.source,
                });
                true
            })?;
        }
        if exceeded {
            return Err(FlowFixedRegionError::FixedRowLimitExceeded {
                limit: max_overlaps,
            });
        }
    }
    Ok(overlaps)
}

/// Inspect caller-positioned measured row boxes against checked fixed regions.
///
/// Unlike conservative full-width row bands, each horizontal row position
/// explicitly identifies the sealed plan's revision, flow, page and source
/// in physical placement order. Only the measured row `width` and `height`
/// form its provisional footprint. Zero-width rows and boundary touches have
/// no positive-area intersection. Every fixed rectangle and every row proposal
/// is checked **before** returning overlaps or applying the result cap.
///
/// This optional read-only inspection does not replace the conservative
/// fixed-row blocking gate. Caller-positioned row boxes are not visible ink,
/// automatic horizontal alignment, or sufficient native Export evidence.
///
/// # Errors
///
/// Returns typed fixed-geometry, row identity/count/footprint or result-limit
/// refusal without exposing a partial list of intersections.
pub fn fixed_row_footprint_overlaps_for_flow(
    plan: &SemanticPaginationPlan,
    fixed_regions: &[FixedRegionLayoutResult],
    positions: &[RowHorizontalPosition],
    max_overlaps: usize,
) -> Result<Vec<FixedRowFootprintOverlap>, FlowFixedRegionError> {
    let writable_by_page = writable_index(plan);
    for fixed in fixed_regions {
        validate_fixed_region_for_flow(plan, fixed, &writable_by_page)?;
    }
    let left_by_source = validate_row_horizontal_positions(plan, positions)?;
    let rows_by_page = measured_row_index(plan);
    let mut overlaps = Vec::new();
    let mut exceeded = false;
    for (fixed_index, fixed) in fixed_regions.iter().enumerate() {
        let rectangle = fixed.rectangle();
        let fixed_left = rectangle.x.micrometres();
        let fixed_right = fixed_left
            .checked_add(rectangle.width.micrometres())
            .ok_or_else(|| FlowFixedRegionError::CoordinateOverflow {
                page: fixed.page(),
                object: fixed.object(),
            })?;
        if let Some(rows) = rows_by_page.get(&fixed.page()) {
            let mut invalid_row = None;
            visit_fixed_measured_rows(fixed, rows, |row, top, height| {
                if row.width == Length::ZERO {
                    return true;
                }
                let Some(&row_left) = left_by_source
                    .get(&(row.source.unit, row.source.fragment))
                else {
                    // Defensive: the plan rows were all indexed above.
                    invalid_row = Some(
                        FlowFixedRegionError::RowFootprintOverflow {
                            page: row.page,
                            fragment_owner: row.owner,
                            source: row.source,
                        },
                    );
                    return false;
                };
                let Some(row_right) = row_left
                    .checked_add(row.width.micrometres())
                else {
                    invalid_row = Some(
                        FlowFixedRegionError::RowFootprintOverflow {
                            page: row.page,
                            fragment_owner: row.owner,
                            source: row.source,
                        },
                    );
                    return false;
                };
                let shared_left = fixed_left.max(row_left);
                let shared_right = fixed_right.min(row_right);
                if shared_left >= shared_right {
                    return true;
                }
                if overlaps.len() == max_overlaps {
                    exceeded = true;
                    return false;
                }
                let Some(shared_width) = shared_right.checked_sub(shared_left)
                else {
                    invalid_row = Some(
                        FlowFixedRegionError::CoordinateOverflow {
                            page: fixed.page(),
                            object: fixed.object(),
                        },
                    );
                    return false;
                };
                overlaps.push(FixedRowFootprintOverlap {
                    fixed_index,
                    flow: plan.flow(),
                    fragment_owner: row.owner,
                    intersection: Rect {
                        x: Length::from_micrometres(shared_left),
                        y: top,
                        width: Length::from_micrometres(shared_width),
                        height,
                    },
                    object: fixed.object(),
                    page: fixed.page(),
                    revision: plan.revision(),
                    source: row.source,
                });
                true
            })?;
            if let Some(error) = invalid_row {
                return Err(error);
            }
        }
        if exceeded {
            return Err(FlowFixedRegionError::FixedRowFootprintLimitExceeded {
                limit: max_overlaps,
            });
        }
    }
    Ok(overlaps)
}

// Validate the entire proposed X-placement sequence before any intersections
// are reported. The source-key index is unique within a sealed pagination plan.
fn validate_row_horizontal_positions(
    plan: &SemanticPaginationPlan,
    positions: &[RowHorizontalPosition],
) -> Result<BTreeMap<(usize, usize), u64>, FlowFixedRegionError> {
    if positions.len() != plan.placements().len() {
        return Err(FlowFixedRegionError::RowPositionCountMismatch {
            expected: plan.placements().len(),
            supplied: positions.len(),
        });
    }
    let mut left_by_source = BTreeMap::new();
    for (index, (row, position)) in plan.placements()
        .iter().zip(positions).enumerate()
    {
        if position.revision != plan.revision()
            || position.flow != plan.flow()
            || position.page != row.page
            || position.source != row.source
        {
            return Err(FlowFixedRegionError::RowPositionMismatch {
                index,
                page: row.page,
                fragment_owner: row.owner,
                source: row.source,
            });
        }
        let left = position.left.micrometres();
        let writable_left = row.writable.x.micrometres();
        let out_of_bounds = match (
            writable_left.checked_add(row.writable.width.micrometres()),
            left.checked_add(row.width.micrometres()),
        ) {
            (Some(writable_right), Some(row_right)) => {
                left < writable_left || row_right > writable_right
            },
            _ => true,
        };
        if out_of_bounds {
            return Err(FlowFixedRegionError::RowFootprintOverflow {
                page: row.page,
                fragment_owner: row.owner,
                source: row.source,
            });
        }
        let _previous = left_by_source.insert(
            (row.source.unit, row.source.fragment), left,
        );
    }
    Ok(left_by_source)
}

fn writable_index(
    plan: &SemanticPaginationPlan,
) -> BTreeMap<AcceptedIdentity, Rect> {
    plan.covered_pages()
        .iter()
        .map(|page| (page.page, page.writable))
        .collect()
}

fn measured_row_index(
    plan: &SemanticPaginationPlan,
) -> BTreeMap<
    AcceptedIdentity,
    Vec<&PlacedFragment<AcceptedIdentity, AcceptedIdentity>>,
> {
    let mut rows_by_page = BTreeMap::new();
    for row in plan.placements() {
        rows_by_page.entry(row.page).or_insert_with(Vec::new).push(row);
    }
    rows_by_page
}

fn validate_fixed_region_for_flow(
    plan: &SemanticPaginationPlan,
    fixed: &FixedRegionLayoutResult,
    writable_by_page: &BTreeMap<AcceptedIdentity, Rect>,
) -> Result<(), FlowFixedRegionError> {
    if fixed.revision() != plan.revision() {
        return Err(FlowFixedRegionError::RevisionMismatch {
            expected: plan.revision(),
            fixed: fixed.revision(),
            object: fixed.object(),
        });
    }
    if !fixed.is_within_bounds() {
        return Err(FlowFixedRegionError::FixedRegionOverflow {
            page: fixed.page(),
            object: fixed.object(),
        });
    }
    if writable_by_page
        .get(&fixed.page())
        .is_some_and(|writable| *writable != fixed.writable())
    {
        return Err(FlowFixedRegionError::WritableMismatch {
            page: fixed.page(),
            object: fixed.object(),
        });
    }
    Ok(())
}

// Forward-only positive-height rows have strictly increasing bottoms per
// page. This retains the indexed search while allowing a first-hit blocking
// check and an independent complete diagnostic traversal to share geometry.
fn visit_fixed_measured_rows(
    fixed: &FixedRegionLayoutResult,
    rows: &[&PlacedFragment<AcceptedIdentity, AcceptedIdentity>],
    mut visit: impl FnMut(
        &PlacedFragment<AcceptedIdentity, AcceptedIdentity>, Length, Length,
    ) -> bool,
) -> Result<(), FlowFixedRegionError> {
    let rectangle = fixed.rectangle();
    if rectangle.width.micrometres() == 0
        || rectangle.height.micrometres() == 0
    {
        return Ok(());
    }
    let fixed_start = rectangle.y.micrometres();
    let fixed_end = fixed_start
        .checked_add(rectangle.height.micrometres())
        .ok_or_else(|| FlowFixedRegionError::CoordinateOverflow {
            page: fixed.page(),
            object: fixed.object(),
        })?;
    let first_candidate = rows.partition_point(|row| {
        row.top.micrometres()
            .saturating_add(row.height.micrometres()) <= fixed_start
    });
    for row in rows.get(first_candidate..).unwrap_or(&[]) {
        let row_start = row.top.micrometres();
        if row_start >= fixed_end {
            break;
        }
        let row_end = row_start
            .checked_add(row.height.micrometres())
            .ok_or_else(|| FlowFixedRegionError::CoordinateOverflow {
                page: fixed.page(),
                object: fixed.object(),
            })?;
        if fixed_start < row_end {
            let shared_top = fixed_start.max(row_start);
            let shared_height = fixed_end
                .min(row_end)
                .checked_sub(shared_top)
                .ok_or_else(|| FlowFixedRegionError::CoordinateOverflow {
                    page: fixed.page(),
                    object: fixed.object(),
                })?;
            if !visit(
                row,
                Length::from_micrometres(shared_top),
                Length::from_micrometres(shared_height),
            ) {
                break;
            }
        }
    }
    Ok(())
}

/// Find the first physical overlap among supplied checked fixed rectangles.
///
/// The existing measured-row clearance and revision/geometry checks run first;
/// this query then inspects **only** the supplied nonempty fixed rectangles.
/// A touching boundary or a zero-area rectangle is not an overlap, and boxes
/// on different pages cannot intersect. Candidate pairs are considered in
/// caller order, with the later index primary and earlier index secondary.
///
/// A returned pair does not imply a blocking collision: explicit layering and
/// overlap permission belong to a future typed placement solver. `None` does
/// not prove that omitted objects, actual ink, or other notebook flows clear.
///
/// # Errors
///
/// Returns the existing typed fixed-vs-flow refusal if submitted evidence is
/// stale, physically overflowing, inconsistent, or crosses measured rows.
pub fn first_fixed_pair_overlap_for_flow(
    plan: &SemanticPaginationPlan,
    fixed_regions: &[FixedRegionLayoutResult],
) -> Result<Option<FixedPairOverlap>, FlowFixedRegionError> {
    let mut first = None;
    visit_fixed_pair_overlaps_for_flow(plan, fixed_regions, |pair| {
        first = Some(pair);
        false
    })?;
    Ok(first)
}

/// Inspect every supplied positive-area fixed-pair overlap within a cap.
///
/// The returned records retain original input indices, semantic owners,
/// accepted revision/flow, physical page and exact intersection rectangle.
/// Pairs are ordered by later input index, then earlier input index. This is
/// read-only geometry evidence: neither a missing proposal nor intentional
/// layering can be resolved by this query. `max_pairs` bounds the number of
/// records materialized, and overflow **refuses** rather than silently
/// truncating diagnostics. A zero cap succeeds only for a clear collection.
///
/// # Errors
///
/// Revision, geometry and conservative measured-row refusals precede pair
/// inspection. More than `max_pairs` intersections returns a typed limit
/// failure with no partial diagnostic result.
pub fn fixed_pair_overlaps_for_flow(
    plan: &SemanticPaginationPlan,
    fixed_regions: &[FixedRegionLayoutResult],
    max_pairs: usize,
) -> Result<Vec<FixedPairOverlap>, FlowFixedRegionError> {
    let mut pairs = Vec::new();
    let mut exceeded = false;
    visit_fixed_pair_overlaps_for_flow(plan, fixed_regions, |pair| {
        if pairs.len() == max_pairs {
            exceeded = true;
            return false;
        }
        pairs.push(pair);
        true
    })?;
    if exceeded {
        return Err(FlowFixedRegionError::FixedPairLimitExceeded {
            limit: max_pairs,
        });
    }
    Ok(pairs)
}

// The visitor keeps first-only inspection short-circuiting while the bounded
// collector traverses all pairs. All checked regions are admitted *before*
// either inspection, including later invalid candidates after a valid pair.
fn visit_fixed_pair_overlaps_for_flow(
    plan: &SemanticPaginationPlan,
    fixed_regions: &[FixedRegionLayoutResult],
    mut visit: impl FnMut(FixedPairOverlap) -> bool,
) -> Result<(), FlowFixedRegionError> {
    check_fixed_regions_for_flow(plan, fixed_regions)?;
    // The checked bounds guarantee non-overflowing physical rectangles. Keep
    // explicit arithmetic refusal rather than ever comparing saturated ends.
    let mut prior_by_page =
        BTreeMap::<AcceptedIdentity, Vec<IndexedFixedRectangle>>::new();
    for (later_index, fixed) in fixed_regions.iter().enumerate() {
        let rect = fixed.rectangle();
        let width = rect.width.micrometres();
        let height = rect.height.micrometres();
        if width == 0 || height == 0 {
            continue;
        }
        let left = rect.x.micrometres();
        let top = rect.y.micrometres();
        let right = left.checked_add(width).ok_or_else(|| {
            FlowFixedRegionError::CoordinateOverflow {
                page: fixed.page(),
                object: fixed.object(),
            }
        })?;
        let bottom = top.checked_add(height).ok_or_else(|| {
            FlowFixedRegionError::CoordinateOverflow {
                page: fixed.page(),
                object: fixed.object(),
            }
        })?;
        let prior = prior_by_page.entry(fixed.page()).or_default();
        for earlier in prior.iter() {
            if earlier.left_um < right
                && left < earlier.right_um
                && earlier.top_um < bottom
                && top < earlier.bottom_um
            {
                let shared_left = left.max(earlier.left_um);
                let shared_top = top.max(earlier.top_um);
                let overflow = || FlowFixedRegionError::CoordinateOverflow {
                    page: fixed.page(),
                    object: fixed.object(),
                };
                let shared_width = right
                    .min(earlier.right_um)
                    .checked_sub(shared_left)
                    .ok_or_else(overflow)?;
                let shared_height = bottom
                    .min(earlier.bottom_um)
                    .checked_sub(shared_top)
                    .ok_or_else(overflow)?;
                if !visit(FixedPairOverlap {
                    earlier_index: earlier.index,
                    earlier_object: earlier.object,
                    flow: plan.flow(),
                    intersection: Rect {
                        x: Length::from_micrometres(shared_left),
                        y: Length::from_micrometres(shared_top),
                        width: Length::from_micrometres(shared_width),
                        height: Length::from_micrometres(shared_height),
                    },
                    later_index,
                    later_object: fixed.object(),
                    page: fixed.page(),
                    revision: plan.revision(),
                }) {
                    return Ok(());
                }
            }
        }
        prior.push(IndexedFixedRectangle {
            bottom_um: bottom,
            index: later_index,
            left_um: left,
            object: fixed.object(),
            right_um: right,
            top_um: top,
        });
    }
    Ok(())
}

/// Paginate complete measured semantic flow through accepted page profiles.
///
/// The function validates the exact measurement revision and flow first,
/// verifies every top-level flow block appears in semantic order, derives page
/// rectangles only from accepted page-profile authority, then delegates those
/// rectangles to the pure flow paginator. A returned page must not own another
/// nonempty flow: this single-flow projection has no measurement or reservation
/// for a peer's physical ink.
///
/// # Errors
///
/// Returns a typed failure for stale measurement, unknown flow or block owner,
/// incomplete or reordered measurement, invalid page-profile authority,
/// unmeasured other flows on occupied pages, or pagination failure.
pub fn paginate_revision(
    revision: &AcceptedRevision,
    measurement: &RevisionFlowMeasurement,
) -> Result<SemanticPaginationPlan, SemanticPaginationError> {
    if measurement.revision != revision.id {
        return Err(SemanticPaginationError::MeasurementRevisionMismatch {
            accepted: revision.id,
            measured: measurement.revision,
        });
    }
    let scope = flow_scope(revision, measurement.flow)?;
    validate_measurement_blocks(revision, scope.blocks, measurement)?;
    let plan = if scope.blocks.is_empty() {
        paginate(&[], &measurement.units)
            .map_err(SemanticPaginationError::Pagination)?
    } else {
        let pages = page_regions(revision, scope.page_index)?;
        let plan = paginate(&pages, &measurement.units)
            .map_err(SemanticPaginationError::Pagination)?;
        reject_unmeasured_page_flows(revision, measurement.flow, &plan)?;
        plan
    };
    Ok(SemanticPaginationPlan {
        covered_pages: plan.covered_pages,
        flow: measurement.flow,
        placements: plan.placements,
        revision: revision.id,
    })
}

// This projection owns only one measured flow and the full writable
// rectangle of each physical page it uses. An unrelated nonempty flow on
// that page has no reserved region here, so emitting both plans would risk
// placing two semantic owners into the same physical ink area.
fn reject_unmeasured_page_flows(
    revision: &AcceptedRevision,
    selected_flow: AcceptedIdentity,
    plan: &PaginationPlan<AcceptedIdentity, AcceptedIdentity>,
) -> Result<(), SemanticPaginationError> {
    let placed_pages = plan.placements.iter()
        .map(|placement| placement.page)
        .collect::<BTreeSet<_>>();
    for page in &revision.notebook.pages {
        if !placed_pages.contains(&page.id) {
            continue;
        }
        if let Some(flow) = page.flows.iter().find(|flow| {
            flow.id != selected_flow && !flow.blocks.is_empty()
        }) {
            return Err(SemanticPaginationError::UnmeasuredFlowOnPage {
                page: page.id,
                flow: flow.id,
            });
        }
    }
    Ok(())
}

fn flow_scope(
    revision: &AcceptedRevision,
    target: AcceptedIdentity,
) -> Result<FlowScope<'_>, SemanticPaginationError> {
    let mut found = None;
    for (page_index, page) in revision.notebook.pages.iter().enumerate() {
        for flow in &page.flows {
            if flow.id != target {
                continue;
            }
            if found.is_some() {
                return Err(SemanticPaginationError::DuplicateFlowIdentity {
                    flow: target,
                });
            }
            found = Some(FlowScope {
                blocks: &flow.blocks,
                page_index,
            });
        }
    }
    let scope =
        found.ok_or(SemanticPaginationError::FlowNotFound { flow: target })?;
    // A unique flow among flows is not authoritative if another semantic
    // owner (including a page, profile, or block) shadows its identity.
    if semantic_duplicate_owner_among(
        &revision.notebook,
        &BTreeSet::from([target]),
    )
    .is_some()
    {
        return Err(SemanticPaginationError::DuplicateFlowIdentity {
            flow: target,
        });
    }
    Ok(scope)
}

fn page_regions(
    revision: &AcceptedRevision,
    start_page_index: usize,
) -> PageRegionsResult {
    // Flow placements start at their owning page, but accepted page
    // identities are global: earlier pages cannot alias later placements.
    let mut page_ids = BTreeSet::new();
    for page in &revision.notebook.pages {
        if !page_ids.insert(page.id) {
            return Err(SemanticPaginationError::Pagination(
                PaginationError::DuplicatePageIdentity { page: page.id },
            ));
        }
    }
    let mut profiles = BTreeMap::new();
    for profile in &revision.notebook.page_profiles {
        if profiles.insert(profile.id, profile).is_some() {
            return Err(SemanticPaginationError::DuplicatePageProfileIdentity {
                profile: profile.id,
            });
        }
    }
    // A page or paper-profile identity also cannot be owned by another
    // semantic kind. References to shared paper profiles are not owners.
    let mut selected = page_ids.clone();
    selected.extend(profiles.keys().copied());
    if let Some(identity) =
        semantic_duplicate_owner_among(&revision.notebook, &selected)
    {
        if page_ids.contains(&identity) {
            return Err(SemanticPaginationError::Pagination(
                PaginationError::DuplicatePageIdentity { page: identity },
            ));
        }
        return Err(SemanticPaginationError::DuplicatePageProfileIdentity {
            profile: identity,
        });
    }
    let remaining_pages =
        revision.notebook.pages.len().saturating_sub(start_page_index);
    let mut regions = Vec::with_capacity(remaining_pages);
    let mut writable_by_profile = BTreeMap::new();
    for page in revision.notebook.pages.iter().skip(start_page_index) {
        let Some(profile) = profiles.get(&page.paper_profile).copied() else {
            return Err(SemanticPaginationError::MissingPageProfile {
                page: page.id,
                profile: page.paper_profile,
            });
        };
        let writable = if let Some(writable) =
            writable_by_profile.get(&page.paper_profile).copied()
        {
            writable
        } else {
            let valid = profile.geometry.validated().map_err(|reason| {
                SemanticPaginationError::InvalidPageProfile {
                    page: page.id,
                    reason,
                }
            })?;
            let writable = valid.writable_region();
            let _previous =
                writable_by_profile.insert(page.paper_profile, writable);
            writable
        };
        regions.push(PageRegion { page: page.id, writable });
    }
    Ok(regions)
}

fn measurement_owner_error(
    blocks: &[Block<AcceptedIdentity>],
    flow: AcceptedIdentity,
    owner: AcceptedIdentity,
) -> SemanticPaginationError {
    if blocks.iter().any(|block| block.id == owner) {
        return SemanticPaginationError::MeasurementBlockSequenceMismatch {
            flow,
        };
    }
    SemanticPaginationError::MeasuredBlockNotInFlow { flow, owner }
}

fn validate_selected_owner_identities(
    revision: &AcceptedRevision,
    blocks: &[Block<AcceptedIdentity>],
    flow: AcceptedIdentity,
) -> Result<(), SemanticPaginationError> {
    // Preserve selected-flow duplicate precedence before scanning the full
    // notebook for cross-kind or cross-flow owner aliases.
    let mut selected = BTreeSet::new();
    for block in blocks {
        if !selected.insert(block.id) {
            return Err(SemanticPaginationError::DuplicateBlockIdentity {
                flow,
                block: block.id,
            });
        }
    }
    if let Some(block) =
        semantic_duplicate_owner_among(&revision.notebook, &selected)
    {
        return Err(SemanticPaginationError::DuplicateBlockIdentity {
            flow,
            block,
        });
    }
    Ok(())
}

fn validate_measurement_blocks(
    revision: &AcceptedRevision,
    blocks: &[Block<AcceptedIdentity>],
    measurement: &RevisionFlowMeasurement,
) -> Result<(), SemanticPaginationError> {
    validate_selected_owner_identities(
        revision, blocks, measurement.flow,
    )?;
    let mut previous_owner = None;
    let mut run_index = 0usize;
    for unit in &measurement.units {
        for fragment in &unit.fragments {
            let owner = fragment.owner;
            if previous_owner == Some(owner) {
                continue;
            }
            previous_owner = Some(owner);
            let Some(expected) = blocks.get(run_index) else {
                return Err(measurement_owner_error(
                    blocks,
                    measurement.flow,
                    owner,
                ));
            };
            if expected.id != owner {
                return Err(measurement_owner_error(
                    blocks,
                    measurement.flow,
                    owner,
                ));
            }
            run_index = run_index.saturating_add(1);
        }
    }
    if let Some(missing) = blocks.get(run_index) {
        return Err(SemanticPaginationError::MeasurementIncomplete {
            flow: measurement.flow,
            missing: missing.id,
        });
    }
    Ok(())
}
