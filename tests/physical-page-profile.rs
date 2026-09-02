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
    PageProfileError, PaperMarkLayer, PaperPattern, Rect, SheetSize,
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
