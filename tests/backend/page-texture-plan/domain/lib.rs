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
//   - Regression evidence for bounded, replayable page-texture intent.
// - Must-Not:
//   - Generate noise, mutate geometry, choose units, visual thresholds, tiling,
//     rasterization, or output behavior.
// - Allows:
//   - Inputs: Deterministic caller-owned texture-plan fixtures.
//   - Outputs: Assertions over bounds, geometry identity, scale, and replay
//     key.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Texture generation or perceptual validation gains independent fixtures.
// - Merge-When:
//   - Texture-plan validation moves into another pure render harness.
// - Summary:
//   - Proves texture intent stays bounded, replayable, and geometry-preserving.
// - Description:
//   - Covers strength range validation plus all retained authority inputs.
// - Usage:
//   - Compile directly against the page-texture-plan domain.
// - Defaults:
//   - No visual interpretation is inferred from caller-owned values.
//
use atrament_page_texture_plan::{
    BoundedTextureStrength, PageTexturePlan, PageTexturePlanError,
    StochasticLayerReplayKey,
};

type ReplayKey = StochasticLayerReplayKey<
    u64,
    &'static str,
    &'static str,
    &'static str,
>;
type Plan = PageTexturePlan<
    &'static str,
    (u64, u64),
    &'static str,
    u16,
    ReplayKey,
>;

fn valid_plan() -> Plan {
    PageTexturePlan {
        geometry_authority: "vector-page-9",
        physical_bounds: (210_000, 297_000),
        physical_scale: "caller-owned-low-frequency-scale",
        replay_key: ReplayKey {
            document_seed: 17,
            material_preset: "paper-preset-a",
            profile_identity: "profile-4",
            semantic_identity: "page-9",
        },
        strength: BoundedTextureStrength {
            maximum: 20,
            minimum: 4,
            selected: 9,
        },
    }
}

#[test]
fn texture_plan_retains_geometry_scale_bounds_and_complete_replay_key() {
    let plan = valid_plan();
    assert_eq!(plan.validate(), Ok(()));
    assert_eq!(plan.geometry_authority, "vector-page-9");
    assert_eq!(plan.physical_bounds, (210_000, 297_000));
    assert_eq!(plan.physical_scale, "caller-owned-low-frequency-scale");
    assert_eq!(plan.replay_key.document_seed, 17);
    assert_eq!(plan.replay_key.semantic_identity, "page-9");
    assert_eq!(plan.replay_key.profile_identity, "profile-4");
    assert_eq!(plan.replay_key.material_preset, "paper-preset-a");
}

#[test]
fn inclusive_strength_boundaries_are_valid() {
    let mut plan = valid_plan();
    plan.strength.selected = plan.strength.minimum;
    assert_eq!(plan.validate(), Ok(()));
    plan.strength.selected = plan.strength.maximum;
    assert_eq!(plan.validate(), Ok(()));
}

#[test]
fn selected_strength_outside_bounds_rejects() {
    let mut plan = valid_plan();
    plan.strength.selected = 3;
    assert_eq!(
        plan.validate(),
        Err(PageTexturePlanError::SelectedOutsideBounds),
    );
    plan.strength.selected = 21;
    assert_eq!(
        plan.validate(),
        Err(PageTexturePlanError::SelectedOutsideBounds),
    );
}

#[test]
fn inverted_strength_envelope_rejects_before_selected_value() {
    let mut plan = valid_plan();
    plan.strength.minimum = 30;
    plan.strength.maximum = 20;
    plan.strength.selected = 25;
    assert_eq!(
        plan.validate(),
        Err(PageTexturePlanError::MinimumAboveMaximum),
    );
}
