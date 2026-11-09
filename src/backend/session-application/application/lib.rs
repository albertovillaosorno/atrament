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
//   - Active-process draft, raw asset bytes, media-job cleanup state, and
//     accepted semantic notebook state.
// - Must-Not:
//   - Persist session state, parse transport input, or duplicate semantic
//     rules.
// - Allows:
//   - Inputs: Existing draft and semantic application-service operations,
//     revision-bound flow measurements, derived fixed placements, media-job
//     terminal/cleanup observations, plus raw bytes already validated by an
//     ingestion boundary.
//   - Outputs: Typed outcomes, read-only pagination and fixed-layout results,
//     checked cross-layout clearance and first-pair inspection,
//     borrowed accepted state, and raw asset bytes.
//   - Side effects: Process-local mutation through owned application services.
// - Split-When:
//   - Derived state or bounded media policy requires an independent owner.
// - Merge-When:
//   - One broader application authority owns every active-session capability.
// - Summary:
//   - Owns mutable Atrament application state for one disposable process.
// - Description:
//   - Gives draft, accepted notebook, retained asset bytes, and media cleanup
//     bookkeeping one process owner while preserving established contracts.
// - Usage:
//   - Construct one instance in the runtime composition root and drop it when
//     the active localhost session ends.
// - Defaults:
//   - Starts with empty draft fields, no accepted revision, and no asset bytes.
//

//! Process-lifetime owner for disposable Atrament application state.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use atrament_autonomous_output_intent::AutonomousOutputIntentSource;
use atrament_explicit_file_path_intent::ExplicitFilePathIntent;
use atrament_export_prerequisite_projection::{
    ExportPrerequisiteDisposition, project_export_prerequisites,
};
use atrament_export_layout_preflight::{
    ExportLayoutPreflightError, ExportLayoutPreflightResult,
    RevisionLayoutDiagnostics, preflight_layout_for_export,
};
use atrament_media_job_session::{
    MediaJobCleanupStatus, MediaJobIdentity, MediaJobIdentityExhausted,
    MediaJobOutcome, MediaJobSessionError, MediaJobSessionService,
    WaveformIntermediateIdentity,
};
use atrament_semantic_flow_pagination::{
    FixedPairOverlap, FixedRowBandOverlap, FixedRowFootprintOverlap,
    FlowFixedRegionError, RevisionFlowMeasurement, RowAlignmentSelection,
    RowHorizontalAlignment, RowHorizontalPosition,
    SemanticPaginationError, SemanticPaginationPlan,
    aligned_row_horizontal_positions_by_selection_for_flow,
    aligned_row_horizontal_positions_for_flow,
    check_fixed_regions_for_flow, first_fixed_pair_overlap_for_flow,
    first_fixed_row_band_overlap_for_flow,
    first_fixed_row_footprint_overlap_for_flow,
    fixed_pair_overlaps_for_flow, fixed_row_band_overlaps_for_flow,
    fixed_row_footprint_overlaps_for_flow, paginate_revision,
};
use atrament_semantic_fixed_region_layout::{
    AcceptedFixedPlacement, FixedRegionLayoutError, FixedRegionLayoutResult,
    validate_fixed_placement,
};
use atrament_semantic_history_result::semantic_history_direction_is_available;
use atrament_semantic_notebook::{
    AcceptedIdentity, AcceptedRevision, CandidateIdentity,
    Notebook, PhysicalPageProfile, RevisionIdentity, SemanticIdentityKind,
    TableCellSpan, TableRowRole,
};
use atrament_semantic_notebook_port::{
    AcceptanceOutcome, CommandCapabilityCompatibilityOutcome,
    CommandGraphLimits,
    CommandFamilyAdmissionOutcome, CommandTargetMaterialOutcome,
    CommandTargetPreconditionOutcome, CommandTargetPreconditions,
    DirectEditBatchApplyOutcome, DirectEditBatchGraphLimitsOutcome,
    DirectEditBatchGraphSizeOutcome, DirectEditBatchProposal,
    DirectEditBatchSelectionBoundedOutcome,
    DirectEditBatchSelectionRequirementsOutcome,
    DirectEditBatchSelectionSummaryOutcome, DirectEditBatchSimulationOutcome,
    DirectEditChangePreviewOutcome,
    DirectEditProposal, DirectEditProposalOutcome, DirectEditSimulationOutcome,
    EditableSemanticValue, EditableValuePreconditionOutcome, FormulaEditOutcome,
    FormulaReplacement,
    HistoryAvailabilityOutcome, HistoryDirection, HistoryTraversalOutcome,
    IdentityAncestryInspectOutcome, IdentityInspectOutcome,
    IdentityKindInspectOutcome, IdentityPrecondition,
    IdentityPreconditionOutcome, PageProfileEditOutcome,
    SemanticCommandCapabilitySnapshot, SemanticCommandFamily,
    SemanticNotebookHistory as _,
    SemanticNotebookSession as _, TableCellSpanEditOutcome,
    TableRowRoleEditOutcome, TextEditOutcome,
};
use atrament_semantic_notebook_session::SemanticNotebookSessionService;
use atrament_session_draft::SessionDraftService;
use atrament_session_draft_port::{DraftField, DraftMutation, SessionDraft};
use atrament_unicode_grapheme_boundary_port::GraphemeBoundaryProvider;
use atrament_unicode_grapheme_cursor::{
    GraphemeCursorError, GraphemeCursorPosition, GraphemeCursorSelection,
    resolve_grapheme_cursor_position, resolve_grapheme_cursor_selection,
    resolve_grapheme_cursor_step,
};
use atrament_unicode_grapheme_edit::{
    GraphemeRange, GraphemeRangeError, replace_grapheme_range,
};

/// One exact grapheme-range text edit against an accepted semantic target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextGraphemeRangeEdit<'edit> {
    /// Accepted revision whose exact text value is the edit base.
    pub base: RevisionIdentity,
    /// Half-open range measured in Unicode extended grapheme clusters.
    pub range: GraphemeRange,
    /// Exact replacement text retained without Unicode normalization.
    pub replacement: &'edit str,
    /// Accepted inline-text identity edited by this request.
    pub target: AcceptedIdentity,
}

/// One read-only cursor-position query against accepted inline text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextGraphemeCursorQuery {
    /// Accepted revision whose exact text value owns the cursor position.
    pub base: RevisionIdentity,
    /// Zero-based extended-grapheme boundary index supplied by the caller.
    pub grapheme_index: usize,
    /// Accepted inline-text identity inspected by this request.
    pub target: AcceptedIdentity,
}

/// One read-only signed-step query against accepted inline text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextGraphemeCursorStepQuery {
    /// Accepted revision whose exact text value owns the cursor position.
    pub base: RevisionIdentity,
    /// Current zero-based extended-grapheme boundary index.
    pub origin_index: usize,
    /// Caller-supplied signed grapheme step; no clamping is implied.
    pub step: isize,
    /// Accepted inline-text identity inspected by this request.
    pub target: AcceptedIdentity,
}

/// One read-only anchor/focus query against accepted inline text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextGraphemeSelectionQuery {
    /// Zero-based anchor boundary index retained in caller order.
    pub anchor_index: usize,
    /// Accepted revision whose exact text owns both selection endpoints.
    pub base: RevisionIdentity,
    /// Zero-based focus boundary index retained in caller order.
    pub focus_index: usize,
    /// Accepted inline-text identity inspected by this request.
    pub target: AcceptedIdentity,
}

/// Read-only semantic outcome of one grapheme cursor-position query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextGraphemeCursorOutcome {
    /// Session has no accepted semantic revision to inspect.
    NoAcceptedRevision,
    /// Exact current text admitted and resolved the requested cursor position.
    Prepared {
        /// Exact validated grapheme/byte cursor position.
        position: GraphemeCursorPosition,
        /// Accepted revision that owns the inspected text.
        revision: RevisionIdentity,
        /// Accepted inline-text identity that owns the cursor.
        target: AcceptedIdentity,
    },
    /// Caller names an accepted revision that is no longer current.
    StaleBase {
        /// Current accepted revision that rejected stale inspection.
        current: RevisionIdentity,
    },
    /// Requested accepted identity is absent from the named revision.
    TargetNotFound {
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Requested identity absent from that revision.
        target: AcceptedIdentity,
    },
    /// Existing semantic target does not expose editable inline text.
    TargetNotText {
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Existing non-text target supplied by the caller.
        target: AcceptedIdentity,
    },
}

/// Read-only semantic outcome of one grapheme selection query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextGraphemeSelectionOutcome {
    /// Session has no accepted semantic revision to inspect.
    NoAcceptedRevision,
    /// Exact current text admitted and resolved both selection endpoints.
    Prepared {
        /// Accepted revision that owns the inspected text.
        revision: RevisionIdentity,
        /// Exact validated caller-ordered selection endpoints.
        selection: GraphemeCursorSelection,
        /// Accepted inline-text identity that owns the selection.
        target: AcceptedIdentity,
    },
    /// Caller names an accepted revision that is no longer current.
    StaleBase {
        /// Current accepted revision that rejected stale inspection.
        current: RevisionIdentity,
    },
    /// Requested accepted identity is absent from the named revision.
    TargetNotFound {
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Requested identity absent from that revision.
        target: AcceptedIdentity,
    },
    /// Existing semantic target does not expose editable inline text.
    TargetNotText {
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Existing non-text target supplied by the caller.
        target: AcceptedIdentity,
    },
}

/// Typed failure to retain or inspect process-owned raw asset bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetBytesError {
    /// The semantic asset exists, but this process retains no bytes for it.
    BytesNotRetained {
        /// Accepted semantic asset identity without retained bytes.
        asset: AcceptedIdentity,
        /// Current accepted revision that admits the asset identity.
        revision: RevisionIdentity,
    },
    /// Session has no accepted semantic revision.
    NoAcceptedRevision,
    /// Caller names an accepted revision that is no longer current.
    StaleBase {
        /// Current accepted revision that rejected the stale request.
        current: RevisionIdentity,
    },
    /// Requested identity exists but is not a semantic asset.
    TargetNotAsset {
        /// Actual semantic kind owned by the requested identity.
        actual: SemanticIdentityKind,
        /// Accepted revision inspected without mutation.
        revision: RevisionIdentity,
        /// Existing non-asset semantic identity.
        target: AcceptedIdentity,
    },
    /// Requested accepted identity is absent from the named revision.
    TargetNotFound {
        /// Accepted revision inspected without mutation.
        revision: RevisionIdentity,
        /// Requested identity absent from that revision.
        target: AcceptedIdentity,
    },
}

/// Typed failure to run layout-only Export preflight through current state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionExportLayoutPreflightError {
    /// Session has no accepted semantic revision for preflight binding.
    NoAcceptedRevision,
    /// Existing layout-only preflight rejected revision or diagnostic binding.
    Preflight(ExportLayoutPreflightError),
}

/// Typed failure to validate derived fixed geometry through current state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionFixedRegionLayoutError {
    /// Accepted placement validation rejected the derived rectangle.
    Layout(FixedRegionLayoutError),
    /// Session has no accepted semantic revision for placement binding.
    NoAcceptedRevision,
}

/// Typed failure to combine flow and checked fixed areas in one live revision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionFlowFixedRegionError {
    /// Supplied fixed-region geometry failed accepted placement validation.
    FixedRegion(FixedRegionLayoutError),
    /// There is no live accepted revision for the combined layout.
    NoAcceptedRevision,
    /// The verified flow row and fixed-region geometry may overlap.
    Overlap(FlowFixedRegionError),
    /// Supplied measured flow is stale or cannot be paginated.
    Pagination(SemanticPaginationError),
}

/// Typed failure to paginate one measured flow through current session state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionPaginationError {
    /// Session has no accepted semantic revision to bind the measurement to.
    NoAcceptedRevision,
    /// Current accepted revision or measured-flow admission rejected the input.
    Pagination(SemanticPaginationError),
}

/// Result of retaining bytes for one already-accepted semantic asset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetBytesRetention {
    /// The asset already owned bytes; its original sequence was preserved.
    AlreadyRetained {
        /// Accepted semantic asset identity whose bytes were left unchanged.
        asset: AcceptedIdentity,
        /// Current accepted revision that still admits the asset identity.
        revision: RevisionIdentity,
    },
    /// The asset had no retained bytes and now owns this exact byte sequence.
    Retained {
        /// Accepted semantic asset identity that owns the bytes.
        asset: AcceptedIdentity,
        /// Complete retained byte count.
        byte_count: usize,
        /// Current accepted revision that admitted the asset identity.
        revision: RevisionIdentity,
    },
}

/// Mutable application state owned by one active Atrament process.
#[derive(Default)]
pub struct SessionApplication {
    asset_bytes: BTreeMap<AcceptedIdentity, Vec<u8>>,
    draft: SessionDraftService,
    media: MediaJobSessionService,
    semantic: SemanticNotebookSessionService,
}

impl fmt::Debug for SessionApplication {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionApplication").finish_non_exhaustive()
    }
}

impl SessionApplication {
    /// Accept one complete candidate through the owned semantic authority.
    pub fn accept_candidate(
        &mut self,
        candidate: Notebook<CandidateIdentity>,
    ) -> AcceptanceOutcome {
        self.mutate_semantic(|semantic| semantic.accept(candidate))
    }

    /// Read the current accepted semantic revision without creating another.
    #[must_use]
    pub fn accepted_revision(&self) -> Option<&AcceptedRevision> {
        self.semantic.current()
    }

    /// Inspect bounded intersections for caller-selected row alignment.
    ///
    /// Unlike arbitrary X proposals, this reconstructs horizontal origins
    /// deterministically from accepted writable geometry. Results remain
    /// provisional rectangles, not visible glyph extents or Export admission.
    ///
    /// # Errors
    ///
    /// Returns typed current-session, measurement, placement, physical
    /// footprint or result-cap refusal, never a partial report.
    pub fn aligned_fixed_row_footprint_overlaps(
        &self,
        measurement: &RevisionFlowMeasurement,
        fixed: &[AcceptedFixedPlacement],
        alignment: RowHorizontalAlignment,
        max_overlaps: usize,
    ) -> Result<Vec<FixedRowFootprintOverlap>, SessionFlowFixedRegionError>
    {
        let (plan, checked) =
            self.checked_measured_flow_and_fixed_regions(measurement, fixed)?;
        let positions =
            aligned_row_horizontal_positions_for_flow(&plan, alignment)
                .map_err(SessionFlowFixedRegionError::Overlap)?;
        fixed_row_footprint_overlaps_for_flow(
            &plan,
            &checked,
            &positions,
            max_overlaps,
        )
        .map_err(SessionFlowFixedRegionError::Overlap)
    }

    /// Apply one transport-neutral semantic batch through the owned authority.
    pub fn apply_direct_edit_batch<CommandIdentity>(
        &mut self,
        batch: DirectEditBatchProposal<CommandIdentity>,
    ) -> DirectEditBatchApplyOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        self.mutate_semantic(|semantic| semantic.apply_direct_edit_batch(batch))
    }

    /// Apply one caller-bounded semantic batch through the owned authority.
    pub fn apply_direct_edit_batch_bounded<CommandIdentity>(
        &mut self,
        batch: DirectEditBatchProposal<CommandIdentity>,
        limits: CommandGraphLimits,
    ) -> DirectEditBatchApplyOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        self.mutate_semantic(|semantic| {
            semantic.apply_direct_edit_batch_bounded(batch, limits)
        })
    }

    /// Borrow raw bytes retained for one current accepted semantic asset.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the revision is unavailable or stale, when
    /// the target is missing or not an asset, or when that asset has no
    /// retained byte sequence in this process.
    pub fn asset_bytes(
        &self,
        revision: RevisionIdentity,
        asset: AcceptedIdentity,
    ) -> Result<&[u8], AssetBytesError> {
        self.validate_asset_identity(revision, asset)?;
        self.asset_bytes.get(&asset).map(Vec::as_slice).ok_or(
            AssetBytesError::BytesNotRetained { asset, revision },
        )
    }

    /// Begin one process-local media job without creating an intermediate.
    ///
    /// # Errors
    ///
    /// Returns typed identity exhaustion if the active-process job sequence is
    /// exhausted.
    pub fn begin_media_job(
        &mut self,
    ) -> Result<MediaJobIdentity, MediaJobIdentityExhausted> {
        self.media.begin_job()
    }

    /// Check one previously bound semantic command behavior version.
    #[must_use]
    pub fn check_command_capability_compatibility(
        &self,
        expected: atrament_semantic_notebook_port::CommandBehaviorVersion,
    ) -> CommandCapabilityCompatibilityOutcome {
        self.semantic.check_command_capability_compatibility(expected)
    }

    /// Check whether one semantic command family is executable for a target.
    #[must_use]
    pub fn check_command_family_admission(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        requested: SemanticCommandFamily,
    ) -> CommandFamilyAdmissionOutcome {
        self.semantic
            .check_command_family_admission(revision, target, requested)
    }

    /// Check complete local command-target preconditions without mutation.
    #[must_use]
    pub fn check_command_target_preconditions(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        preconditions: CommandTargetPreconditions,
    ) -> CommandTargetPreconditionOutcome {
        self.semantic
            .check_command_target_preconditions(revision, target, preconditions)
    }

    /// Compare one exact editable semantic value without mutation.
    #[must_use]
    pub fn check_editable_value_precondition(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        expected: EditableSemanticValue,
    ) -> EditableValuePreconditionOutcome {
        self.semantic
            .check_editable_value_precondition(revision, target, expected)
    }

    /// Check one exact local semantic identity precondition without mutation.
    #[must_use]
    pub fn check_identity_precondition(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        precondition: IdentityPrecondition,
    ) -> IdentityPreconditionOutcome {
        self.semantic
            .check_identity_precondition(revision, target, precondition)
    }

    fn checked_measured_flow_and_fixed_regions(
        &self,
        measurement: &RevisionFlowMeasurement,
        fixed: &[AcceptedFixedPlacement],
    ) -> Result<
        (SemanticPaginationPlan, Vec<FixedRegionLayoutResult>),
        SessionFlowFixedRegionError,
    > {
        let Some(revision) = self.semantic.current() else {
            return Err(SessionFlowFixedRegionError::NoAcceptedRevision);
        };
        let plan = paginate_revision(revision, measurement)
            .map_err(SessionFlowFixedRegionError::Pagination)?;
        let checked = fixed
            .iter()
            .map(|placement| validate_fixed_placement(revision, *placement))
            .collect::<Result<Vec<_>, _>>()
            .map_err(SessionFlowFixedRegionError::FixedRegion)?;
        Ok((plan, checked))
    }

    /// Discover current transport-neutral semantic command behavior.
    #[must_use]
    pub fn command_capability_snapshot(
        &self,
    ) -> SemanticCommandCapabilitySnapshot {
        self.semantic.command_capability_snapshot()
    }

    /// Derive exact local command material for one accepted semantic target.
    #[must_use]
    pub fn command_target_material(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
    ) -> CommandTargetMaterialOutcome {
        self.semantic.command_target_material(revision, target)
    }

    /// Derive exact local command material for one target and family.
    #[must_use]
    pub fn command_target_material_for_family(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        family: SemanticCommandFamily,
    ) -> CommandTargetMaterialOutcome {
        self.semantic
            .command_target_material_for_family(revision, target, family)
    }

    /// Enforce caller-supplied coarse batch graph bounds without mutation.
    #[must_use]
    pub fn direct_edit_batch_graph_limits<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
        limits: CommandGraphLimits,
    ) -> DirectEditBatchGraphLimitsOutcome {
        self.semantic.direct_edit_batch_graph_limits(batch, limits)
    }

    /// Derive exact coarse batch graph size without mutation.
    #[must_use]
    pub fn direct_edit_batch_graph_size<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
    ) -> DirectEditBatchGraphSizeOutcome {
        self.semantic.direct_edit_batch_graph_size(batch)
    }

    /// Analyze omitted dependencies for one in-memory batch selection.
    #[must_use]
    pub fn direct_edit_batch_selection_requirements<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
        selected: &BTreeSet<CommandIdentity>,
    ) -> DirectEditBatchSelectionRequirementsOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        self.semantic
            .direct_edit_batch_selection_requirements(batch, selected)
    }

    /// Analyze omitted dependencies under one caller-supplied report bound.
    #[must_use]
    pub fn direct_edit_batch_selection_requirements_bounded<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
        selected: &BTreeSet<CommandIdentity>,
        maximum_missing_edges: usize,
    ) -> DirectEditBatchSelectionBoundedOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        self.semantic.direct_edit_batch_selection_requirements_bounded(
            batch,
            selected,
            maximum_missing_edges,
        )
    }

    /// Summarize one in-memory batch selection without materializing edges.
    #[must_use]
    pub fn direct_edit_batch_selection_summary<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
        selected: &BTreeSet<CommandIdentity>,
    ) -> DirectEditBatchSelectionSummaryOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        self.semantic
            .direct_edit_batch_selection_summary(batch, selected)
    }

    /// Mark one process-local media job terminal without hiding cleanup work.
    ///
    /// # Errors
    ///
    /// Returns a typed lifecycle error when the job is unknown or terminal.
    pub fn finish_media_job(
        &mut self,
        job: MediaJobIdentity,
        outcome: MediaJobOutcome,
    ) -> Result<MediaJobCleanupStatus, MediaJobSessionError> {
        self.media.finish_job(job, outcome)
    }

    /// Inspect the first fixed hit for caller-selected row alignment.
    ///
    /// Uses the sealed measured row width and current accepted writable region
    /// to derive left, centre or right aligned box positions. No handwriting
    /// ink or persistent layout constraint is inferred; results are read-only.
    ///
    /// # Errors
    ///
    /// Returns typed current-session, measurement, placement or physical
    /// footprint refusal before disclosing any intersection.
    pub fn first_aligned_fixed_row_footprint_overlap(
        &self,
        measurement: &RevisionFlowMeasurement,
        fixed: &[AcceptedFixedPlacement],
        alignment: RowHorizontalAlignment,
    ) -> Result<Option<FixedRowFootprintOverlap>, SessionFlowFixedRegionError>
    {
        let (plan, checked) =
            self.checked_measured_flow_and_fixed_regions(measurement, fixed)?;
        let positions =
            aligned_row_horizontal_positions_for_flow(&plan, alignment)
                .map_err(SessionFlowFixedRegionError::Overlap)?;
        first_fixed_row_footprint_overlap_for_flow(&plan, &checked, &positions)
            .map_err(SessionFlowFixedRegionError::Overlap)
    }

    /// Inspect the first checked fixed-pair intersection on live authority.
    ///
    /// The same revision-bound pagination and accepted fixed-placement
    /// validator as the combined flow query supplies exact physical geometry.
    /// Existing fixed-vs-flow clearance errors take priority over detection.
    /// `Some` identifies a possible intentional layering pair, **not** a
    /// blocking collision; `None` does not prove omitted objects are clear.
    /// This operation is read-only and cannot authorize Export.
    ///
    /// # Errors
    ///
    /// Returns a typed refusal for missing authority, stale or invalid flow
    /// measurements or fixed rectangles, or possible fixed-vs-flow overlap.
    pub fn first_fixed_pair_overlap(
        &self,
        measurement: &RevisionFlowMeasurement,
        fixed: &[AcceptedFixedPlacement],
    ) -> Result<Option<FixedPairOverlap>, SessionFlowFixedRegionError> {
        let (plan, checked) =
            self.checked_measured_flow_and_fixed_regions(measurement, fixed)?;
        first_fixed_pair_overlap_for_flow(&plan, &checked)
            .map_err(SessionFlowFixedRegionError::Overlap)
    }

    /// Inspect the first conservative fixed-versus-measured-row overlap.
    ///
    /// All fixed proposals are checked against live accepted geometry before
    /// the first indexed row-band observation is returned. Unlike the blocking
    /// clearance checker, this yields a sealed fixed index, flow and revision;
    /// unlike complete enumeration, additional valid overlaps are not errors.
    /// The full writable row band is **not** evidence of actual ink contact.
    /// This read-only query cannot authorize Export.
    ///
    /// # Errors
    ///
    /// Returns typed missing-session, measurement, checked-placement or
    /// physical geometry refusal before any observation is returned.
    pub fn first_fixed_row_band_overlap(
        &self,
        measurement: &RevisionFlowMeasurement,
        fixed: &[AcceptedFixedPlacement],
    ) -> Result<Option<FixedRowBandOverlap>, SessionFlowFixedRegionError> {
        let (plan, checked) =
            self.checked_measured_flow_and_fixed_regions(measurement, fixed)?;
        first_fixed_row_band_overlap_for_flow(&plan, &checked)
            .map_err(SessionFlowFixedRegionError::Overlap)
    }

    /// Find the first checked positioned-row/fixed-box physical intersection.
    ///
    /// All fixed proposals and all horizontal positions are validated against
    /// the current accepted revision before returning the first observation.
    /// Unlike complete capped enumeration, additional overlaps do not refuse
    /// this query. Provisional boxes do not establish actual handwriting ink
    /// contact, a fixed-layering policy or native Export readiness.
    ///
    /// # Errors
    ///
    /// Returns typed missing-session, invalid measurement/placement, or
    /// horizontal source/geometry admission failures.
    pub fn first_fixed_row_footprint_overlap(
        &self,
        measurement: &RevisionFlowMeasurement,
        fixed: &[AcceptedFixedPlacement],
        positions: &[RowHorizontalPosition],
    ) -> Result<Option<FixedRowFootprintOverlap>, SessionFlowFixedRegionError>
    {
        let (plan, checked) =
            self.checked_measured_flow_and_fixed_regions(measurement, fixed)?;
        first_fixed_row_footprint_overlap_for_flow(&plan, &checked, positions)
            .map_err(SessionFlowFixedRegionError::Overlap)
    }

    /// Inspect the first fixed intersection for per-row selected alignment.
    ///
    /// The entire revision/source-bound selection sequence must match the
    /// current measured plan before its provisional row positions are used.
    /// This does not approve layering or replace conservative row-band gates.
    ///
    /// # Errors
    ///
    /// Returns typed missing-session, measurement, placement, selection or
    /// geometry refusal before disclosing any first-hit observation.
    pub fn first_selected_aligned_fixed_row_footprint_overlap(
        &self,
        measurement: &RevisionFlowMeasurement,
        fixed: &[AcceptedFixedPlacement],
        selections: &[RowAlignmentSelection],
    ) -> Result<Option<FixedRowFootprintOverlap>, SessionFlowFixedRegionError>
    {
        let (plan, checked) =
            self.checked_measured_flow_and_fixed_regions(measurement, fixed)?;
        let positions = aligned_row_horizontal_positions_by_selection_for_flow(
            &plan, selections,
        )
        .map_err(SessionFlowFixedRegionError::Overlap)?;
        first_fixed_row_footprint_overlap_for_flow(&plan, &checked, &positions)
            .map_err(SessionFlowFixedRegionError::Overlap)
    }

    /// Inspect all supplied fixed-pair intersections on current authority.
    ///
    /// Validation runs for the complete caller-supplied batch before any
    /// intersections are considered. `max_pairs` is an exact output cap;
    /// exceeding it refuses without exposing a truncated set of overlaps.
    /// These records remain nonblocking geometry diagnostics, not permission
    /// or refusal to layer objects, and cannot authorize native Export.
    ///
    /// # Errors
    ///
    /// Returns a typed missing-session, invalid measurement/placement,
    /// conservative row-band or fixed-pair result-limit refusal.
    pub fn fixed_pair_overlaps(
        &self,
        measurement: &RevisionFlowMeasurement,
        fixed: &[AcceptedFixedPlacement],
        max_pairs: usize,
    ) -> Result<Vec<FixedPairOverlap>, SessionFlowFixedRegionError> {
        let (plan, checked) =
            self.checked_measured_flow_and_fixed_regions(measurement, fixed)?;
        fixed_pair_overlaps_for_flow(&plan, &checked, max_pairs)
            .map_err(SessionFlowFixedRegionError::Overlap)
    }

    /// Inspect all supplied fixed-versus-measured-row vertical overlaps.
    ///
    /// Every fixed placement is validated against current accepted geometry
    /// before bounded diagnostics are collected. Overlap coordinates reserve
    /// the measured row's entire horizontal writable region; these are
    /// possible conflicts, not measured horizontal ink intersections.
    /// No partial list is returned when `max_overlaps` is exceeded.
    ///
    /// # Errors
    ///
    /// Returns a typed missing-session, invalid measurement/placement,
    /// physical geometry or result-limit refusal.
    pub fn fixed_row_band_overlaps(
        &self,
        measurement: &RevisionFlowMeasurement,
        fixed: &[AcceptedFixedPlacement],
        max_overlaps: usize,
    ) -> Result<Vec<FixedRowBandOverlap>, SessionFlowFixedRegionError> {
        let (plan, checked) =
            self.checked_measured_flow_and_fixed_regions(measurement, fixed)?;
        fixed_row_band_overlaps_for_flow(&plan, &checked, max_overlaps)
            .map_err(SessionFlowFixedRegionError::Overlap)
    }

    /// Inspect explicit horizontal measured-row footprints on live authority.
    ///
    /// The caller supplies one exact revision/page/source-bound row position
    /// per paginated fragment. Every proposed position and fixed rectangle is
    /// checked before returning bounded physical intersection diagnostics.
    /// Unlike the conservative fixed-row band gate, this optional query uses
    /// caller-positioned measured row widths, **not** visible handwriting ink.
    /// It cannot authorize collision-free Export or disable band clearance.
    ///
    /// # Errors
    ///
    /// Returns typed missing-session, invalid measurement/fixed placement,
    /// row position, physical overflow or result-cap refusal.
    pub fn fixed_row_footprint_overlaps(
        &self,
        measurement: &RevisionFlowMeasurement,
        fixed: &[AcceptedFixedPlacement],
        positions: &[RowHorizontalPosition],
        max_overlaps: usize,
    ) -> Result<Vec<FixedRowFootprintOverlap>, SessionFlowFixedRegionError> {
        let (plan, checked) =
            self.checked_measured_flow_and_fixed_regions(measurement, fixed)?;
        fixed_row_footprint_overlaps_for_flow(
            &plan, &checked, positions, max_overlaps,
        ).map_err(SessionFlowFixedRegionError::Overlap)
    }

    /// Inspect in-memory semantic Undo and Redo availability.
    #[must_use]
    pub fn history_availability(&self) -> HistoryAvailabilityOutcome {
        self.semantic.history_availability()
    }

    /// Inspect one accepted semantic identity without mutation.
    #[must_use]
    pub fn inspect_identity(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
    ) -> IdentityInspectOutcome {
        self.semantic.inspect_identity(revision, target)
    }

    /// Inspect one bounded target-first semantic owner chain without mutation.
    #[must_use]
    pub fn inspect_identity_ancestry_bounded(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        maximum_results: usize,
    ) -> IdentityAncestryInspectOutcome {
        self.semantic.inspect_identity_ancestry_bounded(
            revision,
            target,
            maximum_results,
        )
    }

    /// Inspect one accepted semantic identity kind without mutation.
    #[must_use]
    pub fn inspect_identity_kind(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
    ) -> IdentityKindInspectOutcome {
        self.semantic.inspect_identity_kind(revision, target)
    }

    /// Inspect terminal cleanup status for one process-local media job.
    ///
    /// # Errors
    ///
    /// Returns a typed lifecycle error for unknown or still-active jobs.
    pub fn media_job_cleanup_status(
        &self,
        job: MediaJobIdentity,
    ) -> Result<MediaJobCleanupStatus, MediaJobSessionError> {
        self.media.cleanup_status(job)
    }

    /// Return media-job emptiness for root-fixture lifetime assertions.
    #[cfg(test)]
    #[must_use]
    pub fn media_jobs_are_empty_for_test(&self) -> bool {
        self.media.is_empty()
    }

    fn mutate_semantic<Outcome>(
        &mut self,
        operation: impl FnOnce(&mut SemanticNotebookSessionService) -> Outcome,
    ) -> Outcome {
        let had_redo = self.redo_is_available();
        let outcome = operation(&mut self.semantic);
        self.prune_discarded_redo_asset_bytes(had_redo);
        outcome
    }

    /// Paginate one revision-bound measured flow through current authority.
    ///
    /// This operation is read-only. It never accepts arbitrary page rectangles:
    /// page geometry is derived by the semantic pagination service from the
    /// current accepted revision's page-profile authority.
    ///
    /// # Errors
    ///
    /// Returns a typed failure when no accepted revision exists, when the
    /// measurement belongs to another revision or flow, its block sequence is
    /// invalid, page
    /// authority is invalid, or measured fragments cannot be placed.
    pub fn paginate_measured_flow(
        &self,
        measurement: &RevisionFlowMeasurement,
    ) -> Result<SemanticPaginationPlan, SessionPaginationError> {
        let Some(revision) = self.semantic.current() else {
            return Err(SessionPaginationError::NoAcceptedRevision);
        };
        paginate_revision(revision, measurement)
            .map_err(SessionPaginationError::Pagination)
    }

    /// Inspect one measured flow and supplied fixed regions on live authority.
    ///
    /// This operation checks every supplied fixed region against the same
    /// accepted revision that is used for measured-flow pagination. It refuses
    /// any possible vertical-band overlap, including boxes at the far side of
    /// the writable page because horizontal ink alignment is not measured.
    /// Its success does not assert that all fixed objects were supplied or
    /// authorize collision-free rendering or native Export.
    ///
    /// # Errors
    ///
    /// Returns a typed failure for missing current authority, stale or
    /// invalid measurements/placements, or potential row-band overlap.
    pub fn paginate_measured_flow_with_fixed_regions(
        &self,
        measurement: &RevisionFlowMeasurement,
        fixed: &[AcceptedFixedPlacement],
    ) -> Result<SemanticPaginationPlan, SessionFlowFixedRegionError> {
        let (plan, checked) =
            self.checked_measured_flow_and_fixed_regions(measurement, fixed)?;
        check_fixed_regions_for_flow(&plan, &checked)
            .map_err(SessionFlowFixedRegionError::Overlap)?;
        Ok(plan)
    }

    /// Inspect explicit Export source, path and layout prerequisites.
    ///
    /// This read-only operation binds layout evidence to the active session's
    /// exact accepted revision. Passing this gate does not authorize file I/O:
    /// serializers, output artifacts, path policy, overwrites and commits
    /// remain separate. Source claims must be proven by a trusted caller.
    ///
    /// # Errors
    ///
    /// Returns a typed failure for absent, stale, or misbound revision/layout
    /// authority before projecting any path or output intent result.
    pub fn preflight_explicit_export_prerequisites(
        &self,
        source: AutonomousOutputIntentSource,
        path: &ExplicitFilePathIntent,
        requested: RevisionIdentity,
        layout: RevisionLayoutDiagnostics<'_>,
    ) -> Result<
        ExportPrerequisiteDisposition,
        SessionExportLayoutPreflightError,
    > {
        let bound = self.preflight_layout_for_export(requested, layout)?;
        Ok(project_export_prerequisites(source, path, &bound))
    }

    /// Evaluate layout-only Export preflight through current session authority.
    ///
    /// This read-only operation accepts no path, format, overwrite, retry, or
    /// file-commit input and cannot report a persistent Export result.
    ///
    /// # Errors
    ///
    /// Returns a typed failure when no accepted revision exists or the supplied
    /// layout evidence is stale, misbound, or belongs to another capability.
    pub fn preflight_layout_for_export(
        &self,
        requested: RevisionIdentity,
        layout: RevisionLayoutDiagnostics<'_>,
    ) -> Result<
        ExportLayoutPreflightResult,
        SessionExportLayoutPreflightError,
    > {
        let Some(current) = self.semantic.current() else {
            return Err(SessionExportLayoutPreflightError::NoAcceptedRevision);
        };
        preflight_layout_for_export(current, requested, layout)
            .map_err(SessionExportLayoutPreflightError::Preflight)
    }

    /// Preview one exact direct semantic change without mutation.
    #[must_use]
    pub fn preview_direct_edit_changes(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        requested: EditableSemanticValue,
    ) -> DirectEditChangePreviewOutcome {
        self.semantic
            .preview_direct_edit_changes(revision, target, requested)
    }

    fn prune_discarded_redo_asset_bytes(&mut self, had_redo: bool) {
        if !had_redo
            || self.redo_is_available()
            || self.asset_bytes.is_empty()
        {
            return;
        }
        let reachable = self.semantic.history_reachable_asset_identities();
        self.asset_bytes
            .retain(|asset, _bytes| reachable.contains(asset));
    }

    /// Record a failed media cleanup attempt as retry-required.
    ///
    /// # Errors
    ///
    /// Returns a typed lifecycle error for invalid job or intermediate state.
    pub fn record_media_cleanup_failure(
        &mut self,
        job: MediaJobIdentity,
        intermediate: WaveformIntermediateIdentity,
    ) -> Result<MediaJobCleanupStatus, MediaJobSessionError> {
        self.media.record_cleanup_failure(job, intermediate)
    }

    /// Record successful cleanup after the owned intermediate is gone.
    ///
    /// # Errors
    ///
    /// Returns a typed lifecycle error for invalid job or intermediate state.
    pub fn record_media_cleanup_success(
        &mut self,
        job: MediaJobIdentity,
        intermediate: WaveformIntermediateIdentity,
    ) -> Result<MediaJobCleanupStatus, MediaJobSessionError> {
        self.media.record_cleanup_success(job, intermediate)
    }

    fn redo_is_available(&self) -> bool {
        semantic_history_direction_is_available(
            &self.semantic.history_availability(),
            HistoryDirection::Redo,
        )
    }

    /// Register one job-owned waveform intermediate in process memory.
    ///
    /// This records lifecycle ownership only and never creates a file.
    ///
    /// # Errors
    ///
    /// Returns a typed lifecycle error when the job is unknown, terminal, or
    /// already owns an intermediate, or identity allocation is exhausted.
    pub fn register_media_waveform_intermediate(
        &mut self,
        job: MediaJobIdentity,
    ) -> Result<WaveformIntermediateIdentity, MediaJobSessionError> {
        self.media.register_waveform_intermediate(job)
    }

    /// Replace one mathematical source through the owned semantic authority.
    pub fn replace_formula(
        &mut self,
        base: RevisionIdentity,
        target: AcceptedIdentity,
        replacement: FormulaReplacement,
    ) -> FormulaEditOutcome {
        self.mutate_semantic(|semantic| {
            semantic.replace_formula(base, target, replacement)
        })
    }

    /// Replace one physical page profile through the owned semantic authority.
    pub fn replace_page_profile(
        &mut self,
        base: RevisionIdentity,
        target: AcceptedIdentity,
        geometry: PhysicalPageProfile,
    ) -> PageProfileEditOutcome {
        self.mutate_semantic(|semantic| {
            semantic.replace_page_profile(base, target, geometry)
        })
    }

    /// Replace one table-cell span through the owned semantic authority.
    pub fn replace_table_cell_span(
        &mut self,
        base: RevisionIdentity,
        target: AcceptedIdentity,
        span: TableCellSpan,
    ) -> TableCellSpanEditOutcome {
        self.mutate_semantic(|semantic| {
            semantic.replace_table_cell_span(base, target, span)
        })
    }

    /// Replace one table-row role through the owned semantic authority.
    pub fn replace_table_row_role(
        &mut self,
        base: RevisionIdentity,
        target: AcceptedIdentity,
        role: TableRowRole,
    ) -> TableRowRoleEditOutcome {
        self.mutate_semantic(|semantic| {
            semantic.replace_table_row_role(base, target, role)
        })
    }

    /// Replace one inline text value through the owned semantic authority.
    pub fn replace_text(
        &mut self,
        base: RevisionIdentity,
        target: AcceptedIdentity,
        value: String,
    ) -> TextEditOutcome {
        self.mutate_semantic(|semantic| {
            semantic.replace_text(base, target, value)
        })
    }

    /// Replace one grapheme range inside an accepted inline text value.
    ///
    /// The exact current text is derived from `base` before grapheme boundaries
    /// are queried. The resulting complete string is committed through the
    /// existing atomic whole-text replacement authority.
    ///
    /// # Errors
    ///
    /// Returns a typed grapheme-range failure before semantic mutation when the
    /// requested start/end cannot address the exact current text.
    pub fn replace_text_grapheme_range(
        &mut self,
        boundaries: &dyn GraphemeBoundaryProvider,
        edit: TextGraphemeRangeEdit<'_>,
    ) -> Result<TextEditOutcome, GraphemeRangeError> {
        let TextGraphemeRangeEdit {
            base,
            range,
            replacement,
            target,
        } = edit;
        let material_outcome = self.command_target_material_for_family(
            base,
            target,
            SemanticCommandFamily::TextContent,
        );
        let current_text = match material_outcome {
            CommandTargetMaterialOutcome::NoAcceptedRevision => {
                return Ok(TextEditOutcome::NoAcceptedRevision);
            },
            CommandTargetMaterialOutcome::Prepared { material } => {
                match material.editable_value {
                    Some(EditableSemanticValue::Text(value)) => value,
                    _ => {
                        return Ok(TextEditOutcome::TargetNotText {
                            revision: material.revision,
                            target: material.target,
                        });
                    },
                }
            },
            CommandTargetMaterialOutcome::StaleBase { current } => {
                return Ok(TextEditOutcome::StaleBase { current });
            },
            CommandTargetMaterialOutcome::TargetNotFound {
                revision,
                target: missing_target,
            } => {
                return Ok(TextEditOutcome::TargetNotFound {
                    revision,
                    target: missing_target,
                });
            },
        };
        let value = replace_grapheme_range(
            boundaries,
            &current_text,
            range,
            replacement,
        )?;
        Ok(self.replace_text(base, target, value))
    }

    /// Retain already-validated raw bytes for one accepted semantic asset.
    ///
    /// Existing bytes are immutable for that accepted asset identity. Repeated
    /// retention preserves the original bytes instead of changing render input
    /// behind a stable semantic reference. This is not a media-ingestion or
    /// media-format validation capability.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the revision is unavailable or stale, when
    /// the target is missing or not an asset, or when no accepted revision
    /// exists.
    pub fn retain_asset_bytes(
        &mut self,
        revision: RevisionIdentity,
        asset: AcceptedIdentity,
        bytes: Vec<u8>,
    ) -> Result<AssetBytesRetention, AssetBytesError> {
        self.validate_asset_identity(revision, asset)?;
        match self.asset_bytes.entry(asset) {
            Entry::Occupied(_) => {
                Ok(AssetBytesRetention::AlreadyRetained { asset, revision })
            },
            Entry::Vacant(entry) => {
                let byte_count = bytes.len();
                let _bytes = entry.insert(bytes);
                Ok(AssetBytesRetention::Retained {
                    asset,
                    byte_count,
                    revision,
                })
            },
        }
    }

    /// Return retained asset-entry count for root-fixture lifetime assertions.
    #[cfg(test)]
    #[must_use]
    pub fn retained_asset_byte_count_for_test(&self) -> usize {
        self.asset_bytes.len()
    }

    /// Inspect bounded fixed intersections for per-row selected alignments.
    ///
    /// The complete selection sequence is bound to the live revision, flow,
    /// page and measured source. A heading and its following paragraph may
    /// therefore choose different provisional X positions without injecting
    /// arbitrary coordinates or claiming visible ink collision clearance.
    ///
    /// # Errors
    ///
    /// Returns typed missing-session, stale measurement, checked-placement,
    /// row-selection, geometry or result-cap refusal without a partial report.
    pub fn selected_aligned_fixed_row_footprint_overlaps(
        &self,
        measurement: &RevisionFlowMeasurement,
        fixed: &[AcceptedFixedPlacement],
        selections: &[RowAlignmentSelection],
        max_overlaps: usize,
    ) -> Result<Vec<FixedRowFootprintOverlap>, SessionFlowFixedRegionError>
    {
        let (plan, checked) =
            self.checked_measured_flow_and_fixed_regions(measurement, fixed)?;
        let positions = aligned_row_horizontal_positions_by_selection_for_flow(
            &plan, selections,
        )
        .map_err(SessionFlowFixedRegionError::Overlap)?;
        fixed_row_footprint_overlaps_for_flow(
            &plan, &checked, &positions, max_overlaps,
        )
        .map_err(SessionFlowFixedRegionError::Overlap)
    }

    /// Simulate one established direct semantic edit without mutation.
    #[must_use]
    pub fn simulate_direct_edit(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        requested: EditableSemanticValue,
    ) -> DirectEditSimulationOutcome {
        self.semantic.simulate_direct_edit(revision, target, requested)
    }

    /// Simulate one transport-neutral direct-edit batch without mutation.
    #[must_use]
    pub fn simulate_direct_edit_batch<CommandIdentity>(
        &self,
        batch: DirectEditBatchProposal<CommandIdentity>,
    ) -> DirectEditBatchSimulationOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        self.semantic.simulate_direct_edit_batch(batch)
    }

    /// Simulate one caller-bounded direct-edit batch without mutation.
    #[must_use]
    pub fn simulate_direct_edit_batch_bounded<CommandIdentity>(
        &self,
        batch: DirectEditBatchProposal<CommandIdentity>,
        limits: CommandGraphLimits,
    ) -> DirectEditBatchSimulationOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        self.semantic.simulate_direct_edit_batch_bounded(batch, limits)
    }

    /// Validate and simulate one version-bound direct-edit proposal read-only.
    #[must_use]
    pub fn simulate_direct_edit_proposal(
        &self,
        proposal: DirectEditProposal,
    ) -> DirectEditProposalOutcome {
        self.semantic.simulate_direct_edit_proposal(proposal)
    }

    /// Resolve one grapheme cursor position in exact accepted inline text.
    ///
    /// Semantic revision/target admission runs before the grapheme provider is
    /// queried. This is read-only and does not choose cursor movement,
    /// selection, clamping, visual affinity, or transport behavior.
    ///
    /// # Errors
    ///
    /// Returns a typed cursor-position failure only after the exact current
    /// text target has been admitted for the named revision.
    pub fn text_grapheme_cursor_position(
        &self,
        boundaries: &dyn GraphemeBoundaryProvider,
        query: TextGraphemeCursorQuery,
    ) -> Result<TextGraphemeCursorOutcome, GraphemeCursorError> {
        let TextGraphemeCursorQuery {
            base,
            grapheme_index,
            target,
        } = query;
        let material_outcome = self.command_target_material_for_family(
            base,
            target,
            SemanticCommandFamily::TextContent,
        );
        let current_text = match material_outcome {
            CommandTargetMaterialOutcome::NoAcceptedRevision => {
                return Ok(TextGraphemeCursorOutcome::NoAcceptedRevision);
            },
            CommandTargetMaterialOutcome::Prepared { material } => {
                match material.editable_value {
                    Some(EditableSemanticValue::Text(value)) => value,
                    _ => {
                        return Ok(TextGraphemeCursorOutcome::TargetNotText {
                            revision: material.revision,
                            target: material.target,
                        });
                    },
                }
            },
            CommandTargetMaterialOutcome::StaleBase { current } => {
                return Ok(TextGraphemeCursorOutcome::StaleBase { current });
            },
            CommandTargetMaterialOutcome::TargetNotFound {
                revision,
                target: missing_target,
            } => {
                return Ok(TextGraphemeCursorOutcome::TargetNotFound {
                    revision,
                    target: missing_target,
                });
            },
        };
        let position = resolve_grapheme_cursor_position(
            boundaries,
            &current_text,
            grapheme_index,
        )?;
        Ok(TextGraphemeCursorOutcome::Prepared {
            position,
            revision: base,
            target,
        })
    }

    /// Resolve one caller-supplied signed grapheme step in accepted text.
    ///
    /// Semantic revision/target admission runs before the grapheme provider is
    /// queried. The application does not choose direction, step size, clamping,
    /// selection extension, visual affinity, or transport behavior.
    ///
    /// # Errors
    ///
    /// Returns a typed cursor-step failure only after the exact current text
    /// target has been admitted for the named revision.
    pub fn text_grapheme_cursor_step(
        &self,
        boundaries: &dyn GraphemeBoundaryProvider,
        query: TextGraphemeCursorStepQuery,
    ) -> Result<TextGraphemeCursorOutcome, GraphemeCursorError> {
        let TextGraphemeCursorStepQuery {
            base,
            origin_index,
            step,
            target,
        } = query;
        let material_outcome = self.command_target_material_for_family(
            base,
            target,
            SemanticCommandFamily::TextContent,
        );
        let current_text = match material_outcome {
            CommandTargetMaterialOutcome::NoAcceptedRevision => {
                return Ok(TextGraphemeCursorOutcome::NoAcceptedRevision);
            },
            CommandTargetMaterialOutcome::Prepared { material } => {
                match material.editable_value {
                    Some(EditableSemanticValue::Text(value)) => value,
                    _ => {
                        return Ok(TextGraphemeCursorOutcome::TargetNotText {
                            revision: material.revision,
                            target: material.target,
                        });
                    },
                }
            },
            CommandTargetMaterialOutcome::StaleBase { current } => {
                return Ok(TextGraphemeCursorOutcome::StaleBase { current });
            },
            CommandTargetMaterialOutcome::TargetNotFound {
                revision,
                target: missing_target,
            } => {
                return Ok(TextGraphemeCursorOutcome::TargetNotFound {
                    revision,
                    target: missing_target,
                });
            },
        };
        let position = resolve_grapheme_cursor_step(
            boundaries,
            &current_text,
            origin_index,
            step,
        )?;
        Ok(TextGraphemeCursorOutcome::Prepared {
            position,
            revision: base,
            target,
        })
    }

    /// Resolve one caller-ordered grapheme selection in accepted inline text.
    ///
    /// Semantic revision/target admission runs before the grapheme provider is
    /// queried. This is read-only and does not choose movement, extension,
    /// clamping, visual affinity, or transport behavior.
    ///
    /// # Errors
    ///
    /// Returns a typed endpoint failure only after the exact current text
    /// target has been admitted for the named revision.
    pub fn text_grapheme_selection(
        &self,
        boundaries: &dyn GraphemeBoundaryProvider,
        query: TextGraphemeSelectionQuery,
    ) -> Result<TextGraphemeSelectionOutcome, GraphemeCursorError> {
        let TextGraphemeSelectionQuery {
            anchor_index,
            base,
            focus_index,
            target,
        } = query;
        let material_outcome = self.command_target_material_for_family(
            base,
            target,
            SemanticCommandFamily::TextContent,
        );
        let current_text = match material_outcome {
            CommandTargetMaterialOutcome::NoAcceptedRevision => {
                return Ok(TextGraphemeSelectionOutcome::NoAcceptedRevision);
            },
            CommandTargetMaterialOutcome::Prepared { material } => {
                match material.editable_value {
                    Some(EditableSemanticValue::Text(value)) => value,
                    _ => {
                        return Ok(TextGraphemeSelectionOutcome::TargetNotText {
                            revision: material.revision,
                            target: material.target,
                        });
                    },
                }
            },
            CommandTargetMaterialOutcome::StaleBase { current } => {
                return Ok(TextGraphemeSelectionOutcome::StaleBase { current });
            },
            CommandTargetMaterialOutcome::TargetNotFound {
                revision,
                target: missing_target,
            } => {
                return Ok(TextGraphemeSelectionOutcome::TargetNotFound {
                    revision,
                    target: missing_target,
                });
            },
        };
        let selection = resolve_grapheme_cursor_selection(
            boundaries,
            &current_text,
            anchor_index,
            focus_index,
        )?;
        Ok(TextGraphemeSelectionOutcome::Prepared {
            revision: base,
            selection,
            target,
        })
    }

    /// Traverse one in-memory semantic history transaction.
    pub fn traverse_history(
        &mut self,
        base: RevisionIdentity,
        direction: HistoryDirection,
    ) -> HistoryTraversalOutcome {
        self.semantic.traverse_history(base, direction)
    }

    fn validate_asset_identity(
        &self,
        revision: RevisionIdentity,
        asset: AcceptedIdentity,
    ) -> Result<(), AssetBytesError> {
        match self.semantic.inspect_identity_kind(revision, asset) {
            IdentityKindInspectOutcome::Inspected {
                kind: SemanticIdentityKind::Asset,
                ..
            } => Ok(()),
            IdentityKindInspectOutcome::Inspected {
                kind: actual,
                revision: inspected_revision,
                target,
            } => Err(AssetBytesError::TargetNotAsset {
                actual,
                revision: inspected_revision,
                target,
            }),
            IdentityKindInspectOutcome::NoAcceptedRevision => {
                Err(AssetBytesError::NoAcceptedRevision)
            },
            IdentityKindInspectOutcome::StaleBase { current } => {
                Err(AssetBytesError::StaleBase { current })
            },
            IdentityKindInspectOutcome::TargetNotFound {
                revision: inspected_revision,
                target,
            } => Err(AssetBytesError::TargetNotFound {
                revision: inspected_revision,
                target,
            }),
        }
    }

    /// Validate one revision-bound fixed rectangle through current authority.
    ///
    /// This read-only operation delegates page ownership and page-profile
    /// geometry, bounds checks, and overflow diagnostics to the accepted layout
    /// service.
    /// It does not solve placement or invent anchors, collision policy, or
    /// remediation choices.
    ///
    /// # Errors
    ///
    /// Returns a typed failure when no accepted revision exists or when the
    /// derived placement is stale, mis-owned, invalid, or unrepresentable.
    pub fn validate_fixed_placement(
        &self,
        placement: AcceptedFixedPlacement,
    ) -> Result<FixedRegionLayoutResult, SessionFixedRegionLayoutError> {
        let Some(revision) = self.semantic.current() else {
            return Err(SessionFixedRegionLayoutError::NoAcceptedRevision);
        };
        validate_fixed_placement(revision, placement)
            .map_err(SessionFixedRegionLayoutError::Layout)
    }

}

impl SessionDraft for SessionApplication {
    fn replace(&mut self, field: DraftField, value: String) -> DraftMutation {
        self.draft.replace(field, value)
    }

    fn value(&self, field: DraftField) -> &str {
        self.draft.value(field)
    }
}
