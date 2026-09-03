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

use atrament_flow_pagination::{
    MeasuredFlowUnit, PageRegion, PaginationError, PaginationPlan, paginate,
};
use atrament_semantic_notebook::{
    AcceptedIdentity, AcceptedRevision, Block, PhysicalPageProfileError,
    RevisionIdentity,
};

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
}

#[derive(Clone, Copy, Debug)]
struct FlowScope<'revision> {
    blocks: &'revision [Block<AcceptedIdentity>],
    page_index: usize,
}

/// Paginate complete measured semantic flow through accepted page profiles.
///
/// The function validates the exact measurement revision and flow first,
/// verifies every top-level flow block appears in semantic order, derives page
/// rectangles only from accepted page-profile authority, then delegates those
/// rectangles to the pure flow paginator.
///
/// # Errors
///
/// Returns a typed failure for stale measurement, unknown flow or block owner,
/// incomplete or reordered measurement, invalid page-profile authority, or a
/// measured-flow pagination failure.
pub fn paginate_revision(
    revision: &AcceptedRevision,
    measurement: &RevisionFlowMeasurement,
) -> Result<
    PaginationPlan<AcceptedIdentity, AcceptedIdentity>,
    SemanticPaginationError,
> {
    if measurement.revision != revision.id {
        return Err(SemanticPaginationError::MeasurementRevisionMismatch {
            accepted: revision.id,
            measured: measurement.revision,
        });
    }
    let scope = flow_scope(revision, measurement.flow)?;
    validate_measurement_blocks(scope.blocks, measurement)?;
    let pages = page_regions(revision, scope.page_index)?;
    paginate(&pages, &measurement.units)
        .map_err(SemanticPaginationError::Pagination)
}

fn flow_scope(
    revision: &AcceptedRevision,
    target: AcceptedIdentity,
) -> Result<FlowScope<'_>, SemanticPaginationError> {
    for (page_index, page) in revision.notebook.pages.iter().enumerate() {
        if let Some(flow) = page.flows.iter().find(|flow| flow.id == target) {
            return Ok(FlowScope {
                blocks: &flow.blocks,
                page_index,
            });
        }
    }
    Err(SemanticPaginationError::FlowNotFound { flow: target })
}

fn page_regions(
    revision: &AcceptedRevision,
    start_page_index: usize,
) -> Result<Vec<PageRegion<AcceptedIdentity>>, SemanticPaginationError> {
    let mut regions = Vec::new();
    for page in revision.notebook.pages.iter().skip(start_page_index) {
        let Some(profile) = revision
            .notebook
            .page_profiles
            .iter()
            .find(|profile| profile.id == page.page_profile)
        else {
            return Err(SemanticPaginationError::MissingPageProfile {
                page: page.id,
                profile: page.page_profile,
            });
        };
        let valid = profile.geometry.validate().map_err(|reason| {
            SemanticPaginationError::InvalidPageProfile {
                page: page.id,
                reason,
            }
        })?;
        let writable = valid.writable_region().map_err(|reason| {
            SemanticPaginationError::InvalidPageProfile {
                page: page.id,
                reason,
            }
        })?;
        regions.push(PageRegion { page: page.id, writable });
    }
    Ok(regions)
}

fn validate_measurement_blocks(
    blocks: &[Block<AcceptedIdentity>],
    measurement: &RevisionFlowMeasurement,
) -> Result<(), SemanticPaginationError> {
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
                return if blocks.iter().any(|block| block.id == owner) {
                    Err(
                        SemanticPaginationError::
                            MeasurementBlockSequenceMismatch {
                                flow: measurement.flow,
                            },
                    )
                } else {
                    Err(SemanticPaginationError::MeasuredBlockNotInFlow {
                        flow: measurement.flow,
                        owner,
                    })
                };
            };
            if expected.id != owner {
                if blocks.iter().any(|block| block.id == owner) {
                    return Err(
                        SemanticPaginationError::
                            MeasurementBlockSequenceMismatch {
                                flow: measurement.flow,
                            },
                    );
                }
                return Err(SemanticPaginationError::MeasuredBlockNotInFlow {
                    flow: measurement.flow,
                    owner,
                });
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
