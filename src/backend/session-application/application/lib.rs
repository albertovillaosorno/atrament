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
//   - Active-process pre-acceptance draft and accepted semantic notebook state.
// - Must-Not:
//   - Persist session state, parse transport input, or duplicate semantic
//     rules.
// - Allows:
//   - Inputs: Existing draft and semantic application-service operations.
//   - Outputs: Borrowed access to the active semantic authority and draft port.
//   - Side effects: Process-local mutation through owned application services.
// - Split-When:
//   - Assets, history, or derived state require independently bounded owners.
// - Merge-When:
//   - One broader application authority owns every active-session capability.
// - Summary:
//   - Owns mutable Atrament application state for one disposable process.
// - Description:
//   - Gives draft and accepted notebook state one process-lifetime owner while
//     preserving their established application contracts.
// - Usage:
//   - Construct one instance in the runtime composition root and drop it when
//     the active localhost session ends.
// - Defaults:
//   - Starts with empty draft fields and no accepted semantic revision.
//

//! Process-lifetime owner for disposable Atrament application state.

use std::fmt;

use atrament_semantic_notebook::{
    AcceptedRevision, CandidateIdentity, Notebook, RevisionIdentity,
};
use atrament_semantic_notebook_port::{
    AcceptanceOutcome, HistoryAvailabilityOutcome, HistoryDirection,
    HistoryTraversalOutcome, SemanticNotebookHistory as _,
    SemanticNotebookSession as _,
};
use atrament_semantic_notebook_session::SemanticNotebookSessionService;
use atrament_session_draft::SessionDraftService;
use atrament_session_draft_port::{DraftField, DraftMutation, SessionDraft};

/// Mutable application state owned by one active Atrament process.
#[derive(Default)]
pub struct SessionApplication {
    draft: SessionDraftService,
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
        self.semantic.accept(candidate)
    }

    /// Read the current accepted semantic revision without creating another.
    #[must_use]
    pub fn accepted_revision(&self) -> Option<&AcceptedRevision> {
        self.semantic.current()
    }

    /// Inspect in-memory semantic Undo and Redo availability.
    #[must_use]
    pub fn history_availability(&self) -> HistoryAvailabilityOutcome {
        self.semantic.history_availability()
    }

    /// Traverse one in-memory semantic history transaction.
    pub fn traverse_history(
        &mut self,
        base: RevisionIdentity,
        direction: HistoryDirection,
    ) -> HistoryTraversalOutcome {
        self.semantic.traverse_history(base, direction)
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
