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
//   - Transport-neutral render-manifest input structure for reproducibility.
// - Must-Not:
//   - Compute render identity, serialize manifests, render output, choose model
//     values, include adapter-local state, or persist files.
// - Allows:
//   - Inputs: Accepted source identities, assets, versions, models, seed,
//     appearance authority, quality profile, and output-affecting options.
//   - Outputs: One inspectable manifest value preserving those exact inputs.
//   - Side effects: None.
// - Split-When:
//   - Render identity computation or manifest serialization gains executable
//     authority.
// - Merge-When:
//   - Manifest inputs become inseparable from one render application service.
// - Summary:
//   - Makes deterministic render inputs inspectable without defining rendering.
// - Description:
//   - Separates accepted source, behavior, and appearance inputs structurally.
// - Usage:
//   - Attach to a successful render result before adapter-specific projection.
// - Defaults:
//   - Asset/model order and caller-owned values are preserved without
//     rewriting.
//

//! Reproducible render-manifest inputs independent of adapter representation.

/// Accepted source identities whose values determine one render projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderSourceInputs<
    AssetIdentity,
    HandwritingProfileIdentity,
    PaperProfileIdentity,
    RevisionIdentity,
    SemanticDocumentIdentity,
> {
    /// Accepted asset identities in caller-owned deterministic order.
    pub asset_identities: Vec<AssetIdentity>,
    /// Accepted handwriting profile identity.
    pub handwriting_profile_identity: HandwritingProfileIdentity,
    /// Accepted paper profile identity.
    pub paper_profile_identity: PaperProfileIdentity,
    /// Exact accepted revision consumed by Render.
    pub revision_identity: RevisionIdentity,
    /// Stable semantic document identity.
    pub semantic_document_identity: SemanticDocumentIdentity,
}

/// Versioned model and variation inputs affecting deterministic render
/// behavior.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderBehaviorInputs<EngineVersion, ModelChoice, VariationSeed> {
    /// Accepted render/layout engine behavior version.
    pub engine_version: EngineVersion,
    /// Caller-owned admitted model choices in deterministic order.
    pub model_choices: Vec<ModelChoice>,
    /// Accepted document variation seed.
    pub variation_seed: VariationSeed,
}

/// Appearance and quality inputs affecting deterministic render output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderAppearanceInputs<
    MaterialAuthority,
    QualityProfile,
    RenderOptions,
    RendererVersion,
> {
    /// Material presets, blend order, and appearance behavior authority.
    pub material_authority: MaterialAuthority,
    /// Declared preview or final quality profile.
    pub quality_profile: QualityProfile,
    /// Caller-owned output-affecting render options.
    pub render_options: RenderOptions,
    /// Renderer behavior version.
    pub renderer_version: RendererVersion,
}

/// Complete inspectable deterministic inputs associated with one render result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderManifest<SourceInputs, BehaviorInputs, AppearanceInputs> {
    /// Appearance, quality, and renderer behavior inputs.
    pub appearance: AppearanceInputs,
    /// Engine/model/variation behavior inputs.
    pub behavior: BehaviorInputs,
    /// Accepted source and asset identities.
    pub source: SourceInputs,
}
