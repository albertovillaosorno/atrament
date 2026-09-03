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
//   - Inbound application contract for explicit candidate notebook acceptance.
// - Must-Not:
//   - Parse model text, choose wire IDs, persist revisions, or perform layout.
// - Allows:
//   - Inputs: One already-constructed candidate semantic notebook.
//   - Outputs: Atomic acceptance outcome, identity mapping, and current
//     revision.
//   - Side effects: Application-owned in-memory accepted revision mutation.
// - Split-When:
//   - Candidate review and semantic command Apply need separate capabilities.
// - Merge-When:
//   - Candidate acceptance becomes part of another semantic application port.
// - Summary:
//   - Defines the application boundary that promotes candidate semantics.
// - Description:
//   - Keeps candidate-local identities distinct until explicit acceptance.
// - Usage:
//   - Implement in one active session service and inject into inbound adapters.
// - Defaults:
//   - Rejected candidates leave the current accepted revision unchanged.
//

//! Inbound application port for transactional semantic candidate acceptance.

use std::collections::BTreeSet;

use atrament_semantic_command_graph::{
    CommandGraphError, CommandGraphLimitError, CommandGraphLimits,
    CommandGraphSize, DependencySelectionSummary, MissingDependencyRequirement,
};
use atrament_semantic_notebook::{
    AcceptedIdentity, AcceptedRevision, CandidateIdentity, FormulaMode,
    IdentityExhausted, MathSyntaxError, Notebook, PhysicalPageProfile,
    PhysicalPageProfileError, RevisionIdentity, SemanticIdentityDescriptor,
    SemanticIdentityKind, TableRowRole,
};

/// Maximum admitted block-containment depth for one candidate acceptance.
pub const CANDIDATE_BLOCK_NESTING_LIMIT: usize = 256;

/// Result of one explicit candidate acceptance request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AcceptanceOutcome {
    /// Candidate committed atomically as one new accepted revision.
    Accepted {
        /// Candidate-to-accepted identity mapping finalized at commit.
        mapping: Vec<IdentityMapping>,
        /// New commit-owned accepted revision identity.
        revision: RevisionIdentity,
    },
    /// Backend identity authority exhausted before a commit could complete.
    IdentityExhausted {
        /// Identity sequence that could not allocate another value.
        sequence: IdentityExhausted,
    },
    /// Candidate identity graph is invalid and no accepted mutation occurred.
    InvalidCandidate {
        /// Typed candidate graph failure.
        reason: CandidateGraphError,
    },
}

/// Candidate identity graph failure detected before accepted mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateGraphError {
    /// One candidate-local identity is owned by more than one semantic object.
    Duplicate {
        /// Reused candidate-local identity.
        candidate: CandidateIdentity,
    },
    /// One mathematical source is structurally malformed for its declared mode.
    InvalidMathematics {
        /// Candidate-local formula identity whose source is malformed.
        candidate: CandidateIdentity,
        /// Typed mathematical source-shape failure.
        reason: MathSyntaxError,
    },
    /// One owned physical page profile has invalid exact geometry.
    InvalidPageProfile {
        /// Candidate-local profile identity whose geometry is invalid.
        candidate: CandidateIdentity,
        /// Typed physical geometry validation failure.
        reason: PhysicalPageProfileError,
    },
    /// One candidate semantic reference does not name an owned candidate
    /// object.
    MissingReference {
        /// Candidate-local identity that could not be resolved.
        candidate: CandidateIdentity,
    },
    /// Candidate block containment exceeds the admitted resource bound.
    NestingLimitExceeded {
        /// First candidate block beyond the admitted nesting bound.
        candidate: CandidateIdentity,
        /// Maximum admitted block-containment depth.
        limit: usize,
    },
    /// One reference resolves to an owner of the wrong semantic class.
    ReferenceKindMismatch {
        /// Candidate-local identity resolving to the wrong owner class.
        candidate: CandidateIdentity,
        /// Semantic owner class required at this reference site.
        expected: CandidateReferenceKind,
    },
    /// One balanced mathematical source uses unsupported TeX-like constructs.
    UnsupportedMathematics {
        /// Candidate-local formula identity requiring unresolved treatment.
        candidate: CandidateIdentity,
    },
}

/// Semantic owner class required by one candidate identity reference.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CandidateReferenceKind {
    /// Reference must identify a semantic asset.
    Asset,
    /// Reference must identify a physical page profile.
    PageProfile,
    /// Reference must identify a provenance record.
    Provenance,
    /// Reference may identify any owned semantic object.
    Semantic,
    /// Reference must identify a semantic style.
    Style,
}

/// Result of one direct accepted mathematical source replacement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormulaEditOutcome {
    /// Formula changed and one new accepted revision committed.
    Applied {
        /// Accepted revision used as the edit precondition.
        base: RevisionIdentity,
        /// New accepted revision produced by the edit.
        revision: RevisionIdentity,
        /// Existing semantic formula identity whose source or mode changed.
        target: AcceptedIdentity,
    },
    /// Revision identity allocation exhausted before commit.
    IdentityExhausted {
        /// Identity sequence that could not allocate another value.
        sequence: IdentityExhausted,
    },
    /// Replacement mathematics is structurally malformed; no mutation occurred.
    InvalidMathematics {
        /// Typed mathematical source-shape failure.
        reason: MathSyntaxError,
        /// Unchanged current revision identity.
        revision: RevisionIdentity,
        /// Existing formula identity that rejected replacement.
        target: AcceptedIdentity,
    },
    /// Session has no accepted semantic revision to edit.
    NoAcceptedRevision,
    /// Replacement equals current mode and source; no revision churn occurred.
    NoOp {
        /// Unchanged current revision identity.
        revision: RevisionIdentity,
        /// Existing formula identity whose value already matched.
        target: AcceptedIdentity,
    },
    /// Caller precondition names a revision that is no longer current.
    StaleBase {
        /// Current accepted revision identity that rejected the stale edit.
        current: RevisionIdentity,
    },
    /// Requested accepted identity exists but does not own mathematics.
    TargetNotFormula {
        /// Unchanged current revision identity.
        revision: RevisionIdentity,
        /// Existing non-formula semantic identity that rejected replacement.
        target: AcceptedIdentity,
    },
    /// Requested accepted identity is absent from the current revision.
    TargetNotFound {
        /// Unchanged current revision identity.
        revision: RevisionIdentity,
        /// Requested semantic identity absent from the current revision.
        target: AcceptedIdentity,
    },
    /// Replacement uses unsupported TeX-like constructs; no mutation occurred.
    UnsupportedMathematics {
        /// Unchanged current revision identity.
        revision: RevisionIdentity,
        /// Existing formula identity that rejected replacement.
        target: AcceptedIdentity,
    },
}

/// Result of replacing one accepted physical page profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageProfileEditOutcome {
    /// Physical profile changed and one new accepted revision committed.
    Applied {
        /// Accepted revision used as the edit precondition.
        base: RevisionIdentity,
        /// New accepted revision produced by the edit.
        revision: RevisionIdentity,
        /// Existing physical page-profile identity whose value changed.
        target: AcceptedIdentity,
    },
    /// Revision identity allocation exhausted before commit.
    IdentityExhausted {
        /// Identity sequence that could not allocate another value.
        sequence: IdentityExhausted,
    },
    /// Replacement physical profile is invalid and no mutation occurred.
    InvalidProfile {
        /// Typed physical geometry validation failure.
        reason: PhysicalPageProfileError,
        /// Unchanged current revision identity.
        revision: RevisionIdentity,
        /// Existing physical page-profile identity that rejected replacement.
        target: AcceptedIdentity,
    },
    /// Session has no accepted semantic revision to edit.
    NoAcceptedRevision,
    /// Replacement equals current profile; no revision churn occurred.
    NoOp {
        /// Unchanged current revision identity.
        revision: RevisionIdentity,
        /// Existing physical page-profile identity whose value already
        /// matched.
        target: AcceptedIdentity,
    },
    /// Caller precondition names a revision that is no longer current.
    StaleBase {
        /// Current accepted revision identity that rejected the stale edit.
        current: RevisionIdentity,
    },
    /// Requested accepted identity is absent from the current revision.
    TargetNotFound {
        /// Unchanged current revision identity.
        revision: RevisionIdentity,
        /// Requested semantic identity absent from the current revision.
        target: AcceptedIdentity,
    },
    /// Requested accepted identity exists but is not a physical page profile.
    TargetNotPageProfile {
        /// Unchanged current revision identity.
        revision: RevisionIdentity,
        /// Existing non-profile semantic identity that rejected replacement.
        target: AcceptedIdentity,
    },
}

/// Result of one direct accepted semantic table-row role replacement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableRowRoleEditOutcome {
    /// Row role changed and one new accepted revision committed.
    Applied {
        /// Accepted revision used as the edit precondition.
        base: RevisionIdentity,
        /// New accepted revision produced by the edit.
        revision: RevisionIdentity,
        /// Existing semantic table-row identity whose role changed.
        target: AcceptedIdentity,
    },
    /// Revision identity allocation exhausted before commit.
    IdentityExhausted {
        /// Identity sequence that could not allocate another value.
        sequence: IdentityExhausted,
    },
    /// Session has no accepted semantic revision to edit.
    NoAcceptedRevision,
    /// Replacement equals current role; no revision churn occurred.
    NoOp {
        /// Unchanged current revision identity.
        revision: RevisionIdentity,
        /// Existing table-row identity whose role already matched.
        target: AcceptedIdentity,
    },
    /// Caller precondition names a revision that is no longer current.
    StaleBase {
        /// Current accepted revision identity that rejected the stale edit.
        current: RevisionIdentity,
    },
    /// Requested accepted identity is absent from the current revision.
    TargetNotFound {
        /// Unchanged current revision identity.
        revision: RevisionIdentity,
        /// Requested semantic identity absent from the current revision.
        target: AcceptedIdentity,
    },
    /// Requested accepted identity exists but is not a semantic table row.
    TargetNotTableRow {
        /// Unchanged current revision identity.
        revision: RevisionIdentity,
        /// Existing non-row semantic identity that rejected role replacement.
        target: AcceptedIdentity,
    },
}

/// Result of one direct accepted semantic text replacement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextEditOutcome {
    /// Text changed and one new accepted revision committed.
    Applied {
        /// Accepted revision used as the edit precondition.
        base: RevisionIdentity,
        /// New accepted revision produced by the edit.
        revision: RevisionIdentity,
        /// Existing semantic text identity whose content changed.
        target: AcceptedIdentity,
    },
    /// Revision identity allocation exhausted before commit.
    IdentityExhausted {
        /// Identity sequence that could not allocate another value.
        sequence: IdentityExhausted,
    },
    /// Session has no accepted semantic revision to edit.
    NoAcceptedRevision,
    /// Replacement equals current text; no revision churn occurred.
    NoOp {
        /// Unchanged current revision identity.
        revision: RevisionIdentity,
        /// Existing semantic text identity whose content already matched.
        target: AcceptedIdentity,
    },
    /// Caller precondition names a revision that is no longer current.
    StaleBase {
        /// Current accepted revision identity that rejected the stale edit.
        current: RevisionIdentity,
    },
    /// Requested accepted identity does not own editable inline text.
    TargetNotFound {
        /// Unchanged current revision identity.
        revision: RevisionIdentity,
        /// Requested semantic identity absent from the current revision.
        target: AcceptedIdentity,
    },
    /// Requested accepted identity exists but does not own editable inline
    /// text.
    TargetNotText {
        /// Unchanged current revision identity.
        revision: RevisionIdentity,
        /// Existing non-text semantic identity that rejected text replacement.
        target: AcceptedIdentity,
    },
}

/// Kind of one accepted semantic value with established direct-edit authority.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EditableSemanticValueKind {
    /// Structured mathematical source and presentation family.
    Formula,
    /// Complete exact physical page-profile geometry.
    PageProfile,
    /// Semantic table-row header/body role.
    TableRowRole,
    /// Exact accepted authored inline Unicode text.
    Text,
}

/// Accepted semantic value families with established direct-edit authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EditableSemanticValue {
    /// Structured mathematical source and presentation family.
    Formula {
        /// Semantic mathematical presentation family.
        mode: FormulaMode,
        /// Exact accepted authored source.
        source: String,
    },
    /// Complete exact physical page-profile geometry.
    PageProfile(PhysicalPageProfile),
    /// Semantic table-row header/body role.
    TableRowRole(TableRowRole),
    /// Exact accepted authored inline Unicode text.
    Text(String),
}

/// Net semantic effect classification for one direct-edit prediction.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DirectEditEffectClass {
    /// At least one accepted semantic value would differ from the base
    /// snapshot.
    Mutation,
    /// Final semantic state equals the accepted base for this prediction.
    NoOp,
}

/// Read-only semantic change-set preview for one established direct edit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectEditChangePreviewOutcome {
    /// Simulation succeeded as either one exact change or an empty no-op set.
    Predicted {
        /// Ordered direct semantic changes; empty means semantic no-op.
        changes: Vec<DirectEditSemanticChange>,
        /// Explicit net semantic mutation or no-op classification.
        effect: DirectEditEffectClass,
        /// Conservative seeds for later dependency-expanded impact
        /// calculation.
        impact_seeds: Vec<DirectEditImpactSeed>,
        /// Accepted revision whose immutable state was previewed.
        revision: RevisionIdentity,
    },
    /// Direct-edit simulation rejected before any change set could be
    /// predicted.
    Rejected {
        /// Existing typed simulation rejection, preserved without
        /// reinterpretation.
        outcome: Box<DirectEditSimulationOutcome>,
    },
}

/// Derived-authority family that may require recomputation after a direct edit.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DirectEditDerivedAuthority {
    /// Every derived authority rooted in the selected semantic scope.
    AllDerived,
    /// Acceptance or derived diagnostics depending on changed semantics.
    Diagnostics,
    /// Flow geometry downstream from semantic measurement or wrapping.
    FlowGeometry,
    /// Handwriting projection depending on changed authored semantics.
    Handwriting,
    /// Layout authority depending on changed structured semantics.
    Layout,
    /// Motion projection depending on changed semantic geometry or text.
    Motion,
    /// Output projections depending on changed structured semantics.
    Output,
    /// Rendering projection depending on changed semantics.
    Rendering,
    /// Text shaping depending on changed authored text.
    Shaping,
    /// Typed structure validation depending on structured content.
    StructureValidation,
    /// Text wrapping depending on changed authored text.
    Wrapping,
}

/// Semantic scope that seeds later dependency-expanded impact calculation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DirectEditImpactScope {
    /// One structured block plus its containing flow and page.
    BlockFlow {
        /// Nearest owning semantic block.
        block: AcceptedIdentity,
        /// Containing semantic flow.
        flow: AcceptedIdentity,
        /// Containing semantic page.
        page: AcceptedIdentity,
    },
    /// One semantic flow and its containing page.
    Flow {
        /// Containing semantic flow.
        flow: AcceptedIdentity,
        /// Containing semantic page.
        page: AcceptedIdentity,
    },
    /// Complete notebook fallback when a narrower safe scope is unavailable.
    Notebook {
        /// Accepted semantic notebook identity.
        notebook: AcceptedIdentity,
    },
    /// Exact accepted pages referencing one changed page profile.
    Pages {
        /// Referencing page identities in accepted semantic order.
        pages: Vec<AcceptedIdentity>,
    },
}

/// Conservative backend-owned seed for later dependency-expanded impact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectEditImpactSeed {
    /// Derived-authority families requiring dependency expansion from this
    /// seed.
    pub authorities: Vec<DirectEditDerivedAuthority>,
    /// Semantic region from which dependency expansion begins.
    pub scope: DirectEditImpactScope,
}

/// One command in a transport-neutral direct-edit batch proposal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectEditBatchCommand<CommandIdentity> {
    /// Explicit earlier commands this command semantically depends on.
    pub dependencies: Vec<CommandIdentity>,
    /// Caller-owned batch-local command identity.
    pub id: CommandIdentity,
    /// Complete local target preconditions for this command.
    pub preconditions: CommandTargetPreconditions,
    /// Exact requested replacement semantic value.
    pub requested: EditableSemanticValue,
    /// Existing accepted semantic identity targeted by this command.
    pub target: AcceptedIdentity,
}

/// Ordered in-memory direct-edit batch proposal before protocol normalization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectEditBatchProposal<CommandIdentity> {
    /// Accepted revision required by every command in this proposal.
    pub base: RevisionIdentity,
    /// Capability behavior version used to construct the proposal.
    pub capability_version: CommandBehaviorVersion,
    /// Ordered command sequence; order remains application-significant.
    pub commands: Vec<DirectEditBatchCommand<CommandIdentity>>,
}

/// Read-only exact coarse size for one in-memory direct-edit batch graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectEditBatchGraphSizeOutcome {
    /// Capability behavior changed before graph size derivation.
    CapabilityMismatch {
        /// Current backend-owned capability behavior version.
        current: CommandBehaviorVersion,
        /// Capability behavior version bound by the batch proposal.
        expected: CommandBehaviorVersion,
    },
    /// Session has no accepted semantic revision to inspect.
    NoAcceptedRevision,
    /// Dependency-edge counting exceeded addressable `usize` range.
    SizeOverflow,
    /// Exact command and dependency-edge counts were derived.
    Sized {
        /// Immutable accepted revision whose proposal base was checked.
        revision: RevisionIdentity,
        /// Exact complete in-memory graph size.
        size: CommandGraphSize,
    },
    /// Batch base revision is no longer the current accepted revision.
    StaleBase {
        /// Current accepted revision that rejected stale graph sizing.
        current: RevisionIdentity,
    },
}

/// Caller-bounded coarse resource preflight for one direct-edit batch graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectEditBatchGraphLimitsOutcome {
    /// Batch graph is within the caller-supplied coarse resource bounds.
    Admitted {
        /// Immutable accepted revision whose proposal base was checked.
        revision: RevisionIdentity,
        /// Exact graph size admitted by the supplied bounds.
        size: CommandGraphSize,
    },
    /// Capability behavior changed before graph resource preflight.
    CapabilityMismatch {
        /// Current backend-owned capability behavior version.
        current: CommandBehaviorVersion,
        /// Capability behavior version bound by the batch proposal.
        expected: CommandBehaviorVersion,
    },
    /// Session has no accepted semantic revision to inspect.
    NoAcceptedRevision,
    /// Exact coarse resource sizing rejected against the caller bounds.
    Rejected {
        /// Typed exact coarse resource failure; no graph was truncated.
        reason: CommandGraphLimitError,
    },
    /// Batch base revision is no longer the current accepted revision.
    StaleBase {
        /// Current accepted revision that rejected stale graph preflight.
        current: RevisionIdentity,
    },
}

/// Caller-bounded dependency report for one in-memory batch selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectEditBatchSelectionBoundedOutcome<CommandIdentity> {
    /// Capability behavior changed before bounded selection analysis.
    CapabilityMismatch {
        /// Current backend-owned capability behavior version.
        current: CommandBehaviorVersion,
        /// Capability behavior version bound by the batch proposal.
        expected: CommandBehaviorVersion,
    },
    /// Complete command dependency structure rejected before analysis.
    DependencyGraphRejected {
        /// Typed transport-neutral dependency graph failure.
        reason: CommandGraphError<CommandIdentity>,
    },
    /// Session has no accepted semantic revision to analyze.
    NoAcceptedRevision,
    /// Exact omitted dependency-edge count exceeds the caller bound.
    RequirementCountExceeded {
        /// Exact omitted dependency-edge count.
        actual: usize,
        /// Maximum missing dependency edges admitted for materialization.
        limit: usize,
    },
    /// Exact omitted-dependency edge counting exceeded addressable range.
    RequirementCountOverflow,
    /// Selection is structurally known and its bounded report was materialized.
    Requirements {
        /// Explicit dependency edges absent from the original selection.
        missing: Vec<MissingDependencyRequirement<CommandIdentity>>,
        /// Immutable accepted revision whose proposal base was checked.
        revision: RevisionIdentity,
    },
    /// Batch base revision is no longer the current accepted revision.
    StaleBase {
        /// Current accepted revision that rejected stale selection analysis.
        current: RevisionIdentity,
    },
    /// Selection names no command in the batch proposal.
    UnknownSelection {
        /// Unknown caller-owned command identity.
        command: CommandIdentity,
    },
}

/// Read-only dependency analysis for one in-memory direct-edit batch selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectEditBatchSelectionRequirementsOutcome<CommandIdentity> {
    /// Capability behavior changed before selection analysis.
    CapabilityMismatch {
        /// Current backend-owned capability behavior version.
        current: CommandBehaviorVersion,
        /// Capability behavior version bound by the batch proposal.
        expected: CommandBehaviorVersion,
    },
    /// Complete command dependency structure rejected before selection
    /// analysis.
    DependencyGraphRejected {
        /// Typed transport-neutral dependency graph failure.
        reason: CommandGraphError<CommandIdentity>,
    },
    /// Session has no accepted semantic revision to analyze.
    NoAcceptedRevision,
    /// Selection is structurally known and its omitted requirements are
    /// reported.
    Requirements {
        /// Explicit dependency edges absent from the original selection.
        missing: Vec<MissingDependencyRequirement<CommandIdentity>>,
        /// Immutable accepted revision whose proposal base was checked.
        revision: RevisionIdentity,
    },
    /// Batch base revision is no longer the current accepted revision.
    StaleBase {
        /// Current accepted revision that rejected stale selection analysis.
        current: RevisionIdentity,
    },
    /// Selection names no command in the batch proposal.
    UnknownSelection {
        /// Unknown caller-owned command identity.
        command: CommandIdentity,
    },
}

/// Read-only dependency-closure size for one in-memory batch selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectEditBatchSelectionSummaryOutcome<CommandIdentity> {
    /// Capability behavior changed before selection summary derivation.
    CapabilityMismatch {
        /// Current backend-owned capability behavior version.
        current: CommandBehaviorVersion,
        /// Capability behavior version bound by the batch proposal.
        expected: CommandBehaviorVersion,
    },
    /// Complete command dependency structure rejected before summary
    /// derivation.
    DependencyGraphRejected {
        /// Typed transport-neutral dependency graph failure.
        reason: CommandGraphError<CommandIdentity>,
    },
    /// Session has no accepted semantic revision to analyze.
    NoAcceptedRevision,
    /// Exact omitted-dependency edge counting exceeded addressable range.
    RequirementCountOverflow,
    /// Batch base revision is no longer the current accepted revision.
    StaleBase {
        /// Current accepted revision that rejected stale selection summary.
        current: RevisionIdentity,
    },
    /// Selection is known and its transitive closure size was derived.
    Summarized {
        /// Immutable accepted revision whose proposal base was checked.
        revision: RevisionIdentity,
        /// Exact coarse dependency-closure size without command-ID pairs.
        summary: DependencySelectionSummary,
    },
    /// Selection names no command in the batch proposal.
    UnknownSelection {
        /// Unknown caller-owned command identity.
        command: CommandIdentity,
    },
}

/// One successfully simulated direct-edit batch command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectEditBatchCommandPrediction<CommandIdentity> {
    /// Exact semantic change, absent only for a valid semantic no-op.
    pub change: Option<DirectEditSemanticChange>,
    /// Caller-owned command identity.
    pub command: CommandIdentity,
    /// Executable direct-edit family simulated for this command.
    pub family: SemanticCommandFamily,
    /// Existing accepted semantic target simulated by this command.
    pub target: AcceptedIdentity,
}

/// Typed semantic failure for one command in an otherwise admitted batch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectEditBatchCommandRejection<CommandIdentity> {
    /// Repeated target lacks an explicit dependency on its previous writer.
    MissingPriorTargetDependency {
        /// Previous command that most recently simulated this target.
        dependency: CommandIdentity,
        /// Repeated accepted semantic target.
        target: AcceptedIdentity,
    },
    /// Local command preconditions failed against isolated candidate state.
    Precondition {
        /// Existing typed local-precondition failure.
        outcome: Box<CommandTargetPreconditionOutcome>,
    },
    /// Replacement simulation rejected through an existing typed result.
    Simulation {
        /// Existing direct-edit simulation rejection.
        outcome: Box<DirectEditSimulationOutcome>,
    },
}

/// Read-only ordered direct-edit batch simulation result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectEditBatchSimulationOutcome<CommandIdentity> {
    /// Capability behavior changed before batch simulation.
    CapabilityMismatch {
        /// Current backend-owned capability behavior version.
        current: CommandBehaviorVersion,
        /// Capability behavior version bound by the batch.
        expected: CommandBehaviorVersion,
    },
    /// Command dependency structure rejected before semantic simulation.
    DependencyGraphRejected {
        /// Typed transport-neutral dependency graph failure.
        reason: CommandGraphError<CommandIdentity>,
    },
    /// Session has no accepted semantic revision to simulate.
    NoAcceptedRevision,
    /// Every command simulated successfully against isolated candidate state.
    Predicted {
        /// Exact ordered semantic change set; no-op commands add no entry.
        changes: Vec<DirectEditSemanticChange>,
        /// Ordered per-command simulation evidence.
        commands: Vec<DirectEditBatchCommandPrediction<CommandIdentity>>,
        /// Explicit net semantic mutation or no-op classification.
        effect: DirectEditEffectClass,
        /// Conservative seeds for later dependency-expanded impact
        /// calculation.
        impact_seeds: Vec<DirectEditImpactSeed>,
        /// Immutable accepted base revision used for the simulation.
        revision: RevisionIdentity,
    },
    /// One required command rejected and later commands were not evaluated.
    Rejected {
        /// Command whose semantic validation rejected the batch.
        command: CommandIdentity,
        /// Successfully simulated earlier commands, none committed.
        evaluated: Vec<DirectEditBatchCommandPrediction<CommandIdentity>>,
        /// Later command identities deliberately not evaluated after failure.
        not_evaluated: Vec<CommandIdentity>,
        /// Typed reason the decisive command rejected.
        reason: Box<DirectEditBatchCommandRejection<CommandIdentity>>,
        /// Immutable accepted base revision used for isolated simulation.
        revision: RevisionIdentity,
    },
    /// Caller-supplied coarse resource limits rejected before graph simulation.
    ResourceRejected {
        /// Typed exact command/dependency limit failure; nothing was
        /// truncated.
        reason: CommandGraphLimitError,
    },
    /// Batch base revision is no longer the current accepted revision.
    StaleBase {
        /// Current accepted revision that rejected the stale batch.
        current: RevisionIdentity,
    },
}

/// One version-bound single-target direct-edit proposal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectEditProposal {
    /// Capability behavior version used when the proposal was constructed.
    pub capability_version: CommandBehaviorVersion,
    /// Complete local target preconditions supplied by the caller.
    pub preconditions: CommandTargetPreconditions,
    /// Exact requested replacement semantic value.
    pub requested: EditableSemanticValue,
    /// Accepted revision the proposal requires as its base.
    pub revision: RevisionIdentity,
    /// Existing semantic identity the proposal targets.
    pub target: AcceptedIdentity,
}

/// Read-only result of checking and simulating one direct-edit proposal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectEditProposalOutcome {
    /// Bound capability behavior no longer matches the backend.
    CapabilityMismatch {
        /// Current backend-owned capability behavior version.
        current: CommandBehaviorVersion,
        /// Capability behavior version bound by the proposal.
        expected: CommandBehaviorVersion,
    },
    /// One local target precondition rejected before replacement simulation.
    PreconditionRejected {
        /// Typed local precondition result that rejected the proposal.
        outcome: CommandTargetPreconditionOutcome,
    },
    /// Capability and local preconditions passed; replacement was simulated.
    Simulated {
        /// Read-only replacement simulation result.
        outcome: DirectEditSimulationOutcome,
    },
}

/// Exact semantic before/after change predicted for one direct edit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectEditSemanticChange {
    /// Exact requested semantic value after the predicted edit.
    pub after: EditableSemanticValue,
    /// Exact accepted semantic value before the predicted edit.
    pub before: EditableSemanticValue,
    /// Executable semantic command family owning this change.
    pub family: SemanticCommandFamily,
    /// Stable accepted semantic identity whose revision-owned value changes.
    pub target: AcceptedIdentity,
}

/// Read-only semantic simulation result for one established direct edit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectEditSimulationOutcome {
    /// Replacement is valid and would change accepted semantic state.
    Applicable {
        /// Executable direct-edit family implied by the replacement value.
        family: SemanticCommandFamily,
        /// Exact requested replacement value.
        requested: EditableSemanticValue,
        /// Accepted revision simulated without mutation.
        revision: RevisionIdentity,
        /// Existing semantic target simulated without mutation.
        target: AcceptedIdentity,
    },
    /// Requested mathematical replacement is structurally malformed.
    InvalidMathematics {
        /// Typed mathematical source failure.
        reason: MathSyntaxError,
        /// Accepted revision simulated without mutation.
        revision: RevisionIdentity,
        /// Existing semantic target simulated without mutation.
        target: AcceptedIdentity,
    },
    /// Requested physical page profile is invalid.
    InvalidPageProfile {
        /// Typed physical page-profile failure.
        reason: PhysicalPageProfileError,
        /// Accepted revision simulated without mutation.
        revision: RevisionIdentity,
        /// Existing semantic target simulated without mutation.
        target: AcceptedIdentity,
    },
    /// Session has no accepted semantic revision to simulate.
    NoAcceptedRevision,
    /// Replacement is valid but already equals the accepted semantic value.
    NoOp {
        /// Executable direct-edit family implied by the replacement value.
        family: SemanticCommandFamily,
        /// Accepted revision simulated without mutation.
        revision: RevisionIdentity,
        /// Existing semantic target simulated without mutation.
        target: AcceptedIdentity,
    },
    /// Caller names an accepted revision that is no longer current.
    StaleBase {
        /// Current accepted revision that rejected stale simulation.
        current: RevisionIdentity,
    },
    /// Target exists but has no established direct-edit value projection.
    TargetNotEditableValue {
        /// Semantic kind owned by the existing target.
        kind: SemanticIdentityKind,
        /// Accepted revision simulated without mutation.
        revision: RevisionIdentity,
        /// Existing target with no direct-edit value projection.
        target: AcceptedIdentity,
    },
    /// Requested accepted identity is absent from the named revision.
    TargetNotFound {
        /// Accepted revision simulated without mutation.
        revision: RevisionIdentity,
        /// Requested identity absent from that revision.
        target: AcceptedIdentity,
    },
    /// Requested mathematical replacement uses unsupported source constructs.
    UnsupportedMathematics {
        /// Accepted revision simulated without mutation.
        revision: RevisionIdentity,
        /// Existing semantic target simulated without mutation.
        target: AcceptedIdentity,
    },
    /// Replacement value family does not match the existing editable target.
    ValueFamilyMismatch {
        /// Current editable semantic value family.
        actual: EditableSemanticValueKind,
        /// Requested replacement semantic value family.
        requested: EditableSemanticValueKind,
        /// Accepted revision simulated without mutation.
        revision: RevisionIdentity,
        /// Existing semantic target simulated without mutation.
        target: AcceptedIdentity,
    },
}

/// Result of comparing one established editable value against an exact base.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EditableValuePreconditionOutcome {
    /// Session has no accepted semantic revision to check.
    NoAcceptedRevision,
    /// Current semantic value exactly matches the expected base value.
    Satisfied {
        /// Current accepted semantic value.
        actual: EditableSemanticValue,
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Existing target satisfying the value precondition.
        target: AcceptedIdentity,
    },
    /// Caller names an accepted revision that is no longer current.
    StaleBase {
        /// Current accepted revision that rejected the stale check.
        current: RevisionIdentity,
    },
    /// Target exists but has no established direct-edit value projection.
    TargetNotEditableValue {
        /// Semantic kind owned by the existing target.
        kind: SemanticIdentityKind,
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Existing target with no admitted editable-value projection.
        target: AcceptedIdentity,
    },
    /// Requested target is absent from the named accepted revision.
    TargetNotFound {
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Requested identity absent from that revision.
        target: AcceptedIdentity,
    },
    /// Current semantic value differs from the expected base value.
    ValueMismatch {
        /// Current accepted semantic value.
        actual: EditableSemanticValue,
        /// Semantic base value required by the precondition.
        expected: EditableSemanticValue,
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Existing target that failed the value precondition.
        target: AcceptedIdentity,
    },
}

/// Behavior version for one backend-owned command capability contract.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CommandBehaviorVersion(pub u32);

/// Application operations discoverable through semantic command capabilities.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CommandApplicationCapability {
    /// Atomically apply one validated semantic command batch.
    Apply,
    /// Generate one backend-owned bounded semantic command context.
    CommandContext,
    /// Rebatch a validated subset through an explicitly admitted workflow.
    SelectiveRebatching,
    /// Validate one semantic command batch without accepted mutation.
    Validate,
}

/// Result of checking one previously bound command capability behavior version.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandCapabilityCompatibilityOutcome {
    /// The expected capability behavior is still current.
    Compatible {
        /// Current capability discovery snapshot.
        snapshot: SemanticCommandCapabilitySnapshot,
    },
    /// Capability behavior changed and the caller must refresh its context.
    Mismatch {
        /// Current backend-owned capability behavior version.
        current: CommandBehaviorVersion,
        /// Older or otherwise incompatible behavior version supplied by
        /// caller.
        expected: CommandBehaviorVersion,
    },
}

/// One semantic command family whose behavior is discoverable globally.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandFamilyCapability {
    /// Version of the family behavior contract.
    pub behavior_version: CommandBehaviorVersion,
    /// Semantic mutation family with at least one executable direct target.
    pub family: SemanticCommandFamily,
}

/// Command-mode limits published when their owning capability is admitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandResourceLimits {
    /// Maximum commands accepted in one normalized batch, when admitted.
    pub commands_per_batch: Option<usize>,
    /// Maximum explicit dependency edges per normalized batch, when admitted.
    pub dependency_edges: Option<usize>,
    /// Maximum parsed command envelope size in bytes, when admitted.
    pub envelope_bytes: Option<usize>,
    /// Maximum backend-generated readable context size, when admitted.
    pub readable_context_bytes: Option<usize>,
    /// Maximum writable semantic targets or anchors, when admitted.
    pub writable_targets: Option<usize>,
}

/// Read-only versioned semantic command capability discovery snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticCommandCapabilitySnapshot {
    /// Application-level command operations currently admitted.
    pub admitted_applications: &'static [CommandApplicationCapability],
    /// Version of this capability-discovery behavior.
    pub behavior_version: CommandBehaviorVersion,
    /// Families with at least one currently executable direct-edit target.
    pub family_capabilities: &'static [CommandFamilyCapability],
    /// Versioned normalization behavior, absent until a protocol is admitted.
    pub normalization_version: Option<CommandBehaviorVersion>,
    /// Admitted serialized command protocol behavior versions.
    pub protocol_versions: &'static [CommandBehaviorVersion],
    /// Command-mode resource limits bound to admitted protocol operations.
    pub resource_limits: CommandResourceLimits,
    /// Version of typed local command-target result behavior.
    pub typed_result_version: CommandBehaviorVersion,
}

/// Semantic mutation families frozen by the command capability matrix.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SemanticCommandFamily {
    /// Attach, replace, or remove a reference to an already admitted asset.
    AssetReference,
    /// Insert or delete typed semantic block membership at admitted anchors.
    BlockInsertionAndDeletion,
    /// Change a notebook-wide or output-relevant accepted constraint.
    DocumentConstraint,
    /// Change semantic order or admitted grouping relationships.
    OrderingAndGrouping,
    /// Attach, correct, or remove semantic source and citation metadata.
    Provenance,
    /// Change typed placement, size, crop, anchor, alignment, or layer intent.
    SpatialConstraint,
    /// Change typed children while preserving their structured container.
    StructuredContent,
    /// Change admitted semantic style assignment or role.
    StyleRole,
    /// Change text owned by an existing semantic text identity.
    TextContent,
}

/// Result of checking one requested command family against one target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandFamilyAdmissionOutcome {
    /// Requested family is currently executable for this exact target.
    Admitted {
        /// Complete local precondition material for the admitted target.
        material: CommandTargetMaterial,
    },
    /// Requested family is not currently executable for this exact target.
    FamilyNotExecutable {
        /// Currently executable direct-edit family, if one exists.
        available: Option<SemanticCommandFamily>,
        /// Requested semantic command family.
        requested: SemanticCommandFamily,
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Existing target that rejected family admission.
        target: AcceptedIdentity,
    },
    /// Session has no accepted semantic revision to check.
    NoAcceptedRevision,
    /// Caller names an accepted revision that is no longer current.
    StaleBase {
        /// Current accepted revision that rejected stale admission.
        current: RevisionIdentity,
    },
    /// Requested accepted identity is absent from the named revision.
    TargetNotFound {
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Requested identity absent from that revision.
        target: AcceptedIdentity,
    },
}

/// Complete local checks supplied for one semantic command target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandTargetPreconditions {
    /// Expected current editable base value when compare-and-set intent
    /// applies.
    pub expected_value: Option<EditableSemanticValue>,
    /// Expected semantic kind and direct structural owner.
    pub identity: IdentityPrecondition,
    /// Semantic command family requested for this exact target.
    pub requested_family: SemanticCommandFamily,
}

/// Result of checking all local preconditions for one semantic command target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandTargetPreconditionOutcome {
    /// Requested family is not currently executable for this exact target.
    FamilyNotExecutable {
        /// Currently executable direct-edit family, if one exists.
        available: Option<SemanticCommandFamily>,
        /// Requested semantic command family.
        requested: SemanticCommandFamily,
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Existing target that rejected family admission.
        target: AcceptedIdentity,
    },
    /// Existing target has a semantic kind different from the requirement.
    KindMismatch {
        /// Semantic kind currently owned by the target identity.
        actual: SemanticIdentityKind,
        /// Semantic kind required by the local precondition.
        expected: SemanticIdentityKind,
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Existing target that failed the kind precondition.
        target: AcceptedIdentity,
    },
    /// Session has no accepted semantic revision to check.
    NoAcceptedRevision,
    /// Existing target has a direct owner different from the requirement.
    OwnerMismatch {
        /// Current direct structural owner; `None` means notebook root.
        actual: Option<AcceptedIdentity>,
        /// Direct structural owner required by the local precondition.
        expected: IdentityOwnerExpectation,
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Existing target that failed the owner precondition.
        target: AcceptedIdentity,
    },
    /// Every declared local target precondition is satisfied.
    Satisfied {
        /// Complete backend-derived target material that was checked.
        material: CommandTargetMaterial,
    },
    /// Caller names an accepted revision that is no longer current.
    StaleBase {
        /// Current accepted revision that rejected stale validation.
        current: RevisionIdentity,
    },
    /// Target exists but has no established direct-edit value projection.
    TargetNotEditableValue {
        /// Semantic kind owned by the existing target.
        kind: SemanticIdentityKind,
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Existing target without an editable value projection.
        target: AcceptedIdentity,
    },
    /// Requested accepted identity is absent from the named revision.
    TargetNotFound {
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Requested identity absent from that revision.
        target: AcceptedIdentity,
    },
    /// Current semantic value differs from the expected base value.
    ValueMismatch {
        /// Current accepted semantic value.
        actual: EditableSemanticValue,
        /// Semantic base value required by the precondition.
        expected: EditableSemanticValue,
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Existing target that failed the value precondition.
        target: AcceptedIdentity,
    },
}

/// Backend-derived local precondition material for one writable target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandTargetMaterial {
    /// Current semantic kind and direct structural owner.
    pub descriptor: SemanticIdentityDescriptor<AcceptedIdentity>,
    /// Currently executable direct-edit family for this target, if any.
    pub direct_edit_family: Option<SemanticCommandFamily>,
    /// Exact established editable value when this target has direct-edit
    /// authority.
    pub editable_value: Option<EditableSemanticValue>,
    /// Accepted revision that owns this material.
    pub revision: RevisionIdentity,
    /// Accepted semantic identity this material describes.
    pub target: AcceptedIdentity,
}

/// Result of deriving one writable target's local precondition material.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandTargetMaterialOutcome {
    /// Session has no accepted semantic revision to inspect.
    NoAcceptedRevision,
    /// Target material was derived completely from the named revision.
    Prepared {
        /// Complete local material for the requested target.
        material: CommandTargetMaterial,
    },
    /// Caller names an accepted revision that is no longer current.
    StaleBase {
        /// Current accepted revision that rejected stale derivation.
        current: RevisionIdentity,
    },
    /// Requested accepted identity is absent from the named revision.
    TargetNotFound {
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Requested identity absent from that revision.
        target: AcceptedIdentity,
    },
}

/// Expected direct owner for one local semantic identity precondition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityOwnerExpectation {
    /// Do not constrain the target's direct structural owner.
    Any,
    /// Require one exact direct structural owner identity.
    Direct(AcceptedIdentity),
    /// Require the notebook root, which has no direct structural owner.
    Root,
}

/// Local semantic identity precondition checked against one accepted revision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityPrecondition {
    /// Optional exact semantic-kind requirement.
    pub expected_kind: Option<SemanticIdentityKind>,
    /// Direct structural-owner requirement.
    pub expected_owner: IdentityOwnerExpectation,
}

/// Result of checking one local semantic identity precondition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityPreconditionOutcome {
    /// Existing target has a semantic kind different from the requirement.
    KindMismatch {
        /// Semantic kind currently owned by the target identity.
        actual: SemanticIdentityKind,
        /// Semantic kind required by the local precondition.
        expected: SemanticIdentityKind,
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Existing semantic target that failed the kind precondition.
        target: AcceptedIdentity,
    },
    /// Session has no accepted semantic revision to check.
    NoAcceptedRevision,
    /// Existing target has a direct owner different from the requirement.
    OwnerMismatch {
        /// Current direct structural owner; `None` means notebook root.
        actual: Option<AcceptedIdentity>,
        /// Direct structural owner required by the local precondition.
        expected: IdentityOwnerExpectation,
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Existing semantic target that failed the owner precondition.
        target: AcceptedIdentity,
    },
    /// Target exists and every declared local precondition matches.
    Satisfied {
        /// Current semantic kind and direct structural owner.
        descriptor: SemanticIdentityDescriptor<AcceptedIdentity>,
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Existing semantic target satisfying the precondition.
        target: AcceptedIdentity,
    },
    /// Caller names an accepted revision that is no longer current.
    StaleBase {
        /// Current accepted revision that rejected the stale check.
        current: RevisionIdentity,
    },
    /// Requested target is absent from the named accepted revision.
    TargetNotFound {
        /// Accepted revision checked without mutation.
        revision: RevisionIdentity,
        /// Requested identity absent from that revision.
        target: AcceptedIdentity,
    },
}

/// Result of one exact-revision, single-identity semantic inspection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityInspectOutcome {
    /// Requested identity exists in the named accepted revision.
    Inspected {
        /// Semantic kind and direct structural owner for the requested
        /// identity.
        descriptor: SemanticIdentityDescriptor<AcceptedIdentity>,
        /// Accepted revision that was inspected without mutation.
        revision: RevisionIdentity,
        /// Requested accepted semantic identity.
        target: AcceptedIdentity,
    },
    /// Session has no accepted semantic revision to inspect.
    NoAcceptedRevision,
    /// Caller names an accepted revision that is no longer current.
    StaleBase {
        /// Current accepted revision that rejected stale inspection.
        current: RevisionIdentity,
    },
    /// Requested accepted identity is absent from the named revision.
    TargetNotFound {
        /// Accepted revision that was inspected without mutation.
        revision: RevisionIdentity,
        /// Requested identity absent from that revision.
        target: AcceptedIdentity,
    },
}

/// Result of one exact-revision, single-identity semantic kind inspection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityKindInspectOutcome {
    /// Requested identity exists in the named accepted revision.
    Inspected {
        /// Semantic kind owned by the requested identity.
        kind: SemanticIdentityKind,
        /// Accepted revision that was inspected without mutation.
        revision: RevisionIdentity,
        /// Requested accepted semantic identity.
        target: AcceptedIdentity,
    },
    /// Session has no accepted semantic revision to inspect.
    NoAcceptedRevision,
    /// Caller names an accepted revision that is no longer current.
    StaleBase {
        /// Current accepted revision that rejected stale inspection.
        current: RevisionIdentity,
    },
    /// Requested accepted identity is absent from the named revision.
    TargetNotFound {
        /// Accepted revision that was inspected without mutation.
        revision: RevisionIdentity,
        /// Requested identity absent from that revision.
        target: AcceptedIdentity,
    },
}

/// One candidate-local identity promoted to a new accepted semantic identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityMapping {
    /// Newly allocated accepted semantic identity.
    pub accepted: AcceptedIdentity,
    /// Candidate-local identity consumed by this acceptance.
    pub candidate: CandidateIdentity,
}

/// Application authority for current accepted semantic notebook state.
pub trait SemanticNotebookSession {
    /// Accept one complete candidate as one atomic semantic transaction.
    fn accept(
        &mut self,
        candidate: Notebook<CandidateIdentity>,
    ) -> AcceptanceOutcome;

    /// Check one previously bound command capability behavior version.
    fn check_command_capability_compatibility(
        &self,
        expected: CommandBehaviorVersion,
    ) -> CommandCapabilityCompatibilityOutcome;

    /// Check whether one semantic command family is executable for a target.
    fn check_command_family_admission(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        requested: SemanticCommandFamily,
    ) -> CommandFamilyAdmissionOutcome;

    /// Check family, identity, owner, and optional base value in one snapshot.
    fn check_command_target_preconditions(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        preconditions: CommandTargetPreconditions,
    ) -> CommandTargetPreconditionOutcome;

    /// Compare one established editable semantic value against an exact base.
    fn check_editable_value_precondition(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        expected: EditableSemanticValue,
    ) -> EditableValuePreconditionOutcome;

    /// Check local semantic kind and owner preconditions read-only.
    fn check_identity_precondition(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        precondition: IdentityPrecondition,
    ) -> IdentityPreconditionOutcome;

    /// Discover current semantic command behavior without mutation.
    fn command_capability_snapshot(&self) -> SemanticCommandCapabilitySnapshot;

    /// Derive complete local command-precondition material for one target.
    fn command_target_material(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
    ) -> CommandTargetMaterialOutcome;

    /// Read the current accepted revision without creating another revision.
    fn current(&self) -> Option<&AcceptedRevision>;

    /// Enforce caller-owned coarse graph resource limits read-only.
    fn direct_edit_batch_graph_limits<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
        limits: CommandGraphLimits,
    ) -> DirectEditBatchGraphLimitsOutcome;

    /// Derive exact coarse command and dependency-edge counts read-only.
    fn direct_edit_batch_graph_size<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
    ) -> DirectEditBatchGraphSizeOutcome;

    /// Analyze omitted dependencies for one in-memory batch selection.
    fn direct_edit_batch_selection_requirements<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
        selected: &BTreeSet<CommandIdentity>,
    ) -> DirectEditBatchSelectionRequirementsOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord;

    /// Materialize omitted dependencies up to a caller-supplied edge bound.
    fn direct_edit_batch_selection_requirements_bounded<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
        selected: &BTreeSet<CommandIdentity>,
        maximum_missing_edges: usize,
    ) -> DirectEditBatchSelectionBoundedOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord;

    /// Summarize one in-memory batch selection without materializing edges.
    fn direct_edit_batch_selection_summary<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
        selected: &BTreeSet<CommandIdentity>,
    ) -> DirectEditBatchSelectionSummaryOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord;

    /// Inspect one semantic identity against an exact accepted revision.
    fn inspect_identity(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
    ) -> IdentityInspectOutcome;

    /// Inspect one semantic identity kind against an exact accepted revision.
    fn inspect_identity_kind(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
    ) -> IdentityKindInspectOutcome;

    /// Preview the exact direct semantic change set without mutation.
    fn preview_direct_edit_changes(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        requested: EditableSemanticValue,
    ) -> DirectEditChangePreviewOutcome;

    /// Replace one mathematical source against an exact base revision.
    fn replace_formula(
        &mut self,
        base: RevisionIdentity,
        target: AcceptedIdentity,
        mode: FormulaMode,
        source: String,
    ) -> FormulaEditOutcome;

    /// Replace one physical page profile against an exact base revision.
    fn replace_page_profile(
        &mut self,
        base: RevisionIdentity,
        target: AcceptedIdentity,
        geometry: PhysicalPageProfile,
    ) -> PageProfileEditOutcome;

    /// Replace one semantic table-row role against an exact base revision.
    fn replace_table_row_role(
        &mut self,
        base: RevisionIdentity,
        target: AcceptedIdentity,
        role: TableRowRole,
    ) -> TableRowRoleEditOutcome;

    /// Replace one existing inline text identity against an exact base
    /// revision.
    fn replace_text(
        &mut self,
        base: RevisionIdentity,
        target: AcceptedIdentity,
        value: String,
    ) -> TextEditOutcome;

    /// Simulate one established direct edit without accepted mutation.
    fn simulate_direct_edit(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        requested: EditableSemanticValue,
    ) -> DirectEditSimulationOutcome;

    /// Simulate one ordered direct-edit batch in isolated candidate state.
    fn simulate_direct_edit_batch<CommandIdentity>(
        &self,
        batch: DirectEditBatchProposal<CommandIdentity>,
    ) -> DirectEditBatchSimulationOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord;

    /// Enforce caller graph limits before ordered direct-edit simulation.
    fn simulate_direct_edit_batch_bounded<CommandIdentity>(
        &self,
        batch: DirectEditBatchProposal<CommandIdentity>,
        limits: CommandGraphLimits,
    ) -> DirectEditBatchSimulationOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord;

    /// Check capability and local preconditions, then simulate one proposal.
    fn simulate_direct_edit_proposal(
        &self,
        proposal: DirectEditProposal,
    ) -> DirectEditProposalOutcome;
}
