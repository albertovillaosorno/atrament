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
    validate_preview_final_pair_view,
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

#[test]
fn wrong_final_role_rejects_before_authority_comparison() {
    let preview = profile(
        RenderQualityMode::Preview,
        RenderQualityCost {
            sampling_cost: 2,
            texture_resolution: 512,
        },
    );
    let mut final_profile = profile(
        RenderQualityMode::Preview,
        RenderQualityCost {
            sampling_cost: 8,
            texture_resolution: 2048,
        },
    );
    final_profile.authority.geometry = "different-geometry";
    assert_eq!(
        validate_preview_final_pair(&preview, &final_profile),
        Err(RenderQualityPairError::FinalProfileRole),
    );
}


#[test]
fn all_256_role_and_authority_drift_masks_match_pair_precedence() {
    let mut cases = 0_u16;
    let mut outcomes = [false; 4];
    for mask in 0_u16..256 {
        let preview_role_wrong = mask & (1 << 0) != 0;
        let final_role_wrong = mask & (1 << 1) != 0;
        let mut preview = profile(
            if preview_role_wrong {
                RenderQualityMode::Final
            } else {
                RenderQualityMode::Preview
            },
            RenderQualityCost {
                sampling_cost: 1,
                texture_resolution: 256,
            },
        );
        let mut final_profile = profile(
            if final_role_wrong {
                RenderQualityMode::Preview
            } else {
                RenderQualityMode::Final
            },
            RenderQualityCost {
                sampling_cost: 99,
                texture_resolution: 4096,
            },
        );
        if mask & (1 << 2) != 0 {
            final_profile.authority.blend_order = "drifted-blend-order";
        }
        if mask & (1 << 3) != 0 {
            final_profile.authority.geometry = "drifted-geometry";
        }
        if mask & (1 << 4) != 0 {
            final_profile.authority.material_authority = "drifted-material";
        }
        if mask & (1 << 5) != 0 {
            final_profile.authority.noise_scale = 9_999;
        }
        if mask & (1 << 6) != 0 {
            final_profile.authority.physical_bounds = (1, 2);
        }
        if mask & (1 << 7) != 0 {
            final_profile.authority.seed = 123_456;
        }
        preview.cost.sampling_cost = 2;
        let authority_drift = mask & 0b1111_1100 != 0;
        let expected = if preview_role_wrong {
            outcomes[0] = true;
            Err(RenderQualityPairError::PreviewProfileRole)
        } else if final_role_wrong {
            outcomes[1] = true;
            Err(RenderQualityPairError::FinalProfileRole)
        } else if authority_drift {
            outcomes[2] = true;
            Err(RenderQualityPairError::SharedAuthorityMismatch)
        } else {
            outcomes[3] = true;
            Ok(())
        };
        assert_eq!(
            validate_preview_final_pair(&preview, &final_profile),
            expected,
            "role/authority drift mask {mask:#010b}",
        );
        match expected {
            Ok(()) => {
                let validated = validate_preview_final_pair_view(
                    &preview,
                    &final_profile,
                )
                .expect("matching pair seals");
                assert!(std::ptr::eq(validated.preview(), &preview));
                assert!(std::ptr::eq(
                    validated.final_profile(),
                    &final_profile,
                ));
            },
            Err(reason) => assert_eq!(
                validate_preview_final_pair_view(&preview, &final_profile),
                Err(reason),
                "sealed role/authority drift mask {mask:#010b}",
            ),
        }
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 256);
    assert!(outcomes.into_iter().all(|seen| seen));
}
