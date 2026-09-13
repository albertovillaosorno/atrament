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
//   - Decode images, choose control semantics, derive paths, or render output.
// - Allows:
//   - Inputs: Deterministic source, control, and vector-path fixtures.
//   - Outputs: Assertions over exact controls, fixed appearance, and source
//     linkage.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Extraction algorithms gain independent fixtures.
// - Merge-When:
//   - Line-art evidence moves into an image-placement harness.
// - Summary:
//   - Proves line art stays source-linked and live-compatible by structure.
// - Description:
//   - Covers every configurable control, ordered paths, and fixed appearance.
// - Usage:
//   - Compile directly against the line-art-extraction domain.
// - Defaults:
//   - No control value or path is synthesized implicitly.
//
use atrament_line_art_extraction::{
    LineArtAppearance, LineArtExtractionControls, LineArtExtractionPairError,
    LineArtExtractionRequest, LineArtExtractionResult,
    validate_line_art_extraction_pair, validate_line_art_extraction_pair_view,
};

type Controls = LineArtExtractionControls<u8, u8, u8, u8, u8, u8>;

fn controls() -> Controls {
    LineArtExtractionControls {
        cleanup: 3,
        detail: 4,
        levels: 5,
        minimum_feature: 6,
        preview: 7,
        threshold: 8,
    }
}

fn request() -> LineArtExtractionRequest<Controls, &'static str> {
    LineArtExtractionRequest {
        controls: controls(),
        source_identity: "photo-17",
    }
}

fn result() -> LineArtExtractionResult<Controls, &'static str, &'static str> {
    LineArtExtractionResult {
        appearance: LineArtAppearance::TransparentBlack,
        controls: controls(),
        paths: vec!["path-1", "path-2"],
        source_identity: "photo-17",
    }
}

#[test]
fn request_retains_source_identity_and_every_caller_owned_control() {
    let request = request();
    assert_eq!(request.source_identity, "photo-17");
    assert_eq!(request.controls.cleanup, 3);
    assert_eq!(request.controls.detail, 4);
    assert_eq!(request.controls.levels, 5);
    assert_eq!(request.controls.minimum_feature, 6);
    assert_eq!(request.controls.preview, 7);
    assert_eq!(request.controls.threshold, 8);
}

#[test]
fn result_is_transparent_black_source_linked_and_order_preserving() {
    let result = result();
    assert_eq!(result.appearance, LineArtAppearance::TransparentBlack);
    assert_eq!(result.source_identity, "photo-17");
    assert_eq!(result.paths, ["path-1", "path-2"]);
    assert_eq!(result.controls, controls());
}

#[test]
fn every_extraction_control_is_part_of_exact_request_result_identity() {
    for changed_index in 0..6 {
        let request = request();
        let mut result = result();
        match changed_index {
            0 => result.controls.cleanup += 1,
            1 => result.controls.detail += 1,
            2 => result.controls.levels += 1,
            3 => result.controls.minimum_feature += 1,
            4 => result.controls.preview += 1,
            5 => result.controls.threshold += 1,
            _ => unreachable!("six controls are enumerated"),
        }
        assert_eq!(
            validate_line_art_extraction_pair(&request, &result),
            Err(LineArtExtractionPairError::ControlsMismatch),
            "changed control index {changed_index}",
        );
    }
}

#[test]
fn control_drift_rejects_before_source_identity_drift() {
    let request = request();
    let mut result = result();
    result.controls.threshold += 1;
    result.source_identity = "photo-18";
    assert_eq!(
        validate_line_art_extraction_pair(&request, &result),
        Err(LineArtExtractionPairError::ControlsMismatch),
    );
}

#[test]
fn source_identity_drift_rejects_when_controls_match_exactly() {
    let request = request();
    let mut result = result();
    result.source_identity = "photo-18";
    assert_eq!(
        validate_line_art_extraction_pair(&request, &result),
        Err(LineArtExtractionPairError::SourceIdentityMismatch),
    );
}

#[test]
fn exact_request_result_pair_is_valid_without_interpreting_controls() {
    assert_eq!(
        validate_line_art_extraction_pair(&request(), &result()),
        Ok(()),
    );
}

#[test]
fn every_control_and_source_drift_mask_matches_exact_pair_oracle() {
    let mut cases = 0_u16;
    let mut saw = [false; 3];
    for control_mask in 0_u8..64 {
        for source_drift in [false, true] {
            let request = request();
            let mut result = result();
            if control_mask & 0b00_0001 != 0 {
                result.controls.cleanup += 1;
            }
            if control_mask & 0b00_0010 != 0 {
                result.controls.detail += 1;
            }
            if control_mask & 0b00_0100 != 0 {
                result.controls.levels += 1;
            }
            if control_mask & 0b00_1000 != 0 {
                result.controls.minimum_feature += 1;
            }
            if control_mask & 0b01_0000 != 0 {
                result.controls.preview += 1;
            }
            if control_mask & 0b10_0000 != 0 {
                result.controls.threshold += 1;
            }
            if source_drift {
                result.source_identity = "photo-18";
            }

            let expected = if control_mask != 0 {
                saw[0] = true;
                Err(LineArtExtractionPairError::ControlsMismatch)
            } else if source_drift {
                saw[1] = true;
                Err(LineArtExtractionPairError::SourceIdentityMismatch)
            } else {
                saw[2] = true;
                Ok(())
            };
            assert_eq!(
                validate_line_art_extraction_pair(&request, &result),
                expected,
                "control mask {control_mask:#08b}, source drift {source_drift}",
            );
            match expected {
                Ok(()) => {
                    let validated = validate_line_art_extraction_pair_view(
                        &request,
                        &result,
                    )
                    .expect("exact extraction pair seals");
                    assert!(std::ptr::eq(validated.request(), &request));
                    assert!(std::ptr::eq(validated.result(), &result));
                },
                Err(reason) => assert_eq!(
                    validate_line_art_extraction_pair_view(&request, &result),
                    Err(reason),
                    "sealed control {control_mask:#08b}, source {source_drift}",
                ),
            }
            cases = cases.saturating_add(1);
        }
    }
    assert_eq!(cases, 128);
    assert!(saw.into_iter().all(|seen| seen));
}
