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
    GeometryError, PaperMarkGeometry, ProfilePaperMarksError, RulerOffset,
    RulerSample, RulerSampleError, compile_nominal_marks,
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
