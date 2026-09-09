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
//   - Regression evidence for deterministic preview/final quality differences.
// - Must-Not:
//   - Render pixels, choose quality values, alter material behavior, benchmark,
//     or define output formats.
// - Allows:
//   - Inputs: Deterministic caller-owned authority and quality-cost fixtures.
//   - Outputs: Assertions over roles, shared authority, and allowed
//     differences.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Render scheduling or benchmarking gains independent fixtures.
// - Merge-When:
//   - Quality-pair evidence moves into another pure render harness.
// - Summary:
//   - Proves preview/final changes cannot drift authoritative render inputs.
// - Description:
//   - Covers roles, quality costs, geometry, seed, blend order, and dimensions.
// - Usage:
//   - Compile directly against the render-quality-profile domain.
// - Defaults:
//   - No quality value or performance budget is inferred.
//
use atrament_render_quality_profile::{
    RenderQualityCost, RenderQualityMode, RenderQualityPairError,
    RenderQualityProfile, SharedRenderAuthority, validate_preview_final_pair,
};

type Authority = SharedRenderAuthority<
    &'static str,
    &'static str,
    &'static str,
    u64,
    (u64, u64),
    u64,
>;
type Cost = RenderQualityCost<u32, u32>;

fn authority() -> Authority {
    SharedRenderAuthority {
        blend_order: "accepted-blend-order",
        geometry: "vector-geometry-12",
        material_authority: "calibrated-contact-preset-4",
        noise_scale: 1_200,
        physical_bounds: (210_000, 297_000),
        seed: 17,
    }
}

fn profile(
    mode: RenderQualityMode,
    cost: Cost,
) -> RenderQualityProfile<Authority, Cost> {
    RenderQualityProfile {
        authority: authority(),
        cost,
        mode,
    }
}

#[test]
fn preview_and_final_roles_are_explicit() {
    assert_ne!(RenderQualityMode::Preview, RenderQualityMode::Final);
}

#[test]
fn preview_and_final_may_differ_in_quality_cost_only() {
    let preview = profile(
        RenderQualityMode::Preview,
        RenderQualityCost {
            sampling_cost: 2,
            texture_resolution: 512,
        },
    );
    let final_profile = profile(
        RenderQualityMode::Final,
        RenderQualityCost {
            sampling_cost: 8,
            texture_resolution: 2048,
        },
    );
    assert_eq!(validate_preview_final_pair(&preview, &final_profile), Ok(()));
    assert_ne!(preview.cost, final_profile.cost);
}

#[test]
fn geometry_drift_rejects_preview_final_pair() {
    let preview = profile(
        RenderQualityMode::Preview,
        RenderQualityCost {
            sampling_cost: 2,
            texture_resolution: 512,
        },
    );
    let mut final_profile = profile(
        RenderQualityMode::Final,
        RenderQualityCost {
            sampling_cost: 8,
            texture_resolution: 2048,
        },
    );
    final_profile.authority.geometry = "different-geometry";
    assert_eq!(
        validate_preview_final_pair(&preview, &final_profile),
        Err(RenderQualityPairError::SharedAuthorityMismatch),
    );
}

#[test]
fn material_or_noise_drift_rejects_as_shared_authority_mismatch() {
    let preview = profile(
        RenderQualityMode::Preview,
        RenderQualityCost {
            sampling_cost: 2,
            texture_resolution: 512,
        },
    );
    for authority in [
        SharedRenderAuthority {
            material_authority: "different-contact-preset",
            ..authority()
        },
        SharedRenderAuthority {
            noise_scale: 2_400,
            ..authority()
        },
    ] {
        let final_profile = RenderQualityProfile {
            authority,
            cost: RenderQualityCost {
                sampling_cost: 8,
                texture_resolution: 2048,
            },
            mode: RenderQualityMode::Final,
        };
        assert_eq!(
            validate_preview_final_pair(&preview, &final_profile),
            Err(RenderQualityPairError::SharedAuthorityMismatch),
        );
    }
}

#[test]
fn seed_blend_or_physical_drift_rejects_as_shared_authority_mismatch() {
    let preview = profile(
        RenderQualityMode::Preview,
        RenderQualityCost {
            sampling_cost: 2,
            texture_resolution: 512,
        },
    );
    for authority in [
        SharedRenderAuthority {
            seed: 99,
            ..authority()
        },
        SharedRenderAuthority {
            blend_order: "different-order",
            ..authority()
        },
        SharedRenderAuthority {
            physical_bounds: (216_000, 279_000),
            ..authority()
        },
    ] {
        let final_profile = RenderQualityProfile {
            authority,
            cost: RenderQualityCost {
                sampling_cost: 8,
                texture_resolution: 2048,
            },
            mode: RenderQualityMode::Final,
        };
        assert_eq!(
            validate_preview_final_pair(&preview, &final_profile),
            Err(RenderQualityPairError::SharedAuthorityMismatch),
        );
    }
}

#[test]
fn swapped_quality_roles_reject_before_authority_comparison() {
    let preview = profile(
        RenderQualityMode::Final,
        RenderQualityCost {
            sampling_cost: 2,
            texture_resolution: 512,
        },
    );
    let final_profile = profile(
        RenderQualityMode::Preview,
        RenderQualityCost {
            sampling_cost: 8,
            texture_resolution: 2048,
        },
    );
    assert_eq!(
        validate_preview_final_pair(&preview, &final_profile),
        Err(RenderQualityPairError::PreviewProfileRole),
    );
}
