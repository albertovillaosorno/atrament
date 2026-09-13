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
//   - Decode images, choose control units/ranges/semantics, derive paths,
//     rasterize, render, or mutate source assets.
// - Allows:
//   - Inputs: Caller-owned source identity and configurable extraction
//     controls.
//   - Outputs: Source-linked transparent black vector-path evidence.
//   - Side effects: None.
// - Split-When:
//   - Extraction-control semantics or algorithms gain executable authority.
// - Merge-When:
//   - Line-art extraction becomes inseparable from image-placement authority.
// - Summary:
//   - Freezes the live-compatible transparent black path boundary.
// - Description:
//   - Retains source identity and exact extraction controls without choosing an
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

/// Exact caller-owned controls for one line-art extraction request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineArtExtractionControls<
    Cleanup,
    Detail,
    Levels,
    MinimumFeature,
    Preview,
    Threshold,
> {
    /// Caller-owned cleanup control.
    pub cleanup: Cleanup,
    /// Caller-owned detail control.
    pub detail: Detail,
    /// Caller-owned configurable extraction levels.
    pub levels: Levels,
    /// Caller-owned minimum-feature control.
    pub minimum_feature: MinimumFeature,
    /// Caller-owned preview control.
    pub preview: Preview,
    /// Caller-owned threshold control.
    pub threshold: Threshold,
}

/// Constructor-sealed evidence that one extracted result belongs to the exact
/// originating request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedLineArtExtractionPair<
    'request,
    'result,
    Controls,
    Path,
    SourceIdentity,
> {
    request: &'request LineArtExtractionRequest<Controls, SourceIdentity>,
    result: &'result LineArtExtractionResult<Controls, Path, SourceIdentity>,
}

impl<'request, 'result, Controls, Path, SourceIdentity>
    ValidatedLineArtExtractionPair<
        'request,
        'result,
        Controls,
        Path,
        SourceIdentity,
    >
{
    /// Return the exact request that admitted this result.
    #[must_use]
    pub const fn request(
        &self,
    ) -> &'request LineArtExtractionRequest<Controls, SourceIdentity> {
        self.request
    }

    /// Return the exact result linked to the admitted request.
    #[must_use]
    pub const fn result(
        &self,
    ) -> &'result LineArtExtractionResult<Controls, Path, SourceIdentity> {
        self.result
    }
}

/// Why one extracted result does not belong to its originating request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineArtExtractionPairError {
    /// Result reports a different extraction-control configuration.
    ControlsMismatch,
    /// Result reports a different source image identity.
    SourceIdentityMismatch,
}

/// One source-linked line-art extraction request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineArtExtractionRequest<Controls, SourceIdentity> {
    /// Exact caller-owned extraction controls.
    pub controls: Controls,
    /// Stable identity of the original source image.
    pub source_identity: SourceIdentity,
}

/// One inspectable line-art result produced by a separate extractor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineArtExtractionResult<Controls, Path, SourceIdentity> {
    /// Accepted live-compatible appearance.
    pub appearance: LineArtAppearance,
    /// Exact caller-owned controls used for this extraction.
    pub controls: Controls,
    /// Ordered caller-produced vector paths.
    pub paths: Vec<Path>,
    /// Original source identity retained without overwriting the asset.
    pub source_identity: SourceIdentity,
}

/// Validate that one line-art result belongs to the exact originating request.
///
/// # Errors
///
/// Returns a typed mismatch when source identity or extraction controls drift.
pub fn validate_line_art_extraction_pair<Controls, Path, SourceIdentity>(
    request: &LineArtExtractionRequest<Controls, SourceIdentity>,
    result: &LineArtExtractionResult<Controls, Path, SourceIdentity>,
) -> Result<(), LineArtExtractionPairError>
where
    Controls: PartialEq,
    SourceIdentity: PartialEq,
{
    if request.controls != result.controls {
        return Err(LineArtExtractionPairError::ControlsMismatch);
    }
    if request.source_identity != result.source_identity {
        return Err(LineArtExtractionPairError::SourceIdentityMismatch);
    }
    Ok(())
}

/// Validate and seal one exact request/result extraction pair.
///
/// # Errors
///
/// Returns the same control-first mismatch as
/// [`validate_line_art_extraction_pair`].
pub fn validate_line_art_extraction_pair_view<
    'request,
    'result,
    Controls,
    Path,
    SourceIdentity,
>(
    request: &'request LineArtExtractionRequest<Controls, SourceIdentity>,
    result: &'result LineArtExtractionResult<Controls, Path, SourceIdentity>,
) -> Result<
    ValidatedLineArtExtractionPair<
        'request,
        'result,
        Controls,
        Path,
        SourceIdentity,
    >,
    LineArtExtractionPairError,
>
where
    Controls: PartialEq,
    SourceIdentity: PartialEq,
{
    validate_line_art_extraction_pair(request, result)?;
    Ok(ValidatedLineArtExtractionPair { request, result })
}
