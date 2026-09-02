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
}
