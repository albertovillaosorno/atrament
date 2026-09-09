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
//   - Regression evidence for source-linked live line-art authority.
// - Must-Not:
//   - Decode images, choose extraction controls, derive paths, or render
//     output.
// - Allows:
//   - Inputs: Deterministic source, level, and vector-path fixtures.
//   - Outputs: Assertions over transparent black appearance and source linkage.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Extraction algorithms gain independent fixtures.
// - Merge-When:
//   - Line-art evidence moves into an image-placement harness.
// - Summary:
//   - Proves line art stays source-linked and live-compatible by structure.
// - Description:
//   - Covers configurable levels, ordered paths, and fixed appearance.
// - Usage:
//   - Compile directly against the line-art-extraction domain.
// - Defaults:
//   - No path or level is synthesized implicitly.
//
use atrament_line_art_extraction::{
    LineArtAppearance, LineArtExtractionPairError, LineArtExtractionRequest,
    LineArtExtractionResult, validate_line_art_extraction_pair,
};

#[test]
fn request_retains_source_identity_and_caller_owned_levels() {
    let request = LineArtExtractionRequest {
        levels: [2_u8, 5_u8, 9_u8],
        source_identity: "photo-17",
    };
    assert_eq!(request.source_identity, "photo-17");
    assert_eq!(request.levels, [2, 5, 9]);
}

#[test]
fn result_is_explicitly_transparent_black_and_source_linked() {
    let result = LineArtExtractionResult {
        appearance: LineArtAppearance::TransparentBlack,
        levels: 4_u8,
        paths: vec!["path-1", "path-2"],
        source_identity: "photo-17",
    };
    assert_eq!(
        result.appearance,
        LineArtAppearance::TransparentBlack,
    );
    assert_eq!(result.source_identity, "photo-17");
    assert_eq!(result.paths, ["path-1", "path-2"]);
}

#[test]
fn result_preserves_extractor_path_order_and_exact_level_value() {
    let result = LineArtExtractionResult {
        appearance: LineArtAppearance::TransparentBlack,
        levels: "caller-level-config-v3",
        paths: vec![3_u32, 1_u32, 7_u32],
        source_identity: 88_u64,
    };
    assert_eq!(result.levels, "caller-level-config-v3");
    assert_eq!(result.paths, [3, 1, 7]);
    assert_eq!(result.source_identity, 88);
}

#[test]
fn result_must_match_originating_source_and_level_configuration() {
    let request = LineArtExtractionRequest {
        levels: "levels-v2",
        source_identity: "photo-17",
    };
    let matching = LineArtExtractionResult {
        appearance: LineArtAppearance::TransparentBlack,
        levels: "levels-v2",
        paths: vec!["path-1"],
        source_identity: "photo-17",
    };
    assert_eq!(validate_line_art_extraction_pair(&request, &matching), Ok(()));

    let wrong_levels = LineArtExtractionResult {
        levels: "levels-v3",
        ..matching.clone()
    };
    assert_eq!(
        validate_line_art_extraction_pair(&request, &wrong_levels),
        Err(LineArtExtractionPairError::LevelsMismatch),
    );

    let wrong_source = LineArtExtractionResult {
        source_identity: "photo-18",
        ..matching
    };
    assert_eq!(
        validate_line_art_extraction_pair(&request, &wrong_source),
        Err(LineArtExtractionPairError::SourceIdentityMismatch),
    );
}
