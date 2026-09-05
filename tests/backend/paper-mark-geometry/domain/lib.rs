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
//   - Regression evidence for nominal paper marks and bounded ruler deviation.
// - Must-Not:
//   - Freeze seeded noise, renderer pixels, colors, or custom-paper geometry.
// - Allows:
//   - Inputs: Explicit real-unit regions, paper patterns, and ruler samples.
//   - Outputs: Assertions over exact series, spans, joins, and bounded error.
//   - Side effects: None beyond process-local test allocation.
// - Split-When:
//   - Seeded calibrated path synthesis receives independent executable
//     fixtures.
// - Merge-When:
//   - Paper-mark geometry is fully subsumed by another physical geometry
//     domain.
// - Summary:
//   - Verifies nominal spacing remains exact despite admitted ruler appearance.
// - Description:
//   - Covers compact series, square grids, rules, dots, joins, and error
//     bounds.
// - Usage:
//   - Compile directly against paper-mark and physical-page-profile domains.
// - Defaults:
//   - Region origin is the first nominal mark anchor in each active axis.
//
use atrament_paper_mark_geometry::{
    AxisSeries, GeometryError, PaperMarkGeometry, ProfilePaperMarksError,
    RulerOffset, RulerSample, RulerSampleError, compile_nominal_marks,
    compile_profile_marks, validate_ruler_sample,
};
use atrament_physical_page_profile::{
    BindingEdge, BorderShape, Length, Orientation, PageProfile,
    PageProfileError, PaperMarkAppearance, PaperMarkJoin, PaperMarkLayer,
    PaperPattern, Rect, SheetSize,
};

const FIVE_MM: Length = Length::from_micrometres(5_000);

fn page_profile() -> PageProfile {
    PageProfile {
        binding_edge: BindingEdge::Left,
        border_shape: BorderShape::Rectangle,
        corner_roundness: Length::ZERO,
        orientation: Orientation::Portrait,
        outer_margin: Length::from_micrometres(20_000),
        paper_mark_appearance: rounded_appearance(Length::from_micrometres(
            200,
        )),
        paper_mark_layer: PaperMarkLayer::BelowInk,
        paper_pattern: PaperPattern::Squared { spacing: FIVE_MM },
        printable_region: Rect {
            height: Length::from_micrometres(277_000),
            width: Length::from_micrometres(190_000),
            x: Length::from_micrometres(10_000),
            y: Length::from_micrometres(10_000),
        },
        sheet: SheetSize {
            height: Length::from_micrometres(297_000),
            width: Length::from_micrometres(210_000),
        },
        top_clearance: Length::from_micrometres(10_000),
        writing_inset: Length::from_micrometres(5_000),
    }
}

fn region() -> Rect {
    Rect {
        height: Length::from_micrometres(12_000),
        width: Length::from_micrometres(17_000),
        x: Length::from_micrometres(10_000),
        y: Length::from_micrometres(20_000),
    }
}

fn rounded_appearance(maximum_error: Length) -> PaperMarkAppearance {
    PaperMarkAppearance {
        join: PaperMarkJoin::Rounded {
            radius: Length::from_micrometres(250),
        },
        maximum_ruler_error: maximum_error,
    }
}

fn next_mark_value(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *seed >> 32
}

fn assert_axis_series(
    series: AxisSeries,
    first: u64,
    extent: u64,
    spacing: u64,
    case: usize,
) {
    let expected_count = extent / spacing + 1;
    let expected_last = first + spacing * (expected_count - 1);
    assert_eq!(series.first, Length::from_micrometres(first), "case {case}");
    assert_eq!(
        series.spacing,
        Length::from_micrometres(spacing),
        "case {case}",
    );
    assert_eq!(series.count, expected_count, "case {case}");
    assert_eq!(
        series.coordinate(0),
        Some(Length::from_micrometres(first)),
        "case {case}",
    );
    assert_eq!(
        series.coordinate(expected_count - 1),
        Some(Length::from_micrometres(expected_last)),
        "case {case}",
    );
    assert_eq!(series.coordinate(expected_count), None, "case {case}");
}

#[test]
fn repeated_marks_match_compact_series_reference_oracle() {
    const CASES: usize = 20_000;
    let mut seed = 0x5eed_6a1d_2026_u64;
    for case in 0..CASES {
        let x = next_mark_value(&mut seed) % 1_000;
        let y = next_mark_value(&mut seed) % 1_000;
        let width = next_mark_value(&mut seed) % 1_000 + 1;
        let height = next_mark_value(&mut seed) % 1_000 + 1;
        let spacing = next_mark_value(&mut seed) % 200 + 1;
        let region = Rect {
            height: Length::from_micrometres(height),
            width: Length::from_micrometres(width),
            x: Length::from_micrometres(x),
            y: Length::from_micrometres(y),
        };
        let spacing_length = Length::from_micrometres(spacing);
        match next_mark_value(&mut seed) % 3 {
            0 => {
                let PaperMarkGeometry::Dotted { horizontal, vertical } =
                    compile_nominal_marks(
                        region,
                        PaperPattern::Dotted {
                            spacing: spacing_length,
                        },
                    )
                    .expect("generated dotted geometry")
                else {
                    panic!("generated dotted case must remain dotted");
                };
                assert_axis_series(horizontal, y, height, spacing, case);
                assert_axis_series(vertical, x, width, spacing, case);
            },
            1 => {
                let PaperMarkGeometry::Ruled { horizontal, span } =
                    compile_nominal_marks(
                        region,
                        PaperPattern::Ruled {
                            spacing: spacing_length,
                        },
                    )
                    .expect("generated ruled geometry")
                else {
                    panic!("generated ruled case must remain ruled");
                };
                assert_axis_series(horizontal, y, height, spacing, case);
                assert_eq!(span.start, Length::from_micrometres(x));
                assert_eq!(span.end, Length::from_micrometres(x + width));
            },
            _ => {
                let PaperMarkGeometry::Squared {
                    horizontal,
                    horizontal_span,
                    vertical,
                    vertical_span,
                } = compile_nominal_marks(
                    region,
                    PaperPattern::Squared {
                        spacing: spacing_length,
                    },
                )
                .expect("generated squared geometry")
                else {
                    panic!("generated squared case must remain squared");
                };
                assert_axis_series(horizontal, y, height, spacing, case);
                assert_axis_series(vertical, x, width, spacing, case);
                assert_eq!(horizontal_span.start, Length::from_micrometres(x));
                assert_eq!(
                    horizontal_span.end,
                    Length::from_micrometres(x + width),
                );
                assert_eq!(vertical_span.start, Length::from_micrometres(y));
                assert_eq!(
                    vertical_span.end,
                    Length::from_micrometres(y + height),
                );
            },
        }
    }
}

#[test]
fn ruler_samples_match_bounded_reference_oracle() {
    const CASES: usize = 20_000;
    let mut seed = 0x5eed_7a1e_2026_u64;
    for case in 0..CASES {
        let maximum = next_mark_value(&mut seed) % 501;
        let line_length = next_mark_value(&mut seed) % 1_001;
        let along = next_mark_value(&mut seed) % 1_201;
        let signed = i64::try_from(next_mark_value(&mut seed) % 1_401)
            .expect("small generated offset")
            - 700;
        let sample = RulerSample {
            along: Length::from_micrometres(along),
            normal_offset: RulerOffset::from_micrometres(signed),
        };
        let appearance = PaperMarkAppearance {
            join: PaperMarkJoin::Sharp,
            maximum_ruler_error: Length::from_micrometres(maximum),
        };
        let expected = if along > line_length {
            Err(RulerSampleError::OutsideSpan)
        } else if signed.unsigned_abs() > maximum {
            Err(RulerSampleError::ErrorBoundExceeded)
        } else {
            Ok(sample)
        };
        assert_eq!(
            validate_ruler_sample(
                sample,
                Length::from_micrometres(line_length),
                appearance,
            ),
            expected,
            "ruler oracle mismatch in generated case {case}",
        );
    }
}

#[test]
fn blank_has_no_synthetic_mark_series() {
    assert_eq!(
        compile_nominal_marks(region(), PaperPattern::Blank),
        Ok(PaperMarkGeometry::Blank),
    );
}

#[test]
fn custom_paper_requires_profile_owned_geometry() {
    assert_eq!(
        compile_nominal_marks(region(), PaperPattern::Custom),
        Err(GeometryError::CustomGeometryRequired),
    );
}

#[test]
fn dotted_geometry_uses_exact_independent_axis_series() {
    let geometry = compile_nominal_marks(region(), PaperPattern::Dotted {
        spacing: FIVE_MM,
    })
    .expect("dotted geometry");
    let PaperMarkGeometry::Dotted { horizontal, vertical } = geometry else {
        panic!("fixture must compile as dotted geometry");
    };

    assert_eq!(horizontal.count, 3);
    assert_eq!(
        horizontal.coordinate(0),
        Some(Length::from_micrometres(20_000))
    );
    assert_eq!(
        horizontal.coordinate(1),
        Some(Length::from_micrometres(25_000))
    );
    assert_eq!(
        horizontal.coordinate(2),
        Some(Length::from_micrometres(30_000))
    );
    assert_eq!(horizontal.coordinate(3), None);
    assert_eq!(vertical.count, 4);
    assert_eq!(
        vertical.coordinate(3),
        Some(Length::from_micrometres(25_000))
    );
}

#[test]
fn ruled_geometry_preserves_full_horizontal_span_and_nominal_spacing() {
    let geometry = compile_nominal_marks(region(), PaperPattern::Ruled {
        spacing: FIVE_MM,
    })
    .expect("ruled geometry");
    let PaperMarkGeometry::Ruled { horizontal, span } = geometry else {
        panic!("fixture must compile as ruled geometry");
    };

    assert_eq!(horizontal.spacing, FIVE_MM);
    assert_eq!(span.start, Length::from_micrometres(10_000));
    assert_eq!(span.end, Length::from_micrometres(27_000));
}

#[test]
fn squared_grid_keeps_identical_spacing_on_both_nominal_axes() {
    let geometry = compile_nominal_marks(region(), PaperPattern::Squared {
        spacing: FIVE_MM,
    })
    .expect("squared geometry");
    let PaperMarkGeometry::Squared {
        horizontal,
        horizontal_span,
        vertical,
        vertical_span,
    } = geometry
    else {
        panic!("fixture must compile as squared geometry");
    };

    assert_eq!(horizontal.spacing, FIVE_MM);
    assert_eq!(vertical.spacing, FIVE_MM);
    assert_eq!(
        horizontal.coordinate(1),
        Some(Length::from_micrometres(25_000))
    );
    assert_eq!(
        vertical.coordinate(1),
        Some(Length::from_micrometres(15_000))
    );
    assert_eq!(horizontal_span.start, Length::from_micrometres(10_000));
    assert_eq!(horizontal_span.end, Length::from_micrometres(27_000));
    assert_eq!(vertical_span.start, Length::from_micrometres(20_000));
    assert_eq!(vertical_span.end, Length::from_micrometres(32_000));
}

#[test]
fn ruler_deviation_is_bounded_without_mutating_nominal_grid_anchors() {
    let geometry = compile_nominal_marks(region(), PaperPattern::Squared {
        spacing: FIVE_MM,
    })
    .expect("squared geometry");
    let sample = RulerSample {
        along: Length::from_micrometres(8_000),
        normal_offset: RulerOffset::from_micrometres(-180),
    };
    let appearance = rounded_appearance(Length::from_micrometres(200));

    assert_eq!(
        validate_ruler_sample(
            sample,
            Length::from_micrometres(17_000),
            appearance,
        ),
        Ok(sample),
    );
    assert_eq!(
        geometry,
        compile_nominal_marks(region(), PaperPattern::Squared {
            spacing: FIVE_MM
        },)
        .expect("same nominal geometry"),
    );
}

#[test]
fn ruler_sample_rejects_invalid_appearance_before_sample_geometry() {
    let invalid = PaperMarkAppearance {
        join: PaperMarkJoin::Rounded {
            radius: Length::ZERO,
        },
        maximum_ruler_error: Length::from_micrometres(200),
    };
    assert_eq!(
        validate_ruler_sample(
            RulerSample {
                along: Length::from_micrometres(17_001),
                normal_offset: RulerOffset::from_micrometres(201),
            },
            Length::from_micrometres(17_000),
            invalid,
        ),
        Err(RulerSampleError::InvalidAppearance(
            PageProfileError::PaperMarkRoundedJoinRadiusIsZero,
        )),
    );
}

#[test]
fn ruler_sample_rejects_excess_error_outside_span_and_signed_overflow() {
    let appearance = rounded_appearance(Length::from_micrometres(200));
    assert_eq!(
        validate_ruler_sample(
            RulerSample {
                along: Length::from_micrometres(8_000),
                normal_offset: RulerOffset::from_micrometres(201),
            },
            Length::from_micrometres(17_000),
            appearance,
        ),
        Err(RulerSampleError::ErrorBoundExceeded),
    );
    assert_eq!(
        validate_ruler_sample(
            RulerSample {
                along: Length::from_micrometres(17_001),
                normal_offset: RulerOffset::from_micrometres(0),
            },
            Length::from_micrometres(17_000),
            appearance,
        ),
        Err(RulerSampleError::OutsideSpan),
    );
    assert_eq!(
        validate_ruler_sample(
            RulerSample {
                along: Length::ZERO,
                normal_offset: RulerOffset::from_micrometres(i64::MIN),
            },
            Length::from_micrometres(17_000),
            appearance,
        ),
        Err(RulerSampleError::OffsetMagnitudeOverflow),
    );
}

#[test]
fn invalid_region_and_spacing_fail_without_partial_geometry() {
    let mut empty = region();
    empty.width = Length::ZERO;
    assert_eq!(
        compile_nominal_marks(empty, PaperPattern::Blank),
        Err(GeometryError::EmptyRegion),
    );
    assert_eq!(
        compile_nominal_marks(region(), PaperPattern::Ruled {
            spacing: Length::ZERO,
        },),
        Err(GeometryError::SpacingIsZero),
    );

    let overflowing = Rect {
        height: Length::from_micrometres(1),
        width: Length::from_micrometres(2),
        x: Length::from_micrometres(u64::MAX),
        y: Length::ZERO,
    };
    assert_eq!(
        compile_nominal_marks(overflowing, PaperPattern::Blank),
        Err(GeometryError::RegionOverflow),
    );
}

#[test]
fn compact_series_reports_unrepresentable_anchor_count_instead_of_allocating() {
    let huge = Rect {
        height: Length::from_micrometres(1),
        width: Length::from_micrometres(u64::MAX),
        x: Length::ZERO,
        y: Length::ZERO,
    };
    assert_eq!(
        compile_nominal_marks(huge, PaperPattern::Dotted {
            spacing: Length::from_micrometres(1),
        },),
        Err(GeometryError::RegionOverflow),
    );
}

#[test]
fn profile_plan_keeps_pattern_appearance_and_layer_from_one_profile() {
    let profile = page_profile();
    let plan = compile_profile_marks(region(), profile).expect("profile marks");
    assert_eq!(plan.appearance, profile.paper_mark_appearance);
    assert_eq!(plan.layer, profile.paper_mark_layer);
    let PaperMarkGeometry::Squared { horizontal, vertical, .. } = plan.geometry
    else {
        panic!("profile pattern must compile as squared geometry");
    };
    assert_eq!(horizontal.spacing, FIVE_MM);
    assert_eq!(vertical.spacing, FIVE_MM);
}

#[test]
fn profile_plan_rejects_invalid_complete_page_profile_before_marks() {
    let mut profile = page_profile();
    profile.sheet.width = Length::ZERO;
    assert_eq!(
        compile_profile_marks(region(), profile),
        Err(ProfilePaperMarksError::InvalidProfile(
            PageProfileError::SheetDimensionIsZero,
        )),
    );
}

#[test]
fn profile_plan_preserves_explicit_custom_geometry_requirement() {
    let mut profile = page_profile();
    profile.paper_pattern = PaperPattern::Custom;
    assert_eq!(
        compile_profile_marks(region(), profile),
        Err(ProfilePaperMarksError::Geometry(
            GeometryError::CustomGeometryRequired,
        )),
    );
}

#[test]
fn profile_plan_rejects_mark_region_outside_oriented_sheet() {
    let profile = page_profile();
    let outside = Rect {
        height: Length::from_micrometres(10_000),
        width: Length::from_micrometres(10_000),
        x: Length::from_micrometres(205_000),
        y: Length::from_micrometres(10_000),
    };
    assert_eq!(
        compile_profile_marks(outside, profile),
        Err(ProfilePaperMarksError::RegionOutsideSheet),
    );
}
