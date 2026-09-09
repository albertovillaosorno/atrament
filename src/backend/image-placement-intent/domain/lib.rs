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
//   - Source-preserving image placement and layering intent.
// - Must-Not:
//   - Own image bytes, choose coordinate units, crop math, opacity ranges,
//     z-order algorithms, color conversion, line-art extraction, or rendering.
// - Allows:
//   - Inputs: Caller-owned source identity, geometry, appearance, placement,
//     resolution, and constraint values.
//   - Outputs: One inspectable placed-image intent.
//   - Side effects: None.
// - Split-When:
//   - Image decoding, transform evaluation, or derived line-art gains
//     independent executable authority.
// - Merge-When:
//   - Image placement becomes inseparable from semantic figure authority.
// - Summary:
//   - Keeps image placement editable without overwriting original source.
// - Description:
//   - Freezes four placement modes plus caller-owned placement properties.
// - Usage:
//   - Carry accepted source identity into later layout and rendering.
// - Defaults:
//   - No placement mode, geometry, opacity, or z-order value is inferred.
//

//! Source-preserving image placement intent before layout and rendering.

/// First-release placed-image relationship to surrounding text.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ImagePlacementMode {
    /// Image is layered above text.
    AboveText,
    /// Image is layered below text.
    BelowText,
    /// Image is constrained inside an explicit clipped region.
    ClippedRegion,
    /// Image participates inline with semantic flow.
    Inline,
}

/// Caller-owned spatial controls for one placed image.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImagePlacementGeometry<Position, Size, Transform> {
    /// Caller-owned image position.
    pub position: Position,
    /// Caller-owned placed image size.
    pub size: Size,
    /// Caller-owned additional transform intent.
    pub transform: Transform,
}

/// Complete source-preserving placed-image intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImagePlacementIntent<
    ColorHandling,
    Crop,
    Geometry,
    Opacity,
    PlacementConstraints,
    ResolutionPolicy,
    SourceIdentity,
    ZOrder,
> {
    /// Caller-owned color handling policy or identity.
    pub color_handling: ColorHandling,
    /// Caller-owned non-destructive crop intent.
    pub crop: Crop,
    /// Position, size, and transform intent.
    pub geometry: Geometry,
    /// Caller-owned opacity value.
    pub opacity: Opacity,
    /// Caller-owned placement constraints.
    pub placement_constraints: PlacementConstraints,
    /// Below-text, inline, above-text, or clipped-region relationship.
    pub placement_mode: ImagePlacementMode,
    /// Caller-owned image resolution policy.
    pub resolution_policy: ResolutionPolicy,
    /// Stable identity of the original session asset.
    pub source_identity: SourceIdentity,
    /// Caller-owned layer ordering value.
    pub z_order: ZOrder,
}
