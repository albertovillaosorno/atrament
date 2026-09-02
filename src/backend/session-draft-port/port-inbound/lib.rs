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
//   - Inbound application contract for pre-acceptance session draft text.
// - Must-Not:
//   - Parse HTTP, authenticate callers, or represent accepted notebook state.
// - Allows:
//   - Inputs: Task, source, or raw model-response text replacements.
//   - Outputs: Applied or resource-limit result plus current draft text reads.
//   - Side effects: Application-owned in-memory draft mutation.
// - Split-When:
//   - One draft field needs an independently versioned application capability.
// - Merge-When:
//   - Pre-acceptance draft state becomes part of another application boundary.
// - Summary:
//   - Defines the inbound port for disposable source-preparation draft state.
// - Description:
//   - Separates session draft authority from HTTP and future accepted
//     revisions.
// - Usage:
//   - Implement in the draft application and inject at runtime composition.
// - Defaults:
//   - Draft text is session-private and never accepted notebook authority.
//

//! Inbound application port for pre-acceptance Atrament session draft text.

use atrament_diagnostic::DiagnosticSet;

/// One editable pre-acceptance text field in the first user journey.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DraftField {
    /// Raw external model response retained only for later validation.
    Candidate,
    /// User-supplied source notes and formulas.
    Source,
    /// User-supplied formatting or communication task.
    Task,
}

/// Result of replacing one complete session draft field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DraftMutation {
    /// The complete replacement became current session draft state.
    Applied,
    /// The replacement exceeded the backend-owned field resource limit.
    ResourceLimit {
        /// Shared semantic diagnostic explaining the rejected replacement.
        diagnostics: DiagnosticSet,
    },
}

/// Application service owning disposable pre-acceptance draft text.
pub trait SessionDraft {
    /// Replace one complete draft field without partial truncation.
    fn replace(&mut self, field: DraftField, value: String) -> DraftMutation;

    /// Read one current draft field from active process memory.
    fn value(&self, field: DraftField) -> &str;
}
