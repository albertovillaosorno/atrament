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
//   - Transport-independent semantic diagnostic envelope and version identity.
// - Must-Not:
//   - Choose HTTP status, JSON field names, UI prose, or application results.
// - Allows:
//   - Inputs: Backend-owned condition, operation, location, and evidence
//     values.
//   - Outputs: Stable typed diagnostic values shared by application
//     capabilities.
//   - Side effects: None.
// - Split-When:
//   - One evidence family becomes an independently versioned domain authority.
// - Merge-When:
//   - Diagnostics stop being shared across application capability boundaries.
// - Summary:
//   - Defines one versioned diagnostic value model for the Atrament backend.
// - Description:
//   - Keeps semantic diagnostic meaning independent from adapter presentation.
// - Usage:
//   - Construct diagnostics in application services and project them in
//     adapters.
// - Defaults:
//   - Diagnostics are complete unless an owning operation says otherwise.
//

//! Shared semantic diagnostic envelope for Atrament application capabilities.

/// First-release semantic diagnostic namespace identity.
pub const DIAGNOSTIC_VERSION: &str = "atrament.diagnostic/1";

/// Capability-specific blocking disposition for one diagnostic.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BlockingDisposition {
    /// The condition does not block the operation capability that produced it.
    Advisory,
    /// The condition blocks the operation capability that produced it.
    Blocking,
}

/// Whether the represented diagnostic set is complete for its operation result.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Completeness {
    /// All required diagnostics for the operation result are represented.
    Complete,
    /// Diagnostic detail is explicitly incomplete under an admitted bound.
    Incomplete,
}

/// One transport-independent diagnostic attached to an application result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// Stable condition identity.
    pub code: DiagnosticCode,
    /// Capability-specific blocking disposition.
    pub disposition: BlockingDisposition,
    /// Typed evidence used to explain the condition.
    pub evidence: Vec<Evidence>,
    /// Ordered semantic owners needed to locate the condition.
    pub locations: Vec<SemanticLocation>,
    /// Application operation and authoritative context for this evidence.
    pub operation: OperationBinding,
    /// Backend-owned admissible next-step categories.
    pub remediations: Vec<Remediation>,
    /// Human-attention severity.
    pub severity: Severity,
}

/// Stable backend-owned diagnostic condition identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DiagnosticCode {
    /// A browser/backend required-version identity does not match.
    HandshakeVersionMismatch,
    /// One complete session draft field exceeds its admitted byte limit.
    SessionDraftResourceLimit,
}

/// Diagnostics returned with one application result plus set completeness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticSet {
    /// Explicit completeness of the complete operation diagnostic set.
    pub completeness: Completeness,
    /// Ordered diagnostics admitted for the operation result.
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticCode {
    /// Return the stable semantic code within the diagnostic namespace.
    #[must_use]
    pub const fn stable_name(self) -> &'static str {
        match self {
            Self::HandshakeVersionMismatch => {
                "atrament.handshake.version-mismatch"
            },
            Self::SessionDraftResourceLimit => {
                "atrament.session-draft.resource-limit"
            },
        }
    }
}

/// Structured evidence that explains a diagnostic without relying on prose.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Evidence {
    /// Numeric value exceeded one backend-owned admitted maximum.
    LimitExceeded {
        /// Maximum admitted value.
        maximum: u64,
        /// Complete observed value that was rejected.
        observed: u64,
        /// Unit shared by maximum and observed values.
        unit: EvidenceUnit,
    },
    /// Exact physical length represented in canonical micrometres.
    PhysicalLength {
        /// Signed canonical physical length.
        micrometres: i64,
        /// Semantic measurement represented by the length.
        quantity: PhysicalLengthQuantity,
    },
    /// Backend-required version identity for one compatibility dimension.
    RequiredVersion {
        /// Stable compatibility dimension name.
        dimension: &'static str,
        /// Exact backend-required version identity.
        expected: &'static str,
    },
}

/// Unit carried by typed numeric diagnostic evidence.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EvidenceUnit {
    /// UTF-8 or transport byte count.
    Bytes,
}

/// Semantic owner category used to locate a diagnostic without storage paths.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LocationKind {
    /// One capability region whose compatibility or support is being evaluated.
    Capability,
    /// One semantic command identity or candidate-local command handle.
    Command,
    /// One semantic or application-owned field.
    Field,
    /// One constraint or collision geometry owner.
    Geometry,
    /// One glyph or line owner derived from a semantic text identity.
    Glyph,
    /// One accepted semantic object identity.
    Object,
    /// One page, flow, formula, figure, table cell, or source identity.
    Structure,
}

/// Role of one location within a potentially relational diagnostic.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LocationRole {
    /// Primary owner whose condition is being reported.
    Primary,
    /// Related owner required to understand a relational condition.
    Related,
}

/// Application capability whose operation produced a diagnostic.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Operation {
    /// Caller-authorized persistent export.
    Export,
    /// Accepted revision history traversal.
    HistoryTraversal,
    /// Bounded semantic inspection or context generation.
    Inspect,
    /// Device-neutral live plan compilation.
    Plan,
    /// Deterministic preview or output rendering.
    Render,
    /// Atomic semantic command application.
    SemanticApply,
    /// Semantic command validation without commit.
    SemanticValidate,
    /// Replacement of one pre-acceptance session draft field.
    SessionDraftReplace,
    /// Browser/backend compatibility handshake.
    SessionHandshake,
}

/// Operation plus the authoritative identities required to interpret evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationBinding {
    /// Typed authoritative operation contexts, excluding ambient request IDs.
    pub contexts: Vec<OperationContext>,
    /// Application capability operation.
    pub operation: Operation,
}

/// One authoritative identity bound to the operation producing a diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationContext {
    /// Stable application-owned identity, not an adapter request ID.
    pub identity: String,
    /// Typed role of this identity in the operation context.
    pub kind: OperationContextKind,
}

/// Kind of authoritative operation context carried by a diagnostic.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OperationContextKind {
    /// Accepted notebook revision consumed by the operation.
    AcceptedRevision,
    /// Candidate notebook revision or uncommitted semantic state.
    CandidateRevision,
    /// Semantic command identity.
    Command,
    /// Backend-generated command-context identity.
    CommandContext,
    /// Caller-authorized export intent identity.
    ExportIntent,
    /// Accepted history traversal identity.
    HistoryTraversal,
    /// Physical adapter state or capability snapshot identity.
    PhysicalAdapterState,
    /// Device-neutral planning capability profile identity.
    PlanCapabilityProfile,
    /// Deterministic render-input identity.
    RenderInput,
}

/// Typed physical length meaning used as diagnostic evidence.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PhysicalLengthQuantity {
    /// Amount by which geometry exceeds one admitted physical boundary.
    Overflow,
}

/// Backend-owned remediation category, not an automatically authorized action.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Remediation {
    /// Change one backend-admitted semantic constraint.
    ChangeConstraint,
    /// Edit semantic content through an ordinary application capability.
    EditContent,
    /// Inspect another semantic identity needed to understand the condition.
    InspectRelatedIdentity,
    /// Obtain missing provenance or source evidence.
    ObtainProvenance,
    /// Reduce complete input to satisfy the owning backend resource bound.
    ReduceInput,
    /// Request an explicit supported conversion workflow.
    RequestConversion,
    /// Choose another admitted capability profile.
    SelectCapabilityProfile,
    /// Use a client build compatible with backend-required versions.
    UseCompatibleClient,
}

/// Relationship of one location to the primary semantic owner.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RelationshipKind {
    /// Geometry owners overlap or collide.
    Collision,
    /// One semantic owner depends on another owner.
    Dependency,
    /// Two admitted constraints are mutually incompatible.
    IncompatibleConstraint,
}

/// Semantic location whose identity is meaningful to the owning application.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticLocation {
    /// Stable semantic identity, never a storage or browser presentation path.
    pub identity: String,
    /// Typed semantic owner category.
    pub kind: LocationKind,
    /// Relationship to the primary owner when this location is relational.
    pub relationship: Option<RelationshipKind>,
    /// Primary or related role for this location.
    pub role: LocationRole,
}

/// Human-attention severity independent from operation success or blocking.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Severity {
    /// A condition representing invalid or incompatible operation input.
    Error,
    /// Context useful to a caller but not intrinsically erroneous.
    Informational,
    /// A condition that deserves attention but is not necessarily erroneous.
    Warning,
}
