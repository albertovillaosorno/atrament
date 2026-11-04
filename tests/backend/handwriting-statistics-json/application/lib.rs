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
//   - Native Rust JSON roundtrip and strict refusal regression fixtures.
// - Must-Not:
//   - Claim file persistence, preview rendering, or cross-platform printing.
// - Allows:
//   - Inputs: Authored UTF-8 JSON and typed statistical profile fixtures.
//   - Outputs: Lossless equality and typed refusal assertions.
//   - Side effects: None beyond process-local allocation.
// - Split-When:
//   - Filesystem path and overwrite operations become separate integrations.
// - Merge-When:
//   - A complete notebook profile codec replaces this focused test boundary.
// - Summary:
//   - Proves accepted data survive and malformed or stale profiles refuse.
// - Description:
//   - Exercises NFD, quote escaping, unknown keys, duplicates, large values.
// - Usage:
//   - Run cargo test -p atrament_handwriting_statistics_json.
// - Defaults:
//   - Uses the existing Unicode grapheme segmentation adapter.
//

//! Strict manually editable handwriting JSON codec regressions.
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

use atrament_handwriting_statistics::{
    Dimension, DimensionStatistics, HandwritingStatisticsError,
    HandwritingStatisticsProfile, ScalarStatistics, ScalarStatisticsError,
};
use atrament_handwriting_statistics_json::{
    HandwritingStatisticsJsonError, MAXIMUM_STATISTICS_JSON_BYTES,
    decode_statistics_json, encode_statistics_json,
};
use atrament_profile_json_syntax::ProfileJsonSyntaxError;
use atrament_unicode_grapheme_boundary_port as _;
use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;
use std::str::from_utf8;

const BOUNDARIES: UnicodeGraphemeSegmentation = UnicodeGraphemeSegmentation;

const fn stat(
    minimum: i32,
    maximum: i32,
    mean: i32,
    median: i32,
    deviation: u32,
) -> ScalarStatistics {
    ScalarStatistics {
        minimum,
        maximum,
        mean,
        median,
        standard_deviation: deviation,
    }
}

const fn edit(
    dimension: Dimension,
    statistics: ScalarStatistics,
) -> DimensionStatistics {
    DimensionStatistics {
        dimension,
        statistics,
    }
}

#[test]
fn baseline_encode_decode_encode_is_byte_identical() {
    let original = HandwritingStatisticsProfile::baseline(9);
    let bytes = encode_statistics_json(&original, &BOUNDARIES).unwrap();
    let decoded = decode_statistics_json(&bytes, &BOUNDARIES).unwrap();
    assert_eq!(decoded, original);
    assert_eq!(
        encode_statistics_json(&decoded, &BOUNDARIES).unwrap(),
        bytes
    );
}

#[test]
fn nfc_nfd_guillemets_and_quote_glyph_overrides_survive() {
    let baseline = HandwritingStatisticsProfile::baseline(73)
        .with_global(
            edit(Dimension::Slant, stat(-20_000, 20_000, 0, 0, 10_000)),
            &BOUNDARIES,
        )
        .unwrap();
    let mut profile = baseline;
    for (glyph, height) in [
        ("ñ", 2_500),
        ("n\u{303}", 3_000),
        ("«", 3_500),
        ("\"", 3_600),
        ("\\", 3_700),
    ] {
        profile = profile
            .with_glyph(
                glyph,
                edit(
                    Dimension::Height,
                    stat(2_000, 4_000, height, height, 100),
                ),
                &BOUNDARIES,
            )
            .unwrap();
    }
    let bytes = encode_statistics_json(&profile, &BOUNDARIES).unwrap();
    let decoded = decode_statistics_json(&bytes, &BOUNDARIES).unwrap();
    assert_eq!(decoded, profile);
    let json = from_utf8(&bytes).unwrap();
    assert!(json.contains(r#""grapheme":"ñ""#));
    assert!(json.contains(r#""grapheme":"\\""#));
}

#[test]
fn syntax_gate_rejects_duplicate_root_and_nested_keys() {
    let root_duplicate =
        br#"{"version":"v","version":"v2","seed":0,"globals":[],"glyphs":[]}"#;
    assert_eq!(
        decode_statistics_json(root_duplicate, &BOUNDARIES),
        Err(HandwritingStatisticsJsonError::Syntax(
            ProfileJsonSyntaxError::DuplicateObjectKey
        ))
    );
    let nested_duplicate = concat!(
        r#"{"version":"v","seed":0,"globals":[{"dimension":"height","#,
        r#""dimension":"width"}],"glyphs":[]}"#,
    );
    assert_eq!(
        decode_statistics_json(nested_duplicate.as_bytes(), &BOUNDARIES),
        Err(HandwritingStatisticsJsonError::Syntax(
            ProfileJsonSyntaxError::DuplicateObjectKey
        ))
    );
}

#[test]
fn unknown_fields_and_missing_required_fields_refuse_without_defaults() {
    let original = HandwritingStatisticsProfile::baseline(0);
    let encoded = String::from_utf8(
        encode_statistics_json(&original, &BOUNDARIES).unwrap(),
    )
    .unwrap();
    let unknown_root =
        encoded.replace("\"seed\":0", "\"seed\":0,\"unsafe\":true");
    assert_eq!(
        decode_statistics_json(unknown_root.as_bytes(), &BOUNDARIES),
        Err(HandwritingStatisticsJsonError::UnknownField)
    );
    let missing_seed = encoded.replace("\"seed\":0,", "");
    assert_eq!(
        decode_statistics_json(missing_seed.as_bytes(), &BOUNDARIES),
        Err(HandwritingStatisticsJsonError::MissingField)
    );
    let unknown_nested = encoded.replace(
        "\"dimension\":\"height\"",
        "\"dimension\":\"height\",\"bogus\":1",
    );
    assert_eq!(
        decode_statistics_json(unknown_nested.as_bytes(), &BOUNDARIES),
        Err(HandwritingStatisticsJsonError::UnknownField)
    );
}

#[test]
fn unsupported_version_and_impossible_deviation_fail_typed() {
    let original = HandwritingStatisticsProfile::baseline(0);
    let encoded = String::from_utf8(
        encode_statistics_json(&original, &BOUNDARIES).unwrap(),
    )
    .unwrap();
    let stale = encoded.replace(
        "atrament.handwriting.statistics.v1",
        "atrament.handwriting.statistics.v2",
    );
    assert_eq!(
        decode_statistics_json(stale.as_bytes(), &BOUNDARIES),
        Err(HandwritingStatisticsJsonError::InvalidProfile(
            HandwritingStatisticsError::UnsupportedVersion
        ))
    );
    let bad = encoded.replacen(
        "\"standard_deviation\":0",
        "\"standard_deviation\":1",
        1,
    );
    assert_eq!(
        decode_statistics_json(bad.as_bytes(), &BOUNDARIES),
        Err(HandwritingStatisticsJsonError::InvalidProfile(
            HandwritingStatisticsError::InvalidGlobal(
                Dimension::BaselineOffset,
                ScalarStatisticsError::DeviationImpossible
            )
        ))
    );
}

#[test]
fn signed_slant_and_baseline_are_integer_only() {
    let original = HandwritingStatisticsProfile::baseline(0)
        .with_global(
            edit(
                Dimension::Slant,
                stat(-45_000, 45_000, -10_000, -9_000, 10_000),
            ),
            &BOUNDARIES,
        )
        .unwrap();
    let bytes = encode_statistics_json(&original, &BOUNDARIES).unwrap();
    assert_eq!(decode_statistics_json(&bytes, &BOUNDARIES), Ok(original));
    let decimals = String::from_utf8(bytes)
        .unwrap()
        .replace("\"mean\":-10000", "\"mean\":-10000.5");
    assert_eq!(
        decode_statistics_json(decimals.as_bytes(), &BOUNDARIES),
        Err(HandwritingStatisticsJsonError::InvalidFieldValue)
    );
}

#[test]
fn oversized_input_is_rejected_before_semantic_decoding() {
    let huge = vec![b' '; MAXIMUM_STATISTICS_JSON_BYTES + 1];
    assert_eq!(
        decode_statistics_json(&huge, &BOUNDARIES),
        Err(HandwritingStatisticsJsonError::Syntax(
            ProfileJsonSyntaxError::MaximumBytesExceeded
        ))
    );
}

#[test]
fn an_invalid_profile_cannot_be_encoded_as_accepted_json() {
    let mut original = HandwritingStatisticsProfile::baseline(0);
    original.globals.clear();
    assert_eq!(
        encode_statistics_json(&original, &BOUNDARIES),
        Err(HandwritingStatisticsJsonError::InvalidProfile(
            HandwritingStatisticsError::MissingGlobal(
                Dimension::BaselineOffset
            )
        ))
    );
}
