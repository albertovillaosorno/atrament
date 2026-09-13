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

fn next_index_permutation(values: &mut [usize]) -> bool {
    let Some(pivot) = (0..values.len().saturating_sub(1))
        .rev()
        .find(|&index| values[index] < values[index + 1])
    else {
        return false;
    };
    let successor = (pivot + 1..values.len())
        .rev()
        .find(|&index| values[pivot] < values[index])
        .expect("nonterminal permutation has a successor");
    values.swap(pivot, successor);
    values[pivot + 1..].reverse();
    true
}

#[test]
fn every_layer_order_and_randomness_mask_preserves_exact_authority() {
    type ReplayKey = StochasticLayerReplayKey<u16, u16, u16, u16>;
    type Layer = MaterialLayer<u16, u16, ReplayKey>;
    let kinds = [
        MaterialLayerKind::BaseInk,
        MaterialLayerKind::Color,
        MaterialLayerKind::DepositionTexture,
        MaterialLayerKind::EdgeVariation,
        MaterialLayerKind::Highlight,
        MaterialLayerKind::PaperInteraction,
    ];
    let mut order = [0_usize, 1, 2, 3, 4, 5];
    let mut cases = 0_u32;
    let mut permutations = 0_u16;
    loop {
        permutations = permutations.saturating_add(1);
        for stochastic_mask in 0_u8..64 {
            let layers = order
                .iter()
                .map(|&source_index| {
                    let source = source_index as u16;
                    let randomness = if stochastic_mask
                        & (1_u8 << source_index)
                        == 0
                    {
                        MaterialLayerRandomness::Deterministic
                    } else {
                        MaterialLayerRandomness::Stochastic(ReplayKey {
                            document_seed: 1_000 + source,
                            material_preset: 2_000 + source,
                            profile_identity: 3_000 + source,
                            semantic_identity: 4_000 + source,
                        })
                    };
                    Layer {
                        clip_geometry: 5_000 + source,
                        inputs: 6_000 + source,
                        kind: kinds[source_index],
                        randomness,
                    }
                })
                .collect::<Vec<_>>();
            let plan = MaterialLayerPlan {
                layers,
                physical_bounds: (210_000_u32, 297_000_u32),
            };
            assert_eq!(plan.physical_bounds, (210_000, 297_000));
            for (position, &source_index) in order.iter().enumerate() {
                let source = source_index as u16;
                let layer = &plan.layers[position];
                assert_eq!(
                    layer.kind,
                    kinds[source_index],
                    "kind order {order:?} mask {stochastic_mask:#08b}",
                );
                assert_eq!(layer.clip_geometry, 5_000 + source);
                assert_eq!(layer.inputs, 6_000 + source);
                match &layer.randomness {
                    MaterialLayerRandomness::Deterministic => assert_eq!(
                        stochastic_mask & (1_u8 << source_index),
                        0,
                        "deterministic source {source_index}",
                    ),
                    MaterialLayerRandomness::Stochastic(key) => {
                        assert_ne!(
                            stochastic_mask & (1_u8 << source_index),
                            0,
                            "stochastic source {source_index}",
                        );
                        assert_eq!(key.document_seed, 1_000 + source);
                        assert_eq!(key.material_preset, 2_000 + source);
                        assert_eq!(key.profile_identity, 3_000 + source);
                        assert_eq!(key.semantic_identity, 4_000 + source);
                    },
                }
            }
            cases = cases.saturating_add(1);
        }
        if !next_index_permutation(&mut order) {
            break;
        }
    }
    assert_eq!(permutations, 720);
    assert_eq!(cases, 46_080);
}
