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
//   - Accepted-revision/page/object binding for post-placement fixed geometry.
//   - Page-profile-derived writable bounds and complete overflow diagnostics.
// - Must-Not:
//   - Choose anchors, alignment, minimum size, collision policy, or repairs.
// - Allows:
//   - Inputs: One accepted revision and one revision-bound derived rectangle.
//   - Outputs: Typed in-bounds/overflow result or typed validation failure.
//   - Side effects: Process-local diagnostic/result allocation only.
// - Split-When:
//   - Collision or full constraint solving becomes independently implemented.
// - Merge-When:
//   - Accepted fixed-region validation moves into a larger layout application.
// - Summary:
//   - Validates derived fixed geometry against accepted physical page bounds.
// - Description:
//   - Binds overflow evidence to semantic revision, page, and block authority.
// - Usage:
//   - Supply solver-derived geometry after accepted semantic placement intent.
// - Defaults:
//   - Every crossed edge produces one complete blocking layout diagnostic.
//

//! Accepted semantic binding for post-placement fixed-region bounds checking.

use std::fmt::Debug;

use atrament_diagnostic::{
    BlockingDisposition, Completeness, Diagnostic, DiagnosticCode,
    DiagnosticSet, Evidence, LocationKind, LocationRole, Operation,
    OperationBinding, OperationContext, OperationContextKind,
    PhysicalBoundaryEdge, PhysicalLengthQuantity, Remediation,
    SemanticLocation, Severity,
};
use atrament_fixed_region_bounds::{
    BoundaryEdge, BoundsError, BoundsReport, check_bounds,
};
use atrament_physical_page_profile::Rect;
use atrament_semantic_notebook::{
    AcceptedIdentity, AcceptedRevision, Block, BlockContent,
    PhysicalPageProfileError, RevisionIdentity,
};

/// One solver-derived fixed rectangle bound to accepted semantic authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcceptedFixedPlacement {
    /// Stable accepted block whose derived rectangle is being checked.
    pub object: AcceptedIdentity,
    /// Stable accepted page that owns the fixed block.
    pub page: AcceptedIdentity,
    /// Derived physical rectangle in page coordinates.
    pub rectangle: Rect,
    /// Exact accepted revision whose semantic placement produced the
    /// rectangle.
    pub revision: RevisionIdentity,
}

/// Completed accepted fixed-region bounds result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FixedRegionLayoutResult {
    /// Derived fixed geometry crosses one or more writable page boundaries.
    Overflow {
        /// Complete blocking diagnostics, one for each violated boundary.
        diagnostics: DiagnosticSet,
        /// Stable accepted block validated by this result.
        object: AcceptedIdentity,
        /// Stable accepted page validated by this result.
        page: AcceptedIdentity,
        /// Exact physical bounds report behind the diagnostics.
        report: BoundsReport,
    },
    /// Derived fixed geometry remains fully inside the writable page region.
    WithinBounds {
        /// Stable accepted block validated by this result.
        object: AcceptedIdentity,
        /// Stable accepted page validated by this result.
        page: AcceptedIdentity,
    },
}

/// Typed failure before accepted fixed-region bounds can be evaluated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixedRegionLayoutError {
    /// Physical rectangle coordinate arithmetic cannot be represented.
    Bounds(BoundsError),
    /// Accepted page profile is structurally invalid.
    InvalidPageProfile {
        /// Stable accepted page using the invalid physical profile.
        page: AcceptedIdentity,
        /// Exact physical page-profile validation failure.
        reason: PhysicalPageProfileError,
    },
    /// Accepted page references no page profile in the revision.
    MissingPageProfile {
        /// Stable accepted page whose profile cannot be resolved.
        page: AcceptedIdentity,
        /// Referenced accepted profile identity.
        profile: AcceptedIdentity,
    },
    /// Supplied semantic block does not belong to the supplied accepted page.
    ObjectNotOnPage {
        /// Stable accepted block being checked.
        object: AcceptedIdentity,
        /// Stable accepted page claimed to own the block.
        page: AcceptedIdentity,
    },
    /// Supplied accepted page does not exist in the revision.
    PageNotFound {
        /// Stable accepted page that could not be resolved.
        page: AcceptedIdentity,
    },
    /// Derived placement belongs to another accepted revision.
    RevisionMismatch {
        /// Current accepted revision being evaluated.
        accepted: RevisionIdentity,
        /// Revision that produced the supplied derived placement.
        placement: RevisionIdentity,
    },
}

/// Validate one derived fixed rectangle against accepted writable page bounds.
///
/// The function rejects stale or mismatched semantic ownership first, derives
/// writable geometry only from the accepted page's referenced physical profile,
/// then maps every exact boundary violation into a blocking layout diagnostic.
/// It never moves, clips, resizes, crops, or reflows the supplied geometry.
///
/// # Errors
///
/// Returns a typed failure for stale revision binding, unknown page or object,
/// missing or invalid page-profile authority, or unrepresentable coordinates.
pub fn validate_fixed_placement(
    revision: &AcceptedRevision,
    placement: AcceptedFixedPlacement,
) -> Result<FixedRegionLayoutResult, FixedRegionLayoutError> {
    if placement.revision != revision.id {
        return Err(FixedRegionLayoutError::RevisionMismatch {
            accepted: revision.id,
            placement: placement.revision,
        });
    }
    let Some(page) = revision
        .notebook
        .pages
        .iter()
        .find(|page| page.id == placement.page)
    else {
        return Err(FixedRegionLayoutError::PageNotFound {
            page: placement.page,
        });
    };
    if !page
        .flows
        .iter()
        .any(|flow| blocks_contain(&flow.blocks, placement.object))
    {
        return Err(FixedRegionLayoutError::ObjectNotOnPage {
            object: placement.object,
            page: placement.page,
        });
    }
    let Some(profile) = revision
        .notebook
        .page_profiles
        .iter()
        .find(|profile| profile.id == page.page_profile)
    else {
        return Err(FixedRegionLayoutError::MissingPageProfile {
            page: page.id,
            profile: page.page_profile,
        });
    };
    let valid = profile.geometry.validate().map_err(|reason| {
        FixedRegionLayoutError::InvalidPageProfile { page: page.id, reason }
    })?;
    let writable = valid.writable_region().map_err(|reason| {
        FixedRegionLayoutError::InvalidPageProfile { page: page.id, reason }
    })?;
    let report = check_bounds(writable, placement.rectangle)
        .map_err(FixedRegionLayoutError::Bounds)?;
    if report.is_within_bounds() {
        return Ok(FixedRegionLayoutResult::WithinBounds {
            object: placement.object,
            page: placement.page,
        });
    }
    Ok(FixedRegionLayoutResult::Overflow {
        diagnostics: overflow_diagnostics(revision.id, placement, &report),
        object: placement.object,
        page: placement.page,
        report,
    })
}

fn block_content_contains(
    content: &BlockContent<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> bool {
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            blocks_contain(blocks, target)
        },
        BlockContent::Date(_)
        | BlockContent::Definition(_)
        | BlockContent::Figure(_)
        | BlockContent::Heading(_)
        | BlockContent::Mathematics(_)
        | BlockContent::Paragraph(_)
        | BlockContent::Rule
        | BlockContent::Unresolved(_) => false,
        BlockContent::List(list) => list
            .items
            .iter()
            .any(|item| blocks_contain(&item.blocks, target)),
        BlockContent::Table(table) => table.rows.iter().any(|row| {
            row.cells
                .iter()
                .any(|cell| blocks_contain(&cell.blocks, target))
        }),
    }
}

fn blocks_contain(
    blocks: &[Block<AcceptedIdentity>],
    target: AcceptedIdentity,
) -> bool {
    blocks.iter().any(|block| {
        block.id == target || block_content_contains(&block.content, target)
    })
}

fn diagnostic_identity(value: &impl Debug) -> String {
    format!("{value:?}")
}

const fn diagnostic_edge(edge: BoundaryEdge) -> PhysicalBoundaryEdge {
    match edge {
        BoundaryEdge::Bottom => PhysicalBoundaryEdge::Bottom,
        BoundaryEdge::Left => PhysicalBoundaryEdge::Left,
        BoundaryEdge::Right => PhysicalBoundaryEdge::Right,
        BoundaryEdge::Top => PhysicalBoundaryEdge::Top,
    }
}

fn overflow_diagnostics(
    revision: RevisionIdentity,
    placement: AcceptedFixedPlacement,
    report: &BoundsReport,
) -> DiagnosticSet {
    let revision_identity = diagnostic_identity(&revision);
    let object_identity = diagnostic_identity(&placement.object);
    let page_identity = diagnostic_identity(&placement.page);
    let diagnostics = report
        .violations
        .iter()
        .map(|violation| Diagnostic {
            code: DiagnosticCode::LayoutFixedRegionOverflow,
            disposition: BlockingDisposition::Blocking,
            evidence: vec![
                Evidence::PhysicalBoundary {
                    edge: diagnostic_edge(violation.edge),
                },
                Evidence::PhysicalLength {
                    micrometres: i128::from(violation.amount.micrometres()),
                    quantity: PhysicalLengthQuantity::Overflow,
                },
            ],
            locations: vec![
                SemanticLocation {
                    identity: object_identity.clone(),
                    kind: LocationKind::Object,
                    relationship: None,
                    role: LocationRole::Primary,
                },
                SemanticLocation {
                    identity: page_identity.clone(),
                    kind: LocationKind::Structure,
                    relationship: None,
                    role: LocationRole::Related,
                },
            ],
            operation: OperationBinding {
                contexts: vec![OperationContext {
                    identity: revision_identity.clone(),
                    kind: OperationContextKind::AcceptedRevision,
                }],
                operation: Operation::Layout,
            },
            remediations: vec![Remediation::ChangeConstraint],
            severity: Severity::Error,
        })
        .collect();
    DiagnosticSet {
        completeness: Completeness::Complete,
        diagnostics,
    }
}
