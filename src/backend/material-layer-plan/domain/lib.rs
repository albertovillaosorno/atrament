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
//   - Transport-neutral material-layer ordering and stochastic replay inputs.
// - Must-Not:
//   - Evaluate material transfer, alter vector geometry, choose seeds, blend
//     equations, texture sampling, color spaces, rasterization, or output
//     format.
// - Allows:
//   - Inputs: Caller-owned geometry authority, layer inputs, and replay keys.
//   - Outputs: Ordered material-layer plans preserving clipping and replay
//     data.
//   - Side effects: None.
// - Split-When:
//   - Material evaluation or raster projection gains executable authority.
// - Merge-When:
//   - Layer-plan structure becomes inseparable from one renderer
//     implementation.
// - Summary:
//   - Keeps deterministic layer composition separate from material evaluation.
// - Description:
//   - Freezes accepted material families and complete stochastic replay inputs.
// - Usage:
//   - Carry material intent from vector authority into later CPU rendering.
// - Defaults:
//   - No layer kind is implicitly required, stochastic, or visually
//     interpreted.
//

//! Ordered deterministic material-layer authority before CPU rendering.

/// Accepted deterministic material-layer families.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MaterialLayerKind {
    /// Base ink deposition clipped to authoritative geometry.
    BaseInk,
    /// Optional color layer.
    Color,
    /// Deposition texture layer.
    DepositionTexture,
    /// Edge-variation layer around authoritative geometry.
    EdgeVariation,
    /// Optional highlight layer.
    Highlight,
    /// Paper-interaction layer.
    PaperInteraction,
}

/// Complete replay inputs required by every stochastic material layer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StochasticLayerReplayKey<
    DocumentSeed,
    MaterialPreset,
    ProfileIdentity,
    SemanticIdentity,
> {
    /// Accepted document seed.
    pub document_seed: DocumentSeed,
    /// Caller-owned material preset identity or value.
    pub material_preset: MaterialPreset,
    /// Accepted handwriting or rendering profile identity.
    pub profile_identity: ProfileIdentity,
    /// Stable semantic object identity that owns the stochastic layer.
    pub semantic_identity: SemanticIdentity,
}

/// Whether one layer is deterministic or requires a complete replay key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaterialLayerRandomness<ReplayKey> {
    /// Layer evaluation has no stochastic input.
    Deterministic,
    /// Layer evaluation derives stochastic behavior from this complete key.
    Stochastic(ReplayKey),
}

/// One material layer clipped to caller-owned authoritative geometry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterialLayer<GeometryAuthority, LayerInputs, ReplayKey> {
    /// Geometry authority that clips this material layer.
    pub clip_geometry: GeometryAuthority,
    /// Caller-owned model inputs for this layer.
    pub inputs: LayerInputs,
    /// Layer family.
    pub kind: MaterialLayerKind,
    /// Explicit deterministic or stochastic replay behavior.
    pub randomness: MaterialLayerRandomness<ReplayKey>,
}

/// Ordered material composition for one caller-owned physical page region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterialLayerPlan<PhysicalBounds, Layer> {
    /// Layers in accepted blend order.
    pub layers: Vec<Layer>,
    /// Caller-owned physical dimensions shared by the composition.
    pub physical_bounds: PhysicalBounds,
}
