// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE
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

//! Physical page geometry regression tests.

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

const fn base_profile(binding_edge: BindingEdge) -> PageProfile {
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

const fn next_profile_value(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *seed >> 32
}

const fn next_validation_value(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(2_862_933_555_777_941_757)
        .wrapping_add(3_037_000_493);
    *seed
}

const fn next_validation_selector(seed: &mut u64) -> u64 {
    next_validation_value(seed) >> 32
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
    let right = u128::from(printable.x.micrometres())
        .checked_add(u128::from(width))
        .ok_or(PageProfileError::PrintableRegionOutsideSheet)?;
    let bottom = u128::from(printable.y.micrometres())
        .checked_add(u128::from(height))
        .ok_or(PageProfileError::PrintableRegionOutsideSheet)?;
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
    if profile.border_shape == BorderShape::RoundedRectangle {
        let doubled = u128::from(profile.corner_roundness.micrometres()) * 2;
        if doubled > u128::from(profile.printable_region.width.micrometres())
            || doubled
                > u128::from(profile.printable_region.height.micrometres())
        {
            return Err(PageProfileError::CornerRoundnessExceedsPrintableRegion);
        }
    } else if profile.corner_roundness != Length::ZERO {
        return Err(PageProfileError::CornerRoundnessRequiresRoundedBorder);
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
        .checked_add(u128::from(printable.width.micrometres()))
        .ok_or(PageProfileError::PrintableRegionOutsideSheet)?;
    let bottom = u128::from(printable.y.micrometres())
        .checked_add(u128::from(printable.height.micrometres()))
        .ok_or(PageProfileError::PrintableRegionOutsideSheet)?;
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
    let mut profile = base_profile(match next_validation_selector(seed) & 3 {
        0 => BindingEdge::Bottom,
        1 => BindingEdge::Left,
        2 => BindingEdge::Right,
        _ => BindingEdge::Top,
    });
    profile.orientation = if next_validation_selector(seed) & 1 == 0 {
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
    match case.checked_rem(12).unwrap_or_default() {
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
            let jitter = next_validation_selector(seed)
                .checked_rem(5_000).unwrap_or_default();
            profile.top_clearance = Length::from_micrometres(jitter);
            profile.outer_margin = Length::from_micrometres(
                jitter.checked_div(2).unwrap_or_default(),
            );
            profile.writing_inset = Length::from_micrometres(
                jitter.checked_div(3).unwrap_or_default(),
            );
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
    let mut seed = 0xd1b5_4a32_d192_ed03u64;
    let mut valid = 0usize;
    let mut invalid = 0usize;
    let mut seen_errors = [false; 9];
    let mut seen_binding_orientation = [false; 8];
    for case in 0..CASES {
        let profile = generated_validation_profile(&mut seed, case);
        let binding_index: usize = match profile.binding_edge {
            BindingEdge::Bottom => 0,
            BindingEdge::Left => 1,
            BindingEdge::Right => 2,
            BindingEdge::Top => 3,
        };
        let orientation_index: usize = match profile.orientation {
            Orientation::Portrait => 0,
            Orientation::Landscape => 1,
        };
        let index = binding_index.checked_mul(2)
            .and_then(|value| value.checked_add(orientation_index));
        assert!(index.and_then(|value| seen_binding_orientation.get_mut(value))
            .is_some_and(|seen| { *seen = true; true }));
        let expected = reference_profile_validation(profile);
        if let Err(reason) = expected {
            invalid = invalid.saturating_add(1);
            let error_index = match reason {
                PageProfileError::BindingInsetExhaustsPrintableRegion => 0,
                PageProfileError::CornerRoundnessExceedsPrintableRegion => 1,
                PageProfileError::CornerRoundnessRequiresRoundedBorder => 2,
                PageProfileError::PaperMarkRoundedJoinRadiusIsZero => 3,
                PageProfileError::PatternSpacingIsZero => 4,
                PageProfileError::PrintableRegionIsEmpty => 5,
                PageProfileError::PrintableRegionOutsideSheet => 6,
                PageProfileError::SheetDimensionIsZero => 7,
                PageProfileError::TopClearanceExhaustsPrintableRegion => 8,
            };
            assert!(seen_errors.get_mut(error_index)
                .is_some_and(|seen| { *seen = true; true }));
        } else {
            valid = valid.saturating_add(1);
        }
        assert_eq!(
            profile.validate().map(|_valid| ()),
            expected,
            "profile validation mismatch in generated case {case}",
        );
        let sealed = profile.validated();
        assert_eq!(
            sealed.map(
                atrament_physical_page_profile::ValidatedPageProfile::profile,
            ).map(|_profile| ()),
            expected,
            "sealed profile precedence mismatch in generated case {case}",
        );
        if let Ok(validated) = sealed {
            assert_eq!(
                Ok(validated.oriented_sheet()), profile.oriented_sheet(),
            );
            assert_eq!(
                Ok(validated.writable_region()),
                reference_writable_region(profile),
            );
        }
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
    assert!(seen_errors.into_iter().all(|seen| seen));
    assert!(seen_binding_orientation.into_iter().all(|seen| seen));
}

// Independent reference sampling refuses zero spans and overflow instead of
// hiding invalid generated dimensions through wrapping or saturation.
const fn sample_offset(seed: &mut u64, exclusive_max: u64) -> Option<u64> {
    next_profile_value(seed).checked_rem(exclusive_max)
}

const fn sample_positive_extent(
    seed: &mut u64, full_span: u64, origin: u64,
) -> Option<u64> {
    let Some(room) = full_span.checked_sub(origin) else {
        return None;
    };
    let Some(offset) = sample_offset(seed, room) else {
        return None;
    };
    offset.checked_add(1)
}

// Compute the expected geometry without invoking production layout or
// the separate validation-precedence reference oracle.
fn expected_valid_region(
    binding_edge: BindingEdge, origin_x: u64, origin_y: u64,
    input_width: u64, input_height: u64, top_clearance: u64,
    binding_inset: u64,
) -> Option<Rect> {
    let top = origin_y.checked_add(top_clearance)?;
    let remaining_height = input_height.checked_sub(top_clearance)?;
    let (output_x, output_y, output_width, output_height) = match binding_edge {
        BindingEdge::Bottom => (
            origin_x, top, input_width,
            remaining_height.checked_sub(binding_inset)?,
        ),
        BindingEdge::Left => (
            origin_x.checked_add(binding_inset)?, top,
            input_width.checked_sub(binding_inset)?, remaining_height,
        ),
        BindingEdge::Right => (
            origin_x, top, input_width.checked_sub(binding_inset)?,
            remaining_height,
        ),
        BindingEdge::Top => (
            origin_x, top.checked_add(binding_inset)?, input_width,
            remaining_height.checked_sub(binding_inset)?,
        ),
    };
    Some(Rect {
        height: Length::from_micrometres(output_height),
        width: Length::from_micrometres(output_width),
        x: Length::from_micrometres(output_x),
        y: Length::from_micrometres(output_y),
    })
}

#[test]
fn valid_profiles_match_writable_region_reference_oracle()
-> Result<(), String> {
    const CASES: usize = 20_000;
    let mut seed = 0x5eed_9a6e_2026u64;
    let mut seen_binding_edges = [false; 4];
    let mut seen_orientations = [false; 2];
    for case in 0..CASES {
        let binding_index = usize::try_from(next_profile_value(&mut seed) & 3)
            .map_err(|error| format!("invalid binding index: {error}"))?;
        *seen_binding_edges.get_mut(binding_index)
            .ok_or("binding index out of range")? = true;
        let binding_edge = match binding_index {
            0 => BindingEdge::Bottom,
            1 => BindingEdge::Left,
            2 => BindingEdge::Right,
            _ => BindingEdge::Top,
        };
        let orientation_index =
            usize::from(next_profile_value(&mut seed) & 1 != 0);
        *seen_orientations.get_mut(orientation_index)
            .ok_or("orientation index out of range")? = true;
        let orientation = if orientation_index == 0 {
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
        let x = sample_offset(&mut seed, sheet_width)
            .ok_or("generated horizontal span is empty")?;
        let y = sample_offset(&mut seed, sheet_height)
            .ok_or("generated vertical span is empty")?;
        let width = sample_positive_extent(&mut seed, sheet_width, x)
            .ok_or("generated printable width is invalid")?;
        let height = sample_positive_extent(&mut seed, sheet_height, y)
            .ok_or("generated printable height is invalid")?;
        let top_clearance = sample_offset(&mut seed, height)
            .ok_or("generated top clearance is invalid")?;
        let remaining_height = height.checked_sub(top_clearance)
            .ok_or("generated top clearance exhausts page")?;
        let binding_capacity = match binding_edge {
            BindingEdge::Bottom | BindingEdge::Top => remaining_height,
            BindingEdge::Left | BindingEdge::Right => width,
        };
        let binding_inset = sample_offset(&mut seed, binding_capacity)
            .ok_or("generated binding capacity is empty")?;
        let outer_margin = binding_inset.checked_div(2)
            .ok_or("generated outer margin divisor invalid")?;
        let writing_inset = binding_inset.checked_sub(outer_margin)
            .ok_or("generated writing inset underflows")?;
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
        let expected = expected_valid_region(
            binding_edge, x, y, width, height, top_clearance, binding_inset,
        )
        .ok_or("generated writable rectangle is invalid")?;
        if profile.validate() != Ok(profile) {
            return Err(format!(
                "valid generated profile refused in case {case}",
            ));
        }
        let validated = profile.validated()
            .map_err(|error| format!("case {case}: {error:?}"))?;
        if validated.profile() != profile
            || profile.oriented_sheet() != Ok(validated.oriented_sheet())
            || validated.writable_region() != expected
            || profile.writable_region() != Ok(expected) {
            return Err(format!("reference geometry mismatch in case {case}"));
        }
    }
    if !seen_binding_edges.into_iter().all(|seen| seen)
        || !seen_orientations.into_iter().all(|seen| seen) {
        return Err("generated geometry missed a binding/orientation".into());
    }
    Ok(())
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
