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
