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
//     seeds, and stable semantic identities.
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
    /// Scale at which this parameter operates.
    pub scale: VariationScale,
    /// Caller-owned typed unit identity.
    pub unit: Unit,
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

/// Deterministic replay inputs required by accepted variation sampling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariationReplayKey<Seed, SemanticIdentity> {
    /// Caller-owned accepted document variation seed.
    pub document_seed: Seed,
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

impl<Value, Unit, Distribution, CorrelationGroup, ContextRule>
    VariationParameter<Value, Unit, Distribution, CorrelationGroup, ContextRule>
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
