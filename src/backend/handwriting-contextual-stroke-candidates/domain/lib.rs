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
//   - Transport-neutral contextual stroke-candidate and planner-input
//     structure.
// - Must-Not:
//   - Select or rank candidates, join/deform/space strokes, choose geometry or
//     units, infer language context, render, or emit machine motion.
// - Allows:
//   - Inputs: Caller-owned candidate payloads, semantic/profile provenance,
//     entry/exit conditions, neighboring context, word position, line geometry,
//     semantic role, and calibrated writing style.
//   - Outputs: Inspectable candidate collections and planning context.
//   - Side effects: None.
// - Split-When:
//   - Candidate scoring or contextual planning gains executable policy.
// - Merge-When:
//   - Candidate representation becomes inseparable from stroke-plan output.
// - Summary:
//   - Freezes planner inputs without implementing handwriting planning.
// - Description:
//   - Preserves contextual candidate and profile/semantic evidence before
//     selection.
// - Usage:
//   - Supply one complete input to a later continuous-stroke planner.
// - Defaults:
//   - No candidate preference, join, deformation, or spacing is inferred.
//

//! Contextual handwriting stroke candidates before planner selection.

/// One profile-provided contextual stroke candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextualStrokeCandidate<
    CandidateIdentity,
    CharacterIntent,
    EntryCondition,
    ExitCondition,
    ProfileChoice,
    SemanticOrigin,
    StrokePayload,
> {
    /// Stable caller-owned candidate identity.
    pub candidate_identity: CandidateIdentity,
    /// Caller-owned character or grapheme intent represented by the candidate.
    pub character_intent: CharacterIntent,
    /// Candidate entry condition considered by a later planner.
    pub entry_condition: EntryCondition,
    /// Candidate exit condition considered by a later planner.
    pub exit_condition: ExitCondition,
    /// Handwriting profile choice that contributed this candidate.
    pub profile_choice: ProfileChoice,
    /// Semantic span or object from which this candidate originates.
    pub semantic_origin: SemanticOrigin,
    /// Caller-owned stroke vocabulary payload before planning transformations.
    pub stroke_payload: StrokePayload,
}

/// Caller-owned neighboring context consumed by later stroke planning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrokePlanningContext<
    LineGeometry,
    NeighboringGraphemes,
    SemanticRole,
    WordPosition,
    WritingStyle,
> {
    /// Caller-owned line geometry relevant to spacing and deformation.
    pub line_geometry: LineGeometry,
    /// Caller-owned neighboring grapheme context.
    pub neighboring_graphemes: NeighboringGraphemes,
    /// Caller-owned semantic handwriting role.
    pub semantic_role: SemanticRole,
    /// Caller-owned position within the surrounding word.
    pub word_position: WordPosition,
    /// Caller-owned calibrated handwriting style or profile state.
    pub writing_style: WritingStyle,
}

/// Complete transport-neutral input to a later contextual stroke planner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextualStrokePlanningInput<Candidate, Context> {
    /// Caller-supplied candidates in their source order.
    pub candidates: Vec<Candidate>,
    /// Neighbor, word, line, role, and calibrated-style planning context.
    pub context: Context,
}
