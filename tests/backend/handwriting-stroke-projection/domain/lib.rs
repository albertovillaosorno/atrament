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
fn refuses_singular_or_periodic_shears_even_on_the_lower_baseline() {
    // The baseline has zero vertical shear. A 90/180 degree tangent could
    // otherwise appear valid here and silently misrepresent the mean slant.
    let baseline = AuthoredInkSegment {
        start_um: (200, 4_500),
        end_um: (1_200, 4_500),
        ..ORIGINAL
    };
    let placement_for = |angle| InkPlacement {
        dimensions: Some(MeasuredGlyphDimensions {
            width_um: 3_500,
            height_um: 4_500,
            ink_thickness_um: 300,
            slant_millidegrees: angle,
        }),
        ..placement()
    };
    for angle in [
        -90_000i32, 90_000i32, -180_000i32, 180_000i32,
        i32::MIN, i32::MAX,
    ] {
        assert_eq!(
            project_ink_segment(baseline, placement_for(angle)),
            Err("glyph slant must be between -90 and 90 degrees"),
            "singular/periodic slant {angle} must refuse",
        );
    }
    for angle in [-85_000i32, 0i32, 85_000i32] {
        let projected = project_ink_segment(baseline, placement_for(angle))
            .expect("normal slants retain exact lower baseline geometry");
        assert_eq!(projected.start_um, (10_200, 24_500));
        assert_eq!(projected.end_um, (11_200, 24_500));
    }
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
fn projection_rejects_ink_that_collapses_to_a_point() {
    // Both source points are different and valid. A narrowly scaled mean
    // cell rounds their physical coordinates to the same micrometre.
    let tiny = AuthoredInkSegment {
        start_um: (500, 1_000),
        end_um: (501, 1_000),
        ..ORIGINAL
    };
    let dimensions = MeasuredGlyphDimensions {
        width_um: 100,
        height_um: 100,
        ink_thickness_um: 300,
        slant_millidegrees: 0,
    };
    assert_eq!(
        project_ink_segment(tiny, placement()),
        Ok(PhysicalInkSegment {
            page: 1,
            start_um: (10_500, 21_000),
            end_um: (10_501, 21_000),
            width_um: 200,
            rgb: [0x11, 0x22, 0x33],
        }),
        "the authored 1um line remains valid without statistical scaling"
    );
    assert_eq!(
        project_ink_segment(
            tiny,
            InkPlacement {
                dimensions: Some(dimensions),
                ..placement()
            }
        ),
        Err("projected glyph segment must have distinct endpoints"),
    );
    assert_eq!(
        project_ink_segment(
            AuthoredInkSegment {
                end_um: ORIGINAL.start_um,
                ..ORIGINAL
            },
            placement()
        ),
        Err("projected glyph segment must have distinct endpoints"),
    );
}

#[test]
fn rejects_zero_physical_page_and_flattened_mean_cells() {
    // These each retain two distinct physical endpoints, so a zero-length
    // check alone cannot reject the impossible page or cell dimensions.
    assert_eq!(
        project_ink_segment(
            ORIGINAL,
            InkPlacement { page: 0, ..placement() },
        ),
        Err("glyph page must be positive"),
    );
    let valid = MeasuredGlyphDimensions {
        width_um: 3_500,
        height_um: 4_500,
        ink_thickness_um: 300,
        slant_millidegrees: 0,
    };
    assert!(project_ink_segment(
        ORIGINAL,
        InkPlacement { dimensions: Some(valid), ..placement() },
    ).is_ok());
    for flattened in [
        MeasuredGlyphDimensions { width_um: 0, ..valid },
        MeasuredGlyphDimensions { height_um: 0, ..valid },
    ] {
        assert_eq!(
            project_ink_segment(
                ORIGINAL,
                InkPlacement {
                    dimensions: Some(flattened), ..placement()
                },
            ),
            Err("glyph mean cell width and height must be positive"),
        );
    }
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
fn joins_refuse_forged_nonphysical_strokes_before_emitting_ink() {
    // PhysicalInkSegment is public for exporters, so callers can construct a
    // value without going through project_ink_segment. A join must not turn
    // those values into apparently admitted physical ink.
    let before = PhysicalInkSegment {
        page: 1,
        start_um: (100, 100),
        end_um: (200, 100),
        width_um: 200,
        rgb: [1, 2, 3],
    };
    let after = PhysicalInkSegment {
        start_um: (300, 100),
        end_um: (400, 100),
        ..before
    };
    assert!(matches!(connect_ink_segments(before, after, 100), Ok(Some(_))));
    assert_eq!(
        connect_ink_segments(
            PhysicalInkSegment { page: 0, ..before },
            PhysicalInkSegment { page: 0, ..after },
            100,
        ),
        Err("text join source page must be positive"),
    );
    for width in [0, 79, 1_501, u32::MAX] {
        assert_eq!(
            connect_ink_segments(
                PhysicalInkSegment { width_um: width, ..before },
                PhysicalInkSegment { width_um: width, ..after },
                100,
            ),
            Err("text join source pen width exceeds 80..1500 um"),
        );
    }
    for (left, right) in [
        (PhysicalInkSegment { end_um: before.start_um, ..before }, after),
        (before, PhysicalInkSegment { end_um: after.start_um, ..after }),
        (
            PhysicalInkSegment { end_um: before.start_um, ..before },
            PhysicalInkSegment { start_um: before.start_um, ..after },
        ),
    ] {
        assert_eq!(
            connect_ink_segments(left, right, 100),
            Err("text join source segment must have distinct endpoints"),
        );
    }
}

#[test]
fn two_axis_join_limit_matches_independent_physical_distance_oracle() {
    let left = PhysicalInkSegment {
        page: 3,
        start_um: (10, 10),
        end_um: (5_000, 5_000),
        width_um: 300,
        rgb: [4, 5, 6],
    };
    let mut bridges = 0usize;
    let mut refusals = 0usize;
    let mut coincidences = 0usize;
    // Both signed axes exercise all four quadrants, exact equality, and
    // distances whose individual components fit but Euclidean length does not.
    for dx in (-1_500i32..=1_500i32).step_by(75) {
        for dy in (-1_500i32..=1_500i32).step_by(75) {
            let right = PhysicalInkSegment {
                start_um: (
                    u32::try_from(5_000i32 + dx).expect("bounded oracle x"),
                    u32::try_from(5_000i32 + dy).expect("bounded oracle y"),
                ),
                end_um: (10_000, 10_000),
                ..left
            };
            let horizontal_square = i128::from(dx) * i128::from(dx);
            let vertical_square = i128::from(dy) * i128::from(dy);
            let inside =
                horizontal_square + vertical_square <= 1_200i128 * 1_200;
            let outcome = connect_ink_segments(left, right, 1_200);
            if !inside {
                refusals += 1;
                assert_eq!(outcome, Err("text join gap exceeds 1200 um"));
            } else if dx == 0i32 && dy == 0i32 {
                coincidences += 1;
                assert_eq!(outcome, Ok(None));
            } else {
                bridges += 1;
                assert_eq!(
                    outcome,
                    Ok(Some(PhysicalInkSegment {
                        page: left.page,
                        start_um: left.end_um,
                        end_um: right.start_um,
                        width_um: left.width_um,
                        rgb: left.rgb,
                    })),
                );
            }
        }
    }
    assert!(bridges > 600, "oracle must admit many real bridges");
    assert!(refusals > 600, "oracle must reject diagonal over-limit gaps");
    assert_eq!(coincidences, 1);
}

#[test]
fn caller_selected_join_limit_is_respected_without_false_1200_diagnostic() {
    let before = PhysicalInkSegment {
        page: 1,
        start_um: (100, 200),
        end_um: (200, 200),
        width_um: 250,
        rgb: [3, 4, 5],
    };
    let after = PhysicalInkSegment {
        start_um: (1_600, 200),
        end_um: (1_700, 200),
        ..before
    };
    let expected_bridge = PhysicalInkSegment {
        start_um: before.end_um,
        end_um: after.start_um,
        ..before
    };
    assert_eq!(connect_ink_segments(before, after, 1_400),
        Ok(Some(expected_bridge)));
    assert_eq!(connect_ink_segments(before, after, 1_399),
        Err("text join gap exceeds configured maximum"));
    assert_eq!(connect_ink_segments(before, after, 1_200),
        Err("text join gap exceeds 1200 um"));
    assert_eq!(connect_ink_segments(before, after, 0),
        Err("text join gap exceeds configured maximum"));
    assert_eq!(connect_ink_segments(before, after, u64::MAX),
        Ok(Some(expected_bridge)));
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
