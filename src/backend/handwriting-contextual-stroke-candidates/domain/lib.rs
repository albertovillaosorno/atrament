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

/// Why contextual candidate identities cannot be addressed unambiguously.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextualStrokeCandidateIdentityError {
    /// Later candidate carrying an already-seen identity.
    pub duplicate_index: usize,
    /// Earliest prior candidate carrying that same identity.
    pub first_index: usize,
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

/// Complete planning-input shape for the contextual candidate value above.
pub type ContextualCandidatePlanningInput<
    CandidateIdentity,
    CharacterIntent,
    Context,
    EntryCondition,
    ExitCondition,
    ProfileChoice,
    SemanticOrigin,
    StrokePayload,
> = ContextualStrokePlanningInput<
    ContextualStrokeCandidate<
        CandidateIdentity,
        CharacterIntent,
        EntryCondition,
        ExitCondition,
        ProfileChoice,
        SemanticOrigin,
        StrokePayload,
    >,
    Context,
>;

/// Constructor-sealed planning-input shape for contextual stroke candidates.
pub type ValidatedContextualCandidatePlanningInput<
    'input,
    CandidateIdentity,
    CharacterIntent,
    Context,
    EntryCondition,
    ExitCondition,
    ProfileChoice,
    SemanticOrigin,
    StrokePayload,
> = ValidatedContextualStrokePlanningInput<
    'input,
    ContextualStrokeCandidate<
        CandidateIdentity,
        CharacterIntent,
        EntryCondition,
        ExitCondition,
        ProfileChoice,
        SemanticOrigin,
        StrokePayload,
    >,
    Context,
>;

/// Result of validating and sealing one contextual planning input.
pub type ContextualCandidatePlanningInputValidationResult<
    'input,
    CandidateIdentity,
    CharacterIntent,
    Context,
    EntryCondition,
    ExitCondition,
    ProfileChoice,
    SemanticOrigin,
    StrokePayload,
> = Result<
    ValidatedContextualCandidatePlanningInput<
        'input,
        CandidateIdentity,
        CharacterIntent,
        Context,
        EntryCondition,
        ExitCondition,
        ProfileChoice,
        SemanticOrigin,
        StrokePayload,
    >,
    ContextualStrokeCandidateIdentityError,
>;

/// Complete transport-neutral input to a later contextual stroke planner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextualStrokePlanningInput<Candidate, Context> {
    /// Caller-supplied candidates in their source order.
    pub candidates: Vec<Candidate>,
    /// Neighbor, word, line, role, and calibrated-style planning context.
    pub context: Context,
}

/// Constructor-sealed evidence that one exact planning input has unique
/// candidate identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedContextualStrokePlanningInput<'input, Candidate, Context> {
    input: &'input ContextualStrokePlanningInput<Candidate, Context>,
}

impl<'input, Candidate, Context>
    ValidatedContextualStrokePlanningInput<'input, Candidate, Context>
{
    /// Return caller-order candidates from the exact admitted input.
    #[must_use]
    pub fn candidates(&self) -> &'input [Candidate] {
        &self.input.candidates
    }

    /// Return the exact caller-owned planning context.
    #[must_use]
    pub const fn context(&self) -> &'input Context {
        &self.input.context
    }

    /// Return the exact planning input that produced this admission evidence.
    #[must_use]
    pub const fn input(
        &self,
    ) -> &'input ContextualStrokePlanningInput<Candidate, Context> {
        self.input
    }
}

/// Validate candidate addressing for one complete planning input.
///
/// This is an explicit admission check rather than a constructor side effect.
/// It preserves candidate order and planning context exactly and performs no
/// candidate scoring or selection.
///
/// # Errors
///
/// Returns the same first duplicate identity evidence as
/// [`validate_contextual_stroke_candidate_identities`].
pub fn validate_contextual_stroke_planning_input<
    CandidateIdentity,
    CharacterIntent,
    Context,
    EntryCondition,
    ExitCondition,
    ProfileChoice,
    SemanticOrigin,
    StrokePayload,
>(
    input: &ContextualCandidatePlanningInput<
        CandidateIdentity,
        CharacterIntent,
        Context,
        EntryCondition,
        ExitCondition,
        ProfileChoice,
        SemanticOrigin,
        StrokePayload,
    >,
) -> Result<(), ContextualStrokeCandidateIdentityError>
where
    CandidateIdentity: Eq,
{
    validate_contextual_stroke_candidate_identities(&input.candidates)
}

/// Validate one exact planner input and seal borrowed addressing evidence.
///
/// # Errors
///
/// Returns exactly the same first duplicate identity evidence as
/// [`validate_contextual_stroke_planning_input`].
pub fn validate_contextual_stroke_planning_input_view<
    CandidateIdentity,
    CharacterIntent,
    Context,
    EntryCondition,
    ExitCondition,
    ProfileChoice,
    SemanticOrigin,
    StrokePayload,
>(
    input: &ContextualCandidatePlanningInput<
        CandidateIdentity,
        CharacterIntent,
        Context,
        EntryCondition,
        ExitCondition,
        ProfileChoice,
        SemanticOrigin,
        StrokePayload,
    >,
) -> ContextualCandidatePlanningInputValidationResult<
    '_,
    CandidateIdentity,
    CharacterIntent,
    Context,
    EntryCondition,
    ExitCondition,
    ProfileChoice,
    SemanticOrigin,
    StrokePayload,
>
where
    CandidateIdentity: Eq,
{
    validate_contextual_stroke_planning_input(input)?;
    Ok(ValidatedContextualStrokePlanningInput { input })
}

/// Detect identity collisions without selecting or ranking any candidate.
///
/// Candidate order and payloads remain unchanged. Callers that require stable
/// candidate addressing can invoke this before planning.
///
/// # Errors
///
/// Returns the first duplicate in caller order paired with its earliest prior
/// occurrence.
pub fn validate_contextual_stroke_candidate_identities<
    CandidateIdentity,
    CharacterIntent,
    EntryCondition,
    ExitCondition,
    ProfileChoice,
    SemanticOrigin,
    StrokePayload,
>(
    candidates: &[
        ContextualStrokeCandidate<
            CandidateIdentity,
            CharacterIntent,
            EntryCondition,
            ExitCondition,
            ProfileChoice,
            SemanticOrigin,
            StrokePayload,
        >
    ],
) -> Result<(), ContextualStrokeCandidateIdentityError>
where
    CandidateIdentity: Eq,
{
    for (duplicate_index, candidate) in candidates.iter().enumerate() {
        if let Some((first_index, _)) = candidates
            .iter()
            .take(duplicate_index)
            .enumerate()
            .find(|(_, prior)| {
                prior.candidate_identity == candidate.candidate_identity
            })
        {
            return Err(ContextualStrokeCandidateIdentityError {
                duplicate_index,
                first_index,
            });
        }
    }
    Ok(())
}
