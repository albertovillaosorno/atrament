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
//   - Regression evidence for material-layer order and stochastic replay
//     inputs.
// - Must-Not:
//   - Evaluate material transfer, alter geometry, choose seeds, blend, sample,
//     rasterize, or define output formats.
// - Allows:
//   - Inputs: Deterministic caller-owned layer and replay fixtures.
//   - Outputs: Assertions over layer taxonomy, clipping, order, and replay
//     data.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Material evaluation or raster projection gains independent fixtures.
// - Merge-When:
//   - Layer-plan structural evidence moves into another pure renderer harness.
// - Summary:
//   - Proves layer composition remains geometry-clipped and replayable.
// - Description:
//   - Covers material families, blend order, and complete stochastic keys.
// - Usage:
//   - Compile directly against the material-layer-plan domain.
// - Defaults:
//   - No implicit layer requirement or visual behavior is asserted.
//
use atrament_material_layer_plan::{
    MaterialLayer, MaterialLayerKind, MaterialLayerPlan,
    MaterialLayerRandomness,
    StochasticLayerReplayKey,
};

#[test]
fn accepted_material_layer_families_are_explicit() {
    let kinds = [
        MaterialLayerKind::BaseInk,
        MaterialLayerKind::Color,
        MaterialLayerKind::DepositionTexture,
        MaterialLayerKind::EdgeVariation,
        MaterialLayerKind::Highlight,
        MaterialLayerKind::PaperInteraction,
    ];
    assert_eq!(kinds.len(), 6);
}

#[test]
fn stochastic_layer_replay_key_retains_all_accepted_inputs() {
    let key = StochasticLayerReplayKey {
        document_seed: 17_u64,
        material_preset: "ballpoint-a",
        profile_identity: "writer-profile-4",
        semantic_identity: "span-19",
    };
    assert_eq!(key.document_seed, 17);
    assert_eq!(key.semantic_identity, "span-19");
    assert_eq!(key.profile_identity, "writer-profile-4");
    assert_eq!(key.material_preset, "ballpoint-a");
}

#[test]
fn layer_keeps_geometry_clipping_separate_from_material_inputs() {
    let layer = MaterialLayer {
        clip_geometry: "centerline-contour-7",
        inputs: "measured-contact-output",
        kind: MaterialLayerKind::BaseInk,
        randomness: MaterialLayerRandomness::<()>::Deterministic,
    };
    assert_eq!(layer.clip_geometry, "centerline-contour-7");
    assert_eq!(layer.inputs, "measured-contact-output");
}

#[test]
fn plan_preserves_blend_order_and_physical_bounds_without_evaluation() {
    type ReplayKey =
        StochasticLayerReplayKey<u64, &'static str, &'static str, &'static str>;
    let key = ReplayKey {
        document_seed: 3,
        material_preset: "preset-a",
        profile_identity: "profile-a",
        semantic_identity: "span-a",
    };
    let plan = MaterialLayerPlan {
        layers: vec![
            MaterialLayer {
                clip_geometry: "geometry-a",
                inputs: "base-input",
                kind: MaterialLayerKind::BaseInk,
                randomness: MaterialLayerRandomness::Deterministic,
            },
            MaterialLayer {
                clip_geometry: "geometry-a",
                inputs: "edge-input",
                kind: MaterialLayerKind::EdgeVariation,
                randomness: MaterialLayerRandomness::Stochastic(key),
            },
        ],
        physical_bounds: (210_000_u64, 297_000_u64),
    };
    assert_eq!(plan.layers[0].kind, MaterialLayerKind::BaseInk);
    assert_eq!(plan.layers[1].kind, MaterialLayerKind::EdgeVariation);
    assert_eq!(plan.physical_bounds, (210_000, 297_000));
}
