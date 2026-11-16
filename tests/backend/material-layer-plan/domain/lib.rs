// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE
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

//! Material layer plan domain regression tests.

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
        document_seed: 17u64,
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
        physical_bounds: (210_000u64, 297_000u64),
    };
    assert_eq!(plan.layers.first().map(|layer| layer.kind),
        Some(MaterialLayerKind::BaseInk));
    assert_eq!(plan.layers.get(1).map(|layer| layer.kind),
        Some(MaterialLayerKind::EdgeVariation));
    assert_eq!(plan.physical_bounds, (210_000, 297_000));
}

fn next_index_permutation(values: &mut [usize]) -> bool {
    let Some((pivot, _)) = values.windows(2).enumerate().rev()
        .find(|(_, pair)| {
        pair.first().zip(pair.get(1))
            .is_some_and(|(left, right)| left < right)
    }) else {
        return false;
    };
    let Some(after_pivot) = pivot.checked_add(1) else {
        return false;
    };
    let Some(&pivot_value) = values.get(pivot) else {
        return false;
    };
    let Some(successor) = (after_pivot..values.len()).rev().find(|&index| {
        values.get(index).is_some_and(|&value| pivot_value < value)
    }) else {
        return false;
    };
    values.swap(pivot, successor);
    values.get_mut(after_pivot..).is_some_and(|suffix| {
        suffix.reverse();
        true
    })
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
    let mut order = [0usize, 1, 2, 3, 4, 5];
    let mut cases = 0u32;
    let mut permutations = 0u16;
    loop {
        permutations = permutations.saturating_add(1);
        for stochastic_mask in 0u8..64 {
            let layers = order
                .iter()
                .map(|&source_index| {
                    let source = u16::try_from(source_index)
                        .unwrap_or(u16::MAX);
                    assert_eq!(usize::from(source), source_index);
                    let kind = kinds.get(source_index).copied();
                    assert!(kind.is_some(), "layer kind absent");
                    let randomness = if stochastic_mask
                        & (1u8 << source_index)
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
                        kind: kind.unwrap_or(MaterialLayerKind::BaseInk),
                        randomness,
                    }
                })
                .collect::<Vec<_>>();
            let plan = MaterialLayerPlan {
                layers,
                physical_bounds: (210_000u32, 297_000u32),
            };
            assert_eq!(plan.physical_bounds, (210_000, 297_000));
            assert_eq!(plan.layers.len(), order.len());
            for (position, &source_index) in order.iter().enumerate() {
                let source = u16::try_from(source_index).unwrap_or(u16::MAX);
                assert_eq!(usize::from(source), source_index);
                let Some(layer) = plan.layers.get(position) else {
                    continue;
                };
                assert_eq!(
                    Some(layer.kind),
                    kinds.get(source_index).copied(),
                    "kind order {order:?} mask {stochastic_mask:#08b}",
                );
                assert_eq!(layer.clip_geometry, 5_000 + source);
                assert_eq!(layer.inputs, 6_000 + source);
                match &layer.randomness {
                    MaterialLayerRandomness::Deterministic => assert_eq!(
                        stochastic_mask & (1u8 << source_index),
                        0,
                        "deterministic source {source_index}",
                    ),
                    MaterialLayerRandomness::Stochastic(key) => {
                        assert_ne!(
                            stochastic_mask & (1u8 << source_index),
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
