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
    AcceptedIdentity, AcceptedRevision, CandidateIdentity, IdentityExhausted,
    Notebook, RevisionIdentity,
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
    /// One candidate semantic reference does not name an owned candidate
    /// object.
    MissingReference {
        /// Candidate-local identity that could not be resolved.
        candidate: CandidateIdentity,
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
}
