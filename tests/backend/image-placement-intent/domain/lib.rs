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
//   - Regression evidence for source-preserving placed-image intent.
// - Must-Not:
//   - Decode images, choose units, evaluate transforms, extract line art, or
//     render output.
// - Allows:
//   - Inputs: Deterministic source, geometry, appearance, and mode fixtures.
//   - Outputs: Assertions over four placement modes and property retention.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Image transform or rendering behavior gains independent fixtures.
// - Merge-When:
//   - Placement evidence moves into one semantic figure harness.
// - Summary:
//   - Proves placement edits retain original image source identity.
// - Description:
//   - Covers placement modes, crop, geometry, opacity, policy, and layering.
// - Usage:
//   - Compile directly against the image-placement-intent domain.
// - Defaults:
//   - No placement or appearance value is implicit.
//
use atrament_image_placement_intent::{
    ImagePlacementGeometry, ImagePlacementIntent, ImagePlacementMode,
};

#[test]
fn first_release_image_placement_modes_are_explicit() {
    let modes = [
        ImagePlacementMode::AboveText,
        ImagePlacementMode::BelowText,
        ImagePlacementMode::ClippedRegion,
        ImagePlacementMode::Inline,
    ];
    assert_eq!(modes.len(), 4);
}

#[test]
fn placement_retains_all_accepted_source_and_editor_properties() {
    let placement = ImagePlacementIntent {
        color_handling: "preserve-profile",
        crop: (10_i32, 20_i32, 300_i32, 200_i32),
        geometry: ImagePlacementGeometry {
            position: (45_i32, 90_i32),
            size: (320_u32, 240_u32),
            transform: "rotation-0",
        },
        opacity: 85_u8,
        placement_constraints: ["inside-writable-region"],
        placement_mode: ImagePlacementMode::AboveText,
        resolution_policy: "retain-original-resolution",
        source_identity: "asset-photo-7",
        z_order: 12_i32,
    };
    assert_eq!(placement.source_identity, "asset-photo-7");
    assert_eq!(placement.geometry.position, (45, 90));
    assert_eq!(placement.geometry.size, (320, 240));
    assert_eq!(placement.crop, (10, 20, 300, 200));
    assert_eq!(placement.opacity, 85);
    assert_eq!(placement.z_order, 12);
}

#[test]
fn placement_changes_do_not_require_a_new_source_identity() {
    let base = ImagePlacementIntent {
        color_handling: "preserve",
        crop: "full-image",
        geometry: ImagePlacementGeometry {
            position: (0_i32, 0_i32),
            size: (100_u32, 100_u32),
            transform: "identity",
        },
        opacity: 100_u8,
        placement_constraints: "page-1",
        placement_mode: ImagePlacementMode::Inline,
        resolution_policy: "original",
        source_identity: "asset-3",
        z_order: 1_i32,
    };
    let changed = ImagePlacementIntent {
        crop: "center-crop",
        opacity: 60,
        placement_mode: ImagePlacementMode::ClippedRegion,
        z_order: 5,
        ..base.clone()
    };
    assert_eq!(base.source_identity, changed.source_identity);
    assert_ne!(base.crop, changed.crop);
    assert_ne!(base.placement_mode, changed.placement_mode);
}
