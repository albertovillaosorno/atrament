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
//   - Regression evidence for exact physical page-profile geometry.
// - Must-Not:
//   - Assert pixels, renderer colors, serialization, or ruler imperfections.
// - Allows:
//   - Inputs: Deterministic micrometre-scale physical profile fixtures.
//   - Outputs: Assertions over validation and exact writable rectangles.
//   - Side effects: None beyond process-local test allocation.
// - Split-When:
//   - Calibrated paper-mark geometry gains independent executable fixtures.
// - Merge-When:
//   - Physical page validation moves into another direct domain harness.
// - Summary:
//   - Verifies exact sheet, printable, margin, inset, and border geometry.
// - Description:
//   - Covers orientation, binding edges, pattern spacing, and invalid bounds.
// - Usage:
//   - Compile directly against the physical-page-profile domain crate.
// - Defaults:
//   - Uses canonical micrometres and no visual approximation.
//
use atrament_physical_page_profile::{
    BindingEdge, BorderShape, Length, Orientation, PageProfile,
    PageProfileError, PaperMarkAppearance, PaperMarkJoin, PaperMarkLayer,
    PaperPattern, Rect, SheetSize,
};

const A4_HEIGHT: Length = Length::from_micrometres(297_000);
const A4_WIDTH: Length = Length::from_micrometres(210_000);
const FIVE_MM: Length = Length::from_micrometres(5_000);
const TEN_MM: Length = Length::from_micrometres(10_000);
const TWENTY_MM: Length = Length::from_micrometres(20_000);

fn base_profile(binding_edge: BindingEdge) -> PageProfile {
    PageProfile {
        binding_edge,
        border_shape: BorderShape::RoundedRectangle,
        corner_roundness: FIVE_MM,
        orientation: Orientation::Portrait,
        outer_margin: TWENTY_MM,
        paper_mark_appearance: PaperMarkAppearance {
            join: PaperMarkJoin::Rounded {
                radius: Length::from_micrometres(250),
            },
            maximum_ruler_error: Length::from_micrometres(200),
        },
        paper_mark_layer: PaperMarkLayer::BelowInk,
        paper_pattern: PaperPattern::Squared { spacing: FIVE_MM },
        printable_region: Rect {
            height: Length::from_micrometres(277_000),
            width: Length::from_micrometres(190_000),
            x: TEN_MM,
            y: TEN_MM,
        },
        sheet: SheetSize {
            height: A4_HEIGHT,
            width: A4_WIDTH,
        },
        top_clearance: TEN_MM,
        writing_inset: FIVE_MM,
    }
}

fn next_profile_value(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *seed >> 32
}

fn next_validation_value(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(2_862_933_555_777_941_757)
        .wrapping_add(3_037_000_493);
    *seed
}

fn reference_writable_region(
    profile: PageProfile,
) -> Result<Rect, PageProfileError> {
    if profile.sheet.width == Length::ZERO
        || profile.sheet.height == Length::ZERO
    {
        return Err(PageProfileError::SheetDimensionIsZero);
    }
    let (sheet_width, sheet_height) = match profile.orientation {
        Orientation::Landscape => (
            profile.sheet.height.micrometres(),
            profile.sheet.width.micrometres(),
        ),
        Orientation::Portrait => (
            profile.sheet.width.micrometres(),
            profile.sheet.height.micrometres(),
        ),
    };
    let printable = profile.printable_region;
    let width = printable.width.micrometres();
    let height = printable.height.micrometres();
    if width == 0 || height == 0 {
        return Err(PageProfileError::PrintableRegionIsEmpty);
    }
    let right = u128::from(printable.x.micrometres()) + u128::from(width);
    let bottom = u128::from(printable.y.micrometres()) + u128::from(height);
    if right > u128::from(u64::MAX)
        || bottom > u128::from(u64::MAX)
        || right > u128::from(sheet_width)
        || bottom > u128::from(sheet_height)
    {
        return Err(PageProfileError::PrintableRegionOutsideSheet);
    }
    let top = printable
        .y
        .micrometres()
        .checked_add(profile.top_clearance.micrometres())
        .ok_or(PageProfileError::TopClearanceExhaustsPrintableRegion)?;
    let remaining_height = height
        .checked_sub(profile.top_clearance.micrometres())
        .ok_or(PageProfileError::TopClearanceExhaustsPrintableRegion)?;
    if remaining_height == 0 {
        return Err(PageProfileError::TopClearanceExhaustsPrintableRegion);
    }
    let binding_inset = profile
        .outer_margin
        .micrometres()
        .checked_add(profile.writing_inset.micrometres())
        .ok_or(PageProfileError::BindingInsetExhaustsPrintableRegion)?;
    let (x, y, writable_width, writable_height) = match profile.binding_edge {
        BindingEdge::Left => (
            printable
                .x
                .micrometres()
                .checked_add(binding_inset)
                .ok_or(PageProfileError::BindingInsetExhaustsPrintableRegion)?,
            top,
            width
                .checked_sub(binding_inset)
                .ok_or(PageProfileError::BindingInsetExhaustsPrintableRegion)?,
            remaining_height,
        ),
        BindingEdge::Right => (
            printable.x.micrometres(),
            top,
            width
                .checked_sub(binding_inset)
                .ok_or(PageProfileError::BindingInsetExhaustsPrintableRegion)?,
            remaining_height,
        ),
        BindingEdge::Top => (
            printable.x.micrometres(),
            top.checked_add(binding_inset)
                .ok_or(PageProfileError::BindingInsetExhaustsPrintableRegion)?,
            width,
            remaining_height
                .checked_sub(binding_inset)
                .ok_or(PageProfileError::BindingInsetExhaustsPrintableRegion)?,
        ),
        BindingEdge::Bottom => (
            printable.x.micrometres(),
            top,
            width,
            remaining_height
                .checked_sub(binding_inset)
                .ok_or(PageProfileError::BindingInsetExhaustsPrintableRegion)?,
        ),
    };
    if writable_width == 0 || writable_height == 0 {
        return Err(PageProfileError::BindingInsetExhaustsPrintableRegion);
    }
    Ok(Rect {
        height: Length::from_micrometres(writable_height),
        width: Length::from_micrometres(writable_width),
        x: Length::from_micrometres(x),
        y: Length::from_micrometres(y),
    })
}

fn reference_profile_validation(
    profile: PageProfile,
) -> Result<(), PageProfileError> {
    reference_printable_region(profile)?;
    if matches!(
        profile.paper_mark_appearance.join,
        PaperMarkJoin::Rounded { radius } if radius == Length::ZERO
    ) {
        return Err(PageProfileError::PaperMarkRoundedJoinRadiusIsZero);
    }
    if matches!(
        profile.paper_pattern,
        PaperPattern::Dotted { spacing }
            | PaperPattern::Ruled { spacing }
            | PaperPattern::Squared { spacing }
            if spacing == Length::ZERO
    ) {
        return Err(PageProfileError::PatternSpacingIsZero);
    }
    if profile.border_shape != BorderShape::RoundedRectangle {
        if profile.corner_roundness != Length::ZERO {
            return Err(PageProfileError::CornerRoundnessRequiresRoundedBorder);
        }
    } else {
        let doubled = u128::from(profile.corner_roundness.micrometres()) * 2;
        if doubled > u128::from(profile.printable_region.width.micrometres())
            || doubled
                > u128::from(profile.printable_region.height.micrometres())
        {
            return Err(PageProfileError::CornerRoundnessExceedsPrintableRegion);
        }
    }
    reference_writable_region(profile).map(|_writable| ())
}

fn reference_printable_region(
    profile: PageProfile,
) -> Result<(), PageProfileError> {
    if profile.sheet.width == Length::ZERO
        || profile.sheet.height == Length::ZERO
    {
        return Err(PageProfileError::SheetDimensionIsZero);
    }
    let (sheet_width, sheet_height) = match profile.orientation {
        Orientation::Landscape => (
            profile.sheet.height.micrometres(),
            profile.sheet.width.micrometres(),
        ),
        Orientation::Portrait => (
            profile.sheet.width.micrometres(),
            profile.sheet.height.micrometres(),
        ),
    };
    let printable = profile.printable_region;
    if printable.width == Length::ZERO || printable.height == Length::ZERO {
        return Err(PageProfileError::PrintableRegionIsEmpty);
    }
    let right = u128::from(printable.x.micrometres())
        + u128::from(printable.width.micrometres());
    let bottom = u128::from(printable.y.micrometres())
        + u128::from(printable.height.micrometres());
    if right > u128::from(u64::MAX)
        || bottom > u128::from(u64::MAX)
        || right > u128::from(sheet_width)
        || bottom > u128::from(sheet_height)
    {
        return Err(PageProfileError::PrintableRegionOutsideSheet);
    }
    Ok(())
}

fn generated_validation_profile(seed: &mut u64, case: usize) -> PageProfile {
    let mut profile = base_profile(match next_validation_value(seed) % 4 {
        0 => BindingEdge::Bottom,
        1 => BindingEdge::Left,
        2 => BindingEdge::Right,
        _ => BindingEdge::Top,
    });
    profile.orientation = if next_validation_value(seed) & 1 == 0 {
        Orientation::Portrait
    } else {
        Orientation::Landscape
    };
    if profile.orientation == Orientation::Landscape {
        profile.printable_region = Rect {
            height: Length::from_micrometres(190_000),
            width: Length::from_micrometres(277_000),
            x: TEN_MM,
            y: TEN_MM,
        };
    }
    match case % 12 {
        0 => profile.sheet.width = Length::ZERO,
        1 => profile.printable_region.height = Length::ZERO,
        2 => profile.printable_region.x = Length::from_micrometres(u64::MAX),
        3 => {
            profile.paper_mark_appearance.join =
                PaperMarkJoin::Rounded { radius: Length::ZERO };
            profile.paper_pattern = PaperPattern::Ruled {
                spacing: Length::ZERO,
            };
        },
        4 => {
            profile.paper_pattern = PaperPattern::Squared {
                spacing: Length::ZERO,
            };
        },
        5 => {
            profile.border_shape = BorderShape::Rectangle;
            profile.corner_roundness = FIVE_MM;
        },
        6 => {
            profile.corner_roundness = Length::from_micrometres(u64::MAX);
        },
        7 => profile.top_clearance = profile.printable_region.height,
        8 => {
            profile.outer_margin = Length::from_micrometres(u64::MAX);
            profile.writing_inset = Length::from_micrometres(1);
        },
        9 => {
            profile.outer_margin = match profile.binding_edge {
                BindingEdge::Bottom | BindingEdge::Top => {
                    profile.printable_region.height
                },
                BindingEdge::Left | BindingEdge::Right => {
                    profile.printable_region.width
                },
            };
            profile.writing_inset = Length::ZERO;
        },
        10 => {
            let jitter = next_validation_value(seed) % 5_000;
            profile.top_clearance = Length::from_micrometres(jitter);
            profile.outer_margin = Length::from_micrometres(jitter / 2);
            profile.writing_inset = Length::from_micrometres(jitter / 3);
        },
        _ => {
            profile.paper_mark_appearance.maximum_ruler_error =
                Length::from_micrometres(next_validation_value(seed));
        },
    }
    profile
}

#[test]
fn mixed_profile_failures_match_independent_precedence_oracle() {
    const CASES: usize = 120_000;
    let mut seed = 0xd1b5_4a32_d192_ed03_u64;
    let mut valid = 0usize;
    let mut invalid = 0usize;
    for case in 0..CASES {
        let profile = generated_validation_profile(&mut seed, case);
        let expected = reference_profile_validation(profile);
        if expected.is_ok() {
            valid = valid.saturating_add(1);
        } else {
            invalid = invalid.saturating_add(1);
        }
        assert_eq!(
            profile.validate().map(|_valid| ()),
            expected,
            "profile validation mismatch in generated case {case}",
        );
        if reference_printable_region(profile).is_ok() {
            assert_eq!(
                profile.writable_region(),
                reference_writable_region(profile),
                "writable-region mismatch in generated case {case}",
            );
        }
    }
    assert!(valid > 10_000);
    assert!(invalid > 50_000);
}

#[test]
fn valid_profiles_match_writable_region_reference_oracle() {
    const CASES: usize = 20_000;
    let mut seed = 0x5eed_9a6e_2026_u64;
    for case in 0..CASES {
        let binding_edge = match next_profile_value(&mut seed) % 4 {
            0 => BindingEdge::Bottom,
            1 => BindingEdge::Left,
            2 => BindingEdge::Right,
            _ => BindingEdge::Top,
        };
        let orientation = if next_profile_value(&mut seed) & 1 == 0 {
            Orientation::Portrait
        } else {
            Orientation::Landscape
        };
        let sheet = SheetSize {
            height: Length::from_micrometres(800),
            width: Length::from_micrometres(1_000),
        };
        let (sheet_width, sheet_height) = match orientation {
            Orientation::Landscape => (800u64, 1_000u64),
            Orientation::Portrait => (1_000u64, 800u64),
        };
        let x = next_profile_value(&mut seed) % sheet_width;
        let y = next_profile_value(&mut seed) % sheet_height;
        let width = next_profile_value(&mut seed) % (sheet_width - x) + 1;
        let height = next_profile_value(&mut seed) % (sheet_height - y) + 1;
        let top_clearance = next_profile_value(&mut seed) % height;
        let remaining_height = height - top_clearance;
        let binding_capacity = match binding_edge {
            BindingEdge::Bottom | BindingEdge::Top => remaining_height,
            BindingEdge::Left | BindingEdge::Right => width,
        };
        let binding_inset =
            next_profile_value(&mut seed) % binding_capacity;
        let outer_margin = binding_inset / 2;
        let writing_inset = binding_inset - outer_margin;
        let profile = PageProfile {
            binding_edge,
            border_shape: BorderShape::Rectangle,
            corner_roundness: Length::ZERO,
            orientation,
            outer_margin: Length::from_micrometres(outer_margin),
            paper_mark_appearance: PaperMarkAppearance {
                join: PaperMarkJoin::Sharp,
                maximum_ruler_error: Length::ZERO,
            },
            paper_mark_layer: PaperMarkLayer::BelowInk,
            paper_pattern: PaperPattern::Blank,
            printable_region: Rect {
                height: Length::from_micrometres(height),
                width: Length::from_micrometres(width),
                x: Length::from_micrometres(x),
                y: Length::from_micrometres(y),
            },
            sheet,
            top_clearance: Length::from_micrometres(top_clearance),
            writing_inset: Length::from_micrometres(writing_inset),
        };
        let top = y + top_clearance;
        let expected = match binding_edge {
            BindingEdge::Bottom => Rect {
                height: Length::from_micrometres(
                    remaining_height - binding_inset,
                ),
                width: Length::from_micrometres(width),
                x: Length::from_micrometres(x),
                y: Length::from_micrometres(top),
            },
            BindingEdge::Left => Rect {
                height: Length::from_micrometres(remaining_height),
                width: Length::from_micrometres(width - binding_inset),
                x: Length::from_micrometres(x + binding_inset),
                y: Length::from_micrometres(top),
            },
            BindingEdge::Right => Rect {
                height: Length::from_micrometres(remaining_height),
                width: Length::from_micrometres(width - binding_inset),
                x: Length::from_micrometres(x),
                y: Length::from_micrometres(top),
            },
            BindingEdge::Top => Rect {
                height: Length::from_micrometres(
                    remaining_height - binding_inset,
                ),
                width: Length::from_micrometres(width),
                x: Length::from_micrometres(x),
                y: Length::from_micrometres(top + binding_inset),
            },
        };
        assert_eq!(profile.validate(), Ok(profile), "generated case {case}");
        assert_eq!(
            profile.writable_region(),
            Ok(expected),
            "generated case {case}",
        );
    }
}

#[test]
fn canonical_length_round_trips_exact_micrometres() {
    let length = Length::from_micrometres(123_456);
    assert_eq!(length.micrometres(), 123_456);
}

#[test]
fn landscape_orientation_swaps_nominal_sheet_dimensions() {
    let mut profile = base_profile(BindingEdge::Left);
    profile.orientation = Orientation::Landscape;
    profile.printable_region = Rect {
        height: Length::from_micrometres(190_000),
        width: Length::from_micrometres(277_000),
        x: TEN_MM,
        y: TEN_MM,
    };

    assert_eq!(
        profile.oriented_sheet(),
        Ok(SheetSize {
            height: A4_WIDTH,
            width: A4_HEIGHT,
        }),
    );
    assert_eq!(profile.validate(), Ok(profile));
}

#[test]
fn left_and_right_binding_insets_reduce_only_horizontal_writable_extent() {
    let left = base_profile(BindingEdge::Left);
    assert_eq!(
        left.writable_region(),
        Ok(Rect {
            height: Length::from_micrometres(267_000),
            width: Length::from_micrometres(165_000),
            x: Length::from_micrometres(35_000),
            y: TWENTY_MM,
        }),
    );

    let right = base_profile(BindingEdge::Right);
    assert_eq!(
        right.writable_region(),
        Ok(Rect {
            height: Length::from_micrometres(267_000),
            width: Length::from_micrometres(165_000),
            x: TEN_MM,
            y: TWENTY_MM,
        }),
    );
}

#[test]
fn top_and_bottom_binding_insets_reduce_only_vertical_writable_extent() {
    let top = base_profile(BindingEdge::Top);
    assert_eq!(
        top.writable_region(),
        Ok(Rect {
            height: Length::from_micrometres(242_000),
            width: Length::from_micrometres(190_000),
            x: TEN_MM,
            y: Length::from_micrometres(45_000),
        }),
    );

    let bottom = base_profile(BindingEdge::Bottom);
    assert_eq!(
        bottom.writable_region(),
        Ok(Rect {
            height: Length::from_micrometres(242_000),
            width: Length::from_micrometres(190_000),
            x: TEN_MM,
            y: TWENTY_MM,
        }),
    );
}

#[test]
fn printable_region_must_be_nonempty_and_inside_oriented_sheet() {
    let mut empty = base_profile(BindingEdge::Left);
    empty.printable_region.width = Length::ZERO;
    assert_eq!(
        empty.validate(),
        Err(PageProfileError::PrintableRegionIsEmpty),
    );

    let mut outside = base_profile(BindingEdge::Left);
    outside.printable_region.x = Length::from_micrometres(30_000);
    assert_eq!(
        outside.validate(),
        Err(PageProfileError::PrintableRegionOutsideSheet),
    );
}

#[test]
fn printable_coordinate_overflow_is_a_typed_out_of_sheet_failure() {
    let mut profile = base_profile(BindingEdge::Left);
    profile.printable_region.x = Length::from_micrometres(u64::MAX);
    assert_eq!(
        profile.validate(),
        Err(PageProfileError::PrintableRegionOutsideSheet),
    );
}

#[test]
fn sheet_dimensions_and_repeated_pattern_spacing_must_be_nonzero() {
    let mut zero_sheet = base_profile(BindingEdge::Left);
    zero_sheet.sheet.width = Length::ZERO;
    assert_eq!(
        zero_sheet.validate(),
        Err(PageProfileError::SheetDimensionIsZero),
    );

    for paper_pattern in [
        PaperPattern::Dotted { spacing: Length::ZERO },
        PaperPattern::Ruled { spacing: Length::ZERO },
        PaperPattern::Squared { spacing: Length::ZERO },
    ] {
        let mut profile = base_profile(BindingEdge::Left);
        profile.paper_pattern = paper_pattern;
        assert_eq!(
            profile.validate(),
            Err(PageProfileError::PatternSpacingIsZero),
        );
    }
}

#[test]
fn binding_inset_and_top_clearance_cannot_consume_writable_region() {
    let mut binding = base_profile(BindingEdge::Left);
    binding.outer_margin = binding.printable_region.width;
    assert_eq!(
        binding.validate(),
        Err(PageProfileError::BindingInsetExhaustsPrintableRegion),
    );

    let mut top = base_profile(BindingEdge::Left);
    top.top_clearance = top.printable_region.height;
    assert_eq!(
        top.validate(),
        Err(PageProfileError::TopClearanceExhaustsPrintableRegion),
    );
}

#[test]
fn rounded_border_radius_must_fit_printable_rectangle() {
    let profile = base_profile(BindingEdge::Left);
    assert_eq!(profile.validate(), Ok(profile));

    let mut invalid = profile;
    invalid.corner_roundness = Length::from_micrometres(96_000);
    assert_eq!(
        invalid.validate(),
        Err(PageProfileError::CornerRoundnessExceedsPrintableRegion),
    );
}

#[test]
fn blank_and_custom_patterns_need_no_synthetic_spacing() {
    for paper_pattern in [PaperPattern::Blank, PaperPattern::Custom] {
        let mut profile = base_profile(BindingEdge::Left);
        profile.paper_pattern = paper_pattern;
        assert_eq!(profile.validate(), Ok(profile));
    }
}

#[test]
fn nonrounded_border_rejects_nonzero_corner_roundness() {
    for border_shape in [BorderShape::None, BorderShape::Rectangle] {
        let mut profile = base_profile(BindingEdge::Left);
        profile.border_shape = border_shape;
        assert_eq!(
            profile.validate(),
            Err(PageProfileError::CornerRoundnessRequiresRoundedBorder),
        );
        profile.corner_roundness = Length::ZERO;
        assert_eq!(profile.validate(), Ok(profile));
    }
}

#[test]
fn paper_mark_appearance_validates_without_complete_page_profile() {
    let rounded = PaperMarkAppearance {
        join: PaperMarkJoin::Rounded {
            radius: Length::from_micrometres(250),
        },
        maximum_ruler_error: Length::from_micrometres(200),
    };
    assert_eq!(rounded.validate(), Ok(rounded));

    let sharp = PaperMarkAppearance {
        join: PaperMarkJoin::Sharp,
        maximum_ruler_error: Length::from_micrometres(200),
    };
    assert_eq!(sharp.validate(), Ok(sharp));

    let invalid = PaperMarkAppearance {
        join: PaperMarkJoin::Rounded {
            radius: Length::ZERO,
        },
        maximum_ruler_error: Length::from_micrometres(200),
    };
    assert_eq!(
        invalid.validate(),
        Err(PageProfileError::PaperMarkRoundedJoinRadiusIsZero),
    );
}

#[test]
fn rounded_paper_mark_join_requires_positive_radius() {
    let mut profile = base_profile(BindingEdge::Left);
    profile.paper_mark_appearance.join =
        PaperMarkJoin::Rounded { radius: Length::ZERO };
    assert_eq!(
        profile.validate(),
        Err(PageProfileError::PaperMarkRoundedJoinRadiusIsZero),
    );

    profile.paper_mark_appearance.join = PaperMarkJoin::Sharp;
    assert_eq!(profile.validate(), Ok(profile));
}
