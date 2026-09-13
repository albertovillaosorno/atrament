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
//   - Outputs: Prompt validation, completion state, exact sample replacement,
//     and next work.
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
//     values, including exact-precondition sample replacement.
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

/// Prompts in one caller-supplied guided calibration session.
pub type CalibrationPrompts<Identity, Speed, Size, SampleIdentity> =
    Vec<CalibrationPrompt<Identity, Speed, Size, SampleIdentity>>;

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
    pub prompts: CalibrationPrompts<Identity, Speed, Size, SampleIdentity>,
    /// Caller-owned known physical reference geometry for this session.
    pub reference_geometry: ReferenceGeometry,
}

/// Constructor-sealed evidence that one exact guided calibration session has
/// unique prompt identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedCalibrationSession<
    'session,
    Identity,
    Speed,
    Size,
    SampleIdentity,
    ReferenceGeometry,
> {
    session: &'session CalibrationSession<
        Identity,
        Speed,
        Size,
        SampleIdentity,
        ReferenceGeometry,
    >,
}

impl<'session, Identity, Speed, Size, SampleIdentity, ReferenceGeometry>
    ValidatedCalibrationSession<
        'session,
        Identity,
        Speed,
        Size,
        SampleIdentity,
        ReferenceGeometry,
    >
{
    /// Return completed prompts in caller-supplied guidance order.
    #[must_use]
    pub fn completed_prompts(
        &self,
    ) -> Vec<
        &'session CalibrationPrompt<Identity, Speed, Size, SampleIdentity>,
    > {
        self.session
            .prompts
            .iter()
            .filter(|prompt| {
                matches!(
                    prompt.progress,
                    CalibrationPromptProgress::Completed { .. }
                )
            })
            .collect()
    }

    /// Return whether every caller-supplied prompt is complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.session.prompts.iter().all(|prompt| {
            matches!(
                prompt.progress,
                CalibrationPromptProgress::Completed { .. }
            )
        })
    }

    /// Return the first pending prompt index in caller-supplied order.
    #[must_use]
    pub fn next_pending_prompt_index(&self) -> Option<usize> {
        self.session.prompts.iter().position(|prompt| {
            matches!(prompt.progress, CalibrationPromptProgress::Pending)
        })
    }

    /// Return the exact admitted session without copying caller-owned values.
    #[must_use]
    pub const fn session(
        &self,
    ) -> &'session CalibrationSession<
        Identity,
        Speed,
        Size,
        SampleIdentity,
        ReferenceGeometry,
    > {
        self.session
    }
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

/// Result of replacing one completed prompt's exact current sample link.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CalibrationSampleReplacement {
    /// The replacement differs from the exact previously completed sample.
    Applied,
    /// The requested replacement already equals the completed sample.
    NoOp,
}

/// Why one exact completed-sample replacement cannot be applied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CalibrationSampleReplacementError<Identity, SampleIdentity> {
    /// The owning prompt is still pending and therefore has no sample to
    /// replace.
    PromptPending {
        /// Exact caller-owned prompt identity.
        prompt: Identity,
    },
    /// The completed prompt no longer has the sample the caller inspected.
    SampleMismatch {
        /// Actual current completed sample identity.
        actual: SampleIdentity,
        /// Exact sample identity the caller expected to replace.
        expected: SampleIdentity,
        /// Exact caller-owned prompt identity.
        prompt: Identity,
    },
    /// The session cannot be edited because prompt identities are ambiguous.
    Session {
        /// Existing session validation failure.
        reason: CalibrationSessionError<Identity>,
    },
    /// No prompt in the current plan owns the requested identity.
    UnknownPrompt {
        /// Missing caller-owned prompt identity.
        prompt: Identity,
    },
}

/// Borrowed completed-prompt inspection result.
pub type CalibrationCompletedPromptsResult<
    'session,
    Identity,
    Speed,
    Size,
    SampleIdentity,
> = Result<
    Vec<&'session CalibrationPrompt<Identity, Speed, Size, SampleIdentity>>,
    CalibrationSessionError<Identity>,
>;

/// Exact completed-sample replacement result.
pub type CalibrationSampleReplacementResult<Identity, SampleIdentity> = Result<
    CalibrationSampleReplacement,
    CalibrationSampleReplacementError<Identity, SampleIdentity>,
>;

/// Result of locating the next resumable calibration prompt.
pub type CalibrationPromptIndexResult<Identity> =
    Result<Option<usize>, CalibrationSessionError<Identity>>;

impl<Identity, Speed, Size, SampleIdentity, ReferenceGeometry>
    CalibrationSession<Identity, Speed, Size, SampleIdentity, ReferenceGeometry>
where
    Identity: Clone + Ord,
{
    /// Return completed prompts in caller-supplied guidance order.
    ///
    /// The returned borrowed prompts preserve identity, category, speed, size,
    /// and sample links for read-only inspection. This domain does not decide
    /// whether a completed sample is weak or choose replacement policy.
    ///
    /// # Errors
    ///
    /// Returns duplicate prompt identity before projecting completed work.
    pub fn completed_prompts(
        &self,
    ) -> CalibrationCompletedPromptsResult<
        '_,
        Identity,
        Speed,
        Size,
        SampleIdentity,
    > {
        Ok(self.validate_view()?.completed_prompts())
    }

    /// Return whether every caller-supplied calibration prompt is complete.
    ///
    /// # Errors
    ///
    /// Returns duplicate prompt identity before deriving completion state.
    pub fn is_complete(
        &self,
    ) -> Result<bool, CalibrationSessionError<Identity>> {
        Ok(self.validate_view()?.is_complete())
    }

    /// Return the first pending prompt index in caller-supplied guidance order.
    ///
    /// Completed prompts are skipped so persisted session state resumes at the
    /// next unfinished prompt without changing prompt order or copying data.
    ///
    /// # Errors
    ///
    /// Returns duplicate prompt identity before selecting resumable work.
    pub fn next_pending_prompt_index(
        &self,
    ) -> CalibrationPromptIndexResult<Identity> {
        Ok(self.validate_view()?.next_pending_prompt_index())
    }

    /// Replace the exact sample link of one completed prompt.
    ///
    /// This operation does not classify sample quality or choose replacement
    /// policy. The caller must name both the prompt and exact sample it
    /// previously inspected, so stale inspection cannot overwrite a newer link.
    /// Unrelated prompt progress and metadata remain unchanged.
    ///
    /// # Errors
    ///
    /// Returns session identity ambiguity, an unknown or pending prompt, or an
    /// exact sample mismatch without changing this session.
    pub fn replace_completed_sample(
        &mut self,
        prompt_identity: &Identity,
        expected_sample: &SampleIdentity,
        replacement_sample: SampleIdentity,
    ) -> CalibrationSampleReplacementResult<Identity, SampleIdentity>
    where
        SampleIdentity: Clone + Eq,
    {
        self.validate().map_err(|reason| {
            CalibrationSampleReplacementError::Session { reason }
        })?;
        let Some(prompt) = self
            .prompts
            .iter_mut()
            .find(|prompt| prompt.identity == *prompt_identity)
        else {
            return Err(CalibrationSampleReplacementError::UnknownPrompt {
                prompt: prompt_identity.clone(),
            });
        };
        let CalibrationPromptProgress::Completed { sample } =
            &mut prompt.progress
        else {
            return Err(CalibrationSampleReplacementError::PromptPending {
                prompt: prompt_identity.clone(),
            });
        };
        if sample != expected_sample {
            return Err(CalibrationSampleReplacementError::SampleMismatch {
                actual: sample.clone(),
                expected: expected_sample.clone(),
                prompt: prompt_identity.clone(),
            });
        }
        if sample == &replacement_sample {
            return Ok(CalibrationSampleReplacement::NoOp);
        }
        *sample = replacement_sample;
        Ok(CalibrationSampleReplacement::Applied)
    }

    /// Validate stable prompt identities before resuming this session.
    ///
    /// The owning workflow supplies prompt count, category mix, speeds, sizes,
    /// and reference geometry. This domain rejects only identity ambiguity.
    ///
    /// # Errors
    ///
    /// Returns the first duplicate prompt identity in plan order.
    pub fn validate(&self) -> Result<(), CalibrationSessionError<Identity>> {
        let mut identities = BTreeSet::new();
        for prompt in &self.prompts {
            if !identities.insert(&prompt.identity) {
                return Err(CalibrationSessionError::DuplicatePromptIdentity {
                    prompt: prompt.identity.clone(),
                });
            }
        }
        Ok(())
    }

    /// Validate stable prompt identities and seal this exact session.
    ///
    /// # Errors
    ///
    /// Returns the same first duplicate prompt identity as [`Self::validate`].
    pub fn validate_view(
        &self,
    ) -> Result<
        ValidatedCalibrationSession<
            '_,
            Identity,
            Speed,
            Size,
            SampleIdentity,
            ReferenceGeometry,
        >,
        CalibrationSessionError<Identity>,
    > {
        self.validate()?;
        Ok(ValidatedCalibrationSession { session: self })
    }

}
