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
//   - Bounded, editable manual handwriting statistics and glyph overrides.
// - Must-Not:
//   - Perform filesystem I/O, parse JSON, render strokes, infer calibration,
//     synthesize handwriting, or silently normalize authored graphemes.
// - Allows:
//   - Inputs: Typed scalar statistics, exact grapheme keys, segmentation port.
//   - Outputs: Versioned profile values, edit candidates, typed refusals.
//   - Side effects: Process-local allocation only.
// - Split-When:
//   - Profile serialization or sampling needs independent implementation.
// - Merge-When:
//   - These statistics become inseparable from a shared handwriting profile.
// - Summary:
//   - Enforces measurable limits before editing statistical handwriting data.
// - Description:
//   - Preserves the user-selected parameters without pretending to render ink.
// - Usage:
//   - Validate standalone handwriting statistics before later CLI/UI wiring.
// - Defaults:
//   - A deterministic mean-only baseline with zero sampled deviation.
//

//! Bounded handwriting statistics with exact glyph overrides.
//!
//! Values are integers in the units declared by each [`Dimension`]. This is a
//! profile-validation layer, not a handwriting renderer or calibrated sampler.
//! NFC and NFD spellings remain separate exact keys, as required by the
//! language inventory. A bounded numeric preview is isolated in its module.

#![cfg_attr(
    test,
    expect(
        unused_crate_dependencies,
        reason = "Integration fixtures use the Unicode adapter"
    )
)]

mod preview;

pub use preview::preview_value;

use atrament_unicode_grapheme_boundary_port::GraphemeBoundaryProvider;

/// Version of this manual statistical profile, separate from `.atrament` ZIP.
pub const STATISTICS_PROFILE_VERSION: &str =
    "atrament.handwriting.statistics.v1";

/// Maximum separate exact grapheme overrides in one local profile.
pub const MAXIMUM_GLYPH_OVERRIDES: usize = 512;

/// Maximum UTF-8 bytes for one exact grapheme, including combining marks.
pub const MAXIMUM_GRAPHEME_BYTES: usize = 64;

/// Dimensions implemented as bounded manually editable statistics.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Dimension {
    /// Offset relative to the line baseline, in micrometres.
    BaselineOffset,
    /// Horizontal glyph flattening, in parts per thousand.
    Flattening,
    /// Glyph height, in micrometres.
    Height,
    /// Stroke thickness, in micrometres.
    InkThickness,
    /// Gap between letters, in micrometres.
    LetterSpacing,
    /// Pen pressure, in parts per thousand.
    Pressure,
    /// Corner roundness, in parts per thousand.
    Roundness,
    /// Glyph slant, in millidegrees.
    Slant,
    /// Texture strength, in parts per thousand.
    Texture,
    /// Glyph width, in micrometres.
    Width,
}

impl Dimension {
    /// Stable inventory for the first editable statistical profile.
    pub const ALL: [Self; 10] = [
        Self::BaselineOffset,
        Self::Flattening,
        Self::Height,
        Self::InkThickness,
        Self::LetterSpacing,
        Self::Pressure,
        Self::Roundness,
        Self::Slant,
        Self::Texture,
        Self::Width,
    ];

    /// Deliberately finite safety limits, not measured writer calibration.
    #[must_use]
    pub const fn bounds(self) -> ScalarBounds {
        match self {
            Self::BaselineOffset => ScalarBounds::new(-20_000, 20_000),
            Self::Flattening
            | Self::Pressure
            | Self::Roundness
            | Self::Texture => ScalarBounds::new(0, 1_000),
            Self::Height | Self::Width => ScalarBounds::new(100, 30_000),
            Self::InkThickness => ScalarBounds::new(20, 5_000),
            Self::LetterSpacing => ScalarBounds::new(-10_000, 30_000),
            Self::Slant => ScalarBounds::new(-85_000, 85_000),
        }
    }

    /// Decode only the frozen vocabulary, with no inferred synonyms.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|dimension| dimension.key() == key)
    }

    const fn initial_mean(self) -> i32 {
        match self {
            Self::BaselineOffset
            | Self::Flattening
            | Self::Slant
            | Self::Texture => 0,
            Self::Height => 3_000,
            Self::InkThickness => 300,
            Self::LetterSpacing => 350,
            Self::Pressure | Self::Roundness => 500,
            Self::Width => 2_200,
        }
    }
    /// Stable machine-readable key for one admitted dimension.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::BaselineOffset => "baseline_offset",
            Self::Flattening => "flattening",
            Self::Height => "height",
            Self::InkThickness => "ink_thickness",
            Self::LetterSpacing => "letter_spacing",
            Self::Pressure => "pressure",
            Self::Roundness => "roundness",
            Self::Slant => "slant",
            Self::Texture => "texture",
            Self::Width => "width",
        }
    }

    /// Human-display unit for all five statistics of this parameter.
    #[must_use]
    pub const fn unit(self) -> Unit {
        match self {
            Self::BaselineOffset
            | Self::Height
            | Self::InkThickness
            | Self::LetterSpacing
            | Self::Width => Unit::Micrometres,
            Self::Slant => Unit::Millidegrees,
            Self::Flattening
            | Self::Pressure
            | Self::Roundness
            | Self::Texture => Unit::PartsPerThousand,
        }
    }
}

/// Units shown by the future editor; no floating-point conversion is implicit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Unit {
    /// One thousandth of a millimetre.
    Micrometres,
    /// One thousandth of a degree.
    Millidegrees,
    /// Dimensionless integer ratio from zero to one thousand.
    PartsPerThousand,
}

/// Inclusive absolute manual-editor scalar limits in the parameter's unit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScalarBounds {
    /// Maximum supported value.
    pub maximum: i32,
    /// Minimum supported value.
    pub minimum: i32,
}

impl ScalarBounds {
    const fn new(minimum: i32, maximum: i32) -> Self {
        Self { maximum, minimum }
    }
}

/// Explicit five-statistic description of one manual glyph variable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScalarStatistics {
    /// Maximum permitted sample, in the dimension's unit.
    pub maximum: i32,
    /// User-entered arithmetic mean.
    pub mean: i32,
    /// User-entered median.
    pub median: i32,
    /// Minimum permitted sample, in the dimension's unit.
    pub minimum: i32,
    /// Non-negative standard deviation in the same unit.
    pub standard_deviation: u32,
}

/// Why one manually entered distribution is impossible or out of range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarStatisticsError {
    /// Deviation exceeds the bound possible for this mean and envelope.
    DeviationImpossible,
    /// The maximum is less than the minimum.
    InvertedEnvelope,
    /// The mean is outside the stated envelope.
    MeanOutsideEnvelope,
    /// The requested median and mean cannot coexist within the envelope.
    MedianMeanIncompatible,
    /// The median is outside the stated envelope.
    MedianOutsideEnvelope,
    /// At least one envelope endpoint exceeds the safety limits.
    OutsideDimensionBounds,
}

impl ScalarStatistics {
    /// Validate finite integer ranges and the necessary variance bound.
    ///
    /// The variance bound follows `variance <= (max-mean) * (mean-min)`.
    /// This does not promise that a sampler can match all five statistics.
    ///
    /// # Errors
    ///
    /// Refuses inverted, out-of-range, or statistically impossible values.
    pub fn validate(
        self,
        dimension: Dimension,
    ) -> Result<(), ScalarStatisticsError> {
        if self.minimum > self.maximum {
            return Err(ScalarStatisticsError::InvertedEnvelope);
        }
        let bounds = dimension.bounds();
        if self.minimum < bounds.minimum || self.maximum > bounds.maximum {
            return Err(ScalarStatisticsError::OutsideDimensionBounds);
        }
        if self.mean < self.minimum || self.mean > self.maximum {
            return Err(ScalarStatisticsError::MeanOutsideEnvelope);
        }
        if self.median < self.minimum || self.median > self.maximum {
            return Err(ScalarStatisticsError::MedianOutsideEnvelope);
        }
        let left =
            i128::from(self.maximum).saturating_sub(i128::from(self.mean));
        let right =
            i128::from(self.mean).saturating_sub(i128::from(self.minimum));
        let deviation = i128::from(self.standard_deviation);
        let requested_variance = deviation.saturating_mul(deviation);
        let upper_variance = left.saturating_mul(right);
        if requested_variance > upper_variance {
            return Err(ScalarStatisticsError::DeviationImpossible);
        }
        let doubled_mean = i64::from(self.mean).saturating_mul(2);
        let smallest_sum =
            i64::from(self.minimum).saturating_add(i64::from(self.median));
        let largest_sum =
            i64::from(self.median).saturating_add(i64::from(self.maximum));
        if doubled_mean < smallest_sum || doubled_mean > largest_sum {
            return Err(ScalarStatisticsError::MedianMeanIncompatible);
        }
        Ok(())
    }
}

/// Statistics assigned to one named dimension.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DimensionStatistics {
    /// Authoritative parameter name.
    pub dimension: Dimension,
    /// Five manually entered numerical statistics.
    pub statistics: ScalarStatistics,
}

/// Partial overrides for exactly one authored Unicode grapheme.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlyphStatistics {
    /// The exact NFC or NFD grapheme; no normalization is performed.
    pub grapheme: String,
    /// Zero to ten per-glyph overrides; absent values inherit from global.
    pub parameters: Vec<DimensionStatistics>,
}

/// Versioned manually authored statistical profile, independent from paper.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandwritingStatisticsProfile {
    /// Globally configured parameters; exactly one per supported dimension.
    pub globals: Vec<DimensionStatistics>,
    /// Exact per-grapheme modifications to the global settings.
    pub glyphs: Vec<GlyphStatistics>,
    /// Caller-controlled seed reserved for future deterministic sampling.
    pub seed: u64,
    /// Explicit wire identity; unsupported versions are refused.
    pub version: String,
}

/// Typed profile refusal, including location without logging private glyphs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandwritingStatisticsError {
    /// Two globals name the same dimension.
    DuplicateGlobal(Dimension),
    /// A glyph repeats a dimension.
    DuplicateGlyphDimension {
        /// Zero-based index of the offending glyph.
        glyph_index: usize,
        /// Dimension declared more than once.
        dimension: Dimension,
    },
    /// Two glyphs share the exact same grapheme bytes.
    DuplicateGrapheme {
        /// Earlier declaration index.
        first: usize,
        /// Later duplicate declaration index.
        duplicate: usize,
    },
    /// A global dimension contains invalid statistics.
    InvalidGlobal(Dimension, ScalarStatisticsError),
    /// A glyph override contains invalid statistics.
    InvalidGlyphDimension {
        /// Zero-based index of the offending grapheme.
        glyph_index: usize,
        /// Dimension whose statistics are invalid.
        dimension: Dimension,
        /// Specific statistics refusal.
        reason: ScalarStatisticsError,
    },
    /// One glyph has no overrides or exceeds the supported dimension count.
    InvalidGlyphParameterCount(usize),
    /// A glyph key is empty, oversized, or not exactly one grapheme.
    InvalidGrapheme(usize),
    /// Preview requested an invalid or multi-grapheme exact key.
    InvalidPreviewGrapheme,
    /// A global dimension is missing.
    MissingGlobal(Dimension),
    /// A requested dimension is not overridden for this glyph.
    MissingGlyphDimension(Dimension),
    /// A requested exact glyph override is absent.
    MissingGlyphOverride,
    /// Too many grapheme overrides were supplied.
    TooManyGlyphs,
    /// The profile version is not admitted by this implementation.
    UnsupportedVersion,
}

impl HandwritingStatisticsProfile {
    /// Construct valid baseline controls without inventing sampled variance.
    #[must_use]
    pub fn baseline(seed: u64) -> Self {
        Self {
            globals: Dimension::ALL
                .into_iter()
                .map(|dimension| {
                    let value = dimension.initial_mean();
                    DimensionStatistics {
                        dimension,
                        statistics: ScalarStatistics {
                            maximum: value,
                            mean: value,
                            median: value,
                            minimum: value,
                            standard_deviation: 0,
                        },
                    }
                })
                .collect(),
            glyphs: Vec::new(),
            seed,
            version: STATISTICS_PROFILE_VERSION.to_owned(),
        }
    }

    /// Return a glyph override, or inherit its global value when absent.
    ///
    /// This is exact grapheme matching, not normalization or font fallback.
    #[must_use]
    pub fn statistics_for(
        &self,
        grapheme: &str,
        dimension: Dimension,
    ) -> Option<ScalarStatistics> {
        self.glyphs
            .iter()
            .find(|glyph| glyph.grapheme == grapheme)
            .and_then(|glyph| {
                glyph
                    .parameters
                    .iter()
                    .find(|entry| entry.dimension == dimension)
            })
            .or_else(|| {
                self.globals
                    .iter()
                    .find(|entry| entry.dimension == dimension)
            })
            .map(|entry| entry.statistics)
    }

    /// Validate the whole profile, including inherited and exact glyph data.
    ///
    /// # Errors
    ///
    /// Rejects stale versions, incomplete/ambiguous fields and invalid ranges.
    pub fn validate<Boundary>(
        &self,
        boundaries: &Boundary,
    ) -> Result<(), HandwritingStatisticsError>
    where
        Boundary: GraphemeBoundaryProvider,
    {
        if self.version != STATISTICS_PROFILE_VERSION {
            return Err(HandwritingStatisticsError::UnsupportedVersion);
        }
        if self.glyphs.len() > MAXIMUM_GLYPH_OVERRIDES {
            return Err(HandwritingStatisticsError::TooManyGlyphs);
        }
        for dimension in Dimension::ALL {
            let mut seen = false;
            for entry in &self.globals {
                if entry.dimension == dimension {
                    if seen {
                        return Err(
                            HandwritingStatisticsError::DuplicateGlobal(
                                dimension,
                            ),
                        );
                    }
                    seen = true;
                    entry.statistics.validate(dimension).map_err(|error| {
                        HandwritingStatisticsError::InvalidGlobal(
                            dimension, error,
                        )
                    })?;
                }
            }
            if !seen {
                return Err(HandwritingStatisticsError::MissingGlobal(
                    dimension,
                ));
            }
        }
        for (index, glyph) in self.glyphs.iter().enumerate() {
            if !valid_exact_glyph(&glyph.grapheme, boundaries)
            {
                return Err(HandwritingStatisticsError::InvalidGrapheme(index));
            }
            if let Some(first) = self
                .glyphs
                .iter()
                .take(index)
                .position(|previous| previous.grapheme == glyph.grapheme)
            {
                return Err(HandwritingStatisticsError::DuplicateGrapheme {
                    first,
                    duplicate: index,
                });
            }
            if glyph.parameters.is_empty()
                || glyph.parameters.len() > Dimension::ALL.len()
            {
                return Err(
                    HandwritingStatisticsError::InvalidGlyphParameterCount(
                        index,
                    ),
                );
            }
            for (parameter_index, parameter) in
                glyph.parameters.iter().enumerate()
            {
                if glyph
                    .parameters
                    .iter()
                    .take(parameter_index)
                    .any(|previous| previous.dimension == parameter.dimension)
                {
                    return Err(
                        HandwritingStatisticsError::DuplicateGlyphDimension {
                            glyph_index: index,
                            dimension: parameter.dimension,
                        },
                    );
                }
                parameter.statistics.validate(parameter.dimension).map_err(
                    |reason| {
                        HandwritingStatisticsError::InvalidGlyphDimension {
                            glyph_index: index,
                            dimension: parameter.dimension,
                            reason,
                        }
                    },
                )?;
            }
        }
        Ok(())
    }

    /// Return a validated candidate after replacing one global dimension.
    ///
    /// Original state stays untouched on failure; callers commit the result.
    ///
    /// # Errors
    ///
    /// Refuses a malformed source profile or inadmissible replacement.
    pub fn with_global<Boundary>(
        &self,
        parameter: DimensionStatistics,
        boundaries: &Boundary,
    ) -> Result<Self, HandwritingStatisticsError>
    where
        Boundary: GraphemeBoundaryProvider,
    {
        self.validate(boundaries)?;
        let mut candidate = self.clone();
        if let Some(entry) = candidate
            .globals
            .iter_mut()
            .find(|entry| entry.dimension == parameter.dimension)
        {
            *entry = parameter;
        }
        candidate.validate(boundaries)?;
        Ok(candidate)
    }

    /// Return a validated candidate with one exact grapheme override.
    ///
    /// Replacing a parameter retains other settings for that glyph. A new
    /// grapheme is appended without normalizing NFC/NFD spellings together.
    ///
    /// # Errors
    ///
    /// Refuses malformed source profiles, unsupported keys, or invalid stats.
    pub fn with_glyph<Boundary>(
        &self,
        grapheme: &str,
        parameter: DimensionStatistics,
        boundaries: &Boundary,
    ) -> Result<Self, HandwritingStatisticsError>
    where
        Boundary: GraphemeBoundaryProvider,
    {
        self.validate(boundaries)?;
        let mut candidate = self.clone();
        if let Some(glyph) = candidate
            .glyphs
            .iter_mut()
            .find(|glyph| glyph.grapheme == grapheme)
        {
            if let Some(entry) = glyph
                .parameters
                .iter_mut()
                .find(|entry| entry.dimension == parameter.dimension)
            {
                *entry = parameter;
            } else {
                glyph.parameters.push(parameter);
            }
        } else {
            candidate.glyphs.push(GlyphStatistics {
                grapheme: grapheme.to_owned(),
                parameters: vec![parameter],
            });
        }
        candidate.validate(boundaries)?;
        Ok(candidate)
    }

    /// Return a validated candidate without one exact glyph override.
    ///
    /// When a dimension is selected, only that override is removed; its
    /// inherited global value becomes active. Removing the last dimension
    /// also removes the empty glyph entry. None removes the complete glyph.
    /// Unmatched exact keys and absent dimensions refuse without mutation.
    ///
    /// # Errors
    ///
    /// Refuses invalid source profiles and missing exact overrides.
    pub fn without_glyph_override<Boundary>(
        &self,
        grapheme: &str,
        dimension: Option<Dimension>,
        boundaries: &Boundary,
    ) -> Result<Self, HandwritingStatisticsError>
    where
        Boundary: GraphemeBoundaryProvider,
    {
        self.validate(boundaries)?;
        let mut candidate = self.clone();
        let index = candidate
            .glyphs
            .iter()
            .position(|glyph| glyph.grapheme == grapheme)
            .ok_or(HandwritingStatisticsError::MissingGlyphOverride)?;
        if let Some(selected_dimension) = dimension {
            let glyph = candidate
                .glyphs
                .get_mut(index)
                .ok_or(HandwritingStatisticsError::MissingGlyphOverride)?;
            let before = glyph.parameters.len();
            glyph.parameters
                .retain(|item| item.dimension != selected_dimension);
            if glyph.parameters.len() == before {
                return Err(HandwritingStatisticsError::MissingGlyphDimension(
                    selected_dimension,
                ));
            }
            if glyph.parameters.is_empty() {
                let _removed = candidate.glyphs.remove(index);
            }
        } else {
            let _removed = candidate.glyphs.remove(index);
        }
        candidate.validate(boundaries)?;
        Ok(candidate)
    }
}

/// Whether a bounded key is exactly one statistical handwriting grapheme.
///
/// Transport adapters can validate this source-independent shape before
/// reading an otherwise untrusted selected profile. Preserves NFC and NFD.
/// Unicode source line separators and control characters are not glyphs.
pub fn valid_exact_glyph<Boundary>(
    grapheme: &str, boundaries: &Boundary,
) -> bool
where
    Boundary: GraphemeBoundaryProvider,
{
    !grapheme.is_empty()
        && grapheme.len() <= MAXIMUM_GRAPHEME_BYTES
        && !grapheme.chars().any(|character| {
            character.is_control()
                || character == '\u{2028}'
                || character == '\u{2029}'
        })
        && boundaries.grapheme_count(grapheme) == 1
}
