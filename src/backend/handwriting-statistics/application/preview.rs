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
//   - Repeatable bounded numeric previews of admitted handwriting variables.
// - Must-Not:
//   - Claim calibrated distributions, exact median or variance, glyph paths,
//     pen strokes, natural handwriting, or cryptographic randomness.
// - Allows:
//   - Inputs: Valid profile, exact one-grapheme key, dimension, occurrence.
//   - Outputs: One dimension-bounded deterministic integer preview or refusal.
//   - Side effects: None.
// - Split-When:
//   - Calibrated statistical sampling or ink rendering needs its own engine.
// - Merge-When:
//   - Parameter previews become part of a reusable variation service.
// - Summary:
//   - Separates reproducible numeric variation from handwriting synthesis.
// - Description:
//   - Uses an integer-only bell-like jitter, clamped to authored limits.
// - Usage:
//   - Inspect previews before implementing shape-aware stroke rendering.
// - Defaults:
//   - Zero deviation returns the exact configured mean.
//

//! Deterministic *numeric* variation preview for a manual handwriting profile.
//!
//! The generated sequence is not a calibrated distribution and does not
//! reproduce exact variance, median, or observed extrema. In particular,
//! clipping near an envelope boundary changes its empirical mean and spread.
//! A future handwriting sampler must implement statistical calibration rather
//! than relabel this preview as generated pen strokes.

use atrament_unicode_grapheme_boundary_port::GraphemeBoundaryProvider;

use super::{
    Dimension, HandwritingStatisticsError, HandwritingStatisticsProfile,
    MAXIMUM_GRAPHEME_BYTES,
};

// Stable integer mixing, not the platform-dependent DefaultHasher.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const NORMALIZATION_DENOMINATOR: i64 = 65_536;

const fn mix(mut value: u64) -> u64 {
    value ^= value >> 30u32;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27u32;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31u32)
}

/// Produce a reproducible *numeric* preview, not handwriting or sampled ink.
///
/// Exact grapheme and occurrence identity make output stable across call
/// order. The profile's mean, deviation and envelope influence this preview;
/// authored median and exact target moments are **not** reproduced.
/// Clipping at bounds can reduce empirical variation. An independent
/// calibrated sampler is required before synthesizing real pen strokes.
///
/// # Errors
/// Refuses malformed profiles and invalid or multi-grapheme preview keys.
pub fn preview_value<Boundary>(
    profile: &HandwritingStatisticsProfile,
    grapheme: &str,
    dimension: Dimension,
    occurrence: u64,
    boundaries: &Boundary,
) -> Result<i32, HandwritingStatisticsError>
where
    Boundary: GraphemeBoundaryProvider,
{
    profile.validate(boundaries)?;
    if grapheme.is_empty()
        || grapheme.len() > MAXIMUM_GRAPHEME_BYTES
        || grapheme.chars().any(char::is_control)
        || boundaries.grapheme_count(grapheme) != 1
    {
        return Err(HandwritingStatisticsError::InvalidPreviewGrapheme);
    }
    let statistics = profile
        .statistics_for(grapheme, dimension)
        .ok_or(HandwritingStatisticsError::MissingGlobal(dimension))?;
    if statistics.standard_deviation == 0 {
        return Ok(statistics.mean);
    }
    let mut state = sample_seed(profile.seed, grapheme, dimension, occurrence);
    let mut centered_sum = 0i64;
    // A sum of twelve centered integers is approximately bell-shaped,
    // unlike a calibrated distribution with exact sample moments.
    for _ in 0u8..12u8 {
        state = mix(state.wrapping_add(0x9e37_79b9_7f4a_7c15));
        let chunk = i64::try_from(state >> 48u32)
            .map_err(|_error| {
                HandwritingStatisticsError::InvalidPreviewGrapheme
            })?;
        centered_sum = centered_sum
            .saturating_add(chunk.saturating_sub(32_768));
    }
    let offset = i64::from(statistics.standard_deviation)
        .saturating_mul(centered_sum)
        .checked_div(NORMALIZATION_DENOMINATOR)
        .ok_or(HandwritingStatisticsError::InvalidPreviewGrapheme)?;
    let value = i64::from(statistics.mean)
        .saturating_add(offset)
        .clamp(i64::from(statistics.minimum), i64::from(statistics.maximum));
    i32::try_from(value)
        .map_err(|_error| HandwritingStatisticsError::InvalidPreviewGrapheme)
}

fn sample_seed(
    seed: u64, grapheme: &str, dimension: Dimension, occurrence: u64,
) -> u64 {
    let mut hash = FNV_OFFSET ^ seed;
    for byte in grapheme
        .as_bytes()
        .iter()
        .copied()
        .chain([0xff])
        .chain(dimension.key().bytes())
        .chain([0xfe])
        .chain(occurrence.to_le_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    mix(hash)
}
