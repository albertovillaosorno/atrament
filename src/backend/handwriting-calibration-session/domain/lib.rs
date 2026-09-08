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
//   - Transport-neutral guided calibration prompt categories and resumable
//     state.
// - Must-Not:
//   - Choose a minimum sample set, prompt wording, reference-mark geometry,
//     capture UI, confidence policy, extraction behavior, or physical units.
// - Allows:
//   - Inputs: Caller-owned prompt plans, speed/size values, samples, and
//     reference geometry.
//   - Outputs: Prompt identity validation, completion state, and next work.
//   - Side effects: Process-local validation allocation only.
// - Split-When:
//   - Capture orchestration or reference geometry gains independent authority.
// - Merge-When:
//   - Guided calibration progress becomes inseparable from an application port.
// - Summary:
//   - Makes accepted calibration plans resumable without inventing sample
//     policy.
// - Description:
//   - Preserves prompt order and completed sample links over caller-owned
//     values.
// - Usage:
//   - Resume the first pending prompt from a validated caller-supplied plan.
// - Defaults:
//   - The owning workflow decides which prompts and how many samples are
//     enough.
//

//! Pure guided-calibration session progress over caller-owned prompt plans.

use std::collections::BTreeSet;

/// Accepted guided-calibration prompt category.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CalibrationPromptKind {
    /// Unconstrained free-writing sample.
    FreeWriting,
    /// Heading-scale writing sample.
    Heading,
    /// One isolated character sample.
    IsolatedCharacter,
    /// One common join sample.
    Join,
    /// Mathematical-symbol writing sample.
    MathematicalSymbol,
    /// Numeral writing sample.
    Numeral,
    /// Punctuation writing sample.
    Punctuation,
    /// Sentence writing sample.
    Sentence,
    /// Word writing sample.
    Word,
}

/// Resumable progress for one caller-owned calibration prompt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CalibrationPromptProgress<SampleIdentity> {
    /// Prompt was completed and produced one caller-owned sample identity.
    Completed {
        /// Captured sample identity retained without reinterpretation.
        sample: SampleIdentity,
    },
    /// Prompt still requires capture.
    Pending,
}

/// One ordered prompt in a guided calibration plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalibrationPrompt<Identity, Speed, Size, SampleIdentity> {
    /// Caller-owned stable prompt identity.
    pub identity: Identity,
    /// Accepted calibration content category.
    pub kind: CalibrationPromptKind,
    /// Resumable completion state.
    pub progress: CalibrationPromptProgress<SampleIdentity>,
    /// Caller-owned selected writing size.
    pub size: Size,
    /// Caller-owned selected writing speed.
    pub speed: Speed,
}

/// Caller-supplied guided calibration plan and resumable progress.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalibrationSession<
    Identity,
    Speed,
    Size,
    SampleIdentity,
    ReferenceGeometry,
> {
    /// Prompts in the owning workflow's accepted guidance order.
    pub prompts: Vec<CalibrationPrompt<Identity, Speed, Size, SampleIdentity>>,
    /// Caller-owned known physical reference geometry for this session.
    pub reference_geometry: ReferenceGeometry,
}

/// Why a caller-supplied guided calibration plan cannot be resumed safely.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CalibrationSessionError<Identity> {
    /// Two plan entries use the same stable prompt identity.
    DuplicatePromptIdentity {
        /// Duplicate prompt identity retained exactly for diagnostics.
        prompt: Identity,
    },
}

/// Return whether every caller-supplied calibration prompt is complete.
///
/// # Errors
///
/// Returns duplicate prompt identity before deriving completion state.
pub fn calibration_session_complete<Identity, Speed, Size, SampleIdentity, Ref>(
    session: &CalibrationSession<Identity, Speed, Size, SampleIdentity, Ref>,
) -> Result<bool, CalibrationSessionError<Identity>>
where
    Identity: Clone + Ord,
{
    validate_calibration_session(session)?;
    Ok(session.prompts.iter().all(|prompt| {
        matches!(prompt.progress, CalibrationPromptProgress::Completed { .. })
    }))
}

/// Return the first pending prompt index in caller-supplied guidance order.
///
/// Completed prompts are skipped so persisted session state resumes at the next
/// unfinished prompt without changing prompt order or copying prompt data.
///
/// # Errors
///
/// Returns duplicate prompt identity before selecting resumable work.
pub fn next_pending_calibration_prompt_index<
    Identity,
    Speed,
    Size,
    SampleIdentity,
    Ref,
>(
    session: &CalibrationSession<Identity, Speed, Size, SampleIdentity, Ref>,
) -> Result<Option<usize>, CalibrationSessionError<Identity>>
where
    Identity: Clone + Ord,
{
    validate_calibration_session(session)?;
    Ok(session.prompts.iter().position(|prompt| {
        matches!(prompt.progress, CalibrationPromptProgress::Pending)
    }))
}

/// Validate stable prompt identities before resuming a calibration session.
///
/// The owning workflow supplies prompt count, category mix, speeds, sizes, and
/// reference geometry. This domain rejects only identity ambiguity.
///
/// # Errors
///
/// Returns the first duplicate prompt identity in plan order.
pub fn validate_calibration_session<Identity, Speed, Size, SampleIdentity, Ref>(
    session: &CalibrationSession<Identity, Speed, Size, SampleIdentity, Ref>,
) -> Result<(), CalibrationSessionError<Identity>>
where
    Identity: Clone + Ord,
{
    let mut identities = BTreeSet::new();
    for prompt in &session.prompts {
        if !identities.insert(&prompt.identity) {
            return Err(CalibrationSessionError::DuplicatePromptIdentity {
                prompt: prompt.identity.clone(),
            });
        }
    }
    Ok(())
}
