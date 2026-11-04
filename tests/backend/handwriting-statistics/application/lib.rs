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
//   - Regression coverage for bounded editable handwriting and glyph values.
// - Must-Not:
//   - Pretend profile validation implies rendered handwriting or file support.
// - Allows:
//   - Inputs: Typed manual statistics and real Unicode grapheme boundaries.
//   - Outputs: Equality and typed-error assertions.
//   - Side effects: None beyond test-local allocation.
// - Split-When:
//   - File codecs or deterministic rendering acquire independent tests.
// - Merge-When:
//   - Profile statistics validations move to a shared domain harness.
// - Summary:
//   - Checks accepted values, stale refusal, and atomic candidate edits.
// - Description:
//   - Tests signed offsets, feasible deviations, and NFC/NFD exact keys.
// - Usage:
//   - Run cargo test -p atrament_handwriting_statistics.
// - Defaults:
//   - Uses the pinned real Unicode boundary adapter.
//

//! Executable statistical handwriting profile validation tests.
#![expect(
    clippy::unwrap_used,
    reason = "Test fixtures require explicit valid inputs"
)]
#![expect(
    clippy::non_ascii_literal,
    reason = "Exact Spanish and Unicode glyph fixtures"
)]
#![expect(
    clippy::default_numeric_fallback,
    reason = "Fixed integer fixture measurements"
)]
#![expect(
    clippy::indexing_slicing,
    reason = "Indexes refer to previously constructed fixtures"
)]

use atrament_handwriting_statistics::{
    Dimension, DimensionStatistics, GlyphStatistics,
    HandwritingStatisticsError, HandwritingStatisticsProfile,
    MAXIMUM_GLYPH_OVERRIDES, STATISTICS_PROFILE_VERSION, ScalarStatistics,
    preview_value,
    ScalarStatisticsError, Unit,
};
use atrament_unicode_grapheme_boundary_port as _;
use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

const BOUNDARIES: UnicodeGraphemeSegmentation = UnicodeGraphemeSegmentation;

const fn stat(
    minimum: i32,
    maximum: i32,
    mean: i32,
    median: i32,
    deviation: u32,
) -> ScalarStatistics {
    ScalarStatistics {
        maximum,
        mean,
        median,
        minimum,
        standard_deviation: deviation,
    }
}

const fn parameter(
    dimension: Dimension,
    statistics: ScalarStatistics,
) -> DimensionStatistics {
    DimensionStatistics {
        dimension,
        statistics,
    }
}

#[test]
fn baseline_has_ten_distinct_valid_controls_and_explicit_units() {
    let profile = HandwritingStatisticsProfile::baseline(17);
    assert_eq!(profile.version, STATISTICS_PROFILE_VERSION);
    assert_eq!(profile.seed, 17);
    assert_eq!(profile.globals.len(), Dimension::ALL.len());
    assert_eq!(profile.validate(&BOUNDARIES), Ok(()));
    assert_eq!(Dimension::Slant.unit(), Unit::Millidegrees);
    assert_eq!(Dimension::Width.unit(), Unit::Micrometres);
    assert_eq!(Dimension::Pressure.unit(), Unit::PartsPerThousand);
    assert_eq!(
        profile.statistics_for("ñ", Dimension::Height),
        Some(stat(3_000, 3_000, 3_000, 3_000, 0))
    );
}

#[test]
fn globally_editable_mean_median_min_max_and_deviation_are_preserved() {
    let baseline = HandwritingStatisticsProfile::baseline(9);
    let candidate = baseline
        .with_global(
            parameter(Dimension::Height, stat(2_000, 4_000, 3_000, 3_200, 350)),
            &BOUNDARIES,
        )
        .unwrap();
    assert_eq!(
        candidate.statistics_for("a", Dimension::Height),
        Some(stat(2_000, 4_000, 3_000, 3_200, 350))
    );
    assert_eq!(
        baseline.statistics_for("a", Dimension::Height),
        Some(stat(3_000, 3_000, 3_000, 3_000, 0))
    );
    assert_eq!(candidate.validate(&BOUNDARIES), Ok(()));
}

#[test]
fn nfc_nfd_and_spanish_glyph_overrides_inherit_remaining_fields() {
    let baseline = HandwritingStatisticsProfile::baseline(42);
    let entries = [
        ("ñ", 3_000),
        ("n\u{303}", 3_200),
        ("«", 3_400),
        ("¿", 3_600),
    ];
    let mut candidate = baseline.clone();
    for (glyph, mean) in entries {
        candidate = candidate
            .with_glyph(
                glyph,
                parameter(
                    Dimension::Height,
                    stat(2_000, 5_000, mean, mean, 300),
                ),
                &BOUNDARIES,
            )
            .unwrap();
    }
    assert_eq!(candidate.glyphs.len(), 4);
    assert_ne!(
        candidate.statistics_for("ñ", Dimension::Height),
        candidate.statistics_for("n\u{303}", Dimension::Height)
    );
    assert_eq!(
        candidate.statistics_for("ñ", Dimension::InkThickness),
        baseline.statistics_for("ñ", Dimension::InkThickness)
    );
    assert_eq!(candidate.validate(&BOUNDARIES), Ok(()));
}

#[test]
fn editing_existing_glyph_dimension_preserves_other_overrides() {
    let baseline = HandwritingStatisticsProfile::baseline(0);
    let first = baseline
        .with_glyph(
            "ü",
            parameter(Dimension::Height, stat(2_500, 3_500, 3_000, 3_000, 100)),
            &BOUNDARIES,
        )
        .unwrap();
    let second = first
        .with_glyph(
            "ü",
            parameter(Dimension::Slant, stat(-30_000, 30_000, 0, 0, 15_000)),
            &BOUNDARIES,
        )
        .unwrap();
    let third = second
        .with_glyph(
            "ü",
            parameter(Dimension::Height, stat(2_000, 4_000, 3_500, 3_400, 200)),
            &BOUNDARIES,
        )
        .unwrap();
    assert_eq!(third.glyphs.len(), 1);
    assert_eq!(third.glyphs[0].parameters.len(), 2);
    assert_eq!(
        third.statistics_for("ü", Dimension::Slant),
        second.statistics_for("ü", Dimension::Slant)
    );
    assert_eq!(
        third.statistics_for("ü", Dimension::Height),
        Some(stat(2_000, 4_000, 3_500, 3_400, 200))
    );
}

#[test]
fn impossible_global_update_never_mutates_the_source() {
    let original = HandwritingStatisticsProfile::baseline(19);
    let invalid =
        parameter(Dimension::Height, stat(2_000, 4_000, 2_000, 3_000, 1));
    assert_eq!(
        original.with_global(invalid, &BOUNDARIES),
        Err(HandwritingStatisticsError::InvalidGlobal(
            Dimension::Height,
            ScalarStatisticsError::DeviationImpossible
        ))
    );
    assert_eq!(original, HandwritingStatisticsProfile::baseline(19));
}

#[test]
fn range_checks_reject_negative_size_and_inverted_bounds() {
    assert_eq!(
        stat(-1, 4_000, 3_000, 3_000, 0).validate(Dimension::Height),
        Err(ScalarStatisticsError::OutsideDimensionBounds)
    );
    assert_eq!(
        stat(4_000, 3_000, 3_000, 3_000, 0).validate(Dimension::Height),
        Err(ScalarStatisticsError::InvertedEnvelope)
    );
    assert_eq!(
        stat(2_000, 4_000, 1_000, 3_000, 0).validate(Dimension::Height),
        Err(ScalarStatisticsError::MeanOutsideEnvelope)
    );
    assert_eq!(
        stat(2_000, 4_000, 3_000, 5_000, 0).validate(Dimension::Height),
        Err(ScalarStatisticsError::MedianOutsideEnvelope)
    );
    assert_eq!(
        stat(2_000, 4_000, 3_000, 3_000, 1_001).validate(Dimension::Height),
        Err(ScalarStatisticsError::DeviationImpossible)
    );
    assert_eq!(
        stat(-5_000, 5_000, 0, 0, 2_000).validate(Dimension::BaselineOffset),
        Ok(())
    );
}

#[test]
fn duplicate_and_missing_global_dimensions_fail_closed() {
    let mut duplicate = HandwritingStatisticsProfile::baseline(0);
    duplicate.globals.push(duplicate.globals[0]);
    assert_eq!(
        duplicate.validate(&BOUNDARIES),
        Err(HandwritingStatisticsError::DuplicateGlobal(
            Dimension::BaselineOffset
        ))
    );
    let mut missing = HandwritingStatisticsProfile::baseline(0);
    assert!(missing.globals.pop().is_some());
    assert_eq!(
        missing.validate(&BOUNDARIES),
        Err(HandwritingStatisticsError::MissingGlobal(Dimension::Width))
    );
}

#[test]
fn stale_versions_and_duplicate_graphemes_refuse() {
    let mut stale_profile = HandwritingStatisticsProfile::baseline(3);
    stale_profile.version = "atrament.handwriting.statistics.v2".into();
    assert_eq!(
        stale_profile.validate(&BOUNDARIES),
        Err(HandwritingStatisticsError::UnsupportedVersion)
    );
    let mut profile = HandwritingStatisticsProfile::baseline(3);
    let glyph = GlyphStatistics {
        grapheme: "á".into(),
        parameters: vec![parameter(
            Dimension::Height,
            stat(2_000, 4_000, 3_000, 3_000, 100),
        )],
    };
    profile.glyphs = vec![glyph.clone(), glyph];
    assert_eq!(
        profile.validate(&BOUNDARIES),
        Err(HandwritingStatisticsError::DuplicateGrapheme {
            first: 0,
            duplicate: 1
        })
    );
}

#[test]
fn multi_grapheme_and_empty_keys_are_rejected() {
    let baseline = HandwritingStatisticsProfile::baseline(3);
    let param =
        parameter(Dimension::Height, stat(2_000, 4_000, 3_000, 3_000, 100));
    let long = "a".repeat(65);
    for key in ["", "ab", "\n", long.as_str()] {
        assert_eq!(
            baseline.with_glyph(key, param, &BOUNDARIES),
            Err(HandwritingStatisticsError::InvalidGrapheme(0))
        );
    }
}

#[test]
fn duplicate_glyph_dimension_and_invalid_override_are_refused() {
    let mut profile = HandwritingStatisticsProfile::baseline(0);
    let entry =
        parameter(Dimension::Height, stat(2_000, 4_000, 3_000, 3_000, 100));
    profile.glyphs.push(GlyphStatistics {
        grapheme: "A".into(),
        parameters: vec![entry, entry],
    });
    assert_eq!(
        profile.validate(&BOUNDARIES),
        Err(HandwritingStatisticsError::DuplicateGlyphDimension {
            glyph_index: 0,
            dimension: Dimension::Height
        })
    );
    profile.glyphs[0].parameters = vec![parameter(
        Dimension::InkThickness,
        stat(0, 6_000, 300, 300, 10),
    )];
    assert_eq!(
        profile.validate(&BOUNDARIES),
        Err(HandwritingStatisticsError::InvalidGlyphDimension {
            glyph_index: 0,
            dimension: Dimension::InkThickness,
            reason: ScalarStatisticsError::OutsideDimensionBounds
        })
    );
}

#[test]
fn exact_glyph_limit_rejects_additions_without_mutating_source() {
    let mut profile = HandwritingStatisticsProfile::baseline(0);
    let entry =
        parameter(Dimension::Height, stat(2_000, 4_000, 3_000, 3_000, 100));
    profile.glyphs = (0..MAXIMUM_GLYPH_OVERRIDES)
        .map(|index| GlyphStatistics {
            grapheme: char::from_u32(0x4E00 + u32::try_from(index).unwrap())
                .unwrap()
                .to_string(),
            parameters: vec![entry],
        })
        .collect();
    assert_eq!(profile.validate(&BOUNDARIES), Ok(()));
    let original = profile.clone();
    assert_eq!(
        profile.with_glyph("ñ", entry, &BOUNDARIES),
        Err(HandwritingStatisticsError::TooManyGlyphs)
    );
    assert_eq!(profile, original);
}

#[test]
fn extreme_deviation_fails_without_integer_overflow() {
    assert_eq!(
        stat(2_000, 4_000, 3_000, 3_000, u32::MAX).validate(Dimension::Height),
        Err(ScalarStatisticsError::DeviationImpossible),
    );
}

#[test]
fn stable_parameter_keys_round_trip_without_accepting_unknown_names() {
    for dimension in Dimension::ALL {
        assert_eq!(Dimension::from_key(dimension.key()), Some(dimension));
    }
    assert_eq!(Dimension::from_key("rotation"), None);
    assert_eq!(Dimension::from_key("HEIGHT"), None);
}

#[test]
fn median_and_mean_must_be_jointly_possible_within_envelope() {
    assert_eq!(
        stat(2_000, 4_000, 2_200, 3_900, 0).validate(Dimension::Height),
        Err(ScalarStatisticsError::MedianMeanIncompatible),
    );
    assert_eq!(
        stat(2_000, 4_000, 3_000, 4_000, 0).validate(Dimension::Height),
        Ok(()),
    );
}

#[test]
fn removing_one_override_restores_global_without_changing_other_glyphs() {
    let entry =
        parameter(Dimension::Height, stat(2_000, 4_000, 3_500, 3_400, 200));
    let baseline = HandwritingStatisticsProfile::baseline(77);
    let authored = baseline
        .with_glyph("ñ", entry, &BOUNDARIES)
        .unwrap()
        .with_glyph(
            "ñ",
            parameter(Dimension::Slant, stat(-2_000, 2_000, 0, 0, 100)),
            &BOUNDARIES,
        )
        .unwrap()
        .with_glyph("n\u{303}", entry, &BOUNDARIES)
        .unwrap();

    let removed_height = authored
        .without_glyph_override("ñ", Some(Dimension::Height), &BOUNDARIES)
        .unwrap();
    assert_eq!(removed_height.glyphs.len(), 2);
    assert_eq!(
        removed_height.statistics_for("ñ", Dimension::Height),
        baseline.statistics_for("ñ", Dimension::Height),
    );
    assert_eq!(
        removed_height.statistics_for("ñ", Dimension::Slant),
        authored.statistics_for("ñ", Dimension::Slant),
    );
    assert_eq!(
        removed_height.statistics_for("n\u{303}", Dimension::Height),
        authored.statistics_for("n\u{303}", Dimension::Height),
    );

    let removed_last = removed_height
        .without_glyph_override("ñ", Some(Dimension::Slant), &BOUNDARIES)
        .unwrap();
    assert_eq!(removed_last.glyphs.len(), 1);
    assert_eq!(removed_last.glyphs[0].grapheme, "n\u{303}");
    assert_eq!(removed_last.validate(&BOUNDARIES), Ok(()));
    let removed_full = authored
        .without_glyph_override("ñ", None, &BOUNDARIES)
        .unwrap();
    assert_eq!(removed_full.glyphs, removed_last.glyphs);
    assert_eq!(authored.glyphs.len(), 2);
}

#[test]
fn removing_unowned_exact_glyph_or_dimension_fails_without_mutation() {
    let baseline = HandwritingStatisticsProfile::baseline(11);
    let authored = baseline
        .with_glyph(
            "ñ",
            parameter(Dimension::Height, stat(2_000, 4_000, 3_000, 3_000, 100)),
            &BOUNDARIES,
        )
        .unwrap();
    assert_eq!(
        authored.without_glyph_override("n\u{303}", None, &BOUNDARIES),
        Err(HandwritingStatisticsError::MissingGlyphOverride),
    );
    assert_eq!(
        authored.without_glyph_override(
            "ñ", Some(Dimension::Width), &BOUNDARIES,
        ),
        Err(HandwritingStatisticsError::MissingGlyphDimension(
            Dimension::Width,
        )),
    );
    assert_eq!(
        authored.without_glyph_override("ab", None, &BOUNDARIES),
        Err(HandwritingStatisticsError::MissingGlyphOverride),
    );
    assert_eq!(authored.glyphs.len(), 1);
    assert_eq!(baseline.glyphs.len(), 0);
}

#[test]
fn numeric_preview_replays_per_seed_exact_glyph_and_occurrence() {
    let profile = HandwritingStatisticsProfile::baseline(42)
        .with_global(
            parameter(Dimension::Height, stat(2_000, 4_000, 3_000, 3_000, 450)),
            &BOUNDARIES,
        )
        .unwrap()
        .with_glyph(
            "ñ",
            parameter(Dimension::Height, stat(4_500, 5_500, 5_000, 5_000, 250)),
            &BOUNDARIES,
        )
        .unwrap();
    let sample = |key, ordinal| {
        preview_value(&profile, key, Dimension::Height, ordinal, &BOUNDARIES)
            .unwrap()
    };
    let exact = (0u64..128u64).map(|i| sample("ñ", i)).collect::<Vec<_>>();
    assert_eq!(
        exact,
        (0u64..128u64).map(|i| sample("ñ", i)).collect::<Vec<_>>(),
    );
    assert!(exact.iter().all(|&value| (4_500..=5_500).contains(&value)));
    assert!(exact.iter().any(|&value| value != 5_000));
    assert!(exact.windows(2).any(|pair| pair.first() != pair.get(1)));
    let nfd = (0u64..128u64)
        .map(|i| sample("n\u{303}", i))
        .collect::<Vec<_>>();
    assert!(nfd.iter().all(|&value| (2_000..=4_000).contains(&value)));
    assert_ne!(exact, nfd, "NFC override is not applied to NFD");
    assert_eq!(
        sample("ñ", 5), exact[5],
        "call order cannot advance a global RNG",
    );
    assert_eq!(
        preview_value(&profile, "ñ", Dimension::Width, u64::MAX, &BOUNDARIES),
        Ok(2_200),
        "a non-overridden zero-deviation dimension inherits the exact mean",
    );

    let mut alternate_seed = profile.clone();
    alternate_seed.seed = 43;
    let changed = (0u64..128u64).map(|i| {
        preview_value(&alternate_seed, "ñ", Dimension::Height, i, &BOUNDARIES)
            .unwrap()
    }).collect::<Vec<_>>();
    assert_ne!(exact, changed, "changing the seed changes preview variation");
}

#[test]
fn numeric_preview_rejects_invalid_exact_keys_and_malformed_profiles() {
    let profile = HandwritingStatisticsProfile::baseline(u64::MAX);
    for invalid in ["", "ab", "ñ\n", "\u{1b}", &"a".repeat(65)] {
        assert_eq!(
            preview_value(&profile, invalid, Dimension::Height, 0, &BOUNDARIES),
            Err(HandwritingStatisticsError::InvalidPreviewGrapheme),
        );
    }
    assert_eq!(
        preview_value(
            &profile, "n\u{303}", Dimension::Height, u64::MAX,
            &BOUNDARIES,
        ),
        Ok(3_000),
    );
    let mut stale = profile;
    stale.version = "invalid".into();
    assert_eq!(
        preview_value(&stale, "ñ", Dimension::Height, 0, &BOUNDARIES),
        Err(HandwritingStatisticsError::UnsupportedVersion),
    );
}
