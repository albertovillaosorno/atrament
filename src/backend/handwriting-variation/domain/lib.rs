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
//   - Transport-neutral bounded handwriting-variation parameter invariants.
// - Must-Not:
//   - Choose distribution families, correlation algorithms, context semantics,
//     random generators, parameter vocabularies, units, or sampling policy.
// - Allows:
//   - Inputs: Caller-owned values, units, distribution/correlation/context
//     data,
//     parameter identities, seeds, and stable semantic identities.
//   - Outputs: Typed bound validation and replay-key values.
//   - Side effects: None beyond process-local validation.
// - Split-When:
//   - Sampling, statistical validation, or model fitting gains independent
//     authority.
// - Merge-When:
//   - Variation validation becomes inseparable from one handwriting profile.
// - Summary:
//   - Validates accepted variation envelopes without implementing randomness.
// - Description:
//   - Keeps observed/authorized bounds, scale, metadata, and replay inputs
//     typed.
// - Usage:
//   - Validate profile-owned parameter envelopes before sampling or rendering.
// - Defaults:
//   - No distribution or correlation behavior is inferred from generic
//     metadata.
//

//! Pure bounded-variation parameter invariants for handwriting profiles.

/// Evidentiary basis for one minimum or maximum variation bound.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum VariationBoundBasis {
    /// Bound is an explicit authorized creative limit.
    Authorized,
    /// Bound was observed from calibration evidence.
    Observed,
}

/// One typed minimum or maximum bound with its evidentiary basis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariationBound<Value> {
    /// Whether this bound is observed or explicitly authorized.
    pub basis: VariationBoundBasis,
    /// Caller-owned value in the parameter's declared unit.
    pub value: Value,
}

/// Scale at which one variable handwriting parameter operates.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum VariationScale {
    /// Character-local variation.
    Character,
    /// Document-wide variation.
    Document,
    /// Line-scale correlated variation.
    Line,
    /// Page-scale variation.
    Page,
    /// Profile-wide variation.
    Profile,
    /// Stroke-local variation.
    Stroke,
    /// Word-scale variation.
    Word,
}

/// One complete caller-owned variable-parameter envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariationParameter<
    ParameterIdentity,
    Value,
    Unit,
    Distribution,
    CorrelationGroup,
    ContextRule,
> {
    /// Central tendency in the declared unit.
    pub central_tendency: Value,
    /// Caller-owned contextual applicability rules.
    pub context_rules: Vec<ContextRule>,
    /// Caller-owned correlation-group identities.
    pub correlation_groups: Vec<CorrelationGroup>,
    /// Caller-owned distribution-family metadata.
    pub distribution: Distribution,
    /// Maximum admitted value and its evidence basis.
    pub maximum: VariationBound<Value>,
    /// Minimum admitted value and its evidence basis.
    pub minimum: VariationBound<Value>,
    /// Stable caller-owned identity for this variable parameter.
    pub parameter_identity: ParameterIdentity,
    /// Scale at which this parameter operates.
    pub scale: VariationScale,
    /// Caller-owned typed unit identity.
    pub unit: Unit,
}

/// Why one parameter identity is ambiguous inside a parameter set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VariationParameterIdentityError {
    /// Later parameter carrying an already-seen identity.
    pub duplicate_index: usize,
    /// Earliest prior parameter carrying that same identity.
    pub first_index: usize,
}

/// Why a bounded handwriting-variation parameter is invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VariationParameterError {
    /// Central tendency is greater than the admitted maximum.
    CentralTendencyAboveMaximum,
    /// Central tendency is less than the admitted minimum.
    CentralTendencyBelowMinimum,
    /// Declared minimum is greater than the declared maximum.
    MinimumAboveMaximum,
}

/// Why caller-produced samples contradict deterministic replay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VariationReplayConsistencyError {
    /// Later observation that contradicts the earlier replay result.
    pub conflicting_index: usize,
    /// Earlier observation with the same replay key and a different value.
    pub first_index: usize,
}

/// Deterministic replay inputs for one variable parameter and semantic target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariationReplayKey<ParameterIdentity, Seed, SemanticIdentity> {
    /// Caller-owned accepted document variation seed.
    pub document_seed: Seed,
    /// Stable variable-parameter identity scoped by this replay key.
    pub parameter_identity: ParameterIdentity,
    /// Stable semantic identity that scopes deterministic variation.
    pub semantic_identity: SemanticIdentity,
}

/// One caller-produced variation sample bound to deterministic replay inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariationSample<ReplayKey, Value> {
    /// Exact deterministic replay inputs used for this sample.
    pub replay_key: ReplayKey,
    /// Caller-produced sampled value in the parameter's declared unit.
    pub value: Value,
}

/// Why one caller-produced sample violates its admitted variation envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VariationSampleError {
    /// Sample exceeds the admitted maximum.
    AboveMaximum,
    /// Sample is below the admitted minimum.
    BelowMinimum,
}

/// Why a complete caller-produced sample set is not admissible.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VariationSampleSetError {
    /// The configured parameter envelope is invalid.
    Parameter(VariationParameterError),
    /// One sample belongs to a different variable parameter.
    ParameterIdentityMismatch {
        /// Zero-based observation index in caller order.
        sample_index: usize,
    },
    /// Exact replay inputs produced contradictory sampled values.
    ReplayConflict(VariationReplayConsistencyError),
    /// One sampled value falls outside the admitted parameter envelope.
    Sample {
        /// Bound failure for this sample.
        error: VariationSampleError,
        /// Zero-based observation index in caller order.
        sample_index: usize,
    },
}

impl<
    ParameterIdentity,
    Value,
    Unit,
    Distribution,
    CorrelationGroup,
    ContextRule,
>
    VariationParameter<
        ParameterIdentity,
        Value,
        Unit,
        Distribution,
        CorrelationGroup,
        ContextRule,
    >
where
    Value: Ord,
{
    /// Validate this complete variable-parameter envelope before any sampling.
    ///
    /// # Errors
    ///
    /// Returns a typed failure when the minimum exceeds the maximum or the
    /// central tendency falls outside the admitted envelope.
    pub fn validate(&self) -> Result<(), VariationParameterError> {
        if self.minimum.value > self.maximum.value {
            return Err(VariationParameterError::MinimumAboveMaximum);
        }
        if self.central_tendency < self.minimum.value {
            return Err(VariationParameterError::CentralTendencyBelowMinimum);
        }
        if self.central_tendency > self.maximum.value {
            return Err(VariationParameterError::CentralTendencyAboveMaximum);
        }
        Ok(())
    }

    /// Validate one caller-produced sample against this configured envelope.
    ///
    /// This method does not produce the sample or interpret its replay key.
    ///
    /// # Errors
    ///
    /// Returns a typed failure when the sampled value lies outside the
    /// inclusive envelope.
    pub fn validate_sample<ReplayKey>(
        &self,
        sample: &VariationSample<ReplayKey, Value>,
    ) -> Result<(), VariationSampleError> {
        if sample.value < self.minimum.value {
            return Err(VariationSampleError::BelowMinimum);
        }
        if sample.value > self.maximum.value {
            return Err(VariationSampleError::AboveMaximum);
        }
        Ok(())
    }
}

/// Constructor-sealed evidence that one exact parameter collection has unique
/// caller-owned identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedVariationParameters<'parameters, Parameter> {
    parameters: &'parameters [Parameter],
}

impl<'parameters, Parameter>
    ValidatedVariationParameters<'parameters, Parameter>
{
    /// Return the exact caller-owned parameter collection that was admitted.
    #[must_use]
    pub const fn parameters(&self) -> &'parameters [Parameter] {
        self.parameters
    }
}

/// Result of admitting one exact variation parameter collection.
pub type VariationParameterIdentityValidationResult<
    'parameters,
    ParameterIdentity,
    Value,
    Unit,
    Distribution,
    CorrelationGroup,
    ContextRule,
> = Result<
    ValidatedVariationParameters<
        'parameters,
        VariationParameter<
            ParameterIdentity,
            Value,
            Unit,
            Distribution,
            CorrelationGroup,
            ContextRule,
        >,
    >,
    VariationParameterIdentityError,
>;

/// Constructor-sealed evidence that one exact parameter-bound sample set
/// passed
/// all structural variation invariants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedVariationSampleSet<
    'parameter,
    'samples,
    Parameter,
    Sample,
> {
    parameter: &'parameter Parameter,
    samples: &'samples [Sample],
}

impl<'parameter, 'samples, Parameter, Sample>
    ValidatedVariationSampleSet<'parameter, 'samples, Parameter, Sample>
{
    /// Return the exact caller-owned parameter envelope used for admission.
    #[must_use]
    pub const fn parameter(&self) -> &'parameter Parameter {
        self.parameter
    }

    /// Return the exact caller-produced sample slice used for admission.
    #[must_use]
    pub const fn samples(&self) -> &'samples [Sample] {
        self.samples
    }
}

/// Result of admitting one exact parameter-bound variation sample set.
pub type VariationSampleSetValidationResult<
    'parameter,
    'samples,
    ParameterIdentity,
    Value,
    Unit,
    Distribution,
    CorrelationGroup,
    ContextRule,
    Seed,
    SemanticIdentity,
> = Result<
    ValidatedVariationSampleSet<
        'parameter,
        'samples,
        VariationParameter<
            ParameterIdentity,
            Value,
            Unit,
            Distribution,
            CorrelationGroup,
            ContextRule,
        >,
        VariationSample<
            VariationReplayKey<ParameterIdentity, Seed, SemanticIdentity>,
            Value,
        >,
    >,
    VariationSampleSetError,
>;

/// Require equal sampled values whenever exact replay inputs repeat.
///
/// Samples remain caller-produced. This function does not choose a random
/// generator, distribution, correlation model, or sampling algorithm. It only
/// enforces the accepted replay invariant over already-produced observations.
///
/// # Errors
///
/// Returns the first observation-order conflict, paired with the earliest
/// prior observation carrying the same replay key and a different value.
pub fn validate_variation_replay_consistency<ReplayKey, Value>(
    samples: &[VariationSample<ReplayKey, Value>],
) -> Result<(), VariationReplayConsistencyError>
where
    ReplayKey: Eq,
    Value: Eq,
{
    for (conflicting_index, sample) in samples.iter().enumerate() {
        if let Some((first_index, _)) = samples
            .iter()
            .take(conflicting_index)
            .enumerate()
            .find(|(_, prior)| {
                prior.replay_key == sample.replay_key
                    && prior.value != sample.value
            })
        {
            return Err(VariationReplayConsistencyError {
                conflicting_index,
                first_index,
            });
        }
    }
    Ok(())
}
/// Validate one complete parameter-bound sample set without producing samples.
///
/// Validation order is parameter envelope, caller-order parameter identity,
/// caller-order sample bounds, then replay consistency. This prevents a sample
/// from being interpreted against the wrong variable parameter envelope.
///
/// # Errors
///
/// Returns [`VariationSampleSetError`] for the first invalid parameter, sampled
/// value, or exact-replay contradiction.
pub fn validate_variation_sample_set<
    ParameterIdentity,
    Value,
    Unit,
    Distribution,
    CorrelationGroup,
    ContextRule,
    Seed,
    SemanticIdentity,
>(
    parameter: &VariationParameter<
        ParameterIdentity,
        Value,
        Unit,
        Distribution,
        CorrelationGroup,
        ContextRule,
    >,
    samples: &[
        VariationSample<
            VariationReplayKey<ParameterIdentity, Seed, SemanticIdentity>,
            Value,
        >
    ],
) -> Result<(), VariationSampleSetError>
where
    ParameterIdentity: Eq,
    Seed: Eq,
    SemanticIdentity: Eq,
    Value: Eq + Ord,
{
    parameter
        .validate()
        .map_err(VariationSampleSetError::Parameter)?;
    for (sample_index, sample) in samples.iter().enumerate() {
        if sample.replay_key.parameter_identity
            != parameter.parameter_identity
        {
            return Err(VariationSampleSetError::ParameterIdentityMismatch {
                sample_index,
            });
        }
    }
    for (sample_index, sample) in samples.iter().enumerate() {
        parameter
            .validate_sample(sample)
            .map_err(|error| VariationSampleSetError::Sample {
                error,
                sample_index,
            })?;
    }
    validate_variation_replay_consistency(samples)
        .map_err(VariationSampleSetError::ReplayConflict)
}
/// Validate one complete sample set and seal its exact borrowed evidence.
///
/// # Errors
///
/// Returns exactly the same parameter, identity, bound, then replay failure as
/// [`validate_variation_sample_set`].
pub fn validate_variation_sample_set_view<
    'parameter,
    'samples,
    ParameterIdentity,
    Value,
    Unit,
    Distribution,
    CorrelationGroup,
    ContextRule,
    Seed,
    SemanticIdentity,
>(
    parameter: &'parameter VariationParameter<
        ParameterIdentity,
        Value,
        Unit,
        Distribution,
        CorrelationGroup,
        ContextRule,
    >,
    samples: &'samples [
        VariationSample<
            VariationReplayKey<ParameterIdentity, Seed, SemanticIdentity>,
            Value,
        >
    ],
) -> VariationSampleSetValidationResult<
    'parameter,
    'samples,
    ParameterIdentity,
    Value,
    Unit,
    Distribution,
    CorrelationGroup,
    ContextRule,
    Seed,
    SemanticIdentity,
>
where
    ParameterIdentity: Eq,
    Seed: Eq,
    SemanticIdentity: Eq,
    Value: Eq + Ord,
{
    validate_variation_sample_set(parameter, samples)?;
    Ok(ValidatedVariationSampleSet { parameter, samples })
}

/// Require unique caller-owned identities across one parameter collection.
///
/// This does not choose or interpret a parameter vocabulary. It only prevents
/// two envelopes from claiming the same replay identity.
///
/// # Errors
///
/// Returns the first duplicate in caller order paired with its earliest prior
/// occurrence.
pub fn validate_variation_parameter_identities<
    ParameterIdentity,
    Value,
    Unit,
    Distribution,
    CorrelationGroup,
    ContextRule,
>(
    parameters: &[
        VariationParameter<
            ParameterIdentity,
            Value,
            Unit,
            Distribution,
            CorrelationGroup,
            ContextRule,
        >
    ],
) -> Result<(), VariationParameterIdentityError>
where
    ParameterIdentity: Eq,
{
    for (duplicate_index, parameter) in parameters.iter().enumerate() {
        if let Some((first_index, _)) = parameters
            .iter()
            .take(duplicate_index)
            .enumerate()
            .find(|(_, prior)| {
                prior.parameter_identity == parameter.parameter_identity
            })
        {
            return Err(VariationParameterIdentityError {
                duplicate_index,
                first_index,
            });
        }
    }
    Ok(())
}

/// Validate parameter identity uniqueness and seal the exact collection.
///
/// # Errors
///
/// Returns the same first duplicate and earliest prior owner as
/// [`validate_variation_parameter_identities`].
pub fn validate_variation_parameter_identities_view<
    ParameterIdentity,
    Value,
    Unit,
    Distribution,
    CorrelationGroup,
    ContextRule,
>(
    parameters: &[
        VariationParameter<
            ParameterIdentity,
            Value,
            Unit,
            Distribution,
            CorrelationGroup,
            ContextRule,
        >
    ],
) -> VariationParameterIdentityValidationResult<
    '_,
    ParameterIdentity,
    Value,
    Unit,
    Distribution,
    CorrelationGroup,
    ContextRule,
>
where
    ParameterIdentity: Eq,
{
    validate_variation_parameter_identities(parameters)?;
    Ok(ValidatedVariationParameters { parameters })
}
