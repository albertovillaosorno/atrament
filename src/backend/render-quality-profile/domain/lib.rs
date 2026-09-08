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
//   - Preview/final quality-role identity and shared render-authority
//     invariants.
// - Must-Not:
//   - Render pixels, choose quality values, alter geometry, seeds, blend order,
//     physical dimensions, material semantics, or performance budgets.
// - Allows:
//   - Inputs: Caller-owned shared render authority and quality-cost values.
//   - Outputs: Typed preview/final compatibility validation.
//   - Side effects: None.
// - Split-When:
//   - Render scheduling or CPU benchmarking gains executable authority.
// - Merge-When:
//   - Quality validation becomes inseparable from one render application.
// - Summary:
//   - Restricts preview/final differences to declared quality-cost inputs.
// - Description:
//   - Protects geometry, seed, blend order, and physical dimensions from drift.
// - Usage:
//   - Validate paired quality profiles before comparing render projections.
// - Defaults:
//   - No texture resolution or sampling-cost value is chosen here.
//

//! Preview/final quality-profile invariants for deterministic CPU rendering.

/// Accepted deterministic CPU render-quality roles.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RenderQualityMode {
    /// Final-quality rendering.
    Final,
    /// Lower-cost preview rendering.
    Preview,
}

/// Render authority that preview and final profiles must share exactly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SharedRenderAuthority<BlendOrder, Geometry, PhysicalBounds, Seed> {
    /// Accepted material blend order.
    pub blend_order: BlendOrder,
    /// Authoritative vector geometry identity or value.
    pub geometry: Geometry,
    /// Caller-owned physical page dimensions or bounds.
    pub physical_bounds: PhysicalBounds,
    /// Accepted deterministic render seed input.
    pub seed: Seed,
}

/// Quality-only values that may legitimately differ between preview and final.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderQualityCost<SamplingCost, TextureResolution> {
    /// Caller-owned sampling-cost or density choice.
    pub sampling_cost: SamplingCost,
    /// Caller-owned texture-resolution choice.
    pub texture_resolution: TextureResolution,
}

/// One preview or final profile over shared deterministic render authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderQualityProfile<Authority, Cost> {
    /// Geometry/seed/blend/physical authority shared across quality modes.
    pub authority: Authority,
    /// Quality-only cost inputs.
    pub cost: Cost,
    /// Preview or final profile role.
    pub mode: RenderQualityMode,
}

/// Why a preview/final profile pair violates deterministic render authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderQualityPairError {
    /// The supplied final profile is not marked Final.
    FinalProfileRole,
    /// The supplied preview profile is not marked Preview.
    PreviewProfileRole,
    /// Geometry, seed, blend order, or physical dimensions differ.
    SharedAuthorityMismatch,
}

/// Validate that preview/final profiles differ only in quality-cost values.
///
/// # Errors
///
/// Returns a typed role or shared-authority mismatch before rendering.
pub fn validate_preview_final_pair<Authority, PreviewCost, FinalCost>(
    preview: &RenderQualityProfile<Authority, PreviewCost>,
    final_profile: &RenderQualityProfile<Authority, FinalCost>,
) -> Result<(), RenderQualityPairError>
where
    Authority: PartialEq,
{
    if preview.mode != RenderQualityMode::Preview {
        return Err(RenderQualityPairError::PreviewProfileRole);
    }
    if final_profile.mode != RenderQualityMode::Final {
        return Err(RenderQualityPairError::FinalProfileRole);
    }
    if preview.authority != final_profile.authority {
        return Err(RenderQualityPairError::SharedAuthorityMismatch);
    }
    Ok(())
}
