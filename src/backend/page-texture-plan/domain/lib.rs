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
//   - Transport-neutral bounded page-texture intent and replay authority.
// - Must-Not:
//   - Generate noise, mutate vector geometry, choose physical units, frequency
//     models, legibility thresholds, tiling policy, rasterization, or output.
// - Allows:
//   - Inputs: Caller-owned geometry identity, physical bounds/scale, strength,
//     and the accepted stochastic replay key.
//   - Outputs: Validated bounded page-texture plans.
//   - Side effects: None.
// - Split-When:
//   - Texture generation, tiling diagnostics, or perceptual validation gains
//     independent executable authority.
// - Merge-When:
//   - Page texture becomes inseparable from one renderer implementation.
// - Summary:
//   - Keeps page-level soft-noise intent bounded and replayable.
// - Description:
//   - Retains geometry authority without providing any geometry mutation
//     output.
// - Usage:
//   - Validate texture intent before later deterministic CPU material
//     rendering.
// - Defaults:
//   - No frequency, strength, unit, or visual threshold is chosen here.
//

//! Bounded replayable page-texture intent before deterministic CPU rendering.

pub use atrament_material_layer_plan::StochasticLayerReplayKey;

/// Caller-owned texture strength constrained to an inclusive envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedTextureStrength<Value> {
    /// Maximum admitted texture strength.
    pub maximum: Value,
    /// Minimum admitted texture strength.
    pub minimum: Value,
    /// Selected texture strength for this page plan.
    pub selected: Value,
}

/// One page-level texture intent that cannot itself mutate vector geometry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageTexturePlan<
    GeometryAuthority,
    PhysicalBounds,
    PhysicalScale,
    Strength,
    ReplayKey,
> {
    /// Read-only vector-geometry authority associated with this page texture.
    pub geometry_authority: GeometryAuthority,
    /// Caller-owned physical page bounds.
    pub physical_bounds: PhysicalBounds,
    /// Caller-owned physical scale for the soft-noise field.
    pub physical_scale: PhysicalScale,
    /// Complete replay key for deterministic stochastic evaluation.
    pub replay_key: ReplayKey,
    /// Bounded texture-strength choice.
    pub strength: BoundedTextureStrength<Strength>,
}

/// Constructor-sealed evidence that one exact page-texture plan has an admitted
/// inclusive strength envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedPageTexturePlan<
    'plan,
    GeometryAuthority,
    PhysicalBounds,
    PhysicalScale,
    Strength,
    ReplayKey,
> {
    plan: &'plan PageTexturePlan<
        GeometryAuthority,
        PhysicalBounds,
        PhysicalScale,
        Strength,
        ReplayKey,
    >,
}

impl<
    'plan,
    GeometryAuthority,
    PhysicalBounds,
    PhysicalScale,
    Strength,
    ReplayKey,
> ValidatedPageTexturePlan<
    'plan,
    GeometryAuthority,
    PhysicalBounds,
    PhysicalScale,
    Strength,
    ReplayKey,
> {
    /// Return the exact caller-owned plan whose strength was admitted.
    #[must_use]
    pub const fn plan(
        &self,
    ) -> &'plan PageTexturePlan<
        GeometryAuthority,
        PhysicalBounds,
        PhysicalScale,
        Strength,
        ReplayKey,
    > {
        self.plan
    }
}

/// Why a bounded page-texture strength is invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageTexturePlanError {
    /// Declared minimum strength exceeds maximum strength.
    MinimumAboveMaximum,
    /// Selected strength lies outside the declared inclusive envelope.
    SelectedOutsideBounds,
}

impl<GeometryAuthority, PhysicalBounds, PhysicalScale, Strength, ReplayKey>
    PageTexturePlan<
        GeometryAuthority,
        PhysicalBounds,
        PhysicalScale,
        Strength,
        ReplayKey,
    >
where
    Strength: Ord,
{
    /// Validate the bounded texture-strength envelope before generation.
    ///
    /// # Errors
    ///
    /// Returns a typed range error without generating texture or changing
    /// geometry.
    pub fn validate(&self) -> Result<(), PageTexturePlanError> {
        if self.strength.minimum > self.strength.maximum {
            return Err(PageTexturePlanError::MinimumAboveMaximum);
        }
        if self.strength.selected < self.strength.minimum
            || self.strength.selected > self.strength.maximum
        {
            return Err(PageTexturePlanError::SelectedOutsideBounds);
        }
        Ok(())
    }
}

/// Validate and seal one exact bounded page-texture plan.
///
/// # Errors
///
/// Returns the same range failure as [`PageTexturePlan::validate`].
pub fn validate_page_texture_plan_view<
    GeometryAuthority,
    PhysicalBounds,
    PhysicalScale,
    Strength,
    ReplayKey,
>(
    plan: &PageTexturePlan<
        GeometryAuthority,
        PhysicalBounds,
        PhysicalScale,
        Strength,
        ReplayKey,
    >,
) -> Result<
    ValidatedPageTexturePlan<
        '_,
        GeometryAuthority,
        PhysicalBounds,
        PhysicalScale,
        Strength,
        ReplayKey,
    >,
    PageTexturePlanError,
>
where
    Strength: Ord,
{
    plan.validate()?;
    Ok(ValidatedPageTexturePlan { plan })
}
