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
    PhysicalPageProfileError, RevisionIdentity, SemanticIdentityKind,
    TableRowRole,
};

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

    /// Read the current accepted revision without creating another revision.
    fn current(&self) -> Option<&AcceptedRevision>;

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
}
