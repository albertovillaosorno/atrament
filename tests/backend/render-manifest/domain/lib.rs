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
//   - Regression evidence for deterministic render-manifest input retention.
// - Must-Not:
//   - Compute identities, serialize files, render output, choose model values,
//     or include adapter-local presentation state.
// - Allows:
//   - Inputs: Deterministic caller-owned render-manifest fixtures.
//   - Outputs: Assertions over source, behavior, and appearance input
//     retention.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Render identity or serialization gains independent executable fixtures.
// - Merge-When:
//   - Manifest evidence moves into a render-application harness.
// - Summary:
//   - Proves compact reproducibility inputs remain exact and inspectable.
// - Description:
//   - Covers accepted identities, assets, versions, models, seed, and quality.
// - Usage:
//   - Compile directly against the render-manifest domain.
// - Defaults:
//   - Caller-owned sequence order is preserved exactly.
//
use atrament_render_manifest::{
    RenderAppearanceInputs, RenderBehaviorInputs, RenderManifest,
    RenderSourceInputs,
};

#[test]
fn render_manifest_retains_all_frozen_reproducibility_inputs() {
    let manifest = RenderManifest {
        appearance: RenderAppearanceInputs {
            material_authority: "materials-v4/blend-a",
            quality_profile: "preview",
            render_options: "option-set-2",
            renderer_version: 9_u32,
        },
        behavior: RenderBehaviorInputs {
            engine_version: 12_u32,
            model_choices: vec!["spacing-model-a", "contact-model-b"],
            variation_seed: 44_u64,
        },
        source: RenderSourceInputs {
            asset_identities: vec!["asset-2", "asset-7"],
            handwriting_profile_identity: "writer-profile-3",
            paper_profile_identity: "paper-a4-grid",
            revision_identity: "revision-19",
            semantic_document_identity: "notebook-5",
        },
    };

    assert_eq!(manifest.source.revision_identity, "revision-19");
    assert_eq!(manifest.source.semantic_document_identity, "notebook-5");
    assert_eq!(
        manifest.source.handwriting_profile_identity,
        "writer-profile-3",
    );
    assert_eq!(manifest.source.paper_profile_identity, "paper-a4-grid");
    assert_eq!(manifest.source.asset_identities, ["asset-2", "asset-7"]);
    assert_eq!(manifest.behavior.engine_version, 12);
    assert_eq!(
        manifest.behavior.model_choices,
        ["spacing-model-a", "contact-model-b"],
    );
    assert_eq!(manifest.behavior.variation_seed, 44);
    assert_eq!(manifest.appearance.material_authority, "materials-v4/blend-a");
    assert_eq!(manifest.appearance.renderer_version, 9);
    assert_eq!(manifest.appearance.quality_profile, "preview");
    assert_eq!(manifest.appearance.render_options, "option-set-2");
}

#[test]
fn manifest_preserves_caller_owned_asset_and_model_order() {
    let source = RenderSourceInputs {
        asset_identities: vec![3_u8, 1_u8, 2_u8],
        handwriting_profile_identity: 8_u8,
        paper_profile_identity: 9_u8,
        revision_identity: 10_u8,
        semantic_document_identity: 11_u8,
    };
    let behavior = RenderBehaviorInputs {
        engine_version: 4_u8,
        model_choices: vec![7_u8, 5_u8, 6_u8],
        variation_seed: 12_u8,
    };
    assert_eq!(source.asset_identities, [3, 1, 2]);
    assert_eq!(behavior.model_choices, [7, 5, 6]);
}

fn next_index_permutation(values: &mut [u8]) -> bool {
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
fn every_asset_and_model_order_preserves_complete_manifest_inputs() {
    let mut asset_order = [0_u8, 1, 2, 3, 4];
    let mut asset_permutations = 0_u8;
    let mut cases = 0_u16;
    loop {
        asset_permutations = asset_permutations.saturating_add(1);
        let mut model_order = [0_u8, 1, 2, 3];
        loop {
            let manifest = RenderManifest {
                appearance: RenderAppearanceInputs {
                    material_authority: 21_u8,
                    quality_profile: 22_u8,
                    render_options: 23_u8,
                    renderer_version: 24_u8,
                },
                behavior: RenderBehaviorInputs {
                    engine_version: 31_u8,
                    model_choices: model_order.to_vec(),
                    variation_seed: 32_u8,
                },
                source: RenderSourceInputs {
                    asset_identities: asset_order.to_vec(),
                    handwriting_profile_identity: 41_u8,
                    paper_profile_identity: 42_u8,
                    revision_identity: 43_u8,
                    semantic_document_identity: 44_u8,
                },
            };
            assert_eq!(manifest.source.asset_identities, asset_order);
            assert_eq!(manifest.behavior.model_choices, model_order);
            assert_eq!(manifest.source.handwriting_profile_identity, 41);
            assert_eq!(manifest.source.paper_profile_identity, 42);
            assert_eq!(manifest.source.revision_identity, 43);
            assert_eq!(manifest.source.semantic_document_identity, 44);
            assert_eq!(manifest.behavior.engine_version, 31);
            assert_eq!(manifest.behavior.variation_seed, 32);
            assert_eq!(manifest.appearance.material_authority, 21);
            assert_eq!(manifest.appearance.quality_profile, 22);
            assert_eq!(manifest.appearance.render_options, 23);
            assert_eq!(manifest.appearance.renderer_version, 24);
            cases = cases.saturating_add(1);
            if !next_index_permutation(&mut model_order) {
                break;
            }
        }
        if !next_index_permutation(&mut asset_order) {
            break;
        }
    }
    assert_eq!(asset_permutations, 120);
    assert_eq!(cases, 2_880);
}
