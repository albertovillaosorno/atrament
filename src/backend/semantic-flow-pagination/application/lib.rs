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
// - Must-Not:
//   - Measure text, mutate accepted revisions, invent page geometry, or render.
// - Allows:
//   - Inputs: One accepted revision and revision-bound measured flow units.
//   - Outputs: Typed pagination plan or typed non-mutation failure.
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
    MeasuredFlowUnit, PageRegion, PaginationError, PaginationPlan,
    PlacedFragment, paginate,
};
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

#[derive(Clone, Copy, Debug)]
struct FlowScope<'revision> {
    blocks: &'revision [Block<AcceptedIdentity>],
    page_index: usize,
}

type PageRegionsResult =
    Result<Vec<PageRegion<AcceptedIdentity>>, SemanticPaginationError>;

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
