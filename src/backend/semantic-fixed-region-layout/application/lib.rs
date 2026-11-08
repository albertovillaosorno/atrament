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

use std::collections::BTreeSet;
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
    AcceptedIdentity, AcceptedRevision, PhysicalPageProfileError,
    RevisionIdentity, semantic_block_page, semantic_duplicate_owner_among,
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

/// Constructor-controlled bounds evidence for one accepted fixed placement.
///
/// The exact checked object, page, rectangle, revision, and accepted writable
/// region cannot be replaced by callers. Violations have complete blocking
/// diagnostics; an in-bounds report has none. Neither can be forged as a ready
/// placement without running `validate_fixed_placement`.
///
/// ```compile_fail
/// use atrament_semantic_fixed_region_layout::FixedRegionLayoutResult;
/// fn forge(mut checked: FixedRegionLayoutResult) {
///     checked.revision = checked.revision();
/// }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixedRegionLayoutResult {
    diagnostics: Option<DiagnosticSet>,
    object: AcceptedIdentity,
    page: AcceptedIdentity,
    rectangle: Rect,
    report: BoundsReport,
    revision: RevisionIdentity,
    writable: Rect,
}

impl FixedRegionLayoutResult {
    /// Complete blocking diagnostics only when the rectangle overflows.
    #[must_use]
    pub const fn diagnostics(&self) -> Option<&DiagnosticSet> {
        self.diagnostics.as_ref()
    }

    /// Whether the exact rectangle fits entirely inside the writable page.
    #[must_use]
    pub const fn is_within_bounds(&self) -> bool {
        self.report.is_within_bounds()
    }

    /// Return the stable accepted block checked by this result.
    #[must_use]
    pub const fn object(&self) -> AcceptedIdentity {
        self.object
    }

    /// Return the stable accepted page checked by this result.
    #[must_use]
    pub const fn page(&self) -> AcceptedIdentity {
        self.page
    }

    /// Return the unmodified solver-derived rectangle checked on this page.
    #[must_use]
    pub const fn rectangle(&self) -> Rect {
        self.rectangle
    }

    /// Return the complete physical bounds report for the exact rectangle.
    #[must_use]
    pub const fn report(&self) -> &BoundsReport {
        &self.report
    }

    /// Return the exact accepted revision that supplied the page geometry.
    #[must_use]
    pub const fn revision(&self) -> RevisionIdentity {
        self.revision
    }

    /// Return the exact accepted page-profile writable rectangle used by this
    /// check. This bound is distinct from the supplied object rectangle.
    #[must_use]
    pub const fn writable(&self) -> Rect {
        self.writable
    }
}

/// Typed failure before accepted fixed-region bounds can be evaluated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixedRegionLayoutError {
    /// Physical rectangle coordinate arithmetic cannot be represented.
    Bounds(BoundsError),
    /// Two semantic blocks share the placed object's identity.
    DuplicateObjectIdentity {
        /// Repeated semantic block identity.
        object: AcceptedIdentity,
    },
    /// A page identity is reused by another semantic owner.
    DuplicatePageIdentity {
        /// Colliding physical page identity.
        page: AcceptedIdentity,
    },
    /// A page-profile identity is reused by another semantic owner.
    DuplicatePageProfileIdentity {
        /// Colliding physical page-profile identity.
        profile: AcceptedIdentity,
    },
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
/// missing or ambiguous page/profile/block authority, invalid physical
/// geometry, or unrepresentable coordinates.
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
    // This is normally guaranteed by candidate acceptance. Direct typed
    // revisions must also reject ambiguous first-match page/profile lookups.
    let mut page_ids = BTreeSet::new();
    for page in &revision.notebook.pages {
        if !page_ids.insert(page.id) {
            return Err(FixedRegionLayoutError::DuplicatePageIdentity {
                page: page.id,
            });
        }
    }
    let mut profile_ids = BTreeSet::new();
    for profile in &revision.notebook.page_profiles {
        if !profile_ids.insert(profile.id) {
            return Err(FixedRegionLayoutError::DuplicatePageProfileIdentity {
                profile: profile.id,
            });
        }
    }
    // Reject duplicate owners before page lookup: the first occurrence may
    // belong to another page than the caller's intended fixed placement.
    let selected = BTreeSet::from([placement.object]);
    if semantic_duplicate_owner_among(&revision.notebook, &selected).is_some() {
        return Err(FixedRegionLayoutError::DuplicateObjectIdentity {
            object: placement.object,
        });
    }
    // First-match page and profile authority is unsafe even if the alias
    // belongs to a different semantic kind. Shared profile references do
    // not count as additional owners.
    let mut page_authorities = page_ids.clone();
    page_authorities.extend(profile_ids);
    if let Some(identity) =
        semantic_duplicate_owner_among(&revision.notebook, &page_authorities)
    {
        if page_ids.contains(&identity) {
            return Err(FixedRegionLayoutError::DuplicatePageIdentity {
                page: identity,
            });
        }
        return Err(FixedRegionLayoutError::DuplicatePageProfileIdentity {
            profile: identity,
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
    let object_is_block_on_page =
        semantic_block_page(&revision.notebook, placement.object)
            == Some(placement.page);
    if !object_is_block_on_page {
        return Err(FixedRegionLayoutError::ObjectNotOnPage {
            object: placement.object,
            page: placement.page,
        });
    }
    let Some(profile) = revision
        .notebook
        .page_profiles
        .iter()
        .find(|profile| profile.id == page.paper_profile)
    else {
        return Err(FixedRegionLayoutError::MissingPageProfile {
            page: page.id,
            profile: page.paper_profile,
        });
    };
    let valid = profile.geometry.validated().map_err(|reason| {
        FixedRegionLayoutError::InvalidPageProfile { page: page.id, reason }
    })?;
    let writable = valid.writable_region();
    let report = check_bounds(writable, placement.rectangle)
        .map_err(FixedRegionLayoutError::Bounds)?;
    let diagnostics = if report.is_within_bounds() {
        None
    } else {
        Some(overflow_diagnostics(revision.id, placement, &report))
    };
    Ok(FixedRegionLayoutResult {
        diagnostics,
        object: placement.object,
        page: placement.page,
        rectangle: placement.rectangle,
        report,
        revision: revision.id,
        writable,
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
