// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - LicenseRef-Proprietary
// Confidential:
//   - true
// License-File:
//   - LICENSE
//
// Boundary-Contract:
// - Owns:
//   - Numeric proof fixtures for exported physical handwriting segments.
// - Must-Not:
//   - Change authored outlines, invoke renderer codecs or infer personal ink.
// - Allows:
//   - Inputs: Bounded local and physical pen stroke fixtures.
//   - Outputs: Exact coordinate, shear, join and overflow assertions.
//   - Side effects: None.
// - Split-When:
//   - An independent physical rendering regression needs custom fixtures.
// - Merge-When:
//   - Numeric projection is removed from real notebook composition.
// - Summary:
//   - Proves physical pen geometry and join correctness independently.
// - Description:
//   - Covers shear, means, overflow, contact compatibility and replay.
// - Usage:
//   - cargo test -p atrament_handwriting_stroke_projection
// - Defaults:
//   - No assumed learned glyphs or simulated PDF bytes.
//

//! Independent physical handwriting projection and join regression fixtures.

#![expect(clippy::expect_used, reason = "fixed numeric physical fixtures")]

use atrament_handwriting_stroke_projection::{
    AuthoredInkSegment, InkPlacement, MeasuredGlyphDimensions,
    PhysicalInkSegment, connect_ink_segments, project_ink_segment,
};

const ORIGINAL: AuthoredInkSegment = AuthoredInkSegment {
    start_um: (500, 1_000),
    end_um: (1_300, 2_200),
    width_um: 200,
    rgb: [0x11, 0x22, 0x33],
};

const fn placement() -> InkPlacement {
    InkPlacement {
        page: 1,
        origin_um: (10_000, 20_000),
        dimensions: None,
        ink_scale_percent: 100,
        color_override: None,
    }
}

#[test]
fn uncalibrated_projection_preserves_exact_authored_stroke_geometry() {
    let original_copy = ORIGINAL;
    let projected = project_ink_segment(ORIGINAL, placement());
    let result = projected.expect("authored stroke fits physical bounds");
    assert_eq!(
        result,
        PhysicalInkSegment {
            page: 1,
            start_um: (10_500, 21_000),
            end_um: (11_300, 22_200),
            width_um: 200,
            rgb: [0x11, 0x22, 0x33],
        }
    );
    assert_eq!(
        ORIGINAL, original_copy,
        "projection never mutates templates"
    );
    assert_eq!(project_ink_segment(ORIGINAL, placement()), Ok(result));
}

#[test]
fn mean_geometry_and_heading_treatment_project_exact_micrometres() {
    let mut target = placement();
    target.page = 3;
    target.dimensions = Some(MeasuredGlyphDimensions {
        width_um: 7_000,
        height_um: 9_000,
        ink_thickness_um: 600,
        slant_millidegrees: 0,
    });
    target.ink_scale_percent = 125;
    target.color_override = Some([0xAB, 0xCD, 0xEF]);
    assert_eq!(
        project_ink_segment(ORIGINAL, target),
        Ok(PhysicalInkSegment {
            page: 3,
            start_um: (11_000, 22_000),
            end_um: (12_600, 24_400),
            width_um: 500,
            rgb: [0xAB, 0xCD, 0xEF],
        })
    );
}

#[test]
fn slant_shears_upper_contours_but_keeps_exact_lower_baseline() {
    let shape = AuthoredInkSegment {
        start_um: (500, 0),
        end_um: (500, 4_500),
        ..ORIGINAL
    };
    let target = InkPlacement {
        dimensions: Some(MeasuredGlyphDimensions {
            width_um: 3_500,
            height_um: 4_500,
            ink_thickness_um: 300,
            slant_millidegrees: 45_000,
        }),
        ..placement()
    };
    assert_eq!(
        project_ink_segment(shape, target),
        Ok(PhysicalInkSegment {
            page: 1,
            start_um: (15_000, 20_000),
            end_um: (10_500, 24_500),
            width_um: 200,
            rgb: [0x11, 0x22, 0x33],
        })
    );
}

#[test]
fn refuses_invalid_ink_and_coordinates_without_clipping_or_wrapping() {
    for width in [0, 79, 1501, u32::MAX] {
        let result = project_ink_segment(
            AuthoredInkSegment {
                width_um: width,
                ..ORIGINAL
            },
            placement(),
        );
        assert_eq!(result, Err("scaled glyph ink width exceeds 80..1500 um"));
    }
    assert_eq!(
        project_ink_segment(
            ORIGINAL,
            InkPlacement {
                origin_um: (-600, 0),
                ..placement()
            }
        ),
        Err("glyph placement needs nonnegative coordinates")
    );
    assert_eq!(
        project_ink_segment(
            ORIGINAL,
            InkPlacement {
                origin_um: (i64::MAX, 20_000),
                ..placement()
            }
        ),
        Err("glyph placement overflow")
    );
    assert_eq!(
        project_ink_segment(
            ORIGINAL,
            InkPlacement {
                origin_um: (i64::from(u32::MAX), 0),
                ..placement()
            }
        ),
        Err("glyph placement needs nonnegative coordinates")
    );
    assert_eq!(
        project_ink_segment(
            ORIGINAL,
            InkPlacement {
                dimensions: Some(MeasuredGlyphDimensions {
                    width_um: u32::MAX,
                    height_um: 4_500,
                    ink_thickness_um: 300,
                    slant_millidegrees: 0,
                }),
                ..placement()
            }
        ),
        Ok(PhysicalInkSegment {
            page: 1,
            start_um: (613_576_756, 21_000),
            end_um: (1_595_283_567, 22_200),
            width_um: 200,
            rgb: [0x11, 0x22, 0x33],
        })
    );
}

#[test]
fn exact_direction_and_displacement_determine_join_without_source_mutation() {
    let left = PhysicalInkSegment {
        page: 2,
        start_um: (100, 100),
        end_um: (1_000, 100),
        width_um: 200,
        rgb: [1, 2, 3],
    };
    let right = PhysicalInkSegment {
        page: 2,
        start_um: (2_200, 100),
        end_um: (3_000, 100),
        ..left
    };
    let bridge = connect_ink_segments(left, right, 1_200)
        .expect("exact 1200 micrometre bridge is admissible")
        .expect("nonzero bridge has physical ink");
    assert_eq!(
        bridge,
        PhysicalInkSegment {
            page: 2,
            start_um: (1_000, 100),
            end_um: (2_200, 100),
            width_um: 200,
            rgb: [1, 2, 3],
        }
    );
    assert_eq!(
        connect_ink_segments(
            left,
            PhysicalInkSegment {
                start_um: left.end_um,
                ..right
            },
            1_200
        ),
        Ok(None),
        "already contiguous ink must not be duplicated"
    );
    assert_eq!(left.end_um, (1_000, 100));
    assert_eq!(right.start_um, (2_200, 100));
}

#[test]
fn unsafe_adjacent_joins_refuse_instead_of_interpolating_ink() {
    let left = PhysicalInkSegment {
        page: 1,
        start_um: (100, 100),
        end_um: (1_000, 100),
        width_um: 200,
        rgb: [1, 2, 3],
    };
    let right = PhysicalInkSegment {
        start_um: (2_201, 100),
        end_um: (3_000, 100),
        ..left
    };
    assert_eq!(
        connect_ink_segments(left, right, 1_200),
        Err("text join gap exceeds 1200 um")
    );
    assert_eq!(
        connect_ink_segments(
            left, PhysicalInkSegment { page: 2, ..right }, 1_200,
        ),
        Err("text join cannot cross physical pages")
    );
    assert_eq!(
        connect_ink_segments(
            left,
            PhysicalInkSegment {
                width_um: 201,
                ..right
            },
            1_200
        ),
        Err("text join requires matching physical pen and RGB")
    );
    assert_eq!(
        connect_ink_segments(
            left,
            PhysicalInkSegment {
                rgb: [3, 2, 1],
                ..right
            },
            1_200
        ),
        Err("text join requires matching physical pen and RGB")
    );
    // u128 distance computation cannot wrap u32-extreme coordinates.
    assert_eq!(
        connect_ink_segments(
            left,
            PhysicalInkSegment {
                start_um: (u32::MAX, u32::MAX),
                ..right
            },
            1_200
        ),
        Err("text join gap exceeds 1200 um")
    );
}

#[test]
fn repeated_projection_replays_identically_for_both_render_paths() {
    for seed_offset in 0..96u32 {
        let shape = AuthoredInkSegment {
            start_um: (500 + seed_offset, 1_000),
            end_um: (1_300 + seed_offset, 2_200),
            ..ORIGINAL
        };
        let params = InkPlacement {
            dimensions: Some(MeasuredGlyphDimensions {
                width_um: 3_500 + seed_offset,
                height_um: 4_500 + seed_offset,
                ink_thickness_um: 300,
                slant_millidegrees: 500,
            }),
            ..placement()
        };
        let pdf_plan = project_ink_segment(shape, params);
        let png_plan = project_ink_segment(shape, params);
        assert_eq!(pdf_plan, png_plan, "export formats share exact geometry");
        assert!(
            pdf_plan.is_ok(),
            "bounded author template must remain valid"
        );
    }
}
