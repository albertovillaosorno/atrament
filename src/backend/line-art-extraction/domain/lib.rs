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
//   - Transport-neutral line-art extraction request/result authority.
// - Must-Not:
//   - Decode images, choose thresholds, cleanup/detail/min-feature semantics,
//     derive paths, rasterize, render, or mutate source assets.
// - Allows:
//   - Inputs: Caller-owned source identity and configurable extraction levels.
//   - Outputs: Source-linked transparent black vector-path evidence.
//   - Side effects: None.
// - Split-When:
//   - Extraction controls or algorithms gain independent executable authority.
// - Merge-When:
//   - Line-art extraction becomes inseparable from image-placement authority.
// - Summary:
//   - Freezes the live-compatible transparent black path boundary.
// - Description:
//   - Retains source identity and extraction levels without choosing an
//     algorithm.
// - Usage:
//   - Carry admitted line-art results into later vector/live projections.
// - Defaults:
//   - No level count or path geometry is inferred.
//

//! Line-art extraction authority before decoding, vectorization, or rendering.

/// Accepted first-release line-art appearance for live compatibility.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LineArtAppearance {
    /// Transparent background with single black path color.
    TransparentBlack,
}

/// Why one extracted result does not belong to its originating request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineArtExtractionPairError {
    /// Result reports a different extraction-level configuration.
    LevelsMismatch,
    /// Result reports a different source image identity.
    SourceIdentityMismatch,
}

/// One source-linked line-art extraction request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineArtExtractionRequest<Levels, SourceIdentity> {
    /// Caller-owned configurable extraction levels.
    pub levels: Levels,
    /// Stable identity of the original source image.
    pub source_identity: SourceIdentity,
}

/// One inspectable line-art result produced by a separate extractor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineArtExtractionResult<Levels, Path, SourceIdentity> {
    /// Accepted live-compatible appearance.
    pub appearance: LineArtAppearance,
    /// Exact caller-owned levels used for this extraction.
    pub levels: Levels,
    /// Ordered caller-produced vector paths.
    pub paths: Vec<Path>,
    /// Original source identity retained without overwriting the asset.
    pub source_identity: SourceIdentity,
}

/// Validate that one line-art result belongs to the exact originating request.
///
/// # Errors
///
/// Returns a typed mismatch when source identity or level configuration drifts.
pub fn validate_line_art_extraction_pair<Levels, Path, SourceIdentity>(
    request: &LineArtExtractionRequest<Levels, SourceIdentity>,
    result: &LineArtExtractionResult<Levels, Path, SourceIdentity>,
) -> Result<(), LineArtExtractionPairError>
where
    Levels: PartialEq,
    SourceIdentity: PartialEq,
{
    if request.levels != result.levels {
        return Err(LineArtExtractionPairError::LevelsMismatch);
    }
    if request.source_identity != result.source_identity {
        return Err(LineArtExtractionPairError::SourceIdentityMismatch);
    }
    Ok(())
}
